#!/usr/bin/env python3
"""
jina_fetch.py
-------------
One job: take a URL, run it through Jina Reader (no API key needed for
basic/rate-limited use), get back clean markdown, write it to a .md file.

This is a Layer 4 (working artifact) producer in ICM terms. It does not
parse, summarize, or structure anything — that's the next stage's job.
One stage, one job.

Run this YOURSELF, locally or on your VPS. Claude does not execute this.

Usage:
    python jina_fetch.py <url> [output_path]

Examples:
    python jina_fetch.py https://pmc.ncbi.nlm.nih.gov/articles/PMC13076838/
    python jina_fetch.py https://pmc.ncbi.nlm.nih.gov/articles/PMC13076838/ 01_research/01_fetch/output/cbtI_aasm_guideline.md
"""

import sys
import os
import requests
from datetime import datetime, timezone

JINA_BASE = "https://r.jina.ai/"
TIMEOUT_SECONDS = 30


def fetch_via_jina(target_url: str) -> str:
    """
    Sends target_url through Jina Reader's relay, which strips bot-walls,
    JS rendering, and ads, returning clean markdown text.
    No API key required for key-less/rate-limited use.
    """
    jina_url = JINA_BASE + target_url
    response = requests.get(jina_url, timeout=TIMEOUT_SECONDS)
    response.raise_for_status()
    return response.text


def write_output(content: str, output_path: str, source_url: str) -> None:
    """
    Writes the fetched markdown to disk with a small provenance header.
    Provenance header matters: six months from now you want to know
    exactly which URL and when, without re-deriving it from a filename.
    """
    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)

    header = (
        f"<!-- fetched_via: jina_fetch.py -->\n"
        f"<!-- source_url: {source_url} -->\n"
        f"<!-- fetched_at_utc: {datetime.now(timezone.utc).isoformat()} -->\n\n"
    )

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(header + content)

    print(f"[done] {len(content)} chars written to: {output_path}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python jina_fetch.py <url> [output_path]")
        sys.exit(1)

    target_url = sys.argv[1]

    if len(sys.argv) >= 3:
        output_path = sys.argv[2]
    else:
        # Default: dump next to the script, named for quick recognition.
        # You'll almost always want to pass an explicit output_path in
        # real pipeline use — this default is just for a quick manual test.
        output_path = "jina_output.md"

    print(f"[fetching] {target_url}")
    try:
        markdown_content = fetch_via_jina(target_url)
    except requests.exceptions.HTTPError as e:
        print(f"[error] Jina returned an HTTP error: {e}")
        sys.exit(1)
    except requests.exceptions.RequestException as e:
        print(f"[error] Network/request error: {e}")
        sys.exit(1)

    write_output(markdown_content, output_path, target_url)


if __name__ == "__main__":
    main()
