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
- WP_ID: WP-1-Loom-Storage-Portability-v2
- REFINEMENT_FORMAT_VERSION: 2026-03-08
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-16T19:14:53.9235633Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/records/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: 22d8bc984dcb8552ba1539928a23fe0ca89a54ab
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja160320262020
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Loom-Storage-Portability-v2
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Local `main` already contains the selective Loom portability integration, and the post-hoc audit did not find a concrete storage-portability spec defect in that targeted slice.
- That stronger-but-narrower result is still not enough for operator trust: the implementation was judged underperformed, and the packet evidence did not establish durable clause-by-clause confidence for the current-main storage slice.
- This remediation pass therefore focuses on current-main hardening: re-auditing the landed SQLite/PostgreSQL portability seam, tightening parity tripwires where the proof is still weak, and simplifying any brittle adapter logic discovered during implementation.
- `api/loom.rs` still sits at a wide seam between filesystem writes, content-hash dedup, storage CRUD, preview job launch, and search/view endpoints, so the WP must stay sharply scoped to the storage portability contract and avoid broad workflow/runtime churn.
- Any code changes found necessary during this pass must stay inside the existing Loom storage/API/filesystem/conformance surface and must not reintroduce unrelated MCP, MEX, LLM, or formatter churn.
- The live smoke objective is therefore twofold: remediate any current-main portability rough edges that emerge under fresh review, and prove the slice with stronger WP Validator communication and clause-closure evidence from the start.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 90m
- SEARCH_SCOPE: official backend-search docs, official cloud full-text search docs, typed lineage/provenance specs, recent provenance indexing research, OSS repository patterns for backend-adaptive search layers, and local Handshake storage/API/migration code
- REFERENCES: SQLite FTS5 docs; PostgreSQL full text search docs; Google Cloud Spanner full-text search overview; OpenLineage spec docs; OpenLineage repository; In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines; pgvector repository; current Handshake spec and Loom storage code
- PATTERNS_EXTRACTED: backend-adaptive search internals behind one stable API; typed source/provenance payloads that survive export and replay; provider-local optimization kept out of portable schema law; cross-provider conformance tests focused on semantic parity rather than identical query plans
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT backend-specific search internals behind one stable Loom storage contract; ADAPT lineage-style typed provenance into explicit `source_anchor` and export/replay expectations; REJECT provider-specific ranking or vector-only enhancements as canonical Loom meaning in Phase 1
- LICENSE/IP_NOTES: Reference-only review of official documentation and OSS repositories. No third-party code is intended for direct copy into product code.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.178.md already defines portability pillars, Loom storage trait expectations, backend-agnostic search rules, and canonical portable Loom contracts. This WP is implementation and conformance hardening against the current Main Body.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON_NO: N/A
- SOURCE_MAX_AGE_DAYS: 730
- SOURCE_LOG:
  - Source: SQLite FTS5 docs | Kind: OSS_DOC | Date: 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | URL: https://sqlite.org/fts5.html | Why: canonical reference for SQLite-side full-text search behavior and index locality
  - Source: PostgreSQL full text search docs | Kind: OSS_DOC | Date: 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | URL: https://www.postgresql.org/docs/current/textsearch.html | Why: canonical reference for PostgreSQL-side ranked text search and backend-specific query power
  - Source: Google Cloud Spanner full-text search overview | Kind: BIG_TECH | Date: 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | URL: https://cloud.google.com/spanner/docs/full-text-search | Why: current large-scale vendor reference showing richer backend-specific search features can exist behind one SQL-facing search surface
  - Source: OpenLineage spec docs | Kind: OSS_DOC | Date: 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | URL: https://openlineage.io/docs/spec/ | Why: useful reference for typed lineage and provenance payloads that survive transport and backend changes
  - Source: OpenLineage repository | Kind: GITHUB | Date: 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | URL: https://github.com/OpenLineage/OpenLineage | Why: concrete repository-scale example of typed lineage/provenance contract evolution
  - Source: In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines | Kind: PAPER | Date: 2025-11-05 | Retrieved: 2026-03-13T22:38:08Z | URL: https://arxiv.org/abs/2511.03480 | Why: recent provenance-indexing paper supporting explicit, queryable source/provenance structures instead of opaque backend-local metadata
  - Source: pgvector repository | Kind: GITHUB | Date: 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | URL: https://github.com/pgvector/pgvector | Why: high-signal reference for backend-specific search acceleration that should remain optional rather than canonical in this packet
- RESEARCH_SYNTHESIS:
  - A portability packet should preserve one stable API and semantic contract while allowing provider-specific indexing and query plans behind the boundary.
  - Big-tech search systems confirm that richer backend-specific ranking, tokenization, and query expansion can stay behind a stable query surface instead of redefining canonical filter meaning.
  - Typed provenance payloads are more durable than ad hoc search or edge metadata and map well to Loom `source_anchor` export/replay expectations.
  - Recent provenance-indexing research reinforces that explicit source/provenance structures should stay queryable and transport-stable, which matches Handshake's need for durable `LoomSourceAnchor` semantics across storage, export, and replay.
  - Search parity should mean stable filters, result identity, and semantic guarantees, not identical score math across SQLite and PostgreSQL.
  - Backend-specific enhancements are useful, but they must not become the only definition of Loom search or graph behavior while Handshake still promises SQLite-now and PostgreSQL-ready portability.
