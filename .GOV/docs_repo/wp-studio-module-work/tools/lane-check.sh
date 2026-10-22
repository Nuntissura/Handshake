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
# Exit: clippy's status when it fails (the tests are not run), else the test run's status; 2 usage.
set -uo pipefail
owner="" disk=ssd tests=""
while [ $# -gt 0 ]; do
  case "$1" in
    --owner) owner=${2:?}; shift 2 ;;
    --disk) disk=${2:?}; shift 2 ;;
    --tests) tests=${2:?}; shift 2 ;;
    --) shift; break ;;
    *) echo "lane-check: unknown argument $1 (test command goes after --)" >&2; exit 2 ;;
  esac
done
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
