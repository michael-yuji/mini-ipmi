
use paste::paste;
use macros::*;

use crate::ipmi::summon_from_bytes;
use crate::ipmi::{BytesDeserializable, BytesSerializationSized, BytesSerializable};
use crate::ipmi::Error;

pub trait IpmiCommand: core::marker::Sized {
    fn from_data(data: &crate::ipmi::ipmi::IpmiData) -> Option<Self>;
    fn from_message(msg: &crate::ipmi::ipmi::IpmiMessage) -> Option<Self>;
}

macro_rules! ipmi_cmd {
    ($netfn:expr, $cmd:expr, $name:ident, $req:ty, $res:ty) => {
        #[derive(Debug, Eq, PartialEq)]
        pub enum $name {
            Request($req),
            Response(u8, $res)
        }

        impl IpmiCommand for $name {
            fn from_data(data: &crate::ipmi::ipmi::IpmiData) -> Option<Self> {
                match data {
                    crate::ipmi::ipmi::IpmiData::Request(dat) => {
                        <$req>::from_bytes(dat, true).ok()
                            .map(|req| Self::Request(req))
                    },
                    crate::ipmi::ipmi::IpmiData::Response(code, dat) => {
                        <$res>::from_bytes(dat, true).ok()
                            .map(|res| Self::Response(*code, res))
                    }
                }
            }

            fn from_message(msg: &crate::ipmi::ipmi::IpmiMessage) -> Option<Self>
            {
                let netfn = if msg.netfn % 2 == 0 { 
                    msg.netfn
                } else {
                    msg.netfn - 1
                };

                if msg.cmd != $cmd || netfn != $netfn { return None; }

                Self::from_data(&msg.data)
            }
        }
    };
    ($netfn:expr, $cmd:expr, $name:ident) => {
        paste! {
            ipmi_cmd!($netfn, $cmd, $name, [<$name Request>], [<$name Response>]);
        }
    };
}

ipmi_cmd!(0x06, 0x38, GetChannelAuthCap);
ipmi_cmd!(0x06, 0x39, GetSessionChallenge);
ipmi_cmd!(0x06, 0x3a, ActivateSession);
ipmi_cmd!(0x06, 0x3b, SetSessionPrivLevel);

#[derive(Debug, PartialEq, Eq, BytesSerializationSized, BytesSerializable, BytesDeserializable)]
pub struct GetChannelAuthCapRequest {
    pub channel_number: u8,
    pub max_priv_level: u8
}

#[derive(Debug, PartialEq, Eq, BytesSerializationSized, BytesSerializable, BytesDeserializable)]
pub struct GetChannelAuthCapResponse {
    pub channel_number: u8,
    pub auth_types: u8,
    pub auth_caps: u8,
    pub ipmi2_ext: u8,
    pub oem_id: [u8; 3],
    pub oem_aux: u8
}

#[derive(Debug, PartialEq, Eq, BytesSerializationSized, BytesSerializable, BytesDeserializable)]
pub struct GetSessionChallengeRequest {
    pub auth_type: u8,
    pub username: [u8;16]
}

#[derive(Debug, PartialEq, Eq, BytesSerializationSized, BytesSerializable, BytesDeserializable)]
pub struct GetSessionChallengeResponse
{
    #[bytes_serialize(endian = "le")]
    pub tmp_session_id: u32,
    pub challenge_dat: [u8;16]
}

#[derive(Debug, PartialEq, Eq, BytesSerializationSized, BytesSerializable, BytesDeserializable)]
pub struct ActivateSessionRequest {
    pub auth_type: u8,
    pub max_priv_level: u8,
    pub challenge_string: [u8; 16],
    #[bytes_serialize(endian = "le")]
    pub init_outbound_seq: u32
}

#[derive(Debug, PartialEq, Eq, BytesSerializationSized, BytesSerializable, BytesDeserializable)]
pub struct ActivateSessionResponse {
    pub auth_type: u8,

    #[bytes_serialize(endian = "le")]
    pub session_id: u32,

    #[bytes_serialize(endian = "le")]
    pub init_inbound_seq: u32,

    pub max_priv_level: u8
}

#[derive(Debug, PartialEq, Eq, BytesSerializationSized, BytesSerializable, BytesDeserializable)]
pub struct SetSessionPrivLevelRequest {
    pub priv_level: u8
}

#[derive(Debug, PartialEq, Eq, BytesSerializationSized, BytesSerializable, BytesDeserializable)]
pub struct SetSessionPrivLevelResponse {
    pub priv_level: u8
}
