# Task Packet: WP-1-Loom-Storage-Portability-v1

## METADATA
- TASK_ID: WP-1-Loom-Storage-Portability-v1
- WP_ID: WP-1-Loom-Storage-Portability-v1
- BASE_WP_ID: WP-1-Loom-Storage-Portability
- DATE: 2026-03-14T00:37:30.625Z
- MERGE_BASE_SHA: 1a2568b0842ecc7b9b9aca4efcc9911cc2ce8cc8 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: CODER_B
<!-- Required before packet creation: CODER_A .. CODER_Z -->
- WORKFLOW_AUTHORITY: ORCHESTRATOR
<!-- Current repo-governance owner for workflow steering and hard-gate progression. -->
- TECHNICAL_ADVISOR: WP_VALIDATOR
<!-- Advisory WP-scoped validator; may question and challenge coder work but does not own final merge authority. -->
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
<!-- Final technical verdict authority across the WP batch. -->
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
<!-- Final merge-to-main authority. -->
- AGENTIC_MODE: NO
<!-- Allowed: YES | NO. Default NO; set to YES only with explicit operator-authorized sub-agent use. -->
- ORCHESTRATOR_MODEL: N/A
<!-- Required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL: gpt-5.4
- CODER_REASONING_STRENGTH: EXTRA_HIGH
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: VSCODE_EXTENSION_TERMINAL
- SESSION_HOST_FALLBACK: CLI_ESCALATION_WINDOW
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_PLUGIN_FIRST_WITH_2TRY_ESCALATION
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: .GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: .GOV/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: WINDOWS_TERMINAL
- MODEL_FAMILY_POLICY: OPENAI_GPT_SERIES_ONLY
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Loom-Storage-Portability-v1
- WP_VALIDATOR_LOCAL_BRANCH: validate/WP-1-Loom-Storage-Portability-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wt-WPV-WP-1-Loom-Storage-Portability-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: validate/WP-1-Loom-Storage-Portability-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/validate/WP-1-Loom-Storage-Portability-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Loom-Storage-Portability-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: integrate/WP-1-Loom-Storage-Portability-v1
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../wt-INTV-WP-1-Loom-Storage-Portability-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: integrate/WP-1-Loom-Storage-Portability-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/integrate/WP-1-Loom-Storage-Portability-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Loom-Storage-Portability-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Loom-Storage-Portability-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Loom-Storage-Portability-v1
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- **Status:** Done
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Loom-MVP, WP-1-Storage-Abstraction-Layer, WP-1-Artifact-System-Foundations
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Media-Downloader-Loom-Bridge, WP-1-Video-Archive-Loom-Integration
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Loom-Storage-Portability-v1
- LOCAL_WORKTREE_DIR: ../wt-WP-1-Loom-Storage-Portability-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Loom-Storage-Portability-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Loom-Storage-Portability-v1
- REMOTE_BACKUP_LIFECYCLE: TEMPORARY
<!-- WP backup branches may be deleted after Operator-approved cleanup; later dead links are non-blocking. -->
- BACKUP_PUSH_STATUS: REQUIRED_BEFORE_DESTRUCTIVE_OPS
<!-- Treat the WP backup branch as the phase-boundary recovery branch. Preserve the latest committed restart-safe state at packet/refinement checkpoint, bootstrap claim, skeleton checkpoint, skeleton approval, and before destructive/state-hiding local git actions. -->
- HEARTBEAT_INTERVAL_MINUTES: 15
<!-- Integer minutes; update runtime status/receipts on event boundaries and at this interval only while actively working. -->
- STALE_AFTER_MINUTES: 45
<!-- Integer minutes; heartbeat older than this threshold is stale. -->
- MAX_CODER_REVISION_CYCLES: 3
- MAX_VALIDATOR_REVIEW_CYCLES: 3
- MAX_RELAY_ESCALATION_CYCLES: 2
- WP_COMMUNICATION_DIR: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v1
- WP_THREAD_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: 019ceb0d-9a03-75f3-a6cc-d16396e2e8d0
- INTEGRATION_VALIDATOR_OF_RECORD: 019cee03-c18e-7f41-b3ac-2e0819cc6be1
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- USER_SIGNATURE: ilja140320260134
- PACKET_FORMAT_VERSION: 2026-03-12
## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: NONE
Next: Await Gate 4 acknowledgment before merge to main.

## WP_COMMUNICATIONS (NON-AUTHORITATIVE; REQUIRED FOR NEW PACKETS)
- RULE: The task packet remains authoritative for scope, status, branch/worktree truth, acceptance, and verdict.
- PURPOSE: The per-WP communication folder holds freeform discussion, liveness state, and deterministic receipts for multi-session work.
- COMMUNICATION AUTHORITY RULE:
  - all roles use the packet-declared `WP_COMMUNICATION_DIR`
  - do not improvise role-local inboxes or worktree-local communication authority
  - if there is any dispute about where to write, `WP_COMMUNICATION_DIR` wins
- AUTHORITY SPLIT:
  - `WORKFLOW_AUTHORITY` owns workflow steering and hard gates
  - `TECHNICAL_ADVISOR` is the WP-scoped advisory validator
  - `TECHNICAL_AUTHORITY` owns final technical verdict authority
  - `MERGE_AUTHORITY` owns merge-to-main authority
  - `WP_VALIDATOR_OF_RECORD` and `INTEGRATION_VALIDATOR_OF_RECORD` name the active validator sessions once assigned