- RESEARCH_GAPS_TO_TRACK:
  - NONE
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: SQLite FTS5 docs | Pattern: provider-local full-text indexing hidden behind a stable search interface | Why: directly supports keeping SQLite search internals local while preserving one Loom API contract
  - Source: PostgreSQL full text search docs | Pattern: backend-specific query power behind a shared result surface | Why: reinforces that PostgreSQL can do more internally without redefining the portable meaning of search filters or result identity
  - Source: Google Cloud Spanner full-text search overview | Pattern: advanced backend-specific search features exposed behind one database-facing search surface | Why: supports preserving a stable Handshake Loom API while still permitting backend-local ranking or query-expansion improvements later
- ADAPT_PATTERNS:
  - Source: OpenLineage spec docs | Pattern: typed provenance structures that survive transport and replay | Why: maps well to making `LoomSourceAnchor` a durable portable contract instead of backend-local metadata
  - Source: OpenLineage repository | Pattern: evolving contract ownership through explicit schemas and compatibility posture | Why: useful for treating Loom block, edge, filter, and source-anchor structs as canonical portable library artifacts rather than adapter-only shapes
  - Source: In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines | Pattern: queryable provenance captured at explicit record and attribute boundaries | Why: reinforces that `LoomSourceAnchor` should remain a first-class portable struct with testable round-trip semantics instead of a lossy adapter detail
- REJECT_PATTERNS:
  - Source: pgvector repository | Pattern: making backend-specific acceleration the canonical semantics of search or retrieval | Why: Phase 1 portability requires SQLite and PostgreSQL parity first; vector acceleration can remain optional and downstream
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - backend portable lineage contract repo
  - backend adaptive search storage repo
