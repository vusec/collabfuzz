use super::coverage_utils::{Edge, EdgeRecord};
use super::dfsan_utils::DFSanResult;
use super::utils::get_artifact_path;
use super::{AnalysisType, AnalysisUpdate, GlobalState, PassConfig, PassType, SharedLogger};
use crate::storage::TestCaseHandle;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Bfs, EdgeFiltered, EdgeRef};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;

struct CFGNode {
    seen: bool,
}

impl CFGNode {
    fn new() -> Self {
        Self { seen: false }
    }

    fn set_seen(&mut self) {
        self.seen = true;
    }

    fn is_seen(&self) -> bool {
        self.seen
    }
}

pub struct TestCaseBenefitGlobalState {
    interprocedural_cfg: DiGraph<CFGNode, ()>,
    node_id_to_index: HashMap<u64, NodeIndex>,
    node_index_to_id: HashMap<NodeIndex, u64>,
    test_case_to_frontier: HashMap<TestCaseHandle, Vec<NodeIndex>>,
    node_id_to_terminator_id: HashMap<u64, u64>,
    logger: SharedLogger,
}

impl TestCaseBenefitGlobalState {
    pub fn new(config: &PassConfig, logger: SharedLogger) -> Self {
        let json_cfg_path = get_artifact_path(&config.analysis_artifacts_dir, "bb-reach").unwrap();

        // TODO: Return errors instead of panicking
        let json_cfg =
            fs::read_to_string(json_cfg_path).expect("Could not read JSON adjacency list");
        let adjacency_list: Vec<(u64, Vec<u64>)> =
            serde_json::from_str(&json_cfg).expect("Could not decode JSON adjacency list");

        let mut interprocedural_cfg = DiGraph::<CFGNode, ()>::new();
        let mut node_id_to_index = HashMap::new();
        let mut node_index_to_id = HashMap::new();

        for (node_id, _) in &adjacency_list {
            let node_index = interprocedural_cfg.add_node(CFGNode::new());
            node_id_to_index.insert(*node_id, node_index);
            node_index_to_id.insert(node_index, *node_id);
        }

        for (node_id, neighbors) in &adjacency_list {
            for neighbor_id in neighbors {
                interprocedural_cfg.add_edge(
                    node_id_to_index[&node_id],
                    node_id_to_index[&neighbor_id],
                    (),
                );
            }
        }

        Self {
            interprocedural_cfg,
            node_id_to_index,
            node_index_to_id,
            test_case_to_frontier: HashMap::new(),
            node_id_to_terminator_id: HashMap::new(),
            logger,
        }
    }

    fn update_seen_nodes(&mut self, node_ids: impl IntoIterator<Item = u64>) {
        for node_id in node_ids {
            self.interprocedural_cfg
                .node_weight_mut(self.node_id_to_index[&node_id])
                .unwrap()
                .set_seen();
        }

        // TODO: The frontiers can be shrunk here to speed up the calls made by the scheduler.
        // Benchmarking is needed, however; the time spent shrinking them could be more than the
        // time gained.
    }

    fn add_frontier(&mut self, test_handle: TestCaseHandle, tainted_nodes: &[u64]) -> Vec<u64> {
        // The definition of frontier is: "All the basic blocks whose terminator is tainted by the
        // input that have at least one unseen neighbor".
        let mut frontier_indexes = Vec::new();
        let mut frontier_ids = Vec::new();
        for node_id in tainted_nodes {
            let node_index = self.node_id_to_index[&node_id];
            for neighbor_id in self.interprocedural_cfg.neighbors(node_index) {
                let is_seen = self
                    .interprocedural_cfg
                    .node_weight(neighbor_id)
                    .unwrap()
                    .is_seen();
                if !is_seen {
                    frontier_indexes.push(node_index);
                    frontier_ids.push(*node_id);
                    break;
                }
            }
        }

        self.test_case_to_frontier
            .insert(test_handle, frontier_indexes);

        frontier_ids
    }

