//! Wall-clock times on a diary night.
//!
//! A diary night runs noon to noon: 23:30 and 06:00 belong to the same night.
//! `offset()` places a clock time on that axis so intervals never wrap.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockTime(u16);

impl ClockTime {
    pub const MINUTES_PER_DAY: i32 = 1440;

    pub fn new(hour: u16, minute: u16) -> Option<Self> {
        (hour < 24 && minute < 60).then_some(Self(hour * 60 + minute))
    }

    pub fn from_minutes(m: i32) -> Self {
        Self(m.rem_euclid(Self::MINUTES_PER_DAY) as u16)
    }

    pub fn minutes(self) -> i32 {
        self.0 as i32
    }

    /// Minutes since noon on the night's start date (0..1440).
    pub fn offset(self) -> i32 {
        (self.minutes() - 720).rem_euclid(Self::MINUTES_PER_DAY)
    }

    pub fn from_offset(offset: i32) -> Self {
        Self::from_minutes(offset + 720)
    }

    pub fn plus(self, minutes: i32) -> Self {
        Self::from_minutes(self.minutes() + minutes)
    }

    /// Accepts `23:30`, `2330`, `930`, `9:30`, `23.30`, `23h30`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        let sep = s.find([':', '.', 'h']);
        let (h, m) = match sep {
            Some(i) => {
                let h: u16 = s[..i].trim().parse().ok()?;
                let m: u16 = s[i + 1..].trim().parse().ok()?;
                (h, m)
            }
            None if digits.len() == s.len() && (3..=4).contains(&digits.len()) => {
                let split = digits.len() - 2;
                (digits[..split].parse().ok()?, digits[split..].parse().ok()?)
            }
            None if digits.len() == s.len() && (1..=2).contains(&digits.len()) => {
                (digits.parse().ok()?, 0)
            }
            None => return None,
        };
        Self::new(h, m)
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.0 / 60, self.0 % 60)
    }
}

impl Serialize for ClockTime {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ClockTime {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        ClockTime::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("bad time {s:?}")))
    }
}

/// `345` -> `5h45`.
pub fn fmt_duration(minutes: f64) -> String {
    let m = minutes.round() as i64;
    let sign = if m < 0 { "-" } else { "" };
    let m = m.abs();
    format!("{sign}{}h{:02}", m / 60, m % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_forms() {
        assert_eq!(ClockTime::parse("23:30"), ClockTime::new(23, 30));
        assert_eq!(ClockTime::parse("2330"), ClockTime::new(23, 30));
        assert_eq!(ClockTime::parse("630"), ClockTime::new(6, 30));
        assert_eq!(ClockTime::parse("7"), ClockTime::new(7, 0));
        assert_eq!(ClockTime::parse("6.15"), ClockTime::new(6, 15));
        assert_eq!(ClockTime::parse("24:00"), None);
        assert_eq!(ClockTime::parse("abc"), None);
    }

    #[test]
    fn offset_keeps_night_contiguous() {
        let bed = ClockTime::new(23, 30).unwrap();
        let rise = ClockTime::new(6, 0).unwrap();
        assert_eq!(rise.offset() - bed.offset(), 390);
        assert_eq!(ClockTime::from_offset(bed.offset()), bed);
    }

    #[test]
    fn formats_durations() {
        assert_eq!(fmt_duration(345.0), "5h45");
        assert_eq!(fmt_duration(-15.0), "-0h15");
    }
}
