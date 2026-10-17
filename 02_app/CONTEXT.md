# 02_app — the cbti instrument

**Job:** collect and show one subject's sleep data; recommend sleep-window changes only from reviewed
rules. Rust crate in `cbti/`: `src/lib.rs` = UI-free core, `src/tui/` = ratatui front end.

## Inputs
- Working: the change request in the newest `00_handoffs/SEED-*.md`.
- Reference: `_config/clinical-params.md`, `_config/safety.md`, `cbti/README.md`.

## Nouns (what is X)
| Object | File | Note |
|---|---|---|
| Subject | model.rs | screening flags, prescriptions, sessions, entries |
| DiaryEntry | model.rs | one night (Consensus Sleep Diary); `sleep_aid` is free text |
| Prescription | model.rs | the SLEEP WINDOW (bed/rise). Not a drug prescription |
| SessionRecord | model.rs | session done-steps, ISI |
| SESSIONS, SCREENING | protocol.rs | six sessions; flags incl. `bipolar`, `seizure` |
| FLOOR_MIN, thresholds | titration.rs | hardcoded today; mirrored as `IN_CODE` rows in clinical-params |

## What a change hits
| Change | Also touch, same commit |
|---|---|
| Field on a model struct | `#[serde(default)]` so old JSON loads; store.rs, synth.rs, tui/form.rs, tests |
| A clinical number | titration.rs, README, the clinical-params row |
| Protocol phases or steps | protocol.rs, tui/ui.rs (Protocol view) |
| CLI subcommand | main.rs, README command list |
| Data location | `CBTI_DATA=03_subjects/<id>/data`. `cbti/data/` holds the synthetic demo only |

## Agent-facing CLI follows AXI (`_config/sources.md` #8)
No args = live status, not help · compact output, `--json` escape hatch · explicit empty states ·
non-interactive, idempotent writes, real exit codes · next-step hint after output.
Human check-ins (Termux dialogs) are the only prompts.

## Run
`cd 02_app/cbti && cargo test` · `cargo run --release -- seed | demo | report ID | export ID`

## Human check
`cargo test` green; Luisfe opens the TUI on demo and on the subject; README matches behaviour.
