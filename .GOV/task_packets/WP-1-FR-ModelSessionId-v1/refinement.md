## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by the current ORCHESTRATOR_PROTOCOL refinement workflow.

### METADATA
- WP_ID: WP-1-FR-ModelSessionId-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-05T01:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- SPEC_TARGET_SHA1: 747a1e77cbe2e1c564d1a99d5c39265edc6aeca2
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja060420260754
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-FR-ModelSessionId-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- model_session_id field is defined in spec 11.5.1 FlightRecorderEventBase but is missing from the FlightRecorderEvent struct in flight_recorder/mod.rs.
- DuckDB schema for FR events does not include a model_session_id column.
- 9 session emitters in workflows.rs construct FR events without populating model_session_id.
- Without model_session_id, FR events cannot be correlated to their originating ModelSession, making session-scoped FR queries impossible.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 1h
- SEARCH_SCOPE: Master Spec v02.179 sections 11.5.1, 4.3.9.12, 4.3.9.18.4; product code in handshake_core flight_recorder and workflows modules
- REFERENCES: spec sections 11.5.1, 4.3.9.12, 4.3.9.18.4; flight_recorder/mod.rs, flight_recorder/duckdb.rs, workflows.rs
- PATTERNS_EXTRACTED: envelope-level correlation IDs in structured event systems (OpenTelemetry trace_id/span_id pattern)
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT envelope-level session correlation from OpenTelemetry trace context pattern; no adaption or rejection needed as this is a direct field addition to an existing struct
- LICENSE/IP_NOTES: internal code and spec review only; no third-party code or copyrighted text is intended for direct reuse
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The model_session_id field is already defined in spec 11.5.1 FlightRecorderEventBase. This WP implements an existing spec-defined field. No spec version bump required.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP is strictly internal mechanical work implementing an already spec-defined field (model_session_id) into existing structs, DuckDB schema, and emitters. No external signal scan is needed.
- SOURCE_LOG:
  - NONE
- SOURCE_MAX_AGE_DAYS: N/A
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_SYNTHESIS:
  - NONE
