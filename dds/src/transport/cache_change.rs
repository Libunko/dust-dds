use std::sync::Arc;

use super::types::{ChangeKind, Time};

#[derive(Debug)]
pub struct CacheChange {
    pub kind: ChangeKind,
    pub writer_guid: [u8; 16],
    pub sequence_number: i64,
    pub source_timestamp: Option<Time>,
    pub instance_handle: Option<[u8; 16]>,
    pub data_value: Arc<[u8]>,
}

impl CacheChange {
    pub fn kind(&self) -> ChangeKind {
        self.kind
    }

    pub fn sequence_number(&self) -> i64 {
        self.sequence_number
    }

    pub fn source_timestamp(&self) -> Option<Time> {
        self.source_timestamp
    }

    pub fn data_value(&self) -> &Arc<[u8]> {
        &self.data_value
    }
}
