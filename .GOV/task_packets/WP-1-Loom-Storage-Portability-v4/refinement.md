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
- WP_ID: WP-1-Loom-Storage-Portability-v4
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-26T13:20:29Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: e658a3b8a2d7cdd0d294838151d24a60bc3e034c
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja260320261539
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Loom-Storage-Portability-v4
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Direct current-main inspection shows the old `v3` implementation-gap story is stale: the Storage trait now declares `get_backlinks`, `get_outgoing_edges`, `traverse_graph`, `recompute_block_metrics`, and `recompute_all_metrics`, and both SQLite and PostgreSQL implementations exist on current `main`.
- Current Loom code also contains the previously-missing API routes, graph-filtered PostgreSQL search fields, shared Loom conformance helpers, source-anchor round-trip coverage, and top-level SQLite/PostgreSQL conformance and traversal-performance entrypoints.
- The remaining gap is no longer "implement the missing Loom portability surface." The remaining gap is governed proof freshness: `v4` must revalidate current product code against Master Spec v02.178 and separate real remaining defects from stale packet history.
- This refinement pass did not rerun the PostgreSQL-backed Loom conformance and traversal-performance entrypoints, because those remain environment-gated by `POSTGRES_TEST_URL`. That evidence is still unproven in this pass even though the code and tests exist.
- No fresh current-main Loom defect has been demonstrated yet from this inspection. If validator-owned revalidation also finds no defect, `v4` must collapse to proof-only closure and status-sync instead of reopening broad implementation churn.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 120m
- SEARCH_SCOPE: current Master Spec portability clauses, current Loom storage trait/provider/API/test surfaces on `main`, and existing Loom `v1`/`v2`/`v3` governance artifacts
- REFERENCES: Handshake_Master_Spec_v02.178.md; current `handshake_core` Loom storage code; current `storage/tests.rs` and `tests/storage_conformance.rs`; existing Loom `v1`/`v2`/`v3` refinements, packets, and `v4` stub
- PATTERNS_EXTRACTED: current code plus current spec outrank historical packet narrative; portability closure must be proven by executable dual-backend evidence; proof-only packets are preferable to speculative code churn when current product code already matches the spec surface
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT current product-code-versus-spec inspection as the authority surface; ADAPT the old Loom portability lineage into a narrow proof/remediation `v4`; REJECT reopening the full `v3` implementation scope without a demonstrated current-main defect
- LICENSE/IP_NOTES: Internal product/spec/governance inspection only. No third-party code or external documentation is used in this refinement pass.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.178.md already defines the portability law, Loom trait surface, graph traversal, LM-SEARCH-002, source-anchor portability, and dual-backend testing requirements. `v4` is a current-main proof/remediation packet against existing law.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This `v4` refinement is strictly internal/mechanical and is grounded in current product code, the current Master Spec, and governed packet history. No external signal is needed to decide whether Loom portability is still open on current `main`.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE
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
- Reuse the existing Loom event families; this packet does not introduce new Flight Recorder ids.
- Any fresh remediation must preserve backend-neutral payload meaning for existing Loom search, traversal, view, and edge lifecycle events.
- If `v4` resolves as proof-only, Flight Recorder work is limited to confirming that current-main portability evidence does not contradict the existing event surfaces.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: stale packet history is mistaken for current product truth. Mitigation: require current code-and-test evidence before any closure claim survives `v4`.
- Risk: SQLite-only green tests get overstated as full portability closure. Mitigation: keep PostgreSQL evidence explicitly unproven until validator-owned runs execute.
- Risk: proof-only revalidation turns into broad speculative churn. Mitigation: constrain `v4` to fresh demonstrated defects or evidence repair only.
- Risk: helper-level drift inside search, traversal, or source-anchor handling hides behind top-level passing tests. Mitigation: inspect helper-level coverage and bind closure to clause-owned tripwires.

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
  - `v4` does not create a new Loom primitive family. It revalidates the current implementations of the existing portable Loom contracts and only remediates demonstrated drift.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current appendix already names the Loom block, edge, filter, search-result, and source-anchor primitives needed for this packet.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - `v4` is about proving or repairing current implementation depth against the existing primitive set, not inventing a new primitive family.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new high-signal orphan Loom primitives were discovered in this current-main inspection pass.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Existing appendix ownership notes already cover Loom as a portable backend library surface.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet is backend portability proof/remediation work. Direct Loom UI behavior remains downstream.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Current appendix interaction coverage is sufficient; `v4` is a proof/remediation pass, not a new cross-feature topology change.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep Appendix 12 unchanged and prove or repair current code against existing v02.178 portability law.
  - If current-main inspection uncovers a real missing appendix edge or primitive, handle that as a separate spec-update flow instead of broadening this packet silently.
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
  - PILLAR: Loom | CAPABILITY_SLICE: Current-main storage contract revalidation | SUBFEATURES: verify the present Storage trait, SQLite/PostgreSQL implementations, and Loom API already match the spec-owned contract surface before any new code is written | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomBlockDerived, PRIM-LoomSearchFilters | MECHANICAL: engine.archivist, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: `v4` starts from current code reality rather than reusing the stale `v3` implementation-gap narrative
  - PILLAR: Loom | CAPABILITY_SLICE: Graph traversal and directional edge proof | SUBFEATURES: `traverse_graph`, `get_backlinks`, `get_outgoing_edges`, recursive CTE depth behavior, and validator-owned performance evidence | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomEdgeType | MECHANICAL: engine.archivist, engine.librarian, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the current code appears implemented; `v4` must prove semantic depth on both backends and only patch if a live defect is demonstrated
  - PILLAR: Loom | CAPABILITY_SLICE: Search and source-anchor portability proof | SUBFEATURES: `LoomSearchFilters`, `LoomBlockSearchResult`, LM-SEARCH-002 graph filtering on PostgreSQL, and `LoomSourceAnchor` export/replay durability | PRIMITIVES_FEATURES: PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult, PRIM-LoomSourceAnchor | MECHANICAL: engine.librarian, engine.version, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the goal is to confirm that current filters and anchors stay portable across provider implementations and current tests
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Portable DDL and dual-backend evidence verification | SUBFEATURES: portable schema law, replay-safe migrations, top-level SQLite/PostgreSQL Loom conformance entrypoints, and traversal performance probes | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: `v4` should prove or explicitly mark as unproven the provider parity that earlier packets claimed
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Governance/evidence correction | SUBFEATURES: separate real current-main defects from historical smoke-test failure narrative and reduce the packet to proof-only closure when no fresh defect remains | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | MECHANICAL: engine.version, engine.dba, engine.archivist | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the key scope change from `v3` to `v4`
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Loom block and edge storage parity | JobModel: UI_ACTION | Workflow: loom_surface_crud | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_block_created, loom_block_updated, loom_block_deleted, loom_edge_created, loom_edge_deleted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: `v4` validates that current CRUD parity still holds on present code and only remediates fresh drift if found
  - Capability: Loom graph traversal and directional edge queries | JobModel: UI_ACTION | Workflow: loom_graph_traverse | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_graph_traversed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: current methods and routes exist on `main`; `v4` must prove cross-backend semantics and performance rather than re-implement blindly
  - Capability: Loom metrics recomputation | JobModel: MECHANICAL_TOOL | Workflow: loom_metrics_recompute | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_metrics_recomputed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: rebuildable derived metrics must remain provider-neutral and validator-proven
  - Capability: Loom search portability with graph filtering and source-anchor durability | JobModel: UI_ACTION | Workflow: loom_search | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_search_executed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: current Postgres graph filtering and source-anchor durability must be proven with live contract evidence before closure claims survive
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - Current appendix coverage already names Loom as a portable backend library surface and ties it to storage portability law.
  - `v4` does not need a new IMX edge to start; the open gap is governed proof freshness, not missing appendix topology.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Current appendix interaction coverage already captures the relevant Loom portability relationships; no new IMX edge is required before `v4` activation.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This packet is grounded in current Handshake code/spec/governance evidence. No external combo scan is needed before activation.
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
  - Combo: One Loom storage contract plus dual provider implementations plus shared conformance | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.dba, engine.version | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | Resolution: IN_THIS_WP | Stub: NONE | Notes: `v4` must prove that the current contract still survives both SQLite and PostgreSQL without adapter drift
  - Combo: Graph traversal plus directional edge queries plus performance targets | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.librarian, engine.dba | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomEdgeType | Resolution: IN_THIS_WP | Stub: NONE | Notes: current-main implementations exist and now need live semantic proof on both backends
  - Combo: PostgreSQL graph-filtered search behind one portable search surface | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.librarian, engine.dba, engine.version | Primitives/Features: PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult | Resolution: IN_THIS_WP | Stub: NONE | Notes: LM-SEARCH-002 must be proven in present code, not inherited from legacy packet notes
  - Combo: Source-anchor durability across storage, export, and replay | Pillars: Loom | Mechanical: engine.archivist, engine.version | Primitives/Features: PRIM-LoomSourceAnchor, PRIM-LoomEdge | Resolution: IN_THIS_WP | Stub: NONE | Notes: provenance semantics are only as strong as the current round-trip evidence
  - Combo: Portable DDL plus migration replay plus rebuildable metrics | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.version | Primitives/Features: PRIM-LoomBlockDerived, PRIM-LoomBlock, PRIM-LoomEdge | Resolution: IN_THIS_WP | Stub: NONE | Notes: migration safety and recompute behavior are part of the portability law even when no fresh schema change is required
  - Combo: Current-main proof plus stale-packet correction | Pillars: Loom, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.dba, engine.version, engine.librarian | Primitives/Features: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | Resolution: IN_THIS_WP | Stub: NONE | Notes: `v4` exists to prevent historical smoke-test narratives from outranking present code-and-spec truth
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations found on the touched Loom and portability surfaces resolve inside this packet without requiring new stubs or a spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: Loom MVP and storage-portability packet lineage; current Loom storage trait, SQLite/PostgreSQL implementations, Loom API, conformance helpers, top-level storage conformance entrypoints, and the `v4` stub
- MATCHED_STUBS:
  - Artifact: WP-1-Media-Downloader-Loom-Bridge-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: downloader promotion and media bridge behavior stay downstream of the base Loom storage portability contract
  - Artifact: WP-1-Video-Archive-Loom-Integration-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: video-library composition depends on storage portability but should not redefine the base contract
  - Artifact: WP-1-Loom-Preview-VideoPosterFrames-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: preview-generation behavior is adjacent follow-on work and should not expand this packet
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Loom-MVP-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Loom MVP delivered the base product surface, but portability closure is governed by later packets
  - Artifact: WP-1-Storage-Abstraction-Layer-v3 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: trait-based storage plumbing already exists and is reused by the current Loom implementation
  - Artifact: WP-1-Artifact-System-Foundations-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: artifact portability foundations exist and should only be touched if a fresh Loom path defect is demonstrated
  - Artifact: WP-1-Loom-Storage-Portability-v2 | BoardStatus: SUPERSEDED | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: current `main` now contains much of the implementation depth that earlier Loom portability packets claimed was missing, but the governance evidence is stale and must be re-earned under `v4`
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Loom-Storage-Portability-v2 | Covers: execution | Verdict: IMPLEMENTED | Notes: the current Storage trait declares `get_backlinks`, `get_outgoing_edges`, `traverse_graph`, `recompute_block_metrics`, `recompute_all_metrics`, and `search_loom_blocks`
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | Artifact: WP-1-Loom-Storage-Portability-v2 | Covers: execution | Verdict: IMPLEMENTED | Notes: the current PostgreSQL backend implements the required Loom portability methods and graph-filtered search logic tied to `LoomSearchFilters`
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | Artifact: WP-1-Loom-Storage-Portability-v2 | Covers: execution | Verdict: IMPLEMENTED | Notes: helper-level current-main coverage exists for traversal, directional edges, metrics recomputation, PostgreSQL graph filtering, and source-anchor round trips
  - Path: ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs | Artifact: WP-1-Loom-Storage-Portability-v2 | Covers: execution | Verdict: IMPLEMENTED | Notes: top-level SQLite and PostgreSQL Loom conformance and traversal-performance entrypoints exist on current `main`
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/loom.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: primitive | Verdict: PARTIAL | Notes: canonical Loom structs and `LoomSearchFilters` are present, but the packet still needs current-main proof that their spec meaning is fully covered by executable evidence
  - Path: ../handshake_main/src/backend/handshake_core/src/api/loom.rs | Artifact: WP-1-Loom-MVP-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: current API routes include graph traversal, metrics recomputation, and search surfaces that earlier Loom portability packets treated as missing
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: same-intent Loom portability artifacts exist, but current-main code reality has outpaced the old packet narrative. `v4` must narrow scope to current proof/remediation and must not rerun obsolete implementation work without a live defect.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This activation is backend portability proof/remediation work. Direct Loom UI/viewer behavior remains downstream.
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
- WHAT: Re-open Loom portability as a current-main proof/remediation pass: revalidate the present trait, backend, API, and test surfaces against Master Spec v02.178; repair only any fresh demonstrated defect or missing proof; otherwise close the packet as proof-only plus status-sync.
- WHY: historical `v2`/`v3` Loom portability governance no longer matches current product reality. Current `main` already contains the major implementation items earlier packets treated as open, so `v4` must separate real remaining defects from stale failure narrative.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/loom.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/api/loom.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
  - .GOV/task_packets/WP-1-Loom-Storage-Portability-v2.md
  - .GOV/task_packets/stubs/WP-1-Loom-Storage-Portability-v4.md
