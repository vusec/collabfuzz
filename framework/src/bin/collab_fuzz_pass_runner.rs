use collab_fuzz::{PassConfig, PassType};
use std::env;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    #[structopt(short, long, help = "Pass that should be run on the input file")]
    pass: PassType,
    #[structopt(short, long, help = "Path to file that should be analyzed")]
    input_path: PathBuf,
    #[structopt(
        short,
        long,
        help = "Path to file in which the output should be written"
    )]
    output_path: PathBuf,

    #[structopt(
        short,
        long,
        default_value = "analysis_binaries",
        help = "Folder containing the instrumented target binaries"
    )]
    analysis_binaries_dir: PathBuf,
    #[structopt(multiple(true), last(true))]
    target_arguments: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let opts = Opts::from_args();

    let config = PassConfig {
        program_arguments: opts.target_arguments,
        analysis_artifacts_dir: opts.analysis_binaries_dir,
        analysis_input_dir: env::temp_dir().join("collab_fuzz_runner"),
    };

    fs::create_dir_all(&config.analysis_input_dir)?;

    let pass = opts.pass.get_pass(config)?;

    let input_test_case = fs::read(opts.input_path)?;

    let pass_output = pass.process(&input_test_case)?;

    fs::write(opts.output_path, pass_output)?;

    Ok(())
}
