#!/bin/bash
# MT154 PIN-MEASURE only: one batched schema measurement, not native replay or acceptance.
set -euo pipefail
SHA="${1:?full candidate SHA required}"
WORKTREE="${2:?product worktree required}"
LANE="${3:?existing wpv-c3x lane required}"
TARGET="${4:?existing C warm target required}"
NEXTEST="${5:?pinned nextest executable required}"
ARTIFACTS="${6:?canonical artifacts root required}"
CONSUMED="$LANE/MT154-V14-PIN-MEASURE.started"
[[ "$SHA" =~ ^[0-9a-f]{40}$ && ! -e "$CONSUMED" ]] || exit 2
[[ -d "$LANE" && -d "$TARGET" && -x "$NEXTEST" ]] || exit 2
export GIT_TERMINAL_PROMPT=0 GCM_INTERACTIVE=never
[[ "$(sha256sum "$LANE/nextest-core.toml" | cut -d ' ' -f1)" = a974ab0cc35118a5e825b4f8af8f131360c92ca4de11d87293f0d08368123d2a ]] || exit 2
[[ -z "$(git -C "$WORKTREE" status --porcelain)" ]] || exit 2
[[ "$(git -C "$WORKTREE" rev-parse HEAD)" = "$SHA" ]] || exit 2
[[ "$(git -C "$WORKTREE" ls-remote origin refs/heads/feat/WP-KERNEL-012 | cut -f1)" = "$SHA" ]] || exit 2
check_cap() {
  local bytes free_kib reserve="${1:-0}"
  bytes="$(find "$TARGET" -type f -printf '%s\n' | awk '{s += $1} END {printf "%.0f", s}')"
  free_kib="$(df -Pk "$TARGET" | awk 'NR == 2 {print $4}')"
  [[ "$bytes" =~ ^[0-9]+$ && "$free_kib" =~ ^[0-9]+$ ]] || exit 2
  echo "MT154_PIN_MEASURE target_bytes=$bytes reserve=$reserve cap=150000000000 free_kib=$free_kib"
  (( bytes + reserve <= 150000000000 && free_kib >= 187500000 )) || exit 2
}
check_cap 4000000000
EXPORT="$TARGET/export-${SHA:0:8}"
MARKER="$TARGET/export-${SHA:0:8}.sha"
[[ ! -e "$EXPORT" && ! -e "$MARKER" ]] || exit 2
mkdir "$EXPORT"
git -C "$WORKTREE" archive "$SHA" | tar -x -C "$EXPORT"
printf '%s' "$SHA" > "$MARKER"
export CARGO_TARGET_DIR="$TARGET" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only
export HANDSHAKE_PROOF_SOURCE_SHA="$SHA" HANDSHAKE_ARTIFACTS_ROOT="$ARTIFACTS"
export HANDSHAKE_TEST_ARTIFACTS_ROOT="$LANE/e" HANDSHAKE_TEST_STAGE_BINDING_ROOT="$LANE/stage-binding"
export HANDSHAKE_TEST_SURREAL_SYNC=never SURREAL_DATASTORE_SYNC=never
export HANDSHAKE_SURREAL_TEST_STORE_ROOT="$LANE/runtime" HANDSHAKE_WORKSPACE_ROOT="$LANE/workspace-root"
export TMP="$LANE/tmp" TEMP="$LANE/tmp" TMPDIR="$LANE/tmp" HS_LOG_LEVEL=info
[[ -z "${NEXTEST_RETRIES:-}" && -z "${NEXTEST_PROFILE:-}" ]] || exit 2
mkdir -p "$LANE/logs" "$LANE/tmp" "$LANE/runtime" "$LANE/e" "$LANE/stage-binding" "$LANE/workspace-root"
FILTER='binary(=handshake_core) & test(/^storage::surreal::schema::tests::(declarative_schema_catalog_is_complete_and_content_sensitive|mt139_current_schema_info_pin_matches_fresh_mem_catalog|mt109_loom_catalog_dependencies_are_complete_and_deterministic|mt138_canonical_atelier_catalog_fingerprint_matches_compiled_pin|mt138_full_schema_atelier_noop_matches_bounded_projection|mt109_authority_catalog_pins_are_deterministic|mt139_exact_predecessor_upgrade_preserves_data_and_restarts_current)$/)'
echo 'MT154_PIN_MEASURE compile selected owning library once'
(cd "$EXPORT/src/backend/handshake_core" && cargo test --locked -j 2 --no-run --lib \
  --features app-runtime,surreal-test-support,test-utils)
check_cap
find "$TARGET/debug/deps" -maxdepth 1 -type f -name 'handshake_core-*.exe' -newer "$MARKER" -exec sha256sum '{}' \; \
  > "$LANE/logs/mt154-v14-pin-measure-$SHA.binaries.sha256"
[[ -s "$LANE/logs/mt154-v14-pin-measure-$SHA.binaries.sha256" ]] || exit 4
INVOCATION="$LANE/mt154-v14-pin-measure-$SHA.started"
[[ ! -e "$INVOCATION" ]] || exit 2
(set -o noclobber; printf '%s' "$SHA" > "$CONSUMED") || exit 2
date -u +%Y-%m-%dT%H:%M:%SZ > "$INVOCATION"
JUNIT="$EXPORT/src/backend/handshake_core/target/nextest/default/junit.xml"
set +e
(cd "$EXPORT/src/backend/handshake_core" && "$NEXTEST" nextest run --locked --no-fail-fast --build-jobs 2 \
  --config-file "$LANE/nextest-core.toml" --features app-runtime,surreal-test-support,test-utils \
  --success-output final --lib -E "$FILTER") 2>&1 | tee "$LANE/logs/mt154-v14-pin-measure-$SHA.tests.log"
RESULT=${PIPESTATUS[0]}
set -e
[[ "$RESULT" = 0 || "$RESULT" = 100 ]] || exit "$RESULT"
[[ -f "$JUNIT" && "$JUNIT" -nt "$INVOCATION" ]] || exit 4
COUNT="$(sed -n 's/^<testsuites[^>]* tests="\([0-9][0-9]*\)".*/\1/p' "$JUNIT" | head -n 1)"
[[ "$COUNT" = 7 ]] || exit 4
cp "$JUNIT" "$LANE/junit-$SHA-mt154-v14-pin-measure.xml"
grep -E 'MT109_LOOM_CATALOG_SHA256=|MT109_CURRENT_AUTHORITY_INFO_SHA256=|MT109_PREDECESSOR_AUTHORITY_INFO_SHA256=|MT139_CURRENT_SCHEMA_INFO_SHA256=|EXPECTED_ATELIER_CATALOG_SHA256=|left:' \
  "$LANE/logs/mt154-v14-pin-measure-$SHA.tests.log" > "$LANE/logs/mt154-v14-pin-measure-$SHA.pins.txt" || true
[[ -s "$LANE/logs/mt154-v14-pin-measure-$SHA.pins.txt" ]] || exit 4
sha256sum "$LANE/junit-$SHA-mt154-v14-pin-measure.xml" "$LANE/logs/mt154-v14-pin-measure-$SHA.tests.log" \
  "$LANE/logs/mt154-v14-pin-measure-$SHA.pins.txt" "$LANE/logs/mt154-v14-pin-measure-$SHA.binaries.sha256" \
  "$EXPORT/src/backend/handshake_core/src/storage/surreal/schema.surql" "$EXPORT/src/backend/handshake_core/src/storage/surreal/schema.rs" "$LANE/nextest-core.toml"
check_cap
echo "MT154_PIN_MEASURE completed nextest_exit=$RESULT tests=$COUNT; measurement only"
