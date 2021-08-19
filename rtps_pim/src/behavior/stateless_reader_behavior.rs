use crate::{
    messages::{submessage_elements::Parameter, submessages::DataSubmessage},
    structure::{
        types::{ChangeKind, Guid, GuidPrefix, ENTITYID_UNKNOWN},
        RtpsCacheChange, RtpsEntity, RtpsHistoryCache,
    },
};

use super::reader::reader::RtpsReader;

pub trait StatelessReaderBehavior<P> {
    fn receive_data(&mut self, source_guid_prefix: GuidPrefix, data: &DataSubmessage<P>);
}

impl<'a, 'b, T, P> StatelessReaderBehavior<P> for T
where
    T: RtpsReader + RtpsEntity,
    T::HistoryCacheType: RtpsHistoryCache,
    P: AsRef<[Parameter<'a>]>,
{
    fn receive_data(&mut self, source_guid_prefix: GuidPrefix, data: &DataSubmessage<P>) {
        let reader_id = data.reader_id.value;
        if &reader_id == self.guid().entity_id() || reader_id == ENTITYID_UNKNOWN {
            let reader_cache = self.reader_cache_mut();
            let kind = match (data.data_flag, data.key_flag) {
                (true, false) => ChangeKind::Alive,
                (false, true) => ChangeKind::NotAliveDisposed,
                _ => todo!(),
            };
            let writer_guid = Guid::new(source_guid_prefix, data.writer_id.value);
            let instance_handle = 0;
            let sequence_number = data.writer_sn.value;
            let data_value = data.serialized_payload.value;
            let inline_qos = data.inline_qos.parameter.as_ref();
            let a_change = RtpsCacheChange::new(
                kind,
                writer_guid,
                instance_handle,
                sequence_number,
                data_value,
                inline_qos,
            );
            reader_cache.add_change(&a_change);
        }
    }
}

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use crate::{
        messages::submessage_elements::{
            EntityIdSubmessageElement, ParameterListSubmessageElement,
            SequenceNumberSubmessageElement, SerializedDataSubmessageElement,
        },
        structure::types::{
            EntityId, InstanceHandle, SequenceNumber, GUIDPREFIX_UNKNOWN, GUID_UNKNOWN,
        },
    };

    use super::*;

    struct MockCacheChange {
        kind: ChangeKind,
        writer_guid: Guid,
        sequence_number: SequenceNumber,
        instance_handle: InstanceHandle,
        data: [u8; 1],
        inline_qos: (),
    }

    struct MockHistoryCache(Option<MockCacheChange>);

    impl<'a> RtpsHistoryCache for MockHistoryCache {
        fn new() -> Self
        where
            Self: Sized,
        {
            todo!()
        }

        fn add_change(&mut self, change: &RtpsCacheChange) {
            self.0 = Some(MockCacheChange {
                kind: *change.kind(),
                writer_guid: *change.writer_guid(),
                sequence_number: *change.sequence_number(),
                instance_handle: *change.instance_handle(),
                data: [change.data_value()[0].clone()],
                inline_qos: (),
            });
        }

        fn remove_change(&mut self, _seq_num: &crate::structure::types::SequenceNumber) {
            todo!()
        }

        fn get_change(
            &self,
            _seq_num: &crate::structure::types::SequenceNumber,
        ) -> Option<RtpsCacheChange> {
            todo!()
        }

        fn get_seq_num_min(&self) -> Option<crate::structure::types::SequenceNumber> {
            todo!()
        }

        fn get_seq_num_max(&self) -> Option<crate::structure::types::SequenceNumber> {
            todo!()
        }
    }

    struct MockStatelessReader {
        reader_cache: MockHistoryCache,
    }

    impl<'a> RtpsEntity for MockStatelessReader {
        fn guid(&self) -> &Guid {
            &GUID_UNKNOWN
        }
    }

    impl RtpsReader for MockStatelessReader {
        type HistoryCacheType = MockHistoryCache;

        fn heartbeat_response_delay(&self) -> &crate::behavior::types::Duration {
            todo!()
        }

        fn heartbeat_supression_duration(&self) -> &crate::behavior::types::Duration {
            todo!()
        }

        fn reader_cache(&self) -> &Self::HistoryCacheType {
            todo!()
        }

        fn reader_cache_mut(&mut self) -> &mut Self::HistoryCacheType {
            &mut self.reader_cache
        }

        fn expects_inline_qos(&self) -> bool {
            todo!()
        }
    }

    #[test]
    fn receive_data_one_cache_change() {
        let mut stateless_reader = MockStatelessReader {
            reader_cache: MockHistoryCache(None),
        };
        let source_guid_prefix = GUIDPREFIX_UNKNOWN;
        let writer_entity_id = EntityId::new(
            [1, 2, 3],
            crate::structure::types::EntityKind::BuiltInWriterWithKey,
        );
        let message_sequence_number = 1;
        let data = DataSubmessage {
            endianness_flag: false,
            inline_qos_flag: false,
            non_standard_payload_flag: false,
            data_flag: true,
            key_flag: false,
            reader_id: EntityIdSubmessageElement {
                value: ENTITYID_UNKNOWN,
            },
            writer_id: EntityIdSubmessageElement {
                value: writer_entity_id,
            },
            writer_sn: SequenceNumberSubmessageElement {
                value: message_sequence_number,
            },
            serialized_payload: SerializedDataSubmessageElement { value: &[3] },
            inline_qos: ParameterListSubmessageElement { parameter: [], phantom: PhantomData },
        };
        stateless_reader.receive_data(source_guid_prefix, &data);

        if let Some(cache_change) = &stateless_reader.reader_cache.0 {
            assert_eq!(cache_change.kind, ChangeKind::Alive);
            assert_eq!(
                cache_change.writer_guid,
                Guid::new(source_guid_prefix, writer_entity_id)
            );
            assert_eq!(cache_change.sequence_number, message_sequence_number);
            assert_eq!(cache_change.data, [3]);
            assert_eq!(cache_change.inline_qos, ());
            assert_eq!(cache_change.instance_handle, 0);
        } else {
            panic!("Cache change not created")
        }
    }
}
