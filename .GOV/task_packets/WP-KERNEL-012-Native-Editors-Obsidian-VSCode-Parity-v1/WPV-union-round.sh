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

SHA="${1:?usage: run-round.sh <SHA>}"
[[ "$SHA" =~ ^[0-9a-f]{40}$ ]] || { echo "[run-round] FATAL: full lowercase 40-character SHA required"; exit 2; }
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
[ "$(git -C "$WORKTREE" rev-parse HEAD)" = "$SHA" ] || { echo "[run-round] FATAL: candidate is not builder HEAD"; exit 2; }
REMOTE_SHA="$(git -C "$WORKTREE" ls-remote origin refs/heads/feat/WP-KERNEL-012 | cut -f1)"
[ "$REMOTE_SHA" = "$SHA" ] || { echo "[run-round] FATAL: candidate is not pushed branch tip ($REMOTE_SHA)"; exit 2; }
check_target_cap

mkdir -p "$LANE/logs" "$LANE/tmp" "$LANE/runtime" "$LANE/workspace"

# 1. Select an immutable archive. Never delete or overlay an earlier export.
EXPORT="$EXPORT_ROOT/export-${SHA:0:8}"
MARKER="$EXPORT_ROOT/export-${SHA:0:8}.sha"
if [ -d "$EXPORT" ]; then
  [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "$SHA" ] \
    || { echo "[run-round] FATAL: existing export is incomplete or belongs to another SHA: $EXPORT"; exit 2; }
  echo "[run-round] reusing archived $SHA at $EXPORT"
else
  echo "[run-round] creating new export for $SHA at $EXPORT"
  mkdir -p "$EXPORT"
  git -C "$WORKTREE" archive "$SHA" | tar -x -C "$EXPORT"
  printf '%s' "$SHA" > "$MARKER"
fi
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

