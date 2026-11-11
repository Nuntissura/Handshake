"""Guarded removal from Studio cargo targets, from an Operator-approved plan (HSK-ART-003, HSK-ART-005, CX-107).

Usage: python target_prune.py --plan research/cargo-cleanup-plan-<id>.json --approve ID,ID,... [--after-round R59] [--execute]
A dry run unless --execute, and --execute needs a plan marked APPROVED. It works only on the plan's approved candidates,
of two kinds. A candidate that changed since inspection (newest file), is in use, or differs from the plan by more than
5% is skipped and needs fresh approval. Removal is permanent and goes file by file, throttled. Neither kind touches the
cargo registry, logs, results, fixtures, a worktree or anything outside the candidate's target.
- partial: only inside a lane or round target and only the file kinds the plan names (older_copies: every copy of a test
  or binary in debug/deps except the newest); never libraries, build scripts or fingerprints. A lane target is in use
  while a lane short of READY_FOR_VALIDATION or PASS_V names its tree in target_reuse (plan T N1).
- whole (plan T N2): the directory C_LANES/<owner> and nothing else. The plan path must equal it after normalisation,
  with no symlink or junction on the way or inside; never studio-round or D:. The owner lane must be PASS_V or ABANDONED
  in the ledger (rule P2; READY_FOR_VALIDATION is not enough), or, for a plan entry marked 'no ledger row', the tree
  bound in .studio-workspace-root must no longer exist. No lane short of PASS_V or ABANDONED may name the tree (ledger
  target_reuse tree or old_tree_idle, a ledger lane row's worktree, a lane file's worktree), and no running cargo, rustc,
  link or test process may name the target, the bound tree or the owner.
"""
import argparse
import datetime
import glob
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import time

S = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
KERNEL = os.path.abspath(os.path.join(S, "..", "..", ".."))
WORKTREES = os.path.dirname(KERNEL)
ART = os.environ.get("HANDSHAKE_ARTIFACTS_ROOT") or os.path.join(WORKTREES, "Handshake_Artifacts")
D_ROOT = os.path.join(ART, "WP-KERNEL-STUDIO").replace("\\", "/")
C_LANES = "C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-team-dev/target/lanes"
C_ROUND = "C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-round/target"  # the round target, named in the docstring; plan S, 2026-11-07
HASHED = re.compile(r"^(?:lib)?(?P<name>.+?)-(?P<hash>[0-9a-f]{16})\.(?P<ext>exe|pdb|dll|dll\.lib|dll\.exp|lib|exp)$")
GIB = 1024.0 ** 3
MAX_FILES_PER_SEC = 600
REPARSE = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
OWNER = re.compile(r"[a-z0-9][a-z0-9_.-]*")
NEVER_UNDER = ("studio-round", "registry", "logs", "results", "fixtures")


def allowed_path(p):
    p = p.replace("\\", "/")
    if p == C_ROUND or re.fullmatch(re.escape(C_LANES) + r"/[A-Za-z0-9_.-]+", p):
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
    if c.get("select") == "older_copies":  # plan S: keep the newest copy of each binary, also in the round target
        return older_copies(os.path.join(debug, "deps")), [], 0
    if c["owner"] == "studio-round":
        files = all_binaries(os.path.join(debug, "deps")) + all_binaries(debug)
        inc = os.path.join(debug, "incremental")
        size = 0
        if os.path.isdir(inc):
            for root, _, names in os.walk(inc):
                size += sum(os.path.getsize(os.path.join(root, n)) for n in names)
        return files, ([inc] if os.path.isdir(inc) else []), size
    return older_copies(os.path.join(debug, "deps")), [], 0


def is_link(st):
    return stat.S_ISLNK(st.st_mode) or bool(getattr(st, "st_file_attributes", 0) & REPARSE)


