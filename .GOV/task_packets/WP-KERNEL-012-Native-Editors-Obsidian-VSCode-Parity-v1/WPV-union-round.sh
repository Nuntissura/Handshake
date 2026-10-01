#!/bin/bash
# WP-KERNEL-012 MT-109 wpv-c3x lane round runner.
# Usage: run-round.sh <SHA>
# Builds core+native+backend-bin at <SHA>, runs core nextest and native nextest
# (two invocations - core and native are separate Cargo manifests, not one
# workspace, so a single unified nextest run across both is not possible
# without merging the manifests; this is documented, not silently assumed),
# writes junit-<SHA>-core.xml and junit-<SHA>-native.xml into the lane root.
# No `timeout` wrapper anywhere: nextest's own slow-timeout/terminate-after
# (see nextest.toml) is the hang guard; an outer timeout truncates mid-suite
# with no junit (see run44, 2026-09-24).
set -euo pipefail
export GIT_TERMINAL_PROMPT=0 GCM_INTERACTIVE=never

SHA="${1:?usage: run-round.sh <SHA> [core-only]}"
[[ "$SHA" =~ ^[0-9a-f]{40}$ ]] || { echo "[run-round] FATAL: full lowercase 40-character SHA required"; exit 2; }
# MODE core-only: core + extracted crates only; skips native build, backend binary,
# native preflight and native run (dispatch-scoped rounds whose READY MTs need no native proof).
MODE="${2:-}"
# MODE mt164-capture: Operator-authorized single capture run (packet.json operator_decisions_20260929
# mt164_close) of the MT-164 timing test on an already-built export; lib only, no extracted crates,
# JUnit written to junit-<SHA>-core-mt164-capture.xml, one-shot marker per SHA.
# MODE backend-opt-diag: Operator decision WP012-OPERATOR-DEBUG-BACKEND-DEP-OPT-APPROVED-20261001 condition C1
# (one diagnostic run; CX-EXEC-014 remediation MT-153.json#remediation_20261001_backend_opt_diag). Candidate may be
# an ancestor of the pushed tip; requires the completed union round for <SHA> (the dev-profile reference); no core
# build/run, no extracted crates, no native preflight; builds the backend with WPV-backend-profile.toml and runs
# exactly the two edge-create tests; JUnit junit-<SHA>-native-backend-opt-diag.xml; one-shot marker per SHA.
[[ -z "$MODE" || "$MODE" = core-only || "$MODE" = mt164-capture || "$MODE" = backend-opt-diag ]] || { echo "[run-round] FATAL: unknown mode $MODE"; exit 2; }
NATIVE_SKIP=0; [[ "$MODE" = core-only || "$MODE" = mt164-capture ]] && NATIVE_SKIP=1
echo "[run-round] mode=${MODE:-full}"
LANE="D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-c3x"
TARGET="C:/.target/WP-KERNEL-012/MT-109/wpv-c3x/target-r52"
# The C: grant covers only this build target; source archives are target inputs.
EXPORT_ROOT="$TARGET"
WORKTREE="D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-native-editors-v1"
NEXTEST="D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools/cargo-nextest/0.9.146/cargo-nextest.exe"

check_target_cap() {
  local bytes free_kib
  bytes="$(find "$TARGET" -type f -printf '%s\n' | awk '{sum += $1} END {printf "%.0f", sum}')"
  free_kib="$(df -Pk "$TARGET" | awk 'NR == 2 {print $4}')"
  [[ "$bytes" =~ ^[0-9]+$ && "$free_kib" =~ ^[0-9]+$ ]] || {
    echo "[run-round] FATAL: cannot measure C: target size or free space"; exit 2;
  }
  echo "[run-round] C: target bytes=$bytes cap=150000000000 free_kib=$free_kib floor_kib=187500000"
  (( bytes <= 150000000000 && free_kib >= 187500000 )) || {
    echo "[run-round] FATAL: C: target cap or free-space floor exceeded"; exit 2;
  }
}

[ -d "$LANE" ] && [ -d "$TARGET" ] || { echo "[run-round] FATAL: assigned lane or warm target missing"; exit 2; }
[ -z "$(git -C "$WORKTREE" status --porcelain)" ] || { echo "[run-round] FATAL: builder worktree is dirty"; exit 2; }
REMOTE_SHA="$(git -C "$WORKTREE" ls-remote origin refs/heads/feat/WP-KERNEL-012 | cut -f1)"
if [[ "$MODE" = backend-opt-diag ]]; then
  git -C "$WORKTREE" merge-base --is-ancestor "$SHA" "$REMOTE_SHA" || { echo "[run-round] FATAL: diagnostic candidate is not an ancestor of the pushed branch tip ($REMOTE_SHA)"; exit 2; }
  [[ -f "$LANE/junit-$SHA-native.xml" ]] || { echo "[run-round] FATAL: backend-opt-diag requires the completed union round (dev reference) for $SHA"; exit 2; }
  [[ ! -e "$LANE/BACKEND-OPT-DIAG-${SHA:0:8}.started" ]] || { echo "[run-round] FATAL: backend-opt-diag already consumed for $SHA"; exit 2; }
else
  [ "$(git -C "$WORKTREE" rev-parse HEAD)" = "$SHA" ] || { echo "[run-round] FATAL: candidate is not builder HEAD"; exit 2; }
  [ "$REMOTE_SHA" = "$SHA" ] || { echo "[run-round] FATAL: candidate is not pushed branch tip ($REMOTE_SHA)"; exit 2; }