- MATCHED_PROJECTS:
  - Source: OpenLineage repository | Repo: OpenLineage/OpenLineage | URL: https://github.com/OpenLineage/OpenLineage | Intent: ARCH_PATTERN | Decision: ADAPT | Impact: NONE | Stub: NONE | Notes: useful reference for typed lineage/provenance contract evolution, especially for portable `source_anchor` and export/replay semantics
  - Source: pgvector repository | Repo: pgvector/pgvector | URL: https://github.com/pgvector/pgvector | Intent: IMPLEMENTATION | Decision: TRACK_ONLY | Impact: NONE | Stub: NONE | Notes: helpful future reference for Postgres-only retrieval acceleration, but out of scope for canonical portability law in this packet
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Reuse existing Loom event families; no new Flight Recorder ids are needed for portability activation alone.
- Loom block and edge CRUD should continue to emit the existing Loom lifecycle events.
- Import dedup flow should continue to emit the existing `loom_dedup_hit` event.
- View and search parity checks should continue to rely on the existing Loom view and search events so backend changes do not hide behavior regressions.
- Portability fixes should keep event payload meaning backend-neutral even if query implementation differs underneath.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: provider-specific search logic changes filter meaning across backends. Mitigation: assert semantic parity in shared Loom conformance tests rather than trusting query-shape similarity.
- Risk: source-anchor payloads are dropped or partially serialized on one backend. Mitigation: make `LoomSourceAnchor` round-trip tests part of portability coverage.
- Risk: portability refactors widen into filesystem or workflow-runtime changes. Mitigation: keep this packet centered on storage contracts, migrations, blob-path stability, and parity tests.
- Risk: backend-specific acceleration becomes the only supported semantics. Mitigation: treat SQLite and PostgreSQL parity as the contract and relegate backend-only improvements to optional enhancements.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-LoomBlock
  - PRIM-LoomBlockContentType
  - PRIM-LoomBlockDerived
  - PRIM-LoomEdge
  - PRIM-LoomEdgeType
  - PRIM-LoomViewFilters
  - PRIM-LoomSearchFilters
  - PRIM-LoomBlockSearchResult
  - PRIM-LoomSourceAnchor
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-LoomBlock
  - PRIM-LoomBlockContentType
  - PRIM-LoomBlockDerived
  - PRIM-LoomEdge
  - PRIM-LoomEdgeType
  - PRIM-LoomViewFilters
  - PRIM-LoomSearchFilters
  - PRIM-LoomBlockSearchResult
  - PRIM-LoomSourceAnchor
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - The spec already names the Loom storage-contract primitives. The gap is parity enforcement, migration discipline, filesystem-path stability, and cross-provider conformance coverage.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current spec appendix already contains the relevant Loom block, edge, filter, search-result, and source-anchor primitive ids.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Implementation should align storage code and tests to the existing Loom primitive set rather than creating new ids during activation.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new high-signal orphan Loom primitives were discovered; the appendix already names the portable storage-contract shapes needed here.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Existing appendix ownership notes already cover Loom as a portable backend library surface and identify this packet as the remaining portability gap.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet is backend portability and conformance work. Direct Loom UI/viewer behavior remains downstream of the storage contract.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Current appendix interaction coverage is sufficient; activation does not require a new IMX edge before coding can start.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep Appendix 12 unchanged and implement against the current v02.178 Loom portability law and ownership map.
  - If implementation reveals a missing appendix edge or primitive, handle that as a separate spec-update flow rather than silent packet drift.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: Loom storage portability does not change spatial reasoning or scene contracts | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physical simulation or measurement law is involved in this packet | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers, not direct storage-portability owners, here | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-control or device-IO contract is changed directly | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration surfaces may call Loom later, but no director contract is changed in this packet | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no composition or media arrangement contract is updated here | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is changed by storage portability work | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication surfaces remain downstream of storage parity work | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking or recipe workflow surface is affected | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is involved in Loom storage portability | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: delivery and routing surfaces are not modified directly by this packet | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: Loom is a durable library surface and this packet hardens how blocks, edges, and asset references survive backend swaps, export, and replay | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: view and search filters are part of the portable library contract and must preserve meaning across backends | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytic consumers may read Loom later, but this packet stops at storage parity and conformance | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no external dataset ingestion or wrangling contract is changed directly | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: portable DDL, migration replay safety, dual-backend parity, and provider-local index choices are direct DBA concerns in this packet | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this packet implements existing storage law rather than adding a new governance authority surface | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no guidance or tutoring interface is added here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: NOT_TOUCHED | NOTES: search results are relevant to context retrieval later, but this packet focuses on storage and parity rather than retrieval orchestration | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: migration versioning, provider parity, and export/replay durability are first-class versioning concerns in this packet | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or capability-isolation boundary is changed directly by Loom storage portability | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: existing Loom events remain in place; the packet should preserve their backend-neutral meaning rather than add new ids | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: calendar storage and policy surfaces are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: editor behavior is not changed by Loom storage portability | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: document-editing surfaces are out of scope for this packet | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: spreadsheet surfaces are unrelated to Loom storage portability | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: Locus structured-collaboration surfaces are intentionally kept out of this packet to preserve file-lock separation | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: TOUCHED | NOTES: Loom block, edge, search, view, source-anchor, and asset-path contracts are the direct subject of this packet | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: product work-packet surfaces are unrelated to Loom storage portability | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: product task-board surfaces are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: micro-task execution contracts are not part of the Loom storage portability scope | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: Command Center may observe Loom later, but no direct Command Center behavior is changed here | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: memory-system behavior is unrelated to Loom storage portability | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: NOT_TOUCHED | NOTES: preview job launch is an integration seam, not the center of this portability packet | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no Spec Router or prompt-compilation contract is changed here | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: this packet is a direct backend parity and migration-readiness step for a concrete storage surface | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: NOT_TOUCHED | NOTES: Loom search results matter to retrieval later, but this packet stops at portable storage semantics and parity tests | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Stage artifact portability is a separate packet family | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Studio runtime behavior is out of scope | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: direct UI/viewer behavior is downstream of the storage contract | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: distillation workflows are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE runtime protocol or validator surface is changed directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval consumers may use Loom later, but no RAG contract is changed directly in this packet | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Loom | CAPABILITY_SLICE: Portable block and edge record parity | SUBFEATURES: `LoomBlock`, `LoomEdge`, content-hash dedup, metrics rebuildability, and stable backend-neutral meaning | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomBlockDerived, PRIM-LoomEdgeType | MECHANICAL: engine.archivist, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the packet should guarantee that block and edge semantics survive SQLite and PostgreSQL backends without adapter drift
  - PILLAR: Loom | CAPABILITY_SLICE: Portable view, search, and source-anchor contract | SUBFEATURES: `LoomViewFilters`, `LoomSearchFilters`, `LoomBlockSearchResult`, and `LoomSourceAnchor` parity | PRIMITIVES_FEATURES: PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult, PRIM-LoomSourceAnchor | MECHANICAL: engine.librarian, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the API contract should preserve the same filter meaning and source-anchor durability across both backends
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Loom migration and DDL portability | SUBFEATURES: replay-safe migrations, down migrations, provider-local indexes outside portable DDL, and no trigger-dependent semantics | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the direct portability law bridge from spec to code for the Loom surface
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Cross-provider Loom conformance coverage | SUBFEATURES: shared test helpers for SQLite and PostgreSQL parity over CRUD, search, view, dedup, and anchor round-trips | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | MECHANICAL: engine.archivist, engine.librarian, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity should be proven by tests, not inferred from provider implementations
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Loom block and edge storage parity | JobModel: UI_ACTION | Workflow: loom_surface_crud | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_block_created, loom_block_updated, loom_block_deleted, loom_edge_created, loom_edge_deleted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend parity must preserve CRUD meaning and existing Loom telemetry regardless of provider implementation
  - Capability: Loom import and dedup portability | JobModel: WORKFLOW | Workflow: loom_import | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_dedup_hit, loom_block_created | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: content-hash dedup, asset-path layout, and import-created blocks must preserve the same semantics across backends
  - Capability: Loom view portability | JobModel: UI_ACTION | Workflow: loom_view_query | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_view_queried | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: `LoomViewFilters` and grouped-view semantics must not drift when the backend changes
  - Capability: Loom search portability | JobModel: UI_ACTION | Workflow: loom_search | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_search_executed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: search scoring may differ by provider, but filter meaning, result identity, and backend-neutral contract must remain stable
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Scanned for high-ROI appendix matrix additions and found that current v02.178 already records Loom as a portable backend library surface and ties it to storage portability.
  - The activation need is implementation parity and conformance coverage, not a new appendix interaction edge.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Current appendix interaction coverage already captures the relevant Loom portability relationships; no new IMX edge is required before coding can start.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON_NO: N/A
