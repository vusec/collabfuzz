mod analysis;
mod control;
mod protocol;
mod worker;

pub use worker::Worker;

use crate::analysis::PassType;
use crate::scheduler::SchedulerHandlerControlMessage;
use crate::types::{SharedFuzzersHandler, SharedGlobalStates, SharedLogger, SharedStorage};
use analysis::AnalysisHandler;
use control::ControlHandler;
use protocol::ProtocolError;
use std::error::Error;
use std::fmt;
use std::str;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Debug)]
pub enum ProcessingError {
    Receive(zmq::Error),
    Decode(ProtocolError),
    Send(zmq::Error),
    Generic(Box<dyn Error>),
}

impl fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessingError::Receive(e) => write!(f, "receive error: {}", e),
            ProcessingError::Decode(e) => write!(f, "decode error: {}", e),
            ProcessingError::Send(e) => write!(f, "send error: {}", e),
            ProcessingError::Generic(e) => write!(f, "generic error: {}", e),
        }
    }
}

impl Error for ProcessingError {}

impl ProcessingError {
    pub fn is_zmq_eterm(&self) -> bool {
        let zmq_error = match self {
            ProcessingError::Receive(zmq_error) => Some(zmq_error),
            ProcessingError::Send(zmq_error) => Some(zmq_error),
            _ => None,
        };

        if let Some(zmq_error) = zmq_error {
            zmq_error == &zmq::Error::ETERM
        } else {
            false
        }
    }
}

pub struct Reactor {
    analysis_handler: AnalysisHandler,
    control_handler: ControlHandler,
}

pub struct ReactorSharedObjects {
    pub storage: SharedStorage,
    pub global_states: SharedGlobalStates,
    pub fuzzers_handler: SharedFuzzersHandler,
    pub logger: SharedLogger,
}

impl Reactor {
    pub fn new(
        ctx: zmq::Context,
        fuzzers_report_uri: &str,
        control_uri: &str,
        shared_objs: ReactorSharedObjects,
        scheduler_channel_tx: Sender<SchedulerHandlerControlMessage>,
    ) -> Self {
        let fuzzers_report_socket = ctx.socket(zmq::REP).unwrap();
        fuzzers_report_socket.bind(fuzzers_report_uri).unwrap();

        let workers_report_socket = ctx.socket(zmq::PULL).unwrap();
        workers_report_socket
            .bind("inproc://workers-report")
            .unwrap();

        let control_rep = ctx.socket(zmq::REP).unwrap();
        control_rep.bind(control_uri).unwrap();

        let analysis_handler = AnalysisHandler::new(
            zmq::Context::clone(&ctx),
            fuzzers_report_socket,
            workers_report_socket,
            shared_objs.storage,
            shared_objs.global_states,
            scheduler_channel_tx,
            Arc::clone(&shared_objs.logger),
        );

        let control_handler =
            ControlHandler::new(control_rep, shared_objs.fuzzers_handler, shared_objs.logger);

        Self {
            analysis_handler,
            control_handler,
        }
    }

    pub fn register_pass_type(
        &mut self,
        pass_type: PassType,
        run_on_duplicates: bool,
    ) -> Result<(), ProcessingError> {
        // Registration is kept separate so that it can be made dynamic more easily in the future
        self.analysis_handler
            .handle_registration_request(pass_type, run_on_duplicates)
    }

    pub fn listen(&mut self) -> Result<(), zmq::Error> {
        // TODO: Turn this into a map from poll item to callback
        let mut items = self.analysis_handler.get_poll_items();
        items.append(&mut self.control_handler.get_poll_items());

        log::info!("Reactor is waiting for reports");

        loop {
            if let Err(e) = zmq::poll(&mut items, -1) {
                if e == zmq::Error::ETERM {
                    log::info!("Received ETERM");
                } else {
                    log::error!("ZMQ poll error: {}", e);
                }

                log::info!("Killing reactor");
                break;
            }

            if items[0].is_readable() {
                log::debug!("New message from fuzzers");
                self.analysis_handler.process_fuzzer_report();
            }

            if items[1].is_readable() {
                log::debug!("New message from workers");
                self.analysis_handler.process_worker_report();
            }

            if items[2].is_readable() {
                log::debug!("New control request");
                self.control_handler.process_ctrl_msg();
            }
        }

        Ok(())
    }
}
