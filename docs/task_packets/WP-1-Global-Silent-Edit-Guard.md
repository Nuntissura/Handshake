# Task Packet: WP-1-Global-Silent-Edit-Guard

## METADATA
- TASK_ID: WP-1-Global-Silent-Edit-Guard
- WP_ID: WP-1-Global-Silent-Edit-Guard
- BASE_WP_ID: WP-1-Global-Silent-Edit-Guard (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-28T15:54:34.390Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
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
- Spec anchors (non-negotiable; do not weaken):
  - No Silent Edits / StorageGuard contract: Handshake_Master_Spec_v02.120.md:15161 and Handshake_Master_Spec_v02.120.md:15203 (validate_write required; error code HSK-403-SILENT-EDIT).
  - FR-EVT-003 requirement: Handshake_Master_Spec_v02.120.md:52864 (payload contains diagnostic_id only; no full diagnostic duplication).
  - Existing guard behavior already correct; do not weaken: src/backend/handshake_core/src/storage/mod.rs:752 and src/backend/handshake_core/src/storage/mod.rs:765.

- Interception strategy (single; avoids double-recording):
  - Record the Diagnostic ONLY at the API error-to-HTTP boundary in the write routes in:
    - src/backend/handshake_core/src/api/workspaces.rs
    - src/backend/handshake_core/src/api/canvases.rs
  - Do NOT record inside storage (guard) and do NOT record inside write_context_from_headers.
  - Implementation pattern: on an error path, (1) detect silent edit precisely, (2) record Diagnostic best-effort, (3) return the existing HTTP 403 response.
    - This yields exactly one Diagnostic insert per failing request:
      - If write_context_from_headers rejects (mismatch/unknown job), we record there and the handler returns.
      - If storage validate_write rejects (missing job/workflow), we record on that storage call error path.

- Helpers to add (per module; small and local to avoid scope creep):
  1) `fn is_silent_edit(err: &StorageError) -> bool`
     - MUST match ONLY `StorageError::Guard("HSK-403-SILENT-EDIT")` (and optionally `StorageError::Validation("HSK-403-SILENT-EDIT")` if present).
     - MUST NOT treat all `StorageError::Guard(_)` as silent edit (noise control).

  2) `async fn record_silent_edit_diagnostic(state: &AppState, headers: &HeaderMap, wsid_hint: Option<&str>, ctx_hint: Option<&WriteContext>, err: &StorageError, route_tag: &'static str)`
     - Builds `crate::diagnostics::DiagnosticInput` with:
       - severity = `DiagnosticSeverity::Error`
       - surface = `DiagnosticSurface::System`
       - source = `DiagnosticSource::System` (or `Engine`; choose one and keep consistent)
       - code = `Some("HSK-403-SILENT-EDIT".to_string())`
       - title/message: stable strings (no request-unique IDs) to keep fingerprint stable and avoid noise.
       - wsid: `wsid_hint.map(str::to_string)`
       - job_id:
         - Prefer `ctx_hint.and_then(|c| c.job_id).map(|id| id.to_string())`
         - If ctx_hint is None (error before ctx exists), parse `x-hsk-job-id` best-effort from headers.
       - workflow_id handling (Diagnostic has no workflow_id field):
         - Recommended: add as a stable tag like `workflow_id:<uuid>` only if present, AND add a stable tag for failure mode:
           - `silent_edit:missing_context` vs `silent_edit:context_mismatch`
         - Open question: should we also embed workflow_id into message vs tags-only?
       - tags (stable + non-duplicative): e.g. `["hsk:guard", "hsk:silent_edit", "route:<route_tag>", "..."]`
     - Converts with `DiagnosticInput.into_diagnostic()?` and calls `state.diagnostics.record_diagnostic(diag).await`.
     - MUST NOT emit FR-EVT-003 manually; DuckDbFlightRecorder already emits it when recording the Diagnostic.
     - Failure handling: if record_diagnostic fails, log it (tracing) but still return the original 403 (do not fail open).

- Route wiring plan (keep existing HTTP semantics):
  - Replace `.map_err(map_storage_error)?` on write paths with explicit `match` blocks so we can `await` the diagnostic recording on the error path.
  - On error:
    - if `is_silent_edit(&err)` then `record_silent_edit_diagnostic(...).await` exactly once
    - return `Err(map_storage_error(err))` (same 403 payload as today)

- Test plan (must assert Diagnostic + FR-EVT-003 linkage; no payload duplication):
  - Extend existing test: src/backend/handshake_core/src/api/workspaces.rs:425 `replace_blocks_rejects_ai_when_context_missing`
    - After the 403 rejection:
      - Assert `state.diagnostics.list_diagnostics(...)` returns a Diagnostic with `code == Some("HSK-403-SILENT-EDIT")`.
      - Assert `state.flight_recorder.list_events(...)` contains a `FlightRecorderEventType::Diagnostic` event where `payload["diagnostic_id"] == diagnostic.id`.
      - Do NOT assert the payload contains full diagnostic fields (spec forbids duplication; FR-EVT-003 is link-only).

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
- Current WP_STATUS: In Progress
- What changed in this update: Drafted SKELETON interception plan for recording Diagnostics on HSK-403-SILENT-EDIT (no implementation yet).
- Next step / handoff hint: Await "SKELETON APPROVED" before implementing; then wire diagnostics recording into workspaces/canvases error paths and extend the existing API test.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
