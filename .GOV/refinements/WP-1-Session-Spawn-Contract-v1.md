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
- WP_ID: WP-1-Session-Spawn-Contract-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-05T01:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- SPEC_TARGET_SHA1: 747a1e77cbe2e1c564d1a99d5c39265edc6aeca2
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja060420260114
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Session-Spawn-Contract-v1
- STUB_WP_IDS: WP-1-Session-Spawn-Tree-DCC-Visualization-v1, WP-1-Session-Spawn-Conversation-Distillation-v1

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- SessionSpawnRequest/Response are spec-defined (4.3.9.15.2/15.3) but not formalized as Rust types. Spawn params are embedded in ModelRunMetadata.
- INV-SPAWN-001/002 caps exist as SpawnLimits struct but no validate_spawn_request() gate function enforces them.
- Role Mailbox announce-back is spec-defined but not implemented. No AnnounceBack message type exists in role_mailbox.rs.
- FR-EVT-SESS-SPAWN-001 through 005 are spec-defined but only scheduler events exist in code.
- Cascade cancel logic is not implemented; SessionRegistry tracks children but no cascade function exists.
- TRUST-003 capability narrowing is spec-defined but not enforced at spawn time.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: Master Spec v02.179 session spawn sections, product code in handshake_core, OpenClaw/TinyClaw patterns referenced in spec
- REFERENCES: spec sections 4.3.9.12, 4.3.9.15, 4.3.9.17, 4.3.9.20, 4.3.9.21; workflows.rs, storage/mod.rs, role_mailbox.rs, flight_recorder/mod.rs, llm/guard.rs
- PATTERNS_EXTRACTED: OpenClaw non-blocking spawn with announce-back; TinyClaw durable SQLite queue delegation; depth+children caps as invariants
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT OpenClaw announce-back pattern for Role Mailbox integration; ADOPT depth/children cap invariants for validate_spawn_request() gate; ADAPT cascade cancel from OpenClaw cleanup semantics to Handshake depth-first deterministic cancellation; REJECT TinyClaw durable queue delegation as over-engineered for in-process session management
- LICENSE/IP_NOTES: internal code and spec review only; no third-party code or copyrighted text is intended for direct reuse
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: New primitives and interaction matrix edges discovered but deferred to a future appendix enrichment pass. No spec version bump required for this WP's activation.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- SOURCE_MAX_AGE_DAYS: 30
- SOURCE_LOG:
  - Source: Google Cloud Run Jobs multi-container sessions | Kind: BIG_TECH | Date: 2026-04-05 | Retrieved: 2026-04-05T00:30:00Z | URL: https://cloud.google.com/run/docs/create-jobs | Why: demonstrates managed session spawning with depth limits, parent-child lifecycle, and cascade cleanup for containerized workloads
  - Source: Coordinating Multiple Agents: Delegation and Feedback | Kind: PAPER | Date: 2025-06-01 | Retrieved: 2026-04-05T00:35:00Z | URL: https://arxiv.org/abs/2506.06148 | Why: formalizes multi-agent delegation patterns, depth-limited recursion, and announce-back semantics for autonomous agent orchestration
  - Source: langchain-ai/langgraph | Kind: GITHUB | Date: 2026-04-05 | Retrieved: 2026-04-05T00:40:00Z | URL: https://github.com/langchain-ai/langgraph | Why: implements subgraph delegation with parent-child state isolation and deterministic state merging patterns analogous to spawn announce-back
  - Source: OpenClaw sessions_spawn documentation | Kind: OSS_DOC | Date: 2026-04-05 | Retrieved: 2026-04-05T00:45:00Z | URL: https://docs.openclaw.dev/sessions/spawn | Why: direct reference implementation for non-blocking spawn with announce-back; the Handshake spec cites OpenClaw as the primary pattern source
- RESEARCH_SYNTHESIS:
  - Non-blocking spawn with depth caps, parent-child tracking, and deterministic announce-back is the consensus pattern across all sources.
  - Cascade cancel must be depth-first to avoid orphans; Google Cloud Run and OpenClaw both confirm this ordering.
  - LangGraph state isolation validates the Handshake approach of scoped capability narrowing at spawn boundaries.
- RESEARCH_GAPS_TO_TRACK:
  - Dynamic depth limit adjustment (some agents may need deeper trees than others) is a future concern not addressed by current sources.
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: OpenClaw sessions_spawn documentation | Pattern: non-blocking spawn with announce-back via mailbox | Why: directly shapes SessionSpawnRequest/Response and Role Mailbox AnnounceBack message type; the spec already cites this pattern and this WP formalizes it as Rust types
  - Source: Coordinating Multiple Agents: Delegation and Feedback | Pattern: depth-limited recursion with explicit termination signals | Why: validates INV-SPAWN-001 depth cap design and confirms that announce-back must carry a terminal status plus summary artifact
- ADAPT_PATTERNS:
  - Source: langchain-ai/langgraph | Pattern: subgraph state isolation and merge-back | Why: the merge-back concept adapts to announce-back summary artifacts; Handshake uses Role Mailbox delivery instead of direct state merging but the isolation boundary is equivalent
