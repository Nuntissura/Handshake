"""Capture and compare per-file metadata around a WPV export refresh.

The snapshots live in the owner log directory, never inside export-current.
This is a fail-closed guard for source-path/timestamp preservation; byte-level
candidate verification remains the responsibility of WPV-export-verify.py.
"""
import argparse
import datetime as dt
import json
import os
import subprocess
import sys
import time


def git_bytes(wt, *args):
    return subprocess.run(["git", "-C", wt, *args], check=True, capture_output=True).stdout


def tracked_paths(wt, sha):
    if sha == "none":
        return {}
    entries = git_bytes(wt, "ls-tree", "-r", "-z", sha).split(b"\0")
    result = {}
    for entry in entries:
        if not entry:
            continue
        meta, raw_path = entry.split(b"\t", 1)
        mode, kind, oid = meta.split(b" ")
        result[raw_path.decode("utf-8")] = {"mode": mode.decode(), "kind": kind.decode(), "oid": oid.decode()}
    return result


def excluded(rel):
    return rel == ".git" or "/target/nextest/" in f"/{rel}/" or rel.endswith("/target/nextest")


def observed_files(root):
    if os.name == "nt":
        root = os.path.abspath(root)
        if not root.startswith("\\\\?\\"):
            root = "\\\\?\\UNC\\" + root[2:] if root.startswith("\\\\") else "\\\\?\\" + root
    found = {}
    if not os.path.isdir(root):
        return found
    for walk_root, dirs, files in os.walk(root, followlinks=False):
        rel_root = os.path.relpath(walk_root, root).replace(os.sep, "/")
        if rel_root != ".":
            dirs[:] = [d for d in dirs if not excluded(f"{rel_root}/{d}")]
        for name in files:
            rel = name if rel_root == "." else f"{rel_root}/{name}"
            if excluded(rel):
                continue
            full = os.path.join(root, rel.replace("/", os.sep))
            st = os.lstat(full)
            found[rel] = {
                "mtime_ns": st.st_mtime_ns,
                "size": st.st_size,
                "kind": "symlink" if os.path.islink(full) else "file",
            }
    return found


def utc_now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z")


def write_json(path, data):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(data, fh, sort_keys=True, separators=(",", ":"))
        fh.write("\n")
    os.replace(tmp, path)


def snapshot(args):
    tracked = tracked_paths(args.worktree, args.sha)
    data = {
        "schema": "handshake.wpv.export-file-metadata@1",
        "candidate_sha": args.sha,
        "captured_utc": utc_now(),
        "captured_ns": time.time_ns(),
        "round_instance": args.instance,
        "tracked": tracked,
        "observed": observed_files(args.export),
    }
    write_json(args.output, data)
    print(f"METADATA_SNAPSHOT_OK sha={args.sha} tracked={len(tracked)} observed={len(data['observed'])} file={args.output}")


def load_snapshot(path):
    with open(path, encoding="utf-8") as fh:
        data = json.load(fh)
    if data.get("schema") != "handshake.wpv.export-file-metadata@1":
        raise ValueError(f"unsupported snapshot schema: {path}")
    return data


