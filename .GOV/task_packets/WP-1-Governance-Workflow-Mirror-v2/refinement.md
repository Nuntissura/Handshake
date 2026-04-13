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
- WP_ID: WP-1-Governance-Workflow-Mirror-v2
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-12T02:44:14.8298572Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- SPEC_TARGET_SHA1: 7d6558fab2f3df70669fff6f0a6e6ef9ea395194
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja120420260458
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Governance-Workflow-Mirror-v2
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Current `main` already implements the activation half of the workflow mirror: `gov_work_packet_activated` is emitted and validated, and `traceability_registry_ref` already points at the shared runtime registry rather than the per-WP activation slice.
- Current `main` already exposes a workflow-facing Spec Session Log seam in `role_mailbox.rs`; `v2` must reuse that seam instead of reopening mailbox-owned logging design.
- Current `main` still lacks the gate-transition half of the old WP: no `GovGateTransition` Flight Recorder event and no `gov_gate_transition` payload validator are present.
- Current `main` still lacks the structured workflow-mirror gate/activation summary types that existed on the historical `v1` branch.
- Current `main` still lacks the projection/wiring layer that syncs gate-summary and activation-summary data into workflow/task-board-facing structures.
- Direct whole-file replay from the historical `v1` branch is unsafe because current `main` already contains later runtime behavior, including `SessionSpawnRequested`, `SessionCheckpointCreated`, and `WorkspaceIsolationDenied`.
- Locus remains the canonical work-tracking identity/state surface. `v2` must extend current projections with overlay facts instead of widening base Locus families into a second authority.
- The hard repo/runtime boundary still forbids any product path from reading or writing `/.GOV/`. The viable resolution remains a narrow runtime-owned workflow-mirror slice only for software-delivery governance state.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: Local-first only. Reviewed the current Master Spec, local governance docs, task board, traceability registry, target `v2` stub, current `../handshake_main` backend code, and the historical `../wtc-workflow-mirror-v1` branch/worktree strictly as source material for the missing slice. Inspected only the directly analogous artifact-registry and check-runner refinements.
- REFERENCES: NONE (external prior art is honestly not needed; the authoritative inputs are local spec/docs/code)
- PATTERNS_EXTRACTED:
  - Keep Locus as canonical work-tracking authority and key overlay artifacts by stable Locus/workflow ids.
  - Keep runtime governance state under `.handshake/gov/`; product runtime must not read/write `/.GOV/`.
  - Preserve already-contained current-main behavior and port only branch-only workflow-mirror pieces; do not replay stale whole-file versions onto newer runtime code.
  - Reuse the additive overlay model already defined for imported governance artifacts and governed checks.
  - Reuse existing task-board/work-packet projection posture: structured projections first, Markdown mirrors second.
  - Reuse the existing workflow-facing Spec Session Log append/query seam that current `main` already exposes in `role_mailbox.rs`.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the narrow runtime workflow-mirror overlay; ADAPT current projection and session-log seams; REJECT Locus widening, broad repo-mirror scope, and direct `/.GOV/` IO.
  - ADOPT: a narrowly scoped durable workflow-mirror layer on top of current runtime-governance, artifact-registry, check-runner, workflow-projection, and role-mailbox foundations.
  - ADOPT: per-WP validator gate artifacts under `.handshake/gov/validator_gates/`, because the spec already names that runtime path.
  - ADAPT: existing task-board/work-packet projection patterns so the workflow mirror extends the current runtime governance surface instead of creating a second task-board system.
  - ADAPT: existing `spec_session_log_entries` persistence into a workflow-facing append/query seam.
  - REJECT: widening base Locus schemas for overlay-only workflow-mirror facts.
  - REJECT: a broad repo-governance mirror inside runtime.
  - REJECT: any runtime implementation that reads `/.GOV/` directly.
- LICENSE/IP_NOTES: Local repository evidence only. No external code reuse or third-party licensing constraints were introduced.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The current Master Spec already defines the hard boundary, Locus linkage, Spec Session Log rules, CheckRunner contract, governance snapshot shape, and FR event contracts. This WP is implementation refinement only.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP is a strictly internal mechanical/runtime-governance mirror decision bounded by the current Master Spec and local product code.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE. Local authoritative evidence was sufficient and external current-signal scanning would not materially improve this WP.
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
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- `FR-EVT-GOV-GATES-001` is the missing runtime slice on current `main`; `v2` must add `GovGateTransition`, payload validation, and deterministic emission on gate transitions.
- `FR-EVT-GOV-WP-001` already exists on current `main` and must remain intact while `v2` lands the missing gate-transition parity.
- `FR-EVT-GOV-CHECK-001`, `FR-EVT-GOV-CHECK-002`, and `FR-EVT-GOV-CHECK-003` remain owned by CheckRunner. The workflow mirror should reference their evidence/check ids, not redefine them.
- Workflow-mirror Spec Session Log entries should carry the same `spec_id`, `task_board_id`, `work_packet_id`, `workflow_run_id`, and `model_session_id` already preserved by workflow/task-board projections.
- No new FR event ids are required in this refinement.

### RED_TEAM_ADVISORY (security failure modes)
- Treat runtime workflow mirror records as overlay facts keyed to canonical Locus/workflow ids. Do not permit the mirror to mint a second WP identity namespace.
- Enforce the repo/runtime boundary at code level: runtime mirror code must never read `/.GOV/` paths directly.
- Store hash/ref/evidence identifiers, not raw role-mailbox bodies, secrets, or unbounded logs, in workflow mirror artifacts.
- Make gate writes idempotent enough to resist replay collisions from parallel WP activity.
- Do not allow imported check descriptors or results to mutate base structured-collaboration records outside declared overlay fields.

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
  - No Appendix 12 primitive ids are added or revised in this refinement. The WP composes existing surfaces: Locus workflow/task-board records, runtime governance paths, GovernanceArtifactRegistry, CheckRunner, SpecSessionLogEntry, and FR governance events.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: No new primitive ids or appendix-level primitive reclassification are required.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Reuse current Locus/task-board/work-packet/governance-registry/check-runner surfaces.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No orphan primitive emerged that justifies a new stub or appendix action.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: The Master Spec main body already covers this workflow-mirror surface well enough for implementation refinement.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This WP is backend-heavy runtime-governance work. UI implications are recorded below for packet guidance, but no Master Spec UI appendix update is warranted.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Required interactions are already implied by the current main-body governance clauses and validated sibling WPs.
