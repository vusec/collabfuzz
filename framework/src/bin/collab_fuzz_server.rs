use collab_fuzz::Config;
use collab_fuzz::PassConfig;
use collab_fuzz::SchedulerType;

use std::env;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::time::Duration;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    #[structopt(
        short,
        long,
        help = "Directory containing the seed test cases (NOW UNUSED)"
    )]
    input_dir: PathBuf,

    #[structopt(short, long, help = "Output directory")]
    output_dir: PathBuf,

    #[structopt(
        short,
        long,
        help = "Scheduler to be used for this session",
        default_value = "enfuzz"
    )]
    scheduler: SchedulerType,

    #[structopt(
        short,
        long,
        default_value = "60",
        help = "Maximum time interval between scheduler activations"
    )]
    refresh: u64,

    #[structopt(
        short,
        long,
        default_value = "analysis_binaries",
        help = "Folder containing the target binaries"
    )]
    analysis_binaries_dir: PathBuf,

    #[structopt(multiple(true), last(true))]
    target_arguments: Vec<String>,
}

fn main() {
    env_logger::init();

    let opts = Opts::from_args();

    if !opts.analysis_binaries_dir.is_dir() {
        println!(
            "Analysis directory not found: {}",
            opts.analysis_binaries_dir.display()
        );
        process::exit(1);
    }

    let config = Config {
        name: "collab".to_string(),
        scheduler: opts.scheduler,
        input_dir: opts.input_dir,
        output_dir: opts.output_dir,
        uri_listener: env::var("URI_LISTENER")
            .unwrap_or_else(|_| "ipc:///tmp/server-pull.ipc".to_string()),
        uri_control: env::var("URI_CONTROL")
            .unwrap_or_else(|_| "ipc:///tmp/server-ctrl.ipc".to_string()),
        uri_scheduler: env::var("URI_SCHEDULER")
            .unwrap_or_else(|_| "ipc:///tmp/server-push.ipc".to_string()),
        uri_analysis: env::var("URI_ANALYSIS")
            .unwrap_or_else(|_| "ipc:///tmp/server-analysis.ipc".to_string()),
        pass_config: PassConfig {
            program_arguments: opts.target_arguments,
            analysis_artifacts_dir: opts.analysis_binaries_dir,
            analysis_input_dir: env::temp_dir().join("collab_fuzz_analysis"),
        },
        refresh: Duration::from_secs(opts.refresh),
    };

    let (kill_tx, kill_rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        log::info!("Received Ctrl-C");

        // Send kill signal
        kill_tx.send(()).unwrap();
    })
    .expect("Could not set Ctrl-C handler");

    match collab_fuzz::start(&config, kill_rx) {
        Ok(_) => log::info!("Exiting main thread"),
        Err(f) => log::warn!("Error starting server: {}", f),
    };
}