def compare(args):
    before, after = load_snapshot(args.before), load_snapshot(args.after)
    if after["candidate_sha"] != args.candidate:
        raise ValueError("after snapshot candidate does not match requested candidate")
    old_tracked = before["tracked"]
    new_tracked = after["tracked"]
    old_paths, new_paths = set(old_tracked), set(new_tracked)
    classes = {}
    if args.mode == "full":
        for path in sorted(old_paths - new_paths):
            classes[path] = "D"
        for path in sorted(new_paths):
            classes[path] = "full"
    elif args.mode == "incremental":
        if not args.previous or before["candidate_sha"] != args.previous:
            raise ValueError("incremental before snapshot is not bound to previous SHA")
        raw = git_bytes(args.worktree, "diff", "--name-status", "-z", "--no-renames", args.previous, args.candidate)
        parts = raw.split(b"\0")
        i = 0
        changed = {}
        while i < len(parts) and parts[i]:
            status = parts[i].decode("ascii")
            i += 1
            if status not in {"A", "M", "D", "T"} or i >= len(parts):
                raise ValueError("unsupported git diff status framing")
            path = parts[i].decode("utf-8")
            i += 1
            changed[path] = status
        for path in sorted(old_paths | new_paths):
            classes[path] = changed.get(path, "unchanged")
        if set(changed) != (old_paths ^ new_paths) | {p for p in old_paths & new_paths if old_tracked[p] != new_tracked[p]}:
            raise ValueError("git diff path set does not reconcile to tracked manifest delta")
    else:
        raise ValueError(f"unknown refresh mode: {args.mode}")

    before_obs, after_obs = before["observed"], after["observed"]
    failures = []
    before_tracked, after_tracked = set(old_paths), set(new_paths)
    if args.mode == "incremental":
        for path in sorted(before_tracked - set(before_obs)):
            failures.append({"path": path, "reason": "missing_before"})
        for path in sorted(set(before_obs) - before_tracked):
            failures.append({"path": path, "reason": "extra_before"})
    for path in sorted(after_tracked - set(after_obs)):
        failures.append({"path": path, "reason": "missing_after"})
    for path in sorted(set(after_obs) - after_tracked):
        failures.append({"path": path, "reason": "extra_after"})

    start_ns, end_ns = int(before["captured_ns"]), int(after["captured_ns"])
    if end_ns < start_ns:
        raise ValueError("refresh end precedes start")
    for path, category in classes.items():
        if category == "D":
            if path in after_obs:
                failures.append({"path": path, "reason": "deleted_path_present_after"})
        elif category == "unchanged":
            old, new = before_obs.get(path), after_obs.get(path)
            if old and new and old != new:
                failures.append({"path": path, "reason": "unchanged_metadata_changed"})
        elif category in {"A", "M", "T", "full"}:
            new = after_obs.get(path)
            if new and not (start_ns <= int(new["mtime_ns"]) <= end_ns):
                failures.append({"path": path, "reason": "changed_path_mtime_outside_refresh"})

    report = {
        "schema": "handshake.wpv.export-metadata-comparison@1",
        "candidate_sha": args.candidate,
        "previous_sha": args.previous or "none",
        "refresh_mode": args.mode,
        "refresh_started_ns": start_ns,
        "refresh_ended_ns": end_ns,
        "classifications": dict(sorted(classes.items())),
        "classification_counts": {k: sum(1 for v in classes.values() if v == k) for k in ("A", "M", "D", "T", "unchanged", "full")},
        "failures": failures,
        "result": "PASS" if not failures else "FAIL",
    }
    write_json(args.output, report)
    print(f"METADATA_COMPARE_{report['result']} sha={args.candidate} mode={args.mode} classified={len(classes)} failures={len(failures)} file={args.output}")
    for item in failures[:20]:
        print(f"METADATA_MISMATCH path={item['path']} reason={item['reason']}")
    if failures:
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    snap = sub.add_parser("snapshot")
    snap.add_argument("--worktree", required=True)
    snap.add_argument("--sha", required=True, help="tracked-manifest SHA or 'none' when no prior commit is valid")
    snap.add_argument("--instance", required=True)
    snap.add_argument("--export", required=True)
    snap.add_argument("--output", required=True)
    snap.set_defaults(func=snapshot)
    cmp = sub.add_parser("compare")
    for name in ("worktree", "candidate", "before", "after", "output", "mode"):
        cmp.add_argument(f"--{name}", required=True)
    cmp.add_argument("--previous", default="")
    cmp.set_defaults(func=compare)
    args = parser.parse_args()
    try:
        args.func(args)
    except (OSError, subprocess.CalledProcessError, ValueError, KeyError) as exc:
        print(f"METADATA_FATAL {type(exc).__name__}: {exc}", file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
