use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType, SharedLogger};
use crate::fuzzers::FuzzerId;
use crate::storage::TestCaseHandle;
use std::collections::HashMap;

pub struct FuzzerIdState {
    test_case_to_fuzzer_ids: HashMap<TestCaseHandle, Vec<FuzzerId>>,
}

impl FuzzerIdState {
    pub fn new(_config: &PassConfig, _logger: SharedLogger) -> Self {
        FuzzerIdState {
            test_case_to_fuzzer_ids: HashMap::new(),
        }
    }

    pub fn get_fuzzer_ids(&self, test_handle: &TestCaseHandle) -> Option<&[FuzzerId]> {
        self.test_case_to_fuzzer_ids
            .get(test_handle)
            .map(|v| v.as_slice())
    }
}

impl GlobalState for FuzzerIdState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::FuzzerId
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        None
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let test_handle = update.get_test_handle();
        if self.test_case_to_fuzzer_ids.contains_key(test_handle) {
            // The last duplicate reported will always be the last in the vector
            self.test_case_to_fuzzer_ids
                .get_mut(test_handle)
                .unwrap()
                .push(update.get_fuzzer_id());
        } else {
            self.test_case_to_fuzzer_ids
                .insert(test_handle.clone(), vec![update.get_fuzzer_id()]);
        }
    }
}
