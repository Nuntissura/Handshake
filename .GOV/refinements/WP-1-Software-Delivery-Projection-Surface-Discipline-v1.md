## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by the current ORCHESTRATOR_PROTOCOL refinement workflow.

### METADATA
- WP_ID: WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-27T13:05:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- SPEC_TARGET_SHA1: 231fea32a73934e9f66e00a3bbe26c80b7e058c9
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja270420261840
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Current product code already has partial DCC control-plane, Task Board projection, Role Mailbox export, compact-summary, workflow-state-family, queue-reason, and allowed-action support, but it does not yet prove the v02.181 software-delivery projection-discipline contract as one shared runtime-backed surface across Dev Command Center, Task Board, and Role Mailbox.
- The missing implementation slice is not new Master Spec law. It is the runtime and projection hardening that prevents layout position, unread state, mailbox chronology, packet prose, and repo /.GOV mirrors from becoming hidden authority for validation, ownership, queued steering, recovery, or closeout posture.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 30m
- SEARCH_SCOPE: Repo-local current Master Spec v02.181, target stub, BUILD_ORDER, TASK_BOARD, WP traceability registry, and local product code/runtime surfaces under ../handshake_main/src/backend/handshake_core.
- REFERENCES: NONE - external research is not applicable because this WP is an internal product-governance/runtime projection slice already grounded in current Master Spec law and local implementation truth.
- PATTERNS_EXTRACTED: Reuse the existing structured collaboration base envelope, compact summary contract, workflow-state-family and queue-reason vocabulary, governed action descriptors, DCC snapshot builder, Task Board projection artifacts, and Role Mailbox structured export paths. Add the v02.181 software-delivery overlay fields as runtime-backed projection data rather than a second UI or Markdown authority.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT canonical runtime records and stable identifiers as projection authority; ADAPT existing DCC, Locus, Task Board, and Role Mailbox projection code to include validator-gate, claim/lease, queued-instruction, recovery, and closeout posture; REJECT any design where board lanes, unread mailbox state, packet checklist edits, transcript order, or repo /.GOV mirror freshness decide work legality.
- LICENSE/IP_NOTES: NONE - no third-party code, docs, or design assets are being reused.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Master Spec v02.181 already names the software-delivery runtime-truth, projection-surface discipline, overlay coordination, closeout, and recovery requirements. This WP hydrates implementation/proof obligations against that existing law.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- Rule: if the WP is an internal repo-governed change or product-governance mirror patch already grounded in the current Master Spec plus local code/runtime truth, it is valid and often preferable to set `RESEARCH_CURRENCY_REQUIRED=NO`. Do not force unrelated or generic web research just to populate this section.
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is an internal Handshake product-governance/runtime projection discipline WP. The governing truth is the current Master Spec plus local product code/runtime artifacts; external freshness would not change the contract.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Local evidence shows a partial implementation base: DCC snapshot/compact-summary tests, Task Board structured projection rows, Role Mailbox structured export, and workflow-state-family or queue-reason contracts already exist, so the WP should extend those seams rather than introduce a new projection store.
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
- Reuse existing Locus and governance event families where present, including work_packet_created, work_packet_updated, work_packet_gated, work_packet_completed, task_board_synced, task_board_status_changed, and governance workflow mirror events. Add no new event family in this WP unless implementation discovers that validator-gate, claim/lease, queued-instruction, or checkpoint recovery posture cannot be linked to existing recorder-visible evidence.

### RED_TEAM_ADVISORY (security failure modes)
- A projection surface can accidentally become a write authority if UI lane movement, unread mailbox state, or Markdown mirror freshness mutates workflow state without governed action checks.
- A stale repo /.GOV mirror or packet checklist can mislead operators into closing work while validator-gate, claim/lease, queued-instruction, or evidence records remain unresolved.
- A mailbox reply can be treated as completion or ownership transfer unless the implementation distinguishes mailbox-local actions from governed linked-record mutations.
- A DCC or Task Board view can hide blocked authority under a friendly queue label unless it projects explicit runtime blocker reasons and stable record identifiers.

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
  - The WP should extend existing structured collaboration, workflow, DCC, Task Board, and Role Mailbox contract structs rather than create new Appendix 12 primitive IDs at refinement time.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current Master Spec already carries the needed feature and matrix coverage for v02.181 software-delivery projection discipline. No new primitive ID is required to implement this slice.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No orphan primitive was discovered; the implementation belongs inside existing DCC, Locus, Task Board, Role Mailbox, workflow, and structured collaboration contracts.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: FEAT-DEV-COMMAND-CENTER, FEAT-LOCUS-WORK-TRACKING, FEAT-TASK-BOARD, FEAT-WORK-PACKET-SYSTEM, FEAT-ROLE-MAILBOX, and FEAT-WORKFLOW-ENGINE already contain the v02.181 projection-discipline notes.
