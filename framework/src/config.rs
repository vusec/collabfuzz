use crate::analysis::PassConfig;
use crate::scheduler::SchedulerType;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub name: String,
    pub scheduler: SchedulerType,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub uri_listener: String,
    pub uri_control: String,
    pub uri_scheduler: String,
    pub uri_analysis: String,
    pub pass_config: PassConfig,
    pub refresh: Duration,
}
