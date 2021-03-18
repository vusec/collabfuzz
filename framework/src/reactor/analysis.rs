use super::protocol::{ProtocolError, WorkerReport};
use super::ProcessingError;
use crate::analysis::{AnalysisUpdate, PassType};
use crate::fuzzers::FuzzerId;
use crate::protos::{SeedMsg, TestCaseReportReply};
use crate::scheduler::SchedulerHandlerControlMessage;
use crate::storage::{StoreResult, TestCase, TestCaseHandle};
use crate::types::{SeedType, SharedGlobalStates, SharedLogger, SharedStorage};
use protobuf::Message;
use std::cell::{Ref, RefCell};
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;
use zmq::PollItem;

#[derive(Debug)]
enum TestCaseQueueEntry {
    New(Rc<RefCell<AnalysisUpdate>>),
    Duplicate(Rc<RefCell<AnalysisUpdate>>),
}

impl TestCaseQueueEntry {
    pub fn get_update(&self) -> Ref<AnalysisUpdate> {
        match self {
            TestCaseQueueEntry::New(update) => update.as_ref().borrow(),
            TestCaseQueueEntry::Duplicate(update) => update.as_ref().borrow(),
        }
    }
}

pub struct AnalysisHandler {
    ctx: zmq::Context,

    fuzzers_report_socket: zmq::Socket,
    workers_report_socket: zmq::Socket,

    pass_type_to_socket: HashMap<PassType, zmq::Socket>,
    passes_for_duplicates: Vec<PassType>,

    // Interior mutability is needed to allow changing the internal state while holding PollItem-s
    test_case_queue: RefCell<VecDeque<TestCaseQueueEntry>>,
    serial_to_results: RefCell<HashMap<u64, Rc<RefCell<AnalysisUpdate>>>>,
    test_circular_counter: RefCell<u64>,
    states_updater: StateUpdater,

    storage: SharedStorage,
    logger: SharedLogger,
}

impl AnalysisHandler {
    pub fn new(
        ctx: zmq::Context,
        fuzzers_report_socket: zmq::Socket,
        workers_report_socket: zmq::Socket,
        storage: SharedStorage,
        global_states: SharedGlobalStates,
        scheduler_channel_tx: Sender<SchedulerHandlerControlMessage>,
        logger: SharedLogger,
    ) -> Self {
        let states_updater = StateUpdater::new(global_states, scheduler_channel_tx);

        Self {
            ctx,

            fuzzers_report_socket,
            workers_report_socket,

            pass_type_to_socket: HashMap::new(),
            passes_for_duplicates: Vec::new(),

            test_case_queue: RefCell::new(VecDeque::new()),
            serial_to_results: RefCell::new(HashMap::new()),
            test_circular_counter: RefCell::new(0),
            states_updater,

            storage,
            logger,
        }
    }

    pub fn get_poll_items(&self) -> Vec<PollItem> {
        vec![
            self.fuzzers_report_socket.as_poll_item(zmq::POLLIN),
            self.workers_report_socket.as_poll_item(zmq::POLLIN),
        ]
    }

    pub fn process_fuzzer_report(&self) {
        if let Err(e) = self.process_fuzzer_report_err() {
            if e.is_zmq_eterm() {
                log::info!("Received ETERM");
            } else {
                log::error!("Error while processing fuzzer report: {}", e);

                log::debug!("Sending error reply");
                let mut reply = TestCaseReportReply::new();
                reply.set_error(e.to_string());
                if let Err(e) = self.send_test_case_report_reply(&reply) {
                    log::error!("Error while sending error reply: {}", e);
                }
            }
        }
    }

    fn process_fuzzer_report_err(&self) -> Result<(), ProcessingError> {
        let report_parts = self
            .fuzzers_report_socket
            .recv_multipart(0)
            .map_err(ProcessingError::Receive)?;
        let fuzzer_report =
            SeedMsg::from_multipart(report_parts).map_err(ProcessingError::Decode)?;

        log::info!(
            "Received test case '{}' from fuzzer '{}' with type '{:?}'",
            fuzzer_report.get_id(),
            fuzzer_report.get_fuzzer_id(),
            fuzzer_report.get_field_type(),
        );

        self.handle_fuzzer_report(fuzzer_report)
    }

    pub fn process_worker_report(&self) {
        if let Err(e) = self.process_worker_report_err() {
            if e.is_zmq_eterm() {
                log::info!("Received ETERM");
            } else {
                log::error!("Error while processing worker report: {}", e);
            }
        }
    }

