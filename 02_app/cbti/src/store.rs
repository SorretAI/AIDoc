//! One JSON file per subject, written atomically. CSV export for analysis.

use crate::metrics::night;
use crate::model::Subject;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn subject_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

pub fn load(path: &Path) -> Result<Subject> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

pub fn save(path: &Path, subject: &Subject) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(subject)?)?;
    fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn to_csv(subject: &Subject) -> String {
    let mut out = String::from(
        "night,week,bedtime,lights_out,sol_min,awakenings,waso_min,final_wake,out_of_bed,quality,nap_min,caffeine,alcohol,sleep_aid,notes,tib_min,tst_min,se_pct,rx_bed,rx_rise\n",
    );
    for e in &subject.entries {
        let m = night(e);
        let rx = subject.prescription_for(e.night);
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.1},{},{}\n",
            e.night,
            subject.week_of(e.night) + 1,
            e.bedtime,
            e.lights_out,
            e.sol_min,
            e.awakenings,
            e.waso_min,
            e.final_wake,
            e.out_of_bed,
            e.quality,
            e.nap_min,
            e.caffeine,
            e.alcohol,
            csv_field(&e.sleep_aid),
            csv_field(&e.notes),
            m.tib,
            m.tst,
            m.se,
            rx.map(|r| r.bed.to_string()).unwrap_or_default(),
            rx.map(|r| r.rise.to_string()).unwrap_or_default(),
        ));
    }
    out
}
