use super::utils::get_artifact_path;
use super::{Pass, PassConfig, PassError, PassType};
use crate::analysis::analyses::TerminatorInfo;
use std::cmp::min;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const PASS_NAME: &str = "bytes_tracer";
const MAX_ANALYSIS_TIME: Duration = Duration::from_secs(2);
const DIE_EXIT_CODE: i32 = 42;

pub struct BytesTracerPass {
    executable_path: PathBuf,
    input_file_path: PathBuf,
    output_file_path: PathBuf,
    program_arguments: Vec<String>,
}

impl BytesTracerPass {
    pub fn new(config: PassConfig) -> Result<Self, PassError> {
        let executable_path = get_artifact_path(&config.analysis_artifacts_dir, PASS_NAME)
            .map_err(PassError::FailedToGetBin)?;

        let input_file_name = format!("{}_input", PASS_NAME);
        let input_file_path = config.analysis_input_dir.join(input_file_name);

        let output_file_name = format!("{}_output", PASS_NAME);
        let output_file_path = config.analysis_input_dir.join(output_file_name);

        let mut input_in_stdin = true;
        let program_arguments = config
            .program_arguments
            .iter()
            .map(|arg| {
                if arg == "@@" {
                    input_in_stdin = false;
                    input_file_path.to_string_lossy().into_owned()
                } else {
                    String::from(arg)
                }
            })
            .collect();
        if input_in_stdin {
            return Err(PassError::StdinNotSupported);
        }

        Ok(Self {
            executable_path,
            input_file_path,
            output_file_path,
            program_arguments,
        })
    }

    fn process_range(
        &self,
        range_start: usize,
        range_end: usize,
    ) -> Result<BTreeMap<u64, TerminatorInfo>, ()> {
        log::trace!(
            "Launching instrumented binary {} with range: [{},{})",
            self.executable_path.file_name().unwrap().to_str().unwrap(),
            range_start,
            range_end
        );

        let execution_start = Instant::now();

        let mut child = Command::new(&self.executable_path)
            .args(&self.program_arguments)
            .env(
                "DFSAN_OPTIONS",
                format!("strict_data_dependencies=0,exitcode={}", DIE_EXIT_CODE),
            )
            .env("TRACER_OUTPUT_FILE", &self.output_file_path)
            .env("TRACER_INPUT_FILE", &self.input_file_path)
            .env("TRACER_RANGE_START", range_start.to_string())
            .env("TRACER_RANGE_SIZE", (range_end - range_start).to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Command failed to start");

        // TODO: Use try_wait and sleep to avoid hanging
        log::trace!("Waiting for child process to exit");
        let exit_status = child.wait().expect("Could not wait the child process");

        let execution_end = Instant::now();
        log::trace!(
            "Execution took: {:?}",
            execution_end.duration_since(execution_start)
        );

        if let Some(exit_code) = exit_status.code() {
            if exit_code == DIE_EXIT_CODE {
                return Err(());
            }
        }

        log::trace!("Reading output file: {}", self.output_file_path.display());
        let output_file = File::open(&self.output_file_path).expect("Could not open output file");
        let mut output_reader = BufReader::new(output_file);

        match serde_json::from_reader(output_reader.by_ref()) {
            Ok(chunk_map) => Ok(chunk_map),
            Err(error) => {
                output_reader.seek(SeekFrom::Start(0)).unwrap();
                for line in output_reader.lines() {
                    log::error!("{}", line.unwrap());
                }
                panic!("Failed to parse JSON file: {}", error);
            }
        }
    }

    fn extend_offsets_map(
        ids_to_info: &mut BTreeMap<u64, TerminatorInfo>,
        update_map: &BTreeMap<u64, TerminatorInfo>,
    ) {
        for (instruction_id, chunk_terminator_info) in update_map {
            if let Some(terminator_info) = ids_to_info.get_mut(&instruction_id) {
                terminator_info.extend_input_offsets(chunk_terminator_info.get_input_offsets());
            } else {
                ids_to_info.insert(*instruction_id, chunk_terminator_info.clone());
            }
        }
    }

    fn average_duration(durations: &[Duration]) -> Duration {
        let duration_sum: Duration = durations.iter().sum();
        duration_sum / durations.len() as u32
    }

    fn process_with_chunk_size(
        &self,
        chunk_size: usize,
        test_case_len: usize,
        ids_to_info: &mut BTreeMap<u64, TerminatorInfo>,
        last_end_offt: &mut usize,
    ) -> Result<Duration, Duration> {
        let mut durations = Vec::new();

        for start_offt in (*last_end_offt..test_case_len).step_by(chunk_size) {
            assert_eq!(start_offt, *last_end_offt);
            let end_offt = min(start_offt + chunk_size, test_case_len);

            let process_range_start = Instant::now();
            let process_range_result = self.process_range(start_offt, end_offt);
            let process_range_end = Instant::now();
            durations.push(process_range_end.duration_since(process_range_start));

            if let Ok(chunk_map) = process_range_result {
                Self::extend_offsets_map(ids_to_info, &chunk_map);
                *last_end_offt = end_offt;
            } else {
                return Err(Self::average_duration(&durations));
            }
        }
        assert!(*last_end_offt <= test_case_len);

        Ok(Self::average_duration(&durations))
    }
}

impl Pass for BytesTracerPass {
    fn pass_type(&self) -> PassType {
        PassType::BytesTracer
    }

    fn process(&self, test_case: &[u8]) -> Result<Vec<u8>, PassError> {
        log::debug!("Writing test case to file");
        let mut input_file =
            File::create(&self.input_file_path).expect("Could not open input file");
        input_file
            .write_all(test_case)
            .expect("Could not write input file");

        let mut ids_to_info: BTreeMap<u64, TerminatorInfo> = BTreeMap::new();
        let mut last_end_offt = 0;
        let mut chunk_size = test_case.len();
        let test_case_len = test_case.len();

        loop {
            let process_result = self.process_with_chunk_size(
                chunk_size,
                test_case_len,
                &mut ids_to_info,
                &mut last_end_offt,
            );

            log::debug!("Current progress: {}/{}", last_end_offt, test_case_len);

            if let Ok(average_duration) = process_result {
                log::debug!("Analysis succeeded, avg time: {:?}", average_duration);
                break;
            }

            log::debug!(
                "Analysis failed at offset {}, reducing chunk size",
                last_end_offt
            );
            chunk_size /= 2;

            let remaining_executions = (test_case_len - last_end_offt) / chunk_size;
            let average_error_duration = process_result.unwrap_err();
            let expected_time = average_error_duration * remaining_executions as u32;
            if expected_time > MAX_ANALYSIS_TIME {
                return Err(PassError::AnalysisFailed);
            }

            log::debug!(
                "Trying with chunk size: {}, expected executions: {}, expected time: {:?}",
                chunk_size,
                remaining_executions,
                expected_time
            );
        }
        assert_eq!(test_case_len, last_end_offt);

        let output_bytes =
            serde_json::to_vec(&ids_to_info).expect("Could not serialize aggregated map");

        Ok(output_bytes)
    }
}
