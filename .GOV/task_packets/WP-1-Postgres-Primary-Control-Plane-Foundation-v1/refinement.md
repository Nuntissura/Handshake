<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.refinement_contract@1 source_file=.GOV/task_packets/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/packet.json source_hash=1a62e93708050b99 projection_hash=33bc4cf94970541d generated_at_utc=2026-05-06T15:01:56.008Z generator=wp-contract-import.mjs -->
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
- WP_ID: WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-05-05T17:55:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.182.md
- SPEC_TARGET_SHA1: a29a0f79a628c868e59e5376fb5bf9bcaa7f2d77
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja050520262319
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- STUB_WP_IDS: WP-1-Postgres-Dev-Test-Container-Matrix-v1, WP-1-Postgres-Control-Plane-Leases-Backpressure-v1, WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-FEMS-Postgres-Memory-Store-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
- FOLLOW_UP_STUB_AUTHORITY: ORCHESTRATOR_OWNS_STUB_CREATION_AND_GOV_KERNEL_COMMIT
- ACTIVATION_MANAGER_STUB_SCOPE: ASSESSMENT_ONLY_DO_NOT_EDIT_TASK_BOARD_BUILD_ORDER_TRACEABILITY_OR_STUB_FILES

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Current Master Spec v02.182 now includes the PostgreSQL-primary control-plane pivot; the remaining foundation gap is implementation of the storage-mode/default and fail-closed runtime contract against that updated spec law.
- Current product code already supports both `SqliteDatabase` and `PostgresDatabase`, but `storage::init_storage()` still defaults to `sqlite://data/handshake.db` when `DATABASE_URL` is absent, which conflicts with a PostgreSQL-primary self-hosting control-plane default.
- PostgreSQL now has meaningful structured-collaboration and Locus support, but the spec does not yet define which runtime/control-plane surfaces must treat PostgreSQL as authoritative and which SQLite surfaces remain cache, index, embedded, or offline projections.
- The pivot is larger than one implementation packet. Foundation work must define the spec law, storage-mode/config contract, fail-closed behavior, and minimal bootstrap checks. ModelSession queue workers, workflow durable execution, FEMS memory storage, DCC projections, SQLite fallback boundaries, and lease/backpressure semantics are separable downstream work.
- The current PostgreSQL test harness exists through `POSTGRES_TEST_URL` helpers, but the pivot needs a follow-up developer/container matrix so every downstream Postgres-primary WP does not rediscover service startup, reset, migration, fixture, and skip/fail semantics.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 70m
- SEARCH_SCOPE: Current Master Spec v02.182 PostgreSQL-primary control-plane foundation, storage portability, Locus, ModelSession scheduler, FEMS, Dev Command Center, software-delivery overlay, and SQLite-role anchors; candidate PostgreSQL follow-up WP IDs; current product storage/runtime code under `../handshake_main/src/backend/handshake_core`.
- REFERENCES: `.GOV/spec/Handshake_Master_Spec_v02.182.md`, `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`, `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`, `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/main.rs`, `../handshake_main/src/backend/handshake_core/src/workflows.rs`, `../handshake_main/src/backend/handshake_core/src/storage/tests.rs`, and `../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs`.
- PATTERNS_EXTRACTED: Keep one explicit storage-mode contract; default self-hosted control-plane runtime to PostgreSQL; fail closed for control-plane writes when PostgreSQL is required but unavailable; keep SQLite as explicit cache/index/offline/demo surface rather than hidden runtime authority; split concurrency, worker, memory, workflow, DCC, and developer-test work into downstream candidate WPs owned by Orchestrator stub creation.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT PostgreSQL-primary authority for software-delivery control-plane runtime records; ADAPT existing dual-backend storage and test helpers into explicit mode/config and bootstrap proof; REJECT a silent `DATABASE_URL` absence fallback that makes SQLite the self-hosted control-plane authority after the pivot.
- LICENSE/IP_NOTES: NONE - this refinement uses internal Master Spec and local product-code evidence only; no third-party code or copyrighted implementation text is reused.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Spec enrichment has been applied. `SPEC_CURRENT` now resolves to v02.182, which contains the PostgreSQL-primary control-plane foundation law and Appendix 12 follow-through.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- Rule: if the WP is an internal repo-governed change or product-governance mirror patch already grounded in the current Master Spec plus local code/runtime truth, it is valid and often preferable to set `RESEARCH_CURRENCY_REQUIRED=NO`. Do not force unrelated or generic web research just to populate this section.
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is an internal product architecture pivot driven by the operator and current repo implementation state. The governing truth is the Master Spec, local product code, and the operator direction, not a time-sensitive external standard.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - The local repo already contains PostgreSQL storage support, migrations, dual-backend tests, and structured-collaboration parity work. The missing decision is normative authority and default runtime posture.
  - The safe shape is to land a narrow foundation and route every separable heavy implementation slice into explicit Orchestrator-owned follow-up WPs rather than enlarging the foundation WP.
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
- The foundation WP should not invent a full new event family.
- Bootstrap and storage-mode decisions must remain recorder-visible through existing startup, health, storage capability, workflow recovery, and DCC projection evidence where those surfaces already exist.
- Follow-up stubs must decide whether lease, queue worker, workflow durable execution, or FEMS memory jobs need dedicated event families. Those choices are out of scope for the foundation.

### RED_TEAM_ADVISORY (security failure modes)
- Silent fallback risk: if PostgreSQL is required but unavailable, falling back to SQLite can split-brain control-plane state and make operator-visible runtime truth false.
- Authority drift risk: DCC, Task Board, Role Mailbox, packet prose, or repo `.GOV` mirrors can look current while PostgreSQL runtime records disagree; views must remain projections with source/freshness labels.
- Concurrency risk: foundation scope can accidentally implement half-baked queue claims without real lease/backpressure law; leave worker claims and locking to the dedicated lease/backpressure and queue-worker stubs.
- Migration risk: moving defaults without a reproducible Postgres test/container matrix will make downstream failures environment-specific and hard to validate.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
  - PRIM-PostgresPrimaryControlPlane
  - PRIM-ControlPlaneStorageMode
  - PRIM-SqliteCacheOfflineBoundary
  - PRIM-ModelSession
  - PRIM-WorkflowRun
  - PRIM-MemoryPack
  - PRIM-LocusSyncTaskBoardParams
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-Database
  - PRIM-PostgresDatabase
  - PRIM-SqliteDatabase
  - PRIM-StorageTraits
  - PRIM-PostgresPrimaryControlPlane
  - PRIM-ControlPlaneStorageMode
  - PRIM-SqliteCacheOfflineBoundary
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - SPEC_CURRENT v02.182 contains the PostgreSQL-primary control-plane primitive vocabulary required by this foundation packet and its downstream split.
  - Queue worker, FEMS memory store, workflow durable execution, DCC projection, lease/backpressure, SQLite boundary, and developer-test setup primitives remain downstream implementation work.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: SPEC_CURRENT v02.182 already includes the PostgreSQL-primary control-plane primitive additions required by this refinement.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - No remaining primitive-index update is required for packet signature. v02.182 contains the foundation and downstream primitive vocabulary.
  - Downstream runtime domain primitives remain linked to candidate follow-up WP IDs so the foundation does not overclaim implementation coverage; Orchestrator owns stub-file creation and gov_kernel commit.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: Resolved by v02.182 spec enrichment; follow-up WP IDs remain in top-level STUB_WP_IDS for downstream split tracking.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: SPEC_CURRENT v02.182 already carries the feature-registry update for the PostgreSQL-primary control-plane foundation.
