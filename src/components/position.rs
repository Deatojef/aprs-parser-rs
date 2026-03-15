use std::{
    io::{Read, Write},
    ops::RangeInclusive,
    str,
};

use crate::{AprsCompressedCs, AprsCompressionType, DecodeError, EncodeError};

use crate::AprsAltitude;

use super::lonlat::{Latitude, Longitude, Precision};

#[derive(PartialEq, Debug, Clone)]
pub enum AprsCst {
    CompressedSome {
        cs: AprsCompressedCs,
        t: AprsCompressionType,
    },
    CompressedNone,
    Uncompressed,
}

/// DAO precision extension parsed from `!DAO!` in the comment field.
///
/// When present, DAO refines the latitude and longitude beyond the standard
/// hundredth-of-a-minute precision:
/// - **HumanReadable** (`!wXY!`): adds a third decimal digit to the minutes,
///   giving thousandths-of-a-minute precision (~1.85m).
/// - **Base91** (`!WXY!`): adds sub-hundredth precision via base-91 encoding,
///   giving approximately 0.2m precision.
#[derive(PartialEq, Debug, Clone)]
pub enum Dao {
    /// Base-91 encoded extra precision (uppercase letter prefix, e.g. `!W__!`)
    Base91 { lat_offset: u8, lon_offset: u8 },
    /// Human-readable extra digit (lowercase letter prefix, e.g. `!w__!`)
    HumanReadable { lat_digit: u8, lon_digit: u8 },
}

impl Dao {
    /// Returns the latitude and longitude adjustments in degrees that this DAO
    /// extension adds to the base position.
    ///
    /// The adjustment is always positive — it represents how far into the
    /// current hundredth-of-a-minute cell the true position lies.
    pub fn offsets_degrees(&self) -> (f64, f64) {
        match self {
            Dao::HumanReadable {
                lat_digit,
                lon_digit,
            } => {
                // Each digit adds 0.001 minutes = 0.001/60 degrees
                let lat_adj = (*lat_digit as f64) / 60_000.0;
                let lon_adj = (*lon_digit as f64) / 60_000.0;
                (lat_adj, lon_adj)
            }
            Dao::Base91 {
                lat_offset,
                lon_offset,
            } => {
                // Base-91 value 0..90 maps to 0..0.01 minutes
                // offset / 91 * 0.01 minutes = offset / 91 / 6000 degrees
                let lat_adj = (*lat_offset as f64) / (91.0 * 6000.0);
                let lon_adj = (*lon_offset as f64) / (91.0 * 6000.0);
                (lat_adj, lon_adj)
            }
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Position {
    pub latitude: Latitude,
    pub longitude: Longitude,
    pub precision: Precision,
    pub symbol_table: char,
    pub symbol_code: char,
    pub cst: AprsCst,
    pub altitude: Option<AprsAltitude>,
    pub dao: Option<Dao>,
}

impl Position {
    /// Latitudes in APRS aren't perfectly precise - they have a configurable level of ambiguity. This is stored in the `precision` field on the `Position` struct. This method returns a range of what the actual latitude value might be.
    pub fn latitude_bounding(&self) -> RangeInclusive<f64> {
        self.precision.range(self.latitude.value())
    }

    /// Longitudes in APRS aren't perfectly precise - they have a configurable level of ambiguity. This is stored in the `precision` field on the `Position` struct. This method returns a range of what the actual longitude value might be.
    pub fn longitude_bounding(&self) -> RangeInclusive<f64> {
        self.precision.range(self.longitude.value())
    }

    pub(crate) fn encode_uncompressed<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        self.latitude.encode_uncompressed(buf, self.precision)?;
        write!(buf, "{}", self.symbol_table)?;
        self.longitude.encode_uncompressed(buf)?;
        write!(buf, "{}", self.symbol_code)?;
        Ok(())
    }

    pub(crate) fn encode_compressed<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        write!(buf, "{}", self.symbol_table)?;

        self.latitude.encode_compressed(buf)?;
        self.longitude.encode_compressed(buf)?;

        write!(buf, "{}", self.symbol_code)?;

        match self.cst {
            AprsCst::CompressedSome { cs, t } => cs.encode(buf, t)?,
            AprsCst::CompressedNone => write!(buf, " sT")?,
            AprsCst::Uncompressed => unreachable!(),
        };

        Ok(())
    }

    /// Parse `!DAO!` pattern from comment data.
    pub(crate) fn dao_in_comment(data: &[u8]) -> Option<Dao> {
        // Look for pattern !X__! where X is a letter
        let s = data;
        for i in 0..s.len().saturating_sub(4) {
            if s[i] == b'!' && s.get(i + 4) == Some(&b'!') {
                let prefix = s[i + 1];
                let d1 = s[i + 2];
                let d2 = s[i + 3];
                if prefix.is_ascii_uppercase() {
                    // Base-91: characters are in range 0x21..0x7B (33..123)
                    if d1 >= 0x21 && d1 <= 0x7B && d2 >= 0x21 && d2 <= 0x7B {
                        return Some(Dao::Base91 {
                            lat_offset: d1 - 33,
                            lon_offset: d2 - 33,
                        });
                    }
                } else if prefix.is_ascii_lowercase() {
                    // Human-readable: digits 0-9
                    if d1.is_ascii_digit() && d2.is_ascii_digit() {
                        return Some(Dao::HumanReadable {
                            lat_digit: d1 - b'0',
                            lon_digit: d2 - b'0',
                        });
                    }
                }
            }
        }
        None
    }

    pub(crate) fn altitude_in_comment(data: &[u8]) -> Option<AprsAltitude> {
        // Convert to a string slice.
        let s = str::from_utf8(data).ok()?;

        // Find the starting index of the "/A=" substring.
        let start_index = s.find("/A=")?;

        // Calculate the index where the number begins.
        let number_start_index = start_index + "/A=".len();

        // Get a slice of the string from that point onward.
        let rest = &s[number_start_index..];

        // Find the end of the number by searching for the first non-digit character.
        let number_end_index = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());

        // Slice the string to get only the number part.
        let number_str = &rest[..number_end_index];

        // Parse the number string. If parsing fails, return None.
        let altitude_value = number_str.parse::<u32>().ok()?;

        // return a new AprsAltitude struct
        Some(AprsAltitude::new(altitude_value as f64))
    }

