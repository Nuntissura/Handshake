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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Global-Silent-Edit-Guard.md
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
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.120.md 2.9.2 Mutation Traceability + 2.9.2.2 Storage Guard Trait (HSK-403-SILENT-EDIT); 11.5.1 FR-EVT-003 (DiagnosticEvent)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- N/A

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/refinements/WP-1-Global-Silent-Edit-Guard.md
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
- Added API-boundary recording for HSK-403-SILENT-EDIT that reuses existing DiagnosticsStore + DuckDB Flight Recorder emission path (no manual FR emit).
- Kept StorageGuard semantics unchanged (validate_write still enforces No Silent Edits and returns StorageError::Guard("HSK-403-SILENT-EDIT")).
- Extended the workspaces API rejection test to assert a Diagnostic is recorded and a Flight Recorder Diagnostic event exists with payload.diagnostic_id == Diagnostic.id (no payload duplication).

## HYGIENE
- Ran: `just pre-work WP-1-Global-Silent-Edit-Guard`
- Ran: `just test`
- Ran: `just lint`
- Ran: `just cargo-clean`
- Ran: `just validator-scan`
- Ran: `just validator-dal-audit`
- Ran: `just validator-git-hygiene`
- Ran: `just post-work WP-1-Global-Silent-Edit-Guard`

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/api/canvases.rs`
- **Start**: 1
- **End**: 530
- **Line Delta**: 179
- **Pre-SHA1**: `8d95977f68225f9ad495e46a61cc5535c194f744`
- **Post-SHA1**: `57d9742766122e0a5d4122e337612f06a9259e97`
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

- **Target File**: `src/backend/handshake_core/src/api/workspaces.rs`
- **Start**: 1
- **End**: 823
- **Line Delta**: 254
- **Pre-SHA1**: `04953858abf84376480a7b47f34a3e55c4166f9a`
- **Post-SHA1**: `78ef525e962e60d81e73c2379b9f16d4df676791`
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

- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.120.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Implemented API-boundary Diagnostic recording on HSK-403-SILENT-EDIT for workspaces/canvases, and updated the existing workspaces rejection test to assert Diagnostic + Diagnostic event linkage.
- Touched files:
  - src/backend/handshake_core/src/api/workspaces.rs
  - src/backend/handshake_core/src/api/canvases.rs
- Next step / handoff hint: Commit the staged diff on this branch and notify Validator for review/merge.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Command: `just pre-work WP-1-Global-Silent-Edit-Guard` (exit 0)
  - Output (excerpt):
    - Checking Phase Gate for WP-1-Global-Silent-Edit-Guard...
    - Pre-work validation for WP-1-Global-Silent-Edit-Guard...

- Command: `just test` (exit 0)
  - Output (excerpt):
    - running 151 tests
    - test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

- Command: `just lint` (exit 0)
  - Output (excerpt):
    - > app@0.1.0 lint
    - cd src/backend/handshake_core; cargo clippy --all-targets --all-features

- Command: `just cargo-clean` (exit 0)
  - Output:
    - Removed 1799 files, 9.2GiB total

- Command: `just post-work WP-1-Global-Silent-Edit-Guard` (exit 0)
  - Output:
    - Post-work validation for WP-1-Global-Silent-Edit-Guard (deterministic manifest + gates)...
    - Post-work validation PASSED

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATOR_REPORT (2026-01-28)

VALIDATION REPORT â€” WP-1-Global-Silent-Edit-Guard
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Global-Silent-Edit-Guard.md (status: In Progress)
- Spec: Handshake_Master_Spec_v02.120.md Â§2.9.2, Â§2.9.2.2, Â§11.5.1

Files Checked:
- .GOV/roles_shared/SPEC_CURRENT.md
- .GOV/roles_shared/TASK_BOARD.md
- .GOV/task_packets/WP-1-Global-Silent-Edit-Guard.md
- src/backend/handshake_core/src/api/canvases.rs
- src/backend/handshake_core/src/api/workspaces.rs
- src/backend/handshake_core/src/flight_recorder/mod.rs
- src/backend/handshake_core/src/storage/mod.rs
- src/backend/handshake_core/src/storage/tests.rs

Findings:
- StorageGuard error mapping preserves `HSK-403-SILENT-EDIT`: src/backend/handshake_core/src/storage/mod.rs:735
- Workspaces API records a Diagnostic on silent-edit rejections: src/backend/handshake_core/src/api/workspaces.rs:78
- Canvases API records a Diagnostic on silent-edit rejections: src/backend/handshake_core/src/api/canvases.rs:80
- FR-EVT-003 Diagnostic payload is link-only (requires `diagnostic_id`): src/backend/handshake_core/src/flight_recorder/mod.rs:661
- Automated test asserts Diagnostic + DiagnosticEvent linkage + no payload duplication: src/backend/handshake_core/src/api/workspaces.rs:660

Hygiene:
- `just validator-scan`: PASS

Tests:
- `just test`: PASS (151 tests + integration suites). Existing warnings present in `handshake_core` (unused assignments, dead code).

Coverage note:
- Removing the silent-edit Diagnostic record or breaking DiagnosticEvent emission fails `replace_blocks_rejects_ai_when_context_missing`.

Risks & Suggested Actions:
- Consider refactoring duplicate silent-edit diagnostic helpers between workspaces/canvases into a shared module to reduce drift.

Improvements & Future Proofing:
- Consider tightening `validate_diagnostic_payload` to enforce exact keys (diagnostic_id only) if spec hardens against payload drift.

### REASON FOR PASS
- DONE_MEANS met: silent-edit guard remains enforced with `HSK-403-SILENT-EDIT`, API routes record a Diagnostic on rejection, FR includes a link-only DiagnosticEvent referencing `diagnostic_id`, and a targeted automated test covers the behavior; `just test` + `just validator-scan` pass.

### VALIDATOR_ADDENDUM (2026-01-28)
- Spec resolved at commit time via `.GOV/roles_shared/SPEC_CURRENT.md`: Handshake_Master_Spec_v02.121.md (anchors unchanged for this WP).


