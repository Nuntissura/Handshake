# Task Packet: WP-1-Operator-Consoles-v3

## METADATA
- TASK_ID: WP-1-Operator-Consoles-v3
- WP_ID: WP-1-Operator-Consoles-v3
- DATE: 2026-01-02T21:35:06.121Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja020120262232

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Operator-Consoles-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Bring Operator Consoles (Problems, Jobs, Timeline, Evidence Drawer) back into spec-compliant shape for Master Spec v02.100, including the canonical Diagnostics schema and FR linkage requirements that these consoles depend on.
- Why: Operator Consoles are the debugging control room; if these surfaces are not deterministic and linkable (Diagnostics <-> Jobs <-> Timeline <-> Evidence), the project loses the ability to produce redaction-safe, deterministic evidence and development stalls.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/diagnostics/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/api/diagnostics.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/jobs.rs
  - app/src/components/operator/ProblemsView.tsx
  - app/src/components/operator/JobsView.tsx
  - app/src/components/operator/TimelineView.tsx
  - app/src/components/operator/EvidenceDrawer.tsx
  - app/src/components/operator/DebugBundleExport.tsx
  - app/src/components/operator/index.ts
  - app/src/lib/api.ts
  - app/src/App.tsx
- OUT_OF_SCOPE:
  - Implementing the Debug Bundle exporter contract itself (covered by WP-1-Debug-Bundle-v3, already VALIDATED); only wire consoles to existing export surfaces/endpoints.
  - Implementing Operator Consoles surfaces 10.5.5.5-10.5.5.8 beyond what is required by 10.5.5.1-10.5.5.4 for deep-linking and evidence viewing.
  - Any Master Spec edits (requires separate enrichment workflow and signature).
  - Broad refactors unrelated to console/diagnostics/FR linkage.

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Operator-Consoles-v3

# Spec guard
just validator-spec-regression

# Backend
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Frontend
pnpm -C app run lint
pnpm -C app test

# Optional (recommended) targeted scan
just validator-scan

