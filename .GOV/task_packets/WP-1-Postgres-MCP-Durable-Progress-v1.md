# Task Packet: WP-1-Postgres-MCP-Durable-Progress-v1

## METADATA
- TASK_ID: WP-1-Postgres-MCP-Durable-Progress-v1
- WP_ID: WP-1-Postgres-MCP-Durable-Progress-v1
- BASE_WP_ID: WP-1-Postgres-MCP-Durable-Progress (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-03-05T23:22:55.018Z
- MERGE_BASE_SHA: 49442d61b03f52b4f11e2334933ef5b6283c7a94
- MERGE_BASE_SHA_NOTE: git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence
- REQUESTOR: Operator (ilja) (Postgres MCP durable progress mapping fails: NotImplemented storage methods)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI) (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: 2026-03-05T22:41:53.300Z
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- **Status:** In Progress
- RISK_TIER: HIGH
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Abstraction-Layer, WP-1-Migration-Framework, WP-1-Dual-Backend-Tests
- BUILD_ORDER_BLOCKS: NONE
- USER_SIGNATURE: ilja060320260004
- PACKET_FORMAT_VERSION: 2026-02-01

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: Create docs-only skeleton checkpoint commit; then implement migration-backed durable MCP progress mapping + dual-backend tests.

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Postgres-MCP-Durable-Progress-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement MCP durable progress token persistence + reverse lookup for PostgreSQL (and keep SQLite parity) by introducing a portable schema/migration-backed store (recommended: side-table `ai_job_mcp_fields`) and implementing the `Database` trait MCP methods for both backends.
- Why: Under PostgreSQL, MCP tool-call flows that rely on `mcp_progress_token` durability fail because the Postgres `Database` impl returns NotImplemented for MCP durable fields; Master Spec 11.3.4 + portability pillars require durable mapping and dual-backend parity.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/migrations/
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/tests/mcp_e2e_tests.rs
  - .github/workflows/ci.yml
- OUT_OF_SCOPE:
  - One-time data backfill from legacy SQLite columns into the new side-table (no data transforms in this WP)
  - Postgres-only runtime DDL fixes (must stay migration-backed + portable)
  - Flight Recorder schema changes (this WP restores token->job_id resolvability for existing mcp.progress evidence)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- Waiver ID: CX-573F
  - Date: 2026-03-06
  - Scope: Out-of-scope governance/workflow edits landed on this WP branch before the storage implementation:
    - `.GOV/roles/coder/CODER_PROTOCOL.md`
    - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
    - `.GOV/roles_shared/BUILD_ORDER.md`
    - `.GOV/scripts/validation/{pre-work.mjs,pre-work-check.mjs,post-work-check.mjs,skeleton-approved.mjs}`
    - `justfile`
  - Justification: Operator-requested workflow hard gate ("skeleton approved") needed for this WP flow; kept in-branch for continuity.

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Postgres-MCP-Durable-Progress-v1

# SQLite (smoke):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests mcp_e2e_tests

# Postgres (conformance + MCP durable mapping):
docker compose -f docker-compose.test.yml up -d
# PowerShell:
$env:POSTGRES_TEST_URL="postgres://postgres:postgres@localhost:5432/handshake_test"
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests mcp_e2e_tests

# Workflow closure gate (after any edits + manifest filled):
just cargo-clean
just post-work WP-1-Postgres-MCP-Durable-Progress-v1 --range 49442d61b03f52b4f11e2334933ef5b6283c7a94..HEAD
```

### DONE_MEANS
- Postgres parity: MCP durable storage methods are implemented (no NotImplemented) for Postgres and SQLite:
  - `Database::update_ai_job_mcp_fields`
  - `Database::get_ai_job_mcp_fields`
  - `Database::find_ai_job_id_by_mcp_progress_token`
- Portable persistence: a migration-backed, DB-agnostic schema stores MCP fields (recommended: side-table `ai_job_mcp_fields`) with a UNIQUE or indexed `mcp_progress_token` to enforce 1:1 token->job mapping.
- Local (SQLite): `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance` passes.
- Local (Postgres): with `docker compose -f docker-compose.test.yml up -d` and `POSTGRES_TEST_URL` set, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance` runs the Postgres variant (does not skip) and passes.
- MCP regression: durable progress mapping assertions pass for both backends (SQLite and Postgres) and `find_ai_job_id_by_mcp_progress_token(token)` returns the originating job_id.
- Workflow: `just post-work WP-1-Postgres-MCP-Durable-Progress-v1 --range 49442d61b03f52b4f11e2334933ef5b6283c7a94..HEAD` passes with deterministic manifests filled for all non-`.GOV/` files touched.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.141.md (recorded_at: 2026-03-05T23:22:55.018Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 11.3.4 Implementation Target 3: Durable Progress Mapping (SQLite Integration); 2.3.13.1 Pillar 4: Dual-Backend Testing Early [CX-DBP-013]; 2.3.13.1 Pillar 2: Portable Schema & Migrations [CX-DBP-011]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.141.md
  - .GOV/task_packets/stubs/WP-1-Postgres-MCP-Durable-Progress-v1.md
  - .GOV/refinements/WP-1-Postgres-MCP-Durable-Progress-v1.md
  - src/backend/handshake_core/Cargo.toml
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/migrations/
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/tests/mcp_e2e_tests.rs
- SEARCH_TERMS:
  - "mcp_progress_token"
  - "ai_job_mcp_fields"
  - "update_ai_job_mcp_fields"
  - "get_ai_job_mcp_fields"
  - "find_ai_job_id_by_mcp_progress_token"
- RUN_COMMANDS:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance
  docker compose -f docker-compose.test.yml up -d
  ```
- RISK_MAP:
  - "token mapping ambiguity" -> "mcp.progress events correlate to the wrong job_id"
  - "schema drift" -> "sqlite runtime DDL vs migrations diverge; Postgres fails"
  - "non-portable migrations" -> "CI fails under Postgres"

## SKELETON
- Proposed interfaces/types/contracts:
  - Portable schema (migration-backed):
    - Introduce side-table `ai_job_mcp_fields` keyed by `job_id` (FK -> `ai_jobs`) with optional columns:
      - `mcp_server_id`, `mcp_call_id`, `mcp_progress_token`
    - Enforce 1:1 mapping from `mcp_progress_token` -> `job_id` via UNIQUE constraint/index.
  - Storage trait semantics (existing surface; implement for SQLite + Postgres):
    - `Database::update_ai_job_mcp_fields(job_id, update)` performs an UPSERT into side-table; only fields present in `update` are mutated.
    - `Database::get_ai_job_mcp_fields(job_id)` reads from side-table (preferred); for older SQLite DBs without side-table row, fallback to legacy `ai_jobs` columns if present (no transforms/backfill in this WP).
    - `Database::find_ai_job_id_by_mcp_progress_token(token)` looks up via side-table; fallback to legacy `ai_jobs.mcp_progress_token` for older DBs; returns `None` if unknown.
  - Error/consistency posture:
    - Token collision across jobs is prevented by UNIQUE constraint; treat violations as an error (no silent reassignment).
    - All queries remain parameterized; no runtime DDL.
- Open questions:
  - Confirm current `ai_jobs` primary key column naming (`id` vs `job_id`) in migrations for correct FK definition.
  - Confirm whether SQLite currently persists MCP fields in `ai_jobs` columns and whether any runtime DDL remains; align read-through behavior accordingly.
  - Confirm how `AiJobMcpUpdate` expresses partial updates (Option fields) and whether `mcp_progress_token` is required before issuing MCP tool calls.
  - Confirm intended semantics for clearing fields (if `AiJobMcpUpdate` supports explicit nulling) vs "leave unchanged".
- Notes:
  - Out-of-scope backfill: this WP adds the side-table + read-through; it does not migrate legacy data into the side-table.
  - CI/testing: ensure `storage_conformance` + `mcp_e2e_tests` execute for both SQLite and Postgres per [CX-DBP-013].

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
- Migration-backed portable schema:
  - Added side-table `ai_job_mcp_fields` keyed by `job_id` with UNIQUE `mcp_progress_token` reverse lookup.
- Storage parity (SQLite + Postgres):
  - Implemented durable MCP progress mapping via the side-table for both backends:
    - `Database::update_ai_job_mcp_fields`
    - `Database::get_ai_job_mcp_fields`
    - `Database::find_ai_job_id_by_mcp_progress_token`
  - Postgres: enforces token uniqueness via DB constraint; maps unique violations to `StorageError::Conflict(...)`; updates `ai_jobs.updated_at` on successful update.
  - SQLite: uses side-table as primary store; backward-compat read fallback to legacy `ai_jobs.mcp_*` columns when no side-table row exists (no DDL/backfill).
- Tests/CI:
  - Parameterized MCP e2e test to run on SQLite and Postgres (skips Postgres when `POSTGRES_TEST_URL` is unset).
  - CI: `backend-storage` job runs `mcp_e2e_tests` alongside `storage_conformance` for both backends.

## HYGIENE
- Gates:
  - `just coder-startup`
  - `just pre-work WP-1-Postgres-MCP-Durable-Progress-v1`
- Local test env notes:
  - Windows DuckDB build required a shorter `CARGO_TARGET_DIR` to avoid MSVC path-length issues (`CARGO_TARGET_DIR=D:\\hs-target`).
  - Local Postgres service occupied port 5432; Docker Postgres for this WP was exposed on 5433 via an override compose file under `.handshake/` (not committed).
- Tests run per `TEST_PLAN`: see `## EVIDENCE` logs.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `justfile`
- **Start**: 1
- **End**: 324
- **Line Delta**: 4
- **Pre-SHA1**: `cb2d4748b370cf6e61e3ac0648a7067df6c600a0`
- **Post-SHA1**: `27e34451b173526b04576877377cf0472da10e16`
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
- **Lint Results**: N/A
- **Artifacts**: N/A
- **Timestamp**: 2026-03-06T02:58:00Z
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: Out-of-scope governance change (waived CX-573F).

- **Target File**: `.github/workflows/ci.yml`
- **Start**: 1
- **End**: 333
- **Line Delta**: 3
- **Pre-SHA1**: `f409331fe71c298dcd8ca35db9cda09f180075da`
- **Post-SHA1**: `b069f5b6efd91cfe9c96fdcef516353d33825a76`
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
- **Lint Results**: N/A
- **Artifacts**: N/A
- **Timestamp**: 2026-03-06T02:58:00Z
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**:

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 1
- **End**: 4179
- **Line Delta**: 133
- **Pre-SHA1**: `17bea48df8b40497e40be4bf65d2883f52939892`
- **Post-SHA1**: `6e85127af80185579a7330e7cbbaa0e5000023c3`
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
- **Lint Results**: `cargo test` (see EVIDENCE)
- **Artifacts**: N/A
- **Timestamp**: 2026-03-06T02:58:00Z
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**:

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 4849
- **Line Delta**: 63
- **Pre-SHA1**: `a6ddca565a90bf8d70c7e8b94252b76b8568219e`
- **Post-SHA1**: `89fa6eb668182fe6ff016ad2d416f17f7fcb1798`
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
- **Lint Results**: `cargo test` (see EVIDENCE)
- **Artifacts**: N/A
- **Timestamp**: 2026-03-06T02:58:00Z
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**:

- **Target File**: `src/backend/handshake_core/tests/mcp_e2e_tests.rs`
- **Start**: 1
- **End**: 484
- **Line Delta**: 20
- **Pre-SHA1**: `b3c03021e526a0963b6a628ca5c93d04e796376f`
- **Post-SHA1**: `9b24d18d1d6b7f7c2df64fd2b07df38bda146a1a`
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
- **Lint Results**: `cargo test` (see EVIDENCE)
- **Artifacts**: N/A
- **Timestamp**: 2026-03-06T02:58:00Z
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**:

- **Target File**: `src/backend/handshake_core/migrations/0014_ai_job_mcp_fields.sql`
- **Start**: 1
- **End**: 12
- **Line Delta**: 12
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `d32f921b344e5595d268a051128b0e0509fd6c60`
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
- **Lint Results**: N/A
- **Artifacts**: N/A
- **Timestamp**: 2026-03-06T02:58:00Z
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: New file in this range; pre-SHA1 is a sentinel value.

- **Target File**: `src/backend/handshake_core/migrations/0014_ai_job_mcp_fields.down.sql`
- **Start**: 1
- **End**: 3
- **Line Delta**: 3
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `1d989936f6fb66deba0bd7e16d54dd111201389c`
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
- **Lint Results**: N/A
- **Artifacts**: N/A
- **Timestamp**: 2026-03-06T02:58:00Z
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: New file in this range; pre-SHA1 is a sentinel value.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Ready for Validator
- What changed in this update:
  - Added portable durable MCP mapping store (`ai_job_mcp_fields`) + UNIQUE progress token constraint.
  - Implemented durable MCP mapping methods for Postgres + SQLite using the new side-table (with SQLite legacy fallback).
  - Added dual-backend MCP e2e test coverage and CI wiring.
- Next step / handoff hint:
  - Run `just cargo-clean` and `just post-work WP-1-Postgres-MCP-Durable-Progress-v1 --range 49442d61b03f52b4f11e2334933ef5b6283c7a94..HEAD` then handoff to Validator.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
- REQUIREMENT: "Portable persistence: a migration-backed, DB-agnostic schema stores MCP fields ... side-table `ai_job_mcp_fields` ... UNIQUE ... `mcp_progress_token`"
  - EVIDENCE: `src/backend/handshake_core/migrations/0014_ai_job_mcp_fields.sql:3`
  - EVIDENCE: `src/backend/handshake_core/migrations/0014_ai_job_mcp_fields.sql:11`
- REQUIREMENT: "Postgres parity: MCP durable storage methods are implemented (no NotImplemented) for Postgres and SQLite: update/get/find"
  - EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:3763`
  - EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:3824`
  - EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:3857`
  - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:4334`
  - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:4393`
  - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:4448`
- REQUIREMENT: "MCP regression: durable progress mapping assertions pass for both backends (SQLite and Postgres) and `find_ai_job_id_by_mcp_progress_token(token)` returns the originating job_id."
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_e2e_tests.rs:245`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_e2e_tests.rs:253`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_e2e_tests.rs:417`
- REQUIREMENT: "CI pipeline includes PostgreSQL test variant ... runs storage_conformance + mcp_e2e_tests"
  - EVIDENCE: `.github/workflows/ci.yml:136`
  - EVIDENCE: `.github/workflows/ci.yml:142`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-MCP-Durable-Progress-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
- COMMAND: `docker compose -f docker-compose.test.yml -f .handshake/docker-compose.test.override-5433.yml up -d`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-MCP-Durable-Progress-v1/docker_compose_test_up_5433.log`
  - LOG_SHA256: `acf8ad14753b1164c73893c10976c8458baa4a111444d2e6317fec57d04beee3`
  - PROOF_LINES: `Container wt-wp-1-postgres-mcp-durable-progress-v1-postgres-1  Started`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-MCP-Durable-Progress-v1/cargo_test_storage_conformance_sqlite_final.log`
  - LOG_SHA256: `eeaf3517f383c2c112e4bae252d60a0834bfa8d965d98f8603a49894e3fc364c`
  - PROOF_LINES: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests mcp_e2e_tests`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-MCP-Durable-Progress-v1/cargo_test_mcp_e2e_sqlite_final.log`
  - LOG_SHA256: `8c42325ca6e6787b284f64d90d16d2519229f0a268259836fe342f85674d4139`
  - PROOF_LINES: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;`

- COMMAND: `$env:POSTGRES_TEST_URL=\"postgres://postgres:postgres@127.0.0.1:5433/handshake_test\"; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-MCP-Durable-Progress-v1/cargo_test_storage_conformance_postgres_5433.log`
  - LOG_SHA256: `f6e0b6f9da35299543550601fac3a09598ecb909c9b1d2e15a602206d717ae20`
  - PROOF_LINES: `test postgres_storage_conformance ... ok`

- COMMAND: `$env:POSTGRES_TEST_URL=\"postgres://postgres:postgres@127.0.0.1:5433/handshake_test\"; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests mcp_e2e_tests`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-MCP-Durable-Progress-v1/cargo_test_mcp_e2e_postgres_5433.log`
  - LOG_SHA256: `159ad9abacf821e55913569f2dbbf8e0d940d14c1d9dcbd6659d64936d1a8d4a`
  - PROOF_LINES: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
