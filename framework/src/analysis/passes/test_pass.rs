use super::{Pass, PassConfig, PassError, PassType};

pub const PASS_NAME: &str = "test";

pub struct TestPass;

impl TestPass {
    pub fn new(config: PassConfig) -> Self {
        assert_eq!(
            config.program_arguments,
            vec![String::from("arg1"), String::from("arg2")]
        );

        TestPass
    }
}

impl Pass for TestPass {
    fn pass_type(&self) -> PassType {
        PassType::Test
    }

    fn process(&self, _test_case: &[u8]) -> Result<Vec<u8>, PassError> {
        log::debug!("TestPass: process");

        let report = 42 as u64;
        Ok(report.to_le_bytes().to_vec())
    }
}