fi
# Owned-backend build profile (Operator decision WP012-OPERATOR-DEBUG-BACKEND-DEP-OPT-APPROVED-20261001, C2/C3):
# checked-in allow-list config next to this script, resolved at runtime, passed only to the backend build line.
BACKEND_PROFILE_TOML="$(dirname "$(readlink -f "$0")")/WPV-backend-profile.toml"
[ -f "$BACKEND_PROFILE_TOML" ] || { echo "[run-round] FATAL: backend profile config missing: $BACKEND_PROFILE_TOML"; exit 2; }
BACKEND_PROFILE_CONFIG="$(cygpath -m "$BACKEND_PROFILE_TOML")"
BACKEND_PROFILE_SHA256="$(sha256sum "$BACKEND_PROFILE_TOML" | cut -c1-64)"
echo "[run-round] backend_build_profile=dev+dep-opt2 config=$BACKEND_PROFILE_CONFIG sha256=$BACKEND_PROFILE_SHA256 decision=WP012-OPERATOR-DEBUG-BACKEND-DEP-OPT-APPROVED-20261001 jobs=2"
# Test builds use the same config (Operator decision WP012-OPERATOR-MT167-OPTIMIZE-TEST-DB-20261001). Proof is read
# from the verbose build log, not assumed: every allow-listed crate rustc compiled must carry -C opt-level=2 and
# handshake_core must not; in the core build every allow-listed crate must be compiled at O2 or Fresh (fingerprint
# unchanged since an O2 compile). Any violation is INVALID_CONFIG (exit 5, no verdicts from this round).
OPT_CRATES=(surrealdb:surrealdb surrealdb_core:surrealdb-core librocksdb_sys:surrealdb-librocksdb-sys rocksdb:surrealdb-rocksdb
  surrealdb_collections:surrealdb-collections surrealdb_types:surrealdb-types surrealdb_strand:surrealdb-strand surrealdb_protocol:surrealdb-protocol)
check_test_profile() {
  # $1 label  $2 verbose build log  $3 require-all (1 = every allow-listed crate must be present)
  local label="$1" log="$2" require_all="$3" entry crate pkg lines bad=0 compiled=0 fresh=0 absent=0
  for entry in "${OPT_CRATES[@]}"; do
    crate="${entry%%:*}"; pkg="${entry##*:}"
    lines="$(grep -E -- "--crate-name $crate .*--crate-type lib" "$log" || true)"
    if [[ -n "$lines" ]]; then
      if grep -q -- "-C opt-level=2" <<<"$lines"; then compiled=$((compiled+1)); echo "[run-round] test_profile_proof $label $crate compiled opt-level=2"
      else bad=1; echo "[run-round] test_profile_proof $label $crate compiled WITHOUT opt-level=2"; fi
    elif grep -Eq "^ *Fresh $pkg v" "$log"; then fresh=$((fresh+1)); echo "[run-round] test_profile_proof $label $crate fresh"
    else absent=$((absent+1)); echo "[run-round] test_profile_proof $label $crate absent"; [[ "$require_all" = 1 ]] && bad=1
    fi
  done
  if grep -E -- "--crate-name handshake_core " "$log" | grep -q -- "-C opt-level=2"; then bad=1; echo "[run-round] test_profile_proof $label handshake_core compiled WITH opt-level=2"; fi
  echo "[run-round] test_profile_proof $label compiled_o2=$compiled fresh=$fresh absent=$absent bad=$bad log=$log"
  [[ "$bad" = 0 ]] || { echo "[run-round] INVALID_CONFIG test profile proof failed ($label)"; exit 5; }
}
check_target_cap

mkdir -p "$LANE/logs" "$LANE/tmp" "$LANE/runtime" "$LANE/workspace"

# 1. Stable round source path ([VPX-011], Operator 2026-10-01 "WP-012 scripts now"; CX-VAL-007):
#    one fixed per-owner export-current, refreshed in place to the frozen candidate (changed files only,
#    current mtimes, removed files deleted, round outputs cleared, every file verified against the
#    candidate archive, identity written last; full replace when the identity is missing/partial/mismatched).
#    A per-candidate path (export-<sha>) changes Cargo's hash for the sibling path crates and cold-rebuilds
#    every dependent; the identity record, not the path, now carries candidate provenance.
# shellcheck source=/dev/null
source "$(dirname "$(readlink -f "$0")")/WPV-export-refresh.sh"
wpv_refresh_export_current "$SHA" "$WORKTREE" "$EXPORT_ROOT" "[run-round]" || { echo "[run-round] FATAL: export-current refresh failed"; exit 2; }
EXPORT="$WPV_EXPORT"
echo "[run-round] verdict binding: export identity sha256=$WPV_EXPORT_IDENTITY_SHA256 refresh_mode=$WPV_EXPORT_MODE HANDSHAKE_PROOF_SOURCE_SHA=$SHA"
check_target_cap

