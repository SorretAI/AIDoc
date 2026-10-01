//! Rendering. Reads `App`, never mutates it.

use super::actogram;
use super::app::{App, Overlay, Status, Tab};
use super::form::FIELDS;
use super::theme;
use cbti::clock::fmt_duration;
use cbti::metrics::{deviations, night, summarize};
use cbti::model::{DiaryEntry, Prescription, Subject};
use cbti::protocol::{AASM_NOTES, SCREENING, SESSIONS, isi_band, isi_change, screening_text};
use cbti::titration::{Recommendation, Thresholds, recommend};
use chrono::{Duration, NaiveDate};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, BorderType, Cell, Chart, Clear, Dataset, GraphType, Paragraph, Row, Table, Tabs,
    Wrap,
};

pub fn draw(f: &mut Frame, app: &App) {
    let [header, tabs, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(f.area());
    draw_header(f, header, app);
    draw_tabs(f, tabs, app);
    match app.tab {
        Tab::Tonight => draw_tonight(f, body, app),
        Tab::Diary => draw_diary(f, body, app),
        Tab::Weeks => draw_weeks(f, body, app),
        Tab::Protocol => draw_protocol(f, body, app),
    }
    draw_footer(f, footer, app);
    if let Some(o) = &app.overlay {
        draw_overlay(f, o);
    }
}

fn panel(title: &str, focused: bool) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border(focused))
        .title(Span::styled(format!(" {title} "), theme::title(focused)))
}

fn day(d: NaiveDate) -> String {
    d.format("%a %d %b").to_string()
}

fn window_text(rx: &Prescription) -> String {
    format!(
        "{}–{} ({})",
        rx.bed,
        rx.rise,
        fmt_duration(rx.window_min() as f64)
    )
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.subject;
    let mut spans = vec![
        Span::styled(" CBT-I sleep diary", theme::strong()),
        Span::raw("    "),
        Span::raw(s.id.clone()),
        Span::styled(
            format!("    week {}", s.week_of(app.today) + 1),
            theme::dim(),
        ),
        Span::raw("    "),
    ];
    match s.prescription_for(app.today) {
        Some(rx) => {
            spans.push(Span::styled("window ", theme::dim()));
            spans.push(Span::styled(window_text(rx), theme::sleep()));
        }
        None => spans.push(Span::styled("no sleep window yet", theme::dim())),
    }
    if let Some(first) = s.screening.first() {
        let more = if s.screening.len() > 1 {
            format!(" and {} more", s.screening.len() - 1)
        } else {
            String::new()
        };
        spans.push(Span::styled(
            format!("    caution: {}{more}", screening_text(first)),
            theme::off(),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
    f.render_widget(
        Paragraph::new(Span::styled(format!("{} ", day(app.today)), theme::dim()))
            .alignment(Alignment::Right),
        area,
    );
}

fn draw_tabs(f: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(t.title())).collect();
    let idx = Tab::ALL.iter().position(|t| *t == app.tab).unwrap();
    let tabs = Tabs::new(titles)
        .select(idx)
        .style(theme::dim())
        .highlight_style(theme::sleep().add_modifier(Modifier::BOLD | Modifier::UNDERLINED))
        .divider("   ")
        .padding(" ", "");
    let [row, _] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    f.render_widget(tabs, row);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let line = match &app.status {
        Some(Status::Ok(m)) => Line::styled(format!(" {m}"), theme::good()),
        Some(Status::Err(m)) => Line::styled(format!(" {m}"), theme::off()),
        None => {
            let hints: &[(&str, &str)] = match app.tab {
                Tab::Tonight => &[
                    ("↑↓", "field"),
                    ("Enter", "next"),
                    ("Ctrl+S", "save"),
                    ("Ctrl+N", "blank form"),
                    ("Tab", "next view"),
                    ("F1", "help"),
                    ("Ctrl+Q", "quit"),
                ],
                Tab::Diary => &[
                    ("↑↓", "night"),
                    ("e", "edit"),
                    ("d", "delete"),
                    ("x", "export CSV"),
                    ("Tab", "next view"),
                    ("?", "help"),
                    ("q", "quit"),
                ],
                Tab::Weeks => &[
                    ("a", "apply next step"),
                    ("r", "rise time"),
                    ("o", "thresholds"),
                    ("x", "export CSV"),
                    ("Tab", "next view"),
                    ("q", "quit"),
                ],
                Tab::Protocol => &[
                    ("←→", "pane"),
                    ("↑↓", "move"),
                    ("Space", "tick"),
                    ("i", "ISI score"),
                    ("Tab", "next view"),
                    ("q", "quit"),
                ],
            };
            let mut spans = vec![Span::raw(" ")];
            for (k, v) in hints {
                spans.push(Span::styled(*k, theme::strong()));
                spans.push(Span::styled(format!(" {v}   "), theme::dim()));
            }
            Line::from(spans)
        }
    };
    f.render_widget(Paragraph::new(line), area);
}

// ---------------------------------------------------------------- Log a night

fn draw_tonight(f: &mut Frame, area: Rect, app: &App) {
    let [left, right] =
        Layout::horizontal([Constraint::Length(66), Constraint::Min(30)]).areas(area);
    let form = &app.form;

    let mut lines = Vec::new();
    let existing = form.night().and_then(|n| app.subject.entry(n));
    lines.push(match (form.night(), existing) {
        (Some(_), Some(_)) => Line::styled(
            " This night is already logged. Saving replaces it.",
            theme::lamp(),
        ),
        (Some(n), None) => Line::styled(
            format!(" Answer for the night that began {}.", day(n)),
            theme::dim(),
        ),
        (None, _) => Line::styled(" Enter the night as YYYY-MM-DD.", theme::off()),
    });
    lines.push(Line::raw(""));
    for (i, def) in FIELDS.iter().enumerate() {
        let here = i == form.cursor;
        let value = &form.values[i];
        let marker = if here {
            Span::styled(" › ", theme::sleep())
        } else {
            Span::raw("   ")
        };
        let label_style = if here {
            theme::cursor_row()
        } else {
            theme::text()
        };
        let mut spans = vec![
            marker,
            Span::styled(format!("{:<29}", def.label), label_style),
        ];
        if value.is_empty() && !here {
            spans.push(Span::styled(def.hint, theme::dim()));
        } else {
            spans.push(Span::raw(value.clone()));
        }
        if here {
            spans.push(Span::styled("▏", theme::sleep()));
            if value.is_empty() {
                spans.push(Span::styled(def.hint, theme::dim()));
            }
        }
        lines.push(Line::from(spans));
        if let Some((ei, msg)) = &form.error
            && *ei == i
        {
            lines.push(Line::styled(format!("   {msg}"), theme::off()));
        }
    }
    let title = if existing.is_some() {
        "Edit a night"
    } else {
        "Morning diary"
    };
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(title, true)),
        left,
    );

    draw_preview(f, right, app);
}

