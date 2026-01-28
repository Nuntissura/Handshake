## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Global-Silent-Edit-Guard
- CREATED_AT: 2026-01-28T00:00:00Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.120.md
- SPEC_TARGET_SHA1: 23df30111694202664fc65fe473106527ace5255
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja280120261626
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Global-Silent-Edit-Guard

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. The Master Spec defines MutationMetadata fields, required DB columns + constraints, the StorageGuard trait and its validation rules, and the required error code for silent AI write attempts.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- On StorageGuard rejection (HSK-403-SILENT-EDIT), the failing job/workflow MUST surface a Diagnostic and emit FR-EVT-003 (DiagnosticEvent) with a Diagnostic.id that points to the structured Problem record (no full payload duplication in Flight Recorder).
- Correlate the DiagnosticEvent to the originating job via job_id/workflow context so Operator Consoles can deep-link from Job -> Diagnostic/Problem.

### RED_TEAM_ADVISORY (security failure modes)
- Guard bypass: direct DB writes or storage code paths that do not call StorageGuard.validate_write can reintroduce silent edits; require an audit that all persistence paths route through the guard.
- Forged context: attacker/bug supplies arbitrary job_id/workflow_id to pass validation; require that job_id/workflow_id refer to a real, active job/workflow (and reject unknown IDs deterministically).
- Partial persistence: validating but not persisting returned MutationMetadata (including edit_event_id) breaks traceability; treat as a hard error.
- Cross-backend drift: SQLite vs Postgres constraints differ; ensure the invariant is enforced in both schema (constraints) and application logic.

### PRIMITIVES (traits/structs/enums)
- Mutation metadata: MutationMetadata, WriteActor, edit_event_id (UUID).
- Storage enforcement: StorageGuard trait with validate_write(actor, resource_id, job_id, workflow_id) -> Result<MutationMetadata, GuardError>.
- Diagnostics: Diagnostic + FR-EVT-003 (DiagnosticEvent) linkage on violations.
- Schema fields: last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id.
- Error code: HSK-403-SILENT-EDIT.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS (2.9.2.2 Storage Guard Trait; explicit rules)
- Explicitly named: [x] PASS (StorageGuard trait + "No Silent Edits" policy)
- Specific: [x] PASS (AI Write Context rule; edit_event_id rule; required columns; constraint)
- Measurable acceptance criteria: [x] PASS (reject AI writes with missing job_id; persist MutationMetadata; emit diagnostic)
- No ambiguity: [x] PASS (single interpretation: guard is mandatory on all persistence writes)
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The spec provides concrete schema fields + a StorageGuard trait contract + explicit validation rules and error code for rejecting silent AI writes.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already specifies the guard behavior, persistence requirements, and the required error code/diagnostic linkage; remaining choices (exact GuardError enum shape, where to wire the guard) are implementation details.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.120.md 2.9.2 Mutation Traceability (normative) + 2.9.2.2 Storage Guard Trait (Normative)
- CONTEXT_START_LINE: 15160
- CONTEXT_END_LINE: 15245
- CONTEXT_TOKEN: HSK-403-SILENT-EDIT
- EXCERPT_ASCII_ESCAPED:
  ```text
  2.9.2 Mutation Traceability (normative)

  - Storage Requirement: content tables MUST include last_actor_kind/last_job_id/last_workflow_id.
  - Audit Invariant: rows where last_actor_kind == "AI" MUST have non-null last_job_id.
  - Silent Edit Block: the storage guard MUST verify MutationMetadata is present/valid for AI-authored writes.

  2.9.2.2 Storage Guard Trait (HSK-TRAIT-001)

  - If actor == WriteActor::Ai, guard MUST fail if job_id is None.
  - Guard MUST generate a unique edit_event_id (UUID) for every successful validation and return it in MutationMetadata.
  - Error Codes: Use HSK-403-SILENT-EDIT for rejection.
  - Integration Invariant: all Database persistence methods MUST call validate_write and persist returned MutationMetadata fields.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.120.md 11.5.1 FR-EVT-003 (DiagnosticEvent)
- CONTEXT_START_LINE: 52864
- CONTEXT_END_LINE: 52883
- CONTEXT_TOKEN: type: 'diagnostic';
- EXCERPT_ASCII_ESCAPED:
  ```text
  FR-EVT-003 (DiagnosticEvent)

  A DiagnosticEvent links a Flight Recorder trace to a Diagnostic (Diagnostic.id) without duplicating the full payload.

  interface DiagnosticEvent extends FlightRecorderEventBase {
    type: 'diagnostic';
    diagnostic_id: string;
    severity?: 'fatal' | 'error' | 'warning' | 'info' | 'hint';
    source?: string;
  }
  ```
