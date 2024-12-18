use crate::{
    builtin_topics::{
        ParticipantBuiltinTopicData, PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
        DCPS_PARTICIPANT, DCPS_PUBLICATION, DCPS_SUBSCRIPTION, DCPS_TOPIC,
    },
    domain::domain_participant_factory::DomainId,
    implementation::data_representation_builtin_endpoints::{
        discovered_reader_data::{DiscoveredReaderData, ReaderProxy},
        discovered_writer_data::{DiscoveredWriterData, WriterProxy},
        spdp_discovered_participant_data::{ParticipantProxy, SpdpDiscoveredParticipantData},
    },
    rtps::{
        discovery_types::{
            ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
        },
        message_receiver::MessageReceiver,
        stateful_writer::RtpsStatefulWriter,
        types::ENTITYID_UNKNOWN,
    },
    runtime::{
        actor::{ActorAddress, Mail, MailHandler},
        executor::block_on,
    },
    topic_definition::type_support::{DdsDeserialize, DdsSerialize},
    transport::{
        cache_change::CacheChange,
        reader::ReaderHistoryCache,
        types::{ChangeKind, ReliabilityKind},
        writer::WriterHistoryCache,
    },
};

use super::{
    behavior_types::Duration,
    discovery_types::{
        BuiltinEndpointQos, BuiltinEndpointSet, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
        ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR, ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
        ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
    },
    error::RtpsResult,
    message_sender::MessageSender,
    messages::overall_structure::RtpsMessageRead,
    stateful_reader::RtpsStatefulReader,
    stateless_reader::RtpsStatelessReader,
    stateless_writer::RtpsStatelessWriter,
    types::{
        DurabilityKind, Guid, Locator, ProtocolVersion, SequenceNumber, VendorId,
        PROTOCOLVERSION_2_4, VENDOR_ID_S2E,
    },
};

pub struct RtpsParticipant {
    guid: Guid,
    domain_id: DomainId,
    domain_tag: String,
    protocol_version: ProtocolVersion,
    vendor_id: VendorId,
    default_unicast_locator_list: Vec<Locator>,
    default_multicast_locator_list: Vec<Locator>,
    metatraffic_unicast_locator_list: Vec<Locator>,
    metatraffic_multicast_locator_list: Vec<Locator>,
    builtin_stateless_writer_list: Vec<RtpsStatelessWriter>,
    builtin_stateful_writer_list: Vec<RtpsStatefulWriter>,
    builtin_stateless_reader_list: Vec<RtpsStatelessReader>,
    builtin_stateful_reader_list: Vec<RtpsStatefulReader>,
    user_defined_writer_list: Vec<RtpsStatefulWriter>,
    user_defined_reader_list: Vec<RtpsStatefulReader>,
    message_sender: MessageSender,
    discovered_participant_list: Vec<SpdpDiscoveredParticipantData>,
    discovered_reader_list: Vec<DiscoveredReaderData>,
    discovered_writer_list: Vec<DiscoveredWriterData>,
}

