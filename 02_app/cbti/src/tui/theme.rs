//! Night palette. Colour carries meaning only: indigo is sleep, lamp amber is
//! awake in bed, rose is off-schedule or a caution. Body text uses the
//! terminal's own foreground so light and dark terminals both read.

use ratatui::style::{Color, Modifier, Style};

pub const SLEEP: Color = Color::Rgb(124, 134, 240);
pub const LAMP: Color = Color::Rgb(232, 164, 88);
pub const OFF: Color = Color::Rgb(226, 112, 132);
pub const GOOD: Color = Color::Rgb(122, 192, 144);
pub const DIM: Color = Color::Rgb(112, 120, 146);

pub fn text() -> Style {
    Style::default()
}
pub fn dim() -> Style {
    Style::default().fg(DIM)
}
pub fn strong() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}
pub fn sleep() -> Style {
    Style::default().fg(SLEEP)
}
pub fn lamp() -> Style {
    Style::default().fg(LAMP)
}
pub fn off() -> Style {
    Style::default().fg(OFF)
}
pub fn good() -> Style {
    Style::default().fg(GOOD)
}
pub fn border(focused: bool) -> Style {
    if focused { Style::default() } else { dim() }
}
pub fn title(focused: bool) -> Style {
    if focused { strong() } else { dim() }
}
pub fn selected() -> Style {
    Style::default()
        .fg(SLEEP)
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}
pub fn cursor_row() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

/// Sleep efficiency colour against the titration thresholds.
pub fn se(se: f64, extend_at: f64, reduce_below: f64) -> Style {
    if se >= extend_at {
        good()
    } else if se < reduce_below {
        off()
    } else {
        lamp()
    }
}
