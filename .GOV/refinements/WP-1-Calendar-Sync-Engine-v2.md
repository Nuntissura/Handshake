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
- WP_ID: WP-1-Calendar-Sync-Engine-v2
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-21T08:42:37.6999587Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- SPEC_TARGET_SHA1: 231fea32a73934e9f66e00a3bbe26c80b7e058c9
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja210420261037
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Calendar-Sync-Engine-v2
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- The v1 refinement correctly established that this packet is not greenfield on storage: the calendar substrate already exists and remains the required base for any sync path.
- The v1 packet is now proven materially underscoped. The live workflow dossier records that the missing product gap is not only engine registration and runtime adapter wiring, but also the capability contract that governs `workflow_run` execution for calendar sync.
- The already-proven delta from v1 is fourfold: `capabilities.rs` is missing `calendar.sync.read` and `calendar.sync.write`; `workflow_run` still resolves to the Analyst / `doc.summarize` capability contract; `CapabilityGate` rejects requested calendar capabilities as `HSK-4001 UnknownCapability`; and `mex_tests` lacks `calendar_sync` coverage.
- Build-order and board truth still place the base WP on the same dependency spine as v1: it depends on Calendar Storage, MEX Runtime, and Workflow Engine, and it blocks the downstream Calendar Law Compliance Tests packet.
- The governing spec is not missing law text. The scope defect is implementation-contract scope: v1 did not explicitly carry the capability-profile and workflow-capability surfaces that the spec already makes mandatory for calendar sync.
- Downstream consumers remain separate. Lens projection, ACE/policy integration, mailbox correlation, and law-compliance suites still depend on `calendar_sync`, but none belong inside this v2 engine packet.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 45m
- SEARCH_SCOPE: anchor-only finish from the settled v1 refinement, v1 packet, live workflow dossier, build-order/task-board/traceability records, and the current Master Spec v02.181.
- REFERENCES:
  - .GOV/refinements/WP-1-Calendar-Sync-Engine-v1.md
  - .GOV/task_packets/WP-1-Calendar-Sync-Engine-v1/packet.md
  - .GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md
  - .GOV/roles_shared/records/BUILD_ORDER.md
  - .GOV/roles_shared/records/TASK_BOARD.md
  - .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
  - .GOV/spec/Handshake_Master_Spec_v02.181.md
- PATTERNS_EXTRACTED:
  - Keep the v1 storage and workflow-runtime foundation assumptions. The v2 delta is additive scope repair, not a reset.
  - Treat calendar capability profiles as product contracts, not as implied helper wiring. The spec already makes them explicit backend and AI-job boundaries.
  - Keep the execution path singular: `calendar_sync` must remain a workflow/MEX path with capability enforcement and observable results, never a side helper.
  - Pull the capability repair into the signed packet itself. The dossier proved the packet cannot clear truthfully if capability identifiers, workflow capability routing, and MEX tests remain outside scope.
  - Preserve v1 downstream boundaries. This packet still does not own Lens, policy-routing, mailbox correlation, or law-suite expansion beyond the tripwire tests required for the engine path.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the v1 runtime-bridge plan and the settled storage substrate; ADAPT the packet so capability-profile binding, workflow capability routing, `CapabilityGate` acceptance, and `mex_tests` coverage are first-class in-scope code surfaces; REJECT treating the `HSK-4001 UnknownCapability` finding as a downstream or validator-only concern.
- LICENSE/IP_NOTES: Local repository and spec sources only. No external code or research is required.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The current Master Spec already makes `capability_profile_id` explicit for Calendar and for AI jobs. The gap is packet scope and product realization, not missing normative language.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is an internal repo-governed activation repair grounded in the current Master Spec, the existing v1 packet/refinement, and live workflow dossier evidence. No external source changes the governing decision.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE
- RESEARCH_GAPS_TO_TRACK:
  - NONE
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE

### RESEARCH_DEPTH (prevent shallow source logging)
- ADOPT_PATTERNS:
  - NONE
- ADAPT_PATTERNS:
  - NONE
- REJECT_PATTERNS:
  - NONE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- The v1 expectation remains: this packet must materialize the spec-named `calendar_sync_result` output and keep sync and mutation activity trace-linked to `job_id` and `workflow_run_id` through the existing workflow/MEX/Flight Recorder stack.
