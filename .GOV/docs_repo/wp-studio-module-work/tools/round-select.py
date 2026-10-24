"""Scoped round selection (SMB-STEWARD-ROUND-SELECT; workflow v3 WF3-006, Codex CX-VAL-011 / CX-VAL-002).

Selection for a round = workspace crates changed since the last finalized round's candidate, plus their reverse-
dependency closure (any dependency kind: normal, build, dev), from `cargo metadata` of the candidate tree. Crates
outside it reuse their last PASS proof (CX-VAL-002): one reuse record per crate naming both commits.
A full-workspace union (`--all`) is due when any of these holds:
  - every third round: --rounds-since-full >= 2 (the caller counts finalized rounds since the last full union);
  - the toolchain changed (rust-toolchain.toml);
  - the Cargo.lock dependency set changed (external [[package]] name/version/source set; workspace members excluded);
  - the round entry script, the tools it runs or its configuration changed: every tools/* and .config/* file EXCEPT
    the step-0 policy overlays and the non-round tools below. Full-round triggers today: tools/run-round.sh,
    tools/disk-build-lock.sh, tools/registry-sync.py (step 0 check), .config/nextest.toml, .config/round-features.toml,
    .config/oracle-corpus.toml, .config/oracle-fixtures.toml, any file under .cargo/ (cargo build configuration,
    e.g. .cargo/config.toml jobs/flags: changes every build; R40 finding, LOW), and any NEW tools/ or .config/ file
    (safe default);
  - root Cargo.toml changed outside [workspace.metadata] (profiles, workspace dependencies, members are handled as
    crate changes only when nothing else changed).
Step-0 policy overlays (lead decision 2026-10-22, F-R37-2; NOT round-entry inputs, they select nothing): exactly
  .config/duplicate-fingerprints.toml (V4-05 frozen-duplicate patterns + allowed sites, read by run-round.sh step 0),
  .config/machine-paths.toml (V4-03 machine-local path [pinned] allowances, read by run-round.sh step 0),
  .config/disk-build-lock.toml (disk-wide build lock root marker; lock protocol stays in tools/disk-build-lock.sh).
  The steward repoints the first two in sync commits (steward-sync.sh --overlay .config/*.toml) when lanes move or
  split files; step 0 runs over the whole workspace in every round regardless of the selection, so a repoint is
  re-checked without a full union. They are listed in the record's "policy_overlays_changed".
Non-round tools (never executed or read by tools/run-round.sh; listed in "non_entry_tools_changed"): exactly
  tools/workspace-probe.rs (steward-sync probe, compiled standalone by the steward, not a workspace member),
  tools/camera-matrix-table.py (generator; its output crates/hsk-studio-develop/src/camera_table.rs is crate source
  and selects develop by itself).
Usage:
  python -X utf8 round-select.py --repo GITDIR --base SHA --candidate SHA --metadata-dir DIR --round R37 --base-round R36
      [--rounds-since-full K] [--sub handshake-studio-assets] [--out FILE.json]
    DIR must hold the candidate's workspace (e.g. the steward export refreshed to SHA); `cargo metadata --no-deps
    --offline` runs there. Prints `SELECTION: --all` or `SELECTION: hsk-studio-a hsk-studio-b ...` (the round entry
    script's arguments) and writes the JSON record (selection, reasons, reuse records) to --out.
  python -X utf8 round-select.py --self-test
Exit: 0 ok; 2 usage/inputs; 3 metadata or git failure."""
import argparse, json, os, subprocess, sys, tomllib

ENTRY = ("tools/run-round.sh", "tools/disk-build-lock.sh")
ENTRY_PREFIXES = (".config/", "tools/", ".cargo/")  # .cargo/: cargo build config (jobs, flags) changes every build (R40 LOW)
# Step-0 policy overlays: excluded from ENTRY_PREFIXES (see the header; lead decision F-R37-2).
POLICY_OVERLAYS = frozenset((".config/duplicate-fingerprints.toml", ".config/machine-paths.toml",
                             ".config/disk-build-lock.toml"))