- SOURCE_SCAN:
  - Source: SQLite FTS5 docs | Kind: OSS_DOC | Angle: provider-local full-text search implementation | Pattern: backend-specific indexing hidden behind one search surface | Decision: ADOPT | EngineeringTrick: keep FTS tables and tokenization local to SQLite while preserving one stable `search_loom_blocks` contract | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: supports backend-specific optimization without leaking SQLite-only semantics into the portable contract
  - Source: PostgreSQL full text search docs | Kind: OSS_DOC | Angle: ranked relational search with richer backend capabilities | Pattern: more powerful backend query plan behind the same result surface | Decision: ADOPT | EngineeringTrick: allow richer Postgres internals while keeping filter meaning and result identity stable across providers | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly maps to the spec rule that PostgreSQL may do more internally without redefining the API
  - Source: Google Cloud Spanner full-text search overview | Kind: BIG_TECH | Angle: vendor-scale full-text search that still presents a single query surface | Pattern: advanced backend behavior kept behind a stable search contract | Decision: ADOPT | EngineeringTrick: keep backend-local ranking, tokenization, and optional query expansion on the implementation side of `search_loom_blocks` instead of hardcoding them into portable semantics | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: useful confirmation that richer search backends should not force a new public API shape
  - Source: OpenLineage spec docs | Kind: OSS_DOC | Angle: typed provenance and transport-stable lineage payloads | Pattern: explicit typed provenance instead of adapter-local metadata blobs | Decision: ADAPT | EngineeringTrick: treat `LoomSourceAnchor` as a durable typed contract that survives storage, export, and replay | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: useful model for source-anchor portability and replay safety
  - Source: OpenLineage repository | Kind: GITHUB | Angle: schema evolution of typed lineage contracts | Pattern: explicit compatibility posture around evolving structured payloads | Decision: ADAPT | EngineeringTrick: keep portable Loom contract structs central and test round-trip behavior across backends | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: reinforces schema ownership and compatibility discipline around portable contracts
  - Source: In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines | Kind: PAPER | Angle: recent provenance indexing with explicit record and attribute lineage | Pattern: provenance stays queryable because it is modeled explicitly instead of buried in engine-specific logs | Decision: ADAPT | EngineeringTrick: preserve `LoomSourceAnchor` fields as explicit portable payloads and assert them in backend-parity tests | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly supports replay-safe source-anchor behavior across SQLite and PostgreSQL
  - Source: pgvector repository | Kind: GITHUB | Angle: backend-specific retrieval acceleration | Pattern: optional provider-local acceleration that should not redefine canonical semantics | Decision: REJECT | EngineeringTrick: do not let future Postgres-only acceleration become the only definition of Loom search meaning | ROI: MEDIUM | Resolution: REJECT_DUPLICATE | Stub: NONE | Notes: useful future reference, but out of scope for Phase 1 portability law