impl RtpsParticipant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        guid: Guid,
        domain_id: DomainId,
        domain_tag: String,
        default_unicast_locator_list: Vec<Locator>,
        default_multicast_locator_list: Vec<Locator>,
        metatraffic_unicast_locator_list: Vec<Locator>,
        metatraffic_multicast_locator_list: Vec<Locator>,
        spdp_builtin_participant_reader_history_cache: Box<dyn ReaderHistoryCache>,
        sedp_builtin_topics_reader_history_cache: Box<dyn ReaderHistoryCache>,
        sedp_builtin_publications_reader_history_cache: Box<dyn ReaderHistoryCache>,
        sedp_builtin_subscriptions_reader_history_cache: Box<dyn ReaderHistoryCache>,
    ) -> RtpsResult<Self> {
        let guid_prefix = guid.prefix();
        let message_sender =
            MessageSender::new(guid_prefix, std::net::UdpSocket::bind("0.0.0.0:0000")?);

        let mut spdp_builtin_participant_writer = RtpsStatelessWriter::new(
            Guid::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER),
            DCPS_PARTICIPANT.to_owned(),
        );
        for locator in &metatraffic_multicast_locator_list {
            spdp_builtin_participant_writer.reader_locator_add(*locator);
        }

        let spdp_builtin_participant_reader = RtpsStatelessReader::new(
            Guid::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER),
            DCPS_PARTICIPANT.to_owned(),
            spdp_builtin_participant_reader_history_cache,
        );

        let sedp_builtin_topics_writer = RtpsStatefulWriter::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER),
            DCPS_TOPIC.to_owned(),
        );

        let sedp_builtin_topics_reader = RtpsStatefulReader::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR),
            DCPS_TOPIC.to_owned(),
            sedp_builtin_topics_reader_history_cache,
        );

        let sedp_builtin_publications_writer = RtpsStatefulWriter::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER),
            DCPS_PUBLICATION.to_owned(),
        );

        let sedp_builtin_publications_reader = RtpsStatefulReader::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR),
            DCPS_PUBLICATION.to_owned(),
            sedp_builtin_publications_reader_history_cache,
        );

        let sedp_builtin_subscriptions_writer = RtpsStatefulWriter::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER),
            DCPS_SUBSCRIPTION.to_owned(),
        );

        let sedp_builtin_subscriptions_reader = RtpsStatefulReader::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR),
            DCPS_SUBSCRIPTION.to_owned(),
            sedp_builtin_subscriptions_reader_history_cache,
        );

        let builtin_stateless_writer_list = vec![spdp_builtin_participant_writer];
        let builtin_stateless_reader_list = vec![spdp_builtin_participant_reader];
        let builtin_stateful_writer_list = vec![
            sedp_builtin_topics_writer,
            sedp_builtin_publications_writer,
            sedp_builtin_subscriptions_writer,
        ];
        let builtin_stateful_reader_list = vec![
            sedp_builtin_topics_reader,
            sedp_builtin_publications_reader,
            sedp_builtin_subscriptions_reader,
        ];
        let user_defined_writer_list = Vec::new();
        let user_defined_reader_list = Vec::new();

        Ok(Self {
            guid,
            domain_id,
            domain_tag,
            protocol_version: PROTOCOLVERSION_2_4,
            vendor_id: VENDOR_ID_S2E,
            default_unicast_locator_list,
            default_multicast_locator_list,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            builtin_stateless_writer_list,
            builtin_stateful_writer_list,
            builtin_stateless_reader_list,
            builtin_stateful_reader_list,
            user_defined_writer_list,
            user_defined_reader_list,
            message_sender,
            discovered_participant_list: Vec::new(),
            discovered_reader_list: Vec::new(),
            discovered_writer_list: Vec::new(),
        })
    }

    pub fn guid(&self) -> Guid {
        self.guid
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub fn vendor_id(&self) -> VendorId {
        self.vendor_id
    }

    pub fn default_unicast_locator_list(&self) -> &[Locator] {
        self.default_unicast_locator_list.as_slice()
    }

    pub fn set_default_unicast_locator_list(&mut self, list: Vec<Locator>) {
        self.default_unicast_locator_list = list;
    }

    pub fn default_multicast_locator_list(&self) -> &[Locator] {
        self.default_multicast_locator_list.as_slice()
    }

    pub fn set_default_multicast_locator_list(&mut self, list: Vec<Locator>) {
        self.default_multicast_locator_list = list;
    }

    pub fn metatraffic_unicast_locator_list(&self) -> &[Locator] {
        self.metatraffic_unicast_locator_list.as_ref()
    }

    pub fn set_metatraffic_unicast_locator_list(&mut self, list: Vec<Locator>) {
        self.metatraffic_unicast_locator_list = list;
    }

    pub fn metatraffic_multicast_locator_list(&self) -> &[Locator] {
        self.metatraffic_multicast_locator_list.as_ref()
    }

    pub fn set_metatraffic_multicast_locator_list(&mut self, list: Vec<Locator>) {
        self.metatraffic_multicast_locator_list = list;
    }

    pub fn add_discovered_participant(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        // Check that the domainId of the discovered participant equals the local one.
        // If it is not equal then there the local endpoints are not configured to
        // communicate with the discovered participant.
        // AND
        // Check that the domainTag of the discovered participant equals the local one.
        // If it is not equal then there the local endpoints are not configured to
        // communicate with the discovered participant.
        // IN CASE no domain id was transmitted the a local domain id is assumed
        // (as specified in Table 9.19 - ParameterId mapping and default values)
        let is_domain_id_matching = discovered_participant_data
            .participant_proxy
            .domain_id
            .unwrap_or(self.domain_id)
            == self.domain_id;
        let is_domain_tag_matching =
            discovered_participant_data.participant_proxy.domain_tag == self.domain_tag;

        let is_participant_discovered = self
            .discovered_participant_list
            .contains(discovered_participant_data);
        if is_domain_id_matching && is_domain_tag_matching && !is_participant_discovered {
            self.add_matched_publications_detector(discovered_participant_data);
            self.add_matched_publications_announcer(discovered_participant_data);
            self.add_matched_subscriptions_detector(discovered_participant_data);
            self.add_matched_subscriptions_announcer(discovered_participant_data);
            self.add_matched_topics_detector(discovered_participant_data);
            self.add_matched_topics_announcer(discovered_participant_data);
            self.discovered_participant_list
                .push(discovered_participant_data.clone());
        }
    }

    fn add_matched_publications_detector(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let reader_proxy = ReaderProxy {
                remote_reader_guid,
                remote_group_entity_id,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                expects_inline_qos,
            };
            if let Some(w) = self
                .builtin_stateful_writer_list
                .iter_mut()
                .find(|w| w.guid().entity_id() == ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER)
            {
                w.add_matched_reader(
                    &reader_proxy,
                    ReliabilityKind::Reliable,
                    DurabilityKind::TransientLocal,
                    &[],
                    &[],
                );
                w.send_message(&self.message_sender);
            }
        }
    }

    fn add_matched_publications_announcer(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = Default::default();

            let writer_proxy = WriterProxy {
                remote_writer_guid,
                remote_group_entity_id,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                data_max_size_serialized,
            };
            if let Some(r) = self
                .builtin_stateful_reader_list
                .iter_mut()
                .find(|w| w.guid().entity_id() == ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR)
            {
                r.add_matched_writer(
                    &writer_proxy,
                    ReliabilityKind::Reliable,
                    DurabilityKind::TransientLocal,
                    &[],
                    &[],
                );
            }
        }
    }

    fn add_matched_subscriptions_detector(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let reader_proxy = ReaderProxy {
                remote_reader_guid,
                remote_group_entity_id,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                expects_inline_qos,
            };
            if let Some(w) = self
                .builtin_stateful_writer_list
                .iter_mut()
                .find(|w| w.guid().entity_id() == ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER)
            {
                w.add_matched_reader(
                    &reader_proxy,
                    ReliabilityKind::Reliable,
                    DurabilityKind::TransientLocal,
                    &[],
                    &[],
                );
                w.send_message(&self.message_sender);
            }
        }
    }

    fn add_matched_subscriptions_announcer(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = Default::default();

            let writer_proxy = WriterProxy {
                remote_writer_guid,
                remote_group_entity_id,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                data_max_size_serialized,
            };
            if let Some(r) = self
                .builtin_stateful_reader_list
                .iter_mut()
                .find(|w| w.guid().entity_id() == ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR)
            {
                r.add_matched_writer(
                    &writer_proxy,
                    ReliabilityKind::Reliable,
                    DurabilityKind::TransientLocal,
                    &[],
                    &[],
                );
            }
        }
    }

    fn add_matched_topics_detector(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let reader_proxy = ReaderProxy {
                remote_reader_guid,
                remote_group_entity_id,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                expects_inline_qos,
            };
            if let Some(w) = self
                .builtin_stateful_writer_list
                .iter_mut()
                .find(|w| w.guid().entity_id() == ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER)
            {
                w.add_matched_reader(
                    &reader_proxy,
                    ReliabilityKind::Reliable,
                    DurabilityKind::TransientLocal,
                    &[],
                    &[],
                );
                w.send_message(&self.message_sender);
            }
        }
    }

    fn add_matched_topics_announcer(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = Default::default();

            let writer_proxy = WriterProxy {
                remote_writer_guid,
                remote_group_entity_id,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                data_max_size_serialized,
            };
            if let Some(r) = self
                .builtin_stateful_reader_list
                .iter_mut()
                .find(|w| w.guid().entity_id() == ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR)
            {
                r.add_matched_writer(
                    &writer_proxy,
                    ReliabilityKind::Reliable,
                    DurabilityKind::TransientLocal,
                    &[],
                    &[],
                );
            }
        }
    }

    pub fn add_discovered_writer(&mut self, discovered_writer_data: DiscoveredWriterData) {
        if !self
            .discovered_writer_list
            .contains(&discovered_writer_data)
        {
            for reader in &mut self.user_defined_reader_list {
                if reader.topic_name() == discovered_writer_data.dds_publication_data.topic_name {
                    if let Some(p) = self.discovered_participant_list.iter().find(|x| {
                        x.participant_proxy.guid_prefix
                            == discovered_writer_data
                                .writer_proxy
                                .remote_writer_guid
                                .prefix()
                    }) {
                        let reliability_kind = discovered_writer_data
                            .dds_publication_data
                            .reliability()
                            .into();
                        let durability_kind = discovered_writer_data
                            .dds_publication_data
                            .durability()
                            .into();
                        reader.add_matched_writer(
                            &discovered_writer_data.writer_proxy,
                            reliability_kind,
                            durability_kind,
                            &p.participant_proxy.default_unicast_locator_list,
                            &p.participant_proxy.default_multicast_locator_list,
                        );
                    }
                }
            }
            self.discovered_writer_list.push(discovered_writer_data);
        }
    }

    pub fn add_discovered_reader(&mut self, discovered_reader_data: DiscoveredReaderData) {
        if !self
            .discovered_reader_list
            .contains(&discovered_reader_data)
        {
            for writer in &mut self.user_defined_writer_list {
                if writer.topic_name() == discovered_reader_data.dds_subscription_data.topic_name()
                {
                    if let Some(p) = self.discovered_participant_list.iter().find(|x| {
                        x.participant_proxy.guid_prefix
                            == discovered_reader_data
                                .reader_proxy
                                .remote_reader_guid
                                .prefix()
                    }) {
                        let reliability_kind = discovered_reader_data
                            .dds_subscription_data
                            .reliability()
                            .into();
                        let durability_kind = discovered_reader_data
                            .dds_subscription_data
                            .durability()
                            .into();
                        writer.add_matched_reader(
                            &discovered_reader_data.reader_proxy,
                            reliability_kind,
                            durability_kind,
                            &p.participant_proxy.default_unicast_locator_list,
                            &p.participant_proxy.default_multicast_locator_list,
                        );
                        writer.send_message(&self.message_sender);
                    }
                }
            }
            self.discovered_reader_list.push(discovered_reader_data);
        }
    }

    pub fn participant_proxy(&self) -> ParticipantProxy {
        ParticipantProxy {
            domain_id: Some(self.domain_id),
            domain_tag: self.domain_tag.clone(),
            protocol_version: PROTOCOLVERSION_2_4,
            guid_prefix: self.guid.prefix(),
            vendor_id: VENDOR_ID_S2E,
            expects_inline_qos: false,
            metatraffic_unicast_locator_list: self.metatraffic_unicast_locator_list.clone(),
            metatraffic_multicast_locator_list: self.metatraffic_multicast_locator_list.clone(),
            default_unicast_locator_list: self.default_unicast_locator_list.clone(),
            default_multicast_locator_list: self.default_multicast_locator_list.clone(),
            available_builtin_endpoints: BuiltinEndpointSet::default(),
            manual_liveliness_count: 0,
            builtin_endpoint_qos: BuiltinEndpointQos::default(),
        }
    }

    pub fn create_writer(&mut self, writer_guid: Guid, topic_name: String) {
        let mut writer = RtpsStatefulWriter::new(writer_guid, topic_name);
        for discovered_reader_data in &self.discovered_reader_list {
            if writer.topic_name() == discovered_reader_data.dds_subscription_data.topic_name() {
                if let Some(p) = self.discovered_participant_list.iter().find(|x| {
                    x.participant_proxy.guid_prefix
                        == discovered_reader_data
                            .reader_proxy
                            .remote_reader_guid
                            .prefix()
                }) {
                    let reliability_kind = discovered_reader_data
                        .dds_subscription_data
                        .reliability()
                        .into();
                    let durability_kind = discovered_reader_data
                        .dds_subscription_data
                        .durability()
                        .into();
                    writer.add_matched_reader(
                        &discovered_reader_data.reader_proxy,
                        reliability_kind,
                        durability_kind,
                        &p.participant_proxy.default_unicast_locator_list,
                        &p.participant_proxy.default_multicast_locator_list,
                    );
                    writer.send_message(&self.message_sender);
                }
            }
        }
        self.user_defined_writer_list.push(writer);
    }

    pub fn delete_writer(&mut self, writer_guid: Guid) {
        self.user_defined_writer_list
            .retain(|x| x.guid() != writer_guid);
    }

    pub fn create_reader(
        &mut self,
        reader_guid: Guid,
        topic_name: String,
        reader_history_cache: Box<dyn ReaderHistoryCache>,
    ) {
        let mut reader = RtpsStatefulReader::new(reader_guid, topic_name, reader_history_cache);
        for discovered_writer_data in &self.discovered_writer_list {
            if reader.topic_name() == discovered_writer_data.dds_publication_data.topic_name {
                if let Some(p) = self.discovered_participant_list.iter().find(|x| {
                    x.participant_proxy.guid_prefix
                        == discovered_writer_data
                            .writer_proxy
                            .remote_writer_guid
                            .prefix()
                }) {
                    let reliability_kind = discovered_writer_data
                        .dds_publication_data
                        .reliability()
                        .into();
                    let durability_kind = discovered_writer_data
                        .dds_publication_data
                        .durability()
                        .into();
                    reader.add_matched_writer(
                        &discovered_writer_data.writer_proxy,
                        reliability_kind,
                        durability_kind,
                        &p.participant_proxy.default_unicast_locator_list,
                        &p.participant_proxy.default_multicast_locator_list,
                    );
                }
            }
        }
        self.user_defined_reader_list.push(reader);
    }

    pub fn delete_reader(&mut self, reader_guid: Guid) {
        self.user_defined_reader_list
            .retain(|x| x.guid() != reader_guid);
    }

    pub fn process_builtin_rtps_message(&mut self, message: RtpsMessageRead) {
        MessageReceiver::new(message).process_message(
            &mut self.builtin_stateless_reader_list,
            &mut self.builtin_stateful_reader_list,
            &mut self.builtin_stateful_writer_list,
            &self.message_sender,
        );
    }

    pub fn process_user_defined_rtps_message(&mut self, message: RtpsMessageRead) {
        MessageReceiver::new(message).process_message(
            &mut [],
            &mut self.user_defined_reader_list,
            &mut self.user_defined_writer_list,
            &self.message_sender,
        );
    }
}

