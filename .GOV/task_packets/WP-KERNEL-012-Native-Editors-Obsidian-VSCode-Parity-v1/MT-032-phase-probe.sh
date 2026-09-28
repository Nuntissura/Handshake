#!/bin/bash
# Approved MT032-STATEMENT-DIAGNOSTIC-20260927 extension diagnostic only; never an acceptance union.
set -euo pipefail
SHA="${1:?full candidate SHA required}"
WORKTREE="${2:?product worktree required}"
LANE="${3:?existing wpv-c3x lane required}"
TARGET="${4:?existing C warm target required}"
NEXTEST="${5:?pinned nextest executable required}"
ARTIFACTS="${6:?canonical artifacts root required}"
MODE="${7:-}"
[[ -z "$MODE" || "$MODE" = receipt-inner-v15 ]] || exit 2
CONSUMED="$LANE/MT032-V14-STATEMENT-DIAGNOSTIC.started"
PROBE=mt032-v14-statement-probe
WATCH=mt032-v14-statement-watch
WATCH_SCHEMA=handshake.mt032.statement-watch.ready.v14.1
if [[ "$MODE" = receipt-inner-v15 ]]; then
  CONSUMED="$LANE/MT032-V15-RECEIPT-DIAGNOSTIC.started"
  PROBE=mt032-v15-receipt-probe
  WATCH=mt032-v15-receipt-watch
  WATCH_SCHEMA=handshake.mt032.receipt-watch.ready.v15.1
fi
[[ "$SHA" =~ ^[0-9a-f]{40}$ ]] || exit 2
[[ ! -e "$CONSUMED" ]] || { echo 'MT032_PROBE approval already consumed; no automatic replay'; exit 2; }
export GIT_TERMINAL_PROMPT=0 GCM_INTERACTIVE=never
[[ -d "$LANE" && -d "$TARGET" && -x "$NEXTEST" ]] || exit 2
[[ "$(sha256sum "$LANE/nextest.toml" | cut -d ' ' -f1)" = d828376108c1d72836b94677f1612378095310dee2ed96e2ed12acb3929706a8 ]] || exit 2
if [[ "$MODE" = receipt-inner-v15 ]]; then
  [[ "$(sha256sum "$LANE/nextest-core.toml" | cut -d ' ' -f1)" = a974ab0cc35118a5e825b4f8af8f131360c92ca4de11d87293f0d08368123d2a ]] || exit 2
fi
[[ -z "$(git -C "$WORKTREE" status --porcelain)" ]] || exit 2
[[ "$(git -C "$WORKTREE" rev-parse HEAD)" = "$SHA" ]] || exit 2
[[ "$(git -C "$WORKTREE" ls-remote origin refs/heads/feat/WP-KERNEL-012 | cut -f1)" = "$SHA" ]] || exit 2

check_watcher() {
  local ready="$LANE/logs/$WATCH-$SHA.ready.json"
  [[ -f "$ready" && ! -e "$LANE/logs/$WATCH-$SHA.summary.json" ]] || {
    echo 'MT032_PROBE phase watcher not ready or already stopped'; exit 2;
  }
  grep -Fq "\"schema\":\"$WATCH_SCHEMA\"" "$ready" || exit 2
  grep -Fq "\"candidate_sha\":\"$SHA\"" "$ready" || exit 2
}
check_watcher

check_cap() {
  local bytes free_kib reserve="${1:-0}"
  bytes="$(find "$TARGET" -type f -printf '%s\n' | awk '{s += $1} END {printf "%.0f", s}')"
  free_kib="$(df -Pk "$TARGET" | awk 'NR == 2 {print $4}')"
  [[ "$bytes" =~ ^[0-9]+$ && "$free_kib" =~ ^[0-9]+$ ]] || exit 2
  echo "MT032_PROBE target_bytes=$bytes reserve=$reserve cap=150000000000 free_kib=$free_kib"
  (( bytes + reserve <= 150000000000 && free_kib >= 187500000 )) || exit 2
}
check_cap 4000000000
EXPORT="$TARGET/export-${SHA:0:8}"
MARKER="$TARGET/export-${SHA:0:8}.sha"
[[ ! -e "$EXPORT" && ! -e "$MARKER" ]] || { echo 'MT032_PROBE fresh export required; existing contents preserved'; exit 2; }
mkdir "$EXPORT"
git -C "$WORKTREE" archive "$SHA" | tar -x -C "$EXPORT"
printf '%s' "$SHA" > "$MARKER"

