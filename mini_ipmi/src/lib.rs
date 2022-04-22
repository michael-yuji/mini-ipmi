#![no_std]

pub mod ipmi;

#[cfg(test)]
mod tests {
    use super::ipmi::*;
    use super::ipmi::ipmi::*;
    use super::ipmi::cmd::*;
    use super::ipmi::ipmi::IpmiData;
    use super::ipmi::asf::AsfMessage;
    use super::ipmi::rmcp::{RmcpContent, RmcpMessage};

    #[test]
    fn test_asf_ping() {
        let rmcp_asf_ping = [0x06, 0x00, 0xff, 0x06, 0x00, 0x00, 0x11, 0xbe, 0x80, 0x00, 0x00, 0x00];
        let reference = RmcpMessage::from_asf(AsfMessage::ping());

        let mut out = [0u8;12];
        let decoded = RmcpMessage::from_bytes(&rmcp_asf_ping, true);

        assert_eq!(decoded.is_ok(), true);

        let ping = decoded.unwrap();
        assert_eq!(reference, ping);

        ping.write_to_slice(&mut out, true);
        assert_eq!(rmcp_asf_ping, out);
    }

    #[test]
    fn test_ipmi_get_auth_capabilities_req() {
        let req_bytes = [0x06, 0x00, 0xff, 0x07, 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x09,0x20,0x18,0xc8,0x81,0x04,0x38,0x0e,0x04,0x31];
        let mut out = [0u8; 23];

        let decoded = RmcpMessage::from_bytes(&req_bytes, true);

        assert_eq!(decoded.is_ok(), true);

        let unwrapped = decoded.unwrap();

        if let RmcpContent::Ipmi15(packet) = &unwrapped.data {
            assert_eq!(packet.session_id, 0x0u32);
            assert_eq!(packet.seqnum, 0x0u32);

            assert_eq!(packet.data.netfn, 0x06);
            assert_eq!(packet.data.cmd, 0x38);

            if let IpmiData::Request(reqd) = packet.data.data {
                if let Ok(req) = GetChannelAuthCapRequest::from_bytes(reqd, true) {
                    assert_eq!(req.channel_number, 14);
                    assert_eq!(req.max_priv_level, IPMI_PRIV_LEVEL_ADMIN);
                }

            } else {
                panic!("Should not be a response!")
            }
        } else {
            panic!("Should decode as IPMI 1.5 packet")
        }

        match unwrapped.write_to_slice(&mut out, true) {
            Ok(_) => assert_eq!(out, req_bytes),
            Err(y) => panic!("failed to write ipmi payload: {:?}", y)
        }
    }

    #[test]
    fn test_ipmi_get_auth_capabilities_res() {
        let res_bytes = [0x06, 0x00, 0xff, 0x07, 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x10,0x81,0x1c,0x63,0x20,0x04,0x38,0x00,0x01,0x04,0x14,0x00,0xd6,0xc1,0x00,0x00,0xf4];

        let mut out = [0u8; 30];

        let decoded = RmcpMessage::from_bytes(&res_bytes, true);

        assert_eq!(decoded.is_ok(), true);

        let unwrapped = decoded.unwrap();

        if let RmcpContent::Ipmi15(packet) = &unwrapped.data {
            assert_eq!(packet.session_id, 0x0u32);
            assert_eq!(packet.seqnum, 0x0u32);

            assert_eq!(packet.data.netfn, 0x07);
            assert_eq!(packet.data.cmd, 0x38);

            if let IpmiData::Response(code, resd) = packet.data.data {
                if let Ok(req) = GetChannelAuthCapResponse::from_bytes(resd, true) {
                    assert_eq!(req.channel_number, 1);
                }

            } else {
                panic!("Should not be a request!")
            }
        } else {
            panic!("Should decode as IPMI 1.5 packet")
        }

        match unwrapped.write_to_slice(&mut out, true) {
            Ok(_) => assert_eq!(out, res_bytes),
            Err(y) => panic!("failed to write ipmi payload: {:?}", y)
        }
    }
}
