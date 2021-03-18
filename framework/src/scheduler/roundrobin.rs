use super::util::SchedulerFacadeRef;
use super::{ScheduleMessage, Scheduler};

pub const SCHEDULER_NAME: &str = "roundrobin";

pub struct RoundRobinScheduler {
    facade_ref: SchedulerFacadeRef,
    select_fuzzer_index: usize,
}

impl RoundRobinScheduler {
    pub fn new(facade_ref: SchedulerFacadeRef) -> Self {
        Self {
            facade_ref,
            select_fuzzer_index: 0,
        }
    }
}

impl Scheduler for RoundRobinScheduler {
    fn schedule(&mut self, schedule_message: ScheduleMessage) {
        let test_handle = match schedule_message {
            ScheduleMessage::Timeout => {
                log::debug!("Do nothing on timeout");
                return;
            }
            ScheduleMessage::DuplicateTestCase(_) => {
                log::debug!("Duplicate test case reported, ignoring");
                return;
            }
            ScheduleMessage::NewTestCase(test_handle) => {
                log::debug!("New seed reported, running");
                test_handle
            }
        };

        let mut facade = self.facade_ref.get_facade();

        let fuzzers = facade.get_available_fuzzers();
        if fuzzers.is_empty() {
            return;
        }
        if let Some(fuzzer_type) = fuzzers.get(self.select_fuzzer_index) {
            log::debug!("Dispatching to {:?}", fuzzer_type);
            if let Err(e) = facade.dispatch_test_cases_to_all(vec![test_handle], *fuzzer_type) {
                log::error!("Error while dispatching seed: {}", e);
            }
        }
        self.select_fuzzer_index = (self.select_fuzzer_index + 1) % fuzzers.len();
    }
}