- UI_GUIDANCE_ACTION: NO_CHANGE
- UI_GUIDANCE_REASON: Foundation is backend/runtime configuration and authority law. DCC UI projection work is explicitly deferred to the Orchestrator-owned DCC follow-up WP.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: SPEC_CURRENT v02.182 already carries the interaction-matrix update for the PostgreSQL-primary control-plane foundation.
- APPENDIX_MAINTENANCE_NOTES:
  - Feature registry, primitive index, and interaction matrix updates have been applied in SPEC_CURRENT v02.182.
  - UI guidance remains with the DCC follow-up WP that Orchestrator owns.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene data path changes in this foundation | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no procedure-authoring surface changes | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physics or simulation measurements change | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation consumers remain downstream | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware execution surface changes | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: TOUCHED | NOTES: orchestration authority shifts toward PostgreSQL-primary runtime records; workflow execution details are stubbed | STUB_WP_IDS: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface changes | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative generation surface changes | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: no publishing/export flow changes | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe workflow surface changes | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food safety surface changes | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: TOUCHED | NOTES: queueing and backpressure become explicit downstream control-plane work | STUB_WP_IDS: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: authoritative runtime records move to PostgreSQL-primary posture | STUB_WP_IDS: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-FEMS-Postgres-Memory-Store-v1
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: DCC and Locus projection readers must label Postgres authority and cache/index boundaries | STUB_WP_IDS: WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: no analytics implementation lands in foundation | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset wrangling surface changes | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: foundation creates the PostgreSQL-primary storage-mode and default authority contract | STUB_WP_IDS: WP-1-Postgres-Dev-Test-Container-Matrix-v1
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: authority and fail-closed rules must be explicit before implementation | STUB_WP_IDS: WP-1-SQLite-Cache-Offline-Boundaries-v1
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutorial or explanation engine work | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: ModelSession, FEMS, workflow, and DCC context must resolve to one runtime authority | STUB_WP_IDS: WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-FEMS-Postgres-Memory-Store-v1
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: spec bump and migration/test sequencing are required | STUB_WP_IDS: WP-1-Postgres-Dev-Test-Container-Matrix-v1
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: TOUCHED | NOTES: developer container/test harness is downstream and must keep artifacts out of repo roots | STUB_WP_IDS: WP-1-Postgres-Dev-Test-Container-Matrix-v1
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: NEEDS_STUBS

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). Pillar feature definition and technical implementation MUST be derived from the current Master Spec. If unknown or underspecified, write UNKNOWN and create stubs or spec updates instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: foundation must preserve recorder-visible storage-mode and runtime-health evidence; no new event family in this WP | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: calendar storage remains outside the foundation pivot | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no editor surface changes | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor storage mode changes | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface changes | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus projection/runtime authority must align with PostgreSQL-primary; detailed parity is downstream | STUB_WP_IDS: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Loom graph/search portability is already separate | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: TOUCHED | NOTES: work-packet runtime truth must not be inferred from repo packet prose after the pivot | STUB_WP_IDS: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - PILLAR: Task board (product, not repo) | STATUS: TOUCHED | NOTES: board rows become Postgres-authority projections with freshness/source labels downstream | STUB_WP_IDS: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: micro-task execution depends on queue/lease and workflow durable execution follow-ups | STUB_WP_IDS: WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: DCC must project PostgreSQL runtime truth instead of local process or mirror state | STUB_WP_IDS: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: scheduler/workflow runtime authority is the central pivot pressure, including downstream FEMS memory jobs where they execute as durable runtime work | STUB_WP_IDS: WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-Postgres-Control-Plane-Leases-Backpressure-v1
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no prompt compiler change in foundation | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: foundation creates the explicit pivot law and default runtime/config posture | STUB_WP_IDS: WP-1-Postgres-Dev-Test-Container-Matrix-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: compact summaries and FEMS memory surfaces must expose authority/freshness without forcing long mirror reads | STUB_WP_IDS: WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-FEMS-Postgres-Memory-Store-v1
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no Stage runtime work | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no Studio runtime work | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no Atelier/Lens runtime work | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no distillation runtime work | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE policy change, though fail-closed storage behavior must not bypass existing gates | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval indexes remain outside foundation and align with SQLite/cache boundary follow-up | STUB_WP_IDS: WP-1-SQLite-Cache-Offline-Boundaries-v1
- PILLAR_ALIGNMENT_VERDICT: NEEDS_STUBS

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Derive pillar slices and subfeatures from the current Master Spec; do not invent pillar semantics from memory. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: PostgreSQL-primary control-plane foundation | SUBFEATURES: storage-mode config, default authority, fail-closed semantics, bootstrap health proof | PRIMITIVES_FEATURES: PRIM-Database, PRIM-PostgresDatabase, PRIM-SqliteDatabase, PRIM-StorageTraits, PostgresPrimaryControlPlane, ControlPlaneStorageMode | MECHANICAL: engine.dba, engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: core foundation scope
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: reproducible PostgreSQL developer/test matrix | SUBFEATURES: container service, migration reset, seeded fixtures, CI smoke profiles | PRIMITIVES_FEATURES: PRIM-PostgresDatabase, StorageModeFixtureMatrix | MECHANICAL: engine.dba, engine.sandbox, engine.version | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | NOTES: separable developer setup
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: shared control-plane leases and backpressure | SUBFEATURES: claims, lease expiry, heartbeat, retry, dead-letter, backpressure | PRIMITIVES_FEATURES: PRIM-WorkflowRun, ControlPlaneLease | MECHANICAL: engine.logistics, engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | NOTES: must not be half-implemented by foundation
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: ModelSession PostgreSQL queue workers | SUBFEATURES: model run queue, workers, persisted messages, checkpoints, cancellation, provider profile ids | PRIMITIVES_FEATURES: PRIM-ModelSession, ModelRunQueueWorker | MECHANICAL: engine.context, engine.director | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-ModelSession-Postgres-Queue-Workers-v1 | NOTES: downstream from lease primitives
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: workflow durable execution on PostgreSQL | SUBFEATURES: workflow instance state, node checkpoints, retry state, crash resume | PRIMITIVES_FEATURES: PRIM-WorkflowRun, WorkflowPostgresDurableExecution | MECHANICAL: engine.director, engine.archivist | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | NOTES: separate from storage-mode foundation
  - PILLAR: Command Center | CAPABILITY_SLICE: DCC Postgres control-plane projections | SUBFEATURES: sessions, queues, leases, workflows, memory jobs, dead-letter, source and freshness metadata | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, DccPostgresProjection | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: UI consumes, not foundation
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: SQLite cache/offline boundary | SUBFEATURES: cache/index/offline modes, fail-closed runtime writes, rebuildable projections, freshness metadata | PRIMITIVES_FEATURES: PRIM-SqliteDatabase, SqliteCacheOfflineBoundary | MECHANICAL: engine.sovereign, engine.librarian | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-SQLite-Cache-Offline-Boundaries-v1 | NOTES: prevents fallback split brain
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: storage authority evidence | SUBFEATURES: startup storage mode, health reason, fail-closed storage error, recovery visibility | PRIMITIVES_FEATURES: PRIM-Database, PRIM-PostgresDatabase | MECHANICAL: engine.archivist, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: foundation must make the authority decision recorder-visible without adding a new event family
  - PILLAR: Locus | CAPABILITY_SLICE: Postgres-authority work-state projection | SUBFEATURES: runtime authority source, freshness, workflow links, task-board sync posture | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, PRIM-WorkflowRun | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: detailed Locus projection parity belongs downstream
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: product work-packet runtime authority | SUBFEATURES: structured packet record, workflow binding, source label, mirror freshness | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-LocusSyncTaskBoardParams | MECHANICAL: engine.director, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: packet prose must not become runtime authority
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: task-board projection parity | SUBFEATURES: board row source, freshness, validation posture, queue status, reconciliation state | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, TaskBoardProjection | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: board markdown is a projection after the pivot
  - PILLAR: MicroTask | CAPABILITY_SLICE: micro-task queue occupancy | SUBFEATURES: model-session binding, retry state, lease posture, workflow node link | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-WorkflowRun, ModelRunQueueWorker | MECHANICAL: engine.logistics, engine.director | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-ModelSession-Postgres-Queue-Workers-v1 | NOTES: micro-task semantics depend on queue-worker and lease follow-ups
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: FEMS and compact control-plane summaries | SUBFEATURES: memory pack source, runtime freshness, compact status fields, stale-cache labels | PRIMITIVES_FEATURES: PRIM-MemoryPack, FemsPostgresMemoryStore, DccPostgresProjection | MECHANICAL: engine.context, engine.archivist | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-FEMS-Postgres-Memory-Store-v1 | NOTES: local-small-model reads need compact authoritative summaries
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: PostgreSQL-primary control-plane storage mode | JobModel: MECHANICAL_TOOL | Workflow: startup_storage_bootstrap | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: existing startup/runtime health evidence | Locus: PLANNED | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: foundation declares and proves the active storage authority
  - Capability: PostgreSQL developer/test matrix | JobModel: NONE | Workflow: test_harness | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | Notes: required before heavy follow-up validation
  - Capability: control-plane leases and backpressure | JobModel: WORKFLOW | Workflow: control_plane_queue_claims | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: planned by follow-up | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | Notes: shared concurrency foundation
  - Capability: ModelSession Postgres queue workers | JobModel: AI_JOB | Workflow: model_run | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing FR-EVT-SESS families plus follow-up proof | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-ModelSession-Postgres-Queue-Workers-v1 | Notes: move scheduler authority out of process-local state
  - Capability: FEMS Postgres memory store | JobModel: AI_JOB | Workflow: fems_memory_job | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing FEMS events plus follow-up proof | Locus: PLANNED | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-FEMS-Postgres-Memory-Store-v1 | Notes: shared memory authority
  - Capability: workflow durable execution on PostgreSQL | JobModel: WORKFLOW | Workflow: workflow_run | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing workflow evidence plus follow-up proof | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | Notes: crash-resume and node claim state
  - Capability: DCC Postgres projections | JobModel: UI_ACTION | Workflow: dcc_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: evidence refs only | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | Notes: operator projection surface
  - Capability: SQLite cache/offline boundary | JobModel: MECHANICAL_TOOL | Workflow: storage_mode_guard | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: health/degradation evidence | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-SQLite-Cache-Offline-Boundaries-v1 | Notes: explicit non-authority fallback
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: NEEDS_STUBS

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 45m
- MATRIX_SCAN_NOTES:
  - The pivot creates several high-ROI interactions that cannot be safely absorbed into the foundation packet.
  - The highest-risk combo is control-plane writes plus silent SQLite fallback; foundation must fail closed and the SQLite boundary stub must harden it.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - Edge: IMX-061 FEAT-WORKFLOW-ENGINE -> FEAT-STORAGE-PORTABILITY
  - Kind: workflow artifacts preserve portable manifests; extend with PostgreSQL-primary runtime authority notes
  - ROI: HIGH
  - Effort: MEDIUM
  - Spec refs: Storage portability, workflow execution authority
  - In-scope for this WP: YES
  - If NO: hand the candidate WP ID to Orchestrator for stub creation and gov_kernel commit.
  - Edge: IMX-107 FEAT-MODEL-SESSION-ORCHESTRATION -> FEAT-LOCUS-WORK-TRACKING
  - Kind: parallel sessions bind to work-packet and micro-task occupancy; extend with PostgreSQL queue-worker authority
  - ROI: HIGH
  - Effort: HIGH
  - Spec refs: ModelSession scheduler, Locus work tracking
  - In-scope for this WP: NO
  - If NO: hand the candidate WP ID to Orchestrator for stub creation and gov_kernel commit: WP-1-ModelSession-Postgres-Queue-Workers-v1
  - Edge: IMX-108 FEAT-DEV-COMMAND-CENTER -> FEAT-TASK-BOARD
  - Kind: DCC projects task-board authority and freshness; extend with PostgreSQL source labels
  - ROI: HIGH
  - Effort: MEDIUM
  - Spec refs: DCC control-plane projection, Task Board projection
  - In-scope for this WP: NO
  - If NO: hand the candidate WP ID to Orchestrator for stub creation and gov_kernel commit: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - Edge: IMX-109 FEAT-DEV-COMMAND-CENTER -> FEAT-WORK-PACKET-SYSTEM
  - Kind: DCC projects work-packet contract and activation; extend with PostgreSQL runtime authority
  - ROI: HIGH
  - Effort: MEDIUM
  - Spec refs: Work Packet System, DCC software-delivery runtime truth
  - In-scope for this WP: NO
  - If NO: hand the candidate WP ID to Orchestrator for stub creation and gov_kernel commit: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - Edge: IMX-110 FEAT-TASK-BOARD -> FEAT-LOCUS-WORK-TRACKING
  - Kind: human-readable planning mirror syncs from authoritative work tracking; extend with PostgreSQL-primary freshness semantics
  - ROI: HIGH
  - Effort: MEDIUM
  - Spec refs: Task Board mirror, Locus authority
  - In-scope for this WP: NO
  - If NO: hand the candidate WP ID to Orchestrator for stub creation and gov_kernel commit: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - Edge: IMX-111 FEAT-WORK-PACKET-SYSTEM -> FEAT-WORKFLOW-ENGINE
  - Kind: work-packet activation routes through workflow execution; extend with PostgreSQL durable execution handoff
  - ROI: HIGH
  - Effort: HIGH
  - Spec refs: Work Packet activation, workflow execution
  - In-scope for this WP: NO
  - If NO: hand the candidate WP ID to Orchestrator for stub creation and gov_kernel commit: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: No remaining new IMX edge update is pending after v02.182. SPEC_CURRENT now contains the PostgreSQL-primary control-plane interaction edges; seven candidate follow-up WP IDs still capture separable downstream implementation.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. For internal/product-governance mirror work, it is valid to mark this section `NOT_APPLICABLE` when no directly topical external combo research is needed. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This is an internal authority/default-runtime pivot. External combo research would not decide whether Handshake's current SQLite-default contract must change.
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
  - Combo: Postgres-primary storage mode plus fail-closed control-plane writes | Pillars: SQL to PostgreSQL shift readiness, Execution / Job Runtime | Mechanical: engine.dba, engine.sovereign | Primitives/Features: PRIM-PostgresDatabase, PRIM-StorageTraits, ControlPlaneStorageMode | Resolution: IN_THIS_WP | Stub: NONE | Notes: core foundation
  - Combo: Postgres test matrix plus every downstream runtime packet | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.sandbox, engine.version | Primitives/Features: PRIM-PostgresDatabase, StorageModeFixtureMatrix | Resolution: NEW_STUB | Stub: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | Notes: prevents repeated environment rediscovery
  - Combo: lease/backpressure plus ModelSession workflow and FEMS jobs | Pillars: Execution / Job Runtime | Mechanical: engine.logistics, engine.context | Primitives/Features: PRIM-WorkflowRun, PRIM-ModelSession, ControlPlaneLease, ModelRunQueueWorker, FemsPostgresMemoryStore | Resolution: NEW_STUB | Stub: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | Notes: shared concurrency gate
  - Combo: ModelSession queue workers plus DCC projection | Pillars: Execution / Job Runtime, Command Center | Mechanical: engine.director, engine.librarian | Primitives/Features: PRIM-ModelSession, DccPostgresProjection | Resolution: NEW_STUB | Stub: WP-1-ModelSession-Postgres-Queue-Workers-v1 | Notes: session authority before UI projection
  - Combo: FEMS memory store plus parallel ModelSession memory policy | Pillars: LLM-friendly data, Execution / Job Runtime | Mechanical: engine.context, engine.archivist | Primitives/Features: PRIM-MemoryPack, PRIM-ModelSession, FemsPostgresMemoryStore | Resolution: NEW_STUB | Stub: WP-1-FEMS-Postgres-Memory-Store-v1 | Notes: shared memory substrate
  - Combo: workflow durable execution plus leases and backpressure | Pillars: Execution / Job Runtime, Locus | Mechanical: engine.director, engine.logistics | Primitives/Features: PRIM-WorkflowRun, ControlPlaneLease | Resolution: NEW_STUB | Stub: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | Notes: durable run and node state
  - Combo: Postgres authority plus DCC runtime projection | Pillars: Command Center, LLM-friendly data | Mechanical: engine.librarian, engine.context | Primitives/Features: PRIM-LocusSyncTaskBoardParams, PRIM-PostgresDatabase, DccPostgresProjection | Resolution: NEW_STUB | Stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | Notes: operator visibility
  - Combo: SQLite cache boundary plus no split-brain fallback | Pillars: SQL to PostgreSQL shift readiness, RAG | Mechanical: engine.sovereign, engine.librarian | Primitives/Features: PRIM-SqliteDatabase, SqliteCacheOfflineBoundary | Resolution: NEW_STUB | Stub: WP-1-SQLite-Cache-Offline-Boundaries-v1 | Notes: local-first remains explicit
  - Combo: storage-mode health plus Flight Recorder evidence | Pillars: Flight Recorder | Mechanical: engine.archivist, engine.sovereign | Primitives/Features: PRIM-Database, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: startup and fail-closed authority decisions must be auditable
  - Combo: work-packet runtime authority plus DCC projection | Pillars: Work packets (product, not repo) | Mechanical: engine.director, engine.context | Primitives/Features: PRIM-WorkflowRun, PRIM-LocusSyncTaskBoardParams | Resolution: NEW_STUB | Stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | Notes: packet prose stays readable but non-authoritative
  - Combo: task-board freshness plus PostgreSQL authoritative records | Pillars: Task board (product, not repo) | Mechanical: engine.librarian, engine.context | Primitives/Features: PRIM-LocusSyncTaskBoardParams, TaskBoardProjection | Resolution: NEW_STUB | Stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | Notes: board rows need source and freshness projection
  - Combo: micro-task occupancy plus ModelSession queue workers | Pillars: MicroTask | Mechanical: engine.logistics, engine.director | Primitives/Features: PRIM-ModelSession, PRIM-WorkflowRun, ModelRunQueueWorker | Resolution: NEW_STUB | Stub: WP-1-ModelSession-Postgres-Queue-Workers-v1 | Notes: occupancy depends on durable session queues and lease semantics
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_REASON: The high-ROI combinations are real and now mapped to concrete downstream candidate follow-up WPs; foundation remains narrow and Orchestrator owns stub records.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: Current stubs, official packets, Task Board, WP Traceability Registry, Build Order, current Master Spec v02.182, and product storage/runtime code under `../handshake_main/src/backend/handshake_core`.
- MATCHED_STUBS:
  - Artifact: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: N/A | CodeReality: N/A | Resolution: NEW_STUB | Stub: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | Notes: downstream developer/test container setup
  - Artifact: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: NEW_STUB | Stub: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | Notes: downstream claim/lease/backpressure semantics
  - Artifact: WP-1-ModelSession-Postgres-Queue-Workers-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: NEW_STUB | Stub: WP-1-ModelSession-Postgres-Queue-Workers-v1 | Notes: downstream ModelSession queue-worker persistence
  - Artifact: WP-1-FEMS-Postgres-Memory-Store-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: NEW_STUB | Stub: WP-1-FEMS-Postgres-Memory-Store-v1 | Notes: downstream shared FEMS memory authority
  - Artifact: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: NEW_STUB | Stub: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | Notes: downstream workflow durable execution authority
  - Artifact: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: NEW_STUB | Stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | Notes: downstream operator projection layer
  - Artifact: WP-1-SQLite-Cache-Offline-Boundaries-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: NEW_STUB | Stub: WP-1-SQLite-Cache-Offline-Boundaries-v1 | Notes: downstream fallback/cache boundary
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: covers structured-collaboration parity, not the new default runtime/control-plane pivot.
  - Artifact: WP-1-Dev-Command-Center-Control-Plane-Backend-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: existing DCC backend is a prerequisite for Postgres-authority projection, not a duplicate.
  - Artifact: WP-1-ModelSession-Core-Scheduler-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: core scheduler exists but not Postgres-primary queue-worker authority.
  - Artifact: WP-1-Storage-Trait-Purity-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: removes storage trait leakage but does not change default control-plane authority.
  - Artifact: WP-1-Dual-Backend-Tests-v2 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: test posture exists but the pivot needs a concrete dev/container matrix.
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: NONE | Covers: primitive | Verdict: PARTIAL | Notes: `init_storage()` selects PostgreSQL from `DATABASE_URL` but defaults to SQLite when absent.
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: PostgreSQL backend and migrations exist, including structured collaboration support, but control-plane authority/default law is not defined.
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: `POSTGRES_TEST_URL` helpers exist; reproducible service setup remains a follow-up.
  - Path: ../handshake_main/src/backend/handshake_core/src/main.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: backend startup uses `storage::init_storage()` and can expose fail-closed storage-mode health after foundation.
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_STUBS
- EXISTING_CAPABILITY_ALIGNMENT_REASON: Existing completed packets are prerequisites, not duplicates. The seven downstream candidate follow-up WPs capture separable scope that should not enter the foundation WP; Orchestrator owns their stub files and gov_kernel commit.

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: Foundation has no direct GUI implementation. DCC projection and controls are downstream in WP-1-DCC-Postgres-Control-Plane-Projections-v1.
- GUI_REFERENCE_SCAN:
  - NONE
