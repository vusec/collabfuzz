use super::instruction_count_utils::ConditionCountRecord;
use super::libsvm::{Model, Query};
use super::observed_conditions_utils::{Condition, ConditionRecord};
use super::static_branch_metrics::StaticBranchMetrics;
use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType, SharedLogger};
use crate::fuzzers::{get_fuzzer_types, FuzzerType};
use crate::storage::TestCaseHandle;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;

pub struct RegressorGlobalState {
    logger: SharedLogger,
    static_branch_metrics: StaticBranchMetrics,
    models: HashMap<FuzzerType, Model>,
    id_to_icount: HashMap<u64, u64>,
    id_to_condition: HashMap<u64, Condition>,
    predictions: HashMap<TestCaseHandle, HashMap<u64, HashMap<FuzzerType, f64>>>,
}

impl RegressorGlobalState {
    pub fn new(config: &PassConfig, logger: SharedLogger) -> Self {
        let mut models = HashMap::new();
        for fuzzer_type in get_fuzzer_types() {
            if fuzzer_type == FuzzerType::Unknown || fuzzer_type == FuzzerType::ANGORA {
                continue;
            }
            models.insert(
                fuzzer_type,
                match Model::load(fuzzer_type) {
                    Ok(model) => model,
                    Err(e) => panic!("Failed to load SVM model for {}: {}", fuzzer_type, e),
                },
            );
        }

        Self {
            logger,
            static_branch_metrics: StaticBranchMetrics::new(&config.analysis_artifacts_dir),
            models,
            id_to_icount: HashMap::new(),
            id_to_condition: HashMap::new(),
            predictions: HashMap::new(),
        }
    }

    pub fn get_test_case_predictions(
        &self,
        test_handle: &TestCaseHandle,
    ) -> Option<&HashMap<u64, HashMap<FuzzerType, f64>>> {
        self.predictions.get(test_handle)
    }
}

impl GlobalState for RegressorGlobalState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::RegressorPredictions
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        Some(vec![PassType::CondTracer, PassType::InstructionCounter])
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let test_handle = update.get_test_handle();

        let icount_output = update.get_pass_result(PassType::InstructionCounter);
        let mut reader = csv::Reader::from_reader(icount_output.as_slice());
        let mut tainted_conditions = HashSet::new();
        for result in reader.deserialize() {
            let cond_record: ConditionCountRecord = result.expect("Could not parse CSV entry");

            if !cond_record.is_tainted() {
                continue;
            }

            let condition_id = cond_record.get_condition_id();
            tainted_conditions.insert(condition_id);

            let new_icount = cond_record.get_condition_count();
            if let Some(old_icount) = self.id_to_icount.get(&condition_id) {
                if new_icount < *old_icount {
                    self.id_to_icount.insert(condition_id, new_icount);
                }
            } else {
                self.id_to_icount.insert(condition_id, new_icount);
            }
        }

        log::debug!(
            "Test case '{}' has {} tainted conditions",
            test_handle.get_unique_id(),
            tainted_conditions.len()
        );

        let tracer_output = update.get_pass_result(PassType::CondTracer);
        let mut reader = csv::Reader::from_reader(tracer_output.as_slice());
        let mut unsolved_conditions = HashMap::new();
        for result in reader.deserialize() {
            let cond_record: ConditionRecord = result.expect("Could not parse CSV entry");
            let condition = Condition::try_from(cond_record).expect("Could not parse cases");

            let is_unsolved =
                if let Some(old_condition) = self.id_to_condition.get_mut(&condition.get_id()) {
                    old_condition.update_record(condition.clone());
                    old_condition.is_unsolved()
                } else {
                    self.id_to_condition
                        .insert(condition.get_id(), condition.clone());
                    condition.is_unsolved()
                };

            if is_unsolved && tainted_conditions.contains(&condition.get_id()) {
                unsolved_conditions.insert(condition.get_id(), HashMap::new());
            }
        }

        log::debug!(
            "Test case '{}' has {} unsolved conditions",
            test_handle.get_unique_id(),
            unsolved_conditions.len()
        );

        for (cond, fuzzers_map) in unsolved_conditions.iter_mut() {
            let icount = if let Some(icount) = self.id_to_icount.get(cond) {
                *icount
            } else {
                log::warn!("Instruction count not found for condition {}", cond);
                continue;
            };

            let query = Query::new(self.static_branch_metrics.get_metrics(*cond), icount);

            for (fuzzer_type, model) in self.models.borrow() {
                let prediction = model.predict(&query);
                fuzzers_map.insert(*fuzzer_type, prediction);
            }
        }

        let serialized_state = serde_cbor::to_vec(&unsolved_conditions)
            .expect("Failed to serialize regressor predictions analysis");
        if let Err(e) = self.logger.lock().unwrap().log_analysis_state(
            test_handle.clone(),
            update.get_fuzzer_id(),
            self.analysis_type(),
            serialized_state,
        ) {
            log::error!("Failed to log analysis state: {}", e);
        }

        self.predictions
            .insert(test_handle.clone(), unsolved_conditions);
    }
}
