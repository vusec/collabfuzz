use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType, SharedLogger};
use crate::storage::TestCaseHandle;
use std::collections::HashMap;

// The generation graph is directed from a child towards its parents. This guarantees that, when
// updating the graph with a new child, only nodes starting from it are added. The rest of the
// graph is left untouched.
pub struct GenerationGraphState {
    graph: HashMap<TestCaseHandle, Vec<TestCaseHandle>>,
    logger: SharedLogger,
}

impl GenerationGraphState {
    pub fn new(_config: &PassConfig, logger: SharedLogger) -> Self {
        Self {
            graph: HashMap::new(),
            logger,
        }
    }

    #[allow(dead_code)]
    pub fn get_generation_graph(&self) -> &HashMap<TestCaseHandle, Vec<TestCaseHandle>> {
        &self.graph
    }
}

impl GlobalState for GenerationGraphState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::GenerationGraph
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        None
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let should_be_none = self
            .graph
            .insert(update.test_handle.clone(), update.parent_handles.clone());
        assert!(
            should_be_none.is_none(),
            "Test case already present in graph"
        );

        // Each diff contains a vector with the ids of the parents of the test case that was
        // reported. The parents are identified only through their unique identifier.
        let serialized_parent_handles: Vec<_> = update
            .parent_handles
            .iter()
            .map(|handle| handle.get_unique_id().clone())
            .collect();

        {
            let logger = self.logger.lock().unwrap();
            if let Err(e) = logger.log_analysis_state(
                update.test_handle.clone(),
                self.analysis_type(),
                serde_cbor::to_vec(&serialized_parent_handles)
                    .expect("Failed to serialize parent handles"),
            ) {
                log::error!("Failed to log generation graph: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzers::FuzzerId;
    use crate::logger::tests::{cleanup as logger_cleanup, create_shared_logger};
    use crate::storage::TestCaseHandle;
    use std::path::PathBuf;

    #[test]
    fn test_generation_graph_analysis() {
        // Not used in this test
        let config = PassConfig {
            program_arguments: Vec::new(),
            analysis_artifacts_dir: PathBuf::new(),
            analysis_input_dir: PathBuf::new(),
        };

        let logger_output_dir = "test_generation_graph";
        let mut generation_graph =
            GenerationGraphState::new(&config, create_shared_logger(logger_output_dir));

        eprintln!("Inserting parent");

        let parent_handle = TestCaseHandle::get_fake_handle("parent");
        let parent_update =
            AnalysisUpdate::new(parent_handle.clone(), FuzzerId::new(42), Vec::new());
        generation_graph.update(&parent_update);

        eprintln!("Inserting child");

        let child_handle = TestCaseHandle::get_fake_handle("child");
        let child_update = AnalysisUpdate::new(
            child_handle.clone(),
            FuzzerId::new(42),
            vec![parent_handle.clone()],
        );
        generation_graph.update(&child_update);

        eprintln!("Checking");

        let graph = generation_graph.get_generation_graph();
        let parents = graph.get(&child_handle).unwrap();
        assert_eq!(*parents, vec![parent_handle]);

        logger_cleanup(logger_output_dir);
    }
}
