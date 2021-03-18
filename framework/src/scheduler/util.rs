use super::{ScheduleMessage, SchedulerType};
use crate::analysis::{AnalysisType, GlobalState};
use crate::fuzzers::{FuzzerId, FuzzerType, FuzzersHandler};
use crate::protos::{CtrlCommand, FuzzerCtrlMsg, JobMsg, SeedMsg};
use crate::scheduler::Scheduler;
use crate::storage::TestCaseHandle;
use crate::types::{SharedFuzzersHandler, SharedGlobalStates, SharedLogger, SharedStorage};
use priority_queue::PriorityQueue;
use protobuf::{Message, RepeatedField};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::mem;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// This is a RAII structure that should not be kept alive for long periods of time. It needs to be
// released as soon as possible since it holds mutexes used by other components. It is used to
// provide the guarantee that the state of the server is not changed during callbacks to the
// scheduler.
pub struct SchedulerFacade<'a> {
    test_case_push_socket: &'a zmq::Socket,
    global_states_guard: MutexGuard<'a, HashMap<AnalysisType, Box<dyn GlobalState>>>,
    fuzzers_handler_guard: MutexGuard<'a, FuzzersHandler>,
    storage: SharedStorage,
    logger: SharedLogger,
}

impl SchedulerFacade<'_> {
    #[allow(clippy::borrowed_box)]
    pub fn get_analysis_state(&self, analysis_type: AnalysisType) -> &Box<dyn GlobalState> {
        self.global_states_guard
            .get(&analysis_type)
            .expect("Missing dependency")
    }

    pub fn get_available_fuzzers(&self) -> Vec<FuzzerType> {
        self.fuzzers_handler_guard.get_available_types()
    }

    pub fn get_fuzzer_type(&self, fuzzer_id: FuzzerId) -> Option<FuzzerType> {
        self.fuzzers_handler_guard.get_fuzzer_type(fuzzer_id)
    }

    fn seed_msg_from_handle(&self, test_handle: &TestCaseHandle) -> SeedMsg {
        let mut seed_msg = SeedMsg::new();
        seed_msg.set_id(test_handle.get_unique_id().clone());
        seed_msg.set_content(self.storage.lock().unwrap().retrieve(test_handle).content);
        seed_msg
    }

    pub fn dispatch_test_case(
        &mut self,
        test_handle: TestCaseHandle,
        fuzzer_type: FuzzerType,
    ) -> Result<()> {
        log::debug!("Dispatching to first fuzzer with type: {:?}", fuzzer_type);
        let mut job_msg = JobMsg::new();
        let seed_msg = self.seed_msg_from_handle(&test_handle);
        job_msg.set_seeds(RepeatedField::from_vec(vec![seed_msg]));
        let encoded_job_msg = job_msg.write_to_bytes()?;

        let fuzzer_id = self
            .fuzzers_handler_guard
            .schedule_fuzzer_with_type(fuzzer_type);
        log::debug!("Selected fuzzer: {}", fuzzer_id);

        {
            let logger = self.logger.lock().unwrap();
            if let Err(e) = logger.log_test_case_dispatch(fuzzer_id, test_handle) {
                log::error!("Failed to log test case dispatch: {}", e);
            }
        }

        let encoded_fuzzer_id = fuzzer_id.to_string().as_bytes().to_vec();

        // TODO: This S may be turned into a J
        let parts = [encoded_fuzzer_id, b"S".to_vec(), encoded_job_msg];
        self.test_case_push_socket.send_multipart(&parts, 0)?;

        Ok(())
    }

    pub fn dispatch_test_cases_to_all(
        &mut self,
        test_handles: Vec<TestCaseHandle>,
        fuzzer_type: FuzzerType,
    ) -> Result<()> {
        if test_handles.is_empty() {
            log::warn!("Trying to dispatch 0 test cases, ignoring");
            return Ok(());
        }

        log::debug!("Dispatching to all fuzzers with type: {:?}", fuzzer_type);
        let mut job_msg = JobMsg::new();
        let messages = test_handles
            .iter()
            .map(|tch| self.seed_msg_from_handle(tch))
            .collect();
        job_msg.set_seeds(RepeatedField::from_vec(messages));

        for fuzzer_id in self
            .fuzzers_handler_guard
            .schedule_all_fuzzers_with_type(fuzzer_type)
        {
            log::debug!("Selected fuzzer: {}", fuzzer_id);

            let mut job_msg = job_msg.clone();
            job_msg.set_fuzzer_id(fuzzer_id.to_string());
            let encoded_job_msg = job_msg.write_to_bytes()?;

            let encoded_fuzzer_id = fuzzer_id.to_string().as_bytes().to_vec();

            let batch_insert_start = Instant::now();

            let mut logger = self.logger.lock().unwrap();
            if let Err(e) = logger.log_test_case_dispatch_batch(fuzzer_id, &test_handles) {
                log::error!("Failed to log test case dispatch: {}", e);
            }

            let batch_insert_end = Instant::now();
            log::debug!(
                "Batch test case dispatch insert took {:?}",
                batch_insert_end.duration_since(batch_insert_start)
            );

            // TODO: This S may be turned into a J
            let parts = [encoded_fuzzer_id, b"S".to_vec(), encoded_job_msg];
            self.test_case_push_socket.send_multipart(&parts, 0)?;
        }

        Ok(())
    }

    pub fn run_fuzzer(&self, fuzzer_id: FuzzerId) -> Result<()> {
        log::debug!("Sending RUN message to fuzzer: {}", fuzzer_id);
        self.send_ctrl_msg(fuzzer_id, CtrlCommand::COMMAND_RUN, None)
    }

    pub fn pause_fuzzer(&self, fuzzer_id: FuzzerId) -> Result<()> {
        log::debug!("Sending PAUSE message to fuzzer: {}", fuzzer_id);
        self.send_ctrl_msg(fuzzer_id, CtrlCommand::COMMAND_PAUSE, None)
    }

    pub fn kill_fuzzer(&self, fuzzer_id: FuzzerId) -> Result<()> {
        log::debug!("Sending KILL message to fuzzer: {}", fuzzer_id);
        self.send_ctrl_msg(fuzzer_id, CtrlCommand::COMMAND_KILL, None)
    }

    pub fn set_fuzzer_priority(&self, fuzzer_id: FuzzerId, priority: i32) -> Result<()> {
        log::debug!("Sending SET_PRIORITY message to fuzzer: {}", fuzzer_id);
        self.send_ctrl_msg(fuzzer_id, CtrlCommand::COMMAND_SET_PRIORITY, Some(priority))
    }

    pub fn set_fuzzer_type_priority(
        &mut self,
        fuzzer_type: FuzzerType,
        priority: i32,
    ) -> Result<()> {
        log::debug!(
            "Sending SET_PRIORITY message to fuzzers of type: {}",
            fuzzer_type
        );

        for fuzzer_id in self
            .fuzzers_handler_guard
            .schedule_all_fuzzers_with_type(fuzzer_type)
        {
            self.set_fuzzer_priority(fuzzer_id, priority)?;
        }

        Ok(())
    }

    fn send_ctrl_msg(
        &self,
        fuzzer_id: FuzzerId,
        command: CtrlCommand,
        opt_priority: Option<i32>,
    ) -> Result<()> {
        let mut ctrl_msg = FuzzerCtrlMsg::new();
        ctrl_msg.set_command(command);
        ctrl_msg.set_fuzzer_id(fuzzer_id.to_string());

        if let Some(priority) = opt_priority {
            ctrl_msg.set_fuzzer_priority(priority);
        }

        let encoded_fuzzer_id = fuzzer_id.to_string().as_bytes().to_vec();
        let encoded_ctrl_msg = ctrl_msg.write_to_bytes()?;
        let parts = [encoded_fuzzer_id, b"C".to_vec(), encoded_ctrl_msg];
        self.test_case_push_socket.send_multipart(&parts, 0)?;

        Ok(())
    }
}