NON_ROUND_TOOLS = frozenset(("tools/workspace-probe.rs", "tools/camera-matrix-table.py"))


def lock_deps(text):
    """External dependency set of a Cargo.lock: {(name, version, source)} for packages with a source."""
    if not text:
        return set()
    data = tomllib.loads(text)
    return {(p["name"], p["version"], p["source"]) for p in data.get("package", []) if p.get("source")}


def root_changed_outside_metadata(old_text, new_text):
    if old_text == new_text:
        return False
    try:
        old, new = tomllib.loads(old_text or ""), tomllib.loads(new_text or "")
    except tomllib.TOMLDecodeError:
        return True
    for doc in (old, new):
        ws = doc.get("workspace", {})
        ws.pop("metadata", None)
        ws.pop("members", None)   # member changes show up as crate-dir changes
    return old != new


def decide(changed, packages, rounds_since_full, lock_old, lock_new, root_old, root_new, base, cand, rnd, base_round):
    """Pure decision. changed: workspace-relative paths; packages: [{name, dir, deps:[member names]}] for members."""
    reasons = []
    if rounds_since_full >= 2:
        reasons.append(f"every third round: {rounds_since_full} rounds since the last full union")
    if "rust-toolchain.toml" in changed:
        reasons.append("toolchain changed (rust-toolchain.toml)")
    if lock_deps(lock_old) != lock_deps(lock_new):
        added = sorted(f"{n}@{v}" for n, v, _ in lock_deps(lock_new) - lock_deps(lock_old))
        removed = sorted(f"{n}@{v}" for n, v, _ in lock_deps(lock_old) - lock_deps(lock_new))
        reasons.append(f"Cargo.lock dependency set changed (added {added[:8]}, removed {removed[:8]})")
    policy = sorted(p for p in changed if p in POLICY_OVERLAYS)
    tools_skip = sorted(p for p in changed if p in NON_ROUND_TOOLS)
    entry = sorted(p for p in changed if (p in ENTRY or p.startswith(ENTRY_PREFIXES))
                   and p not in POLICY_OVERLAYS and p not in NON_ROUND_TOOLS)
    if entry:
        reasons.append(f"round entry script or its configuration changed: {entry}")
    if "Cargo.toml" in changed and root_changed_outside_metadata(root_old, root_new):
        reasons.append("root Cargo.toml changed outside [workspace.metadata] / members")
    names = sorted(p["name"] for p in packages)
    by_dir = {p["dir"].rstrip("/") + "/": p["name"] for p in packages}
    direct = sorted({by_dir[d] for path in changed for d in by_dir if path.startswith(d)})
    rev = {n: set() for n in names}
    for p in packages:
        for d in p["deps"]:
            if d in rev:
                rev[d].add(p["name"])
    closure, todo = set(direct), list(direct)
    while todo:
        for r in rev.get(todo.pop(), ()):
            if r not in closure:
                closure.add(r)
                todo.append(r)
    full = bool(reasons)
    selected = names if full else sorted(closure)
    reuse = [] if full else [
        {"crate": n, "reuses_round": base_round, "proof_commit": base, "candidate_commit": cand,
         "basis": "CX-VAL-002: no file of the crate and none of its workspace dependencies changed between the commits"}
        for n in names if n not in closure]
    return {"schema": "hsk.studio.round-selection@1", "round": rnd, "base_round": base_round, "base": base,
            "candidate": cand, "full": full, "full_reasons": reasons, "policy_overlays_changed": policy,
            "non_entry_tools_changed": tools_skip, "changed_crates": direct,
            "selected": selected, "selection_args": ["--all"] if full else selected, "reuse": reuse}


def git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True, encoding="utf-8", check=True).stdout


