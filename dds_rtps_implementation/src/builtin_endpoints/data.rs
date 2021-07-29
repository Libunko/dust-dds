use std::io::Write;

use byteorder::{ByteOrder, LittleEndian};
use cdr::{CdrLe, Infinite};
// use crate::{
//     builtin_endpoints::parameterid_list::PID_DOMAIN_ID,
//     deserialize::from_bytes_le,
//     parameter_list::{ParameterListUdp, ParameterUdp},
//     serialize::{to_bytes_le, to_writer_le},
//     submessage_elements::{
//         CountUdp, EntityIdUdp, GuidPrefixUdp, LocatorUdp, ProtocolVersionUdp, VendorIdUdp,
//     },
// };
// use byteorder::LittleEndian;
use rust_rtps_pim::{behavior::types::Duration, discovery::{
        spdp::spdp_discovered_participant_data::SPDPdiscoveredParticipantData,
        types::{BuiltinEndpointQos, BuiltinEndpointSet, DomainId},
    }, messages::{submessage_elements::{CountSubmessageElementType, EntityIdSubmessageElementType, GuidPrefixSubmessageElementType, Parameter, ProtocolVersionSubmessageElementType, VendorIdSubmessageElementType}, types::{Count, ParameterId}}, structure::types::{EntityId, GuidPrefix, Locator, ProtocolVersion, VendorId, GUID}};
use rust_rtps_udp_psm::{parameter_list::ParameterUdp, serialize::Serialize};
use serde::ser::SerializeStruct;

use crate::builtin_endpoints::parameterid_list::{PID_DOMAIN_ID,
    PID_BUILTIN_ENDPOINT_QOS, PID_BUILTIN_ENDPOINT_SET, PID_DEFAULT_MULTICAST_LOCATOR,
    PID_DEFAULT_UNICAST_LOCATOR, PID_DOMAIN_TAG, PID_EXPECTS_INLINE_QOS,
    PID_METATRAFFIC_MULTICAST_LOCATOR, PID_METATRAFFIC_UNICAST_LOCATOR, PID_PARTICIPANT_GUID,
    PID_PARTICIPANT_LEASE_DURATION, PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, PID_PROTOCOL_VERSION,
    PID_VENDORID,
};
use super::serde_derives::{ProtocolVersionDef, GuidDef, LocatorDef, CountDef, DurationDef};

const PL_CDR_LE: [u8; 4] = [0x00, 0x03, 0x00, 0x00];

#[derive(PartialEq, Debug, serde::Serialize, serde::Deserialize)]
struct ParticipantProxy {
    domain_id: u32,
    domain_tag: String,
    #[serde(with = "ProtocolVersionDef")]
    protocol_version: ProtocolVersion,
    #[serde(with = "GuidDef")]
    guid: GUID,
    vendor_id: VendorId,
    expects_inline_qos: bool,
    // #[serde(with = "LocatorDef")]
    // metatraffic_unicast_locator_list: Vec<Locator>,
    // #[serde(with = "LocatorDef")]
    // metatraffic_multicast_locator_list: Vec<Locator>,
    // #[serde(with = "LocatorDef")]
    // default_unicast_locator_list: Vec<Locator>,
    // #[serde(with = "LocatorDef")]
    // default_multicast_locator_list: Vec<Locator>,
    available_builtin_endpoints: u32,
    #[serde(with = "CountDef")]
    manual_liveliness_count: Count,
    builtin_endpoint_qos: u32,
}

#[derive(PartialEq, Debug, serde::Deserialize)]
pub struct SPDPdiscoveredParticipantDataCdr {
    // ddsParticipantData: DDS::ParticipantBuiltinTopicData,
    participant_proxy: ParticipantProxy,
    #[serde(with = "DurationDef")]
    lease_duration: Duration,
}

fn cdr_parameter<T: serde::Serialize, W: Write>(parameter_id: u16, value: &T, mut writer: W) -> rust_rtps_udp_psm::serialize::Result {
    let mut cdr_writer = Vec::new();
    let mut serializer = cdr::Serializer::<_, LittleEndian>::new(&mut cdr_writer);
    serde::Serialize::serialize(value, &mut serializer).unwrap();

    let parameter = Parameter::new(ParameterId(parameter_id), cdr_writer.as_slice());
    ParameterUdp::from(&parameter).serialize::<_,LittleEndian>(&mut writer)
}

impl rust_rtps_udp_psm::serialize::Serialize for SPDPdiscoveredParticipantDataCdr {
    fn serialize<W, B>(&self, mut writer: W) -> rust_rtps_udp_psm::serialize::Result where W: Write, B: ByteOrder {
        writer.write(PL_CDR_LE.as_ref())?;
        cdr_parameter(PID_DOMAIN_ID, &self.participant_proxy.domain_id, &mut writer)?;
        if &self.participant_proxy.domain_tag != &Self::DEFAULT_DOMAIN_TAG {
            cdr_parameter(PID_DOMAIN_TAG, &self.participant_proxy.domain_tag, &mut writer)?;
        }
        //ProtocolVersionDef::serialize(&self.participant_proxy.protocol_version, serializer)
        let mut cdr_writer = Vec::new();
        let mut serializer = cdr::Serializer::<_, LittleEndian>::new(&mut cdr_writer);
        ProtocolVersionDef::serialize(&self.participant_proxy.protocol_version, &mut serializer).unwrap();
        let parameter = Parameter::new(ParameterId(PID_PROTOCOL_VERSION), cdr_writer.as_slice());
        ParameterUdp::from(&parameter).serialize::<_,LittleEndian>(&mut writer)?;

        // cdr_parameter(PID_PROTOCOL_VERSION, &self.participant_proxy.protocol_version, &mut writer)?;

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.guid)?;
        // parameter.push(ParameterUdp::new(PID_PARTICIPANT_GUID, v).into());

