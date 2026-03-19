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
- WP_ID: WP-1-Loom-Storage-Portability-v3
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-19T05:55:23Z
- SPEC_TARGET_RESOLVED: ../wt-gov-kernel/.GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: 48a826af4092c8773cdd7ca17ddc2e22491196d1
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja190320260922
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Loom-Storage-Portability-v3
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- v2 passed validator but on direct code inspection against master spec v02.178, the following concrete gaps remain on current `main`:
- **Missing `traverse_graph` trait method**: Spec section 10.12 ([LM-GRAPH-001]) requires `traverse_graph(start_block_id, max_depth, edge_types) -> Vec<(LoomBlock, u32)>` with recursive CTE implementation on both SQLite and PostgreSQL. Performance targets: <100ms for 3-hop traversal on 10K blocks (SQLite), <50ms on PostgreSQL. The Storage trait in `storage/mod.rs` does not declare this method, and neither `sqlite.rs` nor `postgres.rs` implements it.
- **Missing `recompute_block_metrics` and `recompute_all_metrics` trait methods**: Spec section 10.12 requires `recompute_block_metrics(block_id)` and `recompute_all_metrics(workspace_id)` as part of the LoomStorage trait for derived/rebuildable metrics. Neither the trait nor any implementation contains these methods.
- **Missing `get_backlinks` and `get_outgoing_edges` trait methods**: Spec section 2.3.13.7 defines `get_backlinks(block_id) -> Vec<LoomEdge>` and `get_outgoing_edges(block_id) -> Vec<LoomEdge>` in the LoomStorage trait. The current Storage trait only has `list_loom_edges_for_block` which is a single method, not the two directional queries the spec requires.
- **LM-SEARCH-002 graph-filtered search on PostgreSQL**: Spec requires that on PostgreSQL, search results MUST be filterable by graph relationships (tags, mentions, backlink depth) within the query. Current `LoomSearchFilters` struct does not expose graph-relationship filter fields, and the Postgres `search_loom_blocks` implementation does not join against `loom_edges` for graph-aware filtering.
- **LoomSourceAnchor export/replay round-trip proof**: Spec requires LoomSourceAnchor to survive export and replay across backends. No conformance test verifies that source anchors round-trip correctly through SQLite and PostgreSQL storage, export, and reimport paths.
- The v2 remediation pass was judged underperformed by operator code inspection. This v3 pass must close the above gaps with implementation and parity proof, not just audit narrative.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 90m
- SEARCH_SCOPE: official backend-search docs, official cloud full-text search docs, typed lineage/provenance specs, recent provenance indexing research, OSS repository patterns for backend-adaptive search layers, graph traversal in relational databases, and local Handshake storage/API/migration code
- REFERENCES: SQLite FTS5 docs; PostgreSQL full text search docs; Google Cloud Spanner full-text search overview; OpenLineage spec docs; OpenLineage repository; In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines; pgvector repository; SQLite recursive CTE docs; PostgreSQL recursive CTE docs; current Handshake spec and Loom storage code
- PATTERNS_EXTRACTED: backend-adaptive search internals behind one stable API; typed source/provenance payloads that survive export and replay; provider-local optimization kept out of portable schema law; cross-provider conformance tests focused on semantic parity rather than identical query plans; recursive CTE graph traversal with depth tracking on both SQLite and PostgreSQL
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT backend-specific search internals behind one stable Loom storage contract; ADOPT recursive CTE graph traversal with provider-local performance tuning; ADAPT lineage-style typed provenance into explicit `source_anchor` and export/replay expectations; REJECT provider-specific ranking or vector-only enhancements as canonical Loom meaning in Phase 1
- LICENSE/IP_NOTES: Reference-only review of official documentation and OSS repositories. No third-party code is intended for direct copy into product code.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.178.md already defines portability pillars, Loom storage trait expectations, backend-agnostic search rules, graph traversal requirements, metrics methods, and canonical portable Loom contracts. This WP is implementation and conformance hardening against the current Main Body.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON_NO: N/A
- SOURCE_MAX_AGE_DAYS: 730
- SOURCE_LOG:
  - Source: SQLite FTS5 docs | Kind: OSS_DOC | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://sqlite.org/fts5.html | Why: canonical reference for SQLite-side full-text search behavior and index locality
  - Source: PostgreSQL full text search docs | Kind: OSS_DOC | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://www.postgresql.org/docs/current/textsearch.html | Why: canonical reference for PostgreSQL-side ranked text search and backend-specific query power
  - Source: Google Cloud Spanner full-text search overview | Kind: BIG_TECH | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://cloud.google.com/spanner/docs/full-text-search | Why: current large-scale vendor reference showing richer backend-specific search features can exist behind one SQL-facing search surface
  - Source: OpenLineage spec docs | Kind: OSS_DOC | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://openlineage.io/docs/spec/ | Why: useful reference for typed lineage and provenance payloads that survive transport and backend changes
  - Source: OpenLineage repository | Kind: GITHUB | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://github.com/OpenLineage/OpenLineage | Why: concrete repository-scale example of typed lineage/provenance contract evolution
  - Source: In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines | Kind: PAPER | Date: 2025-11-05 | Retrieved: 2026-03-19T05:55:23Z | URL: https://arxiv.org/abs/2511.03480 | Why: recent provenance-indexing paper supporting explicit, queryable source/provenance structures instead of opaque backend-local metadata
  - Source: pgvector repository | Kind: GITHUB | Date: 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | URL: https://github.com/pgvector/pgvector | Why: high-signal reference for backend-specific search acceleration that should remain optional rather than canonical in this packet
  - Source: SQLite recursive CTE docs | Kind: OSS_DOC | Date: 2026-03-19 | Retrieved: 2026-03-19T05:55:23Z | URL: https://sqlite.org/lang_with.html | Why: canonical reference for recursive CTE syntax and performance characteristics on SQLite 3.35+
  - Source: PostgreSQL recursive CTE docs | Kind: OSS_DOC | Date: 2026-03-19 | Retrieved: 2026-03-19T05:55:23Z | URL: https://www.postgresql.org/docs/current/queries-with.html | Why: canonical reference for recursive CTE graph traversal performance on PostgreSQL
