
use crate::{take_be_u32, take_u8};
use crate::ipmi::*;

pub const ASF_IANA: u32 = 4542;
pub const ENTITY_IPMI: u8 = 0b10000000;
pub const ENTITY_ASF1: u8 = 0b00000001;

pub const INTERACTION_RMCP_SEC_EXT: u8 = 0b10000000;
pub const INTERACTION_DMTF_DASH:    u8 = 0b00100000;

pub const ASF_MSG_TYPE_PING: u8 = 0x80;
pub const ASF_MSG_TYPE_PONG: u8 = 0x40;

#[derive(PartialEq, Eq, Debug)]
pub struct AsfMessage<'a> {
    pub iana: u32,
    pub msg_type: u8,
    pub msg_tag:  u8,
    pub data_len:  u8,
    pub data: AsfData<'a>

}

#[derive(PartialEq, Eq, Debug)]
pub enum AsfData<'a> {
    Other(&'a [u8]),
    Ping,
    Pong { iana: u32
         , oem_defined: u32
         , entities: u8
         , interactions: u8
         }
}

impl BytesSerializationSized for AsfData<'_> {
    fn size(&self) -> usize {
        match self {
            AsfData::Ping => 0,
            AsfData::Pong { .. }  => 10,
            AsfData::Other(bytes) => bytes.len()
        }
    }
}

impl BytesSerializable for AsfData<'_>
{
    fn write_to_slice(&self, slice: &mut [u8], _strict: bool) -> Result<(), Error>
    {
        if self.size() > slice.len() {
            return Err(Error::OutBufferTooSmall);
        }

        match self {
            AsfData::Ping => Ok(()),
            AsfData::Pong { iana, oem_defined, entities, interactions } => {
                slice[0..4].copy_from_slice(&iana.to_be_bytes());
                slice[4..8].copy_from_slice(&oem_defined.to_be_bytes());
                slice[8] = *entities;
                slice[9] = *interactions;
                Ok(())
            },
            AsfData::Other(bytes) => {
                slice[..bytes.len()].copy_from_slice(bytes);
                Ok(())
            }
        }
    }
}

impl BytesSerializationSized for AsfMessage<'_>
{
    fn size(&self) -> usize {
        8 + self.data.size()
    }
}

impl BytesSerializable for AsfMessage<'_>
{
    fn write_to_slice(&self, bytes: &mut [u8], strict: bool) 
        -> Result<(), Error>
    {
        if bytes.len() < self.size() {
            Err(Error::OutBufferTooSmall)
        } else if strict && self.data.size() != self.data_len as usize {
            Err(Error::InvalidConfiguration)
        } else { 
            let valid_config = !strict || match self.msg_type {
                ASF_MSG_TYPE_PING => self.data_len == 0,
                ASF_MSG_TYPE_PONG => self.data_len == 10,
                _ => true
            };

            if !valid_config {
                Err(Error::InvalidConfiguration)
            } else {
                bytes[..4].copy_from_slice(&self.iana.to_be_bytes());
                bytes[4] = self.msg_type;
                bytes[5] = self.msg_tag;
                bytes[6] = self.data.size() as u8;
                self.data.write_to_slice(&mut bytes[7..], strict)?;
                Ok(())
            }
        }
    }

}

impl<'a> AsfMessage<'a>
{ 
    pub fn ping() -> AsfMessage<'a> {
        AsfMessage {
            iana: ASF_IANA,
            msg_type: ASF_MSG_TYPE_PING,
            msg_tag: 0,
            data_len: 0,
            data: AsfData::Ping
        }
    }

    pub fn pong(iana: u32, oem_defined: u32, entities: u8, interactions: u8)
        -> AsfMessage<'a>
    {
        AsfMessage {
            iana:     ASF_IANA,
            msg_type: ASF_MSG_TYPE_PONG,
            msg_tag:  0,
            data_len: 10,
            data:     AsfData::Pong {iana, oem_defined, entities, interactions}
        }
    }

    pub fn is_ping(&self) -> bool {
        self.data_len == 0 && self.msg_type == ASF_MSG_TYPE_PING
    }

    pub fn is_pong(&self) -> bool {
        self.data_len == 16 && self.msg_type == ASF_MSG_TYPE_PONG
    }
}

impl<'a> BytesDeserializable<'a> for AsfMessage<'a>
{
    fn from_bytes(bytes: &'a [u8], strict: bool) -> Result<AsfMessage<'a>, Error>
    {
        /* 
         * +----Field--(size)-+
         * | IANA Number  (4) |
         * | Message Type (1) |
         * | Message Tag  (1) |
         * | Reserved     (1) |
         * | Data Length  (1) |
         * | Data       (var) |
         * +------------------+
         */
        /* ASF message should have at least 8 bytes, data have most 255 bytes */
        if bytes.len() < 8 {
            Err(Error::PayloadTooSmall)
        } else if strict && usize::from(bytes[7]) + 8 != bytes.len() {
            Err(Error::ExpectedSizeMismatch)
        } else {
            let iana = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
            let msg_type = bytes[4];
            let msg_tag  = bytes[5];
            let data_len  = bytes[7];
            let mut idx = 8;

            let data = match msg_type {
                ASF_MSG_TYPE_PING =>
                    if strict && data_len != 0 { 
                        Err(Error::ExpectedSizeMismatch)
                    } else { 
                        Ok(AsfData::Ping)
                    },
                ASF_MSG_TYPE_PONG => {
                    if bytes.len() < 18 {
                        Err(Error::PayloadTooSmall)
                    } else if strict && data_len > 10 {
                        Err(Error::PayloadTooLarge)
                    } else {
                        let iana         = take_be_u32!(bytes, idx);
                        let oem_defined  = take_be_u32!(bytes, idx);
                        let entities     = take_u8!(bytes, idx);
                        let interactions = bytes[idx];
                        Ok(AsfData::Pong {
                            iana, oem_defined, entities, interactions })
                    }
                },
                _ => Ok(AsfData::Other(&bytes[8..]))
            };

            data.map(|data| AsfMessage { iana, msg_type, msg_tag, data_len, data })
        }
    }
}
