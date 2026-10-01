//! Synthetic subject for building and demoing without real diary data.
//!
//! Four weeks: two baseline weeks of fragmented sleep (SE around 65 %),
//! then a restricted window from day 14 that consolidates sleep.

use crate::clock::ClockTime;
use crate::model::{DiaryEntry, Prescription, SessionRecord, Subject};
use crate::titration::{Recommendation, recommend};
use chrono::{Duration, NaiveDate};

/// Small deterministic generator so the demo is identical on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / ((1u64 << 31) as f64)
    }
    /// Roughly normal (sum of uniforms).
    fn around(&mut self, mean: f64, spread: f64) -> f64 {
        let u: f64 = (0..4).map(|_| self.next()).sum::<f64>() / 4.0;
        mean + (u - 0.5) * 2.0 * spread
    }
}

fn round5(x: f64) -> i32 {
    ((x / 5.0).round() as i32) * 5
}

pub fn generate(id: &str, today: NaiveDate) -> Subject {
    let started = today - Duration::days(28);
    let mut s = Subject::new(id, started);
    s.rise_target = ClockTime::new(6, 30);
    let mut r = Lcg(0x5EED_CB71);

    for day in 0..28i64 {
        let night = started + Duration::days(day);
        // A few missed mornings, as in real diaries.
        if matches!(day, 4 | 17 | 24) {
            continue;
        }
        if day == 14 {
            if let Recommendation::Initial { rx, .. } = recommend(&s, night) {
                s.add_prescription(Prescription { note: "Initial window (session 2)".into(), ..rx });
            }
        }
        let e = match s.prescription_for(night).cloned() {
            None => baseline_night(&mut r, night, day),
            Some(rx) => restricted_night(&mut r, night, day, &rx),
        };
        debug_assert!(e.validate().is_ok(), "{:?}", e.validate());
        s.upsert_entry(e);
    }

    s.sessions = vec![
        SessionRecord {
            number: 1,
            date: Some(started),
            isi: Some(19),
            done: ["dx", "screen", "isi", "meds", "diary"].map(String::from).to_vec(),
            notes: "Long sleep latency and early waking for 2 years. Zolpidem 5 mg most nights.".into(),
        },
        SessionRecord {
            number: 2,
            date: Some(started + Duration::days(14)),
            isi: None,
            done: ["review", "rise", "window", "sc", "safety"].map(String::from).to_vec(),
            notes: String::new(),
        },
        SessionRecord {
            number: 3,
            date: Some(started + Duration::days(21)),
            isi: Some(14),
            done: ["titrate", "isi"].map(String::from).to_vec(),
            notes: "Window kept; two early bedtimes discussed.".into(),
        },
    ];
    s
}

fn baseline_night(r: &mut Lcg, night: NaiveDate, day: i64) -> DiaryEntry {
    let bed = ClockTime::new(22, 30).unwrap().add(round5(r.around(0.0, 40.0)));
    let lights_out = bed.add(round5(r.around(20.0, 15.0)).max(0));
    let final_wake = ClockTime::new(5, 50).unwrap().add(round5(r.around(0.0, 35.0)));
    let out_of_bed = final_wake.add(round5(r.around(70.0, 30.0)).max(5));
    let span = final_wake.offset() - lights_out.offset();
    let sol = round5(r.around(50.0, 30.0)).clamp(10, span / 3);
    let waso = round5(r.around(60.0, 30.0)).clamp(10, span / 3);
    DiaryEntry {
        night,
        bedtime: bed,
        lights_out,
        sol_min: sol as u16,
        awakenings: r.around(3.0, 2.0).round().clamp(1.0, 6.0) as u8,
        waso_min: waso as u16,
        final_wake,
        out_of_bed,
        quality: r.around(2.0, 1.2).round().clamp(1.0, 5.0) as u8,
        nap_min: if r.next() < 0.3 { round5(r.around(40.0, 20.0)).max(10) as u16 } else { 0 },
        caffeine: r.around(3.0, 1.5).round().max(0.0) as u8,
        alcohol: if day % 7 >= 4 { r.around(1.5, 1.5).round().max(0.0) as u8 } else { 0 },
        sleep_aid: if r.next() < 0.6 { "zolpidem 5 mg".into() } else { String::new() },
        notes: String::new(),
    }
}

fn restricted_night(r: &mut Lcg, night: NaiveDate, day: i64, rx: &Prescription) -> DiaryEntry {
    // Week 3 still settling (SE in the 80s), week 4 consolidated (SE above 90).
    let settled = day >= 21;
    let early = if r.next() < if settled { 0.08 } else { 0.25 } { round5(r.around(35.0, 15.0)) } else { round5(r.around(0.0, 8.0)) };
    let bed = rx.bed.add(-early);
    let lights_out = bed.add(round5(r.around(5.0, 5.0)).max(0));
    let out_of_bed = rx.rise.add(round5(r.around(0.0, 8.0)));
    let final_wake = out_of_bed.add(-round5(r.around(if settled { 4.0 } else { 15.0 }, 4.0)).max(0));
    let span = final_wake.offset() - lights_out.offset();
    let sol = round5(r.around(if settled { 9.0 } else { 22.0 }, 5.0)).clamp(5, span / 4);
    let waso = round5(r.around(if settled { 5.0 } else { 25.0 }, 5.0)).clamp(0, span / 4);
    DiaryEntry {
        night,
        bedtime: bed,
        lights_out,
        sol_min: sol as u16,
        awakenings: if waso == 0 { 0 } else { r.around(1.5, 1.0).round().clamp(1.0, 4.0) as u8 },
        waso_min: waso as u16,
        final_wake,
        out_of_bed,
        quality: r.around(if settled { 3.6 } else { 2.8 }, 1.0).round().clamp(1.0, 5.0) as u8,
        nap_min: if !settled && r.next() < 0.15 { 20 } else { 0 },
        caffeine: r.around(2.0, 1.0).round().max(0.0) as u8,
        alcohol: if day % 7 == 5 { 1 } else { 0 },
        sleep_aid: if !settled && r.next() < 0.3 { "zolpidem 5 mg".into() } else { String::new() },
        notes: if early > 20 { "Very sleepy, went to bed early".into() } else { String::new() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::summarize;

    #[test]
    fn synthetic_subject_is_valid_and_improves() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let s = generate("demo", today);
        assert!(s.entries.iter().all(|e| e.validate().is_ok()));
        assert_eq!(s.prescriptions.len(), 1);
        let week = |w: i64| summarize(s.entries.iter().filter(|e| s.week_of(e.night) == w)).unwrap();
        assert!(week(0).se < 75.0, "baseline SE {}", week(0).se);
        assert!(week(3).se > week(0).se + 15.0);
    }
}