- RESEARCH_SYNTHESIS:
  - A portability packet should preserve one stable API and semantic contract while allowing provider-specific indexing and query plans behind the boundary.
  - Big-tech search systems confirm that richer backend-specific ranking, tokenization, and query expansion can stay behind a stable query surface instead of redefining canonical filter meaning.
  - Typed provenance payloads are more durable than ad hoc search or edge metadata and map well to Loom `source_anchor` export/replay expectations.
  - Recursive CTEs are well-supported on both SQLite (3.35+) and PostgreSQL with predictable depth-tracking semantics, making graph traversal portable without provider-specific branching.
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
  - Source: SQLite recursive CTE docs | Pattern: depth-limited recursive traversal with cycle detection | Why: directly maps to the spec-required `traverse_graph` method with portable CTE syntax across both backends
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
  - recursive CTE graph traversal rust repo
- MATCHED_PROJECTS:
  - Source: OpenLineage repository | Repo: OpenLineage/OpenLineage | URL: https://github.com/OpenLineage/OpenLineage | Intent: ARCH_PATTERN | Decision: ADAPT | Impact: NONE | Stub: NONE | Notes: useful reference for typed lineage/provenance contract evolution, especially for portable `source_anchor` and export/replay semantics
  - Source: pgvector repository | Repo: pgvector/pgvector | URL: https://github.com/pgvector/pgvector | Intent: IMPLEMENTATION | Decision: TRACK_ONLY | Impact: NONE | Stub: NONE | Notes: helpful future reference for Postgres-only retrieval acceleration, but out of scope for canonical portability law in this packet
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Reuse existing Loom event families; no new Flight Recorder ids are needed for portability activation alone.
- Loom block and edge CRUD should continue to emit the existing Loom lifecycle events.
- Import dedup flow should continue to emit the existing `loom_dedup_hit` event.
- Graph traversal should emit existing navigation events; no new FR id is needed since traverse_graph is a query, not a mutation.
- Metrics recomputation should emit a lightweight completion event reusing the existing Loom event family.
- View and search parity checks should continue to rely on the existing Loom view and search events so backend changes do not hide behavior regressions.
- Portability fixes should keep event payload meaning backend-neutral even if query implementation differs underneath.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: provider-specific search logic changes filter meaning across backends. Mitigation: assert semantic parity in shared Loom conformance tests rather than trusting query-shape similarity.
- Risk: source-anchor payloads are dropped or partially serialized on one backend. Mitigation: make `LoomSourceAnchor` round-trip tests part of portability coverage.
- Risk: portability refactors widen into filesystem or workflow-runtime changes. Mitigation: keep this packet centered on storage contracts, migrations, blob-path stability, and parity tests.
- Risk: backend-specific acceleration becomes the only supported semantics. Mitigation: treat SQLite and PostgreSQL parity as the contract and relegate backend-only improvements to optional enhancements.
- Risk: recursive CTE graph traversal produces unbounded result sets or cycles. Mitigation: enforce max_depth parameter and cycle detection in both backend implementations.

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
  - The spec already names the Loom storage-contract primitives. The v3 gap is missing trait methods (traverse_graph, recompute_*_metrics, get_backlinks, get_outgoing_edges), graph-filtered search on PostgreSQL, and source-anchor round-trip proof.

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
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: Loom is a durable library surface and this packet hardens how blocks, edges, and asset references survive backend swaps, export, and replay; graph traversal is a core archivist capability | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: view and search filters are part of the portable library contract and must preserve meaning across backends; graph-filtered search extends librarian capability | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytic consumers may read Loom later, but this packet stops at storage parity and conformance | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no external dataset ingestion or wrangling contract is changed directly | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: portable DDL, migration replay safety, dual-backend parity, recursive CTE performance, and provider-local index choices are direct DBA concerns in this packet | STUB_WP_IDS: NONE
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
  - PILLAR: Loom | STATUS: TOUCHED | NOTES: Loom block, edge, search, view, graph traversal, metrics, source-anchor, and asset-path contracts are the direct subject of this packet | STUB_WP_IDS: NONE
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
  - PILLAR: Loom | CAPABILITY_SLICE: Graph traversal and metrics recomputation | SUBFEATURES: `traverse_graph` with recursive CTE, `recompute_block_metrics`, `recompute_all_metrics`, `get_backlinks`, `get_outgoing_edges` | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomBlockDerived | MECHANICAL: engine.archivist, engine.dba, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: these are spec-required trait methods currently missing from both the trait definition and all implementations
  - PILLAR: Loom | CAPABILITY_SLICE: Portable view, search, and source-anchor contract | SUBFEATURES: `LoomViewFilters`, `LoomSearchFilters`, `LoomBlockSearchResult`, and `LoomSourceAnchor` parity; graph-filtered search on PostgreSQL | PRIMITIVES_FEATURES: PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult, PRIM-LoomSourceAnchor | MECHANICAL: engine.librarian, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the API contract should preserve the same filter meaning and source-anchor durability across both backends; PostgreSQL search must support graph-relationship filters per LM-SEARCH-002
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Loom migration and DDL portability | SUBFEATURES: replay-safe migrations, down migrations, provider-local indexes outside portable DDL, and no trigger-dependent semantics | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the direct portability law bridge from spec to code for the Loom surface
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Cross-provider Loom conformance coverage | SUBFEATURES: shared test helpers for SQLite and PostgreSQL parity over CRUD, search, view, dedup, graph traversal, metrics, and anchor round-trips | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | MECHANICAL: engine.archivist, engine.librarian, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity should be proven by tests, not inferred from provider implementations
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Loom block and edge storage parity | JobModel: UI_ACTION | Workflow: loom_surface_crud | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_block_created, loom_block_updated, loom_block_deleted, loom_edge_created, loom_edge_deleted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend parity must preserve CRUD meaning and existing Loom telemetry regardless of provider implementation
  - Capability: Loom graph traversal | JobModel: UI_ACTION | Workflow: loom_graph_traverse | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_graph_traversed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: recursive CTE traversal must meet spec performance targets on both backends; results drive backlink navigation and graph exploration
  - Capability: Loom metrics recomputation | JobModel: MECHANICAL_TOOL | Workflow: loom_metrics_recompute | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_metrics_recomputed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: rebuildable derived metrics must not become migration-coupled state
  - Capability: Loom import and dedup portability | JobModel: WORKFLOW | Workflow: loom_import | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_dedup_hit, loom_block_created | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: content-hash dedup, asset-path layout, and import-created blocks must preserve the same semantics across backends
  - Capability: Loom view portability | JobModel: UI_ACTION | Workflow: loom_view_query | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_view_queried | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: `LoomViewFilters` and grouped-view semantics must not drift when the backend changes
  - Capability: Loom search portability with graph filtering | JobModel: UI_ACTION | Workflow: loom_search | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_search_executed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: PostgreSQL search must support LM-SEARCH-002 graph-relationship filtering; SQLite search preserves stable filter meaning
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Scanned for high-ROI appendix matrix additions and found that current v02.178 already records Loom as a portable backend library surface and ties it to storage portability.
  - The activation need is implementation of missing trait methods, graph-filtered search, and conformance coverage, not a new appendix interaction edge.
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
  - Source: PostgreSQL full text search docs | Kind: OSS_DOC | Angle: ranked relational search with richer backend capabilities | Pattern: more powerful backend query plan behind the same result surface | Decision: ADOPT | EngineeringTrick: allow richer Postgres internals including graph-relationship joins while keeping filter meaning and result identity stable across providers | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly maps to the spec rule that PostgreSQL may do more internally without redefining the API; LM-SEARCH-002 requires graph-filtered search on Postgres
  - Source: SQLite recursive CTE docs | Kind: OSS_DOC | Angle: depth-limited graph traversal on SQLite | Pattern: recursive CTE with UNION ALL and depth counter | Decision: ADOPT | EngineeringTrick: use WITH RECURSIVE for traverse_graph with explicit depth tracking and max_depth limit; cycle detection via visited set | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly implements spec [LM-GRAPH-001] requirement
  - Source: PostgreSQL recursive CTE docs | Kind: OSS_DOC | Angle: optimized recursive graph traversal on PostgreSQL | Pattern: recursive CTE with hash-based cycle detection and index-accelerated joins | Decision: ADOPT | EngineeringTrick: leverage PostgreSQL's native cycle detection (CYCLE clause) and index on loom_edges for performance target <50ms | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly implements spec [LM-GRAPH-001] PostgreSQL performance target
  - Source: OpenLineage spec docs | Kind: OSS_DOC | Angle: typed provenance and transport-stable lineage payloads | Pattern: explicit typed provenance instead of adapter-local metadata blobs | Decision: ADAPT | EngineeringTrick: treat `LoomSourceAnchor` as a durable typed contract that survives storage, export, and replay | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: useful model for source-anchor portability and replay safety
  - Source: pgvector repository | Kind: GITHUB | Angle: backend-specific retrieval acceleration | Pattern: optional provider-local acceleration that should not redefine canonical semantics | Decision: REJECT | EngineeringTrick: do not let future Postgres-only acceleration become the only definition of Loom search meaning | ROI: MEDIUM | Resolution: REJECT_DUPLICATE | Stub: NONE | Notes: useful future reference, but out of scope for Phase 1 portability law
