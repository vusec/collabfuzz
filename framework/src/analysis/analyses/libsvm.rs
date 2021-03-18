use super::static_branch_metrics::StaticMetrics;
use crate::fuzzers::FuzzerType;
use libsvm_sys::{svm_free_and_destroy_model, svm_load_model, svm_model, svm_node, svm_predict};
use std::convert::TryFrom;
use std::env;
use std::ffi::CString;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::ptr::NonNull;

// These models are loaded at compile time into strings and written to files at runtime because we
// want to include them statically in the framework and libSVM reads them from a file.
// NB: creating multiple copies of `Model` for the same fuzzer type in parallel may overlap the
// creation / writing of the model files, making the `svm_load_model` function fail.
const HONGGFUZZ_MODEL: &str = include_str!("ml_models/honggfuzz.model");
const HONGGFUZZ_RANGE: &str = include_str!("ml_models/honggfuzz.range");
const QSYM_MODEL: &str = include_str!("ml_models/qsym.model");
const QSYM_RANGE: &str = include_str!("ml_models/qsym.range");
const LIBFUZZER_MODEL: &str = include_str!("ml_models/libfuzzer.model");
const LIBFUZZER_RANGE: &str = include_str!("ml_models/libfuzzer.range");
const AFL_MODEL: &str = include_str!("ml_models/afl.model");
const AFL_RANGE: &str = include_str!("ml_models/afl.range");
const AFLFAST_MODEL: &str = include_str!("ml_models/aflfast.model");
const AFLFAST_RANGE: &str = include_str!("ml_models/aflfast.range");
const FAIRFUZZ_MODEL: &str = include_str!("ml_models/fairfuzz.model");
const FAIRFUZZ_RANGE: &str = include_str!("ml_models/fairfuzz.range");
const RADAMSA_MODEL: &str = include_str!("ml_models/radamsa.model");
const RADAMSA_RANGE: &str = include_str!("ml_models/radamsa.range");

const NUM_FEATURES: usize = 4;

pub struct Query {
    oviedo: f64,
    chain_size: f64,
    compare_size: f64,
    instruction_count: f64,
}

impl Query {
    pub fn new(static_features: &StaticMetrics, instruction_count: u64) -> Self {
        Self {
            oviedo: static_features.get_oviedo() as f64,
            chain_size: static_features.get_chain_size() as f64,
            compare_size: static_features.get_compare_size() as f64,
            instruction_count: instruction_count as f64,
        }
    }

