use crate::analysis::{PassType, PassTypeDecodingError};
use crate::protos::{FuzzerCtrlMsg, SeedMsg};
use protobuf::error::ProtobufError;
use protobuf::parse_from_bytes;
use std::error::Error;
use std::fmt;
use std::str;
use std::str::{FromStr, Utf8Error};

type Result<T> = std::result::Result<T, ProtocolError>;

// TODO: When all the deserialization will be moved in this mode, the String can be private
#[derive(Debug)]
pub struct ProtocolError(pub String);

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Protocol error: {}", self.0)
    }
}

impl Error for ProtocolError {}

impl From<ProtobufError> for ProtocolError {
    fn from(err: ProtobufError) -> Self {
        ProtocolError(err.to_string())
    }
}

impl From<Utf8Error> for ProtocolError {
    fn from(err: Utf8Error) -> Self {
        ProtocolError(err.to_string())
    }
}

impl From<PassTypeDecodingError> for ProtocolError {
    fn from(err: PassTypeDecodingError) -> Self {
        ProtocolError(err.to_string())
    }
}

impl SeedMsg {
    pub fn from_multipart(report_parts: Vec<Vec<u8>>) -> Result<Self> {
        if report_parts.len() != 2 {
            let error_message = format!("Wrong number of parts: {}", report_parts.len());

            return Err(ProtocolError(error_message));
        } else if report_parts[0].len() != 1 {
            let error_message = format!("Wrong header length: {}", report_parts[0].len());

            return Err(ProtocolError(error_message));
        } else if report_parts[0] != b"S" {
            let header = &report_parts[0];
            let error_message = format!("Wrong message header: {}", header[0]);

            return Err(ProtocolError(error_message));
        }

        Ok(parse_from_bytes::<SeedMsg>(&report_parts[1])?)
    }
}

pub struct WorkerReport {
    test_serial_id: u64,
    pass_type: PassType,
    content: Vec<u8>,
}

impl WorkerReport {
    pub fn from_multipart(mut report_parts: Vec<Vec<u8>>) -> Result<Self> {
        if report_parts.len() != 3 {
            let error_message = format!("Wrong number of parts: {}", report_parts.len());
            return Err(ProtocolError(error_message));
        }

        if report_parts[0].len() != 8 {
            let error_message = format!("Wrong ID length: {}", report_parts[0].len());
            return Err(ProtocolError(error_message));
        }
        let mut serial_id_bytes = [0; 8];
        serial_id_bytes.copy_from_slice(&report_parts[0]);
        let test_serial_id = u64::from_le_bytes(serial_id_bytes);

        let pass_type = PassType::from_str(str::from_utf8(&report_parts[1])?)?;

        Ok(WorkerReport {
            test_serial_id,
            pass_type,
            content: report_parts.remove(2),
        })
    }

    pub fn get_serial_id(&self) -> u64 {
        self.test_serial_id
    }

    pub fn get_pass_type(&self) -> PassType {
        self.pass_type
    }

    pub fn get_content(&self) -> &Vec<u8> {
        &self.content
    }
}

impl FuzzerCtrlMsg {
    pub fn from_multipart(ctrl_msg_parts: Vec<Vec<u8>>) -> Result<Self> {
        if ctrl_msg_parts.len() != 2 {
            let error_message = format!("Wrong number of parts: {}", ctrl_msg_parts.len());

            return Err(ProtocolError(error_message));
        } else if ctrl_msg_parts[0].len() != 1 {
            let error_message = format!("Wrong header length: {}", ctrl_msg_parts[0].len());

            return Err(ProtocolError(error_message));
        } else if ctrl_msg_parts[0] != b"C" {
            let header = &ctrl_msg_parts[0];
            let error_message = format!("Wrong message header: {}", header[0]);

            return Err(ProtocolError(error_message));
        }

        Ok(parse_from_bytes::<FuzzerCtrlMsg>(&ctrl_msg_parts[1])?)
    }
}
