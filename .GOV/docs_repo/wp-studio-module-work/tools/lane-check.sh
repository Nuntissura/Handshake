#!/usr/bin/env bash
# Bundled lane check (Operator 2026-10-20 build strategy): ONE disk-lock hold runs clippy over the tested crates and
# their direct workspace dependents, then the one test run, so a lane never takes separate lock acquisitions for
# check, clippy and each test. The test command is written out literally after `--` (it must start with
# `cargo nextest run`), so the gameplan hook sees and gates it like any test run and the retry guard keys on it.
# Usage (from the lane workspace, leading cd first):
#   cd "<lane worktree>/handshake-studio-assets" && bash "<kernel>/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" \
#     --owner <lane-id-lowercase> [--disk ssd|d] [--tests NAME[,NAME...]] -- cargo nextest run -p <crate> [-p ...] [nextest args]
# --owner     STUDIO_OWNER (required; lane id). --disk: ssd (default for lanes: the granted C: target) or d.
# --tests     test names joined into ONE filter `-E 'test(A) | test(B)'` appended to the test command (substring match);
#             or pass your own -E in the test command (not both).
# Crates: every -p/--package of the test command; their --features/-F apply to clippy of those crates only.
# Dependents: workspace members that depend on a tested crate (any dependency kind), from `cargo metadata --no-deps`;
#   clippy --tests runs over them without features (STUDIO_LANE_CHECK_NO_DEPENDENTS=1 skips them).
# Step 0 (round parity, F-C55-1, lead 2026-10-24): FIRST, before clippy and outside the disk lock (read-only scan,
#   ~1 s), the round's step-0 policy over the WHOLE lane workspace (cwd): V4-03 machine-local paths (crates/*/src and
#   crates/*/tests, .config/machine-paths.toml [pinned]) and V4-05 frozen duplicates (crates/*/src,
#   .config/duplicate-fingerprints.toml allowed sites). Logic = the round's own: the step-0 policy heredoc is read
#   from the lane tree's tools/run-round.sh (the code the round runs on that candidate); the identical embedded copy
#   below is used only if that file or block is missing, and a textual mismatch is reported (the round's wins).
#   A hit prints the round's messages and exits with the round's code (72 V4-03, 71 V4-05) without clippy or tests.
#   lane-check.sh --step0-only            run only step 0 on the cwd workspace (no --owner, no test command).
#   lane-check.sh --step0-self-test DIR   run the round's block and the embedded copy on DIR, compare exit, stdout
#                                         (JSON) and stderr; prints STEP0_SELF_TEST OK|FAIL and exits 0|1.
# Exit: 72/71 step 0 policy hit (round codes), 2 usage or step 0 config error; else clippy's status when it fails
#   (the tests are not run), else the test run's status.
set -uo pipefail
owner="" disk=ssd tests="" step0_only=0 step0_selftest=""
while [ $# -gt 0 ]; do
  case "$1" in
    --owner) owner=${2:?}; shift 2 ;;
    --disk) disk=${2:?}; shift 2 ;;
    --tests) tests=${2:?}; shift 2 ;;
    --step0-only) step0_only=1; shift ;;
    --step0-self-test) step0_selftest=${2:?}; shift 2 ;;
    --) shift; break ;;
    *) echo "lane-check: unknown argument $1 (test command goes after --)" >&2; exit 2 ;;
  esac
