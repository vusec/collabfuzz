use super::utils::get_artifact_path;
use super::{Pass, PassConfig, PassError, PassType};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub const PASS_NAME: &str = "bb_taint_tracer";

pub struct BBTaintTracerPass {
    executable_path: PathBuf,
    input_file_path: PathBuf,
    output_file_path: PathBuf,
    program_arguments: Vec<String>,
    input_in_stdin: bool,
}

impl BBTaintTracerPass {
    pub fn new(config: PassConfig) -> Result<Self, PassError> {
        let executable_path = get_artifact_path(&config.analysis_artifacts_dir, PASS_NAME)
            .map_err(PassError::FailedToGetBin)?;

        let input_file_name = format!("{}_input", PASS_NAME);
        let input_file_path = config.analysis_input_dir.join(input_file_name);

        let output_file_name = format!("{}_output", PASS_NAME);
        let output_file_path = config.analysis_input_dir.join(output_file_name);

        let mut input_in_stdin = true;
        let program_arguments = config
            .program_arguments
            .iter()
            .map(|arg| {
                if arg == "@@" {
                    input_in_stdin = false;
                    input_file_path.to_string_lossy().into_owned()
                } else {
                    String::from(arg)
                }
            })
            .collect();

        Ok(Self {
            executable_path,
            input_file_path,
            output_file_path,
            program_arguments,
            input_in_stdin,
        })
    }
}

impl Pass for BBTaintTracerPass {
    fn pass_type(&self) -> PassType {
        PassType::BBTaintTracer
    }

    fn process(&self, test_case: &[u8]) -> Result<Vec<u8>, PassError> {
        let mut command = Command::new(&self.executable_path);
        command
            .args(&self.program_arguments)
            .env("DFSAN_OPTIONS", "strict_data_dependencies=0")
            .env("TRACER_ENABLE_FILE_OUTPUT", "TRUE")
            .env("TRACER_OUTPUT_FILE", &self.output_file_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        log::debug!(
            "Launching instrumented binary: {}",
            self.executable_path.display()
        );

        let mut child = if self.input_in_stdin {
            log::debug!("Piping stdin");
            command.stdin(Stdio::piped());
            let mut child = command.spawn().expect("Command failed to start");
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(test_case)
                .expect("Failed to write to stdin");

            child
        } else {
            log::debug!("Writing test case to file");
            command.env("TRACER_INPUT_FILE", &self.input_file_path);
            let mut input_file = File::create(&self.input_file_path).expect("Could not input file");
            input_file
                .write_all(test_case)
                .expect("Could not write input file");

            command.spawn().expect("Command failed to start")
        };

        // TODO: Wait with timeout to avoid hanging
        log::debug!("Waiting for child process to exit");
        child.wait().expect("Could not wait the child process");

        log::debug!("Reading output file: {}", self.output_file_path.display());
        let mut output_file =
            File::open(&self.output_file_path).expect("Could not open output file");
        let mut output = Vec::new();
        output_file
            .read_to_end(&mut output)
            .expect("Could not read output file");

        Ok(output)
    }
}
