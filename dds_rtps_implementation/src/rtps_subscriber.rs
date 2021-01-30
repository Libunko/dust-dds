use std::{
    ops::Deref,
    sync::{atomic, Arc, Mutex},
};

use crate::{
    rtps_datareader::RtpsDataReaderInner,
    rtps_topic::RtpsTopic,
    utils::maybe_valid::{MaybeValid, MaybeValidList, MaybeValidRef},
};
use rust_dds_api::{
    domain::domain_participant::{DomainParticipant, DomainParticipantChild, TopicGAT},
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DataReaderQos, SubscriberQos, TopicQos},
        status::{InstanceStateKind, SampleLostStatus, SampleStateKind, StatusMask, ViewStateKind},
    },
    publication::publisher::Publisher,
    subscription::{
        data_reader::{AnyDataReader, DataReader},
        data_reader_listener::DataReaderListener,
        subscriber::Subscriber,
        subscriber_listener::SubscriberListener,
    },
    topic::topic::Topic,
};
use rust_dds_types::{DDSType, InstanceHandle, ReturnCode, ReturnCodes, TopicKind};
use rust_rtps::{
    structure::Group,
    types::{
        constants::{
            ENTITY_KIND_BUILT_IN_READER_WITH_KEY, ENTITY_KIND_USER_DEFINED_READER_NO_KEY,
            ENTITY_KIND_USER_DEFINED_READER_WITH_KEY,
        },
        EntityId, GUID,
    },
};

use super::{
    rtps_datareader::{AnyRtpsReader, RtpsAnyDataReaderRef},
    rtps_participant::RtpsParticipant,
    rtps_topic::AnyRtpsTopic,
};

enum EntityType {
    BuiltIn,
    UserDefined,
}
pub struct RtpsSubscriberInner {
    pub group: Group,
    pub reader_list: MaybeValidList<Box<dyn AnyRtpsReader>>,
    pub reader_count: atomic::AtomicU8,
    pub default_datareader_qos: Mutex<DataReaderQos>,
    pub qos: SubscriberQos,
    pub listener: Option<Box<dyn SubscriberListener>>,
    pub status_mask: StatusMask,
}

impl RtpsSubscriberInner {
    pub fn new(
        guid: GUID,
        qos: SubscriberQos,
        listener: Option<Box<dyn SubscriberListener>>,
        status_mask: StatusMask,
    ) -> Self {
        Self {
            group: Group::new(guid),
            reader_list: Default::default(),
            reader_count: atomic::AtomicU8::new(0),
            default_datareader_qos: Mutex::new(DataReaderQos::default()),
            qos,
            listener,
            status_mask,
        }
    }

    pub fn create_builtin_datareader<T: DDSType>(
        &self,
        a_topic: Arc<dyn AnyRtpsTopic>,
        qos: Option<DataReaderQos>,
        // _a_listener: impl DataReaderListener<T>,
        // _mask: StatusMask
    ) -> Option<RtpsAnyDataReaderRef> {
        self.create_datareader::<T>(a_topic, qos, EntityType::BuiltIn)
    }

    pub fn create_user_defined_datareader<T: DDSType>(
        &self,
        a_topic: Arc<dyn AnyRtpsTopic>,
        qos: Option<DataReaderQos>,
        // _a_listener: impl DataReaderListener<T>,
        // _mask: StatusMask
    ) -> Option<RtpsAnyDataReaderRef> {
        self.create_datareader::<T>(a_topic, qos, EntityType::BuiltIn)
    }

    fn create_datareader<T: DDSType>(
        &self,
        a_topic: Arc<dyn AnyRtpsTopic>,
        qos: Option<DataReaderQos>,
        entity_type: EntityType,
        // _a_listener: impl DataReaderListener<T>,
        // _mask: StatusMask
    ) -> Option<RtpsAnyDataReaderRef> {
        let guid_prefix = self.group.entity.guid.prefix();
        let entity_key = [
            0,
            self.reader_count.fetch_add(1, atomic::Ordering::Relaxed),
            0,
        ];
        let entity_kind = match (a_topic.topic_kind(), entity_type) {
            (TopicKind::WithKey, EntityType::UserDefined) => {
                ENTITY_KIND_USER_DEFINED_READER_WITH_KEY
            }
            (TopicKind::NoKey, EntityType::UserDefined) => ENTITY_KIND_USER_DEFINED_READER_NO_KEY,
            (TopicKind::WithKey, EntityType::BuiltIn) => ENTITY_KIND_BUILT_IN_READER_WITH_KEY,
            (TopicKind::NoKey, EntityType::BuiltIn) => ENTITY_KIND_BUILT_IN_READER_WITH_KEY,
        };
        let entity_id = EntityId::new(entity_key, entity_kind);
        let new_reader_guid = GUID::new(guid_prefix, entity_id);
        let new_reader_qos = qos.unwrap_or(self.get_default_datareader_qos());
        let new_reader: Box<RtpsDataReaderInner<T>> = Box::new(RtpsDataReaderInner::new(
            new_reader_guid,
            a_topic,
            new_reader_qos,
            None,
            0,
        ));
        self.reader_list.add(new_reader)
    }