- OUT_OF_SCOPE:
  - structured-collaboration schema registry and mailbox export schema work
  - frontend Loom viewer UX and preview-surface layout changes
  - downloader-to-Loom and video-archive bridge behavior beyond the base portability contract
  - speculative new Loom features not tied to a demonstrated current-main portability defect
  - broad governance refactors unrelated to Loom portability proof/remediation
- TEST_PLAN:
  ```bash
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_loom_storage_conformance -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_loom_traversal_performance_target -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml postgres_loom_storage_conformance -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml postgres_loom_traversal_performance_target -- --exact
  ```
- DONE_MEANS:
  - Current-main Loom storage surfaces are checked against the exact v02.178 portability clauses instead of inheriting closure from the old `v2`/`v3` packet narrative.
  - Any fresh current-main defect found in trait methods, Postgres graph filtering, source-anchor durability, or portability DDL is fixed with targeted executable proof.
  - SQLite and PostgreSQL Loom conformance entrypoints exist and are validator-runnable; any env-gated PostgreSQL evidence remains explicitly marked unproven until executed.
  - If no fresh defect remains, packet scope collapses to proof-only closure and status-sync instead of speculative code churn.
  - No unrelated Loom feature expansion or adjacent media/archive work is pulled into this packet.
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
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/loom.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/api/loom.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
- SEARCH_TERMS:
  - traverse_graph
  - get_backlinks
  - get_outgoing_edges
  - recompute_block_metrics
  - recompute_all_metrics
  - LoomSourceAnchor
  - LoomSearchFilters
  - loom_search_graph_filter_postgres
  - loom_source_anchor_round_trip
  - sqlite_loom_storage_conformance
  - postgres_loom_storage_conformance
  - sqlite_loom_traversal_performance_target
  - postgres_loom_traversal_performance_target
