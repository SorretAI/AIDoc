# 02_review — extract numbers (human gate)

- **Inputs:** one `01_research/01_fetch/output/<slug>.md`; `_config/clinical-params.md`.
- **Process:** for each actionable number, quote the sentence and section; add a row with status `PENDING`. No code edits.
- **Output:** new rows in `_config/clinical-params.md`; source moved to `_config/sources/` and listed in `_config/sources.md`.
- **Human check:** a person compares each row with its quote. Only a clinician sets `CLINICIAN_REVIEWED` (name + date).
