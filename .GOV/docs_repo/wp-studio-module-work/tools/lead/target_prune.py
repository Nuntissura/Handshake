"""Guarded partial removal from Studio cargo targets, from an Operator-approved plan (HSK-ART-005, CX-107).

Usage: python target_prune.py --plan research/cargo-cleanup-plan-<id>.json --approve ID,ID,... [--after-round R59] [--execute]
A dry run unless --execute. It works only on the plan's approved partial candidates, only inside a lane or round target
and only on the file kinds the plan names. A candidate that changed since inspection, is in use, or differs from the plan
by more than 5% is skipped and needs fresh approval. Removal is permanent and goes file by file, throttled. It never
touches libraries, build scripts, fingerprints, the registry, logs, results or anything outside target/.
"""
import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
import time

S = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
KERNEL = os.path.abspath(os.path.join(S, "..", "..", ".."))
WORKTREES = os.path.dirname(KERNEL)
ART = os.environ.get("HANDSHAKE_ARTIFACTS_ROOT") or os.path.join(WORKTREES, "Handshake_Artifacts")
D_ROOT = os.path.join(ART, "WP-KERNEL-STUDIO").replace("\\", "/")
C_LANES = "C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-team-dev/target/lanes"
HASHED = re.compile(r"^(?:lib)?(?P<name>.+?)-(?P<hash>[0-9a-f]{16})\.(?P<ext>exe|pdb|dll|dll\.lib|dll\.exp|lib|exp)$")
GIB = 1024.0 ** 3
MAX_FILES_PER_SEC = 600


def allowed_path(p):
    p = p.replace("\\", "/")
    if re.fullmatch(re.escape(C_LANES) + r"/[A-Za-z0-9_.-]+", p):
        return True
    return re.fullmatch(re.escape(D_ROOT) + r"/[A-Za-z0-9_.-]+/target", p) is not None