fn draw_preview(f: &mut Frame, area: Rect, app: &App) {
    let block = panel("This night", false);
    let inner = block.inner(area);
    f.render_widget(block, area);
    const PREFIX: usize = 8;
    let w = (inner.width as usize).saturating_sub(PREFIX + 6);
    let th = Thresholds::for_subject(&app.subject);
    let night_of = app.form.night().unwrap_or(app.today - Duration::days(1));
    let rx = app.subject.prescription_for(night_of);
    let row = |label: Span<'static>, bar: Vec<Span<'static>>, tail: Span<'static>| {
        Line::from([vec![label], bar, vec![tail]].concat())
    };
    let se_tail = |se: f64| {
        Span::styled(
            format!(" {:>4.0}%", se),
            theme::se(se, th.extend_at, th.reduce_below),
        )
    };

    // The week leading up to this night, then this night, on one axis.
    let mut lines = vec![
        Line::from(vec![
            Span::raw(" ".repeat(PREFIX)),
            actogram::ruler(w).spans.remove(0),
        ]),
        row(
            Span::styled(format!("{:<PREFIX$}", " window"), theme::dim()),
            actogram::window(rx, w),
            Span::raw(""),
        ),
    ];
    for e in app
        .subject
        .entries
        .iter()
        .filter(|e| e.night < night_of && e.night >= night_of - Duration::days(7))
    {
        lines.push(row(
            Span::styled(format!(" {} ", e.night.format("%a %d")), theme::dim()),
            actogram::bar(e, app.subject.prescription_for(e.night), w),
            se_tail(night(e).se),
        ));
    }
    let label = Span::styled(format!("›{} ", night_of.format("%a %d")), theme::selected());
    let built = app.form.build();
    match &built {
        Ok(e) if e.validate().is_ok() => {
            lines.push(row(label, actogram::bar(e, rx, w), se_tail(night(e).se)))
        }
        _ => lines.push(row(
            label,
            vec![Span::styled(" waiting for the times", theme::dim())],
            Span::raw(""),
        )),
    }
    lines.push(Line::raw(""));

    match built {
        Err(_) => lines.push(Line::styled(
            " Fill in the times and the night is drawn on the last row.",
            theme::dim(),
        )),
        Ok(e) => match e.validate() {
            Err(msg) => lines.push(Line::styled(format!(" {msg}"), theme::off())),
            Ok(()) => {
                let m = night(&e);
                lines.push(Line::from(vec![
                    Span::styled(" In bed ", theme::dim()),
                    Span::raw(fmt_duration(m.tib as f64)),
                    Span::styled("    asleep ", theme::dim()),
                    Span::raw(fmt_duration(m.tst as f64)),
                    Span::styled("    efficiency ", theme::dim()),
                    Span::styled(
                        format!("{:.0}%", m.se),
                        theme::se(m.se, th.extend_at, th.reduce_below),
                    ),
                ]));
                lines.push(Line::styled(
                    format!(
                        " Fell asleep about {}, awake {} min before getting up",
                        e.sleep_onset(),
                        m.ema
                    ),
                    theme::dim(),
                ));
                if let Some(rx) = rx {
                    let devs = deviations(&e, rx);
                    if devs.is_empty() {
                        lines.push(Line::styled(" Inside the sleep window", theme::good()));
                    } else {
                        let d: Vec<String> = devs.iter().map(|d| d.describe()).collect();
                        lines.push(Line::styled(
                            format!(" Off schedule: {}", d.join(", ")),
                            theme::off(),
                        ));
                    }
                }
            }
        },
    }
    match rx {
        Some(rx) => lines.push(Line::from(vec![
            Span::styled(" Prescribed window ", theme::dim()),
            Span::styled(window_text(rx), theme::sleep()),
        ])),
        None => lines.push(Line::styled(
            " Baseline: no sleep window yet, so no schedule to keep.",
            theme::dim(),
        )),
    }
    lines.push(Line::raw(""));
    lines.push(legend());
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn legend() -> Line<'static> {
    Line::from(vec![
        Span::styled(" █", theme::sleep()),
        Span::styled(" asleep   ", theme::dim()),
        Span::styled("░", theme::lamp()),
        Span::styled(" awake in bed   ", theme::dim()),
        Span::styled("█░", theme::off()),
        Span::styled(" in bed outside the window   ", theme::dim()),
        Span::styled("━", theme::sleep()),
        Span::styled(" window", theme::dim()),
    ])
}

