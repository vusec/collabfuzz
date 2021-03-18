use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType, SharedLogger};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// This structure is a copy of the one in the pass runtime
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TerminatorInfo {
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
    pub fn extend_input_offsets(&mut self, other_offsets: &BTreeSet<usize>) {
        self.input_offsets.extend(other_offsets);
    }

    #[allow(dead_code)]
    pub fn get_times_seen(&self) -> usize {
        self.times_seen
    }

    pub fn get_input_offsets(&self) -> &BTreeSet<usize> {
        &self.input_offsets
    }

    pub fn get_conditions_before_count(&self) -> usize {
        self.conditions_before_count
    }

    #[allow(dead_code)]
    pub fn get_tainted_conditions_before_count(&self) -> usize {
        self.tainted_conditions_before_count
    }
}

pub struct ConditionBytesGlobalState {
    ids_to_info: BTreeMap<u64, TerminatorInfo>,
    logger: SharedLogger,
}

impl ConditionBytesGlobalState {
    pub fn new(_config: &PassConfig, logger: SharedLogger) -> Self {
        Self {
            ids_to_info: BTreeMap::new(),
            logger,
        }
    }

    #[allow(dead_code)]
    pub fn get_condition_bytes_map(&self) -> &BTreeMap<u64, TerminatorInfo> {
        &self.ids_to_info
    }
}

impl GlobalState for ConditionBytesGlobalState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::ConditionBytes
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        Some(vec![PassType::BytesTracer])
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let tracer_output = update.get_pass_result(PassType::BytesTracer);
        if tracer_output.is_empty() {
            log::warn!("Bytes tracer analysis failed, skipping update");
            return;
        }

        let ids_to_info_update: BTreeMap<u64, TerminatorInfo> =
            serde_json::from_slice(tracer_output.as_slice()).expect("Could not deserialize JSON");

        let mut logger_update = BTreeMap::new();
        for (instruction_id, info_update) in &ids_to_info_update {
            if let Some(global_info) = self.ids_to_info.get_mut(&instruction_id) {
                // Since within the same execution we are keeping always the first occurrance of a
                // condition, globally it makes sense to keep the one with the lowest number of
                // conditions before it. This is the least deep occurrance in the program.
                if global_info.get_conditions_before_count()
                    > info_update.get_conditions_before_count()
                {
                    *global_info = info_update.clone();
                    logger_update.insert(*instruction_id, info_update.clone());
                }
            } else {
                self.ids_to_info
                    .insert(*instruction_id, info_update.clone());
                logger_update.insert(*instruction_id, info_update.clone());
            }
        }

        let update_serialized = serde_cbor::to_vec(&logger_update)
            .expect("Failed to serialize observed conditions analysis diff");
        if let Err(e) = self.logger.lock().unwrap().log_analysis_state(
            update.get_test_handle().clone(),
            update.get_fuzzer_id(),
            self.analysis_type(),
            update_serialized,
        ) {
            log::error!("Failed to log analysis state: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzers::FuzzerId;
    use crate::logger::tests::{cleanup as logger_cleanup, create_shared_logger};
    use crate::storage::TestCaseHandle;
    use std::env;
    use std::fs;
    use std::fs::File;
    use std::io::Read;
    use std::iter::FromIterator;
    use std::path::PathBuf;

    #[test]
    fn test_condition_bytes_analysis() {
        let _ = env_logger::builder().is_test(true).try_init();

        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_OBJDUMP_PATH"));
        let empty_path = PathBuf::from("tests/assets/empty");

        let temp_dir = env::temp_dir()
            .join("pass_tests")
            .join("condition_bytes_pass");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = PassConfig {
            // XXX: `-d @@` does not work because of an indirect vararg call to fprintf
            program_arguments: vec![String::from("-x"), String::from("@@")],
            analysis_artifacts_dir: binaries_dir,
            analysis_input_dir: temp_dir,
        };
        let bytes_tracer_pass = PassType::BytesTracer.get_pass(config.clone()).unwrap();

        let mut test_case_file = File::open(empty_path).unwrap();
        let mut test_case = Vec::new();
        test_case_file.read_to_end(&mut test_case).unwrap();
        let output = bytes_tracer_pass
            .process(&test_case)
            .expect("process failed");

        let test_handle = TestCaseHandle::get_fake_handle("");

        let logger_output_dir = "test_condition_bytes_pass";
        let mut condition_bytes_state =
            ConditionBytesGlobalState::new(&config, create_shared_logger(logger_output_dir));
        let mut update = AnalysisUpdate::new(test_handle.clone(), FuzzerId::new(42), Vec::new());
        update.add_pass_result(PassType::BytesTracer, output);
        condition_bytes_state.update(&update);

        let ids_to_info = condition_bytes_state.get_condition_bytes_map();

        let target_id = 109025;
        let target_input_offsets =
            BTreeSet::from_iter(vec![40, 41, 42, 43, 44, 45, 46, 47, 60, 61]);
        let target_info = ids_to_info.get(&target_id).expect("Condition not found");
        assert_eq!(target_info.get_times_seen(), 1);
        assert_eq!(target_info.get_input_offsets(), &target_input_offsets);
        assert_eq!(target_info.get_conditions_before_count(), 30);
        assert_eq!(target_info.get_tainted_conditions_before_count(), 8);

        logger_cleanup(logger_output_dir);
    }

    #[test]
    fn test_condition_bytes_analysis_big() {
        let _ = env_logger::builder().is_test(true).try_init();

        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_OBJDUMP_PATH"));
        let empty_path = PathBuf::from("tests/assets/gimp-2.10");

        let temp_dir = env::temp_dir()
            .join("pass_tests")
            .join("condition_bytes_big_pass");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = PassConfig {
            // XXX: `-d @@` does not work because of an indirect vararg call to fprintf
            program_arguments: vec![String::from("-x"), String::from("@@")],
            analysis_artifacts_dir: binaries_dir,
            analysis_input_dir: temp_dir,
        };
        let bytes_tracer_pass = PassType::BytesTracer.get_pass(config.clone()).unwrap();

        let mut test_case_file = File::open(empty_path).unwrap();
        let mut test_case = Vec::new();
        test_case_file.read_to_end(&mut test_case).unwrap();
        let output = bytes_tracer_pass.process(&test_case);

        assert!(output.is_err());

        // let test_handle = TestCaseHandle::get_fake_handle("");

        // let logger_output_dir = "test_condition_bytes_big_pass";
        // let mut condition_bytes_state =
        //     ConditionBytesGlobalState::new(&config, create_shared_logger(logger_output_dir));
        // let mut update = AnalysisUpdate::new(test_handle.clone(), FuzzerId::new(42), Vec::new());
        // update.add_pass_result(PassType::BytesTracer, output.unwrap());
        // condition_bytes_state.update(&update);

        // let ids_to_info = condition_bytes_state.get_condition_bytes_map();

        // let target_id = 109025;
        // let target_input_offsets =
        //     BTreeSet::from_iter(vec![40, 41, 42, 43, 44, 45, 46, 47, 60, 61]);
        // let target_info = ids_to_info.get(&target_id).expect("Condition not found");
        // assert_eq!(target_info.get_times_seen(), 1);
        // assert_eq!(target_info.get_input_offsets(), &target_input_offsets);
        // assert_eq!(target_info.get_conditions_before_count(), 30);
        // assert_eq!(target_info.get_tainted_conditions_before_count(), 8);

        // logger_cleanup(logger_output_dir);
    }
}
