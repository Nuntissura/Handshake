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
- WP_ID: WP-1-Structured-Collaboration-Schema-Registry-v3
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-19T05:55:23Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: 608a586c4afa78f4f625d5cd381a9d3b4fb3e4d9
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja190320260923
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Structured-Collaboration-Schema-Registry-v3
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- v2 passed validator but on direct code inspection against master spec v02.178, the following concrete gaps remain on current `main`:
- **Missing structured diagnostic output for schema version mismatches**: The spec requires that unknown or incompatible schema/profile-extension versions produce deterministic machine-readable validation results instead of silent fallback. Current `validate_structured_collaboration_record()` in `types.rs` returns `Result<()>` with string error messages, not structured diagnostic payloads consumable by Command Center or downstream viewers.
- **Missing runtime integration proof for `validate_summary_detail_pair`**: The spec requires that summary and detail records share the same `record_id`, `project_profile_kind`, and authoritative references so deterministic joins remain possible. No test proves this validation runs on actual artifact emission paths in `workflows.rs` or `runtime_governance.rs`.
- **`TaskBoardEntryRecordV1` missing spec-required fields**: Spec section [ADD v02.168] requires task-board projection rows to include `task_board_id`, `lane_id`, `display_order`, and optional `view_ids` as board-specific fields. Current `TaskBoardEntryRecordV1` in `task_board.rs` lacks these fields.
- **`updated_at` field missing from base envelope enforcement**: Spec [ADD v02.168] requires `updated_at` in the base envelope minimum field set. Current `ensure_schema_registry_fields_work_packet` and `ensure_schema_registry_fields_micro_task` functions verify many envelope fields but do not check or enforce `updated_at`.
- **`RoleMailboxThreadLineV1` field completeness**: Spec requires `RoleMailboxThreadLineV1` to include base envelope fields plus mailbox-specific fields including `transcription_links` array. Current `role_mailbox.rs` emit path needs verification that all spec-required thread line fields are present and correctly typed.
- The v2 remediation pass was judged underperformed by operator code inspection. This v3 pass must close the above gaps with implementation and parity proof, not just audit narrative.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 90m
- SEARCH_SCOPE: official docs for typed work-item fields and projection-over-record layouts; OSS descriptor/schema-extension patterns; compact-context research for summary-first ingestion; local code reality in `src/backend/handshake_core`
- REFERENCES: Atlassian Jira issue fields docs; GitHub Projects fields docs; Backstage descriptor format docs; Backstage repository; FocusLLM paper; current Handshake spec and backend runtime files
- PATTERNS_EXTRACTED: typed canonical records separate from view layouts; stable low-cardinality base envelope plus bounded extension payloads; summary-first loading before full detail hydration; explicit version ids and compatibility policy at the parser boundary; machine-readable validation results rather than silent fallback
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT field-authoritative records with multiple derived views; ADAPT descriptor-plus-extension patterns into strict Handshake `profile_extension` compatibility checks; REJECT layout state, Markdown mirrors, or mailbox transcript order as peer schema authority
- LICENSE/IP_NOTES: Reference-only research and repository inspection. No third-party code or schema text is intended for direct copy into product code.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.178.md already names the base structured-collaboration envelope, compact summary contract, project-profile extension boundary, and Role Mailbox export field set. This WP is an implementation and compatibility-hardening pass against the current Main Body.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON_NO: N/A
- SOURCE_MAX_AGE_DAYS: 730
- SOURCE_LOG:
  - Source: Atlassian Jira Issue Fields docs | Kind: BIG_TECH | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-fields/ | Why: shows typed field authority reused by multiple issue and board views
  - Source: GitHub Projects fields docs | Kind: BIG_TECH | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://docs.github.com/en/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: shows stable project-item fields driving multiple projections and layouts
  - Source: Backstage descriptor format docs | Kind: OSS_DOC | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://backstage.io/docs/features/software-catalog/descriptor-format/ | Why: useful reference for a shared core envelope with bounded extension metadata
  - Source: Backstage repository | Kind: GITHUB | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://github.com/backstage/backstage | Why: concrete repository-scale example of descriptor-backed projections and extensibility pressure
  - Source: FocusLLM paper | Kind: PAPER | Date: 2024-08-21 | Retrieved: 2026-03-19T05:55:23Z | URL: https://arxiv.org/abs/2408.11745 | Why: supports compact-summary-first loading for smaller local models before detail hydration
- RESEARCH_SYNTHESIS:
  - Handshake should keep one field-authoritative collaboration record family and let board, queue, mailbox, and viewer surfaces remain projections over that family.
  - The shared envelope should stay intentionally small and stable while project-specific payloads move behind explicit extension schemas and compatibility checks.
  - Summary artifacts should be first-read surfaces for smaller local models and operator triage, with canonical detail loaded only when required.
  - Strong registry behavior is not just about naming schema ids; it also needs deterministic incompatibility reporting so future kernels do not guess across unknown profile extensions.
  - Structured validation diagnostics (not just string errors) are what make downstream Command Center and viewer surfaces actionable.