// ---------------------------------------------------------------- Diary

enum DiaryRow<'a> {
    Week(i64, Option<&'a Prescription>),
    Night(usize),
    Missing(NaiveDate),
}

fn diary_rows(s: &Subject, today: NaiveDate) -> Vec<DiaryRow<'_>> {
    let last = s.entries.last().map_or(today - Duration::days(1), |e| {
        e.night.max(today - Duration::days(1))
    });
    let first = s
        .entries
        .first()
        .map_or(s.started, |e| e.night.min(s.started));
    let mut rows = Vec::new();
    let mut d = first;
    let mut week = i64::MIN;
    while d <= last {
        let w = s.week_of(d);
        if w != week {
            week = w;
            rows.push(DiaryRow::Week(
                w,
                s.prescription_for(d + Duration::days(6).min(last - d)),
            ));
        }
        match s.entries.iter().position(|e| e.night == d) {
            Some(i) => rows.push(DiaryRow::Night(i)),
            None => rows.push(DiaryRow::Missing(d)),
        }
        d += Duration::days(1);
    }
    rows
}

fn draw_diary(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.subject;
    if s.entries.is_empty() {
        let p = Paragraph::new(vec![
            Line::raw(""),
            Line::raw(" No nights logged yet."),
            Line::styled(
                " Press Tab to go back to Log a night and enter the first morning's answers.",
                theme::dim(),
            ),
        ])
        .block(panel("Nights", true));
        f.render_widget(p, area);
        return;
    }
    let [list, detail] = Layout::vertical([Constraint::Min(6), Constraint::Length(9)]).areas(area);
    let block = panel("Nights", true);
    let inner = block.inner(list);
    f.render_widget(block, list);

    const PREFIX: usize = 12;
    const SUFFIX: usize = 13;
    let bw = (inner.width as usize).saturating_sub(PREFIX + SUFFIX);
    let th = Thresholds::for_subject(s);

    let [ruler_area, rows_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" ".repeat(PREFIX)),
            actogram::ruler(bw).spans.remove(0),
        ])),
        ruler_area,
    );

    let rows = diary_rows(s, app.today);
    let sel_row = rows
        .iter()
        .position(|r| matches!(r, DiaryRow::Night(i) if *i == app.diary_sel))
        .unwrap_or(0);
    let h = rows_area.height as usize;
    let offset = if sel_row < h { 0 } else { sel_row + 1 - h };

    let lines: Vec<Line> = rows
        .iter()
        .skip(offset)
        .take(h)
        .map(|r| match r {
            DiaryRow::Week(w, rx) => {
                let mut spans = vec![Span::styled(
                    format!(" {:<width$}", format!("Week {}", w + 1), width = PREFIX - 1),
                    theme::strong(),
                )];
                spans.extend(actogram::window(*rx, bw));
                let label = rx.map_or("no window".to_string(), |r| {
                    fmt_duration(r.window_min() as f64)
                });
                spans.push(Span::styled(
                    format!(" {label:>width$}", width = SUFFIX - 1),
                    theme::sleep(),
                ));
                Line::from(spans)
            }
            DiaryRow::Missing(d) => Line::from(vec![
                Span::styled(
                    format!(" {:<width$}", day(*d), width = PREFIX - 1),
                    theme::dim(),
                ),
                Span::styled("no entry", theme::dim().add_modifier(Modifier::ITALIC)),
            ]),
            DiaryRow::Night(i) => {
                let e = &s.entries[*i];
                let m = night(e);
                let selected = *i == app.diary_sel;
                let date_style = if selected {
                    theme::selected()
                } else {
                    theme::text()
                };
                let mut spans = vec![
                    Span::raw(if selected { "›" } else { " " }),
                    Span::styled(day(e.night), date_style),
                    Span::raw(" "),
                ];
                spans.extend(actogram::bar(e, s.prescription_for(e.night), bw));
                spans.push(Span::raw(format!(" {:>5}", fmt_duration(m.tst as f64))));
                spans.push(Span::styled(
                    format!(" {:>4.0}%", m.se),
                    theme::se(m.se, th.extend_at, th.reduce_below),
                ));
                Line::from(spans)
            }
        })
        .collect();
    f.render_widget(Paragraph::new(lines), rows_area);

    draw_night_detail(f, detail, app);
}