def whole_path(owner, path):
    """The one directory a whole candidate may remove, C_LANES/<owner>, or None when the plan path is anything else."""
    if not OWNER.fullmatch(owner) or ".." in owner:
        return None
    target = C_LANES + "/" + owner
    parts = target.split("/")
    if parts[0].upper() != "C:" or any(x.lower() in NEVER_UNDER for x in parts):
        return None
    norm = lambda p: os.path.normcase(os.path.normpath(p))
    if norm(path) != norm(target) or norm(os.path.realpath(target)) != norm(target):
        return None
    for i in range(1, len(parts) + 1):  # no symlink or junction on the way, the target itself included
        p = "/".join(parts[:i]) + ("/" if i == 1 else "")
        if os.path.lexists(p) and is_link(os.lstat(p)):
            return None
    return target


def whole_contents(target):
    """Every file (path, size) below target and every directory below it, deepest first. Raises on a link inside."""
    files, dirs, stack = [], [], [target]
    while stack:
        with os.scandir(stack.pop()) as it:
            for e in it:
                st = e.stat(follow_symlinks=False)
                if e.is_symlink() or is_link(st):
                    raise ValueError(f"symlink or junction inside the target: {e.path}")
                if stat.S_ISDIR(st.st_mode):
                    dirs.append(e.path)
                    stack.append(e.path)
                else:
                    files.append((e.path, st.st_size))
    return files, sorted(dirs, key=lambda d: d.replace("\\", "/").count("/"), reverse=True)


def settled_whole(status):
    # Rule P2 (HSK-ART-003): a lane's target is listed for whole removal at PASS or abandonment. READY_FOR_VALIDATION is
    # not enough: a FAIL in the round sends the lane back to its tree and target.
    s = str(status or "")
    return s.startswith("PASS_V") or s.upper() == "ABANDONED"


def tree_of(field):
    """The tree a ledger or lane-file field names: 'wt-studio-lanes/<tree> ...', '<tree> (...)' or a path ending in it."""
    t = str(field or "").replace("\\", "/").strip()
    m = re.search(r"wt-studio-lanes/([A-Za-z0-9_.-]+)", t) or re.match(r"([A-Za-z0-9_.-]+)(?:\s|$)", t)
    return m.group(1) if m else os.path.basename(t.rstrip("/")) or None


def tree_users(led, statuses):
    """{tree: lane ids} of lanes short of PASS_V or ABANDONED that name the tree in the ledger or their lane file."""
    users = {}

    def add(tree, lane, status):
        if tree and not settled_whole(status):
            users.setdefault(tree, set()).add(str(lane))

    for k, v in led["direction"].get("workflow_20261107", {}).get("target_reuse", {}).items():
        for field in ("tree", "old_tree_idle"):
            add(tree_of(v.get(field)), k, statuses.get(k.lower()))
    for x in led["lanes"]:
        if x.get("worktree"):
            add(tree_of(x["worktree"]), x.get("lane_id"), x.get("status"))
    for f in sorted(glob.glob(os.path.join(S, "builders", "lanes", "*.json"))):
        try:
            d = json.load(open(f, encoding="utf-8"))
        except (OSError, ValueError) as err:
            sys.exit(f"target_prune: refused: lane file {os.path.basename(f)} unreadable ({err}); whole candidates need every lane file")
        if isinstance(d, dict) and d.get("worktree"):
            lane = str(d.get("lane") or os.path.basename(f))
            add(tree_of(d["worktree"]), lane, statuses[lane.lower()] if lane.lower() in statuses else d.get("status"))
    return users


