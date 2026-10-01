//! Sleep-continuity measures derived from diary entries.
//!
//! TIB  time in bed           = out of bed - got into bed
//! TST  total sleep time      = (final wake - tried to sleep) - SOL - WASO
//! SE   sleep efficiency      = TST / TIB
//! SOL  sleep onset latency, WASO wake after sleep onset (as reported)
//! EMA  early-morning awake   = out of bed - final wake

use crate::model::{DiaryEntry, Prescription};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NightMetrics {
    pub tib: i32,
    pub tst: i32,
    pub sol: i32,
    pub waso: i32,
    pub ema: i32,
    /// Percent.
    pub se: f64,
}

pub fn night(e: &DiaryEntry) -> NightMetrics {
    let tib = e.out_of_bed.offset() - e.bedtime.offset();
    let tst = (e.final_wake.offset() - e.lights_out.offset() - e.sol_min as i32 - e.waso_min as i32).max(0);
    NightMetrics {
        tib,
        tst,
        sol: e.sol_min as i32,
        waso: e.waso_min as i32,
        ema: e.out_of_bed.offset() - e.final_wake.offset(),
        se: if tib > 0 { 100.0 * tst as f64 / tib as f64 } else { 0.0 },
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub nights: usize,
    pub tib: f64,
    pub tst: f64,
    pub sol: f64,
    pub waso: f64,
    pub ema: f64,
    /// Mean TST over mean TIB, percent.
    pub se: f64,
    pub quality: f64,
    pub nap_days: usize,
}

pub fn summarize<'a>(entries: impl IntoIterator<Item = &'a DiaryEntry>) -> Option<Summary> {
    let mut n = 0usize;
    let (mut tib, mut tst, mut sol, mut waso, mut ema, mut q) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let mut nap_days = 0;
    for e in entries {
        let m = night(e);
        n += 1;
        tib += m.tib as f64;
        tst += m.tst as f64;
        sol += m.sol as f64;
        waso += m.waso as f64;
        ema += m.ema as f64;
        q += e.quality as f64;
        if e.nap_min > 0 {
            nap_days += 1;
        }
    }
    if n == 0 {
        return None;
    }
    let k = n as f64;
    Some(Summary {
        nights: n,
        tib: tib / k,
        tst: tst / k,
        sol: sol / k,
        waso: waso / k,
        ema: ema / k,
        se: if tib > 0.0 { 100.0 * tst / tib } else { 0.0 },
        quality: q / k,
        nap_days,
    })
}

/// How far a night strayed from the prescribed window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deviation {
    EarlyBed(i32),
    LateRise(i32),
    Nap(u16),
}

impl Deviation {
    pub fn describe(&self) -> String {
        match self {
            Deviation::EarlyBed(m) => format!("in bed {m} min early"),
            Deviation::LateRise(m) => format!("up {m} min late"),
            Deviation::Nap(m) => format!("napped {m} min"),
        }
    }
}

/// Minutes of slack before a bed or rise time counts as off-schedule.
pub const TOLERANCE_MIN: i32 = 15;

pub fn deviations(e: &DiaryEntry, rx: &Prescription) -> Vec<Deviation> {
    let mut out = Vec::new();
    let early = rx.bed.offset() - e.bedtime.offset();
    if early > TOLERANCE_MIN {
        out.push(Deviation::EarlyBed(early));
    }
    let late = e.out_of_bed.offset() - rx.rise.offset();
    if late > TOLERANCE_MIN {
        out.push(Deviation::LateRise(late));
    }
    if e.nap_min > 0 {
        out.push(Deviation::Nap(e.nap_min));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::ClockTime;
    use chrono::NaiveDate;

    pub fn entry(bed: &str, lo: &str, sol: u16, waso: u16, fw: &str, oob: &str) -> DiaryEntry {
        DiaryEntry {
            night: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            bedtime: ClockTime::parse(bed).unwrap(),
            lights_out: ClockTime::parse(lo).unwrap(),
            sol_min: sol,
            awakenings: if waso > 0 { 2 } else { 0 },
            waso_min: waso,
            final_wake: ClockTime::parse(fw).unwrap(),
            out_of_bed: ClockTime::parse(oob).unwrap(),
            quality: 3,
            nap_min: 0,
            caffeine: 0,
            alcohol: 0,
            sleep_aid: String::new(),
            notes: String::new(),
        }
    }

    #[test]
    fn computes_consensus_diary_measures() {
        // In bed 22:30-07:00 (510), asleep window 23:00-06:00 (420) minus 30 SOL, 60 WASO.
        let m = night(&entry("22:30", "23:00", 30, 60, "06:00", "07:00"));
        assert_eq!(m.tib, 510);
        assert_eq!(m.tst, 330);
        assert_eq!(m.ema, 60);
        assert!((m.se - 64.705).abs() < 0.01);
    }

    #[test]
    fn flags_schedule_deviations() {
        let rx = Prescription {
            from: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            bed: ClockTime::parse("00:30").unwrap(),
            rise: ClockTime::parse("06:00").unwrap(),
            note: String::new(),
        };
        let e = entry("23:30", "23:45", 10, 0, "05:50", "06:40");
        assert_eq!(deviations(&e, &rx), vec![Deviation::EarlyBed(60), Deviation::LateRise(40)]);
        let ok = entry("00:40", "00:40", 10, 0, "05:50", "06:10");
        assert!(deviations(&ok, &rx).is_empty());
    }
}
