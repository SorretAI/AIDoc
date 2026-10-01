//! Night bars on an 18:00 to 12:00 axis.
//!
//! `█` asleep, `░` awake in bed, blank out of bed. In-bed time outside the
//! prescribed window turns rose. Night awakenings are counted in the diary,
//! not timed, so they are not placed on the bar.

use super::theme;
use cbti::clock::ClockTime;
use cbti::model::{DiaryEntry, Prescription};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

const AXIS_START: i32 = 360; // 18:00 as minutes after noon
const AXIS_LEN: i32 = 1080; // to 12:00 next day
const TICKS: [i32; 6] = [360, 540, 720, 900, 1080, 1260];

fn col_minute(c: usize, w: usize) -> i32 {
    AXIS_START + ((2 * c + 1) as i32 * AXIS_LEN) / (2 * w as i32)
}

fn tick_col(t: i32, w: usize) -> usize {
    (((t - AXIS_START) as i64 * w as i64) / AXIS_LEN as i64) as usize
}

fn in_window(m: i32, rx: Option<&Prescription>) -> bool {
    rx.is_none_or(|r| r.bed.offset() <= m && m < r.rise.offset())
}

/// Merge per-cell glyphs into as few spans as possible.
fn spans(cells: Vec<(char, Style)>) -> Vec<Span<'static>> {
    let mut out: Vec<Span> = Vec::new();
    let mut buf = String::new();
    let mut cur: Option<Style> = None;
    for (ch, st) in cells {
        if cur != Some(st) {
            if let Some(s) = cur {
                out.push(Span::styled(std::mem::take(&mut buf), s));
            }
            cur = Some(st);
        }
        buf.push(ch);
    }
    if let Some(s) = cur {
        out.push(Span::styled(buf, s));
    }
    out
}

pub fn bar(e: &DiaryEntry, rx: Option<&Prescription>, width: usize) -> Vec<Span<'static>> {
    let (bed, onset, fw, oob) = (
        e.bedtime.offset(),
        e.sleep_onset().offset(),
        e.final_wake.offset(),
        e.out_of_bed.offset(),
    );
    let ticks: Vec<usize> = TICKS.iter().map(|&t| tick_col(t, width)).collect();
    let cells = (0..width)
        .map(|c| {
            let m = col_minute(c, width);
            if bed <= m && m < oob {
                let asleep = onset <= m && m < fw;
                let style = if !in_window(m, rx) {
                    theme::off()
                } else if asleep {
                    theme::sleep()
                } else {
                    theme::lamp()
                };
                (if asleep { '█' } else { '░' }, style)
            } else if ticks.contains(&c) {
                ('·', theme::dim())
            } else {
                (' ', theme::dim())
            }
        })
        .collect();
    spans(cells)
}

/// Hour labels: 18 21 00 03 06 09.
pub fn ruler(width: usize) -> Line<'static> {
    let mut chars = vec![' '; width];
    for t in TICKS {
        let c = tick_col(t, width);
        let label = format!("{:02}", ClockTime::from_offset(t).minutes() / 60);
        for (i, ch) in label.chars().enumerate() {
            if c + i < width {
                chars[c + i] = ch;
            }
        }
    }
    Line::styled(chars.into_iter().collect::<String>(), theme::dim())
}

/// The prescribed window as a heavy rule, the rest of the axis faint.
pub fn window(rx: Option<&Prescription>, width: usize) -> Vec<Span<'static>> {
    let cells = (0..width)
        .map(|c| match rx {
            Some(_) if in_window(col_minute(c, width), rx) => ('━', theme::sleep()),
            _ => ('┈', theme::dim()),
        })
        .collect();
    spans(cells)
}