# 3. Complete env set (statically derived from tests/backend_proof_support/mod.rs
#    env::var reads + queue-proof-commands.txt; see 00-lane.txt note dated
#    2026-09-24 for the source trace of each var).
OWNER_ROOT="$LANE"                              # <artifact-root>/WP-KERNEL-012/MT-109/wpv-c3x
export CARGO_TARGET_DIR="$TARGET"
export TMP="$LANE/tmp"; export TEMP="$TMP"; export TMPDIR="$TMP"
export CARGO_PROFILE_DEV_DEBUG=line-tables-only
export CARGO_PROFILE_TEST_DEBUG=line-tables-only
export CARGO_INCREMENTAL=0
export HANDSHAKE_PROOF_SOURCE_SHA="$SHA"
export HANDSHAKE_ARTIFACTS_ROOT="D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts"
# HANDSHAKE_ARTIFACTS_ROOT alone resolves to a 2-component path
# (Handshake_Artifacts/wp-kernel-012) which FAILS canonical_handshake_artifact_boundary
# in tests/backend_proof_support/mod.rs (requires ["wp-kernel-012"],
# ["handshake-test","wp-kernel-012"], or a WP-KERNEL-012/MT-<id>/owner 3+ level
# shape). Keep the root inside the assigned owner. The native proof helper
# appends wp-kernel-012; shortening evidence to e makes the observed run52
# 261-character RocksDB path 254 characters.
export HANDSHAKE_TEST_ARTIFACTS_ROOT="$LANE/e"
mkdir -p "$HANDSHAKE_TEST_ARTIFACTS_ROOT"
export HANDSHAKE_TEST_STAGE_BINDING_ROOT="$OWNER_ROOT/stage-binding"
mkdir -p "$HANDSHAKE_TEST_STAGE_BINDING_ROOT"
# HSK_TEST_BACKEND_TARGET_ROOT must be a strict descendant of HANDSHAKE_ARTIFACTS_ROOT
# (canonical_handshake_artifact_boundary, backend_proof_support/mod.rs:861) - it
# cannot live under $TARGET (C:) while HANDSHAKE_ARTIFACTS_ROOT is D:. run51
# (2026-09-24) failed 31 native tests on exactly this mismatch (mod.rs:814).
# Build the backend binary directly into its final D:-side location so no
# separate copy step is needed.
export HSK_TEST_BACKEND_TARGET_ROOT="$LANE/backend-bin"
export HSK_TEST_BACKEND_BIN="$HSK_TEST_BACKEND_TARGET_ROOT/debug/handshake_core.exe"
export HANDSHAKE_TEST_SURREAL_SYNC=never
export SURREAL_DATASTORE_SYNC=never
export HANDSHAKE_SURREAL_TEST_STORE_ROOT="$LANE/runtime"
export HANDSHAKE_GPU_SCREENSHOT="${HANDSHAKE_GPU_SCREENSHOT:-1}"   # this host has a real GPU (MT-124 GREEN GPU proof, 2026-08-18); default 1 per IV 2026-09-24
# atelier_surreal_support/mod.rs:39 requires an isolated HANDSHAKE_WORKSPACE_ROOT
# ("native artifact fixtures require an isolated HANDSHAKE_WORKSPACE_ROOT");
# run50 (2026-09-24) failed 3 atelier_stealth_window_tests without this set.
export HANDSHAKE_WORKSPACE_ROOT="$OWNER_ROOT/workspace-root"
mkdir -p "$HANDSHAKE_WORKSPACE_ROOT"

# MT-165 disposable session-worktree repo (Operator decision WP012-MT165-DISPOSABLE-GIT-REPO-20260930).
# Session-worktree code runs `git -C <repo> worktree add|remove` where <repo> is fixed at compile time:
# CARGO_MANIFEST_DIR/../../.. (workflows.rs repo_root_from_manifest_dir; api/jobs.rs and
# model_session_scheduler_tests.rs guards) = this export root, which is not a git checkout. Make the
# export a throwaway repo whose git dir lives under the WP artifact root (the export only gets a .git
# pointer file), HEAD = the exact candidate fetched read-only from the builder worktree (no branch, no
# push, nothing ever merged back). HANDSHAKE_SESSION_WORKTREE_ROOT (workspace_safety.rs
# session_worktree_path) points session worktrees into the lane; model_session_scheduler_tests
# overrides it with $TMP/hsk-session-worktrees-model-session-scheduler-<pid>, also in the lane.
# On every exit (success, failure, TERM/INT) the trap records `git worktree list`; with no leftover
# session worktree it moves the git dir, the pointer file and the session root to the Recycle Bin
# (WPV-recycle.ps1, shell API, silent). Leftovers are kept and logged for the MT-165 verdict.
# A force-killed script (TerminateProcess) cannot run the trap; the validator then cleans by hand.
SESSION_GIT_DIR="$LANE/sg/${SHA:0:8}.git"
export HANDSHAKE_SESSION_WORKTREE_ROOT="$LANE/sg/${SHA:0:8}-wt"
SESSION_GIT_LOG="$LANE/logs/session-git-$SHA.log"
RECYCLE_HELPER="$(dirname "$(readlink -f "$0")")/WPV-recycle.ps1"
[ -f "$RECYCLE_HELPER" ] || { echo "[run-round] FATAL: recycle helper missing: $RECYCLE_HELPER"; exit 2; }
[[ "$SESSION_GIT_DIR" == "$HANDSHAKE_ARTIFACTS_ROOT/WP-KERNEL-012/"* ]] || { echo "[run-round] FATAL: session git dir outside WP artifact root"; exit 2; }
[ ! -e "$EXPORT/.git" ] || { echo "[run-round] FATAL: export already holds .git (prior session repo preserved for review): $EXPORT/.git"; exit 2; }
[ ! -e "$SESSION_GIT_DIR" ] && [ ! -e "$HANDSHAKE_SESSION_WORKTREE_ROOT" ] \
  || { echo "[run-round] FATAL: prior session git dir or root preserved for review under $LANE/sg"; exit 2; }
