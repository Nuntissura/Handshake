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
- WP_ID: WP-1-Workflow-Projection-Correlation-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-03-28T19:23:07.9531207+01:00
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- SPEC_TARGET_SHA1: e658a3b8a2d7cdd0d294838151d24a60bc3e034c
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Workflow-Projection-Correlation-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- The Main Body Debug Bundle manifest contract still limits `BundleManifest.scope.kind` to `problem | job | time_window | workspace`, so the spec does not currently allow `workflow_run` or `workflow_node_execution` as canonical bounded export anchors even though the Workflow Engine and recovery law already require stable workflow and node lineage.
- The Main Body `DebugBundleExporter` trait contract still says "for the given scope" without naming workflow-run or workflow-node-execution scope kinds, so code cannot add those anchors without stepping outside the current declared exporter surface.
- The Main Body manifest requirements already require `workflow_run_id`, but they do not say how workflow-scoped or node-scoped bundles are declared, counted, or inventoried, which leaves validator behavior under-specified.
- Current product code already persists `WorkflowRun` and `WorkflowNodeExecution`, and the Workflow Engine already threads `workflow_run_id` into Flight Recorder events, but the current debug bundle exporter only materializes problem/job/time_window/workspace scopes.
- Current product code does not yet expose a canonical workflow-node-execution inventory file or a `list_exportable` projection for workflow-run / node anchors, so the implementation target is only partial even before the spec mismatch is considered.
- This packet therefore cannot honestly activate as pure code work until the Main Body bundle-scope contract is enriched.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 95m
- SEARCH_SCOPE: current Master Spec v02.178 workflow, AI job, recovery, debug bundle, and Locus sections; local product code in `src/backend/handshake_core/src/workflows.rs`, `storage/mod.rs`, `bundles/exporter.rs`, `bundles/schemas.rs`, `bundles/templates.rs`, `locus/task_board.rs`, and existing bundle/storage tests
- REFERENCES: Internal spec-to-code remediation only. Primary sources were `.GOV/spec/Handshake_Master_Spec_v02.178.md`, `.GOV/task_packets/stubs/WP-1-Workflow-Projection-Correlation-v1.md`, `.GOV/roles_shared/records/BUILD_ORDER.md`, `.GOV/roles_shared/records/TASK_BOARD.md`, `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/storage/mod.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/schemas.rs`, `src/backend/handshake_core/src/bundles/templates.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, and `src/backend/handshake_core/src/storage/tests.rs`.
- PATTERNS_EXTRACTED: treat `workflow_run_id` and `workflow_node_execution_id` as bounded export anchors rather than inferred filters; keep bundle scope explicit in the manifest instead of reconstructing it from free-form query parameters; keep workflow correlation portable by reusing existing persisted ids and current Flight Recorder correlation fields instead of adding a second lineage mechanism; keep the packet narrow by expanding exporter contracts and proof only, not by redesigning Task Board row schemas or replay execution.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT explicit workflow-scoped and node-scoped bundle anchors; ADAPT the existing bundle manifest/exporter/validator pattern rather than inventing a parallel workflow export system; REJECT widening this packet into generic Task Board schema redesign, Dev Command Center UI work, or replay execution features.
- LICENSE/IP_NOTES: Internal repository and spec inspection only. No third-party code or text is intended for direct reuse.
- SPEC_IMPACT: YES
- SPEC_IMPACT_REASON: Main Body 2.3.10 Debug Bundle export must change because the current `BundleManifest.scope.kind` union and `DebugBundleExporter` contract omit `workflow_run` and `workflow_node_execution` even though Main Body workflow and recovery clauses already require stable workflow and node lineage.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is a strictly internal spec-to-code remediation pass against the current Handshake Master Spec and the current local codebase. The blocking issue is an internal Main Body contract mismatch, not missing external signal.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Internal sources are sufficient because the unresolved work is explicit in the current Master Spec and in the current local exporter/workflow code.
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
- No new Flight Recorder event ids are required for this packet.
- The packet must preserve current `workflow_run_id` correlation already emitted by the Workflow Engine and use it as a bounded export filter for workflow-scoped and node-scoped bundles.
- Validator proof should confirm that workflow-scoped and node-scoped exports read the existing correlation fields instead of reconstructing lineage from transcript order, message order, or broad time-window guesses.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: without explicit workflow and node scope kinds, operators or models can over-export unrelated diagnostics, jobs, or events by falling back to job/time-window approximations. Mitigation: make workflow-run and node-execution scope kinds first-class and bounded by stable ids.
- Risk: if node-scoped export lineage is reconstructed from broad workflow history instead of explicit node ids, replay and audit views can silently attach the wrong evidence. Mitigation: require `workflow_node_execution_id` as a canonical export anchor and inventory it directly.
- Risk: if manifest counts and inventory files do not mention workflow node executions, validators can accept semantically incomplete bundles. Mitigation: add a workflow-node-execution inventory file and explicit manifest count requirements.
- Risk: widening this packet into Locus UI or replay execution would make proof shallow and leave the core exporter contract unresolved. Mitigation: keep the packet centered on scope law, manifest law, exporter behavior, and bounded tests.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-WorkflowRun
  - PRIM-WorkflowNodeExecution
  - PRIM-AiJob
  - PRIM-FlightRecorder
  - PRIM-DebugBundleExporter
  - PRIM-BundleScope
  - PRIM-BundleManifest
  - PRIM-DebugBundleRequest
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-WorkflowRun
  - PRIM-WorkflowNodeExecution
  - PRIM-DebugBundleExporter
  - PRIM-BundleScope
  - PRIM-BundleManifest
  - PRIM-DebugBundleRequest
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - PRIM-DebugBundleExporter
  - PRIM-BundleScope
  - PRIM-BundleManifest
  - PRIM-DebugBundleRequest
- NOTES:
  - The spec already names the relevant primitives. This packet updates the declared contract for existing workflow and bundle primitives rather than inventing new primitive ids.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current appendix already contains the primitive ids needed for workflow runs, workflow node executions, and debug bundle export. This pass widens existing contract meaning rather than introducing new primitive ids.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - If the Main Body enrichment lands as written, Appendix 12.4 can continue using the same primitive ids without new rows.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new orphan primitives were discovered during this refinement pass.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Existing feature and primitive registry entries already cover the workflow, AI job, Flight Recorder, and debug bundle surfaces involved here.
- UI_GUIDANCE_ACTION: UPDATED
- UI_GUIDANCE_REASON: FEAT-DEBUG-BUNDLE UI guidance should explicitly mention workflow-run and workflow-node-execution scope selection once those scope kinds become normative, so operator surfaces do not silently treat them as broad time-window exports.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Existing interaction-matrix edges are sufficient for this refinement. The packet changes bundle-scope law and exporter proof rather than introducing a new cross-feature topology edge.
- APPENDIX_MAINTENANCE_NOTES:
  - The required enrichment touches Main Body bundle-scope law and the existing FEAT-DEBUG-BUNDLE UI guidance row.
  - UI guidance should call out workflow-run and workflow-node-execution scope selection explicitly so those operator flows remain bounded and understandable.
- APPENDIX_MAINTENANCE_VERDICT: NEEDS_SPEC_UPDATE

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene contract is changed by workflow bundle correlation | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected here | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no physics or measurement law is changed by exporter scope enrichment | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes remain downstream consumers of the bounded workflow evidence | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing capability is changed by this refinement | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: TOUCHED | NOTES: workflow-run and node-execution anchors are orchestration-facing runtime contracts that affect how failures are inspected and exported | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no composition or sequencing surface is expanded | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no rendering or creative surface is touched | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication/export transport is unchanged outside the bounded bundle contract | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no cooking or recipe workflow contract is involved | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food safety or compliance surface is changed here | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no routing or delivery engine behavior is changed directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: bounded debug bundle manifests and workflow-node inventories are archival evidence contracts | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: workflow-correlated bundle inventory improves bounded retrieval of failure evidence without broad time-window scanning | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: TOUCHED | NOTES: deterministic correlation anchors improve replay and analysis surfaces by replacing inference with stable ids | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is changed directly | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: the packet relies on persisted workflow and node records and must keep exporter joins backend-portable | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: no new governance authority surface is introduced; this pass only aligns exporter law to existing workflow law | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or guided interaction surface is expanded | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: bounded workflow and node anchors improve context transport between AI job, Flight Recorder, and bundle export surfaces | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: this is a versioned contract expansion of existing bundle and workflow primitives | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required in this refinement | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: NEEDS_SPEC_UPDATE

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: existing workflow correlation fields in Flight Recorder become the bounded evidence seam for workflow-scoped and node-scoped bundle export | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: calendar storage and policy surfaces are unrelated to this packet | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: editor surfaces are downstream consumers only | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: document editing is not changed by workflow bundle correlation | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: spreadsheet surfaces are out of scope | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: Locus-ready and progress projection should be able to seed bounded debug bundles from stable workflow ids rather than ad hoc scans | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Loom portability remains a separate validated lane and is file-disjoint from this refinement | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: work packet product surfaces may consume the enriched exporter later, but this packet does not change product work packet schema directly | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Task Board schema redesign is explicitly out of scope; only workflow correlation joins are relevant here | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: MicroTask runtime surfaces remain downstream consumers of workflow correlation rather than direct targets of this packet | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: operator UI controls remain downstream of the backend exporter contract and are not implemented here | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: memory-system behavior is not changed directly here | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: the packet formalizes how runtime workflow and node ids become export anchors instead of remaining incidental storage details | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no Spec Router or prompt compilation contract is expanded directly in this pass | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: the exporter changes must remain storage-portable because the underlying workflow lineage already lives in backend-neutral storage contracts | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: canonical workflow and node inventories make downstream model inspection and replay reasoning less dependent on transcript reconstruction | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Stage artifact contracts are unrelated to this workflow bundle scope refinement | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Studio runtime behavior is out of scope | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: viewer follow-on work remains downstream | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: distillation workflows are unaffected by this packet | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE runtime protocol or tool contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval systems may consume the richer bundle inventories later but are not changed in this WP | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: NEEDS_SPEC_UPDATE

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: workflow-run bounded export anchor | SUBFEATURES: workflow-run manifest scope, workflow-run exporter routing, workflow-run list_exportable inventory | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-DebugBundleExporter, PRIM-BundleScope, PRIM-DebugBundleRequest | MECHANICAL: engine.director, engine.context, engine.version | ROI: HIGH | RESOLUTION: SPEC_UPDATE_NOW | STUB: NONE | NOTES: current code substrate exists, but the Main Body bundle-scope union still blocks legitimate activation
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: workflow-node execution bounded export anchor | SUBFEATURES: node-scoped manifest scope, node-scoped bundle filtering, node execution inventory file | PRIMITIVES_FEATURES: PRIM-WorkflowNodeExecution, PRIM-BundleManifest, PRIM-BundleScope, PRIM-DebugBundleExporter | MECHANICAL: engine.archivist, engine.analyst, engine.version | ROI: HIGH | RESOLUTION: SPEC_UPDATE_NOW | STUB: NONE | NOTES: the spec names node lineage in workflow law but not in debug bundle scope law
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: workflow-correlated recorder evidence reuse | SUBFEATURES: export filtering by existing workflow ids, bounded event inclusion, replay-safe chronology | PRIMITIVES_FEATURES: PRIM-FlightRecorder, PRIM-WorkflowRun, PRIM-WorkflowNodeExecution, PRIM-BundleScope | MECHANICAL: engine.context, engine.analyst | ROI: HIGH | RESOLUTION: SPEC_UPDATE_NOW | STUB: NONE | NOTES: reuse current recorder correlation fields instead of adding new event families
  - PILLAR: Locus | CAPABILITY_SLICE: workflow correlation handoff into bounded export | SUBFEATURES: Locus-ready bundle seed path, progress-to-bundle anchor resolution, durable workflow correlation joins | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-DebugBundleRequest, PRIM-BundleManifest | MECHANICAL: engine.director, engine.archivist | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: keep this limited to existing joins after the spec update lands; do not widen into Task Board schema redesign
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: backend-portable workflow correlation joins | SUBFEATURES: storage-neutral workflow-run joins, storage-neutral node execution joins, deterministic bundle filtering across storage backends | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-WorkflowNodeExecution, PRIM-DebugBundleExporter | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: SPEC_UPDATE_NOW | STUB: NONE | NOTES: the bundle-scope contract must stay portable across current SQLite and future PostgreSQL execution paths
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: workflow-node export inventory for machine-readable replay | SUBFEATURES: workflow node execution inventory lines, stable ids, bounded hash fields | PRIMITIVES_FEATURES: PRIM-WorkflowNodeExecution, PRIM-BundleManifest, PRIM-DebugBundleRequest | MECHANICAL: engine.librarian, engine.analyst, engine.context | ROI: HIGH | RESOLUTION: SPEC_UPDATE_NOW | STUB: NONE | NOTES: explicit workflow-node export records reduce later model dependence on transcript reconstruction
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_SPEC_UPDATE

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: Workflow-run scoped debug bundle export | JobModel: WORKFLOW | Workflow: Workflow Engine -> Debug Bundle export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: existing workflow_id-correlated event families | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: the runtime substrate exists, but the Main Body does not yet declare `workflow_run` as a valid bundle scope kind
  - Capability: Workflow-node-execution scoped debug bundle export | JobModel: WORKFLOW | Workflow: Workflow Engine -> Debug Bundle export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: existing workflow_id-correlated event families plus node lineage joins | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: node execution ids are already persisted, but the exporter contract and manifest law do not yet admit them as scope anchors
  - Capability: Exportable inventory for workflow correlation anchors | JobModel: WORKFLOW | Workflow: Debug Bundle export inventory | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: `list_exportable` currently inventories jobs and diagnostics only
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: NEEDS_SPEC_UPDATE

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 45m
- MATRIX_SCAN_NOTES:
  - The highest-ROI combo is existing workflow lineage plus existing bundle export plus existing Flight Recorder correlation. That path produces bounded evidence without inventing a new runtime or a new replay format.
  - Local and cloud models both benefit indirectly because deterministic workflow-correlation exports reduce dependence on transcript reconstruction and broad time-window searches.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - Edge: NONE
  - Kind: NONE
  - ROI: LOW
  - Effort: LOW
  - Spec refs: NONE
  - In-scope for this WP: NO
  - If NO: create a stub WP and record it in TASK_BOARD Stub Backlog (order is not priority).
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: The packet is a Main Body scope-law and proof expansion on top of existing workflow and bundle primitives. No new Appendix 12.6 edge is required before activation.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This refinement is an internal Main Body reconciliation pass. The missing law is already explicit in the local spec and product code.
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
  - Combo: WorkflowRun + FlightRecorder + DebugBundleExporter + Locus correlation | Pillars: Flight Recorder, Locus, Execution / Job Runtime, LLM-friendly data | Mechanical: engine.director, engine.archivist, engine.context | Primitives/Features: PRIM-WorkflowRun, PRIM-FlightRecorder, PRIM-DebugBundleExporter, PRIM-BundleScope, PRIM-BundleManifest | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: current code already has most substrate, but the spec does not yet allow workflow-run bundle scope
  - Combo: WorkflowNodeExecution + BundleManifest + export inventory + replay-safe chronology | Pillars: Execution / Job Runtime, Flight Recorder, LLM-friendly data | Mechanical: engine.archivist, engine.analyst, engine.version | Primitives/Features: PRIM-WorkflowNodeExecution, PRIM-BundleManifest, PRIM-DebugBundleRequest, PRIM-DebugBundleExporter | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: node lineage is already mandatory in recovery law, but not yet legal as a debug bundle scope anchor
  - Combo: WorkflowRun + AiJob runtime identity + workflow-scoped bundle manifest | Pillars: Execution / Job Runtime, LLM-friendly data | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-WorkflowRun, PRIM-AiJob, PRIM-BundleManifest, PRIM-BundleScope | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: makes runtime execution identity portable into bundle export law
  - Combo: WorkflowNodeExecution + FlightRecorder event filtering + targeted diagnostics set | Pillars: Flight Recorder, Execution / Job Runtime | Mechanical: engine.analyst, engine.context | Primitives/Features: PRIM-WorkflowNodeExecution, PRIM-FlightRecorder, PRIM-DebugBundleExporter | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: keeps node-scoped exports bounded and replay-safe
  - Combo: WorkflowRun + list_exportable inventory + operator bundle selection | Pillars: Locus, LLM-friendly data | Mechanical: engine.librarian, engine.context | Primitives/Features: PRIM-WorkflowRun, PRIM-DebugBundleExporter, PRIM-DebugBundleRequest | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: makes workflow anchors discoverable without broad time-window scans
  - Combo: WorkflowNodeExecution + workflow_node_executions.jsonl + downstream replay readers | Pillars: LLM-friendly data, Execution / Job Runtime | Mechanical: engine.librarian, engine.archivist | Primitives/Features: PRIM-WorkflowNodeExecution, PRIM-BundleManifest, PRIM-DebugBundleRequest | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: adds a canonical machine-readable lineage inventory
  - Combo: WorkflowRun + deterministic zip manifest hashes + portable exported entity counts | Pillars: SQL to PostgreSQL shift readiness, LLM-friendly data | Mechanical: engine.version, engine.dba | Primitives/Features: PRIM-WorkflowRun, PRIM-BundleManifest, PRIM-DebugBundleExporter | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: portable deterministic bundle output matters across storage backends
  - Combo: WorkflowNodeExecution + backend-portable storage joins + bounded export filtering | Pillars: SQL to PostgreSQL shift readiness, Execution / Job Runtime | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-WorkflowNodeExecution, PRIM-DebugBundleExporter, PRIM-BundleScope | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: keeps exporter logic neutral to current and future storage engines
  - Combo: Locus-ready projection + workflow_run anchor + direct bundle seed path | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.director, engine.archivist | Primitives/Features: PRIM-WorkflowRun, PRIM-DebugBundleRequest, PRIM-BundleManifest | Resolution: IN_THIS_WP | Stub: NONE | Notes: narrow handoff from Locus correlation into bounded bundle export remains in packet scope
  - Combo: FlightRecorder chronology + workflow_node_execution_id + replay-safe export manifest | Pillars: Flight Recorder, LLM-friendly data | Mechanical: engine.analyst, engine.version | Primitives/Features: PRIM-FlightRecorder, PRIM-WorkflowNodeExecution, PRIM-BundleManifest | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: ties replay chronology to explicit node lineage instead of transcript order
  - Combo: WorkflowRun + templates-generated prompts + exported workflow context | Pillars: Execution / Job Runtime, LLM-friendly data | Mechanical: engine.context, engine.librarian | Primitives/Features: PRIM-WorkflowRun, PRIM-BundleManifest, PRIM-DebugBundleExporter | Resolution: IN_THIS_WP | Stub: NONE | Notes: bundle-side prompts should consume enriched workflow scope metadata without inventing a new prompt contract
  - Combo: WorkflowNodeExecution + diagnostics bridge + export validator proof | Pillars: Execution / Job Runtime, Flight Recorder | Mechanical: engine.analyst, engine.archivist | Primitives/Features: PRIM-WorkflowNodeExecution, PRIM-DebugBundleExporter, PRIM-BundleManifest | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: validator proof must reject semantically incomplete node-scoped bundles
  - Combo: WorkflowRun + workflow_node_execution inventory + storage portability posture | Pillars: SQL to PostgreSQL shift readiness, LLM-friendly data, Execution / Job Runtime | Mechanical: engine.dba, engine.librarian, engine.version | Primitives/Features: PRIM-WorkflowRun, PRIM-WorkflowNodeExecution, PRIM-BundleManifest | Resolution: SPEC_UPDATE_NOW | Stub: NONE | Notes: exporter shape should stay backend-neutral while remaining easy for downstream models to consume
- FORCE_MULTIPLIER_VERDICT: NEEDS_SPEC_UPDATE
- FORCE_MULTIPLIER_REASON: The best combinations are already visible and high ROI, but all of them depend on the missing Main Body scope enrichment.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed workflow/AI job/Flight Recorder/debug bundle/Locus packets, and current product code in `src/backend/handshake_core/src/workflows.rs`, `storage/mod.rs`, `bundles/exporter.rs`, `bundles/schemas.rs`, `bundles/templates.rs`, and `locus/task_board.rs`
- MATCHED_STUBS:
  - Artifact: WP-1-Locus-Debug-Bundle-Bridge-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: MISSING | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Locus-scoped bundle bridging is adjacent, but this packet is about workflow/job/recorder correlation and bounded exporter scope law
  - Artifact: WP-1-Diagnostics-Debug-Bundle-Bridge-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: MISSING | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: diagnostics-scoped bridging is narrower and should reuse the exporter law once this packet is resolved
  - Artifact: WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: MISSING | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: mailbox-scoped bundles are a separate evidence path and should not absorb workflow scope law
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Debug-Bundle-v3 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: exporter, manifest, validator, and deterministic zip behavior already exist, but workflow-run and node-execution scope law is missing
  - Artifact: WP-1-AI-Job-Model-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: `workflow_run_id` runtime identity already exists and should be reused, not reimplemented here
  - Artifact: WP-1-Workflow-Engine-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: workflow and node persistence already exist and form the lineage substrate for this packet
  - Artifact: WP-1-Flight-Recorder-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: current recorder correlation should be reused as evidence input for exporter filtering
  - Artifact: WP-1-Locus-Phase1-Integration-Occupancy-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Locus occupancy/projection exists, but the packet did not formalize workflow-scoped debug bundle anchors
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Workflow-Engine-v4 | Covers: primitive | Verdict: IMPLEMENTED | Notes: `WorkflowRun` and `WorkflowNodeExecution` are persisted and exposed in the storage trait
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Workflow-Engine-v4 | Covers: execution | Verdict: IMPLEMENTED | Notes: workflow execution already propagates `workflow_run_id` into job handling and Flight Recorder events
  - Path: ../handshake_main/src/backend/handshake_core/src/bundles/exporter.rs | Artifact: WP-1-Debug-Bundle-v3 | Covers: primitive | Verdict: PARTIAL | Notes: exporter exists, but `BundleScope` and `list_exportable` only handle problem/job/time_window/workspace
  - Path: ../handshake_main/src/backend/handshake_core/src/bundles/schemas.rs | Artifact: WP-1-Debug-Bundle-v3 | Covers: primitive | Verdict: PARTIAL | Notes: manifest and jobs already carry `workflow_run_id`, but there is no workflow-node execution inventory contract
  - Path: ../handshake_main/src/backend/handshake_core/src/bundles/templates.rs | Artifact: WP-1-Debug-Bundle-v3 | Covers: execution | Verdict: PARTIAL | Notes: generated bundle prompts mention workflow run id but not workflow-node execution inventory
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs | Artifact: WP-1-Locus-Phase1-Integration-Occupancy-v1 | Covers: combo | Verdict: PARTIAL | Notes: downstream projection exists, but packet scope should stay narrow and avoid Task Board schema redesign
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The required substrate already exists across validated packets and current code, but `WP-1-Debug-Bundle-v3` only closed the generic exporter surface. This WP is a narrow scope expansion that still requires a Main Body contract update before activation.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet is a backend contract and proof pass. Any operator UI control for selecting workflow-run or node-scoped bundle export remains downstream of the core contract.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - NONE
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - Tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: This refinement is intentionally limited to backend exporter law and proof surfaces.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Workflow-Engine, WP-1-AI-Job-Model, WP-1-Flight-Recorder, WP-1-Locus-Phase1-Integration-Occupancy, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: NONE
- SPEC_ANCHOR_PRIMARY: Workflow persistence and recovery law already require stable `workflow_run_id` and `workflow_node_execution_id`, but the current Debug Bundle manifest scope union does not allow those ids as canonical bounded export anchors.
- WHAT: Expand the Debug Bundle contract so `workflow_run` and `workflow_node_execution` become first-class bounded export scopes, with matching manifest fields, export inventory, and validator proof.
- WHY: Backend workflow failures should be exportable and replayable from stable workflow lineage rather than reconstructed manually from jobs, diagnostics, or broad time windows.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests
- OUT_OF_SCOPE:
  - Dev Command Center UI redesign
  - generic Task Board schema redesign
  - replay execution beyond bounded export/projection contract
  - mailbox-specific bundle scope work
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture
  ```
