use crate::{
    builtin_topics::SubscriptionBuiltinTopicData,
    implementation::utils::{
        node::{ChildNode, RootNode},
        shared_object::{DdsRwLock, DdsShared},
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DataWriterQos, QosKind},
        status::{
            LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
            PublicationMatchedStatus, StatusKind,
        },
        time::{Duration, Time},
    },
    topic_definition::type_support::{DdsSerializedKey, DdsType},
};

use super::{
    any_data_writer_listener::AnyDataWriterListener,
    domain_participant_impl::DomainParticipantImpl,
    node_user_defined_publisher::UserDefinedPublisherNode,
    node_user_defined_topic::UserDefinedTopicNode, status_condition_impl::StatusConditionImpl,
    user_defined_data_writer::UserDefinedDataWriter, user_defined_publisher::UserDefinedPublisher,
};

#[derive(PartialEq, Debug)]
pub struct UserDefinedDataWriterNode(
    ChildNode<
        UserDefinedDataWriter,
        ChildNode<UserDefinedPublisher, RootNode<DomainParticipantImpl>>,
    >,
);

impl UserDefinedDataWriterNode {
    pub fn new(
        node: ChildNode<
            UserDefinedDataWriter,
            ChildNode<UserDefinedPublisher, RootNode<DomainParticipantImpl>>,
        >,
    ) -> Self {
        Self(node)
    }

    pub fn register_instance_w_timestamp(
        &self,
        instance_serialized_key: DdsSerializedKey,
        timestamp: Time,
    ) -> DdsResult<Option<InstanceHandle>> {
        self.0
            .get()?
            .register_instance_w_timestamp(instance_serialized_key, timestamp)
    }

    pub fn unregister_instance_w_timestamp(
        &self,
        instance_serialized_key: Vec<u8>,
        handle: InstanceHandle,
        timestamp: Time,
    ) -> DdsResult<()> {
        self.0
            .get()?
            .unregister_instance_w_timestamp(instance_serialized_key, handle, timestamp)
    }

    pub fn get_key_value<Foo>(&self, key_holder: &mut Foo, handle: InstanceHandle) -> DdsResult<()>
    where
        Foo: DdsType,
    {
        self.0.get()?.get_key_value(key_holder, handle)
    }

    pub fn lookup_instance(
        &self,
        instance_serialized_key: DdsSerializedKey,
    ) -> DdsResult<Option<InstanceHandle>> {
        self.0.get()?.lookup_instance(instance_serialized_key)
    }

    pub fn write_w_timestamp(
        &self,
        serialized_data: Vec<u8>,
        instance_serialized_key: DdsSerializedKey,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        self.0
            .get()?
            .write_w_timestamp(serialized_data, instance_serialized_key, handle, timestamp)
    }

    pub fn dispose_w_timestamp(
        &self,
        instance_serialized_key: Vec<u8>,
        handle: InstanceHandle,
        timestamp: Time,
    ) -> DdsResult<()> {
        self.0
            .get()?
            .dispose_w_timestamp(instance_serialized_key, handle, timestamp)
    }

    pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> DdsResult<()> {
        self.0.get()?.wait_for_acknowledgments(max_wait)
    }

    pub fn get_liveliness_lost_status(&self) -> DdsResult<LivelinessLostStatus> {
        Ok(self.0.get()?.get_liveliness_lost_status())
    }

    pub fn get_offered_deadline_missed_status(&self) -> DdsResult<OfferedDeadlineMissedStatus> {
        Ok(self.0.get()?.get_offered_deadline_missed_status())
    }

    pub fn get_offered_incompatible_qos_status(&self) -> DdsResult<OfferedIncompatibleQosStatus> {
        Ok(self.0.get()?.get_offered_incompatible_qos_status())
    }

    pub fn get_publication_matched_status(&self) -> DdsResult<PublicationMatchedStatus> {
        Ok(self.0.get()?.get_publication_matched_status())
    }

    pub fn get_topic(&self) -> DdsResult<UserDefinedTopicNode> {
        let data_writer = self.0.get()?;

        let topic = self
            .0
            .parent()
            .parent()
            .get()?
            .topic_list()
            .find(|t| {
                t.get_name() == data_writer.get_topic_name()
                    && t.get_type_name() == data_writer.get_type_name()
            })
            .expect("Topic must exist");

        Ok(UserDefinedTopicNode::new(ChildNode::new(
            topic.downgrade(),
            self.0.parent().parent().clone(),
        )))
    }

    pub fn get_publisher(&self) -> UserDefinedPublisherNode {
        UserDefinedPublisherNode::new(self.0.parent().clone())
    }

    pub fn assert_liveliness(&self) -> DdsResult<()> {
        self.0.get()?.assert_liveliness()
    }

    pub fn get_matched_subscription_data(
        &self,
        subscription_handle: InstanceHandle,
    ) -> DdsResult<SubscriptionBuiltinTopicData> {
        self.0
            .get()?
            .get_matched_subscription_data(subscription_handle)
    }

    pub fn get_matched_subscriptions(&self) -> DdsResult<Vec<InstanceHandle>> {
        self.0.get()?.get_matched_subscriptions()
    }

    pub fn set_qos(&self, qos: QosKind<DataWriterQos>) -> DdsResult<()> {
        self.0.get()?.set_qos(qos)?;

        let data_writer = self.0.get()?;

        if self.0.get()?.is_enabled() {
            let topic = self
                .0
                .parent()
                .parent()
                .get()?
                .topic_list()
                .find(|t| {
                    t.get_name() == data_writer.get_topic_name()
                        && t.get_type_name() == data_writer.get_type_name()
                })
                .expect("Topic must exist");
            self.0
                .get()?
                .announce_writer(&topic.get_qos(), &self.0.parent().get()?.get_qos());
        }
        Ok(())
    }

    pub fn get_qos(&self) -> DdsResult<DataWriterQos> {
        Ok(self.0.get()?.get_qos())
    }

    pub fn set_listener(
        &self,
        a_listener: Option<Box<dyn AnyDataWriterListener + Send + Sync>>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        self.0.get()?.set_listener(a_listener, mask);
        Ok(())
    }

    pub fn get_statuscondition(&self) -> DdsResult<DdsShared<DdsRwLock<StatusConditionImpl>>> {
        Ok(self.0.get()?.get_statuscondition())
    }

    pub fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        Ok(self.0.get()?.get_status_changes())
    }

    pub fn enable(&self) -> DdsResult<()> {
        if !self.0.parent().get()?.is_enabled() {
            return Err(DdsError::PreconditionNotMet(
                "Parent publisher disabled".to_string(),
            ));
        }
        self.0.get()?.enable()?;

        let data_writer = self.0.get()?;

        let topic = self
            .0
            .parent()
            .parent()
            .get()?
            .topic_list()
            .find(|t| {
                t.get_name() == data_writer.get_topic_name()
                    && t.get_type_name() == data_writer.get_type_name()
            })
            .expect("Topic must exist");
        self.0
            .get()?
            .announce_writer(&topic.get_qos(), &self.0.parent().get()?.get_qos());

        Ok(())
    }

    pub fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        Ok(self.0.get()?.get_instance_handle())
    }
}