use crate::ipmi::*;

#[derive(PartialEq, Eq, Debug)]
pub struct IpmiMessage<'a> {
    pub peer_addr:  u8,
    pub netfn:      u8,
    pub peer_lun:   u8,
    pub local_addr: u8,
    pub seqnum:     u8,
    pub local_lun:  u8,
    pub cmd:        u8,
    pub data:       IpmiData<'a>
}

#[derive(PartialEq, Eq, Debug)]
pub struct Ipmi15Packet<'a> {
    pub auth_type:  u8,
    pub seqnum:     u32,
    pub session_id: u32,
    pub auth_code:  Option<&'a [u8]>,
    pub payload_len: u8,
    pub data:       IpmiMessage<'a>
}

#[derive(PartialEq, Eq, Debug)]
pub enum IpmiData<'a> {
    Request(&'a[u8]),
    Response(u8, &'a[u8])
}

pub const IPMI_PRIV_LEVEL_CALLBACK: u8 = 1;
pub const IPMI_PRIV_LEVEL_USER:     u8 = 2;
pub const IPMI_PRIV_LEVEL_OPERATOR: u8 = 3;
pub const IPMI_PRIV_LEVEL_ADMIN:    u8 = 4;
pub const IPMI_PRIV_LEVEL_OEM:      u8 = 5;

pub const IPMI_AUTH_TYPE_NONE: u8 = 0;
pub const IPMI_AUTH_TYPE_MD2:  u8 = 1;
pub const IPMI_AUTH_TYPE_MD5:  u8 = 2;
pub const IPMI_AUTH_TYPE_KEY:  u8 = 3;
pub const IPMI_AUTH_TYPE_OEM:  u8 = 4;

fn ipmi_cksum(slice: &[u8]) -> u8 {
    slice.iter().fold(0u8, |acc, n| acc.wrapping_add(*n)).wrapping_neg()
}

fn ipmi_cksum_verify(slice: &[u8]) -> bool {
    slice.iter().fold(0u8, |acc, n| acc.wrapping_add(*n)) == 0
}

impl IpmiMessage<'_> {
    pub fn rs_addr(&self) -> u8 {
        if self.netfn % 2 == 0 {
            self.peer_addr
        } else {
            self.local_addr
        }
    }

    pub fn rq_addr(&self) -> u8 {
        if self.netfn % 2 == 0 {
            self.local_addr
        } else {
            self.peer_addr
        }
    }

    pub fn rs_lun(&self) -> u8 {
        if self.netfn % 2 == 0 {
            self.peer_lun
        } else {
            self.local_lun
        }
    }

    pub fn rq_lun(&self) -> u8 {
        if self.netfn % 2 == 0 {
            self.local_lun
        } else {
            self.peer_lun
        }
    }
}

impl BytesSerializationSized for Ipmi15Packet<'_> {
    fn size(&self) -> usize {
        match self.auth_code {
            Some(_) => 16 + 10 + self.data.size(),
            None    => 10 + self.data.size()
        }
    }
}

impl<'a> BytesSerializable for Ipmi15Packet<'a>
{
    fn write_to_slice(&self, slice: &mut [u8], strict: bool) -> Result<(), Error>
    {
        if self.size() < slice.len() {
            return Err(Error::OutBufferTooSmall);
        }

        if self.auth_code.is_some() && self.auth_code.unwrap().len() != 16 {
            return Err(Error::InvalidConfiguration);
        }

        if strict {
            if self.data.size() > 255 {
                return Err(Error::InvalidConfiguration);
            }

            if self.data.size() != self.payload_len as usize {
                return Err(Error::InvalidConfiguration);
            }
        }

        slice[0] = self.auth_type;
        slice[1..5].copy_from_slice(&self.seqnum.to_le_bytes());
        slice[5..9].copy_from_slice(&self.session_id.to_le_bytes());
        

        let next_idx = match self.auth_code {
            Some(value) => {
                slice[9..25].copy_from_slice(value);
                25
            }
            _ => 9
        };

        slice[next_idx] = self.payload_len;

        self.data.write_to_slice(&mut slice[(next_idx+1)..], strict)
    }
}

