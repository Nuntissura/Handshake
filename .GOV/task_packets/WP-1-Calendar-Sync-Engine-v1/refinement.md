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
- WP_ID: WP-1-Calendar-Sync-Engine-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-21T00:43:35.9115051Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- SPEC_TARGET_SHA1: 231fea32a73934e9f66e00a3bbe26c80b7e058c9
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja210420260315
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Calendar-Sync-Engine-v1
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- The packet is not greenfield on the storage side anymore: `../handshake_main/src/backend/handshake_core/src/storage/calendar.rs`, `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`, `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`, and `../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs` already provide the durable calendar substrate that this packet must reuse.
- The actual missing capability is the workflow-driven `calendar_sync` bridge. `../handshake_main/src/backend/handshake_core/mechanical_engines.json` does not register `calendar_sync`, so the governed MEX registry cannot load the spec-named engine yet.
- The current workflow runtime only installs adapters for `engine.shell` and `engine.media_downloader` in `build_mex_runtime(...)` inside `../handshake_main/src/backend/handshake_core/src/workflows.rs`; there is no `calendar_sync` adapter, profile wiring, or execution path.
- The existing workflow dispatch code contains media-downloader-specific protocol handling but no calendar-sync job contract, which means the current product cannot execute the spec-defined calendar mutation/sync path even though storage primitives already exist.
- The stub, task-board row, and startup gate were all truthful about governance posture: there was no refinement file, no active packet, and no readiness artifact. The activation blocker is governance authoring plus the missing runtime bridge, not missing calendar law text.
- Downstream consumers already have their own backlog homes and must stay separate from this packet: Lens projection, ACE/policy integration, mailbox/correlation, and law-compliance suites all depend on `calendar_sync`, but none belong inside this v1 engine packet.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 60m
- SEARCH_SCOPE: current Master Spec v02.181 calendar/job/runtime clauses, the live stub + build-order/task-board records, the completed calendar storage refinement/packet, and local product code under `../handshake_main/src/backend/handshake_core`.
- REFERENCES:
  - .GOV/spec/Handshake_Master_Spec_v02.181.md sections 2.6.6, 5.4.6.4, 6.0.1, 10.4.1, 10.4.2.1, 11.5, Appendix 12.4, and Appendix 12.6
  - .GOV/task_packets/stubs/WP-1-Calendar-Sync-Engine-v1.md
  - .GOV/roles_shared/records/BUILD_ORDER.md
  - .GOV/roles_shared/records/TASK_BOARD.md
  - .GOV/refinements/WP-1-Calendar-Storage-v2.md
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- PATTERNS_EXTRACTED:
  - Reuse the validated calendar storage substrate instead of inventing a parallel sync-specific schema.
  - Follow the existing MEX runtime shape already used by `engine.media_downloader`: registry row in `mechanical_engines.json`, runtime adapter installation in `build_mex_runtime(...)`, workflow-dispatch entry points, and Flight Recorder-visible execution.
  - Keep all provider access inside the governed workflow/MEX path; do not create ad hoc background sync helpers, hidden threads, or UI-side provider clients.
  - Keep the v1 packet narrow: one truthful MVP path (local ICS import or one provider in read-only mode) is acceptable; multi-provider breadth, conflict-heavy bidirectional sync, and UX-rich write-back flows remain explicitly out of scope.
  - Preserve idempotent storage semantics around source identity, external IDs, and sync-state recovery rather than letting the engine bypass the existing storage contracts.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the existing calendar storage package plus the current media-downloader/MEX runtime pattern as the implementation baseline; ADAPT those seams into a calendar-specific engine contract with capability gating, sync-state durability, and provider-safe observability; REJECT greenfield storage rewrites, shadow sync daemons, ungated direct provider clients, and widening v1 into a multi-provider/full-write-back program.
