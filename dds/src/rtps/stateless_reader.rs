use super::{
    messages::{self, submessages::data::DataSubmessage},
    types::{Guid, GuidPrefix, ENTITYID_UNKNOWN},
};
use crate::transport::{cache_change::CacheChange, reader::ReaderHistoryCache};
use tracing::error;

pub struct RtpsStatelessReader {
    guid: Guid,
    topic_name: String,
    history_cache: Box<dyn ReaderHistoryCache>,
}

impl RtpsStatelessReader {
    pub fn new(guid: Guid, topic_name: String, history_cache: Box<dyn ReaderHistoryCache>) -> Self {
        Self {
            guid,
            topic_name,
            history_cache,
        }
    }

    pub fn guid(&self) -> Guid {
        self.guid
    }

    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    pub fn on_data_submessage_received(
        &mut self,
        data_submessage: &DataSubmessage,
        source_guid_prefix: GuidPrefix,
        source_timestamp: Option<messages::types::Time>,
    ) {
        if data_submessage.reader_id() == ENTITYID_UNKNOWN
            || data_submessage.reader_id() == self.guid.entity_id()
        {
            if let Ok(change) = CacheChange::try_from_data_submessage(
                data_submessage,
                source_guid_prefix,
                source_timestamp,
            ) {
                // Stateless reader behavior. We add the change if the data is correct. No error is printed
                // because all readers would get changes marked with ENTITYID_UNKNOWN
                self.history_cache.add_change(change);
            } else {
                error!("Error converting data submessage to reader cache change. Discarding data")
            }
        }
    }
}