- RESEARCH_GAPS_TO_TRACK:
  - NONE

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT: NONE
- ADAPT: NONE
- REJECT: NONE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_REASON: RESEARCH_CURRENCY_REQUIRED=NO; this is strictly internal mechanical work against existing spec.

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_REASON: RESEARCH_CURRENCY_REQUIRED=NO; no external project scouting needed for internal field addition.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new Flight Recorder event IDs are introduced. This WP adds the model_session_id field to the existing FlightRecorderEvent envelope so that all existing and future FR events carry session correlation context.
- Affected surface: FlightRecorderEventBase struct gains model_session_id: Option<String> field; DuckDB gains model_session_id TEXT column; 9 existing session emitters are updated to populate the field from the current ModelSession context.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: model_session_id is populated from runtime context; if a caller fabricates a session ID, FR events could be mis-attributed. Mitigation: model_session_id is set by the runtime scheduler from SessionRegistry, not from user input.
- Risk: DuckDB migration adds a nullable column; old events will have NULL model_session_id. Mitigation: NULL is the correct representation for pre-migration events; queries must handle Option semantics.
- Risk: emitter update is incomplete and some code paths emit FR events without model_session_id. Mitigation: tripwire test validates that all session-scoped FR events carry a non-None model_session_id.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightRecorderEntry
  - PRIM-FlightEvent
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightRecorderEntry
  - PRIM-FlightEvent
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - No new primitives are created. Existing FR primitives are extended with the model_session_id field that is already defined in spec 11.5.1.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: No new primitives are created. Existing PRIM-FlightRecorderEventBase, PRIM-FlightRecorderEntry, and PRIM-FlightEvent are extended with a field already defined in spec.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No orphan primitives discovered. This WP implements an existing spec-defined field on existing primitives.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: model_session_id is part of the existing Flight Recorder feature family defined in spec 11.5.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface in this backend WP. Session event timeline filter and DCC event log grouping are downstream of existing DCC stubs.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The FR envelope x ModelSession interaction is already tracked. No new interaction matrix edges are introduced.
- APPENDIX_MAINTENANCE_NOTES:
  - No appendix changes required. This WP implements an existing spec-defined field.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability involved in FR field addition | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication surface involved | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physics surface involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: no simulation surface involved | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware surface involved | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: no orchestration surface modified | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface involved | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: no publication surface involved | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking surface involved | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food safety surface involved | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no logistics surface involved | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: no archival surface involved | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: no retrieval surface involved | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: no analysis surface involved | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset ingestion involved | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: DuckDB migration is a schema change but does not modify DBA engine behavior | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: no governance mechanism modified; model_session_id is a correlation field not a governance gate | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring surface involved | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: model_session_id is correlation context for downstream consumers; enables session-scoped FR queries | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: no versioning surface modified | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox surface involved | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: primary pillar; adding model_session_id to FR event envelope | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: no calendar surface involved | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code editor surface involved | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface involved | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface involved | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: no work tracking modification | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: no artifact surface involved | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no WP contract modification | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task board modification | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: no MT contract modification | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: DCC event log grouping by session is downstream and already covered by existing DCC stubs | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS modification | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: 9 session emitters in workflows.rs updated to populate model_session_id | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no spec-to-prompt surface involved | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: DuckDB-only for FR; no trait boundary change | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: model_session_id enables session-scoped FR queries for local model consumption | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no stage surface involved | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio surface involved | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design surface involved | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no distillation surface involved | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE surface involved | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: no RAG surface involved | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: model_session_id envelope field | SUBFEATURES: struct field addition, DuckDB column migration, builder method update | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase, PRIM-FlightRecorderEntry, PRIM-FlightEvent | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: adds session correlation to every FR event enabling session-scoped queries
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: session emitter model_session_id propagation | SUBFEATURES: 9 emitter call sites updated to pass model_session_id from current ModelSession context | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: ensures all session-scoped FR events carry correlation context
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: session-scoped FR query support | SUBFEATURES: DuckDB WHERE model_session_id = ? queries for session event timelines | PRIMITIVES_FEATURES: PRIM-FlightRecorderEntry | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: enables local models to retrieve all FR events for a specific session
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: model_session_id in FR event envelope | JobModel: WORKFLOW | Workflow: flight_recorder_emit | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: all existing FR events | Locus: NONE | StoragePosture: DUCKDB | Resolution: IN_THIS_WP | Stub: NONE | Notes: every FR event now carries model_session_id for session-scoped correlation
  - Capability: session-scoped FR query | JobModel: QUERY | Workflow: flight_recorder_query_by_session | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: N/A (querying FR, not emitting) | Locus: NONE | StoragePosture: DUCKDB | Resolution: IN_THIS_WP | Stub: NONE | Notes: DuckDB queries can now filter FR events by model_session_id
  - Capability: model_session_id envelope field for FR event correlation | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: VISIBLE | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: adds session correlation to all session-scoped FR events
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 15m
- MATRIX_SCAN_NOTES:
  - This packet adds a correlation field to an existing envelope. The FR envelope x ModelSession interaction is already tracked in the interaction matrix. No new cross-primitive edges are introduced.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: FR envelope x ModelSession is an existing interaction. model_session_id is the implementation of that interaction, not a new edge.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This WP implements an existing spec-defined field. No external research is needed for adding a correlation ID to a struct and schema.
- SOURCE_SCAN:
  - NONE
- MATRIX_GROWTH_CANDIDATES:
  - NONE
- ENGINEERING_TRICKS_CARRIED_OVER:
  - NONE
