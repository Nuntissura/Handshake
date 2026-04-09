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
- WP_ID: WP-1-Session-Observability-Spans-FR-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-09T00:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- SPEC_TARGET_SHA1: 7d6558fab2f3df70669fff6f0a6e6ef9ea395194
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja090420260043
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Session-Observability-Spans-FR-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- The current backend already persists `model_session_id`, `activity_span_id`, and `session_span_id` in the Flight Recorder envelope and DuckDB store, but no runtime path actually binds live ModelSessions and model runs to those span ids.
- `FR-EVT-SESS-SCHED-*` and `FR-EVT-SESS-SPAWN-*` are implemented, but the spec-mandated lifecycle family `FR-EVT-SESS-001..005` (`session.created`, `session.state_change`, `session.completed`, `session.message`, `session.budget_warning`) is not registered or emitted in product code.
- `workflows.rs` emits scheduler and spawn telemetry with `model_session_id`, but no workflow path currently calls `with_activity_span(...)` or `with_session_span(...)`.
- No typed runtime substrate exists yet for `ModelSessionSpanBinding` / `ActivitySpanBinding`, so session timelines and nested tool-call spans cannot be reconstructed from live runtime truth.
- Session-wide queries can filter by `model_session_id`, but the deeper span-aware observability contract required by spec 4.3.9.18 / 11.9.1.X remains unimplemented.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 90m
- SEARCH_SCOPE: Master Spec v02.180 sections 4.3.9.13, 4.3.9.15, 4.3.9.18, 11.5.1, 11.9.1.X; current stub backlog; adjacent refinements/packets (`WP-1-FR-ModelSessionId-v1`, `WP-1-Session-Spawn-Contract-v1`); product code in `src/backend/handshake_core/src/flight_recorder`, `src/backend/handshake_core/src/workflows.rs`, and `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`
- REFERENCES: .GOV/spec/Handshake_Master_Spec_v02.180.md; .GOV/task_packets/stubs/WP-1-Session-Observability-Spans-FR-v1.md; .GOV/refinements/WP-1-FR-ModelSessionId-v1.md; .GOV/refinements/WP-1-Session-Spawn-Contract-v1.md; ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs; ../handshake_main/src/backend/handshake_core/src/flight_recorder/duckdb.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- PATTERNS_EXTRACTED: extend the existing Flight Recorder event envelope and recorder/query path rather than creating a parallel observability subsystem; keep `model_session_id` as the primary correlation key and treat span ids as supplemental nested structure; emit lifecycle and span telemetry from the existing workflow runtime rather than a sidecar pipeline
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the existing Flight Recorder store/API as the single observability substrate; ADAPT the already-present `with_activity_span(...)` and `with_session_span(...)` builder hooks into real workflow emission paths; REJECT any new standalone observability module, duplicate telemetry store, or new narrow governance leaf checks for this packet
- LICENSE/IP_NOTES: internal spec, packet, and code review only; no third-party code or copyrighted text is intended for direct reuse
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The spec already defines the lifecycle event family, span binding rules, correlation rule, and UI/query expectations. This WP activates those existing norms in code and reuses the existing stub backlog entry rather than requesting a spec bump.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP is strictly internal mechanical work against already-normative Master Spec clauses and current repo code. The implementation problem is wiring existing session/runtime/Flight Recorder surfaces together, not selecting a new external pattern.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE
- RESEARCH_GAPS_TO_TRACK:
  - NONE
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - NONE
- ADAPT_PATTERNS:
  - NONE
- REJECT_PATTERNS:
  - NONE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_REASON: RESEARCH_CURRENCY_REQUIRED=NO; this is internal mechanical work against existing spec and current code foundations.

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_REASON: RESEARCH_CURRENCY_REQUIRED=NO; external project scouting would not change the required implementation shape for an already-specified internal observability contract.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- This WP does not create a new observability subsystem. It extends the existing Flight Recorder substrate so the current recorder, DuckDB store, and query API become the authoritative backend truth for session lifecycle and nested span telemetry.
- In scope event families:
  - Existing foundations to preserve: `FR-EVT-SESS-SCHED-*`, `FR-EVT-SESS-SPAWN-*`, `model_session_id` correlation, and DuckDB/API support for `model_session_id`, `activity_span_id`, and `session_span_id`.
  - Missing lifecycle family to implement: `FR-EVT-SESS-001..005` (`session.created`, `session.state_change`, `session.completed`, `session.message`, `session.budget_warning`).
