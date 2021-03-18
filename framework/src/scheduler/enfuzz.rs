use super::util::{QueueSchedulerConfig, QueueSchedulerHelper, SchedulerFacadeRef};
use super::{ScheduleMessage, Scheduler};
use crate::analysis::coverage_utils::Edge;
use crate::analysis::AnalysisType;
use crate::analysis::GlobalCoverageState;
use crate::storage::TestCaseHandle;
use priority_queue::PriorityQueue;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const SCHEDULER_NAME: &str = "enfuzz";

pub struct EnFuzzScheduler {
    facade_ref: Arc<Mutex<SchedulerFacadeRef>>,
    prev_coverage: HashSet<Edge>,
    scheduling_queue: Arc<Mutex<PriorityQueue<TestCaseHandle, usize>>>,
    _helper: QueueSchedulerHelper,
}

impl EnFuzzScheduler {
    pub fn new(facade_ref: SchedulerFacadeRef) -> Self {
        let facade_ref = Arc::new(Mutex::new(facade_ref));
        let scheduling_queue = Arc::new(Mutex::new(PriorityQueue::new()));

        let helper_config = QueueSchedulerConfig {
            interval: Duration::from_secs(120),
            percentage: 1.,
            allow_env_override: false,
        };

        let helper = QueueSchedulerHelper::new(&facade_ref, &scheduling_queue, Some(helper_config));

        Self {
            facade_ref,
            prev_coverage: HashSet::new(),
            scheduling_queue,
            _helper: helper,
        }
    }
}

impl Scheduler for EnFuzzScheduler {
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

        // Preserve the order on the scheduling thread to avoid deadlocks
        let facade_ref = self.facade_ref.lock().unwrap();
        let mut scheduling_queue = self.scheduling_queue.lock().unwrap();

        let facade = facade_ref.get_facade();

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

        log::debug!("Queuing test case");
        scheduling_queue.push(test_handle, 0);
    }
}