- HANDSHAKE_GUI_ADVICE:
  - NONE
- HIDDEN_GUI_REQUIREMENTS:
  - NONE
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - NONE
- GUI_IMPLEMENTATION_ADVICE_VERDICT: NOT_APPLICABLE

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This foundation WP changes backend/runtime authority and config. Operator-facing controls belong to the DCC Postgres projections follow-up.
- UI_SURFACES:
  - NONE in this WP
- UI_CONTROLS (buttons/dropdowns/inputs):
  - NONE in this WP
- UI_STATES (empty/loading/error):
  - PostgreSQL required but unavailable must be represented as a runtime health/degradation state for downstream DCC.
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Storage authority
  - PostgreSQL required
  - SQLite cache/offline
  - Fail-closed
- UI_ACCESSIBILITY_NOTES:
  - DCC follow-up tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
- UI_UX_VERDICT: OK

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: NO
- If YES: update the Roadmap (Spec 7.6) using the fixed per-phase fields below (do not invent new per-phase block types).
- Patch canonical roadmap sections in place. Do not create addendum-style normative text; use `[ADD v<version>]` markers for new lines/blocks.
- Per phase, include exactly:
  - Goal: N/A
  - MUST deliver: N/A
  - Key risks addressed in Phase n: N/A
  - Acceptance criteria: N/A
  - Explicitly OUT of scope: N/A
  - Mechanical Track: N/A
  - Atelier Track: N/A
  - Distillation Track: N/A
  - Vertical slice: N/A

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: ACTIVATION_MANAGER
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.182]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Abstraction-Layer, WP-1-Storage-Trait-Purity, WP-1-Dual-Backend-Tests, WP-1-Postgres-Structured-Collaboration-Artifact-Parity, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-ModelSession-Core-Scheduler
- BUILD_ORDER_BLOCKS: WP-1-Postgres-Dev-Test-Container-Matrix, WP-1-Postgres-Control-Plane-Leases-Backpressure, WP-1-ModelSession-Postgres-Queue-Workers, WP-1-FEMS-Postgres-Memory-Store, WP-1-Workflow-Engine-Postgres-Durable-Execution, WP-1-DCC-Postgres-Control-Plane-Projections, WP-1-SQLite-Cache-Offline-Boundaries
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.182.md 2.3.13.8 PostgreSQL-primary control-plane foundation plus v02.181 software-delivery control-plane projection law.
- WHAT: Implement the minimal PostgreSQL-primary control-plane foundation from SPEC_CURRENT v02.182: storage-mode/default configuration, explicit authority labels, fail-closed startup behavior, and bootstrap proof that self-hosted control-plane runtime state is not silently SQLite-primary.
- WHY: The operator wants Handshake to move to PostgreSQL now while the project is early. Without a foundation WP, each downstream runtime packet will make incompatible assumptions about default storage, fallback, concurrency, and projection authority.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-Postgres-Primary-Control-Plane-Foundation-v1.md
  - .GOV/spec/Handshake_Master_Spec_v02.182.md
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
  - ../handshake_main/src/backend/handshake_core/migrations/