def self_test():
    pk = [{"name": "a", "dir": "crates/a", "deps": []}, {"name": "b", "dir": "crates/b", "deps": ["a"]},
          {"name": "c", "dir": "crates/c", "deps": []}, {"name": "d", "dir": "crates/d", "deps": ["b"]},
          {"name": "e", "dir": "crates/e", "deps": ["c"]}]
    lock1 = '[[package]]\nname = "x"\nversion = "1.0.0"\nsource = "registry+r"\n\n[[package]]\nname = "a"\nversion = "0.1.0"\n'
    lock2 = lock1.replace('"1.0.0"', '"1.1.0"')
    root = '[workspace]\nmembers = ["crates/a"]\n[workspace.metadata.studio]\nx = 1\n'
    run = lambda changed, k=0, l2=lock1, r2=root: decide(changed, pk, k, lock1, l2, root, r2, "B", "C", "R37", "R36")
    checks = []
    ov0 = [".config/duplicate-fingerprints.toml", ".config/machine-paths.toml"]
    r = run(["crates/a/src/lib.rs"])
    checks.append(("leaf change selects reverse closure", r["selected"] == ["a", "b", "d"] and not r["full"]))
    checks.append(("reuse records name both commits", {x["crate"] for x in r["reuse"]} == {"c", "e"}
                   and all(x["proof_commit"] == "B" and x["candidate_commit"] == "C" for x in r["reuse"])))
    r = run(["crates/c/tests/t.rs", "crates/c/Cargo.toml"])
    checks.append(("dev/test change selects its dependents", r["selected"] == ["c", "e"]))
    checks.append(("lock dependency set change -> full", run(["Cargo.lock"], l2=lock2)["full"]))
    checks.append(("member-only lock change -> not full",
                   not run(["Cargo.lock"], l2=lock1 + '\n[[package]]\nname = "f"\nversion = "0.1.0"\n')["full"]))
    checks.append(("entry script change -> full", run(["tools/run-round.sh"])["selection_args"] == ["--all"]))
    checks.append(("round config change -> full", run([".config/nextest.toml"])["full"]))
    checks.append((".cargo/config.toml change -> --all", run([".cargo/config.toml"])["selection_args"] == ["--all"]))
    checks.append(("any .cargo/ file + overlays -> --all", run(ov0 + [".cargo/other.toml"])["selection_args"] == ["--all"]))
    for cfg in (".config/round-features.toml", ".config/oracle-corpus.toml", ".config/oracle-fixtures.toml",
                ".config/new-policy.toml", "tools/registry-sync.py", "tools/disk-build-lock.sh", "tools/new-tool.sh"):
        checks.append((f"{cfg} change -> full", run([cfg])["selection_args"] == ["--all"]))
    r = run(["tools/workspace-probe.rs", "tools/camera-matrix-table.py"])
    checks.append(("non-round tools only -> nothing selected, not full", not r["full"] and r["selected"] == []
                   and r["non_entry_tools_changed"] == ["tools/camera-matrix-table.py", "tools/workspace-probe.rs"]))
    # F-R37-2 (lead decision 2026-10-22): the step-0 policy overlays select nothing extra.
    ov = [".config/disk-build-lock.toml", ".config/duplicate-fingerprints.toml", ".config/machine-paths.toml"]
    r = run(ov)
    checks.append(("policy overlays only -> nothing selected, not full",
                   not r["full"] and r["selected"] == [] and r["selection_args"] == [] and len(r["reuse"]) == 5
                   and r["policy_overlays_changed"] == ov))
    r = run([".config/duplicate-fingerprints.toml", ".config/machine-paths.toml", "crates/c/src/lib.rs"])
    checks.append(("overlays + crate change -> only that crate's closure", not r["full"] and r["selected"] == ["c", "e"]))
    r = run(ov + ["Cargo.lock"], l2=lock2)
    checks.append(("overlays + Cargo.lock dependency change -> --all", r["selection_args"] == ["--all"]))
    checks.append(("Cargo.lock dependency change alone -> --all", run(["Cargo.lock"], l2=lock2)["selection_args"] == ["--all"]))
    checks.append(("overlays + nextest.toml -> --all",
                   run(ov + [".config/nextest.toml"])["selection_args"] == ["--all"]))
    checks.append(("toolchain change -> full", run(["rust-toolchain.toml"])["full"]))
    checks.append(("third round -> full", run([], k=2)["full"] and not run([], k=1)["full"]))
    checks.append(("root metadata-only change -> not full",
                   not run(["Cargo.toml"], r2=root.replace("x = 1", "x = 2"))["full"]))
    checks.append(("root profile change -> full",
                   run(["Cargo.toml"], r2=root + "[profile.dev]\nopt-level = 1\n")["full"]))
    checks.append(("nothing changed -> empty selection, all reused",
                   run([])["selected"] == [] and len(run([])["reuse"]) == 5))
    bad = [n for n, ok in checks if not ok]
    for n, ok in checks:
        print(("PASS " if ok else "FAIL ") + n)
    print(f"ROUND_SELECT_SELF_TEST {'OK' if not bad else 'FAIL'} ({len(checks)} checks)")
    return 0 if not bad else 1