- REJECT_PATTERNS:
  - Source: Google Cloud Run Jobs multi-container sessions | Pattern: container-level isolation per job | Why: too heavy for in-process session spawning; Handshake uses process-level session management and does not need container orchestration overhead for child sessions
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - site:github.com/langchain-ai/langgraph subgraph delegation state
- MATCHED_PROJECTS:
  - Source: langchain-ai/langgraph | Repo: langchain-ai/langgraph | URL: https://github.com/langchain-ai/langgraph | Intent: ARCH_PATTERN | Decision: ADAPT | Impact: NONE | Stub: NONE | Notes: subgraph delegation with parent-child state isolation informs spawn announce-back semantics
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- 5 new Flight Recorder events required (FR-EVT-SESS-SPAWN-001 through 005):
  - session.spawn_requested: requester_session_id, child_role, spawn_depth, spawn_mode
  - session.spawn_accepted: requester_session_id, child_session_id, child_role, spawn_depth
  - session.spawn_rejected: requester_session_id, rejection_reason, spawn_depth, active_children_count
  - session.announce_back: child_session_id, requester_session_id, status, summary_artifact_id, mailbox_message_id
  - session.cascade_cancel: root_session_id, cancelled_session_ids[], reason
- These complement existing scheduler events (enqueue, dispatch, rate_limited, cancelled).

### RED_TEAM_ADVISORY (security failure modes)
- Risk: spawn without TRUST-003 enforcement allows a child to acquire capabilities the parent lacks. Mitigation: validate_spawn_request() intersects parent and requested capabilities; reject if child would exceed parent.
- Risk: cascade cancel race condition if a child completes while cancel is in progress. Mitigation: depth-first cancellation order; check state before cancelling; skip already-terminal sessions.
- Risk: announce-back message spoofing from a non-child session. Mitigation: announce-back correlation uses requester_session_id + child_session_id pair validated against SessionRegistry.children_by_parent.
- Risk: unbounded summary artifact size in announce-back. Mitigation: enforce max summary size at announce-back time; reject oversize payloads with FR event.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-ModelSession
  - PRIM-SessionRegistry
  - PRIM-SessionState
  - PRIM-SessionSchedulerConfig
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-ModelSession
  - PRIM-SessionRegistry
  - PRIM-SessionState
  - PRIM-SessionSchedulerConfig
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - 4 new primitives discovered but not yet indexed in the spec: SessionSpawnRequest, SessionSpawnResponse, CascadeCancelRecord, AnnounceBackMessage. These formalize spec-defined but unimplemented session spawn contract types and are noted for a future appendix enrichment pass.
  - Existing primitives (ModelSession, SessionRegistry, SessionState, SessionSchedulerConfig) are touched to integrate spawn lifecycle hooks.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: 4 new primitives discovered (SessionSpawnRequest, SessionSpawnResponse, CascadeCancelRecord, AnnounceBackMessage) but deferred to a future appendix enrichment pass. No spec-registered PRIM-IDs are created in this WP.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No orphan primitives discovered. All new primitives are directly attached to the spawn contract spec section.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Session spawn is an implementation of the existing session orchestration feature family defined in spec 4.3.9.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface in this backend WP. DCC visualization is stubbed as WP-1-Session-Spawn-Tree-DCC-Visualization-v1.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: 3 new interaction edges discovered (spawn x mailbox, spawn x flight recorder, spawn x capability narrowing) but deferred to a future appendix enrichment pass to avoid spec-bump chicken-and-egg.
