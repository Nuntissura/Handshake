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
- WP_ID: WP-1-Atelier-Collaboration-Panel-v1
- CREATED_AT: 2026-01-31T15:14:47Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- SPEC_TARGET_SHA1: 4D406DCC1A75570D2F17659E0AC40D68A22F211A
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja310120261839
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Atelier-Collaboration-Panel-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Potential ambiguity: "boundary-normalization" is referenced but not defined elsewhere in the spec.
- Proposed resolution (no spec enrichment): ship v1 with boundary-normalization disabled; any patch that modifies outside the selection range is rejected.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-002 `editor_edit` MUST be emitted when the operator applies an approved suggestion to Monaco/Docs, with ops ranges strictly within the selection.
- FR-EVT-006 `llm_inference` MUST be emitted for role suggestion generation (per role pass), with trace_id + model_id required.
- FR-EVT-003 `diagnostic` SHOULD be emitted when validators reject an out-of-scope patch application attempt (selection-bound violation), linked by job_id where applicable.

### RED_TEAM_ADVISORY (security failure modes)
- Range mapping bugs: off-by-one or mismatched range semantics between UI surface and validator can allow edits outside selection (silent edits).
- Boundary-normalization loophole: if enabled without strict rules, it can become a way to modify outside selection; keep disabled for v1.
- Logging leakage: EditorEditEvent may include insert/delete text; policy must avoid logging sensitive content inline (prefer hashes/refs where possible).
- UX bypass: alternate edit paths that do not go through patch-set discipline and validators would defeat selection-bounding.

### PRIMITIVES (traits/structs/enums)
- `SelectionRange` (struct): normalized selection span (surface-specific) for patch validation.
- `RoleSuggestion` (struct): role_id + 0..n suggestions per role, each suggestion maps to a candidate patch within selection.
- `SelectionBoundedPatchValidator` (validator): rejects any patchset that modifies outside selection range.
- `AtelierCollaborationEvent` (struct/enum): correlation glue between role passes (job_id) and applied patch events.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec defines required workflow steps and hard selection-bounded application rules; boundary-normalization can be disabled in v1 without adding new normative text.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Spec already defines the selection-scoped collaboration workflow and validator requirement; boundary-normalization is optional and can remain disabled for v1.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 14.2.1 Atelier Collaboration Panel (selection-scoped) (HARD)
- CONTEXT_START_LINE: 46603
- CONTEXT_END_LINE: 46616
- CONTEXT_TOKEN: Non-selected text MUST remain byte-identical
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 14.2.1 Atelier Collaboration Panel (selection-scoped) (HARD)
  
  Atelier MUST support a \\u201ccollaborate on selection\\u201d workflow in text surfaces (Monaco/Docs):
  
  1. Operator selects a bounded text span.
  2. Operator invokes Atelier collaboration (button/shortcut).
  3. System shows **all roles** in a side panel; each role may emit **0..n suggestions** (multiple suggestions are preferred when available).
  4. Operator checks one or more suggestions and applies them.
  
  Application rules:
  - The resulting `monaco_patchset` / `doc_patchset` MUST be **range-bounded** to the selected span.
  - Validators MUST reject any patch that modifies text outside the selection range (except for explicitly declared boundary-normalization, if enabled).
  - Non-selected text MUST remain byte-identical after patch application.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 14.3 Validators (addendum-required) (ATELIER-LENS-VAL-SCOPE-001)
- CONTEXT_START_LINE: 46617
- CONTEXT_END_LINE: 46630
- CONTEXT_TOKEN: ATELIER-LENS-VAL-SCOPE-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 14.3 Validators (addendum-required)
  
  Add the following validators (names are indicative; binding points are normative):
  
  - `ATELIER-LENS-VAL-SCOPE-001` \\u2014 compose patchsets MUST be selection-bounded; changes outside the operator selection are rejected
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 11.5.1 Flight Recorder Event Shapes (FR-EVT-002 EditorEditEvent)
- CONTEXT_START_LINE: 57565
- CONTEXT_END_LINE: 57634
- CONTEXT_TOKEN: interface EditorEditEvent extends FlightRecorderEventBase
- EXCERPT_ASCII_ESCAPED:
  ```text
  interface FlightRecorderEventBase {
    type: string;
    event_id: string;                 // uuid
    timestamp: string;                // RFC3339
    actor: FlightRecorderActor;
  
    // Correlation / navigation
    job_id?: string;
    model_id?: string;
    wsids?: string[];
    activity_span_id?: string;
    session_span_id?: string;
  
    // Governance
    capability_id?: string;
    policy_decision_id?: string;
  }
  
  interface EditorEditEvent extends FlightRecorderEventBase {
    type: 'editor_edit';
  
    editor_surface: 'monaco' | 'canvas' | 'sheet';
    document_uri?: string | null;
    path?: string | null;
  
    before_hash?: string | null;
    after_hash?: string | null;
    diff_hash?: string | null;
  
    ops: EditorEditOp[];
  }
  ```

