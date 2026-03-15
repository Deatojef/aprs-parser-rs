//! Telemetry report parsing and encoding for APRS.
//!
//! Format: `T#SSS,V1,V2,V3,V4,V5,BBBBBBBB,comment`
//! where SSS is a sequence number, V1-V5 are analog values,
//! and BBBBBBBB is 8 digital bits.

use std::io::Write;

use crate::Callsign;
use crate::DecodeError;
use crate::EncodeError;

/// An APRS telemetry report.
#[derive(PartialEq, Debug, Clone)]
pub struct AprsTelemetry {
    pub to: Callsign,
    pub sequence: Vec<u8>,
    pub analog_values: Vec<Option<f64>>,
    pub digital_bits: Option<u8>,
    pub comment: Vec<u8>,
}

impl AprsTelemetry {
    pub fn decode(b: &[u8], to: Callsign) -> Result<Self, DecodeError> {
        // Format: T#SSS,V1,V2,V3,V4,V5,BBBBBBBB,comment
        // or T#SSS,V1,V2,V3,V4,V5,BBBBBBBB
        if b.len() < 2 || b[0] != b'#' {
            return Err(DecodeError::InvalidTelemetry(b.to_vec()));
        }

        let b = &b[1..]; // skip '#'

        // Split by commas
        let parts: Vec<&[u8]> = b.split(|&c| c == b',').collect();

        if parts.is_empty() {
            return Err(DecodeError::InvalidTelemetry(b.to_vec()));
        }

        let sequence = parts[0].to_vec();

        // Parse up to 5 analog values
        let mut analog_values = Vec::with_capacity(5);
        for i in 1..=5 {
            if let Some(part) = parts.get(i) {
                let val = std::str::from_utf8(part)
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok());
                analog_values.push(val);
            }
        }

        // Parse digital bits (8 binary digits)
        let digital_bits = parts.get(6).and_then(|part| {
            if part.len() >= 8 && part[..8].iter().all(|&c| c == b'0' || c == b'1') {
                let mut val = 0u8;
                for &bit in &part[..8] {
                    val = (val << 1) | (bit - b'0');
                }
                Some(val)
            } else {
                None
            }
        });

        // Remaining parts after digital bits form the comment
        let comment = if parts.len() > 7 {
            // Rejoin remaining parts with commas
            let comment_parts: Vec<&[u8]> = parts[7..].to_vec();
            let mut comment = Vec::new();
            for (i, part) in comment_parts.iter().enumerate() {
                if i > 0 {
                    comment.push(b',');
                }
                comment.extend_from_slice(part);
            }
            comment
        } else {
            vec![]
        };

        Ok(Self {
            to,
            sequence,
            analog_values,
            digital_bits,
            comment,
        })
    }

    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        write!(buf, "T#")?;
        buf.write_all(&self.sequence)?;

        for val in &self.analog_values {
            write!(buf, ",")?;
            if let Some(v) = val {
                // Format without unnecessary decimal places
                if *v == (*v as i64) as f64 {
                    write!(buf, "{}", *v as i64)?;
                } else {
                    write!(buf, "{}", v)?;
                }
            }
        }

        if let Some(bits) = self.digital_bits {
            write!(buf, ",")?;
            for i in (0..8).rev() {
                write!(buf, "{}", (bits >> i) & 1)?;
            }
        }

        if !self.comment.is_empty() {
            write!(buf, ",")?;
            buf.write_all(&self.comment)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_basic_telemetry() {
        let data = b"#001,100,200,300,400,500,10101010";
        let to = Callsign::new_no_ssid("APRS");
        let t = AprsTelemetry::decode(data, to).unwrap();
        assert_eq!(t.sequence, b"001");
        assert_eq!(t.analog_values.len(), 5);
        assert_eq!(t.analog_values[0], Some(100.0));
        assert_eq!(t.analog_values[1], Some(200.0));
        assert_eq!(t.analog_values[2], Some(300.0));
        assert_eq!(t.analog_values[3], Some(400.0));
        assert_eq!(t.analog_values[4], Some(500.0));
        assert_eq!(t.digital_bits, Some(0b10101010));
    }

    #[test]
    fn decode_telemetry_with_comment() {
        let data = b"#001,100,200,300,400,500,11110000,Hello World";
        let to = Callsign::new_no_ssid("APRS");
        let t = AprsTelemetry::decode(data, to).unwrap();
        assert_eq!(t.sequence, b"001");
        assert_eq!(t.digital_bits, Some(0b11110000));
        assert_eq!(t.comment, b"Hello World");
    }

    #[test]
    fn encode_decode_roundtrip() {
        let t = AprsTelemetry {
            to: Callsign::new_no_ssid("APRS"),
            sequence: b"001".to_vec(),
            analog_values: vec![
                Some(100.0),
                Some(200.0),
                Some(300.0),
                Some(400.0),
                Some(500.0),
            ],
            digital_bits: Some(0b10101010),
            comment: b"Test".to_vec(),
        };

        let mut buf = vec![];
        t.encode(&mut buf).unwrap();
        assert_eq!(buf, b"T#001,100,200,300,400,500,10101010,Test");
    }
}
