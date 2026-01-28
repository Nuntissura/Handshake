# Task Packet: WP-1-Global-Silent-Edit-Guard

## METADATA
- TASK_ID: WP-1-Global-Silent-Edit-Guard
- WP_ID: WP-1-Global-Silent-Edit-Guard
- BASE_WP_ID: WP-1-Global-Silent-Edit-Guard (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-28T15:54:34.390Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja280120261626

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Global-Silent-Edit-Guard.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Ensure StorageGuard "No Silent Edits" rejections (HSK-403-SILENT-EDIT) become operator-visible evidence by recording a Diagnostic and emitting FR-EVT-003 (DiagnosticEvent) linked to that Diagnostic.id. Keep storage-boundary enforcement intact across all persistence writes.
- Why: Silent edits are a traceability/audit failure. Guard violations must show up in Problems/Timeline and be linkable to a job so debugging and provenance claims are deterministic.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/api/canvases.rs
  - src/backend/handshake_core/src/api/workspaces.rs
  - src/backend/handshake_core/src/diagnostics/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/0001_init.sql
- OUT_OF_SCOPE:
  - app/ (front-end UX changes beyond existing error surfacing)
  - Changing the Master Spec or Handshake Codex

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Global-Silent-Edit-Guard
just test
just lint
just cargo-clean
just post-work WP-1-Global-Silent-Edit-Guard
```

### DONE_MEANS
- StorageGuard rejects AI write attempts without required context using error code `HSK-403-SILENT-EDIT` (GuardError::SilentEdit -> StorageError::Guard("HSK-403-SILENT-EDIT")). (Spec anchor: 2.9.2.2 Storage Guard Trait)
- Persistence writes continue to call StorageGuard and persist MutationMetadata fields (last_actor_kind/last_actor_id/last_job_id/last_workflow_id/edit_event_id) for core content tables; storage conformance tests remain green for sqlite+postgres where applicable. (Spec anchor: 2.9.2 Mutation Traceability + Integration Invariant)
- On `HSK-403-SILENT-EDIT` rejection from an API write route, a Diagnostic is recorded (code = HSK-403-SILENT-EDIT; severity = error; source = system/engine; job_id included when available) and Flight Recorder contains FR-EVT-003 (DiagnosticEvent) with payload.diagnostic_id == Diagnostic.id (no payload duplication). (Spec anchor: 11.5.1 FR-EVT-003)
- Add/extend an automated test that triggers a silent-edit guard rejection and asserts the DiagnosticEvent is present in Flight Recorder output.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.120.md (recorded_at: 2026-01-28T15:54:34.390Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.120.md 2.9.2 Mutation Traceability + 2.9.2.2 Storage Guard Trait (HSK-403-SILENT-EDIT); 11.5.1 FR-EVT-003 (DiagnosticEvent)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- N/A

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/refinements/WP-1-Global-Silent-Edit-Guard.md
  - Handshake_Master_Spec_v02.120.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/src/api/workspaces.rs
  - src/backend/handshake_core/src/api/canvases.rs
  - src/backend/handshake_core/src/diagnostics/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - "HSK-403-SILENT-EDIT"
  - "StorageGuard"
  - "validate_write"
  - "GuardError::SilentEdit"
  - "StorageError::Guard"
  - "map_storage_error"
  - "WriteContext::ai"
  - "x-hsk-actor-kind"
  - "x-hsk-job-id"
  - "x-hsk-workflow-id"
  - "record_diagnostic"
  - "DiagnosticInput"
  - "FlightRecorderEventType::Diagnostic"
  - "FrEvt003Diagnostic"
  - "build_diagnostic_event"
  - "DiagnosticsStore"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Global-Silent-Edit-Guard
  just test
  just lint
  ```
- RISK_MAP:
  - "Guard violation not recorded as Diagnostic" -> "Silent edit attempts remain invisible in Problems/Timeline; auditability gap persists"
  - "DiagnosticEvent payload invalid/missing diagnostic_id" -> "Flight Recorder event validation fails; UI cannot deep-link to Diagnostic"
  - "Over-recording duplicates" -> "Noise in Problems; operator loses signal"
  - "Guard accidentally weakened" -> "AI writes may proceed without job_id; violates No Silent Edits invariant"
  - "Job/workflow correlation wrong" -> "Events/diagnostics not attributable to the initiating job; hard to debug"

## SKELETON
- Proposed interfaces/types/contracts:
  - Use existing `crate::diagnostics::DiagnosticInput` + `DiagnosticsStore::record_diagnostic` for guard violations.
  - Reuse existing `FR-EVT-003` emission path (DuckDbFlightRecorder implements DiagnosticsStore and emits DiagnosticEvent).
- Open questions:
  - Where is the best interception point to record diagnostics for StorageError::Guard: per-route handlers vs shared helper?
  - Should workflow_id be recorded (message/tags) even though Diagnostic schema has no workflow_id field?
- Notes:

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

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