- APPENDIX_MAINTENANCE_NOTES:
  - No spec version bump is justified by this refinement.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: No spatial runtime surface is involved. | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: No fabrication/tool-path semantics are involved. | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: No physics-model workload is introduced. | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: No simulation loop is added. | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: No hardware integration change is required. | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: No scene-orchestration surface is affected. | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: No composition/media-authoring surface is involved. | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: No visual authoring scope. | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: No publishing/export pipeline change beyond internal governance state. | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: No recipe/procedural content workload applies. | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: Governance mirror does not alter food-safety semantics. | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: No inventory or fulfillment surface is touched. | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: Mirror artifacts, task-board projections, traceability records, and session-log continuity are archival/runtime evidence surfaces. | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: No new retrieval engine is introduced beyond existing structured records. | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: Analytics are downstream consumers, not primary implementation scope here. | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: No dataset-wrangling surface is added. | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: Check-run persistence, session-log access, and runtime-governance durability must stay behind storage/database boundaries. | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: Validator gates, activation traceability, additive overlay rules, and hard boundaries are sovereignty concerns. | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: Guidance copy may surface later in DCC, but no standalone guide-engine behavior is added here. | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: Spec Session Log and structured workflow-mirror artifacts preserve bounded context offload. | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: This WP is driven by spec/version provenance and base->active packet traceability. | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: CheckRunner already owns governed execution/capability controls; this WP reuses that surface without extending sandbox semantics. | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Gate transitions, stub activation, and governed check execution must remain recorder-visible. | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: No calendar behavior is involved. | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: No editor surface is required. | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: No document editor surface is touched. | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: No spreadsheet surface is touched. | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus remains the canonical work-tracking/workflow identity source that the mirror must extend rather than replace. | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: No media/timeline surface is in scope. | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Existing runtime work-packet projections are reused as consumers of the mirror, but this WP does not redefine the product work-packet pillar itself. | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Existing runtime task-board projections consume the mirror output, but the product task-board pillar is not widened in this packet. | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: No micro-task-specific mirror extension is required. | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: Command Center is a downstream consumer of the backend mirror. Dedicated operator-control work belongs to the later Command Center backend packet. | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: FEMS is not the authority for governance workflow mirror state. | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: Workflow mirror updates and governed checks run inside runtime execution surfaces. | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: Prompt/spec generation is upstream of this runtime-governance implementation packet. | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: Mirror persistence must stay behind storage traits and remain SQLite-now/Postgres-ready. | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: Structured-first workflow-mirror records are needed so small/local models do not parse long Markdown by default. | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: No Stage surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: No Studio surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: No Atelier/Lens surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: No distillation/training surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: No ACE-specific surface is changed by this mirror work. | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: TOUCHED | NOTES: Spec Session Log and structured mirror fields are intended for bounded retrieval/context offload. | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: Canonical WP/workflow identity | SUBFEATURES: stable `task_board_id`, `work_packet_id`, `workflow_run_id`, `model_session_id` carry-through into mirror artifacts | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus remains canonical; the mirror stores foreign keys and summaries only.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: Governance gate and activation telemetry | SUBFEATURES: `FR-EVT-GOV-GATES-001`, `FR-EVT-GOV-WP-001`, check-run linkage | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Recorder visibility is a hard requirement.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Activation-bound runtime packet mirror | SUBFEATURES: active packet refs, activation mappings, check summary attachment | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Reuse current runtime work-packet projections rather than creating new packet primitives.
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Human-readable governance mirror | SUBFEATURES: gate summaries, activation visibility, stable task-board routing | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The task board stays the human-readable mirror, not the canonical work-state authority.
  - PILLAR: Command Center | CAPABILITY_SLICE: Operator governance inspection | SUBFEATURES: WP filter, verdict filter, activation card, last check run, session-log timeline | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Backend-first in this WP, but visibility requirements are real.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Governed runtime updates | SUBFEATURES: mirror synchronization, check-run recording, bounded mutation points | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Runtime updates must stay behind product-owned services and database traits.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Durable storage contracts | SUBFEATURES: no direct SQLite outside storage, deterministic record shapes, upgrade-safe persistence | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Current trait gaps are implementation debt, not a reason to bypass the boundary.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Structured-first governance state | SUBFEATURES: JSON-first gate artifacts, structured task-board/traceability summaries, bounded on-demand Markdown | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This directly supports local-small-model planning and replay.
  - PILLAR: RAG | CAPABILITY_SLICE: Session-log retrieval substrate | SUBFEATURES: workflow-facing Spec Session Log append/query with linked artifacts and stable ids | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This WP should preserve RAG-ready indexing semantics already named by the spec.
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Per-WP validator gate mirror | JobModel: WORKFLOW | Workflow: governance_workflow_mirror_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Add runtime-owned gate artifacts under `.handshake/gov/validator_gates/` keyed by canonical WP ids.
  - Capability: Activation traceability mirror | JobModel: WORKFLOW | Workflow: governance_workflow_mirror_activation | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-WP-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Record base->active packet mapping in a narrowly scoped overlay artifact without treating runtime as repo authority.
  - Capability: Workflow-facing Spec Session Log | JobModel: WORKFLOW | Workflow: governance_spec_session_log | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001, FR-EVT-GOV-WP-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse current `spec_session_log_entries` substrate but expose workflow-facing append/query semantics.
  - Capability: Governed check linkage | JobModel: MECHANICAL_TOOL | Workflow: governance_check_runner | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-CHECK-001, FR-EVT-GOV-CHECK-002, FR-EVT-GOV-CHECK-003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse CheckRunner and registry foundations; mirror stores summaries/evidence refs, not raw tool execution state.
  - Capability: Runtime governance query/projection surface | JobModel: UI_ACTION | Workflow: runtime_governance_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Extend current task-board/work-packet projection surface instead of introducing a parallel query stack.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Local-only matrix scan. The relevant high-ROI combinations are already clear from current spec/code coupling: Locus <-> task board/work packet projections, GovernanceArtifactRegistry <-> CheckRunner, and FR governance events <-> workflow mirror records.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE. Existing main-body clauses already express the required interactions for this WP strongly enough to implement without a new Appendix 12.6 edge row.
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: No new appendix interaction edge is required to unblock this runtime mirror implementation.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This WP is an internal runtime-governance mirror decision and the useful combinations are already fully determined by local spec and code.
- SOURCE_SCAN:
  - NONE