- RUN_COMMANDS:
  ```bash
  rg -n "traverse_graph|get_backlinks|get_outgoing_edges|recompute_block_metrics|recompute_all_metrics|LoomSearchFilters|loom_search_graph_filter_postgres|LoomSourceAnchor|loom_source_anchor_round_trip" ../handshake_main/src/backend/handshake_core
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_loom_storage_conformance -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_loom_traversal_performance_target -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml postgres_loom_storage_conformance -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml postgres_loom_traversal_performance_target -- --exact
  ```
- RISK_MAP:
  - "historical packet claims outrank present code reality" -> "false closure or false reopening survives governance"
  - "PostgreSQL evidence is assumed instead of executed" -> "SQLite-only results are overstated as dual-backend portability"
  - "fresh remediation broadens past demonstrated defects" -> "the packet repeats the earlier scope drift that made smoke testing unreliable"
  - "source anchors or search filters drift behind passing wrappers" -> "portable provenance and graph-filter semantics silently regress"
  - "portable DDL assumptions are not checked against current queries and migrations" -> "backend parity appears closed but migration/runtime portability still fails"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - Current stub metadata and BUILD_ORDER ranking already match this activation target. No build-order edit is required unless scope expands beyond the portability boundary.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list the exact Main Body clauses this packet is expected to satisfy, why they are in scope, what code/tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: [CX-DBP-013] Dual-backend testing early | WHY_IN_SCOPE: current Loom portability claims must be proven on both SQLite and PostgreSQL, not inherited from historical packet notes | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/tests.rs, ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs | EXPECTED_TESTS: sqlite_loom_storage_conformance, postgres_loom_storage_conformance, sqlite_loom_traversal_performance_target, postgres_loom_traversal_performance_target | RISK_IF_MISSED: SQLite-only green checks are misreported as full portability closure
  - CLAUSE: 2.3.13.7 Loom storage trait surface (`get_backlinks`, `get_outgoing_edges`, `traverse_graph`, `recompute_block_metrics`, `recompute_all_metrics`) | WHY_IN_SCOPE: the spec names these methods explicitly and `v4` exists to confirm present code reality rather than rerun stale implementation assumptions | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs, ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs, ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs, ../handshake_main/src/backend/handshake_core/src/api/loom.rs | EXPECTED_TESTS: sqlite_loom_storage_conformance, postgres_loom_storage_conformance | RISK_IF_MISSED: historical packet narratives can still hide real method drift or trigger unnecessary rework
  - CLAUSE: [LM-SEARCH-001] and [LM-SEARCH-002] backend-agnostic search plus PostgreSQL graph filtering | WHY_IN_SCOPE: the spec requires one portable search API while PostgreSQL adds graph-relationship filtering semantics inside the query | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/loom.rs, ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs, ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: postgres_loom_storage_conformance | RISK_IF_MISSED: Postgres search behavior can drift from spec while still looking complete at the packet level
  - CLAUSE: 2.3.13.7 LoomSourceAnchor canonical portability and replay durability | WHY_IN_SCOPE: the spec requires LoomSourceAnchor meaning to survive storage, export, and replay instead of hiding behind view-only adapters | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/loom.rs, ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs, ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs, ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: sqlite_loom_storage_conformance, postgres_loom_storage_conformance | RISK_IF_MISSED: provenance portability is overstated and backend swap/export flows can still regress silently
  - CLAUSE: [CX-DBP-011] Portable schema and migrations | WHY_IN_SCOPE: current Loom portability closure is invalid if current DDL/query assumptions still depend on backend-specific behavior | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs, ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs, ../handshake_main/src/backend/handshake_core/migrations/ | EXPECTED_TESTS: sqlite_loom_storage_conformance, postgres_loom_storage_conformance | RISK_IF_MISSED: portability appears closed in code review while migration/runtime behavior remains backend-fragile

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: Storage trait Loom methods | PRODUCER: ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs, ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | CONSUMER: ../handshake_main/src/backend/handshake_core/src/api/loom.rs | SERIALIZER_TRANSPORT: in-process Rust trait dispatch | VALIDATOR_READER: run_loom_storage_conformance in ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | TRIPWIRE_TESTS: sqlite_loom_storage_conformance, postgres_loom_storage_conformance | DRIFT_RISK: method signatures or semantics diverge between trait, backend implementations, and API routes
  - CONTRACT: LoomSearchFilters graph-relationship semantics | PRODUCER: ../handshake_main/src/backend/handshake_core/src/api/loom.rs | CONSUMER: ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs, ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | SERIALIZER_TRANSPORT: serde JSON over the API boundary and Rust structs in storage | VALIDATOR_READER: loom_search_graph_filter_postgres helper in ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | TRIPWIRE_TESTS: postgres_loom_storage_conformance | DRIFT_RISK: filter fields exist structurally but drift semantically across providers or helper/test boundaries
  - CONTRACT: LoomSourceAnchor durable payload | PRODUCER: ../handshake_main/src/backend/handshake_core/src/api/loom.rs and create_loom_edge call paths | CONSUMER: ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs, ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs, export/replay helper logic | SERIALIZER_TRANSPORT: serde JSON plus database columns | VALIDATOR_READER: loom_source_anchor_round_trip helper in ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | TRIPWIRE_TESTS: sqlite_loom_storage_conformance, postgres_loom_storage_conformance | DRIFT_RISK: anchors round-trip in one path but lose meaning or fields in another
  - CONTRACT: Graph traversal performance and result shape | PRODUCER: ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs, ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | CONSUMER: ../handshake_main/src/backend/handshake_core/src/api/loom.rs and downstream Loom consumers | SERIALIZER_TRANSPORT: in-process `Vec<(LoomBlock, u32)>` results | VALIDATOR_READER: run_loom_traversal_performance_probe in ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | TRIPWIRE_TESTS: sqlite_loom_traversal_performance_target, postgres_loom_traversal_performance_target | DRIFT_RISK: the method exists but depth semantics or performance targets drift without being surfaced in packet claims

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - sqlite_loom_storage_conformance -- executes the current Loom portability helper suite on SQLite
  - postgres_loom_storage_conformance -- executes the current Loom portability helper suite on PostgreSQL when `POSTGRES_TEST_URL` is available
  - sqlite_loom_traversal_performance_target -- proves the SQLite traversal performance target on the current graph fixture
  - postgres_loom_traversal_performance_target -- proves the PostgreSQL traversal performance target when `POSTGRES_TEST_URL` is available