- UI_GUIDANCE_ACTION: NO_CHANGE
- UI_GUIDANCE_REASON: v02.181 already requires projection consistency, stable-id previews, mailbox-local-versus-governed action explanation, and no hidden authority from layouts or chronology. This WP implements the guidance rather than changing it.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Existing Appendix 12.6 edges already cover DCC to Locus, DCC to Task Board, DCC to Role Mailbox, Task Board to Locus, Role Mailbox to Work Packet System, and Locus to workflow-state/allowed-action visibility.
- APPENDIX_MAINTENANCE_NOTES:
  - No spec version bump is justified by this activation pass.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: No spatial or scene projection surface is changed. | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: No fabrication or tool-path semantics are involved. | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: No physics model or simulation law is involved. | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: No simulation loop is introduced. | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: No hardware integration surface changes. | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: Workflow direction remains in existing workflow/governed action systems. | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: No media composition surface is involved. | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: No visual authoring capability changes. | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: No publication or export pipeline is widened. | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: No recipe or procedural content surface applies. | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: No food-safety semantics apply. | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: No fulfillment or inventory workflow is introduced. | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: Existing evidence refs and recorder events are consumed, but this WP does not add a new archive feature family. | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: The WP is primarily about queryable projection records and field provenance across DCC, Task Board, and Role Mailbox. | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: Analytics can consume the projection later, but no analytics engine is added. | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: No dataset ingestion or cleaning surface changes. | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: Runtime-backed projection fields must remain storage-shaped and SQLite-now/Postgres-ready instead of UI-local. | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: Authority separation is central: projections may explain or preview governed actions but cannot decide workflow legality by layout or prose. | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: User-facing guidance copy is limited to labels and tooltips in existing DCC/Task Board/Role Mailbox surfaces. | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: Compact summaries and local-small-model planning inputs must carry authority refs and stable ids first. | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: Stable work_packet_id, workflow_run_id, action_request_id, claim_id, queued_instruction_id, gate_record_id, and checkpoint_id continuity is explicit scope. | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: Capability and governed action boundaries are reused without widening sandbox execution. | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). Pillar feature definition and technical implementation MUST be derived from the current Master Spec. If unknown or underspecified, write UNKNOWN and create stubs or spec updates instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: Recorder-linked evidence is consumed through existing refs; no new recorder family is required. | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: No calendar behavior is in scope. | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: No editor shell behavior is in scope. | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: No document editor surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: No spreadsheet surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus-backed work state, task-board rows, workflow-state family, queue reasons, and allowed actions are the canonical substrate for the projection. | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: No Loom media or artifact timeline behavior is in scope. | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Work Packet records are consumed as existing contract records; this WP hardens cross-surface projection discipline rather than creating a standalone packet feature. | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Task Board remains a projection surface over Locus/runtime state; it is covered through Locus and Command Center rows to avoid treating board layout as a separate authority. | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: Micro-task summaries, hard-gate waits, and mailbox-linked waits must remain visible without transcript replay. | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: Dev Command Center is the primary projection/control surface that must explain software-delivery truth without becoming authority. | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: FEMS is not the authority for software-delivery runtime truth here. | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: Workflow-backed start, steer, cancel, close, and recover semantics plus governed action resolution are in scope. | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: This WP consumes current spec law and does not implement a prompt compiler. | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: Storage-readiness is handled through the touched DBA engine; no standalone storage migration is mandated at refinement time. | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: Compact, stable-id-first summaries for local-small-model routing are explicit projection-discipline inputs. | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: No Stage surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: No Studio surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: No Atelier/Lens surface is involved. | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: No training or distillation workflow is involved. | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: No ACE-specific runtime is widened. | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: The projection improves retrieval posture indirectly, but no RAG feature is implemented. | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Derive pillar slices and subfeatures from the current Master Spec; do not invent pillar semantics from memory. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: Software-delivery runtime truth substrate | SUBFEATURES: canonical structured records, workflow-state family, queue-reason code, allowed action ids, gate posture, checkpoint/evidence refs | PRIMITIVES_FEATURES: FEAT-LOCUS-WORK-TRACKING | MECHANICAL: engine.librarian, engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus-backed records remain the join target for DCC, Task Board, and Role Mailbox projections.
  - PILLAR: MicroTask | CAPABILITY_SLICE: Mailbox-linked execution wait projection | SUBFEATURES: micro-task summary, hard-gate state, mailbox wait reason, verifier outcome refs, active session occupancy | PRIMITIVES_FEATURES: FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.context, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Micro-task waits must be visible through compact state rather than transcript replay.
  - PILLAR: Command Center | CAPABILITY_SLICE: Projection discipline and governed action preview | SUBFEATURES: DCC snapshot fields for validator-gate, claim/lease, queued follow-up, recovery, closeout, stale detection, backpressure, and authority refs | PRIMITIVES_FEATURES: FEAT-DEV-COMMAND-CENTER | MECHANICAL: engine.librarian, engine.context, engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: DCC remains the projection/control surface, not the authority.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Workflow-backed start/steer/cancel/close/recover semantics | SUBFEATURES: governed action resolution, workflow binding state, validator-gate phase, checkpoint recovery posture, stable action request ids | PRIMITIVES_FEATURES: FEAT-WORKFLOW-ENGINE | MECHANICAL: engine.dba, engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Control-plane changes must resolve through workflow-backed runtime records.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact summary first projection | SUBFEATURES: compact software-delivery summary rows, authority refs, evidence refs, next action previews, linked mailbox and gate ids | PRIMITIVES_FEATURES: FEAT-LLM-FRIENDLY-DATA | MECHANICAL: engine.context, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Local-small-model routing should consume compact structured records before raw Markdown mirrors.
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Software-delivery projection discipline snapshot | JobModel: UI_ACTION | Workflow: dcc_software_delivery_projection_snapshot | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: work_packet_gated, task_board_synced | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: DCC should read one runtime-backed projection that carries validator-gate, claim/lease, queued-instruction, recovery, closeout, and authority refs.
  - Capability: Task Board runtime-authority projection guard | JobModel: UI_ACTION | Workflow: locus_task_board_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: task_board_status_changed | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Board rows can group or sort work, but validation, ownership, recovery, and closeout posture must come from runtime fields.
  - Capability: Role Mailbox linked-authority preview | JobModel: UI_ACTION | Workflow: role_mailbox_software_delivery_triage | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Mailbox replies and unread order stay advisory until governed action or transcription updates linked runtime records.
  - Capability: Workflow-backed overlay lifecycle projection | JobModel: WORKFLOW | Workflow: software_delivery_overlay_lifecycle | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: workflow_gate_transition | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: start, steer, cancel, close, and recover must expose explicit runtime states and stable ids before mutation.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Current Master Spec v02.181 already contains relevant interaction edges for Dev Command Center, Locus, Task Board, Role Mailbox, Work Packet System, MicroTask, and Workflow Engine. No new IMX edge is required for this WP.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Relevant matrix coverage already exists in v02.181; implementation should consume existing edges rather than declare a spec appendage in this activation.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. For internal/product-governance mirror work, it is valid to mark this section `NOT_APPLICABLE` when no directly topical external combo research is needed. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: The combo space is internal to current Handshake Master Spec v02.181 and local runtime code; no external combo research is needed.
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
  - Combo: DCC snapshot plus Locus runtime truth | Pillars: Command Center, Locus | Mechanical: engine.librarian, engine.version | Primitives/Features: FEAT-DEV-COMMAND-CENTER, FEAT-LOCUS-WORK-TRACKING | Resolution: IN_THIS_WP | Stub: NONE | Notes: One snapshot should expose the same work truth DCC and Locus use.
  - Combo: Task Board rows plus governed action previews | Pillars: Command Center, Locus | Mechanical: engine.sovereign, engine.version | Primitives/Features: FEAT-TASK-BOARD, FEAT-WORKFLOW-ENGINE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Board layout may offer actions only after showing stable governed action ids.
  - Combo: Role Mailbox triage plus linked runtime authority | Pillars: Command Center, MicroTask | Mechanical: engine.context, engine.sovereign | Primitives/Features: FEAT-ROLE-MAILBOX, FEAT-MICRO-TASK-EXECUTOR | Resolution: IN_THIS_WP | Stub: NONE | Notes: Mailbox waits and replies must project linked work state without owning it.
  - Combo: Validator-gate posture plus closeout derivation | Pillars: Execution / Job Runtime, Locus | Mechanical: engine.sovereign, engine.dba | Primitives/Features: FEAT-WORKFLOW-ENGINE, FEAT-LOCUS-WORK-TRACKING | Resolution: IN_THIS_WP | Stub: NONE | Notes: Closeout can become visible only when runtime gate and evidence records allow it.
  - Combo: Claim lease plus queued steering state | Pillars: Execution / Job Runtime, Command Center | Mechanical: engine.version, engine.sovereign | Primitives/Features: FEAT-WORKFLOW-ENGINE, FEAT-DEV-COMMAND-CENTER | Resolution: IN_THIS_WP | Stub: NONE | Notes: Temporary ownership and steer-next intent need durable ids before they affect work.
  - Combo: Compact summary plus local model routing | Pillars: LLM-friendly data, Locus | Mechanical: engine.context, engine.librarian | Primitives/Features: FEAT-LLM-FRIENDLY-DATA, FEAT-LOCUS-WORK-TRACKING | Resolution: IN_THIS_WP | Stub: NONE | Notes: Small models should route from compact summaries, not full packet prose.
  - Combo: Runtime storage plus projection field provenance | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.dba, engine.librarian | Primitives/Features: FEAT-LOCUS-WORK-TRACKING, FEAT-WORKFLOW-ENGINE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Projection fields should say which authoritative record supplied them.
  - Combo: Recovery checkpoint lineage plus stale binding detection | Pillars: Command Center, Execution / Job Runtime | Mechanical: engine.version, engine.context | Primitives/Features: FEAT-DEV-COMMAND-CENTER, FEAT-WORKFLOW-ENGINE | Resolution: IN_THIS_WP | Stub: NONE | Notes: Recovery views need parent checkpoint ids and stale binding indicators.
  - Combo: Backpressure posture plus operator alert projection | Pillars: Command Center, LLM-friendly data | Mechanical: engine.context, engine.sovereign | Primitives/Features: FEAT-DEV-COMMAND-CENTER | Resolution: IN_THIS_WP | Stub: NONE | Notes: Under blocked authority the surface must show backpressure instead of dropping intent.
  - Combo: MicroTask hard gates plus mailbox wait reasons | Pillars: MicroTask, Locus | Mechanical: engine.context, engine.version | Primitives/Features: FEAT-MICRO-TASK-EXECUTOR, FEAT-ROLE-MAILBOX | Resolution: IN_THIS_WP | Stub: NONE | Notes: Micro-task queues should distinguish verifier waits from mailbox-response waits.
  - Combo: Governed action resolution plus UI quick actions | Pillars: Command Center, Execution / Job Runtime | Mechanical: engine.sovereign, engine.librarian | Primitives/Features: FEAT-WORKFLOW-ENGINE, FEAT-DEV-COMMAND-CENTER | Resolution: IN_THIS_WP | Stub: NONE | Notes: Quick actions must preview target records and resolution state before mutation.
  - Combo: Stable projection ids plus cross-surface consistency checks | Pillars: Locus, LLM-friendly data | Mechanical: engine.version, engine.dba | Primitives/Features: FEAT-LOCUS-WORK-TRACKING, FEAT-LLM-FRIENDLY-DATA | Resolution: IN_THIS_WP | Stub: NONE | Notes: Tests should prove the same identifiers survive DCC, Task Board, and mailbox projections.
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations are direct implementation/proof obligations for this WP and require no new stub or spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: Exact target stub, TASK_BOARD, BUILD_ORDER, WP traceability registry, prior DCC/control-plane and structured-collaboration packets, and local product code in ../handshake_main/src/backend/handshake_core.
- MATCHED_STUBS:
  - Artifact: WP-1-Software-Delivery-Projection-Surface-Discipline-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: This is the target stub and should be activated rather than duplicated.
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Dev-Command-Center-Control-Plane-Backend-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: DCC control-plane and compact-summary support exist, but v02.181 software-delivery overlay projection discipline is not proven across DCC, Task Board, and Role Mailbox.
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: Base envelope and mirror posture exist; this WP uses them for software-delivery authority fields.
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: Schema validation exists; this WP adds specific projection discipline proof on top of it.
  - Artifact: WP-1-Role-Mailbox-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: Role Mailbox exists as a structured collaboration surface, but linked software-delivery authority projection remains this WP's slice.
  - Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: Workflow-state family, queue reason, and allowed action ids are reusable inputs for this WP.
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Dev-Command-Center-Control-Plane-Backend-v1 | Covers: execution | Verdict: PARTIAL | Notes: Contains build_dcc_control_plane_snapshot and DCC tests for session binding, mailbox projection, ready queue, and compact summaries, but the v02.181 overlay discipline is not fully represented.
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v4 | Covers: combo | Verdict: PARTIAL | Notes: DccCompactSummaryV1 and schema validation exist as the compact-summary foundation.
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs | Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | Covers: combo | Verdict: PARTIAL | Notes: TaskBoardEntryRecordV1 preserves mirror state, queue reason, workflow-state family, and allowed action ids.
  - Path: ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs | Artifact: WP-1-Role-Mailbox-v1 | Covers: execution | Verdict: PARTIAL | Notes: Structured mailbox exports and authority-boundary checks exist, but software-delivery closeout/gate/claim/queued-instruction projection is not fully proven.
  - Path: ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs | Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: Tests assert queue_reason_code and allowed_action_ids parity for base and mailbox wait cases.
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: Existing packets and code provide the substrate, but none fully implement the v02.181 cross-surface software-delivery projection discipline. Activate this WP rather than create new stubs.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: YES
- UI_UX_REASON_NO: N/A
- UI_SURFACES:
  - Dev Command Center work item detail and queue rows.
  - Task Board derived software-delivery planning rows.
  - Role Mailbox triage rows linked to work packets or micro-tasks.
  - Projection field-provenance and governed-action preview panels.
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Action preview trigger | Type: icon button | Tooltip:Preview target records, governed action id, and required evidence before mutation | Notes: Must not mutate directly.
  - Control: Authority source inspector | Type: icon button | Tooltip:Show canonical runtime fields behind this visible status | Notes: Surfaces field provenance and mirror state.
  - Control: Projection surface switcher | Type: segmented control | Tooltip:Compare DCC, Task Board, and Role Mailbox views of the same work item | Notes: Useful validator spotcheck surface.
  - Control: Claim or lease posture filter | Type: menu | Tooltip:Filter work by claimed, leased, expired, or takeover-eligible posture | Notes: Reads runtime overlay fields only.
  - Control: Queued follow-up filter | Type: menu | Tooltip:Show queued, injected, expired, or rejected steering instructions | Notes: Prevents transcript-only steer-next handling.
  - Control: Closeout eligibility badge | Type: status chip | Tooltip:Explain unresolved gate, evidence, owner, or action blockers | Notes: Badge must be derived, not authoritative.