- OUT_OF_SCOPE:
  - Implementing ModelSession PostgreSQL queue workers, worker claims, or message/checkpoint migration.
  - Implementing FEMS PostgreSQL memory store, bitemporal memory, memory poisoning controls, or memory dashboards.
  - Implementing full workflow-engine durable execution on PostgreSQL.
  - Implementing DCC visual projections or operator action mutations.
  - Implementing SQLite offline sync or cache invalidation beyond naming/fail-closed boundaries.
  - Implementing a full PostgreSQL dev/test container matrix in this foundation WP.
- TEST_PLAN:
  ```bash
  rg -n "init_storage|DATABASE_URL|StorageBackendKind|PostgresDatabase|SqliteDatabase|POSTGRES_TEST_URL|run_migrations" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_defaults_to_postgres_primary_when_required -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_fails_closed_when_postgres_required_without_url -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_cache_mode_is_not_control_plane_authority -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml database_trait_purity_capability_snapshot_reports_postgres -- --exact
  just gov-check
  ```
- ACTIVATION_VERIFICATION_WAIVER:
  - OPERATOR_UPDATE_DATE: 2026-05-05
  - REASON: Host is under heavy load and near OOM risk.
  - WAIVER: Do not run `cargo build`, `cargo check`, `cargo test`, `cargo clippy`, or any Just recipe likely to compile Rust during this activation.
  - ACTIVATION_ALLOWED_VERIFICATION: lightweight governance/metadata checks only.
  - CARGO_TRIPWIRES_STATUS: DECLARED_FOR_SEMANTIC_CLOSURE_BUT_NOT_RUN_UNDER_THIS_ACTIVATION_WAIVER.
