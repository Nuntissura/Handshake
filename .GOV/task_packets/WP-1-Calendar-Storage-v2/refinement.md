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
- WP_ID: WP-1-Calendar-Storage-v2
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-13T00:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- SPEC_TARGET_SHA1: 7d6558fab2f3df70669fff6f0a6e6ef9ea395194
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja130420261117
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Calendar-Storage-v2
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- The v1 implementation is not greenfield anymore: `../handshake_main/src/backend/handshake_core/src/storage/calendar.rs`, `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`, `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`, and `../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql` already exist and must be treated as the starting point for v2.
- The existing schema is only partial against the current v02.180 contract. It persists calendar sources/events and passes a conformance suite, but it still stores attendees and links as JSON columns, has no dedicated `CalendarEventOverride` table, and does not expose a `parse_status` / `source_payload` contract matching the current main-body wording.
- The existing `Database` boundary still exposes calendar methods directly on the monolithic trait. The v2 packet must reconcile that reality with the later storage capability-boundary guidance rather than pretending the methods do not exist.
- The v1 packet was implemented but never validated under the current governed workflow. The highest-value v2 posture is a truthful spec-alignment and validation pass over the existing code, not a fresh greenfield storage build.
- Downstream pillar consumers remain stub-backed: Lens, sync orchestration, policy integration, law-compliance tests, correlation export, and mailbox correlation all depend on this packet's storage/query substrate but are not implemented here.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 45m
- SEARCH_SCOPE: current Master Spec v02.180 calendar/storage clauses, existing v1 packet/refinement, and the live product code under `../handshake_main/src/backend/handshake_core`.
- REFERENCES:
  - .GOV/spec/Handshake_Master_Spec_v02.180.md sections 2.0.4, 2.0.5, 2.1, 2.1.1, 2.1.2, 2.3, 2.3.13.1, 10.4, 10.4.2.1, 11.9.3, and Appendix 12.4.
  - .GOV/task_packets/WP-1-Calendar-Storage-v1.md
  - .GOV/refinements/WP-1-Calendar-Storage-v1.md
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
- PATTERNS_EXTRACTED:
  - Reuse the existing dual-backend calendar storage foundation instead of discarding it; v2 should align and extend the code that already landed.
  - Keep all writes behind storage abstractions and portable migrations; do not regress to runtime DDL or caller-side SQL.
  - Preserve the current dual-backend conformance posture and expand it around the stricter v02.180 invariants.
  - Treat calendar as a backend substrate by preserving stable IDs, time-window queries, provider provenance, and queryable source sync state, while leaving UI/workflow consumers to downstream packets.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the existing v1 migration plus storage/test footprint as the implementation baseline; ADAPT the current code to the later spec/refinement workflow, calendar invariant wording, and storage-boundary expectations; REJECT false greenfield assumptions, deleted-branch archaeology as authority, and speculative new primitives that are not yet present in Appendix 12.4.
- LICENSE/IP_NOTES: Local repository patterns only. No external code reuse is proposed.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The current Master Spec already names the calendar raw entities, temporal and recurrence invariants, storage/query expectations, and portability/testing rules with enough specificity to activate a packet that aligns existing code to that contract. Any future doc-only cleanup of the illustrative DDL block can be handled separately without blocking this implementation pass.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This is a product-internal storage-alignment and governed validation packet grounded in the current Master Spec plus local product code truth under `../handshake_main`. No external source changes the storage-law or governance decision here.
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
- This packet does not introduce a new Flight Recorder event type. Its obligation is to preserve and harden the storage/query substrate that later packets will use when they emit `calendar_mutation`, sync, and correlation events.
- The storage layer must continue to preserve write provenance through the existing write-context columns (`last_job_id`, `last_workflow_id`, `last_actor_id`, `edit_event_id`, `last_actor_kind`) so later workflow-level emitters can deep-link calendar writes back to governed execution.
- Time-window overlap queries remain a hard downstream dependency for the `CalendarEvent` / `ActivitySpan` join semantics in 11.9.3. This packet owns the query substrate, not the correlation/export workflow.

