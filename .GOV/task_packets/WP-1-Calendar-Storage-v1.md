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
- Added `src/backend/handshake_core/src/storage/calendar.rs` with calendar source/event DTOs, sync-state payloads, and typed enums for provider, write policy, status, visibility, and export mode so both backends map the same storage contract.
- Extended `src/backend/handshake_core/src/storage/mod.rs` with six calendar storage methods on `Database` so source/event writes, reads, window queries, and source-scoped cleanup stay behind the trait boundary.
- Implemented backend-parity calendar SQL in `src/backend/handshake_core/src/storage/sqlite.rs` and `src/backend/handshake_core/src/storage/postgres.rs`: deterministic source upsert, provider-backed event upsert on `(source_id, external_id)`, local-only event upsert on stable internal `id`, overlap-based time-window reads, and delete-by-source through FK cascade.
- Added portable migration pair `src/backend/handshake_core/migrations/0015_calendar_storage.sql` and `src/backend/handshake_core/migrations/0015_calendar_storage.down.sql` for `calendar_sources` / `calendar_events`, including sync-state fields, timezone/all-day fidelity fields, recurrence identity, relationship payload JSON, traceability columns, and required indexes.
- Extended shared storage conformance coverage in `src/backend/handshake_core/src/storage/tests.rs`, added focused parity tests in `src/backend/handshake_core/tests/calendar_storage_tests.rs`, and isolated Postgres test runs into per-run schemas so migration-backed storage tests remain replay-safe on shared Postgres instances.

## HYGIENE
- Reverted accidental out-of-scope `cargo fmt` churn so the working diff stayed inside the packet's storage, migration, and calendar test scope.
- Staged only the eight non-`.GOV/` WP files and captured deterministic COR-701 pre/post SHA1 values with `just cor701-sha`.
- Ran the packet test plan plus storage hygiene commands; `just cargo-clean` completed and `just post-work WP-1-Calendar-Storage-v1` passed on the staged diff. Raw outputs are recorded in `## EVIDENCE` and log files under `.handshake/logs/WP-1-Calendar-Storage-v1/`.

## VALIDATION
- **Target File**: `src/backend/handshake_core/migrations/0015_calendar_storage.down.sql`
- **Start**: 1
- **End**: 4
- **Line Delta**: 4
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `4d2f743b58f48669b8091c7a1f45f31625136fb2`
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
- **Lint Results**: Test plan PASS; `just validator-scan` PASS; `just validator-dal-audit` PASS; `just validator-git-hygiene` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-06T12:00:43.3235877+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: New migration rollback file.

- **Target File**: `src/backend/handshake_core/migrations/0015_calendar_storage.sql`
- **Start**: 1
- **End**: 88
- **Line Delta**: 88
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `d956ee3fa51ce1b2bc4844fcefb5e8ea5f184335`
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
- **Lint Results**: Test plan PASS; `just validator-scan` PASS; `just validator-dal-audit` PASS; `just validator-git-hygiene` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-06T12:00:43.3235877+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: New portable calendar schema migration.

- **Target File**: `src/backend/handshake_core/src/storage/calendar.rs`
- **Start**: 1
- **End**: 349
- **Line Delta**: 349
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `9fbd02c81fd0f17cdea6b1bedde2da83797b2e24`
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
- **Lint Results**: Test plan PASS; `just validator-scan` PASS; `just validator-dal-audit` PASS; `just validator-git-hygiene` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-06T12:00:43.3235877+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: New shared calendar storage types module.

- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 1901
- **Line Delta**: 35
- **Pre-SHA1**: `d3bd36e9887c98c6b95e584ec953896945b7ba56`
- **Post-SHA1**: `55f5bd5dd8b28cc860b2eb15752a20e9dfd9196b`
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
- **Lint Results**: Test plan PASS; `just validator-scan` PASS; `just validator-dal-audit` PASS; `just validator-git-hygiene` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-06T12:00:43.3235877+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: Extended the `Database` trait and module exports for calendar storage.

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 1
- **End**: 5078
- **Line Delta**: 899
- **Pre-SHA1**: `6e85127af80185579a7330e7cbbaa0e5000023c3`
- **Post-SHA1**: `e52777c01d146c9c30d5f5696ab88d2ca3223ada`
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
- **Lint Results**: Test plan PASS; `just validator-scan` PASS; `just validator-dal-audit` PASS; `just validator-git-hygiene` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-06T12:00:43.3235877+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: Added Postgres calendar source/event storage parity.

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 5749
- **Line Delta**: 900
- **Pre-SHA1**: `89fa6eb668182fe6ff016ad2d416f17f7fcb1798`
- **Post-SHA1**: `3284fe89e6d4ac792248b5496b640cffff4e4b46`
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
- **Lint Results**: Test plan PASS; `just validator-scan` PASS; `just validator-dal-audit` PASS; `just validator-git-hygiene` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-06T12:00:43.3235877+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: Added SQLite calendar source/event storage parity.

- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 1
- **End**: 1542
- **Line Delta**: 370
- **Pre-SHA1**: `42116cffcd11ee1e30836f0d0cb2341ed1610c08`
- **Post-SHA1**: `477228885d07d32c9b7f3152435f183a2b08f0e4`
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
- **Lint Results**: Test plan PASS; `just validator-scan` PASS; `just validator-dal-audit` PASS; `just validator-git-hygiene` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-06T12:00:43.3235877+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: Extended shared dual-backend storage conformance coverage.

- **Target File**: `src/backend/handshake_core/tests/calendar_storage_tests.rs`
- **Start**: 1
- **End**: 28
- **Line Delta**: 28
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `bd1b612ac02b1b83c8d6fdc17b87bc61b632a037`
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
- **Lint Results**: Test plan PASS; `just validator-scan` PASS; `just validator-dal-audit` PASS; `just validator-git-hygiene` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-06T12:00:43.3235877+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: New focused calendar parity integration test.

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete; GATES_PASS (post-work) PASS; TEST_PLAN results recorded; ready for validation
- What changed in this update: Implemented trait-gated calendar source/event persistence for SQLite and Postgres, added portable `0015_calendar_storage` migrations, expanded shared storage conformance coverage, and added focused dual-backend calendar storage tests.
- Next step / handoff hint: Review `## VALIDATION`, `## EVIDENCE_MAPPING`, and `## EVIDENCE`, then re-run the validator suite after the recorded `just cargo-clean` and `just post-work` outputs are appended.

