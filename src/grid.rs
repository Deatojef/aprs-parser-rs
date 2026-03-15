//! Maidenhead grid locator parsing and encoding for APRS.
//!
//! Format: `[IO91SX]comment` — data type identifier `[`
//! The grid square is 4 or 6 characters enclosed in brackets.

use std::io::Write;

use crate::Callsign;
use crate::DecodeError;
use crate::EncodeError;
use crate::Latitude;
use crate::Longitude;

/// An APRS Maidenhead grid locator report.
#[derive(PartialEq, Debug, Clone)]
pub struct AprsGridLocator {
    pub to: Callsign,
    pub grid: Vec<u8>,
    pub comment: Vec<u8>,
}

impl AprsGridLocator {
    pub fn decode(b: &[u8], to: Callsign) -> Result<Self, DecodeError> {
        // Find the closing bracket
        let end = b
            .iter()
            .position(|&c| c == b']')
            .ok_or_else(|| DecodeError::InvalidGridLocator(b.to_vec()))?;

        let grid = b[..end].to_vec();

        // Grid must be 4 or 6 characters
        if grid.len() != 4 && grid.len() != 6 {
            return Err(DecodeError::InvalidGridLocator(b.to_vec()));
        }

        // Validate grid format: 2 uppercase letters, 2 digits, optionally 2 letters
        if !grid[0].is_ascii_uppercase()
            || !grid[1].is_ascii_uppercase()
            || !grid[2].is_ascii_digit()
            || !grid[3].is_ascii_digit()
        {
            return Err(DecodeError::InvalidGridLocator(b.to_vec()));
        }

        if grid.len() == 6 && (!grid[4].is_ascii_alphabetic() || !grid[5].is_ascii_alphabetic()) {
            return Err(DecodeError::InvalidGridLocator(b.to_vec()));
        }

        let comment = b.get(end + 1..).unwrap_or_default().to_vec();

        Ok(Self { to, grid, comment })
    }

    /// Convert the Maidenhead grid locator to approximate latitude/longitude.
    /// Returns the center of the grid square.
    pub fn to_position(&self) -> Option<(Latitude, Longitude)> {
        if self.grid.len() < 4 {
            return None;
        }

        let field_lon = (self.grid[0] - b'A') as f64;
        let field_lat = (self.grid[1] - b'A') as f64;
        let square_lon = (self.grid[2] - b'0') as f64;
        let square_lat = (self.grid[3] - b'0') as f64;

        let (subsq_lon, subsq_lat) = if self.grid.len() >= 6 {
            let lo = (self.grid[4].to_ascii_lowercase() - b'a') as f64;
            let la = (self.grid[5].to_ascii_lowercase() - b'a') as f64;
            (lo, la)
        } else {
            (0.0, 0.0)
        };

        let lon;
        let lat;

        if self.grid.len() >= 6 {
            // 6-char grid: field(20°) + square(2°) + subsquare(5')
            lon = field_lon * 20.0 + square_lon * 2.0 + subsq_lon * (2.0 / 24.0) + (1.0 / 24.0)
                - 180.0;
            lat = field_lat * 10.0 + square_lat * 1.0 + subsq_lat * (1.0 / 24.0) + (0.5 / 24.0)
                - 90.0;
        } else {
            // 4-char grid: center of the 2°x1° square
            lon = field_lon * 20.0 + square_lon * 2.0 + 1.0 - 180.0;
            lat = field_lat * 10.0 + square_lat * 1.0 + 0.5 - 90.0;
        }

        let latitude = Latitude::new(lat)?;
        let longitude = Longitude::new(lon)?;
        Some((latitude, longitude))
    }

    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        write!(buf, "[")?;
        buf.write_all(&self.grid)?;
        write!(buf, "]")?;
        buf.write_all(&self.comment)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_4char_grid() {
        let data = b"IO91]";
        let to = Callsign::new_no_ssid("APRS");
        let g = AprsGridLocator::decode(data, to).unwrap();
        assert_eq!(g.grid, b"IO91");
        assert_eq!(g.comment, b"");
    }

    #[test]
    fn decode_6char_grid() {
        let data = b"IO91SX]comment here";
        let to = Callsign::new_no_ssid("APRS");
        let g = AprsGridLocator::decode(data, to).unwrap();
        assert_eq!(g.grid, b"IO91SX");
        assert_eq!(g.comment, b"comment here");
    }

    #[test]
    fn decode_invalid_no_bracket() {
        let data = b"IO91SX";
        let to = Callsign::new_no_ssid("APRS");
        assert!(AprsGridLocator::decode(data, to).is_err());
    }

    #[test]
    fn decode_invalid_length() {
        let data = b"IO9]";
        let to = Callsign::new_no_ssid("APRS");
        assert!(AprsGridLocator::decode(data, to).is_err());
    }

    #[test]
    fn to_position_4char() {
        let g = AprsGridLocator {
            to: Callsign::new_no_ssid("APRS"),
            grid: b"JO22".to_vec(),
            comment: vec![],
        };
        let (lat, lon) = g.to_position().unwrap();
        // JO22 center: lon = 9*20 + 2*2 + 1 - 180 = 5, lat = 14*10 + 2 + 0.5 - 90 = 52.5
        assert_relative_eq!(*lat, 52.5, epsilon = 0.1);
        assert_relative_eq!(*lon, 5.0, epsilon = 0.1);
    }

    #[test]
    fn to_position_6char() {
        let g = AprsGridLocator {
            to: Callsign::new_no_ssid("APRS"),
            grid: b"FN31pr".to_vec(),
            comment: vec![],
        };
        let (lat, lon) = g.to_position().unwrap();
        // FN31pr: approximate NYC area
        // lon = 5*20 + 3*2 + 15*(2/24) + (1/24) - 180 = 100+6+1.25+0.0417-180 = -72.71
        // lat = 13*10 + 1 + 17*(1/24) + (0.5/24) - 90 = 130+1+0.708+0.0208-90 = 41.73
        assert!(*lat > 41.0 && *lat < 42.0);
        assert!(*lon > -73.0 && *lon < -72.0);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let g = AprsGridLocator {
            to: Callsign::new_no_ssid("APRS"),
            grid: b"IO91SX".to_vec(),
            comment: b"Hello".to_vec(),
        };
        let mut buf = vec![];
        g.encode(&mut buf).unwrap();
        assert_eq!(buf, b"[IO91SX]Hello");

        // Decode back (skip the leading '[')
        let g2 = AprsGridLocator::decode(&buf[1..], Callsign::new_no_ssid("APRS")).unwrap();
        assert_eq!(g.grid, g2.grid);
        assert_eq!(g.comment, g2.comment);
    }
}
