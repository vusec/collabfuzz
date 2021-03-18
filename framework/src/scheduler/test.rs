use super::util::SchedulerFacadeRef;
use super::{ScheduleMessage, Scheduler};
use crate::analysis::AnalysisType;
use crate::analysis::TestAnalysisState;

pub struct TestScheduler {
    facade_ref: SchedulerFacadeRef,
}

impl TestScheduler {
    pub fn new(facade_ref: SchedulerFacadeRef) -> Self {
        TestScheduler { facade_ref }
    }
}

impl Scheduler for TestScheduler {
    fn schedule(&mut self, schedule_message: ScheduleMessage) {
        match schedule_message {
            ScheduleMessage::Timeout => {
                log::debug!("Do nothing on timeout");
                return;
            }
            ScheduleMessage::DuplicateTestCase(_) => {
                log::debug!("Duplicate test case reported, ignoring");
                return;
            }
            ScheduleMessage::NewTestCase(_) => log::debug!("New seed reported, running"),
        }

        let mut facade = self.facade_ref.get_facade();

        let test_global_state = facade
            .get_analysis_state(AnalysisType::Test)
            .downcast_ref::<TestAnalysisState>()
            .unwrap();

        let observed_test_cases = test_global_state.get_observed();
        let available_fuzzers = facade.get_available_fuzzers();

        for test_handle in observed_test_cases {
            let test_case_handles = vec![test_handle.clone()];
            for fuzzer_type in available_fuzzers.iter() {
                log::debug!("Dispatching to {:?}", fuzzer_type);
                if let Err(e) =
                    facade.dispatch_test_cases_to_all(test_case_handles.clone(), *fuzzer_type)
                {
                    log::error!("Error while dispatching seed: {}", e);
                }
            }
        }
    }
}