- UI_STATES (empty/loading/error):
  - Empty state explains that no software-delivery runtime projection exists for this work item yet and offers no fake status.
  - Loading state preserves the last verified timestamp and blocks mutation controls until authority refs load.
  - Error state distinguishes missing canonical runtime record, stale mirror, mailbox-only advisory state, and unresolved governed action.
  - Conflict state shows DCC, Task Board, and Role Mailbox disagreement with canonical runtime state winning.
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Use labels such as Runtime truth, Projection only, Mailbox advisory, Mirror stale, Governed action required, Closeout blocked, and Claim expired.
  - Avoid wording that implies board lane, unread count, packet checklist, or mailbox order is itself authority.
  - Every quick action should name the target governed action or field before the operator confirms it.
- UI_ACCESSIBILITY_NOTES:
  - Tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
  - Status chips must not rely on color alone; include accessible labels for stale, blocked, advisory, and authoritative states.
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: YES
- GUI_ADVICE_REASON_NO: N/A
- GUI_REFERENCE_SCAN:
  - Surface: DCC and Task Board action preview | Source: NONE | Kind: NONE | Pattern: Show the authority source and pending mutation target before offering a row action | HiddenRequirement: Projection controls must read canonical runtime records before enabling mutation | InteractionContract: Quick actions open preview or confirmation with stable ids, target governed action id, and blocker reason | Accessibility: Keyboard focus exposes the same preview and tooltip content | TooltipStrategy: MIXED | EngineeringTrick: Disable mutation controls until authority_refs and evidence_refs are loaded | Resolution: IN_THIS_WP | Stub: NONE | Notes: Internal spec and code truth are sufficient; no external reference needed.
  - Surface: Role Mailbox triage | Source: NONE | Kind: NONE | Pattern: Distinguish mailbox-local actions from governed linked-record actions in the row itself | HiddenRequirement: Reply chronology cannot update linked work state without governed action or transcription | InteractionContract: Reply, acknowledge, snooze, escalate, and request transcription show whether the result is local, governed, or transcription-required | Accessibility: Focus order must not hide the authority distinction behind hover-only content | TooltipStrategy: MIXED | EngineeringTrick: Carry action_request_id and linked work ids into every triage row | Resolution: IN_THIS_WP | Stub: NONE | Notes: This implements v02.181 mailbox authority law directly.