- MATRIX_GROWTH_CANDIDATES:
  - Combo: Stable search API plus provider-local indexing with graph filtering | Sources: SQLite FTS5 docs, PostgreSQL full text search docs | WhatToSteal: backend-specific indexing and query plans behind one result contract; graph-relationship joins on PostgreSQL | HandshakeCarryOver: preserve one `search_loom_blocks` API and semantic filter meaning while letting SQLite and PostgreSQL optimize differently underneath; PostgreSQL adds graph-filtered search per LM-SEARCH-002 | RuntimeConsequences: Loom search remains portable and future RAG consumers do not need provider-specific branching | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the main portability pattern for search in the packet
  - Combo: Portable graph traversal with recursive CTEs | Sources: SQLite recursive CTE docs, PostgreSQL recursive CTE docs | WhatToSteal: depth-limited recursive traversal with portable SQL syntax | HandshakeCarryOver: one `traverse_graph` trait method with portable CTE implementation meeting spec performance targets on both backends | RuntimeConsequences: graph navigation, backlink exploration, and downstream Loom bridge work get a stable foundation | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is a wholly new trait method required by the spec but absent from current code
  - Combo: Portable source-anchor lineage plus export/replay durability | Sources: OpenLineage spec docs, OpenLineage repository | WhatToSteal: typed provenance structures with explicit contract ownership | HandshakeCarryOver: keep `LoomSourceAnchor` round-trippable and testable across storage, export, and replay paths | RuntimeConsequences: backlinks, context snippets, and downstream Loom bridge work can trust anchors after backend changes | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is the highest-leverage typed-contract pattern outside search parity
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep provider-specific FTS and ranking logic inside backend implementations, not in portable migrations.
  - Assert semantic parity through shared Loom conformance tests rather than comparing SQL query text.
  - Treat `LoomSourceAnchor` and view/search filters as portable contract structs, not adapter-only shapes.
  - Use WITH RECURSIVE for graph traversal on both backends with explicit depth counter and max_depth guard.
  - Implement rebuildable metrics as application-layer recomputation, not migration-coupled state.
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: One Loom storage contract plus dual provider implementations | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.dba, engine.version | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | Resolution: IN_THIS_WP | Stub: NONE | Notes: the packet should make one semantic contract survive both SQLite and PostgreSQL without adapter drift
  - Combo: Graph traversal with portable recursive CTEs | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.dba, engine.librarian | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge | Resolution: IN_THIS_WP | Stub: NONE | Notes: spec requires traverse_graph on both backends with specific performance targets; currently missing entirely
  - Combo: Rebuildable derived metrics plus provider-local indexes | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.dba, engine.version | Primitives/Features: PRIM-LoomBlockDerived, PRIM-LoomEdge, PRIM-LoomBlock | Resolution: IN_THIS_WP | Stub: NONE | Notes: recompute_block_metrics and recompute_all_metrics are spec-required trait methods currently missing
  - Combo: Directional edge queries (backlinks + outgoing) | Pillars: Loom | Mechanical: engine.archivist, engine.librarian | Primitives/Features: PRIM-LoomEdge, PRIM-LoomBlock | Resolution: IN_THIS_WP | Stub: NONE | Notes: spec defines get_backlinks and get_outgoing_edges as separate methods; current code only has list_loom_edges_for_block
  - Combo: SQLite FTS and PostgreSQL text search behind one API with graph filtering | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.librarian, engine.dba, engine.version | Primitives/Features: PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult | Resolution: IN_THIS_WP | Stub: NONE | Notes: LM-SEARCH-002 requires graph-filtered search on PostgreSQL; search scoring may differ but filters and result identity must stay portable
  - Combo: View filter parity across providers | Pillars: Loom | Mechanical: engine.librarian, engine.dba | Primitives/Features: PRIM-LoomViewFilters, PRIM-LoomBlock, PRIM-LoomEdge | Resolution: IN_THIS_WP | Stub: NONE | Notes: grouped and filtered Loom views should preserve the same meaning across both backends
  - Combo: Source-anchor durability across storage, export, and replay | Pillars: Loom | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-LoomSourceAnchor, PRIM-LoomEdge | Resolution: IN_THIS_WP | Stub: NONE | Notes: backlink context and mention/tag provenance depend on anchors staying portable
  - Combo: Shared Loom conformance tests over SQLite and PostgreSQL | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.librarian, engine.dba | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | Resolution: IN_THIS_WP | Stub: NONE | Notes: parity should be demonstrated by tests over the public contract including graph traversal, metrics, and directional edge queries
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
  - Artifact: WP-1-Loom-MVP-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Loom MVP delivered the first implementation surface, but traverse_graph, metrics, directional edge queries, and graph-filtered search are still missing
  - Artifact: WP-1-Storage-Abstraction-Layer-v3 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: trait-based storage plumbing exists, but Loom-specific methods required by spec are still absent from the trait
  - Artifact: WP-1-Artifact-System-Foundations-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: artifact foundations exist, but Loom-specific asset-path and replay portability still need dedicated hardening
  - Artifact: WP-1-Loom-Storage-Portability-v2 | BoardStatus: SUPERSEDED | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: v2 passed validator but code inspection revealed missing trait methods and incomplete parity; v3 supersedes with concrete gap closure
