use crate::ipmi::asf::AsfMessage;
use crate::ipmi::ipmi::Ipmi15Packet;
use crate::ipmi::*;

pub const MSG_CLASS_ASF:  u8 = 0b00000110;
pub const MSG_CLASS_IPMI: u8 = 0b00000111;
pub const MSG_CLASS_OEM:  u8 = 0b00001000;

#[derive(PartialEq, Eq, Debug)]
pub struct RmcpMessage<'a> {
    pub version: u8,  /* must be 0x06 to be compatible wth standard */
    pub reserved: u8, /* must be 0x00 to be compatible with standard */
    pub sequence_number: u8,
    pub message_class:   u8,
    pub data: RmcpContent<'a>
}

#[derive(PartialEq, Eq, Debug)]
pub enum RmcpContent<'a> {
    Ack,
    Asf(crate::ipmi::asf::AsfMessage<'a>),
    Ipmi15(crate::ipmi::ipmi::Ipmi15Packet<'a>),
    Oem { iana: u32, data: &'a [u8] },
    Other(&'a [u8])
}

impl<'a> BytesSerializationSized for RmcpMessage<'a> {
    fn size(&self) -> usize {
        match &self.data {
            RmcpContent::Ack => 4,
            RmcpContent::Asf(asf) => 4 + asf.size(),
            RmcpContent::Oem { iana: _, data } => 4 + 4 + data.len(),
            RmcpContent::Ipmi15(packet) => 4 + packet.size(),
            RmcpContent::Other(bytes)   => 4 + bytes.len()
        }
    }

}

impl<'a> BytesSerializable for RmcpMessage<'a>  {

    fn write_to_slice(&self, slice: &mut [u8], strict: bool) -> Result<(), Error> {
        slice[0] = 0x06;
        slice[1] = 0x00;
        slice[2] = self.sequence_number;
        slice[3] = self.message_class;
        match &self.data {
            RmcpContent::Ack      => Ok(()),
            RmcpContent::Asf(asf) => asf.write_to_slice(&mut slice[4..], strict),
            RmcpContent::Other(bytes) => Ok(slice[4..][..bytes.len()].copy_from_slice(bytes)),
            RmcpContent::Oem { iana, data } => {
                slice[4..8].copy_from_slice(&iana.to_be_bytes());
                slice[8..][..data.len()].copy_from_slice(data);
                Ok(())
            },
            RmcpContent::Ipmi15(packet) => packet.write_to_slice(&mut slice[4..], strict)
        }
    }
}

impl<'a> RmcpMessage<'a>
{
    pub fn from_ack(seqnum: u8) -> RmcpMessage<'a> {
        RmcpMessage {
            version: 0x06,
            reserved: 0x00,
            sequence_number: seqnum,
            message_class: MSG_CLASS_ASF,
            data: RmcpContent::Ack
        }
    }

    pub fn from_asf(msg: AsfMessage<'a>) -> RmcpMessage<'a> {
        RmcpMessage {
            version: 0x06,
            reserved: 0x00,
            sequence_number: 0xff,
            message_class: MSG_CLASS_ASF,
            data: RmcpContent::Asf(msg)
        }
    }
}

impl<'a> BytesDeserializable<'a> for RmcpMessage<'a>
{
    fn from_bytes(bytes: &'a [u8], strict: bool) -> Result<RmcpMessage<'a>, Error>
    {
        if bytes.len() < 4 { return Err(Error::PayloadTooSmall); }

        let version         = bytes[0];
        let reserved        = bytes[1];

        if strict && (version != 0x06 || reserved != 0x00) {
            if version != 0x06 { return Err(Error::InvalidRmcpVersionNumber(version)) }
            if reserved != 0x00 { return Err(Error::InvalidRmcpReservedByte(reserved)) }
        }

        let is_ack          = (bytes[3] & 0b10000000) == 0b10000000;
        let sequence_number = bytes[2];
        let message_class   = bytes[3] & 0b00001111;

        let mut idx = 4;

        let content = {
            if is_ack {
                Ok(RmcpContent::Ack)
            } else {
                match message_class {
                    MSG_CLASS_OEM => {
                        if bytes.len() < 8 { return Err(Error::PayloadTooSmall); }
                        let iana = crate::take_le_u32!(bytes, idx);
                        let data = crate::take_remain!(bytes, idx);
                        let content = RmcpContent::Oem { iana, data };
                        Ok(content)
                    },
                    MSG_CLASS_ASF => {
                        AsfMessage::from_bytes(&bytes[4..], strict)
                            .map(|m| RmcpContent::Asf(m))
                    },
                    MSG_CLASS_IPMI => {
                        /* read ahead the auth format */
                        if bytes[4] == 0x06 {
                            /* Don't have support for RMCP+ / IPMI2 yet */
                            Err(Error::UnsupportedProtocol)
                        } else {
                            Ipmi15Packet::from_bytes(&bytes[4..], strict)
                                .map(|m| RmcpContent::Ipmi15(m))
                        }
                    },
                    _ => 
                        if strict { 
                            Err(Error::UnsupportedProtocol)
                        } else {
                            Ok(RmcpContent::Other(&bytes[4..]))
                        }
                }
            }
        };

        content.map(|data| RmcpMessage {
            version, reserved, sequence_number, message_class, data, })
    }
}