done
# Step 0 policy (round parity). Embedded copy of the step-0 policy heredoc of tools/run-round.sh (handshake-studio-assets
# 42b304c7); keep it byte-identical to the round's block (--step0-self-test compares both).
STEP0_PY_EMBEDDED=$(cat <<'PY'
import json, pathlib, re, sys, tomllib

sys.stdout.reconfigure(newline="\n")
root = pathlib.Path(sys.argv[1])
def load(name, schema):
    p = root / ".config" / name
    if not p.exists():
        print(f"run-round: .config/{name} missing", file=sys.stderr); sys.exit(2)
    doc = tomllib.loads(p.read_text(encoding="utf-8"))
    if doc.get("schema") != schema:
        print(f"run-round: .config/{name} schema must be {schema}", file=sys.stderr); sys.exit(2)
    return doc
def files(*subs):
    for crate in sorted(p for p in (root / "crates").iterdir() if p.is_dir()):
        for sub in subs:
            if (crate / sub).is_dir():
                for f in sorted((crate / sub).rglob("*"), key=lambda q: q.as_posix()):
                    data = f.read_bytes() if f.is_file() else b"\0"
                    if b"\0" not in data:
                        yield f.relative_to(root).as_posix(), data.decode("utf-8", errors="replace").splitlines()
paths_cfg = load("machine-paths.toml", "hsk.studio.machine-paths@1")
dup_cfg = load("duplicate-fingerprints.toml", "hsk.studio.duplicate-fingerprints@1")
pinned = paths_cfg.get("pinned", {})
MP = re.compile(r"(?<![A-Za-z0-9])[A-Za-z]:(?:/|\\(?![\"']))|Users[\\/]")
hits = {}
for rel, lines in files("src", "tests"):
    for i, line in enumerate(lines, 1):
        if MP.search(line):
            hits.setdefault(rel, []).append(f"{rel}:{i}: {line.strip()[:160]}")
path_violations = [{"file": rel, "hits": len(h), "pinned": pinned.get(rel), "lines": h[:20]}
                   for rel, h in sorted(hits.items()) if len(h) > pinned.get(rel, 0)]
pins_dropped = {rel: {"pinned": n, "now": len(hits.get(rel, []))} for rel, n in pinned.items()
                if len(hits.get(rel, [])) < n}
src = list(files("src"))
dups = []
for fp in dup_cfg.get("fingerprint", []):
    pats = [re.compile(p) for p in fp["patterns"]]
    sites = {}
    for rel, lines in src:
        for i, line in enumerate(lines, 1):
            if any(p.search(line) for p in pats):
                sites.setdefault(rel, []).append(f"{rel}:{i}: {line.strip()[:160]}")
    allowed = list(fp["allowed"])  # path prefixes relative to the workspace root (exact files or dirs)
    ok = lambda rel: any(rel.startswith(a) for a in allowed)
    new = {rel: sites[rel][:20] for rel in sorted(sites) if not ok(rel)}
    dups.append({"id": fp["id"], "new_sites": new, "sites": sorted(sites),
                 "allowed_without_match": [a for a in allowed if not any(r.startswith(a) for r in sites)]})
new_dups = [d for d in dups if d["new_sites"]]
print(json.dumps({"machine_paths": {"violations": path_violations, "pins_dropped": pins_dropped,
                                    "files_with_hits": len(hits)},
                  "duplicates": dups}, indent=1))
for v in path_violations:
    print(f"V4-03 machine-local path: {v['file']} hits={v['hits']} pinned={v['pinned']}", *v["lines"], sep="\n  ",
          file=sys.stderr)
for d in new_dups:
    for rel, lines in d["new_sites"].items():
        print(f"V4-05 new {d['id']} site: {rel}", *lines, sep="\n  ", file=sys.stderr)
sys.exit(72 if path_violations else 71 if new_dups else 0)
PY
)
step0_py=$(command -v python || command -v python3) || { echo "lane-check: python 3.11+ required" >&2; exit 2; }
# The round's own block from <ws>/tools/run-round.sh (between the step0.policy.json heredoc line and its PY end).
step0_round_block() {
  [ -f "$1/tools/run-round.sh" ] || return 1
  tr -d '\r' <"$1/tools/run-round.sh" | awk '/step0\.policy\.json.*<<.PY.$/{f=1;next} f&&/^PY$/{exit} f'
}
# step0_scan WS SCRIPT -> exit code of the round's policy; stdout JSON to $step0_json, stderr to $step0_txt.
step0_scan() {
  local out err rc
  out=$(mktemp) err=$(mktemp)
  "$step0_py" -X utf8 -I - "$1" <<<"$2" >"$out" 2>"$err"; rc=$?
  step0_json=$(cat "$out"); step0_txt=$(cat "$err"); rm -f "$out" "$err"
  return $rc
}
lane_step0() {
  local ws=$1 script round rc
  round=$(step0_round_block "$ws")
  if [ -n "$round" ]; then
    script=$round
    [ "$round" = "$STEP0_PY_EMBEDDED" ]       || echo "lane-check: note: embedded step-0 copy differs from $ws/tools/run-round.sh; using the round's block (update lane-check.sh)" >&2
  else
    script=$STEP0_PY_EMBEDDED
    echo "lane-check: note: no step-0 block in $ws/tools/run-round.sh; using the embedded copy" >&2
  fi
  step0_scan "$ws" "$script"; rc=$?
  case $rc in
    0) echo "lane-check: step 0 policy clean (V4-03 machine paths, V4-05 frozen duplicates)" >&2; return 0 ;;
    71|72)
      echo "lane-check: step0 policy: $([ $rc -eq 72 ] && echo 'V4-03 machine-local path' || echo 'V4-05 new frozen-duplicate site') (round would exit $rc; clippy and tests not run)" >&2
      printf '%s
' "$step0_txt" | head -n 40 >&2
      return $rc ;;
    *) printf '%s
