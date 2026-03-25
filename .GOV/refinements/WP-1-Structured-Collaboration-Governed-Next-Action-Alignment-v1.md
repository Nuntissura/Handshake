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
- WP_ID: WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-25T15:20:46.4603911+01:00
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: e658a3b8a2d7cdd0d294838151d24a60bc3e034c
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja250320261614
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- `src/backend/handshake_core/src/workflows.rs` still emits ad hoc compact-summary `next_action` tokens such as `start_work_packet`, `continue_work_packet`, `start_micro_task`, and `review_skipped_micro_task` instead of governed action ids.
- `src/backend/handshake_core/src/workflows.rs` still carries residual `next_action_for_work_packet` and `next_action_for_micro_task` helper paths that describe actions as prose strings such as `triage work packet` and `start the next iteration`.
- `src/backend/handshake_core/src/locus/types.rs` still validates summary `next_action` only as a non-empty string, so unregistered or drifted action tokens can still look valid.
- `src/backend/handshake_core/tests/micro_task_executor_tests.rs` still encodes legacy summary tokens and does not prove that emitted `next_action` values resolve to registered `GovernedActionDescriptorV1.action_id` values only.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 70m
- SEARCH_SCOPE: current Handshake Master Spec v02.178 structured-collaboration summary, workflow-state, governed-action, and Task Board viewer clauses; current product emitters and validators in `src/backend/handshake_core`
- REFERENCES: Internal spec-to-code remediation only. Primary sources were `.GOV/spec/Handshake_Master_Spec_v02.178.md`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs`.
- PATTERNS_EXTRACTED: one canonical governed-action vocabulary should own every machine-readable action hint; compact summary `next_action` should either resolve to a registered action id or be omitted when a single deterministic action is not defensible; validators and tests must reject drift rather than merely checking that the field is non-empty.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the existing `GovernedActionDescriptorV1` registry as the only legal `next_action` vocabulary; ADAPT the current mutation-based JSON drift tests so they also fail invalid `next_action`; REJECT inventing a second summary-only action vocabulary or keeping dead prose helper paths as a shadow contract.
- LICENSE/IP_NOTES: Internal repository and spec inspection only. No third-party code or text is intended for direct reuse.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.178.md already defines the compact summary contract, the governed action registry law, and the board-view requirement to surface next action. The missing work is implementation and proof hardening in product code.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is a strictly internal spec-to-code remediation pass against current Handshake Main Body requirements and current local product code.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Internal sources are sufficient because the unresolved contract drift is explicit in the current spec and current local emitters.
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
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new Flight Recorder ids are required for this packet.
- Existing structured-artifact and validation telemetry remains the relevant evidence seam; this packet changes machine-readable action content, not telemetry taxonomy.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: ad hoc summary action strings can become a shadow authority surface that diverges from the governed action registry. Mitigation: allow only registered governed action ids or omission.
- Risk: dead prose helper functions can be revived by future callers and silently reintroduce non-governed next-action semantics. Mitigation: remove or align those helpers in the same packet.
- Risk: a validator that only checks `next_action` for non-empty string shape can let false machine-readable claims pass downstream. Mitigation: validate registry-backed legality mechanically.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-GovernedActionDescriptorV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-GovernedActionDescriptorV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - The spec already defines the relevant primitives. This packet hardens how existing summary and preview surfaces use them.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current appendix already covers the governed action and structured collaboration primitives required for this remediation.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - No new primitive ids are needed. The work is implementation alignment.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new orphan primitives were discovered during this refinement.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Existing feature-registry coverage is sufficient for a narrow product implementation pass on governed next-action alignment.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet changes backend summary semantics and proof only. No direct UI surface is implemented here.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: No new feature combination or runtime adjacency needs appendix-level interaction matrix growth for this narrow pass.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep Appendix 12 unchanged and implement against current Main Body law.
  - If coding proves that board rows require an inline `next_action` field rather than `summary_ref`-backed lookup, treat that as a fresh spec-update decision instead of silent scope growth.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene contract is changed by summary next-action alignment | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected here | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physical simulation or measurement law is involved in this packet | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers only | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing capability is changed by this remediation | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: TOUCHED | NOTES: governed next actions are orchestration-facing signals even though this packet stays in backend summary emitters | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition or sequencing contract is affected | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is touched | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication surfaces remain downstream consumers of the corrected summaries | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking or recipe workflow surface is relevant here | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed by structured summary action hardening | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no routing or delivery engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: this packet hardens durable summary artifacts and their machine-readable action hints | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval layers may consume summaries later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analysis surfaces remain downstream consumers of the hardened summaries | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is changed directly | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: this packet does not change storage or migration posture directly | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this pass implements existing law rather than adding new governance authority surfaces | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or guidance interface is added here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: compact summary `next_action` is a context-compaction signal for local-small-model and operator routing | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: this packet hardens versioned structured summary contracts against action-vocabulary drift | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required for this pass | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: existing telemetry remains in place; only summary action semantics change | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: calendar surfaces are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: editor surfaces are downstream consumers only | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: document editing is not changed by this packet | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: spreadsheet surfaces are out of scope | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus owns the shared summary contract, governed action registry, and validation boundary targeted here | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Loom portability remains a separate lane | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Work Packet artifacts are in scope as downstream record families, but the implementation work stays centered in shared Locus producers and validators rather than packet-specific product surfaces | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: board surfaces remain downstream readers of `summary_ref` unless code inspection proves inline `next_action` is required | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: Micro-Task compact summaries and status-to-action mapping are direct subjects of this packet | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: Command Center remains a downstream consumer of the corrected backend summaries | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: memory-system behavior is not changed directly here | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: NOT_TOUCHED | NOTES: runtime execution is affected only indirectly through stricter summary semantics | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no Spec Router or prompt-compilation contract is expanded here | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: storage-portability work is not changed directly by this pass | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: compact machine-readable summaries must not advertise an ungoverned action vocabulary to models or operators | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Stage artifact contracts are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Studio runtime behavior is out of scope | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: viewer follow-on work remains downstream of this backend pass | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: distillation workflows are unaffected by this packet | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE runtime protocol or tool contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval systems may consume summaries later but are not changed here | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: governed compact-summary next_action emission | SUBFEATURES: Work Packet summary serializer, Micro-Task summary serializer, governed action mapping helper | PRIMITIVES_FEATURES: PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask | MECHANICAL: engine.director, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the core execution target of the packet.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: compact summary next-action contract | SUBFEATURES: Ready/InProgress/Blocked/Gated/Done/Cancelled Work Packet summary output and packet-adjacent preview helpers | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1 | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Work Packet next-action hints must stop drifting from the governed action registry.
  - PILLAR: MicroTask | CAPABILITY_SLICE: compact summary next-action contract | SUBFEATURES: Pending/InProgress/Completed/Failed/Blocked/Skipped Micro-Task summary output and mutation tests | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1 | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Micro-Task summaries are currently the clearest live drift surface.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: machine-readable next-action hints | SUBFEATURES: summary validation, governed-action legality checks, compact summary drift rejection | PRIMITIVES_FEATURES: PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Compact routing hints must be safe for local model consumption.
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Work Packet compact-summary next_action emission | JobModel: WORKFLOW | Workflow: Locus structured artifact generation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Emission remains a backend workflow concern even though downstream operator and model surfaces will consume it.
  - Capability: Micro-Task compact-summary next_action emission | JobModel: WORKFLOW | Workflow: Locus structured artifact generation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Micro-Task summary output is the main live drift surface today.
  - Capability: Structured summary next_action validation | JobModel: MECHANICAL_TOOL | Workflow: structured collaboration validation | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: This is the mechanical gate that prevents the drift from reappearing.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - No new high-ROI primitive combination was discovered beyond aligning compact summaries to the already-existing governed action registry.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: This is a narrow contract-hardening pass, not a feature-combination expansion.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This is a strictly internal spec-to-code alignment pass.
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
  - Combo: Locus governed next-action helper foundation | Pillars: Locus | Mechanical: engine.director | Primitives/Features: PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: one canonical helper removes summary-token drift at the root contract surface.
  - Combo: Locus Work Packet summary emission parity | Pillars: Locus | Mechanical: engine.archivist | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: Work Packet summaries must advertise governed next actions or omit them.
  - Combo: MicroTask summary emission parity | Pillars: MicroTask | Mechanical: engine.archivist | Primitives/Features: PRIM-TrackedMicroTask, PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: Micro-Task summaries are the clearest live drift surface and must align first.
  - Combo: Locus summary validator legality checks | Pillars: Locus | Mechanical: engine.context | Primitives/Features: PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: machine-readable summaries are only useful if invalid action ids fail mechanically.
  - Combo: LLM-friendly compact-routing safety | Pillars: LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: compact routing hints must stay governed for local execution surfaces.
  - Combo: Locus residual preview-helper cleanup | Pillars: Locus | Mechanical: engine.version | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-GovernedActionDescriptorV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: prose helper paths must not survive as shadow authority.
  - Combo: MicroTask mutation-based drift tripwires | Pillars: MicroTask | Mechanical: engine.version | Primitives/Features: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: tests should prove old token strings cannot silently reappear.
  - Combo: LLM-friendly omission policy for ambiguous states | Pillars: LLM-friendly data | Mechanical: engine.director | Primitives/Features: PRIM-GovernedActionDescriptorV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask | Resolution: IN_THIS_WP | Stub: NONE | Notes: if a single deterministic next action is not defensible, omission is safer than false precision.
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: No additional high-ROI scope expansion was discovered during this refinement.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: existing structured-collaboration stubs and packets, current product emitters, shared validators, and summary fixtures
- MATCHED_STUBS:
  - Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: N/A | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: registry-surface expansion is broader than this packet's summary-alignment target.
  - Artifact: WP-1-Workflow-Transition-Automation-Registry-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: N/A | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: automation and transition law is adjacent but intentionally broader than this compact-summary pass.
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - NONE
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: live summary emitters still use ad hoc next-action tokens and residual prose helpers.
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/types.rs | Artifact: NONE | Covers: primitive | Verdict: PARTIAL | Notes: summary validation still checks `next_action` only as a non-empty string.
  - Path: ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: fixtures and assertions still accept legacy summary tokens.
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: No existing active or completed packet fully closes governed next-action alignment, so a narrow follow-on packet is warranted.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This is a backend summary-contract and validation pass with no direct UI implementation.
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
- GUI_ADVICE_REASON_NO: No direct UI surface is being implemented in this packet.
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
- RISK_TIER: MEDIUM
- SPEC_ADD_MARKER_TARGET: [ADD v02.171]
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Contract-Hardening, WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: NONE
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md compact summary plus governed next-action contract
- WHAT: Align Work Packet and Micro-Task compact-summary `next_action` values and preview helpers to the governed action registry instead of ad hoc summary tokens or prose.
- WHY: Current compact summaries still advertise an ungoverned action vocabulary even after the structured-collaboration contract hardening pass.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - `allowed_action_ids` remediation already closed by WP-1-Structured-Collaboration-Contract-Hardening-v1
  - mailbox export validation
  - broad workflow transition and automation registry work
  - repo-governance and ACP workflow remediation
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- DONE_MEANS:
  - Every emitted in-scope compact-summary `next_action` value is either a registered `GovernedActionDescriptorV1.action_id` or omitted.
  - No live helper path in scope emits ad hoc token strings or prose-only next-action text.
  - Summary validation and tests mechanically fail unregistered or drifted `next_action` values.
- PRIMITIVES_EXPOSED:
  - PRIM-GovernedActionDescriptorV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- SEARCH_TERMS:
  - structured_work_packet_next_action
  - structured_micro_task_next_action
  - next_action_for_work_packet
  - next_action_for_micro_task
  - GovernedActionDescriptorV1
  - next_action
- RUN_COMMANDS:
  ```bash
  rg -n "structured_work_packet_next_action|structured_micro_task_next_action|next_action_for_work_packet|next_action_for_micro_task|next_action" src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/locus/types.rs src/backend/handshake_core/tests/micro_task_executor_tests.rs
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "legacy summary tokens remain live" -> "compact machine-readable next-action hints still drift from governed action law"
  - "dead prose helper survives" -> "future callers can silently reintroduce ungoverned next-action semantics"
  - "validator still accepts any non-empty string" -> "false machine-readable PASS remains possible"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - Stub, Task Board, traceability, and Build Order were already synchronized before this refinement.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: StructuredCollaborationSummaryV1 compact summary contract includes optional `next_action` | WHY_IN_SCOPE: current emitters and tests already use this field, but they populate it with ungoverned tokens | EXPECTED_CODE_SURFACES: `structured_work_packet_next_action`, `structured_micro_task_next_action`, `default_structured_collaboration_summary_record` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` | RISK_IF_MISSED: compact summaries remain machine-readable in shape but semantically weak
  - CLAUSE: Every state-changing operator or model action SHOULD resolve through a registered GovernedActionDescriptorV1 | WHY_IN_SCOPE: compact summary next-action hints currently encode a parallel action vocabulary | EXPECTED_CODE_SURFACES: governed next-action mapping helper in `workflows.rs` and validator legality checks in `locus/types.rs` | EXPECTED_TESTS: mutation-based rejection of unregistered `next_action` values in `micro_task_executor_tests` | RISK_IF_MISSED: summary actions drift from canonical registry law
  - CLAUSE: Task Board and board-adjacent viewers SHOULD expose base-envelope next action before grouping metadata | WHY_IN_SCOPE: board-facing preview helpers must not keep a separate prose or ad hoc token vocabulary | EXPECTED_CODE_SURFACES: residual `next_action_for_work_packet` and `next_action_for_micro_task` helpers in `workflows.rs` | EXPECTED_TESTS: `rg` and summary assertions prove there is one governed next-action contract only | RISK_IF_MISSED: board/detail surfaces can silently diverge from compact summaries

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: Work Packet `summary.json` `next_action` | PRODUCER: `build_structured_work_packet_summary` in `workflows.rs` | CONSUMER: compact summary readers and downstream board/detail surfaces via `summary_ref` | SERIALIZER_TRANSPORT: structured JSON summary artifact | VALIDATOR_READER: `validate_structured_collaboration_record` in `locus/types.rs` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` | DRIFT_RISK: live summary tokens can drift from the governed action registry
  - CONTRACT: Micro-Task `summary.json` `next_action` | PRODUCER: `build_structured_micro_task_summary` in `workflows.rs` | CONSUMER: compact summary readers and local-small-model routing | SERIALIZER_TRANSPORT: structured JSON summary artifact | VALIDATOR_READER: `validate_structured_collaboration_record` in `locus/types.rs` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` | DRIFT_RISK: micro-task summaries continue to advertise legacy tokens
  - CONTRACT: preview helper next-action mapping | PRODUCER: `next_action_for_work_packet` and `next_action_for_micro_task` in `workflows.rs` | CONSUMER: future board/detail callers | SERIALIZER_TRANSPORT: in-memory helper path | VALIDATOR_READER: code review plus grep-based verification | TRIPWIRE_TESTS: `rg -n "next_action_for_work_packet|next_action_for_micro_task" src/backend/handshake_core/src/workflows.rs` | DRIFT_RISK: prose-only helper paths can become shadow authority later

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
- CANONICAL_CONTRACT_EXAMPLES:
  - Ready Work Packet compact summary whose `next_action` resolves to a registered governed action id or is omitted
  - Pending Micro-Task compact summary whose `next_action` resolves to a registered governed action id
  - Mutated Work Packet summary payload with `next_action: "start_work_packet"` rejected as unregistered
  - Mutated Micro-Task summary payload with `next_action: "start_micro_task"` rejected as unregistered

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Add one canonical helper in `src/backend/handshake_core/src/workflows.rs` that derives in-scope compact-summary `next_action` values from governed action ids rather than ad hoc tokens.
  - Use that helper from Work Packet and Micro-Task summary emitters.
  - Remove or align `next_action_for_work_packet` and `next_action_for_micro_task` so no prose-only helper path remains in scope.
  - Harden `src/backend/handshake_core/src/locus/types.rs` so summary `next_action` values are either absent or registered governed action ids.
  - Update and extend `src/backend/handshake_core/tests/micro_task_executor_tests.rs` so legacy summary tokens fail mechanically.
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- CARRY_FORWARD_WARNINGS:
  - Do not reopen `allowed_action_ids`, queue-reason, Task Board authoritative-field, or mailbox leak-safety scope already closed by WP-1-Structured-Collaboration-Contract-Hardening-v1 unless a concrete regression is found.
  - Do not invent a second summary-only action vocabulary.
  - If a status maps to more than one legal governed action and no deterministic rule is defensible, omit `next_action` instead of overclaiming certainty.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - Compact summary `next_action` remains optional but, when present, resolves to a registered governed action id
  - No live helper path in scope emits prose-only or ad hoc token next-action values
  - Summary validation rejects unregistered `next_action` values mechanically
- FILES_TO_READ:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - rg -n "structured_work_packet_next_action|structured_micro_task_next_action|next_action_for_work_packet|next_action_for_micro_task|next_action" src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/locus/types.rs src/backend/handshake_core/tests/micro_task_executor_tests.rs
- POST_MERGE_SPOTCHECKS:
  - Verify no emitted summary payload still uses legacy tokens such as `start_work_packet` or `start_micro_task`
  - Verify no prose helper path remains as a shadow next-action contract
  - Verify invalid `next_action` values fail at the shared validation boundary

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Whether any hidden downstream consumer outside current tests still depends on legacy summary tokens such as `start_work_packet`
  - Whether every current status can map to one deterministic governed action id without needing omission in some states
  - Whether current Task Board surfaces are fully satisfied by `summary_ref`-backed next-action lookup or will later require an inline field

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Main Body already names the compact summary `next_action` field, the governed action registry contract, and the relevant board-view expectation. The missing work is a concrete implementation and proof gap.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.178.md already defines the compact summary field, the governed action contract, and the relevant board-view expectation. The missing work is code and proof alignment.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md StructuredCollaborationSummaryV1 compact summary contract
- CONTEXT_START_LINE: 6086
- CONTEXT_END_LINE: 6095
- CONTEXT_TOKEN: interface StructuredCollaborationSummaryV1 {
- EXCERPT_ASCII_ESCAPED:
  ```text
  interface StructuredCollaborationSummaryV1 {
    schema_id: string;
    schema_version: string;
    record_id: string;
    record_kind: StructuredRecordKind;
    project_profile_kind: ProjectProfileKind;
    status: string;
    title_or_objective: string;
    blockers: string[];
    next_action?: string;
    authority_refs: ArtifactHandle[];
    evidence_refs: ArtifactHandle[];
    updated_at: ISO8601Timestamp;
  }
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- CONTEXT_START_LINE: 6928
- CONTEXT_END_LINE: 6987
- CONTEXT_TOKEN: **Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]

  - Every canonical Work Packet, Micro-Task, Task Board projection row, and Dev Command Center queue row SHALL expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
  - Board position, queue order, and mailbox thread order MUST NOT become substitutes for `workflow_state_family` or `queue_reason_code`.
  - Every state-changing operator or model action SHOULD resolve through a registered `GovernedActionDescriptorV1` so the system knows:
    - who may invoke the action
    - which base families it may start from
    - which family and reason it produces
    - whether approval or evidence is required
    - whether linked record kinds or workflow activation are mandatory
  - Project profiles MAY define `ProjectProfileWorkflowExtensionV1` mappings that rename visible state labels or narrow valid reasons and actions, but those mappings MUST NOT change the meaning of the base families.
  - Local-small-model routing MUST default to `workflow_state_family` plus `queue_reason_code` and only then consult project-profile extensions, note sidecars, or Markdown mirrors.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Task Board projection viewer workflow portability [ADD v02.168-v02.171]