fn draw_night_detail(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.subject;
    let Some(e) = s.entries.get(app.diary_sel) else {
        return;
    };
    let m = night(e);
    let th = Thresholds::for_subject(s);
    let rx = s.prescription_for(e.night);
    let kv = |k: &str, v: String| {
        vec![
            Span::styled(format!("{k} "), theme::dim()),
            Span::raw(v),
            Span::raw("    "),
        ]
    };

    let mut l1 = kv("In bed", format!("{}–{}", e.bedtime, e.out_of_bed));
    l1.extend(kv("tried to sleep", e.lights_out.to_string()));
    l1.extend(kv("to fall asleep", format!("{} min", e.sol_min)));
    l1.extend(kv(
        "woke",
        format!("{} times, {} min", e.awakenings, e.waso_min),
    ));
    l1.extend(kv("final wake", e.final_wake.to_string()));

    let mut l2 = kv("Asleep", fmt_duration(m.tst as f64));
    l2.extend(kv("in bed", fmt_duration(m.tib as f64)));
    l2.push(Span::styled("efficiency ", theme::dim()));
    l2.push(Span::styled(
        format!("{:.0}%    ", m.se),
        theme::se(m.se, th.extend_at, th.reduce_below),
    ));
    l2.extend(kv("quality", format!("{}/5", e.quality)));

    let mut l3 = kv("Naps", format!("{} min", e.nap_min));
    l3.extend(kv("caffeine", e.caffeine.to_string()));
    l3.extend(kv("alcohol", e.alcohol.to_string()));
    l3.extend(kv(
        "medication",
        if e.sleep_aid.is_empty() {
            "none".into()
        } else {
            e.sleep_aid.clone()
        },
    ));

    let l4 = match rx {
        None => Line::styled("Before the first sleep window (baseline)", theme::dim()),
        Some(rx) => {
            let devs = deviations(e, rx);
            if devs.is_empty() {
                Line::styled(
                    format!("On schedule for the window {}", window_text(rx)),
                    theme::good(),
                )
            } else {
                let d: Vec<String> = devs.iter().map(|d| d.describe()).collect();
                Line::styled(
                    format!(
                        "Off schedule: {} (window {})",
                        d.join(", "),
                        window_text(rx)
                    ),
                    theme::off(),
                )
            }
        }
    };
    let mut lines = vec![Line::from(l1), Line::from(l2), Line::from(l3), l4];
    if !e.notes.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Notes ", theme::dim()),
            Span::raw(e.notes.clone()),
        ]));
    }
    let title = format!("Night of {}, week {}", day(e.night), s.week_of(e.night) + 1);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(&title, false).padding(ratatui::widgets::Padding::horizontal(1))),
        area,
    );
}