        // let v = &to_bytes_le(&self.participant_proxy.vendor_id)?;
        // parameter.push(ParameterUdp::new(PID_VENDORID, v).into());

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.expects_inline_qos)?;
        // if &self.participant_proxy.expects_inline_qos != &Self::DEFAULT_EXPECTS_INLINE_QOS {
        //     parameter.push(ParameterUdp::new(PID_EXPECTS_INLINE_QOS, v).into());
        // }

        // let mut serialized_locators = vec![];
        // for metatraffic_unicast_locator in &self.participant_proxy.metatraffic_unicast_locator_list
        // {
        //     serialized_locators.push(rust_serde_cdr::serializer::to_bytes(
        //         &metatraffic_unicast_locator,
        //     )?);
        // }
        // for serialized_locator in &serialized_locators {
        //     let p = ParameterUdp::new(PID_METATRAFFIC_UNICAST_LOCATOR, &serialized_locator);
        //     parameter.push(p.into());
        // }

        // let mut serialized_locators = vec![];
        // for metatraffic_unicast_locator in
        //     &self.participant_proxy.metatraffic_multicast_locator_list
        // {
        //     serialized_locators.push(rust_serde_cdr::serializer::to_bytes(
        //         &metatraffic_unicast_locator,
        //     )?);
        // }
        // for serialized_locator in &serialized_locators {
        //     let p = ParameterUdp::new(PID_METATRAFFIC_MULTICAST_LOCATOR, &serialized_locator);
        //     parameter.push(p.into());
        // }

        // let mut serialized_locators = vec![];
        // for metatraffic_unicast_locator in &self.participant_proxy.default_unicast_locator_list {
        //     serialized_locators.push(rust_serde_cdr::serializer::to_bytes(
        //         &metatraffic_unicast_locator,
        //     )?);
        // }
        // for serialized_locator in &serialized_locators {
        //     let p = ParameterUdp::new(PID_DEFAULT_UNICAST_LOCATOR, &serialized_locator);
        //     parameter.push(p.into());
        // }

        // let mut serialized_locators = vec![];
        // for metatraffic_unicast_locator in &self.participant_proxy.default_multicast_locator_list {
        //     serialized_locators.push(rust_serde_cdr::serializer::to_bytes(
        //         &metatraffic_unicast_locator,
        //     )?);
        // }
        // for serialized_locator in &serialized_locators {
        //     let p = ParameterUdp::new(PID_DEFAULT_MULTICAST_LOCATOR, &serialized_locator);
        //     parameter.push(p.into());
        // }

        // let v = &rust_serde_cdr::serializer::to_bytes(
        //     &self.participant_proxy.available_builtin_endpoints,
        // )?;
        // parameter.push(ParameterUdp::new(PID_BUILTIN_ENDPOINT_SET, v).into());

        // let v =
        //     &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.manual_liveliness_count)?;
        // parameter.push(ParameterUdp::new(
        //     PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT,
        //     v,
        // ).into());

        // let v =
        //     &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.builtin_endpoint_qos)?;
        // if &self.participant_proxy.builtin_endpoint_qos != &Self::DEFAULT_BUILTIN_ENDPOINT_QOS {
        //     parameter.push(ParameterUdp::new(PID_BUILTIN_ENDPOINT_QOS, v).into());
        // }

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.lease_duration)?;
        // if &self.lease_duration != &Self::DEFAULT_PARTICIPANT_LEASE_DURATION {
        //     parameter.push(ParameterUdp::new(PID_PARTICIPANT_LEASE_DURATION, v).into());
        // }
        Ok(())
    }
}

impl SPDPdiscoveredParticipantDataCdr {
    // Constant value from Table 9.14 - ParameterId mapping and default values
    const DEFAULT_DOMAIN_TAG: String = String::new();
    const DEFAULT_EXPECTS_INLINE_QOS: bool = false;
    const DEFAULT_BUILTIN_ENDPOINT_QOS: u32 = 0;
    const DEFAULT_PARTICIPANT_LEASE_DURATION: Duration = Duration {
        seconds: 100,
        fraction: 0,
    };