- HANDSHAKE_GUI_ADVICE:
  - Surface: Dev Command Center | Control: Authority source inspector | Type: icon button | Why: Operators need to see whether a status came from runtime, Task Board mirror, or mailbox advisory state | Microcopy: Runtime truth | Tooltip: Show canonical record ids and evidence refs for this status
  - Surface: Task Board | Control: Lane action preview | Type: icon button | Why: Drag or row actions must preview transition legality before state changes | Microcopy: Preview move | Tooltip: Show transition rule, target workflow state, and blockers
  - Surface: Role Mailbox | Control: Reply authority indicator | Type: status chip | Why: Mailbox replies can be local or governed-linked and must not be confused | Microcopy: Mailbox local or Governed | Tooltip: Explain whether this reply can affect linked work
- HIDDEN_GUI_REQUIREMENTS:
  - Mutation controls remain disabled when canonical runtime fields are absent or stale even if the visible mirror suggests work is ready.
  - Cross-surface conflict state must name DCC, Task Board, and Role Mailbox values while marking canonical runtime state as winning.
  - Closeout controls must show unresolved gate/evidence/owner/action blockers before allowing a close request.
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - Keep projection rows compact but include expandable authority refs and evidence refs.
  - Store action preview payloads as structured data so validators can assert legal transitions without screenshot inspection.
  - Emit one test fixture where DCC, Task Board, and Role Mailbox disagree and runtime truth wins.
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PASS

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: NO

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: ACTIVATION_MANAGER
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.181]
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Project-Agnostic-Workflow-State-Registry, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Role-Mailbox
- BUILD_ORDER_BLOCKS: NONE
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.181.md v02.181 projection-surface discipline and software-delivery runtime truth specialization
- WHAT: Implement and prove the software-delivery projection-discipline contract that keeps Dev Command Center, Task Board, and Role Mailbox as projections over one runtime-backed truth.
- WHY: v02.181 forbids DCC layout position, Task Board mirrors, Role Mailbox chronology, packet prose, or repo /.GOV artifacts from becoming hidden authority for validation, ownership, recovery, queued steering, or closeout posture.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - Cosmetic UI redesign or broad layout registry work.
  - New external research or third-party workflow framework integration.
  - Making repo /.GOV mirrors, packet prose, board lanes, or mailbox chronology canonical product runtime authority.
  - Official packet creation, signature recording, coder launch, or validator launch during this pre-signature refinement pass.