- The v2 delta adds one hard requirement: capability allow/deny posture for the `calendar_sync` path must be inspectable as real workflow capability enforcement, not as an implicit preflight assumption or fallback to another profile.
- `CalendarSourceSyncState` remains durable recovery evidence, not a replacement for Flight Recorder traces. The engine must maintain both when it runs.
- Diagnostics must remain provider-safe: no tokens, secrets, or raw sensitive payloads in logs or artifacts.

### RED_TEAM_ADVISORY (security failure modes)
- Capability-contract drift risk: if `workflow_run` keeps resolving to Analyst or another unrelated profile, the calendar sync path can remain blocked or, worse, run under the wrong authority surface.
- Unknown-capability blind spot: if the packet adds runtime code but leaves `calendar.sync.read` / `calendar.sync.write` unmapped, the path will continue failing at `CapabilityGate` with `HSK-4001`, producing false progress.
- Test illusion risk: if `mex_tests` still lacks `calendar_sync` coverage, registry or runtime edits can look landed while the governed execution path remains unproven.
- Shadow-path risk: if capability repair is bypassed with helper code or hardcoded exemptions, the packet violates the spec's workflow-only mutation and capability law.
- Read-only drift risk: even after capability repair, source write policy must still fail closed for import-only or read-only sources.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-CalendarSyncInput
  - PRIM-CalendarMutation
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-CalendarSyncInput
  - PRIM-CalendarMutation
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - PRIM-CalendarSyncInput
  - PRIM-CalendarMutation
  - PRIM-CalendarSource
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarEventUpsert
- NOTES:
  - The v2 scope delta does not discover new PRIM IDs. It tightens the execution contract over already-existing calendar primitives and the already-declared `capability_profile_id` contract.
  - Appendix 12 already contains the primitive and feature rows this packet needs; the new requirement is to honor those contracts in code and test coverage.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current appendix already contains the calendar sync, mutation, source, storage, and export-posture primitive IDs needed for this packet.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: This refinement surfaced a packet-scope capability-contract gap, not a missing primitive-registration gap.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: FEAT-CALENDAR and the relevant primitive and interaction rows already exist in the current spec.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet still implements the engine/runtime substrate rather than a new user-facing control surface.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The current interaction matrix already captures Calendar -> Capabilities/Consent, Calendar -> AI Job Model, and Calendar -> Storage Portability. v2 realizes those existing edges inside packet scope.