def main():
    if "--self-test" in sys.argv:
        return self_test()
    ap = argparse.ArgumentParser()
    for a in ("--repo", "--base", "--candidate", "--metadata-dir", "--round", "--base-round"):
        ap.add_argument(a, required=True)
    ap.add_argument("--rounds-since-full", type=int, default=0)
    ap.add_argument("--sub", default="handshake-studio-assets")
    ap.add_argument("--out")
    a = ap.parse_args()
    try:
        base, cand = git(a.repo, "rev-parse", a.base).strip(), git(a.repo, "rev-parse", a.candidate).strip()
        prefix = a.sub.rstrip("/") + "/"
        changed = [p[len(prefix):] for p in git(a.repo, "diff", "--name-only", base, cand, "--", a.sub).splitlines()
                   if p.startswith(prefix)]
        show = lambda sha, path: subprocess.run(["git", "-C", a.repo, "show", f"{sha}:{prefix}{path}"],
                                                capture_output=True, text=True, encoding="utf-8").stdout
        lock_old, lock_new = show(base, "Cargo.lock"), show(cand, "Cargo.lock")
        root_old, root_new = show(base, "Cargo.toml"), show(cand, "Cargo.toml")
        with open(os.path.join(a.metadata_dir, "Cargo.toml"), encoding="utf-8") as f:
            on_disk = f.read().replace("\r\n", "\n")
        if root_new.replace("\r\n", "\n") != on_disk:
            print("round-select: --metadata-dir root Cargo.toml differs from the candidate's; refresh it to the candidate",
                  file=sys.stderr)
            return 2
        meta = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--offline", "--format-version", "1"],
                                         cwd=a.metadata_dir, capture_output=True, text=True, encoding="utf-8",
                                         check=True).stdout)
    except (subprocess.CalledProcessError, OSError, json.JSONDecodeError) as e:
        print(f"round-select: {e}", file=sys.stderr)
        return 3
    root = os.path.realpath(meta["workspace_root"])
    members = set(meta["workspace_members"])
    pk = []
    for p in meta["packages"]:
        if p["id"] not in members:
            continue
        d = os.path.relpath(os.path.dirname(os.path.realpath(p["manifest_path"])), root).replace("\\", "/")
        pk.append({"name": p["name"], "dir": d, "deps": [x["name"] for x in p["dependencies"] if x.get("path")]})
    rec = decide(changed, pk, a.rounds_since_full, lock_old, lock_new, root_old, root_new, base, cand, a.round,
                 a.base_round)
    if a.out:
        with open(a.out, "w", encoding="utf-8") as f:
            json.dump(rec, f, indent=1)
    print("SELECTION: " + " ".join(rec["selection_args"]))
    print(f"full={rec['full']} changed={rec['changed_crates']} selected={len(rec['selected'])} reused={len(rec['reuse'])}")
    for r in rec["full_reasons"]:
        print("FULL_REASON: " + r)
    return 0


if __name__ == "__main__":
    sys.exit(main())
