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
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed>
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- **Status:** Ready for Dev
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
Next: N/A

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
- NONE

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
- Open questions:
- Notes:

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
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
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
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-MCP-Durable-Progress-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
