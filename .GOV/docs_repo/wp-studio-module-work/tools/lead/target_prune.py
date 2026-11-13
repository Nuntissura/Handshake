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
- N3 (plan V, Operator pause 2026-11-11), only for a partial older_copies candidate on D: (D_ROOT); C: candidates, whole
  removal and every other rule are unchanged.
  (a) The steward canary target D_ROOT/studio-team-dev/steward-export/target is allowed (that exact path after
      normalisation, no junction on the way); nothing else under studio-team-dev is, for any partial candidate
      (D_ROOT/studio-team-dev/target is not a lane target and is REFUSED_PATH). It must still hold
      .studio-workspace-root and debug/.fingerprint, and it has no lane row, so the lane status check is not applied.
  (b) A tree that unsettled lanes name in target_reuse is accepted (row field accepted_paused) when every one of those
      lanes is in ledger direction.pause_20261111.lanes with "tree" equal to the owner; otherwise it stays
      SKIPPED_IN_USE_BY_LANE (row field not_paused).
  For (a) and (b) the process check also refuses a running process that names the owner (tree name) or the bound
  workspace. A candidate without newest_file_utc (plan V was inspected with WizTree, not target_report.py) is checked
  by size only: the newest file is reported (newest_now) but not compared; the 5% size check applies unchanged.