- CANONICAL_CONTRACT_EXAMPLES:
  - Graph traversal signature for the current test fixture: start -> mid (depth 1) -> leaf (depth 2) -> tag (depth 3)
  - `LoomSearchFilters` contract example with `tag_ids`, `mention_ids`, and `backlink_depth` proving PostgreSQL graph filtering semantics
  - `LoomSourceAnchor` JSON export/replay round-trip that survives write/read/replay without field loss

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Reconfirm the current trait, backend, API, and helper surfaces against the exact v02.178 clauses before editing any product code.
  - If a concrete current-main defect is reproduced, patch the narrowest code surface that fixes it.
  - Extend or correct helper-level and top-level conformance tests only where current proof is missing, stale, or misleading.
  - Keep governance/status evidence aligned with the actual product result; if no live defect remains, collapse the packet to proof-only closure.
  - Do not reopen already-landed Loom implementation work unless present code inspection or executable evidence proves real drift.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/loom.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/api/loom.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_loom_storage_conformance -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_loom_traversal_performance_target -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml postgres_loom_storage_conformance -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml postgres_loom_traversal_performance_target -- --exact
- CARRY_FORWARD_WARNINGS:
  - Treat `v2`/`v3` Loom packet history as suspect evidence, not authoritative closure or failure.
  - Do not claim PostgreSQL portability proof unless the PostgreSQL entrypoints actually ran or were explicitly marked env-gated and unproven.
  - If no fresh defect is found, reduce scope to proof-only closeout instead of inventing new churn.
  - Keep file and scope boundaries tight; Schema Registry and other governance refactors are out of scope here.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - [CX-DBP-013] dual-backend Loom conformance and traversal-performance evidence
  - 2.3.13.7 Loom trait methods and directional edge semantics on current `main`
  - [LM-SEARCH-001] and [LM-SEARCH-002] search contract plus PostgreSQL graph filtering
  - 2.3.13.7 `LoomSourceAnchor` portability and replay durability
  - [CX-DBP-011] portable schema and migration/runtime posture
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/loom.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/api/loom.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
- COMMANDS_TO_RUN:
  - rg -n "traverse_graph|get_backlinks|get_outgoing_edges|recompute_block_metrics|recompute_all_metrics|LoomSearchFilters|loom_search_graph_filter_postgres|LoomSourceAnchor|loom_source_anchor_round_trip" ../handshake_main/src/backend/handshake_core
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_loom_storage_conformance -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_loom_traversal_performance_target -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml postgres_loom_storage_conformance -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml postgres_loom_traversal_performance_target -- --exact
- POST_MERGE_SPOTCHECKS:
  - Confirm PostgreSQL evidence was actually executed or was explicitly recorded as env-gated and unproven.
  - Confirm any code change is scoped to a demonstrated current-main defect or missing proof surface.
  - Confirm final packet claims do not exceed the code and tests actually inspected.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - PostgreSQL-backed Loom conformance and traversal-performance entrypoints were not rerun in this refinement pass because those checks remain environment-gated by `POSTGRES_TEST_URL`.
  - No live backend-swap or end-to-end export/reimport run was executed in this refinement pass beyond helper-level source-anchor contract inspection.
  - This current-main inspection did not uncover a fresh Loom defect yet; `v4` may collapse to proof-only closure once validator-owned runs complete.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Main Body explicitly names the storage portability law, the Loom storage trait surface, LM-SEARCH-002, graph traversal, rebuildable metrics, source-anchor portability, and dual-backend testing. `v4` is narrowly scoped to current-main proof/remediation against those clauses.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.178.md already defines the portability law and the exact Loom clauses this packet must prove or repair. `v4` is implementation/evidence alignment against the current Main Body, not a spec-authoring packet.

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