export CARGO_TARGET_DIR="$TARGET"
export CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS=2
export HANDSHAKE_PROOF_SOURCE_SHA="$SHA" HANDSHAKE_ARTIFACTS_ROOT="$ARTIFACTS"
export HANDSHAKE_TEST_ARTIFACTS_ROOT="$LANE/e" HANDSHAKE_TEST_STAGE_BINDING_ROOT="$LANE/stage-binding"
export HSK_TEST_BACKEND_TARGET_ROOT="$LANE/backend-bin"
export HSK_TEST_BACKEND_BIN="$HSK_TEST_BACKEND_TARGET_ROOT/debug/handshake_core.exe"
export HANDSHAKE_TEST_SURREAL_SYNC=never SURREAL_DATASTORE_SYNC=never
export HANDSHAKE_SURREAL_TEST_STORE_ROOT="$LANE/runtime" HANDSHAKE_GPU_SCREENSHOT=1
export HANDSHAKE_WORKSPACE_ROOT="$LANE/workspace-root"
export TMP="$LANE/tmp" TEMP="$LANE/tmp" TMPDIR="$LANE/tmp" HS_LOG_LEVEL=info
[[ -z "${NEXTEST_RETRIES:-}" && -z "${NEXTEST_PROFILE:-}" ]] || exit 2
mkdir -p "$LANE/logs" "$LANE/tmp" "$LANE/runtime" "$LANE/e" "$LANE/stage-binding" "$LANE/workspace-root"

# Preserve the referenced prior backend artifact before overwriting the warm output.
if [[ -f "$HSK_TEST_BACKEND_BIN" ]]; then
  prior_hash="$(sha256sum "$HSK_TEST_BACKEND_BIN" | cut -d ' ' -f1)"
  mkdir -p "$LANE/backend-history"
  [[ -f "$LANE/backend-history/$prior_hash.exe" ]] || cp "$HSK_TEST_BACKEND_BIN" "$LANE/backend-history/$prior_hash.exe"
  [[ "$(sha256sum "$LANE/backend-history/$prior_hash.exe" | cut -d ' ' -f1)" = "$prior_hash" ]] || exit 2
fi
if [[ "$MODE" = receipt-inner-v15 ]]; then
  CORE_INVOCATION="$LANE/$PROBE-$SHA-core.started"
  CORE_RESULT_PATH="$LANE/$PROBE-$SHA-core.exit"
  CORE_JUNIT="$EXPORT/src/backend/handshake_core/target/nextest/default/junit.xml"
  CORE_RETAINED_JUNIT="$LANE/junit-$SHA-$PROBE-core.xml"
  [[ ! -e "$CORE_INVOCATION" && ! -e "$CORE_RESULT_PATH" && ! -e "$CORE_RETAINED_JUNIT" ]] || exit 2
  # Consume this distinct V15 diagnostic before its first proof invocation.
  (set -o noclobber; printf '%s' "$SHA" > "$CONSUMED") || exit 2
  (set -o noclobber; date -u +%Y-%m-%dT%H:%M:%SZ > "$CORE_INVOCATION") || exit 2
  CORE_FILTER='test(=storage::surreal::event_ledger::tests::measured_receipt_decoding_preserves_empty_denial_and_rejects_extra_rows) | test(=storage::surreal::event_ledger::tests::measured_receipt_append_preserves_exact_replay_and_conflict) | test(=storage::surreal::resource_authority_tests::direct_record_user_foreign_table_operations_are_default_deny)'
  echo 'MT032_PROBE core receipt predecessors'
  set +e
  (cd "$EXPORT/src/backend/handshake_core" && "$NEXTEST" nextest run --locked --no-fail-fast --build-jobs 2 \
    --config-file "$LANE/nextest-core.toml" --features app-runtime,surreal-test-support,test-utils --lib -E "$CORE_FILTER")
  CORE_RESULT=$?
  set -e
  (set -o noclobber; printf '%s\n' "$CORE_RESULT" > "$CORE_RESULT_PATH") || exit 2
  if [[ -f "$CORE_JUNIT" && "$CORE_JUNIT" -nt "$CORE_INVOCATION" ]]; then
    cp "$CORE_JUNIT" "$CORE_RETAINED_JUNIT"
    sha256sum "$CORE_RETAINED_JUNIT"
  fi
  [[ "$CORE_RESULT" = 0 ]] || { echo "MT032_PROBE core predecessors failed exit=$CORE_RESULT; native not started"; exit "$CORE_RESULT"; }
  [[ -f "$CORE_RETAINED_JUNIT" ]] || exit 4
  CORE_COUNT="$(sed -n 's/^<testsuites[^>]* tests="\([0-9][0-9]*\)".*/\1/p' "$CORE_RETAINED_JUNIT" | head -n 1)"
  [[ "$CORE_COUNT" = 3 ]] || { echo "MT032_PROBE invalid core test count=$CORE_COUNT"; exit 4; }
  check_cap
  check_watcher