- DONE_MEANS:
  - Master Spec enrichment for PostgreSQL-primary control-plane authority is approved and `SPEC_CURRENT` resolves to v02.182 before packet coding begins.
  - Runtime configuration can declare `postgres_primary`, `sqlite_cache`, `sqlite_offline`, or equivalent explicit storage mode without relying on ambient defaults.
  - When PostgreSQL-primary is required and no valid PostgreSQL URL/service is available, control-plane startup or control-plane writes fail closed with a clear storage-mode error.
  - SQLite remains explicitly cache/index/offline/demo scoped and cannot silently receive authoritative control-plane writes.
  - Downstream candidate follow-up IDs for queues, leases/backpressure, FEMS, workflow, DCC projection, SQLite boundaries, and test container matrix are listed in packet `STUB_WP_IDS`; Orchestrator-owned stub records are not edited by Activation Manager.
- PRIMITIVES_EXPOSED:
  - PRIM-Database
  - PRIM-PostgresDatabase
  - PRIM-SqliteDatabase
  - PRIM-StorageTraits
  - PRIM-PostgresPrimaryControlPlane
  - PRIM-ControlPlaneStorageMode
  - PRIM-SqliteCacheOfflineBoundary
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/refinements/WP-1-Postgres-Primary-Control-Plane-Foundation-v1.md
  - .GOV/spec/Handshake_Master_Spec_v02.182.md
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - init_storage
  - DATABASE_URL
  - POSTGRES_TEST_URL
  - StorageBackendKind
  - PostgresDatabase
  - SqliteDatabase
  - run_migrations
  - supports_structured_collab_artifacts
  - SessionSchedulerConfig
  - backpressure posture
  - control-plane health
