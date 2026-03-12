## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Structured-Collaboration-Artifact-Family-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-08
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-12T19:00:55.6234539Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: d5225e6ab7a377fd574604160b9dd29bd12009da
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja120320262021
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Structured-Collaboration-Artifact-Family-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- The Master Spec already defines the canonical structured collaboration artifact family, base envelope, summary contract, mirror governance, and portable workflow-state fields; the gap is implementation, not specification.
- Current backend code exposes `TrackedWorkPacket` and `TrackedMicroTask`, but the runtime does not yet materialize the full canonical `packet.json` plus `summary.json` family with the spec-mandated base envelope fields.
- Task Board sync currently rewrites Markdown sections in `TASK_BOARD.md`; it does not yet publish the canonical `task_board/index.json` and `task_board/views/{view_id}.json` projection family.
- Role Mailbox export already writes `index.json` and thread JSONL, but the current export shape is mailbox-specific and does not yet fully converge on the shared structured-collaboration envelope and compact summary contract required by the newer spec additions.
- Runtime governance path helpers still expose `TASK_BOARD.md` and `ROLE_MAILBOX/` roots only; they do not yet model the broader `.handshake/gov/work_packets/` and `.handshake/gov/micro_tasks/` structured artifact family.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: Jira typed issue fields and board projections; GitHub Projects field-driven layouts; Backstage catalog descriptor and repo architecture; compact-context paper patterns for summary-first consumption; repo code reality in `src/backend/handshake_core`
- REFERENCES: Atlassian Jira Issue Fields docs; GitHub Projects roadmap layout docs; Backstage descriptor format docs; Backstage repository; FocusLLM paper; local Handshake spec plus current backend runtime files
- PATTERNS_EXTRACTED: typed canonical records separate from view layouts; low-cardinality base envelope plus extensible profile fields; summary-first consumption before detail hydration; canonical versus readable projection separation; stable schema/version identifiers carried through all emitted records
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT typed base-envelope records and projection-over-record layouts; ADAPT extensible profile metadata into Handshake `profile_extension` payloads with stronger authority and mirror semantics; REJECT making readable Markdown or UI layout state a peer authority to canonical records
- LICENSE/IP_NOTES: Reference-only research and docs review. No code or schema text is copied verbatim into product code from third-party sources.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The current Master Spec Main Body already covers the file family, base envelope, summary contract, mirror semantics, and workflow-state vocabulary that this WP needs. This activation is implementation and alignment work against v02.178, not a request for new normative spec text.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON_NO: N/A
- SOURCE_MAX_AGE_DAYS: 730
- SOURCE_LOG:
  - Source: Atlassian Jira Issue Fields docs | Kind: BIG_TECH | Date: 2026-03-12 | Retrieved: 2026-03-12T19:00:55Z | URL: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-fields/ | Why: validates the value of typed authority fields that drive multiple board and issue views without treating the board layout as the source of truth
  - Source: GitHub Projects roadmap layout docs | Kind: BIG_TECH | Date: 2026-03-12 | Retrieved: 2026-03-12T19:00:55Z | URL: https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-roadmap-layout | Why: shows multiple governed layouts rendered from the same underlying project records
  - Source: Backstage descriptor format docs | Kind: OSS_DOC | Date: 2026-03-12 | Retrieved: 2026-03-12T19:00:55Z | URL: https://backstage.io/docs/features/software-catalog/descriptor-format/ | Why: useful reference for stable base descriptors plus extensible metadata without overfitting everything into one universal schema
  - Source: Backstage repository | Kind: GITHUB | Date: 2026-03-12 | Retrieved: 2026-03-12T19:00:55Z | URL: https://github.com/backstage/backstage | Why: concrete OSS reference for catalog and projection architecture at repository scale
  - Source: FocusLLM paper | Kind: PAPER | Date: 2024-08-21 | Retrieved: 2026-03-12T19:00:55Z | URL: https://arxiv.org/abs/2408.11745 | Why: supports the summary-first, compaction-before-detail pattern that maps well to Handshake local-small-model ingestion
- RESEARCH_SYNTHESIS:
  - Handshake should keep one authoritative structured record family and let board, queue, and roadmap surfaces remain projections over that family.
  - The shared base envelope should stay intentionally small while project-specific fields move behind explicit extension boundaries.
  - Compact summaries should be treated as first-read artifacts for smaller models and operator triage, with canonical detail records and long note streams loaded only when needed.
  - External systems generally succeed when layout state is derived from typed records instead of turning UI position or prose into authority.
