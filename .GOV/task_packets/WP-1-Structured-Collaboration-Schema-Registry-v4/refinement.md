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
- WP_ID: WP-1-Structured-Collaboration-Schema-Registry-v4
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-24T21:56:36.4477374Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: 608a586c4afa78f4f625d5cd381a9d3b4fb3e4d9
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja240320262335
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Structured-Collaboration-Schema-Registry-v4
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Direct comparison of Handshake_Master_Spec_v02.178.md against current `src/backend/handshake_core` shows the remaining gap is no longer emitter presence; it is validator hardness and negative-path proof.
- `validate_structured_collaboration_record()` in `src/backend/handshake_core/src/locus/types.rs` still does not require `workflow_state_family`, `queue_reason_code`, or `allowed_action_ids` on `WorkPacketPacket`, `MicroTaskPacket`, or `TaskBoardEntry`, even though spec [ADD v02.171] says those canonical records SHALL expose them.
- The shared validator accepts `TaskBoardIndex.rows`, `TaskBoardView.rows`, `RoleMailboxIndex.threads`, and `RoleMailboxThreadLine.transcription_links` as arrays, but it does not recursively validate each element shape against the spec-defined nested contract.
- The shared validator currently treats RFC3339 fields such as `updated_at`, `generated_at`, and `created_at` as generic non-empty strings instead of typed timestamps.
- The shared validator currently treats artifact-handle and sha256 fields such as `body_ref`, `body_sha256`, `target_ref`, `target_sha256`, and `note_sha256` as generic strings instead of typed contract fields.
- Existing tests in `micro_task_executor_tests.rs` and `role_mailbox_tests.rs` prove happy-path emission and some schema drift, but they do not yet provide validator-owned negative-path coverage for missing workflow-state fields, malformed nested rows/threads/transcription links, or malformed timestamp/handle/sha payloads.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 75m
- SEARCH_SCOPE: current Master Spec v02.178 sections for shared structured collaboration records, workflow-state law, and role mailbox exports; local code reality in `src/backend/handshake_core`; current regression tests in `src/backend/handshake_core/tests`
- REFERENCES: Internal spec-to-code remediation only. Primary sources were `.GOV/spec/Handshake_Master_Spec_v02.178.md`, `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/role_mailbox.rs`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs`, and `src/backend/handshake_core/tests/role_mailbox_tests.rs`.
- PATTERNS_EXTRACTED: one shared validator should own portable record law; emitters and validators should reuse the same typed invariants; nested arrays need element-level validation instead of outer-array-only checks; negative-path tests should mutate exported JSON at the consumer boundary rather than trust only producer happy paths.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT central validator hardening in `locus/types.rs`; ADAPT existing happy-path export tests into mutation-based negative tests; REJECT widening this pass into Loom portability, Command Center UI, new schema ids, or `.GOV`-only gate redesign.
- LICENSE/IP_NOTES: Internal repository and spec inspection only. No third-party code or text is intended for direct reuse.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.178.md already defines the base structured-collaboration envelope, the project-agnostic workflow-state contract, the Task Board projection rules, the Role Mailbox export record shapes, and the mailbox export gate behavior. This WP is implementation and proof hardening against the current Main Body.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is a strictly internal spec-to-code remediation pass against the current Handshake Master Spec and the current local product code. No external ecosystem signal is needed to decide scope.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Internal sources are sufficient because the missing work is explicit in current spec clauses and current local code paths.
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
- No new Flight Recorder event ids are needed for this WP.
- Work Packet, Micro-Task, Task Board, and Role Mailbox emitters should continue to use their current event families; this packet hardens validation, not event taxonomy.
- Role Mailbox export and transcription telemetry remain the relevant observability surface for mailbox-linked proof, and validator failures should surface as deterministic structured validation issues rather than ad hoc runtime strings.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: missing `workflow_state_family`, `queue_reason_code`, or `allowed_action_ids` is silently accepted, so routing falls back to lane order, mailbox chronology, or prose. Mitigation: shared validator hard-rejects missing workflow-state fields on canonical records.
- Risk: malformed nested rows, threads, or transcription links pass array-only checks and later break consumers. Mitigation: recursively validate nested payload element shapes at the shared validator boundary.
- Risk: malformed timestamps, artifact handles, or sha256 strings pass generic string checks and only fail deep in mailbox-local or runtime-local code. Mitigation: move the typed checks into the shared validator where the spec contract is declared.
- Risk: happy-path export tests create false PASS closure while consumer-facing malformed payloads remain untested. Mitigation: add validator-owned negative-path regression cases that mutate emitted JSON before validation.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - The spec already defines the primitive families required here. The v4 gap is enforcing and proving the shared validator behavior over those existing primitives, not inventing new primitives.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current spec appendix already names the structured collaboration and role mailbox primitives involved in this packet.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Implementation should align validator behavior to existing primitive law rather than introduce new primitive ids.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new orphan primitives were discovered during this remediation pass.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: The feature and primitive registry already describe the structured collaboration and role mailbox surfaces relevant to this packet.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet is backend validation and proof work. No direct UI surface is implemented here.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The current interaction matrix is sufficient for this implementation-hardening pass.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep Appendix 12 unchanged and implement against current Main Body law.
  - If coding uncovers a truly missing primitive or interaction edge, that should become a separate spec-update flow instead of silent packet drift.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene contract is changed by validator hardening | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected here | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physical simulation or measurement law is involved in this packet | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes remain downstream consumers of validated records | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing capability is changed by this remediation | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration consumes the workflow fields later but is not the implementation surface here | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition or sequencing contract is affected | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is touched | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication surfaces stay downstream of the validator boundary | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking or recipe workflow surface is relevant here | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed by structured record validation | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no routing or delivery engine behavior is altered in this packet | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: this packet hardens the durable artifact validation boundary for packet, task-board, and mailbox records | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval layers consume these records later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analysis surfaces remain downstream consumers of the validation output | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is changed directly | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: storage portability remains Loom scope; this packet stays at the record-contract layer | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this pass implements already-declared law and does not add a new governance authority surface | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or guidance interface is added here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: typed compact record validation and workflow-state fields keep local-model routing and summary-first reads trustworthy | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: this packet hardens schema-version-compatible validation, typed field law, and portable workflow-state vocabulary enforcement | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required for this pass | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: existing event families stay intact; this packet hardens validation, not telemetry taxonomy | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: calendar storage and policy surfaces are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: editor surfaces are downstream consumers only | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: document editing is not changed by shared validator hardening | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: spreadsheet surfaces are out of scope | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus owns the shared validator and the canonical packet and task-board record families targeted by this packet | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Loom portability remains a separate live smoke lane and should stay file-disjoint from this schema-registry pass | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: product work-packet operating surfaces remain downstream even though the shared validator underneath them is being hardened here | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: product task-board operating surfaces remain downstream even though nested row validation is being hardened here | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: canonical Micro-Task packet records are directly in scope for workflow-state enforcement | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: Command Center remains a downstream consumer of the hardened validation outputs | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: memory-system behavior is not changed directly here | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: NOT_TOUCHED | NOTES: runtime execution is affected only indirectly through stricter artifact validation | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no Spec Router or prompt compilation contract is expanded here | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: this packet is storage-agnostic and does not change the database boundary | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: portable workflow-state fields and typed structured validation keep compact machine-readable records safe for local-model routing | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Stage artifact contracts are unrelated to the schema-registry validator | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Studio runtime behavior is out of scope | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: UI and typed-viewer follow-on work remain downstream | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: distillation workflows are unaffected by this packet | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE runtime protocol or tool contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval systems may consume these records later but are not changed in this WP | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: Shared validator enforcement of portable workflow-state fields | SUBFEATURES: `workflow_state_family`, `queue_reason_code`, `allowed_action_ids` on Work Packet, Micro-Task, and Task Board records | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: current emitters already populate most of these fields; the remaining gap is central validator enforcement and regression proof
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Canonical packet typed-field validation | SUBFEATURES: RFC3339 `updated_at`, portable workflow-state fields, negative-path record mutation tests | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationEnvelopeV1 | MECHANICAL: engine.archivist, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: work packet records are live smoke-test artifacts and need hard validator proof, not another happy-path-only closure
  - PILLAR: MicroTask | CAPABILITY_SLICE: Canonical micro-task typed-field validation | SUBFEATURES: RFC3339 `updated_at`, portable workflow-state fields, negative-path record mutation tests | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationEnvelopeV1 | MECHANICAL: engine.archivist, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task records share the same validator law and should fail on the same malformed workflow or timestamp drift
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Nested Task Board row validation | SUBFEATURES: index/view `rows[]` element validation, TaskBoardEntry workflow-state enforcement, row-shape negative tests | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1 | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the current validator checks outer arrays but not the nested row contract the spec expects consumers to trust
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Portable machine-readable record trust boundary | SUBFEATURES: typed timestamps, typed sha256 values, typed artifact handles, portable workflow-state vocabulary | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-RoleMailboxThreadLineV1, PRIM-RoleMailboxIndexV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: local models and operator tooling need records that fail deterministically before prose-only recovery paths are considered
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Shared structured-collaboration validator hardening | JobModel: WORKFLOW | Workflow: locus_structured_artifact_validation | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: validation results stay machine-readable and portable across packet, task-board, and mailbox records
  - Capability: Task Board nested row validation | JobModel: WORKFLOW | Workflow: task_board_projection_publish | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: board projections should fail deterministically when row payloads drift from the spec-defined nested contract
  - Capability: Role Mailbox typed export validation | JobModel: WORKFLOW | Workflow: role_mailbox_export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-MAILBOX-002, FR-EVT-GOV-MAILBOX-003 | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox export consumers should not need mailbox-local parsing rules to detect malformed thread lines or transcription links
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Scanned the current spec appendix and live product code for high-ROI interactions beyond the direct validator-hardening scope.
  - The concrete win here is stronger shared validation across packet, task-board, and mailbox records, not a new appendix interaction edge.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Current appendix interaction coverage is sufficient; this packet is implementation hardening, not a new cross-feature surface.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This WP is a strictly internal validator-remediation pass against explicit Master Spec clauses and current local product code.
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
  - Combo: portable workflow-state fields on canonical packet and micro-task records | Pillars: Locus, MicroTask, LLM-friendly data | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the highest-risk remaining spec/code mismatch and the best single ACP proof-run target
  - Combo: portable workflow-state fields on canonical task-board entry records | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: keeps routing law explicit instead of letting board ordering substitute for state
  - Combo: recursive validation of task-board `rows[]` payloads | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: current validator only proves outer arrays for task-board index and view records
  - Combo: recursive validation of mailbox `threads[]` payloads | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-RoleMailboxIndexV1, PRIM-StructuredCollaborationEnvelopeV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox index consumers should trust nested thread items without mailbox-specific fallback parsing
  - Combo: recursive validation of mailbox `transcription_links[]` payloads | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-RoleMailboxThreadLineV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: governance-critical messages need deterministic structured failure when transcription links drift
  - Combo: typed RFC3339 validation for shared structured-collaboration timestamps | Pillars: Locus, MicroTask, LLM-friendly data | Mechanical: engine.version, engine.context | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1, PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: generic string acceptance is too weak for portable consumer logic
  - Combo: typed artifact-handle validation for exported mailbox references | Pillars: Locus, LLM-friendly data | Mechanical: engine.version, engine.context | Primitives/Features: PRIM-RoleMailboxThreadLineV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: exported handle strings should fail before mailbox-local code is invoked
  - Combo: mutation-based negative-path proof over emitted smoke-test artifacts | Pillars: Locus, MicroTask, LLM-friendly data | Mechanical: engine.archivist, engine.context, engine.version | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry, PRIM-RoleMailboxThreadLineV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: converts the current smoke-test evidence from optimistic output checks into adversarial proof
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered here belong inside this remediation packet and do not require a new stub or spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current schema-registry and Loom stubs; historical schema-registry packet variants; current `src/backend/handshake_core` validator, workflow emitter, and mailbox export code; current regression tests that support the live smoke-test claim
- MATCHED_STUBS:
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v4 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this existing stub is the correct governance shell for the remediation pass; activate it instead of creating another packet
  - Artifact: WP-1-Loom-Storage-Portability-v4 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Loom remains the adjacent live smoke lane, but it should follow the schema-registry proof run rather than merge scopes
  - Artifact: WP-1-Project-Profile-Extension-Registry-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: extension registry work stays downstream of the shared validator and should not be folded into this remediation
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: the artifact-family packet landed canonical record families, but it did not complete the validator-hardening and adversarial proof this packet now targets
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v1 | BoardStatus: SUPERSEDED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: earlier registry work does not satisfy the current spec and proof bar
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | BoardStatus: SUPERSEDED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: historically marked PASS but still under-proves current workflow-state and typed nested validation law
  - Artifact: WP-1-Role-Mailbox-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: mailbox export plumbing exists, but shared structured-validator law still needs this focused remediation
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | Covers: primitive | Verdict: PARTIAL | Notes: `validate_structured_collaboration_record()` checks the outer envelope and some family-specific arrays, but it does not require workflow-state fields on canonical records or recursively validate nested payload items
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | Covers: primitive | Verdict: IMPLEMENTED | Notes: `TaskBoardEntryRecordV1` already declares `workflow_state_family`, `queue_reason_code`, `allowed_action_ids`, `task_board_id`, `lane_id`, `display_order`, and `view_ids`
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | Covers: execution | Verdict: IMPLEMENTED | Notes: packet, micro-task, and task-board emitters already populate `workflow_state_family`, `queue_reason_code`, `allowed_action_ids`, and `updated_at`, so the remaining gap is validator enforcement and proof
  - Path: ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | Covers: execution | Verdict: PARTIAL | Notes: mailbox-local code validates sha256 values and artifact handles, but the shared structured validator still treats exported thread-line fields as generic strings and arrays
  - Path: ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | Covers: execution | Verdict: PARTIAL | Notes: tests already cover happy-path emission, `updated_at` presence, and some schema drift, but not missing workflow-state fields or malformed nested task-board rows
  - Path: ../handshake_main/src/backend/handshake_core/tests/role_mailbox_tests.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | Covers: execution | Verdict: PARTIAL | Notes: tests already cover happy-path export and some authority/schema drift, but not malformed nested mailbox threads, transcription links, or typed field failures at the shared validator boundary
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The right governance shell already exists as `WP-1-Structured-Collaboration-Schema-Registry-v4`, but direct code inspection shows the remaining product gap is still real. This packet expands the stub into a focused validator-hardening remediation rather than creating a duplicate lane.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet is backend validator and proof work. Viewer and triage UI remain downstream consumers.
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
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.171]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: WP-1-Project-Profile-Extension-Registry, WP-1-Markdown-Mirror-Sync-Drift-Guard, WP-1-Dev-Command-Center-Structured-Artifact-Viewer
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- WHAT: Harden the shared structured-collaboration validator so Work Packet, Micro-Task, Task Board, and Role Mailbox records enforce portable workflow-state fields, recursively validate nested payload elements, and reject malformed RFC3339 timestamps, artifact-handle strings, and sha256 strings with validator-owned negative-path proof.
- WHY: The v3 smoke-test lane already improved emitters and happy-path outputs, but audit against the current Master Spec proved closure incomplete. The remaining gap is central validator strictness and adversarial proof, not new record families.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- OUT_OF_SCOPE:
  - Loom storage portability and database-backend abstraction work
  - Dev Command Center viewer and layout UI work
  - New schema ids, new primitive families, or spec-version bumps
  - Governance-only `.GOV` ledger or gate redesign
- TEST_PLAN:
  ```bash
  cargo test -p handshake_core schema_registry
  cargo test -p handshake_core task_board
  cargo test -p handshake_core role_mailbox
  cargo test -p handshake_core
  just gov-check
  ```
- DONE_MEANS:
  - `validate_structured_collaboration_record()` rejects missing `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` on canonical Work Packet, Micro-Task, and Task Board records.
  - Task Board `rows[]`, Role Mailbox `threads[]`, and `transcription_links[]` are validated per element shape rather than only by outer-array presence.
  - Shared structured validation rejects malformed RFC3339 timestamps, malformed artifact-handle strings, and malformed sha256 strings on the relevant record families.
  - Regression tests prove the above failures on mutated exported JSON, not only on happy-path emitters.
  - Existing happy-path smoke tests continue to pass after validator hardening.
- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- SEARCH_TERMS:
  - workflow_state_family
  - queue_reason_code
  - allowed_action_ids
  - updated_at
  - generated_at
  - created_at
  - body_ref
  - body_sha256
  - transcription_links
  - threads
  - rows
  - validate_structured_collaboration_record
- RUN_COMMANDS:
  ```bash
  rg -n "workflow_state_family|queue_reason_code|allowed_action_ids|updated_at|generated_at|created_at|body_ref|body_sha256|transcription_links|threads|rows|validate_structured_collaboration_record" src/backend/handshake_core
  cargo test -p handshake_core schema_registry
  cargo test -p handshake_core task_board
  cargo test -p handshake_core role_mailbox
  cargo test -p handshake_core
  just gov-check
  ```
- RISK_MAP:
  - "validator hardening rejects existing fixtures or exports" -> "current smoke tests were masking real malformed payload tolerance"
  - "nested validation pulls in too much emitter refactor" -> "scope creep away from the shared validator into unrelated record redesign"
  - "typed-field validation duplicates mailbox-local helpers poorly" -> "two conflicting validation dialects emerge for the same structured record"
  - "negative-path tests mutate the wrong layer" -> "green tests without proof that consumer-boundary payloads are actually protected"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - Existing stub metadata and current build-order records already identify this lane as the active remediation target for the base WP.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.171] canonical Work Packet, Micro-Task, and Task Board records SHALL expose workflow_state_family, queue_reason_code, allowed_action_ids | WHY_IN_SCOPE: current emitters largely populate these fields but the shared validator does not require them, so malformed records can still pass | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/locus/task_board.rs | EXPECTED_TESTS: cargo test -p handshake_core schema_registry workflow_state | RISK_IF_MISSED: routing falls back to lane order, mailbox chronology, or prose instead of portable state law
  - CLAUSE: [ADD v02.168] Task Board projections and Role Mailbox exports MUST remain field-equivalent at the base-envelope level and obey their nested structured record contracts | WHY_IN_SCOPE: current shared validation checks outer arrays but not nested `rows[]`, `threads[]`, or `transcription_links[]` element shape | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/role_mailbox.rs | EXPECTED_TESTS: cargo test -p handshake_core task_board nested_validation; cargo test -p handshake_core role_mailbox nested_validation | RISK_IF_MISSED: malformed nested payloads pass smoke tests and fail only in downstream consumers
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 typed timestamp, artifact-handle, and sha256 fields | WHY_IN_SCOPE: current shared validation treats these fields as non-empty strings even though the spec gives them typed semantics | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/role_mailbox.rs | EXPECTED_TESTS: cargo test -p handshake_core role_mailbox structured_field_formats | RISK_IF_MISSED: mailbox exports keep a split validation standard where mailbox-local code is stricter than the shared record validator
  - CLAUSE: Mechanical gate (HARD) RoleMailboxExportGate requires valid thread-line field sets and required transcription links | WHY_IN_SCOPE: the live smoke-test claim depends on malformed mailbox exports being caught deterministically before closure | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/tests/role_mailbox_tests.rs | EXPECTED_TESTS: cargo test -p handshake_core role_mailbox export_gate_inputs | RISK_IF_MISSED: mailbox smoke coverage remains optimistic and misses spec-grade export corruption

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: TrackedWorkPacket structured record validation | PRODUCER: src/backend/handshake_core/src/workflows.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and downstream viewers | SERIALIZER_TRANSPORT: serde JSON (packet.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core schema_registry workflow_state | DRIFT_RISK: emitted workflow-state fields exist but malformed or missing values still pass the shared validator
  - CONTRACT: TrackedMicroTask structured record validation | PRODUCER: src/backend/handshake_core/src/workflows.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and downstream runtimes | SERIALIZER_TRANSPORT: serde JSON (packet.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core schema_registry workflow_state | DRIFT_RISK: same portable workflow-state gap as work packets
  - CONTRACT: TaskBoardIndexV1 and TaskBoardViewV1 nested rows | PRODUCER: src/backend/handshake_core/src/workflows.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and task-board viewers | SERIALIZER_TRANSPORT: serde JSON (index.json / views/<view_id>.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core task_board nested_validation | DRIFT_RISK: row arrays remain well-typed only at the outer level while inner row objects silently drift
  - CONTRACT: RoleMailboxIndexV1 nested threads | PRODUCER: src/backend/handshake_core/src/role_mailbox.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and mailbox viewers | SERIALIZER_TRANSPORT: serde JSON (index.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core role_mailbox nested_validation | DRIFT_RISK: mailbox index threads keep their own informal schema instead of the shared typed contract
  - CONTRACT: RoleMailboxThreadLineV1 typed fields and transcription links | PRODUCER: src/backend/handshake_core/src/role_mailbox.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and mailbox export consumers | SERIALIZER_TRANSPORT: JSONL (threads/<thread_id>.jsonl) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core role_mailbox structured_field_formats | DRIFT_RISK: shared validation accepts malformed timestamps, handle strings, or sha256 values that mailbox-local code would reject

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test -p handshake_core schema_registry_workflow_state_fields_required
  - cargo test -p handshake_core task_board_nested_record_validation
  - cargo test -p handshake_core role_mailbox_typed_structured_validation
  - cargo test -p handshake_core role_mailbox_export_nested_validation
- CANONICAL_CONTRACT_EXAMPLES:
  - Mutated Work Packet `packet.json` missing `workflow_state_family`
  - Mutated Micro-Task `packet.json` missing `allowed_action_ids`
  - Mutated Task Board `index.json` with malformed `rows[0]`
  - Mutated Role Mailbox `index.json` with malformed `threads[0].created_at`
  - Mutated Role Mailbox thread line with invalid `body_ref`, `body_sha256`, or `transcription_links[0].target_sha256`

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Add small shared validation helpers in `src/backend/handshake_core/src/locus/types.rs` for RFC3339 timestamps, lowercase hex sha256 strings, artifact-handle strings, required workflow-state fields, and nested object arrays.
  - Extend `validate_structured_collaboration_record()` to require `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` on `WorkPacketPacket`, `MicroTaskPacket`, and `TaskBoardEntry`.
  - Reuse the TaskBoard entry contract to validate nested `rows[]` for Task Board index and view records.
  - Validate nested Role Mailbox `threads[]` and `transcription_links[]` element shapes at the shared validator boundary, keeping the implementation narrow and spec-driven.
  - Reuse mailbox-local parsing helpers only if doing so does not create cross-module coupling or circular dependencies; otherwise add minimal equivalent shared-format validators in `locus/types.rs`.
  - Add mutation-based negative tests in `micro_task_executor_tests.rs` and `role_mailbox_tests.rs` that operate on emitted JSON artifacts before validation.
- HOT_FILES:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core schema_registry
  - cargo test -p handshake_core task_board
  - cargo test -p handshake_core role_mailbox
  - cargo test -p handshake_core
- CARRY_FORWARD_WARNINGS:
  - Do not reopen the already-closed v3 scope around emitter basics or structured-diagnostic introduction unless the current code proves a concrete regression.
  - Keep the change centered on the shared validator and tests; do not widen into Loom portability or `.GOV` governance scripts.
  - Recursive nested validation should stay shallow and explicit enough to audit; avoid a speculative generic schema engine.
  - Do not let mailbox-local stricter validation remain the only typed guard for exported thread-line fields.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - [ADD v02.171] required workflow-state fields on canonical Work Packet, Micro-Task, and Task Board records
  - [ADD v02.168] nested Task Board and Role Mailbox payload contracts
  - RoleMailboxIndexV1 and RoleMailboxThreadLineV1 typed timestamp, artifact-handle, and sha256 field semantics
  - Mechanical gate intent for mailbox exports as proven through malformed export-input tests
- FILES_TO_READ:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core schema_registry
  - cargo test -p handshake_core task_board
  - cargo test -p handshake_core role_mailbox
  - rg -n "workflow_state_family|queue_reason_code|allowed_action_ids|rows|threads|transcription_links|body_ref|body_sha256|target_sha256|created_at|generated_at|updated_at" src/backend/handshake_core
- POST_MERGE_SPOTCHECKS:
  - Verify emitted smoke-test artifacts still pass happy-path validation after the validator becomes stricter.
  - Verify malformed nested rows and transcription links fail at the shared validator boundary, not only in mailbox-local code.
  - Verify no scope drift into Loom files, `.GOV` files, or new schema ids.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Whether the cleanest implementation is shared helper reuse from `role_mailbox.rs` or new minimal typed validators inside `locus/types.rs`
  - How many existing happy-path test fixtures will need touch-ups once nested element validation becomes strict
  - Whether current emitted task-board and mailbox fixtures already contain any silently tolerated malformed fields that the new validator will surface immediately

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Main Body explicitly names the shared base envelope, the portable workflow-state contract, the Task Board structured projection rules, the Role Mailbox record shapes, and the mailbox export gate. The missing work is concrete validator behavior and concrete negative-path proof, not interpretation.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.178.md already defines the exact portable workflow-state fields, nested structured record families, and typed Role Mailbox export semantics this packet needs to enforce. No new normative text is required.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Base structured schema and project-profile extension contract [ADD v02.168]
- CONTEXT_START_LINE: 6839
- CONTEXT_END_LINE: 6882
- CONTEXT_TOKEN: **Base structured schema and project-profile extension contract** [ADD v02.168]
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Base structured schema and project-profile extension contract** [ADD v02.168]

  - Every canonical collaboration artifact family member SHALL implement one shared base envelope before any profile-specific fields are applied. At minimum that base envelope MUST expose:
    - `schema_id`
    - `schema_version`
    - `record_id`
    - `record_kind`
    - `project_profile_kind`
    - `updated_at`
    - `mirror_state`
    - `authority_refs`
    - `evidence_refs`
  - The base envelope MUST remain valid even when no project-profile extension is present. Software-delivery fields such as repository branch names, worktree paths, coding-language hints, or continuous-integration gate identifiers SHALL move into `profile_extension` payloads rather than becoming universal required fields.
  - `project_profile_kind` SHALL be stable and low-cardinality. Phase 1 default kinds are `software_delivery`, `research`, `worldbuilding`, `design`, `generic`, and `custom`.
  - `profile_extension` payloads MUST declare `extension_schema_id`, `extension_schema_version`, and `compatibility` so migration and validation tooling can reject unknown breaking extensions deterministically.
  - `mirror_state` SHALL be one of:
    - `canonical_only`
    - `synchronized`
    - `stale`
    - `advisory_edit`
    - `normalization_required`
  - Implementations MAY denormalize base-envelope fields into top-level record keys, but Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST remain field-equivalent at the base-envelope level so shared viewers, validators, and local-small-model ingestion can reuse the same parser.

  **Compact summary contract for local small models** [ADD v02.168]

  - Every canonical `packet.json`, `index.json`, or `thread.jsonl` collaboration artifact family member SHOULD have a paired bounded summary view that smaller local models can ingest without loading the full long-form note history.
  - `summary.json` records SHOULD implement `StructuredCollaborationSummaryV1` and MUST preserve:
    - `record_id`
    - `record_kind`
    - `project_profile_kind`
    - `status`
    - `title_or_objective`
    - `blockers`
    - `next_action`
    - `authority_refs`
    - `evidence_refs`
    - `updated_at`
  - Dev Command Center, Task Board derived layouts, Role Mailbox triage, and local-small-model planning flows SHOULD default to the compact summary contract first and load canonical detail records or Markdown sidecars only on demand.

  **Task Board and Role Mailbox structured projections** [ADD v02.168]

  - Task Board projection rows SHOULD be serialized as `record_kind=task_board_entry` records that reuse the same base envelope and add only board-specific fields such as `task_board_id`, `work_packet_id`, `lane_id`, `display_order`, and optional `view_ids`.
  - Role Mailbox exports SHOULD reuse the same base envelope for thread indexes and append-only thread lines. Message-level records SHOULD add only mailbox-specific fields such as `thread_id`, `message_type`, `from_role`, `to_roles`, `expected_response`, and `expires_at`.
  - When a collaboration artifact supports both canonical detail and compact summary representations, both records MUST share the same `record_id`, `project_profile_kind`, and authoritative references so deterministic joins remain possible without transcript reconstruction or Markdown parsing.
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
  - `workflow_state_family` MUST stay low-cardinality and project-agnostic. Phase 1 base families are:
    - `intake`
    - `ready`
    - `active`
    - `waiting`
    - `review`
    - `approval`
    - `validation`
    - `blocked`
    - `done`
    - `canceled`
    - `archived`
  - The families SHALL be interpreted as:
    - `intake`: known work that still requires triage or decomposition.
    - `ready`: executable work with enough context, dependencies, and permissions to begin.
    - `active`: work currently being executed by a human, local small model, cloud model, or workflow.
    - `waiting`: work expected to resume after an external response, dependency, or scheduled retry.
    - `review`: work awaiting human or model review rather than new execution.
    - `approval`: work awaiting an explicit governance or operator decision.
    - `validation`: work awaiting deterministic checks, rubric checks, or acceptance verification.
    - `blocked`: work that cannot progress safely until a blocker is cleared.
    - `done`: work completed but still visible to current operating views.
    - `canceled`: work explicitly stopped and not expected to resume automatically.
    - `archived`: closed work retained for history, evidence, or search only.
  - `queue_reason_code` MUST explain why the record is currently routed or grouped where it is. Phase 1 base reasons are:
    - `needs_triage`
    - `dependency_wait`
    - `mailbox_response_wait`
    - `mailbox_snoozed`
    - `human_review_wait`
    - `decision_wait`
    - `approval_wait`
    - `validation_wait`
    - `escalation_wait`
    - `mailbox_expired`
    - `dead_letter_remediation`
    - `operator_pause`
    - `policy_block`
    - `resource_unavailable`
    - `retry_scheduled`
    - `ready_for_local_small_model`
    - `ready_for_cloud_model`
    - `completed`
    - `rejected`
    - `canceled`
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
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md RoleMailboxIndexV1 and RoleMailboxThreadLineV1 base-envelope contract
- CONTEXT_START_LINE: 11023
- CONTEXT_END_LINE: 11083
- CONTEXT_TOKEN: interface RoleMailboxIndexV1 {
- EXCERPT_ASCII_ESCAPED:
  ```text
  interface RoleMailboxIndexV1 {
    schema_id: 'hsk.role_mailbox_index@1';
    schema_version: 'role_mailbox_export_v1';
    record_id: 'role_mailbox_index';
    record_kind: 'generic';
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    updated_at: string; // RFC3339; equals generated_at for full export snapshots
    generated_at: string; // RFC3339
    authority_refs: string[];
    evidence_refs: string[];
    mirror_contract?: MarkdownMirrorContractV1;
    threads: Array<{
      thread_id: string;
      created_at: string; // RFC3339
      closed_at?: string | null; // RFC3339
      participants: string[]; // RoleId rendered as strings
      context: {
        spec_id?: string | null;
        work_packet_id?: string | null;
        task_board_id?: string | null;
        governance_mode: 'gov_strict' | 'gov_standard' | 'gov_light';
        project_id?: string | null;
      };
      subject_redacted: string; // MUST be Secret-Redactor output; bounded
      subject_sha256: string;   // sha256 of original subject bytes (UTF-8)
      message_count: number;
      thread_file: string; // "threads/<thread_id>.jsonl"
    }>;
  }

  // docs/ROLE_MAILBOX/threads/<thread_id>.jsonl (one JSON object per line)
  // This is a canonical JSON encoding of RoleMailboxMessage, but MUST NOT include any inline body.
  type RoleMailboxThreadLineV1 = {
    schema_id: 'hsk.role_mailbox_thread_line@1';
    schema_version: 'role_mailbox_export_v1';
    record_id: string;
    record_kind: 'role_mailbox_message';
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    updated_at: string; // RFC3339; equals created_at unless a mailbox export rewraps the same canonical message
    message_id: string;
    thread_id: string;
    created_at: string; // RFC3339
    from_role: string;
    to_roles: string[];
    message_type: string;
    authority_refs: string[];
    evidence_refs: string[];
    mirror_contract?: MarkdownMirrorContractV1;
    body_ref: string;    // artifact handle string
    body_sha256: string; // sha256
    attachments: string[];
    relates_to_message_id?: string | null;
    transcription_links: Array<{
      target_kind: string;
      target_ref: string;
      target_sha256: string;
      note_redacted: string; // MUST be Secret-Redactor output; bounded
      note_sha256: string;   // sha256 of original note bytes (UTF-8)
    }>;
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Mechanical gate (HARD): RoleMailboxExportGate
- CONTEXT_START_LINE: 11115
- CONTEXT_END_LINE: 11125
- CONTEXT_TOKEN: Mechanical gate (HARD): RoleMailboxExportGate
- EXCERPT_ASCII_ESCAPED:
  ```text
  Mechanical gate (HARD): RoleMailboxExportGate
  - The runtime MUST provide a mechanical gate that verifies the export is in sync and leak-safe.
  - The gate MUST fail if:
    - `export_manifest.json` hashes do not match current `index.json` / thread files,
    - any thread JSONL line is not valid JSON or violates the RoleMailboxThreadLineV1 field set,
    - any governance-critical message lacks required `transcription_links`,
    - any export file contains forbidden inline body fields (e.g., `body`, `body_text`, `raw_body`).
  - The repo MUST expose the gate as a deterministic command and integrate it into the standard workflow gates:
    - Script: `scripts/validation/role_mailbox_export_check.mjs`
    - Command: `just role-mailbox-export-check`
    - Inclusion: `just post-work {WP_ID}` MUST run this gate in GOV_STANDARD/GOV_STRICT workflows.
  ```
