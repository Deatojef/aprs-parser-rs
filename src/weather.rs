//! Weather report parsing and encoding for APRS.
//!
//! Supports both position-based weather (symbol `/_`) where weather data
//! appears in the comment field, and positionless weather reports using
//! the `_` data type identifier.

use std::io::Write;

use crate::Callsign;
use crate::DecodeError;
use crate::EncodeError;

/// Weather data fields common to both position-based and positionless weather reports.
#[derive(PartialEq, Debug, Clone, Default)]
pub struct AprsWeather {
    pub wind_direction: Option<u16>,
    pub wind_speed: Option<u16>,
    pub wind_gust: Option<u16>,
    pub temperature: Option<i16>,
    pub rain_last_hour: Option<u16>,
    pub rain_last_24h: Option<u16>,
    pub rain_since_midnight: Option<u16>,
    pub humidity: Option<u8>,
    pub barometric_pressure: Option<u32>,
    pub luminosity: Option<u16>,
    pub snow_last_24h: Option<f32>,
    pub raw_rain_counter: Option<u16>,
}

fn parse_u16_field(s: &[u8]) -> Option<u16> {
    if s.iter().all(|&c| c == b'.' || c == b' ') {
        return None;
    }
    std::str::from_utf8(s).ok()?.trim().parse().ok()
}

fn parse_i16_field(s: &[u8]) -> Option<i16> {
    if s.iter().all(|&c| c == b'.' || c == b' ') {
        return None;
    }
    std::str::from_utf8(s).ok()?.trim().parse().ok()
}

impl AprsWeather {
    /// Decode weather data from the byte slice.
    /// Expected format: `DDD/SSS` wind direction/speed prefix, then lettered fields.
    /// Example: `220/004g005t077r000p000P000h50b09900`
    pub fn decode(b: &[u8]) -> Result<Self, DecodeError> {
        if b.len() < 7 {
            return Err(DecodeError::InvalidWeather(b.to_vec()));
        }

        let mut wx = AprsWeather::default();

        // First 3 bytes = wind direction
        let dir_bytes = &b[0..3];
        wx.wind_direction = parse_u16_field(dir_bytes);

        // byte 3 should be '/'
        if b[3] != b'/' {
            return Err(DecodeError::InvalidWeather(b.to_vec()));
        }

        // bytes 4..7 = wind speed
        let spd_bytes = &b[4..7];
        wx.wind_speed = parse_u16_field(spd_bytes);

        // Parse lettered fields from position 7 onward
        let mut i = 7;
        while i < b.len() {
            let key = b[i];
            i += 1;
            match key {
                b'g' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    wx.wind_gust = parse_u16_field(&b[i..i + 3]);
                    i += 3;
                }
                b't' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    wx.temperature = parse_i16_field(&b[i..i + 3]);
                    i += 3;
                }
                b'r' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    wx.rain_last_hour = parse_u16_field(&b[i..i + 3]);
                    i += 3;
                }
                b'p' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    wx.rain_last_24h = parse_u16_field(&b[i..i + 3]);
                    i += 3;
                }
                b'P' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    wx.rain_since_midnight = parse_u16_field(&b[i..i + 3]);
                    i += 3;
                }
                b'h' => {
                    if i + 2 > b.len() {
                        break;
                    }
                    let val = parse_u16_field(&b[i..i + 2]);
                    wx.humidity = val.map(|v| if v == 0 { 100 } else { v as u8 });
                    i += 2;
                }
                b'b' => {
                    if i + 5 > b.len() {
                        break;
                    }
                    wx.barometric_pressure = std::str::from_utf8(&b[i..i + 5])
                        .ok()
                        .and_then(|s| s.trim().parse().ok());
                    i += 5;
                }
                b'L' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    wx.luminosity = parse_u16_field(&b[i..i + 3]).map(|v| v + 1000);
                    i += 3;
                }
                b'l' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    wx.luminosity = parse_u16_field(&b[i..i + 3]);
                    i += 3;
                }
                b's' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    if let Some(v) = parse_u16_field(&b[i..i + 3]) {
                        wx.snow_last_24h = Some(v as f32 / 10.0);
                    }
                    i += 3;
                }
                b'#' => {
                    if i + 3 > b.len() {
                        break;
                    }
                    wx.raw_rain_counter = parse_u16_field(&b[i..i + 3]);
                    i += 3;
                }
                _ => {
                    // Unknown field, stop parsing weather data
                    break;
                }
            }
        }

        Ok(wx)
    }

    /// Encode weather data fields to the writer.
    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        // Wind direction
        match self.wind_direction {
            Some(d) => write!(buf, "{:03}", d)?,
            None => write!(buf, "...")?,
        }
        write!(buf, "/")?;
        // Wind speed
        match self.wind_speed {
            Some(s) => write!(buf, "{:03}", s)?,
            None => write!(buf, "...")?,
        }

        if let Some(g) = self.wind_gust {
            write!(buf, "g{:03}", g)?;
        }
        if let Some(t) = self.temperature {
            write!(buf, "t{:03}", t)?;
        }
        if let Some(r) = self.rain_last_hour {
            write!(buf, "r{:03}", r)?;
        }
        if let Some(p) = self.rain_last_24h {
            write!(buf, "p{:03}", p)?;
        }
        if let Some(p) = self.rain_since_midnight {
            write!(buf, "P{:03}", p)?;
        }
        if let Some(h) = self.humidity {
            let h_val = if h == 100 { 0u8 } else { h };
            write!(buf, "h{:02}", h_val)?;
        }
        if let Some(b_val) = self.barometric_pressure {
            write!(buf, "b{:05}", b_val)?;
        }
        if let Some(l) = self.luminosity {
            if l >= 1000 {
                write!(buf, "L{:03}", l - 1000)?;
            } else {
                write!(buf, "l{:03}", l)?;
            }
        }
        if let Some(s) = self.snow_last_24h {
            write!(buf, "s{:03}", (s * 10.0) as u16)?;
        }
        if let Some(r) = self.raw_rain_counter {
            write!(buf, "#{:03}", r)?;
        }

        Ok(())
    }
}