### RED_TEAM_ADVISORY (security failure modes)
- Privacy leak risk: calendar content is high-sensitivity. Titles, descriptions, attendees, links, and provider payloads must not leak into logs or debug output outside governed paths.
- Write-bypass risk: any attempt to route calendar writes around governed workflows or capability checks would violate the write gate. Storage work must preserve workflow-linked provenance rather than creating a shortcut.
- Portability drift risk: the existing migration must remain DB-agnostic and any v2 changes must keep SQLite and Postgres in lockstep.
- Identity drift risk: `(source_id, external_id)` dedupe semantics and `instance_key` stability remain essential to avoid duplicate or broken recurring events.
- Semantic drift risk: JSON storage for attendees/links/provider payload can silently diverge from main-body wording if row mappers, migrations, and tests do not move together.
- Validation debt risk: v1 code exists, but without governed validation it remains too easy to overclaim completeness. v2 must close that gap explicitly.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-Database
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
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
  - PRIM-CalendarSource
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarEvent
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- NOTES:
  - The spec already contains the calendar primitive set needed for this packet. V2 should align existing code to those IDs rather than inventing unsanctioned new primitives in refinement.
  - Missing storage concepts such as a dedicated override row model or explicit parse-status payload contract remain implementation-shape gaps inside this packet, not proof that new appendix primitives must be invented before activation.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already contains the calendar primitive IDs this packet uses. The implementation gap is product-code alignment, not missing appendix identity.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: This refinement did not discover a new primitive that is absent from Appendix 12.4; it discovered that the existing code only partially realizes the already-registered calendar primitive surface.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: FEAT-CALENDAR and the required primitive rows already exist in the current spec.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: Storage-only packet. User-facing calendar controls remain downstream of Lens and policy packets.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Appendix 12.6 already captures the important Calendar interaction edges; this packet is product realization of those existing edges, not a new matrix-authoring pass.
