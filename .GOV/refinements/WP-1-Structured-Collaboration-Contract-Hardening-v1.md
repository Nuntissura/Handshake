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
- WP_ID: WP-1-Structured-Collaboration-Contract-Hardening-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-25T05:07:25.9706637+01:00
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: e658a3b8a2d7cdd0d294838151d24a60bc3e034c
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja250320260532
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Structured-Collaboration-Contract-Hardening-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- `src/backend/handshake_core/src/workflows.rs` still emits `allowed_action_ids` from family-default ad hoc verbs such as `triage`, `assign`, `pause`, `request_changes`, `repair`, `unblock`, and `reopen` instead of from registered `GovernedActionDescriptorV1.action_id` values.
- `src/backend/handshake_core/src/storage/locus_sqlite.rs` still emits the same ad hoc `allowed_action_ids` values in the SQLite-backed micro-task progress path, so the weaker contract exists in more than one producer.
- `src/backend/handshake_core/src/locus/types.rs` still validates `allowed_action_ids` only as a string array and does not prove that each entry resolves to a registered governed action descriptor.
- `src/backend/handshake_core/src/workflows.rs` still derives Task Board row `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` from `TaskBoardStatus` heuristics rather than preserving authoritative workflow semantics from linked backend records.
- `src/backend/handshake_core/src/locus/types.rs` still validates `subject_redacted` and `note_redacted` only as non-empty strings rather than as bounded, single-line redacted outputs, even though the spec requires leak-safe export proof.
- Existing tests in `micro_task_executor_tests.rs` and `role_mailbox_tests.rs` still lack negative-path proof for unregistered action ids, Task Board row workflow fidelity, and malformed redacted mailbox export fields.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 95m
- SEARCH_SCOPE: current Master Spec v02.178 workflow-state, governed-action, Task Board, and Role Mailbox sections; live product code in `src/backend/handshake_core`; current smoke-test and contract tests in `src/backend/handshake_core/tests`
- REFERENCES: Internal spec-to-code remediation only. Primary sources were `.GOV/spec/Handshake_Master_Spec_v02.178.md`, `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/storage/locus_sqlite.rs`, `src/backend/handshake_core/src/role_mailbox.rs`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs`, `src/backend/handshake_core/tests/role_mailbox_tests.rs`, and `.GOV/Audits/smoketest/AUDIT_20260325_SCHEMA_REGISTRY_V4_SMOKETEST_RECOVERY_REVIEW.md`.
- PATTERNS_EXTRACTED: one canonical registry surface should own governed action ids; producer and validator paths should share the same action-id legality checks; Task Board projections should preserve workflow semantics from authoritative records rather than recomputing them from board lanes; leak-safe mailbox export proof should validate bounded redacted outputs at the same mechanical boundary that validates typed fields.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT one shared action-registry helper and route all emitters plus validators through it; ADAPT the current negative-path test style that mutates exported JSON at the consumer boundary; REJECT widening this pass into Loom portability, repo-governance harness work, or broad Dev Command Center UI redesign.
- LICENSE/IP_NOTES: Internal repository and spec inspection only. No third-party code or text is intended for direct reuse.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.178.md already defines the project-agnostic workflow-state contract, the governed action descriptor law, the Task Board projection portability rule, the Role Mailbox export schema, and the leak-safe mailbox export gate. The missing work is implementation and proof hardening against current Main Body text.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is a strictly internal spec-to-code remediation pass against the current Handshake Master Spec and the current local product code. No external ecosystem signal is needed to decide scope.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Internal sources are sufficient because the unresolved work is explicit in the current Master Spec and in the current local producers and validators.
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
- Existing mailbox export telemetry remains the relevant leak-safe evidence seam, and existing workflow artifacts should continue to emit through their current event families.
- Validator failures in this packet should surface as deterministic structured validation issues rather than new ad hoc runtime strings.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: ad hoc `allowed_action_ids` let downstream consumers treat UI verbs as authority instead of using a governed registry contract. Mitigation: create one canonical registry surface and reject unregistered ids at validation boundaries.
- Risk: Task Board rows can flatten real routing semantics into lane-level heuristics, which hides true wait/block/review state. Mitigation: preserve linked workflow-state triplets from authoritative records into projection rows.
- Risk: mailbox export validation can still accept leak-unsafe redacted fields as long as they are non-empty strings. Mitigation: validate bounded, single-line redacted outputs mechanically at the export gate boundary.
- Risk: SQLite-backed progress metadata can silently reintroduce weaker action semantics after the main artifact writer is fixed. Mitigation: align or retire alternate emitters in the same packet rather than trusting one producer path only.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
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
  - The spec already defines the primitives required here. This packet hardens how existing runtime producers and validators use those primitives rather than inventing new primitive ids.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current spec appendix already names the workflow-state, governed-action, Task Board, and Role Mailbox primitives involved in this packet.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Implementation should align runtime behavior to the existing primitive law rather than introduce new primitive ids.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new orphan primitives were discovered during this remediation pass.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: The feature and primitive registry already describes the workflow-state and mailbox export capability surfaces involved here.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet is backend contract hardening and proof work. No direct UI surface is implemented here.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The current interaction matrix is sufficient for this implementation-hardening pass.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep Appendix 12 unchanged and implement against current Main Body law.
  - If coding uncovers a truly missing runtime registry or interaction edge that cannot be expressed with current primitives, that should become a separate spec-update flow instead of silent packet drift.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene contract is changed by workflow-state and mailbox validation hardening | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected here | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physical simulation or measurement law is involved in this packet | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes remain downstream consumers of the hardened records | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing capability is changed by this remediation | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: TOUCHED | NOTES: governed action ids and workflow-state preservation are orchestration-facing runtime contracts even though this packet stays in backend implementation surfaces | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition or sequencing contract is affected | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is touched | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication surfaces stay downstream of the validation boundary | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking or recipe workflow surface is relevant here | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed by structured record hardening | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no routing or delivery engine behavior is altered directly in this packet | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: this packet hardens canonical artifact law for packet, task-board, and mailbox records | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval layers consume these records later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analysis surfaces remain downstream consumers of the hardened contracts | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is changed directly | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: the SQLite-backed progress path is directly in scope because it currently emits weaker action semantics | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this pass implements already-declared law and does not add a new governance authority surface | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or guidance interface is added here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: explicit workflow-state and governed-action semantics keep local-model routing and compact record reads trustworthy | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: this packet hardens versioned structured contracts for action ids, task-board projections, and mailbox export validation | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required for this pass | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: existing event families stay intact; this packet hardens contracts, not telemetry taxonomy | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: calendar storage and policy surfaces are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: editor surfaces are downstream consumers only | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: document editing is not changed by contract hardening | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: spreadsheet surfaces are out of scope | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus owns the shared validator, workflow-state enums, structured record families, and durable runtime law targeted by this packet | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Loom portability remains a separate smoke-test lane and stays file-disjoint from this pass | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Work Packet artifacts are in scope as downstream record families, but the implementation work stays centered in shared Locus producers and validators rather than packet-specific product surfaces | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Task Board row semantics are corrected through shared backend projection code rather than through a board-specific product-surface expansion | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: canonical Micro-Task artifacts and SQLite-backed progress metadata still emit ad hoc action ids and therefore stay in scope | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: Command Center remains a downstream consumer of the hardened backend contracts; no direct UI implementation is planned in this packet | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: memory-system behavior is not changed directly here | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: NOT_TOUCHED | NOTES: runtime execution is affected only indirectly through stricter contract validation | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no Spec Router or prompt compilation contract is expanded here | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: the SQLite-backed micro-task progress path is aligned only to prevent storage-local contract drift; this packet does not change the migration posture itself | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: portable workflow-state fields and leak-safe mailbox exports keep compact machine-readable records safe for model routing | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Stage artifact contracts are unrelated to the workflow-state and mailbox export surfaces here | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Studio runtime behavior is out of scope | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: UI viewer follow-on work remains downstream | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: distillation workflows are unaffected by this packet | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE runtime protocol or tool contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval systems may consume these records later but are not changed in this WP | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: Governed action registry-backed action ids on canonical structured collaboration records | SUBFEATURES: Work Packet packet and summary records, Micro-Task packet and summary records, shared validator legality checks | PRIMITIVES_FEATURES: PRIM-GovernedActionDescriptorV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask | MECHANICAL: engine.director, engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: current producers still emit ad hoc verbs and the shared validator still accepts any string array
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Preserve authoritative workflow-state triplets into Task Board rows | SUBFEATURES: row `workflow_state_family`, `queue_reason_code`, `allowed_action_ids`, lane projection from linked backend truth rather than lane heuristic defaults | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-TrackedWorkPacket | MECHANICAL: engine.director, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: current row emission derives semantics from `TaskBoardStatus`
  - PILLAR: MicroTask | CAPABILITY_SLICE: SQLite-backed progress metadata contract alignment | SUBFEATURES: micro-task progress metadata `allowed_action_ids`, shared action-registry helper reuse, parity with canonical artifact writers | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-GovernedActionDescriptorV1 | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: alternate producer paths must not preserve weaker semantics after the main artifact writer is fixed
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Leak-safe Role Mailbox export validation | SUBFEATURES: bounded `subject_redacted`, bounded `note_redacted`, shared export gate rejection of malformed redacted fields | PRIMITIVES_FEATURES: PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1 | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: current emitter is stronger than the validator and gate proof must close that mismatch
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Negative-path proof of structured collaboration law | SUBFEATURES: rejection of unregistered action ids, rejection of workflow-projection drift, rejection of malformed redacted export text | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry, PRIM-RoleMailboxThreadLineV1 | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this packet should close semantic proof, not only implementation wiring
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Registry-backed `allowed_action_ids` for Work Packet and Micro-Task artifacts | JobModel: WORKFLOW | Workflow: Locus artifact emission | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: emitted action ids must stop being ad hoc verbs and resolve through one canonical runtime registry helper
  - Capability: Task Board projection fidelity from authoritative workflow-state triplets | JobModel: WORKFLOW | Workflow: Task Board sync and projection export | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: row semantics must come from linked backend truth before lane aliasing or display grouping
  - Capability: SQLite-backed micro-task progress metadata parity | JobModel: WORKFLOW | Workflow: micro-task progress persistence and export | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: alternate emitters must align to the same action registry contract
  - Capability: Leak-safe mailbox export validation | JobModel: MECHANICAL_TOOL | Workflow: RoleMailbox export gate | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: FR-EVT-GOV-MAILBOX-002 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: malformed bounded-redaction fields must fail the same mechanical gate that validates thread-line field sets
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - The highest-value combinations are backend-only and already represented by existing primitives. This packet should harden those combinations rather than introduce new UI or tool surfaces.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: This is a contract-hardening packet for already-declared primitive interactions rather than a new combination surface.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This is a strictly internal spec-to-code hardening pass with no external product-design dependency.
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
  - Combo: Locus governed-action registry foundation | Pillars: Locus | Mechanical: engine.director | Primitives/Features: PRIM-GovernedActionDescriptorV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: one canonical registry helper removes producer and validator drift at the root contract surface
  - Combo: Locus action registry wired into Work Packet artifacts | Pillars: Locus | Mechanical: engine.version | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-GovernedActionDescriptorV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: Work Packet artifacts must stop emitting ad hoc verbs even though the implementation remains inside shared workflow emitters
  - Combo: MicroTask action registry wired into canonical artifacts | Pillars: MicroTask | Mechanical: engine.version | Primitives/Features: PRIM-TrackedMicroTask, PRIM-GovernedActionDescriptorV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: Micro-Task packet and summary artifacts must emit only governed action ids
  - Combo: MicroTask SQLite progress parity | Pillars: MicroTask | Mechanical: engine.dba | Primitives/Features: PRIM-TrackedMicroTask | Resolution: IN_THIS_WP | Stub: NONE | Notes: the SQLite-backed progress path must not preserve weaker action semantics than the canonical artifact writer
  - Combo: Locus validator action-id legality checks | Pillars: Locus | Mechanical: engine.archivist | Primitives/Features: PRIM-GovernedActionDescriptorV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask | Resolution: IN_THIS_WP | Stub: NONE | Notes: registry-backed legality must be enforced at the shared validator boundary, not only by emitters
  - Combo: Locus Task Board workflow truth preservation | Pillars: Locus | Mechanical: engine.context | Primitives/Features: PRIM-TaskBoardEntry, PRIM-TrackedWorkPacket | Resolution: IN_THIS_WP | Stub: NONE | Notes: Task Board row semantics must preserve authoritative workflow truth before any lane aliasing
  - Combo: LLM-friendly mailbox export leak-safety | Pillars: LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: compact model-readable exports must prove bounded redacted fields rather than trusting happy-path emitter behavior
  - Combo: Mailbox export gate plus typed redaction validation | Pillars: LLM-friendly data | Mechanical: engine.archivist | Primitives/Features: PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: malformed redacted text must fail the same mechanical gate that already validates thread-line shape
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: The highest-ROI combinations are already inside this packet's narrow product-hardening scope.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed structured-collaboration packets, current product code in `src/backend/handshake_core`, and the 2026-03-25 smoketest review
- MATCHED_STUBS:
  - Artifact: WP-1-Structured-Collaboration-Contract-Hardening-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this stub already captures the correct follow-on scope and should be activated and refined rather than replaced
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - NONE
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: still emits ad hoc action ids and still derives Task Board row workflow semantics from TaskBoardStatus
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: still emits family-default ad hoc action ids in the SQLite-backed micro-task progress path
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/types.rs | Artifact: NONE | Covers: primitive | Verdict: PARTIAL | Notes: still validates allowed_action_ids too shallowly and still treats redacted mailbox fields as generic non-empty strings
  - Path: ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: bounded redaction exists in the emitter, but the mechanical proof boundary remains incomplete
  - Path: ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs | Artifact: NONE | Covers: combo | Verdict: PARTIAL | Notes: still lacks negative-path proof for unregistered action ids and Task Board row fidelity
  - Path: ../handshake_main/src/backend/handshake_core/tests/role_mailbox_tests.rs | Artifact: NONE | Covers: combo | Verdict: PARTIAL | Notes: still lacks negative-path proof for malformed redacted export fields
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The tracked stub is the correct artifact to reuse, but its draft scope had to be expanded and hardened against the live code and audit reality before it can become a real packet.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet is backend contract hardening and proof work. No direct UI surface is being designed or changed.
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
- UI_UX_VERDICT: NOT_APPLICABLE

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No GUI implementation work is planned in this packet.
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
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Role-Mailbox
- BUILD_ORDER_BLOCKS: NONE
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md workflow-state, governed-action, Task Board projection, and RoleMailbox export-gate contracts [ADD v02.171]
- WHAT: Close the remaining Master Spec product gaps left after Schema Registry v4 by replacing ad hoc action ids with governed action descriptors, preserving authoritative Task Board workflow semantics, and hardening leak-safe mailbox export validation.
- WHY: The 2026-03-25 smoketest review proved that v4 fixed the original shallow-validator defects but did not deliver full Master Spec correctness for governed actions, Task Board projection fidelity, or mailbox leak-safe validation.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- OUT_OF_SCOPE:
  - Broad Loom portability work
  - Repo-governance workflow-harness remediation
  - Broad Dev Command Center or Task Board UI redesign beyond the backend contract surfaces needed to prove correctness
  - New spec text or appendix version bumps unless coding proves a real Main Body gap
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- DONE_MEANS:
  - Every in-scope canonical structured-collaboration producer emits `allowed_action_ids` as registered `GovernedActionDescriptorV1.action_id` values only.
  - The shared validator rejects unregistered or malformed `allowed_action_ids`.
  - Task Board rows preserve authoritative `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` from linked backend truth instead of recomputing them from board-status heuristics.
  - RoleMailbox export validation rejects malformed, unbounded, or non-redacted `subject_redacted` and `note_redacted` values.
  - Negative-path tests prove all of the above failures are mechanically blocked.
- PRIMITIVES_EXPOSED:
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- SEARCH_TERMS:
  - GovernedActionDescriptorV1
  - allowed_action_ids
  - task_board_workflow_state
  - workflow_state_family
  - queue_reason_code
  - subject_redacted
  - note_redacted
  - RoleMailboxExportGate
- RUN_COMMANDS:
  ```bash
  rg -n "GovernedActionDescriptorV1|allowed_action_ids|task_board_workflow_state|subject_redacted|note_redacted|workflow_state_family|queue_reason_code" src/backend/handshake_core
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  ```
- RISK_MAP:
  - "Action-id registry hardening exposes hidden coupling between UI verbs and backend mutation routes" -> "emitters, validators, or consumers may fail until they all use one canonical registry helper"
  - "Task Board projection preservation needs a durable backend truth source" -> "row semantics can stay flattened if the implementation only renames heuristics"
  - "Mailbox redaction proof can look green on happy-path data while malformed fields still pass" -> "export gate remains weaker than the spec requires"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - The stub and board entries already exist and no new execution lane sequencing is being introduced at refinement time.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.171] canonical Work Packet and Micro-Task records SHALL expose governed `allowed_action_ids` rather than ad hoc verbs | WHY_IN_SCOPE: current emitters still synthesize action ids from workflow families and the spec explicitly forbids ad hoc UI verbs | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/storage/locus_sqlite.rs`, `src/backend/handshake_core/src/locus/types.rs` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` | RISK_IF_MISSED: downstream consumers continue reading the wrong contract even though the field exists
  - CLAUSE: [ADD v02.171] Task Board rows SHOULD expose portable workflow-state and queue-reason semantics rather than board-status heuristics | WHY_IN_SCOPE: current row emission still derives `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` from `TaskBoardStatus` | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board` | RISK_IF_MISSED: Task Board remains a lossy projection that hides true routing semantics
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 redacted fields MUST be bounded Secret-Redactor outputs | WHY_IN_SCOPE: current validator still accepts `subject_redacted` and `note_redacted` as generic non-empty strings | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/role_mailbox.rs`, `src/backend/handshake_core/tests/role_mailbox_tests.rs` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | RISK_IF_MISSED: export gate can pass leak-unsafe payloads as long as they are syntactically non-empty
  - CLAUSE: Mechanical gate (HARD) RoleMailboxExportGate must reject malformed export thread-line field sets | WHY_IN_SCOPE: the spec requires leak-safe rejection and the current negative-path proof does not cover malformed redacted outputs | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/role_mailbox.rs`, `src/backend/handshake_core/tests/role_mailbox_tests.rs` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | RISK_IF_MISSED: the mechanical gate remains weaker than the contract it claims to enforce

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: Work Packet `packet.json` and `summary.json` workflow-state triplet | PRODUCER: `src/backend/handshake_core/src/workflows.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and downstream viewers | SERIALIZER_TRANSPORT: serde JSON files | VALIDATOR_READER: `validate_structured_collaboration_record` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` | DRIFT_RISK: action ids remain ad hoc strings and the validator still accepts them
  - CONTRACT: Micro-Task packet and summary workflow-state triplet plus SQLite progress metadata | PRODUCER: `src/backend/handshake_core/src/workflows.rs` and `src/backend/handshake_core/src/storage/locus_sqlite.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and progress readers | SERIALIZER_TRANSPORT: serde JSON and SQLite-backed metadata JSON | VALIDATOR_READER: `validate_structured_collaboration_record` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` | DRIFT_RISK: alternate emitters stay weaker than the canonical artifact writer
  - CONTRACT: TaskBoardIndexV1 and TaskBoardViewV1 row workflow-state triplets | PRODUCER: `src/backend/handshake_core/src/workflows.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and Task Board viewers | SERIALIZER_TRANSPORT: serde JSON `index.json` and `views/<view_id>.json` | VALIDATOR_READER: `validate_structured_collaboration_record` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board` | DRIFT_RISK: row semantics are flattened from lane status instead of preserved from linked backend truth
  - CONTRACT: RoleMailboxIndexV1 thread metadata | PRODUCER: `src/backend/handshake_core/src/role_mailbox.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and mailbox viewers | SERIALIZER_TRANSPORT: JSON `index.json` | VALIDATOR_READER: `validate_structured_collaboration_record` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | DRIFT_RISK: bounded redacted subject fields remain syntactically valid but semantically weak
  - CONTRACT: RoleMailboxThreadLineV1 redacted notes and transcription links | PRODUCER: `src/backend/handshake_core/src/role_mailbox.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and `validate_runtime_mailbox_record` | SERIALIZER_TRANSPORT: JSONL thread files | VALIDATOR_READER: `validate_structured_collaboration_record` and mailbox export gate | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | DRIFT_RISK: malformed redacted fields can still pass because the validator only requires non-empty strings

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox`
- CANONICAL_CONTRACT_EXAMPLES:
  - Mutated Work Packet `packet.json` with an unregistered `allowed_action_ids[0]`
  - Mutated Micro-Task `packet.json` with an unregistered `allowed_action_ids[0]`
  - Mutated Task Board `index.json` row whose workflow-state triplet no longer matches linked backend truth
  - Mutated Role Mailbox `index.json` with multiline or oversized `subject_redacted`
  - Mutated Role Mailbox thread line with multiline or oversized `note_redacted`

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Add one canonical governed-action registry helper in `src/backend/handshake_core/src/locus/types.rs` or an adjacent Locus module, using `GovernedActionDescriptorV1` ids as the emitted and validated contract.
  - Route `src/backend/handshake_core/src/workflows.rs` Work Packet, Micro-Task, and Task Board emitters through that registry helper instead of family-default ad hoc verbs.
  - Route `src/backend/handshake_core/src/storage/locus_sqlite.rs` micro-task progress metadata through the same registry helper or retire the weaker emitter path.
  - Replace Task Board row workflow-state derivation from `TaskBoardStatus` with preservation of authoritative linked backend workflow semantics.
  - Harden mailbox export validation in `src/backend/handshake_core/src/locus/types.rs` so `subject_redacted` and `note_redacted` prove bounded, single-line redacted form.
  - Add mutation-based negative tests in `micro_task_executor_tests.rs` and `role_mailbox_tests.rs` for unregistered action ids, Task Board projection drift, and malformed redacted export fields.
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- CARRY_FORWARD_WARNINGS:
  - Do not reopen the already-closed v4 scope around workflow-state field presence, nested row or thread validation, or typed timestamp and sha field enforcement unless the current code proves a concrete regression.
  - Keep the change centered on governed action ids, Task Board workflow fidelity, mailbox redacted-field validation, and negative-path proof. Do not widen into Loom portability or repo governance.
  - Avoid inventing a speculative generic workflow engine; one explicit governed-action registry helper is enough for this pass.
  - Do not let the SQLite-backed progress path remain a second-class producer with weaker semantics after the canonical artifact writer is fixed.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - [ADD v02.171] `allowed_action_ids` resolves to governed action descriptors rather than ad hoc verbs
  - [ADD v02.171] Task Board rows preserve explicit workflow-state and queue-reason semantics from authoritative backend records
  - RoleMailboxIndexV1 and RoleMailboxThreadLineV1 redacted fields are mechanically validated as bounded redacted outputs
  - Mechanical gate (HARD) RoleMailboxExportGate rejects malformed redacted export payloads
- FILES_TO_READ:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  - rg -n "GovernedActionDescriptorV1|allowed_action_ids|task_board_workflow_state|subject_redacted|note_redacted|workflow_state_family|queue_reason_code" src/backend/handshake_core
- POST_MERGE_SPOTCHECKS:
  - Verify no producer path still emits ad hoc action verbs after the main emitters are fixed.
  - Verify Task Board row workflow semantics are preserved from linked backend truth rather than only renamed heuristics.
  - Verify malformed `subject_redacted` and `note_redacted` payloads fail at the shared validation and export-gate boundary.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Whether the cleanest governed-action registry implementation should live fully in `locus/types.rs` or in a nearby Locus helper module
  - Whether preserving Task Board row semantics will require additional durable metadata beyond what the current status-driven projection path exposes
  - Whether any existing mailbox export fixtures already contain tolerated redacted-field shapes that the stricter validator will surface immediately

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Main Body explicitly names governed action descriptors, portable workflow-state fields, Task Board row workflow projection rules, Role Mailbox bounded redacted fields, and the mailbox export mechanical gate. The missing work is concrete implementation and proof, not interpretation.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.178.md already defines the exact governed-action, Task Board projection, and RoleMailbox leak-safe export behavior that this packet needs to implement and prove.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
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

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Task Board projection viewer workflow portability [ADD v02.171]
- CONTEXT_START_LINE: 60910
- CONTEXT_END_LINE: 60922
- CONTEXT_TOKEN: [ADD v02.171] Board rows SHOULD expose `workflow_state_family` and `queue_reason_code` separately from any project-specific display label so queue semantics remain portable across project kernels.
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **Task Board projection viewer**
    - Show structured board rows keyed by stable `task_board_id` and `work_packet_id`, plus freshness, manual-edit detection, and sync status.
    - Any Markdown board is read-only by default from this view unless a governed sync or status-update workflow is being invoked.
    - [ADD v02.168] Board rows SHOULD expose the base-envelope status, next action, blockers, and project-profile kind before board-specific grouping metadata.
    - [ADD v02.170] Board, list, queue, and roadmap layouts SHOULD read from the same row set and declare which lane definitions, grouping keys, and action bindings are active for the current preset.
    - [ADD v02.171] Board rows SHOULD expose `workflow_state_family` and `queue_reason_code` separately from any project-specific display label so queue semantics remain portable across project kernels.
  - **Role Mailbox triage**
    - Show message type, expected response, expiry, evidence references, linked Work Packet or Micro-Task identifiers, and handoff completeness.
    - Role Mailbox remains non-authoritative, but Dev Command Center MUST make collaboration state queryable without reading transcript blobs line by line.
    - [ADD v02.168] Thread and message views SHOULD expose the shared base-envelope fields and any mailbox-specific profile extensions separately.
    - [ADD v02.170] Inbox-triage presets SHOULD group by expected response, expiry, linked work identifier, or escalation posture, and MUST keep any reply or escalation mutation visibly separate from non-authoritative message text.
    - [ADD v02.171] Mailbox rows SHOULD show when expected-response or escalation posture contributes to a linked record's `queue_reason_code`, without turning the mailbox thread into the authority for the linked record's `workflow_state_family`.
  ```