    pub fn new(
        domain_id: &DomainId,
        domain_tag: &str,
        protocol_version: &ProtocolVersion,
        guid: &GUID,
        vendor_id: &VendorId,
        expects_inline_qos: &bool,
        metatraffic_unicast_locator_list: &[Locator],
        metatraffic_multicast_locator_list: &[Locator],
        default_unicast_locator_list: &[Locator],
        default_multicast_locator_list: &[Locator],
        available_builtin_endpoints: &BuiltinEndpointSet,
        manual_liveliness_count: &Count,
        builtin_endpoint_qos: &BuiltinEndpointQos,
        lease_duration: &Duration,
    ) -> Self {
        Self {
            participant_proxy: ParticipantProxy {
                domain_id: *domain_id,
                domain_tag: domain_tag.to_owned(),
                protocol_version: *protocol_version,
                guid: *guid,
                vendor_id: *vendor_id,
                expects_inline_qos: *expects_inline_qos,
                // metatraffic_unicast_locator_list: metatraffic_unicast_locator_list.to_vec(),
                // metatraffic_multicast_locator_list: metatraffic_multicast_locator_list.to_vec(),
                // default_unicast_locator_list: default_unicast_locator_list.to_vec(),
                // default_multicast_locator_list: default_multicast_locator_list.to_vec(),
                available_builtin_endpoints: available_builtin_endpoints.0,
                manual_liveliness_count: *manual_liveliness_count,
                builtin_endpoint_qos: builtin_endpoint_qos.0,
            },
            lease_duration: *lease_duration,
        }
    }

