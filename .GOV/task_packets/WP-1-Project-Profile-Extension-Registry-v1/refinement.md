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
- WP_ID: WP-1-Project-Profile-Extension-Registry-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-31T16:10:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- SPEC_TARGET_SHA1: 747a1e77cbe2e1c564d1a99d5c39265edc6aeca2
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja310320261913
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Project-Profile-Extension-Registry-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- `profile_extension` metadata is only partially implemented today. `StructuredCollaborationSummaryV1`, `TrackedWorkPacketArtifactV1`, and `TrackedMicroTaskArtifactV1` accept optional extension payloads, but there is no explicit registry of supported extension schema ids, versions, and compatibility states.
- Task Board projections still hard-code `software_delivery` and omit any profile-extension boundary, so the shared base-envelope contract is not preserved on entry, index, or view artifacts.
- Role Mailbox export records still hard-code `software_delivery` and omit any profile-extension boundary, so mailbox export portability is weaker than the current Main Body requires.
- Product code does not yet prove a non-software emitted-artifact case, so the current partial plumbing can be socially over-credited as complete even though the spec explicitly calls for software and non-software portability.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: current Master Spec v02.179 sections for shared structured-collaboration envelope law, current local product code in `../handshake_main/src/backend/handshake_core`, current backlog stubs and audits for project-profile portability, and external patterns from GitHub Projects, Backstage, OpenMetadata, and JSON Schema research
- REFERENCES: `.GOV/spec/Handshake_Master_Spec_v02.179.md`, `.GOV/task_packets/stubs/WP-1-Project-Profile-Extension-Registry-v1.md`, `.GOV/Audits/smoketest/AUDIT_20260329_WORKFLOW_PROJECTION_CORRELATION_V1_SMOKETEST_PROOF_RUN_REVIEW.md`, `../handshake_main/src/backend/handshake_core/src/locus/types.rs`, `../handshake_main/src/backend/handshake_core/src/locus/task_board.rs`, `../handshake_main/src/backend/handshake_core/src/workflows.rs`, `../handshake_main/src/backend/handshake_core/src/role_mailbox.rs`, `../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs`, `https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-table-layout`, `https://backstage.io/docs/features/software-catalog/descriptor-format`, `https://github.com/backstage/backstage`, `https://github.com/open-metadata/OpenMetadata`, and `https://arxiv.org/abs/2307.10034`
- PATTERNS_EXTRACTED: keep one small shared envelope; require explicit extension schema ids, versions, and compatibility states; treat grouping and custom fields as viewer/layout configuration rather than authoritative workflow state; keep extension semantics simple enough that validation stays deterministic and understandable
- DECISIONS ADOPT/ADAPT/REJECT: adopt a registry-first contract with explicit compatibility semantics and base-envelope parity across artifact families; adapt viewer-level grouping and custom-field ideas into derived Task Board or Command Center layouts rather than canonical record law; reject free-form extension sprawl or dynamic schema behavior that makes portable validation hard to reason about
- LICENSE/IP_NOTES: Source review informed contract shape and governance choices only. No third-party code or copyrighted text is intended for direct reuse.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.179.md already defines the shared base envelope, project-profile extension metadata, compatibility semantics, and mailbox export boundary. This WP is implementation and proof remediation against the current Main Body, not a spec-version expansion.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON_NO: N/A
- SOURCE_MAX_AGE_DAYS: 30
- SOURCE_LOG:
  - Source: GitHub Projects table layout docs | Kind: BIG_TECH | Date: 2026-03-31 | Retrieved: 2026-03-31T15:20:00Z | URL: https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-table-layout | Why: shows how configurable fields, grouping, slicing, and sorting can stay view-level behavior over shared items instead of redefining the canonical item contract
  - Source: Backstage descriptor format | Kind: OSS_DOC | Date: 2026-03-31 | Retrieved: 2026-03-31T15:21:00Z | URL: https://backstage.io/docs/features/software-catalog/descriptor-format | Why: demonstrates a stable shared envelope with kind-specific extension metadata and annotations
  - Source: backstage/backstage | Kind: GITHUB | Date: 2026-03-31 | Retrieved: 2026-03-31T15:22:00Z | URL: https://github.com/backstage/backstage | Why: provides a large OSS implementation surface for descriptor-based extensibility and catalog-style shared parsing
  - Source: open-metadata/OpenMetadata | Kind: GITHUB | Date: 2026-03-30 | Retrieved: 2026-03-31T15:25:00Z | URL: https://github.com/open-metadata/OpenMetadata | Why: reinforces registry-first schemas plus custom extensions/properties consumed by multiple surfaces from one central metadata repository
  - Source: Validation of Modern JSON Schema: Formalization and Complexity | Kind: PAPER | Date: 2024-02-01 | Retrieved: 2026-03-31T15:25:30Z | URL: https://arxiv.org/abs/2307.10034 | Why: warns that overly dynamic schema semantics become hard to reason about and validate, which argues for explicit low-cardinality compatibility handling in Handshake