mkdir -p "$LANE/sg"
SESSION_GIT_MARKER="$LANE/tmp/session-git-start-$SHA"
touch "$SESSION_GIT_MARKER"
session_git_cleanup() {
  local rc=$?
  trap - EXIT
  set +e
  {
    echo "cleanup_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ) script_exit=$rc"
    echo "--- git worktree list --porcelain (export repo)"
    git -C "$EXPORT" worktree list --porcelain
    echo "--- session root entries: $HANDSHAKE_SESSION_WORKTREE_ROOT"
    [ -d "$HANDSHAKE_SESSION_WORKTREE_ROOT" ] && ls -A "$HANDSHAKE_SESSION_WORKTREE_ROOT"
    echo "--- scheduler roots created this round under $TMP"
    find "$TMP" -maxdepth 1 -name 'hsk-session-worktrees-model-session-scheduler-*' -newer "$SESSION_GIT_MARKER"
  } >> "$SESSION_GIT_LOG" 2>&1
  local registered leftover_dirs scheduler_roots
  registered="$(git -C "$EXPORT" worktree list --porcelain 2>/dev/null | grep -c '^worktree ')"
  leftover_dirs=0; [ -d "$HANDSHAKE_SESSION_WORKTREE_ROOT" ] && leftover_dirs="$(ls -A "$HANDSHAKE_SESSION_WORKTREE_ROOT" | wc -l)"
  scheduler_roots="$(find "$TMP" -maxdepth 1 -name 'hsk-session-worktrees-model-session-scheduler-*' -newer "$SESSION_GIT_MARKER" | wc -l)"
  if [[ "$registered" == 1 && "$leftover_dirs" == 0 && "$scheduler_roots" == 0 ]]; then
    for p in "$SESSION_GIT_DIR" "$EXPORT/.git" "$HANDSHAKE_SESSION_WORKTREE_ROOT"; do
      [ -e "$p" ] || continue
      case "$p" in "$EXPORT/.git") root="$EXPORT" ;; *) root="$HANDSHAKE_ARTIFACTS_ROOT/WP-KERNEL-012" ;; esac
      pwsh -NoProfile -NonInteractive -File "$RECYCLE_HELPER" -Path "$p" -AllowedRoot "$root" >> "$SESSION_GIT_LOG" 2>&1 \
        || echo "SESSION_GIT_RECYCLE_FAILED $p" >> "$SESSION_GIT_LOG"
    done
    echo "[run-round] SESSION_GIT_REMOVED (recycle bin); log $SESSION_GIT_LOG"
  else
    echo "SESSION_GIT_LEFTOVERS registered_worktrees=$registered session_root_entries=$leftover_dirs scheduler_roots=$scheduler_roots" >> "$SESSION_GIT_LOG"
    echo "[run-round] SESSION_GIT_LEFTOVERS kept for MT-165 review; log $SESSION_GIT_LOG"
  fi
  exit "$rc"
}
trap session_git_cleanup EXIT
trap 'exit 143' TERM
trap 'exit 130' INT
git init -q --separate-git-dir="$SESSION_GIT_DIR" "$EXPORT"
git -C "$EXPORT" -c core.longpaths=true fetch -q --depth=1 --no-tags \
  --upload-pack="git -c uploadpack.allowAnySHA1InWant=true upload-pack" "$WORKTREE" "$SHA"
git -C "$EXPORT" update-ref --no-deref HEAD "$SHA"
[ "$(git -C "$EXPORT" rev-parse HEAD)" = "$SHA" ] || { echo "[run-round] FATAL: session repo HEAD is not the candidate"; exit 2; }
echo "[run-round] session git repo $SESSION_GIT_DIR HEAD=$SHA; HANDSHAKE_SESSION_WORKTREE_ROOT=$HANDSHAKE_SESSION_WORKTREE_ROOT" | tee -a "$SESSION_GIT_LOG"

# 2. Build core union (--no-run), native union (--no-run), and the backend
#    binary, one cargo invocation per crate, no timeout wrapper.
CORE_TESTS=(
  knowledge_documents_api_tests knowledge_fail_closed_tests loom_atomic_receipt_tests
  loom_block_collection_views_tests wp_kernel_012_native_editor_routes_tests
  loom_daily_journal_tests loom_transclusion_tests loom_media_tiers_tests
  project_wiki_drift_tests mt154_non_loom_route_authority_tests
  knowledge_crdt_bridge_api_tests knowledge_ingestion_api_tests
  knowledge_memory_api_tests knowledge_retrieval_debug_api_tests
  atelier_stealth_window_tests atelier_loom_projection_api_tests
  calendar_storage_tests mt155_product_screenshot_capture_route_auth_tests
  kernel_product_screenshot_capture_tests mt157_debug_breakpoints_authority_tests
  micro_task_executor_tests mt151_early_lock_release_race_tests
  model_session_scheduler_tests
)
# MT-127 is BLOCKED on its separately governed end-of-WP sweep/build/interactive
# proofs. Select only the other READY MTs' named targets. Native --lib is
# required in full by MT-143 PC-143-07; core --lib is filtered below.
NATIVE_TESTS=(
  test_app_host_mount test_app_host_mount_secondary test_author_id_budget
  test_block_collection_view test_calendar_interop test_canvas_board
  test_canvas_board_argus test_ckc_embed test_code_nav_client
  test_code_note_cross_ref test_completion_hover_accesskit
  test_context_menu test_context_menu_surfaces test_e7_knowledge_accesskit
  test_e7_knowledge_accesskit_argus test_e7_swarm_edit_proof
  test_editor_body_context_menu test_event_emitter test_fems_interop_proofs
  test_find_in_files test_flight_recorder_authz
  test_interconnect_ckc_to_note test_interconnect_loom_backlink_search
  test_interconnect_note_code_crossref test_interconnect_shared_undo_ledger
  test_locus_interop test_loom_address test_lsp_client test_manual_content
  test_mcp_snapshot_viewport test_memory_proposal test_memory_proposal_argus
  test_mt116_chip_pill_containment test_navigation_bus
  test_other_pillar_interop_proofs test_quick_switcher
  test_runtime_chat_pane test_stage_interop test_tags_panel
  test_tags_panel_argus test_theme
)
NATIVE_LIB=1
# Named-test round selection (Operator 2026-09-30: run only the named proof tests of the MTs the round
# judges, CX-EXEC-014; reuse passed results under CX-VAL-002). WPV-round-selection.sh is generated per
# candidate and committed with its WPV-round-selection.json manifest; it overrides CORE_TESTS,
# NATIVE_TESTS, NATIVE_LIB, CORE_FILTER, NATIVE_FILTER and EXTRACTED_CRATES. Mode full requires it.
SELECTION_FILE="$(dirname "$(readlink -f "$0")")/WPV-round-selection.sh"
if [[ -z "$MODE" ]]; then
  [ -f "$SELECTION_FILE" ] || { echo "[run-round] FATAL: round selection missing: $SELECTION_FILE"; exit 2; }
  # shellcheck source=/dev/null
  source "$SELECTION_FILE"
  [ "${ROUND_SELECTION_SHA:-}" = "$SHA" ] || { echo "[run-round] FATAL: round selection is for ${ROUND_SELECTION_SHA:-none}, not $SHA"; exit 2; }
  echo "[run-round] round selection: core ${#CORE_TESTS[@]} targets + lib, native ${#NATIVE_TESTS[@]} targets lib=$NATIVE_LIB"
