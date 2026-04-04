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
- WP_ID: WP-1-Storage-Capability-Boundary-Refactor-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-03T22:49:22.031Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- SPEC_TARGET_SHA1: 747a1e77cbe2e1c564d1a99d5c39265edc6aeca2
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja040420260133
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Storage-Capability-Boundary-Refactor-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- The `Database` trait in `../handshake_main/src/backend/handshake_core/src/storage/mod.rs` still acts as a monolithic boundary for unrelated domains such as documents, canvas, calendar, Loom, Locus, retention, and structured collaboration.
- The short-term trait-purity remediation removed concrete downcasts, but it replaced them with more capability flags and subsystem-specific methods on the same top-level trait instead of introducing narrower domain handles.
- Representative callers in `../handshake_main/src/backend/handshake_core/src/workflows.rs`, `../handshake_main/src/backend/handshake_core/src/api/loom.rs`, and `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs` still receive a broad storage surface even when they only need a narrow structured-collaboration or observability slice.
- There is no explicit compile-time tripwire that prevents future packets from appending more backend-sensitive methods to `Database`, which means portability drift can return in a slower but harder-to-audit form.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 50m
- SEARCH_SCOPE: current Master Spec v02.179 storage-portability clauses, the active boundary-refactor stub, prior storage-portability packets, and current product code under `../handshake_main/src/backend/handshake_core/src/storage`, `../handshake_main/src/backend/handshake_core/src/api`, and `../handshake_main/src/backend/handshake_core/src/workflows.rs`
- REFERENCES: `.GOV/spec/Handshake_Master_Spec_v02.179.md`, `.GOV/task_packets/stubs/WP-1-Storage-Capability-Boundary-Refactor-v1.md`, `.GOV/task_packets/WP-1-Storage-Trait-Purity-v1/packet.md`, `.GOV/task_packets/WP-1-Loom-Storage-Portability-v4/packet.md`, `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`, `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`, `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/storage/retention.rs`, `../handshake_main/src/backend/handshake_core/src/workflows.rs`, `../handshake_main/src/backend/handshake_core/src/api/loom.rs`, and `../handshake_main/src/backend/handshake_core/src/storage/tests.rs`
- PATTERNS_EXTRACTED: keep one authoritative storage boundary, but express it through composed domain interfaces or service handles; isolate backend capability snapshots from operational methods; keep raw pool access confined to backend implementations; prove the boundary with dual-backend tripwires instead of social guidance
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT narrower domain-facing storage handles plus an explicit capability snapshot; ADAPT the current `Database` trait into a composition root rather than a dumping ground for every subsystem hook; REJECT solving future backend differences by appending more ad hoc methods and booleans to `Database`
- LICENSE/IP_NOTES: internal governance and product-code review only; no third-party code or copyrighted text will be copied into the implementation
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.179.md already requires one storage API, a trait-based abstraction boundary, trait purity, and dual-backend tests. This packet is implementation-boundary hardening against existing Main Body law.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP is strictly internal and mechanical. The governing source of truth is the current local Master Spec plus current storage-boundary code reality, not a time-sensitive external vendor or standards change.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - The missing work is structural, not exploratory: Handshake already knows the required portability law, but the current trait shape still invites hidden breadth and future capability drift.
  - The refactor should improve compile-time boundary clarity without reintroducing raw-pool exposure, backend downcasts, or a second storage API.
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
- No new Flight Recorder event ids are required.
- Existing Loom search observability and any storage-boundary telemetry must keep their current semantics if the underlying boundary is split into narrower interfaces.
- This packet is storage-boundary refactoring, not telemetry taxonomy expansion.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: the refactor can simply recreate the same monolith behind a different type name. Mitigation: require a materially narrower caller surface and explicit capability/service boundaries in both code and tests.
- Risk: raw provider pools or backend-specific types can leak into new helper traits while the top-level `Database` trait looks cleaner. Mitigation: keep all raw pool access inside backend implementations only and grep for provider-type leaks in callers.
- Risk: future packets can start re-growing the boundary if no tripwire exists. Mitigation: add tests or structural assertions that fail when subsystem-specific methods accrete back onto the global boundary without an explicit design decision.
- Risk: subsystem consumers can silently keep depending on unrelated storage domains after the refactor. Mitigation: migrate representative callers and prove the narrower interface boundary through compile-time usage and regression tests.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - This packet reorganizes existing storage-boundary primitives and their caller contracts.
  - No new spec primitive family is required at refinement time; the work is a boundary split and capability-shape correction inside existing primitives.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already names the storage abstraction primitives this packet is tightening.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Keep the primitive index unchanged and implement the boundary split against existing storage primitives.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No orphan primitive family was discovered; the issue is accidental breadth on the existing storage boundary.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: This packet refactors storage-boundary shape and does not introduce a new feature family.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface is implemented in this packet.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The packet changes storage-boundary composition, not the catalog of cross-feature product interactions.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep the current spec unchanged and implement against existing Main Body portability law.
  - If coding reveals a genuinely missing primitive or interaction contract, open a separate governed spec-update path instead of silently widening this packet.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability changes are introduced by the storage-boundary split | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no simulation or measurement contract is affected | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes remain downstream storage consumers only | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing execution surface changes here | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration consumes storage but is not the feature surface being designed in this packet | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media-composition surface is touched | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no rendering or art-generation capability is changed | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication and export flows remain downstream consumers | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no compliance or food-safety surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: stored artifacts still exist, but this packet changes the access boundary rather than archivist semantics | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval remains downstream of the storage split | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: no analytics or insight-generation surface is directly changed | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: this packet refactors the database abstraction boundary into a more durable portability shape | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this packet implements existing storage law and does not add new governance authority | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no explanation or tutorial interface is implemented here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: the core value is narrowing which storage context each caller can observe and depend on | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: dual-backend tripwires and boundary-shape regression guards are part of the implementation contract | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Loom search and similar telemetry must keep working through narrower storage interfaces | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: no calendar-facing feature contract changes in this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document-editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus callers should consume only the structured-collaboration and task-board surfaces they actually need | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: TOUCHED | NOTES: Loom observability should depend on a dedicated query or capability view, not the full storage boundary | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no work-packet feature contract is changed directly; this packet only narrows the storage boundary beneath it | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task-board feature semantics are changed directly; only the storage access boundary underneath representative callers | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: no micro-task feature contract is changed directly; this packet only narrows the storage access beneath current readers | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: no new operator UI is introduced, although existing consumers may use narrower storage handles | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS contract changes are introduced directly | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: workflows should depend on focused runtime storage surfaces rather than the whole trait | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no prompt or spec-router feature surface is changed | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: compile-time boundary clarity is a prerequisite for sustainable backend parity work | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: NOT_TOUCHED | NOTES: no data contract changes are introduced directly; existing parser-friendly semantics must simply survive the refactor | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no Stage surface changes are in scope | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no Studio runtime or creative console changes are in scope | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design or capture surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no distillation or adapter-training flow is affected directly | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE runtime surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers of the storage boundary | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: top-level storage boundary shrink plus capability snapshot extraction | SUBFEATURES: smaller caller-facing interface, backend capability view, explicit domain handles | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits, PRIM-SqliteDatabase, PRIM-PostgresDatabase | MECHANICAL: engine.dba, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the main portability-hardening objective
  - PILLAR: Locus | CAPABILITY_SLICE: structured-collaboration and task-board storage access isolation | SUBFEATURES: tracked work-packet readers, micro-task readers, task-board update path, capability gating | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus should not depend on unrelated storage domains
  - PILLAR: Loom | CAPABILITY_SLICE: observability and graph-capability access isolation | SUBFEATURES: observability tier query, graph-filter support query, search-facing storage handle | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: keep telemetry stable while reducing caller breadth
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: workflow storage-handle narrowing | SUBFEATURES: runtime consumers take only the storage slice they execute against | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: prevents future packets from reaching for unrelated trait methods
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: telemetry-facing storage posture queries | SUBFEATURES: backend posture lookup, observability tier query, recorder-stable semantics through narrow handles | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: telemetry consumers should not require the whole storage boundary
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: storage capability snapshot | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: callers should read capability posture without inheriting unrelated domain methods
  - Capability: structured collaboration storage handle | JobModel: WORKFLOW | Workflow: structured_collaboration_runtime | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Locus and workflow code should depend on a narrow domain surface
  - Capability: Loom observability storage handle | JobModel: UI_ACTION | Workflow: NONE | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: LoomSearchExecuted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: keeps search-facing observability queries explicit and bounded
  - Capability: task-board update storage handle | JobModel: WORKFLOW | Workflow: task_board_projection_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: task-board sync should not carry unrelated document or calendar methods
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - This packet tightens existing storage and runtime combinations rather than creating a new product interaction class.
  - The highest-ROI result is compile-time narrowing of storage access, not appendix matrix expansion.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Existing matrix coverage is sufficient because the packet closes boundary-shape drift inside already-declared storage/runtime interactions.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This packet is strictly internal and mechanical. External combo research is not needed to decide whether one trait should keep accumulating unrelated subsystem hooks.
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
  - Combo: capability snapshot plus dual-backend regression guards | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.version, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits, PRIM-SqliteDatabase, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: compile-time boundary narrowing only matters if regression tests stop future accretion
  - Combo: structured-collaboration handle plus Locus runtime consumers | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the clearest narrow-domain slice in the current codebase
  - Combo: Loom observability handle plus Flight Recorder telemetry | Pillars: Loom, Flight Recorder | Mechanical: engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: observability queries should not keep the whole storage boundary broad
  - Combo: task-board projection handle plus workflow runtime | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: task-board sync is a representative caller that should not depend on unrelated domains
  - Combo: retention service handle plus capability snapshot | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: retention updates are another backend-sensitive hotspot that should not force caller breadth
  - Combo: Locus ready-query handle plus explicit backend posture | Pillars: Locus, SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: ready checks should consume focused capability data only
  - Combo: structured-collaboration handle plus summary readers | Pillars: Locus | Mechanical: engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: summary hydration is a good representative narrow read surface
  - Combo: backend capability snapshot plus workflow gating | Pillars: Execution / Job Runtime, SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.context, engine.version | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: workflow gates should read explicit capability posture instead of broad trait reachability
  - Combo: Loom graph-filter capability plus observability tier query | Pillars: Loom, Flight Recorder | Mechanical: engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: keep both Loom backend-sensitive queries on the same narrow surface
  - Combo: caller-boundary shrink plus source grep tripwire | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.version, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: enforce the architecture with a cheap static regression guard
  - Combo: focused runtime handle plus full-suite dual-backend tests | Pillars: Execution / Job Runtime, SQL to PostgreSQL shift readiness | Mechanical: engine.version, engine.dba | Primitives/Features: PRIM-Database, PRIM-SqliteDatabase, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: test posture must prove the narrowed boundary works on both backends
  - Combo: Locus storage handle plus task-board projection writer | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: write paths are as important as read paths when narrowing the interface
  - Combo: telemetry-facing capability snapshot plus recorder-stable semantics | Pillars: Flight Recorder, Loom | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend posture must stay explicit and semantically stable during the split
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered here belong inside the current packet activation and do not require a new stub or a spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed storage/structured-collaboration packets, current Master Spec v02.179, and local product code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Storage-Capability-Boundary-Refactor-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this is the correct governed shell for the current boundary-shape remediation
  - Artifact: WP-1-Storage-No-Runtime-DDL-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: related storage remediation, but focused on migration discipline instead of caller-boundary composition
