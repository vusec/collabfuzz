use crate::dfsan::{dfsan_get_base_labels, dfsan_label};
use crate::IDType;
use once_cell::sync::OnceCell;
use serde::Serialize;
use snafu::{ensure, ResultExt, Snafu};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

static TRACER: OnceCell<Mutex<Tracer>> = OnceCell::new();

#[derive(Serialize)]
struct TerminatorInfo {
    /// Number of times this terminator has been encountered
    times_seen: usize,

    /// Input file offsets that taint this terminator when reached the first time
    input_offsets: BTreeSet<usize>,

    /// Number of tainted conditions that have been observed before reaching this terminator for
    /// the first time
    conditions_before_count: usize,

    /// Number of conditions that are tainted by the same bytes in taint_labels and have been
    /// observed before reaching this terminator for the first time
    tainted_conditions_before_count: usize,
}

impl TerminatorInfo {
    pub fn new(
        terminator_label: dfsan_label,
        terminator_map: &BTreeMap<IDType, TerminatorInfo>,
        translation_map: &BTreeMap<dfsan_label, usize>,
    ) -> Self {
        let input_offsets = Self::resolve_dfsan_label(terminator_label, translation_map);

        let tainted_conditions_before_count = terminator_map
            .values()
            .filter(|info| info.is_tainted_by_offsets(&input_offsets))
            .count();

        Self {
            times_seen: 1,
            input_offsets,
            conditions_before_count: terminator_map.len(),
            tainted_conditions_before_count,
        }
    }

    fn resolve_dfsan_label(
        terminator_label: dfsan_label,
        translation_map: &BTreeMap<dfsan_label, usize>,
    ) -> BTreeSet<usize> {
        let base_labels = dfsan_get_base_labels(terminator_label);

        // Translate labels into offsets
        base_labels
            .iter()
            .map(|base_label| translation_map[&base_label])
            .collect()
    }

    pub fn increment_seen_count(&mut self) {
        self.times_seen += 1;
    }

    fn is_tainted_by_offsets(&self, other_offsets: &BTreeSet<usize>) -> bool {
        !self.input_offsets.is_disjoint(other_offsets)
    }
}

#[derive(Default)]
pub struct Tracer {
    output_path_opt: Option<PathBuf>,
    ids_to_info: BTreeMap<IDType, TerminatorInfo>,
}

impl Tracer {
    pub fn global() -> Option<MutexGuard<'static, Self>> {
        let lock = TRACER.get()?;
        Some(lock.lock().unwrap())
    }

    pub fn is_enabled(&self) -> bool {
        self.output_path_opt.is_some()
    }

    pub fn trace_terminator_taint(
        &mut self,
        instruction_id: IDType,
        traced_value_label: dfsan_label,
        translation_map: &BTreeMap<dfsan_label, usize>,
    ) {
        if self.output_path_opt.is_none() {
            log::trace!("Instrumentation disabled");
            return;
        }

        if traced_value_label == 0 {
            log::trace!("No label present");
            return;
        }

        if let Some(info) = self.ids_to_info.get_mut(&instruction_id) {
            log::trace!("Existing instruction encountered: {}", instruction_id);
            info.increment_seen_count();
        } else {
            log::trace!("New instruction encountered: {}", instruction_id);
            let info = TerminatorInfo::new(traced_value_label, &self.ids_to_info, translation_map);
            self.ids_to_info.insert(instruction_id, info);
        }
    }

    pub fn write_data(&self) -> Result<(), TracerError> {
        let output_path = if let Some(output_path) = self.output_path_opt.as_ref() {
            output_path
        } else {
            log::debug!("Instrumentation disabled");
            return Ok(());
        };

        log::info!("Writing results to file");
        let being_serialization = Instant::now();

        let mut output_file = BufWriter::new(File::create(output_path).context(OpenOutputError)?);
        if log::log_enabled!(log::Level::Debug) {
            serde_json::to_writer_pretty(output_file.by_ref(), &self.ids_to_info)
                .context(JSONSerializeError)?;
        } else {
            serde_json::to_writer(output_file.by_ref(), &self.ids_to_info)
                .context(JSONSerializeError)?;
        }
        output_file.flush().context(FileFlushError)?;

        let end_serialization = Instant::now();
        log::debug!(
            "Serialization took: {:?}",
            end_serialization.duration_since(being_serialization)
        );

        Ok(())
    }
}

pub struct TracerBuilder {
    output_path_opt: Option<PathBuf>,
}

impl TracerBuilder {
    pub fn new() -> Self {
        Self {
            output_path_opt: None,
        }
    }

    pub fn output_file(&mut self, file_path: PathBuf) -> &mut Self {
        self.output_path_opt = Some(file_path);
        self
    }

    pub fn build_global(self) -> Result<(), TracerError> {
        let tracer = if let Some(output_path) = self.output_path_opt {
            Tracer {
                output_path_opt: Some(output_path),
                ..Default::default()
            }
        } else {
            log::info!("Tracer disabled");
            Default::default()
        };

        ensure!(TRACER.set(Mutex::new(tracer)).is_ok(), AlreadyExists);

        Ok(())
    }
}

#[derive(Debug, Snafu)]
pub enum TracerError {
    #[snafu(display("Tracer has already been instantiated"))]
    AlreadyExists,
    #[snafu(display("Could not open output file: {}", source))]
    OpenOutputError { source: io::Error },
    #[snafu(display("Could not serialize analysis output: {}", source))]
    JSONSerializeError { source: serde_json::Error },
    #[snafu(display("Could not flush output file: {}", source))]
    FileFlushError { source: io::Error },
}