- APPENDIX_MAINTENANCE_NOTES:
  - NONE
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: No spatial dimension in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: No tool fabrication in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: No physics simulation in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: No simulation in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: No hardware interaction in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: No orchestration in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: No composition in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: No visual generation in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: No publishing in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: No recipe management in calendar storage. Calendar time-window queries will later support Sous Chef meal-planning correlation but that is downstream. | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: No food safety in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: No logistics in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: No archival in calendar storage. Calendar storage preserves raw payloads per never-lose-data rule but archival workflows are downstream. | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: No library indexing in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: No analytics in calendar storage. Time-window queries support later per-block analytics but that is downstream. | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: No data wrangling in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: Calendar migrations add new tables to the portable schema managed by the DBA engine's migration framework. Migration replay safety and dual-backend conformance are DBA-engine concerns. | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: No governance rule changes in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: No onboarding in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: NOT_TOUCHED | NOTES: CalendarScopeHint is a downstream ACE Context concern (WP-1-Calendar-Policy-Integration). Storage provides the queryable time-window data that context compilation later consumes. | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: No versioning engine interaction in calendar storage. | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: No sandbox interaction in calendar storage. | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Storage must preserve job/workflow-linked provenance and time-window query semantics that later FR emitters and correlation workflows depend on. | STUB_WP_IDS: WP-1-Calendar-Correlation-Export-v1
  - PILLAR: Calendar | STATUS: TOUCHED | NOTES: This is the calendar storage substrate packet and the direct blocker for the rest of the calendar family. | STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: No editor surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: No document-editing surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: No spreadsheet surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: Locus consumes calendar windows later, but this packet does not implement Locus-specific logic. | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Loom correlations remain downstream even though calendar links can eventually target Loom entities. | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: No product work-packet feature work in scope. | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: No task-board product surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: No micro-task surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: Command Center views consume calendar data later, but no DCC surface is implemented here. | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: Existing write-context columns and governed mutation provenance tie calendar storage to workflow/job runtime even though the sync workflow itself remains downstream. | STUB_WP_IDS: WP-1-Calendar-Sync-Engine-v1
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: No spec-to-prompt surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: Calendar storage must remain portable and validated on SQLite and Postgres from day one. | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: Stable IDs, window queries, source sync state, and governed provenance are the backend substrate later retrieval, scope-hint, and summarization flows need. | STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: No Stage surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: No Studio surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: Lens remains a downstream consumer even though this packet provides its data substrate. | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: No distillation surface in scope. | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: Policy integration consumes this substrate later, but no ACE packet work is implemented here. | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: Retrieval consumers remain downstream even though this packet prepares the canonical source/event data they will read. | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: NEEDS_STUBS

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
  - PILLAR: Calendar | CAPABILITY_SLICE: durable source and event storage | SUBFEATURES: source CRUD, event upsert, source-scoped cleanup | PRIMITIVES_FEATURES: PRIM-CalendarSource, PRIM-CalendarSourceUpsert, PRIM-CalendarEvent, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the direct storage baseline already present in v1 code and still owned by v2.
  - PILLAR: Calendar | CAPABILITY_SLICE: deterministic time-window query substrate | SUBFEATURES: overlap queries, source filtering, canonical window shape | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Downstream correlation and Lens consumers depend on this query surface.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: provenance-ready calendar rows | SUBFEATURES: job/workflow-linked write context, stable mutation back-links | PRIMITIVES_FEATURES: PRIM-CalendarEvent, PRIM-CalendarSource | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet preserves storage truth so later FR emitters can link calendar writes to governed execution.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: governed mutation persistence | SUBFEATURES: write-context columns, source sync-state durability, workflow-facing row updates | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarSyncStateStage, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The runtime workflow remains downstream, but the durable storage substrate is in scope here.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: portable calendar migrations and tests | SUBFEATURES: DB-agnostic DDL, replay safety, dual-backend conformance | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the main portability obligation of the packet.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: canonical calendar retrieval substrate | SUBFEATURES: stable IDs, provenance-preserving rows, queryable source sync state | PRIMITIVES_FEATURES: PRIM-CalendarSource, PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet prepares the backend truth later retrieval/scope-hint layers consume.
  - PILLAR: Calendar | CAPABILITY_SLICE: sync orchestration and provider adapters | SUBFEATURES: pull/push workflows, conflict resolution, mutation workflows | PRIMITIVES_FEATURES: PRIM-CalendarMutation, PRIM-CalendarSyncInput, PRIM-CalendarSourceSyncState | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Sync-Engine-v1 | NOTES: Existing storage rows unblock this work, but the workflow and adapter layer is out of scope here.
  - PILLAR: Calendar | CAPABILITY_SLICE: calendar lens and projection consumers | SUBFEATURES: agenda/timeline views, user-facing filters, drill-down queries | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Lens-v3 | NOTES: Lens remains a separate user-facing packet.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: calendar scope-hint and policy compilation | SUBFEATURES: time-range selection, source trust posture, policy-profile attachment | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Policy-Integration-v1 | NOTES: This packet only prepares the rows/query surface that policy compilation will later consume.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: calendar-to-activity correlation and export | SUBFEATURES: overlap joins, debug/export bundles, activity annotation | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Correlation-Export-v1 | NOTES: Storage/query substrate is in scope here; correlation/export workflow is not.
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
  - Capability: calendar source and event persistence | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: This packet owns the storage abstraction, not a runtime-invocable workflow.
  - Capability: calendar write-context and provenance durability | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Existing row metadata already persists job/workflow/actor provenance and must remain aligned across both backends.
  - Capability: calendar time-window query substrate | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Query substrate only; downstream UI/workflow surfaces remain separate.
  - Capability: calendar sync workflow orchestration | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_mutation, calendar_sync_result | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Sync-Engine-v1 | Notes: This packet prepares the storage layer that sync orchestration will consume.
  - Capability: calendar scope-hint and policy projection | JobModel: WORKFLOW | Workflow: calendar_policy_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Policy-Integration-v1 | Notes: Scope-hint/policy projection is downstream of the storage substrate.
  - Capability: calendar lens query consumption | JobModel: UI_ACTION | Workflow: calendar_lens_query | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Lens-v3 | Notes: User-facing calendar views remain downstream.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: NEEDS_STUBS

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - v1 refinement found NONE_FOUND for the primitive matrix. v2 performs deeper cross-cutting substrate analysis per the stub's requirement for calendar-as-substrate design.
  - Key discovery: calendar storage is not an isolated domain -- it is a graph node factory. CalendarEvent.links[] creates edges to docs, tasks, mail threads, and Loom blocks. CalendarEvent.attendees[] creates edges to participant entities. These graph edges are what make calendar a cross-cutting substrate rather than a siloed persistence layer.
  - The interaction matrix already captures 10 FEAT-CALENDAR edges covering: Flight Recorder (IMX-010), Locus (IMX-011), DCC, Mailbox, Project Brain, Debug Bundle, Storage Portability, Capabilities/Consent, AI Job Model, and Spec Router. All of these depend on queryable, graph-connected calendar storage.
  - No local/cloud model compatibility changes are needed in this WP because it is storage-only.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- PRIMITIVE_MATRIX_CANDIDATE_EDGE_COUNT: 1
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - Edge: FEAT-CALENDAR -> FEAT-LOOM
  - Kind: calendar_event_links_loom_blocks
  - ROI: MEDIUM
  - Effort: LOW
  - Spec refs: \u00a72.1 line 55947 (links[] EntityLinkRef -> doc, canvas, task, mail_thread)
  - In-scope for this WP: NO
  - If NO: existing WP-1-Calendar-Correlation-Export-v1 covers correlation; the storage junction table is IN_THIS_WP but the matrix edge addition is a spec update that should accompany the correlation WP.
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: The 10 existing interaction matrix edges already capture the calendar substrate surface. Storage junction tables (attendees, links) implement the graph connectivity that these edges require. The candidate FEAT-CALENDAR -> FEAT-LOOM edge is informational and tracked for the correlation WP but does not require a new matrix entry before this WP can proceed.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This is internal product storage work grounded in the current Master Spec. The calendar law (\u00a710.4.1) already incorporates calendar research v0.4 with temporal/recurrence/sync semantics. No external combo research improves the storage DDL design.
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
  - Combo: portable calendar schema plus dual-backend conformance stays the root blocker remover | Pillars: Calendar, SQL to PostgreSQL shift readiness | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSource, PRIM-CalendarEvent | Resolution: IN_THIS_WP | Stub: NONE | Notes: This is the direct v2 activation target and must land inside the packet.
  - Combo: governed write-context columns preserve Flight Recorder back-linkability | Pillars: Calendar, Flight Recorder, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarEvent, PRIM-CalendarSource | Resolution: IN_THIS_WP | Stub: NONE | Notes: Existing write metadata keeps later mutation traces explainable without adding a new event type here.
  - Combo: source sync-state durability unblocks workflow-driven recovery and retries | Pillars: Calendar, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSourceSyncState, PRIM-CalendarSyncStateStage | Resolution: IN_THIS_WP | Stub: NONE | Notes: Storage truth for sync state belongs in this packet even though orchestration remains downstream.
  - Combo: canonical time-window query shape becomes the shared substrate for later consumers | Pillars: Calendar, LLM-friendly data | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarEventWindowQuery | Resolution: IN_THIS_WP | Stub: NONE | Notes: This packet owns the durable query contract later Lens, policy, and correlation packets consume.
  - Combo: calendar sync orchestration consumes the same storage truth without adding shadow schemas | Pillars: Calendar, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarMutation, PRIM-CalendarSyncInput, PRIM-CalendarSourceSyncState | Resolution: NEW_STUB | Stub: WP-1-Calendar-Sync-Engine-v1 | Notes: The sync workflow remains a downstream packet that should reuse this storage surface rather than fork it.
  - Combo: calendar lens uses the same time-window/query contract instead of bespoke fetch rules | Pillars: Calendar, LLM-friendly data, Atelier/Lens | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Lens-v3 | Notes: Lens is a consumer packet, not part of this storage packet.
  - Combo: scope-hint and policy projection read governed calendar substrate | Pillars: Calendar, LLM-friendly data, ACE | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Policy-Integration-v1 | Notes: Policy compilation should build on the same storage truth, not invent a side channel.
  - Combo: activity correlation and export reuse the time-window substrate | Pillars: Calendar, Flight Recorder, Locus | Mechanical: engine.dba | Primitives/Features: PRIM-CalendarEventWindowQuery | Resolution: NEW_STUB | Stub: WP-1-Calendar-Correlation-Export-v1 | Notes: Correlation/export remains downstream even though this packet owns the join-ready query surface.
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_REASON: The highest-ROI storage and portability combinations are in scope here, while the workflow, Lens, policy, and correlation consumers remain intentionally separate stub-backed packets.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- SCAN_SCOPE: current calendar stub backlog, v1 packet/refinement, current Master Spec v02.180, and product code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Calendar-Storage-v2 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this is the correct activation shell for the remaining spec-alignment and validation work
  - Artifact: WP-1-Calendar-Lens-v3 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Lens remains the user-facing consumer packet and should not be folded into storage
  - Artifact: WP-1-Calendar-Sync-Engine-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Behavioral sync logic, depends on this WP's storage and sync state columns.
  - Artifact: WP-1-Calendar-Policy-Integration-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Capability/consent gating, depends on this WP's capability_profile_id and export_mode storage.
  - Artifact: WP-1-Calendar-Law-Compliance-Tests-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: MISSING | Matrix: N/A | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Compliance test suite covers law invariants; this WP provides the storage-level dual-backend tests.
  - Artifact: WP-1-Calendar-Correlation-Export-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: MISSING | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: ActivitySpan join and correlation export; depends on this WP's time-window queries.
  - Artifact: WP-1-Calendar-Mailbox-Correlation-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: MISSING | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Mailbox-calendar link correlation; depends on this WP's EntityLinkRef junction table.
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Calendar-Storage-v1 | BoardStatus: SUPERSEDED | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: v1 landed real code and tests, but it never completed governed validation and now needs a current-spec alignment pass rather than reuse-as-is
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql | Artifact: WP-1-Calendar-Storage-v1 | Covers: primitive | Verdict: IMPLEMENTED | Notes: portable calendar tables and indexes already exist, proving the packet is not greenfield
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs | Artifact: WP-1-Calendar-Storage-v1 | Covers: primitive | Verdict: IMPLEMENTED | Notes: calendar source/event structs, upsert shapes, enums, and window-query model already exist, but row shape still needs alignment against the current packet's stricter governed scope
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Calendar-Storage-v1 | Covers: primitive | Verdict: IMPLEMENTED | Notes: the monolithic Database trait already exposes calendar methods; v2 must decide whether to preserve or refactor that boundary under current storage guidance
  - Path: ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs | Artifact: WP-1-Calendar-Storage-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: sqlite/postgres calendar storage conformance tests already exist, confirming that v1 shipped code even though governed validation never closed
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The same-intent packet already shipped real code, so v2 must expand and validate that implementation against current spec/governance instead of pretending the capability does not exist or reusing it unchanged.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet is storage-layer only. User-visible surfaces belong to WP-1-Calendar-Lens-v3. No UI code is in scope.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: NONE | Type: NONE | Tooltip: NONE | Notes: Storage-only packet
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - N/A (storage-only)
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: Storage-only WP with no GUI surface. GUI advice belongs to WP-1-Calendar-Lens-v3.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.181]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Migration-Framework, WP-1-Storage-Abstraction-Layer, WP-1-Storage-Trait-Purity, WP-1-Storage-Capability-Boundary-Refactor
- BUILD_ORDER_BLOCKS: WP-1-Calendar-Lens, WP-1-Calendar-Sync-Engine, WP-1-Calendar-Policy-Integration, WP-1-Calendar-Law-Compliance-Tests, WP-1-Calendar-Correlation-Export, WP-1-Calendar-Mailbox-Correlation
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.180.md \u00a710.4 Calendar + \u00a72.3 Storage and indexing + \u00a72.3.13.1 Portability Pillars
- WHAT: Realign and validate the already-landed calendar storage implementation in `../handshake_main` against the current v02.180 calendar/storage contract, keeping portable SQLite/Postgres behavior while closing the governed validation gap left by v1.
- WHY: Calendar storage is the highest-value backend blocker in BUILD_ORDER. The code exists, but until it is truthfully aligned to the current spec/governance workflow and validated, every downstream calendar packet remains blocked on unstable substrate truth.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.down.sql
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- OUT_OF_SCOPE:
  - Calendar Lens UI and user-facing projection work
  - Sync-engine workflow orchestration and provider adapters
  - Capability/consent policy enforcement flows
  - Calendar correlation/export workflows
  - Mailbox-correlation product logic
  - New repo-governance tooling or protocol changes unrelated to this packet