- RESEARCH_GAPS_TO_TRACK:
  - NONE
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: Atlassian Jira Issue Fields docs | Pattern: typed issue fields as authority while board and issue views stay projections | Why: directly reinforces the spec choice that canonical work state lives in records, not in view position
  - Source: GitHub Projects roadmap layout docs | Pattern: multiple layouts over a single underlying record set | Why: matches the Handshake requirement that queue, board, and roadmap layouts can vary without changing authoritative work state
- ADAPT_PATTERNS:
  - Source: Backstage descriptor format docs | Pattern: stable core descriptor plus extension metadata | Why: Handshake needs the same discipline, but with stronger mirror-state, evidence-ref, and compatibility handling for workflow authority and model-safe ingestion
  - Source: FocusLLM paper | Pattern: compressed summary first, hydrate detail only when needed | Why: the paper is about model context compression rather than workflow artifacts, but the same strategy improves local-small-model planning over packet and task state
- REJECT_PATTERNS:
  - Source: Backstage repository | Pattern: broad platform-oriented descriptor surface and plugin-driven metadata sprawl as the starting point | Why: Handshake Phase 1 needs a smaller, opinionated artifact family with deterministic authority rules rather than a large, loosely governed extension ecosystem
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - structured collaboration artifact family schema registry projection repo
  - catalog descriptor summary projection json repo
