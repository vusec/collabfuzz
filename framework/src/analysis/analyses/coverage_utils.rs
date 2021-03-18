use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct EdgeRecord {
    source: u64,
    target: u64,
    count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Edge {
    source: u64,
    target: u64,
}

impl Edge {
    pub fn get_source(&self) -> u64 {
        self.source
    }

    pub fn get_target(&self) -> u64 {
        self.target
    }
}

impl From<EdgeRecord> for Edge {
    fn from(edge_record: EdgeRecord) -> Self {
        Edge {
            source: edge_record.source,
            target: edge_record.target,
        }
    }
}

#[cfg(test)]
impl Edge {
    pub fn new(source: u64, target: u64) -> Self {
        Edge { source, target }
    }
}
