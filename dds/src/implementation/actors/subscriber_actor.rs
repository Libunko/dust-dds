use std::collections::HashMap;

use dust_dds_derive::actor_interface;
use fnmatch_regex::glob_to_regex;
use tracing::warn;

use super::{
    data_reader_actor::{self, DataReaderActor},
    domain_participant_actor::DomainParticipantActor,
    subscriber_listener_actor::SubscriberListenerActor,
};
use crate::{
    implementation::{
        actors::{
            domain_participant_listener_actor::DomainParticipantListenerActor,
            status_condition_actor::StatusConditionActor,
        },
        data_representation_builtin_endpoints::discovered_writer_data::DiscoveredWriterData,
        rtps::{
            group::RtpsGroup,
            messages::overall_structure::{RtpsMessageHeader, RtpsMessageRead},
            types::{Guid, Locator},
        },
        rtps_udp_psm::udp_transport::UdpTransportWrite,
        utils::actor::{spawn_actor, Actor, ActorAddress},
    },
    infrastructure::{
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DataReaderQos, QosKind, SubscriberQos},
        qos_policy::PartitionQosPolicy,
        status::StatusKind,
        time::Time,
    },
    subscription::subscriber_listener::SubscriberListener,
};

pub struct SubscriberActor {
    qos: SubscriberQos,
    rtps_group: RtpsGroup,
    data_reader_list: HashMap<InstanceHandle, Actor<DataReaderActor>>,
    enabled: bool,
    user_defined_data_reader_counter: u8,
    default_data_reader_qos: DataReaderQos,
    status_condition: Actor<StatusConditionActor>,
    listener: Option<Actor<SubscriberListenerActor>>,
    status_kind: Vec<StatusKind>,
}

impl SubscriberActor {
    pub fn new(
        qos: SubscriberQos,
        rtps_group: RtpsGroup,
        listener: Option<Actor<SubscriberListenerActor>>,
        status_kind: Vec<StatusKind>,
    ) -> Self {
        let status_condition = spawn_actor(StatusConditionActor::default());
        SubscriberActor {
            qos,
            rtps_group,
            data_reader_list: HashMap::new(),
            enabled: false,
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: Default::default(),
            status_condition,
            listener,
            status_kind,
        }
    }
}

#[actor_interface]
impl SubscriberActor {
    async fn lookup_datareader(&self, topic_name: String) -> Option<ActorAddress<DataReaderActor>> {
        for dr in self.data_reader_list.values() {
            if dr
                .send_mail_and_await_reply(data_reader_actor::get_topic_name::new())
                .await
                == topic_name
            {
                return Some(dr.address());
            }
        }
        None
    }
    async fn delete_contained_entities(&mut self) {}

    async fn guid(&self) -> Guid {
        self.rtps_group.guid()
    }

    async fn is_empty(&self) -> bool {
        self.data_reader_list.is_empty()
    }

    async fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn get_unique_reader_id(&mut self) -> u8 {
        let counter = self.user_defined_data_reader_counter;
        self.user_defined_data_reader_counter += 1;
        counter
    }

    async fn data_reader_add(
        &mut self,
        instance_handle: InstanceHandle,
        data_reader: Actor<DataReaderActor>,
    ) {
        self.data_reader_list.insert(instance_handle, data_reader);
    }

    async fn data_reader_delete(&mut self, handle: InstanceHandle) {
        self.data_reader_list.remove(&handle);
    }

    async fn set_default_datareader_qos(&mut self, qos: QosKind<DataReaderQos>) -> DdsResult<()> {
        match qos {
            QosKind::Default => self.default_data_reader_qos = DataReaderQos::default(),
            QosKind::Specific(q) => {
                q.is_consistent()?;
                self.default_data_reader_qos = q;
            }
        }
        Ok(())
    }

    async fn get_default_datareader_qos(&self) -> DataReaderQos {
        self.default_data_reader_qos.clone()
    }