- RUN_COMMANDS:
  ```bash
  rg -n "init_storage|DATABASE_URL|POSTGRES_TEST_URL|StorageBackendKind|PostgresDatabase|SqliteDatabase|run_migrations|SessionSchedulerConfig|backpressure|control-plane" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_defaults_to_postgres_primary_when_required -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_fails_closed_when_postgres_required_without_url -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_cache_mode_is_not_control_plane_authority -- --exact
  just gov-check
  ```
- RUN_COMMANDS_ACTIVATION_WAIVER_NOTE: For this activation, run only the `rg` and lightweight governance/metadata portions. The cargo commands remain required semantic tripwires for later closure but are explicitly waived/not run while the 2026-05-05 operator host-load waiver is active.
- RISK_MAP:
  - "Foundation widens into all downstream runtime work" -> "Packet becomes too large and mixes authority definition with queue/FEMS/workflow/DCC implementation."
  - "PostgreSQL required but unavailable silently falls back to SQLite" -> "Control-plane split brain and false operator state."
  - "Spec enrichment skipped" -> "Coder implements against operator intent while current Master Spec still says PostgreSQL is future/Phase 2."
  - "Test container setup deferred without an Orchestrator-owned follow-up WP" -> "Every downstream Postgres packet invents a different service/bootstrap contract."
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - Per operator override, Activation Manager does not edit follow-up stub files, Task Board, WP Traceability, or Build Order. Candidate follow-up IDs remain in this refinement for Orchestrator stub creation and packet-prep dependency mapping.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Proposed [ADD v02.182] PostgreSQL-primary control-plane authority | WHY_IN_SCOPE: This is the operator-directed pivot and the current spec does not yet clearly cover it | EXPECTED_CODE_SURFACES: storage/mod.rs, main.rs, storage/tests.rs | EXPECTED_TESTS: storage_mode_defaults_to_postgres_primary_when_required | RISK_IF_MISSED: Runtime defaults remain SQLite-primary by accident.
  - CLAUSE: Proposed fail-closed control-plane storage mode | WHY_IN_SCOPE: Silent fallback is the main foundation safety risk | EXPECTED_CODE_SURFACES: storage/mod.rs, main.rs | EXPECTED_TESTS: storage_mode_fails_closed_when_postgres_required_without_url | RISK_IF_MISSED: PostgreSQL outages create hidden split brain.
  - CLAUSE: Proposed SQLite cache/offline boundary | WHY_IN_SCOPE: Local-first ergonomics must remain explicit without giving SQLite hidden authority | EXPECTED_CODE_SURFACES: storage/mod.rs, storage/tests.rs | EXPECTED_TESTS: sqlite_cache_mode_is_not_control_plane_authority | RISK_IF_MISSED: Cache/offline mode becomes undeclared runtime authority.
  - CLAUSE: Current storage portability and dual-backend testing law | WHY_IN_SCOPE: Foundation builds on existing database abstraction and test helpers | EXPECTED_CODE_SURFACES: storage/mod.rs, storage/postgres.rs, storage/tests.rs | EXPECTED_TESTS: database_trait_purity_capability_snapshot_reports_postgres | RISK_IF_MISSED: Pivot bypasses established portability discipline.

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: Control-plane storage-mode config | PRODUCER: backend startup/env resolver | CONSUMER: storage init, health endpoint, DCC projections, tests | SERIALIZER_TRANSPORT: environment/config struct and health payload | VALIDATOR_READER: storage-mode tests | TRIPWIRE_TESTS: storage_mode_defaults_to_postgres_primary_when_required | DRIFT_RISK: Defaults differ between startup, tests, and operator docs.
  - CONTRACT: PostgreSQL-required fail-closed error | PRODUCER: storage init and control-plane write guards | CONSUMER: backend startup, DCC health, validators | SERIALIZER_TRANSPORT: structured StorageError or health/degradation payload | VALIDATOR_READER: fail-closed tests | TRIPWIRE_TESTS: storage_mode_fails_closed_when_postgres_required_without_url | DRIFT_RISK: Missing URL/service becomes SQLite fallback.
  - CONTRACT: SQLite cache/offline authority label | PRODUCER: storage mode resolver and cache/index layers | CONSUMER: DCC, tests, downstream SQLite boundary packet | SERIALIZER_TRANSPORT: storage capability snapshot and health payload | VALIDATOR_READER: SQLite boundary tests | TRIPWIRE_TESTS: sqlite_cache_mode_is_not_control_plane_authority | DRIFT_RISK: Cache projection is mistaken for source of truth.
  - CONTRACT: Candidate follow-up manifest | PRODUCER: refinement and packet `STUB_WP_IDS` | CONSUMER: Orchestrator, coder, validator, Build Order | SERIALIZER_TRANSPORT: comma-separated WP IDs plus Orchestrator-owned registry/build-order projections | VALIDATOR_READER: pre-work/coder checks | TRIPWIRE_TESTS: just gov-check | DRIFT_RISK: downstream work disappears from activation scope.

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_defaults_to_postgres_primary_when_required -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_fails_closed_when_postgres_required_without_url -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_cache_mode_is_not_control_plane_authority -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml database_trait_purity_capability_snapshot_reports_postgres -- --exact`
- SEMANTIC_TRIPWIRE_ACTIVATION_WAIVER_NOTE: These cargo tripwires are not run during this activation under the 2026-05-05 operator host-load waiver. Later closure must either run them when host capacity permits or carry an explicit active waiver in the packet/validator report.
- CANONICAL_CONTRACT_EXAMPLES:
  - Example `postgres_primary` runtime config with PostgreSQL URL present and startup succeeding.
  - Example `postgres_primary` runtime config with no PostgreSQL URL/service and startup failing closed.
  - Example `sqlite_cache` or `sqlite_offline` mode declaring non-authoritative runtime posture and source/freshness metadata.
  - Example packet `STUB_WP_IDS` manifest listing the seven downstream PostgreSQL pivot slices as Orchestrator-owned follow-up WPs.

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Confirm approved spec enrichment has advanced `SPEC_CURRENT` before product-code implementation.
  - Inspect `storage::init_storage()`, `StorageBackendKind`, `StorageCapabilitySnapshot`, `PostgresDatabase::connect`, and current `POSTGRES_TEST_URL` helpers.
  - Add the smallest explicit storage-mode resolver needed for PostgreSQL-primary, SQLite cache, SQLite offline, and test modes.
  - Change self-hosted control-plane default behavior only through that resolver; do not hardcode ad hoc checks in callers.
  - Add fail-closed tests for missing PostgreSQL when required and explicit non-authority tests for SQLite cache/offline mode.
  - Leave queue workers, leases/backpressure, FEMS, workflow durable execution, DCC projections, and dev/test containers to the linked stubs.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
- TRIPWIRE_TESTS:
  - storage_mode_defaults_to_postgres_primary_when_required
  - storage_mode_fails_closed_when_postgres_required_without_url
  - sqlite_cache_mode_is_not_control_plane_authority
  - database_trait_purity_capability_snapshot_reports_postgres
- CARRY_FORWARD_WARNINGS:
  - Do not silently fall back to SQLite for authoritative control-plane writes.
  - Do not implement queue leases, worker claims, FEMS storage, workflow durable execution, or DCC projections inside the foundation patch.
  - Do not use repo `.GOV` files, packet prose, or mailbox chronology as product runtime authority.
  - Keep provider/model profiles as declared catalog IDs in downstream work, not ambient CLI aliases.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - Approved [ADD v02.182] PostgreSQL-primary control-plane authority.
  - Approved fail-closed PostgreSQL-required storage mode.
  - Approved SQLite cache/offline non-authority boundary.
  - Current v02.182 storage portability and dual-backend testing law.
- FILES_TO_READ:
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/spec/Handshake_Master_Spec_v02.182.md
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
- COMMANDS_TO_RUN:
  - `rg -n "init_storage|DATABASE_URL|POSTGRES_TEST_URL|StorageBackendKind|PostgresDatabase|SqliteDatabase|run_migrations|storage_mode|postgres_primary|sqlite_cache|sqlite_offline" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_defaults_to_postgres_primary_when_required -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_fails_closed_when_postgres_required_without_url -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_cache_mode_is_not_control_plane_authority -- --exact`
  - `just gov-check`
- COMMANDS_TO_RUN_ACTIVATION_WAIVER_NOTE: While the 2026-05-05 host-load waiver is active, validator/coder activation checks must not run the listed cargo commands or Rust-compiling Just recipes; use only lightweight governance/metadata checks unless the Operator explicitly supersedes the waiver.
- POST_MERGE_SPOTCHECKS:
  - Confirm `STUB_WP_IDS` in the packet still lists all seven downstream slices.
  - Confirm no product code in the foundation implements the downstream worker/lease/FEMS/workflow/DCC scopes.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Exact storage-mode environment variable names and health payload field names must be chosen during coding.
  - Whether PostgreSQL developer setup uses Docker, Podman, or another local service is downstream Orchestrator-owned follow-up work.
  - Whether lease primitives should use row locks, advisory locks, or compare-and-swap is downstream Orchestrator-owned follow-up work.
  - Whether FEMS memory embeddings use pgvector or remain separate cache/index artifacts is downstream Orchestrator-owned follow-up work.

### FEATURE_DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: PRIM-PostgresPrimaryControlPlane, PRIM-ControlPlaneStorageMode, PRIM-SqliteCacheOfflineBoundary, PRIM-ControlPlaneLease, PRIM-ModelRunQueueWorker, PRIM-FemsPostgresMemoryStore, PRIM-WorkflowPostgresDurableExecution, PRIM-DccPostgresProjection
- DISCOVERY_STUBS: ASSESSMENT_ONLY_CANDIDATES_FOR_ORCHESTRATOR_CREATION: WP-1-Postgres-Dev-Test-Container-Matrix-v1, WP-1-Postgres-Control-Plane-Leases-Backpressure-v1, WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-FEMS-Postgres-Memory-Store-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
- DISCOVERY_MATRIX_EDGES: IMX-133, IMX-134, IMX-135
- DISCOVERY_UI_CONTROLS: DCC follow-up should expose storage authority, source/freshness, queue depth, lease state, stalled/dead-letter state, and disabled action reasons.
- DISCOVERY_SPEC_ENRICHMENT: APPLIED - SPEC_CURRENT v02.182 now contains the PostgreSQL-primary control-plane foundation and Appendix 12 follow-through.
- DISCOVERY_JUSTIFICATION: The operator-directed pivot intentionally yielded downstream candidate follow-up WPs and applied spec enrichment; Orchestrator owns stub-file creation and zero-discovery does not apply.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: SPEC_CURRENT v02.182 explicitly names the PostgreSQL-primary control-plane foundation, storage-mode contract, fail-closed behavior, SQLite authority boundary, and downstream activation split.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: The prior Phase 1 SQLite/future PostgreSQL ambiguity is resolved by v02.182 for this foundation packet.

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Applied before signature. SPEC_CURRENT now resolves to v02.182 and contains the PostgreSQL-primary control-plane foundation plus Appendix 12 primitive/interaction follow-through required for this packet.

#### APPLIED_SPEC_ENRICHMENT_REFERENCE (HISTORICAL; SPEC_CURRENT v02.182 NOW INCLUDES THIS TEXT)
```md
#### 2.3.13.8 PostgreSQL-Primary Control-Plane Foundation [ADD v02.182]