- MATRIX_GROWTH_CANDIDATES:
  - Combo: Stable search API plus provider-local indexing | Sources: SQLite FTS5 docs, PostgreSQL full text search docs | WhatToSteal: backend-specific indexing and query plans behind one result contract | HandshakeCarryOver: preserve one `search_loom_blocks` API and semantic filter meaning while letting SQLite and PostgreSQL optimize differently underneath | RuntimeConsequences: Loom search remains portable and future RAG consumers do not need provider-specific branching | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the main portability pattern for search in the packet
  - Combo: Portable source-anchor lineage plus export/replay durability | Sources: OpenLineage spec docs, OpenLineage repository | WhatToSteal: typed provenance structures with explicit contract ownership | HandshakeCarryOver: keep `LoomSourceAnchor` round-trippable and testable across storage, export, and replay paths | RuntimeConsequences: backlinks, context snippets, and downstream Loom bridge work can trust anchors after backend changes | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the highest-leverage typed-contract pattern outside search parity
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep provider-specific FTS and ranking logic inside backend implementations, not in portable migrations.
  - Assert semantic parity through shared Loom conformance tests rather than comparing SQL query text.
  - Treat `LoomSourceAnchor` and view/search filters as portable contract structs, not adapter-only shapes.
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: One Loom storage contract plus dual provider implementations | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.dba, engine.version | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | Resolution: IN_THIS_WP | Stub: NONE | Notes: the packet should make one semantic contract survive both SQLite and PostgreSQL without adapter drift
  - Combo: Portable Loom migrations plus replay-safe and down-safe verification | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.version | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge | Resolution: IN_THIS_WP | Stub: NONE | Notes: portability is incomplete if the Loom tables cannot be replayed and reverted safely in both provider contexts
  - Combo: SQLite FTS and PostgreSQL text search behind one API | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.librarian, engine.dba, engine.version | Primitives/Features: PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult | Resolution: IN_THIS_WP | Stub: NONE | Notes: search scoring may differ, but filters and result identity must stay portable
  - Combo: View filter parity across providers | Pillars: Loom | Mechanical: engine.librarian, engine.dba | Primitives/Features: PRIM-LoomViewFilters, PRIM-LoomBlock, PRIM-LoomEdge | Resolution: IN_THIS_WP | Stub: NONE | Notes: grouped and filtered Loom views should preserve the same meaning across both backends
  - Combo: Source-anchor durability across storage, export, and replay | Pillars: Loom | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-LoomSourceAnchor, PRIM-LoomEdge | Resolution: IN_THIS_WP | Stub: NONE | Notes: backlink context and mention/tag provenance depend on anchors staying portable
  - Combo: Asset blob path stability plus storage metadata parity | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.dba | Primitives/Features: PRIM-LoomBlock, PRIM-LoomBlockDerived | Resolution: IN_THIS_WP | Stub: NONE | Notes: filesystem path stability is part of the portability contract for imported Loom assets and previews
  - Combo: Rebuildable derived metrics plus provider-local indexes | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.dba, engine.version | Primitives/Features: PRIM-LoomBlockDerived, PRIM-LoomEdge, PRIM-LoomBlock | Resolution: IN_THIS_WP | Stub: NONE | Notes: derived counts and indexes should be rebuildable and should not become migration-coupled state
  - Combo: Shared Loom conformance tests over SQLite and PostgreSQL | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.librarian, engine.dba | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | Resolution: IN_THIS_WP | Stub: NONE | Notes: parity should be demonstrated by tests over the public contract, not inferred from similar-looking implementations
  - Combo: Thin API seam over portable storage behavior | Pillars: Loom | Mechanical: engine.archivist, engine.dba | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomBlockSearchResult | Resolution: IN_THIS_WP | Stub: NONE | Notes: `api/loom.rs` should remain a consumer of the portable storage contract rather than a second portability authority
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations identified for the touched Loom and storage-portability surfaces are direct responsibilities of this packet and do not require new stubs or a spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: Loom MVP and storage-portability-related stubs; current storage trait, provider implementations, Loom API, filesystem helpers, migrations, and cross-backend storage test helpers
- MATCHED_STUBS:
  - Artifact: WP-1-Media-Downloader-Loom-Bridge-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: downloader promotion and media bridge behavior should stay downstream of the portable Loom storage contract
  - Artifact: WP-1-Video-Archive-Loom-Integration-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: video-library composition depends on storage portability but should not redefine the base contract
  - Artifact: WP-1-Loom-Preview-VideoPosterFrames-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: preview-generation behavior is an adjacent follow-on and should not expand this storage-portability packet
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Loom-MVP-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Loom MVP delivered the first implementation surface, but the dedicated portability law and dual-backend hardening remain a separate packet
  - Artifact: WP-1-Storage-Abstraction-Layer-v3 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: trait-based storage plumbing exists, but Loom-specific parity and conformance are still missing
  - Artifact: WP-1-Artifact-System-Foundations-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: artifact foundations exist, but Loom-specific asset-path and replay portability still need dedicated hardening