- APPENDIX_MAINTENANCE_NOTES:
  - NONE
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: No spatial modeling in scope. | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: No fabrication behavior in scope. | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: No simulation physics in scope. | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: No simulation runtime in scope. | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: No hardware control in scope. | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: Generic orchestration is reused, but this packet does not implement `engine.director`. | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: No composition surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: No visual generation surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: No publishing engine surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: No food-planning logic in scope. | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: No food safety surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: No logistics surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: The packet preserves durable records but does not implement archival logic. | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: No library/indexing engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: The current wrong Analyst/doc.summarize fallback is a capability-contract bug to remove, not Analyst-engine feature work. | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: No generic wrangling engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: `calendar_sync` persists into the portable calendar storage substrate and updates durable sync-state rows; the capability contract must also align with that governed execution path. | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: No governance-rule-authoring surface in product code is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: No onboarding/help engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: NOT_TOUCHED | NOTES: CalendarScopeHint and policy-profile routing remain downstream. | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: No version-management engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: No sandbox engine surface is implemented here. | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: The packet must keep sync and capability-enforcement execution traceable without inventing a shadow evidence path. | STUB_WP_IDS: WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Law-Compliance-Tests-v1
  - PILLAR: Calendar | STATUS: TOUCHED | NOTES: This is the missing workflow-driven calendar engine plus the missing capability contract required to make it executable. | STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: No editor surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: No document-editing surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: No spreadsheet surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: Locus may later consume sync-backed calendar facts, but this packet does not implement a Locus-specific surface. | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: No Loom-specific capability is in scope. | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: No product work-packet surface is implemented here. | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: No product task-board surface is implemented here. | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: The packet uses workflow/runtime infrastructure, not the micro-task feature surface. | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: `calendar_sync` should remain visible through existing job/workflow inspection surfaces rather than hidden helpers. | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: This packet is a runtime and capability-contract packet: engine registration, workflow dispatch, capability gating, and evidence. | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: No spec-to-prompt surface is implemented here. | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: The engine must reuse the portable calendar storage substrate and keep behavior consistent across both backends. | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: The sync engine and its capability boundaries populate the canonical calendar source/event substrate that later routing and projection consumers depend on. | STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: No Stage surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: No Studio surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: Lens remains a downstream consumer packet rather than part of the engine-runtime work here. | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: No distillation surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: CalendarScopeHint and policy routing remain downstream even though v2 now makes capability-profile posture explicit at job boundaries. | STUB_WP_IDS: WP-1-Calendar-Policy-Integration-v1
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: No retrieval pipeline is implemented here. | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: NEEDS_STUBS

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
  - PILLAR: Calendar | CAPABILITY_SLICE: governed sync-engine registration and execution | SUBFEATURES: engine registry row, runtime adapter install, workflow dispatch contract, idempotent source/event upserts | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation, PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This remains the direct activation target.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: calendar capability profile and workflow capability routing | SUBFEATURES: `calendar.sync.read`, `calendar.sync.write`, workflow-run capability mapping, `CapabilityGate` acceptance, fail-closed denials | PRIMITIVES_FEATURES: PRIM-CalendarSource, PRIM-CalendarSourceWritePolicy, PRIM-CalendarMutation | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the material v2 scope delta proven by the dossier.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: auditable sync and mutation evidence | SUBFEATURES: result artifact, trace linkage, provider-safe diagnostics, retry visibility | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation, PRIM-CalendarSourceSyncState | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The engine must be inspectable through existing runtime evidence surfaces rather than helper-local logs.
  - PILLAR: Command Center | CAPABILITY_SLICE: inspectable workflow and capability outcomes for calendar sync | SUBFEATURES: job visibility, workflow-run status, operator-visible denial posture, failure summaries | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Existing runtime inspection surfaces should expose the sync job without a packet-owned UI.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: portable sync behavior across both supported backends | SUBFEATURES: storage reuse, sync-state persistence, repeat-run consistency, backend parity | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The engine must reuse the portable storage substrate instead of introducing backend-specific sync behavior.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: canonical synced calendar substrate for downstream routing and projection | SUBFEATURES: durable source sync posture, stable event identity, queryable event windows | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet should produce the canonical data shape that downstream routing and projection packets consume.
  - PILLAR: Calendar | CAPABILITY_SLICE: user-facing Lens projection and filters | SUBFEATURES: agenda/timeline rendering, diagnostics display, user controls | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Lens-v3 | NOTES: Lens remains a downstream UI consumer packet.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: policy and routing consumers read sync-backed posture through a dedicated integration packet | SUBFEATURES: policy-profile selection, scope-hint routing, downstream projections | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Policy-Integration-v1 | NOTES: This packet produces the routing inputs without also owning the routing layer.
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
  - Capability: calendar source sync execution | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_sync_result | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: The packet must make the sync run as a governed workflow job rather than a helper thread.
  - Capability: provider adapter calls inside governed runtime | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: MEX_RUNTIME | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: tool_call/tool_result plus diagnostics | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Adapter work belongs inside the existing MEX gate stack and must remain capability-scoped.
  - Capability: calendar mutation apply discipline | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_mutation | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Successful patch application must remain trace-linked and workflow-governed.
  - Capability: calendar capability contract evaluation | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: capability allow/deny evidence | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: The `calendar_sync` path cannot truthfully retain Analyst/doc.summarize capability routing.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - The highest-ROI v2 combination is still the missing runtime bridge, but now joined with the missing capability contract that makes that bridge reachable.
  - The current interaction matrix already carries the important calendar combinations this packet realizes: Calendar -> Storage Portability, Calendar -> Capabilities/Consent, Calendar -> AI Job Model, and Calendar -> Spec Router.
  - No new matrix edge is required to justify v2. The missing work is product realization of already-modeled capability and calendar contracts.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- PRIMITIVE_MATRIX_CANDIDATE_EDGE_COUNT: 0
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: The current spec already captures the cross-feature edges this packet needs. The work is product realization of those existing combos, not discovery of a missing matrix edge.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This packet is bounded to internal calendar/runtime/capability wiring under the current spec and the already-settled v1 dossier evidence.
- SOURCE_SCAN:
  - NONE
- MATRIX_GROWTH_CANDIDATES:
  - NONE
- ENGINEERING_TRICKS_CARRIED_OVER:
  - NONE