- MATCHED_PROJECTS:
  - Source: Backstage repository | Repo: backstage/backstage | URL: https://github.com/backstage/backstage | Intent: ARCH_PATTERN | Decision: ADAPT | Impact: NONE | Stub: NONE | Notes: useful as an architecture reference for stable record descriptors plus downstream projections, but Handshake already has separate downstream stubs for viewer and registry follow-on work
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Reuse existing event families; do not invent a new event namespace for this WP.
- Structured Work Packet emission and mutation should continue to route through `FR-EVT-WP-001` and `FR-EVT-WP-002`.
- Structured Micro-Task registration and mutation should continue to route through the existing Locus `FR-EVT-MT-001..006` family and the executor-side `FR-EVT-MT-001..017` family where applicable.
- Task Board structured projection publication and sync should continue to route through `FR-EVT-TB-001..003`.
- Role Mailbox structured export and transcription linkage should continue to route through `FR-EVT-GOV-MAILBOX-002` and `FR-EVT-GOV-MAILBOX-003`.
- No new Flight Recorder event IDs are needed for activation; this WP should make the existing event families emit against the new structured artifact family consistently.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: a derived Markdown mirror or stale summary could be mistaken for canonical state. Mitigation: emit explicit `mirror_state`, preserve authoritative references, and keep regeneration one-way unless normalized explicitly.
- Risk: summary records could drift from canonical detail. Mitigation: derive summaries mechanically from the same authoritative record and validate shared identity plus reference fields.
- Risk: profile extensions could smuggle software-delivery-only assumptions into the base envelope. Mitigation: keep the base envelope minimal and reject profile-specific required fields at the shared parser boundary.
- Risk: mailbox exports could silently diverge from the shared envelope or leak body content. Mitigation: preserve the existing leak-safe export rules and converge only the shared envelope fields, not inline free text.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-MirrorSyncState
  - PRIM-MarkdownMirrorContractV1
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-MirrorSyncState
  - PRIM-MarkdownMirrorContractV1
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - This WP should align runtime record emission to already-specified primitives rather than invent new appendix primitive ids.
  - The implementation gap is field coverage, file layout, and deterministic summary generation across these existing primitives.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The spec appendix already contains the relevant collaboration, mirror, mailbox, and workflow-state primitive ids; this WP is implementation alignment against those existing ids.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Runtime structs and emitted JSON should be aligned to the existing appendix primitive set rather than adding new primitive ids during activation.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: The high-signal collaboration primitives needed here are already present in Appendix 12.4; no orphan primitive discovery was required for activation.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: The feature family is already represented in the current spec and roadmap; activation does not require another feature-registry addition.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This WP is backend artifact-family implementation work. Direct operator surfaces are owned by the downstream structured artifact viewer and layout projection stubs.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Existing matrix coverage is sufficient for activation; this WP does not need a new appendix edge before implementation can start.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep the spec appendices unchanged and implement against the current v02.178 collaboration primitives and workflow-state vocabulary.
  - If implementation reveals a truly missing primitive or interaction edge, that becomes a new spec-update flow or a new WP variant rather than silent packet drift.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: structured collaboration artifacts do not change spatial reasoning or scene contracts in this WP | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure authoring surface is changed by the artifact family implementation | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physics simulation or measurement contracts are part of this scope | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation workflows are downstream consumers, not part of the artifact-family implementation | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-control contract is affected here | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration direction and multi-surface planning stay downstream of the canonical record implementation | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: media or composition sequencing is outside the collaboration artifact family scope | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering contract changes are introduced by this WP | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication surfaces may consume these records later, but are not implemented in this WP | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking or procedural kitchen workflows are changed here | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no safety rules or compliance surfaces are updated by this artifact implementation | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: routing and delivery flows may later consume the artifact family, but no logistics engine behavior is implemented now | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: this WP defines how durable collaboration records, summaries, note refs, and mirror linkage are stored on disk and reloaded deterministically | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval and catalog search may benefit later, but this WP stops at artifact emission and stable record shape | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytic views remain downstream consumers of the record family rather than in-scope work | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no external dataset wrangling behavior is changed by this collaboration artifact implementation | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: storage portability matters, but this WP does not yet change backend abstraction or migration policy directly | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: governance authority remains unchanged; this WP implements already-governed record law | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no direct guidance or tutoring surface is added by the record family implementation | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: the compact summary contract and summary-first loading path are explicit context-compaction surfaces for local-small-model planning | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: schema ids, schema versions, mirror state, and compatibility posture are first-class versioning concerns in this WP | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandboxing or isolation boundary changes are required to activate the structured artifact family | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: the WP reuses existing event families but does not define a new recorder capability surface | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: no calendar-specific data model or correlation contract changes are in scope | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: editor surfaces may consume these artifacts later, but are not part of this activation | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: document surfaces are not changed by the backend collaboration artifact family implementation | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: spreadsheet surfaces are out of scope for this WP | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus is the current backend home for tracked work records, sync, and queryable work state that this WP must extend into the canonical structured family | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: no media archive or Loom-specific runtime contract is being changed here | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: packet-detail product surfaces are downstream of this backend implementation pass; this WP focuses on the shared runtime record family underneath them | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: task-board product surfaces remain downstream consumers even though this WP adds the structured projection artifacts they will read | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: this WP directly governs canonical micro-task packet and summary records aligned with the shared base envelope | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: viewer and layout consumption are real downstream dependencies, but direct Command Center implementation lives in separate stubs | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: NOT_TOUCHED | NOTES: this WP emits runtime-readable state but does not add a new job model or execution orchestrator surface | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: prompt routing may later consume these records, but this activation is not a spec-router change | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: the record family should remain portable, but storage-backend migration work is not directly part of this WP | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: bounded `summary.json` records and explicit workflow-state fields are direct local-small-model ingestion surfaces | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: stage/media surfaces are not modified by this collaboration artifact activation | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: studio surfaces may later consume these records but are not implemented here | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: lens viewers are downstream consumers, not in-scope product work for this WP | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: distillation remains downstream of micro-task execution, not part of the canonical artifact family activation | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: ACE may later read these artifacts, but the WP does not modify ACE runtime behavior | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval modes stay downstream consumers; this WP focuses on canonical state emission rather than retrieval policy | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: Canonical collaboration record persistence | SUBFEATURES: shared base-envelope alignment for work packets, micro-tasks, task-board rows, and mailbox exports | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry, PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1 | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus already owns tracked work state and is the right backend surface for the canonical artifact family rollout
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Canonical packet detail plus bounded summary pair | SUBFEATURES: `packet.json`, `summary.json`, note refs, mirror refs, and portable workflow-state fields | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1, PRIM-MirrorSyncState, PRIM-MarkdownMirrorContractV1, PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is a first-class deliverable of the WP, not a downstream consumer concern
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Structured projection inventory and views | SUBFEATURES: `task_board/index.json`, `task_board/views/{view_id}.json`, stable row identity, and projection-safe mirror posture | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-MirrorSyncState, PRIM-MarkdownMirrorContractV1, PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this WP should supply the backend projection artifacts that later viewer work depends on
  - PILLAR: MicroTask | CAPABILITY_SLICE: Canonical micro-task detail plus bounded summary pair | SUBFEATURES: per-micro-task `packet.json`, `summary.json`, and shared envelope alignment with work packets | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1, PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task state already exists in runtime structs and should be promoted into the canonical artifact family in this pass
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Summary-first local-small-model ingestion | SUBFEATURES: bounded summary payloads, stable references, blockers, next action, and explicit workflow-state fields | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationSummaryV1, PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: smaller models should be able to operate from summaries without reopening long Markdown or replaying mailbox threads
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Work Packet canonical artifact emission | JobModel: WORKFLOW | Workflow: Locus work-packet persistence and sync | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-WP-001, FR-EVT-WP-002 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: authoritative work-packet records should be emitted from the existing backend work-tracking flow rather than from a UI-only export
  - Capability: Micro-Task canonical artifact emission | JobModel: WORKFLOW | Workflow: Locus micro-task registration and update flow | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-MT-001..006, FR-EVT-MT-001..017 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: micro-task records should align to the same base envelope and summary contract as work packets while preserving existing executor telemetry
  - Capability: Task Board structured projection export | JobModel: WORKFLOW | Workflow: task-board sync and projection rebuild | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-TB-001..003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: projection rows and view files should be generated from canonical state and remain queryable outside Markdown parsing
  - Capability: Role Mailbox structured export convergence | JobModel: WORKFLOW | Workflow: mailbox export and transcription-safe manifest update | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-GOV-MAILBOX-002, FR-EVT-GOV-MAILBOX-003 | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox export already exists and should be aligned to the shared envelope without regressing leak-safe export guarantees
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 45m
- MATRIX_SCAN_NOTES:
  - The relevant cross-primitive interactions for this WP are already represented in the current spec as structured collaboration, mirror governance, and workflow-state contracts.
  - Implementation work should consume those existing interactions rather than force a new appendix edge before backend alignment begins.
  - Local and cloud model compatibility improves through summary-first artifacts and explicit workflow-state fields, but that does not require a new Appendix 12.6 edge for activation.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Current spec matrix coverage is sufficient for activation; this WP is an implementation pass against already-specified collaboration, mirror, and workflow-state interactions rather than a new matrix-expansion pass.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON_NO: N/A