def whole_basis(c, statuses, bound):
    """Why the owner lane is settled for whole removal, or None."""
    no_row = str(c.get("status", "")).startswith("no ledger row")
    if c["owner"] in statuses:
        s = statuses[c["owner"]]
        return str(s) if settled_whole(s) and not no_row else None
    tree = os.path.dirname(bound) if bound and os.path.isabs(bound) else None
    if no_row and tree and not os.path.lexists(tree):
        return "no ledger row; bound tree gone"
    return None


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

    def settled(status):
        # Plan T finding N1 (lead, 2026-11-08): a READY_FOR_VALIDATION lane's commits are on origin (record_ready.py
        # checks this before it writes READY), so its builder no longer writes the target. Removing its older test-binary
        # copies costs a relink only. The process check and the newest-file check below still refuse a target that is
        # being written.
        s = str(status or "")
        return s.startswith("PASS_V") or s == "READY_FOR_VALIDATION"

    in_use = {v["tree"] for k, v in led["direction"].get("workflow_20261107", {}).get("target_reuse", {}).items() if not settled(statuses.get(k.lower()))}
    wants_whole = any(c["id"] in approved and c.get("kind") == "whole" for c in plan["candidates"])
    users = tree_users(led, statuses) if wants_whole else {}
    procs = command_lines()
    results = []
    total = 0
    for c in plan["candidates"]:
        if c["id"] not in approved:
            continue
        row = {"id": c["id"], "owner": c["owner"], "planned_gib": c["gib"]}
        results.append(row)
        path = c["path"].replace("\\", "/")
        whole = c.get("kind") == "whole"
        if whole:
            row["kind"] = "whole"
            path = whole_path(c["owner"], path)
            if path is None:
                row["status"] = "REFUSED_PATH"
                continue
        elif c.get("kind") != "partial" or not allowed_path(path):
            row["status"] = "REFUSED_PATH"
            continue
        if not os.path.isfile(os.path.join(path, ".studio-workspace-root")) or not os.path.isdir(os.path.join(path, "debug", ".fingerprint")):
            row["status"] = "REFUSED_NOT_A_TARGET"
            continue
        if whole:
            bound = open(os.path.join(path, ".studio-workspace-root"), encoding="utf-8").read().strip()
            row["bound_to"] = bound
            if c["owner"] in users:
                row.update(status="SKIPPED_IN_USE_BY_LANE", used_by=sorted(users[c["owner"]]))
                continue
            basis = whole_basis(c, statuses, bound)
            tree = os.path.dirname(bound) if os.path.isabs(bound) else ""
            if basis is None:
                row.update(status="SKIPPED_STATUS", lane_status=statuses.get(c["owner"], "no ledger row"))
                if c["owner"] not in statuses:
                    row["bound_tree_exists"] = os.path.lexists(tree) if tree else None
                continue
            row["settled_by"] = basis
            tree = tree.replace("\\", "/").lower()
            if path.lower() in procs or (tree and tree in procs) or re.search(r"(?<![a-z0-9_.-])" + re.escape(c["owner"].lower()) + r"(?![a-z0-9_.-])", procs):
                row["status"] = "SKIPPED_PROCESS"
                continue
        else:
            if c["owner"] in in_use:
                row["status"] = "SKIPPED_IN_USE_BY_LANE"
                continue
            if c["owner"] != "studio-round" and not settled(statuses.get(c["owner"], "")):
                row["status"] = "SKIPPED_STATUS"
                continue
            if path.lower() in procs:
                row["status"] = "SKIPPED_PROCESS"
                continue
        seen = newest(path)
        if seen != c.get("newest_file_utc"):
            row.update(status="SKIPPED_CHANGED", newest_now=seen, newest_planned=c.get("newest_file_utc"))
            continue
        if whole:
            try:
                files, subdirs = whole_contents(path)
            except ValueError as err:
                row.update(status="REFUSED_LINK_INSIDE", error=str(err))
                continue
            except OSError as err:
                row.update(status="SKIPPED_UNREADABLE", error=str(err))
                continue
            dirs, dir_bytes = [path], 0
        else:
            files, dirs, dir_bytes = select(c)
        size = sum(s for _, s in files) + dir_bytes
        row.update(files=len(files), dirs=[d.replace("\\", "/") for d in dirs], gib=round(size / GIB, 2))
        if whole:
            row["subdirs"] = len(subdirs)
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
            if whole and not p.replace("\\", "/").startswith(path + "/"):
                raise SystemExit(f"target_prune: refused file outside {path}: {p}")
            os.remove(p)
            done += s
            if n % MAX_FILES_PER_SEC == 0:
                time.sleep(max(0.0, n / MAX_FILES_PER_SEC - (time.time() - started)))
        if whole:
            for d in subdirs:
                if not d.replace("\\", "/").startswith(path + "/"):
                    raise SystemExit(f"target_prune: refused directory {d}")
                os.rmdir(d)
            os.rmdir(path)
            dirs = []
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
