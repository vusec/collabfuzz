use super::util::SchedulerFacadeRef;
use super::{ScheduleMessage, Scheduler};

pub const SCHEDULER_NAME: &str = "nop";

pub struct NopScheduler;

/// NOP scheduler, that doesn't send seeds to schedulers at all
/// Can be used to run e.g., baseline benchmarks
impl NopScheduler {
    pub fn new(_facade_ref: SchedulerFacadeRef) -> Self {
        Self
    }
}

impl Scheduler for NopScheduler {
    fn schedule(&mut self, _new_seed: ScheduleMessage) {
        // Do not schedule the seed
    }
}
