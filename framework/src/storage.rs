use crate::types::SeedType;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::path::PathBuf;

const QUEUE_DIR: &str = "queue";
const HANG_DIR: &str = "hangs";
const CRASH_DIR: &str = "crashes";

pub struct TestCase {
    pub case_type: SeedType,
    pub content: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TestCaseHandle {
    hash: String,
    case_type: SeedType,
}

impl TestCaseHandle {
    pub fn get_unique_id(&self) -> &String {
        &self.hash
    }

    pub fn get_type(&self) -> SeedType {
        self.case_type
    }
}

pub enum StoreResult {
    New,
    AlreadyExists,
}

pub struct Storage {
    // TODO: It is possible to add a cache for the content of the files so that there is less disk
    // activity. Do this only if actually needed though.
    output_path: PathBuf,
    ids_to_handles: HashMap<String, TestCaseHandle>,
}

impl Storage {
    pub fn new(output_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        create_dir_all(&output_path)?;
        create_dir_all(output_path.join(QUEUE_DIR))?;
        create_dir_all(output_path.join(CRASH_DIR))?;
        create_dir_all(output_path.join(HANG_DIR))?;

        Ok(Storage {
            output_path,
            ids_to_handles: HashMap::new(),
        })
    }

    pub fn store(&mut self, test_case: TestCase) -> (TestCaseHandle, StoreResult) {
        let hash = Sha256::digest(&test_case.content);
        let hash = format!("{:x}", hash);

        let test_case_handle = TestCaseHandle {
            hash: hash.clone(),
            case_type: test_case.case_type,
        };

        let test_case_path = match test_case.case_type {
            SeedType::NORMAL => self.output_path.join(QUEUE_DIR).join(&hash),
            SeedType::CRASH => self.output_path.join(CRASH_DIR).join(&hash),
            SeedType::HANG => self.output_path.join(HANG_DIR).join(&hash),
        };

        log::debug!("Trying to create test case: {}", test_case_path.display());
        let open_result = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(test_case_path);

        let store_result = match open_result {
            Ok(mut test_case_file) => {
                test_case_file
                    .write_all(&test_case.content)
                    .expect("Could not write test case!");
                self.ids_to_handles.insert(hash, test_case_handle.clone());

                log::debug!("New test case stored");

                StoreResult::New
            }
            Err(error) => match error.kind() {
                ErrorKind::AlreadyExists => {
                    // Duplicate test cases should not be written twice, their handle is simply
                    // returned. However, the error should be signaled back to the caller.
                    log::debug!("Test case already stored, returning existing handle.");

                    StoreResult::AlreadyExists
                }
                _ => panic!("Could not create test case: {}", error),
            },
        };

        (test_case_handle, store_result)
    }

    pub fn retrieve(&self, test_case_handle: &TestCaseHandle) -> TestCase {
        // TODO: Check against existing handles in ids_to_handles first, its faster.

        let test_case_path = match test_case_handle.case_type {
            SeedType::NORMAL => self
                .output_path
                .join(QUEUE_DIR)
                .join(&test_case_handle.hash),
            SeedType::CRASH => self
                .output_path
                .join(CRASH_DIR)
                .join(&test_case_handle.hash),
            SeedType::HANG => self.output_path.join(HANG_DIR).join(&test_case_handle.hash),
        };

        let mut test_case_file = File::open(test_case_path).expect("Invalid file handle");

        let mut content = Vec::new();
        test_case_file
            .read_to_end(&mut content)
            .expect("Could not read test case file");

        TestCase {
            content,
            case_type: test_case_handle.case_type,
        }
    }

    pub fn handle_from_id(&self, id: &str) -> Option<TestCaseHandle> {
        self.ids_to_handles.get(id).cloned()
    }
}

#[cfg(test)]
impl TestCaseHandle {
    pub fn get_fake_handle(hash: &str) -> Self {
        TestCaseHandle {
            hash: String::from(hash),
            case_type: SeedType::NORMAL,
        }
    }
}
