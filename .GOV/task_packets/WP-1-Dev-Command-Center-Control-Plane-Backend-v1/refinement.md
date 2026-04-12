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
- WP_ID: WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-11T03:28:22.475Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- SPEC_TARGET_SHA1: 7d6558fab2f3df70669fff6f0a6e6ef9ea395194- USER_REVIEW_STATUS: APPROVED- USER_SIGNATURE: ilja110420260528
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- NONE

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 30m
- SEARCH_SCOPE: Repo-local Master Spec, repo governance surfaces, activation-manager command surface, and current refinement template for this internal Command Center control-plane backend WP; no external search performed because the WP is repo-governed and grounded in local authority artifacts.
- REFERENCES: NONE - external prior-art research is not applicable for this internal repo-governed WP because the controlling truth is the current Master Spec plus local governance/runtime artifacts.
- PATTERNS_EXTRACTED: Preserve activation-prefixed command-surface parity with the governed runtime, keep refinement handoff file-first, and gate packet/readiness work on explicit signature plus packet hydration.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT unified product-side control-plane projections over authoritative runtime artifacts; ADAPT validated governance, workflow, session, mailbox, approval, VCS, and evidence surfaces into one DCC read/steer layer; REJECT repo-path coupling, frontend-owned orchestration state, and any second authority that tries to replace the underlying runtime systems.
- LICENSE/IP_NOTES: NONE - no third-party code, docs, or design assets are being reused in this refinement pass.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: This bounded repair pass records approval/signature and repo-local governance conclusions only; it does not identify a Master Spec text change requirement.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- Rule: if the WP is an internal repo-governed change or product-governance mirror patch already grounded in the current Master Spec plus local code/runtime truth, it is valid and often preferable to set `RESEARCH_CURRENCY_REQUIRED=NO`. Do not force unrelated or generic web research just to populate this section.
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP is an internal repo-governed activation/refinement lane already controlled by the current Master Spec plus local governance/runtime artifacts, so external freshness does not change the decision surface.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - External signal scanning is intentionally skipped here; the actionable guidance comes from local spec and runtime authority, so the main carryover is to preserve command-surface compatibility and pre-launch gating discipline.
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

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment. If no directly topical project search exists, mark this section `NOT_APPLICABLE`; do not substitute off-topic searches.
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE

