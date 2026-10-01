//! Sleep restriction: setting and titrating the sleep window.
//!
//! Standard CBT-I rules (Spielman; Perlis et al.):
//!   initial window = mean diary TST over the baseline, never below 5 h,
//!   anchored on a fixed rise time;
//!   then weekly on the last 7 nights:
//!     SE >= 90 %  -> extend window 15 min
//!     SE 85-90 %  -> keep window
//!     SE <  85 %  -> shorten window 15 min (not below 5 h)
//! Older adults commonly use 85 / 80 instead of 90 / 85.

use crate::clock::ClockTime;
use crate::metrics::summarize;
use crate::model::{DiaryEntry, Prescription, Subject};
use chrono::NaiveDate;

pub const STEP_MIN: i32 = 15;
pub const FLOOR_MIN: i32 = 300;
pub const BASELINE_NIGHTS: usize = 7;
pub const REVIEW_DAYS: i64 = 7;
pub const REVIEW_MIN_NIGHTS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    pub extend_at: f64,
    pub reduce_below: f64,
}

impl Thresholds {
    pub fn for_subject(s: &Subject) -> Self {
        if s.older_adult {
            Self { extend_at: 85.0, reduce_below: 80.0 }
        } else {
            Self { extend_at: 90.0, reduce_below: 85.0 }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Recommendation {
    /// Not enough baseline diary to set a window.
    CollectBaseline { have: usize, need: usize },
    /// Set the first sleep window.
    Initial { mean_tst: f64, nights: usize, rx: Prescription },
    /// Window in force; review not due yet.
    Wait { since: NaiveDate, days: i64, nights: usize },
    /// Weekly review result.
    Review { se: f64, nights: usize, delta: i32, reason: String, rx: Prescription },
}

impl Recommendation {
    pub fn prescription(&self) -> Option<&Prescription> {
        match self {
            Recommendation::Initial { rx, .. } | Recommendation::Review { rx, .. } => Some(rx),
            _ => None,
        }
    }
}

fn round_to_step(m: f64) -> i32 {
    ((m / STEP_MIN as f64).round() as i32) * STEP_MIN
}

/// Median time out of bed, rounded to 15 min: a default fixed rise time.
pub fn median_rise(entries: &[&DiaryEntry]) -> Option<ClockTime> {
    let mut offs: Vec<i32> = entries.iter().map(|e| e.out_of_bed.offset()).collect();
    if offs.is_empty() {
        return None;
    }
    offs.sort_unstable();
    let mid = offs[offs.len() / 2];
    Some(ClockTime::from_offset(round_to_step(mid as f64)))
}

/// Nights strictly before `today`, most recent `limit`.
fn recent<'a>(entries: &'a [DiaryEntry], from: Option<NaiveDate>, today: NaiveDate, limit: usize) -> Vec<&'a DiaryEntry> {
    let mut v: Vec<&DiaryEntry> = entries
        .iter()
        .filter(|e| e.night < today && from.is_none_or(|f| e.night >= f))
        .collect();
    let skip = v.len().saturating_sub(limit);
    v.drain(..skip);
    v
}

pub fn recommend(s: &Subject, today: NaiveDate) -> Recommendation {
    let th = Thresholds::for_subject(s);
    let Some(current) = s.prescription_for(today) else {
        let baseline = recent(&s.entries, None, today, 14);
        if baseline.len() < BASELINE_NIGHTS {
            return Recommendation::CollectBaseline { have: baseline.len(), need: BASELINE_NIGHTS };
        }
        let sum = summarize(baseline.iter().copied()).unwrap();
        let window = round_to_step(sum.tst).max(FLOOR_MIN);
        let rise = s.rise_target.or_else(|| median_rise(&baseline)).unwrap();
        return Recommendation::Initial {
            mean_tst: sum.tst,
            nights: sum.nights,
            rx: Prescription {
                from: today,
                bed: rise.add(-window),
                rise,
                note: format!("Initial window from mean TST {:.0} min", sum.tst),
            },
        };
    };

    let days = (today - current.from).num_days();
    let window_nights = recent(&s.entries, Some(current.from), today, 7);
    if days < REVIEW_DAYS || window_nights.len() < REVIEW_MIN_NIGHTS {
        return Recommendation::Wait { since: current.from, days, nights: window_nights.len() };
    }
    let sum = summarize(window_nights.iter().copied()).unwrap();
    let window = current.window_min();
    let (delta, reason) = if sum.se >= th.extend_at {
        (STEP_MIN, format!("SE {:.0}% is at or above {:.0}%: extend by 15 min", sum.se, th.extend_at))
    } else if sum.se < th.reduce_below {
        if window - STEP_MIN < FLOOR_MIN {
            (0, format!("SE {:.0}% is below {:.0}% but the window is at the 5 h floor: keep", sum.se, th.reduce_below))
        } else {
            (-STEP_MIN, format!("SE {:.0}% is below {:.0}%: shorten by 15 min", sum.se, th.reduce_below))
        }
    } else {
        (0, format!("SE {:.0}% is between {:.0}% and {:.0}%: keep the window", sum.se, th.reduce_below, th.extend_at))
    };
    let new_window = window + delta;
    Recommendation::Review {
        se: sum.se,
        nights: sum.nights,
        delta,
        reason: reason.clone(),
        rx: Prescription { from: today, bed: current.rise.add(-new_window), rise: current.rise, note: reason },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::ClockTime;

    fn day(n: i64) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 1).unwrap() + chrono::Duration::days(n)
    }

