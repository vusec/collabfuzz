use serde::Deserialize;

#[derive(Deserialize)]
pub struct DFSanResult {
    basic_block_id: u64,
    terminator_id: u64,
    terminator_tainted: bool,
}

impl DFSanResult {
    pub fn get_basic_block_id(&self) -> u64 {
        self.basic_block_id
    }

    pub fn get_terminator_id(&self) -> u64 {
        self.terminator_id
    }

    pub fn is_tainted(&self) -> bool {
        self.terminator_tainted
    }
}
