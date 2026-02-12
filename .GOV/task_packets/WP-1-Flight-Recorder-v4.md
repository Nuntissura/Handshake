# Task Packet: WP-1-Flight-Recorder-v4

## METADATA
- TASK_ID: WP-1-Flight-Recorder-v4
- WP_ID: WP-1-Flight-Recorder-v4
- BASE_WP_ID: WP-1-Flight-Recorder
- DATE: 2026-02-11T22:32:58.696Z
- MERGE_BASE_SHA: 8e3092d65fc485d8b4497adc51da703c3f9678da
- REQUESTOR: ilja (Operator) (repeatable boot failure from legacy flight_recorder.db schema)
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI) (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-11T21:34:17.296Z
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja110220262332
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Flight-Recorder-v4.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Make Flight Recorder DuckDB schema initialization tolerant of legacy on-disk DBs (missing newer columns like `events.trace_id`) by running additive migrations before creating indexes (or retrying index creation after migration). Add a regression test that opens a legacy DB and asserts `DuckDbFlightRecorder::new_on_path(...)` succeeds.
- Why: Prevent repeatable hard boot failures when an existing `data/flight_recorder.db` was created with an older schema. This removes the need for manual DB nuking and makes `pnpm -C app tauri dev` reliable while Phase 1 work continues.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
- OUT_OF_SCOPE:
  - Any changes to FR-EVT schemas / event taxonomy
  - Backfilling legacy rows (no forced `trace_id` population for old rows)
  - Any UI changes (Operator Consoles, frontend timeline)
  - Any manual deletion/reset of user databases

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- WAIVER_ID: WP-1-Flight-Recorder-v4-PRAGMA-001 | Date: 2026-02-12 | Scope: src/backend/handshake_core/src/flight_recorder/duckdb.rs | Justification: user-approved scope flexibility to expand regression test to assert index existence via DuckDB introspection (PRAGMA).

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Flight-Recorder-v4

# Backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Optional manual repro (if available):
pnpm -C app tauri dev

# Mechanical manifest gate:
just post-work WP-1-Flight-Recorder-v4 --range 8e3092d65fc485d8b4497adc51da703c3f9678da..HEAD
```

### DONE_MEANS
- Opening a legacy DuckDB file with an `events` table missing `trace_id` does not hard-fail startup: `DuckDbFlightRecorder::new_on_path(...)` succeeds (covered by regression test).
- `src/backend/handshake_core/src/flight_recorder/duckdb.rs` creates/migrates schema idempotently and creates trace/job/timestamp indexes after migrations.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` passes.
- `just post-work WP-1-Flight-Recorder-v4 --range 8e3092d65fc485d8b4497adc51da703c3f9678da..HEAD` passes.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.125.md (recorded_at: 2026-02-11T22:32:58.696Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 11.5 Flight Recorder Event Shapes & Retention (Trace Invariant; DuckDB sink) + Handshake_Master_Spec_v02.125.md [CX-224] BACKEND_STORAGE (persistence logic + migrations)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - .GOV/task_packets/WP-1-Flight-Recorder.md
  - .GOV/task_packets/WP-1-Flight-Recorder-v2.md
  - .GOV/task_packets/WP-1-Flight-Recorder-v3.md
- Preserved in v4:
  - All v3 validated Flight Recorder behavior and schemas remain intact; this WP is a compatibility hardening only.
- Changed in v4:
  - Schema init/migration ordering for legacy DBs (avoid failing on index creation when new columns are absent).
  - Add regression test that constructs a legacy `events` table (without `trace_id`) and asserts `new_on_path` succeeds.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/refinements/WP-1-Flight-Recorder-v4.md
  - .GOV/task_packets/WP-1-Flight-Recorder-v4.md
  - .GOV/task_packets/WP-1-Flight-Recorder-v3.md
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
- SEARCH_TERMS:
  - "CREATE INDEX IF NOT EXISTS idx_events_trace_id"
  - "ALTER TABLE events ADD COLUMN IF NOT EXISTS trace_id"
  - "DuckDbFlightRecorder::new_on_path"
  - "init_schema"
- RUN_COMMANDS:
  ```bash
  rg -n "CREATE INDEX IF NOT EXISTS idx_events_trace_id" src/backend/handshake_core/src/flight_recorder/duckdb.rs
  rg -n "ALTER TABLE events" src/backend/handshake_core/src/flight_recorder/duckdb.rs
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "migration ordering bug" -> "startup crash when opening legacy DB file; blocks dev loop"
  - "partial migration" -> "retrying init fails; ensure idempotent ALTER + index creation"
  - "legacy rows missing trace_id" -> "must not enforce NOT NULL for migrated column; keep old rows readable"

## SKELETON
- Proposed interfaces/types/contracts:
  - `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
    - Keep public API unchanged:
      - `DuckDbFlightRecorder::new_on_path(path: &Path, retention_days: u32) -> Result<Self, RecorderError>`
    - Change internal schema init ordering in `DuckDbFlightRecorder::init_schema()`:
      1) `CREATE TABLE IF NOT EXISTS events (...)` (table only; no indexes yet)
      2) Additive migrations for legacy DBs:
         - `ALTER TABLE events ADD COLUMN IF NOT EXISTS trace_id UUID;`
         - (plus other existing `ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...` statements already present)
      3) Create indexes AFTER migrations:
         - `CREATE INDEX IF NOT EXISTS idx_events_trace_id ON events(trace_id);`
         - `CREATE INDEX IF NOT EXISTS idx_events_job_id ON events(job_id);`
         - `CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);`
    - (Optional refactor) extract helpers to make ordering explicit and testable:
      - `fn ensure_events_table(conn: &DuckDbConnection) -> Result<(), RecorderError>`
      - `fn migrate_events_table_additive(conn: &DuckDbConnection) -> Result<(), RecorderError>`
      - `fn ensure_events_indexes(conn: &DuckDbConnection) -> Result<(), RecorderError>`
  - Regression test (in same file test module):
    - Create a legacy on-disk DuckDB file whose `events` table lacks `trace_id`.
    - Assert `DuckDbFlightRecorder::new_on_path(&path, 7)` succeeds (this will fail on current buggy ordering due to index creation before migration).