- SOURCE_SCAN:
  - Source: Atlassian Jira Issue Fields docs | Kind: BIG_TECH | Angle: typed issue authority versus board projection | Pattern: fields stay canonical while boards and issue views are projections | Decision: ADOPT | EngineeringTrick: centralize authoritative state in typed records and let view logic depend on those stable fields | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly supports the Work Packet, Micro-Task, and Task Board record split in this WP
  - Source: GitHub Projects roadmap layout docs | Kind: BIG_TECH | Angle: multiple layouts over one project record set | Pattern: one record family rendered as roadmap, table, or board without changing authority | Decision: ADOPT | EngineeringTrick: store layout semantics separately from the record payload so view switching does not mutate canonical state | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: reinforces the spec rule that board and queue layouts remain derived projections
  - Source: Backstage descriptor format docs | Kind: OSS_DOC | Angle: stable descriptor plus extension metadata | Pattern: shared base descriptor with extensible annotations and domain-specific additions | Decision: ADAPT | EngineeringTrick: keep extension metadata explicitly bounded so generic parsers still understand the shared core | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: Handshake needs stronger authority refs, evidence refs, and mirror semantics than a general software catalog
  - Source: Backstage repository | Kind: GITHUB | Angle: repository-scale architecture for catalog and projection surfaces | Pattern: rich plugin growth around a stable record backbone | Decision: REJECT | EngineeringTrick: avoid importing a broad plugin surface when the current need is a narrow canonical artifact family | ROI: MEDIUM | Resolution: REJECT_DUPLICATE | Stub: NONE | Notes: useful reference, but not a scope-expansion signal for this WP
  - Source: FocusLLM paper | Kind: PAPER | Angle: context compression before full-detail decode | Pattern: compact representation first, expand only when needed | Decision: ADAPT | EngineeringTrick: keep bounded summary artifacts cheap to load so smaller models and triage flows can defer full detail hydration | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: the paper is not a workflow tool, but the compaction pattern maps directly to Handshake `summary.json` behavior