- CODE_REALITY_EVIDENCE:
  - Path: src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Loom-Storage-Portability-v2 | Covers: execution | Verdict: PARTIAL | Notes: v2 landed CRUD/search/view methods but trait is missing traverse_graph, recompute_*_metrics, get_backlinks, get_outgoing_edges
  - Path: src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Loom-Storage-Portability-v2 | Covers: execution | Verdict: IMPLEMENTED | Notes: v2 delivered working CRUD, search, view, and dedup methods on the Storage trait for both SQLite and PostgreSQL
  - Path: src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Storage-Abstraction-Layer-v3 | Covers: execution | Verdict: PARTIAL | Notes: Storage trait declares Loom CRUD, search, and view methods but is missing traverse_graph, recompute_block_metrics, recompute_all_metrics, get_backlinks, and get_outgoing_edges
  - Path: src/backend/handshake_core/src/storage/loom.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: primitive | Verdict: PARTIAL | Notes: canonical Loom structs exist; LoomSearchFilters lacks graph-relationship filter fields
  - Path: src/backend/handshake_core/src/storage/sqlite.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: execution | Verdict: PARTIAL | Notes: SQLite implementation includes FTS5 and CRUD but lacks traverse_graph (recursive CTE), metrics recomputation, and directional edge queries
  - Path: src/backend/handshake_core/src/storage/postgres.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: execution | Verdict: PARTIAL | Notes: PostgreSQL implementation exists but lacks traverse_graph, metrics, directional edges, and LM-SEARCH-002 graph-filtered search
  - Path: src/backend/handshake_core/src/storage/tests.rs | Artifact: NONE | Covers: execution | Verdict: NOT_PRESENT | Notes: current storage conformance helpers do not cover graph traversal, metrics, directional edges, graph-filtered search, or source-anchor round-trips
  - Path: src/backend/handshake_core/src/api/loom.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: execution | Verdict: PARTIAL | Notes: API endpoints exist for CRUD/search/view but no graph traversal or metrics recomputation endpoints
  - Path: src/backend/handshake_core/src/loom_fs.rs | Artifact: NONE | Covers: execution | Verdict: PARTIAL | Notes: blob path layout is part of the portable contract and needs to remain stable as backends change
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: v2 passed validator but direct code inspection against spec v02.178 revealed 5 concrete missing trait methods, incomplete graph-filtered search, and absent conformance tests. v3 expands scope to close these specific gaps.

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
- WHAT: Implement the 5 missing Loom storage trait methods (traverse_graph, recompute_block_metrics, recompute_all_metrics, get_backlinks, get_outgoing_edges), add LM-SEARCH-002 graph-filtered search on PostgreSQL, and prove LoomSourceAnchor export/replay round-trip parity across both backends with shared conformance tests.
- WHY: v2 passed validator but operator code inspection against spec v02.178 revealed 5 missing trait methods, incomplete graph-filtered search, and absent source-anchor round-trip proof. This v3 closes those concrete spec compliance gaps.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/migrations/
- OUT_OF_SCOPE:
  - structured-collaboration schema registry and mailbox export schema work
  - frontend Loom viewer UX and preview-surface layout changes
  - preview job protocol redesign or broad workflow-runtime refactors
  - downloader-to-Loom and video-archive bridge behavior beyond the base portability contract
  - capability-registry publication logic unrelated to Loom storage parity
