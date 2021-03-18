use crate::fuzzers::FuzzersHandler;
use crate::logger::Logger;
use crate::storage::Storage;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::analysis::{AnalysisType, GlobalState};
use std::collections::HashMap;

pub type ServerResult<T> = Result<T, Box<dyn Error>>;
pub type SResult = ServerResult<()>;
pub type SharedStorage = Arc<Mutex<Storage>>;
pub type SharedFuzzersHandler = Arc<Mutex<FuzzersHandler>>;
pub type SharedLogger = Arc<Mutex<Logger>>;

pub type SharedGlobalStates = Arc<Mutex<HashMap<AnalysisType, Box<dyn GlobalState>>>>;

#[allow(dead_code)]
pub type ConditionalId = u32;
#[allow(dead_code)]
pub type ConditionalEntry = (ConditionalId, Seed);

// impl Fuzzer {
//     pub fn new(id: FuzzerId, name: String) -> Fuzzer {
//         Fuzzer {
//             id,
//             class_id: DEFAULT_FUZZER_CLASS,
//             name,
//             last_contact: Instant::now(),
//             status: FuzzerStatus::default(),
//         }
//     }

//     #[allow(dead_code)]
//     pub fn update_contact(&mut self) {
//         self.last_contact = Instant::now();
//     }

//     pub fn get_id(&self) -> FuzzerId {
//         self.id.clone()
//     }

//     pub fn get_status(&self) -> FuzzerStatus {
//         self.status.clone()
//     }

//     pub fn set_status(&mut self, status: FuzzerStatus) {
//         self.status = status;
//     }
// }

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum SeedType {
    NORMAL,
    CRASH,
    HANG,
}

pub fn get_test_case_types() -> Vec<SeedType> {
    vec![SeedType::NORMAL, SeedType::CRASH, SeedType::HANG]
}

impl fmt::Display for SeedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeedType::NORMAL => write!(f, "normal"),
            SeedType::CRASH => write!(f, "crash"),
            SeedType::HANG => write!(f, "hang"),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Seed {
    pub name: String,
    data: Vec<u8>,
    seed_type: SeedType,
}

impl Seed {
    pub fn new_from_file(path: &PathBuf) -> Result<Seed, Box<dyn Error>> {
        let mut buf = vec![];
        let stem = path.file_stem().unwrap_or_default().to_str();
        File::open(path)?.read_to_end(&mut buf)?;
        let _name = stem.unwrap_or_default();
        Ok(Seed::new_from_buf(&buf))
    }

    pub fn new_from_buf(buf: &[u8]) -> Seed {
        let mut hasher = Sha256::new();
        hasher.input(buf.to_vec());
        let result = hasher.result();

        Seed {
            name: format!("{:x}", result),
            data: buf.to_vec(),
            seed_type: SeedType::NORMAL,
        }
    }

    pub fn to_bytes(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn get_type(&self) -> SeedType {
        self.seed_type
    }

    pub fn set_type(&mut self, t: SeedType) {
        self.seed_type = t;
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn write_to_disk(&self, path: &Path) -> io::Result<()> {
        if let Ok(mut f) = File::create(path.join(self.get_name())) {
            let mut rd = io::Cursor::new(self.to_bytes());
            io::copy(&mut rd, &mut f)?;
        }
        Ok(())
    }
}
