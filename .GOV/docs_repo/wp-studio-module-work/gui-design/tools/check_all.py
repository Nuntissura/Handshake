"""Cross-checks the design files: JSON parses, references resolve, cited clauses exist, mocks are offline."""
import glob
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..")
KERNEL = os.path.abspath(os.path.join(ROOT, "..", "..", ".."))
SPEC_DIR = os.path.join(KERNEL, "spec", "master-spec-v02.229", "spec-modules")
errs = []


def load(n):
    return json.load(open(os.path.join(ROOT, n), encoding="utf-8"))


ds, comps, lay, acc, bo = (load(n) for n in ("design-system.json", "components.json", "layouts.json", "acceptance-checks.json", "build-order.json"))
spec = open(os.path.join(SPEC_DIR, "14-studio-creative-suite.md"), encoding="utf-8").read() + open(os.path.join(SPEC_DIR, "15-native-desktop-gui-vulkan-gpu-ownership.md"), encoding="utf-8").read()
cited = set()
for name in ("design-system.json", "components.json", "layouts.json", "acceptance-checks.json", "build-order.json"):
    t = open(os.path.join(ROOT, name), encoding="utf-8").read()
    cited |= set(re.findall(r"\b(?:STU-[A-Z]+-\d+[A-Z]?|GUI-DESKTOP-\d+|MENU-INV-\d|SHL-P-\d+)\b", t))
for c in sorted(cited):
    if c not in spec:
        errs.append("cited but not in spec: " + c)

ids = {c["id"] for g in comps["groups"].values() for c in g}
used = {c for s in bo["steps"] for c in s["components"]}
for c in sorted(used - ids):
    errs.append("build-order names unknown component " + c)
unassigned = sorted(ids - used)
known_implicit = {"menu_item"}

steps = {s["id"] for s in bo["steps"]} | {d["id"] for d in bo["decisions_needed_first"]["items"]}
for s in bo["steps"]:
    for d in s["depends_on"]:
        if d not in steps:
            errs.append("%s depends on unknown %s" % (s["id"], d))
ac_ids = {c["id"] for c in acc["checks"]}
for s in bo["steps"]:
    for p in s["proof"]:
        m = re.match(r"AC-[A-Z]+-\d+", p)
        if m and m.group(0) not in ac_ids:
            errs.append("%s proof %s unknown" % (s["id"], p))

for path in glob.glob(os.path.join(ROOT, "mock-*.html")):
    t = open(path, encoding="utf-8").read()
    # SVG namespace free: no scheme at all may appear in a mock
    if re.search(r"https?://|@import|url\((?!#)", t):
        errs.append("external reference in " + os.path.basename(path))
for p in glob.glob(os.path.join(ROOT, "**", "*"), recursive=True):
    if " " in os.path.relpath(p, ROOT):
        errs.append("space in name: " + p)

print("cited clauses", len(cited), "components", len(ids), "checks", len(ac_ids), "steps", len(bo["steps"]))
print("components with no build step:", unassigned)
print("fit problems:", [(d, r["window"]) for d, rows in lay["fit_table"]["results"].items() for r in rows if not r["ok"]])
if errs:
    print("\n".join(errs))
    sys.exit(1)
print("ok")