- TEST_PLAN:
  ```bash
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml run_calendar_storage_conformance -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
  ```
- DONE_MEANS:
  - The calendar storage code under `../handshake_main` is truthfully aligned to the current v02.180 packet scope rather than the stale v1 assumptions.
  - SQLite and Postgres calendar storage tests pass from the product worktree under the packet's chosen test plan.
  - The storage boundary, migrations, row models, and conformance tests agree on the same governed calendar source/event contract.
  - WP Validator and Integration Validator pass, and the validated code is integrated into `main`.
- PRIMITIVES_EXPOSED:
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
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
- SEARCH_TERMS:
  - upsert_calendar_source
  - upsert_calendar_event
  - query_calendar_events
  - CalendarSourceSyncState
  - CalendarEventWindowQuery
  - provider_payload_json
  - attendees_json
  - links_json
- RUN_COMMANDS:
  ```bash
  rg -n "upsert_calendar_source|upsert_calendar_event|query_calendar_events|CalendarSourceSyncState|CalendarEventWindowQuery|provider_payload_json|attendees_json|links_json" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
  ```
- RISK_MAP:
  - "v2 keeps pretending the packet is greenfield" -> "coder and validator act on false premises and miss real code-regression risk"
  - "schema and row-model drift remain hidden behind passing legacy tests" -> "calendar substrate looks done while current spec semantics still diverge"
  - "trait-boundary expectations stay unresolved" -> "future storage packets keep duplicating or bypassing calendar access patterns"
  - "dual-backend coverage regresses during v2 alignment" -> "Postgres readiness erodes while SQLite still passes"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - BUILD_ORDER.md already lists WP-1-Calendar-Storage with the correct dependency/blocker topology. No stubs, dependencies, or execution lane changes from this refinement.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSE_ROWS:
  - CLAUSE: [HSK-CAL-WRITE-GATE] mutation governance | WHY_IN_SCOPE: storage must preserve governed write-context truth and must not create an easier bypass path while v2 realigns existing code | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | RISK_IF_MISSED: calendar writes remain technically possible but governance truth becomes harder to prove
  - CLAUSE: temporal invariants (2.1.1) | WHY_IN_SCOPE: v2 must prove the existing row models and migrations still preserve the required time fields and query semantics | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql; ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | RISK_IF_MISSED: silent time-window drift and user-visible corruption
  - CLAUSE: recurrence invariants (2.1.2) | WHY_IN_SCOPE: v2 must decide whether the current storage shape is sufficient or needs code changes to preserve recurrence semantics under the current packet scope | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | RISK_IF_MISSED: recurring-event behavior looks complete while drift stays latent
  - CLAUSE: portable schema and migrations [CX-DBP-011] plus dual-backend testing [CX-DBP-013] | WHY_IN_SCOPE: this is a backend blocker whose value comes from remaining portable and validated across SQLite and Postgres | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql; ../handshake_main/src/backend/handshake_core/src/storage/tests.rs; ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | RISK_IF_MISSED: storage appears finished while Postgres drift or migration debt survives
  - CLAUSE: CalendarEvent and ActivitySpan join semantics (11.9.3) | WHY_IN_SCOPE: the packet owns the overlap-query substrate that downstream correlation packets consume | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | RISK_IF_MISSED: later correlation/export packets inherit a broken substrate

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CONTRACT_ROWS:
  - CONTRACT: `CalendarSource` and `CalendarSourceUpsert` row contract | PRODUCER: storage/calendar.rs | CONSUMER: sqlite.rs, postgres.rs, storage/tests.rs | SERIALIZER_TRANSPORT: sqlx row mapping and JSON sync-state payload | VALIDATOR_READER: calendar_storage_tests.rs plus storage/tests.rs | TRIPWIRE_TESTS: calendar storage conformance suite | DRIFT_RISK: source sync-state and write-policy fields drift between structs and SQL
  - CONTRACT: `CalendarEvent` and `CalendarEventUpsert` row contract | PRODUCER: storage/calendar.rs | CONSUMER: sqlite.rs, postgres.rs, storage/tests.rs | SERIALIZER_TRANSPORT: sqlx row mapping and JSON attendees/links/provider payload columns | VALIDATOR_READER: calendar_storage_tests.rs plus storage/tests.rs | TRIPWIRE_TESTS: calendar storage conformance suite | DRIFT_RISK: migrations and row mappers silently disagree about recurrence, payload, or provenance fields
  - CONTRACT: `CalendarEventWindowQuery` overlap semantics | PRODUCER: storage/calendar.rs | CONSUMER: sqlite.rs, postgres.rs, downstream correlation, Lens, and policy packets | SERIALIZER_TRANSPORT: query inputs and SQL overlap filters | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: storage calendar query tests | DRIFT_RISK: later consumers assume half-open overlap semantics that the storage layer no longer actually enforces
  - CONTRACT: calendar migration 0015 <-> runtime structs/tests | PRODUCER: migrations/0015_calendar_storage.sql | CONSUMER: storage/calendar.rs; sqlite.rs; postgres.rs; tests | SERIALIZER_TRANSPORT: SQL DDL | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: migration plus calendar storage test plan | DRIFT_RISK: the migration keeps legacy shapes while structs/tests evolve separately

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
  - targeted validator review of calendar row shape, migration truth, and dual-backend behavior
