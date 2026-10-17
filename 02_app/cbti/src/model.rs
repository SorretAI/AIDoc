//! Subject record: diary entries, sleep-window prescriptions, sessions.
//!
//! Diary items follow the core Consensus Sleep Diary (Carney et al., 2012).

use crate::clock::ClockTime;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiaryEntry {
    /// Date the night began (an entry written on Tuesday morning is Monday's night).
    pub night: NaiveDate,
    /// Q1 What time did you get into bed?
    pub bedtime: ClockTime,
    /// Q2 What time did you try to go to sleep?
    pub lights_out: ClockTime,
    /// Q3 How long did it take you to fall asleep? (minutes)
    pub sol_min: u16,
    /// Q4 How many times did you wake up, not counting your final awakening?
    pub awakenings: u8,
    /// Q5 In total, how long did these awakenings last? (minutes)
    pub waso_min: u16,
    /// Q6 What time was your final awakening?
    pub final_wake: ClockTime,
    /// Q7 What time did you get out of bed for the day?
    pub out_of_bed: ClockTime,
    /// Q8 How would you rate the quality of your sleep? 1 very poor .. 5 very good
    pub quality: u8,
    #[serde(default)]
    pub nap_min: u16,
    #[serde(default)]
    pub caffeine: u8,
    #[serde(default)]
    pub alcohol: u8,
    #[serde(default)]
    pub sleep_aid: String,
    #[serde(default)]
    pub notes: String,
}

impl DiaryEntry {
    pub fn sleep_onset(&self) -> ClockTime {
        self.lights_out.plus(self.sol_min as i32)
    }

    /// Checks the night is internally consistent. Times must run
    /// bed -> lights out -> final wake -> out of bed within one noon-to-noon night.
    pub fn validate(&self) -> Result<(), String> {
        let (bed, lo, fw, oob) = (
            self.bedtime.offset(),
            self.lights_out.offset(),
            self.final_wake.offset(),
            self.out_of_bed.offset(),
        );
        if lo < bed {
            return Err("Tried to sleep is before getting into bed".into());
        }
        if fw < lo {
            return Err("Final awakening is before trying to sleep".into());
        }
        if oob < fw {
            return Err("Got out of bed before the final awakening".into());
        }
        let window = fw - lo;
        if (self.sol_min + self.waso_min) as i32 > window {
            return Err(format!(
                "Time to fall asleep plus time awake ({} min) exceeds the {} min between trying to sleep and final awakening",
                self.sol_min + self.waso_min,
                window
            ));
        }
        if self.waso_min > 0 && self.awakenings == 0 {
            return Err("Minutes awake after sleep is set but times woke up is 0".into());
        }
        if !(1..=5).contains(&self.quality) {
            return Err("Sleep quality must be 1 to 5".into());
        }
        Ok(())
    }
}

/// A prescribed sleep window: time allowed in bed, set by the clinician.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prescription {
    pub from: NaiveDate,
    pub bed: ClockTime,
    pub rise: ClockTime,
    #[serde(default)]
    pub note: String,
}

impl Prescription {
    pub fn window_min(&self) -> i32 {
        (self.rise.minutes() - self.bed.minutes()).rem_euclid(ClockTime::MINUTES_PER_DAY)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub number: u8,
    #[serde(default)]
    pub date: Option<NaiveDate>,
    /// Insomnia Severity Index, 0..28.
    #[serde(default)]
    pub isi: Option<u8>,
    /// Keys of completed protocol steps.
    #[serde(default)]
    pub done: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subject {
    pub id: String,
    /// First day of baseline diary collection.
    pub started: NaiveDate,
    /// Uses the gentler 85/80 titration thresholds often applied to older adults.
    #[serde(default)]
    pub older_adult: bool,
    /// Fixed rise time agreed with the subject.
    #[serde(default)]
    pub rise_target: Option<ClockTime>,
    /// Keys of screening items that are present (cautions for sleep restriction).
    #[serde(default)]
    pub screening: Vec<String>,
    #[serde(default)]
    pub prescriptions: Vec<Prescription>,
    #[serde(default)]
    pub sessions: Vec<SessionRecord>,
    #[serde(default)]
    pub entries: Vec<DiaryEntry>,
}

impl Subject {
    pub fn new(id: impl Into<String>, started: NaiveDate) -> Self {
        Self {
            id: id.into(),
            started,
            older_adult: false,
            rise_target: None,
            screening: Vec::new(),
            prescriptions: Vec::new(),
            sessions: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// Inserts or replaces the entry for that night. Returns true if replaced.
    pub fn upsert_entry(&mut self, entry: DiaryEntry) -> bool {
        let replaced = match self.entries.iter_mut().find(|e| e.night == entry.night) {
            Some(slot) => {
                *slot = entry;
                true
            }
            None => {
                self.entries.push(entry);
                false
            }
        };
        self.entries.sort_by_key(|e| e.night);
        replaced
    }

    pub fn remove_entry(&mut self, night: NaiveDate) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.night != night);
        self.entries.len() != before
    }

    pub fn entry(&self, night: NaiveDate) -> Option<&DiaryEntry> {
        self.entries.iter().find(|e| e.night == night)
    }

    /// The prescription in force on a given night.
    pub fn prescription_for(&self, night: NaiveDate) -> Option<&Prescription> {
        self.prescriptions
            .iter()
            .filter(|p| p.from <= night)
            .max_by_key(|p| p.from)
    }

    pub fn add_prescription(&mut self, rx: Prescription) {
        self.prescriptions.retain(|p| p.from != rx.from);
        self.prescriptions.push(rx);
        self.prescriptions.sort_by_key(|p| p.from);
    }

    /// Zero-based protocol week a night falls in.
    pub fn week_of(&self, night: NaiveDate) -> i64 {
        (night - self.started).num_days().div_euclid(7)
    }

    pub fn session(&self, number: u8) -> Option<&SessionRecord> {
        self.sessions.iter().find(|s| s.number == number)
    }

    pub fn session_mut(&mut self, number: u8) -> &mut SessionRecord {
        if let Some(i) = self.sessions.iter().position(|s| s.number == number) {
            return &mut self.sessions[i];
        }
        self.sessions.push(SessionRecord {
            number,
            ..Default::default()
        });
        self.sessions.sort_by_key(|s| s.number);
        let i = self
            .sessions
            .iter()
            .position(|s| s.number == number)
            .unwrap();
        &mut self.sessions[i]
    }

    pub fn toggle_screening(&mut self, key: &str) {
        if let Some(i) = self.screening.iter().position(|k| k == key) {
            self.screening.remove(i);
        } else {
            self.screening.push(key.to_string());
        }
    }
}
