# Workspace contract

## Lanes (number = dependency order)
| # | Folder | Job | Writes |
|---|---|---|---|
| 00 | `00_handoffs/` | Thread-to-thread seeds (the recursive loop) | `SEED-<topic>-vN.md` |
| 01 | `01_research/` | Paper -> candidate clinical numbers | `PENDING` rows in `_config/clinical-params.md` |
| 02 | `02_app/` | Encode reviewed rules; collect data | code in `02_app/cbti/`, phone tools |
| 03 | `03_subjects/` | One private record per subject | diary, check-ins, visits, `rx.md` |
| -- | `_config/` | Factory: params, safety, sources. Stable every run | changed only at review gates |

## Flow
paper -> `01_fetch` -> `02_review` (human) -> param `PENDING` -> clinician -> `CLINICIAN_REVIEWED`
-> `02_app` encodes -> subject uses app -> `03_subjects` record -> visit -> `rx.md` -> next SEED

## Human gates
- Research -> `_config`: a person checks every extracted number against its quote.
- `PENDING` -> `CLINICIAN_REVIEWED`: only a clinician flips it (name + date in the row).
- App change: `cargo test` green, then Luisfe walks the TUI.
- Visit: decisions copied verbatim into `rx.md`; a transcript is read by a human before use.

## Status by scan (no hand-kept status files)
- Unreviewed papers: files in `01_research/01_fetch/output/` not yet in `_config/sources/`.
- Open params: `grep -c PENDING _config/clinical-params.md`.
- Subject: nights in `03_subjects/<id>/data/<id>.json`, newest `visits/*.md`, last row of `rx.md`.
- Thread: newest `00_handoffs/SEED-*.md`.