- THREAD.md:
  - append-only freeform conversation for Operator, Orchestrator, Coder, and Validator
  - may contain steering, questions, clarifications, and soft coordination
- RUNTIME_STATUS.json:
  - non-authoritative liveness and watch state
  - uses repo-governance runtime states: `submitted | working | input_required | completed | failed | canceled`
  - use for next expected actor, waiting state, validator trigger, heartbeat posture, validation readiness, and bounded review-loop counters
- RECEIPTS.jsonl:
  - append-only deterministic receipt ledger
  - one JSON object per line
  - use for assignment, status, heartbeat, steering, repair, validation, and handoff receipts
- SESSION START + WAKE RULE:
  - only `WORKFLOW_AUTHORITY` may start repo-governed Coder, WP Validator, and Integration Validator sessions
  - primary launch path is `SESSION_HOST_PREFERENCE` via `SESSION_PLUGIN_BRIDGE_ID`
  - if the plugin path fails or times out `SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION` times for the same role/WP session, `CLI_ESCALATION_HOST_DEFAULT` is allowed as the escalation host
  - `SESSION_PLUGIN_REQUESTS_FILE` and `SESSION_REGISTRY_FILE` are the deterministic launch/watch state for those sessions
- WATCH RULE:
  - primary wake-up channel is `SESSION_WAKE_CHANNEL_PRIMARY`
  - fallback wake-up channel is `SESSION_WAKE_CHANNEL_FALLBACK`
  - do not rely on continuous polling when a watch event can be used
- CONFLICT RULE:
  - if THREAD.md, RUNTIME_STATUS.json, or RECEIPTS.jsonl conflicts with this packet, this packet wins
- LOOP LIMIT RULE:
  - do not exceed `MAX_CODER_REVISION_CYCLES`, `MAX_VALIDATOR_REVIEW_CYCLES`, or `MAX_RELAY_ESCALATION_CYCLES` without explicit Operator steering recorded in the packet or thread
- HEARTBEAT RULE:
  - do not poll continuously
  - update `RUNTIME_STATUS.json` and append a receipt on session start, major phase change, blocker/unblock, handoff, completion, and every `HEARTBEAT_INTERVAL_MINUTES` only while active

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- NOTE: `AGENTIC_MODE: YES` means sub-agent use is explicitly authorized for this WP; `AGENTIC_MODE: NO` means all roles remain single-session.
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Loom-Storage-Portability-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [OSS_DOC] SQLite FTS5 docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://sqlite.org/fts5.html | Why: canonical reference for SQLite-side full-text search behavior and index locality
  - [OSS_DOC] PostgreSQL full text search docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://www.postgresql.org/docs/current/textsearch.html | Why: canonical reference for PostgreSQL-side ranked text search and backend-specific query power
  - [BIG_TECH] Google Cloud Spanner full-text search overview | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://cloud.google.com/spanner/docs/full-text-search | Why: current large-scale vendor reference showing richer backend-specific search features can exist behind one SQL-facing search surface
  - [OSS_DOC] OpenLineage spec docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://openlineage.io/docs/spec/ | Why: useful reference for typed lineage and provenance payloads that survive transport and backend changes
  - [GITHUB] OpenLineage repository | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://github.com/OpenLineage/OpenLineage | Why: concrete repository-scale example of typed lineage/provenance contract evolution
  - [PAPER] In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines | 2025-11-05 | Retrieved: 2026-03-13T22:38:08Z | https://arxiv.org/abs/2511.03480 | Why: recent provenance-indexing paper supporting explicit, queryable source/provenance structures instead of opaque backend-local metadata
  - [GITHUB] pgvector repository | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://github.com/pgvector/pgvector | Why: high-signal reference for backend-specific search acceleration that should remain optional rather than canonical in this packet
- RESEARCH_SYNTHESIS:
  - A portability packet should preserve one stable API and semantic contract while allowing provider-specific indexing and query plans behind the boundary.
  - Big-tech search systems confirm that richer backend-specific ranking, tokenization, and query expansion can stay behind a stable query surface instead of redefining canonical filter meaning.
  - Typed provenance payloads are more durable than ad hoc search or edge metadata and map well to Loom `source_anchor` export/replay expectations.
  - Recent provenance-indexing research reinforces that explicit source/provenance structures should stay queryable and transport-stable, which matches Handshake's need for durable `LoomSourceAnchor` semantics across storage, export, and replay.
  - Search parity should mean stable filters, result identity, and semantic guarantees, not identical score math across SQLite and PostgreSQL.
  - Backend-specific enhancements are useful, but they must not become the only definition of Loom search or graph behavior while Handshake still promises SQLite-now and PostgreSQL-ready portability.