pub struct ProcessBuiltinRtpsMessage {
    pub rtps_message: RtpsMessageRead,
}
impl Mail for ProcessBuiltinRtpsMessage {
    type Result = ();
}
impl MailHandler<ProcessBuiltinRtpsMessage> for RtpsParticipant {
    fn handle(
        &mut self,
        message: ProcessBuiltinRtpsMessage,
    ) -> <ProcessBuiltinRtpsMessage as Mail>::Result {
        self.process_builtin_rtps_message(message.rtps_message);
    }
}

pub struct ProcessUserDefinedRtpsMessage {
    pub rtps_message: RtpsMessageRead,
}
impl Mail for ProcessUserDefinedRtpsMessage {
    type Result = ();
}
impl MailHandler<ProcessUserDefinedRtpsMessage> for RtpsParticipant {
    fn handle(
        &mut self,
        message: ProcessUserDefinedRtpsMessage,
    ) -> <ProcessUserDefinedRtpsMessage as Mail>::Result {
        self.process_user_defined_rtps_message(message.rtps_message);
    }
}

pub struct SendHeartbeat;
impl Mail for SendHeartbeat {
    type Result = ();
}
impl MailHandler<SendHeartbeat> for RtpsParticipant {
    fn handle(&mut self, _: SendHeartbeat) -> <SendHeartbeat as Mail>::Result {
        for builtin_writer in self.builtin_stateful_writer_list.iter_mut() {
            builtin_writer.send_message(&self.message_sender);
        }
        for user_defined_writer in self.user_defined_writer_list.iter_mut() {
            user_defined_writer.send_message(&self.message_sender);
        }
    }
}

