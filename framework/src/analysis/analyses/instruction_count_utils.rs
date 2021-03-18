use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ConditionCountRecord {
    condition_id: u64,
    condition_count: u64,
}

impl ConditionCountRecord {
    pub fn get_condition_id(&self) -> u64 {
        self.condition_id
    }

    pub fn get_condition_count(&self) -> u64 {
        self.condition_count
    }

    pub fn is_tainted(&self) -> bool {
        // A count of 0 means that the condition is not tainted
        self.condition_count > 0
    }
}