- RESEARCH_GAPS_TO_TRACK:
  - NONE
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: Atlassian Jira Issue Fields docs | Pattern: typed field authority survives multiple presentation layouts | Why: directly matches the need for one canonical parser contract across packet, summary, task-board, and mailbox records
  - Source: GitHub Projects fields docs | Pattern: one stable item model backing board, table, and roadmap views | Why: reinforces that Task Board and Command Center projections must not become competing schema authorities
- ADAPT_PATTERNS:
  - Source: Backstage descriptor format docs | Pattern: shared core descriptor plus bounded extension metadata | Why: Handshake needs this split, but with stricter compatibility enforcement, mirror semantics, and explicit authority refs than a general software catalog
  - Source: FocusLLM paper | Pattern: compact representation first, hydrate detail only when needed | Why: the paper is about context compression, but the same technique maps well to `summary.json` contract enforcement for local-small-model planning
- REJECT_PATTERNS:
  - Source: Backstage repository | Pattern: broad plugin-first metadata sprawl as the starting point | Why: this WP needs a small deterministic registry and compatibility boundary, not a large extension ecosystem that weakens parser guarantees
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - structured record schema registry projection repo
  - descriptor extension compatibility validation repo
- MATCHED_PROJECTS:
  - Source: Backstage repository | Repo: backstage/backstage | URL: https://github.com/backstage/backstage | Intent: ARCH_PATTERN | Decision: ADAPT | Impact: NONE | Stub: NONE | Notes: useful as a reference for stable descriptor projection, but Handshake should stay stricter on schema version compatibility and not copy plugin-surface breadth
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Reuse existing structured collaboration event families; do not invent a new Flight Recorder namespace for registry activation alone.
- Work Packet and Micro-Task artifact emission should continue to flow through existing Locus and Work Packet event families.
- Task Board projection publication should continue to reuse existing Task Board event families.
- Role Mailbox export flow should keep using existing mailbox export and transcription linkage events.
- Registry validation failures should surface as deterministic structured validation outputs consumed by runtime and viewer layers, not as ad hoc new event ids.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: unknown schema versions silently degrade and are interpreted as valid. Mitigation: explicit compatibility policy and machine-readable structured mismatch results.
- Risk: `profile_extension` payloads smuggle software-delivery-only required fields into the shared envelope. Mitigation: reject unknown breaking extensions and keep base-envelope validation separate from extension validation.
- Risk: summary records drift from canonical detail and become the de facto authority. Mitigation: validate shared identity, authority refs, and summary linkage on every paired record family.
- Risk: governance-side mailbox exports and product-runtime mailbox artifacts are conflated. Mitigation: scope validation to the packet-declared product artifact family and keep repo control-plane schemas separate.
- Risk: TaskBoardEntryRecordV1 missing fields cause silent data loss on board projections. Mitigation: add spec-required fields and enforce them in validation.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-MirrorSyncState
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-MirrorSyncState
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
  - The spec already owns the primitive ids that matter here. The v3 gap is structured diagnostic output, runtime integration proof for summary/detail validation, missing TaskBoardEntryRecordV1 fields, updated_at enforcement, and RoleMailboxThreadLineV1 completeness.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The shared collaboration-envelope, summary, profile-extension, mirror-state, and mailbox primitive ids already exist in the current spec appendix.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Implementation should align runtime schema registration and validator outputs to the existing primitive set instead of introducing new primitive ids.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new high-signal orphan primitive ids were discovered; the appendix already names the relevant collaboration primitives.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Existing appendix ownership notes already cover the collaboration envelope and downstream registry/viewer split.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet is backend registry and validator work. Direct viewer behavior remains downstream viewer and triage-surface work.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Current appendix interaction coverage is sufficient; activation does not require new IMX edges.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep Appendix 12 unchanged and implement against the current v02.178 collaboration primitive and ownership map.
  - If implementation reveals a truly missing appendix edge or primitive, that should become a new spec-update flow rather than silent packet drift.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene contract is changed by schema registry activation | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected here | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physical simulation or measurement law is involved in this packet | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers, not registry authors, in this scope | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing capability is affected by collaboration schema validation | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration may consume registry outputs later, but no director contract is changed here | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition or sequencing surface is in scope | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering or generation surface is touched | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication surfaces may consume validated records later, but not in this packet | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking or recipe workflow surface is affected | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is affected by collaboration schema registration | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no routing or delivery engine behavior is changed in this packet | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: this WP governs how durable collaboration artifacts identify themselves and validate across packet, summary, board, and mailbox families | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval layers consume these records later, but this packet stops at schema authority and validation | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analysis surfaces remain downstream consumers of machine-readable validation outputs | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset ingestion or wrangling contract is changed directly | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: storage portability matters downstream, but this packet does not alter backend SQL behavior directly | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: the packet implements already-governed schema law and does not add a new governance authority surface | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or guidance interface is added here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: compact-summary validation is a context-compaction boundary for local-small-model routing and triage; structured diagnostics improve context quality | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: schema ids, schema versions, compatibility readers, extension version policy, and updated_at enforcement are first-class versioning concerns in this packet | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required to activate the schema registry | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: existing event families remain intact; registry validation does not require new FR ids | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: calendar storage and policy surfaces are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: editor surfaces are downstream consumers only | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: document editing is not changed by collaboration schema registration | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: spreadsheet surfaces are out of scope | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus owns core tracked work records and task-board projections that need one registry and compatibility reader path | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Loom storage portability is a separate backend lane and should stay file-disjoint from this packet | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: product packet-detail surfaces remain downstream even though this packet hardens the shared registry underneath them | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: product board UX is downstream; this packet only adds missing struct fields to the shared registry validation surface | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: Micro-Task packet and summary artifacts are direct registry subjects | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: viewer and triage implementation remains downstream even though this packet defines the machine-readable registry outputs those surfaces will consume | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: memory-system behavior is not changed directly by this backend packet | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: NOT_TOUCHED | NOTES: runtime job execution is only indirectly affected through clearer artifact validation boundaries | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no Spec Router or prompt-compilation contract is expanded here | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: this packet is backend-neutral but does not change database trait or migration behavior directly | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: summary-first local-model ingestion depends on a stable compact-summary contract, updated_at enforcement, and structured validation diagnostics | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Stage artifact contracts are unrelated to collaboration schema registration | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Studio runtime behavior is out of scope | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: UI/viewer follow-on work remains downstream of this registry packet | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: distillation workflows are unaffected by this packet | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE runtime protocol or validator shape is changed directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval systems may consume summaries later, but no RAG contract is changed directly in this packet | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: Shared collaboration base-envelope validation with updated_at | SUBFEATURES: Work Packet, Micro-Task, Task Board record identity and compatibility checks; updated_at enforcement in base envelope | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the registry must validate one field-equivalent envelope including updated_at across the main Locus-owned record families
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Canonical packet and summary schema registration with structured diagnostics | SUBFEATURES: `packet.json`, `summary.json`, compatibility readers, profile-extension enforcement, structured mismatch output | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: packet and summary validation must share ids, authorities, and extension policy; mismatch output must be structured, not string-only
  - PILLAR: MicroTask | CAPABILITY_SLICE: Canonical micro-task packet and summary schema registration | SUBFEATURES: `packet.json`, `summary.json`, compatibility readers, machine-readable mismatch results | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task routing and bounded execution depend on the same registry guarantees as work packets
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Structured projection schema registration with missing fields | SUBFEATURES: `index.json`, `views/{view_id}.json`, row validation, task_board_id/lane_id/display_order/view_ids fields, shared summary joins | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1, PRIM-MirrorSyncState | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: TaskBoardEntryRecordV1 must gain spec-required board-specific fields
  - PILLAR: Command Center | CAPABILITY_SLICE: Registry-driven structured validation diagnostics | SUBFEATURES: unknown-schema, incompatible-extension, updated_at-missing, and summary-drift outputs as structured payloads consumable by generic viewers | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-StructuredCollaborationSummaryV1, FEAT-DEV-COMMAND-CENTER | MECHANICAL: engine.context, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the packet should emit deterministic structured validator outputs that the viewer packet can consume later
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact-summary-first compatibility enforcement with updated_at | SUBFEATURES: shared identity and authority refs across detail and summary records; updated_at as base envelope requirement | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationSummaryV1, PRIM-MirrorSyncState, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: small-model reads must not guess across mismatched summaries or unknown extensions; updated_at enables freshness-based routing
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Shared collaboration schema registry with structured diagnostics | JobModel: WORKFLOW | Workflow: locus_structured_artifact_publish | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: registry outputs should be visible as structured diagnostic payloads to both generic viewers and runtime artifact producers
  - Capability: Compact summary compatibility validation | JobModel: WORKFLOW | Workflow: compact_summary_emit | ToolSurface: COMMAND_CENTER | ModelExposure: LOCAL | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: summary-first local-model routing depends on deterministic summary/detail compatibility checks with runtime integration proof
  - Capability: Task Board structured projection validation with missing fields | JobModel: WORKFLOW | Workflow: task_board_projection_publish | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: board projections need spec-required fields (task_board_id, lane_id, display_order, view_ids) and structured validation diagnostics
  - Capability: Role Mailbox export schema validation | JobModel: WORKFLOW | Workflow: role_mailbox_export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox export validation must verify RoleMailboxThreadLineV1 field completeness including transcription_links
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Scanned for high-ROI appendix matrix additions and found that current v02.178 already records the key collaboration-envelope ownership and downstream viewer split for this packet.
  - The main activation need is structured diagnostics, missing fields, updated_at enforcement, and runtime integration proof, not a new appendix interaction edge.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Current appendix interaction coverage already captures the relevant collaboration-envelope and viewer relationships; this activation does not require a new IMX edge before coding can start.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON_NO: N/A