pub struct CreateWriter {
    pub writer_guid: Guid,
    pub topic_name: String,
    pub rtps_participant_address: ActorAddress<RtpsParticipant>,
}

impl Mail for CreateWriter {
    type Result = Box<dyn WriterHistoryCache>;
}
impl MailHandler<CreateWriter> for RtpsParticipant {
    fn handle(&mut self, message: CreateWriter) -> <CreateWriter as Mail>::Result {
        self.create_writer(message.writer_guid, message.topic_name);

        struct RtpsUserDefinedWriterHistoryCache {
            rtps_participant_address: ActorAddress<RtpsParticipant>,
            guid: Guid,
        }
        impl WriterHistoryCache for RtpsUserDefinedWriterHistoryCache {
            fn guid(&self) -> [u8; 16] {
                self.guid.into()
            }

            fn add_change(&mut self, cache_change: CacheChange) {
                self.rtps_participant_address
                    .send_actor_mail(AddUserDefinedCacheChange {
                        guid: self.guid,
                        cache_change,
                    })
                    .ok();
            }

            fn remove_change(&mut self, sequence_number: SequenceNumber) {
                self.rtps_participant_address
                    .send_actor_mail(RemoveUserDefinedCacheChange {
                        guid: self.guid,
                        sequence_number,
                    })
                    .ok();
            }

            fn is_change_acknowledged(&self, sequence_number: SequenceNumber) -> bool {
                block_on(
                    self.rtps_participant_address
                        .send_actor_mail(IsChangeAcknowledged {
                            guid: self.guid,
                            sequence_number,
                        })
                        .expect("Actor must exist")
                        .receive_reply(),
                )
            }
        }

        Box::new(RtpsUserDefinedWriterHistoryCache {
            rtps_participant_address: message.rtps_participant_address,
            guid: message.writer_guid,
        })
    }
}

