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
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
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
- Proposed interfaces/types/contracts:
  - LocusStore (trait): persistence for WPs/MTs/dependencies/events
  - LocusService: orchestrates operations + emits Flight Recorder events
  - Locus operation job_kind: locus_operation + protocol_id locus_*_v1 (per spec 2.3.15.3 examples)
- Open questions:
  - Where is the authoritative home for Locus tables/migrations (existing storage layer vs dedicated module)?
  - What is the minimal Phase 1 implementation for locus_search (basic keyword search per Phase 1 pointer vs full hybrid)?
  - How to persist WPBronze/WPSilver (reuse existing AI-ready data medallion primitives vs new tables)?
- Notes:
  - Keep all artifacts deterministic (IDs, ordering, and Task Board sync behavior).

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
- **End**: 604
- **Line Delta**: 81
- **Pre-SHA1**: `5bca895d0a202f503ccc5200d1e8aac2bd56e617`
- **Post-SHA1**: `62d7fb55123c0ae1c5e62b57ce147a6ecf122a4a`
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
- **End**: 822
- **Line Delta**: 822
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `fc428affaabd118ce51c9d237077e3ec437ad3b3`
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
- **Post-SHA1**: `d0c14fbc5edb5e3f06d90634eb0dc4fbb2721eff`
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
- **End**: 462
- **Line Delta**: 462
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `a130b449614f3c9eaa0b996648e953c7e6e962f9`
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
- **End**: 1544
- **Line Delta**: 3
- **Pre-SHA1**: `8d4536e8d5be6d31c380981ce326e4828b18e9e4`
- **Post-SHA1**: `ca9da2989919de930b42c73f0917d0c092b98cc0`
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
- **End**: 7717
- **Line Delta**: 1117
- **Pre-SHA1**: `2893f87593559afa3644edfd13b1f69f8d57899b`
- **Post-SHA1**: `b53afda5cf934ac6eee48a81566fa4dd73bea6b1`
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
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

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