- Span propagation rules to implement:
  - one `ModelSessionSpan` per `ModelSession` lifecycle
  - one `ActivitySpan` per `model_run`
  - child `ActivitySpan`s for nested tool calls under the model run span
  - emitted FR events carry `model_session_id` as the primary correlation field and `session_span_id` / `activity_span_id` as supplemental nested structure
- Queryability expectation:
  - downstream consumers should be able to reconstruct a session timeline from existing Flight Recorder rows without introducing a second telemetry store

### RED_TEAM_ADVISORY (security failure modes)
- Risk: session lifecycle events are added but span ids are generated ad hoc per event instead of being bound once and reused. Mitigation: create a typed binding substrate (or equivalent runtime record) and reuse bound ids across all lifecycle/model/tool emissions.
- Risk: nested tool calls omit `activity_span_id` / `session_span_id` on one or more execution paths, making the timeline look complete while silently dropping child work. Mitigation: centralize propagation in existing workflow/tool-call helpers and add targeted tripwire coverage.
- Risk: lifecycle events are emitted without `model_session_id`, regressing the hard correlation rule already established by `WP-1-FR-ModelSessionId-v1`. Mitigation: preserve `model_session_id` as the primary correlation key; tests must fail when session-scoped events omit it.
- Risk: a new sidecar telemetry store is introduced for spans, causing truth drift between emitted FR events and session observability state. Mitigation: keep Flight Recorder as the single backend truth; any typed binding records must feed the existing recorder/query surfaces.
- Risk: cost/budget rollups are computed from inconsistent runtime fields and disagree with session completion payloads. Mitigation: derive session aggregate fields from the same token/cost sources already used by the workflow runtime and validate them through the lifecycle event path.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-ModelSession
  - PRIM-ModelSessionSpanBinding
  - PRIM-ActivitySpanBinding
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightEvent
  - PRIM-SessionRegistry
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-ModelSession
  - PRIM-ModelSessionSpanBinding
  - PRIM-ActivitySpanBinding
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightEvent
  - PRIM-SessionRegistry
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - No new primitive family is introduced. This WP operationalizes existing spec primitives and bindings that are already present in Appendix 12.4 but not yet wired into product code.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: `PRIM-ModelSessionSpanBinding`, `PRIM-ActivitySpanBinding`, and the relevant Flight Recorder/session primitives already exist in the current spec appendix. This WP activates them in code rather than minting new appendix rows.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No orphan primitive was discovered; the gap is implementation coverage, not appendix inventory.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Session observability spans already belong to the existing multi-session / Flight Recorder capability family tracked in the current spec.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet builds backend observability substrate only. DCC swim-lane rendering, filters, and deep-link UI remain downstream of `WP-1-Dev-Command-Center-Control-Plane-Backend-v1`.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The spec already encodes the session observability relationship between ModelSession, spans, Flight Recorder, and downstream UI/query surfaces. This WP fills the code gap instead of declaring a new appendix edge.
- APPENDIX_MAINTENANCE_NOTES:
  - No appendix update is required as long as the implementation stays within the already-defined lifecycle family, binding schemas, and query/UI requirements.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is modified by session observability wiring | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no simulation or measurement logic is involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers only | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing surface is modified | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: the orchestration model already exists; this WP adds observability substrate to it | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition capability is changed | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: no publication/export controller behavior is modified | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: span ids improve auditability, but archival engine behavior is not directly changed here | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: no retrieval engine behavior is modified directly | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: downstream analytics consume span data later, but this packet does not modify analyst engine behavior directly | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: DuckDB persistence is extended within the existing recorder surface, not by changing DBA engine semantics | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this packet adds observability data, not a new governance gate | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or explanation interface is implemented here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: session/span correlation is contextual runtime state that downstream operators, models, and UIs consume | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: no versioning or branch-management surface is modified | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no workspace/sandbox policy surface is altered by this packet | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: primary pillar; lifecycle events and span ids are emitted through the existing Flight Recorder substrate | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: no calendar surface is changed in this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor UI surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: no work-tracking contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: no artifact/media surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: the packet system is only the governance wrapper for this activation, not a product-surface target | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task-board product capability is changed | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: no MicroTask contract is modified | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: this packet provides the backend session timeline / cost / deep-link substrate that the DCC control-plane backend will consume next | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: session lifecycle, model_run, and nested tool-call runtime paths must bind and emit span-aware observability data | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no prompt/spec-router surface is modified | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: the existing recorder store/query posture remains SQLite-now/Postgres-ready; no boundary change is introduced | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: session-scoped span-aware telemetry becomes directly queryable structured data for operators and model-assisted debugging | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no stage workflow surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio runtime surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design/capture surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no training or distillation pipeline is modified | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE validator/runtime surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers only | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: session lifecycle event family | SUBFEATURES: `session.created`, `session.state_change`, `session.completed`, `session.message`, `session.budget_warning` payload validation and emission | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase, PRIM-FlightEvent, PRIM-ModelSession | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: fills the still-missing `FR-EVT-SESS-001..005` family on top of the existing scheduler/spawn foundation
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: ModelSessionSpan and ActivitySpan runtime binding | SUBFEATURES: open/close session span, bind model_run activity span, bind nested tool-call child spans, reuse ids across emitted events | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-ModelSessionSpanBinding, PRIM-ActivitySpanBinding, PRIM-SessionRegistry | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: binds the live workflow runtime to the span contract already declared in spec
  - PILLAR: Command Center | CAPABILITY_SLICE: backend timeline/cost projection inputs | SUBFEATURES: session timeline reconstruction inputs, per-session aggregate cost payloads, deep-linkable span ids | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase, PRIM-ModelSessionSpanBinding | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: backend projection surface only; DCC rendering remains downstream
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: session-scoped observability query substrate | SUBFEATURES: model_session_id plus span ids in recorder rows, filterable session timelines, stable nested identifiers for debugging and replay-adjacent analysis | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase, PRIM-FlightEvent, PRIM-ActivitySpanBinding | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: makes session telemetry structurally useful to both operators and local/cloud model workflows
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: session lifecycle Flight Recorder events | JobModel: WORKFLOW | Workflow: model_session_lifecycle | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-SESS-001..005 | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: lifecycle creation/state/complete/message/budget events become first-class recorder rows for every ModelSession
  - Capability: ModelSessionSpan plus model_run/tool ActivitySpan binding | JobModel: AI_JOB | Workflow: model_run execution plus nested tool calls | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: model_session_id plus session_span_id/activity_span_id on session/model/tool rows | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: existing builder/storage fields are finally driven by live runtime binding
  - Capability: session-scoped timeline/cost query substrate | JobModel: NONE | Workflow: flight_recorder session timeline query | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: query by model_session_id with nested span structure available for reconstruction | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend query substrate only; DCC rendering is explicitly downstream
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - This packet fills a code gap in already-declared session observability interactions rather than adding a new primitive family or new interaction matrix edge.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Session observability already exists as a spec-defined interaction between ModelSession, Flight Recorder, span bindings, and downstream DCC consumers. This packet implements that relationship instead of creating a new matrix edge.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This packet is activating an already-specified internal observability contract on top of adjacent validated foundations. The remaining work is local repo integration, not external pattern selection.
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
  - Combo: ModelSession lifecycle events + ModelSessionSpan binding + session cost aggregation | Pillars: Flight Recorder, Execution / Job Runtime, Command Center, LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-ModelSession, PRIM-ModelSessionSpanBinding, PRIM-FlightRecorderEventBase, PRIM-FlightEvent | Resolution: IN_THIS_WP | Stub: NONE | Notes: the lifecycle family only becomes useful when it is emitted with stable session span ids and completion aggregates
  - Combo: model_run event emission + nested tool-call activity spans + existing `model_session_id` correlation | Pillars: Flight Recorder, Execution / Job Runtime, LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-ActivitySpanBinding, PRIM-FlightRecorderEventBase, PRIM-SessionRegistry | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the core runtime binding gap left after the scheduler/spawn/model_session_id packets
  - Combo: session timeline query substrate + DCC control-plane backend consumer | Pillars: Flight Recorder, Command Center, LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-FlightRecorderEventBase, PRIM-ModelSessionSpanBinding | Resolution: IN_THIS_WP | Stub: NONE | Notes: this packet provides the backend substrate; DCC rendering stays downstream without needing a new stub here
  - Combo: lifecycle budget warnings + session completion totals + existing runtime token/cost accounting | Pillars: Flight Recorder, Execution / Job Runtime, LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-FlightEvent, PRIM-ModelSession, PRIM-FlightRecorderEventBase | Resolution: IN_THIS_WP | Stub: NONE | Notes: budget-warning and completion rows must agree with the same runtime accounting source to stay trustworthy
  - Combo: spawn/announce-back session links + session span correlation ids + existing scheduler/spawn event families | Pillars: Flight Recorder, Execution / Job Runtime, Command Center | Mechanical: engine.context | Primitives/Features: PRIM-ModelSessionSpanBinding, PRIM-SessionRegistry, PRIM-FlightRecorderEventBase | Resolution: IN_THIS_WP | Stub: NONE | Notes: adjacent validated spawn foundations should remain span-correlatable so downstream cross-session timeline reconstruction does not fork into a second contract
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations collapse into the current packet scope and can be implemented by extending the existing recorder/runtime surfaces instead of creating new leaf packets or new narrow governance checks.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, BUILD_ORDER, TASK_BOARD, validated adjacent packets (`WP-1-FR-ModelSessionId-v1`, `WP-1-ModelSession-Core-Scheduler-v1`, `WP-1-Session-Spawn-Contract-v1`), current Master Spec v02.180 anchors, and repo code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Session-Observability-Spans-FR-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: exact backlog artifact already exists and should be activated rather than replaced
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-FR-ModelSessionId-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: landed the hard `model_session_id` correlation rule and storage/query support that this packet must preserve
  - Artifact: WP-1-ModelSession-Core-Scheduler-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: landed the scheduler/runtime foundation that emits session-scoped work but not lifecycle/span binding
  - Artifact: WP-1-Session-Spawn-Contract-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: landed spawn telemetry and cross-session execution semantics that this packet extends with span-aware observability
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs | Artifact: WP-1-FR-ModelSessionId-v1 | Covers: primitive | Verdict: PARTIAL | Notes: event envelope already carries `model_session_id`, `activity_span_id`, and `session_span_id`, and builder hooks exist, but lifecycle event registration is missing
  - Path: ../handshake_main/src/backend/handshake_core/src/flight_recorder/duckdb.rs | Artifact: WP-1-FR-ModelSessionId-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: DuckDB schema, indexes, API filters, and tripwires already support session and span fields
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: NONE | Covers: combo | Verdict: PARTIAL | Notes: workflow runtime emits scheduler/spawn rows with `model_session_id`, but does not yet stamp lifecycle rows or bind activity/session spans
  - Path: ../handshake_main/src/backend/handshake_core/tests/model_session_scheduler_tests.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: execution | Verdict: PARTIAL | Notes: adjacent session scheduler coverage exists, but no proof yet for lifecycle family emission or span propagation
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The exact stub already exists and the missing work is a code-level expansion on top of validated adjacent foundations; no new stub or spec update is justified.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet only delivers backend observability substrate. Any operator-facing session swim-lanes, filters, overlays, or deep-links belong to downstream DCC packets.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - NONE
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - Tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No direct GUI surface is implemented in this packet. The only relevant UI semantics are already captured by the spec anchors and remain downstream implementation guidance for DCC.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.180]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-FR-ModelSessionId, WP-1-ModelSession-Core-Scheduler, WP-1-Session-Spawn-Contract
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.180.md 4.3.9.18 Session Observability: ActivitySpan and ModelSessionSpan Binding (Normative) [ADD v02.137]
- WHAT: Implement the missing session lifecycle Flight Recorder family and wire live `ModelSessionSpan` / `ActivitySpan` binding through the existing workflow runtime. Reuse the current recorder, DuckDB store, and query API so session timelines can be reconstructed from canonical FR rows.
- WHY: The refactor foundations already landed `model_session_id`, scheduler events, and spawn events, but the spec-required lifecycle/span observability contract is still missing in product code. DCC backend work depends on this substrate being real rather than implied.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- OUT_OF_SCOPE:
  - DCC rendering, swim-lane UI, and operator-facing controls
  - Stage or Calendar span projections
  - replay/audit features beyond the existing Flight Recorder truth surface
  - any new observability store, sidecar pipeline, or appendix/spec expansion
