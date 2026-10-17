# Safety — who decides, red flags, privacy

## Who decides
- Prescriber: every drug, dose, time, start and stop. Logged in `03_subjects/<id>/rx.md` with date + source.
- App: measures, shows, reminds. Never proposes a drug or a dose. Self-reported meds go in `profile.md`
  marked `self_report`.
- Sleep restriction: none in phase 00. With screening flag `bipolar` or `seizure`: no restriction until
  a clinician reviews `window_floor_bipolar`.

## Red flags -> contact prescriber (app shows this text; it decides nothing)
- Sleeping less but feeling MORE energy / less need for sleep (not tired). Thresholds: `clinical-params.md`.
- Racing thoughts, unusual spending or risk-taking, irritability.
- New skin rash while on lamotrigine -> same day.
- After any new serotonergic drug: agitation, tremor, sweating, fever, diarrhea -> urgent.
- Prolonged or painful erection on trazodone -> emergency.
- Thoughts of self-harm or suicide -> emergency services or a crisis line now.
- Sleepy at work or driving -> stop the risky task.

## Privacy
- Repo is public. `03_subjects/*` is gitignored and every subject folder self-ignores (`.gitignore` = `*`).
- Discord gets status only ("AM check-in done/missed"). Never values, meds or notes.
- Recording a visit needs everyone's OK at the start. Transcribe locally. Audio and transcripts never leave the machine.
- A written prescription or doctor's note beats any transcript.