    fn process_worker_report_err(&self) -> Result<(), ProcessingError> {
        let report_parts = self
            .workers_report_socket
            .recv_multipart(0)
            .map_err(ProcessingError::Receive)?;
        let worker_report =
            WorkerReport::from_multipart(report_parts).map_err(ProcessingError::Decode)?;

        log::debug!(
            "Received report '{}' for test case '{}'",
            worker_report.get_pass_type(),
            worker_report.get_serial_id()
        );

        self.handle_worker_report(worker_report)
    }

    pub fn handle_registration_request(
        &mut self,
        pass_type: PassType,
        run_on_duplicates: bool,
    ) -> Result<(), ProcessingError> {
        log::info!("Registering pass type: {}", pass_type);

        let pass_socket = self.ctx.socket(zmq::PUSH).map_err(ProcessingError::Send)?;
        pass_socket
            .bind(&format!("inproc://{}-distribute", pass_type))
            .map_err(ProcessingError::Send)?;

        self.pass_type_to_socket.insert(pass_type, pass_socket);

        if run_on_duplicates {
            self.passes_for_duplicates.push(pass_type);
        }

        Ok(())
    }

    fn handle_fuzzer_report(&self, fuzzer_report: SeedMsg) -> Result<(), ProcessingError> {
        let function_start = Instant::now();

        let test_case_type = SeedType::from(fuzzer_report.get_field_type());
        let test_case = TestCase {
            case_type: test_case_type,
            content: fuzzer_report.get_content().to_vec(),
        };

        let fuzzer_id: FuzzerId = match fuzzer_report.get_fuzzer_id().parse() {
            Ok(fuzzer_id) => fuzzer_id,
            Err(_) => {
                return Err(ProcessingError::Decode(ProtocolError(format!(
                    "Could not decode fuzzer ID: {}",
                    fuzzer_report.get_fuzzer_id()
                ))));
            }
        };

        let (test_case_handle, store_result) = self.storage.lock().unwrap().store(test_case);

        {
            let logger = self.logger.lock().unwrap();
            if let Err(e) = logger.log_test_case(test_case_handle.clone(), fuzzer_id) {
                log::error!("Failed to log new test case: {}", e);
            }
        }

        if test_case_type == SeedType::NORMAL {
            match store_result {
                StoreResult::New => {
                    self.handle_queue_report(test_case_handle.clone(), fuzzer_id, fuzzer_report)?
                }
                StoreResult::AlreadyExists => self.handle_duplicate_report(
                    test_case_handle.clone(),
                    fuzzer_id,
                    fuzzer_report,
                )?,
            }
        }

        let function_end = Instant::now();
        log::debug!(
            "Handling fuzzer report took {:?}",
            function_end.duration_since(function_start)
        );

        // The positive TestCaseReportReply should be sent after any possible ProcessingError is
        // returned. If a ProcessingError is returned, process_fuzzer_report will try to send an
        // error reply and thus the positive one should not be sent.
        let mut report_reply = TestCaseReportReply::new();
        report_reply.set_id(test_case_handle.get_unique_id().to_string());
        self.send_test_case_report_reply(&report_reply)
    }

    fn handle_queue_report(
        &self,
        handle: TestCaseHandle,
        fuzzer_id: FuzzerId,
        fuzzer_report: SeedMsg,
    ) -> Result<(), ProcessingError> {
        log::debug!("Handling as new test case report");

        let test_serial_id = self.get_new_serial_id();

        let test_case_results = AnalysisUpdate::new(
            handle,
            fuzzer_id,
            self.decode_parent_handles(&fuzzer_report)?,
        );

        log::debug!(
            "Requesting analysis for test case with ID: {}",
            test_serial_id
        );
        let mut analysis_count = 0;
        for pass_socket in self.pass_type_to_socket.values() {
            let parts = [&test_serial_id.to_le_bytes(), fuzzer_report.get_content()];
            pass_socket
                .send_multipart(parts.iter(), 0)
                .map_err(ProcessingError::Send)?;
            analysis_count += 1;
        }

        let test_case_results_ref = Rc::new(RefCell::new(test_case_results));
        self.test_case_queue
            .borrow_mut()
            .push_back(TestCaseQueueEntry::New(Rc::clone(&test_case_results_ref)));

        log::debug!("Requested {} analyses", analysis_count);
        if analysis_count >= 1 {
            self.serial_to_results
                .borrow_mut()
                .insert(test_serial_id, test_case_results_ref);
        } else {
            // No analysis was requested, so drop Rc and try flushing the queue immediately
            mem::drop(test_case_results_ref);
            self.flush_test_case_queue();
        }

        Ok(())
    }