### RED_TEAM_ADVISORY (security failure modes)
- NONE

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - NONE
- PRIMITIVES_EXPOSED (IDs):
  - NONE
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - NONE

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: This governance-only refinement repair does not add, expose, or modify product primitives.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: NONE

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: The current Master Spec already frames the Dev Command Center backend/control-plane surface well enough for implementation refinement; this pass does not define a new feature family.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This WP is backend-first control-plane projection/API work. Dedicated DCC UI packets remain the right place for UI appendix changes.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The required work packet, micro-task, session, approval, mailbox, governance, VCS, and evidence joins are already implied by current main-body DCC/runtime clauses and validated sibling packets.
- APPENDIX_MAINTENANCE_NOTES:
  - No spec version bump is justified by this refinement.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: No spatial or scene surface is involved in the DCC control-plane backend. | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: No fabrication or tool-path semantics are introduced. | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: No physics-model workload is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: No simulation loop or sandboxed world-state engine is added. | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: No hardware integration surface changes are required. | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: DCC is consuming orchestration state, not adding a new director engine. | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: No media composition or authoring surface is involved. | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: No visual authoring workload is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: No publication/export pipeline is changed beyond internal control-plane read models. | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: No recipe or procedural content surface applies here. | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: The DCC backend does not alter food-safety semantics. | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: No fulfillment or inventory surface is added. | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: The backend must expose recorder-linked evidence, governance overlay summaries, and durable projection records. | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: The packet is explicitly about unified query and projection surfaces for work, session, approval, mailbox, and evidence state. | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: Analytics are downstream consumers of the control plane, not a new implementation surface here. | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: No dataset-ingestion or wrangling capability is added. | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: Projection durability, correlation contracts, and summary joins must stay behind product-owned storage and query boundaries. | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: Approval state, capability posture, governance overlay execution, and no-bypass control-plane rules are sovereignty concerns. | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: Guidance copy may appear later in DCC UI work, but no standalone guide engine behavior is added here. | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: The packet requires compact-summary-first payloads suitable for local-small-model routing and operator views. | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: Stable projection identifiers, workflow/session correlation, and VCS/worktree binding state are explicit scope items. | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: Existing capability and governed execution surfaces are reused rather than widened in this packet. | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). Pillar feature definition and technical implementation MUST be derived from the current Master Spec. If unknown or underspecified, write UNKNOWN and create stubs or spec updates instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Recorder-linked evidence joins and operator-visible evidence summaries are explicit backend scope items. | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: No calendar behavior is part of this packet. | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: No editor shell work belongs to this backend packet. | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: No document editor surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: No spreadsheet surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Stable projection identifiers and workflow/session correlation must remain aligned with canonical Locus-backed work tracking. | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: No media or artifact-timeline surface is in scope. | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Work-packet state is consumed through the primary Command Center and runtime-alignment rows in this packet; the standalone product work-packet pillar is not widened independently here. | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Task-board freshness is consumed through the primary Command Center and runtime-alignment rows in this packet; the standalone product task-board pillar is not widened independently here. | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: Micro-task state is part of the control-plane projection surface called out by the stub. | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: This is the primary pillar for the packet: the backend projection and API layer that powers Dev Command Center. | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: FEMS is not the authority for DCC control-plane state in this packet. | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: Session scheduler state, approval/capability posture, governance overlay execution, and worktree binding are runtime-backed surfaces. | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: This packet consumes existing governed artifacts but does not add a new spec-to-prompt flow. | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: Projection identifiers, summary payloads, and query surfaces must remain SQLite-now and Postgres-ready behind storage boundaries. | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: Compact-summary-first payloads for local-small-model routing are part of the packet intent. | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: No Stage surface is introduced. | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: No Studio surface is introduced. | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: No Atelier/Lens surface is introduced. | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: No distillation or LoRA training surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: No ACE-specific runtime surface is widened here. | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: The packet improves structured retrieval posture indirectly, but it does not implement a dedicated RAG surface. | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Derive pillar slices and subfeatures from the current Master Spec; do not invent pillar semantics from memory. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: Evidence and replay joins | SUBFEATURES: recorder-linked evidence refs, latest event summaries, and traceability joins across work, session, and governance state | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: DCC should consume existing recorder evidence as first-class control-plane inputs instead of forcing timeline replay.
  - PILLAR: Locus | CAPABILITY_SLICE: Canonical identity and correlation keys | SUBFEATURES: stable work_packet_id, micro_task_id, task_board_id, workflow_run_id, and model_session_id carry-through across every projection | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus-backed identifiers remain authoritative while DCC exposes readable projections over them.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Activation and steering detail projection | SUBFEATURES: status, blockers, approval posture, active session or worktree bindings, and evidence refs | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.version, engine.sovereign, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Work Packet steering stays governed through existing backend artifacts; DCC only unifies the operator-facing control plane.
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Freshness and ready-query planning surface | SUBFEATURES: sync freshness, ready-query results, queue reasons, and derived board summaries keyed by stable ids | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Human-readable board views remain mirrors over authoritative backend state.
  - PILLAR: MicroTask | CAPABILITY_SLICE: Micro-task coordination summary surface | SUBFEATURES: hard-gate state, iteration progress, mailbox-linked waits, and active session occupancy | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: DCC needs durable micro-task summaries without relying on executor-local loops or transcript replay.
  - PILLAR: Command Center | CAPABILITY_SLICE: Unified control-plane API and read model | SUBFEATURES: work, session, approval, mailbox, governance, VCS, and evidence summary views | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.librarian, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the core packet outcome: one product-side control plane over existing authorities.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Session and governed-action runtime visibility | SUBFEATURES: workflow runs, session scheduler state, capability posture, governance execution state, and worktree bindings | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Runtime state must stay inspectable and steerable through governed backend seams rather than drawer-local UI state.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Durable control-plane contracts | SUBFEATURES: storage-bound read models, stable identifiers, migration-safe payloads, and no hidden UI-only authority | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The DCC backend must remain SQLite-now and Postgres-ready behind product storage boundaries.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact-summary-first routing payloads | SUBFEATURES: concise readiness, blockers, queue reasons, and status cards for local-small-model routing and operator views | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.archivist | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The packet should reduce transcript or Markdown dependency for routing and triage decisions.
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Unified DCC control-plane projection | JobModel: WORKFLOW | Workflow: dcc_control_plane_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Unify authoritative backend artifacts into structured read and steer summaries instead of exposing fragmented subsystem views.
  - Capability: Work Packet and Task Board readiness query surface | JobModel: UI_ACTION | Workflow: dcc_work_query_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Expose tracked status, ready-query state, freshness, blockers, and queue reasons by stable identifiers.
  - Capability: Session scheduler and worktree binding state | JobModel: WORKFLOW | Workflow: dcc_session_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Project model-session occupancy, workflow-linked bindings, and workspace/worktree posture without relying on ad hoc git or tab state.
  - Capability: Approval and capability posture summary | JobModel: WORKFLOW | Workflow: dcc_capability_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Surface effective capability state and approval posture while keeping policy authority in existing backend capability systems.
  - Capability: Role Mailbox coordination summary | JobModel: WORKFLOW | Workflow: dcc_role_mailbox_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Present expected-response, wait, handoff, and triage summaries as structured control-plane data instead of mailbox-only drilldown.
  - Capability: Governance overlay and evidence joins | JobModel: WORKFLOW | Workflow: dcc_governance_evidence_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001, FR-EVT-GOV-WP-001, FR-EVT-GOV-CHECK-001, FR-EVT-GOV-CHECK-002, FR-EVT-GOV-CHECK-003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse governance mirror and check-runner evidence surfaces rather than inventing a second governance authority.
  - Capability: Workspace runtime and VCS posture surface | JobModel: UI_ACTION | Workflow: dcc_workspace_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Project workspace readiness, worktree binding, and promotion posture through governed backend seams rather than raw command output.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Local-only matrix scan. The highest-ROI combinations are already clear from current spec and validated sibling packets: work packet plus task board plus session joins, governance plus evidence correlation, mailbox triage plus queue reasons, and compact summary payloads for local-small-model routing.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE. The current Dev Command Center control-plane, work-orchestration, mailbox-triage, and governance sweep clauses already name the combinations this packet needs.
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: No new Appendix 12.6 edge is required to unblock this DCC backend implementation because current main-body DCC sweeps already cover the relevant joins.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. For internal/product-governance mirror work, it is valid to mark this section `NOT_APPLICABLE` when no directly topical external combo research is needed. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: External combo research is not needed for this internal governance WP because local spec/runtime evidence is the authoritative source.
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
  - Combo: Locus Correlation Spine | Pillars: Locus | Mechanical: engine.version, engine.context | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Stable identifiers must connect work packets, micro-tasks, task-board rows, workflow runs, and model sessions across all DCC projections.
  - Combo: Work Packet Readiness Surface | Pillars: Command Center | Mechanical: engine.version, engine.sovereign, engine.context | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: DCC needs packet status, blockers, approvals, bindings, and evidence in one backend surface.
  - Combo: Task Board Freshness and Ready Query | Pillars: Locus | Mechanical: engine.archivist, engine.librarian | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Freshness, ready-query state, and queue reasons should remain visible without kanban-only inference.
  - Combo: MicroTask Coordination Summary | Pillars: MicroTask | Mechanical: engine.context, engine.sovereign | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Hard-gate state, iteration progress, and mailbox-linked waits must be queryable without executor-local replay.
  - Combo: Command Center Unified Control Plane | Pillars: Command Center | Mechanical: engine.context, engine.librarian, engine.sovereign | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: The packet's primary goal is one product-side control plane over existing backend authorities.
  - Combo: Execution Runtime Session Steering | Pillars: Execution / Job Runtime | Mechanical: engine.version, engine.sovereign, engine.dba | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Session scheduler state, active bindings, and governed actions need one runtime-backed projection surface.
  - Combo: Durable Projection Contracts | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.version | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Control-plane payloads and identifiers must stay migration-safe behind storage boundaries.
  - Combo: Compact Summary Routing | Pillars: LLM-friendly data | Mechanical: engine.context, engine.archivist | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Local-small-model routing should resolve from bounded structured summaries before long Markdown or transcripts.
  - Combo: Evidence and Replay Joins | Pillars: Flight Recorder | Mechanical: engine.archivist, engine.context | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Recorder-linked evidence needs to be first-class DCC input instead of a separate drilldown-only surface.
  - Combo: Mailbox Triage in the Command Center | Pillars: Command Center, MicroTask | Mechanical: engine.context, engine.sovereign | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Mailbox waits, triage posture, and coordination state should project into DCC without mailbox-only views.
  - Combo: Governance Evidence in the Command Center | Pillars: Command Center, Flight Recorder | Mechanical: engine.archivist, engine.sovereign | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Governance overlay execution and evidence joins should be visible through the same control plane as work and session state.
  - Combo: Session Steering with Operator Views | Pillars: Command Center, Execution / Job Runtime | Mechanical: engine.version, engine.sovereign | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Operators need active-session, worktree-binding, and steering-legality state without switching subsystems.
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations resolve inside this packet by reusing existing validated backend foundations rather than creating new stubs.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: Target stub, current task board and build order, DCC-related Master Spec clauses, validated sibling governance and session packets, and current `../handshake_main/src/backend/handshake_core` runtime/governance/locus/mailbox/workspace code.
- MATCHED_STUBS:
  - Artifact: WP-1-Dev-Command-Center-Control-Plane-Backend-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: N/A | Matrix: N/A | UI: N/A | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: This refinement hydrates the existing stub directly; no duplicate stub should be created.
  - Artifact: WP-1-Consent-Audit-Projection-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Consent-audit projection is a downstream consumer-facing follow-on packet, not the backend unification layer itself.
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Role-Mailbox-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: PARTIAL | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Role Mailbox artifacts and API surfaces already exist and should be projected, not duplicated.
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: PARTIAL | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Structured collaboration artifacts and compact-summary posture already exist and should anchor DCC payload design.
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: PARTIAL | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Shared schema and profile-extension boundaries are prerequisites for generic DCC viewers and summary payloads.
  - Artifact: WP-1-Workflow-Projection-Correlation-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: PARTIAL | CodeReality: IMPLEMENTED | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: Stable workflow/task-board/model-session correlation exists, but DCC still needs a unified control-plane read model over it.
  - Artifact: WP-1-Product-Governance-Artifact-Registry-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Governance artifact registry surfaces already exist and should feed DCC governance views.
  - Artifact: WP-1-Product-Governance-Check-Runner-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Check-runner results and evidence should be joined into DCC, not reimplemented.
  - Artifact: WP-1-Session-Spawn-Contract-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Session spawn and session-registry contracts are upstream control-plane inputs.
  - Artifact: WP-1-Session-Crash-Recovery-Checkpointing-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Recovery and checkpoint state should surface through DCC rather than becoming a separate subsystem.
  - Artifact: WP-1-Session-Observability-Spans-FR-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Session observability and FR evidence are prerequisites for DCC session and evidence views.
  - Artifact: WP-1-Workspace-Safety-Parallel-Sessions-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Workspace safety and multi-session posture already exist and should feed DCC worktree/session state.
  - Artifact: WP-1-Governance-Workflow-Mirror-v1 | BoardStatus: OUTDATED_ONLY | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Governance workflow mirror remains a key backend dependency even though the earlier packet is no longer current.
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs | Artifact: WP-1-Product-Governance-Artifact-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: Governance runtime surfaces exist, but they are not yet unified into a DCC-specific control-plane view.
  - Path: ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs | Artifact: WP-1-Role-Mailbox-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: Mailbox coordination artifacts already exist and should be projected into DCC rather than remodelled.
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Workflow-Projection-Correlation-v1 | Covers: execution | Verdict: PARTIAL | Notes: Workflow and session state exist, but DCC-facing joins remain fragmented across backend seams.
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs | Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: Task-board and Locus query substrate already provide stable identifiers and freshness-backed projection inputs.
  - Path: ../handshake_main/src/backend/handshake_core/src/capabilities.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: Capability posture exists, but it is not yet surfaced as one coherent DCC control-plane summary.
  - Path: ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: Workspace/worktree APIs exist, but DCC-facing worktree binding and readiness summaries remain incomplete.
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: Existing validated backend surfaces are necessary inputs, but no current packet unifies them into one DCC control-plane projection layer; this packet must expand over those partial foundations rather than merely reuse one prior artifact.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet is backend-first control-plane projection and API work. Dedicated DCC UI packets remain the correct place for concrete surface/control design.
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
- GUI_ADVICE_REASON_NO: This packet is backend-first control-plane/API work; concrete GUI implementation guidance belongs to the downstream DCC UI packets once the backend projection layer exists.
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
- REQUESTOR: OPERATOR
- AGENT_ID: ORCHESTRATOR
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.160]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Governance-Workflow-Mirror, WP-1-Session-Spawn-Contract, WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Session-Crash-Recovery-Checkpointing, WP-1-Session-Observability-Spans-FR, WP-1-Locus-Phase1-QueryContract-Autosync, WP-1-Role-Mailbox, WP-1-Workflow-Projection-Correlation, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-MVP, WP-1-Dev-Command-Center-Layout-Projection-Registry, WP-1-Dev-Command-Center-Structured-Artifact-Viewer, WP-1-Consent-Audit-Projection
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.180.md 10.11 Dev Command Center control-plane, work-orchestration, and collaboration-state backend projections
- WHAT: Build the backend projection and API layer that unifies authoritative work, session, approval, mailbox, governance, VCS, and evidence state into one Dev Command Center control plane.
- WHY: Downstream DCC UI packets need one governed product-side backend surface; without it, Dev Command Center would remain a thin shell over disconnected subsystem APIs and local-only state.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
- OUT_OF_SCOPE:
  - Full Dev Command Center frontend shell and layout work
  - Monaco editor shell, terminal shell polish, and typed-viewer UX
  - New repo-governance files becoming runtime authority
  - Consent-audit viewer or projection follow-on UI packet work