    pub fn get_terminator_id(&self, node_id: u64) -> u64 {
        self.node_id_to_terminator_id[&node_id]
    }

    pub fn count_reachable_unseen_nodes_per_node(
        &self,
        test_handle: &TestCaseHandle,
    ) -> HashMap<u64, usize> {
        let mut reachable_nodes = HashMap::new();

        let filtered_graph = EdgeFiltered::from_fn(&self.interprocedural_cfg, |edge| {
            !&self
                .interprocedural_cfg
                .node_weight(edge.target())
                .unwrap()
                .is_seen()
        });

        let test_frontier = self.test_case_to_frontier.get(test_handle).unwrap();
        for frontier_node_idx in test_frontier {
            let mut bfs = Bfs::new(&filtered_graph, *frontier_node_idx);
            let mut count = 0;
            while let Some(node_idx) = bfs.next(&filtered_graph) {
                let node = &self.interprocedural_cfg.node_weight(node_idx).unwrap();

                // The first node in the visit will always be seen, it should be ignored.
                if !node.is_seen() {
                    count += 1;
                }
            }
            let frontier_terminator_id =
                self.get_terminator_id(self.node_index_to_id[frontier_node_idx]);
            reachable_nodes.insert(frontier_terminator_id, count);
        }

        reachable_nodes
    }

    pub fn count_reachable_unseen_nodes(&self, test_handle: &TestCaseHandle) -> usize {
        let reachable_nodes_cell = RefCell::new(HashSet::new());

        let filtered_graph = EdgeFiltered::from_fn(&self.interprocedural_cfg, |edge| {
            !&self
                .interprocedural_cfg
                .node_weight(edge.target())
                .unwrap()
                .is_seen()
                && !reachable_nodes_cell.borrow().contains(&edge.target())
        });

        let test_frontier = self.test_case_to_frontier.get(test_handle).unwrap();
        for frontier_node_idx in test_frontier {
            let mut bfs = Bfs::new(&filtered_graph, *frontier_node_idx);
            while let Some(node_idx) = bfs.next(&filtered_graph) {
                let node = &self.interprocedural_cfg.node_weight(node_idx).unwrap();

                // The first node in the visit will always be seen, it should be ignored.
                if !node.is_seen() {
                    reachable_nodes_cell.borrow_mut().insert(node_idx);
                }
            }
        }

        let reachable_nodes = reachable_nodes_cell.borrow();
        reachable_nodes.len()
    }
}

impl GlobalState for TestCaseBenefitGlobalState {
    fn analysis_type(&self) -> AnalysisType {
        AnalysisType::TestCaseBenefit
    }

    fn get_required_passes(&self) -> Option<Vec<PassType>> {
        Some(vec![PassType::EdgeTracer, PassType::BBTaintTracer])
    }