- RESEARCH_SYNTHESIS:
  - The base envelope should stay small, typed, and portable while project-specific data moves into explicit versioned extensions.
  - Viewer/layout customization should operate over canonical shared fields rather than silently becoming the authority for workflow or portability semantics.
  - Extension compatibility needs to be explicit and simple, because dynamic or annotation-heavy schema behavior quickly becomes difficult to validate and explain.
- RESEARCH_GAPS_TO_TRACK:
  - Full Dev Command Center viewer ergonomics for unknown extensions remain downstream of the current backend-oriented remediation and should stay with existing layout/viewer backlog.
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: Backstage descriptor format | Pattern: stable shared envelope with kind-specific extension metadata | Why: aligns directly with Handshake's base-envelope plus `profile_extension` split
  - Source: open-metadata/OpenMetadata | Pattern: central registry plus custom extensions/properties consumed by multiple surfaces | Why: supports a registry-first implementation instead of per-surface ad hoc field drift
- ADAPT_PATTERNS:
  - Source: GitHub Projects table layout docs | Pattern: grouping, slicing, sorting, and custom fields at the view layer | Why: useful for Task Board and Command Center projection behavior, but Handshake must keep those as derived layout choices over canonical records
- REJECT_PATTERNS:
  - Source: Validation of Modern JSON Schema: Formalization and Complexity | Pattern: highly dynamic schema behavior and annotation-dependent validation | Why: Handshake should reject extension semantics that are too dynamic to validate deterministically across artifact families
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - `site:github.com/backstage/backstage descriptor metadata envelope annotations`
  - `site:github.com/open-metadata/OpenMetadata metadata schemas custom properties`
