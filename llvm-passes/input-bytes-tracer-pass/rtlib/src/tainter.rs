use crate::dfsan::{dfsan_create_label, dfsan_label, dfsan_set_label, size_t};
use once_cell::sync::OnceCell;
use snafu::{ensure, ResultExt, Snafu};
use std::cmp::{max, min};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::os::raw::c_void;
use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, MutexGuard};

static TAINTER: OnceCell<Mutex<Tainter>> = OnceCell::new();

#[derive(Default)]
pub struct Tainter {
    canonical_path_opt: Option<PathBuf>,
    tainted_range: ByteRange,
    tainted_file_descriptors: BTreeSet<RawFd>,
    offsets_to_labels: BTreeMap<usize, dfsan_label>,
    labels_to_offsets: BTreeMap<dfsan_label, usize>,
}

impl Tainter {
    pub fn global() -> Option<MutexGuard<'static, Self>> {
        let lock = TAINTER.get()?;
        Some(lock.lock().unwrap())
    }

    pub fn is_enabled(&self) -> bool {
        self.canonical_path_opt.is_some()
    }

    pub fn trace_open(&mut self, fd: RawFd, current_path: impl AsRef<Path>) {
        let tainted_path = if let Some(tainted_path) = self.canonical_path_opt.as_ref() {
            tainted_path
        } else {
            // Instrumentation disabled
            return;
        };

        // Canonicalize may fail. In that case, ignore the error, open should have failed anyway.
        let canonical_current_path =
            if let Ok(canonical_current_path) = current_path.as_ref().canonicalize() {
                canonical_current_path
            } else {
                return;
            };

        if tainted_path == &canonical_current_path {
            log::debug!(
                "Matching file detected: {}",
                canonical_current_path.display()
            );
            log::debug!("File descriptor: {}", fd);
            self.tainted_file_descriptors.insert(fd);
        }
    }

    fn get_or_create_label(&mut self, offset: usize) -> dfsan_label {
        if let Some(label) = self.offsets_to_labels.get(&offset) {
            *label
        } else {
            log::trace!("Creating label for offset: {}", offset);

            // Trust dfsan to create the label correctly, or die
            let new_label = unsafe { dfsan_create_label(ptr::null(), ptr::null_mut()) };
            self.offsets_to_labels.insert(offset, new_label);
            self.labels_to_offsets.insert(new_label, offset);
            new_label
        }
    }

    pub fn trace_read(&mut self, fd: RawFd, addr: *mut c_void, offset: usize, size: usize) {
        if !self.tainted_file_descriptors.contains(&fd) {
            // Not target file, zero out all labels
            unsafe { dfsan_set_label(0, addr, size as size_t) };
            return;
        }

        let range_start = max(self.tainted_range.start, offset);
        let range_end = min(self.tainted_range.end, offset + size);
        if range_start >= range_end {
            // Not in range, zero out all labels
            unsafe { dfsan_set_label(0, addr, size as size_t) };
            return;
        }

        let overlap_range = ByteRange::new(range_start, range_end);

        // Zero out labels before the tainted interval
        let size_before_tainted = overlap_range.start - offset;
        if size_before_tainted > 0 {
            log::debug!(
                "Zeroing labels before tainted interval: {}",
                size_before_tainted
            );
            unsafe { dfsan_set_label(0, addr, size_before_tainted as size_t) };
        }

        // Set labels for the tainted interval
        log::debug!("Tainting read with bytes: {}", overlap_range);
        for idx in &overlap_range {
            let idx_label = self.get_or_create_label(idx);
            // addr + idx is guaranteed to stay within the range that was read. If it does not,
            // there is a bug in one of the wrappers
            unsafe { dfsan_set_label(idx_label, addr.add(idx), 1) };
        }

        // Zero out labels after the tainted interval
        let size_after_tainted = offset + size - overlap_range.end;
        if size_after_tainted > 0 {
            log::debug!(
                "Zeroing labels after tainted interval: {}",
                size_after_tainted
            );
            unsafe { dfsan_set_label(0, addr.add(overlap_range.end), size_after_tainted as u64) };
        }
    }

    pub fn get_byte_label(&mut self, fd: RawFd, offset: usize) -> Option<dfsan_label> {
        if !self.tainted_file_descriptors.contains(&fd) {
            // Not target file
            return None;
        }

        if !self.tainted_range.contains_byte(offset) {
            // Not in range
            return None;
        }

        Some(self.get_or_create_label(offset))
    }

    pub fn trace_close(&mut self, fd: RawFd) {
        if self.tainted_file_descriptors.remove(&fd) {
            log::debug!("Removed file descriptor: {}", fd);
        }
    }

    pub fn get_label_to_offsets_map(&self) -> &BTreeMap<dfsan_label, usize> {
        &self.labels_to_offsets
    }
}