- TEST_PLAN:
  ```bash
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_software_delivery_projection_cannot_override_runtime_truth -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox_software_delivery_triage_remains_advisory -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_projection_requires_gate_evidence_and_owner_truth -- --exact
  just gov-check
  ```
- DONE_MEANS:
  - DCC, Task Board, and Role Mailbox projections for one software-delivery work item expose the same runtime truth by stable identifiers.
  - Projection rows expose validator-gate, governed-action, claim/lease, queued-instruction, checkpoint/recovery, evidence, stale, and closeout posture from canonical runtime records.
  - Board lane, unread mailbox state, transcript order, packet prose, and repo /.GOV mirrors cannot authorize validation, ownership, recovery, queued steering, or closeout.
  - Governed action previews name target records and action ids before mutation.
  - Tests include a conflict case where DCC, Task Board, and Role Mailbox advisory states disagree and runtime truth wins.
- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/task_packets/stubs/WP-1-Software-Delivery-Projection-Surface-Discipline-v1.md
  - .GOV/spec/Handshake_Master_Spec_v02.181.md
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- SEARCH_TERMS:
  - build_dcc_control_plane_snapshot
  - DccCompactSummaryV1
  - TaskBoardEntryRecordV1
  - queue_reason_code
  - allowed_action_ids
  - role_mailbox
  - validator_gate
  - claim lease queued instruction closeout checkpoint
- RUN_COMMANDS:
  ```bash
  rg -n "build_dcc_control_plane_snapshot|DccCompactSummaryV1|TaskBoardEntryRecordV1|queue_reason_code|allowed_action_ids|role_mailbox|validator_gate|claim|lease|queued|closeout|checkpoint" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_software_delivery_projection_cannot_override_runtime_truth -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox_software_delivery_triage_remains_advisory -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_projection_requires_gate_evidence_and_owner_truth -- --exact
  just gov-check
  ```
