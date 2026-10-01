//! Application state and key handling. Every change is saved to disk at once.

use super::form::Form;
use cbti::clock::ClockTime;
use cbti::metrics::night;
use cbti::model::{Prescription, Subject};
use cbti::protocol::{SCREENING, SESSIONS};
use cbti::store;
use chrono::{Duration, NaiveDate};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Tonight,
    Diary,
    Weeks,
    Protocol,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Tonight, Tab::Diary, Tab::Weeks, Tab::Protocol];
    pub fn title(self) -> &'static str {
        match self {
            Tab::Tonight => "Log a night",
            Tab::Diary => "Diary",
            Tab::Weeks => "Weeks",
            Tab::Protocol => "Protocol",
        }
    }
    fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap()
    }
}

pub enum Pending {
    DeleteEntry(NaiveDate),
    ApplyRx(Prescription),
}

pub enum PromptKind {
    RiseTarget,
    Isi(u8),
}

pub enum Overlay {
    Confirm {
        message: String,
        action: Pending,
    },
    Prompt {
        label: String,
        buffer: String,
        kind: PromptKind,
    },
    Help,
}

pub enum Status {
    Ok(String),
    Err(String),
}

pub struct App {
    pub subject: Subject,
    pub path: PathBuf,
    pub today: NaiveDate,
    pub tab: Tab,
    pub form: Form,
    /// Index into `subject.entries`.
    pub diary_sel: usize,
    /// 0 is the screening list, 1..=6 the sessions.
    pub proto_sel: usize,
    pub proto_item: usize,
    pub proto_items_focused: bool,
    pub overlay: Option<Overlay>,
    pub status: Option<Status>,
    pub quit: bool,
}

impl App {
    pub fn new(subject: Subject, path: PathBuf, today: NaiveDate) -> Self {
        let diary_sel = subject.entries.len().saturating_sub(1);
        let mut app = Self {
            form: Form::blank(today - Duration::days(1)),
            subject,
            path,
            today,
            tab: Tab::Tonight,
            diary_sel,
            proto_sel: 1,
            proto_item: 0,
            proto_items_focused: false,
            overlay: None,
            status: None,
            quit: false,
        };
        app.form = Form::blank(app.next_night(None));
        app
    }

    /// Last night by default. After saving a night, step to the following one while
    /// back-filling, so a paper diary can be typed in order. Older gaps are not
    /// offered: diaries are meant to be filled the next morning.
    fn next_night(&self, after: Option<NaiveDate>) -> NaiveDate {
        let last_night = self.today - Duration::days(1);
        match after.map(|d| d + Duration::days(1)) {
            Some(n) if n < last_night && self.subject.entry(n).is_none() => n,
            _ => last_night,
        }
    }

    fn persist(&mut self, ok: String) {
        match store::save(&self.path, &self.subject) {
            Ok(()) => self.status = Some(Status::Ok(ok)),
            Err(e) => self.status = Some(Status::Err(format!("Not saved: {e:#}"))),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        self.status = None;
        if ctrl && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q')) {
            self.quit = true;
            return;
        }
        if self.overlay.is_some() {
            self.on_overlay_key(key);
            return;
        }
        match key.code {
            KeyCode::Tab => return self.switch_tab(1),
            KeyCode::BackTab => return self.switch_tab(Tab::ALL.len() - 1),
            KeyCode::F(1) => {
                self.overlay = Some(Overlay::Help);
                return;
            }
            _ => {}
        }
        if self.tab != Tab::Tonight {
            match key.code {
                KeyCode::Char('q') => {
                    self.quit = true;
                    return;
                }
                KeyCode::Char('?') => {
                    self.overlay = Some(Overlay::Help);
                    return;
                }
                KeyCode::Char(c @ '1'..='4') => {
                    self.tab = Tab::ALL[c as usize - '1' as usize];
                    return;
                }
                _ => {}
            }
        }
        match self.tab {
            Tab::Tonight => self.on_form_key(key, ctrl),
            Tab::Diary => self.on_diary_key(key),
            Tab::Weeks => self.on_weeks_key(key),
            Tab::Protocol => self.on_protocol_key(key),
        }
    }

    fn switch_tab(&mut self, step: usize) {
        self.tab = Tab::ALL[(self.tab.index() + step) % Tab::ALL.len()];
    }

    fn on_form_key(&mut self, key: KeyEvent, ctrl: bool) {
        match key.code {
            KeyCode::Char('s') if ctrl => self.save_form(),
            KeyCode::Char('n') if ctrl => {
                self.form = Form::blank(self.next_night(None));
                self.status = Some(Status::Ok("New blank night".into()));
            }
            KeyCode::Up => self.form.prev(),
            KeyCode::Down => self.form.next(),
            KeyCode::Enter => {
                if self.form.cursor == super::form::FIELDS.len() - 1 {
                    self.save_form();
                } else {
                    self.form.next();
                }
            }
            KeyCode::Backspace => self.form.backspace(),
            KeyCode::Char(c) if !ctrl => self.form.push(c),
            _ => {}
        }
    }