Handshake now treats PostgreSQL as the primary runtime authority for the self-hosted control plane. This is an early-development pivot over the earlier "PostgreSQL later" posture; it does not remove local-first ergonomics, but it does make storage authority explicit.

Normative requirements:

- The self-hosted control plane MUST support an explicit storage mode contract with at least these semantic modes: `postgres_primary`, `sqlite_cache`, `sqlite_offline`, and `test`.
- In `postgres_primary`, authoritative control-plane records MUST be written to PostgreSQL. This includes software-delivery runtime truth, ModelSession scheduling/queue state, workflow durable execution state, Locus work-tracking projection authority, Role Mailbox linked-control-plane state, Dev Command Center runtime projections, FEMS memory authority, and governed action/claim/lease/backpressure records as those feature slices are implemented.
- If `postgres_primary` is required and no valid PostgreSQL connection is available, startup or the attempted authoritative control-plane write MUST fail closed. The runtime MUST NOT silently fall back to SQLite for authoritative control-plane writes.
- SQLite remains valid for local search indexes, rebuildable projections, cache data, explicit offline/demo modes, and tests that declare SQLite authority. SQLite-backed surfaces MUST expose source/freshness/authority metadata when they are derived from PostgreSQL or another authority.
- The foundation slice for `WP-1-Postgres-Primary-Control-Plane-Foundation-v1` is limited to the spec law, storage-mode/default configuration, fail-closed behavior, bootstrap/health proof, and downstream candidate-WP handoff. It MUST NOT implement the full queue-worker, lease/backpressure, FEMS memory-store, workflow durable-execution, DCC projection, SQLite fallback, or developer-container matrix surfaces.

