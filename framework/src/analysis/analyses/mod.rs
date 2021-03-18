use super::{AnalysisUpdate, GlobalState, PassConfig, PassType};

pub mod coverage_utils;
mod dfsan_utils;
mod instruction_count_utils;
mod observed_conditions_utils;

mod libsvm;
mod static_branch_metrics;

mod test_analysis;
pub use test_analysis::TestAnalysisState;

mod fuzzer_coverage_analysis;
pub use global_coverage_analysis::GlobalCoverageState;

mod global_coverage_analysis;
pub use fuzzer_coverage_analysis::FuzzerCoverageState;

mod observed_conditions_analysis;
pub use observed_conditions_analysis::ObservedConditionsState;

mod fuzzer_observed_conditions_analysis;
pub use fuzzer_observed_conditions_analysis::FuzzerObservedConditionsState;

// XXX: Needs restructuring since each fuzzer has it's own generation graph
// mod generation_graph_analysis;
// pub use generation_graph_analysis::GenerationGraphState;

mod test_case_benefit_analysis;
pub use test_case_benefit_analysis::TestCaseBenefitGlobalState;

mod instruction_count_analysis;
pub use instruction_count_analysis::InstructionCountGlobalState;

mod fuzzer_info;
pub use fuzzer_info::FuzzerIdState;

mod tainted_conditions_analysis;
pub use tainted_conditions_analysis::TaintedConditionsGlobalState;

mod regressor_analysis;
pub use regressor_analysis::RegressorGlobalState;

mod condition_bytes_analysis;
pub use condition_bytes_analysis::{ConditionBytesGlobalState, TerminatorInfo};

use super::utils;
use crate::types::SharedLogger;
use std::fmt;

// Add a case to this enum for the new analysis
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum AnalysisType {
    Test,
    GlobalCoverage,
    FuzzerCoverage,
    ObservedConditions,
    FuzzerObservedConditions,
    // GenerationGraph,
    TestCaseBenefit,
    InstructionCount,
    FuzzerId,
    TaintedConditions,
    RegressorPredictions,
    ConditionBytes,
}

pub fn get_analysis_types() -> Vec<AnalysisType> {
    vec![
        AnalysisType::Test,
        AnalysisType::GlobalCoverage,
        AnalysisType::FuzzerCoverage,
        AnalysisType::ObservedConditions,
        AnalysisType::FuzzerObservedConditions,
        // AnalysisType::GenerationGraph,
        AnalysisType::TestCaseBenefit,
        AnalysisType::InstructionCount,
        AnalysisType::FuzzerId,
        AnalysisType::TaintedConditions,
        AnalysisType::RegressorPredictions,
        AnalysisType::ConditionBytes,
    ]
}

impl AnalysisType {
    pub fn get_analysis_state(
        self,
        config: &PassConfig,
        logger: SharedLogger,
    ) -> Box<dyn GlobalState> {
        // Add a case to this match to allocate the corresponding analysis state
        let analysis_state: Box<dyn GlobalState> = match self {
            AnalysisType::Test => Box::new(TestAnalysisState::new(config)),
            AnalysisType::GlobalCoverage => Box::new(GlobalCoverageState::new(config, logger)),
            AnalysisType::FuzzerCoverage => Box::new(FuzzerCoverageState::new(config, logger)),
            AnalysisType::ObservedConditions => {
                Box::new(ObservedConditionsState::new(config, logger))
            }
            AnalysisType::FuzzerObservedConditions => {
                Box::new(FuzzerObservedConditionsState::new(config, logger))
            }
            // AnalysisType::GenerationGraph => Box::new(GenerationGraphState::new(config, logger)),
            AnalysisType::TestCaseBenefit => {
                Box::new(TestCaseBenefitGlobalState::new(config, logger))
            }
            AnalysisType::InstructionCount => {
                Box::new(InstructionCountGlobalState::new(config, logger))
            }
            AnalysisType::FuzzerId => Box::new(FuzzerIdState::new(config, logger)),
            AnalysisType::TaintedConditions => {
                Box::new(TaintedConditionsGlobalState::new(config, logger))
            }
            AnalysisType::RegressorPredictions => {
                Box::new(RegressorGlobalState::new(config, logger))
            }
            AnalysisType::ConditionBytes => {
                Box::new(ConditionBytesGlobalState::new(config, logger))
            }
        };
        assert_eq!(
            self,
            analysis_state.analysis_type(),
            "GlobalState has wrong type"
        );

        analysis_state
    }

    pub fn needs_duplicates(self) -> bool {
        match self {
            AnalysisType::Test => false,
            AnalysisType::GlobalCoverage => false,
            AnalysisType::FuzzerCoverage => true,
            AnalysisType::ObservedConditions => false,
            AnalysisType::FuzzerObservedConditions => true,
            // AnalysisType::GenerationGraph => true,
            AnalysisType::TestCaseBenefit => false,
            AnalysisType::InstructionCount => false,
            AnalysisType::FuzzerId => true,
            AnalysisType::TaintedConditions => false,
            AnalysisType::RegressorPredictions => true,
            AnalysisType::ConditionBytes => false,
        }
    }
}

impl fmt::Display for AnalysisType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalysisType::Test => write!(f, "test"),
            AnalysisType::GlobalCoverage => write!(f, "global_coverage"),
            AnalysisType::FuzzerCoverage => write!(f, "fuzzer_coverage"),
            AnalysisType::ObservedConditions => write!(f, "observed_conditions"),
            AnalysisType::FuzzerObservedConditions => write!(f, "fuzzer_observed_conditions"),
            // AnalysisType::GenerationGraph => write!(f, "generation_graph"),
            AnalysisType::TestCaseBenefit => write!(f, "test_case_benefit"),
            AnalysisType::InstructionCount => write!(f, "instruction_count"),
            AnalysisType::FuzzerId => write!(f, "fuzzer_id"),
            AnalysisType::TaintedConditions => write!(f, "tainted_conditions"),
            AnalysisType::RegressorPredictions => write!(f, "regressor_predictions"),
            AnalysisType::ConditionBytes => write!(f, "condition_bytes"),
        }
    }
}