    fn save_form(&mut self) {
        let Some(entry) = self.form.submit() else {
            if let Some((_, msg)) = &self.form.error {
                self.status = Some(Status::Err(msg.clone()));
            }
            return;
        };
        let label = entry.night.format("%a %d %b").to_string();
        let se = night(&entry).se;
        let replaced = self.subject.upsert_entry(entry.clone());
        self.diary_sel = self
            .subject
            .entries
            .iter()
            .position(|e| e.night == entry.night)
            .unwrap_or(0);
        let verb = if replaced { "Updated" } else { "Saved" };
        self.persist(format!(
            "{verb} the night of {label}, sleep efficiency {se:.0}%"
        ));
        self.form = Form::blank(self.next_night(Some(entry.night)));
    }

    fn on_diary_key(&mut self, key: KeyEvent) {
        let n = self.subject.entries.len();
        if n == 0 {
            return;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.diary_sel = self.diary_sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.diary_sel = (self.diary_sel + 1).min(n - 1),
            KeyCode::PageUp => self.diary_sel = self.diary_sel.saturating_sub(7),
            KeyCode::PageDown => self.diary_sel = (self.diary_sel + 7).min(n - 1),
            KeyCode::Home | KeyCode::Char('g') => self.diary_sel = 0,
            KeyCode::End | KeyCode::Char('G') => self.diary_sel = n - 1,
            KeyCode::Char('e') | KeyCode::Enter => {
                self.form = Form::from_entry(&self.subject.entries[self.diary_sel]);
                self.tab = Tab::Tonight;
                self.status = Some(Status::Ok(
                    "Editing this night. Ctrl+S saves over it.".into(),
                ));
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                let night = self.subject.entries[self.diary_sel].night;
                self.overlay = Some(Overlay::Confirm {
                    message: format!(
                        "Delete the diary entry for the night of {}?",
                        night.format("%a %d %b")
                    ),
                    action: Pending::DeleteEntry(night),
                });
            }
            KeyCode::Char('x') => self.export_csv(),
            _ => {}
        }
    }

    fn export_csv(&mut self) {
        let path = self.path.with_extension("csv");
        match std::fs::write(&path, store::to_csv(&self.subject)) {
            Ok(()) => self.status = Some(Status::Ok(format!("Exported {}", path.display()))),
            Err(e) => self.status = Some(Status::Err(format!("Export failed: {e}"))),
        }
    }