# Determinism / workflow gates
just cargo-clean
just post-work WP-1-Operator-Consoles-v3
```

### DONE_MEANS
- SPEC 10.5.5.1 Problems MUST:
  - Render normalized diagnostics (DIAG-SCHEMA-001) and not raw/unstructured blobs.
  - Provide filters: severity, source, surface, wsid, job_id, time_range.
  - Group by deterministic fingerprint (DIAG-SCHEMA-003) while retaining access to raw instances.
  - Support local-only statuses: open, ack, mute, resolved.
  - Open Evidence Drawer on selection.
- SPEC 10.5.5.2 Jobs MUST:
  - List jobs with filters: status, kind, wsid, time range.
  - Provide Job Inspector tabs: Summary, Timeline, Inputs/Outputs (hash-based), Diagnostics, Policy.
  - Allow exporting a Debug Bundle scoped to a job (via existing Debug Bundle implementation).
- SPEC 10.5.5.3 Timeline MUST:
  - Render a time-window view over Flight Recorder events (canonical: 11.5).
  - Provide filters: job_id, wsid, actor, surface, event types.
  - Allow opening Evidence Drawer for any event.
  - Support a stable "pin this slice" mechanism for bundle export (query determinism).
- SPEC 10.5.5.4 Evidence Drawer MUST:
  - Show an evidence card for a selected diagnostic or event including: raw JSON (redacted view default), linked entities (job/wsid/spans), relevant policy/capability decisions, related artifacts by hash, link_confidence + correlation explanation.
  - Provide an "Export Debug Bundle" entrypoint.
- SPEC 11.4 Diagnostics MUST:
  - Conform to DIAG-SCHEMA-001 field names (including DiagnosticRange: startLine/startColumn/endLine/endColumn).
  - Implement DIAG-SCHEMA-003 fingerprint determinism rules (canonical tuple; normalize path separators; normalize CRLF to LF; absent fields canonicalized to explicit null; sha256 lowercase hex).
  - Store diagnostics in DuckDB using the 11.4.2 schema types (severity enum; link_confidence enum; indexes present).
  - Provide a DiagnosticsStore implementation matching the 11.4.3 trait signatures (record_diagnostic, list_problems).
- SPEC 11.5 FR-EVT-003 MUST:
  - Ensure any diagnostic-to-Flight-Recorder linkage uses FR-EVT-003 (DiagnosticEvent) with diagnostic_id == Diagnostic.id (no duplicate full diagnostic payload).
- Workflow gates MUST:
  - `just pre-work WP-1-Operator-Consoles-v3` passes before implementation starts.
  - `just post-work WP-1-Operator-Consoles-v3` passes after changes (COR-701 manifest present and accurate).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.100.md (recorded_at: 2026-01-02T21:35:06.121Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 10.5.5.1-10.5.5.4; 11.4 (DIAG-SCHEMA-001/002/003 + 11.4.2 + 11.4.3); 11.5 FR-EVT-003
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md
  - .GOV/roles/validator/VALIDATOR_PROTOCOL.md
  - .GOV/task_packets/WP-1-Operator-Consoles-v2.md
  - .GOV/task_packets/WP-1-Debug-Bundle-v3.md
  - Handshake_Master_Spec_v02.100.md
  - src/backend/handshake_core/src/diagnostics/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/api/diagnostics.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - app/src/components/operator/ProblemsView.tsx
  - app/src/components/operator/JobsView.tsx
  - app/src/components/operator/TimelineView.tsx
  - app/src/components/operator/EvidenceDrawer.tsx
- SEARCH_TERMS:
  - "DIAG-SCHEMA-001"
  - "DIAG-SCHEMA-003"
  - "DiagnosticRange"
  - "fingerprint"
  - "link_confidence"
  - "DiagnosticsStore"
  - "record_diagnostic"
  - "list_problems"
  - "FR-EVT-003"
  - "DiagnosticEvent"
  - "ProblemsView"
  - "JobsView"
  - "TimelineView"
  - "EvidenceDrawer"
- RUN_COMMANDS:
  ```bash
  # Phase gate
  just pre-work WP-1-Operator-Consoles-v3

  # Backend
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml

  # Frontend
  pnpm -C app run lint
  pnpm -C app test

  # Determinism gates
  just cargo-clean
  just post-work WP-1-Operator-Consoles-v3
  ```
- RISK_MAP:
  - "Fingerprint non-determinism" -> "Problems grouping becomes unstable; repeated failures become invisible"
  - "Evidence Drawer not redacted-by-default" -> "Secrets/PII leakage risk"
  - "Deep-linking breaks (job_id/diagnostic_id/event_id/wsid)" -> "Operator cannot trace causality; debugging stalls"
  - "Incorrect link_confidence semantics" -> "UI presents ambiguous links as direct; mis-triage risk"
  - "FR-EVT-003 mismatch" -> "Diagnostics cannot be navigated from Timeline; audit trail breaks"
  - "DuckDB schema drift" -> "Queries/filters fail; Operators lose ability to filter deterministically"

## SKELETON
- Proposed interfaces/types/contracts:
- Confirm existing types match the canonical spec shapes:
  - DiagnosticRange / Diagnostic / ProblemGroup / DiagFilter
  - DiagnosticsStore trait + storage schema
  - Flight Recorder DiagnosticEvent (FR-EVT-003) linkage
- Validator Decisions (binding; record before any code changes):
  - Timeline selection is a time-range query surface:
    - Treat "pin this slice" and "export from event" as `time_window` scope by default.
    - Spec anchors: Handshake_Master_Spec_v02.100.md:33812, Handshake_Master_Spec_v02.100.md:33843, Handshake_Master_Spec_v02.100.md:26481.
  - Evidence Drawer export default (event selection):
    - Default export scope = `{ kind: "time_window", time_range: { start, end }, wsid? }` using the current Timeline filter window (not job scope).
    - If there is no active time window in UI state, do not guess a window size; require the user to set/confirm a window, OR provide an explicit secondary "Export job bundle" option if `job_id` exists.
    - Reason: calendar semantics and overlap attribution are time-window based; jobs may not capture cross-job spans.
      - Spec anchors: Handshake_Master_Spec_v02.100.md:24705, Handshake_Master_Spec_v02.100.md:33821.
  - Pinned slice -> Debug Bundle mapping (given current exporter contract):
    - Persist the pinned slice as a deterministic "query object" (time_range + wsid + user-selected filters).
    - Derive a stable `slice_id` from a normalized representation.
    - When calling Debug Bundle export, map to supported scope only: `time_window` (+ optional `wsid`).
      - Frontend contract: app/src/lib/api.ts:505
      - Backend parser: src/backend/handshake_core/src/api/bundles.rs:96
    - UI disclaimer: "Export uses time window (+wsid) only; actor/surface/event_type filters are UI-only until Debug Bundle contract expands in a separate WP."
    - Spec anchor for deterministic query requirement: Handshake_Master_Spec_v02.100.md:33843.
  - Correlation explanation behavior:
    - Implement pure rule-based, deterministic explanations (no extra fetch/lookups by default).
    - Reason: calendar/external text is treated as untrusted class and determinism is required.
      - Anchors: src/backend/handshake_core/src/ace/validators/injection.rs:6, Handshake_Master_Spec_v02.100.md:33843.
  - Scope hygiene:
    - Do not change Debug Bundle API contracts in this WP; only wire consoles to existing `time_window`/`job` scopes.

- Open questions:
  - None beyond the binding decisions above; implementation will follow these constraints exactly.
- Notes:
  - Implementation remains blocked until an explicit "SKELETON APPROVED ..." marker is issued by the Validator (phase gate).

SKELETON APPROVED
SKELETON_APPROVED_BY: ilja020120262232

## IMPLEMENTATION
- Backend: extend Flight Recorder API event envelope returned by `/api/flight_recorder` to include model/span/policy/capability fields needed by Evidence Drawer for event selections.
- Frontend:
  - Evidence Drawer: event export defaults to `time_window` using the current Timeline filter window; disables export when no active window is set; provides explicit secondary "Export job bundle" when `job_id` exists.
  - Timeline: "Pin this slice" requires a valid From/To window; pinned slices persist as deterministic query objects with stable `slice_id`; export mapping uses `time_window` (+ optional `wsid`) only and displays the required disclaimer.
  - App wiring: propagate current Timeline time window to Evidence Drawer for time-window export behavior.
  - API types: extend `FlightEvent` to surface span/policy/capability fields.

## HYGIENE
- Commands run (details in `## EVIDENCE`):
  - just pre-work WP-1-Operator-Consoles-v3
  - just validator-spec-regression
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - pnpm -C app run lint
  - pnpm -C app test
  - just validator-scan
  - just cargo-clean

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- **Target File**: `src/backend/handshake_core/src/api/flight_recorder.rs`
- **Start**: 21
- **End**: 124
- **Line Delta**: 10
- **Pre-SHA1**: `98c453acac64a4ee29ab8f0027d9af8e602d345b`
- **Post-SHA1**: `5bb4905700f59229bd26a57ab0afbe71e4c4b4b2`
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