    // pub fn to_bytes(&self) -> Result<Vec<u8>, rust_serde_cdr::error::Error> {
        // let mut parameter = Vec::new();

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.domain_id)?;
        // parameter.push(ParameterUdp::new(PID_DOMAIN_ID, v).into());

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.domain_tag)?;
        // if &self.participant_proxy.domain_tag != &Self::DEFAULT_DOMAIN_TAG {
        //     parameter.push(ParameterUdp::new(PID_DOMAIN_TAG, v).into());
        // }

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.protocol_version)?;
        // parameter.push(ParameterUdp::new(PID_PROTOCOL_VERSION, v).into());

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.guid)?;
        // parameter.push(ParameterUdp::new(PID_PARTICIPANT_GUID, v).into());

        // let v = &to_bytes_le(&self.participant_proxy.vendor_id)?;
        // parameter.push(ParameterUdp::new(PID_VENDORID, v).into());

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.expects_inline_qos)?;
        // if &self.participant_proxy.expects_inline_qos != &Self::DEFAULT_EXPECTS_INLINE_QOS {
        //     parameter.push(ParameterUdp::new(PID_EXPECTS_INLINE_QOS, v).into());
        // }

        // let mut serialized_locators = vec![];
        // for metatraffic_unicast_locator in &self.participant_proxy.metatraffic_unicast_locator_list
        // {
        //     serialized_locators.push(rust_serde_cdr::serializer::to_bytes(
        //         &metatraffic_unicast_locator,
        //     )?);
        // }
        // for serialized_locator in &serialized_locators {
        //     let p = ParameterUdp::new(PID_METATRAFFIC_UNICAST_LOCATOR, &serialized_locator);
        //     parameter.push(p.into());
        // }

        // let mut serialized_locators = vec![];
        // for metatraffic_unicast_locator in
        //     &self.participant_proxy.metatraffic_multicast_locator_list
        // {
        //     serialized_locators.push(rust_serde_cdr::serializer::to_bytes(
        //         &metatraffic_unicast_locator,
        //     )?);
        // }
        // for serialized_locator in &serialized_locators {
        //     let p = ParameterUdp::new(PID_METATRAFFIC_MULTICAST_LOCATOR, &serialized_locator);
        //     parameter.push(p.into());
        // }

        // let mut serialized_locators = vec![];
        // for metatraffic_unicast_locator in &self.participant_proxy.default_unicast_locator_list {
        //     serialized_locators.push(rust_serde_cdr::serializer::to_bytes(
        //         &metatraffic_unicast_locator,
        //     )?);
        // }
        // for serialized_locator in &serialized_locators {
        //     let p = ParameterUdp::new(PID_DEFAULT_UNICAST_LOCATOR, &serialized_locator);
        //     parameter.push(p.into());
        // }

        // let mut serialized_locators = vec![];
        // for metatraffic_unicast_locator in &self.participant_proxy.default_multicast_locator_list {
        //     serialized_locators.push(rust_serde_cdr::serializer::to_bytes(
        //         &metatraffic_unicast_locator,
        //     )?);
        // }
        // for serialized_locator in &serialized_locators {
        //     let p = ParameterUdp::new(PID_DEFAULT_MULTICAST_LOCATOR, &serialized_locator);
        //     parameter.push(p.into());
        // }

        // let v = &rust_serde_cdr::serializer::to_bytes(
        //     &self.participant_proxy.available_builtin_endpoints,
        // )?;
        // parameter.push(ParameterUdp::new(PID_BUILTIN_ENDPOINT_SET, v).into());

        // let v =
        //     &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.manual_liveliness_count)?;
        // parameter.push(ParameterUdp::new(
        //     PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT,
        //     v,
        // ).into());

        // let v =
        //     &rust_serde_cdr::serializer::to_bytes(&self.participant_proxy.builtin_endpoint_qos)?;
        // if &self.participant_proxy.builtin_endpoint_qos != &Self::DEFAULT_BUILTIN_ENDPOINT_QOS {
        //     parameter.push(ParameterUdp::new(PID_BUILTIN_ENDPOINT_QOS, v).into());
        // }

        // let v = &rust_serde_cdr::serializer::to_bytes(&self.lease_duration)?;
        // if &self.lease_duration != &Self::DEFAULT_PARTICIPANT_LEASE_DURATION {
        //     parameter.push(ParameterUdp::new(PID_PARTICIPANT_LEASE_DURATION, v).into());
        // }

        // let mut bytes = PL_CDR_LE.to_vec();
        // to_writer_le(&ParameterListUdp { parameter }, &mut bytes)
        //     .unwrap();
        // Ok(bytes)
        // todo!()
    // }

    // pub fn from_bytes(buf: &[u8]) -> Result<Self, rust_serde_cdr::error::Error> {
        // let _representation: [u8; 4] = rust_serde_cdr::deserializer::from_bytes(&buf[0..4])?;
        // let parameter_list: ParameterListUdp = from_bytes_le(&buf[4..])?;

        // let domain_id =
        //     parameter_list
        //         .get(PID_DOMAIN_ID)
        //         .ok_or(rust_serde_cdr::error::Error::Message(
        //             "Missing PID_DOMAIN_ID parameter".to_string(),
        //         ))?;

        // let domain_tag = parameter_list
        //     .get(PID_DOMAIN_TAG)
        //     .unwrap_or(Self::DEFAULT_DOMAIN_TAG);

        // let protocol_version: ProtocolVersionUdp = parameter_list.get(PID_PROTOCOL_VERSION).ok_or(
        //     rust_serde_cdr::error::Error::Message(
        //         "Missing PID_PROTOCOL_VERSION parameter".to_string(),
        //     ),
        // )?;

        // let guid: GUIDUdp = parameter_list.get(PID_PARTICIPANT_GUID).ok_or(
        //     rust_serde_cdr::error::Error::Message(
        //         "Missing PID_PARTICIPANT_GUID parameter".to_string(),
        //     ),
        // )?;

        // let vendor_id: VendorIdUdp =
        //     parameter_list
        //         .get(PID_VENDORID)
        //         .ok_or(rust_serde_cdr::error::Error::Message(
        //             "Missing PID_VENDORID parameter".to_string(),
        //         ))?;

        // let expects_inline_qos = parameter_list
        //     .get(PID_EXPECTS_INLINE_QOS)
        //     .unwrap_or(Self::DEFAULT_EXPECTS_INLINE_QOS);

        // let metatraffic_unicast_locator_list =
        //     parameter_list.get_list(PID_METATRAFFIC_UNICAST_LOCATOR);

        // let metatraffic_multicast_locator_list =
        //     parameter_list.get_list(PID_METATRAFFIC_MULTICAST_LOCATOR);

        // let default_unicast_locator_list = parameter_list.get_list(PID_DEFAULT_UNICAST_LOCATOR);

        // let default_multicast_locator_list = parameter_list.get_list(PID_DEFAULT_MULTICAST_LOCATOR);

        // let available_builtin_endpoints = parameter_list.get(PID_BUILTIN_ENDPOINT_SET).ok_or(
        //     rust_serde_cdr::error::Error::Message(
        //         "Missing PID_BUILTIN_ENDPOINT_SET parameter".to_string(),
        //     ),
        // )?;

        // let manual_liveliness_count = parameter_list
        //     .get(PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT)
        //     .ok_or(rust_serde_cdr::error::Error::Message(
        //         "Missing PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT parameter".to_string(),
        //     ))?;

        // let builtin_endpoint_qos = parameter_list
        //     .get(PID_BUILTIN_ENDPOINT_QOS)
        //     .unwrap_or(Self::DEFAULT_BUILTIN_ENDPOINT_QOS);

        // let lease_duration = parameter_list
        //     .get(PID_PARTICIPANT_LEASE_DURATION)
        //     .unwrap_or(Self::DEFAULT_PARTICIPANT_LEASE_DURATION);

        // let participant_proxy = ParticipantProxy {
        //     domain_id,
        //     domain_tag,
        //     protocol_version,
        //     guid,
        //     vendor_id,
        //     expects_inline_qos,
        //     metatraffic_unicast_locator_list,
        //     metatraffic_multicast_locator_list,
        //     default_unicast_locator_list,
        //     default_multicast_locator_list,
        //     available_builtin_endpoints,
        //     manual_liveliness_count,
        //     builtin_endpoint_qos,
        // };

        // Ok(Self {
        //     participant_proxy: participant_proxy,
        //     lease_duration: lease_duration,
        // })
        // todo!()
    // }
}

impl SPDPdiscoveredParticipantData for SPDPdiscoveredParticipantDataCdr {
    type LocatorListType = Vec<Locator>;

    fn domain_id(&self) -> DomainId {
        self.participant_proxy.domain_id
    }

    fn domain_tag(&self) -> &str {
        &self.participant_proxy.domain_tag
    }

    fn protocol_version(&self) -> ProtocolVersion {
        self.participant_proxy.protocol_version
    }

    fn guid_prefix(&self) -> GuidPrefix {
        *self.participant_proxy.guid.prefix()
    }

    fn vendor_id(&self) -> VendorId {
        self.participant_proxy.vendor_id
    }