- CANONICAL_CONTRACT_EXAMPLES:
  - a calendar source row carrying sync-state and governed write-context metadata
  - a calendar event row proving time-window query shape and provider-payload preservation
  - a same-source/same-external-id upsert path that stays idempotent across both backends

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Audit the existing calendar migration, structs, storage trait methods, backend implementations, and tests against the signed refinement instead of starting greenfield.
  - Patch the smallest truthful set of storage-model, migration, and test changes needed to align current code to the v2 scope.
  - Re-run the calendar-specific and storage-wide proof commands from the product worktree until they pass cleanly.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
- CARRY_FORWARD_WARNINGS:
  - Do not act on the deleted-branch and greenfield story from the rejected first draft; the real code already exists.
  - Do not widen into Lens, sync orchestration, policy integration, correlation export, or mailbox correlation.
  - Do not silently mint new PRIM IDs or appendix claims that are not already present in the current spec.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - current code reality versus the signed v2 scope
  - temporal and recurrence storage truth
  - portable migration plus dual-backend conformance
  - governed write-context and provenance preservation
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
- POST_MERGE_SPOTCHECKS:
  - verify the validated calendar storage changes are present on `main`

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- NOT_PROVEN_ITEMS:
  - This refinement does not prove whether the current JSON-based attendees/links/provider-payload storage is sufficient under the final validator interpretation or whether a narrower row-shape change is still required.
  - This refinement does not prove that the monolithic `Database` trait is the final acceptable boundary shape under current storage-capability guidance; coder and validators must settle that on real code, not prose.
  - This refinement does not prove any downstream Lens, sync, policy, correlation, or mailbox consumer behavior.

### DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: NONE_DISCOVERED (The current spec already contains the calendar primitive IDs this packet needs; the discovery was partial code realization, not a new primitive.)
- DISCOVERY_STUBS: NONE_CREATED (All relevant downstream calendar consumer stubs already existed before this refinement pass.)
- DISCOVERY_MATRIX_EDGES: NONE_FOUND (The current spec already contains the meaningful Calendar interaction edges; v2 is implementation alignment against them.)
- DISCOVERY_UI_CONTROLS: NONE_APPLICABLE (This packet is storage-only and does not create a new GUI surface.)
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED (The current spec already provides the normative storage contract for this activation.)
- DISCOVERY_JUSTIFICATION: This refinement still delivered high value by replacing a false greenfield activation story with real code truth, narrowing v2 to the actual spec-alignment and validation gap, and hydrating the exact product files and tests the governed run must use.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Master Spec v02.180 explicitly names the calendar entities, invariants, mutation-governance requirements, storage/indexing substrate, and portability/testing expectations this packet operates under. The activation gap is not missing spec intent; it is that already-landed product code must now be validated and, where needed, patched to match that intent truthfully.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE. The remaining uncertainty is implementation-state uncertainty inside the product worktree, not ambiguity in the governing spec clauses for this packet.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: This activation is a product-code alignment and validation pass against already-sufficient spec text. Any future appendix or DDL cleanup can be handled separately without blocking governed execution of this WP.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.0.4 Mutation governance (Hard Invariant) [ilja251220250127]
- CONTEXT_START_LINE: 55894
- CONTEXT_END_LINE: 55899
- CONTEXT_TOKEN: calendar_mutation
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **[HSK-CAL-WRITE-GATE]:** Direct database writes to `calendar_events` are **PROHIBITED** from the API layer or UI components.
  - All mutations MUST be submitted as `CalendarMutation` patches via a `WorkflowRun` targeting the `calendar_sync` mechanical engine.
  - Every successful mutation MUST emit a `Flight Recorder` span of type `calendar_mutation` with a back-link to the `job_id`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.0.4 Mutation and governance rules