- **Target File**: `app/src/lib/api.ts`
- **Start**: 153
- **End**: 157
- **Line Delta**: 4
- **Pre-SHA1**: `885d97355ff6e261f27336a1d2161bfe05535267`
- **Post-SHA1**: `2f9501c83f5b31f76c43fccf6f8287a59c94e53c`
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

- **Target File**: `app/src/App.tsx`
- **Start**: 27
- **End**: 178
- **Line Delta**: -4
- **Pre-SHA1**: `45ee7c0d8dca4d4900d9a7472e1abd2649be4c3a`
- **Post-SHA1**: `571f5f6bd5a446684e1a5c3f452a8e3519bcd528`
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

- **Target File**: `app/src/components/operator/EvidenceDrawer.tsx`
- **Start**: 2
- **End**: 376
- **Line Delta**: 71
- **Pre-SHA1**: `c7de86a02a6feb5e66f682ad5f1521f750b31d62`
- **Post-SHA1**: `33beaaeb330ae9d74fb5ded3c1299fe984bc0791`
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

- **Target File**: `app/src/components/operator/TimelineView.tsx`
- **Start**: 2
- **End**: 402
- **Line Delta**: 158
- **Pre-SHA1**: `57098b77b1cb6b661af6a4cf8da1e2b7d9eab868`
- **Post-SHA1**: `9285dff0c32e8c6f223f660b4160c50f14340e92`
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
- **Lint Results**:
- pnpm -C app run lint: OK
- just validator-scan: OK
- **Artifacts**:
- None (no new build artifacts committed).
- **Timestamp**:
- 2026-01-02T22:39:47Z
- **Operator**:
- GPT-5.2 (Codex CLI)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- **Notes**:
- Manifests captured against staged diff; `just post-work WP-1-Operator-Consoles-v3` executed successfully.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:
  - Current WP_STATUS: Implementation complete; ready for Validator review.
  - What changed in this update:
    - Timeline time-window export is enforced (no guessed windows); pinned slices persist with stable slice_id; exports map to time_window (+wsid) only with required disclaimer.
    - Evidence Drawer event export defaults to time_window using the current Timeline window; job export is explicit secondary.
    - Flight Recorder API includes span/policy/capability fields so Evidence Drawer can show linked entities for events.
  - Next step / handoff hint:
    - Validator: run `just post-work WP-1-Operator-Consoles-v3` on the staged diff and review operator console behavior vs DONE_MEANS 10.5.5.1-10.5.5.4.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Command: cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - Result: OK (all tests passed; see console output)
