"""Read-only inspection of Studio lane cargo targets (HSK-ART-005, Operator 2026-11-07). It deletes nothing.

Usage: python target_report.py [--disk c|d|all] [--out FILE] [owner ...]
For each lane target it reports size by part, the crates built there, the tree it is bound to, the lane's status,
whether the pair can be reused by a new lane, and what a partial removal would cost. A line here is a proposal for the
Operator, never an approval.
"""
import argparse
import collections
import datetime
import json
import os
import re
import subprocess

S = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
KERNEL = os.path.abspath(os.path.join(S, "..", "..", ".."))
WORKTREES = os.path.dirname(KERNEL)
LANES = os.path.join(WORKTREES, "wt-studio-lanes")
ART = os.environ.get("HANDSHAKE_ARTIFACTS_ROOT") or os.path.join(WORKTREES, "Handshake_Artifacts")
D_ROOT = os.path.join(ART, "WP-KERNEL-STUDIO")
C_ROOT = "C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-team-dev/target/lanes"
HASHED = re.compile(r"^(?:lib)?(?P<name>.+?)-(?P<hash>[0-9a-f]{16})(?:\.(?P<ext>.+))?$")
WORKSPACE_CRATE = re.compile(r"^(hsk_studio|hsk_)")
GIB = 1024.0 ** 3


def walk_size(path):
    total, newest = 0, 0.0
    stack = [path]
    while stack:
        try:
            with os.scandir(stack.pop()) as it:
                for e in it:
                    try:
                        if e.is_dir(follow_symlinks=False):
                            stack.append(e.path)
                        else:
                            st = e.stat(follow_symlinks=False)
                            total += st.st_size
                            newest = max(newest, st.st_mtime)
                    except OSError:
                        pass
        except OSError:
            pass
    return total, newest


def inspect_deps(deps):
    """Split a deps folder into third-party libraries, workspace libraries and test or binary outputs."""
    parts = collections.Counter()
    groups = collections.defaultdict(list)
    crates = set()
    newest = 0.0
    try:
        entries = list(os.scandir(deps))
    except OSError:
        return parts, 0, crates, newest
    for e in entries:
        try:
            if e.is_dir(follow_symlinks=False):
                continue
            st = e.stat(follow_symlinks=False)
        except OSError:
            continue
        newest = max(newest, st.st_mtime)
        m = HASHED.match(e.name)
        name = m.group("name") if m else e.name
        ext = (m.group("ext") if m else "") or ""
        if ext in ("exe", "pdb", "dll", "dll.lib", "dll.exp", "lib", "exp"):
            part = "test_and_binary_outputs"
            groups[(name, ext)].append((st.st_mtime, st.st_size))
        elif WORKSPACE_CRATE.match(name):
            part = "workspace_libraries"
            if ext == "rlib":
                crates.add(name)
        elif ext in ("rlib", "rmeta"):
            part = "third_party_libraries"
        else:
            part = "dep_info_and_other"
        parts[part] += st.st_size
    older = sum(s for rows in groups.values() for _, s in sorted(rows)[:-1])
    return parts, older, crates, newest