- MATRIX_GROWTH_CANDIDATES:
  - NONE
- ENGINEERING_TRICKS_CARRIED_OVER:
  - NONE
- MATRIX_RESEARCH_VERDICT: NOT_APPLICABLE

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: Flight Recorder + Gate Mirror | Pillars: Flight Recorder | Mechanical: engine.sovereign, engine.archivist | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Every validator-gate transition must remain flight-recorder-visible while persisting as workflow-mirror state.
  - Combo: Locus + Activation Traceability | Pillars: Locus | Mechanical: engine.version, engine.sovereign | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Keep Locus canonical while adding runtime-only activation lineage keyed by stable workflow and WP ids.
  - Combo: Runtime Governance + Durable Gate Storage | Pillars: Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Per-WP gate artifacts must persist through product-owned runtime/storage seams rather than direct repo reads.
  - Combo: SQLite-now Contract + Postgres-ready Shapes | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.version | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Gate, activation, and session-log records must stay deterministic and migration-safe behind storage traits.
  - Combo: Structured Gate Summaries + Small-Model Routing | Pillars: LLM-friendly data | Mechanical: engine.context | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: The mirror should expose bounded structured fields first so local models do not need long Markdown to route governance work.
  - Combo: Session Log Retrieval + Workflow Replay | Pillars: RAG | Mechanical: engine.archivist, engine.context | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Workflow-facing Spec Session Log access should preserve retrieval-friendly records and linked artifacts.
  - Combo: Flight Recorder + Idempotent Gate Transitions | Pillars: Flight Recorder, Execution / Job Runtime | Mechanical: engine.sovereign, engine.version | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Replay-safe gate writes prevent double-counted runtime evidence during parallel workflow activity.
  - Combo: Runtime Projection + Structured Session Log | Pillars: Execution / Job Runtime, LLM-friendly data | Mechanical: engine.context, engine.archivist | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: The mirror should project stable gate and activation summaries into workflow-facing records without flattening everything into prose.
  - Combo: Locus Keys + Storage Boundary | Pillars: Locus, SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.sovereign | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Stable ids must flow through storage-backed artifacts so runtime consumers never infer identity from filenames alone.
  - Combo: Structured Retrieval + Gate Evidence | Pillars: RAG, LLM-friendly data | Mechanical: engine.archivist | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Gate evidence and activation refs should stay cheap to retrieve and filter without requiring chat-history reconstruction.
  - Combo: Command Center Consumer Readiness | Pillars: Command Center, Execution / Job Runtime | Mechanical: engine.context | Primitives/Features: NONE | Resolution: IN_THIS_WP | Stub: NONE | Notes: This packet only establishes backend-ready mirror/query surfaces so the later Command Center backend WP can consume them cleanly.
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations resolve in-scope with existing validated foundations; no new stub is genuinely required.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- SCAN_SCOPE: Target stub, current task board and traceability registry, validated sibling governance packets, structured-collaboration/workflow/role-mailbox packets, and `../handshake_main` runtime-governance/workflow/check-runner code.
- MATCHED_STUBS:
  - Artifact: WP-1-Governance-Workflow-Mirror-v2 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: N/A | Matrix: N/A | UI: N/A | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: This refinement hydrates the active remediation stub directly; the historical `v1` packet remains audit evidence only.
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Product-Governance-Artifact-Registry-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: N/A | CodeReality: IMPLEMENTED | Resolution: REUSE_EXISTING | Stub: NONE | Notes: Registry types and store contracts already exist and should be consumed rather than reimplemented.
  - Artifact: WP-1-Product-Governance-Check-Runner-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: N/A | CodeReality: IMPLEMENTED | Resolution: REUSE_EXISTING | Stub: NONE | Notes: CheckRunner and FR event plumbing already exist; this WP should link to them instead of duplicating governed execution logic.
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: N/A | CodeReality: IMPLEMENTED | Resolution: REUSE_EXISTING | Stub: NONE | Notes: Existing structured-collaboration families already cover task-board/work-packet/governance-registry foundations.
  - Artifact: WP-1-Workflow-Projection-Correlation-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: PARTIAL | CodeReality: IMPLEMENTED | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: Stable workflow/task-board/model-session correlation exists, but current `main` still lacks gate-summary and activation-summary projection parity from the historical workflow-mirror branch.
  - Artifact: WP-1-Role-Mailbox-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: N/A | CodeReality: IMPLEMENTED | Resolution: REUSE_EXISTING | Stub: NONE | Notes: The workflow-facing Spec Session Log append/query seam already exists on current `main`; `v2` should reuse it rather than redesign it.
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs | Artifact: NONE | Covers: execution | Verdict: IMPLEMENTED | Notes: Runtime governance helpers already expose validator-gate paths and the shared WP traceability registry display used by the current-main activation event path.
  - Path: ../handshake_main/src/backend/handshake_core/src/governance_artifact_registry.rs | Artifact: WP-1-Product-Governance-Artifact-Registry-v1 | Covers: primitive | Verdict: IMPLEMENTED | Notes: `GovernanceArtifactKind`, `GovernanceArtifactRegistryManifest`, and store traits are present and reusable.
  - Path: ../handshake_main/src/backend/handshake_core/src/governance_check_runner.rs | Artifact: WP-1-Product-Governance-Check-Runner-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: `CheckRunner`, `CheckDescriptor`, and bounded execution behavior are present.
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v4 | Covers: primitive | Verdict: IMPLEMENTED | Notes: Workflow state families and structured-collaboration descriptors already preserve stable task-board/work-packet contracts.
  - Path: ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs | Artifact: WP-1-Role-Mailbox-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: `append_workflow_spec_session_log`, `query_workflow_spec_session_log`, and the runtime-artifact boundary test already exist on current `main` and should be reused intact.
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Product-Governance-Check-Runner-v1 | Covers: execution | Verdict: PARTIAL | Notes: Governance check run structs exist, but default trait methods still return `NotImplemented`.
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Workflow-Projection-Correlation-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: Current `main` already emits `gov_work_packet_activated` and preserves stable workflow/task-board/work-packet/model-session identifiers through the projection path.
  - Path: ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: `GovWorkPacketActivated` is implemented and validated on current `main`, but `GovGateTransition` and its payload validator are still missing.
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: Core runtime-governance, registry, check-runner, schema, projection, and mailbox foundations already exist, but this WP still needs to add the narrow workflow-mirror overlay and workflow-facing seams on top of them.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet is backend-first runtime-governance composition only. Dedicated operator-facing UI/control work is intentionally deferred to `WP-1-Dev-Command-Center-Control-Plane-Backend-v1`.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - NONE
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - NONE
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: This WP intentionally stops at backend/runtime mirror readiness. Dedicated operator-facing GUI implementation advice belongs to the later Command Center backend packet.
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

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.180]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Artifact-Registry, WP-1-Product-Governance-Check-Runner, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Workflow-Projection-Correlation, WP-1-Role-Mailbox
- BUILD_ORDER_BLOCKS: WP-1-Project-Agnostic-Workflow-State-Registry, WP-1-Workflow-Transition-Automation-Registry, WP-1-Governance-Pack
- SPEC_ANCHOR_PRIMARY: 2.3.15 tracked work-packet gates/task-packet linkage + 2.6.8.8 Spec Session Log + 7.5.4.8 repo/runtime boundary + 7.5.4.9 CheckRunner + 11.5.4 FR governance events
- WHAT: Finish the missing workflow-mirror parity slice on current `main`: add the gate-transition FR event and payload validation, restore structured gate/activation summary types, and wire those summaries into the current workflow/task-board projection path without replaying stale whole-file branch snapshots.
- WHY: Current product already contains workflow-facing session-log support and the activation-side `gov_work_packet_activated` path, but it still lacks the gate-transition and structured-summary half of the original workflow-mirror design. This remediation closes that specific gap while preserving later mainline runtime behavior.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
- OUT_OF_SCOPE:
  - Any runtime code that reads from or writes to `/.GOV/`
  - Replacing Locus as the canonical work-tracking authority
  - Replaying the historical `v1` branch wholesale onto current `main`
  - A broad repo-governance mirror beyond validator gates, activation traceability, and workflow-facing session-log state
  - New Master Spec text, appendix updates, or packet creation in this turn