- TEST_PLAN:
  ```bash
  rg -n "runtime_governance|role_mailbox|task_board|capabil|workspace|model_session|workflow_run_id|work_packet_id|micro_task|ready-query" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml model_session_scheduler -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml terminal_session -- --nocapture
  ```
- DONE_MEANS:
  - The backend exposes one coherent DCC projection surface for work, session, approval, mailbox, governance, VCS, and evidence state.
  - No projection path becomes a second authority; all state is sourced from existing backend artifacts and governed operations.
  - Stable work_packet_id, micro_task_id, task_board_id, workflow_run_id, and model_session_id values survive across every DCC-facing projection.
  - Compact-summary-first payloads exist for local-small-model routing and operator views without transcript or Markdown replay.
  - Tests prove session or worktree binding, queue reasons, mailbox waits, governance evidence joins, and readiness summaries round-trip through the backend surface.
- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.180.md
  - .GOV/task_packets/stubs/WP-1-Dev-Command-Center-Control-Plane-Backend-v1.md
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
- SEARCH_TERMS:
  - runtime_governance
  - role_mailbox
  - task_board
  - ready-query
  - model_session_id
  - workflow_run_id
  - work_packet_id
  - micro_task
  - workspace_safety
  - capabilities
- RUN_COMMANDS:
  ```bash
  rg -n "runtime_governance|role_mailbox|task_board|capabil|workspace|model_session|workflow_run_id|work_packet_id|micro_task|ready-query" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml model_session_scheduler -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor -- --nocapture
  ```