- SOURCE_SCAN:
  - Source: Atlassian Jira Issue Fields docs | Kind: BIG_TECH | Angle: typed issue fields feeding multiple issue and board layouts | Pattern: stable field authority separate from derived view configuration | Decision: ADOPT | EngineeringTrick: keep canonical field validation independent from layout rendering so board changes never mutate schema meaning | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly supports shared validation across packets, board rows, and mailbox summaries
  - Source: GitHub Projects fields docs | Kind: BIG_TECH | Angle: one item model backing board, table, and roadmap views | Pattern: multiple projections over a common record set | Decision: ADOPT | EngineeringTrick: validate record shape once and reuse the same parsed fields across all projections | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: reinforces that Task Board and Command Center views should not define schema law
  - Source: Backstage descriptor format docs | Kind: OSS_DOC | Angle: shared descriptor plus bounded extension metadata | Pattern: stable envelope with domain-specific extension hooks | Decision: ADAPT | EngineeringTrick: make extension schema id/version compatibility explicit so parsers fail deterministically with structured diagnostics instead of loosely passing unknown shapes | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: maps well to Handshake `profile_extension` policy with structured mismatch output
  - Source: Backstage repository | Kind: GITHUB | Angle: repository-scale extensibility pressure on shared descriptors | Pattern: plugin growth around a stable core record | Decision: REJECT | EngineeringTrick: resist expanding the collaboration registry into a broad plugin ecosystem during Phase 1 | ROI: MEDIUM | Resolution: REJECT_DUPLICATE | Stub: NONE | Notes: useful warning signal, not a scope expansion target
  - Source: FocusLLM paper | Kind: PAPER | Angle: compact context before full-detail retrieval | Pattern: bounded representation first, hydrate detail later | Decision: ADAPT | EngineeringTrick: validate summary/detail joins explicitly with updated_at freshness so summary-first loading is safe for local-small-model flows | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: the paper is not a workflow system, but the compact-summary pattern is directly applicable