- Open questions:
  - Should the regression test also assert the trace/job/timestamp indexes exist via DuckDB introspection (PRAGMA)? (Optional; constructor success already implies index creation did not fail.)
- Notes:
  - Keep migrations additive/idempotent; do NOT backfill legacy rows; migrated `trace_id` remains nullable for old rows (OUT_OF_SCOPE explicitly forbids forced population).

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- TRUST_BOUNDARY: N/A
- SERVER_SOURCES_OF_TRUTH:
  - N/A
- REQUIRED_PROVENANCE_FIELDS:
  - N/A
- VERIFICATION_PLAN:
  - N/A
- ERROR_TAXONOMY_PLAN:
  - N/A
- UI_GUARDRAILS:
  - N/A
- VALIDATOR_ASSERTIONS:
  - N/A

## IMPLEMENTATION
- Reordered `DuckDbFlightRecorder::init_schema()` so additive `ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...` migrations run before creating `events` indexes, preventing legacy on-disk DB startup failures when newer columns (e.g., `trace_id`) are missing.
- Added a regression test that creates an on-disk legacy DuckDB schema (missing `events.trace_id`), then asserts `DuckDbFlightRecorder::new_on_path(...)` succeeds and the `idx_events_*` indexes exist.
- Index introspection uses `PRAGMA index_list('events')` on non-Windows; on Windows it falls back to querying the DuckDB index catalog due to a DuckDB native crash observed when running `PRAGMA index_list`.

## HYGIENE
- Ran `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`.
- Prepared COR-701 manifest values (line window + SHA1 variants via `just cor701-sha`).

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 195
- **End**: 1055
- **Line Delta**: 94
- **Pre-SHA1**: `dc555b976a909e98402f7906b3dbfe3bac77220b`
- **Post-SHA1**: `d0e3e5a37fddcc497c7483701ee002d7701618b9`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**: 2026-02-12T04:09:57.5476449+01:00
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: DuckDB `PRAGMA index_list('events')` triggered a native `STATUS_STACK_BUFFER_OVERRUN` abort on Windows in local testing, so the regression test uses PRAGMA only on non-Windows and falls back to the DuckDB index catalog on Windows.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Implemented schema init ordering fix + added legacy DB regression test (including index existence assertion via DuckDB introspection); ran backend tests and post-work gate; committed implementation.
- Next step / handoff hint: Validator audit + review packet evidence/mapping; if needed, rerun `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` and `just post-work WP-1-Flight-Recorder-v4 --range 8e3092d65fc485d8b4497adc51da703c3f9678da..HEAD`.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
- REQUIREMENT: "Opening a legacy DuckDB file with an `events` table missing `trace_id` does not hard-fail startup: `DuckDbFlightRecorder::new_on_path(...)` succeeds (covered by regression test)."
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/duckdb.rs:1010`
- REQUIREMENT: "`src/backend/handshake_core/src/flight_recorder/duckdb.rs` creates/migrates schema idempotently and creates trace/job/timestamp indexes after migrations."
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/duckdb.rs:195`
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/duckdb.rs:242`
- REQUIREMENT: "Handshake_Master_Spec_v02.125.md 11.5 Flight Recorder Event Shapes & Retention (Trace Invariant; DuckDB sink) + Handshake_Master_Spec_v02.125.md [CX-224] BACKEND_STORAGE (persistence logic + migrations)"
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/duckdb.rs:195`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Flight-Recorder-v4/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

