use super::instruction_count_utils::ConditionCountRecord;
use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType, SharedLogger};
use crate::storage::TestCaseHandle;
use serde::{Serialize, Serializer};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct ConditionCount {
    id: u64,
    count: u64,
    #[serde(serialize_with = "serialize_handle")]
    test_handle: TestCaseHandle,
}

fn serialize_handle<S>(handle: &TestCaseHandle, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(handle.get_unique_id())
}

impl ConditionCount {
    pub fn get_count(&self) -> u64 {
        self.count
    }
}

pub struct InstructionCountGlobalState {
    id_to_condition: HashMap<u64, ConditionCount>,
    logger: SharedLogger,
}

impl InstructionCountGlobalState {
    pub fn new(_config: &PassConfig, logger: SharedLogger) -> Self {
        Self {
            id_to_condition: HashMap::new(),
            logger,
        }
    }

    #[allow(dead_code)]
    pub fn get_condition_counts(&self) -> &HashMap<u64, ConditionCount> {
        &self.id_to_condition
    }
}

impl GlobalState for InstructionCountGlobalState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::InstructionCount
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        Some(vec![PassType::InstructionCounter])
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let tracer_output = update.get_pass_result(PassType::InstructionCounter);
        let mut reader = csv::Reader::from_reader(tracer_output.as_slice());
        let mut conditions = vec![];
        for result in reader.deserialize() {
            let cond_record: ConditionCountRecord = result.expect("Could not parse CSV entry");

            if !cond_record.is_tainted() {
                continue;
            }

            let condition = ConditionCount {
                id: cond_record.get_condition_id(),
                count: cond_record.get_condition_count(),
                test_handle: update.get_test_handle().clone(),
            };

            if let Some(old_condition) = self.id_to_condition.get(&condition.id) {
                if old_condition.get_count() > condition.get_count() {
                    self.id_to_condition
                        .insert(condition.id, condition.clone())
                        .unwrap();
                    conditions.push(condition);
                }
            } else {
                self.id_to_condition.insert(condition.id, condition.clone());
                conditions.push(condition);
            }
        }

        let conditions_serialized = serde_cbor::to_vec(&conditions)
            .expect("Failed to serialize observed conditions analysis diff");
        if let Err(e) = self.logger.lock().unwrap().log_analysis_state(
            update.get_test_handle().clone(),
            update.get_fuzzer_id(),
            self.analysis_type(),
            conditions_serialized,
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
    use std::path::PathBuf;

    #[test]
    fn test_instruction_count_analysis() {
        env::set_var("LD_LIBRARY_PATH", env!("RTLIBS_INSTALL_DIR"));
        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_OBJDUMP_PATH"));
        let empty_path = PathBuf::from("tests/assets/empty");

        let temp_dir = env::temp_dir()
            .join("pass_tests")
            .join("instruction_count_pass");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = PassConfig {
            program_arguments: vec![String::from("-x"), String::from("@@")],
            analysis_artifacts_dir: binaries_dir,
            analysis_input_dir: temp_dir,
        };
        let inst_count_pass = PassType::InstructionCounter
            .get_pass(config.clone())
            .unwrap();

        let mut test_case_file = File::open(empty_path).unwrap();
        let mut test_case = Vec::new();
        test_case_file.read_to_end(&mut test_case).unwrap();
        let output = inst_count_pass.process(&test_case).expect("process failed");

        let test_handle = TestCaseHandle::get_fake_handle("");

        let logger_output_dir = "test_instruction_count_pass";
        let mut instruction_count_state =
            InstructionCountGlobalState::new(&config, create_shared_logger(logger_output_dir));
        let mut update = AnalysisUpdate::new(test_handle.clone(), FuzzerId::new(42), Vec::new());
        update.add_pass_result(PassType::InstructionCounter, output);
        instruction_count_state.update(&update);

        let condition_counts = instruction_count_state.get_condition_counts();
        assert_eq!(condition_counts[&112720].get_count(), 18);

        logger_cleanup(logger_output_dir);
    }

    #[test]
    fn test_instruction_count_analysis_with_count() {
        env::set_var("LD_LIBRARY_PATH", env!("RTLIBS_INSTALL_DIR"));
        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_COUNT_PATH"));
        let cutoff_0_path = PathBuf::from("tests/assets/cutoff_0");
        let cutoff_1_path = PathBuf::from("tests/assets/cutoff_1");

        let temp_dir = env::temp_dir()
            .join("pass_tests")
            .join("instruction_count_count_pass");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = PassConfig {
            program_arguments: vec![],
            analysis_artifacts_dir: binaries_dir.clone(),
            analysis_input_dir: temp_dir,
        };

        let instruction_counter_pass = PassType::InstructionCounter
            .get_pass(config.clone())
            .unwrap();

        let logger_output_dir = "instruction_count_count_pass";
        let mut icount_global_state = InstructionCountGlobalState::new(
            &PassConfig {
                program_arguments: vec![],
                analysis_artifacts_dir: binaries_dir,
                analysis_input_dir: PathBuf::new(),
            },
            create_shared_logger(logger_output_dir),
        );

        let fuzzer_id = FuzzerId::new(42);

        println!("Process cutoff_0");
        let mut cutoff_0_file = File::open(cutoff_0_path).unwrap();
        let mut cutoff_0 = Vec::new();
        cutoff_0_file.read_to_end(&mut cutoff_0).unwrap();
        let cutoff_0_icount_tracer_output = instruction_counter_pass
            .process(&cutoff_0)
            .expect("process failed");
        let cutoff_0_handle = TestCaseHandle::get_fake_handle("cutoff_0");

        let mut update = AnalysisUpdate::new(cutoff_0_handle.clone(), fuzzer_id, Vec::new());
        update.add_pass_result(PassType::InstructionCounter, cutoff_0_icount_tracer_output);
        icount_global_state.update(&update);

        let condition_counts = icount_global_state.get_condition_counts();
        assert_eq!(condition_counts[&36].get_count(), 4);

        println!("Process cutoff_1");
        let mut cutoff_1_file = File::open(cutoff_1_path).unwrap();
        let mut cutoff_1 = Vec::new();
        cutoff_1_file.read_to_end(&mut cutoff_1).unwrap();
        let cutoff_1_icount_tracer_output = instruction_counter_pass
            .process(&cutoff_1)
            .expect("process failed");
        let cutoff_1_handle = TestCaseHandle::get_fake_handle("cutoff_1");

        let mut update = AnalysisUpdate::new(cutoff_1_handle.clone(), fuzzer_id, Vec::new());
        update.add_pass_result(PassType::InstructionCounter, cutoff_1_icount_tracer_output);
        icount_global_state.update(&update);

        let condition_counts = icount_global_state.get_condition_counts();
        assert_eq!(condition_counts[&36].get_count(), 1);

        logger_cleanup(logger_output_dir);
    }
}