pub struct CreateReader {
    pub reader_guid: Guid,
    pub topic_name: String,
    pub reader_history_cache: Box<dyn ReaderHistoryCache>,
}

impl Mail for CreateReader {
    type Result = ();
}
impl MailHandler<CreateReader> for RtpsParticipant {
    fn handle(&mut self, message: CreateReader) -> <CreateReader as Mail>::Result {
        self.create_reader(
            message.reader_guid,
            message.topic_name,
            message.reader_history_cache,
        )
    }
}

pub struct AddParticipantDiscoveryCacheChange {
    pub cache_change: CacheChange,
}
impl Mail for AddParticipantDiscoveryCacheChange {
    type Result = ();
}
impl MailHandler<AddParticipantDiscoveryCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: AddParticipantDiscoveryCacheChange,
    ) -> <AddParticipantDiscoveryCacheChange as Mail>::Result {
        let participant_proxy = self.participant_proxy();
        if let Some(w) = self
            .builtin_stateless_writer_list
            .iter_mut()
            .find(|dw| dw.guid().entity_id() == ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER)
        {
            match message.cache_change.kind {
                ChangeKind::Alive => {
                    if let Ok(dds_participant_data) = ParticipantBuiltinTopicData::deserialize_data(
                        message.cache_change.data_value.as_ref(),
                    ) {
                        let spdp_discovered_participant_data = SpdpDiscoveredParticipantData {
                            dds_participant_data,
                            participant_proxy,
                            lease_duration: Duration::new(100, 0).into(),
                            discovered_participant_list: vec![],
                        };

                        let mut cache_change = message.cache_change;
                        cache_change.data_value = spdp_discovered_participant_data
                            .serialize_data()
                            .unwrap()
                            .into();
                        w.add_change(cache_change);
                        w.send_message(&self.message_sender);
                    }
                }
                ChangeKind::NotAliveDisposed => {
                    w.add_change(message.cache_change);
                    w.send_message(&self.message_sender);
                }
                ChangeKind::AliveFiltered
                | ChangeKind::NotAliveUnregistered
                | ChangeKind::NotAliveDisposedUnregistered => unimplemented!(),
            }
        }
    }
}

