use std::collections::{HashMap,  VecDeque};

use crate::structure::HistoryCache;
use crate::structure::CacheChange;
use crate::serialized_payload::ParameterList;
use crate::types::{ChangeKind, InstanceHandle, Locator, ReliabilityKind, SequenceNumber, TopicKind, GUID, };
use crate::messages::RtpsSubmessage;
use crate::messages::message_sender::Sender;
use super::reader_locator::ReaderLocator;

use rust_dds_interface::qos::DataWriterQos;

pub struct StatelessWriter {
    /// Entity base class (contains the GUID)
    guid: GUID,
    // entity: Entity,

    // Endpoint base class:
    /// Used to indicate whether the Endpoint supports instance lifecycle management operations. Indicates whether the Endpoint is associated with a DataType that has defined some fields as containing the DDS key.
    topic_kind: TopicKind,
    /// The level of reliability supported by the Endpoint.
    reliability_level: ReliabilityKind,

    // The stateless writer does not receive messages so these fields are left out

    /// List of unicast locators (transport, address, port combinations) that can be used to send messages to the Endpoint. The list may be empty
    // unicast_locator_list: Vec<Locator>,
    /// List of multicast locators (transport, address, port combinations) that can be used to send messages to the Endpoint. The list may be empty.
    // multicast_locator_list: Vec<Locator>,

    
    last_change_sequence_number: SequenceNumber,
    writer_cache: HistoryCache,
    data_max_sized_serialized: Option<i32>,

    reader_locators: HashMap<Locator, ReaderLocator>,
}

impl StatelessWriter {
    pub fn new(
        guid: GUID,
        topic_kind: TopicKind,
        writer_qos: &DataWriterQos
    ) -> Self {
        StatelessWriter {
            guid,
            topic_kind,
            reliability_level: ReliabilityKind::BestEffort,
            last_change_sequence_number: 0,
            writer_cache: HistoryCache::new(&writer_qos.resource_limits),
            data_max_sized_serialized: None,
            reader_locators: HashMap::new(),
        }
    }

    pub fn new_change(
        &mut self,
        kind: ChangeKind,
        data: Option<Vec<u8>>,
        inline_qos: Option<ParameterList>,
        handle: InstanceHandle,
    ) -> CacheChange {
        self.last_change_sequence_number += 1;

        CacheChange::new(
            kind,
            self.guid,
            handle,
            self.last_change_sequence_number,
            data,
            inline_qos,
        )
    }

    pub fn writer_cache(&self) -> &HistoryCache {
        &self.writer_cache
    }

    pub fn reader_locator_add(&mut self, a_locator: Locator) {
        self.reader_locators.insert(a_locator, ReaderLocator::new(a_locator, self.guid.entity_id(), false /*expects_inline_qos*/));
    }

    pub fn reader_locator_remove(&mut self, a_locator: &Locator) {
        self.reader_locators.remove(a_locator);
    }

    pub fn unsent_changes_reset(&mut self) {
        for (_, rl) in self.reader_locators.iter_mut() {
            rl.unsent_changes_reset();
        }
    }

    pub fn run(&mut self) {
        let last_change_sequence_number = self.last_change_sequence_number;
        for (_, reader_locator) in self.reader_locators.iter_mut() {
            reader_locator.run(&self.writer_cache, last_change_sequence_number);
        }
    }
}

impl Sender for StatelessWriter {
    fn pop_send_messages(&mut self) -> Vec<(Vec<Locator>, VecDeque<RtpsSubmessage>)> {
        self.reader_locators.iter_mut()
            .filter_map(|(_, reader_locator)| reader_locator.pop_send_messages())
            .collect()
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::constants::*;
    use crate::types::*;

    #[test]
    fn test_writer_new_change() {
        let writer_qos = DataWriterQos::default();
        let mut writer = StatelessWriter::new(
            GUID::new([0; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER),
            TopicKind::WithKey,
            &writer_qos
        );

        let cache_change_seq1 = writer.new_change(
            ChangeKind::Alive,
            Some(vec![1, 2, 3]), 
            None,                
            [1; 16],             
        );

        let cache_change_seq2 = writer.new_change(
            ChangeKind::NotAliveUnregistered,
            None,    
            None,    
            [1; 16], 
        );

        assert_eq!(cache_change_seq1.sequence_number(), 1);
        assert_eq!(cache_change_seq1.change_kind(), &ChangeKind::Alive);
        assert_eq!(cache_change_seq1.inline_qos().len(), 0);
        assert_eq!(cache_change_seq1.instance_handle(), &[1; 16]);

        assert_eq!(cache_change_seq2.sequence_number(), 2);
        assert_eq!(
            cache_change_seq2.change_kind(),
            &ChangeKind::NotAliveUnregistered
        );
        assert_eq!(cache_change_seq2.inline_qos().len(), 0);
        assert_eq!(cache_change_seq2.instance_handle(), &[1; 16]);
    }

    #[test]
    fn test_best_effort_stateless_writer_run() {
        // Create the stateless writer
        let writer_qos = DataWriterQos::default();

        let mut stateless_writer = StatelessWriter::new(
            GUID::new([0; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER),
            TopicKind::WithKey,
            &writer_qos
        );

        // Add two locators
        let locator1 = Locator::new(0, 7400, [1; 16]);
        let locator2 = Locator::new(0, 7500, [2; 16]);
        stateless_writer.reader_locator_add(locator1);
        stateless_writer.reader_locator_add(locator2);

        // Add two changes to the cache change
        let cache_change_seq1 = stateless_writer.new_change(
            ChangeKind::Alive,
            Some(vec![1, 2, 3]), 
            None,                
            [1; 16],             
        );

        let cache_change_seq2 = stateless_writer.new_change(
            ChangeKind::Alive,
            Some(vec![4, 5, 6]), 
            None,                
            [1; 16],             
        );

        stateless_writer.writer_cache().add_change(cache_change_seq1).unwrap();
        stateless_writer.writer_cache().add_change(cache_change_seq2).unwrap();

        stateless_writer.run();

        let send_messages = stateless_writer.pop_send_messages();
        // assert!(writer.pop_send_messages().is_none());

        // if let RtpsSubmessage::Data(data_message_1) = &messages[0] {
        //     assert_eq!(data_message_1.reader_id(), ENTITYID_UNKNOWN);
        //     assert_eq!(data_message_1.writer_id(), ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER);
        //     assert_eq!(data_message_1.writer_sn(), 1);
        //     assert_eq!(data_message_1.serialized_payload(), &vec![1, 2, 3]);
        // } else {
        //     panic!("Wrong message type");
        // };

        // if let RtpsSubmessage::Data(data_message_2) = &messages[1] {
        //     assert_eq!(data_message_2.reader_id(), ENTITYID_UNKNOWN);
        //     assert_eq!(data_message_2.writer_id(), ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER);
        //     assert_eq!(data_message_2.writer_sn(), 2);
        //     assert_eq!(data_message_2.serialized_payload(), &vec![4, 5, 6]);
        // } else {
        //     panic!("Wrong message type");
        // };

        // // Test that nothing more is sent after the first time
        // writer.run();
        // assert!(writer.pop_send_messages().is_none());
    }
}