- TEST_PLAN:
  ```bash
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml --lib governance_workflow_mirror_gate_transition_emits_fr_event -- --exact --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml --lib test_gov_work_packet_activated_payload_validation -- --exact --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml workflows -- --nocapture
  ```
- DONE_MEANS:
  - Runtime-owned validator-gate artifacts exist per WP without collisions across parallel WPs.
  - Current-main activation traceability behavior stays intact and continues to point `traceability_registry_ref` at the shared runtime registry.
  - Gate transitions append workflow-facing Spec Session Log entries and emit the required `gov_gate_transition` / `FR-EVT-GOV-GATES-001` event.
  - Structured gate-summary and activation-summary data are visible through the current workflow/task-board projection surfaces without creating a second authority.
  - No implementation path reads from or writes to `/.GOV/`.
- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/locus/types.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/workflows.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - validator_gates
  - gov_gate_transition
  - GovGateTransition
  - WorkflowMirrorGateSummary
  - WorkflowMirrorActivationSummary
  - activation_summary
  - gate_summary
  - spec_session_log
  - task_board_id
  - work_packet_id
- RUN_COMMANDS:
  ```bash
  rg -n "gov_gate_transition|GovGateTransition|WorkflowMirrorGateSummary|WorkflowMirrorActivationSummary|activation_summary|gate_summary" ../handshake_main/src/backend/handshake_core/src ../wtc-workflow-mirror-v1/src/backend/handshake_core/src
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml --lib governance_workflow_mirror_gate_transition_emits_fr_event -- --exact --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml --lib test_gov_work_packet_activated_payload_validation -- --exact --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml workflows -- --nocapture
  ```