- MATRIX_RESEARCH_VERDICT: NOT_APPLICABLE

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: existing calendar storage plus governed sync-engine registration removes the shadow-pipeline gap | Pillars: Calendar, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSyncInput, PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | Resolution: IN_THIS_WP | Stub: NONE | Notes: This remains the direct blocker-removal combination.
  - Combo: workflow capability routing plus calendar capability identifiers removes the `HSK-4001 UnknownCapability` hard-stop | Pillars: Execution / Job Runtime, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSource, PRIM-CalendarMutation | Resolution: IN_THIS_WP | Stub: NONE | Notes: This is the new high-ROI v2 delta.
  - Combo: provider-safe result evidence keeps sync inspectable in existing workflow consoles | Pillars: Flight Recorder, Command Center | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSyncInput, PRIM-CalendarMutation | Resolution: IN_THIS_WP | Stub: NONE | Notes: Sync failures and successes must be operator-visible through existing runtime evidence surfaces.
  - Combo: portable storage reuse keeps SQLite and Postgres sync behavior aligned | Pillars: SQL to PostgreSQL shift readiness, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | Resolution: IN_THIS_WP | Stub: NONE | Notes: The engine must not introduce backend-specific sync semantics.
  - Combo: idempotent synced rows become the canonical calendar substrate for downstream consumers | Pillars: LLM-friendly data, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | Resolution: IN_THIS_WP | Stub: NONE | Notes: Canonical event identity and sync posture belong in durable storage, not helper caches.
  - Combo: read-only and write-policy enforcement keeps remote provider access fail-closed | Pillars: Calendar, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarMutation, PRIM-CalendarSourceWritePolicy, PRIM-CalendarEventExportMode | Resolution: IN_THIS_WP | Stub: NONE | Notes: Even an import-first MVP must still prove that remote writes cannot slip through.
  - Combo: Lens projections consume the same synced storage truth rather than bespoke provider fetches | Pillars: Calendar, LLM-friendly data, Atelier/Lens | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Lens-v3 | Notes: Lens remains separate, but this packet removes its biggest upstream runtime blocker.
  - Combo: policy and routing consumers read sync-backed posture through a dedicated integration packet | Pillars: LLM-friendly data, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Policy-Integration-v1 | Notes: This packet should produce the routing inputs without also owning the routing layer.
  - Combo: law-compliance validation can finally exercise a real governed sync path | Pillars: Flight Recorder, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarMutation, PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Law-Compliance-Tests-v1 | Notes: Validator-heavy law-compliance coverage remains blocked until the engine and capability path both exist.
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_REASON: The highest-ROI runtime combinations belong in this packet, while the user-facing and validator-heavy consumers already have separate stub homes and should stay there.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- SCAN_SCOPE: current v1 refinement and packet, current Master Spec v02.181, live workflow dossier findings, and the build-order/task-board/traceability projections
- MATCHED_STUBS:
  - Artifact: WP-1-Calendar-Lens-v3 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Lens remains the downstream projection/UI consumer packet and should not be folded into this engine packet.
  - Artifact: WP-1-Calendar-Policy-Integration-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: ACE/routing and capability-policy projection remain a separate packet.
  - Artifact: WP-1-Calendar-Law-Compliance-Tests-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: validator-heavy law-compliance coverage remains blocked by this packet.
