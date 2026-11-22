#!/usr/bin/env python3
"""Enforce this repository's own documentation policy.

The specs preach "enforcement, not recommendations" — this script applies
that to the docs themselves. Checks (all failing):

  1. Relative markdown links resolve.
  2. No Cyrillic anywhere in .md / .svg / .html (English-only policy).
  3. Frontmatter with `status:` and `type:` on every normative doc
     (docs/{specs,decisions,vision,business,guides}/*.md, README.md exempt).
  4. No marketing superlatives (honest-by-policy).
  5. Every MUST in the specs has a requirement identifier, the index is
     current, and conformance vectors cite only identifiers that exist
     (delegated to scripts/requirements.py --check).

Run from the repository root: python3 scripts/lint-docs.py
"""
import glob
import os
import re
import subprocess
import sys

errors: list[str] = []

# 1. Relative links -----------------------------------------------------------
for f in glob.glob("**/*.md", recursive=True):
    d = os.path.dirname(f)
    text = open(f, encoding="utf-8").read()
    # strip fenced code blocks: link syntax inside them is illustrative
    stripped = re.sub(r"```.*?```", "", text, flags=re.S)
    for m in re.finditer(r"\]\(([^)\s]+)\)", stripped):
        link = m.group(1).split("#")[0]
        if not link or link.startswith(("http://", "https://", "mailto:")):
            continue
        if not os.path.exists(os.path.join(d, link)):
            errors.append(f"{f}: broken link → {link}")

# 2. Cyrillic ------------------------------------------------------------------
for pattern in ("**/*.md", "**/*.svg", "**/*.html"):
    for f in glob.glob(pattern, recursive=True):
        for i, line in enumerate(open(f, encoding="utf-8", errors="ignore"), 1):
            if re.search(r"[Ѐ-ӿ]", line):
                errors.append(f"{f}:{i}: Cyrillic text (docs are English-only)")

# 3. Frontmatter on normative docs ---------------------------------------------
for sub in ("specs", "decisions", "vision", "business", "guides"):
    for f in glob.glob(f"docs/{sub}/*.md"):
        if os.path.basename(f) == "README.md":
            continue
        head = open(f, encoding="utf-8").read(400)
        if not head.startswith("---") or "status:" not in head or "type:" not in head:
            errors.append(f"{f}: missing frontmatter with status: and type:")

# 4. Marketing superlatives ------------------------------------------------------
FORBIDDEN = re.compile(
    r"\b(blazing(ly)?[- ]fast|revolutionary|game[- ]chang\w+|"
    r"best[- ]in[- ]class|next[- ]generation|cutting[- ]edge)\b",
    re.I,
)
for f in glob.glob("**/*.md", recursive=True):
    for i, line in enumerate(open(f, encoding="utf-8"), 1):
        if FORBIDDEN.search(line):
            errors.append(f"{f}:{i}: marketing superlative — say what it does instead")

# 5. Requirement identifiers --------------------------------------------------
req = subprocess.run(
    [sys.executable, os.path.join(os.path.dirname(__file__), "requirements.py"), "--check"],
    capture_output=True, text=True,
)
if req.returncode != 0:
    # Its first stdout line is a count; a crash leaves only a traceback on stderr.
    errors.extend(l.strip() for l in req.stdout.splitlines()[1:])
    if req.stderr.strip():
        errors.append("scripts/requirements.py crashed: " + req.stderr.strip().splitlines()[-1])

if errors:
    print(f"❌ {len(errors)} problem(s):")
    for e in errors:
        print("  " + e)
    sys.exit(1)
print("✅ docs lint clean")