fi
CORE_SKIP=0
NATIVE_JUNIT_NAME="junit-$SHA-native.xml"
if [[ "$MODE" = backend-opt-diag ]]; then
  CORE_SKIP=1
  EXTRACTED_CRATES=()
  NATIVE_TESTS=(test_block_collection_view test_tags_panel)
  NATIVE_LIB=0
  NATIVE_FILTER='(binary(=test_block_collection_view) & test(=block_collection_views_live_surrealdb_self_seed_full_round_trip)) | (binary(=test_tags_panel) & test(=tags_tag_hub_live_surrealdb_self_seeds_mounted_round_trip))'
  NATIVE_JUNIT_NAME="junit-$SHA-native-backend-opt-diag.xml"
  echo "[run-round] backend-opt-diag selection: native ${NATIVE_TESTS[*]} filter $NATIVE_FILTER"
fi
NATIVE_TARGET_ARGS=(); [[ "$NATIVE_LIB" = 1 ]] && NATIVE_TARGET_ARGS=(--lib)
for t in "${NATIVE_TESTS[@]}"; do NATIVE_TARGET_ARGS+=(--test "$t"); done

core_test_args=(); for t in "${CORE_TESTS[@]}"; do core_test_args+=(--test "$t"); done
CORE_JUNIT_NAME="junit-$SHA-core.xml"
if [[ "$MODE" = mt164-capture ]]; then
  CAPTURE_MARKER="$LANE/MT164-CAPTURE-${SHA:0:8}.started"
  [[ ! -e "$CAPTURE_MARKER" ]] || { echo "[run-round] FATAL: mt164 capture already consumed for $SHA"; exit 2; }
  [[ -f "$LANE/junit-$SHA-core.xml" ]] || { echo "[run-round] FATAL: capture requires the completed union round for $SHA"; exit 2; }
  core_test_args=()
  EXTRACTED_CRATES=()
  CORE_JUNIT_NAME="junit-$SHA-core-mt164-capture.xml"
fi
[[ -n "$MODE" || -n "${ROUND_SELECTION_SHA:-}" ]] || EXTRACTED_CRATES=(handshake_document handshake_storage_support)
[[ "$MODE" = core-only ]] && EXTRACTED_CRATES=(handshake_document handshake_storage_support)
[[ "$MODE" = mt164-capture || "$MODE" = backend-opt-diag ]] && EXTRACTED_CRATES=()

BUILD_START_MARKER="$LANE/tmp/build-start-$SHA"
touch "$BUILD_START_MARKER"
if [[ "$CORE_SKIP" != 1 ]]; then
echo "[run-round] building core union"
# -j 1: handshake_core lib and lib-test compiled in parallel hit rustc-LLVM out of memory on 1f4e0f69
# (2026-09-30, host shared with foreign builds); build them one at a time.
CORE_BUILD_LOG="$LANE/logs/core-build-v-$SHA.log"
echo "[run-round] build_profile line=core-test config_sha256=$BACKEND_PROFILE_SHA256 (dev/test + 8-crate opt-level 2) log=$CORE_BUILD_LOG"
( cd "$EXPORT/src/backend/handshake_core" && \
  cargo test --locked -j 1 --no-run -v --config "$BACKEND_PROFILE_CONFIG" --lib --features app-runtime,surreal-test-support,test-utils "${core_test_args[@]}" ) 2>&1 | tee "$CORE_BUILD_LOG"
check_test_profile core "$CORE_BUILD_LOG" 1
check_target_cap
fi

if [[ "$NATIVE_SKIP" != 1 ]]; then
echo "[run-round] building native union"
NATIVE_BUILD_LOG="$LANE/logs/native-build-$SHA.log"
echo "[run-round] build_profile line=native-test config_sha256=$BACKEND_PROFILE_SHA256 (inert: handshake_native graph has no surrealdb crate) log=$NATIVE_BUILD_LOG"
( cd "$EXPORT/src/frontend/handshake_native" && \
  cargo test --locked -j 2 --no-run --config "$BACKEND_PROFILE_CONFIG" --features integration,integration_tests,wgpu_screenshots "${NATIVE_TARGET_ARGS[@]}" ) 2>&1 | tee "$NATIVE_BUILD_LOG"
if grep -Eq '^ *Compiling (surrealdb|surrealdb-[a-z-]+) v' "$NATIVE_BUILD_LOG"; then
  echo "[run-round] INVALID_CONFIG native build compiled an allow-listed surrealdb crate (expected inert)"; exit 5
fi
echo "[run-round] test_profile_proof native inert (no allow-listed crate compiled)"
check_target_cap
fi

