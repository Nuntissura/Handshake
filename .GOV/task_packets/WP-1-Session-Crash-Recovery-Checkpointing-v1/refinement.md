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
- WP_ID: WP-1-Session-Crash-Recovery-Checkpointing-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-05T01:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- SPEC_TARGET_SHA1: 747a1e77cbe2e1c564d1a99d5c39265edc6aeca2
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja060420260752
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Session-Crash-Recovery-Checkpointing-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- ModelSession is missing checkpoint fields (checkpoint_artifact_id, last_checkpoint_at, checkpoint_count). No checkpoint state is persisted.
- No SessionCheckpoint struct exists. The spec defines the checkpoint contract (4.3.9.19) but no Rust type formalizes it.
- No checkpoint creation logic at required boundaries (tool completion, state transitions). Checkpoints are not written at any lifecycle point.
- No checkpoint-based recovery function exists. After a crash, sessions cannot be resumed from their last known good state.
- Startup scan does not check session checkpoints. Orphaned sessions with valid checkpoints are not offered for recovery.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: Master Spec v02.179 session checkpoint sections, product code in handshake_core, existing storage and flight recorder implementations
- REFERENCES: spec sections 4.3.9.19, 4.3.9.12, 4.3.9.21; workflows.rs, storage/mod.rs, storage/sqlite.rs, storage/postgres.rs, flight_recorder/mod.rs
- PATTERNS_EXTRACTED: WAL-style checkpointing for session state snapshots; boundary-triggered checkpoints at tool completion; recovery-first startup scan
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT boundary-triggered checkpoint creation at tool completion and state transitions; ADOPT recovery-first startup scan that detects orphaned sessions with valid checkpoints; ADAPT WAL-style checkpointing to session-granularity snapshots stored as structured JSON artifacts; REJECT continuous streaming checkpoints as over-engineered for session-granularity recovery
- LICENSE/IP_NOTES: internal code and spec review only; no third-party code or copyrighted text is intended for direct reuse
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Spec 4.3.9.19 already defines the checkpoint contract and recovery flow. No new normative text is needed; this WP implements the existing spec-defined behavior.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP is strictly internal mechanical work implementing the existing spec-defined checkpoint contract (4.3.9.19). The checkpoint schema, creation boundaries, and recovery flow are fully specified. No external research is needed to shape the implementation.
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
- ADOPT_PATTERNS:
  - NONE
- ADAPT_PATTERNS:
  - NONE
- REJECT_PATTERNS:
  - NONE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- 2 new Flight Recorder events required:
  - session.checkpoint_created: session_id, checkpoint_id, checkpoint_artifact_id, checkpoint_count, boundary_trigger (tool_completion|state_transition), tool_call_id (nullable)
  - session.recovery_attempted: session_id, checkpoint_id, recovery_result (success|failed|skipped), failure_reason (nullable), recovered_state