- GITHUB_PROJECT_DECISIONS:
  - OpenLineage/OpenLineage -> ADAPT (NONE)
  - pgvector/pgvector -> TRACK_ONLY (NONE)
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_VERDICT: PASS
- SOURCE_SCAN_DECISIONS:
  - SQLite FTS5 docs -> ADOPT (IN_THIS_WP)
  - PostgreSQL full text search docs -> ADOPT (IN_THIS_WP)
  - Google Cloud Spanner full-text search overview -> ADOPT (IN_THIS_WP)
  - OpenLineage spec docs -> ADAPT (IN_THIS_WP)
  - OpenLineage repository -> ADAPT (IN_THIS_WP)
  - In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines -> ADAPT (IN_THIS_WP)
  - pgvector repository -> REJECT (REJECT_DUPLICATE)
- MATRIX_GROWTH_CANDIDATES:
  - Stable search API plus provider-local indexing -> IN_THIS_WP (stub: NONE)
  - Portable source-anchor lineage plus export/replay durability -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep provider-specific FTS and ranking logic inside backend implementations, not in portable migrations.
  - Assert semantic parity through shared Loom conformance tests rather than comparing SQL query text.
  - Treat `LoomSourceAnchor` and view/search filters as portable contract structs, not adapter-only shapes.
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-LoomBlock
  - PRIM-LoomBlockContentType
  - PRIM-LoomBlockDerived
  - PRIM-LoomEdge
  - PRIM-LoomEdgeType
  - PRIM-LoomViewFilters
  - PRIM-LoomSearchFilters
  - PRIM-LoomBlockSearchResult
  - PRIM-LoomSourceAnchor
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
- MECHANICAL_ENGINES_TOUCHED:
  - engine.archivist
  - engine.librarian
  - engine.dba
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Loom
  - SQL to PostgreSQL shift readiness
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - One Loom storage contract plus dual provider implementations -> IN_THIS_WP (stub: NONE)
  - Portable Loom migrations plus replay-safe and down-safe verification -> IN_THIS_WP (stub: NONE)
  - SQLite FTS and PostgreSQL text search behind one API -> IN_THIS_WP (stub: NONE)
  - View filter parity across providers -> IN_THIS_WP (stub: NONE)
  - Source-anchor durability across storage, export, and replay -> IN_THIS_WP (stub: NONE)
  - Asset blob path stability plus storage metadata parity -> IN_THIS_WP (stub: NONE)
  - Rebuildable derived metrics plus provider-local indexes -> IN_THIS_WP (stub: NONE)
  - Shared Loom conformance tests over SQLite and PostgreSQL -> IN_THIS_WP (stub: NONE)
  - Thin API seam over portable storage behavior -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Loom | CAPABILITY_SLICE: Portable block and edge record parity | SUBFEATURES: `LoomBlock`, `LoomEdge`, content-hash dedup, metrics rebuildability, and stable backend-neutral meaning | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomBlockDerived, PRIM-LoomEdgeType | MECHANICAL: engine.archivist, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the packet should guarantee that block and edge semantics survive SQLite and PostgreSQL backends without adapter drift
  - PILLAR: Loom | CAPABILITY_SLICE: Portable view, search, and source-anchor contract | SUBFEATURES: `LoomViewFilters`, `LoomSearchFilters`, `LoomBlockSearchResult`, and `LoomSourceAnchor` parity | PRIMITIVES_FEATURES: PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult, PRIM-LoomSourceAnchor | MECHANICAL: engine.librarian, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the API contract should preserve the same filter meaning and source-anchor durability across both backends
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Loom migration and DDL portability | SUBFEATURES: replay-safe migrations, down migrations, provider-local indexes outside portable DDL, and no trigger-dependent semantics | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the direct portability law bridge from spec to code for the Loom surface
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Cross-provider Loom conformance coverage | SUBFEATURES: shared test helpers for SQLite and PostgreSQL parity over CRUD, search, view, dedup, and anchor round-trips | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | MECHANICAL: engine.archivist, engine.librarian, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity should be proven by tests, not inferred from provider implementations
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Loom block and edge storage parity | JobModel: UI_ACTION | Workflow: loom_surface_crud | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_block_created, loom_block_updated, loom_block_deleted, loom_edge_created, loom_edge_deleted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend parity must preserve CRUD meaning and existing Loom telemetry regardless of provider implementation
  - Capability: Loom import and dedup portability | JobModel: WORKFLOW | Workflow: loom_import | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_dedup_hit, loom_block_created | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: content-hash dedup, asset-path layout, and import-created blocks must preserve the same semantics across backends
  - Capability: Loom view portability | JobModel: UI_ACTION | Workflow: loom_view_query | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_view_queried | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: `LoomViewFilters` and grouped-view semantics must not drift when the backend changes
  - Capability: Loom search portability | JobModel: UI_ACTION | Workflow: loom_search | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_search_executed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: search scoring may differ by provider, but filter meaning, result identity, and backend-neutral contract must remain stable
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Media-Downloader-Loom-Bridge-v1 -> KEEP_SEPARATE
  - WP-1-Video-Archive-Loom-Integration-v1 -> KEEP_SEPARATE
  - WP-1-Loom-Preview-VideoPosterFrames-v1 -> KEEP_SEPARATE
  - WP-1-Loom-MVP-v1 -> KEEP_SEPARATE
  - WP-1-Storage-Abstraction-Layer-v3 -> KEEP_SEPARATE
  - WP-1-Artifact-System-Foundations-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (WP-1-Storage-Abstraction-Layer-v3)
  - src/backend/handshake_core/src/storage/loom.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/sqlite.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/postgres.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/tests.rs -> NOT_PRESENT (NONE)
  - src/backend/handshake_core/src/api/loom.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/loom_fs.rs -> PARTIAL (NONE)
  - src/backend/handshake_core/migrations/0013_loom_mvp.sql -> PARTIAL (WP-1-Loom-MVP-v1)
## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: NO
- GUI_IMPLEMENTATION_ADVICE_VERDICT: NOT_APPLICABLE
- GUI_REFERENCE_DECISIONS:
  - NONE
- HANDSHAKE_GUI_ADVICE:
  - NONE
- HIDDEN_GUI_REQUIREMENTS:
  - NONE
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - NONE
## SCOPE
- What: Harden Loom block, edge, search, view, source-anchor, migration, and asset-path behavior into one portable backend contract that preserves meaning across SQLite and PostgreSQL implementations.
- Why: The codebase already has Loom implementations on both providers, but parity is not proven by shared conformance tests and the portable contract is still vulnerable to adapter drift. This packet makes the spec's Loom portability law executable and trustworthy for downstream Loom bridge work.
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
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- WAIVER-LIVE-SMOKETEST-GOV-SYNC-WP-1-Loom-Storage-Portability-v1-001 [CX-573F]
  - Date: 2026-03-14
  - Scope: `.GOV/roles_shared/BUILD_ORDER.md`, `.GOV/roles_shared/TASK_BOARD.md`, `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`, `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v1/RECEIPTS.jsonl`, `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v1/RUNTIME_STATUS.json`, `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v1/THREAD.md`, `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v1/RECEIPTS.jsonl`, `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v1/RUNTIME_STATUS.json`, `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v1/THREAD.md`, `.GOV/scripts/create-task-packet.mjs`, `.GOV/scripts/validation/spec-eof-appendices-check.mjs`, `.GOV/scripts/wp-communications-lib.mjs`, `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md`, `.GOV/task_packets/WP-1-Structured-Collaboration-Artifact-Family-v1.md`, and `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v1.md`
  - Justification: Operator explicitly authorized live-smoketest governance remediation and workflow/tooling repair while keeping product-code scope restricted to the Loom packet. These governance files are required to preserve packet truth, validator evidence, and cross-packet repository compliance for this WP branch.
  - Approver: Operator (chat instruction on 2026-03-14)
  - Expiry: On WP closure (validation complete).

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test -p handshake_core loom
  cargo test -p handshake_core --test storage_conformance
  just gov-check
```

### DONE_MEANS
- Loom block, edge, view, search, and source-anchor semantics remain stable across SQLite and PostgreSQL implementations.
- Portable migrations and down migrations for Loom tables stay replay-safe and provider-neutral.
- Shared Loom conformance tests prove parity for CRUD, dedup, views, search filters, and source-anchor round-trips across both backends.
- Filesystem asset-path layout remains stable and compatible with the portable storage contract.

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

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.178.md (recorded_at: 2026-03-14T00:37:30.625Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.156]
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
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
  cargo test -p handshake_core loom
  cargo test -p handshake_core --test storage_conformance
  just gov-check
  ```
- RISK_MAP:
  - "provider-specific search logic changes filter meaning" -> "view and search parity break across SQLite and PostgreSQL"
  - "source anchors fail to round-trip on one backend" -> "backlinks, context snippets, and downstream bridge packets lose stable provenance"
  - "filesystem asset-path layout drifts from storage metadata" -> "export, replay, and dedup behavior become unreliable"
  - "Loom portability work widens into workflow-runtime changes" -> "packet loses file-lock isolation and becomes harder to validate"
## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- REASON_NO: This packet is a backend portability and conformance activation pass; no separate bootstrap-time end-to-end closure plan is required beyond the signed scope, DONE_MEANS, and validator checks.

## IMPLEMENTATION
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- Repaired scope after an interrupted formatter/test pass widened product diffs into out-of-scope files; preserved safety with `git stash push -m "SAFETY: before Loom scope repair after interrupted format" -- <out-of-scope files>` and returned `src/backend/handshake_core` to Loom-only dirt before resuming.
- Trimmed formatting-only drift out of `src/backend/handshake_core/src/api/loom.rs` and `src/backend/handshake_core/src/storage/loom.rs` with a safety stash so the final packet diff stays focused on files with Loom portability value.
- Captured deterministic SHA manifests with `just cor701-sha` for every changed non-`.GOV/` file that remains in scope.
- Verified the Loom unit/API/migration surface with `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` and the dual-backend parity helper with `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance`.
- `just gov-check` initially failed on unrelated shared-governance drift (`.GOV/roles_shared/BUILD_ORDER.md` out of date); ran the minimal allowed governance repair `just build-order-sync`, then re-ran `just gov-check` to PASS. Governance edits remain unstaged and outside the packet product manifest.

