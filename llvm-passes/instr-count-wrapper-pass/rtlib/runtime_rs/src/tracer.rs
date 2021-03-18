use super::{IDType, ShadowType};
use csv::Writer;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize)]
struct ConditionRecord {
    condition_id: String,
    condition_count: ShadowType,
}

pub struct Tracer {
    output_path: PathBuf,
    enable_output: bool,
    conditions_to_counts: HashMap<IDType, ShadowType>,
}

impl Tracer {
    pub fn new(output_path: PathBuf, enable_output: bool) -> Self {
        Self {
            output_path,
            enable_output,
            conditions_to_counts: HashMap::new(),
        }
    }

    pub fn trace_terminator_taint(&mut self, terminator_id: IDType, current_count: ShadowType) {
        if !self.enable_output {
            return;
        }

        if let Some(count_ref) = self.conditions_to_counts.get_mut(&terminator_id) {
            if *count_ref == 0 || *count_ref > current_count {
                *count_ref = current_count;
            }
        } else {
            self.conditions_to_counts
                .insert(terminator_id, current_count);
        }
    }

    pub fn write_data(&self) -> csv::Result<()> {
        if !self.enable_output {
            return Ok(());
        }

        let mut writer = Writer::from_path(&self.output_path)?;
        for (condition_id_ref, condition_count_ref) in &self.conditions_to_counts {
            writer.serialize(ConditionRecord {
                condition_id: format!("{:#x}", *condition_id_ref),
                condition_count: *condition_count_ref,
            })?;
        }

        Ok(())
    }
}