- MATRIX_RESEARCH_VERDICT: NOT_APPLICABLE

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: FlightRecorderEventBase model_session_id + DuckDB session-scoped queries | Pillars: Flight Recorder, LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-FlightRecorderEventBase, PRIM-FlightRecorderEntry | Resolution: IN_THIS_WP | Stub: NONE | Notes: core deliverable; session correlation enables downstream FR queries
  - Combo: model_session_id propagation + 9 session emitters | Pillars: Flight Recorder, Execution / Job Runtime | Mechanical: engine.context | Primitives/Features: PRIM-FlightRecorderEventBase, PRIM-FlightEvent | Resolution: IN_THIS_WP | Stub: NONE | Notes: all session-scoped emitters updated to populate the field
  - Combo: model_session_id DuckDB column + LLM-friendly session timeline | Pillars: Flight Recorder, LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-FlightRecorderEntry | Resolution: IN_THIS_WP | Stub: NONE | Notes: DuckDB column enables local models to query complete session event timelines via simple WHERE clause
  - Combo: session emitter propagation + LLM-friendly session context | Pillars: Execution / Job Runtime, LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-FlightRecorderEventBase, PRIM-FlightRecorderEntry | Resolution: IN_THIS_WP | Stub: NONE | Notes: emitters supply model_session_id at write time so downstream LLM queries get pre-correlated session data without post-hoc joins
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations resolve through IN_THIS_WP; no stubs needed and no silent drops.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed FR packets, current Master Spec v02.179, and local product code under src/backend/handshake_core
- MATCHED_STUBS:
  - Artifact: WP-1-Session-Observability-Spans-FR-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: observability spans are a complementary telemetry layer built on top of FR events; this WP provides the model_session_id field that spans will consume for session-scoped grouping
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Flight-Recorder | BoardStatus: SUPERSEDED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established FlightRecorderEvent struct and DuckDB storage; model_session_id was spec-defined but not implemented in the initial FR packet; superseded by WP-1-Flight-Recorder-v4
  - Artifact: WP-1-ModelSession-Core-Scheduler-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established ModelSession with session_id; this WP bridges FR events to ModelSession via model_session_id
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs | Artifact: WP-1-Flight-Recorder | Covers: primitive | Verdict: PARTIAL | Notes: FlightRecorderEvent struct exists but model_session_id field is missing
  - Path: ../handshake_main/src/backend/handshake_core/src/flight_recorder/duckdb.rs | Artifact: WP-1-Flight-Recorder | Covers: primitive | Verdict: PARTIAL | Notes: DuckDB schema for FR events exists but model_session_id column is missing
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: execution | Verdict: PARTIAL | Notes: 9 session emitters construct FR events but do not populate model_session_id
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: All KEEP_SEPARATE; no duplication. This WP fills the model_session_id gap between the existing FR event envelope and ModelSession primitives.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet implements a backend struct field, DuckDB migration, and emitter updates. GUI surfaces consuming model_session_id (session event timeline filter in FR viewer, session span grouping in DCC event log) are downstream of existing DCC stubs.
- UI_SURFACES:
  - NONE (downstream of existing DCC stubs)
- UI_CONTROLS (buttons/dropdowns/inputs):
  - NONE (downstream)
- UI_STATES (empty/loading/error):
  - NONE (downstream)
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE (downstream)
- UI_ACCESSIBILITY_NOTES:
  - NONE (downstream)
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet. Session event timeline and DCC event log are downstream.
- GUI_REFERENCE_SCAN:
  - NONE
- HANDSHAKE_GUI_ADVICE:
  - NONE