- MATRIX_GROWTH_CANDIDATES:
  - Combo: Shared base envelope plus compact summary pairing with structured diagnostics | Sources: Atlassian Jira Issue Fields docs, FocusLLM paper | WhatToSteal: stable typed authority plus summary-first consumption with machine-readable validation errors | HandshakeCarryOver: one registry validates both canonical detail and bounded summary records with structured diagnostic output instead of string errors | RuntimeConsequences: local-small-model planning and operator triage can load summary artifacts first and get actionable error payloads on mismatch | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the highest-leverage runtime payoff of the packet
  - Combo: Base descriptor plus bounded extension compatibility with updated_at | Sources: Backstage descriptor format docs | WhatToSteal: shared parser core with explicit extension boundaries and freshness tracking | HandshakeCarryOver: generic viewers can parse the shared envelope including updated_at while deterministic compatibility policy guards project-specific payloads | RuntimeConsequences: future kernels reuse the same record family without inheriting software-delivery-only mandatory fields | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: updated_at enforcement closes a specific gap found in v2 code inspection
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep schema id/version constants and compatibility policy in one registry surface instead of scattering them across emitters.
  - Validate summary/detail shared identity and authority refs mechanically before allowing summary-first reads.
  - Separate base-envelope validation from extension validation so unknown extensions never force parser forks or silent fallback.
  - Return structured validation diagnostic payloads (not just Result<()> with strings) so downstream surfaces can act on specific failures.
  - Enforce updated_at in base envelope validation alongside schema_id, record_id, record_kind, project_profile_kind, mirror_state.
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: Locus tracked record families plus one shared registry with structured diagnostics | Pillars: Locus | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry | Resolution: IN_THIS_WP | Stub: NONE | Notes: field-equivalent validation with structured output is the direct backend unlocker for later viewer and profile-extension work
  - Combo: Work Packet detail plus compact summary join validation with updated_at | Pillars: Locus, LLM-friendly data | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: keeps summary-first reads safe and deterministic with freshness tracking
  - Combo: Micro-Task detail plus compact summary join validation | Pillars: MicroTask, LLM-friendly data | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: gives bounded execution and retry flows one consistent parser boundary
  - Combo: Task Board row, index, and view validation with missing spec fields | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1, PRIM-MirrorSyncState | Resolution: IN_THIS_WP | Stub: NONE | Notes: TaskBoardEntryRecordV1 must gain task_board_id, lane_id, display_order, view_ids to match spec; this is shared registry work not product board UX
  - Combo: Command Center diagnostics over structured registry mismatch results | Pillars: LLM-friendly data, Locus | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1, PRIM-ProjectProfileExtensionV1, FEAT-DEV-COMMAND-CENTER | Resolution: IN_THIS_WP | Stub: NONE | Notes: viewer work downstream depends on structured registry output instead of best-effort field guesses
  - Combo: Role Mailbox export validation with complete ThreadLineV1 fields | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1, PRIM-StructuredCollaborationEnvelopeV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: the packet should verify all spec-required fields including transcription_links
  - Combo: Schema-version mismatch structured diagnostics at the parser boundary | Pillars: Locus, LLM-friendly data | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1, PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: readers need structured mismatch payloads instead of string error messages when schema versions drift
  - Combo: Profile-extension compatibility gating over canonical records | Pillars: Locus, MicroTask | Mechanical: engine.version | Primitives/Features: PRIM-ProjectProfileExtensionV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask | Resolution: IN_THIS_WP | Stub: NONE | Notes: extension schema ids and compatibility posture should be enforced before record-specific logic runs
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations identified for the touched collaboration surfaces are direct responsibilities of this packet and do not require new stubs or a spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: structured-collaboration stubs and validated packets; current Locus, runtime-governance, Task Board, mailbox export, and workflow artifact-emission code; governance validation scripts that might be confused with product runtime authority
- MATCHED_STUBS:
  - Artifact: WP-1-Project-Profile-Extension-Registry-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: extension registry work is downstream of the shared schema registry and should not be folded into this packet
  - Artifact: WP-1-Markdown-Mirror-Sync-Drift-Guard-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: mirror reconciliation is a controller concern after registry and compatibility policy exist
  - Artifact: WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: viewer work depends on registry outputs and should not define schema authority itself
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: the artifact-family packet materialized canonical records, but it intentionally left registry and compatibility hardening as a separate follow-on
  - Artifact: WP-1-Role-Mailbox-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: mailbox export plumbing exists, but generic schema registration and compatibility law are still missing
  - Artifact: WP-1-Locus-Phase1-Integration-Occupancy-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Locus tracking exists, but the packet did not centralize schema registration or validator behavior across all record families
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | BoardStatus: SUPERSEDED | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: v2 passed validator but code inspection revealed missing structured diagnostics, TaskBoard fields, updated_at enforcement, and incomplete ThreadLine fields; v3 supersedes
- CODE_REALITY_EVIDENCE:
  - Path: src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | Covers: primitive | Verdict: PARTIAL | Notes: v2 landed validation functions but they return string errors not structured diagnostics; ensure_schema_registry_fields_* do not check updated_at
  - Path: src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Structured-Collaboration-Schema-Registry-v2 | Covers: primitive | Verdict: IMPLEMENTED | Notes: v2 delivered schema_id constants, validate_structured_collaboration_record, validate_profile_extension, ensure_schema_registry_fields_* functions, and StructuredCollaborationRecordFamily enum
  - Path: src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | Covers: primitive | Verdict: PARTIAL | Notes: record structs and enums exist; validate_structured_collaboration_record returns Result<()> with string errors, not structured diagnostics; ensure_schema_registry_fields_* do not check updated_at
  - Path: src/backend/handshake_core/src/locus/task_board.rs | Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | Covers: primitive | Verdict: PARTIAL | Notes: TaskBoardEntryRecordV1 exists but lacks spec-required fields: task_board_id, lane_id, display_order, view_ids
  - Path: src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | Covers: execution | Verdict: PARTIAL | Notes: schema ids are emitted through hardcoded constants; no runtime integration test proves validate_summary_detail_pair runs on actual emission paths
  - Path: src/backend/handshake_core/src/runtime_governance.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: runtime path authority exists for packet, summary, micro-task, and task-board artifact paths, but not a central validator contract with structured output
  - Path: src/backend/handshake_core/src/role_mailbox.rs | Artifact: WP-1-Role-Mailbox-v1 | Covers: execution | Verdict: PARTIAL | Notes: mailbox export includes schema fields, but RoleMailboxThreadLineV1 field completeness (especially transcription_links) needs verification
  - Path: src/backend/handshake_core/src/api/role_mailbox.rs | Artifact: WP-1-Role-Mailbox-v1 | Covers: execution | Verdict: PARTIAL | Notes: mailbox API export exists on local main, but it still inherits mailbox-local assumptions rather than one remediated shared artifact contract
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: v2 passed validator but direct code inspection against spec v02.178 revealed structured diagnostic gap, missing TaskBoard fields, updated_at enforcement gap, and incomplete ThreadLine fields. v3 expands scope to close these specific gaps.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This activation is backend registry and validator work. Viewer and triage controls remain downstream packets.
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
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet; downstream viewer work owns interaction and accessibility details.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.168]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: WP-1-Project-Profile-Extension-Registry, WP-1-Markdown-Mirror-Sync-Drift-Guard, WP-1-Dev-Command-Center-Structured-Artifact-Viewer
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md Base structured schema and project-profile extension contract [ADD v02.168]
- WHAT: Implement structured diagnostic output for schema validation, add missing TaskBoardEntryRecordV1 fields (task_board_id, lane_id, display_order, view_ids), enforce updated_at in base envelope, prove validate_summary_detail_pair runtime integration, preserve legacy-compatible micro-task progress payload shape at the locus storage boundary, and verify RoleMailboxThreadLineV1 field completeness.
- WHY: v2 passed validator but operator code inspection against spec v02.178 revealed string-only validation errors, missing board-projection fields, no updated_at enforcement, unproven summary/detail runtime integration, and incomplete ThreadLine fields. This v3 closes those concrete spec compliance gaps.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - Loom storage, search, source-anchor, and asset portability
  - frontend Command Center viewer implementation and layout UX
  - project-profile-specific extension payload design beyond compatibility hooks and validation boundaries
  - Markdown mirror reconciliation controllers and overwrite policy
  - governance-only `.GOV` mailbox ledgers or session-control schemas