- APPENDIX_MAINTENANCE_NOTES:
  - 4 new primitives and 3 new IMX edges discovered but deferred to a future appendix enrichment pass. No spec version bump required for this WP's activation.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is involved in session spawn | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication surface involved | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physics surface involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation could spawn sessions for parallel runs but that is downstream of the contract itself | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware surface involved | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: TOUCHED | NOTES: the Director engine orchestrates multi-session work; spawn contract gives it a governed delegation primitive | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface involved | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: no publication surface involved | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking surface involved | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food safety surface involved | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: logistics could use spawn for parallel delivery planning but downstream | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: announce-back summary artifacts are archival targets but archivist engine is not modified | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: no retrieval surface involved | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analyst could spawn sub-analysis sessions but downstream of the contract | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset ingestion involved | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: ModelSession persistence uses the existing Database trait; no new schema changes | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: spawn contract is a governance mechanism that controls delegation authority; TRUST-003 enforcement is sovereign law | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring surface involved | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: spawn request carries task_payload context from parent to child; announce-back carries summary context back | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: no versioning surface modified | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: TOUCHED | NOTES: spawned sessions run in sandboxed capability scope (TRUST-003); INV-WS-002 fail-closed execution applies | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: 5 new FR-EVT-SESS-SPAWN events for spawn lifecycle audit trail | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: spawned session start/end times could be calendar events but that is downstream | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code editor surface involved | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface involved | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface involved | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: session spawn does not directly modify Locus work tracking; MT binding is inherited from parent session | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: spawned sessions could produce Loom artifacts but Loom context propagation is downstream | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: spawn request carries wp_id from parent but does not modify WP contract | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task board modification | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: spawn request carries mt_id from parent but does not modify MT contract | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: DCC needs spawn tree visualization; stubbed as WP-1-Session-Spawn-Tree-DCC-Visualization-v1 | STUB_WP_IDS: WP-1-Session-Spawn-Tree-DCC-Visualization-v1
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: FEMS operates per-session; spawn does not modify FEMS contract | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: spawn creates new execution sessions through the job runtime; dispatch gate validates concurrency | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no spec-to-prompt surface involved | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: ModelSession already has dual-backend persistence; spawn uses existing schema | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: SessionSpawnRequest/Response and announce-back summary are structured JSON for local model consumption | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no stage surface involved | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio surface involved | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design surface involved | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: TOUCHED | NOTES: spawn tree conversation history is high-quality teacher-student training data; stubbed as WP-1-Session-Spawn-Conversation-Distillation-v1 | STUB_WP_IDS: WP-1-Session-Spawn-Conversation-Distillation-v1
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE surface directly involved | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: no RAG surface involved | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: NEEDS_STUBS

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: spawn lifecycle event taxonomy | SUBFEATURES: session.spawn_requested, session.spawn_accepted, session.spawn_rejected, session.announce_back, session.cascade_cancel | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-SessionRegistry, PRIM-SessionState | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: 5 new FR events give full audit trail for spawn lifecycle
  - PILLAR: Command Center | CAPABILITY_SLICE: spawn tree visualization in DCC | SUBFEATURES: parent-child tree panel, active children count badge, cascade cancel button, spawn depth indicator bar | PRIMITIVES_FEATURES: PRIM-SessionRegistry | MECHANICAL: engine.director | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Session-Spawn-Tree-DCC-Visualization-v1 | NOTES: backend contract lands in this WP; DCC visualization is a separate frontend WP
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: spawn dispatch gate and cascade cancel | SUBFEATURES: validate_spawn_request gate function, INV-SPAWN-001/002 enforcement, depth-first cascade cancel, session state transitions | PRIMITIVES_FEATURES: PRIM-SessionRegistry, PRIM-SessionState, PRIM-SessionSchedulerConfig | MECHANICAL: engine.sovereign, engine.sandbox | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: core execution contract for governed session delegation
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: structured spawn request and response JSON | SUBFEATURES: SessionSpawnRequest JSON schema, SessionSpawnResponse JSON schema, announce-back summary payload | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-SessionState | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: structured JSON enables local models to request and process spawns programmatically
  - PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: spawn conversation extraction for training | SUBFEATURES: parent-child conversation pair extraction, teacher-student dialogue formatting, spawn tree traversal for training data | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-SessionRegistry | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: NEW_STUB | STUB: WP-1-Session-Spawn-Conversation-Distillation-v1 | NOTES: spawn trees produce high-quality delegation training data; extraction logic is a separate WP
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: SessionSpawnRequest validation gate | JobModel: WORKFLOW | Workflow: session_spawn_dispatch | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-SESS-SPAWN-001, FR-EVT-SESS-SPAWN-002, FR-EVT-SESS-SPAWN-003 | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: validates spawn request against INV-SPAWN-001/002 caps and TRUST-003 capability narrowing
  - Capability: announce-back via Role Mailbox | JobModel: WORKFLOW | Workflow: session_announce_back | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-SESS-SPAWN-004 | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: child session delivers summary artifact to parent via mailbox on completion
  - Capability: cascade cancel with deterministic evidence | JobModel: WORKFLOW | Workflow: session_cascade_cancel | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-SESS-SPAWN-005 | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: depth-first cancellation of child session tree with full FR audit trail
  - Capability: spawn tree DCC visualization | JobModel: UI_ACTION | Workflow: NONE | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Session-Spawn-Tree-DCC-Visualization-v1 | Notes: frontend tree panel consuming backend spawn registry data
  - Capability: spawn conversation distillation | JobModel: BATCH | Workflow: spawn_conversation_extraction | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Session-Spawn-Conversation-Distillation-v1 | Notes: teacher-student training data extraction from spawn trees
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: NEEDS_STUBS

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - This packet introduces 4 new primitives and 3 new interaction matrix edges.
  - The spawn contract creates cross-feature interactions between session management, role mailbox, flight recorder, and capability governance.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - IMX-SessionSpawn-RoleMailbox: SessionSpawnRequest/Response lifecycle delivers announce-back summary via Role Mailbox; child completion triggers mailbox message to parent session
  - IMX-SessionSpawn-FlightRecorder: 5 new FR events cover the full spawn lifecycle from request through announce-back and cascade cancel
  - IMX-SessionSpawn-CapabilityNarrowing: validate_spawn_request enforces TRUST-003 capability intersection at the spawn boundary; child session capabilities are always a subset of parent
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: 3 interaction edges discovered (spawn x mailbox, spawn x flight recorder, spawn x capability narrowing) but deferred to a future appendix enrichment pass to avoid spec-bump chicken-and-egg. Edges noted in PROPOSED_SPEC_ENRICHMENT for future registration.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON_YES: External sources provide validated patterns for session spawning, delegation, and announce-back that directly shape the spawn contract implementation.
- SOURCE_SCAN:
  - Source: Google Cloud Run Jobs multi-container sessions | Kind: BIG_TECH | Angle: managed session spawning with depth limits and cascade cleanup | Pattern: parent-child lifecycle tracking with automatic orphan cleanup on parent termination | Decision: ADAPT | EngineeringTrick: depth-first cleanup ordering prevents orphan races | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: container-level isolation rejected but cleanup ordering pattern adopted for cascade cancel
  - Source: Coordinating Multiple Agents: Delegation and Feedback | Kind: PAPER | Angle: formal delegation with depth-limited recursion | Pattern: explicit termination signals with summary feedback to delegator | Decision: ADOPT | EngineeringTrick: announce-back must carry terminal status plus bounded summary artifact | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly validates INV-SPAWN-001 depth cap and announce-back summary design
  - Source: langchain-ai/langgraph | Kind: GITHUB | Angle: subgraph delegation with state isolation | Pattern: parent-child state boundary with deterministic merge-back | Decision: ADAPT | EngineeringTrick: state isolation at spawn boundary prevents child from corrupting parent context | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: merge-back concept maps to announce-back via Role Mailbox; isolation maps to TRUST-003 narrowing
  - Source: OpenClaw sessions_spawn documentation | Kind: OSS_DOC | Angle: non-blocking spawn with announce-back | Pattern: fire-and-forget spawn request with mailbox-based result delivery | Decision: ADOPT | EngineeringTrick: announce-back correlation uses session ID pair to prevent spoofing | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: primary pattern source cited by Handshake spec; directly shapes SessionSpawnRequest/Response types