/// A positionless weather report (data type identifier `_`).
/// Format: `_MMDDHHMM` followed by weather data and optional comment.
#[derive(PartialEq, Debug, Clone)]
pub struct AprsPositionlessWeather {
    pub to: Callsign,
    pub timestamp: Vec<u8>,
    pub weather: AprsWeather,
    pub comment: Vec<u8>,
}

impl AprsPositionlessWeather {
    pub fn decode(b: &[u8], to: Callsign) -> Result<Self, DecodeError> {
        // Positionless weather: _MMDDHHMM followed by weather data
        if b.len() < 8 {
            return Err(DecodeError::InvalidWeather(b.to_vec()));
        }

        let timestamp = b[0..8].to_vec();
        let weather_data = &b[8..];

        let weather = AprsWeather::decode(weather_data)?;

        Ok(Self {
            to,
            timestamp,
            weather,
            comment: vec![],
        })
    }

    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        write!(buf, "_")?;
        buf.write_all(&self.timestamp)?;
        self.weather.encode(buf)?;
        buf.write_all(&self.comment)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_basic_weather() {
        let data = b"220/004g005t077r000p000P000h50b09900";
        let wx = AprsWeather::decode(data).unwrap();
        assert_eq!(wx.wind_direction, Some(220));
        assert_eq!(wx.wind_speed, Some(4));
        assert_eq!(wx.wind_gust, Some(5));
        assert_eq!(wx.temperature, Some(77));
        assert_eq!(wx.rain_last_hour, Some(0));
        assert_eq!(wx.rain_last_24h, Some(0));
        assert_eq!(wx.rain_since_midnight, Some(0));
        assert_eq!(wx.humidity, Some(50));
        assert_eq!(wx.barometric_pressure, Some(9900));
    }

    #[test]
    fn decode_weather_no_wind() {
        let data = b".../...g005t077";
        let wx = AprsWeather::decode(data).unwrap();
        assert_eq!(wx.wind_direction, None);
        assert_eq!(wx.wind_speed, None);
        assert_eq!(wx.wind_gust, Some(5));
        assert_eq!(wx.temperature, Some(77));
    }

    #[test]
    fn decode_negative_temperature() {
        let data = b"000/000g000t-10";
        let wx = AprsWeather::decode(data).unwrap();
        assert_eq!(wx.temperature, Some(-10));
    }

    #[test]
    fn decode_humidity_100() {
        let data = b"000/000h00";
        let wx = AprsWeather::decode(data).unwrap();
        assert_eq!(wx.humidity, Some(100));
    }

    #[test]
    fn decode_luminosity_high() {
        let data = b"000/000L042";
        let wx = AprsWeather::decode(data).unwrap();
        assert_eq!(wx.luminosity, Some(1042));
    }

    #[test]
    fn decode_luminosity_low() {
        let data = b"000/000l042";
        let wx = AprsWeather::decode(data).unwrap();
        assert_eq!(wx.luminosity, Some(42));
    }

    #[test]
    fn encode_decode_roundtrip() {
        let wx = AprsWeather {
            wind_direction: Some(220),
            wind_speed: Some(4),
            wind_gust: Some(5),
            temperature: Some(77),
            rain_last_hour: Some(0),
            rain_last_24h: Some(0),
            rain_since_midnight: Some(0),
            humidity: Some(50),
            barometric_pressure: Some(9900),
            luminosity: None,
            snow_last_24h: None,
            raw_rain_counter: None,
        };

        let mut buf = vec![];
        wx.encode(&mut buf).unwrap();
        let decoded = AprsWeather::decode(&buf).unwrap();
        assert_eq!(wx, decoded);
    }

    #[test]
    fn decode_positionless_weather() {
        let data = b"10071820220/004g005t077";
        let to = Callsign::new_no_ssid("APRS");
        let pw = AprsPositionlessWeather::decode(data, to).unwrap();
        assert_eq!(pw.timestamp, b"10071820");
        assert_eq!(pw.weather.wind_direction, Some(220));
        assert_eq!(pw.weather.wind_speed, Some(4));
        assert_eq!(pw.weather.temperature, Some(77));
    }

    #[test]
    fn encode_positionless_weather() {
        let pw = AprsPositionlessWeather {
            to: Callsign::new_no_ssid("APRS"),
            timestamp: b"10071820".to_vec(),
            weather: AprsWeather {
                wind_direction: Some(220),
                wind_speed: Some(4),
                wind_gust: Some(5),
                temperature: Some(77),
                ..AprsWeather::default()
            },
            comment: vec![],
        };

        let mut buf = vec![];
        pw.encode(&mut buf).unwrap();
        assert_eq!(buf, b"_10071820220/004g005t077");
    }

    #[test]
    fn positionless_roundtrip() {
        let data = b"10071820220/004g005t077";
        let to = Callsign::new_no_ssid("APRS");
        let pw = AprsPositionlessWeather::decode(data, to.clone()).unwrap();
        let mut buf = vec![];
        pw.encode(&mut buf).unwrap();
        // strip the leading '_' for comparison with input
        assert_eq!(&buf[1..], data);
    }
}