- MATCHED_ACTIVE_PACKETS:
  - Artifact: WP-1-Calendar-Sync-Engine-v1 | BoardStatus: IN_PROGRESS | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: v1 targeted the runtime bridge but underspecified the capability-contract work now proven necessary.
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Calendar-Storage-v2 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides the durable source/event substrate and sync-state row models that this packet must consume.
  - Artifact: WP-1-Workflow-Engine-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: workflow persistence and job execution exist, but the calendar-specific execution and capability branch is still absent under signed scope.
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/capabilities.rs | Artifact: WP-1-Calendar-Sync-Engine-v1 | Covers: execution | Verdict: NOT_PRESENT | Notes: `calendar.sync.read` and `calendar.sync.write` are absent.
  - Path: ../handshake_main/src/backend/handshake_core/src/capabilities.rs | Artifact: WP-1-Calendar-Sync-Engine-v1 | Covers: execution | Verdict: PARTIAL | Notes: `workflow_run` still resolves to Analyst / `doc.summarize` capability posture instead of a calendar sync contract.
  - Path: ../handshake_main/src/backend/handshake_core/src/mex/gates.rs | Artifact: WP-1-Calendar-Sync-Engine-v1 | Covers: execution | Verdict: PARTIAL | Notes: requested calendar capabilities currently fail as `HSK-4001 UnknownCapability`.
  - Path: ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | Artifact: WP-1-Calendar-Sync-Engine-v1 | Covers: execution | Verdict: PARTIAL | Notes: existing MEX tests do not yet cover `calendar_sync`.
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: v2 is the bounded scope repair required to align the packet with already-proven code reality and already-existing spec law, without widening into downstream packets.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet still implements engine/runtime plumbing only. It should surface through existing workflow/job inspection surfaces, but it does not add a new dedicated UI owned by this packet.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: NONE | Type: NONE | Tooltip: NONE | Notes: UI ownership lives in downstream packets such as Calendar Lens and broader Command Center work.
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - N/A (no new direct UI surface in this packet)
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No new GUI is implemented here.
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
- AGENT_ID: ActivationManager
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.181]
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Calendar-Storage, WP-1-MEX-v1-2-Runtime, WP-1-Workflow-Engine
- BUILD_ORDER_BLOCKS: WP-1-Calendar-Law-Compliance-Tests
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.181.md \u00a72.6.6 capability profiles + \u00a710.4 Calendar capability contracts + \u00a76.0.1 workflow capability checks
- WHAT: Register and implement `calendar_sync` as a governed workflow-driven mechanical engine and repair the calendar-specific capability contract surfaces that currently block `workflow_run` execution and `mex_tests` proof coverage.
- WHY: The spec already defines `calendar_sync` as the only legal path for external calendar mutation and provider sync, and it already defines explicit capability profiles for Calendar and AI jobs. The v1 dossier proved the current product is blocked at both layers: missing engine/runtime wiring and missing calendar capability routing.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- OUT_OF_SCOPE:
  - Multi-provider breadth beyond one truthful MVP adapter/import path
  - Rich bidirectional write-back UX, conflict-resolution policy, and multi-source reconciliation
  - Calendar Lens UI implementation
  - CalendarScopeHint / ACE policy-routing implementation
  - Calendar correlation export and mailbox correlation product logic
  - Repo-governance tooling or protocol changes unrelated to this packet
- TEST_PLAN:
  ```bash
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  ```
- DONE_MEANS:
  - `calendar_sync` exists in `mechanical_engines.json` and is loadable by `MexRegistry`.
  - The workflow runtime installs a `calendar_sync` adapter and dispatches the chosen protocol/job contract through governed workflow execution instead of ad hoc helpers.
  - `capabilities.rs` exposes the calendar sync capability contract needed for this path, and `workflow_run` no longer inherits the Analyst / `doc.summarize` capability contract for the calendar sync flow.
  - `CapabilityGate` continues to fail closed for invalid requests but no longer blocks the signed calendar sync path as `HSK-4001 UnknownCapability`.
  - `mex_tests` contains `calendar_sync` registry/runtime/capability coverage, and the existing storage tests still prove idempotent sync-state behavior.
  - WP Validator and Integration Validator pass, and the validated code is integrated into `main`.
- PRIMITIVES_EXPOSED:
  - PRIM-CalendarSyncInput
  - PRIM-CalendarMutation
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- SEARCH_TERMS:
  - calendar_sync
  - calendar.sync.read
  - calendar.sync.write
  - workflow_run
  - doc.summarize
  - UnknownCapability
  - HSK-4001
  - capability_profile_id
  - CalendarSourceSyncState
  - calendar_mutation