- HIDDEN_GUI_REQUIREMENTS:
  - NONE
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - NONE
- GUI_IMPLEMENTATION_ADVICE_VERDICT: NOT_APPLICABLE

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: NO
- If YES: update the Roadmap (Spec 7.6) using the fixed per-phase fields below (do not invent new per-phase block types).
- Patch canonical roadmap sections in place. Do not create addendum-style normative text; use `[ADD v<version>]` markers for new lines/blocks.
- Per phase, include exactly:
  - Goal:
  - MUST deliver:
  - Key risks addressed in Phase n:
  - Acceptance criteria:
  - Explicitly OUT of scope:
  - Mechanical Track:
  - Atelier Track:
  - Distillation Track:
  - Vertical slice:

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Flight-Recorder, WP-1-ModelSession-Core-Scheduler
- BUILD_ORDER_BLOCKS: WP-1-Session-Observability-Spans-FR, WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md 11.5.1 FlightRecorderEventBase (model_session_id field)
- WHAT: Add model_session_id field to FlightRecorderEvent struct, DuckDB schema migration, and update 9 session emitters to populate the field from ModelSession context.
- WHY: Without model_session_id, FR events cannot be correlated to their originating ModelSession. This blocks session-scoped FR queries, observability spans, and DCC session event timeline display.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
- OUT_OF_SCOPE:
  - session observability spans (WP-1-Session-Observability-Spans-FR)
  - DCC session event timeline display (WP-1-Dev-Command-Center-Control-Plane-Backend)
  - new FR event IDs (no new events added; existing events gain the field)