- MATRIX_GROWTH_CANDIDATES:
  - Combo: Shared base envelope plus per-record bounded summary | Sources: Atlassian Jira Issue Fields docs, FocusLLM paper | WhatToSteal: typed authority fields plus summary-first consumption | HandshakeCarryOver: every canonical collaboration record emits a compact summary with stable ids, blockers, next action, and authority refs | RuntimeConsequences: local-small-model routing can default to summary artifacts before loading full packet detail or long note streams | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is one of the highest-leverage gains in the spec and should land in the first implementation pass
  - Combo: Canonical records plus view-specific projection files | Sources: GitHub Projects roadmap layout docs, Atlassian Jira Issue Fields docs | WhatToSteal: one record family rendered into multiple governed layouts | HandshakeCarryOver: Task Board index and per-view projection files should derive from the same canonical collaboration records without mutating authority | RuntimeConsequences: future Command Center and queue surfaces can switch layouts without forking state or scraping Markdown | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the direct backend precondition for the downstream structured viewer stub
  - Combo: Base descriptor plus bounded extension surface | Sources: Backstage descriptor format docs | WhatToSteal: stable core schema with optional extension payloads | HandshakeCarryOver: keep `project_profile_kind` and `profile_extension` explicit so future kernels reuse the artifact family safely | RuntimeConsequences: parsers, validators, and local models can operate on the shared envelope even when an extension schema is unknown | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: this supports future project-kernel portability without requiring schema-registry work in the same packet
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep record identity, summary identity, and authority refs mechanically joinable so readers never reconstruct state from transcript order or Markdown prose.
  - Separate projection layout logic from canonical record mutation so board or queue re-layouts stay cheap and non-authoritative.
  - Bound summary payloads deliberately for local-small-model first reads and only hydrate longer detail when a workflow step truly requires it.
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: Locus canonical packet emission plus durable summary pairing | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: makes work-packet state readable by both humans and models without opening Markdown first
  - Combo: Locus canonical micro-task emission plus bounded execution summary | Pillars: Locus, MicroTask | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: keeps micro-task execution state compact and queryable across runtime and later viewer surfaces
  - Combo: Task Board projection files over canonical records | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-TaskBoardEntry, PRIM-MarkdownMirrorContractV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: enables multiple board layouts without turning lane position into authority
  - Combo: Portable workflow-state fields on canonical work-packet records | Pillars: Locus, LLM-friendly data | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: gives small models and operators explicit routing fields instead of relying on prose or board placement
  - Combo: Portable workflow-state fields on canonical micro-task records | Pillars: MicroTask, LLM-friendly data | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: keeps execution-ready versus waiting posture explicit at the micro-task level
  - Combo: Mirror-state plus authority refs on packet and task summaries | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-MirrorSyncState, PRIM-MarkdownMirrorContractV1, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: summary-first reads stay safe only if mirror drift and authority posture are explicit
  - Combo: Mailbox export convergence with shared collaboration envelope | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1, PRIM-MirrorSyncState | Resolution: IN_THIS_WP | Stub: NONE | Notes: keeps mailbox exports compatible with the same generic readers and validators as other collaboration artifacts
  - Combo: One parser surface across work packets, micro-tasks, and task-board rows | Pillars: Locus, MicroTask, LLM-friendly data | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: a shared base envelope is only valuable if all record families remain field-equivalent at the parser boundary
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: The highest-ROI combinations for the touched pillar and mechanical-engine surface area are all direct implementation responsibilities of this WP and do not require new stubs or a spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: related structured-collaboration stubs; validated Role Mailbox and Locus packets; current backend runtime structs, task-board sync code, role-mailbox export code, and runtime governance path helpers
- MATCHED_STUBS:
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: schema registry is a downstream validator and compatibility surface; this WP owns canonical file-family implementation first
  - Artifact: WP-1-Markdown-Mirror-Sync-Drift-Guard-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: mirror reconciliation behavior is a downstream controller concern; this WP only needs the canonical artifact family and mirror metadata fields in place
  - Artifact: WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: viewer work consumes the artifact family but should not redefine backend record law
  - Artifact: WP-1-Project-Profile-Extension-Registry-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: N/A | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: extension registry work follows after the shared artifact family exists in code
  - Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: portable workflow-state vocabulary is already in the spec and becomes a downstream registry/hardening pass after the artifact family starts emitting those fields
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Role-Mailbox-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: mailbox export plumbing already exists, but it predates the shared structured-collaboration envelope and summary contract required by the newer spec additions
  - Artifact: WP-1-Locus-Phase1-Integration-Occupancy-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Locus already tracks work-packet and micro-task state, but it does not yet emit the full canonical artifact family required here