- RUN_COMMANDS:
  ```bash
  rg -n "calendar_sync|calendar.sync.read|calendar.sync.write|workflow_run|doc.summarize|UnknownCapability|HSK-4001|capability_profile_id" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "calendar_sync is added as a one-off helper instead of a MEX/workflow contract" -> "capability gates, provenance, and replay guarantees drift immediately"
  - "workflow_run keeps the wrong capability contract" -> "calendar sync remains blocked or runs under unrelated authority semantics"
  - "calendar capabilities stay undefined while runtime code lands" -> "the path still fails as `HSK-4001 UnknownCapability` and the packet reports false progress"
  - "tests cover registry happy-path only and skip capability/routing execution" -> "the packet looks landed while real governed execution still fails"
  - "read-only or import-only sources can still push remote mutations" -> "consent and capability law is violated"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - BUILD_ORDER.md already lists the correct dependency/blocker topology for this packet and its downstream law-compliance blocker.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSE_ROWS:
  - CLAUSE: surface mutation discipline plus write gate | WHY_IN_SCOPE: the packet must make `calendar_sync` the real workflow-only mutation path instead of a paper contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: direct helper or UI-side writes can still bypass governed execution
  - CLAUSE: workflow capability profile and required-capabilities contract | WHY_IN_SCOPE: the v2 packet must repair the calendar sync path so `workflow_run` and capability gating use the intended calendar capability contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/capabilities.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/gates.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: `calendar_sync` remains blocked by the wrong capability contract or by `HSK-4001 UnknownCapability`
  - CLAUSE: Cross-Tool Interaction Map no-shadow-pipeline rule | WHY_IN_SCOPE: provider sync must run through Workflow Engine + MEX runtime, not hidden background helpers | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests | RISK_IF_MISSED: provider access happens outside the contract the spec requires
  - CLAUSE: `calendar_sync` engine contract and output | WHY_IN_SCOPE: the packet exists to realize the engine input/behavior/output contract already named in the spec | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | RISK_IF_MISSED: the engine may exist nominally but still fail to honor spec-defined behavior
  - CLAUSE: `CalendarSourceSyncState` as single source of truth for recovery | WHY_IN_SCOPE: retries, backoff, and recovery are core parts of a sync engine, not optional extras | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: the engine becomes non-recoverable or duplicates data under retry
  - CLAUSE: MCP/provider adapter guidance plus read-only mode | WHY_IN_SCOPE: the spec explicitly prefers provider access through tools inside the engine and names read-only behavior as a first-class posture | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: provider access and write posture drift from the calendar law contract

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CONTRACT_ROWS:
  - CONTRACT: `calendar_sync` engine registry contract | PRODUCER: mechanical_engines.json | CONSUMER: mex/registry.rs, workflows.rs, tests/mex_tests.rs | SERIALIZER_TRANSPORT: JSON engine registry | VALIDATOR_READER: mex_tests.rs | TRIPWIRE_TESTS: mex registry/runtime tests | DRIFT_RISK: engine is declared but not executable, or executable but not declared consistently
  - CONTRACT: calendar sync job input / protocol contract | PRODUCER: workflows.rs job/profile parser and engine runner | CONSUMER: calendar-sync adapter implementation, storage layer, validators | SERIALIZER_TRANSPORT: workflow payload plus PlannedOperation inputs | VALIDATOR_READER: workflow/job tests plus validator inspection | TRIPWIRE_TESTS: targeted calendar-sync execution tests plus full cargo test | DRIFT_RISK: job payload shape and adapter expectations silently diverge
  - CONTRACT: calendar sync capability contract | PRODUCER: capabilities.rs plus workflow capability-profile binding | CONSUMER: workflows.rs, mex/gates.rs, tests/mex_tests.rs | SERIALIZER_TRANSPORT: capability profile ids and requested capability strings | VALIDATOR_READER: mex_tests.rs plus validator inspection | TRIPWIRE_TESTS: mex capability-path tests plus full cargo test | DRIFT_RISK: requested calendar capabilities remain undefined, misnamed, or bound to the wrong workflow profile
  - CONTRACT: `CalendarSourceSyncState` durable recovery contract | PRODUCER: storage/calendar.rs plus engine runner | CONSUMER: sqlite.rs, postgres.rs, later recovery/retry flows | SERIALIZER_TRANSPORT: sqlx row mapping and JSON-ish sync-state payloads | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: calendar storage tests plus sync retry/idempotency tests | DRIFT_RISK: sync token/backoff/watermark state is lost or inconsistently updated
  - CONTRACT: calendar event upsert/idempotency contract | PRODUCER: engine runner and adapter | CONSUMER: storage backends and later Lens/policy consumers | SERIALIZER_TRANSPORT: storage upsert calls keyed by source/external identity | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: calendar storage tests plus repeat-sync tests | DRIFT_RISK: repeated sync runs duplicate events or destabilize identity

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - targeted validator review of capability-contract wiring, engine registration, runtime adapter installation, gated sync execution, and sync-state durability
- CANONICAL_CONTRACT_EXAMPLES:
  - a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback
  - a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes
  - a repeated identical sync run that keeps stable identity and produces no duplicate events
  - a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Audit the existing storage/runtime seams first so the packet reuses completed work instead of recreating it.
  - Repair the calendar sync capability contract before or alongside runtime adapter wiring so the path can execute under the intended `workflow_run` profile.
  - Add `calendar_sync` to the mechanical engine registry and wire its adapter into `build_mex_runtime(...)`.
  - Reuse existing calendar storage upserts and sync-state contracts for the smallest truthful sync path.
  - Extend `mex_tests` and storage/runtime tests to prove engine registration, governed execution, capability routing, idempotent sync behavior, and fail-closed posture.
  - Re-run the proof commands from the product worktree until they pass cleanly.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- CARRY_FORWARD_WARNINGS:
  - Do not reimplement calendar storage or invent shadow tables; the completed storage packet is the substrate.
  - Do not add ad hoc background sync threads or direct provider clients outside workflow/MEX runtime.
  - Do not silently reuse Analyst/doc.summarize or any unrelated `workflow_run` capability contract for the calendar sync path.
  - Do not widen the packet into Lens, ACE policy integration, multi-provider breadth, or rich write-back UX.
  - Do not mint new PRIM IDs or new top-level Flight Recorder schemas to paper over runtime gaps.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - engine registration and runtime adapter installation
  - workflow dispatch and governed execution path
  - workflow capability profile binding and requested-capability routing
  - capability posture plus read-only/write-policy fail-closed behavior
  - sync-state durability and repeat-run idempotency
  - trace/result evidence linkage
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - verify `calendar_sync` still exists in `mechanical_engines.json` on `main`
  - verify workflow runtime still installs the calendar adapter on `main`
  - verify `workflow_run` no longer routes the calendar sync path through Analyst/doc.summarize capability posture
  - verify there is still no direct provider-write path that bypasses capability gates

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- NOT_PROVEN_ITEMS:
  - This refinement does not prove whether the smallest truthful MVP should be local ICS import only or one external provider in read-only mode; both remain spec-compatible and must be resolved during implementation.
  - This refinement does not freeze the exact protocol_id/schema naming for the calendar-sync job contract; coder and validators must align those names to current workflow conventions.
  - This refinement does not prove bidirectional write-back, conflict resolution, CalendarScopeHint policy projection, Lens UX, or downstream correlation behavior.

### DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (The current appendix already contains the calendar sync, mutation, capability-profile, and storage primitive IDs this packet needs.)
- DISCOVERY_STUBS: NONE_CREATED (All relevant downstream consumer and validator stubs already existed before this refinement pass.)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (The interaction matrix already captures the important Calendar cross-feature edges this packet realizes.)
- DISCOVERY_UI_CONTROLS: NONE_APPLICABLE (This packet does not own a new GUI surface.)
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED (The current spec already provides the normative engine/runtime and capability-contract rules required to activate this WP.)
- DISCOVERY_JUSTIFICATION: This refinement still delivered high value by converting a runtime-only v1 scope into a truthful v2 packet boundary that now includes the already-proven capability-contract gap, without reopening broad discovery.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Master Spec v02.181 explicitly names the workflow-only mutation discipline, `calendar_sync` as the engine for external calendar mutation/sync, `capability_profile_id` as a canonical Calendar backend contract, and capability profiles as the AI-job write/read authority boundary. The v2 packet gap is product realization of those runtime and capability contracts, not missing spec intent.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE. The remaining uncertainty is implementation-choice uncertainty (which MVP adapter path is smallest and truthful), not ambiguity in the governing spec clauses.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The current Master Spec already provides a sufficiently specific calendar sync law and capability-contract law. This activation pass should produce packet-scope and product-runtime wiring work, not new normative spec text.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a72.6.6 AI Job Model surface mutation discipline
- CONTEXT_START_LINE: 9408
- CONTEXT_END_LINE: 9410
- CONTEXT_TOKEN: Any external calendar mutation is executed only by the mechanical engine `calendar_sync`
- EXCERPT_ASCII_ESCAPED:
  ```text
  Surface mutation discipline (non-negotiable)
  - Calendar UI remains view-only; all calendar mutations are expressed as validated patch-sets and applied by the host after Gate checks.
  - Any external calendar mutation is executed only by the mechanical engine `calendar_sync` under explicit capability+consent.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a75.4.6.4 Calendar Law compliance tests