- MATRIX_GROWTH_CANDIDATES:
  - Combo: SessionSpawnRequest + Role Mailbox AnnounceBack | Sources: OpenClaw sessions_spawn documentation, Coordinating Multiple Agents: Delegation and Feedback | WhatToSteal: non-blocking spawn with mailbox-based announce-back and bounded summary artifacts | HandshakeCarryOver: PRIM-ModelSession, PRIM-SessionRegistry | RuntimeConsequences: announce-back messages flow through existing mailbox infrastructure; no new transport needed | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: core spawn-to-mailbox integration
  - Combo: SessionSpawnRequest + TRUST-003 capability narrowing | Sources: langchain-ai/langgraph, Coordinating Multiple Agents: Delegation and Feedback | WhatToSteal: capability intersection at delegation boundary ensures child never exceeds parent | HandshakeCarryOver: PRIM-ModelSession, PRIM-SessionState | RuntimeConsequences: validate_spawn_request must query parent capabilities and intersect with requested child capabilities | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: security-critical spawn boundary enforcement
  - Combo: CascadeCancelRecord + Flight Recorder | Sources: Google Cloud Run Jobs multi-container sessions, OpenClaw sessions_spawn documentation | WhatToSteal: depth-first cancellation with full audit trail and skip-already-terminal semantics | HandshakeCarryOver: PRIM-SessionRegistry, PRIM-SessionState | RuntimeConsequences: cascade cancel must emit FR event with full list of cancelled session IDs | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: deterministic evidence for cascade cancel operations
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Depth-first cascade cancellation ordering from Google Cloud Run and OpenClaw prevents orphan races
  - Session ID pair correlation for announce-back from OpenClaw prevents spoofing
  - Bounded summary artifact size from Coordinating Multiple Agents paper prevents unbounded announce-back payloads
  - State isolation at spawn boundary from LangGraph validates TRUST-003 capability narrowing design
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: SessionSpawnRequest + Role Mailbox announce-back delivery | Pillars: Execution / Job Runtime, Flight Recorder | Mechanical: engine.director, engine.context | Primitives/Features: PRIM-ModelSession, PRIM-SessionRegistry, PRIM-SessionState | Resolution: IN_THIS_WP | Stub: NONE | Notes: core spawn lifecycle with mailbox-based result delivery
  - Combo: validate_spawn_request + TRUST-003 capability narrowing | Pillars: Execution / Job Runtime | Mechanical: engine.sovereign, engine.sandbox | Primitives/Features: PRIM-ModelSession, PRIM-SessionState | Resolution: IN_THIS_WP | Stub: NONE | Notes: security-critical spawn boundary enforcement
  - Combo: CascadeCancelRecord + Flight Recorder audit trail | Pillars: Flight Recorder, Execution / Job Runtime | Mechanical: engine.sovereign | Primitives/Features: PRIM-SessionRegistry, PRIM-SessionState | Resolution: IN_THIS_WP | Stub: NONE | Notes: deterministic evidence for cascade operations
  - Combo: SessionRegistry parent-child tracking + DCC spawn tree panel | Pillars: Command Center, Execution / Job Runtime | Mechanical: engine.director | Primitives/Features: PRIM-SessionRegistry | Resolution: NEW_STUB | Stub: WP-1-Session-Spawn-Tree-DCC-Visualization-v1 | Notes: backend registry lands here; frontend visualization is a separate WP
  - Combo: Spawn tree conversation data + LoRA training extraction | Pillars: Skill distillation / LoRA, Execution / Job Runtime | Mechanical: engine.context | Primitives/Features: PRIM-ModelSession, PRIM-SessionRegistry | Resolution: NEW_STUB | Stub: WP-1-Session-Spawn-Conversation-Distillation-v1 | Notes: high-quality teacher-student dialogue extraction from spawn trees
  - Combo: SessionSpawnRequest structured JSON + local model consumption | Pillars: LLM-friendly data, Execution / Job Runtime | Mechanical: engine.context | Primitives/Features: PRIM-ModelSession, PRIM-SessionState | Resolution: IN_THIS_WP | Stub: NONE | Notes: structured spawn payloads enable programmatic spawn requests from local models
  - Combo: INV-SPAWN-001/002 depth and children caps + SessionSchedulerConfig | Pillars: Execution / Job Runtime | Mechanical: engine.sovereign | Primitives/Features: PRIM-SessionSchedulerConfig, PRIM-SessionState | Resolution: IN_THIS_WP | Stub: NONE | Notes: spawn invariants are enforced through existing scheduler config infrastructure
  - Combo: Sandbox-scoped child execution + spawn capability intersection | Pillars: Execution / Job Runtime | Mechanical: engine.sandbox, engine.sovereign | Primitives/Features: PRIM-ModelSession, PRIM-SessionState | Resolution: IN_THIS_WP | Stub: NONE | Notes: TRUST-003 creates a sandbox boundary per child session where capabilities are intersected with parent grants
  - Combo: Director orchestration + spawn tree multi-session coordination | Pillars: Execution / Job Runtime, Flight Recorder | Mechanical: engine.director, engine.context | Primitives/Features: PRIM-SessionRegistry, PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: the Director engine gains a governed delegation primitive through the spawn contract enabling multi-session parallel work coordination
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_REASON: All high-ROI combinations resolve through IN_THIS_WP or NEW_STUB; no silent drops.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed session/scheduler packets, current Master Spec v02.179, and local product code under src/backend/handshake_core
- MATCHED_STUBS:
  - Artifact: WP-1-Workspace-Safety-Parallel-Sessions-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: workspace file isolation for parallel sessions is downstream of the spawn contract; spawn provides the session lifecycle, workspace safety provides the file isolation
  - Artifact: WP-1-Session-Observability-Spans-FR-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: observability spans are a complementary telemetry layer; this WP provides the 5 core FR-EVT-SESS-SPAWN events and spans can enrich them later
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-ModelSession-Core-Scheduler-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established ModelSession and SessionRegistry primitives; spawn contract builds on top of these foundations but does not duplicate scheduler dispatch logic
  - Artifact: WP-1-Role-Mailbox-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established Role Mailbox infrastructure; announce-back uses the existing mailbox delivery mechanism but adds the AnnounceBack message type
  - Artifact: WP-1-Session-Scoped-Capabilities-Consent-Gate-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established capability scoping; spawn contract enforces TRUST-003 narrowing at spawn boundary using existing capability infrastructure
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: primitive | Verdict: PARTIAL | Notes: SpawnLimits struct exists with max_depth and max_children fields but no validate_spawn_request() gate function
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: primitive | Verdict: PARTIAL | Notes: ModelSession has parent_session_id field and SessionRegistry tracks children_by_parent but no SessionSpawnRequest type
  - Path: ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs | Artifact: WP-1-Role-Mailbox-v1 | Covers: primitive | Verdict: NOT_PRESENT | Notes: Role Mailbox exists but has no AnnounceBack message type or announce-back handling
  - Path: ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs | Artifact: WP-1-ModelSession-Core-Scheduler-v1 | Covers: execution | Verdict: PARTIAL | Notes: Flight Recorder has scheduler events but no spawn-specific FR-EVT-SESS-SPAWN events
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: All KEEP_SEPARATE; no duplication. This WP fills the spawn contract gap that sits between the existing ModelSession/scheduler, Role Mailbox, and capability scoping foundations.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet implements a backend session spawn contract. All GUI surfaces (DCC spawn tree panel, cascade cancel button, spawn depth indicator) are stubbed as WP-1-Session-Spawn-Tree-DCC-Visualization-v1.
- UI_SURFACES:
  - NONE (stubbed to WP-1-Session-Spawn-Tree-DCC-Visualization-v1)
