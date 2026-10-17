# cbti

Terminal app for running CBT-I (cognitive behavioural therapy for insomnia) with one subject:
collect the morning sleep diary, read it as an actogram, titrate the sleep window, and track
the six-session protocol.

```
cargo run --release -- seed            # synthetic 4-week subject at data/demo.json
cargo run --release -- demo            # open it
cargo run --release -- subject-01      # start (or reopen) a real subject
```

Other commands: `report SUBJECT` prints weekly averages and the next step, `export SUBJECT`
writes `data/SUBJECT.csv`. Options: `--data DIR` (or `$CBTI_DATA`), `--today YYYY-MM-DD`.

## Views

| View | What it does |
|---|---|
| Log a night | Consensus Sleep Diary questions. The night is drawn live under the week before it. |
| Diary | Every night since the start on an 18:00 to 12:00 axis, gaps shown, week headers carry the window. |
| Weeks | Weekly averages, nightly efficiency against the titration thresholds, and the next step. |
| Protocol | Screening cautions, six sessions with checklists, ISI scores and change from baseline. |

Actogram: `█` asleep, `░` awake in bed, rose = in bed outside the prescribed window.
Night awakenings are counted in the diary, not timed, so they are not placed on the bar.

## Clinical rules (src/titration.rs)

- First window = mean diary total sleep time over up to 14 baseline nights (at least 7), rounded
  to 15 min, never under 5 h, ending at the fixed rise time.
- Review after 7 days with 5+ nights logged, on the last 7 nights: SE >= 90% extend 15 min,
  85-90% keep, < 85% shorten 15 min (5 h floor). `o` switches to 85/80 for older adults.
- A recommendation is only applied when the clinician presses `a` and confirms.

This supports a clinician delivering CBT-I; it does not diagnose or decide treatment.

## Layout

`src/lib.rs` is the UI-free core (model, metrics, titration, protocol, store, synthetic data) so
other front ends can reuse it. `src/tui/` is the ratatui front end. Data is one JSON file per
subject, written atomically on every change.