for crate in "${EXTRACTED_CRATES[@]}"; do
  echo "[run-round] building extracted $crate unit target"
  EXTRACTED_BUILD_LOG="$LANE/logs/extracted-build-v-$crate-$SHA.log"
  echo "[run-round] build_profile line=extracted-test:$crate config_sha256=$BACKEND_PROFILE_SHA256 log=$EXTRACTED_BUILD_LOG"
  ( cd "$EXPORT/src/backend/$crate" && \
    cargo test --locked -j 2 --no-run -v --config "$BACKEND_PROFILE_CONFIG" --lib --features surreal-test-support ) 2>&1 | tee "$EXTRACTED_BUILD_LOG"
  check_test_profile "extracted:$crate" "$EXTRACTED_BUILD_LOG" 0
  check_target_cap
done

if [[ "$NATIVE_SKIP" != 1 ]]; then
echo "[run-round] building backend binary for HSK_TEST_BACKEND_BIN"
echo "[run-round] build_profile line=backend config_sha256=$BACKEND_PROFILE_SHA256 (dev + 8-crate opt-level 2, -j 2)"
( cd "$EXPORT/src/backend/handshake_core" && \
  cargo build --locked -j 2 --config "$BACKEND_PROFILE_CONFIG" --target-dir "$HSK_TEST_BACKEND_TARGET_ROOT" --bin handshake_core --features app-runtime,surreal-test-support )
check_target_cap
[ -f "$HSK_TEST_BACKEND_BIN" ] || { echo "[run-round] FATAL: backend bin not found at $HSK_TEST_BACKEND_BIN"; exit 3; }

if [[ "$MODE" = backend-opt-diag ]]; then
  echo "[run-round] backend-opt-diag: native nextest MT-008 preflight skipped (would build unselected binaries)"
else
# Validate the changed native config against the just-built union binaries
# before any test executes. This is the same candidate/build, not a separate
# per-MT build or proof run.
echo "[run-round] native nextest selection/config preflight"
( cd "$EXPORT/src/frontend/handshake_native" && \
  "$NEXTEST" nextest list --locked --config-file "$LANE/nextest.toml" \
    --features integration,integration_tests,wgpu_screenshots \
    --test test_code_nav_client --test test_completion_hover_accesskit \
    > "$LANE/logs/native-nextest-list-$SHA.log" )
( cd "$EXPORT/src/frontend/handshake_native" && \
  "$NEXTEST" nextest show-config --config-file "$LANE/nextest.toml" \
    test-groups --no-pager --groups owned-backend \
    --features integration,integration_tests,wgpu_screenshots \
    --test test_code_nav_client --test test_completion_hover_accesskit \
    > "$LANE/logs/native-nextest-groups-$SHA.log" )
grep -q 'test_code_nav_client' "$LANE/logs/native-nextest-groups-$SHA.log" \
  || { echo "[run-round] FATAL: MT-008 code-nav absent from owned-backend group"; exit 3; }
grep -q 'test_completion_hover_accesskit' "$LANE/logs/native-nextest-groups-$SHA.log" \
  || { echo "[run-round] FATAL: MT-008 completion/hover absent from owned-backend group"; exit 3; }
echo "[run-round] native nextest MT-008 owned-backend group verified"
fi
fi

# HBR-COMPART-002 / [VPX-011]: per-round rebuild accounting. Only the changed crates and their dependents
# should compile; more is a rule defect to report.
COMPILED_CRATES="$(find "$TARGET/debug/deps" -maxdepth 1 \( -name '*.rlib' -o -name '*.rmeta' \) -newer "$BUILD_START_MARKER" -printf '%f\n' 2>/dev/null | sed -E 's/^lib//; s/-[0-9a-f]{16}\..*$//' | sort -u || true)"
RELINKED_BINS="$(find "$TARGET/debug/deps" -maxdepth 1 -name '*.exe' -newer "$BUILD_START_MARKER" 2>/dev/null | wc -l || true)"
BACKEND_RELINKED=0; [ -f "$HSK_TEST_BACKEND_BIN" ] && [ "$HSK_TEST_BACKEND_BIN" -nt "$BUILD_START_MARKER" ] && BACKEND_RELINKED=1
echo "[run-round] rebuild accounting: compiled_crates=$(printf '%s\n' "$COMPILED_CRATES" | grep -c .) relinked_test_binaries=$RELINKED_BINS backend_bin_relinked=$BACKEND_RELINKED refresh_mode=$WPV_EXPORT_MODE"
echo "[run-round] compiled crate names: $(printf '%s ' $COMPILED_CRATES)"

