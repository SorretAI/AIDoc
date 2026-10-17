# 01_fetch — URL -> clean markdown

- **Input:** one URL (guideline, trial, drug label).
- **Run (Luisfe, locally):** `python 01_research/01_fetch/jina_fetch.py <url> 01_research/01_fetch/output/<slug>.md`
- **Output:** `output/<slug>.md`, source URL + fetch time on top. No parsing, no summary.
- **Human check:** header present; full text, not a paywall or captcha page. Then `02_review/`.