- MATCHED_ACTIVE_PACKETS:
- Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | BoardStatus: IN_PROGRESS | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: parity implementation is a downstream consumer of this boundary and should not proceed on the same overlapping file-lock set first
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Dual-Backend-Tests-v2 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides test posture, but not the storage-boundary split or trait-growth tripwires required here
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Storage-Capability-Boundary-Refactor-v1 | Covers: primitive | Verdict: PARTIAL | Notes: the top-level `Database` trait still mixes broad document/storage concerns with subsystem-specific capability hooks and structured-collaboration readers
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Storage-Capability-Boundary-Refactor-v1 | Covers: execution | Verdict: PARTIAL | Notes: workflow code uses only narrow structured-collaboration slices, but still depends on the global storage boundary
  - Path: ../handshake_main/src/backend/handshake_core/src/api/loom.rs | Artifact: WP-1-Storage-Capability-Boundary-Refactor-v1 | Covers: execution | Verdict: PARTIAL | Notes: Loom observability reads hang directly off the top-level storage trait instead of a focused interface
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs | Artifact: WP-1-Storage-Capability-Boundary-Refactor-v1 | Covers: execution | Verdict: PARTIAL | Notes: Locus helpers currently bundle structured-collaboration and task-board logic on the same broad storage boundary
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | Artifact: WP-1-Storage-Capability-Boundary-Refactor-v1 | Covers: execution | Verdict: PARTIAL | Notes: tests prove capability posture, but not that the caller-facing boundary stopped accreting unrelated domains
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The existing stub is correct, but activation must explicitly cover caller-surface narrowing, capability-view extraction, and regression tripwires against future trait accretion.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet changes an internal backend abstraction and does not implement a new GUI surface.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Trait-Purity
- BUILD_ORDER_BLOCKS: WP-1-Postgres-Structured-Collaboration-Artifact-Parity
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md One Storage API plus Trait Purity Invariant and Dual-Backend Testing [CX-DBP-010]/[CX-DBP-040]/[CX-DBP-013]
- WHAT: Refactor the storage access boundary so representative subsystems consume narrower capability or domain interfaces instead of inheriting one monolithic `Database` trait.
- WHY: The current trait is downcast-free but still too broad, which makes future backend work easy to land in the wrong place and hard to audit for portability drift.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- OUT_OF_SCOPE:
  - implementing full PostgreSQL structured-collaboration parity
  - runtime DDL or migration-framework cleanup
  - new GUI surfaces or new Flight Recorder event families
  - long-tail migration of every storage caller beyond the representative hotspots identified in this refinement
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - The top-level storage boundary is materially smaller or composition-focused compared with the current monolithic `Database` trait.
  - Representative callers no longer depend on unrelated storage domains just to reach one narrow runtime capability.
  - New backend-sensitive feature work must extend a dedicated domain interface or capability snapshot rather than appending another ad hoc method to `Database`.
  - Regression tests fail if raw provider types, backend downcasts, or broad caller-boundary accretion return.