pub struct TainterBuilder {
    tainted_path_opt: Option<PathBuf>,
    tainted_range: Option<ByteRange>,
}

impl TainterBuilder {
    pub fn new() -> Self {
        Self {
            tainted_path_opt: None,
            tainted_range: None,
        }
    }

    pub fn taint_file(&mut self, file_path: PathBuf) -> &mut Self {
        self.tainted_path_opt = Some(file_path);
        self
    }

    pub fn taint_range(&mut self, tainted_range: ByteRange) -> &mut Self {
        self.tainted_range = Some(tainted_range);
        self
    }

    fn deduce_range_from_file(tainted_path: impl AsRef<Path>) -> Result<ByteRange, TainterError> {
        let tainted_path_metadata = fs::metadata(tainted_path).context(UnknownFileSize)?;

        log::debug!(
            "Range deduced from file size: {}",
            tainted_path_metadata.len()
        );

        Ok(ByteRange {
            start: 0,
            end: tainted_path_metadata.len() as usize,
        })
    }

    pub fn build_global(self) -> Result<(), TainterError> {
        let tainter = if let Some(tainted_path) = self.tainted_path_opt {
            let canonical_path = tainted_path.canonicalize().context(InvalidTaintPath)?;

            log::info!("Tainted file: {}", canonical_path.display());

            let file_range = Self::deduce_range_from_file(&canonical_path)?;
            let tainted_range = if let Some(user_range) = self.tainted_range {
                if file_range.contains_range(&user_range) {
                    user_range
                } else {
                    return InvalidRange {
                        file_range,
                        user_range,
                    }
                    .fail();
                }
            } else {
                file_range
            };

            log::info!("Tainted range: {}", tainted_range);

            Tainter {
                canonical_path_opt: Some(canonical_path),
                tainted_range,
                ..Default::default()
            }
        } else {
            log::info!("Tainter disabled");
            if self.tainted_range.is_some() {
                log::warn!("Tainted range ignored");
            }

            Default::default()
        };

        ensure!(TAINTER.set(Mutex::new(tainter)).is_ok(), AlreadyExists);

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ByteRange {
    start: usize,
    end: usize,
}

impl ByteRange {
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end);

        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains_byte(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }

    pub fn contains_range(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

impl IntoIterator for &ByteRange {
    type Item = usize;
    type IntoIter = std::ops::Range<usize>;

    fn into_iter(self) -> Self::IntoIter {
        std::ops::Range {
            start: self.start,
            end: self.end,
        }
    }
}

impl fmt::Display for ByteRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {})", self.start, self.end)
    }
}

#[derive(Debug, Snafu)]
pub enum TainterError {
    #[snafu(display("Cannot canonicalize tainted file path: {}", source))]
    InvalidTaintPath { source: std::io::Error },
    #[snafu(display("Cannot retrieve tainted file size: {}", source))]
    UnknownFileSize { source: std::io::Error },
    #[snafu(display(
        "Range outsize of file bounds {} has been specified: {}",
        file_range,
        user_range
    ))]
    InvalidRange {
        file_range: ByteRange,
        user_range: ByteRange,
    },
    #[snafu(display("Tainter has already been instantiated"))]
    AlreadyExists,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_empty() {
        let range = ByteRange::new(2, 2);

        assert!(range.is_empty());
    }

    #[test]
    fn range_contains_byte() {
        let range = ByteRange::new(1, 3);

        assert!(!range.contains_byte(0));
        assert!(range.contains_byte(1));
        assert!(range.contains_byte(2));
        assert!(!range.contains_byte(3));
    }

    #[test]
    fn range_contains_range() {
        let range = ByteRange::new(1, 3);
        let range_small = ByteRange::new(2, 3);
        let range_big = ByteRange::new(0, 4);

        assert!(range.contains_range(&range_small));
        assert!(range.contains_range(&range));
        assert!(!range.contains_range(&range_big));
    }
}
