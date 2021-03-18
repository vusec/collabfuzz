use crate::protos::{SeedMsg, SeedMsg_SeedType};
use crate::types::{Seed, SeedType};
// use std::error::Error;
// use std::fs;
// use std::path::{Path, PathBuf};

//pub fn get_input_seeds(dir: &Path) -> io::Result<Vec<Seed>> {
//pub fn get_input_seeds(dir: &Path) -> Result<Vec<Seed>, Box<dyn Error>> {
//    log::info!("READING from input dir {:?}", dir);
//    let seeds = fs::read_dir(dir)?
//        .filter_map(|entry| entry.ok())
//        .filter(|entry| entry.metadata().unwrap().len() > 0)
//        .filter_map(|e| e.path().to_str().and_then(|s| Some(String::from(s))))
//        .map(|s| Seed::new_from_file(&PathBuf::from(s)))
//        .collect();
//    //.collect::<Vec<Seed>>();

//    return seeds;
//}

impl From<Seed> for SeedMsg {
    fn from(seed: Seed) -> Self {
        let mut m = SeedMsg::new();
        let v = seed.to_bytes().clone();
        m.set_content(v);
        m.set_field_type(SeedMsg_SeedType::from(seed.get_type()));
        m.set_fuzzer_id(format!("{:03}", 1));
        m.set_id(seed.get_name());
        m
    }
}

/*
impl From<ScheduleJob> for SeedMsg {
    fn from(job: ScheduleJob) -> Self {
        let mut m = SeedMsg::new();
        let fuzzer_id = job.0;
        let seed = &job.1[0];
        let v = seed.to_bytes().clone();
        m.set_content(v);
        m.set_field_type(SeedMsg_SeedType::from(seed.get_type()));
        m.set_fuzzer_id(format!("{:03}", fuzzer_id.0));
        m.set_id(seed.get_name());
        m
    }
}
*/

impl From<SeedMsg> for Seed {
    fn from(msg: SeedMsg) -> Self {
        let _name = msg.get_id().to_string();

        // Seed::new_from_buf(name, &msg.get_content().to_vec())
        Seed::new_from_buf(&msg.get_content().to_vec())
    }
}

impl From<SeedMsg_SeedType> for SeedType {
    fn from(s: SeedMsg_SeedType) -> Self {
        match s {
            SeedMsg_SeedType::NORMAL => SeedType::NORMAL,
            SeedMsg_SeedType::CRASH => SeedType::CRASH,
            SeedMsg_SeedType::HANG => SeedType::HANG,
        }
    }
}

impl From<SeedType> for SeedMsg_SeedType {
    fn from(s: SeedType) -> Self {
        match s {
            SeedType::NORMAL => SeedMsg_SeedType::NORMAL,
            SeedType::CRASH => SeedMsg_SeedType::CRASH,
            SeedType::HANG => SeedMsg_SeedType::HANG,
        }
    }
}