- TEST_PLAN:
  ```bash
  rg -n "session.created|session.state_change|session.completed|session.message|session.budget_warning|with_activity_span|with_session_span|model_session_id" src/backend/handshake_core/src/flight_recorder/mod.rs src/backend/handshake_core/src/flight_recorder/duckdb.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/model_session_scheduler_tests.rs
  cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test session_scheduler_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test model_run_spawn_announce_back_event_is_emitted_for_parented_completion --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - `FR-EVT-SESS-001..005` is registered, shape-validated, and emitted through the existing Flight Recorder surface
  - session lifecycle rows carry `model_session_id` and stable `session_span_id`
  - model runs and nested tool calls stamp coherent `activity_span_id` values under the correct session span
  - the existing DuckDB/query/API substrate remains the single backend truth for session timeline reconstruction
  - tests cover lifecycle emission and span propagation without regressing the adjacent scheduler/spawn/model_session_id foundations
- PRIMITIVES_EXPOSED:
  - PRIM-ModelSession
  - PRIM-ModelSessionSpanBinding
  - PRIM-ActivitySpanBinding
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightEvent
  - PRIM-SessionRegistry
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.180.md
  - .GOV/task_packets/stubs/WP-1-Session-Observability-Spans-FR-v1.md
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- SEARCH_TERMS:
  - session.created
  - session.state_change
  - session.completed
  - session.message
  - session.budget_warning
  - with_activity_span
  - with_session_span
  - model_session_id
- RUN_COMMANDS:
  ```bash
  just phase-check STARTUP WP-1-Session-Observability-Spans-FR-v1 CODER
  just launch-coder WP-1-Session-Observability-Spans-FR-v1
  just session-send CODER WP-1-Session-Observability-Spans-FR-v1 "Implement the packet exactly as written; extend existing Flight Recorder/runtime surfaces and avoid introducing a second observability store."
  just phase-check HANDOFF WP-1-Session-Observability-Spans-FR-v1 CODER
  just phase-check HANDOFF WP-1-Session-Observability-Spans-FR-v1 WP_VALIDATOR
  ```
- RISK_MAP:
  - "Lifecycle rows emit without stable session span ids" -> "session timeline becomes fragmented and downstream DCC consumers cannot reconstruct the session correctly"
  - "Nested tool-call paths skip activity span propagation" -> "child work disappears from observability and cost/debug traces become misleading"
  - "Span binding lands outside the existing recorder/query path" -> "truth drifts between runtime state and persisted Flight Recorder evidence"
  - "Adding spans regresses the `model_session_id` hard rule" -> "existing session-scoped queries and validation tripwires fail"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - BUILD_ORDER was already synced during startup repair. Recheck before approval only if the packet hydration fields, dependency list, or blocked lane change again.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: 4.3.9.18.2 Span Binding (Normative) | WHY_IN_SCOPE: this packet exists to bind live `ModelSession` / `model_run` / tool execution to session and activity spans | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/flight_recorder/mod.rs` | EXPECTED_TESTS: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml` | RISK_IF_MISSED: session and activity spans remain decorative fields with no runtime truth
  - CLAUSE: 4.3.9.18.4 Flight Recorder Events (Session Lifecycle) | WHY_IN_SCOPE: the spec-mandated lifecycle family is still absent in product code | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/workflows.rs` | EXPECTED_TESTS: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` | RISK_IF_MISSED: DCC/backend consumers get no canonical session creation, transition, completion, message, or budget-warning rows
  - CLAUSE: 11.5 schema registry must include subsystem event families | WHY_IN_SCOPE: any newly added lifecycle rows must remain part of the canonical schema validation surface | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/flight_recorder/duckdb.rs` | EXPECTED_TESTS: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` | RISK_IF_MISSED: unknown or malformed lifecycle events can drift silently into persisted history
  - CLAUSE: 11.9.1.X Session-Scoped Observability Requirements | WHY_IN_SCOPE: this packet is the backend substrate that makes timeline, cost, filter, and deep-link reconstruction possible | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/tests/model_session_scheduler_tests.rs` | EXPECTED_TESTS: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml` | RISK_IF_MISSED: session-scoped observability remains incomplete even if individual rows exist

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: `FR-EVT-SESS-001..005` event family registration and payload schemas | PRODUCER: workflow runtime plus Flight Recorder event constructors | CONSUMER: recorder schema validator, DuckDB persistence, API/query readers, downstream DCC backend | SERIALIZER_TRANSPORT: canonical `FlightRecorderEvent` envelope persisted through DuckDB | VALIDATOR_READER: schema validation and session lifecycle tests | TRIPWIRE_TESTS: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` | DRIFT_RISK: lifecycle rows exist in one layer but not another, causing silent replay/query breakage
  - CONTRACT: ModelSessionSpanBinding / ActivitySpanBinding runtime semantics | PRODUCER: workflow/session execution helpers | CONSUMER: Flight Recorder event stamping, DuckDB row consumers, downstream session timeline reconstruction | SERIALIZER_TRANSPORT: runtime-bound ids copied into `session_span_id` / `activity_span_id` event fields | VALIDATOR_READER: span propagation tests and code inspection | TRIPWIRE_TESTS: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml` | DRIFT_RISK: spans are generated inconsistently or rebound per event, breaking hierarchy
  - CONTRACT: `model_session_id` plus session/activity span propagation across session/model/tool events | PRODUCER: workflow runtime emitters | CONSUMER: query filters, validators, DCC backend, operators | SERIALIZER_TRANSPORT: `FlightRecorderEventBase` stored in DuckDB and exposed through API filters | VALIDATOR_READER: existing `model_session_id` tripwires plus new span tests | TRIPWIRE_TESTS: `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml` | DRIFT_RISK: correlation keys drift between event families and break cross-row session reconstruction

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_scheduler_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test model_run_spawn_announce_back_event_is_emitted_for_parented_completion --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just gov-check`
- CANONICAL_CONTRACT_EXAMPLES:
  - Root session creation emits `session.created` with `model_session_id` and stable `session_span_id`
  - A `model_run` row within that session emits an `activity_span_id` nested under the session span
  - A nested tool call reuses the session id and carries a child `activity_span_id` rather than dropping out of the span tree
  - `session.completed` totals align with runtime token/cost aggregation for that session

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Add the missing lifecycle event family (`FR-EVT-SESS-001..005`) to the existing Flight Recorder schema/validation surface in `flight_recorder/mod.rs`.
  - Introduce or wire the runtime session/activity span binding substrate in the existing workflow execution path; reuse current builder hooks instead of creating a second observability layer.
  - Extend storage/query proof and scheduler-session tests so lifecycle emission and span propagation are covered without weakening existing `model_session_id` or spawn/scheduler tripwires.