- CONTEXT_START_LINE: 23624
- CONTEXT_END_LINE: 23628
- CONTEXT_TOKEN: Outbox is idempotent
- EXCERPT_ASCII_ESCAPED:
  ```text
  Key invariants covered:
  - RBC is view-only: UI may render calendar state, but MUST NOT write to calendar tables directly.
  - All mutations are patch-sets: changes flow through the AI Job Model + Workflow Engine, then `calendar_sync` applies them.
  - External writes are gated: any provider-side mutation requires explicit capabilities + consent prompts.
  - Outbox is idempotent: every outbound change has a stable idempotency key; retries must not duplicate events.
  - Full observability: every calendar mutation emits Flight Recorder spans and links back to `job_id`.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 Mutation governance (Hard Invariant) [ilja251220250127]
- CONTEXT_START_LINE: 55965
- CONTEXT_END_LINE: 55969
- CONTEXT_TOKEN: calendar_mutation
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Direct database writes to `calendar_events` are PROHIBITED from the API layer or UI components.
  - All mutations MUST be submitted as `CalendarMutation` patches via a `WorkflowRun` targeting the `calendar_sync` mechanical engine.
  - Every successful mutation MUST emit a Flight Recorder span of type `calendar_mutation` with a back-link to the `job_id`.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 Mutation and governance rules
- CONTEXT_START_LINE: 55983
- CONTEXT_END_LINE: 55990
- CONTEXT_TOKEN: Patch-sets are the only write primitive
- EXCERPT_ASCII_ESCAPED:
  ```text
  - No direct UI writes: UI gestures emit jobs; only the host applies patches after validation and gates.
  - Patch-sets are the only write primitive: all calendar writes (local or external) are expressed as validated patch-sets with preconditions, effect, and provenance.
  - External writes are explicitly gated: any write that leaves the device requires capability + user confirmation unless the source is configured as `auto_export=true`.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 Mechanical engine: `calendar_sync`
