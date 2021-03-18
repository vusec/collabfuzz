use super::observed_conditions_utils::{Condition, ConditionRecord};
use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType, SharedLogger};
use std::collections::HashMap;
use std::convert::TryFrom;

pub struct ObservedConditionsState {
    id_to_condition: HashMap<u64, Condition>,
    logger: SharedLogger,
}

impl ObservedConditionsState {
    pub fn new(_config: &PassConfig, logger: SharedLogger) -> Self {
        Self {
            id_to_condition: HashMap::new(),
            logger,
        }
    }

    #[allow(dead_code)]
    pub fn get_observed_conditions(&self) -> &HashMap<u64, Condition> {
        &self.id_to_condition
    }
}

impl GlobalState for ObservedConditionsState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::ObservedConditions
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        Some(vec![PassType::CondTracer])
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let tracer_output = update.get_pass_result(PassType::CondTracer);
        let mut reader = csv::Reader::from_reader(tracer_output.as_slice());
        let mut conditions = vec![];
        for result in reader.deserialize() {
            let cond_record: ConditionRecord = result.expect("Could not parse CSV entry");
            let condition = Condition::try_from(cond_record).expect("Could not parse cases");

            if let Some(old_condition) = self.id_to_condition.get_mut(&condition.get_id()) {
                old_condition.update_record(condition.clone());
            } else {
                self.id_to_condition
                    .insert(condition.get_id(), condition.clone());
            }
            conditions.push(condition);
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
    use fixedbitset::FixedBitSet;
    use std::env;
    use std::fs;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

    #[test]
    fn test_observed_conditions_analysis() {
        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_OBJDUMP_PATH"));
        let empty_path = PathBuf::from("tests/assets/empty");

        let temp_dir = env::temp_dir()
            .join("pass_tests")
            .join("observed_conditions_pass");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = PassConfig {
            program_arguments: vec![String::from("-d"), String::from("@@")],
            analysis_artifacts_dir: binaries_dir,
            analysis_input_dir: temp_dir,
        };
        let cond_tracer_pass = PassType::CondTracer.get_pass(config.clone()).unwrap();

        let mut test_case_file = File::open(empty_path).unwrap();
        let mut test_case = Vec::new();
        test_case_file.read_to_end(&mut test_case).unwrap();
        let output = cond_tracer_pass
            .process(&test_case)
            .expect("process failed");

        let test_handle = TestCaseHandle::get_fake_handle("");

        let logger_output_dir = "test_observed_conditions_pass";
        let mut observed_conditions_state =
            ObservedConditionsState::new(&config, create_shared_logger(logger_output_dir));
        let mut update = AnalysisUpdate::new(test_handle.clone(), FuzzerId::new(42), Vec::new());
        update.add_pass_result(PassType::CondTracer, output);
        observed_conditions_state.update(&update);

        let observed_conditions = observed_conditions_state.get_observed_conditions();

        // Looking for: 0xe2b4,11010101000000100010001100101100001
        let mut observed_states = FixedBitSet::with_capacity(35);
        observed_states.extend(vec![0, 1, 3, 5, 7, 14, 18, 22, 23, 26, 28, 29, 34]);

        let target_id = 0xe2b4;
        let test_condition = Condition::new(target_id, observed_states);
        let target_condition = observed_conditions
            .get(&target_id)
            .expect("Condition not found");
        assert_eq!(*target_condition, test_condition);

        logger_cleanup(logger_output_dir);
    }
}