    fn expects_inline_qos(&self) -> bool {
        self.participant_proxy.expects_inline_qos
    }

    fn metatraffic_unicast_locator_list(&self) -> Self::LocatorListType {
        // self.participant_proxy
        //     .metatraffic_unicast_locator_list
        //     .clone()
        todo!()
    }

    fn metatraffic_multicast_locator_list(&self) -> Self::LocatorListType {
        // self.participant_proxy
        //     .metatraffic_multicast_locator_list
        //     .clone()
        todo!()
    }

    fn default_unicast_locator_list(&self) -> Self::LocatorListType {
        //self.participant_proxy.default_unicast_locator_list.clone()
        todo!()
    }

    fn default_multicast_locator_list(&self) -> Self::LocatorListType {
        // self.participant_proxy
        //     .default_multicast_locator_list
        //     .clone()
        todo!()
    }

    fn available_builtin_endpoints(&self) -> BuiltinEndpointSet {
        BuiltinEndpointSet(self.participant_proxy.available_builtin_endpoints)
    }

    fn manual_liveliness_count(&self) -> Count {
        self.participant_proxy.manual_liveliness_count
    }

    fn builtin_endpoint_qos(&self) -> BuiltinEndpointQos {
        BuiltinEndpointQos(self.participant_proxy.builtin_endpoint_qos)
    }
}

#[cfg(test)]
mod tests {
    use cdr::{CdrLe, Infinite, PlCdrLe};
    use rust_rtps_pim::structure::types::ENTITYID_PARTICIPANT;

    use super::*;

    #[test]
    pub fn serialize_complete_spdp_discovered_participant_data() {
        let locator1 = Locator::new(1, 1, [1; 16]);
        let locator2 = Locator::new(2, 2, [2; 16]);

        let domain_id = 1;
        let domain_tag = "abc";
        let protocol_version = ProtocolVersion { major: 2, minor: 4 };
        let guid = GUID::new([1; 12], ENTITYID_PARTICIPANT);
        let vendor_id = [9, 9];
        let expects_inline_qos = true;
        let metatraffic_unicast_locator_list = &[locator1, locator2];
        let metatraffic_multicast_locator_list = &[locator1, locator2];
        let default_unicast_locator_list = &[locator1, locator2];
        let default_multicast_locator_list = &[locator1, locator2];
        let available_builtin_endpoints =
            BuiltinEndpointSet::new(BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR);
        let manual_liveliness_count = Count(2);
        let builtin_endpoint_qos = BuiltinEndpointQos::new(
            BuiltinEndpointQos::BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER,
        );
        let lease_duration = Duration {
            seconds: 10,
            fraction: 0,
        };

        let spdp_discovered_participant_data = SPDPdiscoveredParticipantDataCdr::new(
            &domain_id,
            domain_tag,
            &protocol_version,
            &guid,
            &vendor_id,
            &expects_inline_qos,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
            &available_builtin_endpoints,
            &manual_liveliness_count,
            &builtin_endpoint_qos,
            &lease_duration,
        );

        let serialized_data = rust_rtps_udp_psm::serialize::to_bytes_le(&spdp_discovered_participant_data).unwrap();
        let expected_data = vec![
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x0f, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0x00, 0x00, // DomainId(1)
            0x14, 0x40, 0x08, 0x00, // PID_DOMAIN_TAG, Length: 8
            0x04, 0x00, 0x00, 0x00, // DomainTag(length: 4)
            b'a', b'b', b'c', 0x00, // DomainTag('abc')
            0x15, 0x00, 0x04, 0x00, // PID_PROTOCOL_VERSION, Length: 4
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion{major:2, minor:4}
            0x50, 0x00, 0x10, 0x00, // PID_PARTICIPANT_GUID, Length: 16
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x00, 0x00, 0x01, 0xc1, // EntityId(ENTITYID_PARTICIPANT)
            0x16, 0x00, 0x04, 0x00, // PID_VENDORID, Length:4,
            0x09, 0x09, 0x00, 0x00, // VendorId([9,9])
            0x43, 0x00, 0x04, 0x00, // PID_EXPECTS_INLINE_QOS, Length: 4,
            0x01, 0x00, 0x00, 0x00, // True
            0x32, 0x00, 0x18, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR, Length: 24,
            0x01, 0x00, 0x00, 0x00, // Locator{kind:1
            0x01, 0x00, 0x00, 0x00, // port:1,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address: [1;16]
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x32, 0x00, 0x18, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR, Length: 24,
            0x02, 0x00, 0x00, 0x00, // Locator{kind:2
            0x02, 0x00, 0x00, 0x00, // port:2,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address: [2;16]
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            0x33, 0x00, 0x18, 0x00, // PID_METATRAFFIC_MULTICAST_LOCATOR, Length: 24,
            0x01, 0x00, 0x00, 0x00, // Locator{kind:1
            0x01, 0x00, 0x00, 0x00, // port:1,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address: [1;16]
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x33, 0x00, 0x18, 0x00, // PID_METATRAFFIC_MULTICAST_LOCATOR, Length: 24,
            0x02, 0x00, 0x00, 0x00, // Locator{kind:2
            0x02, 0x00, 0x00, 0x00, // port:2,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address: [2;16]
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            0x31, 0x00, 0x18, 0x00, // PID_DEFAULT_UNICAST_LOCATOR, Length: 24,
            0x01, 0x00, 0x00, 0x00, // Locator{kind:1
            0x01, 0x00, 0x00, 0x00, // port:1,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address: [1;16]
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x31, 0x00, 0x18, 0x00, // PID_DEFAULT_UNICAST_LOCATOR, Length: 24,
            0x02, 0x00, 0x00, 0x00, // Locator{kind:2
            0x02, 0x00, 0x00, 0x00, // port:2,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address: [2;16]
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            0x48, 0x00, 0x18, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR, Length: 24,
            0x01, 0x00, 0x00, 0x00, // Locator{kind:1
            0x01, 0x00, 0x00, 0x00, // port:1,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address: [1;16]
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x48, 0x00, 0x18, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR, Length: 24,
            0x02, 0x00, 0x00, 0x00, // Locator{kind:2
            0x02, 0x00, 0x00, 0x00, // port:2,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address: [2;16]
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            0x58, 0x00, 0x04, 0x00, // PID_BUILTIN_ENDPOINT_SET, Length: 4
            0x02, 0x00, 0x00, 0x00, // BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
            0x34, 0x00, 0x04, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, Length: 4
            0x02, 0x00, 0x00, 0x00, // Count(2)
            0x77, 0x00, 0x04, 0x00, // PID_BUILTIN_ENDPOINT_QOS, Length: 4
            0x00, 0x00, 0x00, 0x20, // BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER
            0x02, 0x00, 0x08, 0x00, // PID_PARTICIPANT_LEASE_DURATION, Length: 8
            0x0a, 0x00, 0x00, 0x00, // Duration{seconds:30,
            0x00, 0x00, 0x00, 0x00, //          fraction:0}
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ];

        assert_eq!(serialized_data, expected_data);
    }

