use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn get_artifact_path(
    analysis_artifacts_dir: &Path,
    artifact_suffix: &str,
) -> io::Result<PathBuf> {
    if !analysis_artifacts_dir.is_dir() {
        let error_message = format!(
            "analysis_artifacts is not a directory: {}",
            analysis_artifacts_dir.display()
        );
        return Err(io::Error::new(io::ErrorKind::InvalidInput, error_message));
    }

    for entry_res in fs::read_dir(analysis_artifacts_dir)? {
        let entry = entry_res?;
        if entry
            .file_name()
            .to_str()
            .unwrap()
            .ends_with(artifact_suffix)
        {
            return Ok(entry.path());
        }
    }

    let error_message = format!("Could not find artifact with suffix: {}", artifact_suffix);
    Err(io::Error::new(io::ErrorKind::NotFound, error_message))
}