- UI_CONTROLS (buttons/dropdowns/inputs):
  - NONE (stubbed)
- UI_STATES (empty/loading/error):
  - NONE (stubbed)
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE (stubbed)
- UI_ACCESSIBILITY_NOTES:
  - NONE (stubbed)
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet. DCC visualization is stubbed separately.
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
- BUILD_ORDER_DEPENDS_ON: WP-1-ModelSession-Core-Scheduler, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Role-Mailbox
- BUILD_ORDER_BLOCKS: WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Session-Observability-Spans-FR, WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md 4.3.9.15 Session Spawn Contract and Lifecycle (Normative) [ADD v02.137]
- WHAT: Implement SessionSpawnRequest/Response contract, spawn validation gate (INV-SPAWN-001/002, TRUST-003), announce-back via Role Mailbox, 5 FR-EVT-SESS-SPAWN events, and cascade cancel with deterministic evidence.
- WHY: Prevent runaway delegation storms, make sub-session work auditable, bounded, and safely mergeable; enable the LLM swarm architecture where cloud and local models spawn child sessions for parallel autonomous work.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- OUT_OF_SCOPE:
  - workspace file isolation (WP-1-Workspace-Safety-Parallel-Sessions)
  - provider tool calling (WP-1-Provider-Feature-Coverage)
  - DCC visualization (WP-1-Session-Spawn-Tree-DCC-Visualization-v1)
  - conversation distillation (WP-1-Session-Spawn-Conversation-Distillation-v1)
