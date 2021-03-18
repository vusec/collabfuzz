use super::ShadowType;

use std::collections::HashSet;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};

pub struct Tainter {
    input_label: ShadowType,
    input_path_opt: Option<PathBuf>,
    input_file_descriptors: HashSet<RawFd>,
    enable_debug: bool,
}

impl Tainter {
    pub fn new(input_label: ShadowType, input_path_opt: Option<PathBuf>, enable_debug: bool) -> Self {
        let mut input_file_descriptors = HashSet::new();
        if input_path_opt.is_none() {
            // If no input file is provided, assume stdin is used for input
            input_file_descriptors.insert(io::stdin().as_raw_fd());
        }

        Self {
            input_label,
            input_path_opt,
            input_file_descriptors,
            enable_debug,
        }
    }

    pub fn trace_open(&mut self, fd: RawFd, current_path: impl AsRef<Path>) {
        if self.enable_debug {
            eprintln!("tainter: open(\"{}\") -> {}", current_path.as_ref().display(), fd);
        }

        if let Some(input_path) = self.input_path_opt.as_ref() {
            // Canonicalize may fail if the file does not exist. In that case, ignore the error.
            if let Ok(canonical_current_path) = current_path.as_ref().canonicalize() {
                if input_path == &canonical_current_path {
                    self.input_file_descriptors.insert(fd);
                }
            }
        }
    }

    pub fn trace_close(&mut self, fd: RawFd) {
        if self.enable_debug {
            eprintln!("tainter: close({})", fd);
        }

        self.input_file_descriptors.remove(&fd);
    }

    pub fn is_input_fd(&self, fd: RawFd) -> bool {
        let is_input = self.input_file_descriptors.contains(&fd);

        if self.enable_debug {
            if is_input {
                eprintln!("tainter: input file matched: {}", fd);
            } else {
                eprintln!("tainter: input file not matched: {}", fd);
            }
        }

        is_input
    }

    pub fn is_debug_enabled(&self) -> bool {
        self.enable_debug
    }

    pub fn get_input_label(&self) -> ShadowType {
        self.input_label
    }
}
