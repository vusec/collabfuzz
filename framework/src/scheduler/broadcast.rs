use super::util::SchedulerFacadeRef;
use super::{ScheduleMessage, Scheduler};
use crate::analysis::coverage_utils::Edge;
use crate::analysis::{AnalysisType, GlobalCoverageState};
use std::collections::HashSet;

pub const SCHEDULER_NAME: &str = "broadcast";

pub struct BroadcastScheduler {
    facade_ref: SchedulerFacadeRef,
    prev_coverage: HashSet<Edge>,
}

impl BroadcastScheduler {
    pub fn new(facade_ref: SchedulerFacadeRef) -> Self {
        Self {
            facade_ref,
            prev_coverage: HashSet::new(),
        }
    }
}

impl Scheduler for BroadcastScheduler {
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

        let edge_tracer_global_state = facade
            .get_analysis_state(AnalysisType::GlobalCoverage)
            .downcast_ref::<GlobalCoverageState>()
            .unwrap();

        let new_coverage = edge_tracer_global_state.get_global_coverage();
        let coverage_increment = new_coverage.difference(&self.prev_coverage);
        if coverage_increment.count() == 0 {
            log::debug!("New test case did not produce new coverage, ignoring");
            return;
        }
        self.prev_coverage = new_coverage.clone();

        for fuzzer_type in facade.get_available_fuzzers() {
            log::debug!("Dispatching to {:?}", fuzzer_type);

            if let Err(e) =
                facade.dispatch_test_cases_to_all(vec![test_handle.clone()], fuzzer_type)
            {
                log::error!("Error while dispatching seed: {}", e);
            }
        }
    }
}