pub struct SchedulerFacadeRef {
    test_case_push_socket: zmq::Socket,
    global_states: SharedGlobalStates,
    fuzzers_handler: SharedFuzzersHandler,
    storage: SharedStorage,
    logger: SharedLogger,
}

impl SchedulerFacadeRef {
    pub fn new(
        ctx: zmq::Context,
        test_case_push_uri: String,
        global_states: SharedGlobalStates,
        fuzzers_handler: SharedFuzzersHandler,
        storage: SharedStorage,
        logger: SharedLogger,
    ) -> Result<Self> {
        let test_case_push_socket = ctx.socket(zmq::PUB)?;
        test_case_push_socket.bind(&test_case_push_uri)?;

        Ok(Self {
            test_case_push_socket,
            global_states,
            fuzzers_handler,
            storage,
            logger,
        })
    }

    pub fn get_facade(&self) -> SchedulerFacade {
        SchedulerFacade {
            test_case_push_socket: &self.test_case_push_socket,
            global_states_guard: self.global_states.lock().unwrap(),
            fuzzers_handler_guard: self.fuzzers_handler.lock().unwrap(),
            storage: Arc::clone(&self.storage),
            logger: Arc::clone(&self.logger),
        }
    }
}

pub enum SchedulerHandlerControlMessage {
    NewTestCase(TestCaseHandle),
    DuplicateTestCase(TestCaseHandle),
    Die,
}