// ---------------------------------------------------------------- Weeks

fn draw_weeks(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.subject;
    let th = Thresholds::for_subject(s);
    let last_night = app.today - Duration::days(1);
    let weeks = if last_night < s.started {
        0
    } else {
        s.week_of(last_night) + 1
    };
    let table_h = (weeks as u16 + 3).min(area.height / 2).max(4);
    let [top, bottom] =
        Layout::vertical([Constraint::Length(table_h), Constraint::Min(8)]).areas(area);
    let [chart_area, next_area] =
        Layout::horizontal([Constraint::Min(30), Constraint::Length(54)]).areas(bottom);

    let header = Row::new([
        "Week",
        "Dates",
        "Nights",
        "In bed",
        "Asleep",
        "To sleep",
        "Awake",
        "Efficiency",
        "Quality",
        "Off schedule",
        "Window",
    ])
    .style(theme::dim());
    let rows: Vec<Row> = (0..weeks)
        .map(|w| {
            let from = s.started + Duration::days(w * 7);
            let to = from + Duration::days(6);
            let nights: Vec<&DiaryEntry> = s
                .entries
                .iter()
                .filter(|e| s.week_of(e.night) == w)
                .collect();
            let rx = s.prescription_for(to.min(last_night));
            let off = nights
                .iter()
                .filter(|e| {
                    s.prescription_for(e.night)
                        .is_some_and(|r| !deviations(e, r).is_empty())
                })
                .count();
            let dates = format!("{}–{}", from.format("%d %b"), to.format("%d %b"));
            let window = rx.map_or("baseline".to_string(), |r| format!("{}–{}", r.bed, r.rise));
            match summarize(nights.iter().copied()) {
                None => Row::new(vec![
                    Cell::from(format!("{}", w + 1)),
                    Cell::from(dates),
                    Cell::from("0"),
                ])
                .style(theme::dim()),
                Some(m) => Row::new(vec![
                    Cell::from(format!("{}", w + 1)),
                    Cell::from(dates),
                    Cell::from(format!(
                        "{}{}",
                        m.nights,
                        if to >= app.today { " so far" } else { "" }
                    )),
                    Cell::from(fmt_duration(m.tib)),
                    Cell::from(fmt_duration(m.tst)),
                    Cell::from(format!("{:.0} min", m.sol)),
                    Cell::from(format!("{:.0} min", m.waso)),
                    Cell::from(Span::styled(
                        format!("{:.0}%", m.se),
                        theme::se(m.se, th.extend_at, th.reduce_below),
                    )),
                    Cell::from(format!("{:.1}", m.quality)),
                    Cell::from(Span::styled(
                        if rx.is_some() {
                            format!("{off} of {}", m.nights)
                        } else {
                            "–".into()
                        },
                        if off > 0 { theme::off() } else { theme::dim() },
                    )),
                    Cell::from(Span::styled(
                        window,
                        if rx.is_some() {
                            theme::sleep()
                        } else {
                            theme::dim()
                        },
                    )),
                ]),
            }
        })
        .collect();
    let widths = [4, 14, 9, 7, 7, 9, 7, 11, 8, 13, 12].map(Constraint::Length);
    f.render_widget(
        Table::new(rows, widths)
            .header(header)
            .column_spacing(2)
            .block(panel("Weekly averages", false)),
        top,
    );

    draw_se_chart(f, chart_area, app, th);
    draw_next_step(f, next_area, app, th);
}

