"""Lead: turn scoping reports (research/scope-*.json) into lanes that reuse a completed lane's tree and target.

Usage: python wave_from_scope.py --base <sha> [--hold "text"] [--tree AREA=TREE ...] <area> [<area> ...]
Checks every tree first and changes nothing if any check fails. Then switches each tree to its new lane branch and writes
brief, lane file and ledger row through lanegen. The lead commits and pushes the records. HANDOFF topic workflow, item 2.
"""
import argparse
import json
import os
import subprocess
import sys

S = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
WORKTREES = os.path.dirname(os.path.abspath(os.path.join(S, "..", "..", "..")))
LANES = os.path.join(WORKTREES, "wt-studio-lanes")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lanegen import dump, write_wave  # noqa: E402

COMMON = ("\n\nOPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. "
          "Message the lead with your tip before the bundled run, so review agents inspect it first. "
          "Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. "
          "No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.")
REQUIRED = ("id", "file", "brief", "crate", "crates_head", "summary", "target", "diag", "attr", "paths", "suites", "closes", "mts", "tree", "disk")


def git(tree, *args):
    return subprocess.run(["git", "-C", tree, *args], capture_output=True, text=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--hold", default="")
    ap.add_argument("--tree", action="append", default=[])
    ap.add_argument("areas", nargs="+")
    a = ap.parse_args()
    trees = dict(x.split("=", 1) for x in a.tree)
    led = json.load(open(os.path.join(S, "ledger.json"), encoding="utf-8"))
    status = {str(x.get("lane_id", "")).lower(): x.get("status") for x in led["lanes"]}
    # A READY lane whose tip is on origin hands over its tree and target (direction.blocker_decisions_20261107.capacity).
    done = lambda s: str(s).startswith("PASS_V") or s == "READY_FOR_VALIDATION"
    in_use = {v["tree"] for k, v in led["direction"]["workflow_20261107"]["target_reuse"].items() if not done(status.get(k.lower(), ""))}
    lanes = []
    for area in a.areas:
        L = json.load(open(os.path.join(S, "research", f"scope-20261107-{area}.json"), encoding="utf-8"))
        missing = [k for k in REQUIRED if k not in L]
        if missing:
            sys.exit(f"{area}: scope lacks {missing}")
        L["tree"] = trees.get(area, L["tree"])
        L["disk"] = "d" if str(L["disk"]).lower().startswith("d") else "ssd"
        L["target"] = L["target"] + (("\n\n" + a.hold) if a.hold else "") + COMMON
        t = L["tree"]
        full = os.path.join(LANES, t)
        if t in in_use:
            sys.exit(f"{area}: tree {t} is in use")
        if not done(status.get(t, "")):
            sys.exit(f"{area}: tree {t} status is {status.get(t)}")
        if git(full, "status", "--porcelain").stdout.strip():
            sys.exit(f"{area}: tree {t} is not clean")
        head = git(full, "rev-parse", "HEAD").stdout.strip()
        if git(full, "merge-base", "--is-ancestor", head, a.base).returncode != 0:
            sys.exit(f"{area}: tree {t} tip is not on {a.base}")
        if git(full, "rev-parse", "--verify", "-q", "lane/" + L["id"].lower()).returncode == 0:
            sys.exit(f"{area}: branch for {L['id']} exists")
        in_use.add(t)
        lanes.append(L)
    moves = {}
    for L in lanes:
        full = os.path.join(LANES, L["tree"])
        old = git(full, "branch", "--show-current").stdout.strip()
        new = "lane/" + L["id"].lower()
        r = git(full, "switch", "-q", "-c", new, a.base)
        if r.returncode != 0:
            sys.exit(f"{L['id']}: {r.stderr}")
        moves[L["id"]] = {"tree": L["tree"], "branch": new, "previous_branch": old}
        print(L["id"], "on", L["tree"], "disk", L["disk"], "branch", new)
    now = write_wave(lanes, a.base, "new lanes from scoping reports, reusing completed trees and targets: " + "; ".join(f"{L['id']} on {L['tree']}" for L in lanes))
    led = json.load(open(os.path.join(S, "ledger.json"), encoding="utf-8"))
    w = led["direction"]["workflow_20261107"]
    w["lane_branches"] += [m["branch"] for m in moves.values()]
    w["target_reuse"].update(moves)
    dump(os.path.join(S, "ledger.json"), led, 1)
    print(now)


if __name__ == "__main__":
    main()