- RISK_MAP:
  - "Projection rows become a second authority" -> "Operators can close, validate, or reroute work from stale display state."
  - "Mailbox chronology is treated as completion" -> "Completion or ownership can be inferred without governed action proof."
  - "Task Board lane placement outranks runtime truth" -> "Validation and closeout posture can drift from gate/evidence records."
  - "Stable ids are missing from summaries" -> "DCC, Task Board, and mailbox views cannot prove they describe the same work item."
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - BUILD_ORDER already contains the stub with SPEC_TARGET Handshake_Master_Spec_v02.181.md and the declared dependency set; no build-order mutation is required for this pre-signature refinement.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: v02.181 Projection-surface discipline | WHY_IN_SCOPE: This is the exact stub target: DCC, Task Board, and Role Mailbox projections must explain one underlying software-delivery runtime state without making mirrors or chronology authoritative | EXPECTED_CODE_SURFACES: workflows.rs, locus/types.rs, locus/task_board.rs, role_mailbox.rs, runtime_governance.rs | EXPECTED_TESTS: dcc_software_delivery_projection_surface_keeps_runtime_authority | RISK_IF_MISSED: Display state silently becomes operational authority.
  - CLAUSE: Software-delivery overlay runtime truth specialization | WHY_IN_SCOPE: Software-delivery work meaning must resolve through canonical structured records rather than packet prose, board ordering, mailbox chronology, or side ledgers | EXPECTED_CODE_SURFACES: locus/types.rs, locus/task_board.rs, workflows.rs | EXPECTED_TESTS: task_board_software_delivery_projection_cannot_override_runtime_truth | RISK_IF_MISSED: Board or packet edits can mask invalid runtime state.
  - CLAUSE: Software-delivery closeout derivation | WHY_IN_SCOPE: Closeout must be derived from workflow, validator-gate, governed-action, owner, and evidence truth | EXPECTED_CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs | EXPECTED_TESTS: closeout_projection_requires_gate_evidence_and_owner_truth | RISK_IF_MISSED: Work can look complete while gates or evidence are unresolved.
  - CLAUSE: Software-delivery overlay extension records and lifecycle semantics | WHY_IN_SCOPE: Claim/lease and queued instruction posture must be explicit and stable-id backed | EXPECTED_CODE_SURFACES: workflows.rs, locus/types.rs, role_mailbox.rs | EXPECTED_TESTS: projection_surface_exposes_claim_and_queued_instruction_ids | RISK_IF_MISSED: Ownership and steer-next intent are inferred from comments or mailbox order.
  - CLAUSE: Role Mailbox authority boundary | WHY_IN_SCOPE: Mailbox replies and triage rows may inform linked work but cannot mutate authoritative state without governed action or transcription | EXPECTED_CODE_SURFACES: role_mailbox.rs, api/role_mailbox.rs, workflows.rs | EXPECTED_TESTS: role_mailbox_software_delivery_triage_remains_advisory | RISK_IF_MISSED: A reply or unread badge can substitute for workflow state or completion evidence.

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: Software-delivery projection summary payload | PRODUCER: DCC/workflow projection builder | CONSUMER: DCC UI, Task Board comparison view, Role Mailbox triage, validators | SERIALIZER_TRANSPORT: structured runtime summary keyed by work_packet_id and workflow_run_id | VALIDATOR_READER: DCC projection tests | TRIPWIRE_TESTS: dcc_software_delivery_projection_surface_keeps_runtime_authority | DRIFT_RISK: Surfaces disagree about status, owner, gate, or closeout.
  - CONTRACT: Task Board projection row authority fields | PRODUCER: Locus task-board projection layer | CONSUMER: Task Board derived layouts and DCC planning views | SERIALIZER_TRANSPORT: TaskBoardEntryRecordV1 plus software-delivery profile extension | VALIDATOR_READER: task-board projection tests | TRIPWIRE_TESTS: task_board_software_delivery_projection_cannot_override_runtime_truth | DRIFT_RISK: Lane placement or mirror state becomes hidden authority.
  - CONTRACT: Role Mailbox linked-authority triage row | PRODUCER: Role Mailbox export/projection layer | CONSUMER: Role Mailbox triage and DCC collaboration inbox | SERIALIZER_TRANSPORT: structured mailbox index/thread records with linked work ids and action_request_id | VALIDATOR_READER: role-mailbox projection tests | TRIPWIRE_TESTS: role_mailbox_software_delivery_triage_remains_advisory | DRIFT_RISK: Mailbox chronology or summary text mutates linked work.
  - CONTRACT: Governed action preview payload | PRODUCER: workflow/governed action registry and DCC projection layer | CONSUMER: DCC quick actions, Task Board row actions, mailbox escalation controls | SERIALIZER_TRANSPORT: preview record with action_request_id, target record refs, eligibility, blockers, and evidence refs | VALIDATOR_READER: governed action projection tests | TRIPWIRE_TESTS: projection_surface_previews_governed_action_before_mutation | DRIFT_RISK: UI actions skip policy, approval, or evidence gates.
  - CONTRACT: Closeout and recovery posture payload | PRODUCER: workflow runtime, validator-gate records, checkpoint lineage | CONSUMER: DCC close/recover controls, Task Board badges, Role Mailbox follow-up | SERIALIZER_TRANSPORT: structured closeout/recovery summary with gate_record_id, checkpoint_id, unresolved blockers, and authority refs | VALIDATOR_READER: closeout/recovery tests | TRIPWIRE_TESTS: closeout_projection_requires_gate_evidence_and_owner_truth | DRIFT_RISK: Recovery or closeout depends on transcript reconstruction or packet surgery.

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_software_delivery_projection_cannot_override_runtime_truth -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox_software_delivery_triage_remains_advisory -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml projection_surface_previews_governed_action_before_mutation -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_projection_requires_gate_evidence_and_owner_truth -- --exact`
- CANONICAL_CONTRACT_EXAMPLES:
  - Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth.
  - Example Task Board row with stale mirror state but runtime validator-gate blocker winning.
  - Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution.
  - Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id.
  - Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action.

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Inspect current DCC snapshot, DccCompactSummaryV1, TaskBoardEntryRecordV1, Role Mailbox export, workflow-state-family, queue-reason, and allowed-action code before adding new fields.
  - Define the minimal software-delivery profile extension or summary payload needed to carry validator-gate, governed-action, claim/lease, queued-instruction, recovery, evidence, and closeout posture.
  - Wire DCC, Task Board, and Role Mailbox projections to read the same runtime-backed fields by stable identifiers.
  - Add tests with intentional DCC/Task Board/mailbox advisory disagreement where canonical runtime truth wins.
  - Keep repo /.GOV mirrors, Markdown packet prose, board lanes, and mailbox chronology as readable/advisory inputs only.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - `dcc_software_delivery_projection_surface_keeps_runtime_authority`
  - `task_board_software_delivery_projection_cannot_override_runtime_truth`
  - `role_mailbox_software_delivery_triage_remains_advisory`
  - `projection_surface_previews_governed_action_before_mutation`
  - `closeout_projection_requires_gate_evidence_and_owner_truth`
- CARRY_FORWARD_WARNINGS:
  - Do not create a second DCC-only or board-only truth store.
  - Do not treat mailbox replies, unread badges, transcript order, packet prose, or mirror freshness as authority.
  - Keep stable identifiers and authority_refs/evidence_refs visible enough for validators to inspect.
  - If implementation discovers missing base schema support, report bounded spec/stub need rather than silently broadening scope.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - v02.181 projection-surface discipline.
  - v02.181 software-delivery overlay runtime truth specialization.
  - v02.181 closeout derivation.
  - v02.181 overlay extension records and lifecycle semantics.
  - v02.173 Role Mailbox authority boundary.
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- COMMANDS_TO_RUN:
  - `rg -n "build_dcc_control_plane_snapshot|DccCompactSummaryV1|TaskBoardEntryRecordV1|queue_reason_code|allowed_action_ids|role_mailbox|validator_gate|claim|lease|queued|closeout|checkpoint" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_software_delivery_projection_cannot_override_runtime_truth -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox_software_delivery_triage_remains_advisory -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_projection_requires_gate_evidence_and_owner_truth -- --exact`
  - `just gov-check`
- POST_MERGE_SPOTCHECKS:
  - Verify DCC, Task Board, and Role Mailbox projection rows expose the same canonical work_packet_id and workflow_run_id.
  - Verify a stale Task Board mirror and advisory mailbox reply cannot override validator-gate or closeout blockers.
  - Verify governed action previews expose target record ids and blockers before mutation.
  - Verify recovery and queued steering posture remain stable-id backed and do not require transcript replay.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Exact final field names for claim/lease, queued instruction, validator-gate, recovery, and closeout projection payloads are not proven until implementation inspects current struct boundaries.
  - Whether the best landing surface is a software-delivery profile extension, DccCompactSummaryV1 extension, or workflow projection helper split is not proven yet.
  - No product tests have been run in this pre-signature refinement pass.

### DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (The WP composes existing DCC, Locus, Task Board, Role Mailbox, workflow, and compact-summary contracts; no new primitive ID is needed.)
- DISCOVERY_STUBS: NONE_CREATED (The target stub already captures this high-ROI slice and existing dependencies cover adjacent work.)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (v02.181 already contains the relevant DCC/Locus/Task Board/Role Mailbox/Workflow matrix edges.)
- DISCOVERY_UI_CONTROLS: Action preview trigger; Authority source inspector; Projection surface switcher; Claim or lease posture filter; Queued follow-up filter; Closeout eligibility badge.
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED (Current Master Spec v02.181 clearly covers the required projection discipline.)
- DISCOVERY_JUSTIFICATION: The refinement discovered that local product code has a strong partial substrate but lacks a single proof slice for v02.181 software-delivery projection discipline; the value is scope consolidation, runtime-authority proof, UI/control guardrails, and exact handoff tests rather than new primitives or spec text.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.181 explicitly requires software-delivery runtime truth, projection-surface discipline, closeout derivation, overlay claim/lease and queued-instruction records, and workflow-backed start/steer/cancel/close/recover semantics. The stub and this refinement map those clauses to concrete DCC, Task Board, Role Mailbox, Locus, workflow, and test surfaces.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The current Master Spec v02.181 already names the projection-surface discipline and the required software-delivery runtime truth, closeout, overlay coordination, and lifecycle semantics. This WP is an implementation/proof activation, not a spec repair.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 10.11 Dev Command Center integration [ADD v02.181]
- CONTEXT_START_LINE: 6678
- CONTEXT_END_LINE: 6680
- CONTEXT_TOKEN: mirror freshness alone
- EXCERPT_ASCII_ESCAPED:
  ```text
  - DCC is the canonical operator/developer surface to **view** Locus WPs/MTs and bind a **worktree-backed workspace** to a `wp_id`/`mt_id`/`session_id` context.
  - DCC MUST NOT become an alternate authority for work status; it MUST read/write via `locus_*` operations and treat `.handshake/gov/TASK_BOARD.md` as the human-readable mirror.
  - [ADD v02.181] For `project_profile_kind=software_delivery`, Dev Command Center SHOULD project work contract state, workflow-binding state, pending governed actions, validator-gate posture, checkpoint lineage, evidence readiness, claim/lease posture, queued follow-up instructions, binding health, stale detection, and backpressure posture from canonical runtime records. Dev Command Center MAY start, steer, cancel, close, or recover those records only through governed actions or workflow-backed control-plane mutations, and it MUST NOT infer authority from layout position, unread state, or mirror freshness alone.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 2.6.8.8 compact summary and structured projection contract
