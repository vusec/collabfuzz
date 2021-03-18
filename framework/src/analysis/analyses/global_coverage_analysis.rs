use super::coverage_utils::{Edge, EdgeRecord};
use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType, SharedLogger};
use std::collections::HashSet;

pub struct GlobalCoverageState {
    edges: HashSet<Edge>,
    logger: SharedLogger,
}

impl GlobalCoverageState {
    pub fn new(_config: &PassConfig, logger: SharedLogger) -> Self {
        GlobalCoverageState {
            edges: HashSet::new(),
            logger,
        }
    }

    pub fn get_global_coverage(&self) -> &HashSet<Edge> {
        &self.edges
    }
}

impl GlobalState for GlobalCoverageState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::GlobalCoverage
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        Some(vec![PassType::EdgeTracer])
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let tracer_output = update.get_pass_result(PassType::EdgeTracer);
        let mut reader = csv::Reader::from_reader(tracer_output.as_slice());
        let mut diff = HashSet::new();
        for result in reader.deserialize() {
            let edge_record: EdgeRecord = result.expect("Could not parse CSV entry");
            let edge = Edge::from(edge_record);
            if self.edges.insert(edge) {
                diff.insert(edge);
            }
        }

        let diff_vec = serde_cbor::to_vec(&diff).expect("Failed to serialize analysis");
        if let Err(e) = self.logger.lock().unwrap().log_analysis_state(
            update.get_test_handle().clone(),
            update.get_fuzzer_id(),
            self.analysis_type(),
            diff_vec,
        ) {
            log::error!("Failed to log analysis state: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzers::FuzzerId;
    use crate::logger::tests::{cleanup as logger_cleanup, create_shared_logger};
    use crate::storage::TestCaseHandle;
    use std::env;
    use std::fs;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

    #[test]
    fn test_global_coverage_analysis() {
        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_OBJDUMP_PATH"));
        let empty_path = PathBuf::from("tests/assets/empty");

        let temp_dir = env::temp_dir().join("pass_tests").join("edge_tracer_pass");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = PassConfig {
            program_arguments: vec![String::from("-d"), String::from("@@")],
            analysis_artifacts_dir: binaries_dir,
            analysis_input_dir: temp_dir,
        };
        let edge_tracer_pass = PassType::EdgeTracer.get_pass(config.clone()).unwrap();

        let mut test_case_file = File::open(empty_path).unwrap();
        let mut test_case = Vec::new();
        test_case_file.read_to_end(&mut test_case).unwrap();
        let output = edge_tracer_pass
            .process(&test_case)
            .expect("process failed");

        let test_handle = TestCaseHandle::get_fake_handle("");

        let logger_output_dir = "test_edge_tracer_pass";
        let mut global_coverage_state =
            GlobalCoverageState::new(&config, create_shared_logger(logger_output_dir));
        let mut update = AnalysisUpdate::new(test_handle.clone(), FuzzerId::new(42), Vec::new());
        update.add_pass_result(PassType::EdgeTracer, output);
        global_coverage_state.update(&update);

        let global_coverage = global_coverage_state.get_global_coverage();

        let test_edge = Edge::new(0xd9fd, 0xda01);
        let assert_value = global_coverage.contains(&test_edge);
        if !assert_value {
            println!("test_edge: {:?}", test_edge);
            println!("global_coverage: {:?}", global_coverage);
        }

        assert!(assert_value);

        logger_cleanup(logger_output_dir);
    }
}