#### ANCHOR 3
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

  **Required queue and routing behavior**
  - Local-small-model queues SHOULD prefer records where:
    - `workflow_state_family=ready`
    - `queue_reason_code=ready_for_local_small_model`
  - Cloud-model routing SHOULD prefer records where:
    - `workflow_state_family=ready`
    - `queue_reason_code=ready_for_cloud_model`
      or
    - `workflow_state_family=waiting`
    - `queue_reason_code=escalation_wait`
  - Review and approval queues MUST distinguish:
    - `workflow_state_family=review`
    - `queue_reason_code=human_review_wait`
    - `workflow_state_family=approval`
    - `queue_reason_code=approval_wait`
  - Validation queues MUST use `workflow_state_family=validation` plus explicit validation reasons rather than generic blocked state.
  - Mailbox-linked waits MUST remain visible as `queue_reason_code=mailbox_response_wait`, but the mailbox thread itself MUST NOT become the authority for the linked record's state family.

  **Required action behavior**
  - `GovernedActionDescriptorV1` SHOULD be the reusable contract for verbs such as:
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md RoleMailboxIndexV1 and RoleMailboxThreadLineV1 base-envelope contract
- CONTEXT_START_LINE: 11023
- CONTEXT_END_LINE: 11084
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
    idempotency_key: string;
  };
  ```

#### ANCHOR 5
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