pub struct SchedulerHandler {
    scheduler: Box<dyn Scheduler>,
    thread_control: mpsc::Receiver<SchedulerHandlerControlMessage>,
    timeout: Duration,
}

impl SchedulerHandler {
    pub fn new(
        scheduler_type: SchedulerType,
        facade_ref: SchedulerFacadeRef,
        thread_control: mpsc::Receiver<SchedulerHandlerControlMessage>,
        timeout: Duration,
    ) -> Self {
        SchedulerHandler {
            scheduler: scheduler_type.get_scheduler(facade_ref),
            thread_control,
            timeout,
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.thread_control.recv_timeout(self.timeout) {
                Ok(ctrl_message) => match ctrl_message {
                    SchedulerHandlerControlMessage::NewTestCase(handle) => {
                        log::debug!("New test case processed, running scheduler");
                        self.scheduler
                            .schedule(ScheduleMessage::NewTestCase(handle));
                    }
                    SchedulerHandlerControlMessage::DuplicateTestCase(handle) => {
                        log::debug!("Duplicate test case received, running scheduler");
                        self.scheduler
                            .schedule(ScheduleMessage::DuplicateTestCase(handle));
                    }
                    SchedulerHandlerControlMessage::Die => {
                        log::info!("Exiting scheduler");
                        break;
                    }
                },
                Err(recv_error) => match recv_error {
                    mpsc::RecvTimeoutError::Timeout => {
                        log::debug!("Timeout expired, running scheduler");
                        self.scheduler.schedule(ScheduleMessage::Timeout);
                    }
                    mpsc::RecvTimeoutError::Disconnected => {
                        log::error!("Scheduler control socket disconnected unexpectedly!");
                        break;
                    }
                },
            }
        }
    }
}

const FLUSH_INTERVAL_VAR_NAME: &str = "COLLAB_FUZZ_TC_FLUSH_INTERVAL";
const FLUSH_PERCENTAGE_VAR_NAME: &str = "COLLAB_FUZZ_TC_FLUSH_PERCENTAGE";

#[derive(Debug, Clone, Copy)]
pub struct QueueSchedulerConfig {
    pub interval: Duration,
    pub percentage: f64,
    pub allow_env_override: bool,
}

const QUEUE_SCHEDULER_DEFAULTS: QueueSchedulerConfig = QueueSchedulerConfig {
    interval: Duration::from_secs(5),
    percentage: 0.01,
    allow_env_override: true,
};