    fn on_weeks_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('a') => {
                let rec = cbti::titration::recommend(&self.subject, self.today);
                match rec.prescription() {
                    Some(rx) => {
                        self.overlay = Some(Overlay::Confirm {
                            message: format!(
                                "Set the sleep window to {}–{} ({}) from tonight?",
                                rx.bed,
                                rx.rise,
                                cbti::clock::fmt_duration(rx.window_min() as f64)
                            ),
                            action: Pending::ApplyRx(rx.clone()),
                        })
                    }
                    None => {
                        self.status = Some(Status::Err(
                            "Nothing to apply yet. See the next step panel.".into(),
                        ))
                    }
                }
            }
            KeyCode::Char('r') => {
                self.overlay = Some(Overlay::Prompt {
                    label: "Fixed rise time, every day of the week".into(),
                    buffer: self
                        .subject
                        .rise_target
                        .map(|t| t.to_string())
                        .unwrap_or_default(),
                    kind: PromptKind::RiseTarget,
                })
            }
            KeyCode::Char('o') => {
                self.subject.older_adult = !self.subject.older_adult;
                let msg = if self.subject.older_adult {
                    "Using older-adult thresholds: extend at 85%, shorten below 80%"
                } else {
                    "Using standard thresholds: extend at 90%, shorten below 85%"
                };
                self.persist(msg.into());
            }
            KeyCode::Char('x') => self.export_csv(),
            _ => {}
        }
    }

    pub fn proto_item_count(&self) -> usize {
        if self.proto_sel == 0 {
            SCREENING.len()
        } else {
            SESSIONS[self.proto_sel - 1].steps.len()
        }
    }

    fn on_protocol_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.proto_items_focused = false,
            KeyCode::Right | KeyCode::Char('l') => self.proto_items_focused = true,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.proto_items_focused {
                    self.proto_item = self.proto_item.saturating_sub(1);
                } else {
                    self.proto_sel = self.proto_sel.saturating_sub(1);
                    self.proto_item = 0;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.proto_items_focused {
                    self.proto_item = (self.proto_item + 1).min(self.proto_item_count() - 1);
                } else {
                    self.proto_sel = (self.proto_sel + 1).min(SESSIONS.len());
                    self.proto_item = 0;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if !self.proto_items_focused {
                    self.proto_items_focused = true;
                    return;
                }
                self.toggle_proto_item();
            }
            KeyCode::Char('i') if self.proto_sel > 0 => {
                let number = self.proto_sel as u8;
                self.overlay = Some(Overlay::Prompt {
                    label: format!("ISI score for session {number}, 0 to 28"),
                    buffer: self
                        .subject
                        .session(number)
                        .and_then(|s| s.isi)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    kind: PromptKind::Isi(number),
                });
            }
            _ => {}
        }
    }

    fn toggle_proto_item(&mut self) {
        if self.proto_sel == 0 {
            let item = &SCREENING[self.proto_item];
            self.subject.toggle_screening(item.key);
            let present = self.subject.screening.iter().any(|k| k == item.key);
            let msg = if present {
                "Caution recorded"
            } else {
                "Caution cleared"
            };
            self.persist(msg.into());
            return;
        }
        let def = &SESSIONS[self.proto_sel - 1];
        let key = def.steps[self.proto_item].key.to_string();
        let today = self.today;
        let rec = self.subject.session_mut(def.number);
        if let Some(i) = rec.done.iter().position(|k| *k == key) {
            rec.done.remove(i);
        } else {
            rec.done.push(key);
            rec.date.get_or_insert(today);
        }
        let (done, total) = (rec.done.len(), def.steps.len());
        self.persist(format!(
            "Session {}: {done} of {total} steps done",
            def.number
        ));
    }

    fn on_overlay_key(&mut self, key: KeyEvent) {
        let Some(overlay) = self.overlay.take() else {
            return;
        };
        match overlay {
            Overlay::Help => {}
            Overlay::Confirm { message, action } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm(action),
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.status = Some(Status::Ok("Cancelled".into()))
                }
                _ => self.overlay = Some(Overlay::Confirm { message, action }),
            },
            Overlay::Prompt {
                label,
                mut buffer,
                kind,
            } => match key.code {
                KeyCode::Esc => self.status = Some(Status::Ok("Cancelled".into())),
                KeyCode::Enter => self.submit_prompt(label, buffer, kind),
                KeyCode::Backspace => {
                    buffer.pop();
                    self.overlay = Some(Overlay::Prompt {
                        label,
                        buffer,
                        kind,
                    });
                }
                KeyCode::Char(c) if c.is_ascii_digit() || c == ':' => {
                    if buffer.len() < 5 {
                        buffer.push(c);
                    }
                    self.overlay = Some(Overlay::Prompt {
                        label,
                        buffer,
                        kind,
                    });
                }
                _ => {
                    self.overlay = Some(Overlay::Prompt {
                        label,
                        buffer,
                        kind,
                    })
                }
            },
        }
    }

    fn submit_prompt(&mut self, label: String, buffer: String, kind: PromptKind) {
        match kind {
            PromptKind::RiseTarget => match ClockTime::parse(&buffer) {
                Some(t) => {
                    self.subject.rise_target = Some(t);
                    self.persist(format!("Fixed rise time set to {t}"));
                }
                None => {
                    self.status = Some(Status::Err("Enter a time like 06:30".into()));
                    self.overlay = Some(Overlay::Prompt {
                        label,
                        buffer,
                        kind,
                    });
                }
            },
            PromptKind::Isi(number) => match buffer.parse::<u8>() {
                Ok(v) if v <= 28 => {
                    let today = self.today;
                    let rec = self.subject.session_mut(number);
                    rec.isi = Some(v);
                    rec.date.get_or_insert(today);
                    self.persist(format!("ISI {v} recorded for session {number}"));
                }
                _ if buffer.is_empty() => {
                    self.subject.session_mut(number).isi = None;
                    self.persist(format!("ISI cleared for session {number}"));
                }
                _ => {
                    self.status = Some(Status::Err("ISI is a whole number from 0 to 28".into()));
                    self.overlay = Some(Overlay::Prompt {
                        label,
                        buffer,
                        kind,
                    });
                }
            },
        }
    }

    fn confirm(&mut self, action: Pending) {
        match action {
            Pending::DeleteEntry(night) => {
                self.subject.remove_entry(night);
                self.diary_sel = self
                    .diary_sel
                    .min(self.subject.entries.len().saturating_sub(1));
                self.persist(format!("Deleted the night of {}", night.format("%a %d %b")));
            }
            Pending::ApplyRx(rx) => {
                let msg = format!("Sleep window is now {}–{} from tonight", rx.bed, rx.rise);
                self.subject.add_prescription(rx);
                self.persist(msg);
            }
        }
    }
}
