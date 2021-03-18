use super::dfsan_utils::DFSanResult;
use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType};
use crate::types::SharedLogger;

pub struct TaintedConditionsGlobalState {
    logger: SharedLogger,
}

impl TaintedConditionsGlobalState {
    pub fn new(_config: &PassConfig, logger: SharedLogger) -> Self {
        Self { logger }
    }
}

impl GlobalState for TaintedConditionsGlobalState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::TaintedConditions
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        Some(vec![PassType::BBTaintTracer])
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let bb_tracer_output = update.get_pass_result(PassType::BBTaintTracer);
        let mut reader = csv::Reader::from_reader(bb_tracer_output.as_slice());

        let mut tainted_conditions = Vec::new();
        for result in reader.deserialize() {
            let dfsan_result: DFSanResult = result.expect("Could not parse DFSan CSV entry");
            if dfsan_result.is_tainted() {
                tainted_conditions.push(dfsan_result.get_terminator_id());
            }
        }

        let serialized_tainted = serde_cbor::to_vec(&tainted_conditions)
            .expect("Failed to serialize tainted conditions");

        {
            let logger = self.logger.lock().unwrap();
            if let Err(e) = logger.log_analysis_state(
                update.get_test_handle().clone(),
                update.get_fuzzer_id(),
                self.analysis_type(),
                serialized_tainted,
            ) {
                log::error!("Failed to log analysis state: {}", e);
            }
        }
    }
}