- DONE_MEANS:
  - Main Body explicitly allows `workflow_run` and `workflow_node_execution` bundle scope kinds.
  - exporter, manifest schema, and validator all accept and prove those bounded scope kinds.
  - workflow-scoped and node-scoped bundle exports only include correlated jobs/events/node records.
  - canonical export inventory includes workflow node execution records and manifest counts.
- PRIMITIVES_EXPOSED:
  - PRIM-WorkflowRun
  - PRIM-WorkflowNodeExecution
  - PRIM-DebugBundleExporter
  - PRIM-BundleScope
  - PRIM-BundleManifest
  - PRIM-DebugBundleRequest
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - workflow_run_id
  - workflow_node_execution
  - BundleScope
  - build_manifest_scope
  - list_exportable
  - export_for_job
- RUN_COMMANDS:
  ```bash
  rg -n "WorkflowRun|WorkflowNodeExecution|workflow_run_id|workflow_node_execution" src/backend/handshake_core/src
  rg -n "enum BundleScope|build_manifest_scope|collect_events|collect_jobs|list_exportable|export_for_job" src/backend/handshake_core/src/bundles/exporter.rs
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  ```
- RISK_MAP:
  - "workflow-scoped export leaks unrelated records" -> "debug bundles become over-broad and unsafe for replay or sharing"
  - "node-scoped export lacks canonical node inventory" -> "validators can pass semantically incomplete bundles"
  - "implementation reconstructs lineage from time windows" -> "chronology and evidence can drift silently"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - This refinement requires a Main Body spec update and a future `SPEC_CURRENT` advance before packet approval/signature.
  - Re-run `just build-order-sync` after the spec update lands and before activating the official packet.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: HSK-WF-001 durable node persistence plus recovery-safe node lineage must become bounded export anchors | WHY_IN_SCOPE: persisted workflow and node ids are only useful for bounded export if the bundle contract admits them directly | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/storage/mod.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/schemas.rs` | EXPECTED_TESTS: `storage::tests::workflow_node_execution_persists_inputs_and_outputs`; `bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage` | RISK_IF_MISSED: node lineage stays persisted but not exportable, forcing manual reconstruction
  - CLAUSE: AI Job Model runtime identity requires `workflow_run_id` to remain a first-class runtime anchor | WHY_IN_SCOPE: workflow-scoped bundle export must bind to runtime execution rather than only to logical job identity | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/templates.rs` | EXPECTED_TESTS: `bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes`; `bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors` | RISK_IF_MISSED: exports silently collapse runtime lineage into job-level approximations
  - CLAUSE: Debug Bundle manifest scope and exporter contract must admit workflow-run and node-execution scope kinds | WHY_IN_SCOPE: current Main Body scope union blocks the intended packet scope | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/schemas.rs`, `src/backend/handshake_core/src/bundles/templates.rs` | EXPECTED_TESTS: `bundles::validator::tests::val_bundle_001_reports_missing_files`; `bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes`; `bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage` | RISK_IF_MISSED: code either remains incomplete or drifts outside the spec
  - CLAUSE: export manifest requirements that already mention `workflow_run_id` must be reconciled with explicit scope law and workflow-node inventory proof | WHY_IN_SCOPE: manifest law is currently internally inconsistent across required ids, scope kinds, and inventory counts | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/bundles/schemas.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/validator.rs` | EXPECTED_TESTS: `bundles::validator::tests::val_bundle_001_reports_missing_files`; `bundles::zip::tests::bundle_determinism_hash_stable`; golden manifest assertions for workflow scope kinds | RISK_IF_MISSED: validators can pass bundles that claim workflow correlation without complete manifest evidence

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: `DebugBundleRequest.scope` | PRODUCER: `src/backend/handshake_core/src/bundles/exporter.rs` callers | CONSUMER: `src/backend/handshake_core/src/bundles/exporter.rs` exporter implementation | SERIALIZER_TRANSPORT: in-process Rust struct serialized into manifest | VALIDATOR_READER: bundle validator and downstream manifest readers | TRIPWIRE_TESTS: workflow-run scope and workflow-node scope exporter tests | DRIFT_RISK: code can add new scope kinds without matching manifest/validator support
  - CONTRACT: `BundleManifest.scope` | PRODUCER: `build_manifest_scope` in `src/backend/handshake_core/src/bundles/exporter.rs` | CONSUMER: bundle validator, export templates, operator tooling | SERIALIZER_TRANSPORT: `export_manifest.json` | VALIDATOR_READER: bundle validator manifest parsing | TRIPWIRE_TESTS: golden manifest assertions for `workflow_run` and `workflow_node_execution` | DRIFT_RISK: manifest can overstate workflow correlation without canonical scope fields
  - CONTRACT: workflow node execution inventory file | PRODUCER: bundle exporter inventory writer | CONSUMER: validator, replay/audit readers, future operator tooling | SERIALIZER_TRANSPORT: `workflow_node_executions.jsonl` | VALIDATOR_READER: bundle validator inventory checks | TRIPWIRE_TESTS: node-scope export test plus validator fixture checks | DRIFT_RISK: node lineage remains implicit and semantically unverified
  - CONTRACT: exportable inventory projection | PRODUCER: `list_exportable` in `src/backend/handshake_core/src/bundles/exporter.rs` | CONSUMER: operator tooling and future Command Center bundle pickers | SERIALIZER_TRANSPORT: in-process response payloads | VALIDATOR_READER: targeted unit tests over inventory rows | TRIPWIRE_TESTS: `bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors` | DRIFT_RISK: workflow anchors remain invisible even after backend support lands

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture`
- CANONICAL_CONTRACT_EXAMPLES:
  - `export_manifest.json` with `scope.kind = "workflow_run"` and explicit `workflow_run_id`
  - `export_manifest.json` with `scope.kind = "workflow_node_execution"` and explicit `workflow_run_id` plus `workflow_node_execution_id`
  - `workflow_node_executions.jsonl` containing one exported node execution line with stable ids and bounded hashes
  - exportable inventory row that surfaces a workflow-run anchor without degrading to a broad time-window entry

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Wait for the Main Body enrichment and `SPEC_CURRENT` advance; do not start code against the old scope contract.
  - Extend bundle scope and manifest schema for `workflow_run` and `workflow_node_execution`.
  - Implement workflow-run and node-execution export filtering in `bundles/exporter.rs` using existing persisted lineage and current Flight Recorder correlation fields.
  - Add canonical workflow-node execution inventory emission and manifest counts.
  - Extend `list_exportable` to surface workflow correlation anchors.
  - Add targeted exporter and validator tripwire tests for workflow-run and node-execution scope semantics.
- HOT_FILES:
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture
- CARRY_FORWARD_WARNINGS:
  - Do not widen into generic Task Board schema redesign or Dev Command Center UI work.
  - Do not invent new workflow ids or recorder ids when persisted workflow and node lineage already exist.
  - Do not use broad time-window exports as a substitute for explicit workflow-run or node-execution scope kinds.
  - Keep mailbox, diagnostics-only, and Locus-only bundle bridges as separate follow-on packets.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - workflow and recovery law requiring stable `workflow_run_id` and `workflow_node_execution_id`
  - debug bundle scope union and exporter contract after enrichment
  - export manifest requirements for `workflow_run_id` and workflow-node inventory proof
  - exportable inventory visibility for workflow correlation anchors
- FILES_TO_READ:
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture
  - rg -n "BundleScope|workflow_run_id|workflow_node_execution|list_exportable|build_manifest_scope|collect_events|collect_jobs" src/backend/handshake_core/src
- POST_MERGE_SPOTCHECKS:
  - Verify no workflow-scoped export silently falls back to time-window semantics.
  - Verify node-scoped export includes only the targeted node lineage and bound workflow run.
  - Verify manifest counts and inventory files stay deterministic across repeated exports.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The final bundle inventory filename can be `workflow_node_executions.jsonl` as proposed here, but the exact filename is not yet product-code-proven.
  - The cleanest internal join strategy for node-scoped export across diagnostics, jobs, and Flight Recorder events is not yet code-proven.
  - Whether Command Center should surface workflow-run and node-execution anchors in one picker or separate grouped views remains intentionally out of scope for this packet.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: FAIL
- CLEARLY_COVERS_REASON: The current Main Body explicitly names workflow persistence, runtime `workflow_run_id`, recovery-safe node lineage, and debug bundle export, but it does not explicitly name workflow-run or workflow-node-execution bundle scope kinds. That leaves a real contract gap between the workflow law and the exporter law.
- AMBIGUITY_FOUND: YES
- AMBIGUITY_REASON: The spec already requires stable workflow and node lineage, and it already requires `workflow_run_id` in bundle manifests, but it still omits workflow-run and node-execution scope kinds from the canonical Debug Bundle scope union.

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: YES
- REASON_NO_ENRICHMENT: N/A (ENRICHMENT_NEEDED=YES)

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
[ADD v02.179] **Workflow correlation bundle scopes (Normative)**

The Debug Bundle contract MUST treat `workflow_run_id` and `workflow_node_execution_id` as canonical bounded export anchors.

- A `workflow_run` scoped export MUST include only records correlated to the targeted `workflow_run_id`.
- A `workflow_node_execution` scoped export MUST include the targeted node execution record, its bound `workflow_run_id`, its bound `job_id` when present, and only the diagnostics, Flight Recorder events, and bundle-side artifact references correlated to that node lineage.
- Operators and tooling MUST NOT reconstruct workflow-scoped or node-scoped bundles only from broad time-window filtering when stable workflow lineage ids are available.

Update the canonical manifest scope shape to:

~~~ts
scope: {
  kind: "problem" | "job" | "workflow_run" | "workflow_node_execution" | "time_window" | "workspace";
  problem_id?: string;
  job_id?: string;
  workflow_run_id?: string;
  workflow_node_execution_id?: string;
  time_range?: {
    start: string;
    end: string;
  };
  wsid?: string;
};
~~~

[ADD v02.179] **DebugBundleExporter workflow scope law (Normative)**

`DebugBundleExporter` MUST accept `workflow_run` and `workflow_node_execution` scopes in addition to the existing scope kinds.

- `list_exportable` SHOULD expose workflow-run and workflow-node-execution anchors when enough correlated records exist to materialize a bounded bundle.
- Exporters MUST preserve deterministic output for identical inputs across workflow-run and workflow-node-execution scope kinds.

[ADD v02.179] **Workflow node execution inventory (Normative)**

Workflow-correlated debug bundles MUST include a canonical workflow node execution inventory file:

- File: `workflow_node_executions.jsonl`
- Format: one JSON object per exported node execution

Each line MUST include:

- `workflow_node_execution_id`
- `workflow_run_id`
- `node_id`
- `status`
- `started_at`
- `finished_at` when known
- `job_id` when present
- `input_sha256` when an input payload is exported
- `output_sha256` when an output payload is exported

[ADD v02.179] **Manifest count and id requirements for workflow-correlated exports (Normative)**

- `export_manifest.json` counts MUST include `workflow_node_execution_count` whenever any workflow node execution records are exported.
- When `scope.kind = "workflow_run"`, the manifest MUST include `workflow_run_id`.
- When `scope.kind = "workflow_node_execution"`, the manifest MUST include both `workflow_run_id` and `workflow_node_execution_id`.
- Workflow-correlated bundles MUST remain bounded by the targeted ids and MUST NOT include unrelated workflow runs or unrelated node executions.

[ADD v02.179] **Appendix 12.5 FEAT-DEBUG-BUNDLE UI guidance update**

Update the existing `FEAT-DEBUG-BUNDLE` UI guidance row so operator surfaces treat workflow-correlated exports as explicit bounded scopes:

- add `Workflow run export` and `Workflow node execution export` to `entry_points`
- add `workflow_scope_not_found` to `interaction_contract.errors`
- add the following test to `tests`:
  - `Workflow-run and workflow-node-execution scope selection preserves explicit ids across export, status polling, validation, and download.`
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.6 Workflow & Automation Engine [HSK-WF-001] Durable Node Persistence (Normative)
- CONTEXT_START_LINE: 9175
- CONTEXT_END_LINE: 9177
- CONTEXT_TOKEN: The Workflow Engine MUST persist every node execution, status transition, and input/output payload to the database.
- EXCERPT_ASCII_ESCAPED:
  ```text
  **[HSK-WF-001] Durable Node Persistence (Normative):**
  The Workflow Engine MUST persist every node execution, status transition, and input/output payload to the database. A "minimal" async wrapper that only logs start/end events is insufficient.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.6.6 AI Job Model runtime identity relationship
