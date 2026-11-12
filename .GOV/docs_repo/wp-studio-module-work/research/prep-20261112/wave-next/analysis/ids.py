"""Scratch: list MISSING/PARTIAL spec ids per section for given prefixes and section substrings (read-only)."""
import glob
import json
import os
import re
import sys

R = sys.argv[1]
pfx = sys.argv[2].split(",")
secs = [s.lower() for s in sys.argv[3].split("|")] if len(sys.argv) > 3 and sys.argv[3] else []
rows = []
for f in sorted(glob.glob(os.path.join(R, "spec-coverage-20261111-g*.json"))):
    rows += json.load(open(f, encoding="utf-8"))["rows"]

def rng(ids):
    nums = sorted(ids, key=lambda i: (re.sub(r"\d+.*$", "", i), int(re.search(r"(\d+)", i.split("-")[-1]).group(1)) if re.search(r"\d", i.split("-")[-1]) else 0, i))
    return ", ".join(nums)

by = {}
for r in rows:
    p = re.match(r"(STU-[A-Z]+)", r["id"])
    if not p or p.group(1) not in pfx:
        continue
    sec = r.get("section") or ""
    if secs and not any(s in sec.lower() for s in secs):
        continue
    by.setdefault(sec, {}).setdefault(r["status"], []).append(r["id"])
for sec, st in by.items():
    m, p = st.get("MISSING", []), st.get("PARTIAL", [])
    if not m and not p:
        continue
    print(f"## {sec[:80]}  M={len(m)} P={len(p)} V={len(st.get('VALIDATED', []))}")
    if m:
        print("  M:", rng(m))
    if p:
        print("  P:", rng(p))