- CONTEXT_START_LINE: 56205
- CONTEXT_END_LINE: 56227
- CONTEXT_TOKEN: calendar_sync_result
- EXCERPT_ASCII_ESCAPED:
  ```text
  Mechanical engine: `calendar_sync`
  Engine input includes `CalendarSource.id`, direction, and time_window.
  Behavior includes pulling from provider sources, pushing mirrored events when allowed, and always recording sync activity in Flight Recorder.
  Output includes `calendar_sync_result` plus updated CalendarEvent rows.
  All external writes are capability-gated and must go through the Workflow Engine, not ad hoc helpers.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 `CalendarSourceSyncState`
- CONTEXT_START_LINE: 56316
- CONTEXT_END_LINE: 56323
- CONTEXT_TOKEN: CalendarSourceSyncState
- EXCERPT_ASCII_ESCAPED:
  ```text
  Each `CalendarSource` persists a sync state record.
  This is the single source of truth for incremental sync and recovery.
  `CalendarSourceSyncState` carries stage, sync token, last-ok timestamps, and recovery fields.
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 MCP client for external calendars
- CONTEXT_START_LINE: 56838
- CONTEXT_END_LINE: 56842
- CONTEXT_TOKEN: Use these tools **inside** the `calendar_sync` engine
- EXCERPT_ASCII_ESCAPED:
  ```text
  Implement MCP tools that wrap Google Calendar, Outlook/Exchange, and generic CalDAV.
  Use these tools inside the `calendar_sync` engine instead of hardcoding clients.
  This lets the orchestrator call provider operations uniformly regardless of provider.
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 read-only source posture
- CONTEXT_START_LINE: 57016
- CONTEXT_END_LINE: 57019
- CONTEXT_TOKEN: write_back=false
- EXCERPT_ASCII_ESCAPED:
  ```text
  Some sources may be used in read-only mode.
  `CalendarSource` has a flag `write_back=false`.
  `calendar_sync_google` for that source only pulls; it never calls insert/update/delete.
  ```

#### ANCHOR 9
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4 Calendar backend force-multiplier capability contract
- CONTEXT_START_LINE: 55877
- CONTEXT_END_LINE: 55877
- CONTEXT_TOKEN: capability_profile_id
- EXCERPT_ASCII_ESCAPED:
  ```text
  [ADD v02.155] In Phase 1, Calendar is also a backend force multiplier: `CalendarSourceSyncState`, `CalendarSource.write_policy`, `CalendarEvent.export_mode`, `capability_profile_id`, and `CalendarScopeHint` are canonical backend contracts for sync recovery, consent posture, AI-job mutation discipline, and scope-hint routing.
  ```

#### ANCHOR 10
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a72.6.6.5.2 AI Job capability profiles
- CONTEXT_START_LINE: 9763
- CONTEXT_END_LINE: 9777
- CONTEXT_TOKEN: capability_profile_id
- EXCERPT_ASCII_ESCAPED:
  ```text
  Jobs are evaluated under capability profiles:

  | Field | Role |
  |-------|------|
  | `capability_profile_id` | Determines what the job can read/write in the workspace |
  | `access_mode` | Read-only, preview-only, or scoped-apply |
  | `layer_scope` | Which layers (raw/derived/display) are writable |

  Enforcement Points:
  1. Before `queued`: Basic capability check
  2. At `awaiting_validation`: Full capability and policy check
  3. On commit: Final verification that only allowed entities were modified
  ```
