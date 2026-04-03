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
- WP_ID: WP-1-Storage-Trait-Purity-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-03T00:30:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- SPEC_TARGET_SHA1: 747a1e77cbe2e1c564d1a99d5c39265edc6aeca2
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja030420260212
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Storage-Trait-Purity-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- The `Database` trait still exposes `fn as_any(&self) -> &dyn Any` in `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`, which preserves a production downcast escape hatch even though current spec law forbids backend-specific type exposure at the trait boundary.
- Production code still branches on concrete backend types in `../handshake_main/src/backend/handshake_core/src/workflows.rs`, `../handshake_main/src/backend/handshake_core/src/api/loom.rs`, `../handshake_main/src/backend/handshake_core/src/storage/retention.rs`, and `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs`.
- SQLite-only behavior is therefore still hidden as implementation trivia instead of being expressed as explicit backend identity or capability semantics, which weakens PostgreSQL readiness and makes portability auditing harder than the current Main Body allows.
- Dual-backend tests exist in principle, but this packet still needs targeted tripwires that fail if production paths reintroduce downcasts or implicit SQLite-only behavior.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 45m
- SEARCH_SCOPE: current Master Spec v02.179 storage-portability clauses, current local product code under `../handshake_main/src/backend/handshake_core`, current stub backlog, and related validated storage packets
- REFERENCES: `.GOV/spec/Handshake_Master_Spec_v02.179.md`, `.GOV/task_packets/stubs/WP-1-Storage-Trait-Purity-v1.md`, `.GOV/task_packets/stubs/WP-1-Storage-No-Runtime-DDL-v1.md`, `.GOV/task_packets/WP-1-Storage-Abstraction-Layer-v3.md`, `.GOV/task_packets/WP-1-Loom-Storage-Portability-v4/packet.md`, `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`, `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`, `../handshake_main/src/backend/handshake_core/src/storage/retention.rs`, `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/workflows.rs`, `../handshake_main/src/backend/handshake_core/src/api/loom.rs`, and `../handshake_main/src/backend/handshake_core/src/storage/tests.rs`
- PATTERNS_EXTRACTED: keep one trait object boundary with explicit backend identity or capability methods; express unsupported backend behavior as explicit capability denial or terminal error rather than hidden concrete-type branching; keep direct pool access inside backend implementations; prove portability with dual-backend tripwires instead of social assumptions
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT explicit backend identity or capability query methods on `Database`; ADAPT existing SQLite-only seams into explicit unsupported-feature or capability-gated paths instead of downcasts; REJECT keeping `as_any` as a production trait escape hatch
- LICENSE/IP_NOTES: internal code and governance review only; no third-party code or copyrighted text is intended for direct reuse
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.179.md already states the One Storage API law, the Trait Purity Invariant, and the dual-backend testing requirement. This WP is implementation remediation against current Main Body truth.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP is strictly internal and mechanical. The governing requirement is the current local Master Spec plus current product-code reality, not a time-sensitive external market or vendor behavior.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Current-spec storage-portability law is already stronger than the current product implementation.
  - The missing work is removal of production downcast seams and replacement with explicit backend identity or capability contracts.
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
- Existing `LoomSearchExecuted` telemetry must keep its current semantics if backend tier or backend identity remains visible in payloads, but that value must come from an explicit `Database` capability or backend-kind query rather than a concrete-type downcast.
- This packet is boundary hardening, not telemetry taxonomy expansion.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: if `as_any` remains available on the production `Database` trait, future call sites can silently bypass the storage abstraction again. Mitigation: remove or isolate the escape hatch from production code paths and add static or grep-style tripwires.
- Risk: if SQLite-only behavior is converted into silent no-op behavior instead of explicit capability denial or terminal error, Postgres readiness will be overstated. Mitigation: unsupported paths must fail explicitly and deterministically.
- Risk: if Loom search or structured-artifact emission changes semantics while removing downcasts, operator evidence and runtime behavior can drift without notice. Mitigation: preserve existing behavior through explicit capability methods and regression tests.
- Risk: if direct pool access leaks farther outside storage implementations during the refactor, the packet can appear to improve trait purity while actually worsening storage-boundary sprawl. Mitigation: keep raw pool access confined to backend implementations only.

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
  - The spec already defines the trait-based storage abstraction and backend implementations as primitive surfaces.
  - The gap is implementation conformance, not a new primitive family.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already names the database and storage-traits primitives this packet uses.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Keep Appendix 12.4 unchanged and implement against existing storage primitive law.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new primitive family was discovered during this remediation.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: This packet closes storage-boundary conformance and does not introduce a new feature family.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface is implemented in this packet.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: This packet repairs storage-boundary execution semantics within existing features and does not add a new cross-feature interaction class.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep the current spec unchanged and implement against existing Main Body law.
  - If coding reveals a truly missing primitive or interaction edge, treat that as a separate spec-update flow instead of silently widening this packet.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is changed by storage trait remediation | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no simulation or measurement logic is involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers only | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing execution surface changes here | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration consumes the storage boundary but is not the implementation focus of this packet | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication or export controllers remain downstream consumers | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: artifact retention and evidence still rely on storage, but this packet does not add new archivist behavior | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval remains downstream of the storage boundary work | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytics surfaces consume stored data later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: this packet directly hardens the database abstraction boundary and explicit backend capability posture | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this packet implements existing law and does not add new governance authority | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or explanation interface is implemented here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: explicit backend capability vocabulary keeps downstream consumers from depending on hidden concrete-type knowledge | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: no versioned artifact or migration-law expansion is introduced here | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: existing Loom search telemetry depends on backend classification and must stay stable without downcasts | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: no calendar-facing surface depends directly on this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: current Locus SQLite helpers downcast the trait object and need explicit backend capability semantics instead | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: TOUCHED | NOTES: Loom search currently derives tier information via concrete backend inspection | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no product work-packet feature contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task-board-specific feature contract is modified | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: no micro-task feature contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: no new operator surface is implemented | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: workflow-side structured-artifact emission currently depends on SQLite downcasting and must move to explicit capability law | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no prompt or spec-router surface is altered | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: this packet directly hardens the portability boundary and reduces hidden SQLite assumptions | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: NOT_TOUCHED | NOTES: no new data shape or parser contract is introduced | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no stage workflow surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio runtime or creative console behavior is touched | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design or capture surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no training or distillation pipeline depends directly on this work | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE execution surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers of the storage boundary | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: backend-neutral Loom search telemetry classification | SUBFEATURES: `tier_used` meaning, backend-kind exposure, and recorder payload stability without concrete-type checks | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LoomStorage | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: touched because current telemetry branches on concrete backend type
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: explicit backend identity and capability query surface | SUBFEATURES: backend_kind, supports(feature), removal of production downcasts | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits, PRIM-SqliteDatabase, PRIM-PostgresDatabase | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the core portability boundary the spec already requires
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: structured-artifact emission capability gate | SUBFEATURES: workflow-side artifact emission path stops assuming SQLite storage | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: runtime behavior must stay explicit when a backend lacks a SQLite-only path
  - PILLAR: Loom | CAPABILITY_SLICE: backend-tier classification without concrete-type branching | SUBFEATURES: Loom search tier metadata and any backend-sensitive search path logic | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LoomStorage | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: keep current search semantics while removing hidden backend inspection
  - PILLAR: Locus | CAPABILITY_SLICE: SQLite-only locus path expressed as explicit capability | SUBFEATURES: locus operation dispatch, readiness queries, task-board sync helpers | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LocusQueryReadyParams, PRIM-LocusGetWpStatusParams, PRIM-LocusSyncTaskBoardParams | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: if PostgreSQL does not support a path yet, that must be explicit and testable
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: explicit backend identity and capability queries on `Database` | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: internal runtime contract used by multiple backend consumers
  - Capability: structured collaboration artifact emission backend gate | JobModel: WORKFLOW | Workflow: structured_collaboration_artifact_emission | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: unsupported backend behavior must become explicit instead of requiring a SQLite downcast
  - Capability: Loom search backend tier classification | JobModel: UI_ACTION | Workflow: NONE | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: LoomSearchExecuted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: preserve current telemetry semantics while removing concrete-type branching
  - Capability: Locus SQLite-only operation routing via explicit backend feature gate | JobModel: WORKFLOW | Workflow: locus_operation_dispatch | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: make unsupported backend status explicit and deterministic
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - This packet hardens existing primitive relationships rather than creating a new cross-feature interaction class.
  - No appendix interaction-matrix expansion is required if the work remains limited to storage-boundary conformance.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Current matrix coverage is sufficient; the packet closes implementation drift inside existing storage and feature seams.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This packet is strictly internal and mechanical. External combo research is not needed to decide whether production storage code may downcast a trait object.
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
  - Combo: `Database` trait plus Locus operation routing | Pillars: Locus, SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-LocusQueryReadyParams, PRIM-LocusGetWpStatusParams, PRIM-LocusSyncTaskBoardParams | Resolution: IN_THIS_WP | Stub: NONE | Notes: explicit backend capability law is the right place to close this drift
  - Combo: `Database` trait plus Loom search telemetry | Pillars: Loom, Flight Recorder | Mechanical: engine.context | Primitives/Features: PRIM-Database, PRIM-LoomStorage | Resolution: IN_THIS_WP | Stub: NONE | Notes: keep event semantics while removing concrete backend inspection
  - Combo: `Database` trait plus workflow-side structured artifact emission | Pillars: Execution / Job Runtime, SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: the packet should make unsupported backend behavior explicit instead of hidden behind SQLite-only helper access
  - Combo: `Database` trait plus retention pin updates | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-RetentionPolicy, PRIM-RetentionReport | Resolution: IN_THIS_WP | Stub: NONE | Notes: retention updates still use a SQLite downcast and need the same explicit capability posture
  - Combo: `Database` trait plus dual-backend conformance tripwires | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.version | Primitives/Features: PRIM-Database, PRIM-StorageTraits, PRIM-SqliteDatabase, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: test posture is part of the spec-owned portability contract, not optional hygiene
  - Combo: `Database` trait plus unsupported-backend failure semantics | Pillars: Execution / Job Runtime, Locus | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-LocusQueryReadyParams, PRIM-LocusGetWpStatusParams | Resolution: IN_THIS_WP | Stub: NONE | Notes: unsupported PostgreSQL paths must fail explicitly and deterministically rather than through missing downcast branches
  - Combo: `Database` trait plus downstream Stage/media portability tracks | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: this packet is a backend prerequisite for later Stage/media portability work and must leave an explicit capability surface instead of hidden SQLite assumptions
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered here belong inside this activation of the existing stub and do not require a new stub or spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed storage packets, current Master Spec v02.179, and local product code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Storage-Trait-Purity-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this is the correct governed shell for the remaining trait-purity gap
  - Artifact: WP-1-Storage-No-Runtime-DDL-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: related storage remediation, but focused on runtime DDL rather than downcast escape hatches
  - Artifact: WP-1-Stage-Media-Artifact-Portability-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: that downstream portability track depends on this packet but should not absorb the storage-boundary remediation itself
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Storage-Abstraction-Layer-v3 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established the storage layer, but current code still leaves production downcast seams
  - Artifact: WP-1-Loom-Storage-Portability-v2 | BoardStatus: SUPERSEDED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Loom portability relies on the same backend boundary, but its packet family does not close generic trait purity across the codebase
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Storage-Trait-Purity-v1 | Covers: primitive | Verdict: PARTIAL | Notes: `Database` still exposes `as_any`, which enables backend downcasts in production code
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Storage-Trait-Purity-v1 | Covers: execution | Verdict: PARTIAL | Notes: structured collaboration artifact emission currently requires a SQLite downcast helper
  - Path: ../handshake_main/src/backend/handshake_core/src/api/loom.rs | Artifact: WP-1-Storage-Trait-Purity-v1 | Covers: execution | Verdict: PARTIAL | Notes: Loom search derives `tier_used` by checking the concrete backend type
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/retention.rs | Artifact: WP-1-Storage-Trait-Purity-v1 | Covers: execution | Verdict: PARTIAL | Notes: retention pin updates still branch through a SQLite downcast and direct pool access
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs | Artifact: WP-1-Storage-Trait-Purity-v1 | Covers: execution | Verdict: PARTIAL | Notes: Locus helper functions still downcast the trait object to `SqliteDatabase`
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The existing stub is the correct authority for the real remaining gap, but current product code is still only partial and the packet must explicitly expand into the concrete downcast-removal and capability-query remediation scope.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet hardens an internal backend trait boundary and does not implement a new GUI surface directly.
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
- RISK_TIER: MEDIUM
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Abstraction-Layer
- BUILD_ORDER_BLOCKS: NONE
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md One Storage API plus Trait Purity Invariant [CX-DBP-010]/[CX-DBP-040]
- WHAT: Remove production `Database` downcast escape hatches and replace them with explicit backend identity or capability queries so Loom, Locus, retention, and workflow code remain inside the storage abstraction boundary.
- WHY: Current product code still depends on `as_any` and concrete `SqliteDatabase` checks, which violates the current portability law and hides SQLite-only behavior behind implementation details instead of explicit capability semantics.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- OUT_OF_SCOPE:
  - runtime DDL cleanup and migration restructuring
  - broader Stage/media portability work
  - new GUI surfaces or new Flight Recorder event families
  - unrelated storage feature expansion beyond explicit backend identity or capability semantics
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - Production code no longer depends on `Database` downcasts for behavior decisions.
  - Unsupported backend behavior is explicit through backend identity or capability methods, not hidden concrete-type branching.
  - Loom, Locus, retention, and workflow-side consumers keep their intended behavior without using `as_any`.
  - Dual-backend or backend-sensitive tripwire tests fail if a production downcast seam returns.
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
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - as_any
  - downcast_ref
  - SqliteDatabase::pool
  - backend_kind
  - supports(
  - sqlite_pool_for_structured_artifacts
  - locus sqlite
  - tier_used
- RUN_COMMANDS:
  ```bash
  rg -n "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Production code keeps `as_any` escape hatches" -> "backend divergence can reappear without audit visibility"
  - "SQLite-only behavior remains implicit" -> "Postgres readiness is overstated and failures surface late"
  - "Loom or Locus semantics drift during the refactor" -> "runtime behavior changes without matching capability gates or tests"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just orchestrator-prepare-and-packet` will sync Task Board and Build Order truth.
  - After packet creation, verify the Build Order still shows this WP as the active packet for the base id and that downstream Stage/media dependencies continue to point at the base storage-trait-purity track.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: One Storage API [CX-DBP-010] | WHY_IN_SCOPE: product code still makes behavior decisions by concrete backend type instead of staying behind one storage boundary | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/api/loom.rs; src/backend/handshake_core/src/storage/retention.rs; src/backend/handshake_core/src/storage/locus_sqlite.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity; cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier | RISK_IF_MISSED: business logic will continue to rely on hidden backend branching and portability claims will remain overstated
  - CLAUSE: Trait Purity Invariant [CX-DBP-040] | WHY_IN_SCOPE: the `Database` trait still exposes `as_any`, which enables backend-specific production logic through downcasts | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity; rg -n "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" src/backend/handshake_core/src | RISK_IF_MISSED: future call sites can keep escaping the trait boundary without clear audit pressure
  - CLAUSE: Dual-Backend Testing Early [CX-DBP-013] | WHY_IN_SCOPE: the refactor is only valuable if both backends remain explicit and testable after downcast removal | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/tests.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability; cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: SQLite and Postgres behavior can drift again while still appearing green in narrow local checks

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: `Database` trait object boundary | PRODUCER: storage/sqlite.rs and storage/postgres.rs implementations | CONSUMER: workflows.rs, api/loom.rs, storage/retention.rs, storage/locus_sqlite.rs | SERIALIZER_TRANSPORT: in-process trait object | VALIDATOR_READER: storage/tests.rs and source grep tripwires | TRIPWIRE_TESTS: database_trait_purity plus source grep for production downcasts | DRIFT_RISK: concrete backend assumptions can return silently through helper methods or trait escape hatches
  - CONTRACT: explicit backend identity or capability query surface | PRODUCER: `Database` implementations | CONSUMER: Loom search tier selection, structured-artifact emission, Locus routing, retention update paths | SERIALIZER_TRANSPORT: in-process enum or capability methods | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: database_trait_purity; locus_backend_capability; loom_search_backend_tier | DRIFT_RISK: capability names or semantics drift and callers start reintroducing ad hoc backend checks
  - CONTRACT: Locus SQLite-only operation gate | PRODUCER: storage/locus_sqlite.rs | CONSUMER: workflow and task-board coordination paths | SERIALIZER_TRANSPORT: in-process result or terminal error | VALIDATOR_READER: storage/tests.rs and Locus-facing tests | TRIPWIRE_TESTS: locus_backend_capability | DRIFT_RISK: unsupported backend behavior becomes silent or inconsistent instead of explicit
  - CONTRACT: Loom search `tier_used` recorder payload | PRODUCER: api/loom.rs | CONSUMER: Flight Recorder and downstream diagnostics | SERIALIZER_TRANSPORT: Flight Recorder JSON event payload | VALIDATOR_READER: loom tests and event assertions | TRIPWIRE_TESTS: loom_search_backend_tier | DRIFT_RISK: telemetry stays green while semantic backend classification changes unexpectedly

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - a `Database` trait object backed by SQLite reports explicit backend identity or capability without requiring downcast access
  - a `Database` trait object backed by PostgreSQL rejects SQLite-only Locus or structured-artifact paths through explicit capability law or terminal error, not concrete-type checks
  - Loom search records the same `tier_used` semantics using explicit backend identity or capability queries instead of `as_any`

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Add explicit backend identity or capability methods to the `Database` trait and remove or isolate `as_any` from production paths.
  - Refactor Loom, workflows, retention, and Locus callers to use the explicit trait surface instead of concrete-type branching.
  - Add regression tests that fail if production downcasts return or if backend-sensitive behavior becomes implicit again.
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
- CARRY_FORWARD_WARNINGS:
  - Do not replace downcasts with hidden helper wrappers that still leak concrete backend types.
  - Do not silently no-op unsupported PostgreSQL paths; fail explicitly through capability law or deterministic terminal error.
  - Do not widen this packet into runtime DDL cleanup or unrelated storage feature work.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - One Storage API boundary remains intact in production code
  - Trait Purity Invariant is satisfied without production `as_any` escape hatches
  - backend-sensitive Locus, Loom, retention, and workflow behavior is explicit and tested
- FILES_TO_READ:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- COMMANDS_TO_RUN:
  - rg -n "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" src/backend/handshake_core/src
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - verify production-source grep stays free of backend downcasts except for test-only or implementation-local code inside backend modules that does not cross the trait boundary

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The final shape of the explicit backend identity or capability API is not proven until coding and validation complete.
  - Full PostgreSQL runtime behavior is not proven at refinement time; it depends on the later coding and dual-backend validation pass.
  - Whether any test-only downcasts remain acceptable behind isolated test helpers will need inspection during implementation and validation.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Current Main Body law explicitly defines one storage boundary, forbids backend-specific type exposure from the `Database` trait, and requires dual-backend test posture. The remaining work is specific implementation remediation in current product code.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Current Handshake_Master_Spec_v02.179.md already defines the storage-boundary law this packet needs to implement and prove. No Main Body or appendix update is required before packet activation.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.1 One Storage API [CX-DBP-010]
- CONTEXT_START_LINE: 3260
- CONTEXT_END_LINE: 3277
- CONTEXT_TOKEN: **Pillar 1: One Storage API [CX-DBP-010]**
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.13.1 Four Portability Pillars [CX-DBP-002]

  **Pillar 1: One Storage API [CX-DBP-010]**

  All database operations MUST flow through a single storage module boundary. No business logic code may directly access database connections.

  - FORBIDDEN: Direct `sqlx::query()` in API handlers
  - FORBIDDEN: Direct `state.pool` or `state.fr_pool` access outside `src/storage/`
  - REQUIRED: All DB operations via `state.storage.*` interface
  - REQUIRED: AppState MUST NOT expose raw `SqlitePool` or `DuckDbConnection`

  **Enforcement:**
  Pre-commit validation checks for direct pool access in API handlers (FAIL on violation).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.3 Trait Purity Invariant [CX-DBP-040]
- CONTEXT_START_LINE: 3361
- CONTEXT_END_LINE: 3368
- CONTEXT_TOKEN: **[CX-DBP-040] Trait Purity Invariant (Normative):**
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.13.3 Storage API Abstraction Pattern [CX-DBP-021]

  The storage module MUST define a trait-based interface that hides database differences. This contract is MANDATORY for all storage implementations.

  **[CX-DBP-040] Trait Purity Invariant (Normative):**
  The `Database` trait MUST NOT expose any methods that return concrete, backend-specific types (e.g., `SqlitePool`, `PgPool`, `DuckDbConnection`). All implementations MUST encapsulate their internal connection pools.
  - **Violation:** `fn sqlite_pool(&self) -> Option<&SqlitePool>` is strictly FORBIDDEN.
  - **Remediation:** Any service requiring database access (e.g., Janitor, Search) MUST consume the generic `Database` trait methods or be refactored into a trait-compliant operation.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.1 Dual-Backend Testing Early [CX-DBP-013]
- CONTEXT_START_LINE: 3313
- CONTEXT_END_LINE: 3323
- CONTEXT_TOKEN: **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**

  Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.

  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant (can use PostgreSQL in Docker)
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge

  **Rationale:**
  Testing only against SQLite means PostgreSQL bugs are discovered during Phase 2 migration (expensive). Testing both backends in Phase 1 catches portability issues immediately.
  ```
