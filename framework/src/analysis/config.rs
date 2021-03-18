use std::path::PathBuf;

// TODO: Pass configuration should be built based on a configuration file or command line
// arguments. The class will also need to be filled with the appropriate content. It should
// be possible to specify a per pass configuration. This could be a map on PassType.

#[derive(Clone, Debug)]
pub struct PassConfig {
    pub program_arguments: Vec<String>,
    pub analysis_artifacts_dir: PathBuf,
    pub analysis_input_dir: PathBuf,
}