- Command: pnpm -C app run lint
  - Result: OK
- Command: pnpm -C app test
  - Result: OK (5 files, 8 tests)
- Command: just validator-spec-regression
  - Result: OK
- Command: just validator-scan
  - Result: OK
- Command: just cargo-clean
  - Result: OK
- Command: just post-work WP-1-Operator-Consoles-v3
  - Result: OK ("Post-work validation PASSED")

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT - WP-1-Operator-Consoles-v3 (2026-01-03)
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Operator-Consoles-v3.md (Status: In Progress)
- Refinement: .GOV/refinements/WP-1-Operator-Consoles-v3.md (approved/signed)
- Spec target resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md

Commit / Diff:
- Commit reviewed: 063de0ac (feat: operator consoles time-window exports [WP-1-Operator-Consoles-v3])
- Files changed:
  - app/src/App.tsx
  - app/src/components/operator/EvidenceDrawer.tsx
  - app/src/components/operator/TimelineView.tsx
  - app/src/lib/api.ts
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/task_packets/WP-1-Operator-Consoles-v3.md

Pre-Flight / Gates:
- just pre-work WP-1-Operator-Consoles-v3: PASS
- Phase gate: sequence respected; SKELETON marker present:
  - .GOV/task_packets/WP-1-Operator-Consoles-v3.md:206 (SKELETON APPROVED)

Spec / DONE_MEANS requirements -> evidence mapping (selected MUST items):
- Timeline MUST support "pin this slice" (stable query) for bundle export (Handshake_Master_Spec_v02.100.md:26481):
  - Time-window is required; no guessed windows: app/src/components/operator/TimelineView.tsx:193
  - Deterministic pinned query + stable slice_id (sha256 over stable-stringified query): app/src/components/operator/TimelineView.tsx:52
  - Export maps to Debug Bundle time_window (+ optional wsid) only: app/src/components/operator/TimelineView.tsx:207
  - Disclaimer shown that UI-only filters are not applied to export scope: app/src/components/operator/TimelineView.tsx:316
- Evidence Drawer MUST provide correlation explanation + Export Debug Bundle entrypoint (Handshake_Master_Spec_v02.100.md:26492):
  - Rule-based correlation explanation (deterministic; no lookups): app/src/components/operator/EvidenceDrawer.tsx:19
  - Event export defaults to time_window using current Timeline window; disables when window absent; job export is explicit secondary: app/src/components/operator/EvidenceDrawer.tsx:293
  - Raw JSON redacted-by-default for diagnostics and events: app/src/components/operator/EvidenceDrawer.tsx:257 and app/src/components/operator/EvidenceDrawer.tsx:337
- Evidence Drawer requires linked entities (job/wsid/spans) and policy/capability IDs when present (Handshake_Master_Spec_v02.100.md:26492):
  - Flight Recorder API returns model_id/activity_span_id/session_span_id/capability_id/policy_decision_id: src/backend/handshake_core/src/api/flight_recorder.rs:12
  - Frontend type surfaces those fields: app/src/lib/api.ts:136
- App wiring: Timeline window propagates to Evidence Drawer export behavior:
  - Timeline emits active window: app/src/components/operator/TimelineView.tsx:183
  - App stores and passes window: app/src/App.tsx:31 and app/src/App.tsx:156

Forbidden Patterns (Codex CX-573E / Validator Protocol 2B):
- just validator-scan: PASS
- Spot check rg on touched files for split_whitespace/unwrap/expect/todo!/unimplemented!/dbg!/println!/eprintln!: no matches

Tests / Commands (Validator re-run):
- just validator-spec-regression: PASS
- just cargo-clean: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- pnpm -C app run lint: PASS
- pnpm -C app test: PASS

Deterministic Manifest (COR-701):
- Packet contains per-file manifests for all changed non-doc files: .GOV/task_packets/WP-1-Operator-Consoles-v3.md:227
- Independent verification of just post-work (must be run against a staged diff):
  - Created an isolated validation worktree at pre-change HEAD (4e57fd93), applied 063de0ac as staged changes (cherry-pick -n), then ran:
    - just post-work WP-1-Operator-Consoles-v3: PASS

Hygiene / Notes:
- Sandbox note: some Node child-process spawns (cmd.exe) required escalated execution for validator .GOV/scripts/tests in this environment; commands above were re-run and passed with full outputs available in console history.

REASON FOR PASS:
- DONE_MEANS requirements for the changed surfaces are met with direct evidence (file:line), forbidden-pattern scan is clean, required tests/lint pass, and COR-701 post-work determinism gate passes for the staged WP diff in a clean validation worktree.