pub struct RemoveParticipantDiscoveryCacheChange {
    pub sequence_number: SequenceNumber,
}
impl Mail for RemoveParticipantDiscoveryCacheChange {
    type Result = ();
}
impl MailHandler<RemoveParticipantDiscoveryCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: RemoveParticipantDiscoveryCacheChange,
    ) -> <RemoveParticipantDiscoveryCacheChange as Mail>::Result {
        if let Some(w) = self
            .builtin_stateless_writer_list
            .iter_mut()
            .find(|dw| dw.guid().entity_id() == ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER)
        {
            w.remove_change(message.sequence_number);
        }
    }
}

pub struct AddTopicsDiscoveryCacheChange {
    pub cache_change: CacheChange,
}
impl Mail for AddTopicsDiscoveryCacheChange {
    type Result = ();
}
impl MailHandler<AddTopicsDiscoveryCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: AddTopicsDiscoveryCacheChange,
    ) -> <AddTopicsDiscoveryCacheChange as Mail>::Result {
        if let Some(w) = self
            .builtin_stateful_writer_list
            .iter_mut()
            .find(|dw| dw.guid().entity_id() == ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER)
        {
            w.add_change(message.cache_change, &self.message_sender);
            w.send_message(&self.message_sender);
        }
    }
}

pub struct RemoveTopicsDiscoveryCacheChange {
    pub sequence_number: SequenceNumber,
}
impl Mail for RemoveTopicsDiscoveryCacheChange {
    type Result = ();
}
impl MailHandler<RemoveTopicsDiscoveryCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: RemoveTopicsDiscoveryCacheChange,
    ) -> <RemoveTopicsDiscoveryCacheChange as Mail>::Result {
        if let Some(w) = self
            .builtin_stateless_writer_list
            .iter_mut()
            .find(|dw| dw.guid().entity_id() == ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER)
        {
            w.remove_change(message.sequence_number);
        }
    }
}