- CODE_REALITY_EVIDENCE:
  - Path: src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Locus-Phase1-Integration-Occupancy-v1 | Covers: primitive | Verdict: PARTIAL | Notes: defines `TrackedWorkPacket` and `TrackedMicroTask`, but omits the shared `schema_id`, `record_kind`, `project_profile_kind`, `mirror_state`, `authority_refs`, `evidence_refs`, and summary companion behavior required by the current spec
  - Path: src/backend/handshake_core/src/locus/task_board.rs | Artifact: WP-1-Locus-Phase1-Integration-Occupancy-v1 | Covers: execution | Verdict: PARTIAL | Notes: rewrites Markdown Task Board sections only; there is no canonical `task_board/index.json` or per-view projection file family yet
  - Path: src/backend/handshake_core/src/role_mailbox.rs | Artifact: WP-1-Role-Mailbox-v1 | Covers: execution | Verdict: PARTIAL | Notes: exports `index.json` and thread JSONL with `role_mailbox_export_v1`, but the current payload does not yet converge fully on the shared collaboration envelope and compact summary-first contract
  - Path: src/backend/handshake_core/src/runtime_governance.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: path helpers currently expose only `TASK_BOARD.md`, `SPEC_CURRENT.md`, and `ROLE_MAILBOX/` roots; work-packet and micro-task artifact-family roots are not modeled yet
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The gap is real and implementation-oriented. Related stubs and validated packets cover adjacent or prerequisite concerns, but none already deliver the full canonical artifact family in code under the current spec.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This activation is backend artifact-family work. Direct UI and operator-surface implications are intentionally deferred to the downstream structured viewer and layout projection packets.
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
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this WP; downstream viewer work owns UI interaction and accessibility specifics.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.167]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Locus-Phase1-Integration-Occupancy, WP-1-Role-Mailbox, WP-1-Micro-Task-Executor
- BUILD_ORDER_BLOCKS: WP-1-Markdown-Mirror-Sync-Drift-Guard, WP-1-Dev-Command-Center-Structured-Artifact-Viewer
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md 2.3.15.5 Canonical structured collaboration artifact family [ADD v02.167]
- WHAT: Implement the canonical structured collaboration artifact family for Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports using the v02.178 base envelope, bounded summary contract, and mirror-governance fields.
- WHY: Current spec coverage is already strong, but the runtime still relies on partial structs, Markdown-only board sync, and mailbox-specific exports. This WP converts that gap into one reusable backend record family that later schema-registry, mirror-sync, and viewer work can build on safely.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
- OUT_OF_SCOPE:
  - frontend Dev Command Center viewer and layout work
  - schema-registry and project-profile extension registry hardening beyond the runtime fields needed for this implementation
  - standalone mirror-reconciliation controllers and overwrite-safe normalization policy
  - non-software project-profile packs beyond the shared base envelope and extension boundary
- TEST_PLAN:
  ```bash
  cargo test -p handshake_core
  just gov-check
  ```
- DONE_MEANS:
  - Work Packet, Micro-Task, Task Board, and Role Mailbox runtime artifacts are emitted in a canonical structured family aligned to the v02.178 base envelope.
  - Each canonical collaboration artifact family member exposes a bounded summary path or summary payload that smaller local models can consume first.
  - Runtime artifact paths and serialization stay deterministic and preserve mirror-state plus authoritative-reference semantics.
  - Existing mailbox leak-safety and current Locus/Task Board behavior do not regress while the new artifact family is added.
- PRIMITIVES_EXPOSED:
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-MirrorSyncState
  - PRIM-MarkdownMirrorContractV1
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - TrackedWorkPacket
  - TrackedMicroTask
  - role_mailbox_export_v1
  - summary.json
  - packet.json
  - workflow_state_family
  - queue_reason_code
  - mirror_state
- RUN_COMMANDS:
  ```bash
  rg -n "TrackedWorkPacket|TrackedMicroTask|role_mailbox_export_v1|workflow_state_family|queue_reason_code|mirror_state|summary.json|packet.json" src/backend/handshake_core
  cargo test -p handshake_core
  just gov-check
  ```