## VALIDATION
- **Target File**: `src/backend/handshake_core/src/loom_fs.rs`
- **Start**: 1
- **End**: 90
- **Line Delta**: 54
- **Pre-SHA1**: `a5fa14bb612e83624ad8b6f5b54e81733f1847bb`
- **Post-SHA1**: `fef3ab6cbe0f9cb344beb2b8e20d035811f8da8e`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance` PASS; `just gov-check` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-14T22:33:21.0517425+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Added Loom asset-path invariance tests for original, preview, proxy, and fallback tiers.

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 1
- **End**: 4879
- **Line Delta**: 158
- **Pre-SHA1**: `e52777c01d146c9c30d5f5696ab88d2ca3223ada`
- **Post-SHA1**: `f105bd3fb4bfda5fb9259a330365f651038c4c03`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance` PASS; `just gov-check` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-14T22:33:21.0517425+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Aligned PostgreSQL Loom search with the portable search surface, escaped literal wildcard tokens, and fixed sorted-view filter group selection.

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 5515
- **Line Delta**: 130
- **Pre-SHA1**: `3284fe89e6d4ac792248b5496b640cffff4e4b46`
- **Post-SHA1**: `6d815d9ff393eb7073462bcc57bd14286049c6e2`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance` PASS; `just gov-check` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-14T22:33:21.0517425+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Fixed sorted-view filter semantics so SQLite and PostgreSQL select the same filtered Loom groups.

- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 1
- **End**: 2263
- **Line Delta**: 720
- **Pre-SHA1**: `477228885d07d32c9b7f3152435f183a2b08f0e4`
- **Post-SHA1**: `83a3a74e37c6ddcd7c335a6cc22694eba328fbfc`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance` PASS; `just gov-check` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-14T22:33:21.0517425+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Added shared Loom conformance coverage for CRUD, dedup, filtered views, search parity, source-anchor round-trips, and portable migration assertions.

- **Target File**: `src/backend/handshake_core/tests/storage_conformance.rs`
- **Start**: 1
- **End**: 54
- **Line Delta**: 25
- **Pre-SHA1**: `726bb066a5eb261596002b602bb13bf5ea59bfc8`
- **Post-SHA1**: `654cec765edf820ee71d4b3663823110f9341632`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance` PASS; `just gov-check` PASS
- **Artifacts**: See `## EVIDENCE` log entries for command outputs and SHA256 values.
- **Timestamp**: 2026-03-14T22:33:21.0517425+01:00
- **Operator**: Codex CLI (Coder)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Wired the shared Loom conformance helper into both SQLite and PostgreSQL integration entrypoints.

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete; GATES_PASS (`just post-work`) PASS; TEST_PLAN evidence recorded; ready for validator re-review on the staged packet-scoped diff
- What changed in this update: Fixed PostgreSQL Loom search portability so the backend searches only `title`, `original_filename`, and `full_text_index`, escapes literal `%` / `_` tokens, and still applies tag/mention/mime/content filters; fixed sorted-view filter group selection parity in both backends; added shared Loom conformance coverage for CRUD, dedup, views, search filters, source-anchor round-trips, and portable migration expectations; added Loom asset-path invariance tests.
- Next step / handoff hint: Re-run validator review against the staged Loom diff and the recorded `## VALIDATION`, `## EVIDENCE_MAPPING`, and `## EVIDENCE` blocks; the only post-work warning was that the broader worktree still has unrelated unstaged governance churn outside the staged packet diff.

## EVIDENCE_MAPPING
- REQUIREMENT: "Loom block, edge, view, search, and source-anchor semantics remain stable across SQLite and PostgreSQL implementations."
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:1987`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2336`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2407`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2755`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:573`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1060`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1114`
- EVIDENCE: `src/backend/handshake_core/tests/storage_conformance.rs:32`
- EVIDENCE: `src/backend/handshake_core/tests/storage_conformance.rs:40`
- REQUIREMENT: "Portable migrations and down migrations for Loom tables stay replay-safe and provider-neutral."
- EVIDENCE: `src/backend/handshake_core/migrations/0013_loom_mvp.sql:39`
- EVIDENCE: `src/backend/handshake_core/migrations/0013_loom_mvp.sql:74`
- EVIDENCE: `src/backend/handshake_core/migrations/0013_loom_mvp.down.sql:2`
- EVIDENCE: `src/backend/handshake_core/migrations/0013_loom_mvp.down.sql:3`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:2108`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:2187`
- REQUIREMENT: "Shared Loom conformance tests prove parity for CRUD, dedup, views, search filters, and source-anchor round-trips across both backends."
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:573`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1114`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1131`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1148`
- EVIDENCE: `src/backend/handshake_core/tests/storage_conformance.rs:32`
- EVIDENCE: `src/backend/handshake_core/tests/storage_conformance.rs:40`
- REQUIREMENT: "Filesystem asset-path layout remains stable and compatible with the portable storage contract."
- EVIDENCE: `src/backend/handshake_core/src/loom_fs.rs:41`
- EVIDENCE: `src/backend/handshake_core/src/loom_fs.rs:58`
- REQUIREMENT: "[ADD v02.156] LoomBlock/LoomEdge records, LoomViewFilters, LoomSearchFilters, LoomBlockSearchResult, and LoomSourceAnchor are canonical portable backend library contracts. Their meaning MUST survive SQLite-now / PostgreSQL-ready storage, export, and replay instead of being hidden behind view-only adapters."
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:1987`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2407`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:573`
- EVIDENCE: `src/backend/handshake_core/tests/storage_conformance.rs:32`
- REQUIREMENT: "[LM-SEARCH-001] The search API MUST be backend-agnostic. The storage trait exposes `search_loom_blocks(query, filters) -> Vec<LoomBlockSearchResult>`. The implementation varies by backend."
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2336`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2403`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2419`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2431`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2755`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1114`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1131`
- EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1148`
- REQUIREMENT: "[LM-SEARCH-002] On PostgreSQL, search results MUST be filterable by graph relationships (tags, mentions, backlink depth) within the query."
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2419`
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2431`