- CONTEXT_START_LINE: 60910
- CONTEXT_END_LINE: 60922
- CONTEXT_TOKEN: [ADD v02.168] Board rows SHOULD expose the base-envelope status, next action, blockers, and project-profile kind before board-specific grouping metadata.
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **Task Board projection viewer**
    - Show structured board rows keyed by stable `task_board_id` and `work_packet_id`, plus freshness, manual-edit detection, and sync status.
    - Any Markdown board is read-only by default from this view unless a governed sync or status-update workflow is being invoked.
    - [ADD v02.168] Board rows SHOULD expose the base-envelope status, next action, blockers, and project-profile kind before board-specific grouping metadata.
    - [ADD v02.170] Board, list, queue, and roadmap layouts SHOULD read from the same row set and declare which lane definitions, grouping keys, and action bindings are active for the current preset.
    - [ADD v02.171] Board rows SHOULD expose `workflow_state_family` and `queue_reason_code` separately from any project-specific display label so queue semantics remain portable across project kernels.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Required state contract and governed action behavior [ADD v02.171]
- CONTEXT_START_LINE: 61025
- CONTEXT_END_LINE: 61054
- CONTEXT_TOKEN: **Required state contract**
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Required state contract**
  - Canonical records SHOULD expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
    - optional project-profile display labels
  - `workflow_state_family` MUST remain portable across record kinds.
  - `queue_reason_code` MUST explain why the record is currently grouped, queued, or blocked.
  - `allowed_action_ids` MUST reference registered `GovernedActionDescriptorV1` records rather than ad hoc user-interface verbs.

  **Required action behavior**
  - `GovernedActionDescriptorV1` SHOULD be the reusable contract for verbs such as:
  ```