' "$step0_txt" >&2; echo "lane-check: step 0 policy check failed (exit $rc)" >&2; return 2 ;;
  esac
}
if [ -n "$step0_selftest" ]; then
  round=$(step0_round_block "$step0_selftest") || round=""
  [ -n "$round" ] || { echo "lane-check: $step0_selftest/tools/run-round.sh has no step-0 block" >&2; exit 1; }
  step0_scan "$step0_selftest" "$round"; rc_r=$?; json_r=$step0_json; txt_r=$step0_txt
  step0_scan "$step0_selftest" "$STEP0_PY_EMBEDDED"; rc_e=$?
  ident=$([ "$round" = "$STEP0_PY_EMBEDDED" ] && echo yes || echo no)
  echo "step0 self-test on $step0_selftest: round exit=$rc_r embedded exit=$rc_e text_identical=$ident"
  printf '%s
' "$txt_r"
  if [ $rc_r -eq $rc_e ] && [ "$json_r" = "$step0_json" ] && [ "$txt_r" = "$step0_txt" ]; then
    echo "STEP0_SELF_TEST OK (exit $rc_r)"; exit 0
  fi
  echo "STEP0_SELF_TEST FAIL"; exit 1
fi
if [ $step0_only -eq 1 ]; then lane_step0 "$PWD"; exit $?; fi
[ -n "$owner" ] || { echo "lane-check: --owner <lane-id-lowercase> is required" >&2; exit 2; }
[ "${1:-}" = cargo ] && [ "${2:-}" = nextest ] && [ "${3:-}" = run ] \
  || { echo "lane-check: after -- give the literal test command: cargo nextest run -p <crate> ..." >&2; exit 2; }
shift 3
run_args=("$@")
crates=() feats=() a="" i=0
while [ $i -lt ${#run_args[@]} ]; do
  a=${run_args[$i]}
  case "$a" in
    -p|--package) i=$((i + 1)); crates+=("${run_args[$i]:-}") ;;
    --package=*) crates+=("${a#--package=}") ;;
    -F|--features) i=$((i + 1)); feats+=(--features "${run_args[$i]:-}") ;;
    --features=*) feats+=("$a") ;;
    --all-features) feats+=("$a") ;;
    -E|--filterset|--filter-expr) [ -z "$tests" ] || { echo "lane-check: give --tests or -E, not both" >&2; exit 2; } ;;
  esac
  i=$((i + 1))
done
[ ${#crates[@]} -gt 0 ] || { echo "lane-check: the test command needs at least one -p <crate>" >&2; exit 2; }
if [ -n "$tests" ]; then
  expr=""
  IFS=, read -ra names <<<"$tests"
  for n in "${names[@]}"; do [ -n "$n" ] && expr+="${expr:+ | }test(${n})"; done
  run_args+=(-E "$expr")
fi
lane_step0 "$PWD" || exit $?
case "$disk" in ssd) export STUDIO_BUILD_DISK=ssd ;; d|D) export STUDIO_BUILD_DISK=d ;; *) echo "lane-check: --disk ssd|d" >&2; exit 2 ;; esac
export STUDIO_OWNER=$owner
unset STUDIO_DISK_BUILD_LOCK_DIR _STUDIO_ENV_LOCK_DERIVED
TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$TOOLS/../builders/studio-env.sh" || exit 2
dependents=()
if [ "${STUDIO_LANE_CHECK_NO_DEPENDENTS:-0}" != 1 ]; then
  mapfile -t dependents < <(command cargo metadata --no-deps --format-version 1 2>/dev/null | python -c '
import json, sys
tested = set(sys.argv[1:])
meta = json.load(sys.stdin)
for p in meta["packages"]:
    if p["name"] not in tested and any(d["name"] in tested and d.get("path") for d in p["dependencies"]):
        print(p["name"])' "${crates[@]}" | tr -d '\r' | sort -u)
fi
pk=(); for c in "${crates[@]}"; do pk+=(-p "$c"); done
dk=(); for c in "${dependents[@]}"; do dk+=(-p "$c"); done
echo "lane-check: owner=$owner disk=$STUDIO_BUILD_DISK target=$CARGO_TARGET_DIR crates=[${crates[*]}] dependents=[${dependents[*]}]" >&2
lane_check_body() {
  cargo clippy "${pk[@]}" ${feats[@]+"${feats[@]}"} --tests || return $?
  if [ ${#dk[@]} -gt 0 ]; then cargo clippy "${dk[@]}" --tests || return $?; fi
  cargo nextest run "${run_args[@]}"
}
studio_disk_lock_run "lane-check $owner: clippy [${crates[*]}] + dependents [${dependents[*]}], nextest run ${run_args[*]}" lane_check_body