- TEST_PLAN:
  ```bash
  cargo test fr_model_session_id
  cargo test flight_recorder_round_trip
  cargo test query_by_session
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - FlightRecorderEvent struct has model_session_id: Option<String> field.
  - DuckDB schema includes model_session_id TEXT column via migration.
  - All 9 session emitters in workflows.rs populate model_session_id from the current ModelSession context.
  - Round-trip test: emit FR event with model_session_id, read back, verify field is preserved.
  - Query-by-session test: emit multiple FR events across two sessions, query by model_session_id, verify correct filtering.
  - Tripwire test: verify all session-scoped FR events carry non-None model_session_id.
- PRIMITIVES_EXPOSED:
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightRecorderEntry
  - PRIM-FlightEvent
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - FlightRecorderEvent
  - FlightRecorderEventBase
  - model_session_id
  - session_id
  - duckdb
  - emit_event
  - flight_recorder
- RUN_COMMANDS:
  ```bash
  rg -n "FlightRecorderEvent|model_session_id|emit_event" src/backend/handshake_core/src
  cargo test fr_model_session_id
  cargo test flight_recorder_round_trip
  cargo test query_by_session
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Missing model_session_id in emitter" -> "FR event is emitted without session correlation; downstream queries return incomplete results"
  - "DuckDB migration failure" -> "model_session_id column missing; all new FR events fail to persist the field"
  - "Incorrect session context propagation" -> "model_session_id is populated from wrong session; FR events are mis-attributed"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just orchestrator-prepare-and-packet` will sync Task Board and Build Order truth.
  - After packet creation, verify the Build Order shows this WP blocking WP-1-Session-Observability-Spans-FR and WP-1-Dev-Command-Center-Control-Plane-Backend.
  - Confirm dependency on WP-1-Flight-Recorder and WP-1-ModelSession-Core-Scheduler is reflected in the Build Order graph.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: 11.5.1 FlightRecorderEventBase model_session_id field | WHY_IN_SCOPE: spec defines model_session_id as a field of FlightRecorderEventBase but it is missing from the Rust struct | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/flight_recorder/mod.rs | EXPECTED_TESTS: cargo test fr_model_session_id; cargo test flight_recorder_round_trip | RISK_IF_MISSED: FR events cannot be correlated to sessions; session-scoped queries are impossible
  - CLAUSE: 4.3.9.18.4 FR correlation rule (HARD) model_session_id | WHY_IN_SCOPE: spec mandates model_session_id as a HARD correlation field for FR events; without it the correlation rule is violated | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test fr_model_session_id; cargo test query_by_session | RISK_IF_MISSED: FR correlation rule violation; session observability is broken
  - CLAUSE: DuckDB schema for model_session_id | WHY_IN_SCOPE: FR events must persist model_session_id to enable session-scoped queries | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/flight_recorder/duckdb.rs | EXPECTED_TESTS: cargo test flight_recorder_round_trip; cargo test query_by_session | RISK_IF_MISSED: model_session_id is in struct but not persisted; queries return NULL for all events

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: FlightRecorderEvent struct model_session_id field | PRODUCER: 9 session emitters in workflows.rs | CONSUMER: DuckDB storage, FR query API, DCC event log (downstream) | SERIALIZER_TRANSPORT: in-process struct with serde JSON serialization for DuckDB storage | VALIDATOR_READER: fr_model_session_id and flight_recorder_round_trip tests | TRIPWIRE_TESTS: cargo test fr_model_session_id; cargo test flight_recorder_round_trip | DRIFT_RISK: emitter forgets to populate field; consumer queries return NULL
  - CONTRACT: DuckDB model_session_id column | PRODUCER: DuckDB migration in duckdb.rs | CONSUMER: FR query functions, DCC session event timeline (downstream) | SERIALIZER_TRANSPORT: DuckDB SQL INSERT/SELECT | VALIDATOR_READER: query_by_session tests | TRIPWIRE_TESTS: cargo test query_by_session | DRIFT_RISK: migration not applied; column missing in schema
  - CONTRACT: FR event builder model_session_id method | PRODUCER: FlightRecorderEvent builder in mod.rs | CONSUMER: 9 session emitters in workflows.rs | SERIALIZER_TRANSPORT: in-process builder pattern | VALIDATOR_READER: fr_model_session_id tests | TRIPWIRE_TESTS: cargo test fr_model_session_id | DRIFT_RISK: builder does not expose method; emitters cannot set field

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test fr_model_session_id
  - cargo test flight_recorder_round_trip
  - cargo test query_by_session
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - an FR event emitted during a ModelSession with session_id "sess-abc-123" carries model_session_id = Some("sess-abc-123") after round-trip through DuckDB
  - querying DuckDB with WHERE model_session_id = 'sess-abc-123' returns exactly the events emitted during that session and excludes events from other sessions
  - an FR event emitted outside any ModelSession context carries model_session_id = None
  - all 9 session emitters in workflows.rs produce FR events with non-None model_session_id when called within a session context

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - MT-001: Add model_session_id: Option<String> field to FlightRecorderEvent struct in flight_recorder/mod.rs. Update the builder to accept model_session_id. Add DuckDB migration in duckdb.rs to add model_session_id TEXT column to the FR events table.
  - MT-002: Update all 9 session emitters in workflows.rs to pass model_session_id from the current ModelSession context when constructing FR events.
  - MT-003: Add tests: round-trip test (emit with model_session_id, read back, verify preserved), query-by-session test (emit across two sessions, query by model_session_id, verify filtering), tripwire test (verify all session-scoped FR events carry non-None model_session_id).
- HOT_FILES:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
- TRIPWIRE_TESTS:
  - cargo test fr_model_session_id
  - cargo test flight_recorder_round_trip
  - cargo test query_by_session
- CARRY_FORWARD_WARNINGS:
  - Do not add model_session_id as a non-optional field; existing events have no session context and must remain valid with None.
  - Do not skip DuckDB migration; the column must exist before new events are inserted.
  - Do not populate model_session_id from user input; always derive from SessionRegistry/ModelSession runtime context.
  - Verify all 9 emitters are updated; a partial update silently breaks session-scoped queries.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - FlightRecorderEvent struct has model_session_id: Option<String> field
  - DuckDB schema includes model_session_id TEXT column
  - All 9 session emitters populate model_session_id from ModelSession context
  - Round-trip test confirms field persistence through DuckDB
  - Query-by-session test confirms correct filtering
  - Tripwire test confirms all session-scoped events carry non-None model_session_id
- FILES_TO_READ:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
- COMMANDS_TO_RUN:
  - rg -n "model_session_id" src/backend/handshake_core/src
  - cargo test fr_model_session_id
  - cargo test flight_recorder_round_trip
  - cargo test query_by_session
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- POST_MERGE_SPOTCHECKS:
  - verify model_session_id is Option<String> not String (old events must remain valid)
  - verify DuckDB migration adds the column as nullable TEXT
  - verify all 9 session emitters populate the field (count emitter call sites)
  - verify no emitter uses user-supplied values for model_session_id

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The exact list of 9 session emitters is identified by code inspection but may change if new emitters are added before this WP is implemented.
  - Whether a DuckDB index on model_session_id is needed for query performance is not determined at refinement time; depends on FR event volume.
  - The interaction between model_session_id and the existing event_id/trace_id fields in FR events is not fully characterized; they are orthogonal but downstream consumers may need to correlate across all three.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec section 11.5.1 explicitly defines model_session_id as a field of FlightRecorderEventBase. Section 4.3.9.18.4 defines the FR correlation rule as HARD. All acceptance criteria (struct field, DuckDB column, 9 emitter updates, round-trip test, query test, tripwire test) map directly to normative spec anchors.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The model_session_id field is already defined in spec 11.5.1 FlightRecorderEventBase. This WP implements an existing spec-defined field. No new primitives, no interaction matrix changes, no appendix updates. No spec version bump required.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 11.5.1 FlightRecorderEventBase (model_session_id field)
- CONTEXT_START_LINE: 66862
- CONTEXT_END_LINE: 66870
- CONTEXT_TOKEN: model_session_id
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 11.5.1 FlightRecorderEventBase

  FlightRecorderEventBase defines the envelope fields for all Flight Recorder
  events. Fields include event_id, timestamp, event_type, and model_session_id.
  The model_session_id field correlates each FR event to its originating
  ModelSession, enabling session-scoped queries and audit trails.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.12 ModelSession (session_id definition)
- CONTEXT_START_LINE: 32175
- CONTEXT_END_LINE: 32185
- CONTEXT_TOKEN: session_id
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.12 ModelSession

  ModelSession tracks the lifecycle of a single LLM interaction session.
  Fields include session_id, parent_session_id (nullable for root sessions),
  role, capabilities, state, and timestamps. The SessionRegistry maintains
  children_by_parent for parent-child relationship tracking.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.18.4 FR correlation rule (HARD)
- CONTEXT_START_LINE: 32683
- CONTEXT_END_LINE: 32695
- CONTEXT_TOKEN: model_session_id
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 4.3.9.18.4 Flight Recorder Correlation Rules

  HARD: Every FR event emitted within a ModelSession context MUST carry
  model_session_id set to the session\\u2019s session_id. This enables
  session-scoped FR queries and is a prerequisite for observability spans
  and DCC session event timeline display.
  ```

