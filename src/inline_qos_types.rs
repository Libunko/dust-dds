/// 
/// This files shall only contain the types as listed in the DDSI-RTPS Version 2.3
/// in the sub clauses of 9.6.3 ParameterId Definitions used to Represent In-line QoS
///  
 
use std::convert::From;
use crate::types::{ChangeKind, };
use crate::messages::types::{ParameterId, };
use crate::messages::submessage_elements::Pid;

use serde::{Serialize, Deserialize};

const PID_TOPIC_NAME : ParameterId = 0x0005;
const PID_KEY_HASH : ParameterId = 0x0070;
const PID_STATUS_INFO : ParameterId = 0x0071;
  
#[derive(Debug, PartialEq, Clone, Eq, Serialize, Deserialize)]
pub struct TopicName(pub Vec<u8>);
impl Pid for TopicName {
    fn pid() -> ParameterId {
        PID_TOPIC_NAME
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, Serialize, Deserialize)]
pub struct KeyHash(pub [u8; 16]);

impl Pid for KeyHash {
    fn pid() -> ParameterId {
        PID_KEY_HASH
    }
}


#[derive(Debug, PartialEq, Clone, Copy, Eq, Serialize, Deserialize)]
pub struct StatusInfo(pub [u8;4]);

impl StatusInfo {
    const DISPOSED_FLAG_MASK : u8 = 0b0000_0001;
    const UNREGISTERED_FLAG_MASK : u8 = 0b0000_0010;
    const FILTERED_FLAG_MASK : u8 = 0b0000_0100;

    pub fn disposed_flag(&self) -> bool {
        self.0[3] & StatusInfo::DISPOSED_FLAG_MASK == StatusInfo::DISPOSED_FLAG_MASK
    }

    pub fn unregistered_flag(&self) -> bool {
        self.0[3] & StatusInfo::UNREGISTERED_FLAG_MASK == StatusInfo::UNREGISTERED_FLAG_MASK
    }

    pub fn filtered_flag(&self) -> bool {
        self.0[3] & StatusInfo::FILTERED_FLAG_MASK == StatusInfo::FILTERED_FLAG_MASK
    }
}

impl Pid for StatusInfo {
    fn pid() -> ParameterId {
        PID_STATUS_INFO
    }
}

impl From<ChangeKind> for StatusInfo {
    fn from(change_kind: ChangeKind) -> Self {
        match change_kind {
            ChangeKind::Alive => StatusInfo([0,0,0,0]),
            ChangeKind::NotAliveDisposed => StatusInfo([0,0,0,StatusInfo::DISPOSED_FLAG_MASK]),
            ChangeKind::NotAliveUnregistered => StatusInfo([0,0,0,StatusInfo::UNREGISTERED_FLAG_MASK]),
            ChangeKind::AliveFiltered => StatusInfo([0,0,0,StatusInfo::FILTERED_FLAG_MASK]),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;


    ///////////////////////// StatusInfo Tests ////////////////////////
    #[test]
    fn test_status_info_change_kind_conversions() {
        assert_eq!(ChangeKind::try_from(StatusInfo::from(ChangeKind::Alive)).unwrap(), ChangeKind::Alive);
        assert_eq!(ChangeKind::try_from(StatusInfo::from(ChangeKind::AliveFiltered)).unwrap(), ChangeKind::AliveFiltered);
        assert_eq!(ChangeKind::try_from(StatusInfo::from(ChangeKind::NotAliveUnregistered)).unwrap(), ChangeKind::NotAliveUnregistered);
        assert_eq!(ChangeKind::try_from(StatusInfo::from(ChangeKind::NotAliveDisposed)).unwrap(), ChangeKind::NotAliveDisposed);
    }
}