- LICENSE/IP_NOTES: Local repository patterns only. No external code reuse is proposed.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The current Master Spec already names `calendar_sync`, its workflow-only mutation posture, its input/output contract, sync-state truth, adapter strategy, read-only mode, and visibility obligations with enough precision to activate this packet. The gap is product realization, not missing normative language.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is a product-internal runtime activation packet grounded in the current Master Spec plus local code truth under `../handshake_main`. No external source changes the governing decision about how Handshake must wire `calendar_sync`.
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
- This packet must materialize the spec-named `calendar_sync_result` execution output and keep sync/mutation activity trace-linked to `job_id` and `workflow_run_id` through the existing workflow/MEX/Flight Recorder stack.
- `calendar_mutation` is already the normative span/event contract for successful patch application. This refinement does not request a new top-level Flight Recorder schema; it requires the implementation to honor the existing calendar law contract.
- Source-level progress, retries, and failures may emit diagnostics or runtime events through existing envelopes, but they must remain provider-safe: no tokens, secrets, or raw sensitive payloads in logs.
- `CalendarSourceSyncState` updates are durable recovery evidence, not a replacement for Flight Recorder traces. The engine must maintain both when it runs.

### RED_TEAM_ADVISORY (security failure modes)
- Capability bypass risk: if `calendar_sync` is wired outside the MEX gate stack or invoked through a helper path, the packet immediately violates the spec's workflow-only mutation law.
- Secret leakage risk: provider credentials, sync tokens, and remote identifiers are easy to leak through logs, diagnostics, or result artifacts unless the engine redacts aggressively.
- Read-only source drift: sources configured for import-only posture must fail closed on any push/write-back path. A partial implementation that silently writes anyway is worse than no sync.
- Idempotency drift: repeat pulls or retries must not duplicate events or create unstable instance identities. Reusing existing storage contracts is the intended mitigation.
- Recovery drift: if sync-state fields are not updated consistently with engine execution, backoff, retry, and invalid-token recovery will become lossy and un-auditable.
- Untrusted content risk: external calendar titles, descriptions, attendees, and links are untrusted data and must not be promoted into prompt surfaces or diagnostics without the later policy/redaction rules.

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
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarEvent
  - PRIM-CalendarEventUpsert
- NOTES:
  - Appendix 12.4 already contains the sync, mutation, source, storage, and export-posture primitive IDs this packet needs. The implementation gap is runtime wiring, not missing primitive identity.
  - Existing product code already realizes most storage-side calendar enums and structs. The sync engine must consume those contracts instead of minting a shadow model family.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The current appendix already contains the calendar sync and calendar storage primitive IDs needed for this packet. No new primitive registration is required before activation.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: This refinement discovered a missing product runtime bridge, not a missing appendix primitive.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: FEAT-CALENDAR, the calendar primitive rows, and the relevant interaction-matrix coverage already exist in the current spec.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This packet implements the engine/runtime substrate, not a new user-facing control surface.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The current interaction matrix already captures the important Calendar edges for storage portability, capabilities/consent, AI-job execution, and routing. This packet realizes those existing edges in product code.
