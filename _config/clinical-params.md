# Clinical params — the only home for clinical numbers

Status: `IN_CODE` = hardcoded before this gate existed (still needs clinician review) ·
`PENDING` = proposed, not actionable · `CLINICIAN_REVIEWED` = actionable (add who + date).
Table is machine-read: keep 6 columns, `value` numeric or `?`, no `|` inside cells.

| key | value | unit | lives in | source | status |
|---|---|---|---|---|---|
| baseline_nights_min | 7 | nights | titration.rs | CBT-I manuals (Perlis; Edinger & Carney), cbti README | IN_CODE |
| baseline_nights_target | 14 | nights | titration.rs | same | IN_CODE |
| window_floor | 300 | min | titration.rs FLOOR_MIN | standard CBT-I | IN_CODE |
| window_step | 15 | min | titration.rs | standard CBT-I | IN_CODE |
| se_extend | 90 | % | titration.rs | standard CBT-I | IN_CODE |
| se_shorten | 85 | % | titration.rs | standard CBT-I | IN_CODE |
| se_extend_older | 85 | % | titration.rs | standard CBT-I | IN_CODE |
| se_shorten_older | 80 | % | titration.rs | standard CBT-I | IN_CODE |
| window_floor_bipolar | 390 | min | not in code | Harvey 2015 CBT-I for bipolar, sources.md #3 (verify in paper) | PENDING |
| med_stable_days | ? | days | not in code | prescriber (fluoxetine has a long half-life) | PENDING |
| asrm_flag | 6 | score | not in code | Altman 1997, sources.md #6 | PENDING |
| redflag_short_high_nights | ? | nights | not in code | prescriber | PENDING |
