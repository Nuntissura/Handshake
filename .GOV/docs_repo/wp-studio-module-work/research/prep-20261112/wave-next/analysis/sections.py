"""Scratch: aggregate spec-coverage-20261111 rows by prefix and section (read-only)."""
import collections
import glob
import json
import os
import re
import sys

R = sys.argv[1]
OUT = sys.argv[2]
rows = []
for f in sorted(glob.glob(os.path.join(R, "spec-coverage-20261111-g*.json"))):
    g = json.load(open(f, encoding="utf-8"))
    for r in g["rows"]:
        r["_g"] = g["group"]
        rows.append(r)

def prefix(i):
    m = re.match(r"(STU-[A-Z]+)", i)
    return m.group(1) if m else i

agg = collections.defaultdict(lambda: collections.Counter())
for r in rows:
    key = (r["_g"], prefix(r["id"]), r.get("section") or "")
    st = r["status"]
    agg[key][st] += 1
    if st in ("VALIDATED", "BUILT_NOT_VALIDATED") and r.get("ui") == "command":
        agg[key]["CMD_ONLY"] += 1
    if st == "PARTIAL" and r.get("ui") == "command":
        agg[key]["PARTIAL_CMD"] += 1

lines = []
for key, c in sorted(agg.items(), key=lambda kv: -(kv[1]["MISSING"] + kv[1]["PARTIAL"])):
    lines.append(f"{key[0]}\t{key[1]}\t{key[2][:70]}\tM={c['MISSING']}\tP={c['PARTIAL']}\tPcmd={c['PARTIAL_CMD']}\tV={c['VALIDATED']}\tB={c['BUILT_NOT_VALIDATED']}\tCMDONLY={c['CMD_ONLY']}\tL={c['LOCAL']}")
open(OUT, "w", encoding="utf-8").write("\n".join(lines) + "\n")

# per prefix
pp = collections.defaultdict(collections.Counter)
for r in rows:
    pp[prefix(r["id"])][r["status"]] += 1
    if r["status"] in ("VALIDATED", "BUILT_NOT_VALIDATED") and r.get("ui") == "command":
        pp[prefix(r["id"])]["CMD_ONLY"] += 1
for p, c in sorted(pp.items(), key=lambda kv: -kv[1]["MISSING"]):
    print(p, "M", c["MISSING"], "P", c["PARTIAL"], "V", c["VALIDATED"], "B", c["BUILT_NOT_VALIDATED"], "CMDONLY", c["CMD_ONLY"], "L", c["LOCAL"])
