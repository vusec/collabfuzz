use super::util::SchedulerFacadeRef;
use super::{ScheduleMessage, Scheduler};
use crate::analysis::{AnalysisType, FuzzerIdState};
use crate::fuzzers::FuzzerType;
use std::collections::HashSet;

pub const SCHEDULER_NAME: &str = "selective";

struct SelectiveConfig {
    pub from: HashSet<FuzzerType>,
    pub to: HashSet<FuzzerType>,
}

pub struct SelectiveScheduler {
    facade_ref: SchedulerFacadeRef,
    config: SelectiveConfig,
}

impl SelectiveScheduler {
    pub fn new<'a, I: IntoIterator<Item = &'a FuzzerType>>(
        facade_ref: SchedulerFacadeRef,
        from: I,
        to: I,
    ) -> Self {
        SelectiveScheduler {
            facade_ref,
            config: SelectiveConfig {
                from: from.into_iter().copied().collect(),
                to: to.into_iter().copied().collect(),
            },
        }
    }

    fn should_skip_sender(&self, fuzzer_type: FuzzerType) -> bool {
        !self.config.from.contains(&fuzzer_type)
    }

    fn should_skip_receiver(&self, fuzzer_type: FuzzerType) -> bool {
        !self.config.to.contains(&fuzzer_type)
    }
}

impl Scheduler for SelectiveScheduler {
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

        if self.should_skip_sender(sender_type) {
            log::debug!(
                "Skipping test case from sender {} ({})",
                sender_id,
                sender_type
            );
            return;
        }

        for fuzzer_type in facade.get_available_fuzzers() {
            if self.should_skip_receiver(fuzzer_type) {
                continue;
            }

            log::debug!("Dispatching to {:?}", fuzzer_type);

            if let Err(e) =
                facade.dispatch_test_cases_to_all(vec![test_handle.clone()], fuzzer_type)
            {
                log::error!("Error while dispatching seed: {}", e);
            }
        }
    }
}