- APPENDIX_MAINTENANCE_NOTES:
  - NONE
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: No spatial modeling in scope. | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: No fabrication or manufacturing behavior in scope. | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: No simulation physics in scope. | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: No simulation engine behavior in scope. | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: No hardware control in scope. | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: The packet relies on generic workflow orchestration, but it does not implement `engine.director`. | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: No composition surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: No visual generation surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: No publishing engine surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: No food-planning logic in scope. | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: No food safety surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: No logistics surface in scope. | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: The packet preserves durable records but does not implement archival logic. | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: No library/indexing engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: No analytics engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: No generic wrangling engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: `calendar_sync` persists into the portable calendar storage substrate and updates durable sync-state rows; that makes the DBA/storage law directly relevant even though the packet is not a schema-only effort. | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: No governance-rule-authoring surface in product code is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: No onboarding/help engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: NOT_TOUCHED | NOTES: CalendarScopeHint and policy-profile routing remain downstream of this packet; this engine packet only preserves the durable calendar substrate those later flows consume. | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: No version-management engine work is in scope. | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: No sandbox engine surface is implemented here. | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: The packet must make sync and mutation execution traceable without inventing a shadow evidence path. | STUB_WP_IDS: WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Law-Compliance-Tests-v1
  - PILLAR: Calendar | STATUS: TOUCHED | NOTES: This is the missing workflow-driven calendar engine that sits between the completed storage substrate and downstream consumers. | STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: No editor surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: No document-editing surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: No spreadsheet surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: Locus may later consume sync-backed calendar facts, but this packet does not implement a Locus-specific surface. | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: No Loom-specific capability is in scope. | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: No product work-packet surface is implemented here. | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: No product task-board surface is implemented here. | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: The packet uses workflow/runtime infrastructure, not the micro-task feature surface. | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: `calendar_sync` should be visible through existing job/workflow inspection surfaces rather than hidden helpers, even though no new Command Center UI is introduced. | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: This packet is primarily a runtime wiring packet: engine registration, workflow dispatch, capability gating, and evidence. | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: No spec-to-prompt surface is implemented here. | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: The engine must reuse the portable calendar storage substrate and keep behavior consistent across both backends. | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: The sync engine populates the canonical calendar source/event substrate that later retrieval, routing, and projection consumers depend on. | STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: No Stage surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: No Studio surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: Lens remains a downstream consumer packet rather than part of the engine-runtime work here. | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: No distillation surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: CalendarScopeHint and policy routing remain downstream of this engine packet even though the engine must preserve the data those flows consume. | STUB_WP_IDS: WP-1-Calendar-Policy-Integration-v1
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: No retrieval pipeline is implemented here. | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: NEEDS_STUBS

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
  - PILLAR: Calendar | CAPABILITY_SLICE: governed sync-engine registration and storage-backed execution | SUBFEATURES: engine registry row, runtime adapter install, workflow dispatch contract, idempotent source/event upserts | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation, PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the direct activation target and the missing bridge between current spec law and current product code.
  - PILLAR: Calendar | CAPABILITY_SLICE: user-facing Lens projection and filters | SUBFEATURES: agenda/timeline rendering, diagnostics display, user controls | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Lens-v3 | NOTES: Lens remains the downstream UI consumer packet that will reuse the synced storage truth.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: auditable sync and mutation evidence | SUBFEATURES: result artifact, trace linkage, provider-safe diagnostics, retry visibility | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation, PRIM-CalendarSourceSyncState | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The engine must be inspectable through existing runtime evidence surfaces rather than helper-local logs.
  - PILLAR: Command Center | CAPABILITY_SLICE: inspectable workflow/job execution for calendar sync | SUBFEATURES: job visibility, workflow-run status, operator-visible failure posture | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Existing runtime inspection surfaces should expose the sync job without requiring a new packet-owned UI.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: capability-gated external calendar access | SUBFEATURES: consent posture, read-only source enforcement, fail-closed write-back behavior | PRIMITIVES_FEATURES: PRIM-CalendarMutation, PRIM-CalendarSourceWritePolicy, PRIM-CalendarEventExportMode | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Even if v1 chooses an import-only MVP, it must still enforce write posture correctly.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: portable sync behavior across both supported backends | SUBFEATURES: storage reuse, sync-state persistence, repeat-run consistency, backend parity | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The engine must reuse the portable storage substrate instead of introducing backend-specific sync behavior.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: canonical synced calendar substrate for downstream routing and projection | SUBFEATURES: durable source sync posture, stable event identity, queryable event windows | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Policy-Integration-v1 | NOTES: The engine packet should produce the canonical data shape that downstream routing and projection packets consume.
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
  - Capability: calendar source sync execution | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_sync_result | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: The packet must make the sync run as a governed workflow job rather than a helper thread.
  - Capability: provider adapter calls inside governed runtime | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: MEX_RUNTIME | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: tool_call/tool_result plus diagnostics | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Adapter work belongs inside the existing MEX gate stack and must remain capability-scoped.
  - Capability: calendar mutation apply discipline | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_mutation | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Successful patch application must remain trace-linked and workflow-governed.
  - Capability: calendar scope-hint and policy projection | JobModel: AI_JOB | Workflow: calendar_policy_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: ContextSnapshot routing fields | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Policy-Integration-v1 | Notes: This packet prepares the sync truth those routing flows need, but does not implement them.
  - Capability: calendar law-compliance validation suite | JobModel: VALIDATOR | Workflow: calendar_law_compliance | ToolSurface: CI | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Law-Compliance-Tests-v1 | Notes: Dedicated law-compliance assertions remain a separate blocked packet.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: NEEDS_STUBS

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - The highest-ROI combination for this packet is not a new primitive family; it is the missing runtime bridge between already-existing calendar storage primitives and already-existing workflow/MEX runtime infrastructure.
  - The current interaction matrix already carries the important calendar combinations this packet realizes in code: Calendar -> Storage Portability, Calendar -> Capabilities/Consent, Calendar -> AI Job Model, and Calendar -> Spec Router.
  - The sync packet should not create a second path for provider access or mutation. The whole point of the packet is to make the existing cross-feature contract executable through one governed runtime path.
  - No local/cloud prompt-routing matrix expansion is required in this packet because provider text remains a stored/runtime concern here, not a prompt-compilation feature.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- PRIMITIVE_MATRIX_CANDIDATE_EDGE_COUNT: 0
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: The current spec already captures the cross-feature edges this packet needs. The work is product realization of those existing combos, not discovery of a missing matrix edge.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This packet is bounded to internal calendar/runtime wiring under the current spec and local codebase. External combo research would not improve the decision about how Handshake must route `calendar_sync`.
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
  - Combo: existing calendar storage plus governed sync-engine registration removes the shadow-pipeline gap | Pillars: Calendar, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSyncInput, PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | Resolution: IN_THIS_WP | Stub: NONE | Notes: This is the direct blocker-removal combination and the core purpose of the packet.
  - Combo: provider-safe result evidence keeps sync inspectable in existing workflow consoles | Pillars: Flight Recorder, Command Center | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSyncInput, PRIM-CalendarMutation | Resolution: IN_THIS_WP | Stub: NONE | Notes: Sync failures and successes must be operator-visible through existing runtime evidence surfaces.
  - Combo: portable storage reuse keeps SQLite and Postgres sync behavior aligned | Pillars: SQL to PostgreSQL shift readiness, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | Resolution: IN_THIS_WP | Stub: NONE | Notes: The engine must not introduce backend-specific sync semantics.
  - Combo: idempotent synced rows become the canonical calendar substrate for model-safe downstream consumers | Pillars: LLM-friendly data, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | Resolution: IN_THIS_WP | Stub: NONE | Notes: Canonical event identity and sync posture belong in durable storage, not ephemeral helper caches.
  - Combo: read-only/write-policy enforcement keeps remote provider access fail-closed | Pillars: Calendar, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarMutation, PRIM-CalendarSourceWritePolicy, PRIM-CalendarEventExportMode | Resolution: IN_THIS_WP | Stub: NONE | Notes: Even an import-first MVP must still prove that remote writes cannot slip through.
  - Combo: Lens projections consume the same synced storage truth rather than bespoke provider fetches | Pillars: Calendar, LLM-friendly data, Atelier/Lens | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Lens-v3 | Notes: Lens remains separate, but this packet removes its biggest upstream runtime blocker.
  - Combo: policy and routing consumers read sync-backed calendar posture through a dedicated integration packet | Pillars: LLM-friendly data, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Policy-Integration-v1 | Notes: This packet should produce the routing inputs without also owning the routing layer.
  - Combo: law-compliance validation can finally exercise a real governed sync path | Pillars: Flight Recorder, Calendar | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarMutation, PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Law-Compliance-Tests-v1 | Notes: Validator-heavy law-compliance coverage remains blocked until the engine path exists.
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_REASON: The highest-ROI runtime combinations belong in this packet, while the user-facing and validator-heavy consumers already have separate stub homes and should stay there.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- SCAN_SCOPE: current calendar stub backlog, current Master Spec v02.181, completed dependency packets, and product code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Calendar-Sync-Engine-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this is the correct activation shell, but its original stub text underspecifies how much existing runtime/storage infrastructure already exists
  - Artifact: WP-1-Calendar-Lens-v3 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: PARTIAL | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Lens is the downstream projection/UI consumer packet and should not be folded into this engine packet
  - Artifact: WP-1-Calendar-Policy-Integration-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: ACE/routing and capability-policy projection remain a separate packet
  - Artifact: WP-1-Calendar-Law-Compliance-Tests-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: validator-heavy law-compliance coverage remains blocked by this packet
  - Artifact: WP-1-Calendar-Correlation-Export-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: correlation/export remains downstream of the engine and storage substrate
  - Artifact: WP-1-Calendar-Mailbox-Correlation-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: mailbox/calendar linking is downstream and should not widen this packet
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Calendar-Storage-v2 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides the durable source/event substrate and sync-state row models that this packet must consume
  - Artifact: WP-1-Workflow-Engine-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: workflow persistence and job execution exist, but the calendar-specific execution branch is still absent
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs | Artifact: WP-1-Calendar-Storage-v2 | Covers: primitive | Verdict: IMPLEMENTED | Notes: durable calendar source/event/sync-state structs already exist and must be reused
  - Path: ../handshake_main/src/backend/handshake_core/mechanical_engines.json | Artifact: WP-1-Calendar-Sync-Engine-v1 | Covers: execution | Verdict: NOT_PRESENT | Notes: registry currently contains shell, media_downloader, and other engines, but not `calendar_sync`
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Calendar-Sync-Engine-v1 | Covers: execution | Verdict: PARTIAL | Notes: `build_mex_runtime(...)` installs shell and media-downloader adapters only, and job dispatch only contains media-downloader-specific branches
  - Path: ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | Artifact: WP-1-Calendar-Sync-Engine-v1 | Covers: execution | Verdict: PARTIAL | Notes: existing registry/runtime tests provide the extension pattern for a new engine, but there is no calendar-sync-specific coverage yet
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The sync packet should be activated as a runtime-bridge packet over existing storage/runtime foundations, not as a greenfield calendar rewrite. The stub intent was directionally correct but materially under-hydrated for current repo truth.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet implements engine/runtime plumbing only. It should surface through existing workflow/job inspection surfaces, but it does not add a new dedicated UI owned by this packet.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: NONE | Type: NONE | Tooltip: NONE | Notes: UI ownership lives in downstream packets such as Calendar Lens and broader Command Center work
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - N/A (no new direct UI surface in this packet)
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No new GUI is implemented here. Existing runtime/job inspection surfaces can consume the engine output without this packet taking on GUI ownership.
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
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.181.md \u00a72.6.6 surface mutation discipline + \u00a710.4.1 calendar_sync engine contract + \u00a76.0.1 Cross-Tool Interaction Map
- WHAT: Register and implement `calendar_sync` as a governed workflow-driven mechanical engine that reads CalendarSource providers, writes idempotent CalendarEvent updates into the existing storage substrate, and emits observable results through the current MEX/Workflow runtime.
- WHY: The spec already defines `calendar_sync` as the only legal path for external calendar mutation and provider sync, but the product currently stops at storage plus generic runtime foundations. Without this bridge, calendar sync remains a paper contract and the downstream law-compliance packet stays blocked.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
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
  - A truthful MVP sync path reuses existing calendar storage upserts and updates `CalendarSourceSyncState` without duplicating events across repeat runs.
  - Capability posture, read-only/write-policy behavior, and sync/mutation evidence are fail-closed and trace-linked through existing runtime diagnostics and Flight Recorder surfaces.
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
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- SEARCH_TERMS:
  - calendar_sync
  - with_adapter
  - mechanical_engines.json
  - CalendarSourceSyncState
  - upsert_calendar_source
  - upsert_calendar_event
  - calendar_mutation
  - capability_profile_id