- RISK_MAP:
  - "DCC becomes a second authority instead of a projection layer" -> "state drift and unsafe steering"
  - "session or worktree bindings do not round-trip by stable ids" -> "operators route work against stale or ambiguous state"
  - "mailbox, governance, and evidence joins stay siloed" -> "DCC triage remains fragmented and transcript-heavy"
  - "compact summary payloads are missing or incomplete" -> "local-small-model routing still depends on long Markdown or chat replay"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Activation will move this work packet out of stub state. Sync BUILD_ORDER when the official packet is created so the active artifact path and live status stay aligned.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: 10.11 [ADD v02.160] Dev Command Center control-plane state | WHY_IN_SCOPE: This packet exists to project workflow runs, artificial intelligence job state, capability posture, session state, and work packet or worktree bindings through one governed backend surface | EXPECTED_CODE_SURFACES: `runtime_governance.rs`, `workflows.rs`, `capabilities.rs`, `api/workspaces.rs` | EXPECTED_TESTS: targeted DCC projection tests plus runtime_governance and session scheduler cargo tests | RISK_IF_MISSED: DCC remains a shell over fragmented subsystem state
  - CLAUSE: 10.11 [ADD v02.162] Dev Command Center work-orchestration state | WHY_IN_SCOPE: The backend must surface tracked Work Packet status, Task Board freshness, ready-query results, Micro-Task summaries, and parallel model session occupancy | EXPECTED_CODE_SURFACES: `locus/task_board.rs`, `workflows.rs`, `runtime_governance.rs`, `terminal/session.rs` | EXPECTED_TESTS: task-board and micro-task projection tests plus scheduler-related cargo tests | RISK_IF_MISSED: routing and planning continue to depend on kanban-only or transcript-derived state
  - CLAUSE: 10.11 [ADD v02.166] Dev Command Center collaboration state | WHY_IN_SCOPE: The backend must project structured Work Packet records, Micro-Task definitions, Task Board rows, note timelines, and Role Mailbox triage state | EXPECTED_CODE_SURFACES: `role_mailbox.rs`, `api/role_mailbox.rs`, `locus/task_board.rs`, `workflows.rs` | EXPECTED_TESTS: role-mailbox and structured-collaboration projection tests | RISK_IF_MISSED: DCC triage and handoff remain prose-heavy and non-deterministic
  - CLAUSE: 10.11 [ADD v02.171] workflow_state_family and queue_reason_code visibility | WHY_IN_SCOPE: Local-small-model routing and operator triage need explicit queue reasons and allowed actions instead of lane-name heuristics | EXPECTED_CODE_SURFACES: `workflows.rs`, `locus/types.rs`, `locus/task_board.rs` | EXPECTED_TESTS: ready-query and queue-reason projection tests | RISK_IF_MISSED: control-plane routing semantics collapse back into ad hoc labels
  - CLAUSE: 6.3 compact summary contract for DCC and Role Mailbox triage | WHY_IN_SCOPE: This packet must default DCC routing and triage to compact summaries before canonical detail or Markdown mirrors | EXPECTED_CODE_SURFACES: `locus/types.rs`, `role_mailbox.rs`, DCC projection serializers in runtime or workflow code | EXPECTED_TESTS: contract-shape tests proving summary-first payloads and stable ids | RISK_IF_MISSED: small-model routing and operator views stay expensive and brittle

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: Unified DCC control-plane summary payload | PRODUCER: runtime governance and workflow projection layer | CONSUMER: DCC UI packets, local-small-model routing, operator control views | SERIALIZER_TRANSPORT: structured backend projection payloads keyed by stable ids | VALIDATOR_READER: DCC projection tests and packet validator review | TRIPWIRE_TESTS: projection tests proving coherent work/session/mailbox/governance summaries | DRIFT_RISK: DCC surfaces show mutually inconsistent subsystem state
  - CONTRACT: Work Packet plus Task Board readiness summary | PRODUCER: Locus-backed projection layer | CONSUMER: DCC queue views, ready-work routing, operator planning surfaces | SERIALIZER_TRANSPORT: structured summary rows with work_packet_id, task_board_id, queue reasons, and freshness | VALIDATOR_READER: task-board and ready-query tests | TRIPWIRE_TESTS: stable-id projection, freshness visibility, ready-query determinism | DRIFT_RISK: ready work becomes UI-derived instead of backend-backed
  - CONTRACT: Session and worktree binding summary | PRODUCER: workflow and workspace/session surfaces | CONSUMER: DCC session panel, steering actions, operator recovery flows | SERIALIZER_TRANSPORT: structured runtime summary with model_session_id, workflow_run_id, workspace/worktree refs, and capability posture | VALIDATOR_READER: session scheduler and workspace/runtime tests | TRIPWIRE_TESTS: session/worktree binding coherence and legal-steering projection checks | DRIFT_RISK: work appears bound to the wrong session or workspace
  - CONTRACT: Role Mailbox triage projection | PRODUCER: role mailbox artifact and API surfaces | CONSUMER: DCC inbox/coordination views, local-small-model routing, operator follow-up | SERIALIZER_TRANSPORT: structured mailbox summary rows with expected response, wait posture, evidence refs, and linkage ids | VALIDATOR_READER: role_mailbox tests and DCC projection inspection | TRIPWIRE_TESTS: mailbox wait reasons and handoff posture survive projection without transcript replay | DRIFT_RISK: DCC triage diverges from mailbox authority
  - CONTRACT: Governance and evidence join summary | PRODUCER: governance runtime, artifact registry, check runner, and recorder-backed evidence surfaces | CONSUMER: DCC governance controls, approval posture, evidence drilldown links | SERIALIZER_TRANSPORT: structured summary with verdict, evidence refs, and check/run ids | VALIDATOR_READER: runtime_governance and governance_check_runner tests | TRIPWIRE_TESTS: governance result and evidence refs remain aligned across projections | DRIFT_RISK: operators act on governance state unsupported by authoritative evidence

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_control_plane_projection_keeps_stable_ids -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_ready_query_projection_is_backend_backed -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_mailbox_projection_preserves_wait_reasons -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_session_binding_projection_matches_runtime_state -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_governance_evidence_join_is_consistent -- --exact`
- CANONICAL_CONTRACT_EXAMPLES:
  - Example DCC summary payload keyed by `work_packet_id`, `task_board_id`, `workflow_run_id`, and `model_session_id` for one active work packet.
  - Example ready-query summary row showing queue reason, freshness, blockers, and active session binding without Markdown parsing.
  - Example mailbox-triage projection row preserving expected response, wait posture, evidence refs, and linked work ids.
  - Example governance/evidence summary row linking verdict, check refs, and recorder-backed evidence identifiers.

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Extend the existing runtime/workflow projection surfaces so DCC can read one coherent summary of work, session, approval, mailbox, governance, VCS, and evidence state.
  - Reuse current Locus/task-board/workflow/session/mailbox/governance artifacts; do not introduce a second authority or DCC-only truth store.
  - Add compact-summary-first serializers and stable-id joins before any DCC-specific API or steering endpoints.
  - Add or extend tests that prove queue reasons, ready-query state, mailbox waits, governance evidence joins, and session/worktree bindings round-trip coherently.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
- TRIPWIRE_TESTS:
  - `dcc_control_plane_projection_keeps_stable_ids`
  - `dcc_ready_query_projection_is_backend_backed`
  - `dcc_mailbox_projection_preserves_wait_reasons`
  - `dcc_session_binding_projection_matches_runtime_state`
  - `dcc_governance_evidence_join_is_consistent`
- CARRY_FORWARD_WARNINGS:
  - Do not let DCC become a second authority over work, session, mailbox, or governance state.
  - Do not rely on kanban ordering, transcript replay, or Markdown mirrors as the only routing surface.
  - Do not bypass existing runtime, storage, or governance boundaries to make the projection easier.
  - Keep stable ids and compact summaries first-class so local-small-model routing remains cheap and deterministic.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - 10.11 [ADD v02.160] Dev Command Center control-plane state
  - 10.11 [ADD v02.162] Dev Command Center work-orchestration state
  - 10.11 [ADD v02.166] Dev Command Center collaboration state
  - 10.11 [ADD v02.171] queue_reason_code and allowed_action visibility
  - 6.3 compact summary contract for DCC, Task Board, and Role Mailbox triage
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
- COMMANDS_TO_RUN:
  - `rg -n "runtime_governance|role_mailbox|task_board|capabil|workspace|model_session|workflow_run_id|work_packet_id|micro_task|ready-query" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml model_session_scheduler -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor -- --nocapture`
- POST_MERGE_SPOTCHECKS:
  - Verify DCC-facing summaries preserve stable ids across work, session, mailbox, and governance state.
  - Verify ready-query, freshness, mailbox waits, and governance evidence remain backend-backed rather than drawer-local.
  - Verify worktree or session steering views agree with runtime state and do not imply authority outside governed operations.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The exact final DCC summary payload shape and API boundary are not proven yet; only the semantic requirements and backend source surfaces are fixed here.
  - The final division between runtime-governance projection helpers and workflow-specific projection services is not proven yet.
  - Downstream frontend layout, typed-viewer behavior, and operator control affordances remain unproven until the UI packets consume this backend surface.

### DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (This refinement composes existing runtime, Locus, mailbox, governance, workspace, and evidence surfaces instead of introducing new Appendix 12 primitive IDs.)
- DISCOVERY_STUBS: NONE_CREATED (No new stub is warranted; this packet hydrates the existing DCC backend stub and keeps downstream DCC UI/control packets separate.)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (Current Dev Command Center control-plane, work-orchestration, collaboration, and compact-summary clauses already cover the interaction edges this packet needs.)
- DISCOVERY_UI_CONTROLS: NONE_APPLICABLE (Concrete UI controls belong to the downstream DCC UI packets; this packet is backend-first.)
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED (The current spec already names the DCC control-plane, work-orchestration, collaboration, compact-summary, and stable-id requirements required here.)
- DISCOVERY_JUSTIFICATION: This refinement still delivered value by collapsing the packet onto the correct backend unification layer, proving reuse boundaries against validated sibling packets, and hydrating coder and validator proof obligations without requiring a spec bump.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Master Spec explicitly names Dev Command Center control-plane, work-orchestration, collaboration-state, compact-summary, and stable-id projection requirements. This refinement maps those clauses to concrete backend surfaces and proof obligations.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The current Master Spec already contains the normative DCC backend clauses this packet needs: control-plane state, work-orchestration state, collaboration-state projection, compact-summary rules, and stable-id routing semantics. The remaining work is backend implementation and composition, not spec repair.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 10.11 Dev Command Center control-plane state [ADD v02.160]
- CONTEXT_START_LINE: 60562
- CONTEXT_END_LINE: 60566
- CONTEXT_TOKEN: model-session scheduler snapshots
- EXCERPT_ASCII_ESCAPED:
  ```text
  [ADD v02.159] Within this umbrella, Operator Consoles are the specialized evidence and diagnostics cluster. Dev Command Center owns control, projection, orchestration, approval, and worktree/session binding state; Operator Consoles own Problems, Jobs, Timeline, and Evidence drilldown surfaces.

  [ADD v02.160] Dev Command Center control-plane state MUST project workflow runs, artificial intelligence job state, model-session scheduler snapshots, effective capability state, approval decisions, and work packet or worktree or session bindings from authoritative backend artifacts. It MAY steer or approve only through governed backend operations and MUST NOT mutate long-lived orchestration state solely through user-interface-local caches.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 10.11 Dev Command Center work-orchestration state [ADD v02.162]
