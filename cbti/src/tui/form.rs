//! Morning diary entry form. Wording follows the Consensus Sleep Diary.

use cbti::clock::ClockTime;
use cbti::model::DiaryEntry;
use chrono::NaiveDate;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Date,
    Time,
    Minutes,
    Count,
    Quality,
    Text,
}

pub struct FieldDef {
    pub label: &'static str,
    pub hint: &'static str,
    pub kind: Kind,
    pub required: bool,
}

const fn f(label: &'static str, hint: &'static str, kind: Kind, required: bool) -> FieldDef {
    FieldDef {
        label,
        hint,
        kind,
        required,
    }
}

pub const NIGHT: usize = 0;
pub const FIELDS: &[FieldDef] = &[
    f("Night of", "date the evening began", Kind::Date, true),
    f("Got into bed", "e.g. 23:15", Kind::Time, true),
    f("Tried to sleep", "lights out", Kind::Time, true),
    f(
        "Minutes to fall asleep",
        "a best guess is fine",
        Kind::Minutes,
        true,
    ),
    f("Times woke up", "not counting the last", Kind::Count, false),
    f(
        "Minutes awake in the night",
        "all awakenings together",
        Kind::Minutes,
        false,
    ),
    f("Final awakening", "e.g. 05:40", Kind::Time, true),
    f("Got out of bed", "for the day", Kind::Time, true),
    f(
        "Sleep quality",
        "1 very poor to 5 very good",
        Kind::Quality,
        true,
    ),
    f("Naps that day (minutes)", "0 if none", Kind::Minutes, false),
    f(
        "Caffeine drinks",
        "after noon counts most",
        Kind::Count,
        false,
    ),
    f("Alcohol drinks", "", Kind::Count, false),
    f(
        "Sleep medication",
        "name and dose, blank if none",
        Kind::Text,
        false,
    ),
    f("Notes", "anything unusual", Kind::Text, false),
];

pub struct Form {
    pub values: Vec<String>,
    pub cursor: usize,
    pub error: Option<(usize, String)>,
}

impl Form {
    pub fn blank(night: NaiveDate) -> Self {
        let mut values = vec![String::new(); FIELDS.len()];
        values[NIGHT] = night.to_string();
        Self {
            values,
            cursor: 1,
            error: None,
        }
    }

    pub fn from_entry(e: &DiaryEntry) -> Self {
        let values = vec![
            e.night.to_string(),
            e.bedtime.to_string(),
            e.lights_out.to_string(),
            e.sol_min.to_string(),
            e.awakenings.to_string(),
            e.waso_min.to_string(),
            e.final_wake.to_string(),
            e.out_of_bed.to_string(),
            e.quality.to_string(),
            e.nap_min.to_string(),
            e.caffeine.to_string(),
            e.alcohol.to_string(),
            e.sleep_aid.clone(),
            e.notes.clone(),
        ];
        Self {
            values,
            cursor: 1,
            error: None,
        }
    }

    pub fn night(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(self.values[NIGHT].trim(), "%Y-%m-%d").ok()
    }

    pub fn accepts(&self, c: char) -> bool {
        match FIELDS[self.cursor].kind {
            Kind::Date => c.is_ascii_digit() || c == '-',
            Kind::Time => c.is_ascii_digit() || matches!(c, ':' | '.' | 'h'),
            Kind::Minutes | Kind::Count | Kind::Quality => c.is_ascii_digit(),
            Kind::Text => !c.is_control(),
        }
    }

    pub fn push(&mut self, c: char) {
        if self.accepts(c) && self.values[self.cursor].chars().count() < 120 {
            self.values[self.cursor].push(c);
            self.error = None;
        }
    }

    pub fn backspace(&mut self) {
        self.values[self.cursor].pop();
        self.error = None;
    }

    pub fn next(&mut self) {
        self.cursor = (self.cursor + 1).min(FIELDS.len() - 1);
        self.normalize_previous();
    }

    pub fn prev(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Tidy a time typed as `2330` into `23:30` after leaving the field.
    fn normalize_previous(&mut self) {
        let i = self.cursor.saturating_sub(1);
        if FIELDS[i].kind == Kind::Time
            && let Some(t) = ClockTime::parse(&self.values[i])
        {
            self.values[i] = t.to_string();
        }
    }

    fn time(&self, i: usize) -> Result<ClockTime, (usize, String)> {
        ClockTime::parse(&self.values[i])
            .ok_or_else(|| (i, format!("{}: enter a time like 23:15", FIELDS[i].label)))
    }

    fn num<T: std::str::FromStr + Default>(&self, i: usize) -> Result<T, (usize, String)> {
        let v = self.values[i].trim();
        if v.is_empty() {
            if FIELDS[i].required {
                return Err((i, format!("{}: required", FIELDS[i].label)));
            }
            return Ok(T::default());
        }
        v.parse()
            .map_err(|_| (i, format!("{}: enter a whole number", FIELDS[i].label)))
    }

    /// Builds the entry, without the cross-field consistency checks.
    pub fn build(&self) -> Result<DiaryEntry, (usize, String)> {
        let night = self
            .night()
            .ok_or((NIGHT, "Night of: enter a date like 2026-09-30".to_string()))?;
        Ok(DiaryEntry {
            night,
            bedtime: self.time(1)?,
            lights_out: self.time(2)?,
            sol_min: self.num(3)?,
            awakenings: self.num(4)?,
            waso_min: self.num(5)?,
            final_wake: self.time(6)?,
            out_of_bed: self.time(7)?,
            quality: self.num(8)?,
            nap_min: self.num(9)?,
            caffeine: self.num(10)?,
            alcohol: self.num(11)?,
            sleep_aid: self.values[12].trim().to_string(),
            notes: self.values[13].trim().to_string(),
        })
    }

    /// Builds and validates; on failure records which field to fix.
    pub fn submit(&mut self) -> Option<DiaryEntry> {
        let result = self
            .build()
            .and_then(|e| e.validate().map(|_| e).map_err(|m| (1, m)));
        match result {
            Ok(e) => Some(e),
            Err((i, msg)) => {
                self.cursor = i;
                self.error = Some((i, msg));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_an_entry() {
        let mut form = Form::blank(NaiveDate::from_ymd_opt(2026, 9, 29).unwrap());
        for (i, v) in ["2330", "23:45", "20", "2", "30", "0610", "06:30", "3"]
            .iter()
            .enumerate()
        {
            form.values[i + 1] = v.to_string();
        }
        let e = form.submit().expect("valid");
        assert_eq!(e.bedtime.to_string(), "23:30");
        assert_eq!(Form::from_entry(&e).build().unwrap(), e);
    }

    #[test]
    fn points_at_the_missing_field() {
        let mut form = Form::blank(NaiveDate::from_ymd_opt(2026, 9, 29).unwrap());
        form.values[1] = "23:30".into();
        assert!(form.submit().is_none());
        assert_eq!(form.cursor, 2);
    }
}