- RISK_MAP:
  - "whole-file replay from historical v1 overwrites later current-main runtime behavior" -> "regression in session spawn/checkpoint/workspace isolation flows"
  - "mirror widens into a general repo-governance clone" -> "split authority and boundary violation"
  - "gate state not keyed by canonical WP/task-board ids" -> "parallel-WP collisions and untrustworthy UI state"
  - "session-log seam is reimplemented instead of reused" -> "workflow mirror drifts from the already-contained current-main behavior"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - No build-order sync is required in this refinement-only pass because no new stubs, dependencies, or spec version changes were introduced. Re-check BUILD_ORDER only when the official packet is created.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: 2.3.15 tracked work-packet `gates.pre_work`, `task_packet_path`, and `task_board_status` remain canonical workflow fields | WHY_IN_SCOPE: The workflow mirror must extend these fields with runtime-owned validator-gate and activation artifacts without replacing Locus authority | EXPECTED_CODE_SURFACES: `runtime_governance.rs`, `workflows.rs`, `locus/types.rs` | EXPECTED_TESTS: runtime-governance/workflow projection tests proving per-WP gate files and stable id carry-through | RISK_IF_MISSED: split-brain state between Locus and runtime mirror
  - CLAUSE: 2.6.8.8 Spec Session Log + 2.6.8.9 integration hooks | WHY_IN_SCOPE: Gate transitions and stub activation must append human-facing ledger entries and remain separately queryable from Flight Recorder | EXPECTED_CODE_SURFACES: `role_mailbox.rs`, workflow-mirror adapter in `workflows.rs` or adjacent runtime-governance service | EXPECTED_TESTS: session-log tests proving append/query behavior and stable `spec_id`/`task_board_id`/`work_packet_id` linkage | RISK_IF_MISSED: operators and models lose the required parallel planning ledger
  - CLAUSE: 7.5.4.8 hard repo/runtime boundary | WHY_IN_SCOPE: The product runtime mirror must be product-owned and must not read/write `/.GOV/` | EXPECTED_CODE_SURFACES: `runtime_governance.rs`, any new workflow-mirror service, boundary tests | EXPECTED_TESTS: negative-path tests proving `.GOV/` access is rejected and runtime roots stay under `.handshake/gov/` | RISK_IF_MISSED: the implementation violates a hard spec boundary
  - CLAUSE: 7.5.4.9 Governance Check Runner additive overlay rule and storage boundary | WHY_IN_SCOPE: This WP should reuse typed check execution/results and persist summaries through existing boundaries, not invent a side channel | EXPECTED_CODE_SURFACES: `governance_check_runner.rs`, `governance_artifact_registry.rs`, `storage/mod.rs`, workflow-mirror linkage surfaces | EXPECTED_TESTS: check-linkage tests proving result/evidence refs are persisted and projected without direct SQLite bypass | RISK_IF_MISSED: governed check state drifts from the runtime mirror or bypasses the storage contract
  - CLAUSE: 11.5.4 `FR-EVT-GOV-GATES-001` and `FR-EVT-GOV-WP-001` | WHY_IN_SCOPE: The workflow mirror is the runtime surface that must emit those governance events on state change | EXPECTED_CODE_SURFACES: `flight_recorder/mod.rs`, workflow-mirror update paths in `workflows.rs` or adjacent service | EXPECTED_TESTS: FR payload tests validating event kind, refs, and idempotency behavior | RISK_IF_MISSED: governance transitions become invisible to the authoritative system log

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: `.handshake/gov/validator_gates/{wp_id}.json` gate artifact | PRODUCER: workflow-mirror runtime service | CONSUMER: task-board/work-packet projections, Command Center governance views, validator logic | SERIALIZER_TRANSPORT: deterministic JSON file under runtime governance root | VALIDATOR_READER: runtime-governance path helpers plus projection tests | TRIPWIRE_TESTS: per-WP isolation, deterministic serialization, verdict transition event emission | DRIFT_RISK: parallel WPs collide or UI reads stale/malformed gate state
  - CONTRACT: runtime activation-traceability record | PRODUCER: stub-activation workflow path | CONSUMER: task-board mirror, FR event payload, operator audit views | SERIALIZER_TRANSPORT: structured runtime artifact plus optional human-readable mirror | VALIDATOR_READER: workflow projection and FR payload tests | TRIPWIRE_TESTS: base->active mapping correctness, stable refs, idempotent re-activation handling | DRIFT_RISK: active packet lineage becomes ambiguous
  - CONTRACT: workflow-facing Spec Session Log entry | PRODUCER: gate transition and activation mirror updates | CONSUMER: RAG/context offload, operator timelines, workflow replay/debug | SERIALIZER_TRANSPORT: database-backed record with stable structured fields and linked artifact refs | VALIDATOR_READER: role-mailbox/session-log query tests | TRIPWIRE_TESTS: append/query by `spec_id` + `task_board_id` + `work_packet_id`, no mailbox-body leakage | DRIFT_RISK: human-facing ledger loses continuity or leaks unrelated mailbox data
  - CONTRACT: governed check run linkage | PRODUCER: CheckRunner + workflow-mirror linkage layer | CONSUMER: runtime governance views, WP gate summaries, validator workflows | SERIALIZER_TRANSPORT: database row plus evidence/hash refs in mirror summaries | VALIDATOR_READER: storage trait tests and projection tests | TRIPWIRE_TESTS: result/evidence refs survive persistence and projection without raw tool-log duplication | DRIFT_RISK: mirror shows check status unsupported by stored evidence
  - CONTRACT: task-board/work-packet mirror summaries | PRODUCER: workflow projection layer | CONSUMER: Command Center, local/cloud models, operator audits | SERIALIZER_TRANSPORT: structured projection JSON plus human-readable Markdown mirror | VALIDATOR_READER: workflow/task-board projection tests | TRIPWIRE_TESTS: stable ids, gate summary presence, activation summary presence, no second task-board authority | DRIFT_RISK: human-readable mirrors and structured records diverge silently

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml governance_workflow_mirror_gate_transition_emits_fr_event -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml test_gov_work_packet_activated_payload_validation -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml governance_workflow_mirror_spec_session_log_enforces_runtime_artifact_boundary -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml workflows -- --nocapture`
- CANONICAL_CONTRACT_EXAMPLES:
  - Example per-WP gate artifact at `.handshake/gov/validator_gates/WP-1-Governance-Workflow-Mirror-v2.json` with stable `wp_id`, verdict, gate-state ref, and evidence/check refs.
  - Example activation-traceability record linking `base_wp_id`, `work_packet_id`, `task_board_id`, `active_packet_ref`, and runtime-owned traceability ref.
  - Example task-board projection row whose structured fields include `task_board_id`, `work_packet_id`, gate summary, and activation summary.
  - Example workflow-facing `SpecSessionLogEntry` record covering a gate transition and a stub activation event with linked runtime artifacts only.

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Start from current `main`, not from the historical branch snapshot.
  - Port only the missing branch-only workflow-mirror slice: `GovGateTransition`, structured gate/activation summary types, and the projection wiring that consumes them.
  - Preserve the current-main `gov_work_packet_activated` path and the existing workflow-facing Spec Session Log seam in `role_mailbox.rs`.
  - Extend task-board/work-packet projections and FR emission paths without replaying stale whole-file versions from the historical branch.
  - Add deterministic tests and negative boundary checks after the minimal port is in place.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/workflows.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/locus/types.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/flight_recorder/mod.rs
- TRIPWIRE_TESTS:
  - `governance_workflow_mirror_gate_transition_emits_fr_event`
  - `test_gov_work_packet_activated_payload_validation`
  - `governance_workflow_mirror_spec_session_log_enforces_runtime_artifact_boundary`
  - `workflows -- --nocapture`
- CARRY_FORWARD_WARNINGS:
  - Do not widen base Locus families to absorb overlay-only workflow mirror facts.
  - Do not create a second task-board authority; extend the current runtime task-board/work-packet projection surface.
  - Do not let runtime read or write `/.GOV/`.
  - Do not overwrite current-main `SessionSpawnRequested`, `SessionCheckpointCreated`, or `WorkspaceIsolationDenied` behavior by replaying stale branch code.
  - Keep structured-first records canonical and Markdown mirrors derived or reconciled.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - 2.3.15 tracked work-packet gate/task-packet/task-board linkage
  - 2.6.8.8 Spec Session Log + 2.6.8.9 integration hooks
  - 7.5.4.8 repo/runtime hard boundary
  - 7.5.4.9 CheckRunner additive overlay/storage boundary
  - 11.5.4 `FR-EVT-GOV-GATES-001` and `FR-EVT-GOV-WP-001`
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/governance_check_runner.rs
  - ../handshake_main/src/backend/handshake_core/src/governance_artifact_registry.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
- COMMANDS_TO_RUN:
  - `rg -n "validator_gates|WP_TRACEABILITY_REGISTRY|spec_session_log|FR-EVT-GOV|GovernanceCheckRun" ../handshake_main/src/backend/handshake_core/src`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml governance_check_runner -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml workflows -- --nocapture`