### DISCOVERY (RGF-94 discovery fields; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (extends existing PRIM-FlightRecorderEventBase)
- DISCOVERY_STUBS: NONE_CREATED (downstream DCC visualization is already covered by existing DCC stubs)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (FR envelope x ModelSession is an existing interaction)
- DISCOVERY_UI_CONTROLS: session event timeline filter in FR viewer, session span grouping in DCC event log
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED
- DISCOVERY_JUSTIFICATION: This WP implements an existing spec-defined field. The interaction between FR events and ModelSession is already tracked. No new primitive family or cross-feature edge is introduced.

### MICROTASKS (implementation breakdown for PACKET_HYDRATION)
- MT-001: Struct field + builder + DuckDB migration
  - Add model_session_id: Option<String> to FlightRecorderEvent struct
  - Update builder with .model_session_id() method
  - Add DuckDB migration to add model_session_id TEXT column
  - Files: flight_recorder/mod.rs, flight_recorder/duckdb.rs
- MT-002: Update 9 session emitters
  - Update all 9 session emitters in workflows.rs to populate model_session_id from current ModelSession context
  - Files: workflows.rs
- MT-003: Tests (round-trip, query-by-session, tripwire)
  - Round-trip: emit FR event with model_session_id, read back, verify field preserved
  - Query-by-session: emit across two sessions, query by model_session_id, verify filtering
  - Tripwire: verify all session-scoped FR events carry non-None model_session_id
  - Files: flight_recorder/mod.rs or tests module
