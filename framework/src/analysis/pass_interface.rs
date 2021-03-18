use super::PassType;
use std::error::Error;
use std::fmt;
use std::io;

pub trait Pass: Send {
    fn pass_type(&self) -> PassType;
    fn process(&self, test_case: &[u8]) -> Result<Vec<u8>, PassError>;
}

#[derive(Debug)]
pub enum PassError {
    Generic(String),
    FailedToGetBin(io::Error),
    StdinNotSupported,
    AnalysisFailed,
}

impl fmt::Display for PassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PassError::Generic(e) => write!(f, "Error in pass: {}", e),
            PassError::FailedToGetBin(e) => write!(f, "Error getting instrumented binary: {}", e),
            PassError::StdinNotSupported => write!(f, "Stdin input is not supported for this pass"),
            PassError::AnalysisFailed => write!(f, "Analysis could not be completed"),
        }
    }
}

impl Error for PassError {}

impl From<&str> for PassError {
    fn from(message: &str) -> Self {
        PassError::Generic(String::from(message))
    }
}