- POST_MERGE_SPOTCHECKS:
  - Verify runtime-generated gate refs stay under `.handshake/gov/validator_gates/`.
  - Verify activation traceability and task-board/work-packet projections agree on the same WP/task-board/workflow ids.
  - Verify Spec Session Log entries exist for gate transitions and activation without leaking mailbox message bodies.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The exact final runtime artifact format for activation traceability (single structured artifact vs structured artifact plus Markdown mirror) is not proven in code yet, only bounded semantically by this refinement.
  - The storage-backed implementation depth for `create_governance_check_run` and `list_governance_check_runs` is not proven; only the trait seam and structs exist today.
  - The final Command Center operator surface is not proven; this refinement only establishes the backend/runtime visibility requirements.

### DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (This pass composes existing runtime-governance, registry, workflow, and mailbox surfaces instead of introducing new Appendix 12 primitives.)
- DISCOVERY_STUBS: NONE_CREATED (No new stub was warranted because the next Command Center backend WP already exists and the current packet stays within the target stub scope.)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (The required interactions are already expressed through existing Locus, workflow-projection, CheckRunner, and session-log relationships.)
- DISCOVERY_UI_CONTROLS: NONE_APPLICABLE (Operator-facing controls are intentionally deferred to `WP-1-Dev-Command-Center-Control-Plane-Backend-v1`.)
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED (The current Master Spec already names the boundary, events, path posture, and session-log requirements this packet implements.)
- DISCOVERY_JUSTIFICATION: This refinement still delivered value by collapsing scope to the correct narrow runtime workflow-mirror layer, reusing validated foundations, and proving that no spec repair or new stub creation is needed before packet hydration.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current spec already names runtime gate artifacts, Spec Session Log behavior, hard repo/runtime boundary, CheckRunner, and FR governance events. This refinement makes the implementation decision explicit: add a narrow runtime workflow-mirror layer on top of current foundations instead of widening base Locus state.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The current Master Spec already contains the normative clauses needed to implement this WP: runtime gate path expectations, Spec Session Log rules, the repo/runtime hard boundary, bounded CheckRunner behavior, deterministic governance snapshot shapes, and FR governance event schemas. The remaining work is product/runtime implementation and composition, not spec text repair.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 2.3.15 Locus Work Tracking System (tracked work packet gate/task packet linkage)
- CONTEXT_START_LINE: 6134
- CONTEXT_END_LINE: 6143
- CONTEXT_TOKEN: validator_gates/{WP_ID}.json
- EXCERPT_ASCII_ESCAPED:
  ```text
  // Gate status (Validator integration)
  gates: {
    pre_work: GateStatus;            // From .handshake/gov/validator_gates/{WP_ID}.json
    post_work: GateStatus;
  };

  // Task Packet reference
  task_packet_path?: string;         // ".handshake/gov/task_packets/WP-1-Auth.md"
  task_board_status: TaskBoardStatus;
  };
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 2.6.8.8 Spec Session Log (Task Board + Work Packets)
- CONTEXT_START_LINE: 10511
- CONTEXT_END_LINE: 10543
- CONTEXT_TOKEN: SpecSessionLogEntry
- EXCERPT_ASCII_ESCAPED:
  ```text
  Task Board items and Work Packets together form a Spec Session Log that runs in parallel to the Flight Recorder. Flight Recorder remains the authoritative system log; the Spec Session Log captures human-facing planning state and is used for context offload.

  pub struct SpecSessionLogEntry {
      pub entry_id: String,
      pub spec_id: String,
      pub task_board_id: String,
      pub work_packet_id: Option<String>,
      pub event_type: String,
      pub governance_mode: GovernanceMode,
      pub actor: String,
      pub timestamp: DateTime<Utc>,
      pub summary: String,
      pub linked_artifacts: Vec<ArtifactHandle>,
  }

  Rules:
  - Every Task Board or Work Packet change MUST emit a SpecSessionLogEntry stored in the workspace and indexed for RAG.
  - The Spec Session Log MUST NOT replace Flight Recorder; it is a parallel, human-facing ledger.
  - Spec Session Log entries MUST reference the same spec_id and work_packet_id used in SpecIntent and WorkPacketBinding.
  - SpecSessionLogEntry.entry_id MUST be unique within the workspace.
  - SpecSessionLogEntry.governance_mode MUST match the active mode at the time of the event; mode transitions require a dedicated entry.
  - [ADD v02.163] Task Board entries and Work Packet artifacts MUST remain separately queryable coordination surfaces: Task Board is the human-readable mirror, while Work Packet artifacts preserve scoped execution contracts, workflow-linked activation, and session binding state.
  - [ADD v02.163] Dev Command Center, Locus Work Tracking, Workflow Engine, and Model Session Orchestration projections MUST preserve stable task_board_id, work_packet_id, workflow_run_id, and model_session_id values so parallel-model planning never depends on manual board interpretation.
  - [ADD v02.166] Work Packet and Task Board artifacts SHOULD expose canonical structured representations that are cheap to filter, route, and replay. Human-readable Markdown mirrors MAY remain, but they MUST be derived from or reconciled against the same stable identifiers and field values.
  - [ADD v02.166] Local-small-model planning and execution SHOULD read bounded structured fields first and only load long-form notes or Markdown mirrors on demand.

  #### 2.6.8.9 Integration Hooks (Normative)

  - Flight Recorder logs every router decision, refinement pass, signature, gate outcome, and spec status change.
    - Governance gate transitions MUST emit `FR-EVT-GOV-GATES-001`.
    - Stub activation (stub -> official packet + traceability mapping) MUST emit `FR-EVT-GOV-WP-001`.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 7.5.4.8 Repo/runtime boundary (HARD)