fn draw_se_chart(f: &mut Frame, area: Rect, app: &App, th: Thresholds) {
    let s = &app.subject;
    if s.entries.is_empty() {
        f.render_widget(
            Paragraph::new(" No nights yet.")
                .style(theme::dim())
                .block(panel("Sleep efficiency by night", false)),
            area,
        );
        return;
    }
    let first = s.entries.first().unwrap().night;
    let last = s.entries.last().unwrap().night;
    let x = |d: NaiveDate| (d - first).num_days() as f64;
    let span = x(last).max(1.0);
    let points: Vec<(f64, f64)> = s
        .entries
        .iter()
        .map(|e| (x(e.night), night(e).se))
        .collect();
    let extend = [(0.0, th.extend_at), (span, th.extend_at)];
    let reduce = [(0.0, th.reduce_below), (span, th.reduce_below)];
    let rx_marks: Vec<(f64, f64)> = s
        .prescriptions
        .iter()
        .filter(|p| p.from >= first && p.from <= last)
        .flat_map(|p| (40..=100).step_by(4).map(move |y| (x(p.from), y as f64)))
        .collect();
    let datasets = vec![
        Dataset::default()
            .name(format!("extend at {:.0}%", th.extend_at))
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(theme::good())
            .data(&extend),
        Dataset::default()
            .name(format!("shorten below {:.0}%", th.reduce_below))
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(theme::off())
            .data(&reduce),
        Dataset::default()
            .name("window changed")
            .marker(Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(theme::dim())
            .data(&rx_marks),
        Dataset::default()
            .name("efficiency")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(theme::sleep())
            .data(&points),
    ];
    let mid = first + Duration::days((span / 2.0) as i64);
    let chart = Chart::new(datasets)
        .block(panel("Sleep efficiency by night", false))
        .x_axis(
            Axis::default()
                .bounds([0.0, span])
                .style(theme::dim())
                .labels(
                    [first, mid, last]
                        .map(|d| Span::styled(d.format("%d %b").to_string(), theme::dim())),
                ),
        )
        .y_axis(
            Axis::default()
                .bounds([40.0, 100.0])
                .style(theme::dim())
                .labels(["40%", "70%", "100%"].map(|l| Span::styled(l, theme::dim()))),
        )
        .legend_position(Some(ratatui::widgets::LegendPosition::BottomRight))
        .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));
    f.render_widget(chart, area);
}

fn draw_next_step(f: &mut Frame, area: Rect, app: &App, th: Thresholds) {
    let s = &app.subject;
    let rec = recommend(s, app.today);
    let mut lines: Vec<Line> = Vec::new();
    match &rec {
        Recommendation::CollectBaseline { have, need } => {
            lines.push(Line::styled(
                "Keep collecting the baseline diary",
                theme::strong(),
            ));
            lines.push(Line::raw(format!("{have} of {need} nights logged.")));
            lines.push(Line::styled(
                "The first sleep window is set from at least a week of nights.",
                theme::dim(),
            ));
        }
        Recommendation::Initial {
            mean_tst,
            nights,
            rx,
        } => {
            lines.push(Line::styled("Set the first sleep window", theme::strong()));
            lines.push(Line::raw(format!(
                "Mean sleep over {nights} nights: {}",
                fmt_duration(*mean_tst)
            )));
            lines.push(Line::from(vec![
                Span::raw("Window "),
                Span::styled(window_text(rx), theme::sleep()),
            ]));
            lines.push(Line::styled(
                "Never shorter than 5 h. Anchored on the fixed rise time.",
                theme::dim(),
            ));
            if !s.screening.is_empty() {
                lines.push(Line::styled(
                    "Screening cautions are recorded. Check safety first and consider a gentler start such as sleep compression.",
                    theme::off(),
                ));
            }
            lines.push(Line::from(vec![
                Span::styled("a", theme::strong()),
                Span::styled(" apply from tonight", theme::dim()),
            ]));
        }
        Recommendation::Wait {
            since,
            days,
            nights,
        } => {
            lines.push(Line::styled(
                "Window in force, review not due",
                theme::strong(),
            ));
            lines.push(Line::raw(format!(
                "Since {}: {days} days, {nights} nights logged.",
                day(*since)
            )));
            lines.push(Line::styled(
                "Review after 7 days with at least 5 nights logged.",
                theme::dim(),
            ));
        }
        Recommendation::Review {
            se,
            nights,
            delta,
            reason,
            rx,
        } => {
            let verb = match delta.signum() {
                1 => "Extend the window",
                -1 => "Shorten the window",
                _ => "Keep the window",
            };
            lines.push(Line::styled(verb.to_string(), theme::strong()));
            lines.push(Line::from(vec![
                Span::raw(format!("Last {nights} nights: efficiency ")),
                Span::styled(
                    format!("{se:.0}%"),
                    theme::se(*se, th.extend_at, th.reduce_below),
                ),
            ]));
            lines.push(Line::styled(reason.clone(), theme::dim()));
            lines.push(Line::from(vec![
                Span::raw("New window "),
                Span::styled(window_text(rx), theme::sleep()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("a", theme::strong()),
                Span::styled(" apply from tonight", theme::dim()),
            ]));
        }
    }
    lines.push(Line::raw(""));
    let rise = s
        .rise_target
        .map_or("not set, uses the median from the diary".to_string(), |t| {
            t.to_string()
        });
    lines.push(Line::styled(
        format!("Fixed rise time {rise}"),
        theme::dim(),
    ));
    lines.push(Line::styled(
        format!(
            "{} thresholds: extend at {:.0}%, shorten below {:.0}%",
            if s.older_adult {
                "Older-adult"
            } else {
                "Standard"
            },
            th.extend_at,
            th.reduce_below
        ),
        theme::dim(),
    ));
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel("Next step", true).padding(ratatui::widgets::Padding::horizontal(1))),
        area,
    );
}