## EVIDENCE_MAPPING
- REQUIREMENT: "Portable migration pair `0015_calendar_storage.*` creates `calendar_sources` and `calendar_events` plus required foreign keys and indexes using DB-agnostic SQL; replay-safe and undo-safe migration checks continue to pass on SQLite and Postgres."
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:3`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:38`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:81`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:84`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:87`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.down.sql:1`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:536`
- EVIDENCE: `src/backend/handshake_core/tests/calendar_storage_tests.rs:15`
- REQUIREMENT: "Calendar source persistence covers the spec-defined identity, provider/write-policy, default timezone, sync-state, and capability-profile fields needed by downstream sync and policy packets."
- EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:124`
- EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:142`
- EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:160`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2732`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2301`
- REQUIREMENT: "Calendar event persistence covers source identity, external identity/etag, UTC range plus `tzid`/`all_day` semantics, status/visibility/export mode, RRULE/is_recurring/instance_key, and raw relationship payload fields without lossy normalization."
- EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:243`
- EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:275`
- EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:311`
- EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:344`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:38`
- REQUIREMENT: "Source and event write semantics enforce idempotent `(source_id, external_id)` behavior and support source-scoped deletion plus time-window querying by workspace and time range."
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2987`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:3333`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:3400`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2554`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2895`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2962`
- REQUIREMENT: "Dual-backend tests prove calendar source/event CRUD, time-window query behavior, migration replay/undo safety, and idempotent upsert semantics on SQLite and Postgres."
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:536`
- EVIDENCE: `src/backend/handshake_core/tests/calendar_storage_tests.rs:7`
- EVIDENCE: `src/backend/handshake_core/tests/calendar_storage_tests.rs:15`
- REQUIREMENT: "SPEC_ANCHOR 2.0.4 Mutation governance (Hard Invariant) / 2.3 Storage and indexing / 2.3.13.1 Pillar 2 / Pillar 4"
- EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1693`
- EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1705`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:81`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:84`
- EVIDENCE: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:87`

## EVIDENCE
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/storage_conformance_no_env.log`
- LOG_SHA256: `a1909c06473d74ca1b0828ffb3682a08844327b9d343dffb75185171ba2d712c`
- PROOF_LINES: `test postgres_calendar_storage_conformance ... ok`; `test sqlite_calendar_storage_conformance ... ok`; `test postgres_storage_conformance ... ok`; `test sqlite_storage_conformance ... ok`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/calendar_storage_tests_no_env.log`
- LOG_SHA256: `e8690b005411e3d268b63b8749e05ded38d143a9b7bcd3ae3533bcf7981e60a3`
- PROOF_LINES: `test postgres_calendar_storage_conformance ... ok`; `test sqlite_calendar_storage_conformance ... ok`; `test result: ok. 2 passed; 0 failed`

- COMMAND: `docker compose -f docker-compose.test.yml up -d`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/docker_compose_test_up.log`
- LOG_SHA256: `1cfd5f66ba2a6f82161a78ac0882a65a956346a9f40c665d02c0e691bdfa6c54`
- PROOF_LINES: `Container wt-wp-1-calendar-storage-v1-postgres-1  Started`

- COMMAND: `$env:POSTGRES_TEST_URL="postgres://postgres:postgres@localhost:5432/handshake_test"; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests storage_conformance`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/storage_conformance_postgres_env.log`
- LOG_SHA256: `7542719dbe286d8b6fae41b355b38312902e2d37df2271ad300a085c4f4f1a3d`
- PROOF_LINES: `test sqlite_calendar_storage_conformance ... ok`; `test postgres_calendar_storage_conformance ... ok`; `test sqlite_storage_conformance ... ok`; `test postgres_storage_conformance ... ok`

- COMMAND: `$env:POSTGRES_TEST_URL="postgres://postgres:postgres@localhost:5432/handshake_test"; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/calendar_storage_tests_postgres_env.log`
- LOG_SHA256: `5cfc4b579f5dc202e6eae8d96a8bfe9b9570a3918f49402d749bb9fab3ffc725`
- PROOF_LINES: `test sqlite_calendar_storage_conformance ... ok`; `test postgres_calendar_storage_conformance ... ok`; `test result: ok. 2 passed; 0 failed`

- COMMAND: `just validator-scan`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/validator_scan.log`
- LOG_SHA256: `454af19a73fe81c71396d5dfdbc477b7d1b6e92622985cbc11ad1cef6bc55924`
- PROOF_LINES: `validator-scan: PASS - no forbidden patterns detected in backend/frontend sources.`

- COMMAND: `just validator-dal-audit`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/validator_dal_audit.log`
- LOG_SHA256: `3543f1aae3d1b56bc92c529262139b83f01ca307641e76aa64f84f6119e92bc5`
- PROOF_LINES: `validator-dal-audit: PASS (DAL checks clean).`

- COMMAND: `just validator-git-hygiene`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/validator_git_hygiene.log`
- LOG_SHA256: `e239d430a5fb2b9d05980f218b54c6a65e67f92d8d9deed67980fdc11ea66d23`
- PROOF_LINES: `validator-git-hygiene: PASS - .gitignore coverage and artifact checks clean.`

- COMMAND: `just cargo-clean`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v1/cargo_clean.log`
- LOG_SHA256: `71e43461618bcb06ff6b2eb7bf39f72dd8f0d6fa563ca66fae6fceb4fbd12949`
- PROOF_LINES: `Removed 2185 files, 16.3GiB total`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