### RED_TEAM_ADVISORY (security failure modes)
- Risk: checkpoint data could contain sensitive tool outputs that persist beyond session lifetime. Mitigation: checkpoints follow the same retention policy as the parent session; cleanup removes checkpoints when session is purged.
- Risk: recovery from a stale checkpoint could replay already-completed tool calls. Mitigation: checkpoint includes last_tool_call_id to enable idempotent recovery; recovery skips already-completed work.
- Risk: corrupted checkpoint artifact leads to crash loop on recovery. Mitigation: checkpoint validation on load; if checkpoint fails validation, mark session as unrecoverable and emit FR event with failure_reason.
- Risk: concurrent checkpoint writes during rapid state transitions could produce inconsistent snapshots. Mitigation: checkpoint creation is serialized per session; only one checkpoint write is in-flight at a time.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-ModelSession
  - PRIM-SessionCheckpoint
  - PRIM-SessionState
  - PRIM-SessionRegistry
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-ModelSession
  - PRIM-SessionCheckpoint
  - PRIM-SessionState
  - PRIM-SessionRegistry
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - PRIM-SessionCheckpoint is spec-defined (4.3.9.19) but not yet implemented as a Rust type. This WP creates the struct but does not register a new PRIM-ID because the spec already defines it.
  - PRIM-ModelSession is extended with checkpoint_artifact_id, last_checkpoint_at, and checkpoint_count fields.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: All primitives touched (ModelSession, SessionCheckpoint, SessionState, SessionRegistry) are already defined in the spec. No new PRIM-IDs are created. SessionCheckpoint struct is the implementation of an existing spec-defined primitive.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No orphan primitives discovered. All primitives are directly attached to the checkpoint spec section (4.3.9.19).

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Appendix-only updates still count as a spec update boundary.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Session crash recovery and checkpointing is an implementation of the existing session orchestration feature family defined in spec 4.3.9.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface in this backend WP. DCC crashed session badges and resume/cancel buttons are discovery UI controls noted for downstream work.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Checkpoint x artifact is an existing storage interaction already covered by the spec-defined checkpoint contract. No new interaction edges are introduced.
- APPENDIX_MAINTENANCE_NOTES:
  - No new primitives, no new interaction matrix edges, no appendix updates required. This WP implements the existing spec-defined checkpoint contract.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is involved in session checkpointing | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication surface involved | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physics surface involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: no simulation surface involved | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware surface involved | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: Director orchestrates sessions but checkpoint is a storage-level concern below orchestration | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface involved | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: no publication surface involved | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking surface involved | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food safety surface involved | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no logistics surface involved | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: checkpoint artifacts are archival targets but archivist engine is not modified | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: no retrieval surface involved | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: no analysis surface involved | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset ingestion involved | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: ModelSession schema migration adds checkpoint_artifact_id, last_checkpoint_at, and checkpoint_count columns; SessionCheckpoint table creation for both SQLite and Postgres backends | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: checkpoint is governance evidence for session lifecycle; recovery decision is a governed operation with FR audit trail | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring surface involved | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: checkpoint carries session context snapshot for recovery; recovered session resumes with checkpointed context | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: no versioning surface modified | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: checkpoint does not modify sandbox boundaries; recovered sessions inherit original sandbox scope | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: 2 new FR events (session.checkpoint_created, session.recovery_attempted) for checkpoint lifecycle audit trail | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: checkpoint timestamps are internal; no calendar event surface | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code editor surface involved | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface involved | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface involved | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: checkpoint does not modify Locus work tracking | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: checkpoint artifacts are not Loom documents | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: checkpoint does not modify WP contract | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task board modification | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: checkpoint does not modify MT contract | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: DCC crashed session badges and resume/cancel buttons are discovery UI controls but DCC backend is not modified in this WP | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: FEMS operates per-session; checkpoint does not modify FEMS contract | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: checkpoint boundaries at tool completion and state transitions; recovery restores session into the execution runtime | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no spec-to-prompt surface involved | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: ModelSession schema migration and SessionCheckpoint table must work for both SQLite and Postgres backends | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: SessionCheckpoint is structured JSON for recovery tooling; local models can inspect checkpoint state | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no stage surface involved | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio surface involved | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design surface involved | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: checkpoint data is not training material | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE surface directly involved | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: no RAG surface involved | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: checkpoint lifecycle event taxonomy | SUBFEATURES: session.checkpoint_created, session.recovery_attempted | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-SessionCheckpoint, PRIM-SessionState | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: 2 new FR events give full audit trail for checkpoint creation and recovery attempts
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: checkpoint creation at execution boundaries | SUBFEATURES: checkpoint at tool completion, checkpoint at state transition, serialized checkpoint writes | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-SessionCheckpoint, PRIM-SessionState | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: core checkpoint creation logic at spec-defined boundaries
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: dual-backend checkpoint schema | SUBFEATURES: ModelSession checkpoint columns migration, SessionCheckpoint table DDL for SQLite and Postgres | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-SessionCheckpoint | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: schema changes must be portable across both storage backends
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: structured checkpoint JSON for recovery tooling | SUBFEATURES: SessionCheckpoint JSON schema, checkpoint state snapshot format, recovery decision payload | PRIMITIVES_FEATURES: PRIM-SessionCheckpoint | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: structured JSON enables local models to inspect and reason about session recovery
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: create_session_checkpoint at boundaries | JobModel: WORKFLOW | Workflow: session_checkpoint_create | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: session.checkpoint_created | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: creates checkpoint artifact at tool completion and state transition boundaries
  - Capability: recover_session_from_checkpoint | JobModel: WORKFLOW | Workflow: session_recovery | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: session.recovery_attempted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: restores session state from last valid checkpoint; validates checkpoint integrity before recovery
  - Capability: startup orphan checkpoint scan | JobModel: WORKFLOW | Workflow: startup_session_scan | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: session.recovery_attempted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends existing startup scan to detect sessions with valid checkpoints and offer recovery
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - This packet extends existing primitives with checkpoint fields and creates the SessionCheckpoint struct.
  - The checkpoint contract creates a storage-level interaction between session management and the artifact system.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE (checkpoint x artifact is an existing storage interaction covered by the spec-defined checkpoint contract)
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Checkpoint x artifact is an existing storage interaction already covered by spec 4.3.9.19. No new interaction matrix edges are introduced by this WP.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This WP implements the existing spec-defined checkpoint contract (4.3.9.19). The checkpoint schema, creation boundaries, and recovery flow are fully specified in the Master Spec. No external matrix research is needed.
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
  - Combo: SessionCheckpoint + Flight Recorder audit trail | Pillars: Flight Recorder, Execution / Job Runtime | Mechanical: engine.sovereign | Primitives/Features: PRIM-SessionCheckpoint, PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: checkpoint creation and recovery attempts emit FR events for governance audit
  - Combo: ModelSession checkpoint fields + dual-backend schema migration | Pillars: SQL to PostgreSQL shift readiness, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-ModelSession, PRIM-SessionCheckpoint | Resolution: IN_THIS_WP | Stub: NONE | Notes: schema migration must be portable across SQLite and Postgres
  - Combo: SessionCheckpoint JSON + local model recovery tooling | Pillars: LLM-friendly data, Execution / Job Runtime | Mechanical: engine.context | Primitives/Features: PRIM-SessionCheckpoint | Resolution: IN_THIS_WP | Stub: NONE | Notes: structured checkpoint JSON enables local models to inspect and drive recovery decisions
  - Combo: startup orphan scan + checkpoint recovery | Pillars: Execution / Job Runtime | Mechanical: engine.sovereign | Primitives/Features: PRIM-SessionRegistry, PRIM-SessionCheckpoint, PRIM-SessionState | Resolution: IN_THIS_WP | Stub: NONE | Notes: existing startup orphan scan extended to detect recoverable sessions
  - Combo: checkpoint context snapshot + DBA schema migration | Pillars: SQL to PostgreSQL shift readiness, LLM-friendly data | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-SessionCheckpoint, PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: checkpoint JSON snapshot must be portable across SQLite and Postgres; DBA migration and context format must stay aligned
  - Combo: Flight Recorder checkpoint events + Sovereign governance trail | Pillars: Flight Recorder | Mechanical: engine.sovereign | Primitives/Features: PRIM-SessionCheckpoint, PRIM-SessionState | Resolution: IN_THIS_WP | Stub: NONE | Notes: checkpoint FR events provide governed evidence for session lifecycle compliance and audit
  - Combo: checkpoint idempotent recovery + Context engine session resume | Pillars: Execution / Job Runtime, LLM-friendly data | Mechanical: engine.context, engine.sovereign | Primitives/Features: PRIM-SessionCheckpoint, PRIM-SessionState, PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: recovery must restore context snapshot and skip already-completed tool calls for idempotent resume
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations resolve through IN_THIS_WP; no stubs needed and no silent drops.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed session/scheduler packets, current Master Spec v02.179, and local product code under src/backend/handshake_core
- MATCHED_STUBS:
  - NONE
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-ModelSession-Core-Scheduler-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established ModelSession and SessionRegistry primitives; crash recovery extends ModelSession with checkpoint fields but does not duplicate scheduler logic
  - Artifact: WP-1-Session-Spawn-Contract-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: spawn contract handles session lifecycle delegation; crash recovery handles session lifecycle durability after unexpected termination; complementary but distinct concerns
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: primitive | Verdict: PARTIAL | Notes: ModelSession struct exists but has no checkpoint_artifact_id, last_checkpoint_at, or checkpoint_count fields
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: primitive | Verdict: NOT_PRESENT | Notes: SQLite schema has no session_checkpoints table and no checkpoint columns on model_sessions table
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: primitive | Verdict: NOT_PRESENT | Notes: Postgres schema has no session_checkpoints table and no checkpoint columns on model_sessions table
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: execution | Verdict: NOT_PRESENT | Notes: no create_session_checkpoint or recover_session_from_checkpoint functions exist
  - Path: ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: execution | Verdict: NOT_PRESENT | Notes: Flight Recorder has scheduler events but no checkpoint-specific FR events
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: All KEEP_SEPARATE; no duplication. This WP fills the crash recovery and checkpointing gap that sits on top of the existing ModelSession/scheduler foundation.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet implements a backend session crash recovery and checkpointing contract. GUI surfaces (crashed session badge in DCC, resume/cancel buttons, checkpoint timeline) are noted as discovery UI controls for downstream work.
- UI_SURFACES:
  - NONE (backend only; DCC surfaces noted in DISCOVERY_UI_CONTROLS)
