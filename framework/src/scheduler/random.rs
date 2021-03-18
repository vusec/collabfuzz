use super::util::SchedulerFacadeRef;
use super::{ScheduleMessage, Scheduler};
use rand::seq::SliceRandom;

pub const SCHEDULER_NAME: &str = "random";

pub struct RandomScheduler {
    facade_ref: SchedulerFacadeRef,
}

impl RandomScheduler {
    pub fn new(facade_ref: SchedulerFacadeRef) -> Self {
        Self { facade_ref }
    }
}

impl Scheduler for RandomScheduler {
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
        if let Some(fuzzer_type) = fuzzers.choose(&mut rand::thread_rng()) {
            log::debug!("Dispatching to {:?}", fuzzer_type);
            if let Err(e) = facade.dispatch_test_cases_to_all(vec![test_handle], *fuzzer_type) {
                log::error!("Error while dispatching seed: {}", e);
            }
        }
    }
}