- CONTEXT_START_LINE: 55912
- CONTEXT_END_LINE: 55920
- CONTEXT_TOKEN: Patch-sets are the only write primitive
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **No direct UI writes:** UI gestures emit jobs; only the host applies patches after validation and gates.
  - **Patch-sets are the only write primitive:** all calendar writes (local or external) are expressed as validated patch-sets with:
    - preconditions (`expected_etag`, `expected_local_rev`)
    - effect (`set`, `unset`, `append`, `remove`)
    - provenance (`job_id`, `client_op_id`, `idempotency_key`)
  - **External writes are explicitly gated:** any write that leaves the device requires capability + user confirmation unless the source is configured as `auto_export=true`.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.1 Raw entities
- CONTEXT_START_LINE: 55927
- CONTEXT_END_LINE: 55966
- CONTEXT_TOKEN: CalendarSource (RawContent)
- EXCERPT_ASCII_ESCAPED:
  ```text
  CalendarEvent (RawContent)
  - id (RID)
  - workspace_id
  - source_id (CalendarSource.id, e.g. "local", "google:...", "ics:...")
  - external_id (nullable; provider-specific event id)
  ...
  - attendees[] (ParticipantRef)
  - links[] (EntityLinkRef -> doc, canvas, task, mail_thread, etc.)
  - created_by (User/Agent RID)

  CalendarSource (RawContent)
  - id: "local:<id>" | "google:<account_id>:<calendar_id>" | "ics:<url>" | ...
  - type: "local" | "google" | "ics" | "caldav" | "other"
  ...
  - capability_profile_id: which jobs/agents may touch this source
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.1.1 Temporal invariants
- CONTEXT_START_LINE: 55976
- CONTEXT_END_LINE: 55995
- CONTEXT_TOKEN: Canonical storage
- EXCERPT_ASCII_ESCAPED:
  ```text
  Handshake must treat time as a **deterministic, lossless** domain.
  - **Canonical storage:** store `start_ts_utc` and `end_ts_utc` as UTC instants, and also store the originating `tzid` ...
  Required fields (additions to `CalendarEvent`):
  - `tzid: string`
  - `start_ts_utc: timestamp`
  - `end_ts_utc: timestamp`
  - `start_local: string?`
  - `end_local: string?`
  - `was_floating: bool`
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.1.2 Recurrence invariants
- CONTEXT_START_LINE: 55996
- CONTEXT_END_LINE: 56025
- CONTEXT_TOKEN: CalendarEventOverride
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **RRULE is source-of-truth:** store RRULE + `DTSTART` semantics + exceptions (`EXDATE`, `RDATE`) without lossy \u201cflattening\u201d.
  Required fields / structures:
  - `rrule: string?`
  - `rdate: string[]?`
  - `exdate: string[]?`
  - `series_id: string?`
  - `instance_key: string?`
  - `is_override: bool`
  CalendarEventOverride
  - id
  - series_id
  - instance_key
  - patch_set (start/end/title/attendees/etc)
  - created_by (human | job_id)
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.0.5 Never-lose-data rule
- CONTEXT_START_LINE: 55921
- CONTEXT_END_LINE: 55925
- CONTEXT_TOKEN: source_payload
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Preserve the original provider payload in `source_payload` (encrypted-at-rest if needed).
  - If parsing fails, store the raw record with `parse_status="failed"` and surface it as \u201cunparsed event\u201d, never drop it.
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.3 Storage and indexing
- CONTEXT_START_LINE: 56066
- CONTEXT_END_LINE: 56121
- CONTEXT_TOKEN: CREATE TABLE calendar_sources
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Relational table `calendar_events` with indices on `(workspace_id, start_ts, end_ts)` and full-text on `title`, `description`, `location`.
  CREATE TABLE calendar_sources (
      id TEXT PRIMARY KEY NOT NULL,
      ...
  );
  CREATE TABLE calendar_events (
      id TEXT PRIMARY KEY NOT NULL,
      ...
      FOREIGN KEY (source_id) REFERENCES calendar_sources(id)
  );
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.3.13.1 Pillar 2: Portable Schema & Migrations [CX-DBP-011]
- CONTEXT_START_LINE: 3282
- CONTEXT_END_LINE: 3296
- CONTEXT_TOKEN: Pillar 2: Portable Schema
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Pillar 2: Portable Schema & Migrations [CX-DBP-011]**
  All migrations MUST be written in DB-agnostic SQL. SQLite-specific syntax is forbidden.
  - FORBIDDEN: `strftime()`, SQLite datetime functions
  - FORBIDDEN: SQLite placeholder syntax `?1`, `?2`
  - FORBIDDEN: SQLite triggers with `OLD`/`NEW` semantics
  - REQUIRED: Migrations use version-managed framework (compatible with sqlx::migrate or similar)
  - REQUIRED: Schema definitions are pure DDL (no data transforms)
  ```

#### ANCHOR 9
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.3.13.1 Pillar 4: Dual-Backend Testing Early [CX-DBP-013]
- CONTEXT_START_LINE: 3314
- CONTEXT_END_LINE: 3325
- CONTEXT_TOKEN: Pillar 4: Dual-Backend Testing
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**
  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge
  ```

