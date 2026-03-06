# Task Packet: WP-1-Calendar-Storage-v1

## METADATA
- TASK_ID: WP-1-Calendar-Storage-v1
- WP_ID: WP-1-Calendar-Storage-v1
- BASE_WP_ID: WP-1-Calendar-Storage
- DATE: 2026-03-06T09:14:17.773Z
- MERGE_BASE_SHA: 96e7cc5bf2e88fc49e1da1cf98a997a012f9b472
- MERGE_BASE_SHA_NOTE: git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence
- REQUESTOR: Operator (ilja) (activate next high-value packet: Calendar Storage)
- AGENT_ID: CodexCLI-GPT-5 (Orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: NO
<!-- Allowed: YES | NO -->
- ORCHESTRATOR_MODEL: GPT-5 (Codex CLI)
<!-- Required if AGENTIC_MODE=YES -->
- ORCHESTRATION_STARTED_AT_UTC: 2026-03-06T08:26:50.861Z
<!-- RFC3339 UTC; required if AGENTIC_MODE=YES -->
- CODER_MODEL: GPT-5 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- **Status:** In Progress
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Migration-Framework, WP-1-Storage-Abstraction-Layer
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Calendar-Lens, WP-1-Calendar-Sync-Engine, WP-1-Calendar-Policy-Integration, WP-1-Calendar-Law-Compliance-Tests
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- USER_SIGNATURE: ilja060320260955
- PACKET_FORMAT_VERSION: 2026-03-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: Create the docs-only skeleton checkpoint commit, then STOP for `just skeleton-approved WP-1-Calendar-Storage-v1`.

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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Calendar-Storage-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-Database
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
  - PRIM-CalendarMutation
  - PRIM-CalendarSyncInput
  - PRIM-FlightRecorder
- PRIMITIVE_INDEX_ACTION: NO_CHANGE (UPDATED | NO_CHANGE)
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND (OK | NEEDS_STUBS | NONE_FOUND)
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1 (comma-separated WP-... IDs | NONE)

## SCOPE
- What: Add portable, dual-backend persistent storage for `CalendarSource` and `CalendarEvent` through the `Database` trait, including migrations, source/event upsert paths, time-window queries, and source-scoped cleanup.
- Why: Calendar storage is the foundation for Calendar Lens, `calendar_sync`, policy enforcement, and compliance tests; without it the spec's write-gate and provenance rules cannot be implemented or audited.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/calendar.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/tests/calendar_storage_tests.rs
  - src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - src/backend/handshake_core/migrations/0015_calendar_storage.down.sql
- OUT_OF_SCOPE:
  - Calendar Lens UI, API handlers, or search/full-text surfaces
  - `calendar_sync` workflow orchestration, provider adapters, or Flight Recorder event emission
  - Policy/capability enforcement beyond persisting required source metadata fields
  - Recurrence expansion or conflict resolution beyond storing RRULE, instance identity, and raw provider fields
  - Direct SQL from API/UI code or runtime DDL workarounds

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Calendar-Storage-v1

cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests

docker compose -f docker-compose.test.yml up -d
$env:POSTGRES_TEST_URL="postgres://postgres:postgres@localhost:5432/handshake_test"
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests

just cargo-clean
just post-work WP-1-Calendar-Storage-v1 --range 96e7cc5bf2e88fc49e1da1cf98a997a012f9b472..HEAD
```

### DONE_MEANS
- `Database` exposes calendar storage primitives for sources and events, and both `SqliteDatabase` and `PostgresDatabase` implement them without `NotImplemented` stubs or caller-side SQL bypasses.
- Portable migration pair `0015_calendar_storage.*` creates `calendar_sources` and `calendar_events` plus required foreign keys and indexes using DB-agnostic SQL; replay-safe and undo-safe migration checks continue to pass on SQLite and Postgres.
- Calendar source persistence covers the spec-defined identity, provider/write-policy, default timezone, sync-state, and capability-profile fields needed by downstream sync and policy packets.
- Calendar event persistence covers source identity, external identity/etag, UTC range plus `tzid`/`all_day` semantics, status/visibility/export mode, RRULE/is_recurring/instance_key, and raw relationship payload fields without lossy normalization.
- Source and event write semantics enforce idempotent `(source_id, external_id)` behavior and support source-scoped deletion plus time-window querying by workspace and time range.
- Dual-backend tests prove calendar source/event CRUD, time-window query behavior, migration replay/undo safety, and idempotent upsert semantics on SQLite and Postgres.
- `just post-work WP-1-Calendar-Storage-v1 --range 96e7cc5bf2e88fc49e1da1cf98a997a012f9b472..HEAD` passes with deterministic manifests for every non-`.GOV/` file changed.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.141.md (recorded_at: 2026-03-06T09:14:17.773Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.0.4 Mutation governance (Hard Invariant) [ilja251220250127]; 2.0.4 Mutation and governance rules; 2.1 Raw entities; 2.3 Storage and indexing; 2.3.13.1 Pillar 2: Portable Schema & Migrations [CX-DBP-011]; 2.3.13.1 Pillar 4: Dual-Backend Testing Early [CX-DBP-013]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- BASE_WP_ID: WP-1-Calendar-Storage
- WP_ID: WP-1-Calendar-Storage-v1
- SPEC_TARGET: Handshake_Master_Spec_v02.141.md (from .GOV/roles_shared/SPEC_CURRENT.md)
- Prior packets:
  - .GOV/task_packets/stubs/WP-1-Calendar-Storage-v1.md (status: STUB; activated into this official packet)
- Carry-forward:
  - Preserved from stub: portable migration requirement, trait-based storage boundary, source/event persistence, and the recurrence/full-text out-of-scope guardrails.
  - Added in official packet: signed Main Body anchors, exact file scope, dual-backend acceptance criteria, and coder bootstrap/handoff material.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.141.md
  - .GOV/task_packets/stubs/WP-1-Calendar-Storage-v1.md
  - .GOV/refinements/WP-1-Calendar-Storage-v1.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/migrations/0014_ai_job_mcp_fields.sql
- SEARCH_TERMS:
  - "calendar_mutation"
  - "CalendarEvent"
  - "CalendarSource"
  - "calendar_sources"
  - "calendar_events"
  - "source_id"
  - "external_id"
  - "instance_key"
  - "tzid"
  - "capability_profile_id"
  - "run_storage_conformance"
  - "migrations_are_replay_safe_postgres"
  - "migrations_can_undo_to_baseline_postgres"
  - "sqlx::migrate!"
- RUN_COMMANDS:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests
  docker compose -f docker-compose.test.yml up -d
  ```
- RISK_MAP:
  - "SQLite-only migration syntax" -> "Postgres divergence blocks downstream calendar work"
  - "lossy time normalization" -> "all-day/tz behavior drifts and events render incorrectly"
  - "missing (source_id, external_id) idempotency" -> "duplicate events and broken sync replays"
  - "trait bypass or raw SQL outside storage layer" -> "calendar write gate and auditability are violated"
  - "schema too narrow for source sync state" -> "later sync/policy packets need incompatible follow-up migrations"

## SKELETON
- Proposed interfaces/types/contracts:
  - Add `src/backend/handshake_core/src/storage/calendar.rs` and export it from `storage/mod.rs`. The module will define:
    - `CalendarSource`, `CalendarSourceSyncState`, and `CalendarSourceUpsert`
    - `CalendarEvent`, `CalendarEventUpsert`, and `CalendarEventWindowQuery`
    - typed string enums for source provider/write policy and event status/visibility/export mode so row mapping stays backend-parity safe
  - Extend `Database` with six calendar-only storage methods behind the trait boundary:
    - `upsert_calendar_source(&self, ctx: &WriteContext, source: CalendarSourceUpsert) -> StorageResult<CalendarSource>`
    - `list_calendar_sources(&self, workspace_id: &str) -> StorageResult<Vec<CalendarSource>>`
    - `get_calendar_source(&self, workspace_id: &str, source_id: &str) -> StorageResult<Option<CalendarSource>>`
    - `upsert_calendar_event(&self, ctx: &WriteContext, event: CalendarEventUpsert) -> StorageResult<CalendarEvent>`
    - `query_calendar_events(&self, query: CalendarEventWindowQuery) -> StorageResult<Vec<CalendarEvent>>`
    - `delete_calendar_data_by_source(&self, ctx: &WriteContext, workspace_id: &str, source_id: &str) -> StorageResult<()>`
  - `sqlite.rs` and `postgres.rs` will share the same behavior:
    - source writes are deterministic UPSERTs keyed by `calendar_sources.id`
    - provider-backed event writes are deterministic UPSERTs keyed by `(source_id, external_id)`
    - local-only events keep `external_id = NULL` and rely on stable internal `id`
    - time-window reads use overlap semantics (`start_ts_utc < window_end AND end_ts_utc > window_start`) and remain scoped by `workspace_id`
  - Migration `0015_calendar_storage` will create portable relational tables with existing storage traceability columns (`last_actor_kind`, `last_actor_id`, `last_job_id`, `last_workflow_id`, `edit_event_id`) and timestamps:
    - `calendar_sources`
      - identity/scope: `id`, `workspace_id`
      - source contract: `display_name`, `provider_type`, `write_policy`, `default_tzid`, `credentials_ref`, `provider_calendar_id`, `capability_profile_id`
      - sync/provenance payload: explicit `sync_token`, `last_sync_ts`, `last_full_sync_ts`, `last_error`, plus `config_json` for provider-specific lossless fields
    - `calendar_events`
      - identity/scope: `id`, `workspace_id`, `source_id`, `external_id`, `external_etag`
      - event contract: `title`, `description`, `location`, `start_ts_utc`, `end_ts_utc`, `tzid`, `all_day`, `was_floating`, `status`, `visibility`, `export_mode`, `rrule`, `is_recurring`, `instance_key`
      - downstream auditability fields: `created_by`, `attendees_json`, `links_json`, `provider_payload_json`
    - FKs / delete semantics:
      - `calendar_sources.workspace_id -> workspaces.id ON DELETE CASCADE`
      - `calendar_events.workspace_id -> workspaces.id ON DELETE CASCADE`
      - `calendar_events.source_id -> calendar_sources.id ON DELETE CASCADE`
    - indexes:
      - `idx_calendar_events_workspace_window(workspace_id, start_ts_utc, end_ts_utc)`
      - `idx_calendar_events_source_external(source_id, external_id)` as the provider idempotency path
      - `idx_calendar_events_source_instance(source_id, instance_key)` for recurrence-safe lookup
      - `idx_calendar_sources_workspace(workspace_id, provider_type)` for scoped source listing
  - Test surface:
    - extend `src/backend/handshake_core/src/storage/tests.rs::run_storage_conformance` with a minimal calendar source/event smoke path so `storage_conformance.rs` exercises both backends
    - add `src/backend/handshake_core/tests/calendar_storage_tests.rs` for focused source upsert/list/get, event upsert, duplicate provider upsert, time-window query, and delete-by-source parity checks
- Open questions:
  - No blocking questions. Assumptions for the checkpoint:
    - store `attendees`, `links`, provider config, and raw relationship payloads as JSON text columns rather than auxiliary tables in this WP
    - keep explicit sync token / last sync / last full sync / last error columns on `calendar_sources`, with `config_json` reserved for provider extras rather than required sync state
    - set `created_by` from `WriteContext.actor_id` on insert and keep it immutable across subsequent event upserts
- Notes:
  - No Flight Recorder event type is added in this WP; only preserve storage fields and IDs needed by later `calendar_mutation` / `calendar_sync` events.
  - No API/UI/workflow code will talk to calendar tables directly; only the `Database` trait implementations in `sqlite.rs` / `postgres.rs` will own SQL for this feature.
  - The migration remains DB-agnostic SQL only; no SQLite/Postgres-only DDL branches, no runtime `ALTER TABLE` path for calendar storage.
  - The focused integration test file will stay backend-parameterized rather than splitting into SQLite-only / Postgres-only copies.

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
  - LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
