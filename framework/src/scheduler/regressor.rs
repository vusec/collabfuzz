use super::{AnalysisType, ScheduleMessage, Scheduler, SchedulerFacadeRef};
use crate::analysis::{FuzzerIdState, RegressorGlobalState};
use std::collections::HashSet;

pub const SCHEDULER_NAME: &str = "regressor";

pub struct RegressorScheduler {
    facade_ref: SchedulerFacadeRef,
}

impl RegressorScheduler {
    pub fn new(facade_ref: SchedulerFacadeRef) -> Self {
        Self { facade_ref }
    }
}

impl Scheduler for RegressorScheduler {
    fn schedule(&mut self, schedule_message: ScheduleMessage) {
        let test_handle = match schedule_message {
            ScheduleMessage::Timeout => {
                log::debug!("Do nothing on timeout");
                return;
            }
            ScheduleMessage::DuplicateTestCase(test_handle) => {
                log::debug!("Duplicate test case reported, running");
                test_handle
            }
            ScheduleMessage::NewTestCase(test_handle) => {
                log::debug!("New seed reported, running");
                test_handle
            }
        };

        let mut facade = self.facade_ref.get_facade();

        let regressor_state = facade
            .get_analysis_state(AnalysisType::RegressorPredictions)
            .downcast_ref::<RegressorGlobalState>()
            .unwrap();

        let predictions = regressor_state
            .get_test_case_predictions(&test_handle)
            .unwrap_or_else(|| {
                panic!(
                    "Could not find predictions for test case {}",
                    test_handle.get_unique_id()
                );
            });

        let fuzzer_id_state = facade
            .get_analysis_state(AnalysisType::FuzzerId)
            .downcast_ref::<FuzzerIdState>()
            .unwrap();

        let sender_id = *fuzzer_id_state
            .get_fuzzer_ids(&test_handle)
            .unwrap_or_else(|| {
                panic!(
                    "Failed to retrieve fuzzer id for test case {}",
                    test_handle.get_unique_id()
                )
            })
            .last()
            .unwrap_or_else(|| {
                panic!(
                    "Failed to retrieve fuzzer id for test case {}",
                    test_handle.get_unique_id()
                )
            });

        let sender_type = facade
            .get_fuzzer_type(sender_id)
            .unwrap_or_else(|| panic!("Failed to get fuzzer type for fuzzer id {}", sender_id));

        let available_fuzzers = facade.get_available_fuzzers();
        let mut winners = HashSet::new();
        for cond_predictions in predictions.values() {
            let mut min = 0.;
            let mut min_fuzz = None;
            for (fuzzer_type, prediction) in cond_predictions {
                if available_fuzzers.contains(fuzzer_type)
                    && *fuzzer_type != sender_type
                    && (min_fuzz.is_none() || *prediction < min)
                {
                    min = *prediction;
                    min_fuzz = Some(fuzzer_type);
                }
            }

            if let Some(f) = min_fuzz {
                winners.insert(*f);
            }
        }

        for fuzzer_type in winners {
            log::debug!("Dispatching to {:?}", fuzzer_type);

            if let Err(e) =
                facade.dispatch_test_cases_to_all(vec![test_handle.clone()], fuzzer_type)
            {
                log::error!("Error while dispatching seed: {}", e);
            }
        }
    }
}
