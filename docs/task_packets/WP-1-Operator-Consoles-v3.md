# Task Packet: WP-1-Operator-Consoles-v3

## METADATA
- TASK_ID: WP-1-Operator-Consoles-v3
- WP_ID: WP-1-Operator-Consoles-v3
- DATE: 2026-01-02T21:35:06.121Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja020120262232

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Operator-Consoles-v3.md
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
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 10.5.5.1-10.5.5.4; 11.4 (DIAG-SCHEMA-001/002/003 + 11.4.2 + 11.4.3); 11.5 FR-EVT-003
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ORCHESTRATOR_PROTOCOL.md
  - docs/VALIDATOR_PROTOCOL.md
  - docs/task_packets/WP-1-Operator-Consoles-v2.md
  - docs/task_packets/WP-1-Debug-Bundle-v3.md
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
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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
