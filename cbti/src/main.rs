//! cbti: CBT-I sleep diary and protocol tracker.
//!
//!   cbti [SUBJECT]                 open the TUI (creates the subject if new)
//!   cbti seed [SUBJECT] [--force]  write a synthetic 4-week subject (default: demo)
//!   cbti report [SUBJECT]          print weekly averages and the next step
//!   cbti export [SUBJECT]          write SUBJECT.csv next to the JSON
//!
//! Options: --data DIR (default ./data, or $CBTI_DATA), --today YYYY-MM-DD

mod tui;

use anyhow::{Context, Result, bail};
use cbti::clock::fmt_duration;
use cbti::metrics::summarize;
use cbti::model::Subject;
use cbti::titration::{Recommendation, recommend};
use cbti::{store, synth};
use chrono::{Local, NaiveDate};
use std::path::PathBuf;

struct Args {
    command: String,
    subject: Option<String>,
    data: PathBuf,
    today: NaiveDate,
    force: bool,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        command: "tui".into(),
        subject: None,
        data: std::env::var_os("CBTI_DATA").map_or_else(|| PathBuf::from("data"), PathBuf::from),
        today: Local::now().date_naive(),
        force: false,
    };
    let mut it = std::env::args().skip(1);
    let mut positional = Vec::new();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data" => args.data = it.next().context("--data needs a directory")?.into(),
            "--today" => {
                let d = it.next().context("--today needs a date")?;
                args.today = NaiveDate::parse_from_str(&d, "%Y-%m-%d").context("--today must be YYYY-MM-DD")?;
            }
            "--force" => args.force = true,
            "-h" | "--help" => {
                println!("{}", include_str!("main.rs").lines().take_while(|l| l.starts_with("//!")).map(|l| l.trim_start_matches("//!").trim_start_matches(' ')).collect::<Vec<_>>().join("\n"));
                std::process::exit(0);
            }
            s if s.starts_with("--") => bail!("unknown option {s}"),
            _ => positional.push(a),
        }
    }
    let mut pos = positional.into_iter();
    if let Some(first) = pos.next() {
        if matches!(first.as_str(), "seed" | "report" | "export" | "snapshot") {
            args.command = first;
            args.subject = pos.next();
        } else {
            args.subject = Some(first);
        }
    }
    Ok(args)
}

fn main() -> Result<()> {
    let args = parse_args()?;
    match args.command.as_str() {
        "seed" => {
            let id = args.subject.unwrap_or_else(|| "demo".into());
            let path = store::subject_path(&args.data, &id);
            if path.exists() && !args.force {
                bail!("{} exists; pass --force to overwrite it", path.display());
            }
            store::save(&path, &synth::generate(&id, args.today))?;
            println!("Wrote synthetic subject to {}", path.display());
        }
        "report" => report(&load(&args, false)?.0, args.today),
        "export" => {
            let (s, path) = load(&args, false)?;
            let out = path.with_extension("csv");
            std::fs::write(&out, store::to_csv(&s))?;
            println!("Wrote {}", out.display());
        }
        "snapshot" => {
            // Hidden: render each view to HTML for visual review. CBTI_KEYS adds key presses
            // first, as space-separated tokens: Tab, Down, Up, Enter, Space, or literal text.
            use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
            let (s, path) = load(&args, false)?;
            let mut keys = Vec::new();
            for tok in std::env::var("CBTI_KEYS").unwrap_or_default().split_whitespace() {
                match tok {
                    "Tab" => keys.push(KeyEvent::from(KeyCode::Tab)),
                    "Down" => keys.push(KeyEvent::from(KeyCode::Down)),
                    "Up" => keys.push(KeyEvent::from(KeyCode::Up)),
                    "Right" => keys.push(KeyEvent::from(KeyCode::Right)),
                    "Enter" => keys.push(KeyEvent::from(KeyCode::Enter)),
                    "Space" => keys.push(KeyEvent::from(KeyCode::Char(' '))),
                    "F1" => keys.push(KeyEvent::from(KeyCode::F(1))),
                    "CtrlS" => keys.push(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)),
                    t => keys.extend(t.chars().map(|c| KeyEvent::from(KeyCode::Char(c)))),
                }
            }
            let (w, h) = (140, 42);
            println!("{}", tui::snapshot_html(s, path, args.today, &keys, w, h)?);
        }
        _ => {
            let (s, path) = load(&args, true)?;
            tui::run(s, path, args.today)?;
        }
    }
    Ok(())
}

fn load(args: &Args, create: bool) -> Result<(Subject, PathBuf)> {
    let id = args.subject.clone().unwrap_or_else(|| "demo".into());
    let path = store::subject_path(&args.data, &id);
    if path.exists() {
        return Ok((store::load(&path)?, path));
    }
    if !create {
        bail!("no subject file at {}", path.display());
    }
    let s = Subject::new(&id, args.today);
    store::save(&path, &s)?;
    Ok((s, path))
}

fn report(s: &Subject, today: NaiveDate) {
    println!("{}  started {}  {} nights logged", s.id, s.started, s.entries.len());
    println!("{:>4}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>5}", "week", "nights", "in bed", "asleep", "SOL", "WASO", "SE");
    let weeks = s.entries.last().map_or(0, |e| s.week_of(e.night) + 1);
    for w in 0..weeks {
        if let Some(m) = summarize(s.entries.iter().filter(|e| s.week_of(e.night) == w)) {
            println!(
                "{:>4}  {:>6}  {:>6}  {:>6}  {:>6.0}  {:>6.0}  {:>4.0}%",
                w + 1, m.nights, fmt_duration(m.tib), fmt_duration(m.tst), m.sol, m.waso, m.se
            );
        }
    }
    match recommend(s, today) {
        Recommendation::CollectBaseline { have, need } => println!("next: collect baseline ({have}/{need} nights)"),
        Recommendation::Initial { rx, mean_tst, .. } => println!("next: set first window {}-{} (mean TST {})", rx.bed, rx.rise, fmt_duration(mean_tst)),
        Recommendation::Wait { since, days, nights } => println!("next: wait (window since {since}, {days} days, {nights} nights)"),
        Recommendation::Review { reason, rx, .. } => println!("next: {reason} -> {}-{}", rx.bed, rx.rise),
    }
}