- MATCHED_PROJECTS:
  - Source: backstage/backstage | Repo: backstage/backstage | URL: https://github.com/backstage/backstage | Intent: ARCH_PATTERN | Decision: ADAPT | Impact: NONE | Stub: NONE | Notes: useful shared-envelope pattern, but Handshake needs stricter compatibility declarations than free-form annotations alone provide
  - Source: open-metadata/OpenMetadata | Repo: open-metadata/OpenMetadata | URL: https://github.com/open-metadata/OpenMetadata | Intent: ARCH_PATTERN | Decision: ADOPT | Impact: NONE | Stub: NONE | Notes: schema registry plus extensible properties is the right direction for portable record consumers
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new Flight Recorder event ids are needed for this WP.
- Existing Locus, Task Board, and Role Mailbox event families remain the telemetry seams; this packet closes record portability and emitted-artifact parity rather than event taxonomy.
- Unknown or incompatible profile-extension failures should surface as deterministic structured validation outcomes instead of mailbox-local or viewer-local fallback behavior.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: partial `profile_extension` support in summaries and packet artifacts is mistaken for full closure. Mitigation: prove end-to-end parity across Task Board and Role Mailbox emitted artifacts before claiming completion.
- Risk: unknown extensions are silently dropped and operators think a record is complete when only the base envelope was understood. Mitigation: preserve explicit compatibility semantics and keep base-envelope fallback visible.
- Risk: Task Board and Role Mailbox continue to flatten records to `software_delivery`, making future non-software kernels inherit hidden repository assumptions. Mitigation: propagate `project_profile_kind` and `profile_extension` boundaries through those projections and exports.
- Risk: scope creeps into project-agnostic workflow-state or transition-law work. Mitigation: keep this packet limited to profile kinds, extension schema ids and compatibility, artifact propagation, and generic-fallback proof.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-ProjectProfileExtensionV1
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-ProjectProfileExtensionV1
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
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
  - The spec already declares the required primitive families. The remaining gap is implementation and proof across all emitted artifact families, not the invention of new primitive ids.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current spec appendix already names the project-profile extension and shared structured-collaboration primitives this packet uses.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Keep Appendix 12.4 unchanged and implement against existing primitive law.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new primitive family was discovered during this remediation.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Existing feature registry entries already cover the Work Packet, Micro-Task, Task Board, Role Mailbox, and Command Center surfaces affected here.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This remediation closes backend contract propagation and proof. It does not implement a new GUI surface directly.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Current Appendix 12.6 already contains the relevant Role Mailbox, Task Board, Work Packet, and Dev Command Center interaction edges.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep the current spec unchanged and implement against existing Main Body law.
  - If coding reveals a genuinely missing primitive or interaction edge, treat that as a separate spec-update flow instead of silent packet drift.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is changed by profile-extension registry remediation | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physical simulation or measurement law is involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers only | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing execution surface changes here | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration consumes these contracts later but is not the implementation surface in this packet | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication/export controllers remain downstream consumers of the shared records | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: this packet hardens durable artifact portability and shared parsing across packet, board, and mailbox records | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval remains downstream of the registry and fallback contract | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytics surfaces consume these records later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: storage portability remains a separate concern; this packet stays at the record-contract layer | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this packet implements existing law and does not add new governance authority | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or explanation interface is implemented here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: generic consumers and local-small-model readers depend on the base-envelope versus extension boundary staying explicit | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: explicit extension schema ids, versions, and compatibility semantics are central to this remediation | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: no new event taxonomy is introduced; existing event families remain sufficient | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: no calendar-facing surface depends directly on this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus owns the structured-collaboration contract and emitted-artifact validation seams this packet repairs | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: storage portability remains separate from the profile-extension registry gap | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: work packet artifacts are consumers of the shared contract here, but this packet is not a top-level Work Packets pillar expansion | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: task board projections are repaired as consumers of the shared contract, not as a separate Task Board pillar expansion | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: micro-task detail and summary artifacts are part of the same shared artifact family and need the same contract proof | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: future viewers benefit from this contract, but this packet does not implement a Command Center surface directly | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: NOT_TOUCHED | NOTES: no job runner or scheduler logic changes are required | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: this packet is downstream of prompt/spec generation and does not alter it | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: storage-backend portability remains unaffected by this record-contract pass | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: NOT_TOUCHED | NOTES: the packet preserves existing small-model-friendly parsing but does not open a new LLM-friendly data subsystem | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no media staging workflow is affected | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio runtime or creative console behavior is touched | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design or capture surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no training or distillation pipeline depends directly on this work | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE execution surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers of the shared records | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: project-profile extension validation and compatibility | SUBFEATURES: shared base envelope, extension schema ids, compatibility semantics | PRIMITIVES_FEATURES: PRIM-ProjectProfileExtensionV1, PRIM-StructuredCollaborationEnvelopeV1, FEAT-WORK-PACKET-SYSTEM, FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.archivist, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: central registry and validation logic belong here
  - PILLAR: MicroTask | CAPABILITY_SLICE: canonical detail and compact summary boundary | SUBFEATURES: micro-task packet.json, summary.json, non-software proof case | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1, FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task artifacts already carry partial support and need the same registry-backed proof as work packets
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: profile-extension registry enforcement | JobModel: NONE | Workflow: structured_collaboration_validation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is shared backend validation logic, not a standalone operator tool
  - Capability: Task Board projection parity for project-profile fields | JobModel: WORKFLOW | Workflow: task_board_projection_refresh | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: generic viewers depend on these projections preserving base-envelope semantics
  - Capability: Role Mailbox export parity for project-profile fields | JobModel: WORKFLOW | Workflow: role_mailbox_export | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: export consumers should parse mailbox records without software-only assumptions
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Existing Appendix 12.6 already contains the relevant Role Mailbox, Task Board, Work Packet, and Dev Command Center interaction edges. This packet uses those edges but does not need a new matrix row.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Current matrix coverage is sufficient; this packet closes implementation drift rather than creating a new cross-feature interaction class.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON_NO: N/A
- SOURCE_SCAN:
  - Source: GitHub Projects table layout docs | Kind: BIG_TECH | Angle: configurable views over shared items | Pattern: group and sort by fields without redefining the item schema | Decision: ADAPT | EngineeringTrick: keep field-driven layouts as projection state, not canonical workflow law | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: useful for Task Board and future Command Center behavior once base-envelope parity is restored
  - Source: Backstage descriptor format | Kind: OSS_DOC | Angle: envelope plus extensions | Pattern: common descriptor metadata with kind-specific spec and annotations | Decision: ADOPT | EngineeringTrick: preserve one parser-friendly envelope and move special data behind explicit extension boundaries | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly applicable to Handshake's shared artifact family
  - Source: open-metadata/OpenMetadata | Kind: GITHUB | Angle: registry plus extensible metadata | Pattern: core schemas with custom properties consumed by multiple surfaces | Decision: ADOPT | EngineeringTrick: centralize the schema authority rather than letting each consumer invent its own field contract | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: reinforces a registry-first approach
  - Source: Validation of Modern JSON Schema: Formalization and Complexity | Kind: PAPER | Angle: validation complexity | Pattern: complex dynamic schema semantics are hard to validate and explain | Decision: REJECT | EngineeringTrick: keep compatibility states low-cardinality and explicit | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: argues against dynamic extension semantics