- CODE_REALITY_EVIDENCE:
  - Path: src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Storage-Abstraction-Layer-v3 | Covers: execution | Verdict: PARTIAL | Notes: the Database trait already exposes Loom methods, but shared Loom conformance coverage is absent
  - Path: src/backend/handshake_core/src/storage/loom.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: primitive | Verdict: PARTIAL | Notes: canonical Loom structs exist, but portability-hardening and compatibility tests are still missing
  - Path: src/backend/handshake_core/src/storage/sqlite.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: execution | Verdict: PARTIAL | Notes: SQLite implementation includes FTS5 and CRUD behavior, but parity is not proven by shared Loom contract tests
  - Path: src/backend/handshake_core/src/storage/postgres.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: execution | Verdict: PARTIAL | Notes: PostgreSQL implementation exists, but contract parity with SQLite is not proven by a dedicated Loom test suite
  - Path: src/backend/handshake_core/src/storage/tests.rs | Artifact: NONE | Covers: execution | Verdict: NOT_PRESENT | Notes: current storage conformance helpers do not cover Loom block, edge, view, search, or source-anchor parity
  - Path: src/backend/handshake_core/src/api/loom.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: execution | Verdict: PARTIAL | Notes: API endpoints exist, but they currently rely on storage parity being correct rather than proving it
  - Path: src/backend/handshake_core/src/loom_fs.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: blob path layout is part of the portable contract and needs to remain stable as backends change
  - Path: src/backend/handshake_core/migrations/0013_loom_mvp.sql | Artifact: WP-1-Loom-MVP-v1 | Covers: execution | Verdict: PARTIAL | Notes: portable DDL exists, but dedicated Loom replay/down assertions are not part of a shared Loom portability suite yet
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The capability is already selectively integrated on local main, and the prior audit did not isolate a single concrete portability defect. The remediation need is a narrower hardening/proof pass over the landed storage-portability slice, not a greenfield Loom feature packet.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This activation is backend portability and conformance work. Direct Loom UI/viewer behavior remains downstream.
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
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet; downstream Loom surface work owns interaction and accessibility details.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.156]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Loom-MVP, WP-1-Storage-Abstraction-Layer, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-Media-Downloader-Loom-Bridge, WP-1-Video-Archive-Loom-Integration
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]
- WHAT: Harden Loom block, edge, search, view, source-anchor, migration, and asset-path behavior into one portable backend contract that preserves meaning across SQLite and PostgreSQL implementations.
- WHY: Local `main` already includes the selective Loom portability integration, but operator review still judged the implementation underperformed and the prior audit only established a narrower correctness slice. This packet re-audits and, where needed, remediates the landed storage-portability seam with stronger parity proof and validator pressure.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/migrations/0013_loom_mvp.sql
  - src/backend/handshake_core/migrations/0013_loom_mvp.down.sql
