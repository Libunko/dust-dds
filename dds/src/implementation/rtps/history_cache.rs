use crate::{
    implementation::rtps::messages::types::{FragmentNumber, ULong, UShort},
    infrastructure::{instance::InstanceHandle, time::Time},
};

use super::{
    messages::{
        submessage_elements::{Parameter, ParameterList, SequenceNumberSet},
        submessages::{DataFragSubmessage, DataSubmessage, GapSubmessage},
        types::{ParameterId, SerializedPayload},
    },
    types::{ChangeKind, EntityId, Guid, SequenceNumber},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RtpsParameter {
    parameter_id: ParameterId,
    value: Vec<u8>,
}

impl RtpsParameter {
    pub fn new(parameter_id: ParameterId, value: Vec<u8>) -> Self {
        Self {
            parameter_id,
            value,
        }
    }

    pub fn parameter_id(&self) -> ParameterId {
        self.parameter_id
    }

    pub fn value(&self) -> &[u8] {
        self.value.as_ref()
    }
}

pub struct RtpsWriterCacheChange {
    kind: ChangeKind,
    writer_guid: Guid,
    sequence_number: SequenceNumber,
    instance_handle: InstanceHandle,
    timestamp: Time,
    data: Vec<u8>,
    inline_qos: Vec<RtpsParameter>,
}

impl RtpsWriterCacheChange {
    pub fn as_gap_message(&self, reader_id: EntityId) -> GapSubmessage {
        GapSubmessage {
            endianness_flag: true,
            reader_id,
            writer_id: self.writer_guid.entity_id(),
            gap_start: self.sequence_number,
            gap_list: SequenceNumberSet {
                base: self.sequence_number + 1,
                set: vec![],
            },
        }
    }

    pub fn as_data_submessage(&self, reader_id: EntityId) -> DataSubmessage {
        let (data_flag, key_flag) = match self.kind() {
            ChangeKind::Alive => (true, false),
            ChangeKind::NotAliveDisposed | ChangeKind::NotAliveUnregistered => (false, true),
            _ => todo!(),
        };
        let inline_qos = ParameterList {
            parameter: self
                .inline_qos()
                .iter()
                .map(|p| Parameter {
                    parameter_id: p.parameter_id.0,
                    length: p.value.len() as i16,
                    value: p.value.as_ref(),
                })
                .collect(),
        };
        DataSubmessage {
            endianness_flag: true,
            inline_qos_flag: true,
            data_flag,
            key_flag,
            non_standard_payload_flag: false,
            reader_id,
            writer_id: self.writer_guid().entity_id(),
            writer_sn: self.sequence_number(),
            inline_qos,
            serialized_payload: SerializedPayload::new(self.data_value()),
        }
    }

    pub fn as_data_frag_submessages(
        &self,
        max_bytes: usize,
        reader_id: EntityId,
    ) -> Vec<DataFragSubmessage> {
        let data = self.data_value();
        let data_size = ULong::new(data.len() as u32);
        let mut fragment_starting_num = FragmentNumber::new(1);
        const FRAGMENTS_IN_SUBMESSAGE: UShort = UShort::new(1);

        let mut messages = Vec::new();

        let mut data_fragment;
        let mut data_remaining = data;

        while !data_remaining.is_empty() {
            if data_remaining.len() >= max_bytes {
                (data_fragment, data_remaining) = data_remaining.split_at(max_bytes);
            } else {
                data_fragment = data_remaining;
                data_remaining = &[];
            }

            let endianness_flag = true;
            let inline_qos_flag = true;
            let key_flag = match self.kind() {
                ChangeKind::Alive => false,
                ChangeKind::NotAliveDisposed | ChangeKind::NotAliveUnregistered => true,
                _ => todo!(),
            };
            let non_standard_payload_flag = false;
            let writer_id = self.writer_guid().entity_id();
            let writer_sn = self.sequence_number();
            let inline_qos = ParameterList {
                parameter: self
                    .inline_qos()
                    .iter()
                    .map(|p| Parameter {
                        parameter_id: p.parameter_id.0,
                        length: p.value.len() as i16,
                        value: p.value.as_ref(),
                    })
                    .collect(),
            };
            let serialized_payload = SerializedPayload::new(data_fragment);
            let message = DataFragSubmessage {
                endianness_flag,
                inline_qos_flag,
                non_standard_payload_flag,
                key_flag,
                reader_id,
                writer_id,
                writer_sn,
                fragment_starting_num,
                fragments_in_submessage: FRAGMENTS_IN_SUBMESSAGE,
                data_size,
                fragment_size: UShort::new(max_bytes as u16),
                inline_qos,
                serialized_payload,
            };

            messages.push(message);

            fragment_starting_num += FragmentNumber::new(1);
        }
        messages
    }
}

impl RtpsWriterCacheChange {
    pub fn new(
        kind: ChangeKind,
        writer_guid: Guid,
        instance_handle: InstanceHandle,
        sequence_number: SequenceNumber,
        timestamp: Time,
        data_value: Vec<u8>,
        inline_qos: Vec<RtpsParameter>,
    ) -> Self {
        Self {
            kind,
            writer_guid,
            sequence_number,
            instance_handle,
            timestamp,
            data: data_value,
            inline_qos,
        }
    }
}

impl RtpsWriterCacheChange {
    pub fn kind(&self) -> ChangeKind {
        self.kind
    }

    pub fn writer_guid(&self) -> Guid {
        self.writer_guid
    }

    pub fn instance_handle(&self) -> InstanceHandle {
        self.instance_handle
    }

    pub fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub fn timestamp(&self) -> Time {
        self.timestamp
    }

    pub fn data_value(&self) -> &[u8] {
        self.data.as_ref()
    }

    pub fn inline_qos(&self) -> &[RtpsParameter] {
        &self.inline_qos
    }
}

#[derive(Default)]
pub struct WriterHistoryCache {
    changes: Vec<RtpsWriterCacheChange>,
}

impl WriterHistoryCache {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    pub fn changes(&self) -> &[RtpsWriterCacheChange] {
        &self.changes
    }

    pub fn add_change(&mut self, change: RtpsWriterCacheChange) {
        self.changes.push(change);
    }

    pub fn remove_change<F>(&mut self, mut f: F)
    where
        F: FnMut(&RtpsWriterCacheChange) -> bool,
    {
        self.changes.retain(|cc| !f(cc));
    }

    pub fn get_seq_num_min(&self) -> Option<SequenceNumber> {
        self.changes.iter().map(|cc| cc.sequence_number).min()
    }

    pub fn get_seq_num_max(&self) -> Option<SequenceNumber> {
        self.changes.iter().map(|cc| cc.sequence_number).max()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        implementation::rtps::types::GUID_UNKNOWN,
        infrastructure::{instance::HANDLE_NIL, time::TIME_INVALID},
    };

    use super::*;

    #[test]
    fn remove_change() {
        let mut hc = WriterHistoryCache::new();
        let change = RtpsWriterCacheChange::new(
            ChangeKind::Alive,
            GUID_UNKNOWN,
            HANDLE_NIL,
            SequenceNumber::new(1),
            TIME_INVALID,
            vec![],
            vec![],
        );
        hc.add_change(change);
        hc.remove_change(|cc| cc.sequence_number() == SequenceNumber::new(1));
        assert!(hc.changes().is_empty());
    }

    #[test]
    fn get_seq_num_min() {
        let mut hc = WriterHistoryCache::new();
        let change1 = RtpsWriterCacheChange::new(
            ChangeKind::Alive,
            GUID_UNKNOWN,
            HANDLE_NIL,
            SequenceNumber::new(1),
            TIME_INVALID,
            vec![],
            vec![],
        );
        let change2 = RtpsWriterCacheChange::new(
            ChangeKind::Alive,
            GUID_UNKNOWN,
            HANDLE_NIL,
            SequenceNumber::new(2),
            TIME_INVALID,
            vec![],
            vec![],
        );
        hc.add_change(change1);
        hc.add_change(change2);
        assert_eq!(hc.get_seq_num_min(), Some(SequenceNumber::new(1)));
    }

    #[test]
    fn get_seq_num_max() {
        let mut hc = WriterHistoryCache::new();
        let change1 = RtpsWriterCacheChange::new(
            ChangeKind::Alive,
            GUID_UNKNOWN,
            HANDLE_NIL,
            SequenceNumber::new(1),
            TIME_INVALID,
            vec![],
            vec![],
        );
        let change2 = RtpsWriterCacheChange::new(
            ChangeKind::Alive,
            GUID_UNKNOWN,
            HANDLE_NIL,
            SequenceNumber::new(2),
            TIME_INVALID,
            vec![],
            vec![],
        );
        hc.add_change(change1);
        hc.add_change(change2);
        assert_eq!(hc.get_seq_num_max(), Some(SequenceNumber::new(2)));
    }
}
