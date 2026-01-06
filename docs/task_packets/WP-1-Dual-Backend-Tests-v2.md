# Task Packet: WP-1-Dual-Backend-Tests-v2

## METADATA
- TASK_ID: WP-1-Dual-Backend-Tests-v2
- WP_ID: WP-1-Dual-Backend-Tests-v2
- DATE: 2026-01-06T22:41:01.555Z
- REQUESTOR: ilja
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja060120262333

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Dual-Backend-Tests-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Ensure storage conformance tests run against SQLite + PostgreSQL locally and in CI (and block PR merge on failure), per CX-DBP-013 / CX-DBP-030.
- Why: Phase 1 closure requirement for storage portability; catches SQLite-only assumptions before Phase 2 PostgreSQL migration.
- IN_SCOPE_PATHS:
  - .github/workflows/ci.yml
  - docker-compose.test.yml
  - src/backend/handshake_core/Cargo.toml
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
- OUT_OF_SCOPE:
  - Production PostgreSQL deployment.
  - Schema redesign or migration framework work beyond what is required for conformance tests to run.
  - Non-storage CI changes.

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Dual-Backend-Tests-v2

# Local smoke (SQLite):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance

# Local conformance (Postgres):
docker compose -f docker-compose.test.yml up -d
# PowerShell:
$env:POSTGRES_TEST_URL="postgres://postgres:postgres@localhost:5432/handshake_test"
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance

# Optional: unset POSTGRES_TEST_URL to confirm postgres test skips on sqlite runs
# Remove-Item Env:POSTGRES_TEST_URL -ErrorAction SilentlyContinue

# Workflow closure gate (after any edits + manifest filled):
just cargo-clean
just post-work WP-1-Dual-Backend-Tests-v2
```

### DONE_MEANS
- Spec alignment: implementation satisfies Handshake_Master_Spec_v02.101.md Pillar 4 (CX-DBP-013) and is tracked as the named closure requirement (CX-DBP-030).
- Local: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance` passes on SQLite (postgres test may skip when POSTGRES_TEST_URL is unset).
- Local: with `docker compose -f docker-compose.test.yml up -d` and POSTGRES_TEST_URL set, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance` passes and the postgres conformance test does NOT skip.
- CI: `.github/workflows/ci.yml` job `backend-storage` runs matrix `backend=[sqlite, postgres]` and passes (failure on either backend blocks merge).
- Workflow: `just post-work WP-1-Dual-Backend-Tests-v2` passes after any changes with deterministic manifest(s) filled for all non-doc files touched.

### ROLLBACK_HINT
```bash
# If the last commit(s) on this branch are from this WP, revert them:
git revert HEAD
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.101.md (recorded_at: 2026-01-06T22:41:01.555Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 2.3.12.1 (Pillar 4, CX-DBP-013); 2.3.12.5 (Phase 1 closure WPs, WP-1-Dual-Backend-Tests, CX-DBP-030)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.101.md
  - docs/TASK_BOARD.md
  - docs/task_packets/WP-1-Dual-Backend-Tests.md
  - src/backend/handshake_core/Cargo.toml
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - .github/workflows/ci.yml
  - docker-compose.test.yml
- SEARCH_TERMS:
  - "CX-DBP-013"
  - "CX-DBP-030"
  - "Pillar 4"
  - "POSTGRES_TEST_URL"
  - "postgres_backend_from_env"
  - "sqlite_backend"
  - "run_storage_conformance"
  - "backend-storage"
  - "matrix.backend"
  - "services:"
  - "postgres:"
  - "pg_isready"
  - "DATABASE_URL"
  - "PostgresDatabase::connect"
  - "run_migrations"
  - "Skipping postgres storage conformance"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Dual-Backend-Tests-v2
  docker compose -f docker-compose.test.yml up -d
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance
  just cargo-clean
  just post-work WP-1-Dual-Backend-Tests-v2
  ```
- RISK_MAP:
  - "postgres test silently skipped" -> "POSTGRES_TEST_URL not set (CI matrix misconfigured)"
  - "postgres startup race" -> "healthcheck/ready wait insufficient; flaky CI"
  - "migration portability failure" -> "migrations or queries use sqlite-only syntax; postgres run fails"
  - "state leakage between tests" -> "tests assume empty DB or sqlite semantics; nondeterministic failures"
  - "secrets in logs" -> "connection URLs printed in stderr/stdout"
  - "false green CI" -> "backend-storage job removed/disabled; failures no longer block merge"

## SKELETON
- Proposed interfaces/types/contracts:
  - Reuse `handshake_core::storage::tests::{sqlite_backend, postgres_backend_from_env, run_storage_conformance}`.
  - Postgres enablement contract: `POSTGRES_TEST_URL` must be set for postgres conformance (and must be set in CI postgres matrix).
- Open questions:
  - None.
- Notes:
  - This WP is expected to be primarily governance revalidation; only change code/CI if pre-work/post-work or the postgres matrix run reveals a gap.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