    fn handle_duplicate_report(
        &self,
        handle: TestCaseHandle,
        fuzzer_id: FuzzerId,
        fuzzer_report: SeedMsg,
    ) -> Result<(), ProcessingError> {
        log::debug!("Handling as duplicate report");

        let test_serial_id = self.get_new_serial_id();

        let mut test_case_results = AnalysisUpdate::new(
            handle,
            fuzzer_id,
            self.decode_parent_handles(&fuzzer_report)?,
        );

        log::debug!(
            "Requesting analysis for test case with ID: {}",
            test_serial_id
        );
        let mut analysis_count = 0;
        for (pass_type, pass_socket) in &self.pass_type_to_socket {
            if self.passes_for_duplicates.contains(&pass_type) {
                let parts = [&test_serial_id.to_le_bytes(), fuzzer_report.get_content()];
                pass_socket
                    .send_multipart(parts.iter(), 0)
                    .map_err(ProcessingError::Send)?;
                analysis_count += 1;
            } else {
                test_case_results.skip_pass(*pass_type);
            }
        }

        let test_case_results_ref = Rc::new(RefCell::new(test_case_results));
        self.test_case_queue
            .borrow_mut()
            .push_back(TestCaseQueueEntry::Duplicate(Rc::clone(
                &test_case_results_ref,
            )));

        log::debug!("Requested {} analyses", analysis_count);
        if analysis_count >= 1 {
            self.serial_to_results
                .borrow_mut()
                .insert(test_serial_id, test_case_results_ref);
        } else {
            // No analysis was requested, so drop Rc and try flushing the queue immediately
            mem::drop(test_case_results_ref);
            self.flush_test_case_queue();
        }

        Ok(())
    }

    fn decode_parent_handles(
        &self,
        fuzzer_report: &SeedMsg,
    ) -> Result<Vec<TestCaseHandle>, ProcessingError> {
        let mut parent_handles = Vec::new();
        let storage = self.storage.lock().unwrap();
        for parent_id in fuzzer_report.get_parent_ids() {
            let parent_handle = storage.handle_from_id(parent_id).ok_or_else(|| {
                ProcessingError::Decode(ProtocolError(String::from(
                    "Invalid parent handle received",
                )))
            })?;
            parent_handles.push(parent_handle);
        }
        Ok(parent_handles)
    }

    fn get_new_serial_id(&self) -> u64 {
        let mut circular_counter = self.test_circular_counter.borrow_mut();
        let test_serial_id = *circular_counter;
        *circular_counter += 1;

        test_serial_id
    }

    fn handle_worker_report(&self, worker_report: WorkerReport) -> Result<(), ProcessingError> {
        log::debug!(
            "Adding '{}' report for test case '{}'",
            worker_report.get_serial_id(),
            worker_report.get_pass_type()
        );

        let mut serial_to_results = self.serial_to_results.borrow_mut();

        let test_case_results = serial_to_results
            .get_mut(&worker_report.get_serial_id())
            .expect("Unknown test case ID");
        test_case_results.borrow_mut().add_pass_result(
            worker_report.get_pass_type(),
            worker_report.get_content().clone(),
        );

        let pass_types: Vec<_> = self.pass_type_to_socket.keys().collect();
        if !test_case_results.borrow().has_pass_results(&pass_types) {
            return Ok(());
        }

        log::debug!(
            "Analysis for test case '{}' completed",
            worker_report.get_serial_id()
        );

        // Remove test case from analysis map, now the only reference is in the queue
        serial_to_results
            .remove(&worker_report.get_serial_id())
            .unwrap();

        // Flush queue if possible
        self.flush_test_case_queue();

        Ok(())
    }

    fn flush_test_case_queue(&self) {
        let pass_types: Vec<_> = self.pass_type_to_socket.keys().collect();

        loop {
            {
                let test_case_queue = self.test_case_queue.borrow();
                let front = if let Some(front) = test_case_queue.front() {
                    front
                } else {
                    // No test cases in queue
                    break;
                };

                let update = front.get_update();
                if !update.has_pass_results(&pass_types) {
                    // Head analysis not yet completed
                    break;
                }
            }

            let queue_entry = self.test_case_queue.borrow_mut().pop_front().unwrap();
            log::debug!(
                "Forwarding fully analyzed test cases: {}",
                queue_entry.get_update().get_fuzzer_id()
            );
            self.states_updater.enqueue_update(queue_entry);
        }
    }

    fn send_test_case_report_reply(
        &self,
        report_reply: &TestCaseReportReply,
    ) -> Result<(), ProcessingError> {
        let encoded_reply = report_reply
            .write_to_bytes()
            .expect("Could not encode reply");
        self.fuzzers_report_socket
            .send(encoded_reply, 0)
            .map_err(ProcessingError::Send)
    }
}

