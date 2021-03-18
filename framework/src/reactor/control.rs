use super::protocol::ProtocolError;
use super::ProcessingError;
use crate::fuzzers::FuzzerType;
use crate::logger::FuzzerEvent;
use crate::protos::{CtrlCommand, FuzzerCtrlMsg};
use crate::types::{SharedFuzzersHandler, SharedLogger};
use protobuf::Message;
use zmq::PollItem;

pub struct ControlHandler {
    control_rep: zmq::Socket,
    fuzzers_handler: SharedFuzzersHandler,
    logger: SharedLogger,
}

impl ControlHandler {
    pub fn new(
        control_rep: zmq::Socket,
        fuzzers_handler: SharedFuzzersHandler,
        logger: SharedLogger,
    ) -> Self {
        Self {
            control_rep,
            fuzzers_handler,
            logger,
        }
    }

    pub fn get_poll_items(&self) -> Vec<PollItem> {
        vec![self.control_rep.as_poll_item(zmq::POLLIN)]
    }

    pub fn process_ctrl_msg(&self) {
        let result = self.process_ctrl_msg_err();

        if let Err(e) = result {
            if e.is_zmq_eterm() {
                log::info!("Received ETERM");
            } else {
                log::error!("Error while processing control message: {}", e);

                log::debug!("Sending error response");
                let mut res = FuzzerCtrlMsg::new();
                res.set_command(CtrlCommand::COMMAND_ERR);
                if let Err(e) = self.send_ctrl_reply(&res) {
                    log::error!("Error while sending error message: {}", e);
                }
            }
        }
    }

    fn process_ctrl_msg_err(&self) -> Result<(), ProcessingError> {
        let msg_parts = self
            .control_rep
            .recv_multipart(0)
            .map_err(ProcessingError::Receive)?;
        let ctrl_msg = FuzzerCtrlMsg::from_multipart(msg_parts).map_err(ProcessingError::Decode)?;

        log::debug!("Received control message: {:?}", ctrl_msg);

        match ctrl_msg.get_command() {
            CtrlCommand::COMMAND_REGISTER => self.handle_ctrl_msg_register(&ctrl_msg),
            CtrlCommand::COMMAND_DEREGISTER => self.handle_ctrl_msg_deregister(&ctrl_msg),
            CtrlCommand::COMMAND_READY => self.handle_ctrl_msg_ready(&ctrl_msg),
            _ => unimplemented!(),
        }
    }

    fn handle_ctrl_msg_ready(&self, msg: &FuzzerCtrlMsg) -> Result<(), ProcessingError> {
        log::debug!("Got READY control message");

        let fuzzer_id = msg.get_fuzzer_id().parse().map_err(|_| {
            ProcessingError::Decode(ProtocolError(String::from(
                // TODO: Do not deserialize here
                "Cannot decode fuzzer identifier!",
            )))
        })?;

        log::debug!("Fuzzer {} is READY!", fuzzer_id);

        if let Err(e) = self
            .logger
            .lock()
            .unwrap()
            .log_fuzzer_event(fuzzer_id, FuzzerEvent::Ready)
        {
            log::error!("Failed to log fuzzer event: {}", e);
        }

        let mut fuzzers_handler = self.fuzzers_handler.lock().unwrap();
        fuzzers_handler
            .mark_as_ready(fuzzer_id)
            .map_err(ProcessingError::Generic)?;

        let mut res = FuzzerCtrlMsg::new();
        res.set_command(CtrlCommand::COMMAND_ACK);

        log::debug!("Sending ack response");
        self.send_ctrl_reply(&res)
    }

    fn handle_ctrl_msg_register(&self, ctrl_msg: &FuzzerCtrlMsg) -> Result<(), ProcessingError> {
        let mut fuzzers_handler = self.fuzzers_handler.lock().unwrap();

        let fuzzer_type = FuzzerType::from(ctrl_msg.get_fuzzer_type());
        let fuzzer_id = fuzzers_handler.register_fuzzer(fuzzer_type);
        log::info!("Registered fuzzer {}!", fuzzer_id);

        if let Err(e) = self
            .logger
            .lock()
            .unwrap()
            .log_fuzzer_event(fuzzer_id, FuzzerEvent::Registration(fuzzer_type))
        {
            log::error!("Failed to log fuzzer event: {}", e);
        }

        let mut res = FuzzerCtrlMsg::new();
        res.set_fuzzer_id(fuzzer_id.to_string());
        res.set_command(CtrlCommand::COMMAND_REGISTER);
        self.send_ctrl_reply(&res)
    }

    fn handle_ctrl_msg_deregister(&self, msg: &FuzzerCtrlMsg) -> Result<(), ProcessingError> {
        let mut fuzzers_handler = self.fuzzers_handler.lock().unwrap();

        let id = msg.get_fuzzer_id().parse().map_err(|_| {
            ProcessingError::Decode(ProtocolError(String::from(
                // TODO: Do not deserialize here
                "Cannot decode fuzzer identifier!",
            )))
        })?;

        if let Err(e) = self
            .logger
            .lock()
            .unwrap()
            .log_fuzzer_event(id, FuzzerEvent::Deregistration)
        {
            log::error!("Failed to log fuzzer event: {}", e);
        }

        let reply_command = match fuzzers_handler.deregister_fuzzer(id) {
            Some(_) => {
                log::info!("Deregistered fuzzer {}", id);
                CtrlCommand::COMMAND_ACK
            }
            None => {
                log::error!("Unrecognized fuzzer {}, could not deregister!", id);
                CtrlCommand::COMMAND_ERR
            }
        };

        let mut res = FuzzerCtrlMsg::new();
        res.set_command(reply_command);
        self.send_ctrl_reply(&res)
    }

    fn send_ctrl_reply(&self, ctrl_msg: &FuzzerCtrlMsg) -> Result<(), ProcessingError> {
        let encoded_ctrl_msg = ctrl_msg.write_to_bytes().expect("Could not encode message");
        let parts = [b"C".to_vec(), encoded_ctrl_msg];
        self.control_rep
            .send_multipart(&parts, 0)
            .map_err(ProcessingError::Send)
    }
}
