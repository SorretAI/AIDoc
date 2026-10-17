# 01_research — paper -> candidate clinical numbers

Two stages, in order. Each stops at a human check.

| Stage | Reads | Writes |
|---|---|---|
| `01_fetch/` | a URL | `01_fetch/output/<slug>.md` with provenance header |
| `02_review/` | `01_fetch/output/<slug>.md` | `PENDING` rows in `_config/clinical-params.md`; source moved to `_config/sources/` |

Reference: `_config/sources.md` (what we already hold). Nothing here touches code.