- N4 (plan W, 2026-11-13): whole also on the Studio artifact root, the directory D_ROOT/<owner>/target and nothing else
  (D_ROOT resolved as above, never written out; <owner>/tmp and the rest of <owner> stay). Same path rules as C: (equal
  after normalisation, no link on the way or inside), never studio-round, studio-team-dev, an MT-* folder, the cargo
  registry or cargo-home. The owner needs a ledger lane row with status PASS_*, SUPERSEDED, LOST or DONE (no 'no ledger
  row' basis on D:), and no lane outside that set (READY, active, blocked or paused) may name the owner tree or the tree
  bound in .studio-workspace-root; a bound workspace outside wt-studio-lanes must no longer exist. Process, newest-file,
  5% size and real-target (.studio-workspace-root, debug/.fingerprint) checks are the C: ones.
  For every whole candidate, C: and D:: no lane may name the bound tree (as for the owner tree), no running process may
  name the bound tree, no held lock or queued ticket under the Studio lock roots (D_ROOT, the C: Studio root and its
  parent: .disk-build-lock, .gpu-test-lock; STUDIO_DISK_BUILD_LOCK_DIR) may name the target, owner, bound tree or bound
  workspace (SKIPPED_LOCK; an unreadable held lock counts), and no round may hold the disk build lock of the target's
  drive (plan W before_execution). Paused lanes in ledger direction.pause_* records count as naming their tree.
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
STEWARD_CANARY = D_ROOT + "/studio-team-dev/steward-export/target"  # N3 (a)
LANES_ROOT = os.path.join(WORKTREES, "wt-studio-lanes").replace("\\", "/")
C_STUDIO = C_LANES.split("/studio-team-dev/")[0]
LOCK_PARENTS = (D_ROOT, C_STUDIO, os.path.dirname(C_STUDIO))  # N4: the lock roots .disk-build-lock and .gpu-test-lock
NOT_D_OWNER = ("studio-round", "studio-team-dev", "cargo-home", "registry")  # N4


def norm(p):
    return os.path.normcase(os.path.normpath(p))


def msys(text):
    """Lowercase, / separators, MSYS /d/... paths as d:/... (lock files and Git Bash command lines use that form)."""
    return re.sub(r"(?<![a-z0-9_.:-])/([a-z])/", r"\1:/", text.replace("\r", "").replace("\\", "/").lower())


def named(text, name):
    return re.search(r"(?<![a-z0-9_.-])" + re.escape(name.lower()) + r"(?![a-z0-9_.-])", text) is not None


def on_d_root(p):
    return norm(p).startswith(norm(D_ROOT) + os.sep)


def steward_canary(p):
    """N3 (a): STEWARD_CANARY when p is exactly that path after normalisation and nothing on the way is a link, else None."""
    if norm(p) != norm(STEWARD_CANARY) or norm(os.path.realpath(STEWARD_CANARY)) != norm(STEWARD_CANARY):
        return None
    parts = STEWARD_CANARY.split("/")
    for i in range(1, len(parts) + 1):
        q = "/".join(parts[:i]) + ("/" if i == 1 else "")
        if os.path.lexists(q) and is_link(os.lstat(q)):
            return None
    return STEWARD_CANARY


def allowed_path(p, older_d=False):
    p = p.replace("\\", "/")
    if p == C_ROUND or re.fullmatch(re.escape(C_LANES) + r"/[A-Za-z0-9_.-]+", p):
        return True
    if older_d and steward_canary(p):  # N3 (a)
        return True
    # N3 (a): nothing else under studio-team-dev on D: (D_ROOT/studio-team-dev/target is not a lane target)
    return re.fullmatch(re.escape(D_ROOT) + r"/(?!(?i:studio-team-dev)/)[A-Za-z0-9_.-]+/target", p) is not None


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
    """The one directory a whole candidate may remove, C_LANES/<owner> or (N4) D_ROOT/<owner>/target, or None when the
    plan path is anything else."""
    if not OWNER.fullmatch(owner) or ".." in owner:
        return None
    if on_d_root(path):  # N4
        if owner.lower() in NOT_D_OWNER or owner.lower().startswith("mt-"):
            return None
        target = D_ROOT + "/" + owner + "/target"
        parts = target.split("/")
        if any(x.lower() in NEVER_UNDER + NOT_D_OWNER for x in parts):
            return None
    else:
        target = C_LANES + "/" + owner
        parts = target.split("/")
        if parts[0].upper() != "C:" or any(x.lower() in NEVER_UNDER for x in parts):
            return None
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


def settled_n4(status):
    # N4 (plan W group c): a D: owner lane, or a lane that names a D: candidate's tree, is settled at PASS_*,
    # SUPERSEDED, LOST or DONE; READY_FOR_VALIDATION, unset (active), BLOCKED, PAUSED and anything else is not.
    s = str(status or "").upper()
    return s.startswith("PASS_") or s in ("SUPERSEDED", "LOST", "DONE")


def lane_tree(bound):
    """The wt-studio-lanes tree a .studio-workspace-root path lies in, or None when it lies elsewhere."""
    p = str(bound or "").replace("\\", "/")
    if not os.path.isabs(p) or not norm(p).startswith(norm(LANES_ROOT) + os.sep):
        return None
    tree = os.path.relpath(p, LANES_ROOT).replace("\\", "/").split("/")[0]
    return tree if OWNER.fullmatch(tree.lower()) else None


def lock_files():
    """N4: [(lock root, 'held' or 'queue', text)] for each held lock and queued ticket under the Studio lock roots; text
    as msys() gives it, None for a held lock whose owner file cannot be read."""
    roots = {r.replace("\\", "/") for d in LOCK_PARENTS for r in glob.glob(os.path.join(d, ".*-lock")) if os.path.isdir(r)}
    if os.environ.get("STUDIO_DISK_BUILD_LOCK_DIR"):
        roots.add(os.environ["STUDIO_DISK_BUILD_LOCK_DIR"].replace("\\", "/"))
    out = []
    for r in sorted(roots):
        files = [(r + "/held/owner", "held")] if os.path.lexists(r + "/held") else []
        files += [(f, "queue") for f in sorted(glob.glob(os.path.join(r, "queue", "*")))]
        for f, kind in files:
            try:
                with open(f, encoding="utf-8", errors="replace") as h:
                    out.append((r, kind, msys(h.read())))
            except OSError:
                if kind == "held" and os.path.lexists(r + "/held"):
                    out.append((r, kind, None))
    return out


def lock_hits(locks, paths, names, drive):
    """N4: why a held lock or queued ticket stops a whole candidate: it names one of paths or names, a held owner file
    is unreadable, or a round holds the disk build lock of the target's drive. [] when none."""
    hits = []
    for root, kind, text in locks:
        if text is None:
            hits.append(f"{root} held: owner file unreadable")
            continue
        said = [p for p in paths if p and msys(p) in text] + [n for n in names if n and named(text, n)]
        if said:
            hits.append(f"{root} {kind} ({' '.join(re.findall(r'^(?:class|owner)=.*$', text, re.M))}) names {', '.join(said)}")
        elif (kind == "held" and os.path.basename(root) == ".disk-build-lock" and re.search(r"^class=round$", text, re.M)
              and os.path.splitdrive(root)[0].lower() == drive):
            hits.append(f"{root} held by a round on drive {drive}")
    return hits


def tree_of(field):
    """The tree a ledger or lane-file field names: 'wt-studio-lanes/<tree> ...', '<tree> (...)' or a path ending in it."""
    t = str(field or "").replace("\\", "/").strip()
    m = re.search(r"wt-studio-lanes/([A-Za-z0-9_.-]+)", t) or re.match(r"([A-Za-z0-9_.-]+)(?:\s|$)", t)
    return m.group(1) if m else os.path.basename(t.rstrip("/")) or None


def tree_users(led, statuses, settled=settled_whole):
    """{tree: lane ids} of lanes short of settled (C: PASS_V or ABANDONED; N4 settled_n4) that name the tree in the
    ledger (target_reuse, lane rows, N4: direction.pause_* lanes) or their lane file."""
    users = {}

    def add(tree, lane, status):
        if tree and not settled(status):
            users.setdefault(tree, set()).add(str(lane))

    for k, v in led["direction"].get("workflow_20261107", {}).get("target_reuse", {}).items():
        for field in ("tree", "old_tree_idle"):
            add(tree_of(v.get(field)), k, statuses.get(k.lower()))
    for key, rec in led["direction"].items():  # N4: a paused lane names the tree it was paused on
        lanes = rec.get("lanes") if key.startswith("pause_") and isinstance(rec, dict) else None
        for k, v in (lanes.items() if isinstance(lanes, dict) else ()):
            if isinstance(v, dict) and v.get("tree"):
                add(tree_of(v["tree"]), k, statuses.get(str(k).lower()))
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


def whole_basis(c, statuses, bound, d_root=False):
    """Why the owner lane is settled for whole removal, or None. N4 (d_root): only a ledger row in settled_n4."""
    no_row = str(c.get("status", "")).startswith("no ledger row")
    if d_root:
        s = statuses.get(c["owner"].lower())
        return str(s) if c["owner"].lower() in statuses and settled_n4(s) and not no_row else None
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
    return msys(out.stdout)


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

    in_use = {}  # tree: unsettled lanes that name it in target_reuse
    for k, v in led["direction"].get("workflow_20261107", {}).get("target_reuse", {}).items():
        if not settled(statuses.get(k.lower())):
            in_use.setdefault(v["tree"], set()).add(k)
    paused = {str(k).lower(): v for k, v in led["direction"].get("pause_20261111", {}).get("lanes", {}).items()}  # N3 (b)
    wants_whole = any(c["id"] in approved and c.get("kind") == "whole" for c in plan["candidates"])
    users = tree_users(led, statuses) if wants_whole else {}
    users_c = {k.lower(): v for k, v in users.items()}
    users_d = {k.lower(): v for k, v in tree_users(led, statuses, settled_n4).items()} if wants_whole else {}  # N4
    locks = lock_files() if wants_whole else []
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
        older_d = c.get("kind") == "partial" and c.get("select") == "older_copies" and path.startswith(D_ROOT + "/")  # N3
        canary = older_d and steward_canary(path) is not None
        if canary:
            path = STEWARD_CANARY
        d_whole = whole and on_d_root(path)  # N4
        if whole:
            row["kind"] = "whole"
            if d_whole:
                row["n4"] = True
            path = whole_path(c["owner"], path)
            if path is None:
                row["status"] = "REFUSED_PATH"
                continue
        elif c.get("kind") != "partial" or not allowed_path(path, older_d):
            row["status"] = "REFUSED_PATH"
            continue
        if not os.path.isfile(os.path.join(path, ".studio-workspace-root")) or not os.path.isdir(os.path.join(path, "debug", ".fingerprint")):
            row["status"] = "REFUSED_NOT_A_TARGET"
            continue
        if whole:
            bound = open(os.path.join(path, ".studio-workspace-root"), encoding="utf-8").read().strip()
            row["bound_to"] = bound
            if d_whole and c["owner"].lower() in users_d:  # N4: lanes outside settled_n4
                row.update(status="SKIPPED_IN_USE_BY_LANE", used_by=sorted(users_d[c["owner"].lower()]))
                continue
            if not d_whole and c["owner"] in users:
                row.update(status="SKIPPED_IN_USE_BY_LANE", used_by=sorted(users[c["owner"]]))
                continue
            bt = lane_tree(bound)  # N4, every whole candidate: the tree bound in .studio-workspace-root
            bt_users = (users_d if d_whole else users_c).get(bt.lower()) if bt else None
            if bt_users:
                row.update(status="SKIPPED_IN_USE_BY_LANE", used_by=sorted(bt_users), via_bound_tree=bt)
                continue
            tree = os.path.dirname(bound) if os.path.isabs(bound) else ""
            if d_whole and not bt and (not tree or os.path.lexists(tree)):  # N4: a bound workspace outside the lane trees
                row.update(status="SKIPPED_BOUND_TREE", bound_tree_exists=os.path.lexists(tree) if tree else None)
                continue
            basis = whole_basis(c, statuses, bound, d_whole)
            if basis is None:
                row.update(status="SKIPPED_STATUS", lane_status=statuses.get(c["owner"].lower() if d_whole else c["owner"], "no ledger row"))
                if not d_whole and c["owner"] not in statuses:
                    row["bound_tree_exists"] = os.path.lexists(tree) if tree else None
                continue
            row["settled_by"] = basis
            tree = msys(tree)
            names = [c["owner"]] + ([bt] if bt and bt.lower() != c["owner"].lower() else [])
            if path.lower() in procs or (tree and tree in procs) or any(named(procs, n) for n in names):
                row["status"] = "SKIPPED_PROCESS"
                continue
            hits = lock_hits(locks, [path, tree], names, os.path.splitdrive(path)[0].lower())  # N4
            if hits:
                row.update(status="SKIPPED_LOCK", locks=hits)
                continue
        else:
            if c["owner"] in in_use:
                lanes = sorted(in_use[c["owner"]])
                not_paused = [x for x in lanes if not (older_d and isinstance(paused.get(x.lower()), dict) and paused[x.lower()].get("tree") == c["owner"])]
                if not_paused:
                    row["status"] = "SKIPPED_IN_USE_BY_LANE"
                    if older_d:
                        row["not_paused"] = not_paused
                    continue
                row["accepted_paused"] = lanes  # N3 (b): every unsettled lane on this tree is paused on it
            if c["owner"] != "studio-round" and not canary and not settled(statuses.get(c["owner"], "")):
                row["status"] = "SKIPPED_STATUS"
                continue
            if path.lower() in procs:
                row["status"] = "SKIPPED_PROCESS"
                continue
            if canary or "accepted_paused" in row:  # N3: no process may name the tree (owner) or the bound workspace either
                ws = os.path.dirname(open(os.path.join(path, ".studio-workspace-root"), encoding="utf-8").read().strip())
                ws = ws.replace("\\", "/").lower()
                if (ws and ws in procs) or re.search(r"(?<![a-z0-9_.-])" + re.escape(c["owner"].lower()) + r"(?![a-z0-9_.-])", procs):
                    row["status"] = "SKIPPED_PROCESS"
                    continue
        seen = newest(path)
        if older_d and not c.get("newest_file_utc") and c.get("gib"):  # N3: no newest in the plan, size check only
            row.update(newest_now=seen, newest_check="size only (plan has no newest_file_utc)")
        elif seen != c.get("newest_file_utc"):
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
            files, dirs, dir_bytes = select(dict(c, path=path) if canary else c)
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
