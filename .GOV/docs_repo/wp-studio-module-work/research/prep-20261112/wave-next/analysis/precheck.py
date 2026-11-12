"""Scratch, read-only: the checks wave_from_scope.py runs before it changes anything, applied to the draft scopes.

It never switches a branch and writes nothing. Usage: python precheck.py <S> <drafts dir> <base>
"""
import glob
import json
import os
import subprocess
import sys

S, DRAFTS, BASE = sys.argv[1], sys.argv[2], sys.argv[3]
WORKTREES = os.path.dirname(os.path.abspath(os.path.join(S, "..", "..", "..")))
LANES = os.path.join(WORKTREES, "wt-studio-lanes")
REQUIRED = ("id", "file", "brief", "crate", "crates_head", "summary", "target", "diag", "attr", "paths", "suites", "closes", "mts", "tree", "disk")


def git(tree, *args):
    return subprocess.run(["git", "-C", tree, *args], capture_output=True, text=True)


led = json.load(open(os.path.join(S, "ledger.json"), encoding="utf-8"))
status = {str(x.get("lane_id", "")).lower(): x.get("status") for x in led["lanes"]}
done = lambda s: str(s).startswith("PASS_V") or s == "READY_FOR_VALIDATION"
in_use = {v["tree"]: k for k, v in led["direction"]["workflow_20261107"]["target_reuse"].items() if not done(status.get(k.lower(), ""))}
known = {x.get("lane_id") for x in led["lanes"]}
seen = set()
ok = True
for f in sorted(glob.glob(os.path.join(DRAFTS, "scope-20261107-*.json"))):
    L = json.load(open(f, encoding="utf-8"))
    probs = []
    miss = [k for k in REQUIRED if k not in L]
    if miss:
        probs.append(f"lacks {miss}")
    t = L["tree"]
    full = os.path.join(LANES, t)
    if t in in_use:
        probs.append(f"tree {t} in use by {in_use[t]}")
    if t in seen:
        probs.append(f"tree {t} named twice in this wave")
    if not done(status.get(t, "")):
        probs.append(f"tree {t} status {status.get(t)}")
    if git(full, "status", "--porcelain").stdout.strip():
        probs.append(f"tree {t} not clean")
    head = git(full, "rev-parse", "HEAD").stdout.strip()
    if git(full, "merge-base", "--is-ancestor", head, BASE).returncode != 0:
        probs.append(f"tree {t} tip {head[:8]} not on {BASE}")
    if git(full, "rev-parse", "--verify", "-q", "lane/" + L["id"].lower()).returncode == 0:
        probs.append("branch exists")
    if L["id"] in known:
        probs.append("lane id already in ledger")
    seen.add(t)
    ok &= not probs
    print(L["id"], "tree", t, "head", head[:8], "OK" if not probs else "BLOCKED: " + "; ".join(probs))
print("ALL_OK" if ok else "SOME_BLOCKED")