- COMMAND: `just pre-work WP-1-Flight-Recorder-v4`
  - EXIT_CODE: 0
  - WORKTREE_DIR: `../wt-WP-1-Flight-Recorder-v4`
  - BRANCH: `feat/WP-1-Flight-Recorder-v4`
  - GIT_SHA_BEFORE: `47a673740564ed803ff7aa3184ed72affbb6e4b9`
  - GIT_SHA_AFTER: `47a673740564ed803ff7aa3184ed72affbb6e4b9`
  - OUTPUT_SHA256: `ae7477c372bf63ecfc0c1e180cd04c55e3311cc1d0b30f541fee080105a1ded4`
  - PROOF_LINES:
    - `Pre-work validation PASSED`
    - `PASS: Branch matches PREPARE (feat/WP-1-Flight-Recorder-v4)`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - EXIT_CODE: 0
  - WORKTREE_DIR: `../wt-WP-1-Flight-Recorder-v4`
  - BRANCH: `feat/WP-1-Flight-Recorder-v4`
  - GIT_SHA_BEFORE: `7b34b1785dfbf05ccddee47df3dc1c18a5d4cfd9`
  - GIT_SHA_AFTER: `7b34b1785dfbf05ccddee47df3dc1c18a5d4cfd9`
  - OUTPUT_SHA256: `d7956a3a93b4949cec3571c24d430ceb8be3ea6a7fb98094fd25c68546c15439`
  - PROOF_LINES:
    - `test flight_recorder::duckdb::tests::test_new_on_path_migrates_legacy_db_and_creates_indexes ... ok`
    - `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s`

- COMMAND: `just post-work WP-1-Flight-Recorder-v4`
  - EXIT_CODE: 0
  - WORKTREE_DIR: `../wt-WP-1-Flight-Recorder-v4`
  - BRANCH: `feat/WP-1-Flight-Recorder-v4`
  - GIT_SHA_BEFORE: `7b34b1785dfbf05ccddee47df3dc1c18a5d4cfd9`
  - GIT_SHA_AFTER: `7b34b1785dfbf05ccddee47df3dc1c18a5d4cfd9`
  - OUTPUT_SHA256: `9be54ead2b314d0591432da157d2e85d4261f06dbd3afab00ef97a73e8611fc4`
  - PROOF_LINES:
    - `Diff selection: staged (staged changes present)`
    - `Post-work validation PASSED (deterministic manifest gate; not tests)`

- COMMAND: `just post-work WP-1-Flight-Recorder-v4 --range 8e3092d65fc485d8b4497adc51da703c3f9678da..HEAD`
  - EXIT_CODE: 0
  - WORKTREE_DIR: `../wt-WP-1-Flight-Recorder-v4`
  - BRANCH: `feat/WP-1-Flight-Recorder-v4`
  - GIT_SHA_BEFORE: `016cf9ab8d88f7ea8868b26c12fba7024bbeb5cd`
  - GIT_SHA_AFTER: `016cf9ab8d88f7ea8868b26c12fba7024bbeb5cd`
  - OUTPUT_SHA256: `4e92c6bb6dd9c20ea7fa32e06d2a43b536696af95f6211414a7ad7d0769e1060`
  - PROOF_LINES:
    - `Diff selection: range (explicit --range)`
    - `Git range: 8e3092d65fc485d8b4497adc51da703c3f9678da..016cf9ab8d88f7ea8868b26c12fba7024bbeb5cd`
    - `Post-work validation PASSED (deterministic manifest gate; not tests) with warnings`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