# 2. Build core union (--no-run), native union (--no-run), and the backend
#    binary, one cargo invocation per crate, no timeout wrapper.
CORE_TESTS=(
  knowledge_documents_api_tests loom_atomic_receipt_tests
  loom_block_collection_views_tests wp_kernel_012_native_editor_routes_tests
  loom_daily_journal_tests loom_transclusion_tests loom_media_tiers_tests
  project_wiki_drift_tests mt154_non_loom_route_authority_tests
  knowledge_crdt_bridge_api_tests knowledge_ingestion_api_tests
  knowledge_memory_api_tests knowledge_retrieval_debug_api_tests
  atelier_stealth_window_tests atelier_loom_projection_api_tests
  calendar_storage_tests mt155_product_screenshot_capture_route_auth_tests
  kernel_product_screenshot_capture_tests mt157_debug_breakpoints_authority_tests
  micro_task_executor_tests mt151_early_lock_release_race_tests
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
NATIVE_TARGET_ARGS=(--lib)
for t in "${NATIVE_TESTS[@]}"; do NATIVE_TARGET_ARGS+=(--test "$t"); done

core_test_args=(); for t in "${CORE_TESTS[@]}"; do core_test_args+=(--test "$t"); done

echo "[run-round] building core union"
( cd "$EXPORT/src/backend/handshake_core" && \
  cargo test --locked -j 2 --no-run --lib --features app-runtime,surreal-test-support,test-utils "${core_test_args[@]}" )
check_target_cap

echo "[run-round] building native union"
( cd "$EXPORT/src/frontend/handshake_native" && \
  cargo test --locked -j 2 --no-run --features integration,integration_tests,wgpu_screenshots "${NATIVE_TARGET_ARGS[@]}" )
check_target_cap

echo "[run-round] building backend binary for HSK_TEST_BACKEND_BIN"
( cd "$EXPORT/src/backend/handshake_core" && \
  cargo build --locked --target-dir "$HSK_TEST_BACKEND_TARGET_ROOT" --bin handshake_core --features app-runtime,surreal-test-support )
check_target_cap
[ -f "$HSK_TEST_BACKEND_BIN" ] || { echo "[run-round] FATAL: backend bin not found at $HSK_TEST_BACKEND_BIN"; exit 3; }

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
CORE_FILTER='not binary(handshake_core) or test(/^(api::flight_recorder::tests::document_saved_receipt_|storage::surreal::schema::tests::(declarative_schema_catalog_is_complete_and_content_sensitive|mt139_current_schema_info_pin_matches_fresh_mem_catalog|mt109_loom_catalog_dependencies_are_complete_and_deterministic|mt138_canonical_atelier_catalog_fingerprint_matches_compiled_pin|mt138_full_schema_atelier_noop_matches_bounded_projection|mt109_authority_catalog_pins_are_deterministic|mt139_exact_predecessor_upgrade_preserves_data_and_restarts_current|canvas_receipt_revision_158_upgrade_requires_exact_catalog_and_restarts_current|standalone_loom_revision_159_upgrade_requires_exact_catalog_and_restarts_current|schema_delta_upgrade_statements_re_emit_every_mt154_delta)$|api::loom::tests::(mt153_loom_route_family_authority_matrix|mounted_record_user_loom_creates_are_atomic_and_denied_writes_leave_no_rows)$|api::workspaces::tests::(owned_workspace_delete_cascades_documents_versions_and_canvas_with_audit|mt109_c2_memory_surfaces_provisioned_and_process_routes_deny_by_default|mt154_owner_workspace_delete_removes_calendar_stage_canvas_rows)$|api::kernel::tests::|storage::surreal::mt136_database_surface_proof_(a|b|c)::|api::debug_adapter::|api::jobs::tests::(create_job_rejects_unknown_job_kind|create_job_allows_terminal_when_authorized)$)/)'

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
    cp "$CORE_JUNIT" "$LANE/junit-$SHA-core.xml"
    echo "[run-round] core nextest exit=$CORE_NEXTEST_EXIT tests=$CORE_TEST_COUNT"
  else
    echo "[run-round] CORE_ZERO_OR_UNPARSEABLE_TEST_COUNT: $CORE_JUNIT"
    CORE_INVALID=1
  fi
else
  echo "[run-round] CORE_JUNIT_MISSING_OR_STALE: $CORE_JUNIT"
  CORE_INVALID=1
fi

echo "[run-round] nextest NATIVE run (excluding duplicated failure_diagnostic_tests except $OWNER_BIN)"
NATIVE_JUNIT="$EXPORT/src/frontend/handshake_native/target/nextest/default/junit.xml"
NATIVE_JUNIT_MARKER="$LANE/tmp/native-junit-start-$SHA"
touch "$NATIVE_JUNIT_MARKER"
set +e
( cd "$EXPORT/src/frontend/handshake_native" && \
  "$NEXTEST" nextest run --locked --no-fail-fast \
    --config-file "$LANE/nextest.toml" \
    --features integration,integration_tests,wgpu_screenshots -E "$EXCLUDE_FILTER" "${NATIVE_TARGET_ARGS[@]}" )
NATIVE_NEXTEST_EXIT=$?
set -e
if [[ "$NATIVE_NEXTEST_EXIT" != 0 && "$NATIVE_NEXTEST_EXIT" != 100 ]]; then
  echo "[run-round] NATIVE_NEXTEST_ABNORMAL exit=$NATIVE_NEXTEST_EXIT"
  NATIVE_INVALID=1
fi
if [[ "${NATIVE_INVALID:-0}" != 1 && -f "$NATIVE_JUNIT" && "$NATIVE_JUNIT" -nt "$NATIVE_JUNIT_MARKER" ]]; then
  NATIVE_TEST_COUNT="$(sed -n 's/^<testsuites[^>]* tests="\([0-9][0-9]*\)".*/\1/p' "$NATIVE_JUNIT" | head -n 1)"
  if [[ -n "$NATIVE_TEST_COUNT" && "$NATIVE_TEST_COUNT" -gt 0 ]]; then
    cp "$NATIVE_JUNIT" "$LANE/junit-$SHA-native.xml"
    echo "[run-round] native nextest exit=$NATIVE_NEXTEST_EXIT tests=$NATIVE_TEST_COUNT"
  else
    echo "[run-round] NATIVE_ZERO_OR_UNPARSEABLE_TEST_COUNT: $NATIVE_JUNIT"
    NATIVE_INVALID=1
  fi
else
  echo "[run-round] NATIVE_JUNIT_MISSING_OR_STALE: $NATIVE_JUNIT"
  NATIVE_INVALID=1
fi

if [[ "${CORE_INVALID:-0}" == 1 || "${NATIVE_INVALID:-0}" == 1 ]]; then
  echo "[run-round] INVALID_ROUND_RESULT core_exit=$CORE_NEXTEST_EXIT native_exit=$NATIVE_NEXTEST_EXIT"
  exit 4
fi
echo "[run-round] done. junit: $LANE/junit-$SHA-core.xml , $LANE/junit-$SHA-native.xml (core_exit=$CORE_NEXTEST_EXIT native_exit=$NATIVE_NEXTEST_EXIT)"