impl<'a> Ipmi15Packet<'a>
{
    pub fn from_bytes(bytes: &'a [u8], strict: bool) -> Result<Ipmi15Packet, Error>
    {
        /* that is 10 bytes min for ipmi header + 7 bytes min for msg header */
        if bytes.len() < 17 { return Err(Error::PayloadTooSmall); }

        /* \forall t \in ipmi 1.5 auth type, t \in [0, 5] */
        if strict && bytes[0] > 5 {
            return Err(Error::UndefinedAuthType(bytes[0]));
        }

        let mut idx    = 0;
        let auth_type  = crate::take_u8!(bytes, idx);
        let seqnum     = crate::take_le_u32!(bytes, idx);
        let session_id = crate::take_le_u32!(bytes, idx);
        let mut auth_code: Option<&'a [u8]> = None;

        /* in case the packet contains auth code, we need 16 bytes more */
        if auth_type != IPMI_AUTH_TYPE_NONE {
            if bytes.len() < 29 { return Err(Error::PayloadTooSmall); }
            auth_code = Some(crate::take!(bytes, idx, 16))
        }

        let payload_len = crate::take_u8!(bytes, idx);
        let data = IpmiMessage::from_bytes(crate::take_remain!(bytes, idx), strict)?;

        if data.size() != payload_len as usize {
            return Err(Error::ExpectedSizeMismatch);
        }

        Ok(Ipmi15Packet {
            auth_type, 
            seqnum,
            session_id,
            auth_code,
            payload_len,
            data
        })

    }
}

impl<'a> BytesSerializationSized for IpmiMessage<'_> {
    fn size(&self) -> usize {
        match self.data {
            IpmiData::Request(dat) => dat.len() + 7,
            IpmiData::Response(_, dat) => dat.len() + 8
        }
    }
}

impl<'a> BytesSerializable for IpmiMessage<'a>
{
    fn write_to_slice(&self, slice: &mut [u8], strict: bool) -> Result<(), Error>
    {
        if strict {
            if self.peer_lun > 0b00000011 || self.local_lun > 0b00000011 
                || self.seqnum > 0b11111100
            {
                return Err(Error::InvalidConfiguration)
            }
        }

        slice[0] = self.peer_addr;
        slice[1] = (self.netfn << 2) | (self.peer_lun & 0b00000011);
        slice[2] = ipmi_cksum(&slice[0..2]);

        slice[3] = self.local_addr;
        slice[4] = (self.seqnum << 2) | (self.local_lun & 0b00000011);

        slice[5] = self.cmd;

        match self.data {
            IpmiData::Request(dat) => slice[6..][..dat.len()].copy_from_slice(dat),
            IpmiData::Response(code, dat) => {
                slice[6] = code;
                slice[7..][..dat.len()].copy_from_slice(dat)
            } 
        };

        let cksum_size = 3 + match self.data {
            IpmiData::Request(dat) => dat.len(),
            IpmiData::Response(_, dat) => dat.len() + 1
        };

        slice[3 + cksum_size] = ipmi_cksum(&slice[3..][..cksum_size]);
        Ok(())
    }
}

impl<'a> BytesDeserializable<'a> for IpmiMessage<'a>
{
    fn from_bytes(bytes: &'a [u8], _strict: bool) -> Result<IpmiMessage<'a>, Error> 
    {
        if bytes.len() < 7 {
            return Err(Error::PayloadTooSmall);
        }

        let (fst, snd) = bytes.split_at(3);

        if !ipmi_cksum_verify(fst) || !ipmi_cksum_verify(snd) {
            return Err(Error::InvalidChecksum);
        }

        let peer_addr = fst[0];
        let netfn_lun = fst[1];

        let local_addr = snd[0];
        let seqnum_lun = snd[1];
        let cmd        = snd[2];

        let netfn      = netfn_lun >> 2;
        let peer_lun   = netfn_lun & 0b00000011;

        let seqnum = seqnum_lun >> 2;
        let local_lun = seqnum_lun & 0b00000011;

        /* remove the checksum byte, this can never fail as we checked 
         * payload length earlier 
         */
        let (_, dat)   = bytes[6..].split_last().unwrap();

        let data = if netfn % 2 == 0 {
                IpmiData::Request(dat)
            } else {
                IpmiData::Response(dat[0], &dat[1..])
            };

        Ok(IpmiMessage { peer_addr, netfn, local_addr, local_lun, seqnum, 
            peer_lun, cmd, data })
    }
}