- TEST_PLAN:
  ```bash
  cargo test -p handshake_core
  just gov-check
  ```
- DONE_MEANS:
  - `validate_structured_collaboration_record()` and related validation functions return structured diagnostic payloads (with field name, expected value, actual value, severity) instead of string-only errors.
  - `TaskBoardEntryRecordV1` includes `task_board_id`, `lane_id`, `display_order`, and `view_ids` fields matching spec [ADD v02.168].
  - `ensure_schema_registry_fields_work_packet` and `ensure_schema_registry_fields_micro_task` enforce `updated_at` as a required base envelope field.
  - Runtime integration test proves `validate_summary_detail_pair` runs on actual artifact emission paths in `workflows.rs`.
  - `RoleMailboxThreadLineV1` emit path includes all spec-required fields including `transcription_links` array.
  - The packet keeps product-runtime artifact authority distinct from governance-side `.GOV` control-plane ledgers and validators.
- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-MirrorSyncState
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
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
- SEARCH_TERMS:
  - schema_id
  - schema_version
  - project_profile_kind
  - mirror_state
  - authority_refs
  - evidence_refs
  - summary.json
  - profile_extension
  - role_mailbox_index
  - role_mailbox_thread_line
  - updated_at
  - task_board_id
  - lane_id
  - display_order
  - view_ids
  - transcription_links
  - validate_structured_collaboration_record
  - validate_summary_detail_pair
