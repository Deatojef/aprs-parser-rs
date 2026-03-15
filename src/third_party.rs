//! Third-party packet handling for APRS.
//!
//! Data type identifier `}` — wraps a complete APRS packet that has been
//! forwarded from one network to another (e.g., RF to APRS-IS gateway).
//! Format: `}INNER_PACKET`
//!
//! The inner packet is a full APRS packet (header:body) that is recursively
//! decoded.

use std::io::Write;

use crate::AprsPacket;
use crate::Callsign;
use crate::DecodeError;
use crate::EncodeError;

/// A third-party APRS packet, containing a forwarded inner packet.
#[derive(PartialEq, Debug, Clone)]
pub struct AprsThirdParty {
    pub to: Callsign,
    /// The forwarded inner packet, recursively decoded.
    pub inner: Box<AprsPacket>,
}

impl AprsThirdParty {
    pub fn decode(b: &[u8], to: Callsign) -> Result<Self, DecodeError> {
        let inner = AprsPacket::decode_textual(b)?;
        Ok(Self {
            to,
            inner: Box::new(inner),
        })
    }

    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        write!(buf, "}}")?;
        self.inner.encode_textual(buf)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AprsData;

    #[test]
    fn decode_third_party() {
        let data = b"W0RO-11>APRX29,TCPIP*,qAC,T2MCI:}WB0VGI-7>APOT30,W0RO-11*,WIDE2-1:!4228.35N/09101.45W_PHG2360";
        let packet = AprsPacket::decode_textual(data).unwrap();

        assert_eq!(packet.from, Callsign::new_with_ssid("W0RO", "11"));

        match &packet.data {
            AprsData::ThirdParty(tp) => {
                assert_eq!(tp.inner.from, Callsign::new_with_ssid("WB0VGI", "7"));
                match &tp.inner.data {
                    AprsData::Position(pos) => {
                        assert_relative_eq!(*pos.position.latitude, 42.4725, epsilon = 0.01);
                    }
                    _ => panic!("Expected Position in inner packet"),
                }
            }
            _ => panic!("Expected ThirdParty"),
        }
    }

    #[test]
    fn decode_third_party_with_message() {
        let data = b"RELAY>APRS,TCPIP:}W1ABC>APRS,WIDE1-1::DEST     :Hello from third party{123";
        let packet = AprsPacket::decode_textual(data).unwrap();

        match &packet.data {
            AprsData::ThirdParty(tp) => {
                assert_eq!(tp.inner.from, Callsign::new_no_ssid("W1ABC"));
                match &tp.inner.data {
                    AprsData::Message(msg) => {
                        assert_eq!(msg.addressee, b"DEST");
                        assert_eq!(msg.text, b"Hello from third party");
                        assert_eq!(msg.id, Some(b"123".to_vec()));
                    }
                    _ => panic!("Expected Message in inner packet"),
                }
            }
            _ => panic!("Expected ThirdParty"),
        }
    }

    #[test]
    fn encode_roundtrip() {
        let data = b"RELAY>APRS,TCPIP:}W1ABC>APRS,WIDE1-1::DEST     :Hello from third party{123";
        let packet = AprsPacket::decode_textual(data).unwrap();
        let mut buf = vec![];
        packet.encode_textual(&mut buf).unwrap();
        assert_eq!(data[..], buf[..]);
    }
}