- HOT_FILES:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- TRIPWIRE_TESTS:
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`
- CARRY_FORWARD_WARNINGS:
  - Do not replace `model_session_id` with span ids; it remains the primary session-wide correlation key.
  - Do not add a new observability store, sidecar pipeline, or duplicate runtime truth surface.
  - Do not regress the already-landed scheduler/spawn/model_session_id foundations while wiring lifecycle and span telemetry.
  - Prefer extending the existing recorder/runtime/test surfaces over adding new narrow checks or helper commands unless a real blind spot remains.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - Confirm `FR-EVT-SESS-001..005` exists in the canonical schema/validation surface and is actually emitted by workflow/session runtime paths.
  - Confirm every lifecycle row carries `model_session_id` and coherent `session_span_id` values.
  - Confirm model runs and nested tool calls stamp `activity_span_id` consistently under the correct session span.
  - Confirm the existing DuckDB/query/API surfaces remain the single backend truth for session reconstruction.
- FILES_TO_READ:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- COMMANDS_TO_RUN:
  - `rg -n "session.created|session.state_change|session.completed|session.message|session.budget_warning|with_activity_span|with_session_span|model_session_id" src/backend/handshake_core/src/flight_recorder/mod.rs src/backend/handshake_core/src/flight_recorder/duckdb.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/model_session_scheduler_tests.rs`
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just gov-check`
- POST_MERGE_SPOTCHECKS:
  - Confirm the downstream DCC backend packet can query session timelines without asking for a second recorder surface.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The exact in-memory persistence shape for span bindings is not proven yet; it may live as explicit typed records or equivalent runtime state as long as the emitted Flight Recorder semantics remain identical and there is no second truth store.
  - Downstream DCC rendering, swim-lane layout, and deep-link UX remain unproven until the control-plane backend and UI packets consume this substrate.
  - Cross-session mailbox/announce-back correlation beyond the current packet scope is not fully proven at refinement time.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The spec explicitly defines the lifecycle family, span binding rules, correlation rule, and session-scoped observability outcomes, and the refinement maps those clauses to concrete code surfaces and executable proof.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The required norms already exist in the current Master Spec. This packet activates them in code and does not need a version bump or appendix expansion.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 4.3.9.18.2 Span Binding (Normative)
