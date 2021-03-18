use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType};
use crate::storage::TestCaseHandle;
use std::collections::HashSet;

pub struct TestAnalysisState {
    observed_files: HashSet<TestCaseHandle>,
}

impl TestAnalysisState {
    pub fn new(_config: &PassConfig) -> Self {
        Self {
            observed_files: HashSet::new(),
        }
    }

    pub fn get_observed(&self) -> HashSet<TestCaseHandle> {
        self.observed_files.clone()
    }
}

impl GlobalState for TestAnalysisState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::Test
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        Some(vec![PassType::Test])
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        log::debug!("TestGlobalState: update");

        let result = update.get_pass_result(PassType::Test);

        if result.len() != 8 {
            panic!("Wrong report length!");
        }

        let mut val_bytes = [0; 8];
        val_bytes.clone_from_slice(&result);
        let val = u64::from_le_bytes(val_bytes);
        if val != 42 {
            panic!("Wrong report content!");
        }

        self.observed_files.insert(update.get_test_handle().clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzers::FuzzerId;
    use std::path::PathBuf;

    #[test]
    fn test_test_analysis() {
        let config = PassConfig {
            program_arguments: vec![String::from("arg1"), String::from("arg2")],
            analysis_artifacts_dir: PathBuf::new(),
            analysis_input_dir: PathBuf::new(),
        };
        let test_pass = PassType::Test.get_pass(config.clone()).unwrap();

        let test_case = Vec::new();
        let output = test_pass.process(&test_case).expect("process failed");

        let test_handle = TestCaseHandle::get_fake_handle("");

        let mut test_global_state = TestAnalysisState::new(&config);
        let mut update = AnalysisUpdate::new(test_handle.clone(), FuzzerId::new(42), Vec::new());
        update.add_pass_result(PassType::Test, output);
        test_global_state.update(&update);

        let observed_files = test_global_state.get_observed();
        assert!(observed_files.contains(&test_handle));
    }
}