- TEST_PLAN:
  ```bash
  cargo test -p handshake_core loom
  cargo test -p handshake_core --test storage_conformance
  just gov-check
  ```
- DONE_MEANS:
  - `traverse_graph` trait method exists on Storage trait with SQLite (recursive CTE) and PostgreSQL implementations meeting spec performance targets (<100ms 3-hop on 10K blocks SQLite, <50ms PostgreSQL).
  - `recompute_block_metrics` and `recompute_all_metrics` trait methods exist on Storage trait with both backend implementations.
  - `get_backlinks` and `get_outgoing_edges` trait methods exist as separate directional edge queries on Storage trait with both backend implementations.
  - PostgreSQL `search_loom_blocks` supports graph-relationship filtering (tags, mentions, backlink depth) per LM-SEARCH-002.
  - `LoomSourceAnchor` round-trip tests prove export/replay durability across both backends in shared conformance suite.
  - Shared Loom conformance tests cover graph traversal, metrics recomputation, directional edges, graph-filtered search, and source-anchor round-trips.
  - No unrelated product families are touched; changes stay inside Loom storage/API/test surface.
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
  - .GOV/spec/Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - traverse_graph
  - recompute_block_metrics
  - recompute_all_metrics
  - get_backlinks
  - get_outgoing_edges
  - create_loom_block
  - create_loom_edge
  - query_loom_view
  - search_loom_blocks
  - LoomSourceAnchor
  - LoomSearchFilters
  - loom_blocks
  - loom_edges
  - loom_blocks_fts