- RISK_MAP:
  - "base-envelope drift between record families" -> "shared readers and validators become unreliable"
  - "summary artifacts diverge from canonical detail" -> "local-small-model routing and operator triage become unsafe"
  - "mailbox export convergence regresses leak-safe behavior" -> "governance-critical data could be exposed incorrectly"
  - "task-board projection generation stays Markdown-only" -> "downstream viewer and layout packets remain blocked"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - Current stub metadata and BUILD_ORDER ranking already match this activation target. No pre-approval build-order edit is needed unless refinement scope changes.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Master Spec explicitly names the canonical structured collaboration artifact family, the shared base envelope, compact summaries, mirror-state governance, task-board and mailbox projections, and portable workflow-state fields. This WP is therefore clearly specified and executable without further spec enrichment.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.178 already defines the collaboration artifact family, summary contract, mirror semantics, and workflow-state vocabulary needed for this WP. The missing work is runtime implementation and alignment, not new normative spec text.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.15.5 Canonical structured collaboration artifact family [ADD v02.167]
- CONTEXT_START_LINE: 6816
- CONTEXT_END_LINE: 6837
- CONTEXT_TOKEN: Canonical structured collaboration artifact family
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Canonical structured collaboration artifact family** [ADD v02.167]

  - The canonical file standard for Work Packets, Micro-Tasks, and Task Board projections SHALL be versioned JavaScript Object Notation documents. Role Mailbox thread bodies MAY use JavaScript Object Notation Lines where append-only streaming is materially simpler than rewriting a full array document.
  - Recommended portable Phase 1 layout:
    - `.handshake/gov/work_packets/{wp_id}/packet.json`
    - `.handshake/gov/work_packets/{wp_id}/summary.json`
    - `.handshake/gov/work_packets/{wp_id}/notes/{note_id}.md`
    - `.handshake/gov/micro_tasks/{wp_id}/{mt_id}/packet.json`
    - `.handshake/gov/micro_tasks/{wp_id}/{mt_id}/summary.json`
    - `.handshake/gov/task_board/index.json`
    - `.handshake/gov/task_board/views/{view_id}.json`
    - `.handshake/gov/role_mailbox/index.json`
    - `.handshake/gov/role_mailbox/threads/{thread_id}.jsonl`
  - Every canonical structured collaboration record SHOULD expose a compact, bounded summary alongside any longer note stream. At minimum the summary SHOULD surface identity, title or objective, current status, blockers, next action, and relevant stable references so local small models do not need to ingest long Markdown narratives on every turn.
  - Every canonical structured collaboration record MUST expose:
    - a schema identifier and schema version
    - a stable record identifier
    - an updated timestamp
    - a profile kind such as `software_delivery`, `research`, `worldbuilding`, `design`, `generic`, or `custom`
    - references to note sidecars, mirrors, or evidence artifacts when present
  - Project-specific details such as repository branch policy, coding-path scope, or design-review metadata MUST live inside profile extensions instead of becoming mandatory base-envelope fields for every Handshake project type.
  - Future board surfaces, including kanban or Jira-like views, SHALL be projections over these structured records. Board layout, swimlanes, grouping, and sorting MAY change without changing the authoritative work state until a governed structured-record edit is committed.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.15.5 Base structured schema and project-profile extension contract [ADD v02.168]
