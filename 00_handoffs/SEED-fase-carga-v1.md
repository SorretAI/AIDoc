# SEED fase-carga v1 — for Claude Code, run in ~/AIDoc

## Role
Implementer inside an ICM workspace. You build the measuring instrument; the clinician decides
treatment. Never write a drug suggestion, a dose, or an unsourced clinical number into code.
Numbers come from `_config/clinical-params.md`. `PENDING` = show it, never act on it.

## Read, in order, nothing else up front
1. `CLAUDE.md` 2. `02_app/CONTEXT.md` 3. `_config/clinical-params.md` 4. `_config/safety.md`
5. `03_subjects/CONTEXT.md`. Subject facts: `03_subjects/subject-01/profile.md` and `rx.md`
   (PRIVATE: never quote them in code, commits, logs or webhooks).

## Ask the user before coding
- Phone: Termux + Termux:API installed? Run `cbti` on the phone (`pkg install rust`) or only
  check-ins on the phone + sync to the PC? Sync how (Syncthing, manual copy)?
- Discord webhook: kept in an untracked `.env` as `AIDOC_DISCORD_WEBHOOK`?
- Subject rise/bed times (fill `profile.md` front matter if empty).
- Was the visit recorded with consent? Where is the audio?

## Build — one jj change per step, `cargo test` green after each
1. **Phase 00 "Fase de carga"** in `protocol.rs`, before Session 1: diary, screening, med timeline,
   consent, visit gate. No sleep window is set in phase 00.
2. **Params loader:** parse the `_config/clinical-params.md` table (6 fixed columns, tested). Use a
   value only when status is `CLINICIAN_REVIEWED` or `IN_CODE`. Flag `bipolar`/`seizure` with
   `window_floor_bipolar` not reviewed -> recommendation "clinician must set the floor"; never shorten.
3. **Med log:** `MedEvent { date, drug, dose_mg, time, source: rx|self_report, note }` on `Subject`,
   `#[serde(default)]`. Display only. No logic reads a dose.
4. **Check-ins:** `CheckIn { ts, kind: am|pm, energy -3..3, need_for_sleep: less|same|more,
   racing 0..3, mood -3..3, note }`, idempotent per (date, kind). Weekly ASRM (5 items, 0-4).
   Derived view: short night + high energy + "need less" -> show the safety.md red-flag text.
   Thresholds from params; while `PENDING`, show the raw pattern, no verdict.
5. **AXI CLI** (rules in `02_app/CONTEXT.md`; AXI is a design guide, not a Rust crate):
   `cbti status ID` (default view), `cbti checkin ID --am|--pm ...` (non-interactive),
   `cbti brief ID` -> markdown for the doctor: last 14 nights, check-in trend, med timeline, open questions.
6. **Termux** `02_app/termux/`: `checkin.sh am|pm` (termux-dialog -> `cbti checkin`); schedule with
   termux-job-scheduler or cronie; `termux-notification --ongoing --id checkin-<kind>` stays until
   answered; max one pending (no stacking); never inside the sleep window. Read the flags from
   `termux-notification -h` and `termux-dialog -h` on the device; do not guess them.
7. **Discord:** `02_app/termux/ping.sh` posts only "<id> AM check-in done|missed". No values.
8. **Visit transcript:** `02_app/tools/transcribe.sh <audio> <id> <date>` -> whisper.cpp, local ->
   `03_subjects/<id>/visits/<date>.transcript.md`. Refuse unless that visit file says `consent_recording: yes`.

## Done when
`cargo test` green · `cbti status subject-01` shows phase 00, nights logged, open params ·
Luisfe walked the TUI · a human read `cbti brief` · you wrote `00_handoffs/SEED-<next>-v2.md`
(what changed, what is open, what the visit decided — no health values).