- RUN_COMMANDS:
  ```bash
  rg -n "traverse_graph|recompute_block_metrics|recompute_all_metrics|get_backlinks|get_outgoing_edges|create_loom_block|search_loom_blocks|LoomSourceAnchor|LoomSearchFilters" src/backend/handshake_core
  cargo test -p handshake_core loom
  cargo test -p handshake_core --test storage_conformance
  just gov-check
  ```
- RISK_MAP:
  - "traverse_graph recursive CTE hits cycle or exceeds depth" -> "unbounded result sets or infinite loops in graph navigation"
  - "provider-specific search logic changes filter meaning" -> "view and search parity break across SQLite and PostgreSQL"
  - "source anchors fail to round-trip on one backend" -> "backlinks, context snippets, and downstream bridge packets lose stable provenance"
  - "LM-SEARCH-002 graph-filtered search widens Postgres search beyond portable contract" -> "Postgres-only search semantics become de facto requirement"
  - "the remediation pass drifts into unrelated runtime families" -> "the packet loses file-lock isolation and repeats the earlier live-smoke scope failure"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - Current stub metadata and BUILD_ORDER ranking already match this activation target. No build-order edit is required unless scope expands beyond the portability boundary.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: [LM-GRAPH-001] Graph traversal with recursive CTEs on both backends | WHY_IN_SCOPE: spec requires traverse_graph with performance targets; method is completely absent from current code | EXPECTED_CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | EXPECTED_TESTS: storage_conformance::loom_traverse_graph_depth_limit, storage_conformance::loom_traverse_graph_cycle_detection | RISK_IF_MISSED: graph navigation and backlink exploration have no backend-portable foundation
  - CLAUSE: [LM-SEARCH-002] PostgreSQL search filterable by graph relationships | WHY_IN_SCOPE: spec requires PostgreSQL search to support tag/mention/backlink-depth filtering; current LoomSearchFilters lacks these fields | EXPECTED_CODE_SURFACES: storage/loom.rs (LoomSearchFilters), postgres.rs (search_loom_blocks) | EXPECTED_TESTS: storage_conformance::loom_search_graph_filter_postgres | RISK_IF_MISSED: PostgreSQL search lacks spec-mandated graph-aware filtering capability
  - CLAUSE: 2.3.13.7 get_backlinks and get_outgoing_edges | WHY_IN_SCOPE: spec defines two directional edge query methods; current code only has list_loom_edges_for_block | EXPECTED_CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | EXPECTED_TESTS: storage_conformance::loom_directional_edge_queries | RISK_IF_MISSED: downstream consumers cannot distinguish incoming from outgoing edges portably
  - CLAUSE: 2.3.13.7 recompute_block_metrics and recompute_all_metrics | WHY_IN_SCOPE: spec requires rebuildable derived metrics methods; absent from trait and all implementations | EXPECTED_CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | EXPECTED_TESTS: storage_conformance::loom_metrics_recompute_idempotent | RISK_IF_MISSED: derived metrics become migration-coupled state instead of rebuildable
  - CLAUSE: [CX-DBP-011] Portable schema and migrations | WHY_IN_SCOPE: any new Loom DDL must follow portable SQL rules; existing migrations must stay replay-safe | EXPECTED_CODE_SURFACES: migrations/ | EXPECTED_TESTS: cargo test -p handshake_core loom migration | RISK_IF_MISSED: new methods introduce SQLite-specific or Postgres-specific schema that breaks portability
  - CLAUSE: 2.3.13.7 LoomSourceAnchor export/replay durability | WHY_IN_SCOPE: spec requires source anchors to survive export and replay across backends; no conformance test exists | EXPECTED_CODE_SURFACES: storage/loom.rs, sqlite.rs, postgres.rs | EXPECTED_TESTS: storage_conformance::loom_source_anchor_round_trip | RISK_IF_MISSED: source-anchor provenance is silently lost on backend swap or export/reimport

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: Storage trait LoomStorage methods | PRODUCER: sqlite.rs, postgres.rs | CONSUMER: api/loom.rs, workflows.rs | SERIALIZER_TRANSPORT: in-process Rust trait dispatch | VALIDATOR_READER: storage_conformance tests | TRIPWIRE_TESTS: storage_conformance::loom_* | DRIFT_RISK: trait method signatures drift between trait declaration and provider implementations
  - CONTRACT: LoomSearchFilters struct | PRODUCER: api/loom.rs (from HTTP params) | CONSUMER: sqlite.rs, postgres.rs (search_loom_blocks) | SERIALIZER_TRANSPORT: serde JSON | VALIDATOR_READER: storage_conformance tests | TRIPWIRE_TESTS: storage_conformance::loom_search_filter_parity | DRIFT_RISK: graph-relationship filter fields added to struct but not wired in one backend
  - CONTRACT: LoomSourceAnchor struct | PRODUCER: api/loom.rs (edge creation) | CONSUMER: sqlite.rs, postgres.rs (edge storage), export/replay paths | SERIALIZER_TRANSPORT: serde JSON to DB columns | VALIDATOR_READER: storage_conformance tests | TRIPWIRE_TESTS: storage_conformance::loom_source_anchor_round_trip | DRIFT_RISK: anchor fields lost during serialization on one backend
  - CONTRACT: traverse_graph result shape | PRODUCER: sqlite.rs, postgres.rs | CONSUMER: api/loom.rs, future graph navigation UI | SERIALIZER_TRANSPORT: in-process Vec<(LoomBlock, u32)> | VALIDATOR_READER: storage_conformance tests | TRIPWIRE_TESTS: storage_conformance::loom_traverse_graph_* | DRIFT_RISK: depth values or cycle handling differs between backends
  - CONTRACT: Loom migration DDL | PRODUCER: migrations/*.sql | CONSUMER: sqlx migrate | SERIALIZER_TRANSPORT: SQL files | VALIDATOR_READER: migration replay test | TRIPWIRE_TESTS: cargo test migration_replay | DRIFT_RISK: new columns or indexes use provider-specific syntax

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - storage_conformance::loom_traverse_graph_depth_limit -- proves traverse_graph stops at max_depth on both backends
  - storage_conformance::loom_traverse_graph_cycle_detection -- proves cycle detection prevents infinite loops
  - storage_conformance::loom_traverse_graph_edge_type_filter -- proves edge_types parameter filters correctly
  - storage_conformance::loom_search_graph_filter_postgres -- proves LM-SEARCH-002 graph-relationship filtering on PostgreSQL
  - storage_conformance::loom_directional_edge_queries -- proves get_backlinks and get_outgoing_edges return correct subsets
  - storage_conformance::loom_metrics_recompute_idempotent -- proves recompute is idempotent and matches fresh computation
  - storage_conformance::loom_source_anchor_round_trip -- proves LoomSourceAnchor survives write/read/export/reimport on both backends
- CANONICAL_CONTRACT_EXAMPLES:
  - Golden traverse_graph result for a known 3-hop graph on 10+ blocks
  - Golden LoomSourceAnchor JSON that must round-trip identically through both backends
  - Golden LoomSearchFilters with graph-relationship fields for PostgreSQL-specific test

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Add traverse_graph, recompute_block_metrics, recompute_all_metrics, get_backlinks, get_outgoing_edges to Storage trait in mod.rs
  - Implement all 5 methods in sqlite.rs using recursive CTEs and application-layer logic
  - Implement all 5 methods in postgres.rs using recursive CTEs with PostgreSQL optimizations
  - Add graph-relationship filter fields to LoomSearchFilters in loom.rs
  - Wire graph-filtered search in postgres.rs search_loom_blocks implementation
  - Add API endpoints for graph traversal and metrics recomputation in api/loom.rs
  - Write shared Loom conformance tests covering all new methods and source-anchor round-trips
  - Verify all existing Loom tests still pass
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core loom
  - cargo test -p handshake_core --test storage_conformance
- CARRY_FORWARD_WARNINGS:
  - v2 passed validator but failed operator code inspection; v3 must close gaps with actual implementation, not narrative
  - Do not widen scope beyond Loom storage/API surface; keep file-lock isolation
  - Performance targets from spec: traverse_graph <100ms 3-hop on 10K blocks (SQLite), <50ms (PostgreSQL)
  - LM-SEARCH-002 graph filtering is PostgreSQL-only; SQLite search keeps existing behavior
  - All new methods must follow portable SQL rules (no strftime, no SQLite triggers, $1/$2 placeholders)

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - [LM-GRAPH-001] traverse_graph exists on trait + both implementations with recursive CTE
  - [LM-SEARCH-002] PostgreSQL search_loom_blocks accepts and uses graph-relationship filters
  - 2.3.13.7 get_backlinks and get_outgoing_edges exist as separate trait methods
  - 2.3.13.7 recompute_block_metrics and recompute_all_metrics exist on trait + both implementations
  - [CX-DBP-011] Any new migrations follow portable SQL rules
  - LoomSourceAnchor round-trip proof in conformance tests
- FILES_TO_READ:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core loom
  - cargo test -p handshake_core --test storage_conformance
  - rg -n "traverse_graph|recompute_block_metrics|recompute_all_metrics|get_backlinks|get_outgoing_edges" src/backend/handshake_core/src/storage/
- POST_MERGE_SPOTCHECKS:
  - Verify traverse_graph recursive CTE syntax is portable (no provider-specific extensions)
  - Verify LoomSearchFilters graph fields are present in struct definition
  - Verify conformance tests run against both SQLite and PostgreSQL

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Exact traverse_graph performance on 10K-block datasets has not been benchmarked; spec targets are stated but actual performance depends on index strategy and query plan
  - LM-SEARCH-002 graph-filtered search may require additional Postgres indexes beyond current schema; exact index requirements are implementation-dependent
  - LoomSourceAnchor round-trip fidelity through external export/reimport paths (beyond direct DB write/read) depends on export format decisions not yet finalized in the codebase

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Main Body explicitly names Storage Backend Portability law, the Loom portable schema and storage-trait example including traverse_graph/recompute/backlinks/outgoing, the backend-agnostic Loom search rule with LM-SEARCH-002 graph filtering, and Loom as a portable backend library surface.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.178.md already defines the portability pillars, Loom storage contract including all required trait methods, backend-agnostic search expectations with LM-SEARCH-002, graph traversal requirements with LM-GRAPH-001, and portable Loom primitives in the Main Body and appendix notes. This packet is a code and conformance alignment pass.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

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
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Loom search API backend-agnostic contract, graph traversal, and metrics
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