enum StateUpdateMessage {
    New(AnalysisUpdate),
    Duplicate(AnalysisUpdate),
    Die,
}

impl From<TestCaseQueueEntry> for StateUpdateMessage {
    fn from(queue_entry: TestCaseQueueEntry) -> Self {
        match queue_entry {
            TestCaseQueueEntry::New(update_rc) => {
                let update_ref =
                    Rc::try_unwrap(update_rc).expect("Entry has more than one reference");
                Self::New(update_ref.into_inner())
            }
            TestCaseQueueEntry::Duplicate(update_rc) => {
                let update_ref =
                    Rc::try_unwrap(update_rc).expect("Entry has more than one reference");
                Self::Duplicate(update_ref.into_inner())
            }
        }
    }
}

struct StatesUpdaterThread {
    update_rx: mpsc::Receiver<StateUpdateMessage>,

    global_states: SharedGlobalStates,
    scheduler_channel_tx: Sender<SchedulerHandlerControlMessage>,
}

impl StatesUpdaterThread {
    pub fn new(
        update_rx: mpsc::Receiver<StateUpdateMessage>,
        global_states: SharedGlobalStates,
        scheduler_channel_tx: Sender<SchedulerHandlerControlMessage>,
    ) -> Self {
        Self {
            update_rx,

            global_states,
            scheduler_channel_tx,
        }
    }

    pub fn run(&self) {
        loop {
            let message = self.update_rx.recv().expect("Sender disconnected");

            match message {
                StateUpdateMessage::Die => break,
                StateUpdateMessage::New(_) => {
                    self.update_global_states(message);
                }
                StateUpdateMessage::Duplicate(_) => {
                    self.update_global_states(message);
                }
            }
        }

        log::debug!("Global states updater thread terminating");
    }

    fn update_global_states(&self, update_message: StateUpdateMessage) {
        let global_state_update_start = Instant::now();

        // Lock here to guarantee that global states are always consistent with each other
        let mut global_states = self.global_states.lock().unwrap();
        match update_message {
            StateUpdateMessage::New(update) => {
                for global_state in global_states.values_mut() {
                    let begin_update = Instant::now();

                    global_state.update(&update);

                    let end_update = Instant::now();
                    log::debug!(
                        "Updating {} global state took: {:?}",
                        global_state.analysis_type(),
                        end_update.duration_since(begin_update)
                    );
                }

                self.scheduler_channel_tx
                    .send(SchedulerHandlerControlMessage::NewTestCase(
                        update.get_test_handle().clone(),
                    ))
                    .expect("Receiver is not listening");
            }
            StateUpdateMessage::Duplicate(update) => {
                for global_state in global_states.values_mut() {
                    if global_state.analysis_type().needs_duplicates() {
                        global_state.update(&update);
                    }
                }

                self.scheduler_channel_tx
                    .send(SchedulerHandlerControlMessage::DuplicateTestCase(
                        update.get_test_handle().clone(),
                    ))
                    .expect("Receiver is not listening");
            }
            _ => panic!("update_global_states called with invalid message"),
        }

        let global_state_update_end = Instant::now();
        log::info!(
            "Global states update took {:?}",
            global_state_update_end.duration_since(global_state_update_start)
        );
    }
}

struct StateUpdater {
    updater_thread: Option<thread::JoinHandle<()>>,
    update_tx: mpsc::Sender<StateUpdateMessage>,
}

impl StateUpdater {
    pub fn new(
        global_states: SharedGlobalStates,
        scheduler_channel_tx: Sender<SchedulerHandlerControlMessage>,
    ) -> Self {
        let (update_tx, update_rx) = mpsc::channel();

        let updater_thread = thread::spawn(move || {
            let updater_thread =
                StatesUpdaterThread::new(update_rx, global_states, scheduler_channel_tx);
            updater_thread.run();
        });

        Self {
            updater_thread: Some(updater_thread),
            update_tx,
        }
    }

    pub fn enqueue_update(&self, test_case_update: TestCaseQueueEntry) {
        self.update_tx
            .send(test_case_update.into())
            .expect("Updater thread is not listening");
    }
}

impl Drop for StateUpdater {
    fn drop(&mut self) {
        log::debug!("Global state updater dropped");
        self.update_tx
            .send(StateUpdateMessage::Die)
            .expect("Could not send kill message");

        let update_thread = mem::take(&mut self.updater_thread).unwrap();
        update_thread.join().expect("Updater thread panicked");
    }
}