- UI_CONTROLS (buttons/dropdowns/inputs):
  - NONE (backend only)
- UI_STATES (empty/loading/error):
  - NONE (backend only)
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE (backend only)
- UI_ACCESSIBILITY_NOTES:
  - NONE (backend only)
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet. DCC visualization of crashed sessions is a downstream concern.
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
- BUILD_ORDER_DEPENDS_ON: WP-1-ModelSession-Core-Scheduler, WP-1-Unified-Tool-Surface-Contract, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md 4.3.9.19 Session Crash Recovery and Checkpointing (Normative)
- WHAT: Implement SessionCheckpoint struct, add checkpoint fields to ModelSession, create checkpoint at required boundaries (tool completion, state transitions), implement checkpoint-based recovery, extend startup scan to detect recoverable sessions, emit 2 FR events for checkpoint lifecycle.
- WHY: Sessions that crash mid-execution lose all progress and context; checkpoint-based recovery enables resumption from the last known good state, preserving expensive LLM work and maintaining session continuity for operators.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- OUT_OF_SCOPE:
  - DCC crashed session visualization (downstream UI concern)
  - session spawn contract (WP-1-Session-Spawn-Contract)
  - checkpoint retention policy tuning (future operational concern)
  - cross-node checkpoint replication (single-node scope for now)