    #[test]
    pub fn serialize_spdp_default_data() {
        let domain_id = 1;
        let domain_tag = &SPDPdiscoveredParticipantDataCdr::DEFAULT_DOMAIN_TAG;
        let protocol_version = ProtocolVersion { major: 2, minor: 4 };
        let guid = GUID::new([1; 12], ENTITYID_PARTICIPANT);
        let vendor_id = [9, 9];
        let expects_inline_qos = SPDPdiscoveredParticipantDataCdr::DEFAULT_EXPECTS_INLINE_QOS;
        let metatraffic_unicast_locator_list = &[];
        let metatraffic_multicast_locator_list = &[];
        let default_unicast_locator_list = &[];
        let default_multicast_locator_list = &[];
        let available_builtin_endpoints =
            BuiltinEndpointSet::new(BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR);
        let manual_liveliness_count = Count(2);
        let builtin_endpoint_qos = BuiltinEndpointQos::new(0);
        let lease_duration =
            SPDPdiscoveredParticipantDataCdr::DEFAULT_PARTICIPANT_LEASE_DURATION;

        let spdp_discovered_participant_data = SPDPdiscoveredParticipantDataCdr::new(
            &domain_id,
            domain_tag,
            &protocol_version,
            &guid,
            &vendor_id,
            &expects_inline_qos,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
            &available_builtin_endpoints,
            &manual_liveliness_count,
            &builtin_endpoint_qos,
            &lease_duration,
        );

        let serialized_data = rust_rtps_udp_psm::serialize::to_bytes_le(&spdp_discovered_participant_data).unwrap();
        let expected_data = vec![
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x0f, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0x00, 0x00, // DomainId(1)
            0x15, 0x00, 0x04, 0x00, // PID_PROTOCOL_VERSION, Length: 4
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion{major:2, minor:4}
            0x50, 0x00, 0x10, 0x00, // PID_PARTICIPANT_GUID, Length: 16
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x00, 0x00, 0x01, 0xc1, // EntityId(ENTITYID_PARTICIPANT)
            0x16, 0x00, 0x04, 0x00, // PID_VENDORID, Length:4,
            0x09, 0x09, 0x00, 0x00, // VendorId([9,9])
            0x58, 0x00, 0x04, 0x00, // PID_BUILTIN_ENDPOINT_SET, Length: 4
            0x02, 0x00, 0x00, 0x00, // BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
            0x34, 0x00, 0x04, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, Length: 4
            0x02, 0x00, 0x00, 0x00, // Count(2)
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ];

        assert_eq!(serialized_data, expected_data);
    }