## EVIDENCE
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v1/loom_lib.log`
- LOG_SHA256: `8fd29dcddd38b38d688743b251add6913854d52ff4673caf438f895fe7f14f4b`
- PROOF_LINES: `test api::loom::tests::view_and_search_emit_events ... ok`; `test api::loom::tests::import_dedup_emits_fr_evt_loom_006 ... ok`; `test storage::tests::loom_migration_schema_is_portable_postgres ... ok`; `test storage::tests::loom_migration_schema_is_portable_sqlite ... ok`; `test result: ok. 6 passed; 0 failed`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v1/storage_conformance.log`
- LOG_SHA256: `d5cf132585d48499a5ec454e8eb92a01716a1a39914a03aaa5cfdca9623df5cf`
- PROOF_LINES: `test postgres_loom_storage_conformance ... ok`; `test postgres_storage_conformance ... ok`; `test sqlite_loom_storage_conformance ... ok`; `test sqlite_storage_conformance ... ok`; `test result: ok. 4 passed; 0 failed`

- COMMAND: `just build-order-sync`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v1/build_order_sync.log`
- LOG_SHA256: `8e2785da99fde70c8d65cdf9b6908b4709b59f734b80293666a7dccea5aaa3d0`
- PROOF_LINES: `build-order-sync ok: .GOV/roles_shared/BUILD_ORDER.md already up to date`

- COMMAND: `just gov-check`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v1/gov_check.log`
- LOG_SHA256: `a3843e88b44f6f6c7b21f5f041986ec5bdbabc8c58a6fdeec2e2d96e37f8efe4`
- PROOF_LINES: `build-order-check ok`; `task-packet-claim-check ok`; `session-policy-check ok`; `session-control-runtime-check ok`; `gov-check ok`

- COMMAND: `just post-work WP-1-Loom-Storage-Portability-v1`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v1/post_work.log`
- LOG_SHA256: `ff9588c63f931c75036b4b63339f8d599cdf82d800217ee71e889f2423f9e405`
- PROOF_LINES: `Post-work validation PASSED (deterministic manifest gate; not tests) with warnings`; `RESULT: PASS`; `WHY: Post-work checks passed.`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

## VALIDATION REPORT - WP-1-Loom-Storage-Portability-v1 (2026-03-14)
Verdict: FAIL

Validation Claims:
- GATES_PASS (deterministic manifest gate: `just post-work WP-1-Loom-Storage-Portability-v1`; not tests): FAIL
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS
- SPEC_CONFORMANCE_CONFIRMED (DONE_MEANS + SPEC_ANCHOR -> evidence mapping): NO

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md` (status: In Progress)
- Spec: `Handshake_Master_Spec_v02.178.md` (`2.3.13 Storage Backend Portability Architecture`, `2.3.13.7 Loom Storage Trait + Portable Schema`, `10.12 Loom`, including `LM-SEARCH-001` and `LM-SEARCH-002`)

Files Checked:
- `src/backend/handshake_core/src/api/loom.rs`
- `src/backend/handshake_core/src/loom_fs.rs`
- `src/backend/handshake_core/src/storage/loom.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/tests.rs`
- `src/backend/handshake_core/tests/storage_conformance.rs`
- `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md`
- `.GOV/refinements/WP-1-Loom-Storage-Portability-v1.md`

Findings:
- Packet handoff incomplete [CX-573][COR-701]: `## VALIDATION`, `## STATUS_HANDOFF`, `## EVIDENCE_MAPPING`, and `## EVIDENCE` still contain template placeholders instead of the required manifest hashes, file windows, requirement-to-evidence mappings, and command evidence. `just post-work WP-1-Loom-Storage-Portability-v1` therefore fails before a PASS verdict can be considered.
- PostgreSQL search token handling is not backend-agnostic for wildcard and special-character queries. `normalize_loom_search_tokens` leaves `%` and `_` intact, and `search_loom_blocks` turns each token into `ILIKE '%{token}%'` predicates. Queries such as `%` or `_` will broad-match rows on PostgreSQL while the SQLite FTS path tokenizes and quotes those inputs differently, so the portable search contract does not currently hold.
- The new Loom conformance tests do not cover the special-character search path. Current added coverage exercises alphanumeric queries (`portable parity`, `plan`, `metadata_shadow`) but does not protect the `%` / `_` parity case that the PostgreSQL implementation now exposes.

Hygiene / Forbidden Patterns:
- `just validator-scan`: PASS
- `just validator-dal-audit`: PASS
- `just gov-check`: PASS

Tests:
- `cargo test -p handshake_core loom --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake Artifacts/handshake-cargo-target"`: PASS
- `cargo test -p handshake_core --test storage_conformance --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake Artifacts/handshake-cargo-target"`: PASS

