use super::utils;
use super::{Pass, PassConfig, PassError};

// Add a new import for each new pass
mod test_pass;
use test_pass::TestPass;

mod generic_instr_pass;
use generic_instr_pass::GenericInstrPass;
const EDGE_TRACER_PASS_NAME: &str = "edge_tracer";
const COND_TRACER_PASS_NAME: &str = "cond_tracer";

mod bb_taint_tracer_pass;
use bb_taint_tracer_pass::BBTaintTracerPass;

mod instruction_counter_pass;
use instruction_counter_pass::InstructionCounterPass;

mod bytes_tracer_pass;
use bytes_tracer_pass::BytesTracerPass;

use std::error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct PassTypeDecodingError(String);

impl fmt::Display for PassTypeDecodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown pass type name: {}", self)
    }
}

impl error::Error for PassTypeDecodingError {}

// Add a case to this enum for the new pass
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum PassType {
    Test,
    EdgeTracer,
    CondTracer,
    BBTaintTracer,
    InstructionCounter,
    BytesTracer,
}

impl FromStr for PassType {
    type Err = PassTypeDecodingError;

    fn from_str(type_name: &str) -> Result<Self, Self::Err> {
        // Add a case to this match using a unique string
        match type_name {
            test_pass::PASS_NAME => Ok(PassType::Test),
            EDGE_TRACER_PASS_NAME => Ok(PassType::EdgeTracer),
            COND_TRACER_PASS_NAME => Ok(PassType::CondTracer),
            bb_taint_tracer_pass::PASS_NAME => Ok(PassType::BBTaintTracer),
            instruction_counter_pass::PASS_NAME => Ok(PassType::InstructionCounter),
            bytes_tracer_pass::PASS_NAME => Ok(PassType::BytesTracer),
            _ => Err(PassTypeDecodingError(String::from(type_name))),
        }
    }
}

impl fmt::Display for PassType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Add a case to this match using the same unique string
        let type_name = match self {
            PassType::Test => test_pass::PASS_NAME,
            PassType::EdgeTracer => EDGE_TRACER_PASS_NAME,
            PassType::CondTracer => COND_TRACER_PASS_NAME,
            PassType::BBTaintTracer => bb_taint_tracer_pass::PASS_NAME,
            PassType::InstructionCounter => instruction_counter_pass::PASS_NAME,
            PassType::BytesTracer => bytes_tracer_pass::PASS_NAME,
        };
        assert_eq!(
            self,
            &PassType::from_str(type_name).unwrap(),
            "PassType has wrong name"
        );

        write!(f, "{}", type_name)
    }
}

impl PassType {
    pub fn get_pass(self, config: PassConfig) -> Result<Box<dyn Pass>, PassError> {
        // Add a case to this match to allocate the new pass
        let pass: Box<dyn Pass> = match self {
            PassType::Test => Box::new(TestPass::new(config)),
            PassType::EdgeTracer => Box::new(GenericInstrPass::new(config, PassType::EdgeTracer)?),
            PassType::CondTracer => Box::new(GenericInstrPass::new(config, PassType::CondTracer)?),
            PassType::BBTaintTracer => Box::new(BBTaintTracerPass::new(config)?),
            PassType::InstructionCounter => Box::new(InstructionCounterPass::new(config)?),
            PassType::BytesTracer => Box::new(BytesTracerPass::new(config)?),
        };
        assert_eq!(self, pass.pass_type(), "Pass has wrong type");

        Ok(pass)
    }
}