    fn update(&mut self, update: &AnalysisUpdate) {
        let edge_tracer_output = update.get_pass_result(PassType::EdgeTracer);
        let mut reader = csv::Reader::from_reader(edge_tracer_output.as_slice());

        let mut nodes_seen = HashSet::new();
        for result in reader.deserialize() {
            let edge_record: EdgeRecord = result.expect("Could not parse edge CSV entry");
            let edge = Edge::from(edge_record);
            nodes_seen.insert(edge.get_source());
            nodes_seen.insert(edge.get_target());
        }

        self.update_seen_nodes(nodes_seen);

        let bb_tracer_output = update.get_pass_result(PassType::BBTaintTracer);
        let mut reader = csv::Reader::from_reader(bb_tracer_output.as_slice());

        let mut tainted_nodes = Vec::new();
        for result in reader.deserialize() {
            let dfsan_result: DFSanResult = result.expect("Could not parse DFSan CSV entry");
            if dfsan_result.is_tainted() {
                tainted_nodes.push(dfsan_result.get_basic_block_id());
            }
            self.node_id_to_terminator_id.insert(
                dfsan_result.get_basic_block_id(),
                dfsan_result.get_terminator_id(),
            );
        }

        let new_frontier = self.add_frontier(update.get_test_handle().clone(), &tainted_nodes);

        // The logger serializes just the new frontier being added, without recording any changes
        // in the old frontiers. This allows to do diff logging and should not be an issue since
        // removing nodes is just an optimization done internally.
        let serialized_frontier =
            serde_cbor::to_vec(&new_frontier).expect("Failed to serialize frontier");

        {
            let logger = self.logger.lock().unwrap();
            if let Err(e) = logger.log_analysis_state(
                update.get_test_handle().clone(),
                update.get_fuzzer_id(),
                self.analysis_type(),
                serialized_frontier,
            ) {
                log::error!("Failed to log analysis state: {}", e);
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
    use std::env;
    use std::fs;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

    #[test]
    fn test_test_case_benefit_analysis() {
        env::set_var("LD_LIBRARY_PATH", env!("RTLIBS_INSTALL_DIR"));
        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_OBJDUMP_PATH"));
        let empty_path = PathBuf::from("tests/assets/empty");
        let empty_stripped_path = PathBuf::from("tests/assets/empty_stripped");

        let temp_dir = env::temp_dir()
            .join("pass_tests")
            .join("test_case_benefit_pass");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = PassConfig {
            // XXX: `-d @@` does not work because of an indirect vararg call to fprintf
            program_arguments: vec![String::from("-x"), String::from("@@")],
            analysis_artifacts_dir: binaries_dir.clone(),
            analysis_input_dir: temp_dir,
        };

        let edge_tracer_pass = PassType::EdgeTracer.get_pass(config.clone()).unwrap();
        let bb_tracer_pass = PassType::BBTaintTracer.get_pass(config.clone()).unwrap();

        let logger_output_dir = "test_case_benefit_pass";
        let mut test_case_benefit_global_state = TestCaseBenefitGlobalState::new(
            &PassConfig {
                program_arguments: vec![],
                analysis_artifacts_dir: binaries_dir,
                analysis_input_dir: PathBuf::new(),
            },
            create_shared_logger(logger_output_dir),
        );

        let fuzzer_id = FuzzerId::new(42);

        let mut empty_stripped_file = File::open(empty_stripped_path).unwrap();
        let mut empty_stripped = Vec::new();
        empty_stripped_file
            .read_to_end(&mut empty_stripped)
            .unwrap();
        let empty_stripped_edge_tracer_output = edge_tracer_pass
            .process(&empty_stripped)
            .expect("process failed");
        let empty_stripped_bb_tracer_output = bb_tracer_pass
            .process(&empty_stripped)
            .expect("process failed");
        let empty_stripped_handle = TestCaseHandle::get_fake_handle("empty_stripped");

        let mut update = AnalysisUpdate::new(empty_stripped_handle.clone(), fuzzer_id, Vec::new());
        update.add_pass_result(PassType::EdgeTracer, empty_stripped_edge_tracer_output);
        update.add_pass_result(PassType::BBTaintTracer, empty_stripped_bb_tracer_output);
        test_case_benefit_global_state.update(&update);

        let count =
            test_case_benefit_global_state.count_reachable_unseen_nodes(&empty_stripped_handle);
        assert_eq!(count, 15784);

        let mut empty_file = File::open(empty_path).unwrap();
        let mut empty = Vec::new();
        empty_file.read_to_end(&mut empty).unwrap();
        let empty_edge_tracer_output = edge_tracer_pass.process(&empty).expect("process failed");
        let empty_bb_tracer_output = bb_tracer_pass.process(&empty).expect("process failed");
        let empty_handle = TestCaseHandle::get_fake_handle("empty");

        let mut update = AnalysisUpdate::new(empty_handle.clone(), fuzzer_id, Vec::new());
        update.add_pass_result(PassType::EdgeTracer, empty_edge_tracer_output);
        update.add_pass_result(PassType::BBTaintTracer, empty_bb_tracer_output);
        test_case_benefit_global_state.update(&update);

        let empty_stripped_count =
            test_case_benefit_global_state.count_reachable_unseen_nodes(&empty_stripped_handle);
        let empty_count =
            test_case_benefit_global_state.count_reachable_unseen_nodes(&empty_handle);
        assert_eq!(empty_stripped_count, 15682);
        assert_eq!(empty_count, 15724);

        logger_cleanup(logger_output_dir);
    }

    #[test]
    fn test_test_case_benefit_analysis_with_cutoff() {
        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_CUTOFF_PATH"));
        let cutoff_0_path = PathBuf::from("tests/assets/cutoff_0");
        let cutoff_1_path = PathBuf::from("tests/assets/cutoff_1");

        let temp_dir = env::temp_dir()
            .join("pass_tests")
            .join("test_case_benefit_cutoff_pass");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = PassConfig {
            program_arguments: vec![],
            analysis_artifacts_dir: binaries_dir.clone(),
            analysis_input_dir: temp_dir,
        };

        let edge_tracer_pass = PassType::EdgeTracer.get_pass(config.clone()).unwrap();
        let bb_tracer_pass = PassType::BBTaintTracer.get_pass(config.clone()).unwrap();

        let logger_output_dir = "test_case_benefit_cutoff_pass";
        let mut test_case_benefit_global_state = TestCaseBenefitGlobalState::new(
            &PassConfig {
                program_arguments: vec![],
                analysis_artifacts_dir: binaries_dir,
                analysis_input_dir: PathBuf::new(),
            },
            create_shared_logger(logger_output_dir),
        );

        let fuzzer_id = FuzzerId::new(42);

        println!("Process cutoff_0");
        let mut cutoff_0_file = File::open(cutoff_0_path).unwrap();
        let mut cutoff_0 = Vec::new();
        cutoff_0_file.read_to_end(&mut cutoff_0).unwrap();
        let cutoff_0_edge_tracer_output =
            edge_tracer_pass.process(&cutoff_0).expect("process failed");
        let cutoff_0_bb_tracer_output = bb_tracer_pass.process(&cutoff_0).expect("process failed");
        let cutoff_0_handle = TestCaseHandle::get_fake_handle("cutoff_0");

        let mut update = AnalysisUpdate::new(cutoff_0_handle.clone(), fuzzer_id, Vec::new());
        update.add_pass_result(PassType::EdgeTracer, cutoff_0_edge_tracer_output);
        update.add_pass_result(PassType::BBTaintTracer, cutoff_0_bb_tracer_output);
        test_case_benefit_global_state.update(&update);

        let count = test_case_benefit_global_state.count_reachable_unseen_nodes(&cutoff_0_handle);
        assert_eq!(count, 2);

        println!("Process cutoff_1");
        let mut cutoff_1_file = File::open(cutoff_1_path).unwrap();
        let mut cutoff_1 = Vec::new();
        cutoff_1_file.read_to_end(&mut cutoff_1).unwrap();
        let cutoff_1_edge_tracer_output =
            edge_tracer_pass.process(&cutoff_1).expect("process failed");
        let cutoff_1_bb_tracer_output = bb_tracer_pass.process(&cutoff_1).expect("process failed");
        let cutoff_1_handle = TestCaseHandle::get_fake_handle("cutoff_1");

        let mut update = AnalysisUpdate::new(cutoff_1_handle.clone(), fuzzer_id, Vec::new());
        update.add_pass_result(PassType::EdgeTracer, cutoff_1_edge_tracer_output);
        update.add_pass_result(PassType::BBTaintTracer, cutoff_1_bb_tracer_output);
        test_case_benefit_global_state.update(&update);

        let cutoff_0_count =
            test_case_benefit_global_state.count_reachable_unseen_nodes(&cutoff_0_handle);
        let cutoff_1_count =
            test_case_benefit_global_state.count_reachable_unseen_nodes(&cutoff_1_handle);
        assert_eq!(cutoff_0_count, 0);
        assert_eq!(cutoff_1_count, 1);

        logger_cleanup(logger_output_dir);
    }
}
