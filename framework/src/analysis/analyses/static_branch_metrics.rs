use super::utils::get_artifact_path;
use serde::de;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct StaticMetrics {
    #[serde(rename = "BasicBlock")]
    basic_block_id: u64,
    #[serde(rename = "Condition")]
    condition_id: u64,
    cyclomatic: u64,
    oviedo: u64,
    chain_size: u64,
    compare_size: u64,
    #[serde(deserialize_with = "bool_from_string")]
    compares_constant: bool,
    #[serde(deserialize_with = "bool_from_string")]
    compares_pointer: bool,
    #[serde(deserialize_with = "bool_from_string")]
    is_equality: bool,
    #[serde(deserialize_with = "bool_from_string")]
    is_constant: bool,
    cases: u64,
}

impl StaticMetrics {
    pub fn get_oviedo(&self) -> u64 {
        self.oviedo
    }

    pub fn get_chain_size(&self) -> u64 {
        self.chain_size
    }

    pub fn get_compare_size(&self) -> u64 {
        self.compare_size
    }
}

fn bool_from_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deserializer)?.as_ref() {
        "1" => Ok(true),
        "0" => Ok(false),
        other => Err(de::Error::invalid_value(
            de::Unexpected::Str(other),
            &"Expected to be 0 or 1",
        )),
    }
}

pub struct StaticBranchMetrics {
    condition_metrics: HashMap<u64, StaticMetrics>,
}

impl StaticBranchMetrics {
    pub fn new(analysis_artifacts_dir: &PathBuf) -> Self {
        let csv_path = get_artifact_path(analysis_artifacts_dir, "static-metrics").unwrap();

        let mut reader =
            csv::Reader::from_path(csv_path).expect("Failed to read static metrics CSV");

        let mut condition_metrics = HashMap::new();
        for row in reader.deserialize() {
            let m: StaticMetrics = row.expect("Could not parse static metrics CSV entry");
            condition_metrics.insert(m.condition_id, m);
        }

        Self { condition_metrics }
    }

    pub fn get_metrics(&self, condition_id: u64) -> &StaticMetrics {
        self.condition_metrics
            .get(&condition_id)
            .unwrap_or_else(|| panic!("No static metrics for condition {}", condition_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_branch_metrics() {
        let binaries_dir = PathBuf::from(env!("ANALYSIS_BINARIES_OBJDUMP_PATH"));
        let static_branch_metrics = StaticBranchMetrics::new(&binaries_dir);

        let true_metrics = vec![
            StaticMetrics {
                basic_block_id: 174377,
                condition_id: 174381,
                cyclomatic: 60,
                oviedo: 231,
                chain_size: 7,
                compare_size: 0,
                compares_constant: true,
                compares_pointer: true,
                is_equality: true,
                is_constant: false,
                cases: 2,
            },
            StaticMetrics {
                basic_block_id: 51047,
                condition_id: 51064,
                cyclomatic: 117,
                oviedo: 594,
                chain_size: 25,
                compare_size: 16,
                compares_constant: true,
                compares_pointer: false,
                is_equality: true,
                is_constant: false,
                cases: 2,
            },
            StaticMetrics {
                basic_block_id: 114065,
                condition_id: 114076,
                cyclomatic: 145,
                oviedo: 603,
                chain_size: 3,
                compare_size: 0,
                compares_constant: true,
                compares_pointer: false,
                is_equality: true,
                is_constant: false,
                cases: 43,
            },
            StaticMetrics {
                basic_block_id: 267214,
                condition_id: 267223,
                cyclomatic: 908,
                oviedo: 6332,
                chain_size: 10,
                compare_size: 64,
                compares_constant: false,
                compares_pointer: false,
                is_equality: true,
                is_constant: false,
                cases: 2,
            },
        ];

        for true_m in true_metrics {
            let m = static_branch_metrics.get_metrics(true_m.condition_id);
            assert_eq!(true_m, *m);
        }
    }
}
