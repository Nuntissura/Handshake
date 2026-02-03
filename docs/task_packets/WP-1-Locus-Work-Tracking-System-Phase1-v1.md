# Task Packet: WP-1-Locus-Work-Tracking-System-Phase1-v1

## METADATA
- TASK_ID: WP-1-Locus-Work-Tracking-System-Phase1-v1
- WP_ID: WP-1-Locus-Work-Tracking-System-Phase1-v1
- BASE_WP_ID: WP-1-Locus-Work-Tracking-System-Phase1 (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-03T07:53:09.870Z
- MERGE_BASE_SHA: 85e20bf1071facd9b7e89e2777203f60b1b59b7c (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
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
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/mex/**
  - src/backend/handshake_core/src/flight_recorder/**
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
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
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
  - LOG_PATH: `.handshake/logs/WP-1-Locus-Work-Tracking-System-Phase1-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
