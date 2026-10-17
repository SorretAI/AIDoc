//! Terminal front end (ratatui).

mod actogram;
mod app;
mod form;
mod theme;
mod ui;

use anyhow::Result;
use cbti::model::Subject;
use chrono::NaiveDate;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::path::PathBuf;

pub fn run(subject: Subject, path: PathBuf, today: NaiveDate) -> Result<()> {
    let mut app = app::App::new(subject, path, today);
    let mut terminal = ratatui::init();
    let result = (|| -> Result<()> {
        while !app.quit {
            terminal.draw(|f| ui::draw(f, &app))?;
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                app.on_key(key);
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}

/// Renders views off-screen to coloured HTML, for docs and visual review.
/// `keys` is a list of key presses replayed first (e.g. Tab, Down, chars).
pub fn snapshot_html(
    subject: Subject,
    path: PathBuf,
    today: NaiveDate,
    keys: &[ratatui::crossterm::event::KeyEvent],
    width: u16,
    height: u16,
) -> Result<String> {
    use ratatui::backend::TestBackend;
    use ratatui::style::{Color, Modifier};
    let mut app = app::App::new(subject, path, today);
    for k in keys {
        app.on_key(*k);
    }
    let mut terminal = ratatui::Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|f| ui::draw(f, &app))?;
    let buf = terminal.backend().buffer();
    let hex = |c: Color, default: &str| match c {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => default.to_string(),
    };
    let mut html = String::from(
        "<meta charset=\"utf-8\"><pre style=\"background:#14161f;color:#d8dce8;font:14px/1.25 'JetBrains Mono',monospace;padding:12px;display:inline-block\">",
    );
    for y in 0..height {
        for x in 0..width {
            let cell = &buf[(x, y)];
            let (mut fg, mut bg) = (hex(cell.fg, "#d8dce8"), hex(cell.bg, "#14161f"));
            if cell.modifier.contains(Modifier::REVERSED) {
                std::mem::swap(&mut fg, &mut bg);
            }
            let bold = if cell.modifier.contains(Modifier::BOLD) {
                "font-weight:bold;"
            } else {
                ""
            };
            let ul = if cell.modifier.contains(Modifier::UNDERLINED) {
                "text-decoration:underline;"
            } else {
                ""
            };
            let sym = match cell.symbol() {
                "<" => "&lt;",
                ">" => "&gt;",
                "&" => "&amp;",
                s => s,
            };
            html.push_str(&format!("<span style=\"display:inline-block;width:1ch;overflow:hidden;color:{fg};background:{bg};{bold}{ul}\">{sym}</span>"));
        }
        html.push('\n');
    }
    html.push_str("</pre>");
    Ok(html)
}