- CONTEXT_START_LINE: 60568
- CONTEXT_END_LINE: 60570
- CONTEXT_TOKEN: ready-query results
- EXCERPT_ASCII_ESCAPED:
  ```text
  [ADD v02.162] Dev Command Center work-orchestration state MUST project tracked Work Packet status, Task Board freshness, ready-query results, Micro-Task summaries, hard-gate state, workflow-linked work packet activation, and parallel model session occupancy from authoritative backend artifacts. It MUST route or steer work only through governed backend operations and MUST preserve stable work-packet, micro-task, workflow-run, and model-session identifiers across every projection surface.

  [ADD v02.163] Dev Command Center planning-and-coordination state MUST also project Task Board entries, Work Packet bindings, Spec Session Log continuity, workflow-linked activation, ready-work selection, and parallel-session occupancy from authoritative backend artifacts. It MUST NOT infer work authority from kanban-only ordering, mailbox-only coordination, or packet-local prose summaries.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 10.11 Dev Command Center collaboration state [ADD v02.166]
- CONTEXT_START_LINE: 60576
- CONTEXT_END_LINE: 60580
- CONTEXT_TOKEN: Role Mailbox triage state
- EXCERPT_ASCII_ESCAPED:
  ```text
  [ADD v02.166] Dev Command Center collaboration state MUST also project structured Work Packet records, structured Micro-Task definitions, Task Board projection rows, append-only note timelines, and Role Mailbox triage state from authoritative backend artifacts. It MUST render typed fields before raw Markdown or raw JSON blobs and MUST NOT make prose-only summaries the only operator surface for routing, replay, or handoff decisions.

  [ADD v02.167] Dev Command Center board and queue state MUST be derived from the same canonical structured artifacts that back Work Packet, Micro-Task, Task Board, and Role Mailbox records. Kanban, Jira-like, list, queue, roadmap, or timeline layouts MAY vary by view configuration, but they MUST NOT create a competing source of truth for status, scope, or routing.

  [ADD v02.168] Dev Command Center typed viewers and derived layouts MUST understand the shared structured-collaboration base envelope, project-profile extension metadata, compact summaries, and mirror-state semantics. Operators MUST be able to distinguish base-envelope fields from profile-specific extensions without opening raw files or reconstructing state from prose.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 6.3 compact summary contract for DCC, Task Board, and Role Mailbox triage
- CONTEXT_START_LINE: 6878
- CONTEXT_END_LINE: 6884
- CONTEXT_TOKEN: compact summary contract
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Dev Command Center, Task Board derived layouts, Role Mailbox triage, and local-small-model planning flows SHOULD default to the compact summary contract first and load canonical detail records or Markdown sidecars only on demand.

  - Task Board projection rows SHOULD be serialized as `record_kind=task_board_entry` records that reuse the same base envelope and add only board-specific fields such as `task_board_id`, `work_packet_id`, `lane_id`, `display_order`, and optional `view_ids`.
  - Role Mailbox exports SHOULD reuse the same base envelope for thread indexes and append-only thread lines. Message-level records SHOULD add only mailbox-specific fields such as `thread_id`, `message_type`, `from_role`, `to_roles`, `expected_response`, and `expires_at`.
  - When a collaboration artifact supports both canonical detail and compact summary representations, both records MUST share the same `record_id`, `project_profile_kind`, and authoritative references so deterministic joins remain possible without transcript reconstruction or Markdown parsing.
  ```