- CONTEXT_START_LINE: 32710
- CONTEXT_END_LINE: 32734
- CONTEXT_TOKEN: ModelSessionSpanBinding
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 4.3.9.18.2 Span Binding (Normative)

  - Every `ModelSession` MUST create a `ModelSessionSpan` at session creation and close it at session completion/cancellation.
  - Every `model_run` job within a session MUST create an `ActivitySpan` nested under the session's `ModelSessionSpan`.
  - Tool calls within a model run MUST create child `ActivitySpan`s under the model run span.

  ```yaml
  # ADD v02.137
  ModelSessionSpanBinding:
    session_id: string
    model_session_span_id: string
    parent_model_session_span_id: string | null  # null for root sessions; parent session's span for children

  ActivitySpanBinding:
    activity_span_id: string
    model_session_span_id: string
    job_id: string
    model_id: ModelId
    start_time: string
    end_time: string | null
    token_count: int | null
    cost_usd: number | null
  ```

  **Correlation rule (HARD):** Every Flight Recorder event emitted in the context of a `ModelSession` MUST set `FlightRecorderEventBase.model_session_id = ModelSession.session_id`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 4.3.9.18.4 Flight Recorder Events (Session Lifecycle)
- CONTEXT_START_LINE: 32745
- CONTEXT_END_LINE: 32769
- CONTEXT_TOKEN: FR-EVT-SESS-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 4.3.9.18.4 Flight Recorder Events (Session Lifecycle)

  ```yaml
  # ADD v02.137
  FR-EVT-SESS-001:
    event_type: "session.created"
    payload: { session_id, model_id, backend, role, wp_id, mt_id, memory_policy, spawn_depth }

  FR-EVT-SESS-002:
    event_type: "session.state_change"
    payload: { session_id, from_state, to_state, reason }

  FR-EVT-SESS-003:
    event_type: "session.completed"
    payload: { session_id, total_tokens, total_cost_usd, duration_ms, messages_count }

  FR-EVT-SESS-004:
    event_type: "session.message"
    payload: { session_id, message_id, role, content_hash, token_count }
    # NOTE: content is stored as artifact; event carries hash only (INV-SESS-002)

  FR-EVT-SESS-005:
    event_type: "session.budget_warning"
    payload: { session_id, budget_type, current_value, threshold_value }
  ```
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 11.5 Schema Registry Event Family Coverage
- CONTEXT_START_LINE: 66867
- CONTEXT_END_LINE: 66881
- CONTEXT_TOKEN: Schema registry must include subsystem event families
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **Schema registry must include subsystem event families:** The Flight Recorder schema validator MUST include and validate Locus event families defined in \u00a72.3.15.6 (FR-EVT-WP-*, FR-EVT-MT-*, FR-EVT-DEP-*, FR-EVT-TB-*, FR-EVT-SYNC-*, FR-EVT-QUERY-*), and Multi-Session event families defined in \u00a74.3.9.13/\u00a74.3.9.15/\u00a74.3.9.18 (FR-EVT-SESS-*, FR-EVT-SESS-SCHED-*, FR-EVT-SESS-SPAWN-*). Unknown event IDs MUST be rejected.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 11.9.1.X Session-Scoped Observability Requirements [ADD v02.137]
- CONTEXT_START_LINE: 70713
- CONTEXT_END_LINE: 70731
- CONTEXT_TOKEN: Timeline view:
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.9.1.X Session-Scoped Observability Requirements [ADD v02.137]

  ##### 11.9.1.X.1 ModelSessionSpan Binding to ModelSession

  Every `ModelSession` (\u00a74.3.9.12) MUST be associated with exactly one `ModelSessionSpan` (distinct from the operator `SessionSpan`). The span:
  - opens at session creation,
  - closes at session completion/cancellation/failure,
  - contains all `ActivitySpan`s for `model_run` jobs and tool calls within the session.

  ##### 11.9.1.X.2 Cross-Session Correlation

  When sessions communicate (announce-back, Role Mailbox), the spans MUST carry correlation IDs so that the full conversation across parent and child sessions can be reconstructed in a timeline view.

  ##### 11.9.1.X.3 UI Surface Requirements

  - **Timeline view:** Sessions displayed as horizontal swim-lanes with nested activity spans.
  - **Cost overlay:** Token cost per span, aggregated per session.
  - **Filter:** By session_id, role, model, WP, time range.
  - **Deep-link:** From any span \u2192 Flight Recorder events \u2192 artifacts \u2192 session message thread.
  ```

### DISCOVERY (RGF-94 discovery fields; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (activates existing PRIM-ModelSessionSpanBinding and PRIM-ActivitySpanBinding rather than minting new primitives)
- DISCOVERY_STUBS: NONE_CREATED (the exact WP stub already exists and the downstream DCC backend dependency is already represented in BUILD_ORDER)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (session observability is already a spec-defined interaction between ModelSession, Flight Recorder, and span bindings)
- DISCOVERY_UI_CONTROLS: downstream session timeline swim-lanes, per-session cost overlay, session filters, and deep-links from spans into artifacts/events
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED (current spec already names the lifecycle family, span binding contract, schema-registry coverage, and session-scoped observability outcomes)
- DISCOVERY_JUSTIFICATION: This refinement activates an existing stub on top of validated adjacent foundations. The work is implementation coverage and packet activation, not primitive discovery or spec expansion.

### MICROTASKS (implementation breakdown for PACKET_HYDRATION)
- MT-001: Add lifecycle family coverage in the existing recorder schema surface
  - Register and validate `FR-EVT-SESS-001..005` in `flight_recorder/mod.rs`
  - Keep the current recorder envelope and schema validation path as the only contract surface
  - Files: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- MT-002: Wire live session/activity span binding through the existing workflow runtime
  - Bind one session span per `ModelSession`
  - Stamp model-run and nested tool-call activity spans through the existing event builder hooks
  - Files: `src/backend/handshake_core/src/workflows.rs`
- MT-003: Preserve recorder/query/storage coherence
  - Ensure DuckDB/query/API surfaces continue to round-trip `model_session_id`, `session_span_id`, and `activity_span_id`
  - Avoid introducing any second observability store
  - Files: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- MT-004: Prove lifecycle emission and span propagation
  - Add or extend tests for lifecycle payload validation, span propagation, and regression protection for existing scheduler/spawn/model_session_id behavior
  - Files: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`