- CONTEXT_START_LINE: 31897
- CONTEXT_END_LINE: 31901
- CONTEXT_TOKEN: Runtime governance state MUST live in product-owned storage
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Repo/runtime boundary (HARD)**
  - `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
  - `docs/` MAY exist as a temporary compatibility bundle only (non-authoritative governance state).
  - Handshake product runtime MUST NOT read from or write to `/.GOV/` (hard boundary; enforce via CI/gates).
  - Runtime governance state MUST live in product-owned storage. Handshake default: `.handshake/gov/` (configurable). This directory contains runtime governance state only.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 7.5.4.9 Governance Check Runner: Bounded Execution Contract
- CONTEXT_START_LINE: 31907
- CONTEXT_END_LINE: 31951
- CONTEXT_TOKEN: Additive Overlay Rule
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Purpose**
  Define a bounded, observable execution layer for imported governance checks so that Handshake validates software-delivery workflows through capability-gated, recorder-visible, product-owned execution instead of raw shell bypass.

  **Definitions**
  - **CheckDescriptor**: a typed execution record derived from a `GovernanceArtifactRegistryEntry` with `kind=Checks` or `kind=Rubrics`. It carries the check identifier, required capabilities, timeout_ms, input schema, and version provenance from the registry.
  - **CheckResult**: a typed result contract with exactly five variants:
    - `PASS` -- check completed and all assertions satisfied
    - `FAIL` -- check completed and one or more assertions failed
    - `BLOCKED` -- check could not execute due to capability denial or precondition failure
    - `ADVISORY_ONLY` -- check completed but findings are informational and do not gate progress
    - `UNSUPPORTED` -- check kind or descriptor version is not executable in the current runtime
  - **CheckRunner**: the product service that executes a `CheckDescriptor` through the Unified Tool Surface Contract and produces a `CheckResult` with evidence.

  **Execution Lifecycle**
  The CheckRunner MUST implement a three-phase bounded lifecycle:
  1. **PreCheck**: validate `CheckDescriptor` schema, verify required capabilities through `CapabilityGate`, and confirm `timeout_ms` is within runtime bounds. Failure here MUST produce `CheckResult::Blocked` immediately without proceeding to execution.
  2. **Check**: invoke the check body through the `governance.check.run` tool surface. Execution is bounded by `timeout_ms`. A timeout or execution error MUST produce `CheckResult::Blocked`.
  3. **PostCheck**: capture the raw result, map it to the `CheckResult` enum, store evidence artifacts with content hash, and emit Flight Recorder events.

  **Tool Surface**
  The `governance.check.run` tool_id MUST be registered under the Unified Tool Surface Contract (6.0.2) with:
  - `side_effect: GOVERNED_WRITE`
  - Required capabilities declared in the `CheckDescriptor`
  - Input schema: `{ check_id: string, descriptor_ref: string, input_args: object }`
  - Output schema: `CheckResult` JSON

  **Flight Recorder Events**
  Every check execution MUST emit the following FR events:
  - `FR-EVT-GOV-CHECK-001` (`governance.check.started`): payload includes `check_id`, `session_id`, `check_descriptor_hash`
  - `FR-EVT-GOV-CHECK-002` (`governance.check.completed`): payload includes `check_id`, `session_id`, `result_status`, `duration_ms`, `evidence_artifact_id`
  - `FR-EVT-GOV-CHECK-003` (`governance.check.blocked`): payload includes `check_id`, `session_id`, `blocked_reason`

  FR events MUST be emitted for all result variants including `BLOCKED` and `UNSUPPORTED`. Silent skip is prohibited.

  **Additive Overlay Rule**
  Imported governance checks MUST extend the product governance surface additively. No imported check MAY:
  - overwrite or disable native Handshake governance rules
  - modify base-envelope structured collaboration records
  - acquire capabilities beyond those declared in its `CheckDescriptor`

  **Unsupported Checks**
  A check descriptor with an unrecognized `kind`, unsupported schema version, or missing required execution surface MUST return `CheckResult::Unsupported` with an explicit reason string. Silent skip is prohibited.

  **Storage**
  All `CheckDescriptor` and `CheckResult` persistence MUST go through the `Database` trait boundary. No direct SQLite calls outside the storage module are permitted.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 7.5.4.11 Product Governance Snapshot (HARD)
- CONTEXT_START_LINE: 46952
- CONTEXT_END_LINE: 46999
- CONTEXT_TOKEN: wp_gate_summaries
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Purpose**
  Provide a deterministic, leak-safe snapshot of the current governance state for a product/repo so a fresh agent (or auditor) can reconstruct "what is true" without relying on chat history.

  **Definition**
  A "Product Governance Snapshot" is a machine-readable JSON export derived ONLY from canonical governance artifacts (no repo scan; no extras):
  - `.GOV/spec/SPEC_CURRENT.md`
  - resolved spec file referenced inside it (e.g., `Handshake_Master_Spec_v02.125.md`)
  - `.GOV/roles_shared/records/TASK_BOARD.md`
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
  - `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
  - `.GOV/validator_gates/*.json` (if present)

  **Output location (HARD)**
  - Default path: `.GOV/roles_shared/runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json`
  - The export MUST be deterministic for a given set of input files.
  - The export MUST NOT include wall-clock timestamps.
  - The export MAY include the current git HEAD sha (if available) as provenance.
  - The output bytes MUST be `JSON.stringify(obj, null, 2) + "\n"` (force `\n` newlines; no locale formatting).

  **Determinism (HARD)**
  - Generator MUST enforce stable ordering:
    - `inputs` sorted by `path` (ascending).
    - `task_board.entries` sorted by `wp_id` (ascending).
    - `traceability.mappings` sorted by `base_wp_id` (ascending).
    - `signatures.consumed` sorted by `signature` (ascending).
    - `gates.validator.wp_gate_summaries` sorted by `wp_id` (ascending) if present.
  - Generator MUST avoid locale/time dependent formatting (no wall clock calls).

  **Minimum schema (normative)**
  ProductGovernanceSnapshot
  - schema_version: "hsk.product_governance_snapshot@0.1"
  - spec: { spec_target: string, spec_sha1: string }
  - git: { head_sha?: string } (generator SHOULD default to `git: {}`; omit head_sha unless explicitly enabled)
  - inputs: [{ path: string, sha256: string }]
  - task_board: { entries: [{ wp_id: string, status_token: string }] }
  - traceability: { mappings: [{ base_wp_id: string, active_packet_path: string }] }
  - signatures: { consumed: [{ signature: string, purpose: string, wp_id?: string }] }
  - gates: { orchestrator: { last_refinement?: string, last_signature?: string, last_prepare?: string }, validator: { wp_gate_summaries?: [{ wp_id: string, verdict?: string, status?: string, gates_passed?: string[] }] } }
    - `wp_gate_summaries` MUST be a list (not a map/object) and MUST omit timestamps and raw logs/bodies.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 11.5.4 FR-EVT-GOV-GATES-001 / FR-EVT-GOV-WP-001