- CONTEXT_START_LINE: 6878
- CONTEXT_END_LINE: 6888
- CONTEXT_TOKEN: compact summary contract
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Dev Command Center, Task Board derived layouts, Role Mailbox triage, and local-small-model planning flows SHOULD default to the compact summary contract first and load canonical detail records or Markdown sidecars only on demand.

  **Task Board and Role Mailbox structured projections** [ADD v02.168]

  - Task Board projection rows SHOULD be serialized as `record_kind=task_board_entry` records that reuse the same base envelope and add only board-specific fields such as `task_board_id`, `work_packet_id`, `lane_id`, `display_order`, and optional `view_ids`.
  - Role Mailbox exports SHOULD reuse the same base envelope for thread indexes and append-only thread lines. Message-level records SHOULD add only mailbox-specific fields such as `thread_id`, `message_type`, `from_role`, `to_roles`, `expected_response`, and `expires_at`.
  - When a collaboration artifact supports both canonical detail and compact summary representations, both records MUST share the same `record_id`, `project_profile_kind`, and authoritative references so deterministic joins remain possible without transcript reconstruction or Markdown parsing.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 2.6.8.8 software-delivery overlay runtime truth specialization [ADD v02.181]
- CONTEXT_START_LINE: 6915
- CONTEXT_END_LINE: 6925
- CONTEXT_TOKEN: authoritative work meaning
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Software-delivery overlay runtime truth specialization** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, authoritative work meaning MUST resolve through canonical structured records instead of packet prose, board ordering, mailbox chronology, or side-ledger files.
  - Software-delivery structured collaboration state MUST preserve, at minimum, canonical truth for scoped work contract semantics, workflow binding semantics, governed action request/resolution posture, validator-gate posture, and checkpoint/evidence references.
  - Readable task-packet Markdown, Task Board mirrors, and mailbox summaries MAY remain source artifacts and human-readable projections, but they MUST NOT act as the mutable operational ledger for software-delivery execution.
  - Software-delivery-specific fields SHOULD remain profile extensions or profile-specialized records over the shared base envelope so the shared parser, compact summary contract, and validator surface stay reusable across project kinds.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 2.6.8.8 software-delivery closeout, overlay records, and control-plane behaviors [ADD v02.181]