- CONTEXT_START_LINE: 6839
- CONTEXT_END_LINE: 6882
- CONTEXT_TOKEN: Base structured schema and project-profile extension contract
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

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.15.5 Canonical-to-mirror synchronization and drift governance [ADD v02.169]
- CONTEXT_START_LINE: 6884
- CONTEXT_END_LINE: 6907
- CONTEXT_TOKEN: Canonical-to-mirror synchronization and drift governance
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Canonical-to-mirror synchronization and drift governance** [ADD v02.169]

  - Canonical JavaScript Object Notation or JavaScript Object Notation Lines records remain the only executable authority for workflow routing, validation, and readiness state. Markdown mirrors are readable projections and MUST reconcile against canonical records rather than acting as peer authorities.
  - Collaboration artifact families SHOULD expose `mirror_contract` metadata so validators, Dev Command Center, and local-small-model ingestion can see:
    - `authority_mode`
    - `markdown_mirror_path`
    - `template_id`
    - `canonical_content_hash`
    - `mirror_content_hash`
    - `last_reconciled_at`
    - `manual_edit_zones`
    - `reconciliation_action`
  - `authority_mode` SHALL be interpreted as:
    - `derived_readonly`: generated Markdown is never authoritative and SHOULD be regenerated directly from canonical state when stale.
    - `advisory_editable`: operators MAY write bounded human edits, but those edits remain advisory until normalized back into canonical structured fields or note sidecars.
    - `notes_sidecar_only`: long-form human narrative SHOULD live in append-only sidecars and generated mirrors SHOULD stay minimal.
  - `reconciliation_action` SHALL be interpreted as:
    - `none`: mirror and canonical state are aligned enough that no repair is pending.
    - `regenerate_mirror`: canonical state changed and the readable mirror should be rebuilt deterministically.
    - `promote_advisory_note`: an operator-authored advisory edit or note fragment is ready to be normalized into canonical data or a durable sidecar.
    - `manual_resolution_required`: drift cannot be repaired safely without an explicit operator decision.
  - Automatic regeneration MAY occur only when the current mirror is purely derived or stale. Automatic regeneration MUST NOT silently overwrite advisory operator content or append-only note streams.
  - If the canonical record moves, disappears, or changes template semantics in a way that invalidates the current Markdown path, the mirror SHALL move to `normalization_required` until reconciliation records the new linkage explicitly.
  - Compact summaries remain the first-read path for local small models. Local small models SHOULD ignore stale or advisory Markdown mirrors unless the operator explicitly asks for readable narrative context.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.15.5 Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- CONTEXT_START_LINE: 6928
- CONTEXT_END_LINE: 6979
- CONTEXT_TOKEN: workflow_state_family
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
    - `new_untriaged`
    - `dependency_wait`
    - `ready_for_local_small_model`
    - `ready_for_cloud_model`
    - `ready_for_human`
    - `review_wait`
    - `approval_wait`
    - `validation_wait`
    - `mailbox_response_wait`
    - `timer_wait`
    - `blocked_missing_context`
    - `blocked_policy`
    - `blocked_capability`
    - `blocked_error`
  - Board position, queue order, and mailbox thread order MUST NOT become substitutes for `workflow_state_family` or `queue_reason_code`.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.6.8.10 Role Mailbox repo export format and base-envelope preservation [ADD v02.168] [ADD v02.169]
- CONTEXT_START_LINE: 11014
- CONTEXT_END_LINE: 11113
- CONTEXT_TOKEN: role_mailbox_export_v1
- EXCERPT_ASCII_ESCAPED:
  ```text
  Export format (normative):
  - `docs/ROLE_MAILBOX/index.json` (one JSON object; thread inventory)
  - `docs/ROLE_MAILBOX/threads/<thread_id>.jsonl` (one JSON object per line; messages)
  - `docs/ROLE_MAILBOX/export_manifest.json` (export metadata + hashes)

  Export schemas (normative; role_mailbox_export_v1):

  // docs/ROLE_MAILBOX/index.json
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

  // docs/ROLE_MAILBOX/export_manifest.json
  interface RoleMailboxExportManifestV1 {
    schema_id: 'hsk.role_mailbox_export_manifest@1';
    schema_version: 'role_mailbox_export_v1';
    record_id: 'role_mailbox_export_manifest';
    record_kind: 'generic';
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    updated_at: string; // RFC3339; equals generated_at for manifest snapshots
    mirror_contract?: MarkdownMirrorContractV1;
    export_root: 'docs/ROLE_MAILBOX/';
    generated_at: string; // RFC3339
    index_sha256: string;
    thread_files: Array<{
      path: string;   // "threads/<thread_id>.jsonl"
      sha256: string; // sha256 of file bytes
      message_count: number;
    }>;
  }

  - [ADD v02.168] Role Mailbox export schemas MUST preserve the base structured-collaboration envelope fields even when the concrete export file is mailbox-specific. Unknown project-profile fields MAY appear only inside profile extensions or context sub-objects that preserve backward-compatible parsing of the base envelope.
  - [ADD v02.169] Mailbox exports SHOULD also preserve mirror-contract metadata whenever a readable Markdown or note-sidecar representation exists, so generic reconciliation tooling can distinguish derived exports from advisory human narrative.
  ```
