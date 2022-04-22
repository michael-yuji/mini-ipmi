# mini-ipmi

`#![no-std]`:
This is a Rust crate that serialize / deserialize IPMI over LAN data. 
This crate does not depend on `std` nor need `alloc`.

This crate support serialize/deserialize generic IPMI request/response to/from 
byte slices. *Battery is not yet included*, user of the crate will have to 
implement their own logic for anything useful.

For IPMI over LAN, IPMI 1.5 and RMCP is supported. 
IPMI 2.0 / RMCP+ is not supported yet.

:::info
Instead of creating many enum and type to provide more contextual and typed
value. This crate is sticking with raw values (like `u8`). The reason behind
it is mostly to support unconventional usages like pan-testing / security
research. In the future this may change, or being offered as a separate feature
:::


## Usage
Assume we have an arary of bytes containing an IPMI over RMCP message.
The following example to to decode the RMCP message and validate the message
is not malformed. Allowing non-strict decoding can be useful for some usages
like security research.

Decoding to typed IPMI request/response is also supported

### Deserialize
```rust
let·bytes·=·[
    /* RMCP header */
    0x06,·0x00,·0xff,·0x07,
    /* IPMI1.5 header*/
    0x00, 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x09, 
    /* IPMI GetChannelAuthCapabilities Request */
    0x20,0x18,0xc8,0x81,0x04,0x38,0x0e,0x04,0x31
];

let validate_msg: bool = true;
let rmcp_message = RmcpMessage::from_bytes(&bytes, validate_msg)?;

if let RmcpContent::Ipmi15(ipmi_pkt) = rmcp_message.data {
    /* IPMI 1.5 session data */
    assert_eq!(ipmi_pkt.session_id, 0u32);
    assert_eq!(ipmi_pkt.seqnum,     0u32);
    
    /* down field `data` for IPMI message header */
    assert_eq!(ipmi_pkt.data.netfn, 0x06u8);
    
    /**
     * Some Ipmi command that's included in this crate can be decoded directly
     * to a typed value. Th
     */
    
    /* to decode any request message, and deal with the raw bytes */
    if let IpmiData::Request(rq_bytes) = ipmi_pkt.data.data {
        todo!();
    }
    
     /* to decode any response message, and deal with the raw bytes */   
    if let IpmiData::Response(rs_code, rs_bytes) = ipmi_pkt.data.data {
        panic!("The byte array in this example contais req data, not res");
    }
}
```

### Serialize
:::warning
:warning: Unlike `copy_from_slice` in rust, `write_to_slice` in this crate 
can write to slice with size greater or equals to the content it's going
to write. use `foo.size()` to get the size it will be written.
:::
```rust
/* continue from previous code block */
let mut out = [0u8; 23];
rmcp_message.write_to_slice(&mut out, validate_msg);
assert_eq!(out, bytes);
```

## IPMI over LAN overview
This section breifly describe how IPMI over LAN work and their mapping to `struct`/`enum` in this crate.

IPMI is basically a req/res protocol. The req/res and their header 
(`ipmi::IpmiMessage` in this crate) are encapsulated in multiple layers when
using LAN as a medium for transportation.

In the case of IPMI 1.5, data are first wrapped within a IPMI 1.5 session wrapper,
which contains information about the session id, seq number, and optionally cryptographic signature. (`ipmi::Ipmi15Packet`).

Finally the packet is encapulated in a RMCP packet (`rmcp::RMCPMessage`). The
purpose of RMCP is to "multiplex" different protocol. For example, service
discovery is done by responding ASF pong message (`asf::AsfMessage`) via ASF
over RMCP.