impl QueueSchedulerConfig {
    fn parse_env(&self) -> Self {
        let mut config = *self;
        if !self.allow_env_override {
            return config;
        }

        if let Ok(val) = env::var(FLUSH_INTERVAL_VAR_NAME) {
            config.interval = Duration::from_secs(
                val.parse::<u64>()
                    .unwrap_or_else(|val| panic!("Invalid {}: {}", FLUSH_INTERVAL_VAR_NAME, val)),
            )
        }

        if let Ok(val) = env::var(FLUSH_PERCENTAGE_VAR_NAME) {
            config.percentage = val
                .parse::<f64>()
                .unwrap_or_else(|val| panic!("Invalid {}: {}", FLUSH_PERCENTAGE_VAR_NAME, val))
        }

        config
    }
}

pub struct QueueSchedulerHelper {
    scheduling_thread: Option<JoinHandle<()>>,
    kill_tx: mpsc::Sender<()>,
}

impl QueueSchedulerHelper {
    pub fn new(
        facade_ref: &Arc<Mutex<SchedulerFacadeRef>>,
        scheduling_queue: &Arc<Mutex<PriorityQueue<TestCaseHandle, usize>>>,
        default_config: Option<QueueSchedulerConfig>,
    ) -> Self {
        let config = default_config
            .unwrap_or(QUEUE_SCHEDULER_DEFAULTS)
            .parse_env();

        let (kill_tx, kill_rx) = mpsc::channel();
        let scheduling_thread = {
            let facade_ref = Arc::clone(&facade_ref);
            let scheduling_queue = Arc::clone(&scheduling_queue);

            thread::spawn(move || {
                // An error is returned when the timeout expires
                while kill_rx.recv_timeout(config.interval).is_err() {
                    log::debug!("Timeout expired, flushing");
                    Self::flush_queue(&facade_ref, &scheduling_queue, config.percentage);
                }

                log::debug!("Scheduling thread is dying");
            })
        };

        Self {
            kill_tx,
            scheduling_thread: Some(scheduling_thread),
        }
    }

    fn flush_queue(
        facade_ref: &Arc<Mutex<SchedulerFacadeRef>>,
        scheduling_queue: &Arc<Mutex<PriorityQueue<TestCaseHandle, usize>>>,
        flush_percentage: f64,
    ) {
        let flush_queue_start = Instant::now();

        // Preserve the order in the schedule function to avoid deadlocks
        let facade_ref = facade_ref.lock().unwrap();
        let mut scheduling_queue = scheduling_queue.lock().unwrap();

        let mut facade = facade_ref.get_facade();

        if scheduling_queue.len() == 0 {
            log::debug!("Queue is empty, no need to flush");
            return;
        }

        let flush_count: usize = (scheduling_queue.len() as f64 * flush_percentage).ceil() as usize;
        let mut test_case_handles = Vec::new();
        for _ in 0..flush_count {
            test_case_handles.push(scheduling_queue.pop().unwrap().0);
        }

        for fuzzer_type in facade.get_available_fuzzers() {
            log::debug!("Dispatching to {:?}", fuzzer_type);

            if let Err(e) =
                facade.dispatch_test_cases_to_all(test_case_handles.clone(), fuzzer_type)
            {
                log::error!("Error while dispatching seed: {}", e);
            }
        }

        log::debug!("Queue size is now: {}", scheduling_queue.len());

        let flush_queue_end = Instant::now();
        log::debug!(
            "Flushing the queue took {:?}",
            flush_queue_end.duration_since(flush_queue_start)
        );
    }
}

impl Drop for QueueSchedulerHelper {
    fn drop(&mut self) {
        log::debug!("QueueSchedulerHelper dropped");
        self.kill_tx.send(()).expect("Could not send kill message");

        let scheduling_thread = mem::replace(&mut self.scheduling_thread, None).unwrap();
        scheduling_thread.join().unwrap();
    }
}