    /// this function assumes we are getting the head of a byte list
    /// representing a compressed or uncompressed position
    ///
    /// all position representations interleave the symbol table and code
    /// so we stuff it all in here
    pub(crate) fn decode(b: &[u8]) -> Result<(Option<&[u8]>, Self), DecodeError> {
        // make sure we're not empty
        if b.is_empty() {
            return Err(DecodeError::InvalidPosition(b.to_vec()));
        }
        let is_uncompressed_position = (*b.first().unwrap_or(&0) as char).is_numeric();
        if is_uncompressed_position {
            if b.len() < 19 {
                return Err(DecodeError::InvalidPosition(b.to_vec()));
            }
            let (latitude, precision) = Latitude::parse_uncompressed(&b[0..8])?;
            let longitude = Longitude::parse_uncompressed(&b[9..18], precision)?;

            let symbol_table = b[8] as char;
            let symbol_code = b[18] as char;

            // search the comment field for an altitude (e.g. '/A=aaaaaa')
            let comment_bytes = b.get(19..).unwrap_or_default();
            let altitude = Position::altitude_in_comment(comment_bytes);
            let dao = Position::dao_in_comment(comment_bytes);

            // Apply DAO precision offsets to refine lat/lon
            let (latitude, longitude) = if let Some(ref dao) = dao {
                let (lat_adj, lon_adj) = dao.offsets_degrees();
                // Determine sign: offsets are always added in the direction
                // of the base coordinate's sign
                let lat_sign = if latitude.value() >= 0.0 { 1.0 } else { -1.0 };
                let lon_sign = if longitude.value() >= 0.0 { 1.0 } else { -1.0 };
                let new_lat =
                    Latitude::new(latitude.value() + lat_sign * lat_adj).unwrap_or(latitude);
                let new_lon =
                    Longitude::new(longitude.value() + lon_sign * lon_adj).unwrap_or(longitude);
                (new_lat, new_lon)
            } else {
                (latitude, longitude)
            };

            Ok((
                b.get(19..),
                Self {
                    latitude,
                    longitude,
                    precision,
                    symbol_code,
                    symbol_table,
                    cst: AprsCst::Uncompressed,
                    altitude,
                    dao,
                },
            ))
        } else {
            if b.len() < 13 {
                return Err(DecodeError::InvalidPosition(b.to_vec()));
            }
            let symbol_table = b[0] as char;
            let comp_lat = &b[1..5];
            let comp_lon = &b[5..9];
            let symbol_code = b[9] as char;
            let course_speed = &b[10..12];
            let comp_type = b[12];

            b.take(12);

            let latitude = Latitude::parse_compressed(comp_lat)?;
            let longitude = Longitude::parse_compressed(comp_lon)?;

            // From the APRS spec - if the c value is a space,
            // the csT doesn't matter
            let cst = match course_speed[0] {
                b' ' => AprsCst::CompressedNone,
                _ => {
                    let t = comp_type
                        .checked_sub(33)
                        .ok_or_else(|| DecodeError::InvalidPosition(b.to_owned()))?
                        .into();
                    let cs = AprsCompressedCs::parse(course_speed[0], course_speed[1], t)?;
                    AprsCst::CompressedSome { cs, t }
                }
            };

            // get the altitude value
            let altitude: Option<AprsAltitude> = match cst {
                AprsCst::CompressedSome {
                    cs: AprsCompressedCs::Altitude(a),
                    ..
                } => Some(a),
                _ => None,
            };

            Ok((
                b.get(13..),
                Self {
                    latitude,
                    longitude,
                    precision: Precision::default(),
                    symbol_code,
                    symbol_table,
                    cst,
                    altitude,
                    dao: None,
                },
            ))
        }
    }
}