- RUN_COMMANDS:
  ```bash
  rg -n "schema_id|schema_version|project_profile_kind|mirror_state|updated_at|task_board_id|lane_id|display_order|view_ids|transcription_links|validate_structured_collaboration_record|validate_summary_detail_pair" src/backend/handshake_core
  cargo test -p handshake_core
  just gov-check
  ```
- RISK_MAP:
  - "structured diagnostics widen into a full error-framework redesign" -> "scope creep beyond the spec-required machine-readable mismatch output"
  - "TaskBoardEntryRecordV1 field additions break existing board emitters" -> "current task-board emission paths produce invalid records"
  - "updated_at enforcement fails existing records that lack the field" -> "migration or data-repair needed before validation can pass"
  - "summary/detail validation integration test is fragile" -> "false confidence in runtime coverage if test is too narrow"
  - "runtime and governance mailbox paths remain conflated" -> "the packet validates the wrong authority surface and hides real product regressions"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - Current stub metadata and BUILD_ORDER ranking already match this activation target. No build-order edit is required unless scope expands beyond the registry boundary.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.168] Base envelope MUST expose updated_at | WHY_IN_SCOPE: spec requires updated_at as minimum base envelope field; current ensure_schema_registry_fields_* functions do not check it | EXPECTED_CODE_SURFACES: locus/types.rs (ensure_schema_registry_fields_work_packet, ensure_schema_registry_fields_micro_task) | EXPECTED_TESTS: cargo test -p handshake_core schema_registry updated_at | RISK_IF_MISSED: base envelope validation accepts records without freshness tracking, breaking summary-first routing
  - CLAUSE: [ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids | WHY_IN_SCOPE: spec requires board-specific fields; current TaskBoardEntryRecordV1 lacks them | EXPECTED_CODE_SURFACES: locus/task_board.rs (TaskBoardEntryRecordV1), workflows.rs (board emission) | EXPECTED_TESTS: cargo test -p handshake_core task_board_entry | RISK_IF_MISSED: board projections are structurally incomplete vs spec and downstream viewers cannot sort or filter by lane/position
  - CLAUSE: [ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas | WHY_IN_SCOPE: spec requires structured mismatch output; current validation returns string errors | EXPECTED_CODE_SURFACES: locus/types.rs (validate_structured_collaboration_record, validate_profile_extension) | EXPECTED_TESTS: cargo test -p handshake_core validation_diagnostics | RISK_IF_MISSED: Command Center and downstream viewers cannot programmatically act on validation failures
  - CLAUSE: [ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs | WHY_IN_SCOPE: spec requires deterministic joins between summary and detail; no runtime integration test proves this | EXPECTED_CODE_SURFACES: workflows.rs (artifact emission), runtime_governance.rs | EXPECTED_TESTS: cargo test -p handshake_core summary_detail_integration | RISK_IF_MISSED: summary-first reads may consume mismatched records without detection
  - CLAUSE: RoleMailboxThreadLineV1 field completeness including transcription_links | WHY_IN_SCOPE: spec defines full ThreadLineV1 shape; current emit path needs verification | EXPECTED_CODE_SURFACES: role_mailbox.rs, api/role_mailbox.rs | EXPECTED_TESTS: cargo test -p handshake_core role_mailbox thread_line | RISK_IF_MISSED: mailbox exports silently drop fields required by spec

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: Structured validation diagnostic payload | PRODUCER: locus/types.rs validation functions | CONSUMER: Command Center viewer, runtime_governance.rs | SERIALIZER_TRANSPORT: serde JSON | VALIDATOR_READER: downstream viewer tests | TRIPWIRE_TESTS: cargo test -p handshake_core validation_diagnostics | DRIFT_RISK: diagnostic payload shape not stabilized; consumers expect different fields
  - CONTRACT: TaskBoardEntryRecordV1 struct | PRODUCER: workflows.rs (board emission) | CONSUMER: task_board.rs validators, Command Center viewer | SERIALIZER_TRANSPORT: serde JSON (packet.json / index.json) | VALIDATOR_READER: governance validator, storage conformance | TRIPWIRE_TESTS: cargo test -p handshake_core task_board_entry | DRIFT_RISK: field additions in struct not reflected in emission or validation
  - CONTRACT: TrackedWorkPacket base envelope | PRODUCER: workflows.rs | CONSUMER: locus/types.rs validators, summary emitter | SERIALIZER_TRANSPORT: serde JSON (packet.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core schema_registry | DRIFT_RISK: updated_at enforcement added but emission paths do not set the field
  - CONTRACT: TrackedMicroTask base envelope | PRODUCER: workflows.rs | CONSUMER: locus/types.rs validators, summary emitter | SERIALIZER_TRANSPORT: serde JSON (packet.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core micro_task schema_registry | DRIFT_RISK: same as TrackedWorkPacket
  - CONTRACT: StructuredCollaborationSummaryV1 | PRODUCER: workflows.rs (summary.json emission) | CONSUMER: compact_summary validator, local-model routing | SERIALIZER_TRANSPORT: serde JSON (summary.json) | VALIDATOR_READER: validate_summary_detail_pair | TRIPWIRE_TESTS: cargo test -p handshake_core summary_detail_integration | DRIFT_RISK: summary emitter silently drops fields that detail includes
  - CONTRACT: RoleMailboxThreadLineV1 | PRODUCER: role_mailbox.rs | CONSUMER: api/role_mailbox.rs, governance export | SERIALIZER_TRANSPORT: JSONL (threads/<id>.jsonl) | VALIDATOR_READER: role_mailbox_tests | TRIPWIRE_TESTS: cargo test -p handshake_core role_mailbox thread_line | DRIFT_RISK: transcription_links array omitted or typed incorrectly

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test -p handshake_core schema_registry_updated_at -- proves updated_at is enforced in base envelope validation
  - cargo test -p handshake_core task_board_entry_spec_fields -- proves TaskBoardEntryRecordV1 includes task_board_id, lane_id, display_order, view_ids
  - cargo test -p handshake_core validation_diagnostics_structured -- proves validation returns structured payloads, not string errors
  - cargo test -p handshake_core summary_detail_integration -- proves validate_summary_detail_pair runs on actual emission paths
  - cargo test -p handshake_core role_mailbox_thread_line_completeness -- proves ThreadLineV1 includes all spec-required fields
- CANONICAL_CONTRACT_EXAMPLES:
  - Golden structured diagnostic payload JSON for schema version mismatch
  - Golden TaskBoardEntryRecordV1 JSON with all spec-required fields
  - Golden TrackedWorkPacket JSON with updated_at in base envelope
  - Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Define structured validation diagnostic type (e.g., SchemaValidationDiagnostic with field, expected, actual, severity) in locus/types.rs
  - Refactor validate_structured_collaboration_record and validate_profile_extension to return Vec<SchemaValidationDiagnostic> instead of Result<()>
  - Add updated_at check to ensure_schema_registry_fields_work_packet and ensure_schema_registry_fields_micro_task
  - Add task_board_id, lane_id, display_order, view_ids fields to TaskBoardEntryRecordV1 in task_board.rs
  - Update task board emission paths in workflows.rs to populate new fields
  - Verify and fix RoleMailboxThreadLineV1 field completeness in role_mailbox.rs including transcription_links
  - Write runtime integration test proving validate_summary_detail_pair runs on actual artifact emission paths
  - Write tests for structured diagnostic output, updated_at enforcement, TaskBoard fields, and ThreadLine completeness
- HOT_FILES:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core
  - cargo test -p handshake_core schema_registry
  - cargo test -p handshake_core task_board
  - cargo test -p handshake_core role_mailbox
- CARRY_FORWARD_WARNINGS:
  - v2 passed validator but failed operator code inspection; v3 must close gaps with actual implementation, not narrative
  - Do not widen scope beyond schema registry/validation surface; keep file-lock isolation from Loom storage
  - Structured diagnostic type should be simple and flat; do not over-engineer into a full error framework
  - TaskBoardEntryRecordV1 field additions must be backward-compatible (Optional/nullable for existing records)
  - RoleMailboxThreadLineV1 transcription_links must match spec shape exactly (Array of objects with target_kind, target_ref, target_sha256, note_redacted, note_sha256)
  - Keep product-runtime validation separate from .GOV governance-side validation

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - [ADD v02.168] updated_at enforced in base envelope validation (ensure_schema_registry_fields_*)
  - [ADD v02.168] TaskBoardEntryRecordV1 includes task_board_id, lane_id, display_order, view_ids
  - [ADD v02.168] Validation returns structured diagnostics, not string errors
  - [ADD v02.168] Summary/detail join validation runs on actual emission paths (runtime integration)
  - RoleMailboxThreadLineV1 field completeness including transcription_links
- FILES_TO_READ:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core
  - rg -n "updated_at|task_board_id|lane_id|display_order|view_ids|transcription_links|SchemaValidationDiagnostic|validate_summary_detail_pair" src/backend/handshake_core
- POST_MERGE_SPOTCHECKS:
  - Verify structured diagnostic payload is used in all validation paths, not just new ones
  - Verify TaskBoardEntryRecordV1 new fields are populated by emission paths, not just declared
  - Verify RoleMailboxThreadLineV1 transcription_links matches spec type shape

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Exact structured diagnostic payload shape has not been designed; the coder must choose a representation that is both machine-readable and stable
  - Whether existing TaskBoard emission paths set task_board_id/lane_id/display_order at all, or whether those values need to be threaded from upstream context
  - Whether RoleMailboxThreadLineV1 transcription_links is currently emitted as an empty array, partially populated, or completely absent
  - How many existing test fixtures will need updating after TaskBoardEntryRecordV1 gains new fields

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Main Body explicitly names the shared base envelope with updated_at, compact summary contract, project-profile extension boundary, Task Board projection fields, and Role Mailbox export field family including ThreadLineV1. The implementation work is therefore clearly specified and testable without new normative text.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.178.md already defines the shared collaboration envelope with updated_at, compact summary contract, profile-extension boundary, Task Board projection field requirements, and Role Mailbox export field family in the Main Body and appendix ownership notes. This packet is a runtime registry, validation, and field-completeness alignment pass.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md interface TrackedWorkPacket shared structured-collaboration envelope
- CONTEXT_START_LINE: 6101
- CONTEXT_END_LINE: 6113
- CONTEXT_TOKEN: interface TrackedWorkPacket {
- EXCERPT_ASCII_ESCAPED:
  ```text
  interface TrackedWorkPacket {
    // Shared structured-collaboration envelope
    schema_id: "hsk.tracked_work_packet@1";
    schema_version: "1";
    record_id: string;                   // Stable alias of wp_id
    record_kind: "work_packet";
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    authority_refs: ArtifactHandle[];
    evidence_refs: ArtifactHandle[];
    summary_record_path?: string;        // ".handshake/gov/work_packets/WP-1-Auth-System/summary.json"
    mirror_contract?: MarkdownMirrorContractV1;
    profile_extension?: ProjectProfileExtensionV1;
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md interface TrackedMicroTask shared structured-collaboration envelope
- CONTEXT_START_LINE: 6230
- CONTEXT_END_LINE: 6242
- CONTEXT_TOKEN: interface TrackedMicroTask {
- EXCERPT_ASCII_ESCAPED:
  ```text
  interface TrackedMicroTask {
    // Shared structured-collaboration envelope
    schema_id: "hsk.tracked_micro_task@1";
    schema_version: "1";
    record_id: string;                   // Stable alias of mt_id
    record_kind: "micro_task";
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    authority_refs: ArtifactHandle[];
    evidence_refs: ArtifactHandle[];
    summary_record_path?: string;        // ".handshake/gov/micro_tasks/WP-1-Auth-System/MT-001/summary.json"
    mirror_contract?: MarkdownMirrorContractV1;
    profile_extension?: ProjectProfileExtensionV1;
  ```

#### ANCHOR 3
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

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md RoleMailboxIndexV1 and RoleMailboxThreadLineV1 base envelope
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