def inspect_target(owner, path, disk, statuses, reused):
    row = {"owner": owner, "disk": disk, "path": path}
    parts = collections.Counter()
    newest = 0.0
    older, crates = 0, set()
    try:
        tops = list(os.scandir(path))
    except OSError as err:
        row["error"] = str(err)
        return row
    for top in tops:
        if not top.is_dir(follow_symlinks=False):
            continue
        if top.name not in ("debug", "release"):
            size, new = walk_size(top.path)
            parts["other:" + top.name] += size
            newest = max(newest, new)
            continue
        for sub in os.scandir(top.path):
            if sub.is_dir(follow_symlinks=False) and sub.name == "deps":
                got, old, names, new = inspect_deps(sub.path)
                parts.update(got)
                older += old
                crates |= names
                newest = max(newest, new)
            elif sub.is_dir(follow_symlinks=False):
                label = {"build": "build_scripts", ".fingerprint": "fingerprints", "incremental": "incremental"}.get(sub.name, "other:" + sub.name)
                size, new = walk_size(sub.path)
                parts[label] += size
                newest = max(newest, new)
            else:
                try:
                    st = sub.stat(follow_symlinks=False)
                    parts["test_and_binary_outputs"] += st.st_size
                    newest = max(newest, st.st_mtime)
                except OSError:
                    pass
    bind = os.path.join(path, ".studio-workspace-root")
    row["bound_to"] = open(bind, encoding="utf-8").read().strip() if os.path.isfile(bind) else None
    row["gib"] = round(sum(parts.values()) / GIB, 2)
    row["parts_gib"] = {k: round(v / GIB, 2) for k, v in sorted(parts.items(), key=lambda kv: -kv[1])}
    row["older_binary_variants_gib"] = round(older / GIB, 2)
    row["newest_file_utc"] = datetime.datetime.fromtimestamp(newest, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%MZ") if newest else None
    row["workspace_crates_built"] = sorted(c.replace("hsk_studio_", "") for c in crates)
    row["lane_status"] = statuses.get(owner, "no ledger row")
    tree = os.path.join(LANES, owner)
    row["tree_exists"] = os.path.isdir(tree)
    clean = None
    if row["tree_exists"]:
        out = subprocess.run(["git", "-C", tree, "status", "--porcelain"], capture_output=True, text=True)
        clean = out.returncode == 0 and not out.stdout.strip()
        row["tree_branch"] = subprocess.run(["git", "-C", tree, "branch", "--show-current"], capture_output=True, text=True).stdout.strip()
        row["tree_head"] = subprocess.run(["git", "-C", tree, "rev-parse", "--short=8", "HEAD"], capture_output=True, text=True).stdout.strip()
    row["tree_clean"] = clean
    done = str(row["lane_status"]).startswith("PASS_V")
    in_use = reused.get(owner)
    row["in_use_by"] = in_use
    row["reuse_candidate"] = bool(done and clean and not in_use)
    row["proposal"] = (
        f"in use by {in_use}: keep" if in_use
        else "live: keep" if not done and row["lane_status"] != "no ledger row"
        else "reuse with its tree" if row["reuse_candidate"]
        else "inspect by hand: no reusable tree"
    )
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--disk", choices=("c", "d", "all"), default="all")
    ap.add_argument("--out")
    ap.add_argument("owners", nargs="*")
    a = ap.parse_args()
    led = json.load(open(os.path.join(S, "ledger.json"), encoding="utf-8"))
    statuses = {str(x.get("lane_id", "")).lower(): x.get("status") for x in led["lanes"]}
    reuse = led["direction"].get("workflow_20261107", {}).get("target_reuse", {})
    reused = {v["tree"]: k for k, v in reuse.items() if not str(statuses.get(k.lower(), "")).startswith("PASS_V")}
    wanted = []
    if a.disk in ("c", "all") and os.path.isdir(C_ROOT):
        wanted += [(n, os.path.join(C_ROOT, n), "C") for n in sorted(os.listdir(C_ROOT)) if os.path.isdir(os.path.join(C_ROOT, n))]
    if a.disk in ("d", "all") and os.path.isdir(D_ROOT):
        wanted += [(n, os.path.join(D_ROOT, n, "target"), "D") for n in sorted(os.listdir(D_ROOT)) if os.path.isdir(os.path.join(D_ROOT, n, "target"))]
    if a.owners:
        wanted = [w for w in wanted if w[0] in a.owners]
    rows = [inspect_target(o, p, d, statuses, reused) for o, p, d in wanted]
    report = {
        "schema": "hsk.studio.target_inspection@1",
        "generated_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "note": "Read-only. Nothing was deleted. A proposal is not an approval (HSK-ART-005, CX-107).",
        "total_gib": {d: round(sum(r.get("gib", 0) for r in rows if r["disk"] == d), 2) for d in ("C", "D")},
        "targets": rows,
    }
    if a.out:
        with open(a.out, "w", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps(report, indent=1, ensure_ascii=False) + "\n")
    print("disk owner gib third_party workspace binaries older_bin status reuse crates")
    for r in sorted(rows, key=lambda x: -x.get("gib", 0)):
        p = r.get("parts_gib", {})
        print(r["disk"], r["owner"], r.get("gib"), p.get("third_party_libraries", 0), p.get("workspace_libraries", 0), p.get("test_and_binary_outputs", 0), r.get("older_binary_variants_gib"), r.get("lane_status"), "Y" if r.get("reuse_candidate") else "-", ",".join(r.get("workspace_crates_built", []))[:90])
    print("total_gib", report["total_gib"])


if __name__ == "__main__":
    main()