    #[test]
    fn deserialize_complete_spdp_discovered_participant_data() {
        #[rustfmt::skip]
        let spdp_discovered_participant_data = cdr::deserialize::<SPDPdiscoveredParticipantDataCdr>(&[
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x0f, 0x00, 4, 0x00,    // PID_DOMAIN_ID, Length
            0x01, 0x00, 0x00, 0x00, // DomainId
            0x14, 0x40, 8, 0x00,    // PID_DOMAIN_TAG, Length
            4,    0,    0,    0,             // Length: 4
            b'a', b'b', b'c', 0x00, // DomainTag
            0x15, 0x00, 4, 0x00,    // PID_PROTOCOL_VERSION, Length
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion: major, minor
            0x50, 0x00, 16, 0x00,   // PID_PARTICIPANT_GUID, Length
            0x01, 0x01, 0x01, 0x01, // GuidPrefix
            0x01, 0x01, 0x01, 0x01, // GuidPrefix
            0x01, 0x01, 0x01, 0x01, // GuidPrefix
            0x00, 0x00, 0x01, 0xc1, // EntityId(ENTITYID_PARTICIPANT)
            0x16, 0x00, 4, 0x00,    // PID_VENDORID, Length
            0x09, 0x09, 0x00, 0x00, // VendorId
            0x43, 0x00, 4, 0x00,    // PID_EXPECTS_INLINE_QOS, Length
            0x01, 0x00, 0x00, 0x00, // True
            0x32, 0x00, 24, 0x00,   // PID_METATRAFFIC_UNICAST_LOCATOR, Length
            0x01, 0x00, 0x00, 0x00, // Locator: kind
            0x01, 0x00, 0x00, 0x00, // Locator: port
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x32, 0x00, 24, 0x00,   // PID_METATRAFFIC_UNICAST_LOCATOR, Length
            0x02, 0x00, 0x00, 0x00, // Locator: kind
            0x02, 0x00, 0x00, 0x00, // Locator: port
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x33, 0x00, 24, 0x00,   // PID_METATRAFFIC_MULTICAST_LOCATOR, Length
            0x01, 0x00, 0x00, 0x00, // Locator: kind
            0x01, 0x00, 0x00, 0x00, // Locator: port
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x33, 0x00, 24, 0x00,   // PID_METATRAFFIC_MULTICAST_LOCATOR, Length
            0x02, 0x00, 0x00, 0x00, // Locator: kind
            0x02, 0x00, 0x00, 0x00, // Locator: port,
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x31, 0x00, 24, 0x00,   // PID_DEFAULT_UNICAST_LOCATOR, Length
            0x01, 0x00, 0x00, 0x00, // Locator: kind
            0x01, 0x00, 0x00, 0x00, // Locator: port
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x31, 0x00, 24, 0x00,   // PID_DEFAULT_UNICAST_LOCATOR, Length
            0x02, 0x00, 0x00, 0x00, // Locator: kind
            0x02, 0x00, 0x00, 0x00, // Locator: port
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x48, 0x00, 24, 0x00,   // PID_DEFAULT_MULTICAST_LOCATOR, Length
            0x01, 0x00, 0x00, 0x00, // Locator: kind
            0x01, 0x00, 0x00, 0x00, // Locator: port
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x01, 0x01, 0x01, 0x01, // Locator: address
            0x48, 0x00, 024, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR, Length,
            0x02, 0x00, 0x00, 0x00, // Locator: kind
            0x02, 0x00, 0x00, 0x00, // Locator: port
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x02, 0x02, 0x02, 0x02, // Locator: address
            0x58, 0x00, 4, 0x00,    // PID_BUILTIN_ENDPOINT_SET, Length
            0x02, 0x00, 0x00, 0x00, // BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
            0x34, 0x00, 4, 0x00,    // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, Length
            0x02, 0x00, 0x00, 0x00, // Count
            0x77, 0x00, 4, 0x00,    // PID_BUILTIN_ENDPOINT_QOS, Length: 4
            0x00, 0x00, 0x00, 0x20, // BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER
            0x02, 0x00, 8, 0x00,    // PID_PARTICIPANT_LEASE_DURATION, Length
            10, 0x00, 0x00, 0x00,   // Duration: seconds
            0x00, 0x00, 0x00, 0x00, // Duration: fraction
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length
        ]).unwrap();

        let locator1 = Locator::new(1, 1, [1; 16]);
        let locator2 = Locator::new(2, 2, [2; 16]);

        let domain_id = 1;
        let domain_tag = "abc";
        let protocol_version = ProtocolVersion { major: 2, minor: 4 };
        let guid = GUID::new([1; 12], ENTITYID_PARTICIPANT);
        let vendor_id = [9, 9];
        let expects_inline_qos = true;
        let metatraffic_unicast_locator_list = &[locator1, locator2];
        let metatraffic_multicast_locator_list = &[locator1, locator2];
        let default_unicast_locator_list = &[locator1, locator2];
        let default_multicast_locator_list = &[locator1, locator2];
        let available_builtin_endpoints =
            BuiltinEndpointSet::new(BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR);
        let manual_liveliness_count = Count(2);
        let builtin_endpoint_qos = BuiltinEndpointQos::new(
            BuiltinEndpointQos::BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER,
        );
        let lease_duration = Duration {
            seconds: 10,
            fraction: 0,
        };

        let expected_spdp_discovered_participant_data = SPDPdiscoveredParticipantDataCdr::new(
            &domain_id,
            domain_tag,
            &protocol_version,
            &guid,
            &vendor_id,
            &expects_inline_qos,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
            &available_builtin_endpoints,
            &manual_liveliness_count,
            &builtin_endpoint_qos,
            &lease_duration,
        );

        assert_eq!(
            spdp_discovered_participant_data,
            expected_spdp_discovered_participant_data
        );
    }