fi
echo 'MT032_PROBE native compile'
(cd "$EXPORT/src/frontend/handshake_native" && cargo test --locked -j 2 --no-run \
  --features integration,integration_tests,wgpu_screenshots --test test_loom_address)
check_cap
echo 'MT032_PROBE backend compile'
(cd "$EXPORT/src/backend/handshake_core" && cargo build --locked -j 2 \
  --target-dir "$HSK_TEST_BACKEND_TARGET_ROOT" --bin handshake_core --features app-runtime,surreal-test-support)
[[ -f "$HSK_TEST_BACKEND_BIN" ]] || exit 3
check_cap

# WPV also verifies watcher process identity/liveness and continuously supervises it.
check_watcher
INVOCATION="$LANE/$PROBE-$SHA.started"
[[ ! -e "$INVOCATION" ]] || { echo 'MT032_PROBE already invoked; no automatic replay'; exit 2; }
if [[ "$MODE" != receipt-inner-v15 ]]; then
  (set -o noclobber; printf '%s' "$SHA" > "$CONSUMED") || exit 2
fi
date -u +%Y-%m-%dT%H:%M:%SZ > "$INVOCATION"
JUNIT="$EXPORT/src/frontend/handshake_native/target/nextest/default/junit.xml"
FILTER='binary(=test_loom_address) & (test(=live_surrealdb_owned_restart_preserves_document_backlink_and_content_hash) | test(=live_surrealdb_self_seeded_loom_block_backlink_hash_and_ui_proof))'
set +e
(cd "$EXPORT/src/frontend/handshake_native" && "$NEXTEST" nextest run --locked --no-fail-fast --build-jobs 2 \
  --config-file "$LANE/nextest.toml" --features integration,integration_tests,wgpu_screenshots \
  --test test_loom_address -E "$FILTER")
RESULT=$?
set -e
if [[ "$MODE" = receipt-inner-v15 ]]; then
  (set -o noclobber; printf '%s\n' "$RESULT" > "$LANE/$PROBE-$SHA.exit") || exit 2
fi
[[ "$RESULT" = 0 || "$RESULT" = 100 ]] || exit "$RESULT"
[[ -f "$JUNIT" && "$JUNIT" -nt "$INVOCATION" ]] || exit 4
COUNT="$(sed -n 's/^<testsuites[^>]* tests="\([0-9][0-9]*\)".*/\1/p' "$JUNIT" | head -n 1)"
[[ "$COUNT" = 2 ]] || { echo "MT032_PROBE invalid test count=$COUNT"; exit 4; }
cp "$JUNIT" "$LANE/junit-$SHA-$PROBE.xml"
sha256sum "$LANE/junit-$SHA-$PROBE.xml" "$HSK_TEST_BACKEND_BIN"
check_cap
echo "MT032_PROBE completed nextest_exit=$RESULT tests=$COUNT; diagnostic only"
