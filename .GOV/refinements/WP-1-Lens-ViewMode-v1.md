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
- WP_ID: WP-1-Lens-ViewMode-v1
- CREATED_AT: 2026-02-24T10:04:30Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.137.md
- SPEC_TARGET_SHA1: 258012967E37EECAF5EABF3B163D7A363AFAD5B1
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja240220261300
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Lens-ViewMode-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (no Master Spec enrichment required for this WP).
- Concurrency constraint (Operator request): scope this WP to avoid editing the Unified Tool Surface Contract WP files:
  - assets/schemas/htc_v1.json
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/mcp/fr_events.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/tests/mcp_gate_tests.rs

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new Flight Recorder event families are required for ViewMode itself.
- ViewMode MUST be recorded as a metadata filter in QueryPlan/RetrievalTrace (ACE-RAG-001) as specified (see anchors).

### RED_TEAM_ADVISORY (security failure modes)
- Do-not-mutate: SFW projection MUST NOT write back into stored Raw/Derived artifacts.
- Hard-drop: in ViewMode="SFW", adult-tier items MUST be excluded from result sets (no "collapsed but revealable" leakage).
- Consistency: ViewMode filtering must be applied centrally so all Lens result surfaces honor it (search, panels, exports).

### PRIMITIVES (traits/structs/enums)
- ViewMode enum/type: "NSFW" | "SFW"
- Lens query filter includes view_mode and propagates to retrieval + rendering projection markers (projection_kind="SFW").

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.137 defines ViewMode semantics (projection-only, no write-back), the hard-drop rule for SFW, and the requirement to record ViewMode as a filter in QueryPlan/RetrievalTrace.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec already defines ViewMode behavior, the SFW hard-drop rule, and trace-recording requirements needed to implement and test this WP.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.137.md Addendum 2.4 ViewMode (SFW/NSFW)
- CONTEXT_START_LINE: 26976
- CONTEXT_END_LINE: 26988
- CONTEXT_TOKEN: **Addendum: 2.4 ViewMode (SFW/NSFW)**
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Addendum: 2.4 ViewMode (SFW/NSFW)**

  ```ts
  type ViewMode = "NSFW" | "SFW";

  /*
  NSFW: raw descriptors and raw rendering
  SFW: filtered projection for retrieval + rendering; never modifies stored descriptors
  */
  ```
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.137.md 6.3.3.5.7.22 NSFW/SFW policy (raw ingest; filtered view/output only)
- CONTEXT_START_LINE: 27443
- CONTEXT_END_LINE: 27466
- CONTEXT_TOKEN: Rule (hard drop):
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 6.3.3.5.7.22 NSFW/SFW policy (raw ingest; filtered view/output only) [ADD v02.123]

  Addendum: 11.2 SFW affects retrieval + output text only (HARD)

  - retrieval: strict drop -- Lens MUST exclude any candidate/result whose content_tier is not sfw.
  - output: apply projection rules during rendering only for remaining SFW-visible items.

  Rule (hard drop): In ViewMode="SFW", Lens MUST NOT return "collapsed/blurred but revealable" result items.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.137.md 2.3.14.x Hybrid Search + Two-Stage Retrieval (Lens filters must be recorded in trace)
- CONTEXT_START_LINE: 4520
- CONTEXT_END_LINE: 4520
- CONTEXT_TOKEN: Role filters (role_id, ViewMode, profile_id, etc.) are treated as metadata filters and must be recorded in the trace.
- EXCERPT_ASCII_ESCAPED:
  ```text
  [ADD v02.123] Lens role lanes are first-class retrieval lanes: role filters (role_id, ViewMode, profile_id, etc.) are treated as metadata filters and must be recorded in the trace.
  ```
