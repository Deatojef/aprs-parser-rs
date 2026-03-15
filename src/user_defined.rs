//! User-defined packet handling for APRS.
//!
//! Data type identifier `{` — experimental or user-defined data.
//! Format: `{UDATA` where U is a one-character user ID and DATA is
//! the user-defined data. The content is opaque to the parser.

use std::io::Write;

use crate::Callsign;
use crate::EncodeError;

/// A user-defined APRS packet with an experimenter ID and opaque payload.
#[derive(PartialEq, Debug, Clone)]
pub struct AprsUserDefined {
    pub to: Callsign,
    /// Single-character user/experimenter ID.
    pub user_id: u8,
    /// User-defined packet type character.
    pub packet_type: u8,
    /// The raw user-defined data (everything after the type character).
    pub data: Vec<u8>,
}

impl AprsUserDefined {
    pub fn decode(b: &[u8], to: Callsign) -> Self {
        let user_id = b.first().copied().unwrap_or(0);
        let packet_type = b.get(1).copied().unwrap_or(0);
        let data = b.get(2..).unwrap_or_default().to_vec();
        Self {
            to,
            user_id,
            packet_type,
            data,
        }
    }

    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        write!(buf, "{{")?;
        buf.write_all(&[self.user_id, self.packet_type])?;
        buf.write_all(&self.data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AprsData, AprsPacket};

    #[test]
    fn decode_user_defined() {
        let data = b"W1ABC>APRS:{Qhello world";
        let packet = AprsPacket::decode_textual(data).unwrap();

        match &packet.data {
            AprsData::UserDefined(ud) => {
                assert_eq!(ud.user_id, b'Q');
                assert_eq!(ud.packet_type, b'h');
                assert_eq!(ud.data, b"ello world");
            }
            _ => panic!("Expected UserDefined"),
        }
    }

    #[test]
    fn encode_roundtrip() {
        let data = b"W1ABC>APRS:{Qhello world";
        let packet = AprsPacket::decode_textual(data).unwrap();
        let mut buf = vec![];
        packet.encode_textual(&mut buf).unwrap();
        assert_eq!(data[..], buf[..]);
    }

    #[test]
    fn decode_minimal() {
        let ud = AprsUserDefined::decode(b"AB", Callsign::new_no_ssid("APRS"));
        assert_eq!(ud.user_id, b'A');
        assert_eq!(ud.packet_type, b'B');
        assert_eq!(ud.data, b"");
    }
}