pub struct AddPublicationsDiscoveryCacheChange {
    pub cache_change: CacheChange,
}
impl Mail for AddPublicationsDiscoveryCacheChange {
    type Result = ();
}
impl MailHandler<AddPublicationsDiscoveryCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: AddPublicationsDiscoveryCacheChange,
    ) -> <AddPublicationsDiscoveryCacheChange as Mail>::Result {
        if let Some(w) = self
            .builtin_stateful_writer_list
            .iter_mut()
            .find(|dw| dw.guid().entity_id() == ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER)
        {
            match message.cache_change.kind {
                ChangeKind::Alive => {
                    if let Ok(dds_publication_data) = PublicationBuiltinTopicData::deserialize_data(
                        message.cache_change.data_value.as_ref(),
                    ) {
                        if let Some(writer_proxy) = self
                            .user_defined_writer_list
                            .iter()
                            .find(|w| w.guid() == dds_publication_data.key.value.into())
                            .map(|w| w.writer_proxy())
                        {
                            let mut cache_change = message.cache_change;
                            let discovered_writer_data = DiscoveredWriterData {
                                dds_publication_data,
                                writer_proxy,
                            };
                            cache_change.data_value =
                                discovered_writer_data.serialize_data().unwrap().into();
                            w.add_change(cache_change, &self.message_sender);
                            w.send_message(&self.message_sender);
                        }
                    }
                }
                ChangeKind::NotAliveDisposed => {
                    w.add_change(message.cache_change, &self.message_sender);
                    w.send_message(&self.message_sender);
                }
                ChangeKind::AliveFiltered
                | ChangeKind::NotAliveUnregistered
                | ChangeKind::NotAliveDisposedUnregistered => unimplemented!(),
            }
        }
    }
}

pub struct RemovePublicationsDiscoveryCacheChange {
    pub sequence_number: SequenceNumber,
}
impl Mail for RemovePublicationsDiscoveryCacheChange {
    type Result = ();
}
impl MailHandler<RemovePublicationsDiscoveryCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: RemovePublicationsDiscoveryCacheChange,
    ) -> <RemovePublicationsDiscoveryCacheChange as Mail>::Result {
        if let Some(w) = self
            .builtin_stateful_writer_list
            .iter_mut()
            .find(|dw| dw.guid().entity_id() == ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER)
        {
            w.remove_change(message.sequence_number);
        }
    }
}

pub struct AddSubscriptionsDiscoveryCacheChange {
    pub cache_change: CacheChange,
}
impl Mail for AddSubscriptionsDiscoveryCacheChange {
    type Result = ();
}
impl MailHandler<AddSubscriptionsDiscoveryCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: AddSubscriptionsDiscoveryCacheChange,
    ) -> <AddSubscriptionsDiscoveryCacheChange as Mail>::Result {
        if let Some(w) = self
            .builtin_stateful_writer_list
            .iter_mut()
            .find(|dw| dw.guid().entity_id() == ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER)
        {
            match message.cache_change.kind {
                ChangeKind::Alive => {
                    if let Ok(dds_subscription_data) =
                        SubscriptionBuiltinTopicData::deserialize_data(
                            message.cache_change.data_value.as_ref(),
                        )
                    {
                        if let Some(reader_proxy) = self
                            .user_defined_reader_list
                            .iter()
                            .find(|r| r.guid() == dds_subscription_data.key.value.into())
                            .map(|r| r.reader_proxy())
                        {
                            let mut cache_change = message.cache_change;
                            let discovered_reader_data = DiscoveredReaderData {
                                dds_subscription_data,
                                reader_proxy,
                            };
                            cache_change.data_value =
                                discovered_reader_data.serialize_data().unwrap().into();
                            w.add_change(cache_change, &self.message_sender);
                            w.send_message(&self.message_sender);
                        }
                    }
                }
                ChangeKind::NotAliveDisposed => {
                    w.add_change(message.cache_change, &self.message_sender);
                    w.send_message(&self.message_sender);
                }
                ChangeKind::AliveFiltered
                | ChangeKind::NotAliveUnregistered
                | ChangeKind::NotAliveDisposedUnregistered => unimplemented!(),
            }
        }
    }
}