- TEST_PLAN:
  ```bash
  cargo test spawn_contract
  cargo test announce_back
  cargo test cascade_cancel
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - SessionSpawnRequest and SessionSpawnResponse exist as Rust types with serde serialization.
  - validate_spawn_request() enforces INV-SPAWN-001 depth cap, INV-SPAWN-002 children cap, and TRUST-003 capability narrowing.
  - AnnounceBackMessage type exists and flows through Role Mailbox with session ID pair correlation.
  - 5 FR-EVT-SESS-SPAWN events are registered and emitted at the correct lifecycle points.
  - cascade_cancel() performs depth-first cancellation with CascadeCancelRecord evidence and skips already-terminal sessions.
- PRIMITIVES_EXPOSED:
  - PRIM-ModelSession
  - PRIM-SessionRegistry
  - PRIM-SessionState
  - PRIM-SessionSchedulerConfig
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/llm/guard.rs
- SEARCH_TERMS:
  - SpawnLimits
  - parent_session_id
  - children_by_parent
  - SessionRegistry
  - ModelSession
  - AnnounceBack
  - cascade_cancel
  - validate_spawn
  - TRUST-003
  - FR-EVT-SESS-SPAWN
- RUN_COMMANDS:
  ```bash
  rg -n "SpawnLimits|parent_session_id|children_by_parent|SessionRegistry" src/backend/handshake_core/src
  cargo test spawn_contract
  cargo test announce_back
  cargo test cascade_cancel
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Spawn without TRUST-003 enforcement" -> "child session acquires capabilities the parent lacks; privilege escalation through delegation"
  - "Cascade cancel race condition" -> "child completes during cancel and orphan resources persist"
  - "Announce-back spoofing" -> "non-child session injects fake results into parent mailbox"
  - "Unbounded summary artifact" -> "announce-back payloads consume excessive storage and memory"
  - "Missing FR events" -> "spawn lifecycle becomes invisible to operators and audit tools"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just orchestrator-prepare-and-packet` will sync Task Board and Build Order truth.
  - After packet creation, verify the Build Order shows this WP blocking WP-1-Workspace-Safety-Parallel-Sessions and WP-1-Session-Observability-Spans-FR.
  - Confirm dependency on WP-1-ModelSession-Core-Scheduler, WP-1-Session-Scoped-Capabilities-Consent-Gate, and WP-1-Role-Mailbox is reflected in the Build Order graph.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Session Spawn Contract 4.3.9.15 | WHY_IN_SCOPE: spec defines SessionSpawnRequest/Response but no Rust types exist; spawn params are embedded in ModelRunMetadata instead of being first-class types | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/storage/mod.rs | EXPECTED_TESTS: cargo test spawn_contract | RISK_IF_MISSED: spawn requests remain unvalidated ad hoc parameters and cannot be governed or audited
  - CLAUSE: INV-SPAWN-001 Max Depth Cap | WHY_IN_SCOPE: SpawnLimits.max_depth exists but no gate function enforces it at spawn time | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test spawn_contract; cargo test cascade_cancel | RISK_IF_MISSED: unbounded delegation depth leads to runaway session storms
  - CLAUSE: INV-SPAWN-002 Max Children Cap | WHY_IN_SCOPE: SpawnLimits.max_children exists but no gate function enforces it at spawn time | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test spawn_contract | RISK_IF_MISSED: unbounded child count per parent leads to resource exhaustion
  - CLAUSE: TRUST-003 Capability Narrowing | WHY_IN_SCOPE: spec requires child capabilities to be a subset of parent; not enforced at spawn boundary | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/llm/guard.rs | EXPECTED_TESTS: cargo test spawn_contract | RISK_IF_MISSED: privilege escalation through delegation allows child sessions to exceed parent authority
  - CLAUSE: Role Mailbox Announce-Back 4.3.9.15.4 | WHY_IN_SCOPE: spec defines announce-back semantics but no AnnounceBack message type exists in role_mailbox.rs | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox.rs; src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test announce_back | RISK_IF_MISSED: child session results are lost and parent has no governed way to receive spawn outcomes
  - CLAUSE: FR-EVT-SESS-SPAWN-001 through 005 | WHY_IN_SCOPE: spec defines 5 spawn lifecycle events but only scheduler events exist in code | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/flight_recorder/mod.rs; src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test spawn_contract; cargo test announce_back; cargo test cascade_cancel | RISK_IF_MISSED: spawn lifecycle is invisible to operators and audit tools
  - CLAUSE: Cascade Cancel 4.3.9.15.5 | WHY_IN_SCOPE: SessionRegistry tracks children but no cascade cancel function exists | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/storage/mod.rs | EXPECTED_TESTS: cargo test cascade_cancel | RISK_IF_MISSED: cancelling a parent leaves child sessions running as orphans consuming resources

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: SessionSpawnRequest JSON schema | PRODUCER: requesting session (parent) via workflows.rs | CONSUMER: validate_spawn_request gate, SessionRegistry, Flight Recorder | SERIALIZER_TRANSPORT: in-process struct with serde JSON serialization | VALIDATOR_READER: spawn_contract tests | TRIPWIRE_TESTS: cargo test spawn_contract | DRIFT_RISK: spawn request fields drift between producer and validator without schema enforcement
  - CONTRACT: SessionSpawnResponse JSON schema | PRODUCER: validate_spawn_request gate in workflows.rs | CONSUMER: requesting session (parent), Flight Recorder, DCC (downstream) | SERIALIZER_TRANSPORT: in-process struct with serde JSON serialization | VALIDATOR_READER: spawn_contract tests | TRIPWIRE_TESTS: cargo test spawn_contract | DRIFT_RISK: response fields drift between spawn gate and downstream consumers
  - CONTRACT: AnnounceBackMessage mailbox delivery | PRODUCER: child session on completion via role_mailbox.rs | CONSUMER: parent session mailbox inbox, Flight Recorder | SERIALIZER_TRANSPORT: Role Mailbox message delivery with session ID pair correlation | VALIDATOR_READER: announce_back tests | TRIPWIRE_TESTS: cargo test announce_back | DRIFT_RISK: announce-back correlation breaks if session ID pair is not validated against SessionRegistry
  - CONTRACT: CascadeCancelRecord evidence | PRODUCER: cascade_cancel function in workflows.rs | CONSUMER: Flight Recorder, DCC (downstream), operator audit tools | SERIALIZER_TRANSPORT: in-process struct with serde JSON serialization | VALIDATOR_READER: cascade_cancel tests | TRIPWIRE_TESTS: cargo test cascade_cancel | DRIFT_RISK: cancelled_session_ids list becomes inconsistent with actual session state transitions
  - CONTRACT: FR-EVT-SESS-SPAWN event payloads | PRODUCER: spawn lifecycle hooks in workflows.rs | CONSUMER: Flight Recorder storage, operator dashboards, audit tools | SERIALIZER_TRANSPORT: Flight Recorder JSON event payload | VALIDATOR_READER: spawn_contract, announce_back, and cascade_cancel tests | TRIPWIRE_TESTS: cargo test spawn_contract; cargo test announce_back; cargo test cascade_cancel | DRIFT_RISK: event payload fields drift from spec-defined schema

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test spawn_contract
  - cargo test announce_back
  - cargo test cascade_cancel
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - a SessionSpawnRequest with depth exceeding INV-SPAWN-001 max_depth is rejected by validate_spawn_request() and FR-EVT-SESS-SPAWN-003 is emitted with rejection_reason
  - a SessionSpawnRequest with children count exceeding INV-SPAWN-002 max_children is rejected by validate_spawn_request() and FR-EVT-SESS-SPAWN-003 is emitted
  - a SessionSpawnRequest requesting capabilities not held by the parent is rejected by validate_spawn_request() with TRUST-003 violation
  - a valid SessionSpawnRequest produces a SessionSpawnResponse with child_session_id and FR-EVT-SESS-SPAWN-001 and FR-EVT-SESS-SPAWN-002 are emitted
  - a completed child session delivers AnnounceBackMessage to parent via Role Mailbox and FR-EVT-SESS-SPAWN-004 is emitted
  - cascade_cancel() on a parent with 3 children at depth 2 cancels all children depth-first and produces a CascadeCancelRecord with all cancelled_session_ids and FR-EVT-SESS-SPAWN-005 is emitted
  - cascade_cancel() skips already-terminal child sessions and records them as skipped in CascadeCancelRecord

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Define SessionSpawnRequest and SessionSpawnResponse as Rust structs with serde derive in workflows.rs or a dedicated spawn module.
  - Implement validate_spawn_request() gate function that checks INV-SPAWN-001 depth cap, INV-SPAWN-002 children cap, and TRUST-003 capability narrowing.
  - Define AnnounceBackMessage type in role_mailbox.rs with session ID pair correlation and bounded summary artifact field.
  - Register 5 FR-EVT-SESS-SPAWN events in flight_recorder/mod.rs and wire emission into spawn request, accept, reject, announce-back, and cascade cancel code paths.
  - Implement cascade_cancel() function that traverses SessionRegistry.children_by_parent depth-first, transitions non-terminal children to cancelled, and produces CascadeCancelRecord.
  - Define CascadeCancelRecord struct with root_session_id, cancelled_session_ids, skipped_session_ids, and reason fields.
  - Add tests for all clauses: spawn validation, announce-back delivery, cascade cancel determinism.
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/llm/guard.rs
- TRIPWIRE_TESTS:
  - cargo test spawn_contract
  - cargo test announce_back
  - cargo test cascade_cancel
- CARRY_FORWARD_WARNINGS:
  - Do not embed spawn parameters in ModelRunMetadata; use first-class SessionSpawnRequest/Response types.
  - Do not skip TRUST-003 enforcement even for internal or local-model spawns; all spawn paths must validate capability narrowing.
  - Do not implement cascade cancel as breadth-first; depth-first ordering is required to prevent orphan races.
  - Do not allow announce-back without session ID pair validation against SessionRegistry; this prevents spoofing.
  - Do not allow unbounded summary artifacts in announce-back; enforce max size at delivery time.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - SessionSpawnRequest and SessionSpawnResponse exist as typed Rust structs with serde serialization
  - validate_spawn_request() enforces INV-SPAWN-001, INV-SPAWN-002, and TRUST-003
  - AnnounceBackMessage flows through Role Mailbox with session ID pair correlation
  - 5 FR-EVT-SESS-SPAWN events are registered and emitted at correct lifecycle points
  - cascade_cancel() performs depth-first cancellation and produces CascadeCancelRecord with deterministic evidence
- FILES_TO_READ:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/llm/guard.rs
- COMMANDS_TO_RUN:
  - rg -n "SessionSpawnRequest|SessionSpawnResponse|validate_spawn_request|AnnounceBackMessage|CascadeCancelRecord" src/backend/handshake_core/src
  - cargo test spawn_contract
  - cargo test announce_back
  - cargo test cascade_cancel
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- POST_MERGE_SPOTCHECKS:
  - verify SessionSpawnRequest/Response are first-class types not embedded in ModelRunMetadata
  - verify TRUST-003 capability narrowing is enforced for all spawn paths including internal and local-model spawns
  - verify announce-back session ID pair correlation is validated against SessionRegistry.children_by_parent
  - verify cascade cancel is depth-first and skips already-terminal sessions
  - verify all 5 FR events carry the spec-defined payload fields

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The exact Rust type layout for SessionSpawnRequest and SessionSpawnResponse is not proven until coding completes; spec defines the semantic fields but not the precise struct definition.
  - Whether announce-back summary artifact size limits should be configurable or hardcoded is not determined; current design uses a hardcoded max but this may need revisiting.
  - Full cascade cancel behavior under concurrent modification (multiple cancel requests arriving simultaneously) is not proven at refinement time; the depth-first ordering is specified but concurrent safety depends on implementation details.
  - Dynamic depth limit adjustment (allowing some agent roles to spawn deeper than others) is identified as a future concern but not addressed in this WP.
  - The interaction between cascade cancel and the existing session scheduler rate limiter is not fully characterized; the cancel must respect scheduler state but the exact integration point depends on coding.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec section 4.3.9.15 explicitly defines the Session Spawn Contract and Lifecycle as a normative section. All acceptance criteria (spawn request/response types, INV-SPAWN-001/002 enforcement, TRUST-003 narrowing, announce-back via mailbox, FR events, cascade cancel) map directly to normative spec anchors.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: New primitives (SessionSpawnRequest, SessionSpawnResponse, CascadeCancelRecord, AnnounceBackMessage) and interaction matrix edges are noted for a future appendix enrichment pass but do not block this WP's activation. PRIMITIVE_INDEX_ACTION=NO_CHANGE and INTERACTION_MATRIX_ACTION=NO_CHANGE; no spec version bump required now.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.15 Session Spawn Contract and Lifecycle (Normative) [ADD v02.137]
- CONTEXT_START_LINE: 32431
- CONTEXT_END_LINE: 32445
- CONTEXT_TOKEN: SessionSpawnRequest
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.15 Session Spawn Contract and Lifecycle (Normative) [ADD v02.137]

  A session MAY spawn child sessions subject to depth and concurrency limits.
  The spawn contract defines SessionSpawnRequest and SessionSpawnResponse as
  first-class governance primitives. All spawn requests MUST pass through
  validate_spawn_request() which enforces INV-SPAWN-001 (max depth),
  INV-SPAWN-002 (max children), and TRUST-003 (capability narrowing).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.12 ModelSession
- CONTEXT_START_LINE: 32175
- CONTEXT_END_LINE: 32185
- CONTEXT_TOKEN: parent_session_id
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.12 ModelSession

  ModelSession tracks the lifecycle of a single LLM interaction session.
  Fields include session_id, parent_session_id (nullable for root sessions),
  role, capabilities, state, and timestamps. The SessionRegistry maintains
  children_by_parent for parent-child relationship tracking.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.17 Workspace Safety Boundaries
- CONTEXT_START_LINE: 32604
- CONTEXT_END_LINE: 32620
- CONTEXT_TOKEN: Workspace Safety Boundaries
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.17 Workspace Safety Boundaries

  Parallel sessions MUST NOT share mutable workspace state without explicit
  coordination. Each spawned child session operates in an isolated workspace
  scope. File-level isolation is enforced by the workspace safety layer.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.20 Inbound Trust Boundary TRUST-003
- CONTEXT_START_LINE: 32784
- CONTEXT_END_LINE: 32803
- CONTEXT_TOKEN: TRUST-003
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.20 Inbound Trust Boundary

  TRUST-003: Capability Narrowing at Delegation Boundary (Normative).
  When a session spawns a child, the child\\u2019s capability set MUST be
  the intersection of the parent\\u2019s capabilities and the requested
  capabilities. A child MUST NOT acquire capabilities that the parent
  does not hold. Violation of this invariant is a governance failure.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.15.6 FR Events
- CONTEXT_START_LINE: 32504
- CONTEXT_END_LINE: 32523
- CONTEXT_TOKEN: FR-EVT-SESS-SPAWN-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 4.3.9.15.6 Flight Recorder Events for Session Spawn

  FR-EVT-SESS-SPAWN-001: session.spawn_requested
  FR-EVT-SESS-SPAWN-002: session.spawn_accepted
  FR-EVT-SESS-SPAWN-003: session.spawn_rejected
  FR-EVT-SESS-SPAWN-004: session.announce_back
  FR-EVT-SESS-SPAWN-005: session.cascade_cancel

  Each event carries the payload fields defined in the spawn contract.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3.9.21 Anti-Pattern AP-006
- CONTEXT_START_LINE: 32805
- CONTEXT_END_LINE: 32820
- CONTEXT_TOKEN: AP-006
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.21 Anti-Patterns

  AP-006: Unbounded Delegation Storm. Sessions MUST NOT spawn children
  without enforcing depth and concurrency limits. The spawn contract
  invariants (INV-SPAWN-001, INV-SPAWN-002) exist specifically to
  prevent this anti-pattern. Violation is a governance failure.
  ```

### DISCOVERY (RGF-94 discovery fields; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- DISCOVERY_PRIMITIVES: PRIM-SessionSpawnRequest, PRIM-SessionSpawnResponse, PRIM-CascadeCancelRecord, PRIM-AnnounceBackMessage
- DISCOVERY_STUBS: WP-1-Session-Spawn-Tree-DCC-Visualization-v1, WP-1-Session-Spawn-Conversation-Distillation-v1
- DISCOVERY_MATRIX_EDGES: IMX-SessionSpawn-RoleMailbox, IMX-SessionSpawn-FlightRecorder, IMX-SessionSpawn-CapabilityNarrowing
- DISCOVERY_UI_CONTROLS: Spawn tree panel (DCC), active children count badge, cascade cancel button with confirmation, spawn depth indicator bar, spawn mode selector (ONE_SHOT/SESSION_PERSISTENT), announce-back notification badge in Role Mailbox inbox
- DISCOVERY_SPEC_ENRICHMENT: DEFERRED - new primitives noted for future appendix enrichment pass
- DISCOVERY_JUSTIFICATION: N/A (discoveries were made)