Downstream activation split:

- `WP-1-Postgres-Dev-Test-Container-Matrix-v1` owns reproducible PostgreSQL service startup, reset, fixtures, migration smoke profiles, and CI-ready test matrix behavior.
- `WP-1-Postgres-Control-Plane-Leases-Backpressure-v1` owns shared claim, lease, heartbeat, retry, dead-letter, and backpressure primitives.
- `WP-1-ModelSession-Postgres-Queue-Workers-v1` owns PostgreSQL authoritative ModelSession queue workers, persisted messages, checkpoints, cancellation, and provider-profile persistence.
- `WP-1-FEMS-Postgres-Memory-Store-v1` owns PostgreSQL authoritative FEMS memory records, memory packs, memory jobs, replay metadata, and parallel-safe memory writes.
- `WP-1-Workflow-Engine-Postgres-Durable-Execution-v1` owns PostgreSQL workflow instance state, node execution checkpoints, retries, terminal outcomes, and crash-resume semantics.
- `WP-1-DCC-Postgres-Control-Plane-Projections-v1` owns Dev Command Center projections over PostgreSQL runtime truth for sessions, queues, leases, workflows, memory jobs, and dead-letter states.
- `WP-1-SQLite-Cache-Offline-Boundaries-v1` owns the explicit boundary between PostgreSQL authority and SQLite cache, index, offline, demo, or rebuildable projection usage.

Appendix 12.4 update:

- Add `PRIM-PostgresPrimaryControlPlane`: the product-owned runtime authority surface for self-hosted control-plane records backed by PostgreSQL.
- Add `PRIM-ControlPlaneStorageMode`: the explicit storage-mode primitive that decides whether a runtime surface is PostgreSQL-authoritative, SQLite-cache, SQLite-offline, or test-scoped.
- Add `PRIM-SqliteCacheOfflineBoundary`: the primitive that prevents SQLite cache/index/offline surfaces from becoming hidden control-plane authority.
- Add `PRIM-ControlPlaneLease`, `PRIM-ModelRunQueueWorker`, `PRIM-FemsPostgresMemoryStore`, `PRIM-WorkflowPostgresDurableExecution`, and `PRIM-DccPostgresProjection` as downstream primitive vocabulary linked to the candidate follow-up WPs.

Appendix 12.6 update:

- Add `IMX-133`: `FEAT-STORAGE-PORTABILITY -> FEAT-WORKFLOW-ENGINE`, kind `postgres_primary_storage_mode_bounds_workflow_runtime_authority`, ROI `HIGH`.
- Add `IMX-134`: `FEAT-STORAGE-PORTABILITY -> FEAT-MODEL-SESSION-ORCHESTRATION`, kind `postgres_primary_storage_mode_bounds_model_session_queue_authority`, ROI `HIGH`.
- Add `IMX-135`: `FEAT-STORAGE-PORTABILITY -> FEAT-DEV-COMMAND-CENTER`, kind `postgres_primary_authority_projects_into_dcc_with_source_freshness`, ROI `HIGH`.
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 2.3.13.8 PostgreSQL-Primary Control-Plane Foundation [ADD v02.182]
- CONTEXT_START_LINE: 3623
- CONTEXT_END_LINE: 3646
- CONTEXT_TOKEN: postgres_primary
- EXCERPT_ASCII_ESCAPED:
  ```text
  Handshake now treats PostgreSQL as the primary runtime authority for the self-hosted control plane.
  The self-hosted control plane MUST support an explicit storage mode contract with semantic modes including postgres_primary, sqlite_cache, sqlite_offline, and test.
  If postgres_primary is required and no valid PostgreSQL connection is available, startup or the attempted authoritative control-plane write MUST fail closed.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 2.3.15 Locus Work Tracking System storage and projection posture
- CONTEXT_START_LINE: 6680
- CONTEXT_END_LINE: 7090
- CONTEXT_TOKEN: backpressure posture
- EXCERPT_ASCII_ESCAPED:
  ```text
  Software-delivery control-plane state should preserve health posture, stale-detection posture, backpressure posture, and operator-alert posture by stable runtime identifiers.
  Under load or blocked authority, the system MUST surface backpressure explicitly instead of silently dropping or reordering control-plane intent.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 4.3.9.12-4.3.9.13 ModelSession and Session Scheduler
- CONTEXT_START_LINE: 32440
- CONTEXT_END_LINE: 32500
- CONTEXT_TOKEN: INV-SCHED-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  The Session Scheduler introduces job_kind = "model_run" into the AI Job Model.
  INV-SCHED-001: All model invocations in RuntimeMode=AI_ENABLED MUST be routed through the Session Scheduler.
  This foundation does not reimplement the scheduler; it sets the PostgreSQL-primary runtime authority boundary for downstream queue-worker work.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 2.6.6.7.6.2 FEMS and ModelSession memory policy
- CONTEXT_START_LINE: 11950
- CONTEXT_END_LINE: 12005
- CONTEXT_TOKEN: ModelSession.memory_policy
- EXCERPT_ASCII_ESCAPED:
  ```text
  FEMS defines read, write, validation, consolidation, and MemoryPack behavior.
  The spec also references ModelSession.memory_policy as the integration point.
  PostgreSQL-primary memory storage is a downstream follow-up WP because the foundation only establishes runtime authority and storage-mode law.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 2.3.5.6 The Role of SQLite
- CONTEXT_START_LINE: 2247
- CONTEXT_END_LINE: 2264
- CONTEXT_TOKEN: SQLite is used for
- EXCERPT_ASCII_ESCAPED:
  ```text
  Important: SQLite is used for indexing, not as the primary data store.
  The pivot preserves this intent by making SQLite cache/index/offline authority explicit instead of allowing silent control-plane fallback.
  ```