- PRIMITIVES_EXPOSED:
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - trait Database
  - supports_structured_collab_artifacts
  - loom_search_observability_tier
  - supports_loom_graph_filtering
  - locus_task_board_update_work_packet
  - structured_collab_
  - retention
- RUN_COMMANDS:
  ```bash
  rg -n "trait Database|supports_structured_collab_artifacts|loom_search_observability_tier|supports_loom_graph_filtering|locus_task_board_update_work_packet|structured_collab_" src/backend/handshake_core/src/storage/mod.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/api/loom.rs
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "refactor recreates the same monolith behind facade types" -> "future portability drift returns while the packet appears complete"
  - "raw backend types leak into new helper interfaces" -> "trait-purity law is weakened under a different name"
  - "representative callers are not actually narrowed" -> "the top-level boundary stays broad and the refactor has little durable value"
  - "packet execution overlaps with Postgres parity on the same files" -> "parallel implementation becomes unsafe and governance truth drifts"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just orchestrator-prepare-and-packet` will sync Task Board and Build Order truth.
  - Build Order must also record that `WP-1-Postgres-Structured-Collaboration-Artifact-Parity` is downstream of this packet because both scopes overlap on `storage/mod.rs`, `workflows.rs`, and storage-test surfaces.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: One Storage API [CX-DBP-010] | WHY_IN_SCOPE: representative callers still depend on one global storage trait instead of narrow domain surfaces | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/api/loom.rs; src/backend/handshake_core/src/storage/locus_sqlite.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: business logic remains coupled to a broad storage boundary and future backend work keeps accreting in the wrong place
  - CLAUSE: Trait Purity Invariant [CX-DBP-040] | WHY_IN_SCOPE: the current boundary is downcast-free but still exposes backend-sensitive and subsystem-specific breadth at the wrong level | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity; cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage | RISK_IF_MISSED: future packets can preserve trait purity in name while still weakening portability through accidental breadth
  - CLAUSE: Dual-Backend Testing Early [CX-DBP-013] | WHY_IN_SCOPE: the boundary split only helps if both backends keep implementing and proving the same caller contract after the refactor | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/tests.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: the boundary can drift differently on SQLite and PostgreSQL while still looking cleaner in a narrow local read

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: top-level storage composition boundary | PRODUCER: storage/mod.rs plus backend implementations | CONSUMER: workflows.rs, api/loom.rs, storage/locus_sqlite.rs, storage/retention.rs | SERIALIZER_TRANSPORT: in-process trait or service handle | VALIDATOR_READER: storage/tests.rs plus source inspection | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage | DRIFT_RISK: callers silently keep depending on unrelated domains through a facade that still behaves like one monolithic trait
  - CONTRACT: storage capability snapshot | PRODUCER: SQLite and PostgreSQL storage implementations | CONSUMER: workflow routing, Loom observability, backend-sensitive guards | SERIALIZER_TRANSPORT: in-process enum or capability view | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity | DRIFT_RISK: capability semantics drift and callers start growing bespoke checks again
  - CONTRACT: structured-collaboration storage handle | PRODUCER: storage boundary split in mod.rs | CONSUMER: workflows.rs and storage/locus_sqlite.rs | SERIALIZER_TRANSPORT: in-process focused interface | VALIDATOR_READER: storage/tests.rs and workflow-facing checks | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage | DRIFT_RISK: Locus and workflow code keep inheriting unrelated storage methods
  - CONTRACT: Loom observability storage handle | PRODUCER: storage boundary split in mod.rs | CONSUMER: api/loom.rs and Flight Recorder emission | SERIALIZER_TRANSPORT: in-process focused interface | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage | DRIFT_RISK: observability queries still require the broad top-level boundary and reintroduce drift pressure

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - workflow and Locus consumers receive a focused structured-collaboration storage surface instead of the whole `Database` trait
  - Loom observability reads backend posture through an explicit capability or dedicated interface rather than a broad trait dependency
  - future Postgres parity work can extend a dedicated structured-collaboration store instead of appending more methods to the global boundary

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Define the target boundary split or capability snapshot in `storage/mod.rs` without leaking raw provider types.
  - Migrate representative callers in workflows, Locus helpers, Loom, and retention to the narrowed surfaces.
  - Add regression tests or structural tripwires that fail when caller breadth or backend-sensitive accretion returns.
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage
- CARRY_FORWARD_WARNINGS:
  - Do not replace one monolithic trait with a facade that still hands every caller the same broad surface.
  - Do not leak raw `SqlitePool`, `PgPool`, or provider-specific helper types into caller code.
  - Do not run this packet in parallel with Postgres parity on overlapping files unless scope is explicitly re-split and re-signed.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - One Storage API [CX-DBP-010]
  - Trait Purity Invariant [CX-DBP-040]
  - Dual-Backend Testing Early [CX-DBP-013]