- TEST_PLAN:
  ```bash
  cargo test session_checkpoint
  cargo test session_recovery
  cargo test startup_scan
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - SessionCheckpoint struct exists as a Rust type with serde serialization, containing session_id, checkpoint_id, checkpoint_artifact_id, session_state_snapshot, last_tool_call_id, and created_at.
  - ModelSession has checkpoint_artifact_id (nullable), last_checkpoint_at (nullable), and checkpoint_count fields.
  - DB migration adds checkpoint columns to model_sessions table and creates session_checkpoints table for both SQLite and Postgres.
  - create_session_checkpoint() is called at tool completion and state transition boundaries.
  - recover_session_from_checkpoint() validates checkpoint integrity and restores session state.
  - Startup scan detects sessions in non-terminal state with valid checkpoints and offers recovery.
  - session.checkpoint_created FR event is emitted on checkpoint creation.
  - session.recovery_attempted FR event is emitted on recovery attempt (success or failure).
- PRIMITIVES_EXPOSED:
  - PRIM-ModelSession
  - PRIM-SessionCheckpoint
  - PRIM-SessionState
  - PRIM-SessionRegistry
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - ModelSession
  - SessionCheckpoint
  - checkpoint_artifact_id
  - last_checkpoint_at
  - checkpoint_count
  - session_checkpoints
  - create_session_checkpoint
  - recover_session_from_checkpoint
  - startup_scan
  - SessionState
  - SessionRegistry
- RUN_COMMANDS:
  ```bash
  rg -n "ModelSession|SessionCheckpoint|checkpoint|session_checkpoints" src/backend/handshake_core/src
  cargo test session_checkpoint
  cargo test session_recovery
  cargo test startup_scan
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Checkpoint with sensitive tool outputs" -> "sensitive data persists beyond session lifetime if checkpoint retention is not aligned with session cleanup"
  - "Recovery replays completed tool calls" -> "duplicate side effects from tool re-execution if last_tool_call_id is not tracked"
  - "Corrupted checkpoint crash loop" -> "recovery from invalid checkpoint data causes repeated crashes if checkpoint validation is not performed"
  - "Concurrent checkpoint writes" -> "inconsistent session state snapshot if checkpoint creation is not serialized per session"
  - "Missing FR events" -> "checkpoint lifecycle becomes invisible to operators and audit tools"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just orchestrator-prepare-and-packet` will sync Task Board and Build Order truth.
  - After packet creation, verify the Build Order shows this WP blocking WP-1-Dev-Command-Center-Control-Plane-Backend.
  - Confirm dependency on WP-1-ModelSession-Core-Scheduler, WP-1-Unified-Tool-Surface-Contract, and WP-1-Artifact-System-Foundations is reflected in the Build Order graph.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Session Crash Recovery and Checkpointing 4.3.9.19 | WHY_IN_SCOPE: spec defines SessionCheckpoint contract and recovery flow but no Rust types or checkpoint logic exist | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test session_checkpoint; cargo test session_recovery | RISK_IF_MISSED: crashed sessions lose all progress and cannot be resumed; expensive LLM work is wasted
  - CLAUSE: ModelSession checkpoint fields 4.3.9.12 | WHY_IN_SCOPE: ModelSession struct is missing checkpoint_artifact_id, last_checkpoint_at, and checkpoint_count fields defined in spec | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs | EXPECTED_TESTS: cargo test session_checkpoint | RISK_IF_MISSED: no checkpoint state can be tracked per session; recovery is impossible
  - CLAUSE: Checkpoint creation at boundaries | WHY_IN_SCOPE: spec requires checkpoints at tool completion and state transition boundaries but no create_session_checkpoint function exists | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test session_checkpoint | RISK_IF_MISSED: sessions run without checkpoints and crash recovery has nothing to recover from
  - CLAUSE: Checkpoint-based recovery | WHY_IN_SCOPE: spec defines recovery flow from last valid checkpoint but no recover_session_from_checkpoint function exists | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test session_recovery | RISK_IF_MISSED: even with checkpoints stored, crashed sessions cannot be resumed
  - CLAUSE: Startup orphan checkpoint scan | WHY_IN_SCOPE: spec requires startup scan to detect sessions with valid checkpoints; existing scan does not check checkpoint state | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test startup_scan | RISK_IF_MISSED: recoverable sessions are silently discarded at startup instead of being offered for recovery
  - CLAUSE: Anti-Pattern AP-008 | WHY_IN_SCOPE: spec defines AP-008 as failing to checkpoint sessions before risky operations; this WP prevents that anti-pattern | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test session_checkpoint | RISK_IF_MISSED: sessions proceed through risky operations without checkpoint safety net

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: SessionCheckpoint JSON schema | PRODUCER: create_session_checkpoint in workflows.rs | CONSUMER: recover_session_from_checkpoint, startup scan, Flight Recorder | SERIALIZER_TRANSPORT: in-process struct with serde JSON serialization stored in DB | VALIDATOR_READER: session_checkpoint tests | TRIPWIRE_TESTS: cargo test session_checkpoint | DRIFT_RISK: checkpoint fields drift between creation and recovery without schema enforcement
  - CONTRACT: ModelSession checkpoint columns | PRODUCER: create_session_checkpoint updates ModelSession record | CONSUMER: recover_session_from_checkpoint reads checkpoint_artifact_id, startup scan reads last_checkpoint_at | SERIALIZER_TRANSPORT: SQL columns in model_sessions table (SQLite and Postgres) | VALIDATOR_READER: session_checkpoint tests | TRIPWIRE_TESTS: cargo test session_checkpoint | DRIFT_RISK: schema migration drifts between SQLite and Postgres DDL
  - CONTRACT: FR event payloads for checkpoint lifecycle | PRODUCER: checkpoint creation and recovery hooks in workflows.rs | CONSUMER: Flight Recorder storage, operator dashboards | SERIALIZER_TRANSPORT: Flight Recorder JSON event payload | VALIDATOR_READER: session_checkpoint and session_recovery tests | TRIPWIRE_TESTS: cargo test session_checkpoint; cargo test session_recovery | DRIFT_RISK: event payload fields drift from spec-defined schema

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test session_checkpoint
  - cargo test session_recovery
  - cargo test startup_scan
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - a session completes a tool call and create_session_checkpoint() writes a SessionCheckpoint with the tool_call_id and current session state snapshot; session.checkpoint_created FR event is emitted
  - a session transitions state and create_session_checkpoint() writes a checkpoint at the transition boundary
  - a crashed session with a valid checkpoint is detected by startup scan; recover_session_from_checkpoint() validates the checkpoint, restores session state, and emits session.recovery_attempted with recovery_result=success
  - a crashed session with a corrupted checkpoint is detected by startup scan; recover_session_from_checkpoint() fails validation and emits session.recovery_attempted with recovery_result=failed and failure_reason
  - a session with checkpoint_count > 0 but no valid checkpoint artifact is marked unrecoverable and skipped during startup scan
  - ModelSession.checkpoint_artifact_id is updated on each successful checkpoint creation; last_checkpoint_at is set to current timestamp; checkpoint_count is incremented

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - MT-001: Define SessionCheckpoint struct with session_id, checkpoint_id, checkpoint_artifact_id, session_state_snapshot, last_tool_call_id, and created_at fields. Add checkpoint_artifact_id (nullable), last_checkpoint_at (nullable), and checkpoint_count fields to ModelSession. Create DB migration for both SQLite and Postgres: ALTER model_sessions table and CREATE session_checkpoints table.
  - MT-002: Implement create_session_checkpoint() function in workflows.rs. Call it at tool completion boundaries and state transition boundaries. Serialize session state as structured JSON artifact. Emit session.checkpoint_created FR event with session_id, checkpoint_id, checkpoint_artifact_id, checkpoint_count, boundary_trigger, and tool_call_id.
  - MT-003: Implement recover_session_from_checkpoint() function in workflows.rs. Validate checkpoint integrity on load. Restore session state from checkpoint snapshot. Update session state to reflect recovery. Emit session.recovery_attempted FR event with session_id, checkpoint_id, recovery_result, failure_reason, and recovered_state. Extend startup scan to detect sessions in non-terminal state with valid checkpoints and invoke recovery.
  - MT-004: Integration tests for checkpoint creation at boundaries, recovery from valid checkpoint, recovery failure from corrupted checkpoint, startup scan with recoverable sessions, FR event emission verification.
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- TRIPWIRE_TESTS:
  - cargo test session_checkpoint
  - cargo test session_recovery
  - cargo test startup_scan
- CARRY_FORWARD_WARNINGS:
  - Do not store checkpoint state only in memory; checkpoints must be persisted to DB so they survive process crashes.
  - Do not skip checkpoint validation on recovery; corrupted checkpoints must be detected and rejected to prevent crash loops.
  - Do not allow concurrent checkpoint writes for the same session; serialize checkpoint creation per session to prevent inconsistent snapshots.
  - Do not forget to update ModelSession.checkpoint_count on each checkpoint creation; recovery logic depends on this field to determine if a session has ever been checkpointed.
  - Do not omit last_tool_call_id from checkpoint; recovery must know which tool calls have already completed to prevent duplicate side effects.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - SessionCheckpoint exists as a typed Rust struct with serde serialization
  - ModelSession has checkpoint_artifact_id, last_checkpoint_at, and checkpoint_count fields
  - DB migration adds checkpoint columns and session_checkpoints table for both SQLite and Postgres
  - create_session_checkpoint() is called at tool completion and state transition boundaries
  - recover_session_from_checkpoint() validates checkpoint integrity and restores session state
  - Startup scan detects sessions with valid checkpoints and invokes recovery
  - session.checkpoint_created and session.recovery_attempted FR events are registered and emitted
- FILES_TO_READ:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- COMMANDS_TO_RUN:
  - rg -n "SessionCheckpoint|checkpoint_artifact_id|last_checkpoint_at|checkpoint_count|create_session_checkpoint|recover_session_from_checkpoint" src/backend/handshake_core/src
  - cargo test session_checkpoint
  - cargo test session_recovery
  - cargo test startup_scan
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- POST_MERGE_SPOTCHECKS:
  - verify SessionCheckpoint is a first-class struct not embedded in other types
  - verify ModelSession checkpoint fields are nullable for backward compatibility
  - verify DB migration works for both SQLite and Postgres backends
  - verify checkpoint creation is serialized per session (no concurrent writes)
  - verify recovery validates checkpoint integrity before restoring state
  - verify startup scan correctly distinguishes recoverable vs unrecoverable sessions
  - verify both FR events carry the spec-defined payload fields

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The exact session state snapshot format is not proven until coding completes; spec defines the semantic content but not the precise JSON schema for the snapshot.
  - Whether checkpoint creation should be synchronous or asynchronous at tool completion boundaries is not determined; synchronous is simpler but may add latency to tool calls.
  - Full recovery behavior under concurrent session operations (another session modifying shared state while recovery is in progress) is not proven at refinement time.
  - The maximum checkpoint artifact size and whether it needs compression is not characterized; depends on actual session state size in practice.
  - Checkpoint retention policy (how many checkpoints to keep per session, when to prune old checkpoints) is identified as a future operational concern but not addressed in this WP.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec section 4.3.9.19 explicitly defines Session Crash Recovery and Checkpointing as a normative section. All acceptance criteria (SessionCheckpoint struct, ModelSession checkpoint fields, checkpoint creation at boundaries, checkpoint-based recovery, startup scan extension, FR events) map directly to normative spec anchors.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Spec 4.3.9.19 already defines the checkpoint contract and recovery flow. SessionCheckpoint schema, creation boundaries, and recovery semantics are fully specified. No spec version bump required for this WP's activation.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.19 Session Crash Recovery and Checkpointing (Normative)
- CONTEXT_START_LINE: 32728
- CONTEXT_END_LINE: 32745
- CONTEXT_TOKEN: SessionCheckpoint
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.19 Session Crash Recovery and Checkpointing (Normative)

  Sessions MUST persist checkpoint state at defined boundaries so that
  crash recovery can resume from the last known good state. The
  SessionCheckpoint contract defines the checkpoint schema, creation
  boundaries (tool completion, state transitions), and recovery flow.
  Checkpoints are stored as structured JSON artifacts linked to the
  parent ModelSession via checkpoint_artifact_id.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.12 ModelSession (checkpoint fields)
- CONTEXT_START_LINE: 32200
- CONTEXT_END_LINE: 32205
- CONTEXT_TOKEN: checkpoint_artifact_id
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.12 ModelSession (checkpoint fields)

  ModelSession tracks checkpoint state via checkpoint_artifact_id
  (nullable reference to the latest checkpoint artifact),
  last_checkpoint_at (timestamp of last checkpoint), and
  checkpoint_count (total checkpoints created for this session).
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.21 Anti-Pattern AP-008
- CONTEXT_START_LINE: 32819
- CONTEXT_END_LINE: 32821
- CONTEXT_TOKEN: AP-008
- EXCERPT_ASCII_ESCAPED:
  ```text
  AP-008: Uncheckpointed Risky Operations. Sessions MUST NOT proceed
  through risky operations (tool calls with side effects, state
  transitions) without first creating a checkpoint. Violation of this
  anti-pattern leaves the session unrecoverable after a crash.
  ```

### DISCOVERY (RGF-94 discovery fields; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (extends existing PRIM-ModelSession and PRIM-SessionCheckpoint)
- DISCOVERY_STUBS: NONE_CREATED (DCC notification is already covered by DCC stubs)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (checkpoint x artifact is an existing storage interaction)
- DISCOVERY_UI_CONTROLS: crashed session badge in DCC, resume/cancel buttons per crashed session, checkpoint timeline in session detail view
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED
- DISCOVERY_JUSTIFICATION: This WP implements existing spec-defined checkpoint contract (4.3.9.19). The SessionCheckpoint schema and recovery flow are fully specified. No new primitive family is introduced.