- CONTEXT_START_LINE: 9689
- CONTEXT_END_LINE: 9693
- CONTEXT_TOKEN: - `workflow_run_id` is the **runtime** instance (one per execution attempt)
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Relationship:**
  - `job_id` is the **logical** identity (stable across retries, visible to users)
  - `workflow_run_id` is the **runtime** instance (one per execution attempt)

  **Key Principle:** There is no separate AI jobs executor. The workflow engine (Section 2.6) is the **only** execution path for AI jobs.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 4.3 Recovery-safe run history [ADD v02.165]
- CONTEXT_START_LINE: 32777
- CONTEXT_END_LINE: 32780
- CONTEXT_TOKEN: stable `workflow_run_id`, `workflow_node_execution_id`
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **INV-RECOVER-003:** All recovery actions MUST be logged in Flight Recorder with `FR-EVT-WF-RECOVERY` correlation.

  [ADD v02.165] Recovery-safe run history MUST preserve queue-state transitions, workflow-node execution lineage, tool-call lineage, checkpoint chronology, and operator replay decisions by stable `workflow_run_id`, `workflow_node_execution_id`, `session_id`, `tool_call_id`, and `checkpoint_id` values.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.10 Debug Bundle export manifest scope contract
- CONTEXT_START_LINE: 57822
- CONTEXT_END_LINE: 57832
- CONTEXT_TOKEN: kind: "problem" | "job" | "time_window" | "workspace";
- EXCERPT_ASCII_ESCAPED:
  ```text
  // Scope
  scope: {
    kind: "problem" | "job" | "time_window" | "workspace";
    problem_id?: string;
    job_id?: string;
    time_range?: {
      start: string;
      end: string;
    };
    wsid?: string;
  };
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.10 DebugBundleExporter trait contract
- CONTEXT_START_LINE: 58243
- CONTEXT_END_LINE: 58249
- CONTEXT_TOKEN: /// Export a debug bundle for the given scope.
- EXCERPT_ASCII_ESCAPED:
  ```text
  #[async_trait]
  pub trait DebugBundleExporter: Send + Sync {
      /// Export a debug bundle for the given scope.
      ///
      /// # Arguments
      /// * `request` - Export parameters including scope and redaction mode
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 10.5.7 export manifest requirements
- CONTEXT_START_LINE: 58670
- CONTEXT_END_LINE: 58676
- CONTEXT_TOKEN: - `job_id`, `workflow_run_id`
- EXCERPT_ASCII_ESCAPED:
  ```text
  Must include:
  - `bundle_kind`, `schema_version`, `created_at`
  - workspace identifier (wsid) and exported entity IDs
  - `job_id`, `workflow_run_id`
  - `export_profile_id`
  - tool/renderer versions used
  - input hashes (raw/canonical) and output hash list
  ```