    fn night(n: i64, bed: &str, sol: u16, waso: u16, fw: &str, oob: &str) -> DiaryEntry {
        DiaryEntry {
            night: day(n),
            bedtime: ClockTime::parse(bed).unwrap(),
            lights_out: ClockTime::parse(bed).unwrap(),
            sol_min: sol,
            awakenings: if waso > 0 { 1 } else { 0 },
            waso_min: waso,
            final_wake: ClockTime::parse(fw).unwrap(),
            out_of_bed: ClockTime::parse(oob).unwrap(),
            quality: 2,
            nap_min: 0,
            caffeine: 0,
            alcohol: 0,
            sleep_aid: String::new(),
            notes: String::new(),
        }
    }

    #[test]
    fn needs_seven_baseline_nights() {
        let mut s = Subject::new("t", day(0));
        for n in 0..5 {
            s.upsert_entry(night(n, "22:30", 60, 60, "05:30", "07:00"));
        }
        assert_eq!(recommend(&s, day(5)), Recommendation::CollectBaseline { have: 5, need: 7 });
    }

    #[test]
    fn initial_window_is_mean_tst_with_floor() {
        let mut s = Subject::new("t", day(0));
        s.rise_target = ClockTime::parse("06:30");
        // Asleep window 22:30-05:30 = 420 - 60 - 60 = 300 -> 5h, bed 01:30.
        for n in 0..7 {
            s.upsert_entry(night(n, "22:30", 60, 60, "05:30", "07:00"));
        }
        let Recommendation::Initial { rx, .. } = recommend(&s, day(7)) else { panic!() };
        assert_eq!(rx.window_min(), 300);
        assert_eq!(rx.bed, ClockTime::parse("01:30").unwrap());

        // Even worse sleep never goes below the 5 h floor.
        for n in 0..7 {
            s.upsert_entry(night(n, "22:30", 120, 120, "05:30", "07:00"));
        }
        let rec = recommend(&s, day(7));
        assert_eq!(rec.prescription().unwrap().window_min(), FLOOR_MIN);
    }

    #[test]
    fn weekly_review_follows_se_thresholds() {
        let mut s = Subject::new("t", day(0));
        s.add_prescription(Prescription {
            from: day(0),
            bed: ClockTime::parse("00:30").unwrap(),
            rise: ClockTime::parse("06:30").unwrap(),
            note: String::new(),
        });
        // 360 min in bed, 20 min awake -> SE 94% -> extend.
        for n in 0..7 {
            s.upsert_entry(night(n, "00:30", 10, 10, "06:30", "06:30"));
        }
        let Recommendation::Review { delta, rx, .. } = recommend(&s, day(7)) else { panic!() };
        assert_eq!(delta, 15);
        assert_eq!(rx.bed, ClockTime::parse("00:15").unwrap());

        // 360 min in bed, 72 min awake -> SE 80% -> shorten.
        for n in 0..7 {
            s.upsert_entry(night(n, "00:30", 36, 36, "06:30", "06:30"));
        }
        let Recommendation::Review { delta, .. } = recommend(&s, day(7)) else { panic!() };
        assert_eq!(delta, -15);

        // Too early for a review.
        assert!(matches!(recommend(&s, day(4)), Recommendation::Wait { .. }));
    }
}