// ---------------------------------------------------------------- Protocol

fn draw_protocol(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.subject;
    let [left, right] =
        Layout::horizontal([Constraint::Length(58), Constraint::Min(30)]).areas(area);
    let [list_area, notes_area] =
        Layout::vertical([Constraint::Min(9), Constraint::Length(9)]).areas(left);

    let list_focus = !app.proto_items_focused;
    let mut lines = Vec::new();
    let row = |i: usize, label: String, right: Vec<Span<'static>>| {
        let sel = i == app.proto_sel;
        let style = match (sel, list_focus) {
            (true, true) => theme::selected(),
            (true, false) => theme::strong(),
            _ => theme::text(),
        };
        let mut spans = vec![
            Span::raw(if sel { "›" } else { " " }),
            Span::styled(label, style),
        ];
        spans.extend(right);
        Line::from(spans)
    };
    let cautions = s.screening.len();
    lines.push(row(
        0,
        format!("{:<41}", "Screening"),
        vec![if cautions > 0 {
            Span::styled(
                format!("{cautions} caution{}", if cautions == 1 { "" } else { "s" }),
                theme::off(),
            )
        } else {
            Span::styled("none recorded", theme::dim())
        }],
    ));
    lines.push(Line::raw(""));
    for def in SESSIONS {
        let rec = s.session(def.number);
        let done = rec.map_or(0, |r| r.done.len());
        let total = def.steps.len();
        let progress = Span::styled(
            format!("{done}/{total}"),
            if done == total {
                theme::good()
            } else if done > 0 {
                theme::lamp()
            } else {
                theme::dim()
            },
        );
        let isi = rec
            .and_then(|r| r.isi)
            .map_or(String::new(), |v| format!("  ISI {v}"));
        lines.push(row(
            def.number as usize,
            format!("{:<41}", format!("{} {}", def.number, def.title)),
            vec![progress, Span::styled(isi, theme::dim())],
        ));
    }
    f.render_widget(
        Paragraph::new(lines).block(panel("Sessions", list_focus)),
        list_area,
    );

    let mut notes: Vec<Line> = AASM_NOTES
        .iter()
        .map(|n| Line::styled(n.to_string(), theme::dim()))
        .collect();
    notes.insert(
        0,
        Line::styled("AASM combination-treatment guideline", theme::text()),
    );
    f.render_widget(
        Paragraph::new(notes)
            .wrap(Wrap { trim: true })
            .block(panel("Guideline", false).padding(ratatui::widgets::Padding::horizontal(1))),
        notes_area,
    );

    draw_protocol_items(f, right, app);
}

