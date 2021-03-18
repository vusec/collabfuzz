use super::util::{QueueSchedulerHelper, SchedulerFacadeRef};
use super::{ScheduleMessage, Scheduler};
use crate::analysis::AnalysisType;
use crate::analysis::TestCaseBenefitGlobalState;
use crate::storage::TestCaseHandle;
use priority_queue::PriorityQueue;
use std::sync::{Arc, Mutex};

pub const SCHEDULER_NAME: &str = "test_case_benefit";

pub struct TestCaseBenefitScheduler {
    facade_ref: Arc<Mutex<SchedulerFacadeRef>>,
    scheduling_queue: Arc<Mutex<PriorityQueue<TestCaseHandle, usize>>>,
    _helper: QueueSchedulerHelper,
}

impl TestCaseBenefitScheduler {
    pub fn new(facade_ref: SchedulerFacadeRef) -> Self {
        let facade_ref = Arc::new(Mutex::new(facade_ref));
        let scheduling_queue = Arc::new(Mutex::new(PriorityQueue::new()));

        let helper = QueueSchedulerHelper::new(&facade_ref, &scheduling_queue, None);

        Self {
            facade_ref,
            scheduling_queue,
            _helper: helper,
        }
    }
}

impl Scheduler for TestCaseBenefitScheduler {
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
        let test_case_benefit_global_state = facade
            .get_analysis_state(AnalysisType::TestCaseBenefit)
            .downcast_ref::<TestCaseBenefitGlobalState>()
            .unwrap();

        log::debug!("Update priority for queued test cases");
        let mut queue_handles = Vec::new();
        for (queue_handle, _priority) in scheduling_queue.iter() {
            queue_handles.push(queue_handle.clone());
        }
        for queue_handle in queue_handles {
            let new_benefit =
                test_case_benefit_global_state.count_reachable_unseen_nodes(&queue_handle);
            scheduling_queue.change_priority(&queue_handle, new_benefit);
        }

        log::debug!("Queuing new test case");
        let test_case_benefit =
            test_case_benefit_global_state.count_reachable_unseen_nodes(&test_handle);
        scheduling_queue.push(test_handle, test_case_benefit);
    }
}
