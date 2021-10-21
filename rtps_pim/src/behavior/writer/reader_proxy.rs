use crate::structure::types::{EntityId, Guid, SequenceNumber};

pub struct RtpsReaderProxy<L> {
    pub remote_reader_guid: Guid,
    pub remote_group_entity_id: EntityId,
    pub unicast_locator_list: L,
    pub multicast_locator_list: L,
    pub expects_inline_qos: bool,
}

impl<L> RtpsReaderProxy<L> {
    pub fn new(
        remote_reader_guid: Guid,
        remote_group_entity_id: EntityId,
        unicast_locator_list: L,
        multicast_locator_list: L,
        expects_inline_qos: bool,
    ) -> Self {
        Self {
            remote_reader_guid,
            remote_group_entity_id,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
        }
    }
}

pub trait RtpsReaderProxyOperations {
    type SequenceNumberVector;

    fn acked_changes_set(&mut self, committed_seq_num: SequenceNumber);
    fn next_requested_change(&mut self) -> Option<SequenceNumber>;
    fn next_unsent_change(&mut self) -> Option<SequenceNumber>;
    fn unsent_changes(&self) -> Self::SequenceNumberVector;
    fn requested_changes(&self) -> Self::SequenceNumberVector;
    fn requested_changes_set(&mut self, req_seq_num_set: Self::SequenceNumberVector);
    fn unacked_changes(&self) -> Self::SequenceNumberVector;
}
