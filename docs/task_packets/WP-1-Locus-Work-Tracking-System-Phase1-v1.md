# Task Packet: WP-1-Locus-Work-Tracking-System-Phase1-v1

## METADATA
- TASK_ID: WP-1-Locus-Work-Tracking-System-Phase1-v1
- WP_ID: WP-1-Locus-Work-Tracking-System-Phase1-v1
- BASE_WP_ID: WP-1-Locus-Work-Tracking-System-Phase1 (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-03T07:53:09.870Z
- MERGE_BASE_SHA: 85e20bf1071facd9b7e89e2777203f60b1b59b7c
- REQUESTOR: ilja (Operator)
- AGENT_ID: user_orchestrator (Codex CLI)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja030220260848
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Locus-Work-Tracking-System-Phase1-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement Phase 1 Locus Work Tracking System (storage + core operations + integrations + events) so Handshake can track Work Packets and Micro-Tasks end-to-end.
- Why: Provide governance-aware, deterministic work tracking that integrates with Spec Router, MT Executor, Task Board, Task Packets, and Flight Recorder.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/**
  - src/backend/handshake_core/src/locus/mod.rs
  - src/backend/handshake_core/src/locus/sqlite_store.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/mex/**
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/mechanical_engines.json
  - assets/capability_registry.json
  - src/backend/handshake_core/tests/**
  - docs/TASK_BOARD.md
  - docs/WP_TRACEABILITY_REGISTRY.md
  - docs/refinements/WP-1-Locus-Work-Tracking-System-Phase1-v1.md
  - docs/task_packets/WP-1-Locus-Work-Tracking-System-Phase1-v1.md
- OUT_OF_SCOPE:
  - Phase 2+: Postgres backend, CRDT, WebSocket real-time collaboration, multi-user workspaces (spec 2.3.15.8; roadmap 7.6.4+)
  - Full hybrid search (vector + keyword + graph + RRF) and Calendar policy modulation beyond Phase 1 baseline (roadmap 7.6.4)
  - Advanced analytics / auto-archival / AI insights (roadmap 7.6.6)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Locus-Work-Tracking-System-Phase1-v1

# Backend tests (adjust filters as tests land):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Registry sanity (if assets/capability_registry.json is updated):
node -e "JSON.parse(require('fs').readFileSync('assets/capability_registry.json','utf8')); console.log('capability_registry.json: ok')"

just cargo-clean
just post-work WP-1-Locus-Work-Tracking-System-Phase1-v1 --range 85e20bf1071facd9b7e89e2777203f60b1b59b7c..HEAD
```

### DONE_MEANS
- Locus storage schema exists for Phase 1 tracking per spec 2.3.15.5 (SQLite backend; work_packets, micro_tasks, mt_iterations, dependencies; plus locus_events where required).
- Core mechanical operations implemented per spec 2.3.15.3 and Phase 1 roadmap pointer (create/update/gate/close WP; register/start/record/complete MT; dependency add/remove with cycle detection; basic queries: query_ready, get_wp_status, get_mt_progress).
- Task Board bidirectional sync implemented per spec 2.3.15.4/2.3.15.3 (locus_sync_task_board reads/writes docs/TASK_BOARD.md; no drift after sync; 1:1 mapping between Task Board entries and Locus wp_id).
- Spec Router WorkPacketBinding enforcement implemented per spec 2.3.15.4 and Phase 1 roadmap pointer: work_packet_id must refer to an existing Locus wp_id; invalid/missing fails with Diagnostics and produces no side effects.
- Flight Recorder emits and validates Locus event families per spec 2.3.15.6: FR-EVT-WP-001..005, FR-EVT-MT-001..006, FR-EVT-DEP-001..002, FR-EVT-TB-001..003, FR-EVT-SYNC-001..003, FR-EVT-QUERY-001; unknown event_type fails fast in Diagnostics.
- Capability registry updated per spec 11.1 and Phase 1 roadmap pointer: locus.read/write/gate/delete/sync present in CapabilityRegistry SSoT; assets/capability_registry.json regenerated; HSK-4001 UnknownCapability coverage exists for locus operations.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.123.md (recorded_at: 2026-02-03T07:53:09.870Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.123.md 2.3.15 (Locus Work Tracking System)
  - Handshake_Master_Spec_v02.123.md 2.3.15.3 (Mechanical Operations)
  - Handshake_Master_Spec_v02.123.md 2.3.15.4 (Integration Points)
  - Handshake_Master_Spec_v02.123.md 2.3.15.5 (Storage Architecture)
  - Handshake_Master_Spec_v02.123.md 2.3.15.6 (Event Sourcing / Flight Recorder)
  - Handshake_Master_Spec_v02.123.md 2.3.15.7 (Query Interface)
  - Handshake_Master_Spec_v02.123.md 11.1 (Capabilities & Consent Model)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior planning artifact (stub): docs/task_packets/stubs/WP-1-Locus-Work-Tracking-System-Phase1-v1.md
- This packet activates the stub into an executable task packet (no prior executable packet exists for this Base WP ID).
- Preserved: scope sketch, dependencies, and Phase 1 intent.
- Changed: upgraded to signed refinement + executable packet with concrete IN_SCOPE_PATHS, TEST_PLAN, and DONE_MEANS.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - docs/refinements/WP-1-Locus-Work-Tracking-System-Phase1-v1.md
  - docs/task_packets/WP-1-Locus-Work-Tracking-System-Phase1-v1.md
  - Handshake_Master_Spec_v02.123.md
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - "Locus Work Tracking System"
  - "TrackedWorkPacket"
  - "locus_create_wp"
  - "locus_sync_task_board"
  - "FR-EVT-WP-001"
  - "locus.read"
- RUN_COMMANDS:
  ```bash
  rg -n "locus_" src/backend/handshake_core/src
  rg -n "FR-EVT-WP-|FR-EVT-MT-|FR-EVT-DEP-|FR-EVT-TB-|FR-EVT-SYNC-|FR-EVT-QUERY-" src/backend/handshake_core/src
  ```
- RISK_MAP:
  - "cross-cutting integration" -> "touches workflows, storage, and flight recorder; regressions can block other systems"
  - "schema drift" -> "missing FR event IDs / capability IDs causes hard-deny failures"

## SKELETON
- Goal (blocking): Restore the DAL boundary so `just validator-dal-audit` no longer flags CX-DBP-VAL-010/012, without changing Locus semantics.
- Hard rule after refactor:
  - Zero `sqlx::query*` and zero `SqlitePool` references in:
    - `src/backend/handshake_core/src/workflows.rs`
    - `src/backend/handshake_core/src/locus/*`
- Pattern choice: Option B (minimal diff): keep existing downcast to `crate::storage::sqlite::SqliteDatabase`, but move all SQLx usage into `src/backend/handshake_core/src/storage/*`.
- Proposed module move:
  - Add `src/backend/handshake_core/src/storage/locus_sqlite.rs` (SQLite Locus DAL; owns all SQLx for Locus persistence).
  - Move all functions that take `&SqlitePool` / call `sqlx::query*` from `src/backend/handshake_core/src/locus/sqlite_store.rs` into `src/backend/handshake_core/src/storage/locus_sqlite.rs` (semantics preserved; copy SQL verbatim).
- Storage-facing API (no SQLite types exposed outside storage):
  - `pub async fn execute_sqlite_locus_operation(sqlite: &SqliteDatabase, op: LocusOperation) -> StorageResult<Value>`
  - `pub async fn locus_work_packet_exists(sqlite: &SqliteDatabase, wp_id: &str) -> StorageResult<bool>` (replaces workflows.rs:2105)
  - `pub async fn locus_task_board_get_row(sqlite: &SqliteDatabase, wp_id: &str) -> StorageResult<Option<(String, String)>>` (replaces workflows.rs:326)
  - `pub async fn locus_task_board_update_row(sqlite: &SqliteDatabase, wp_id: &str, status: &str, task_board_status: &str, updated_at: &str, metadata_json: &str) -> StorageResult<()>` (replaces workflows.rs:378)
  - `pub async fn locus_task_board_list_rows(sqlite: &SqliteDatabase) -> StorageResult<Vec<(String, String, String)>>` (replaces workflows.rs:411)
- Non-storage module outcomes:
  - `src/backend/handshake_core/src/locus/sqlite_store.rs` becomes parse-only (`parse_locus_operation` stays; no SQLx/SqlitePool types).
  - `src/backend/handshake_core/src/workflows.rs` uses the storage-layer DAL helpers for the 4 flagged call sites; no inline SQL remains.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: job->storage (+ Task Board sync write)
- SERVER_SOURCES_OF_TRUTH:
  - Locus DB state (SQLite) for WP/MT/dependency truth
  - docs/TASK_BOARD.md content for Task Board sync (treated as untrusted input; validated)
- REQUIRED_PROVENANCE_FIELDS:
  - work_packet_id (wp_id), mt_id, spec_session_id (when present), task_packet_path (when present)
  - capability_id decisions for locus.* operations (allow/deny recorded)
  - emitted Flight Recorder event IDs for each operation
- VERIFICATION_PLAN:
  - Validator asserts: operations emit required FR-EVT-* families and reject unknown event_type.
  - Validator asserts: WorkPacketBinding fails closed (no side effects) on invalid wp_id.
- ERROR_TAXONOMY_PLAN:
  - UnknownCapability (HSK-4001) vs InvalidWorkPacketId vs SchemaValidationFailed vs StorageError vs SyncConflict
- UI_GUARDRAILS:
  - N/A (Phase 1 backend-focused; UI work is out of scope for this packet)
- VALIDATOR_ASSERTIONS:
  - FR event families emitted/validated per spec 2.3.15.6 and 11.5.
  - Capabilities enforced per spec 11.1; locus.* IDs present; unknown IDs fail with HSK-4001.
  - Task Board sync is deterministic and does not drift after locus_sync_task_board.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Enables `just post-work`. This section is NOT an official validation verdict.)

- **Target File**: `assets/capability_registry.json`
- **Start**: 1
- **End**: 503
- **Line Delta**: 65
- **Pre-SHA1**: `3608110366ffd1b93944aaf2f5232a319e755cf4`
- **Post-SHA1**: `ed2867907e26ac3739c7d1376d7d4d711cc06243`
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

- **Target File**: `src/backend/handshake_core/src/capabilities.rs`
- **Start**: 1
- **End**: 622
- **Line Delta**: 99
- **Pre-SHA1**: `5bca895d0a202f503ccc5200d1e8aac2bd56e617`
- **Post-SHA1**: `c6a94e35829d6f560ff0dea250dabd9fb7cc96c1`
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

- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 1
- **End**: 1240
- **Line Delta**: 28
- **Pre-SHA1**: `0d51d61a439ddace9226aec28bd01f07e9ccb035`
- **Post-SHA1**: `dc555b976a909e98402f7906b3dbfe3bac77220b`
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

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 3675
- **Line Delta**: 617
- **Pre-SHA1**: `c7f920abf3faa138cfe4db2315487d2c9bb1356e`
- **Post-SHA1**: `38b6d0fc8b1cf7bdd74d2213b9c272aa79e30c19`
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

- **Target File**: `src/backend/handshake_core/src/locus/mod.rs`
- **Start**: 1
- **End**: 5
- **Line Delta**: 5
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `e3a47c5cea8bba4bdb60865fe229a2ddcd7da9eb`
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

- **Target File**: `src/backend/handshake_core/src/locus/sqlite_store.rs`
- **Start**: 1
- **End**: 61
- **Line Delta**: 61
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `93213598b743263de25c0e52891a8c9f58b52d35`
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

- **Target File**: `src/backend/handshake_core/src/locus/task_board.rs`
- **Start**: 1
- **End**: 202
- **Line Delta**: 202
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `d0191f5ca5ca233afef59714dd8de131452c3bde`
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

- **Target File**: `src/backend/handshake_core/src/locus/types.rs`
- **Start**: 1
- **End**: 464
- **Line Delta**: 464
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `1b6afa556fcf2ed68eeca33fcbb4b0cd2170caf1`
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

- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 1545
- **Line Delta**: 4
- **Pre-SHA1**: `8d4536e8d5be6d31c380981ce326e4828b18e9e4`
- **Post-SHA1**: `bdebf6460660ffcf5a6efbc1609007f17abef1d1`
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

- **Target File**: `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Start**: 1
- **End**: 854
- **Line Delta**: 854
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `039c8f967d482d2a0568602e94bed658d478c90d`
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

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 2725
- **Line Delta**: 77
- **Pre-SHA1**: `6b4597ac725d3128685ab1b389384ca647503b7c`
- **Post-SHA1**: `dcd3454d782f76d23c5dc9988e05e71a1776a548`
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

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 7694
- **Line Delta**: 1094
- **Pre-SHA1**: `2893f87593559afa3644edfd13b1f69f8d57899b`
- **Post-SHA1**: `3dc957091d65d3f93da13208277a53af9b969b70`
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

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Bootstrap claim started; preparing DAL boundary remediation (remove SQLx + SqlitePool leakage from non-storage code).
- Next step / handoff hint: Produce SKELETON for the DAL boundary refactor and request approval before implementation.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
 - REQUIREMENT: "Locus storage schema exists for Phase 1 tracking per spec 2.3.15.5"
   - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:260`
 - REQUIREMENT: "Task Board bidirectional sync implemented per spec 2.3.15.4/2.3.15.3"
   - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:189`
 - REQUIREMENT: "WorkPacketBinding enforcement implemented per spec 2.3.15.4"
   - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2001`
 - REQUIREMENT: "Core mechanical operations implemented (create/update/gate/close WP; register/start/record/complete MT; deps)"
   - EVIDENCE: `src/backend/handshake_core/src/locus/sqlite_store.rs:159`
 - REQUIREMENT: "Flight Recorder validates Locus event families per spec 2.3.15.6"
   - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:2337`
 - REQUIREMENT: "Capability registry updated: locus.read present in CapabilityRegistry SSoT"
   - EVIDENCE: `assets/capability_registry.json:402`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Locus-Work-Tracking-System-Phase1-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
 - COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml -q`
   - EXIT_CODE: 0
 - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib -q`
   - EXIT_CODE: 0
 - COMMAND: `node -e "JSON.parse(require('fs').readFileSync('assets/capability_registry.json','utf8')); console.log('capability_registry.json: ok')"`
   - EXIT_CODE: 0

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
