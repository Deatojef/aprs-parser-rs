//! Raw NMEA sentence handling for APRS.
//!
//! Data type identifier `$` — stores the raw NMEA sentence without parsing.

use std::io::Write;

use crate::Callsign;
use crate::EncodeError;

/// A raw NMEA sentence received via APRS.
#[derive(PartialEq, Debug, Clone)]
pub struct AprsNmea {
    pub to: Callsign,
    pub data: Vec<u8>,
}

impl AprsNmea {
    pub fn decode(b: &[u8], to: Callsign) -> Self {
        Self {
            to,
            data: b.to_vec(),
        }
    }

    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        write!(buf, "$")?;
        buf.write_all(&self.data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_nmea() {
        let data = b"GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,47.0,M,,*47";
        let to = Callsign::new_no_ssid("APRS");
        let n = AprsNmea::decode(data, to);
        assert_eq!(n.data, data);
    }

    #[test]
    fn encode_nmea() {
        let n = AprsNmea {
            to: Callsign::new_no_ssid("APRS"),
            data: b"GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,47.0,M,,*47".to_vec(),
        };
        let mut buf = vec![];
        n.encode(&mut buf).unwrap();
        assert_eq!(
            buf,
            b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,47.0,M,,*47"
        );
    }

    #[test]
    fn roundtrip() {
        let data = b"GPRMC,081836,A,3751.65,S,14507.36,E,000.0,360.0,130998,011.3,E*62";
        let to = Callsign::new_no_ssid("APRS");
        let n = AprsNmea::decode(data, to.clone());
        let mut buf = vec![];
        n.encode(&mut buf).unwrap();
        // strip leading '$'
        let n2 = AprsNmea::decode(&buf[1..], to);
        assert_eq!(n.data, n2.data);
    }
}