- CONTEXT_START_LINE: 67220
- CONTEXT_END_LINE: 67249
- CONTEXT_TOKEN: traceability_registry_ref
- EXCERPT_ASCII_ESCAPED:
  ```text
  // FR-EVT-GOV-GATES-001
  interface GovernanceGateTransitionEvent extends FlightRecorderEventBase {
    type: 'gov_gate_transition';

    spec_id: string | null;
    work_packet_id: string | null;

    role: 'operator' | 'orchestrator' | 'coder' | 'validator' | 'system';
    gate_kind: 'orchestrator' | 'validator';
    gate: string;                     // e.g. REPORT_PRESENTED, USER_ACKNOWLEDGED, WP_APPENDED, COMMITTED, REFINE_RECORDED, SIGNATURE_RECORDED, PREPARE_RECORDED
    verdict?: 'PASS' | 'FAIL' | null;  // REQUIRED for REPORT_PRESENTED; otherwise null/omitted

    gate_state_ref: string;           // e.g. .handshake/gov/validator_gates/WP-1-Example-v1.json (or other artifact handle)
    idempotency_key: string;
  }

  // FR-EVT-GOV-WP-001
  interface WorkPacketActivatedEvent extends FlightRecorderEventBase {
    type: 'gov_work_packet_activated';

    spec_id: string | null;
    base_wp_id: string;
    work_packet_id: string;

    stub_packet_ref: string;          // e.g. .handshake/gov/task_packets/stubs/WP-...md
    active_packet_ref: string;         // e.g. .handshake/gov/task_packets/WP-...md

    traceability_registry_ref: string; // e.g. .handshake/gov/WP_TRACEABILITY_REGISTRY.md
    task_board_ref: string;            // e.g. .handshake/gov/TASK_BOARD.md
  ```