def newest(path):
    best = 0.0
    stack = [path]
    while stack:
        try:
            with os.scandir(stack.pop()) as it:
                for e in it:
                    if e.is_dir(follow_symlinks=False):
                        stack.append(e.path)
                    else:
                        best = max(best, e.stat(follow_symlinks=False).st_mtime)
        except OSError:
            pass
    return datetime.datetime.fromtimestamp(best, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%MZ") if best else None


def older_copies(deps):
    groups = {}
    for e in os.scandir(deps):
        if e.is_file(follow_symlinks=False):
            m = HASHED.match(e.name)
            if m:
                st = e.stat(follow_symlinks=False)
                groups.setdefault((m["name"], m["ext"]), []).append((st.st_mtime, st.st_size, e.path))
    return [(path, size) for rows in groups.values() for _, size, path in sorted(rows)[:-1]]


def all_binaries(folder):
    out = []
    for e in os.scandir(folder):
        if e.is_file(follow_symlinks=False) and e.name.lower().endswith((".exe", ".pdb")):
            out.append((e.path, e.stat(follow_symlinks=False).st_size))
    return out


def select(c):
    """Returns (files with sizes, directories to remove whole, their bytes)."""
    debug = os.path.join(c["path"], "debug")
    if c["owner"] == "studio-round":
        files = all_binaries(os.path.join(debug, "deps")) + all_binaries(debug)
        inc = os.path.join(debug, "incremental")
        size = 0
        if os.path.isdir(inc):
            for root, _, names in os.walk(inc):
                size += sum(os.path.getsize(os.path.join(root, n)) for n in names)
        return files, ([inc] if os.path.isdir(inc) else []), size
    return older_copies(os.path.join(debug, "deps")), [], 0


def command_lines():
    out = subprocess.run(["powershell", "-NoProfile", "-Command",
                          "Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and $_.Name -in 'cargo.exe','rustc.exe','link.exe','rust-lld.exe','cargo-nextest.exe' -or $_.Name -like '*test*' } | ForEach-Object { $_.CommandLine }"],
                         capture_output=True, text=True)
    return out.stdout.replace("\\", "/").lower()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--plan", required=True)
    ap.add_argument("--approve", required=True)
    ap.add_argument("--after-round")
    ap.add_argument("--execute", action="store_true")
    a = ap.parse_args()
    plan = json.load(open(a.plan, encoding="utf-8"))
    assert plan.get("schema") == "hsk.studio.cleanup_plan@1"
    led = json.load(open(os.path.join(S, "ledger.json"), encoding="utf-8"))
    if a.after_round and f"round_{a.after_round}" not in led:
        sys.exit(f"target_prune: refused: ledger has no round_{a.after_round}; the approved condition is not met")
    if a.execute and not str(plan.get("status", "")).startswith("APPROVED"):
        sys.exit("target_prune: refused: the plan is not marked APPROVED")
    approved = set(a.approve.split(","))
    statuses = {str(x.get("lane_id", "")).lower(): x.get("status") for x in led["lanes"]}
    in_use = {v["tree"] for k, v in led["direction"].get("workflow_20261107", {}).get("target_reuse", {}).items() if not str(statuses.get(k.lower(), "")).startswith("PASS_V")}
    procs = command_lines()
    results = []
    total = 0
    for c in plan["candidates"]:
        if c["id"] not in approved:
            continue
        row = {"id": c["id"], "owner": c["owner"], "planned_gib": c["gib"]}
        results.append(row)
        path = c["path"].replace("\\", "/")
        if c.get("kind") != "partial" or not allowed_path(path):
            row["status"] = "REFUSED_PATH"
            continue
        if not os.path.isfile(os.path.join(path, ".studio-workspace-root")) or not os.path.isdir(os.path.join(path, "debug", ".fingerprint")):
            row["status"] = "REFUSED_NOT_A_TARGET"
            continue
        if c["owner"] in in_use:
            row["status"] = "SKIPPED_IN_USE_BY_LANE"
            continue
        if c["owner"] != "studio-round" and not str(statuses.get(c["owner"], "")).startswith("PASS_V"):
            row["status"] = "SKIPPED_STATUS"
            continue
        if path.lower() in procs:
            row["status"] = "SKIPPED_PROCESS"
            continue
        seen = newest(path)
        if seen != c.get("newest_file_utc"):
            row.update(status="SKIPPED_CHANGED", newest_now=seen, newest_planned=c.get("newest_file_utc"))
            continue
        files, dirs, dir_bytes = select(c)
        size = sum(s for _, s in files) + dir_bytes
        row.update(files=len(files), dirs=[d.replace("\\", "/") for d in dirs], gib=round(size / GIB, 2))
        if c["gib"] and abs(size / GIB - c["gib"]) > 0.05 * c["gib"] + 0.02:
            row["status"] = "SKIPPED_DIFFERS_FROM_PLAN"
            continue
        if not a.execute:
            row["status"] = "WOULD_REMOVE"
            total += size
            continue
        done = 0
        started = time.time()
        for n, (p, s) in enumerate(files, 1):
            os.remove(p)
            done += s
            if n % MAX_FILES_PER_SEC == 0:
                time.sleep(max(0.0, n / MAX_FILES_PER_SEC - (time.time() - started)))
        for d in dirs:
            if os.path.basename(d) != "incremental" or not d.replace("\\", "/").startswith(path + "/"):
                raise SystemExit(f"target_prune: refused directory {d}")
            shutil.rmtree(d)
            done += dir_bytes
        total += done
        row["status"] = "REMOVED"
    report = {
        "schema": "hsk.studio.cleanup_result@1", "plan": plan.get("file_id"), "execute": a.execute,
        "at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "total_gib": round(total / GIB, 2), "results": results,
    }
    print(json.dumps(report, indent=1, ensure_ascii=False))
    if a.execute:
        out = os.path.join(S, "research", os.path.basename(a.plan).replace("plan", "result"))
        with open(out, "w", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps(report, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
