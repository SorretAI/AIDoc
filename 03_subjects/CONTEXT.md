# 03_subjects — record library (PRIVATE)

**Unit:** one person going through the protocol, folder `03_subjects/<id>/` (gitignored).
**New subject:** `cp -r 03_subjects/_templates/subject 03_subjects/<id>` then
`echo '*' > 03_subjects/<id>/.gitignore` (folder hides itself from git and jj), fill `profile.md`.

## Record
| File | Holds | Written by |
|---|---|---|
| `profile.md` | screening, prescriber, consent, self-reported meds, schedule | subject / Luisfe |
| `rx.md` | clinician decisions only, dated, verbatim. SOURCE OF TRUTH | copied from doctor |
| `data/<id>.json` | diary + check-ins (`CBTI_DATA=03_subjects/<id>/data`) | app |
| `visits/YYYY-MM-DD.md` | questions before, decisions after | subject + review |
| `visits/YYYY-MM-DD.transcript.md` | local transcript, only with consent | tool |

## Phases
- **00 Fase de carga (baseline):** morning diary + evening check-in, screening, med timeline,
  consent. No sleep restriction. No med change unless `rx.md` says so. Ends at a clinician visit.
- **Gate:** visit -> decisions into `rx.md` -> params flipped or left `PENDING`.
- **Drug trial (optional):** exists only as an `rx.md` row. One change at a time; that day starts a new measuring window.
- **01-06:** CBT-I sessions (`02_app/cbti/src/protocol.rs`). Flag `bipolar`/`seizure` -> no restriction below a clinician-reviewed floor.

## Human check
Before any phase change, a person reads the last 7 nights and the newest `rx.md` row.