    async fn set_qos(&mut self, qos: QosKind<SubscriberQos>) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => Default::default(),
            QosKind::Specific(q) => q,
        };

        if self.enabled {
            self.qos.check_immutability(&qos)?;
        }

        self.qos = qos;

        Ok(())
    }

    async fn enable(&mut self) {
        self.enabled = true;
    }

    async fn get_instance_handle(&self) -> InstanceHandle {
        self.rtps_group.guid().into()
    }

    async fn get_statuscondition(&self) -> ActorAddress<StatusConditionActor> {
        self.status_condition.address()
    }

    async fn get_qos(&self) -> SubscriberQos {
        self.qos.clone()
    }

    async fn data_reader_list(&self) -> Vec<ActorAddress<DataReaderActor>> {
        self.data_reader_list
            .values()
            .map(|dr| dr.address())
            .collect()
    }

    async fn get_listener(&self) -> Option<ActorAddress<SubscriberListenerActor>> {
        self.listener.as_ref().map(|l| l.address())
    }

    async fn get_status_kind(&self) -> Vec<StatusKind> {
        self.status_kind.clone()
    }

    async fn send_message(
        &self,
        header: RtpsMessageHeader,
        udp_transport_write: ActorAddress<UdpTransportWrite>,
    ) {
        for data_reader_address in self.data_reader_list.values() {
            data_reader_address
                .send_mail(data_reader_actor::send_message::new(
                    header,
                    udp_transport_write.clone(),
                ))
                .await;
        }
    }

    async fn process_rtps_message(
        &self,
        message: RtpsMessageRead,
        reception_timestamp: Time,
        participant_address: ActorAddress<DomainParticipantActor>,
        subscriber_address: ActorAddress<SubscriberActor>,
        participant_mask_listener: (
            Option<ActorAddress<DomainParticipantListenerActor>>,
            Vec<StatusKind>,
        ),
    ) -> DdsResult<()> {
        let subscriber_mask_listener = (
            self.listener.as_ref().map(|a| a.address()),
            self.status_kind.clone(),
        );

        for data_reader_address in self.data_reader_list.values().map(|a| a.address()) {
            data_reader_address
                .send_mail_and_await_reply(data_reader_actor::process_rtps_message::new(
                    message.clone(),
                    reception_timestamp,
                    data_reader_address.clone(),
                    subscriber_address.clone(),
                    participant_address.clone(),
                    self.status_condition.address().clone(),
                    subscriber_mask_listener.clone(),
                    participant_mask_listener.clone(),
                ))
                .await??;
        }
        Ok(())
    }

    async fn add_matched_writer(
        &self,
        discovered_writer_data: DiscoveredWriterData,
        default_unicast_locator_list: Vec<Locator>,
        default_multicast_locator_list: Vec<Locator>,
        subscriber_address: ActorAddress<SubscriberActor>,
        participant_address: ActorAddress<DomainParticipantActor>,
        participant_mask_listener: (
            Option<ActorAddress<DomainParticipantListenerActor>>,
            Vec<StatusKind>,
        ),
    ) {
        if self.is_partition_matched(discovered_writer_data.dds_publication_data().partition()) {
            for data_reader in self.data_reader_list.values() {
                let subscriber_mask_listener = (
                    self.listener.as_ref().map(|l| l.address()),
                    self.status_kind.clone(),
                );
                let data_reader_address = data_reader.address();
                let subscriber_qos = self.qos.clone();
                data_reader
                    .send_mail_and_await_reply(data_reader_actor::add_matched_writer::new(
                        discovered_writer_data.clone(),
                        default_unicast_locator_list.clone(),
                        default_multicast_locator_list.clone(),
                        data_reader_address,
                        subscriber_address.clone(),
                        participant_address.clone(),
                        subscriber_qos,
                        subscriber_mask_listener,
                        participant_mask_listener.clone(),
                    ))
                    .await;
            }
        }
    }

    async fn remove_matched_writer(
        &self,
        discovered_writer_handle: InstanceHandle,
        subscriber_address: ActorAddress<SubscriberActor>,
        participant_address: ActorAddress<DomainParticipantActor>,
        participant_mask_listener: (
            Option<ActorAddress<DomainParticipantListenerActor>>,
            Vec<StatusKind>,
        ),
    ) {
        for data_reader in self.data_reader_list.values() {
            let data_reader_address = data_reader.address();
            let subscriber_mask_listener = (
                self.listener.as_ref().map(|l| l.address()),
                self.status_kind.clone(),
            );
            data_reader
                .send_mail_and_await_reply(data_reader_actor::remove_matched_writer::new(
                    discovered_writer_handle,
                    data_reader_address,
                    subscriber_address.clone(),
                    participant_address.clone(),
                    subscriber_mask_listener,
                    participant_mask_listener.clone(),
                ))
                .await;
        }
    }

    async fn set_listener(
        &mut self,
        listener: Option<Box<dyn SubscriberListener + Send + 'static>>,
        status_kind: Vec<StatusKind>,
    ) {
        self.listener = listener.map(|l| spawn_actor(SubscriberListenerActor::new(l)));
        self.status_kind = status_kind;
    }
}

impl SubscriberActor {
    fn is_partition_matched(&self, discovered_partition_qos_policy: &PartitionQosPolicy) -> bool {
        let is_any_name_matched = discovered_partition_qos_policy
            .name
            .iter()
            .any(|n| self.qos.partition.name.contains(n));

        let is_any_received_regex_matched_with_partition_qos = discovered_partition_qos_policy
            .name
            .iter()
            .filter_map(|n| match glob_to_regex(n) {
                Ok(regex) => Some(regex),
                Err(e) => {
                    warn!(
                        "Received invalid partition regex name {:?}. Error {:?}",
                        n, e
                    );
                    None
                }
            })
            .any(|regex| self.qos.partition.name.iter().any(|n| regex.is_match(n)));

        let is_any_local_regex_matched_with_received_partition_qos = self
            .qos
            .partition
            .name
            .iter()
            .filter_map(|n| match glob_to_regex(n) {
                Ok(regex) => Some(regex),
                Err(e) => {
                    warn!(
                        "Invalid partition regex name on subscriber qos {:?}. Error {:?}",
                        n, e
                    );
                    None
                }
            })
            .any(|regex| {
                discovered_partition_qos_policy
                    .name
                    .iter()
                    .any(|n| regex.is_match(n))
            });

        discovered_partition_qos_policy == &self.qos.partition
            || is_any_name_matched
            || is_any_received_regex_matched_with_partition_qos
            || is_any_local_regex_matched_with_received_partition_qos
    }
}