- RUN_COMMANDS:
  ```bash
  rg -n "calendar_sync|with_adapter|CalendarSourceSyncState|upsert_calendar_source|upsert_calendar_event|calendar_mutation|capability_profile_id" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "calendar_sync is added as a one-off helper instead of a MEX/workflow contract" -> "capability gates, provenance, and replay guarantees drift immediately"
  - "engine writes ignore existing sync-state and storage upsert contracts" -> "retries/backoff/recovery become lossy and duplicate events appear"
  - "read-only or import-only sources can still push remote mutations" -> "consent and capability law is violated"
  - "tests cover registry happy-path only and skip dispatch/runtime execution" -> "the packet looks registered while real workflow execution still fails"
  - "sync result evidence omits job/workflow linkage" -> "law-compliance and operator inspection cannot prove what happened"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - BUILD_ORDER.md already lists the correct dependency/blocker topology for this packet and its downstream law-compliance blocker.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSE_ROWS:
  - CLAUSE: surface mutation discipline plus write gate | WHY_IN_SCOPE: the packet must make `calendar_sync` the real workflow-only mutation path instead of a paper contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: direct helper or UI-side writes can still bypass governed execution
  - CLAUSE: Cross-Tool Interaction Map no-shadow-pipeline rule | WHY_IN_SCOPE: provider sync must run through Workflow Engine + MEX runtime, not hidden background helpers | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests | RISK_IF_MISSED: provider access happens outside the contract the spec requires
  - CLAUSE: `calendar_sync` engine contract and output | WHY_IN_SCOPE: the packet exists to realize the engine input/behavior/output contract already named in the spec | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | RISK_IF_MISSED: the engine may exist nominally but still fail to honor spec-defined behavior
  - CLAUSE: `CalendarSourceSyncState` as single source of truth for recovery | WHY_IN_SCOPE: retries, backoff, and recovery are core parts of a sync engine, not optional extras | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: the engine becomes non-recoverable or duplicates data under retry
  - CLAUSE: MCP/provider adapter guidance plus read-only mode | WHY_IN_SCOPE: the spec explicitly prefers provider access through tools inside the engine and names read-only behavior as a first-class posture | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: provider access and write posture drift from the calendar law contract

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CONTRACT_ROWS:
  - CONTRACT: `calendar_sync` engine registry contract | PRODUCER: mechanical_engines.json | CONSUMER: mex/registry.rs, workflows.rs, tests/mex_tests.rs | SERIALIZER_TRANSPORT: JSON engine registry | VALIDATOR_READER: mex_tests.rs | TRIPWIRE_TESTS: mex registry/runtime tests | DRIFT_RISK: engine is declared but not executable, or executable but not declared consistently
  - CONTRACT: calendar sync job input / protocol contract | PRODUCER: workflows.rs job/profile parser and engine runner | CONSUMER: calendar-sync adapter implementation, storage layer, validators | SERIALIZER_TRANSPORT: workflow payload plus PlannedOperation inputs | VALIDATOR_READER: workflow/job tests plus validator inspection | TRIPWIRE_TESTS: targeted calendar-sync execution tests plus full cargo test | DRIFT_RISK: job payload shape and adapter expectations silently diverge
  - CONTRACT: `CalendarSourceSyncState` durable recovery contract | PRODUCER: storage/calendar.rs plus engine runner | CONSUMER: sqlite.rs, postgres.rs, later recovery/retry flows | SERIALIZER_TRANSPORT: sqlx row mapping and JSON-ish sync-state payloads | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: calendar storage tests plus sync retry/idempotency tests | DRIFT_RISK: sync token/backoff/watermark state is lost or inconsistently updated
  - CONTRACT: calendar event upsert/idempotency contract | PRODUCER: engine runner and adapter | CONSUMER: storage backends and later Lens/policy consumers | SERIALIZER_TRANSPORT: storage upsert calls keyed by source/external identity | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: calendar storage tests plus repeat-sync tests | DRIFT_RISK: repeated sync runs duplicate events or destabilize identity

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - targeted validator review of engine registration, runtime adapter installation, gated sync execution, and sync-state durability
- CANONICAL_CONTRACT_EXAMPLES:
  - a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes
  - a repeated identical sync run that keeps stable identity and produces no duplicate events
  - a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Audit the existing storage/runtime seams first so the packet reuses completed work instead of recreating it.
  - Add `calendar_sync` to the mechanical engine registry and wire its adapter into `build_mex_runtime(...)` before writing provider-specific logic.
  - Implement the smallest truthful sync path that satisfies the signed packet scope, reusing existing calendar storage upserts and sync-state contracts.
  - Extend registry/runtime/storage tests to prove engine registration, governed execution, idempotent sync behavior, and fail-closed capability posture.
  - Re-run the proof commands from the product worktree until they pass cleanly.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
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
  - Do not widen v1 into Lens, ACE policy integration, multi-provider breadth, or rich write-back UX.
  - Do not mint new PRIM IDs or new top-level Flight Recorder schemas to paper over runtime gaps.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - engine registration and runtime adapter installation
  - workflow dispatch and governed execution path
  - capability posture plus read-only/write-policy fail-closed behavior
  - sync-state durability and repeat-run idempotency
  - trace/result evidence linkage
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
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
  - verify there is still no direct provider-write path that bypasses capability gates

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- NOT_PROVEN_ITEMS:
  - This refinement does not prove whether the smallest truthful MVP should be local ICS import only or one external provider in read-only mode; both are spec-compatible and must be resolved against code reality during implementation.
  - This refinement does not freeze the exact protocol_id/schema naming for the calendar-sync job contract; coder and validators must align those names to current workflow conventions.
  - This refinement does not prove bidirectional write-back, conflict resolution, CalendarScopeHint policy projection, Lens UX, or downstream correlation behavior.

### DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (The current appendix already contains the calendar sync, mutation, and storage primitive IDs this packet needs.)
- DISCOVERY_STUBS: NONE_CREATED (All relevant downstream consumer and validator stubs already existed before this refinement pass.)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (The interaction matrix already captures the important Calendar cross-feature edges this packet realizes.)
- DISCOVERY_UI_CONTROLS: NONE_APPLICABLE (This packet does not own a new GUI surface.)
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED (The current spec already provides the normative engine/runtime contract required to activate this WP.)
- DISCOVERY_JUSTIFICATION: This refinement still delivered high value by replacing a thin stub story with current product/runtime truth, proving that the missing gap is a specific `calendar_sync` runtime bridge over already-completed storage and MEX/workflow foundations.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Master Spec v02.181 explicitly names the workflow-only mutation discipline, `calendar_sync` as the engine for external calendar mutation/sync, the engine's input/output behavior, durable sync-state requirements, provider adapter strategy, read-only posture, and observability expectations. The packet gap is runtime realization in product code, not missing spec intent.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE. The remaining uncertainty is implementation-choice uncertainty (which MVP adapter path is smallest and truthful), not ambiguity in the governing spec clauses.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The current Master Spec already provides a sufficiently specific calendar sync law and runtime contract. This activation pass should produce product/runtime wiring work, not new normative spec text.

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
