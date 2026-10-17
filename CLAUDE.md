# AIDoc — CBT-I instrument (ICM workspace)

**Where am I:** the sleep piece of AIQ. A Rust CBT-I app (`cbti`) that collects one subject's
sleep diary and check-ins, plus the research that feeds its clinical rules.
Method: ICM (Van Clief). Folders are the pipeline. Files are the state.

**Start every thread:** read the newest `00_handoffs/SEED-*.md`, then only the contract it names.

## Routing
| Task | Go to |
|---|---|
| Continue last thread | newest `00_handoffs/SEED-*.md` |
| Fetch or review a paper | `01_research/CONTEXT.md` |
| Change the app (Rust, Termux, Discord, CLI) | `02_app/CONTEXT.md` |
| Work with a subject (diary, check-ins, doctor visit) | `03_subjects/CONTEXT.md` |
| Any clinical number | `_config/clinical-params.md` (its only home) |
| Red flags, privacy, who decides | `_config/safety.md` |
| Links and evidence | `_config/sources.md` |
| How the pieces connect, status | `CONTEXT.md` |

## Laws
1. The app measures and reports. The clinician decides. The app never suggests a drug or a dose.
2. Clinical numbers live only in `_config/clinical-params.md`. Status `PENDING` = show, never act.
3. Repo is PUBLIC. Subject data lives in `03_subjects/<id>/` (gitignored). Never commit, push,
   paste or webhook health values.
4. One folder, one job. Read only the contract and inputs your step names.
5. Every stage stops at a human check before the next stage reads its output.

## Compat links (sunset 2026-11-16)
`cbti` -> `02_app/cbti`; `stage/...` -> new homes. Symlinks only. Delete them once nothing breaks.