- MATRIX_GROWTH_CANDIDATES:
  - Combo: base-envelope registry plus view-level custom grouping | Sources: GitHub Projects table layout docs, Backstage descriptor format | WhatToSteal: canonical shared fields with configurable views on top | HandshakeCarryOver: restore Task Board and Command Center freedom to relabel or group without forking base portability | RuntimeConsequences: backend records stay stable while viewers remain configurable | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: strongest direct carry-over for this packet
  - Combo: registry-first schemas plus explicit compatibility semantics | Sources: open-metadata/OpenMetadata, Validation of Modern JSON Schema: Formalization and Complexity | WhatToSteal: central registry authority and simple compatibility states | HandshakeCarryOver: avoid dynamic extension semantics that would make validators and generic viewers ambiguous | RuntimeConsequences: simpler emitted-artifact validation and clearer fallback behavior | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: exactly matches the missing product gap
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep canonical shared fields visible even when extensions are unknown or collapsed.
  - Separate viewer grouping and custom-field behavior from authoritative workflow and portability semantics.
  - Make extension compatibility explicit enough that validators can reject or degrade deterministically.
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: base-envelope parity across work-packet and micro-task detail artifacts | Pillars: Locus, MicroTask | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1, PRIM-ProjectProfileExtensionV1, FEAT-WORK-PACKET-SYSTEM, FEAT-MICRO-TASK-EXECUTOR | Resolution: IN_THIS_WP | Stub: NONE | Notes: closes the core registry gap on canonical packet-family artifacts
  - Combo: task-board projection parity from the Locus registry | Pillars: Locus | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1, FEAT-TASK-BOARD | Resolution: IN_THIS_WP | Stub: NONE | Notes: removes the most obvious software-only flattening bug from board projections
  - Combo: role-mailbox export parity from the Locus registry | Pillars: Locus | Mechanical: engine.archivist, engine.context | Primitives/Features: PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1, PRIM-ProjectProfileExtensionV1, FEAT-ROLE-MAILBOX | Resolution: IN_THIS_WP | Stub: NONE | Notes: keeps mailbox exports portable across non-software kernels
  - Combo: unknown-extension fallback on micro-task summaries | Pillars: MicroTask | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1, FEAT-MICRO-TASK-EXECUTOR | Resolution: IN_THIS_WP | Stub: NONE | Notes: proves generic readers still function when extension-specific fields are unavailable
  - Combo: non-software proof case across packet and micro-task artifact pairs | Pillars: Locus, MicroTask | Mechanical: engine.version | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-ProjectProfileExtensionV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: prevents closure claims from relying only on software-delivery examples
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered here belong inside this activation of the existing stub and do not require a new stub or spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed structured-collaboration packets, current Master Spec v02.179, and local product code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Project-Profile-Extension-Registry-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this is the correct governed shell for the remaining implementation gap
  - Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: downstream portable workflow-state law depends on this packet but should not be folded into it
  - Artifact: WP-1-Workflow-Transition-Automation-Registry-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: transition and automation law remain downstream of the profile-extension registry
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established the artifact family but did not close the project-profile registry
  - Artifact: WP-1-Role-Mailbox-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: mailbox export plumbing exists, but project-profile parity remains incomplete
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Project-Profile-Extension-Registry-v1 | Covers: primitive | Verdict: PARTIAL | Notes: `ProjectProfileKind` and optional `profile_extension` validation exist, but there is no explicit registry of supported extension schema ids and compatibility contracts
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs | Artifact: WP-1-Project-Profile-Extension-Registry-v1 | Covers: primitive | Verdict: NOT_PRESENT | Notes: Task Board entry, index, and view records do not currently carry `profile_extension`
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Project-Profile-Extension-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: some packet and micro-task emitters copy `profile_extension`, but Task Board projections still hard-code `software_delivery`
  - Path: ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs | Artifact: WP-1-Project-Profile-Extension-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: mailbox export records still hard-code `software_delivery` and omit the profile-extension boundary
  - Path: ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs | Artifact: WP-1-Project-Profile-Extension-Registry-v1 | Covers: execution | Verdict: PARTIAL | Notes: only a negative-path extension compatibility fixture exists; no full registry or non-software emitted-artifact proof exists
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The existing stub is the correct authoritative shell for this gap. Product code is still only partial, but no new stub or spec update is required.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet closes backend contract propagation and proof. It does not implement a new GUI surface directly.
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
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet. Current viewer work remains downstream of the existing Command Center layout/viewer backlog.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.168]
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md Base structured schema and project-profile extension contract [ADD v02.168]
- WHAT: Complete the explicit project-profile extension registry and propagate the base-envelope versus `profile_extension` boundary through Work Packet, Micro-Task, Task Board, and Role Mailbox emitted artifacts, including generic-fallback proof and one non-software example.
- WHY: Current product code over-credits partial envelope plumbing as if the registry were done. Downstream portable workflow-law work remains unsafe until registry, projection, and export truth are aligned end-to-end.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- OUT_OF_SCOPE:
  - Project-agnostic workflow-state registry or transition automation work
  - Full Dev Command Center frontend or typed-viewer implementation
  - Main Body or appendix spec updates
  - Loom storage portability or runtime-backend refactors
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - Shared structured-collaboration validation enforces explicit profile-extension schema id, version, and compatibility semantics instead of accepting opaque partial metadata as if the registry were complete.
  - Task Board and Role Mailbox emitted artifacts preserve `project_profile_kind` and the profile-extension boundary instead of hard-coding `software_delivery`.
  - Base-envelope parsing still works when an extension is unknown or omitted, and that fallback is covered by tests.
  - At least one software-delivery example and one non-software emitted-artifact example are produced and validated without breaking base-envelope validity.