Risks & Suggested Actions:
- Fill the packet handoff sections with the real deterministic manifest, requirement-to-file evidence, and command evidence, then re-run `just post-work WP-1-Loom-Storage-Portability-v1`.
- Escape `%` and `_` (or switch to a typed full-text search path) in PostgreSQL Loom search so user queries cannot silently change semantics by backend.
- Add a dual-backend Loom search test that asserts identical behavior for wildcard/special-character queries.

Reason for FAIL:
- The deterministic manifest gate failed, which is a hard blocker for validation closure.
- The PostgreSQL search implementation still permits backend-specific wildcard semantics, so the packet's portability target is not yet met.

## VALIDATION REPORT - WP-1-Loom-Storage-Portability-v1 (2026-03-14 REVALIDATION)
Verdict: FAIL

Validation Claims:
- GATES_PASS (deterministic manifest gate: `just post-work WP-1-Loom-Storage-Portability-v1`; not tests): PASS
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS
- SPEC_CONFORMANCE_CONFIRMED (DONE_MEANS + SPEC_ANCHOR -> evidence mapping): NO

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md` (status: In Progress)
- Spec: `Handshake_Master_Spec_v02.178.md` (`2.3.13 Storage Backend Portability Architecture`, `2.3.13.7 Loom Storage Trait + Portable Schema`, `10.12 Loom`, including `LM-SEARCH-001` and `LM-SEARCH-002`)
- Revalidated commit: `32948835b3d9c8c7b1cbeb7b20b79ed81350acae`

Files Checked:
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/storage/tests.rs`
- `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md`
- `.GOV/roles_shared/TASK_BOARD.md`
- `Handshake_Master_Spec_v02.178.md`
- `.GOV/scripts/validation/spec-eof-appendices-check.mjs`

Findings:
- The prior Loom blockers are fixed for the committed HEAD. The packet now contains a populated validation/evidence handoff, `just post-work WP-1-Loom-Storage-Portability-v1 --rev 32948835b3d9c8c7b1cbeb7b20b79ed81350acae` passes, PostgreSQL search now escapes `%` and `_`, and regression coverage exists for literal percent/underscore searches.
- Final integration closure is still blocked by governance drift. `just gov-check` fails because `Handshake_Master_Spec_v02.178.md` still lists `WP-1-Structured-Collaboration-Artifact-Family-v1` under `FEAT-LOCUS-WORK-TRACKING` `gap_stub_ids`, while `.GOV/roles_shared/TASK_BOARD.md` marks that packet `[VALIDATED]`. The validator gate only accepts gap references that are `[STUB]` or active official packets (`[READY_FOR_DEV]`, `[IN_PROGRESS]`, `[BLOCKED]`, `[ACTIVE]`), so the current committed state is not merge-ready.

Hygiene / Forbidden Patterns:
- `just validator-scan`: PASS
- `just validator-dal-audit`: PASS
- `just gov-check`: FAIL

Tests:
- `cargo test -p handshake_core loom --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake Artifacts/handshake-cargo-target"`: PASS
- `cargo test -p handshake_core --test storage_conformance --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake Artifacts/handshake-cargo-target"`: PASS
- `just post-work WP-1-Loom-Storage-Portability-v1 --rev 32948835b3d9c8c7b1cbeb7b20b79ed81350acae`: PASS

Risks & Suggested Actions:
- Remove or replace the stale `WP-1-Structured-Collaboration-Artifact-Family-v1` gap reference from the spec appendix row for `FEAT-LOCUS-WORK-TRACKING`, or move the packet back into an active task-board state that matches the appendix semantics.
- Re-run `just gov-check` after that governance correction before requesting another final integration validation.

Reason for FAIL:
- The committed Loom implementation now validates at the code and manifest levels, but the branch still fails the required merge-blocking governance gate.

## VALIDATION REPORT - WP-1-Loom-Storage-Portability-v1 (2026-03-15 REVALIDATION)
Verdict: FAIL

Validation Claims:
- GATES_PASS (deterministic manifest gate: `just post-work WP-1-Loom-Storage-Portability-v1`; not tests): FAIL
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS
- SPEC_CONFORMANCE_CONFIRMED (DONE_MEANS + SPEC_ANCHOR -> evidence mapping): NO

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md` (status: In Progress)
- Spec: `Handshake_Master_Spec_v02.178.md` (`2.3.13 Storage Backend Portability Architecture`, `2.3.13.7 Loom Storage Trait + Portable Schema`, `10.12 Loom`, including `LM-SEARCH-001` and `LM-SEARCH-002`)
- Revalidated commit: `c4ba7deb57810293d4f6e4bea207eeb2e56a645b`

Files Checked:
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/storage/tests.rs`
- `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md`
- `.GOV/task_packets/WP-1-Structured-Collaboration-Artifact-Family-v1.md`
- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/roles_shared/BUILD_ORDER.md`
- `.GOV/scripts/validation/spec-eof-appendices-check.mjs`
- `.GOV/scripts/validation/task-board-check.mjs`

Findings:
- The prior Loom implementation issues remain fixed. The packet TEST_PLAN commands pass for the current head: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` and `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance`.
- The governance fix commit resolves the previous appendix-state blocker by allowing validated and other non-active official packet states in `spec-eof-appendices-check.mjs`, and `spec-eof-appendices-check` now passes inside `just gov-check`.
- Final integration closure is still blocked by a different governance defect. `just gov-check` now fails because `.GOV/roles_shared/TASK_BOARD.md` marks `WP-1-Structured-Collaboration-Artifact-Family-v1` as `[VALIDATED]`, while `.GOV/task_packets/WP-1-Structured-Collaboration-Artifact-Family-v1.md` only contains `Verdict: PENDING`. `task-board-check.mjs` requires a `Verdict: PASS|FAIL|OUTDATED_ONLY` line for modern packets that are marked done/validated.
- `just post-work WP-1-Loom-Storage-Portability-v1 --rev c4ba7deb57810293d4f6e4bea207eeb2e56a645b` fails for the current committed head. The commit changes `.GOV/roles_shared/BUILD_ORDER.md`, but the Loom packet manifest is still scoped to the earlier Loom product-file set and pre-SHA chain, so the deterministic manifest gate reports an out-of-scope governance file plus stale `pre_sha1` expectations for the Loom files.