    #[test]
    fn deserialize_default_spdp_discovered_participant_data() {
        #[rustfmt::skip]
        let spdp_discovered_participant_data = cdr::deserialize::<SPDPdiscoveredParticipantDataCdr>(&[
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x0f, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0x00, 0x00, // DomainId(1)
            0x15, 0x00, 0x04, 0x00, // PID_PROTOCOL_VERSION, Length: 4
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion{major:2, minor:4}
            0x50, 0x00, 0x10, 0x00, // PID_PARTICIPANT_GUID, Length: 16
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x00, 0x00, 0x01, 0xc1, // EntityId(ENTITYID_PARTICIPANT)
            0x16, 0x00, 0x04, 0x00, // PID_VENDORID, Length:4,
            0x09, 0x09, 0x00, 0x00, // VendorId([9,9])
            0x58, 0x00, 0x04, 0x00, // PID_BUILTIN_ENDPOINT_SET, Length: 4
            0x02, 0x00, 0x00, 0x00, // BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
            0x34, 0x00, 0x04, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, Length: 4
            0x02, 0x00, 0x00, 0x00, // Count(2)
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ]).unwrap();

        let domain_id = 1;
        let domain_tag = &SPDPdiscoveredParticipantDataCdr::DEFAULT_DOMAIN_TAG;
        let protocol_version = ProtocolVersion { major: 2, minor: 4 };
        let guid = GUID::new([1; 12], ENTITYID_PARTICIPANT);
        let vendor_id = [9, 9];
        let expects_inline_qos = SPDPdiscoveredParticipantDataCdr::DEFAULT_EXPECTS_INLINE_QOS;
        let metatraffic_unicast_locator_list = &[];
        let metatraffic_multicast_locator_list = &[];
        let default_unicast_locator_list = &[];
        let default_multicast_locator_list = &[];
        let available_builtin_endpoints =
            BuiltinEndpointSet::new(BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR);
        let manual_liveliness_count = Count(2);
        let builtin_endpoint_qos = BuiltinEndpointQos::new(0);
        let lease_duration =
            SPDPdiscoveredParticipantDataCdr::DEFAULT_PARTICIPANT_LEASE_DURATION;

        let expected_spdp_discovered_participant_data = SPDPdiscoveredParticipantDataCdr::new(
            &domain_id,
            domain_tag,
            &protocol_version,
            &guid,
            &vendor_id,
            &expects_inline_qos,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
            &available_builtin_endpoints,
            &manual_liveliness_count,
            &builtin_endpoint_qos,
            &lease_duration,
        );

        assert_eq!(
            spdp_discovered_participant_data,
            expected_spdp_discovered_participant_data
        );
    }

    #[test]
    fn deserialize_wrong_spdp_discovered_participant_data() {
        #[rustfmt::skip]
        let spdp_discovered_participant_data_missing_protocol_version = cdr::deserialize::<SPDPdiscoveredParticipantDataCdr>(&[
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x0f, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0x00, 0x00, // DomainId(1)
            0x50, 0x00, 0x10, 0x00, // PID_PARTICIPANT_GUID, Length: 16
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x01, 0x01, 0x01, 0x01, // GuidPrefix([1;12])
            0x00, 0x00, 0x01, 0xc1, // EntityId(ENTITYID_PARTICIPANT)
            0x16, 0x00, 0x04, 0x00, // PID_VENDORID, Length:4,
            0x09, 0x09, 0x00, 0x00, // VendorId([9,9])
            0x58, 0x00, 0x04, 0x00, // PID_BUILTIN_ENDPOINT_SET, Length: 4
            0x02, 0x00, 0x00, 0x00, // BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
            0x34, 0x00, 0x04, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, Length: 4
            0x02, 0x00, 0x00, 0x00, // Count(2)
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ]);

        // match spdp_discovered_participant_data_missing_protocol_version {
        //     Result::Err(rust_serde_cdr::error::Error::Message(msg)) => {
        //         assert_eq!(&msg, "Missing PID_PROTOCOL_VERSION parameter")
        //     }
        //     _ => panic!(),
        // };

        #[rustfmt::skip]
        let spdp_discovered_participant_data_missing_participant_guid = cdr::deserialize::<SPDPdiscoveredParticipantDataCdr>(&[
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x0f, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0x00, 0x00, // DomainId(1)
            0x15, 0x00, 0x04, 0x00, // PID_PROTOCOL_VERSION, Length: 4
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion{major:2, minor:4}
            0x16, 0x00, 0x04, 0x00, // PID_VENDORID, Length:4,
            0x09, 0x09, 0x00, 0x00, // VendorId([9,9])
            0x58, 0x00, 0x04, 0x00, // PID_BUILTIN_ENDPOINT_SET, Length: 4
            0x02, 0x00, 0x00, 0x00, // BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
            0x34, 0x00, 0x04, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, Length: 4
            0x02, 0x00, 0x00, 0x00, // Count(2)
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ]);

        // match spdp_discovered_participant_data_missing_participant_guid {
        //     Result::Err(rust_serde_cdr::error::Error::Message(msg)) => {
        //         assert_eq!(&msg, "Missing PID_PARTICIPANT_GUID parameter")
        //     }
        //     _ => panic!(),
        // };
    }
}
