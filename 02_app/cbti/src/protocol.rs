//! A six-session CBT-I protocol, screening items and ISI interpretation.
//!
//! Session content follows the standard multicomponent CBT-I outline
//! (Perlis et al., Cognitive Behavioral Treatment of Insomnia; Edinger & Carney)
//! and the components named in the AASM behavioral and combination-treatment
//! guidelines: sleep restriction, stimulus control, cognitive therapy,
//! sleep hygiene and relaxation.

pub struct Step {
    pub key: &'static str,
    pub text: &'static str,
}

pub struct SessionDef {
    pub number: u8,
    pub title: &'static str,
    pub aim: &'static str,
    pub steps: &'static [Step],
}

const fn step(key: &'static str, text: &'static str) -> Step {
    Step { key, text }
}

pub const SESSIONS: &[SessionDef] = &[
    SessionDef {
        number: 1,
        title: "Assessment and baseline",
        aim: "Confirm the diagnosis, screen for cautions, and start the diary.",
        steps: &[
            step(
                "dx",
                "Confirm chronic insomnia: 3+ nights a week, 3+ months, with daytime impairment",
            ),
            step(
                "screen",
                "Work through the screening list for sleep restriction cautions",
            ),
            step("isi", "Record a baseline ISI score"),
            step(
                "meds",
                "Note current sleep medication and who prescribes it",
            ),
            step(
                "diary",
                "Teach the diary: fill it in each morning, estimates are fine",
            ),
        ],
    },
    SessionDef {
        number: 2,
        title: "Sleep restriction and stimulus control",
        aim: "Set the first sleep window from the baseline diary and the bed-only rules.",
        steps: &[
            step("review", "Review at least 7 nights of baseline diary"),
            step("rise", "Agree a fixed rise time for all 7 days"),
            step(
                "window",
                "Set the window from mean total sleep time, never under 5 h",
            ),
            step(
                "sc",
                "Stimulus control: bed for sleep and sex only; get up if awake about 20 min",
            ),
            step("naps", "No napping; no lying down outside the window"),
            step(
                "safety",
                "Warn about sleepiness: no driving or risky tasks when drowsy",
            ),
        ],
    },
    SessionDef {
        number: 3,
        title: "Titration and adherence",
        aim: "Adjust the window by sleep efficiency and solve adherence problems.",
        steps: &[
            step(
                "titrate",
                "Titrate the window by last week's sleep efficiency",
            ),
            step(
                "adherence",
                "Go through early bedtimes, late rises and naps",
            ),
            step(
                "hygiene",
                "Sleep hygiene: caffeine, alcohol, light, exercise, bedroom",
            ),
            step("isi", "Record ISI"),
        ],
    },
    SessionDef {
        number: 4,
        title: "Cognitive therapy",
        aim: "Work on the beliefs and worry that keep insomnia going.",
        steps: &[
            step("titrate", "Titrate the window"),
            step(
                "beliefs",
                "Identify unhelpful beliefs about sleep (DBAS-16)",
            ),
            step(
                "decat",
                "Test catastrophic predictions about the day after a poor night",
            ),
            step(
                "worry",
                "Constructive worry: write worries and plans before the wind-down",
            ),
        ],
    },
    SessionDef {
        number: 5,
        title: "Consolidation and relaxation",
        aim: "Keep extending the window while sleep stays efficient.",
        steps: &[
            step("titrate", "Titrate the window"),
            step("relax", "Practise a relaxation or wind-down routine"),
            step(
                "meds",
                "Review medication plan with the prescriber if tapering",
            ),
            step("isi", "Record ISI"),
        ],
    },
    SessionDef {
        number: 6,
        title: "Relapse prevention",
        aim: "Close treatment with a plan for bad nights.",
        steps: &[
            step("isi", "Record a final ISI and compare it with baseline"),
            step("works", "Summarise what helped most"),
            step(
                "setback",
                "Setback plan: back to the window and bed-only rules after a run of poor nights",
            ),
            step("followup", "Book follow-up or booster session"),
        ],
    },
];

/// Conditions that call for caution or modification of sleep restriction.
pub const SCREENING: &[Step] = &[
    step("bipolar", "Bipolar disorder (sleep loss can trigger mania)"),
    step("seizure", "Seizure disorder"),
    step("osa", "Suspected or untreated sleep apnea"),
    step("parasomnia", "Parasomnia"),
    step("driving", "Drives or works in a safety-critical job"),
    step("falls", "Falls risk"),
    step("pregnancy", "Pregnancy"),
];

pub fn screening_text(key: &str) -> &'static str {
    SCREENING
        .iter()
        .find(|s| s.key == key)
        .map_or("Unknown item", |s| s.text)
}

/// AASM combination-treatment guideline, recommendations 1 and 2
/// (_config/sources/cbtI_aasm_guideline.md, workspace root).
pub const AASM_NOTES: &[&str] = &[
    "CBT-I with medication is suggested over medication alone (conditional, low certainty).",
    "Adding medication to CBT-I is suggested against over CBT-I alone (conditional, low certainty).",
    "Combination can suit people who most value faster gains in sleep time early in treatment.",
];

pub fn isi_band(score: u8) -> &'static str {
    match score {
        0..=7 => "no clinically significant insomnia",
        8..=14 => "subthreshold insomnia",
        15..=21 => "moderate clinical insomnia",
        _ => "severe clinical insomnia",
    }
}

/// Change from baseline: response is a drop of 8 or more, remission is a score under 8.
pub fn isi_change(baseline: u8, latest: u8) -> String {
    let drop = baseline as i16 - latest as i16;
    let mut s = format!("{baseline} to {latest} ({:+})", -drop);
    if latest < 8 {
        s.push_str(", remission");
    } else if drop >= 8 {
        s.push_str(", treatment response");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isi_bands_and_response() {
        assert_eq!(isi_band(19), "moderate clinical insomnia");
        assert_eq!(isi_change(19, 10), "19 to 10 (-9), treatment response");
        assert_eq!(isi_change(19, 6), "19 to 6 (-13), remission");
    }
}
