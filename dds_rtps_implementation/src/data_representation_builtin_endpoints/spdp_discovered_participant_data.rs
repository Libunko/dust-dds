use rust_dds_api::builtin_topics::ParticipantBuiltinTopicData;
use rust_rtps_pim::{
    behavior::types::Duration, discovery::spdp::participant_proxy::ParticipantProxy,
    structure::types::Locator,
};

use crate::{
    data_representation_builtin_endpoints::parameter_id_values::{
        PID_DOMAIN_TAG, PID_EXPECTS_INLINE_QOS, PID_METATRAFFIC_UNICAST_LOCATOR,
        PID_PARTICIPANT_LEASE_DURATION, PID_USER_DATA,
    },
    data_serialize_deserialize::{
        MappingWriteByteOrdered, ParameterListSerialize, ParameterSerialize,
    },
    dds_type::DdsSerialize,
};

pub struct SpdpDiscoveredParticipantData<'a, L> {
    pub dds_participant_data: ParticipantBuiltinTopicData,
    pub participant_proxy: ParticipantProxy<'a, L>,
    pub lease_duration: Duration,
}

impl<'a> DdsSerialize for SpdpDiscoveredParticipantData<'a, Vec<Locator>> {
    fn serialize<W: std::io::Write, E: crate::dds_type::Endianness>(
        &self,
        writer: W,
    ) -> rust_dds_api::return_type::DDSResult<()> {
        let mut parameter_list: Vec<
            ParameterSerialize<Box<dyn erased_serde::Serialize + 'static>>,
        > = vec![
            ParameterSerialize::new(
                PID_PARTICIPANT_LEASE_DURATION,
                Box::new(DurationSerde(self.lease_duration)),
            ),
            ParameterSerialize::new(
                PID_DOMAIN_TAG,
                Box::new(self.participant_proxy.domain_tag.to_string()),
            ),
            ParameterSerialize::new(
                PID_EXPECTS_INLINE_QOS,
                Box::new(self.participant_proxy.expects_inline_qos),
            ),
            ParameterSerialize::new(
                PID_USER_DATA,
                Box::new(self.dds_participant_data.user_data.value),
            ),
        ];
        for metatraffic_unicast_locator in &self.participant_proxy.metatraffic_unicast_locator_list
        {
            parameter_list.push(ParameterSerialize::new(
                PID_METATRAFFIC_UNICAST_LOCATOR,
                Box::new(LocatorSerde(*metatraffic_unicast_locator)),
            ));
        }

        ParameterListSerialize(parameter_list)
            .write_ordered::<_, E>(writer)
            .unwrap();
        Ok(())
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(remote = "Duration")]
struct DurationDef {
    seconds: i32,
    fraction: u32,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct DurationSerde(#[serde(with = "DurationDef")] Duration);

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(remote = "Locator")]
struct LocatorDef {
    kind: i32,
    port: u32,
    address: [u8; 16],
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct LocatorSerde(#[serde(with = "LocatorDef")] Locator);