    pub fn get_default_datareader_qos(&self) -> DataReaderQos {
        self.default_datareader_qos.lock().unwrap().clone()
    }

    pub fn set_default_datawriter_qos(&self, qos: Option<DataReaderQos>) -> ReturnCode<()> {
        let datareader_qos = qos.unwrap_or_default();
        datareader_qos.is_consistent()?;
        *self.default_datareader_qos.lock().unwrap() = datareader_qos;
        Ok(())
    }
}

pub type RtpsSubscriberRef<'a> = MaybeValidRef<'a, Box<RtpsSubscriberInner>>;

impl<'a> RtpsSubscriberRef<'a> {
    pub(crate) fn get(&self) -> ReturnCode<&Box<RtpsSubscriberInner>> {
        MaybeValid::get(self).ok_or(ReturnCodes::AlreadyDeleted)
    }

    pub(crate) fn delete(&self) {
        MaybeValid::delete(self)
    }

    pub(crate) fn get_qos(&self) -> ReturnCode<SubscriberQos> {
        Ok(self.get()?.qos.clone())
    }
}

pub struct RtpsSubscriber<'a> {
    parent_participant: &'a RtpsParticipant,
    subscriber_ref: RtpsSubscriberRef<'a>,
}

impl<'a> RtpsSubscriber<'a> {
    pub(crate) fn new(
        parent_participant: &'a RtpsParticipant,
        subscriber_ref: RtpsSubscriberRef<'a>,
    ) -> Self {
        Self {
            parent_participant,
            subscriber_ref,
        }
    }

    pub(crate) fn subscriber_ref(&self) -> &RtpsSubscriberRef<'a> {
        &self.subscriber_ref
    }
}

impl<'a, T: DDSType> TopicGAT<'a, T> for RtpsSubscriber<'a> {
    type TopicType = RtpsTopic<'a, T>;
}

impl<'a> DomainParticipantChild for RtpsSubscriber<'a> {
    type DomainParticipantType = RtpsParticipant;

    fn get_participant(&self) -> &Self::DomainParticipantType {
        &self.parent_participant
    }
}

impl<'a> Subscriber<'a> for RtpsSubscriber<'a> {
    fn create_datareader<T: DDSType>(
        &'a self,
        _a_topic: &'a <Self as TopicGAT<'a,T>>::TopicType,
        _qos: Option<DataReaderQos>,
        _a_listener: Option<Box<dyn DataReaderListener<T>>>,
        _mask: StatusMask,
    ) -> Option<Box<dyn DataReader<T> + 'a>> {
        todo!()
    }

    fn delete_datareader<T: DDSType>(
        &'a self,
        _a_datareader: &'a Box<dyn DataReader<T> + 'a>,
    ) -> ReturnCode<()> {
        todo!()
    }

    fn lookup_datareader<T: DDSType>(
        &self,
        _topic: &Box<dyn Topic<T>>,
    ) -> Option<Box<dyn DataReader<T>>> {
        todo!()
    }

    fn begin_access(&self) -> ReturnCode<()> {
        todo!()
    }

    fn end_access(&self) -> ReturnCode<()> {
        todo!()
    }

    fn notify_datareaders(&self) -> ReturnCode<()> {
        todo!()
    }

    fn get_sample_lost_status(&self, _status: &mut SampleLostStatus) -> ReturnCode<()> {
        todo!()
    }

    fn delete_contained_entities(&self) -> ReturnCode<()> {
        todo!()
    }

    fn set_default_datareader_qos(&self, _qos: Option<DataReaderQos>) -> ReturnCode<()> {
        todo!()
    }

    fn get_default_datareader_qos(&self) -> ReturnCode<DataReaderQos> {
        todo!()
    }

    fn copy_from_topic_qos(
        &self,
        _a_datareader_qos: &mut DataReaderQos,
        _a_topic_qos: &TopicQos,
    ) -> ReturnCode<()> {
        todo!()
    }

    fn get_datareaders(
        &self,
        _readers: &mut [&mut dyn AnyDataReader],
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
    ) -> ReturnCode<()> {
        todo!()
    }
}

impl<'a> Entity for RtpsSubscriber<'a> {
    type Qos = SubscriberQos;
    type Listener = Box<dyn SubscriberListener>;

    fn set_qos(&self, _qos: Option<Self::Qos>) -> ReturnCode<()> {
        todo!()
    }

    fn get_qos(&self) -> ReturnCode<Self::Qos> {
        self.subscriber_ref.get_qos()
    }

    fn set_listener(&self, _a_listener: Self::Listener, _mask: StatusMask) -> ReturnCode<()> {
        todo!()
    }

    fn get_listener(&self) -> &Self::Listener {
        todo!()
    }

    fn get_statuscondition(&self) -> StatusCondition {
        todo!()
    }

    fn get_status_changes(&self) -> StatusMask {
        todo!()
    }

    fn enable(&self) -> ReturnCode<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> ReturnCode<InstanceHandle> {
        todo!()
    }
}