- PRIMITIVES_EXPOSED:
  - PRIM-ProjectProfileExtensionV1
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- SEARCH_TERMS:
  - project_profile_kind
  - profile_extension
  - extension_schema_id
  - extension_schema_version
  - compatibility
  - software_delivery
- RUN_COMMANDS:
  ```bash
  rg -n "project_profile_kind|profile_extension|extension_schema_id|software_delivery" src/backend/handshake_core/src src/backend/handshake_core/tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  just gov-check
  ```
- RISK_MAP:
  - "Task Board or mailbox exports still flatten to software_delivery" -> "future non-software kernels inherit hidden repository assumptions"
  - "Unknown extensions silently disappear from generic readers" -> "operators and small models over-trust partial records"
  - "Packet widens into workflow-state or transition law" -> "remediation becomes too broad and loses proof quality"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just create-task-packet` will run `build-order-sync` automatically.
  - After packet creation, verify TASK_BOARD, WP traceability, and Build Order projections all reflect the activated packet truth.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Base structured schema and project-profile extension contract [ADD v02.168] | WHY_IN_SCOPE: current product code only partially implements the required profile-extension registry and parity across artifact families | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/locus/task_board.rs; src/backend/handshake_core/src/role_mailbox.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension; cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board; cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox | RISK_IF_MISSED: the registry will still be socially treated as done while consumers remain software-only
  - CLAUSE: `profile_extension` payloads declare schema id, schema version, and compatibility and keep the base envelope valid when absent | WHY_IN_SCOPE: current validation accepts extension-shaped metadata but does not prove a real registry-backed contract or fallback behavior end-to-end | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/tests/micro_task_executor_tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension | RISK_IF_MISSED: unknown or incompatible extensions will keep failing late or silently
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 share the base envelope and project-profile extension boundary | WHY_IN_SCOPE: mailbox exports currently flatten to `software_delivery` and omit the explicit boundary | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox.rs; src/backend/handshake_core/tests/role_mailbox_tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox | RISK_IF_MISSED: mailbox exports will remain the easiest place for hidden repository assumptions to leak back into the shared artifact family

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: StructuredCollaborationSummaryV1 `profile_extension` payload | PRODUCER: locus summary emitters | CONSUMER: generic readers and small-model summary flows | SERIALIZER_TRANSPORT: packet summary JSON | VALIDATOR_READER: validate_profile_extension in locus/types.rs | TRIPWIRE_TESTS: profile_extension positive and negative cases in micro_task_executor_tests.rs | DRIFT_RISK: summary-level support can look complete even when downstream detail artifacts are not
  - CONTRACT: TrackedWorkPacketArtifactV1 and TrackedMicroTaskArtifactV1 project-profile boundary | PRODUCER: workflows.rs emitters | CONSUMER: Work Packet and Micro-Task detail readers | SERIALIZER_TRANSPORT: packet.json and summary.json | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: profile_extension filter tests plus emitted-artifact parity tests | DRIFT_RISK: packet and micro-task paths can diverge from Task Board or mailbox behavior silently
  - CONTRACT: TaskBoardEntryRecordV1, TaskBoardIndexV1, and TaskBoardViewV1 project-profile fields | PRODUCER: workflows.rs task-board projection builder | CONSUMER: Task Board and generic board viewers | SERIALIZER_TRANSPORT: task_board entry/index/view JSON | VALIDATOR_READER: task board record validators in locus/types.rs | TRIPWIRE_TESTS: task_board-targeted cargo tests with software and non-software examples | DRIFT_RISK: board views can stay software-only even when packet artifacts evolve
  - CONTRACT: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 project-profile fields | PRODUCER: role_mailbox.rs export writers | CONSUMER: mailbox triage and generic export readers | SERIALIZER_TRANSPORT: index.json and thread.jsonl | VALIDATOR_READER: mailbox record validators in locus/types.rs and export checks | TRIPWIRE_TESTS: role_mailbox cargo tests with unknown-extension fallback and non-software example | DRIFT_RISK: mailbox export portability silently diverges from the rest of the artifact family

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - Work Packet detail plus summary artifacts with `project_profile_kind=software_delivery` and a valid software-delivery `profile_extension`
  - Work Packet or Micro-Task detail plus summary artifacts with `project_profile_kind=research` and a valid non-software `profile_extension`
  - Task Board and Role Mailbox exported artifacts that preserve base-envelope fields and remain parseable when `profile_extension` is unknown or omitted

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Define the explicit registry and compatibility enforcement in `src/backend/handshake_core/src/locus/types.rs`.
  - Propagate `project_profile_kind` and the profile-extension boundary through Task Board and Role Mailbox emitted artifacts.
  - Add software-delivery and non-software proof cases plus unknown-extension fallback regression coverage.
- HOT_FILES:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
- CARRY_FORWARD_WARNINGS:
  - Do not hard-code `software_delivery` anywhere this packet touches.
  - Do not widen into workflow-state registry or transition-automation law.
  - Do not satisfy fallback behavior by dropping `profile_extension` while also dropping base-envelope parity.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - Base-envelope parity across Work Packet, Micro-Task, Task Board, and Role Mailbox records
  - Explicit extension schema id, version, and compatibility enforcement
  - Non-software emitted-artifact proof and unknown-extension fallback behavior
- FILES_TO_READ:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - NONE

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Current local `cargo test` runtime is slow and previous targeted commands timed out during audit, so runtime proof still depends on the actual coding and validation passes.
  - This packet does not prove a full Dev Command Center GUI implementation; it proves the backend contract and emitted-artifact fallback that current and future viewers rely on.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Current Main Body law explicitly defines the base envelope, project-profile extension metadata, compatibility semantics, and mailbox export shapes. The remaining work is specific and measurable in product code.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Current Handshake_Master_Spec_v02.179.md already defines the contract this packet needs to implement and prove. No Main Body or appendix update is required before packet activation.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md Base structured schema and project-profile extension contract [ADD v02.168]
- CONTEXT_START_LINE: 6840
- CONTEXT_END_LINE: 6861
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
  - Implementations MAY denormalize base-envelope fields into top-level record keys, but Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST remain field-equivalent at the base-envelope level so shared viewers, validators, and local-small-model ingestion can reuse the same parser.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md RoleMailboxIndexV1 and RoleMailboxThreadLineV1 export schemas
- CONTEXT_START_LINE: 11020
- CONTEXT_END_LINE: 11086
- CONTEXT_TOKEN: interface RoleMailboxIndexV1 {
- EXCERPT_ASCII_ESCAPED:
  ```text
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
  ```