#### ANCHOR 10
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a710.4.0 Scope and positioning
- CONTEXT_START_LINE: 55799
- CONTEXT_END_LINE: 55806
- CONTEXT_TOKEN: backend force multiplier
- EXCERPT_ASCII_ESCAPED:
  ```text
  [ADD v02.155] In Phase 1, Calendar is also a backend force multiplier: `CalendarSourceSyncState`, `CalendarSource.write_policy`, `CalendarEvent.export_mode`, `capability_profile_id`, and `CalendarScopeHint` are canonical backend contracts for sync recovery, consent posture, AI-job mutation discipline, and scope-hint routing. These contracts MUST remain portable across SQLite-now / PostgreSQL-ready storage...
  ```

#### ANCHOR 11
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a711.9.3 CalendarEvent and ActivitySpan Join Semantics
- CONTEXT_START_LINE: 70743
- CONTEXT_END_LINE: 70772
- CONTEXT_TOKEN: CalendarEvent and ActivitySpan Join
- EXCERPT_ASCII_ESCAPED:
  ```text
  A calendar block is a time window; activity is a set of spans.
  Overlap definition:
  - Represent all spans as half-open intervals: `[start_ts, end_ts)`.
  - A span \u201cbelongs\u201d to an event if:
    - `span.start_ts < event.end_ts` AND `span.end_ts > event.start_ts` (any overlap)
  ```

#### ANCHOR 12
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a710.4.2.1 CalendarScopeHint
- CONTEXT_START_LINE: 57452
- CONTEXT_END_LINE: 57468
- CONTEXT_TOKEN: CalendarScopeHint
- EXCERPT_ASCII_ESCAPED:
  ```text
  CalendarScopeHint (DerivedContent, ephemeral)
  - time_range: [start_ts, end_ts)
  - active_event_id?: CalendarEvent.id
  - source: (active_event | manual_override | none)
  - policy_profile_id?: string
  - projection: (minimal | full | analytics_only)
  ...
  - trust_level: (local_authoritative | external_import | unknown)
  - confidence: float
  ```