- FILES_TO_READ:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - confirm no follow-on packet widened `Database` again while addressing backend-specific work

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The exact final interface names and file count are not proven at refinement time.
  - The long-tail migration of every storage caller is not in scope; this packet only proves the boundary split on representative hotspots.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Main Body already requires one storage API, trait purity, and dual-backend testing, and this refinement turns the stub into a bounded caller-boundary refactor with concrete code surfaces and tripwire tests.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The current Master Spec already provides the governing portability and trait-purity law. This packet is product-code conformance work against that existing law.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.1 Pillar 1 One Storage API [CX-DBP-010]
- CONTEXT_START_LINE: 3264
- CONTEXT_END_LINE: 3277
- CONTEXT_TOKEN: Pillar 1: One Storage API [CX-DBP-010]
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Pillar 1: One Storage API [CX-DBP-010]**

  All database operations MUST flow through a single storage module boundary. No business logic code may directly access database connections.

  - FORBIDDEN: Direct `sqlx::query()` in API handlers
  - FORBIDDEN: Direct `state.pool` or `state.fr_pool` access outside `src/storage/`
  - REQUIRED: All DB operations via `state.storage.*` interface
  - REQUIRED: AppState MUST NOT expose raw `SqlitePool` or `DuckDbConnection`
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.1 Pillar 4 Dual-Backend Testing Early [CX-DBP-013]
- CONTEXT_START_LINE: 3313
- CONTEXT_END_LINE: 3323
- CONTEXT_TOKEN: Pillar 4: Dual-Backend Testing Early [CX-DBP-013]
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**

  Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.

  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend blocks PR merge
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.3 Trait Purity Invariant [CX-DBP-040]
- CONTEXT_START_LINE: 3361
- CONTEXT_END_LINE: 3368
- CONTEXT_TOKEN: Trait Purity Invariant
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.13.3 Storage API Abstraction Pattern [CX-DBP-021]

  The storage module MUST define a trait-based interface that hides database differences. This contract is MANDATORY for all storage implementations.

  **[CX-DBP-040] Trait Purity Invariant (Normative):**
  The `Database` trait MUST NOT expose any methods that return concrete, backend-specific types.
  - Violation: `fn sqlite_pool(&self) -> Option<&SqlitePool>` is strictly FORBIDDEN.
  - Remediation: services requiring database access MUST consume generic `Database` trait methods or a trait-compliant operation.
  ```