# 4. nextest run: core and native; the separately governed ignored proofs for
#    BLOCKED MT-068/098/140 await their bounded supervisors, not this round.
#    excluding the duplicated failure_diagnostic_tests module from every
#    native binary except its declared owner.
#    OWNER NOT YET CONFIRMED BY AUTHORITY: provisionally test_app_host_mount
#    (first binary in the union that includes tests/backend_proof_support
#    directly). Flagged to IV/builder 2026-09-24; correct OWNER_BIN below
#    once confirmed.
OWNER_BIN="test_app_host_mount"
EXCLUDE_FILTER="not (test(/backend_proof_support::failure_diagnostic_tests::/) and not binary($OWNER_BIN))"
[[ -n "${ROUND_SELECTION_SHA:-}" ]] || CORE_FILTER='not binary(handshake_core) or test(/^(api::flight_recorder::tests::document_saved_receipt_|storage::surreal::retry::tests::|storage::surreal::resource_authority_tests::|storage::surreal::schema::tests::(declarative_schema_catalog_is_complete_and_content_sensitive|mt139_current_schema_info_pin_matches_fresh_mem_catalog|mt109_loom_catalog_dependencies_are_complete_and_deterministic|mt138_canonical_atelier_catalog_fingerprint_matches_compiled_pin|mt138_full_schema_atelier_noop_matches_bounded_projection|mt109_authority_catalog_pins_are_deterministic|mt139_exact_predecessor_upgrade_preserves_data_and_restarts_current|canvas_receipt_revision_158_upgrade_requires_exact_catalog_and_restarts_current|standalone_loom_revision_159_upgrade_requires_exact_catalog_and_restarts_current|schema_delta_upgrade_statements_re_emit_every_mt154_delta|document_grant_revision_160_upgrade_requires_exact_catalog_and_restarts_current|indexed_grant_revision_161_upgrade_preserves_data_and_restarts_current|document_grant_single_live_policy_witness_controls_record_user_visibility)$|api::loom::tests::(mt153_loom_route_family_authority_matrix|mounted_record_user_loom_creates_are_atomic_and_denied_writes_leave_no_rows)$|api::workspaces::tests::(owned_workspace_delete_cascades_documents_versions_and_canvas_with_audit|mt109_c2_memory_surfaces_provisioned_and_process_routes_deny_by_default|mt154_owner_workspace_delete_removes_calendar_stage_canvas_rows)$|api::kernel::tests::|storage::surreal::mt136_database_surface_proof_(a|b|c)::|api::debug_adapter::|api::jobs::tests::(create_job_rejects_unknown_job_kind|create_job_allows_terminal_when_authorized|create_model_run_job_launches_runtime_session_and_preserves_native_binding)$)/)'
if [[ "$MODE" = mt164-capture ]]; then
  CORE_FILTER='binary(=handshake_core) & test(=storage::surreal::resource_authority_tests::grant_check_cost_is_independent_of_grant_count)'
  (set -o noclobber; date -u +%Y-%m-%dT%H:%M:%SZ > "$CAPTURE_MARKER") || exit 2
fi

# Test-process thread stacks: 10 MiB (MT-154 FAIL_V3 remediation, diagnosis
# MT164-UDF-AND-CHAIN-STACK-OVERFLOW; SurrealDB 3.2.0 embedded docs recommend 10 MiB).
# Exported after all builds so only test processes inherit it. Covers libtest test threads
# (test/src/lib.rs:695 thread::Builder::new() without stack_size) and tokio runtime workers
# (tokio-1.53.1 runtime/blocking/pool.rs:463-467 sets stack_size only when configured);
# std resolves unset sizes from RUST_MIN_STACK (std/src/thread/lifecycle.rs:29-45, rust 1.91.1).
# Threads with an explicit Builder::stack_size (e.g. storage/tests.rs test store runtime) keep it.
export RUST_MIN_STACK=10485760
echo "[run-round] test-process RUST_MIN_STACK=$RUST_MIN_STACK"
if [[ "$CORE_SKIP" = 1 ]]; then
  echo "[run-round] $MODE mode: core build and run skipped"
  CORE_NEXTEST_EXIT=skipped
else
echo "[run-round] nextest CORE run"
CORE_JUNIT="$EXPORT/src/backend/handshake_core/target/nextest/default/junit.xml"
CORE_JUNIT_MARKER="$LANE/tmp/core-junit-start-$SHA"
touch "$CORE_JUNIT_MARKER"
# nextest exits non-zero (100) whenever any test fails, even with --no-fail-fast
# letting the full suite run to completion; that non-zero exit is expected and
# must NOT abort this script under `set -e` (run50, 2026-09-24, aborted here
# before the native phase ever ran because a real test failure alone tripped
# `set -e`). Capture the exit code explicitly instead of relying on script-level
# error propagation, and only treat it as fatal if nextest exits abnormally
# (>1: crash / usage error), not on 100 (test run failed).
set +e
( cd "$EXPORT/src/backend/handshake_core" && \
  "$NEXTEST" nextest run --locked --no-fail-fast \
    --config-file "$LANE/nextest-core.toml" \
    --features app-runtime,surreal-test-support,test-utils -E "$CORE_FILTER" --lib "${core_test_args[@]}" )
CORE_NEXTEST_EXIT=$?
set -e
if [[ "$CORE_NEXTEST_EXIT" != 0 && "$CORE_NEXTEST_EXIT" != 100 ]]; then
  echo "[run-round] CORE_NEXTEST_ABNORMAL exit=$CORE_NEXTEST_EXIT"
  CORE_INVALID=1
fi
if [[ "${CORE_INVALID:-0}" != 1 && -f "$CORE_JUNIT" && "$CORE_JUNIT" -nt "$CORE_JUNIT_MARKER" ]]; then
  CORE_TEST_COUNT="$(sed -n 's/^<testsuites[^>]* tests="\([0-9][0-9]*\)".*/\1/p' "$CORE_JUNIT" | head -n 1)"
  if [[ -n "$CORE_TEST_COUNT" && "$CORE_TEST_COUNT" -gt 0 ]]; then
    cp "$CORE_JUNIT" "$LANE/$CORE_JUNIT_NAME"
    echo "[run-round] core nextest exit=$CORE_NEXTEST_EXIT tests=$CORE_TEST_COUNT"
  else
    echo "[run-round] CORE_ZERO_OR_UNPARSEABLE_TEST_COUNT: $CORE_JUNIT"
    CORE_INVALID=1
  fi
else
  echo "[run-round] CORE_JUNIT_MISSING_OR_STALE: $CORE_JUNIT"
  CORE_INVALID=1
fi
fi

if [[ "$NATIVE_SKIP" = 1 ]]; then
  echo "[run-round] $MODE mode: native build and run skipped"
  NATIVE_NEXTEST_EXIT=skipped
