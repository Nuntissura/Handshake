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
- WP_ID: WP-1-Project-Agnostic-Workflow-State-Registry-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-13T00:01:25Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- SPEC_TARGET_SHA1: 7d6558fab2f3df70669fff6f0a6e6ef9ea395194
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja130420260159
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Project-Agnostic-Workflow-State-Registry-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Local product code already emits `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`, but the implementation only covers a narrower reason-code set than the current v02.180 contract.
- `GovernedActionDescriptorV1` exists only as a thin record shape in `../handshake_main/src/backend/handshake_core/src/locus/types.rs`; the richer spec-defined action semantics still live in helper logic instead of a durable registry-backed contract.
- The current product backend does not expose code-backed equivalents for `ProjectProfileWorkflowExtensionV1`, `WorkflowTransitionRuleV1`, `QueueAutomationRuleV1`, or `ExecutorEligibilityPolicyV1`, even though the spec now requires those surfaces or an equivalent durable contract keyed by stable ids.
- Mailbox-linked waits are reflected indirectly in compact summary and blocker prose, but the linked record posture is not yet promoted to the canonical queue-reason contract the spec expects.
- `allowed_action_ids` logic is duplicated in both `workflows.rs` and `storage/locus_sqlite.rs`, which creates drift risk before the portable registry semantics are complete.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: current Master Spec v02.180 workflow-state and transition clauses, current target stub, current product-governance sibling refinements for boundary alignment, and current local product code under `../handshake_main/src/backend/handshake_core`
- REFERENCES: `.GOV/spec/Handshake_Master_Spec_v02.180.md`, `.GOV/task_packets/stubs/WP-1-Project-Agnostic-Workflow-State-Registry-v1.md`, `.GOV/refinements/WP-1-Project-Profile-Extension-Registry-v1.md`, `.GOV/refinements/WP-1-Governance-Workflow-Mirror-v2.md`, `../handshake_main/src/backend/handshake_core/src/locus/types.rs`, `../handshake_main/src/backend/handshake_core/src/workflows.rs`, `../handshake_main/src/backend/handshake_core/src/locus/task_board.rs`, `../handshake_main/src/backend/handshake_core/src/runtime_governance.rs`, `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs`, `../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- PATTERNS_EXTRACTED: keep one portable workflow vocabulary across artifact families; let project-profile overlays relabel display text without changing base semantics; require queue posture and allowed actions to resolve from stable identifiers instead of viewer-local lane names or mailbox order
- DECISIONS ADOPT/ADAPT/REJECT: adopt the current spec-defined workflow families, queue reasons, and governed action ids as the contract of record; adapt the existing helper-based action plumbing into registry-backed backend surfaces; reject any approach where board labels, mailbox thread order, or freeform notes become workflow authority
- LICENSE/IP_NOTES: Local spec and local product code review only. No third-party code or text is intended for reuse.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.180.md already defines the project-agnostic workflow-state, queue-reason, governed-action, transition, automation, and executor-eligibility contract. The remaining gap is product implementation and proof.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This packet is already fully grounded by the current Master Spec and local product backend reality. External research would not materially change the activation boundary or the required product work.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Current v02.180 spec text is already specific enough to decide scope without external comparison.
  - Local product code shows a concrete implementation gap: partial field emission exists, but the full registry-backed contract does not.
- RESEARCH_GAPS_TO_TRACK:
  - Downstream Dev Command Center visual treatment of workflow labels and rule provenance remains separate from this backend packet.
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
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new Flight Recorder event family is required for this packet.
- Existing Locus, Task Board, runtime-governance, and mailbox export seams remain the relevant observability boundary.
- If transition, automation, or eligibility registry rows are added in code, later product telemetry should reuse existing work-state and queue-change event families instead of inventing a packet-local event taxonomy here.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: viewers or operators over-trust current `allowed_action_ids` output even though action semantics are still helper-defined rather than registry-backed. Mitigation: make the packet prove stable action descriptors and remove duplicated ad hoc action maps.
- Risk: mailbox or board posture backdoors canonical state through lane labels or thread ordering. Mitigation: keep `workflow_state_family` and `queue_reason_code` authoritative and explicit on the linked record itself.
- Risk: project-profile relabeling starts changing workflow meaning rather than display language. Mitigation: constrain overlay semantics to label and narrowing behavior only, never base-family reinterpretation.
- Risk: transition, automation, and executor-eligibility semantics are implied socially instead of exposed as durable ids. Mitigation: require explicit backend surfaces or equivalent durable contracts before closure.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-WorkflowTransitionRuleV1
  - PRIM-QueueAutomationRuleV1
  - PRIM-ExecutorEligibilityPolicyV1
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-WorkflowTransitionRuleV1
  - PRIM-QueueAutomationRuleV1
  - PRIM-ExecutorEligibilityPolicyV1
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - The spec already defines the relevant primitive families. This packet activates the existing portable workflow-state contract in product code instead of minting new Appendix 12 primitives.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already contains the workflow-state, queue-reason, governed-action, workflow-extension, transition, automation, and executor-eligibility primitive rows this packet uses.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Keep Appendix 12.4 unchanged and implement against the existing primitive ids.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new primitive family was discovered in this pass.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Current feature registry coverage already names the Locus, Task Board, Role Mailbox, Micro-Task, Work Packet, and Dev Command Center surfaces this packet depends on.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet is backend and contract focused. Any new operator-facing workflow controls remain downstream of existing Dev Command Center UI work.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Current Appendix 12.6 interaction coverage is sufficient for this implementation pass; the gap is backend realization of existing workflow-state law rather than a missing new interaction class.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep the current spec unchanged and implement against existing v02.180 clauses.
  - If coding reveals a genuinely missing durable contract surface, route that as a later spec-update flow instead of widening this packet silently.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability changes here | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface changes here | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physical simulation or measurement law is involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes may consume the workflow contract later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing execution surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: TOUCHED | NOTES: the workflow registry defines the portable action and queue posture orchestration later consumes | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication/export viewers remain downstream consumers only | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food compliance surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: durable packet, board, mailbox, and compact-summary artifacts all depend on the same portable workflow metadata | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval remains downstream of this registry work | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytics can consume this vocabulary later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: the packet touches durable record shape and persistence drift risk in SQLite-backed workflow projections | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this packet implements existing governance law and does not create new authority posture | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or explanation interface is implemented here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: local-small-model and generic readers depend on stable family and reason fields instead of prose or layout names | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: registry-backed workflow and executor contracts depend on stable ids and durable versioned surfaces | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no isolation behavior changes are required | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: no new event taxonomy is introduced here | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: no calendar-facing surface depends directly on this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus owns the shared workflow-state and validation seams this packet closes | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: storage portability remains separate from this activation | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: work-packet artifacts are direct consumers of the portable workflow-state contract, but this packet closes the shared backend registry beneath them rather than expanding the Work Packets pillar as its own product surface | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: task-board rows and views consume the same family, reason, and action posture, but this packet repairs the shared backend semantics rather than opening a board-specific product-surface packet | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: micro-task detail and summary artifacts are in the same shared workflow family scope | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: this packet defines backend semantics Command Center later consumes, but does not implement the UI surface | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: executor eligibility and queue posture are runtime-facing backend semantics in scope | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: prompt generation is downstream of the portable workflow contract | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: persistence portability is unchanged beyond eliminating duplicated workflow helper drift | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: local-small-model routing explicitly depends on low-cardinality family and reason fields | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no media staging workflow is affected | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio runtime or creative console behavior is affected | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design or capture surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no training or distillation pipeline depends directly on this work | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE execution surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers of the shared workflow metadata | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: portable workflow-state registry enforcement | SUBFEATURES: family enum coverage, queue-reason coverage, governed action descriptors, durable transition and eligibility ids | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-LOCUS-WORK-TRACKING | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the core contract belongs in product backend validation and artifact emission logic
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: packet-detail workflow parity over the shared registry | SUBFEATURES: packet family fields, packet queue posture, packet governed next actions | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-WORK-PACKET-SYSTEM | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: packet artifacts are first-order consumers of the shared registry even though the pillar itself is not widened as a separate product lane
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: queue posture parity on entry/index/view records | SUBFEATURES: family badge source truth, queue reason parity, governed next-action previews | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-TASK-BOARD | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: board projections must consume the same backend workflow vocabulary instead of helper-only lane semantics
  - PILLAR: MicroTask | CAPABILITY_SLICE: micro-task detail and summary workflow parity | SUBFEATURES: compact summary readiness, governed next-action parity, durable waiting posture | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task artifacts already carry partial workflow posture and need the full registry-backed contract
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: durable transition, automation, and executor eligibility posture | SUBFEATURES: rule ids, automation ids, executor policy ids, local-small-model readiness posture | PRIMITIVES_FEATURES: PRIM-WorkflowTransitionRuleV1, PRIM-QueueAutomationRuleV1, PRIM-ExecutorEligibilityPolicyV1, FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.director, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: executor routing should be backed by stable contract ids rather than inferred from prose
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: low-cardinality workflow routing substrate | SUBFEATURES: ready-vs-waiting posture, local-small-model reason routing, display-label degradation to base families | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-ProjectProfileWorkflowExtensionV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: small-model routing should depend on compact family and reason fields, not prose or viewer-local labels
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: workflow-state registry enforcement | JobModel: NONE | Workflow: structured_collaboration_validation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: shared backend validation logic, not a standalone operator tool
  - Capability: governed action and transition parity across packet and board projections | JobModel: WORKFLOW | Workflow: task_board_projection_refresh | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: board rows, packet artifacts, and compact summaries should expose the same action posture
  - Capability: executor eligibility and mailbox-linked wait posture | JobModel: WORKFLOW | Workflow: runtime_governance_queue_refresh | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox-linked waits must remain explicit queue reasons on the linked record rather than viewer-local annotations
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Existing Appendix 12.6 interaction coverage is sufficient for this activation. The immediate gap is backend realization of already-declared workflow and executor contracts.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: No new cross-feature interaction edge must be added before this packet can activate. The packet should implement the already-declared workflow-state contract in current product surfaces.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: Local current-spec and local product code were sufficient to resolve the backend implementation gap and packet boundary.
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
  - Combo: canonical family and reason parity in packet and micro-task artifacts | Pillars: Locus, MicroTask | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, FEAT-WORK-PACKET-SYSTEM, FEAT-MICRO-TASK-EXECUTOR | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the core activation target and should not be split again
  - Combo: governed action registry replaces helper-only action maps | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.director, engine.version | Primitives/Features: PRIM-GovernedActionDescriptorV1, FEAT-LOCUS-WORK-TRACKING | Resolution: IN_THIS_WP | Stub: NONE | Notes: action legality needs one durable source of truth
  - Combo: workflow transition rule ids back runtime queue moves | Pillars: Execution / Job Runtime | Mechanical: engine.director, engine.version | Primitives/Features: PRIM-WorkflowTransitionRuleV1, FEAT-MICRO-TASK-EXECUTOR | Resolution: IN_THIS_WP | Stub: NONE | Notes: state movement should stop being inferred from helper branches alone
  - Combo: queue automation rule ids explain non-manual posture changes | Pillars: Execution / Job Runtime | Mechanical: engine.director, engine.context | Primitives/Features: PRIM-QueueAutomationRuleV1, FEAT-ROLE-MAILBOX | Resolution: IN_THIS_WP | Stub: NONE | Notes: automation must remain visible and approval-safe
  - Combo: executor eligibility policy gates local-small-model pickup | Pillars: Execution / Job Runtime, LLM-friendly data | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-ExecutorEligibilityPolicyV1, PRIM-WorkflowQueueReasonCode | Resolution: IN_THIS_WP | Stub: NONE | Notes: local routing should depend on stable queue posture and explicit executor limits
  - Combo: mailbox-linked waits become canonical queue reasons on linked records | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.context, engine.archivist | Primitives/Features: PRIM-WorkflowQueueReasonCode, FEAT-ROLE-MAILBOX | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox threads must inform waiting posture without becoming the authority themselves
  - Combo: project-profile workflow label overrides degrade cleanly to base semantics | Pillars: Locus, LLM-friendly data | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-ProjectProfileWorkflowExtensionV1, PRIM-WorkflowStateFamily | Resolution: IN_THIS_WP | Stub: NONE | Notes: display labels may vary, but routing and meaning must remain portable
  - Combo: compact summaries preserve the same governed next-action posture as detail records | Pillars: MicroTask, LLM-friendly data | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-GovernedActionDescriptorV1, FEAT-MICRO-TASK-EXECUTOR | Resolution: IN_THIS_WP | Stub: NONE | Notes: summary consumers should not see a smaller or weaker action vocabulary
  - Combo: durable storage stops duplicating family-to-action legality maps | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.dba, engine.version | Primitives/Features: PRIM-GovernedActionDescriptorV1, PRIM-WorkflowStateFamily | Resolution: IN_THIS_WP | Stub: NONE | Notes: persistence-backed emitters should not maintain a shadow legality table
  - Combo: task-board and compact-summary consumers stay mirrors over the same backend workflow contract | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.dba | Primitives/Features: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, FEAT-TASK-BOARD | Resolution: IN_THIS_WP | Stub: NONE | Notes: consumer views should inherit shared semantics from backend records rather than lane-local state
  - Combo: packet, micro-task, and runtime-governance paths all read the same versioned workflow contract | Pillars: Locus, MicroTask, Execution / Job Runtime | Mechanical: engine.version, engine.context | Primitives/Features: PRIM-WorkflowStateFamily, PRIM-GovernedActionDescriptorV1, PRIM-ExecutorEligibilityPolicyV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: this closes the remaining semantic drift between emitters, summaries, and queue consumers
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered here fit inside this packet's existing stub boundary and do not require a new stub or spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, adjacent product-governance refinements, current Master Spec v02.180, and local product code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this is the correct packet shell for the remaining implementation gap
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Project-Profile-Extension-Registry-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: profile-extension registry remains a sibling contract; this packet only uses its workflow overlay semantics
  - Artifact: WP-1-Governance-Workflow-Mirror-v2 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: the governance mirror consumes portable workflow posture but is not the authority for this registry
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | Covers: primitive | Verdict: PARTIAL | Notes: family and reason enums exist, but reason coverage is narrower than the current spec and `GovernedActionDescriptorV1` is still minimal
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: `allowed_action_ids` are emitted, but the action map is helper-driven and duplicated rather than registry-backed
  - Path: ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs | Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: compact summaries preserve family and reason posture, but mailbox-linked waits still surface mainly as blocker prose and counts
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs | Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: persistence logic duplicates family-to-action mapping and increases drift risk
  - Path: ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs | Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: tests prove `allowed_action_ids` validation and packet-row parity, but not the full v02.180 registry vocabulary or durable rule families
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The existing stub is the right packet shell and the spec already covers the contract. Product code remains partial, so this packet should expand and close the implementation gap without creating another stub.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet is backend and control-plane focused. It prepares workflow metadata that downstream viewers can consume, but it does not implement a new GUI surface directly.
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
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet. Any workflow-state inspector, label-mapping explainer, or rule provenance UI remains downstream of existing Dev Command Center work.
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
- AGENT_ID: ActivationManager
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.171]
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry, WP-1-Project-Profile-Extension-Registry, WP-1-Governance-Workflow-Mirror
- BUILD_ORDER_BLOCKS: NONE
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.180.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- WHAT: Implement and prove one product-backed registry surface for workflow-state families, queue-reason codes, governed action descriptors, workflow transition rules, queue automation rules, executor eligibility posture, and project-profile workflow label overrides across packet, micro-task, task-board, mailbox-linked, and compact-summary surfaces.
- WHY: Current product code already exposes partial workflow metadata, but it does not yet satisfy the full v02.180 portable registry contract. Until that closes, queue posture, action legality, and executor routing remain partly helper-defined and drift-prone.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - Dev Command Center UI implementation or visual workflow editors
  - Repo-governance task board, broker, runtime, or role-protocol maintenance
  - New Master Spec or appendix updates
  - Full mailbox workflow ownership changes beyond explicit linked-record queue posture
- TEST_PLAN:
  ```bash
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  ```
- DONE_MEANS:
  - Product code exposes the full spec-defined workflow family and queue reason vocabulary or an explicitly equivalent durable contract.
  - Governed action legality no longer depends on duplicated helper-only maps.
  - Packet, micro-task, task-board, and compact-summary surfaces remain family and reason equivalent under the same backend contract.
  - Mailbox-linked waits remain visible as canonical queue reasons on the linked record instead of being only sidecar prose or ordering hints.
- PRIMITIVES_EXPOSED:
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-WorkflowTransitionRuleV1
  - PRIM-QueueAutomationRuleV1
  - PRIM-ExecutorEligibilityPolicyV1
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.180.md
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- SEARCH_TERMS:
  - workflow_state_family
  - queue_reason_code
  - allowed_action_ids
  - GovernedActionDescriptorV1
  - WorkflowTransitionRuleV1
  - QueueAutomationRuleV1
  - ExecutorEligibilityPolicyV1
  - mailbox_response_wait
- RUN_COMMANDS:
  ```bash
  rg -n "workflow_state_family|queue_reason_code|allowed_action_ids|GovernedActionDescriptorV1|WorkflowTransitionRuleV1|QueueAutomationRuleV1|ExecutorEligibilityPolicyV1" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
  ```
- RISK_MAP:
  - "reason vocabulary stays narrower than the spec" -> "queue grouping and routing remain inconsistent across surfaces"
  - "action legality remains helper-defined in multiple places" -> "durable governed action semantics drift and become harder to audit"
  - "mailbox waits remain sidecar-only hints" -> "linked records lose canonical waiting posture and small-model routing degrades"
  - "project-profile label overrides mutate semantics instead of labels" -> "portable workflow meaning collapses across product kernels"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation should preserve the existing base-WP dependency chain rather than inventing a new product-governance lane.
  - No build-order or task-board maintenance work is part of this refinement pass itself.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] | WHY_IN_SCOPE: current product code emits partial workflow metadata but still falls short of the full portable registry contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs; ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board | RISK_IF_MISSED: surfaces will keep emitting workflow posture that looks canonical but is still semantically incomplete
  - CLAUSE: Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171] | WHY_IN_SCOPE: v02.180 explicitly requires portable family, reason, and governed-action semantics across artifact families and queues | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests | RISK_IF_MISSED: `allowed_action_ids` legality and reason coverage will continue to drift across emitters and persistence paths
  - CLAUSE: Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172] | WHY_IN_SCOPE: the packet must either expose or durably reference transition, automation, and executor policy ids before queue posture can be trusted end to end | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: executor routing and automatic queue posture will remain social convention rather than durable product truth

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: `workflow_state_family` and `queue_reason_code` on packet and micro-task artifacts | PRODUCER: workflows.rs emitters | CONSUMER: packet readers, micro-task readers, compact summaries, local-small-model routing | SERIALIZER_TRANSPORT: packet.json and summary.json | VALIDATOR_READER: locus/types.rs validators | TRIPWIRE_TESTS: micro_task_executor_tests workflow drift checks | DRIFT_RISK: detail artifacts and summaries can silently diverge
  - CONTRACT: `allowed_action_ids` backed by durable governed action semantics | PRODUCER: workflows.rs and storage/locus_sqlite.rs | CONSUMER: task board, compact summaries, runtime governance, validator reads | SERIALIZER_TRANSPORT: structured collaboration JSON payloads | VALIDATOR_READER: schema registry and action validation paths | TRIPWIRE_TESTS: negative-path unregistered action-id tests | DRIFT_RISK: duplicated helper maps can disagree while all rows still look syntactically valid
  - CONTRACT: transition, automation, and executor policy identifiers | PRODUCER: locus/workflow registry surfaces | CONSUMER: runtime governance queues, task board projections, queue refresh flows | SERIALIZER_TRANSPORT: structured collaboration records or equivalent linked contract ids | VALIDATOR_READER: runtime-governance and queue readers | TRIPWIRE_TESTS: runtime_governance targeted tests plus full cargo test | DRIFT_RISK: queue moves and executor selection stay implicit instead of inspectable
  - CONTRACT: mailbox-linked wait posture on linked records | PRODUCER: runtime_governance.rs and mailbox-linked refresh paths | CONSUMER: queue views, compact summaries, local-small-model routing | SERIALIZER_TRANSPORT: compact summary JSON and linked work record fields | VALIDATOR_READER: workflow-state validators and queue readers | TRIPWIRE_TESTS: runtime_governance targeted tests | DRIFT_RISK: waits remain hidden in prose or thread ordering instead of canonical reason codes

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- CANONICAL_CONTRACT_EXAMPLES:
  - A work packet artifact, a micro-task artifact, and a compact summary that all expose the same `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`
  - A mailbox-linked wait that still leaves the linked record in a canonical waiting posture with an explicit queue reason
  - A project-profile label override that changes display wording while preserving the base family, reason, and governed action ids

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Expand `locus/types.rs` to align workflow family, reason, governed action, transition, automation, and executor surfaces with the v02.180 contract.
  - Remove helper-only legality drift by routing emitters and persistence-backed readers through one durable workflow/action contract.
  - Propagate canonical waiting and executor posture through packet, micro-task, task-board, and compact-summary consumers.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
- CARRY_FORWARD_WARNINGS:
  - Do not widen into repo-governance tooling, task-board maintenance, role-protocol work, or `.GOV` process changes.
  - Do not let project-profile workflow overrides mutate base-family semantics.
  - Do not preserve duplicated action legality helpers as a second source of truth.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - Full family and queue-reason parity across packet, micro-task, task-board, and compact-summary surfaces
  - Durable governed action, transition, automation, and executor posture instead of helper-only inference
  - Canonical mailbox-linked wait reasons and display-only project-profile relabeling
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - NONE

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - This refinement does not prove the final code shape for durable transition, automation, and executor-policy ids; it only proves those contract surfaces are already required by the current spec.
  - This refinement does not prove any Dev Command Center GUI affordance; it only bounds the backend workflow registry semantics those viewers must consume.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: v02.180 explicitly names the workflow-state, queue-reason, governed-action, transition, automation, executor-eligibility, and project-profile workflow overlay contract this packet activates. The remaining gap is specific product implementation and proof.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Current Handshake_Master_Spec_v02.180.md already defines the workflow-state registry contract, transition law, and primitive coverage this packet needs. No spec or appendix update is required before activation.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- CONTEXT_START_LINE: 6930
- CONTEXT_END_LINE: 7020
- CONTEXT_TOKEN: **Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]

  - Every canonical Work Packet, Micro-Task, Task Board projection row, and Dev Command Center queue row SHALL expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
  - `workflow_state_family` MUST stay low-cardinality and project-agnostic.
  - `queue_reason_code` MUST explain why the record is currently routed or grouped where it is.
  - Board position, queue order, and mailbox thread order MUST NOT become substitutes for `workflow_state_family` or `queue_reason_code`.
  - Every state-changing operator or model action SHOULD resolve through a registered `GovernedActionDescriptorV1`.
  - Project profiles MAY define `ProjectProfileWorkflowExtensionV1` mappings that rename visible state labels or narrow valid reasons and actions, but those mappings MUST NOT change the meaning of the base families.
  - Local-small-model routing MUST default to `workflow_state_family` plus `queue_reason_code`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]
- CONTEXT_START_LINE: 61135
- CONTEXT_END_LINE: 61192
- CONTEXT_TOKEN: #### 10.11.5.21 Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 10.11.5.21 Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]

  The Dev Command Center and every structured collaboration artifact family member MUST share one portable workflow-state and action vocabulary.

  **Required state contract**
  - Canonical records SHOULD expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
    - optional project-profile display labels
  - `allowed_action_ids` MUST reference registered `GovernedActionDescriptorV1` records rather than ad hoc user-interface verbs.
  - Mailbox-linked waits MUST remain visible as `queue_reason_code=mailbox_response_wait`, but the mailbox thread itself MUST NOT become the authority for the linked record's state family.
  - Project-profile extensions MAY relabel families for display, but they MUST NOT change the base semantic meaning.
  - Unknown project-profile workflow extensions MUST degrade to the base workflow-state families, reason codes, and governed action ids rather than hiding the record.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]
- CONTEXT_START_LINE: 61192
- CONTEXT_END_LINE: 61260
- CONTEXT_TOKEN: #### 10.11.5.22 Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 10.11.5.22 Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]

  The Dev Command Center and every structured collaboration artifact family member MUST now share one portable transition matrix.

  **Required transition contract**
  - Canonical records SHOULD expose or reference:
    - `transition_rule_ids`
    - `queue_automation_rule_ids`
    - `executor_eligibility_policy_ids`
  - `WorkflowTransitionRuleV1` MUST remain portable across Work Packets, Micro-Tasks, Task Board rows, and Role Mailbox-linked waits.
  - `QueueAutomationRuleV1` SHOULD be the reusable contract for triggers such as dependency cleared, mailbox response received, validation passed, and retry timer elapsed.
  - `ExecutorEligibilityPolicyV1` SHOULD be the reusable contract for executor kinds such as `operator`, `local_small_model`, `cloud_model`, `workflow_engine`, `reviewer`, and `governance`.
  - Local-small-model eligibility MUST require a compact summary and a ready-family state.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Appendix 12.4 workflow-state primitive rows
- CONTEXT_START_LINE: 76323
- CONTEXT_END_LINE: 76354
- CONTEXT_TOKEN: "primitive_id": "PRIM-WorkflowStateFamily"
- EXCERPT_ASCII_ESCAPED:
  ```text
      "primitive_id": "PRIM-WorkflowStateFamily",
      "title": "WorkflowStateFamily",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-WorkflowQueueReasonCode",
      "title": "WorkflowQueueReasonCode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-GovernedActionDescriptorV1",
      "title": "GovernedActionDescriptorV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProjectProfileWorkflowExtensionV1",
      "title": "ProjectProfileWorkflowExtensionV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkflowTransitionRuleV1",
      "title": "WorkflowTransitionRuleV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-QueueAutomationRuleV1",
      "title": "QueueAutomationRuleV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExecutorEligibilityPolicyV1",
      "title": "ExecutorEligibilityPolicyV1",
      "kind": "ts_interface"
    }
  ```

### DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (This refinement activates existing workflow-state, queue-reason, governed-action, workflow-extension, transition, automation, and executor-eligibility primitives that are already present in v02.180 Appendix 12.4.)
- DISCOVERY_STUBS: NONE_CREATED (The current target stub is the correct packet shell and no additional product-scope stub is required.)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (Current Appendix 12.6 interaction coverage is sufficient for this implementation pass; the gap is product realization of existing law, not a missing new IMX edge.)
- DISCOVERY_UI_CONTROLS: NONE_APPLICABLE (Concrete workflow-state inspector, rule provenance, and label-mapping controls remain downstream of existing Dev Command Center UI packets.)
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED (The current Master Spec already names the contract and primitive surfaces required for this packet.)
- DISCOVERY_JUSTIFICATION: This refinement still delivered value by collapsing the packet onto the correct product-backend registry gap, proving that no spec bump or new stub is required, and hydrating the exact code, proof, and validation surfaces needed for the later packet.