    fn to_svm_nodes(&self) -> [svm_node; NUM_FEATURES + 1] {
        [
            svm_node {
                index: 1,
                value: self.oviedo,
            },
            svm_node {
                index: 2,
                value: self.chain_size,
            },
            svm_node {
                index: 3,
                value: self.compare_size,
            },
            svm_node {
                index: 4,
                value: self.instruction_count,
            },
            svm_node {
                index: -1,
                value: 0.,
            },
        ]
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct ScalerRange {
    min: f64,
    max: f64,
}

impl ScalerRange {
    fn scale(&self, v: f64, bounds: &Bounds) -> f64 {
        // https://github.com/cjlin1/libsvm/blob/v324/svm-scale.c#L372-L392
        if (self.min - self.max).abs() < f64::EPSILON {
            0.
        } else if (v - self.min).abs() < f64::EPSILON {
            bounds.lower
        } else if (v - self.max).abs() < f64::EPSILON {
            bounds.upper
        } else {
            bounds.lower + (bounds.upper - bounds.lower) * (v - self.min) / (self.max - self.min)
        }
    }
}

impl TryFrom<&[&str]> for ScalerRange {
    type Error = std::num::ParseFloatError;
    fn try_from(split: &[&str]) -> Result<Self, Self::Error> {
        Ok(ScalerRange {
            min: split[0].parse()?,
            max: split[1].parse()?,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Bounds {
    lower: f64,
    upper: f64,
}

impl TryFrom<&[&str]> for Bounds {
    type Error = std::num::ParseFloatError;
    fn try_from(split: &[&str]) -> Result<Self, Self::Error> {
        Ok(Bounds {
            lower: split[0].parse()?,
            upper: split[1].parse()?,
        })
    }
}

#[derive(Debug)]
pub enum ModelError {
    Parsing(String, Option<Box<dyn std::error::Error + 'static>>),
    Loading(String, Option<Box<dyn std::error::Error + 'static>>),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parsing(e, s) => match s {
                Some(s) => write!(f, "{}: {}", e, s),
                None => write!(f, "{}", e),
            },
            Self::Loading(e, s) => match s {
                Some(s) => write!(f, "{}: {}", e, s),
                None => write!(f, "{}", e),
            },
        }
    }
}

impl std::error::Error for ModelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parsing(_, source) => source.as_ref().map(|s| s.as_ref()),
            Self::Loading(_, source) => source.as_ref().map(|s| s.as_ref()),
        }
    }
}

pub struct Model {
    model: NonNull<svm_model>,
    features_range: [ScalerRange; NUM_FEATURES],
    features_bounds: Bounds,
    #[allow(dead_code)]
    y_range: Option<ScalerRange>,
    #[allow(dead_code)]
    y_bounds: Option<Bounds>,
}

unsafe impl std::marker::Send for Model {}

impl Model {
    fn store_file(path: PathBuf, contents: &str) -> Result<String, ModelError> {
        let mut file = File::create(&path).map_err(|e| {
            ModelError::Loading(
                format!("Failed to create model file {}", path.to_string_lossy(),),
                Some(Box::new(e)),
            )
        })?;

        file.write_all(contents.as_bytes()).map_err(|e| {
            ModelError::Loading(
                format!("Failed to write to model file {}", path.to_string_lossy(),),
                Some(Box::new(e)),
            )
        })?;

        Ok(path.to_string_lossy().into_owned())
    }

    pub fn load(fuzzer_type: FuzzerType) -> Result<Self, ModelError> {
        let (model_str, range_str) = match fuzzer_type {
            FuzzerType::HONGGFUZZ => (HONGGFUZZ_MODEL, HONGGFUZZ_RANGE),
            FuzzerType::QSYM => (QSYM_MODEL, QSYM_RANGE),
            FuzzerType::LIBFUZZER => (LIBFUZZER_MODEL, LIBFUZZER_RANGE),
            FuzzerType::AFL => (AFL_MODEL, AFL_RANGE),
            FuzzerType::AFLFAST => (AFLFAST_MODEL, AFLFAST_RANGE),
            FuzzerType::FAIRFUZZ => (FAIRFUZZ_MODEL, FAIRFUZZ_RANGE),
            FuzzerType::RADAMSA => (RADAMSA_MODEL, RADAMSA_RANGE),
            FuzzerType::Unknown | FuzzerType::ANGORA => {
                unimplemented!("SVM model is not available for {}", fuzzer_type)
            }
        };

        let mut features_range: [ScalerRange; NUM_FEATURES] =
            [ScalerRange { min: 0., max: 0. }; NUM_FEATURES];
        let mut features_bounds = Bounds {
            lower: -1.,
            upper: 1.,
        };
        let mut y_range = None;
        let mut y_bounds = None;

        // Whether we are paring feature ranges or y ranges
        let mut parsing_feature = false;
        // Whether we are parsing bounds
        let mut parsing_bounds = false;
        for range_line in range_str.lines() {
            if range_line.starts_with('y') {
                parsing_feature = false;
                parsing_bounds = true;
                continue;
            } else if range_line.starts_with('x') {
                parsing_feature = true;
                parsing_bounds = true;
                continue;
            }

            if parsing_bounds {
                let split: Vec<_> = range_line.split(' ').collect();
                if split.len() != 2 {
                    return Err(ModelError::Parsing(
                        format!("Expected 2 values, got {}", split.len()),
                        None,
                    ));
                }

                let bounds = Bounds::try_from(&split[..]).map_err(|e| {
                    ModelError::Parsing(
                        "Failed to parse bounds from range file".to_string(),
                        Some(Box::new(e)),
                    )
                })?;

                if parsing_feature {
                    features_bounds = bounds;
                } else {
                    y_bounds = Some(bounds);
                }

                parsing_bounds = false;
            } else if parsing_feature {
                let split: Vec<_> = range_line.split(' ').collect();
                if split.len() != 3 {
                    return Err(ModelError::Parsing(
                        format!("Expected 3 values, got {}", split.len()),
                        None,
                    ));
                }

                let feat_idx: usize = split[0].parse().map_err(|e| {
                    ModelError::Parsing(
                        "Could not parse index value from range file".to_string(),
                        Some(Box::new(e)),
                    )
                })?;

                if feat_idx == 0 || feat_idx > NUM_FEATURES {
                    return Err(ModelError::Parsing(
                        format!("Range file contains unexpected index value {}", feat_idx),
                        None,
                    ));
                }

                features_range[feat_idx - 1] = ScalerRange::try_from(&split[1..]).map_err(|e| {
                    ModelError::Parsing(
                        "Failed to parse feature scaler range".to_string(),
                        Some(Box::new(e)),
                    )
                })?;
            } else {
                let split: Vec<_> = range_line.split(' ').collect();
                if split.len() != 2 {
                    return Err(ModelError::Parsing(
                        format!("Expected 2 values, got {}", split.len()),
                        None,
                    ));
                }
                y_range = Some(ScalerRange::try_from(&split[..]).map_err(|e| {
                    ModelError::Parsing(
                        "Failed to parse target scaler range".to_string(),
                        Some(Box::new(e)),
                    )
                })?);
            }
        }

        if y_range.is_some() {
            if y_bounds.is_none() {
                return Err(ModelError::Parsing(
                    "Y range present but not bounds".to_string(),
                    None,
                ));
            }
        } else if y_bounds.is_some() {
            return Err(ModelError::Parsing(
                "Y bounds present but not range".to_string(),
                None,
            ));
        }

        let dir = env::temp_dir().join("collab-fuzz");
        create_dir_all(&dir).map_err(|e| {
            ModelError::Loading(
                format!("Failed to create directory {}", dir.to_string_lossy(),),
                Some(Box::new(e)),
            )
        })?;

        let model_path = Self::store_file(dir.join(format!("{}.model", fuzzer_type)), model_str)?;

        // Make it so the string is null-terminated
        let model_path = CString::new(model_path.as_bytes()).expect("Failed to build CString");
        let model =
            NonNull::new(unsafe { svm_load_model(model_path.as_ptr()) }).ok_or_else(|| {
                ModelError::Loading(
                    format!("Could not load SVM model for {}", fuzzer_type),
                    None,
                )
            })?;

        Ok(Self {
            model,
            features_range,
            features_bounds,
            y_range,
            y_bounds,
        })
    }

    pub fn predict(&self, query: &Query) -> f64 {
        let mut svm_nodes = query.to_svm_nodes();
        for svm_node in svm_nodes.iter_mut() {
            if svm_node.index == -1 {
                break;
            }
            svm_node.value = self.features_range[svm_node.index as usize - 1]
                .scale(svm_node.value, &self.features_bounds);
        }

        unsafe { svm_predict(self.model.as_ptr(), svm_nodes.as_ptr()) }
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe { svm_free_and_destroy_model(&mut self.model.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model() {
        let model = Model::load(FuzzerType::HONGGFUZZ).unwrap();

        assert_eq!(model.y_range, None);
        assert_eq!(model.y_bounds, None);

        assert_eq!(
            model.features_bounds,
            Bounds {
                lower: -1.,
                upper: 1.
            }
        );

        assert_eq!(model.features_range[0].min, 6.);
        assert_eq!(model.features_range[0].max, 1211.);

        assert_eq!(model.features_range[1].min, 3.);
        assert_eq!(model.features_range[1].max, 26.);

        assert_eq!(model.features_range[2].min, 0.);
        assert_eq!(model.features_range[2].max, 64.);

        assert_eq!(model.features_range[3].min, 1.);
        assert_eq!(model.features_range[3].max, 10234.);

        let q = Query {
            oviedo: 206.,
            chain_size: 11.,
            compare_size: 32.,
            instruction_count: 12.,
        };

        let mut svm_nodes = q.to_svm_nodes();
        for svm_node in svm_nodes.iter_mut() {
            if svm_node.index == -1 {
                break;
            }
            svm_node.value = model.features_range[svm_node.index as usize - 1]
                .scale(svm_node.value, &model.features_bounds);
        }

        // NB: because `svm-scale` saves only 6 decimal places, we have slightly different scaled
        // values than reported by the utility; because of this the prediction results need to be
        // checked up to a certain decimal point manually and not with something like
        // `f64::EPSILON`. This affects only testing, the model using scaled values in memory
        // should be more precise than using scaled values from a file as they loose precision.

        assert_eq!(format!("{:.6}", svm_nodes[0].value), "-0.668050");
        assert_eq!(format!("{:.6}", svm_nodes[1].value), "-0.304348");
        assert_eq!(format!("{:.6}", svm_nodes[2].value), "0.000000");
        assert_eq!(format!("{:.6}", svm_nodes[3].value), "-0.997850");

        let prediction = model.predict(&q);

        assert_eq!(
            format!("{:.4}", prediction),
            format!("{:.4}", 1052.406789621172)
        );
    }
}