else
echo "[run-round] nextest NATIVE run (excluding duplicated failure_diagnostic_tests except $OWNER_BIN)"
NATIVE_JUNIT="$EXPORT/src/frontend/handshake_native/target/nextest/default/junit.xml"
NATIVE_JUNIT_MARKER="$LANE/tmp/native-junit-start-$SHA"
touch "$NATIVE_JUNIT_MARKER"
if [[ "$MODE" = backend-opt-diag ]]; then
  (set -o noclobber; date -u +%Y-%m-%dT%H:%M:%SZ > "$LANE/BACKEND-OPT-DIAG-${SHA:0:8}.started") || exit 2
fi
set +e
( cd "$EXPORT/src/frontend/handshake_native" && \
  "$NEXTEST" nextest run --locked --no-fail-fast \
    --config-file "$LANE/nextest.toml" \
    --features integration,integration_tests,wgpu_screenshots -E "${NATIVE_FILTER:-$EXCLUDE_FILTER}" "${NATIVE_TARGET_ARGS[@]}" )
NATIVE_NEXTEST_EXIT=$?
set -e
if [[ "$NATIVE_NEXTEST_EXIT" != 0 && "$NATIVE_NEXTEST_EXIT" != 100 ]]; then
  echo "[run-round] NATIVE_NEXTEST_ABNORMAL exit=$NATIVE_NEXTEST_EXIT"
  NATIVE_INVALID=1
fi
if [[ "${NATIVE_INVALID:-0}" != 1 && -f "$NATIVE_JUNIT" && "$NATIVE_JUNIT" -nt "$NATIVE_JUNIT_MARKER" ]]; then
  NATIVE_TEST_COUNT="$(sed -n 's/^<testsuites[^>]* tests="\([0-9][0-9]*\)".*/\1/p' "$NATIVE_JUNIT" | head -n 1)"
  if [[ -n "$NATIVE_TEST_COUNT" && "$NATIVE_TEST_COUNT" -gt 0 ]]; then
    cp "$NATIVE_JUNIT" "$LANE/$NATIVE_JUNIT_NAME"
    echo "[run-round] native nextest exit=$NATIVE_NEXTEST_EXIT tests=$NATIVE_TEST_COUNT"
  else
    echo "[run-round] NATIVE_ZERO_OR_UNPARSEABLE_TEST_COUNT: $NATIVE_JUNIT"
    NATIVE_INVALID=1
  fi
else
  echo "[run-round] NATIVE_JUNIT_MISSING_OR_STALE: $NATIVE_JUNIT"
  NATIVE_INVALID=1
fi
fi

EXTRACTED_INVALID=0
EXTRACTED_RESULTS=()
for crate in "${EXTRACTED_CRATES[@]}"; do
  echo "[run-round] nextest extracted $crate unit run"
  CRATE_JUNIT="$EXPORT/src/backend/$crate/target/nextest/default/junit.xml"
  CRATE_JUNIT_MARKER="$LANE/tmp/$crate-junit-start-$SHA"
  touch "$CRATE_JUNIT_MARKER"
  set +e
  ( cd "$EXPORT/src/backend/$crate" && \
    "$NEXTEST" nextest run --locked --no-fail-fast \
      --config-file "$LANE/nextest-core.toml" --lib --features surreal-test-support )
  CRATE_NEXTEST_EXIT=$?
  set -e
  EXTRACTED_RESULTS+=("$crate:exit=$CRATE_NEXTEST_EXIT")
  if [[ "$CRATE_NEXTEST_EXIT" != 0 && "$CRATE_NEXTEST_EXIT" != 100 ]]; then
    echo "[run-round] EXTRACTED_NEXTEST_ABNORMAL crate=$crate exit=$CRATE_NEXTEST_EXIT"
    EXTRACTED_INVALID=1
  elif [[ -f "$CRATE_JUNIT" && "$CRATE_JUNIT" -nt "$CRATE_JUNIT_MARKER" ]]; then
    CRATE_TEST_COUNT="$(sed -n 's/^<testsuites[^>]* tests="\([0-9][0-9]*\)".*/\1/p' "$CRATE_JUNIT" | head -n 1)"
    if [[ -n "$CRATE_TEST_COUNT" && "$CRATE_TEST_COUNT" -gt 0 ]]; then
      cp "$CRATE_JUNIT" "$LANE/junit-$SHA-$crate.xml"
      echo "[run-round] extracted crate=$crate exit=$CRATE_NEXTEST_EXIT tests=$CRATE_TEST_COUNT"
    else
      echo "[run-round] EXTRACTED_ZERO_OR_UNPARSEABLE_TEST_COUNT crate=$crate"
      EXTRACTED_INVALID=1
    fi
  else
    echo "[run-round] EXTRACTED_JUNIT_MISSING_OR_STALE crate=$crate path=$CRATE_JUNIT"
    EXTRACTED_INVALID=1
  fi
done

if [[ "${CORE_INVALID:-0}" == 1 || "${NATIVE_INVALID:-0}" == 1 || "$EXTRACTED_INVALID" == 1 ]]; then
  echo "[run-round] INVALID_ROUND_RESULT core_exit=$CORE_NEXTEST_EXIT native_exit=$NATIVE_NEXTEST_EXIT extracted_invalid=$EXTRACTED_INVALID extracted=${EXTRACTED_RESULTS[*]}"
  exit 4
fi
echo "[run-round] done. junit: $LANE/$CORE_JUNIT_NAME , $LANE/$NATIVE_JUNIT_NAME , $LANE/junit-$SHA-handshake_document.xml , $LANE/junit-$SHA-handshake_storage_support.xml (core_exit=$CORE_NEXTEST_EXIT native_exit=$NATIVE_NEXTEST_EXIT extracted=${EXTRACTED_RESULTS[*]})"