fn draw_protocol_items(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.subject;
    let focus = app.proto_items_focused;
    let mut lines = Vec::new();
    let item =
        |i: usize, checked: bool, mark: &'static str, mark_style: Style, text: &'static str| {
            let here = focus && i == app.proto_item;
            Line::from(vec![
                Span::styled(if here { "› " } else { "  " }, theme::sleep()),
                Span::styled(
                    if checked { mark } else { "[ ]" },
                    if checked { mark_style } else { theme::dim() },
                ),
                Span::raw(" "),
                Span::styled(
                    text,
                    if here {
                        theme::cursor_row()
                    } else {
                        theme::text()
                    },
                ),
            ])
        };

    let title;
    if app.proto_sel == 0 {
        title = "Screening".to_string();
        lines.push(Line::styled(
            "Tick anything present. Each is a reason for caution or a modified sleep restriction.",
            theme::dim(),
        ));
        lines.push(Line::raw(""));
        for (i, st) in SCREENING.iter().enumerate() {
            let present = s.screening.iter().any(|k| k == st.key);
            lines.push(item(i, present, "[!]", theme::off(), st.text));
        }
    } else {
        let def = &SESSIONS[app.proto_sel - 1];
        let rec = s.session(def.number);
        title = format!("Session {}: {}", def.number, def.title);
        lines.push(Line::styled(def.aim, theme::dim()));
        lines.push(Line::raw(""));
        for (i, st) in def.steps.iter().enumerate() {
            let done = rec.is_some_and(|r| r.done.iter().any(|k| k == st.key));
            lines.push(item(i, done, "[x]", theme::good(), st.text));
        }
        lines.push(Line::raw(""));
        let date = rec
            .and_then(|r| r.date)
            .map_or("not held yet".to_string(), day);
        lines.push(Line::from(vec![
            Span::styled("Date ", theme::dim()),
            Span::raw(date),
        ]));
        match rec.and_then(|r| r.isi) {
            Some(v) => {
                lines.push(Line::from(vec![
                    Span::styled("ISI ", theme::dim()),
                    Span::raw(format!("{v}, {}", isi_band(v))),
                ]));
                let baseline = s
                    .sessions
                    .iter()
                    .filter(|r| r.number < def.number)
                    .find_map(|r| r.isi);
                if let Some(b) = baseline {
                    lines.push(Line::from(vec![
                        Span::styled("Since baseline ", theme::dim()),
                        Span::raw(isi_change(b, v)),
                    ]));
                }
            }
            None => lines.push(Line::styled(
                "No ISI recorded. Press i to add one.",
                theme::dim(),
            )),
        }
        if let Some(r) = rec.filter(|r| !r.notes.is_empty()) {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled("Notes ", theme::dim()),
                Span::raw(r.notes.clone()),
            ]));
        }
    }
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(&title, focus).padding(ratatui::widgets::Padding::horizontal(1))),
        area,
    );
}

// ---------------------------------------------------------------- Overlays

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let [r] = Layout::horizontal([Constraint::Length(w.min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [r] = Layout::vertical([Constraint::Length(h.min(area.height))])
        .flex(Flex::Center)
        .areas(r);
    r
}

fn draw_overlay(f: &mut Frame, o: &Overlay) {
    let (title, lines, h): (&str, Vec<Line>, u16) = match o {
        Overlay::Confirm { message, .. } => (
            "Confirm",
            vec![
                Line::raw(message.clone()),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("y", theme::strong()),
                    Span::styled(" yes    ", theme::dim()),
                    Span::styled("n", theme::strong()),
                    Span::styled(" no", theme::dim()),
                ]),
            ],
            7,
        ),
        Overlay::Prompt { label, buffer, .. } => (
            "Enter a value",
            vec![
                Line::raw(label.clone()),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("› ", theme::sleep()),
                    Span::styled(buffer.clone(), theme::strong()),
                    Span::styled("▏", theme::sleep()),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("Enter", theme::strong()),
                    Span::styled(" save    ", theme::dim()),
                    Span::styled("Esc", theme::strong()),
                    Span::styled(" cancel", theme::dim()),
                ]),
            ],
            9,
        ),
        Overlay::Help => {
            let k = |k: &str, v: &str| {
                Line::from(vec![
                    Span::styled(format!("{k:<14}"), theme::strong()),
                    Span::styled(v.to_string(), theme::text()),
                ])
            };
            (
                "Keys",
                vec![
                    k("Tab", "next view (Shift+Tab back); 1 to 4 outside the form"),
                    k("Ctrl+Q", "quit (q also works outside the form)"),
                    Line::raw(""),
                    Line::styled("Log a night", theme::sleep()),
                    k("↑ ↓ Enter", "move between questions"),
                    k("Ctrl+S", "save the night (Enter on Notes also saves)"),
                    k("Ctrl+N", "start a blank form"),
                    Line::styled("Times accept 23:15, 2315 or 23.15", theme::dim()),
                    Line::raw(""),
                    Line::styled("Diary", theme::sleep()),
                    k("e  d  x", "edit, delete, export CSV"),
                    Line::styled("Weeks", theme::sleep()),
                    k(
                        "a  r  o",
                        "apply next step, rise time, older-adult thresholds",
                    ),
                    Line::styled("Protocol", theme::sleep()),
                    k("← → Space i", "switch pane, tick a step, record ISI"),
                    Line::raw(""),
                    Line::styled("Any key closes this.", theme::dim()),
                ],
                22,
            )
        }
    };
    let area = centered(f.area(), 76, h);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(title, true).padding(ratatui::widgets::Padding::new(2, 2, 1, 0))),
        area,
    );
}