pub struct RemoveSubscriptionsDiscoveryCacheChange {
    pub sequence_number: SequenceNumber,
}
impl Mail for RemoveSubscriptionsDiscoveryCacheChange {
    type Result = ();
}
impl MailHandler<RemoveSubscriptionsDiscoveryCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: RemoveSubscriptionsDiscoveryCacheChange,
    ) -> <RemoveSubscriptionsDiscoveryCacheChange as Mail>::Result {
        if let Some(w) = self
            .builtin_stateful_writer_list
            .iter_mut()
            .find(|dw| dw.guid().entity_id() == ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER)
        {
            w.remove_change(message.sequence_number);
        }
    }
}

pub struct AddUserDefinedCacheChange {
    pub guid: Guid,
    pub cache_change: CacheChange,
}
impl Mail for AddUserDefinedCacheChange {
    type Result = ();
}
impl MailHandler<AddUserDefinedCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: AddUserDefinedCacheChange,
    ) -> <AddUserDefinedCacheChange as Mail>::Result {
        if let Some(w) = self
            .user_defined_writer_list
            .iter_mut()
            .find(|dw| dw.guid() == message.guid)
        {
            w.add_change(message.cache_change, &self.message_sender);
            w.send_message(&self.message_sender);
        }
    }
}

pub struct RemoveUserDefinedCacheChange {
    pub guid: Guid,
    pub sequence_number: SequenceNumber,
}
impl Mail for RemoveUserDefinedCacheChange {
    type Result = ();
}
impl MailHandler<RemoveUserDefinedCacheChange> for RtpsParticipant {
    fn handle(
        &mut self,
        message: RemoveUserDefinedCacheChange,
    ) -> <RemoveUserDefinedCacheChange as Mail>::Result {
        if let Some(w) = self
            .user_defined_writer_list
            .iter_mut()
            .find(|dw| dw.guid() == message.guid)
        {
            w.remove_change(message.sequence_number);
        }
    }
}

pub struct AddDiscoveredParticipant {
    pub discovered_participant_data: SpdpDiscoveredParticipantData,
}
impl Mail for AddDiscoveredParticipant {
    type Result = ();
}
impl MailHandler<AddDiscoveredParticipant> for RtpsParticipant {
    fn handle(
        &mut self,
        message: AddDiscoveredParticipant,
    ) -> <AddDiscoveredParticipant as Mail>::Result {
        self.add_discovered_participant(&message.discovered_participant_data);
    }
}

pub struct AddDiscoveredWriter {
    pub discovered_writer_data: DiscoveredWriterData,
}
impl Mail for AddDiscoveredWriter {
    type Result = ();
}
impl MailHandler<AddDiscoveredWriter> for RtpsParticipant {
    fn handle(&mut self, message: AddDiscoveredWriter) -> <AddDiscoveredWriter as Mail>::Result {
        self.add_discovered_writer(message.discovered_writer_data);
    }
}

pub struct AddDiscoveredReader {
    pub discovered_reader_data: DiscoveredReaderData,
}
impl Mail for AddDiscoveredReader {
    type Result = ();
}
impl MailHandler<AddDiscoveredReader> for RtpsParticipant {
    fn handle(&mut self, message: AddDiscoveredReader) -> <AddDiscoveredReader as Mail>::Result {
        self.add_discovered_reader(message.discovered_reader_data);
    }
}

pub struct IsChangeAcknowledged {
    pub guid: Guid,
    pub sequence_number: SequenceNumber,
}
impl Mail for IsChangeAcknowledged {
    type Result = bool;
}
impl MailHandler<IsChangeAcknowledged> for RtpsParticipant {
    fn handle(&mut self, message: IsChangeAcknowledged) -> <IsChangeAcknowledged as Mail>::Result {
        if let Some(w) = self
            .user_defined_writer_list
            .iter_mut()
            .find(|dw| dw.guid() == message.guid)
        {
            w.is_change_acknowledged(message.sequence_number)
        } else {
            false
        }
    }
}

pub struct IsHistoricalDataReceived {
    pub guid: Guid,
}
impl Mail for IsHistoricalDataReceived {
    type Result = bool;
}
impl MailHandler<IsHistoricalDataReceived> for RtpsParticipant {
    fn handle(
        &mut self,
        message: IsHistoricalDataReceived,
    ) -> <IsHistoricalDataReceived as Mail>::Result {
        if let Some(r) = self
            .user_defined_reader_list
            .iter_mut()
            .find(|dw| dw.guid() == message.guid)
        {
            r.is_historical_data_received()
        } else {
            false
        }
    }
}