- CONTEXT_START_LINE: 7032
- CONTEXT_END_LINE: 7058
- CONTEXT_TOKEN: closeout_pending
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Software-delivery closeout derivation** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, authoritative closeout MUST be derived from canonical workflow state, validator-gate posture, governed action resolutions, and evidence references rather than from packet-local checklist surgery, board reshuffling, or manual side-ledger convergence.
  - Human-readable closeout sections, packets, and board badges MAY be synchronized after authoritative closeout becomes true, but they MUST NOT define closeout legality on their own.

  **Software-delivery overlay extension records and lifecycle semantics** [ADD v02.181]

  - When `project_profile_kind=software_delivery` requires bounded temporary ownership, takeover policy, or steer-next behavior, canonical runtime state SHOULD expose `GovernanceClaimLeaseRecord` and `GovernanceQueuedInstructionRecord` or equivalent stable overlay records keyed by `work_packet_id`, `workflow_run_id`, `workflow_binding_id`, `model_session_id`, or other canonical runtime identifiers.
  - Software-delivery workflow bindings SHOULD preserve explicit states `created`, `queued`, `claimed`, `node_active`, `approval_wait`, `validation_wait`, `closeout_pending`, `settled`, `failed`, and `canceled`. `approval_wait` requires unresolved governed actions, `validation_wait` requires active validator-gate records, and `closeout_pending` is derived from canonical runtime truth rather than packet prose.

  **Software-delivery overlay control-plane behaviors** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, start, steer, cancel, close, and recover MUST resolve through workflow-backed governed actions and canonical runtime records instead of repo ledgers, mailbox chronology, or transcript-only intent.
  - Software-delivery control-plane state SHOULD preserve health posture, stale-detection posture, backpressure posture, and operator-alert posture by stable runtime identifiers. Under load or blocked authority, the system MUST surface backpressure explicitly instead of silently dropping or reordering control-plane intent.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 2.6.8.10 Role Mailbox authority boundary [ADD v02.173]
- CONTEXT_START_LINE: 7061
- CONTEXT_END_LINE: 7069
- CONTEXT_TOKEN: mailbox chronology
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Role Mailbox message contract, thread lifecycle, and authority boundary** [ADD v02.173]

  - Role Mailbox SHALL separate thread lifecycle from message delivery state. At minimum, thread lifecycle MUST distinguish `open`, `awaiting_response`, `waiting_on_linked_authority`, `escalated`, `resolved`, `expired`, and `archived`, while message delivery MUST distinguish `queued`, `delivered`, `acknowledged`, `replied`, `ignored`, `failed`, and `dead_lettered`.
  - Every actionable mailbox message SHOULD expose a bounded action-request envelope that declares allowed responses, due or expiry posture, optional snooze posture, and the stable linked record identifiers that the message refers to.
  - Replying to a mailbox thread MUST NOT silently mutate Work Packet, Micro-Task, Task Board, or Locus authority. Any linked change MUST resolve through a governed action, transition rule, or explicit transcription into the authoritative artifact.
  - Local-small-model and cloud-model routing MAY consume compact mailbox summaries, but mailbox chronology, unread badges, or transcript order MUST NOT become substitutes for workflow state, dependency state, or completion evidence.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md Phase 1 roadmap projection-surface discipline coverage
- CONTEXT_START_LINE: 48042
- CONTEXT_END_LINE: 48048
- CONTEXT_TOKEN: Projection-surface discipline
- EXCERPT_ASCII_ESCAPED:
  ```text
  - [ADD v02.181] Software-delivery runtime truth: at least one workflow-backed software-delivery work item exposes product-owned runtime state and linked governed actions by stable identifiers instead of relying on packet prose, mailbox order, or Markdown mirrors as the operational authority surface.
  - [ADD v02.181] Validator-gate and closeout posture: at least one workflow-backed software-delivery work item exposes validator-gate summaries, evidence-linked gate posture, and derived closeout posture by stable identifiers without requiring packet surgery to explain why the item may proceed or close.
  - [ADD v02.181] Projection-surface discipline: Dev Command Center, Task Board, and Role Mailbox projections for at least one software-delivery work item explain the same underlying state without turning repo `/.GOV/**`, Markdown mirrors, or mailbox chronology into authority.
  - [ADD v02.181] Overlay coordination records: at least one software-delivery work item exposes overlay claim/lease state and queued steering or follow-up state by stable identifiers so actor ownership, takeover legality, and deferred steering are visible without transcript reconstruction.
  - [ADD v02.181] Overlay lifecycle and recovery posture: at least one software-delivery work item exposes checkpoint-backed recovery posture plus workflow-backed start/steer/cancel/close/recover semantics by stable identifiers so restart-safe replay and control decisions remain inspectable.
  ```