- OUT_OF_SCOPE:
  - structured-collaboration schema registry and mailbox export schema work
  - frontend Loom viewer UX and preview-surface layout changes
  - preview job protocol redesign or broad workflow-runtime refactors
  - downloader-to-Loom and video-archive bridge behavior beyond the base portability contract
  - capability-registry publication logic unrelated to Loom storage parity
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance
  just gov-check
  ```
- DONE_MEANS:
  - Loom block, edge, view, search, and source-anchor semantics are either confirmed unchanged or explicitly remediated on the landed current-main slice, with no vague "looks fine" closeout.
  - Portable migrations and down migrations for Loom tables remain replay-safe and provider-neutral.
  - Shared Loom conformance tests and semantic tripwires give explicit parity coverage for CRUD, dedup, views, search filters, literal search escaping, and source-anchor round-trips across both backends.
  - Filesystem asset-path layout remains stable and compatible with the portable storage contract, and no unrelated product families are touched.
- PRIMITIVES_EXPOSED:
  - PRIM-LoomBlock
  - PRIM-LoomBlockContentType
  - PRIM-LoomBlockDerived
  - PRIM-LoomEdge
  - PRIM-LoomEdgeType
  - PRIM-LoomViewFilters
  - PRIM-LoomSearchFilters
  - PRIM-LoomBlockSearchResult
  - PRIM-LoomSourceAnchor
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/0013_loom_mvp.sql
  - src/backend/handshake_core/migrations/0013_loom_mvp.down.sql
- SEARCH_TERMS:
  - create_loom_block
  - create_loom_edge
  - query_loom_view
  - search_loom_blocks
  - LoomSourceAnchor
  - LoomViewFilters
  - LoomSearchFilters
  - loom_blocks
  - loom_edges
  - loom_blocks_fts
- RUN_COMMANDS:
  ```bash
  rg -n "create_loom_block|create_loom_edge|query_loom_view|search_loom_blocks|LoomSourceAnchor|LoomViewFilters|LoomSearchFilters|loom_blocks|loom_edges|loom_blocks_fts" src/backend/handshake_core
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance
  just gov-check
  ```
- RISK_MAP:
  - "provider-specific search logic changes filter meaning" -> "view and search parity break across SQLite and PostgreSQL"
  - "source anchors fail to round-trip on one backend" -> "backlinks, context snippets, and downstream bridge packets lose stable provenance"
  - "filesystem asset-path layout drifts from storage metadata" -> "export, replay, and dedup behavior become unreliable"
  - "the remediation pass drifts into unrelated runtime families" -> "the packet loses file-lock isolation and repeats the earlier live-smoke scope failure"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - Current stub metadata and BUILD_ORDER ranking already match this activation target. No build-order edit is required unless scope expands beyond the portability boundary.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Main Body explicitly names Storage Backend Portability law, the Loom portable schema and storage-trait example, the backend-agnostic Loom search rule, and Loom as a portable backend library surface. This makes the packet clearly specified and testable without additional normative text.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.178.md already defines the portability pillars, Loom storage contract, backend-agnostic search expectations, and portable Loom primitives in the Main Body and appendix notes. This packet is a code and conformance alignment pass.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]
- CONTEXT_START_LINE: 3242
- CONTEXT_END_LINE: 3319
- CONTEXT_TOKEN: ### 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]

  **Why**
  Handshake's local-first philosophy (\u00A71.1.4) requires flexibility to support future migrations from SQLite (local) to PostgreSQL (cloud-optional). Building portability constraints now (Phase 1) prevents exponential rework costs in Phase 2+.

  **What**
  Defines four mandatory architectural pillars for ensuring database backend flexibility: single storage API, portable schema/migrations, rebuildable indexes, and dual-backend testing.

  **Pillar 2: Portable Schema & Migrations [CX-DBP-011]**

  All migrations MUST be written in DB-agnostic SQL. SQLite-specific syntax is forbidden.

  - FORBIDDEN: `strftime()`, SQLite datetime functions -> REQUIRED: Parameterized timestamps
  - FORBIDDEN: SQLite placeholder syntax `?1`, `?2` -> REQUIRED: Portable syntax `$1`, `$2`
  - FORBIDDEN: SQLite triggers with `OLD`/`NEW` semantics -> REQUIRED: Application-layer mutation tracking
  - REQUIRED: Migrations use version-managed framework (compatible with sqlx::migrate or similar)
  - REQUIRED: Schema definitions are pure DDL (no data transforms)

  **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**

  Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.

  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant (can use PostgreSQL in Docker)
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]
- CONTEXT_START_LINE: 3518
- CONTEXT_END_LINE: 3606
- CONTEXT_TOKEN: #### 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]

  Loom's tables are a portability reference implementation: no triggers, portable SQL types, and rebuildable derived indexes/metrics.

  [ADD v02.156] LoomBlock/LoomEdge records, LoomViewFilters, LoomSearchFilters, LoomBlockSearchResult, and LoomSourceAnchor are canonical portable backend library contracts. Their meaning MUST survive SQLite-now / PostgreSQL-ready storage, export, and replay instead of being hidden behind view-only adapters.

  **Portable schema (SQLite + PostgreSQL)**

  -- LoomBlocks table
  CREATE TABLE loom_blocks (
      block_id UUID PRIMARY KEY,
      workspace_id UUID NOT NULL,
      content_type TEXT NOT NULL,       -- 'note', 'file', 'annotated_file', 'tag_hub', 'journal'
      document_id UUID,                 -- FK to documents table (nullable)
      asset_id UUID,                    -- FK to assets table (nullable)
      title TEXT,
      original_filename TEXT,
      content_hash TEXT,                -- SHA-256 hex
      pinned BOOLEAN NOT NULL DEFAULT FALSE,
      journal_date TEXT,                -- ISO date string (YYYY-MM-DD)
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      imported_at TIMESTAMP
  );

  -- LoomEdges table (Knowledge Graph edges for Loom features)
  CREATE TABLE loom_edges (
      edge_id UUID PRIMARY KEY,
      source_block_id UUID NOT NULL REFERENCES loom_blocks(block_id),
      target_block_id UUID NOT NULL REFERENCES loom_blocks(block_id),
      edge_type TEXT NOT NULL,         -- 'mention', 'tag', 'sub_tag', 'parent', 'ai_suggested'
      created_by TEXT NOT NULL,        -- 'user' or 'ai'
      crdt_site_id TEXT,
      source_anchor_doc_id UUID,
      source_anchor_block_id UUID,
      source_anchor_offset_start INTEGER,
      source_anchor_offset_end INTEGER,
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

  **Storage trait extension (conceptual)**
  This extends the existing Storage API boundary (\u00A72.3.13.3). It is not a parallel storage layer.

  trait LoomStorage {
      // Block CRUD
      async fn create_loom_block(&self, block: &LoomBlock) -> Result<UUID>;
      async fn get_loom_block(&self, block_id: UUID) -> Result<Option<LoomBlock>>;
      async fn update_loom_block(&self, block_id: UUID, update: &LoomBlockUpdate) -> Result<()>;
      async fn delete_loom_block(&self, block_id: UUID) -> Result<()>;

      // Deduplication
      async fn find_by_content_hash(&self, workspace_id: UUID, hash: &str) -> Result<Option<UUID>>;

      // Edge CRUD
      async fn create_loom_edge(&self, edge: &LoomEdge) -> Result<UUID>;
      async fn delete_loom_edge(&self, edge_id: UUID) -> Result<()>;
      async fn get_backlinks(&self, block_id: UUID) -> Result<Vec<LoomEdge>>;
      async fn get_outgoing_edges(&self, block_id: UUID) -> Result<Vec<LoomEdge>>;

      // View queries
      async fn query_all_view(&self, workspace_id: UUID, pagination: &Pagination) -> Result<Vec<LoomBlock>>;
      async fn query_unlinked_view(&self, workspace_id: UUID, pagination: &Pagination) -> Result<Vec<LoomBlock>>;
      async fn query_sorted_view(&self, workspace_id: UUID, group_by: LoomEdgeType) -> Result<Vec<LoomGroup>>;
      async fn query_pinned_view(&self, workspace_id: UUID) -> Result<Vec<LoomBlock>>;

      // Search (backend-adaptive)
      async fn search_loom_blocks(&self, workspace_id: UUID, query: &str, filters: &LoomSearchFilters) -> Result<Vec<LoomBlockSearchResult>>;
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]
- CONTEXT_START_LINE: 62252
- CONTEXT_END_LINE: 62318
- CONTEXT_TOKEN: ## 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]

  Loom is a **library + context** surface derived from Heaper patterns: a unified "block" object that can represent a note, a file, or a file-with-annotation, with fast browsing views and a lightweight relational model (mentions, tags, backlinks).

  - Core entity + edge definitions are integrated into \u00A72.2.1.14 (LoomBlock) and \u00A72.3.7.1 (LoomEdge).
  - This section preserves the full Loom integration spec (imported) to avoid loss of detail/intent.

  #### 1. Purpose and Scope

  This document extracts validated UX patterns and architectural concepts from Heaper (a local-first, linked note-taking application spanning notes, media, and files) and maps them onto Handshake's existing architecture. The goal is to absorb Heaper's strengths - particularly its "block-as-unit-of-meaning" information model, relational organization via links/tags/mentions, and cache-tiered media browsing - without importing Heaper's stack, deployment model, or limitations.

  - A **pattern integration spec** that translates Heaper's product concepts into Handshake-native schemas, requirements, and roadmap items.
  - A **gap analysis** identifying where Heaper's features fill genuine holes in Handshake's current specification.
  - A **PostgreSQL expansion plan** showing how Handshake's database-portable architecture can implement these patterns at a level Heaper's own stack cannot reach.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Loom search API backend-agnostic contract and portable schema continuation
- CONTEXT_START_LINE: 62794
- CONTEXT_END_LINE: 63004
- CONTEXT_TOKEN: **[LM-SEARCH-001]** The search API MUST be backend-agnostic.
- EXCERPT_ASCII_ESCAPED:
  ```text
  **[LM-SEARCH-001]** The search API MUST be backend-agnostic. The storage trait exposes `search_loom_blocks(query, filters) -> Vec<LoomBlockSearchResult>`. The implementation varies by backend.

  **[LM-SEARCH-002]** On PostgreSQL, search results MUST be filterable by graph relationships (tags, mentions, backlink depth) within the query. This is a key improvement over Heaper's client-side-only search.

  **[LM-GRAPH-001]** Graph traversal queries MUST work on both SQLite (using recursive CTEs, available since SQLite 3.35+) and PostgreSQL. Performance targets: <100ms for 3-hop traversal on 10K blocks (SQLite), <50ms on PostgreSQL.

  ##### 11.1 Schema (Portable - SQLite and PostgreSQL)

  All schemas follow \u00A72.3.13 Storage Backend Portability requirements:
  - No `strftime()` or SQLite-specific functions.
  - Portable placeholder syntax (`$1`, `$2`).
  - No triggers - application-layer mutation tracking.
  - TIMESTAMP instead of DATETIME.

  -- LoomBlocks table
  CREATE TABLE loom_blocks (
      block_id UUID PRIMARY KEY,
      workspace_id UUID NOT NULL,
      content_type TEXT NOT NULL,       -- 'note', 'file', 'annotated_file', 'tag_hub', 'journal'
      document_id UUID,                 -- FK to documents table (nullable)
      asset_id UUID,                    -- FK to assets table (nullable)
      title TEXT,
      original_filename TEXT,
      content_hash TEXT,                -- SHA-256 hex
      pinned BOOLEAN NOT NULL DEFAULT FALSE,
      journal_date TEXT,                -- ISO date string
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      imported_at TIMESTAMP
  );

  -- LoomEdges table (Knowledge Graph edges for Loom features)
  CREATE TABLE loom_edges (
      edge_id UUID PRIMARY KEY,
      source_block_id UUID NOT NULL REFERENCES loom_blocks(block_id),
      target_block_id UUID NOT NULL REFERENCES loom_blocks(block_id),
      edge_type TEXT NOT NULL,         -- 'mention', 'tag', 'sub_tag', 'parent', 'ai_suggested'
      created_by TEXT NOT NULL,        -- 'user' or 'ai'
      crdt_site_id TEXT,
      source_anchor_doc_id UUID,
      source_anchor_block_id UUID,
      source_anchor_offset_start INTEGER,
      source_anchor_offset_end INTEGER,
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

  trait LoomStorage {
      // Search (backend-adaptive)
      async fn search_loom_blocks(&self, workspace_id: UUID, query: &str, filters: &LoomSearchFilters) -> Result<Vec<LoomBlockSearchResult>>;

      // Graph traversal
      async fn traverse_graph(&self, start_block_id: UUID, max_depth: u32, edge_types: &[LoomEdgeType]) -> Result<Vec<(LoomBlock, u32)>>;

      // Metrics (derived, rebuildable)
      async fn recompute_block_metrics(&self, block_id: UUID) -> Result<()>;
      async fn recompute_all_metrics(&self, workspace_id: UUID) -> Result<()>;
  }
  ```
