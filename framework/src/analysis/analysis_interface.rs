use super::{AnalysisType, PassType};
use crate::fuzzers::FuzzerId;
use crate::storage::TestCaseHandle;
use downcast_rs::{impl_downcast, Downcast};
use std::collections::HashMap;

pub trait GlobalState: Send + Downcast {
    fn analysis_type(&self) -> AnalysisType;
    fn get_required_passes(&self) -> Option<Vec<PassType>>;
    fn update(&mut self, update: &AnalysisUpdate);
}
impl_downcast!(GlobalState);

#[derive(Debug)]
pub struct AnalysisUpdate {
    // Not fuzzer specific
    test_handle: TestCaseHandle,
    pass_type_to_result: HashMap<PassType, Option<Vec<u8>>>,

    // Fuzzer specific
    fuzzer_id: FuzzerId,
    parent_handles: Vec<TestCaseHandle>,
}

impl AnalysisUpdate {
    pub fn new(
        test_handle: TestCaseHandle,
        fuzzer_id: FuzzerId,
        parent_handles: Vec<TestCaseHandle>,
    ) -> Self {
        Self {
            test_handle,
            fuzzer_id,
            parent_handles,
            pass_type_to_result: HashMap::new(),
        }
    }

    pub fn add_pass_result(&mut self, analysis_type: PassType, result: Vec<u8>) {
        self.pass_type_to_result.insert(analysis_type, Some(result));
    }

    pub fn skip_pass(&mut self, analysis_type: PassType) {
        self.pass_type_to_result.insert(analysis_type, None);
    }

    pub fn has_pass_results(&self, analysis_types: &[&PassType]) -> bool {
        for analysis_type in analysis_types {
            if !self.pass_type_to_result.contains_key(analysis_type) {
                return false;
            }
        }

        true
    }

    pub fn get_pass_result(&self, analysis_type: PassType) -> &Vec<u8> {
        self.pass_type_to_result
            .get(&analysis_type)
            .unwrap_or_else(|| panic!("Pass result not available: {}", analysis_type))
            .as_ref()
            .unwrap_or_else(|| panic!("Pass not performed on duplicates"))
    }

    pub fn get_test_handle(&self) -> &TestCaseHandle {
        &self.test_handle
    }

    pub fn get_fuzzer_id(&self) -> FuzzerId {
        self.fuzzer_id
    }

    pub fn get_parent_handles(&self) -> &[TestCaseHandle] {
        self.parent_handles.as_slice()
    }
}