Hygiene / Forbidden Patterns:
- `just validator-scan`: PASS
- `just validator-dal-audit`: PASS
- `just gov-check`: FAIL

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom --target-dir "../Handshake Artifacts/handshake-cargo-target"`: PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance --target-dir "../Handshake Artifacts/handshake-cargo-target"`: PASS
- `just post-work WP-1-Loom-Storage-Portability-v1 --rev c4ba7deb57810293d4f6e4bea207eeb2e56a645b`: FAIL

Risks & Suggested Actions:
- Add the required Validator verdict line to `WP-1-Structured-Collaboration-Artifact-Family-v1` or move that packet out of `[VALIDATED]` state before expecting `just gov-check` to pass.
- Reconcile the Loom packet manifest with the governance-only follow-up commit on this WP branch, or move the governance repair out of this WP branch so `just post-work --rev c4ba7deb57810293d4f6e4bea207eeb2e56a645b` validates the actual committed diff.
- Re-run `just gov-check` and `just post-work WP-1-Loom-Storage-Portability-v1 --rev c4ba7deb57810293d4f6e4bea207eeb2e56a645b` before requesting another final integration validation.

Reason for FAIL:
- The current committed Loom branch state still fails both the merge-blocking governance check and the deterministic manifest gate.

## VALIDATION REPORT - WP-1-Loom-Storage-Portability-v1 (2026-03-15 FINAL REVALIDATION)
Verdict: PASS

Validation Claims:
- GATES_PASS (deterministic manifest gate: `just post-work WP-1-Loom-Storage-Portability-v1`; not tests): PASS
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS
- SPEC_CONFORMANCE_CONFIRMED (DONE_MEANS + SPEC_ANCHOR -> evidence mapping): YES

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md` (status: Done)
- Spec: `Handshake_Master_Spec_v02.178.md` (`2.3.13 Storage Backend Portability Architecture`, `2.3.13.7 Loom Storage Trait + Portable Schema`, `10.12 Loom`, including `LM-SEARCH-001` and `LM-SEARCH-002`)
- Revalidated range: `1a2568b0842ecc7b9b9aca4efcc9911cc2ce8cc8..def20eafe2a401587e4166463638129e3f1fa912`
- Candidate commit: `def20eafe2a401587e4166463638129e3f1fa912`

Files Checked:
- `src/backend/handshake_core/src/loom_fs.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/tests.rs`
- `src/backend/handshake_core/tests/storage_conformance.rs`
- `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md`
- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/roles_shared/BUILD_ORDER.md`
- `.GOV/scripts/validation/spec-eof-appendices-check.mjs`
- `.GOV/scripts/validation/task-board-check.mjs`

Findings:
- No blocking technical or governance defects remain in the instructed candidate range.
- `just topology-registry-sync` was run in the clean detached validation worktree before `just gov-check` and produced no tracked diff.
- `just post-work WP-1-Loom-Storage-Portability-v1 --range 1a2568b0842ecc7b9b9aca4efcc9911cc2ce8cc8..def20eafe2a401587e4166463638129e3f1fa912` passes. It reports CX-573F warnings for waived out-of-scope governance/support files, but the waiver is present and the deterministic gate result is PASS.

Hygiene / Forbidden Patterns:
- `just validator-scan`: PASS
- `just validator-dal-audit`: PASS
- `just gov-check`: PASS

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom --target-dir "../Handshake Artifacts/handshake-cargo-target"`: PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance --target-dir "../Handshake Artifacts/handshake-cargo-target"`: PASS
- `just post-work WP-1-Loom-Storage-Portability-v1 --range 1a2568b0842ecc7b9b9aca4efcc9911cc2ce8cc8..def20eafe2a401587e4166463638129e3f1fa912`: PASS (with CX-573F warnings only)

Risks & Residual Notes:
- The candidate range includes waived out-of-scope governance/support files in addition to the Loom portability files. That residual scope expansion is documented by the packet waiver and does not block merge.

Reason for PASS:
- The instructed committed range validates at the packet, governance, product-test, and deterministic manifest levels.
