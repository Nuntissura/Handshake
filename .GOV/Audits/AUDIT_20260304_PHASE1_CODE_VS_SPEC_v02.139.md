# Audit: Phase 1 Code vs Master Spec v02.139

Template: `.GOV/templates/AUDIT_TEMPLATE.md`

## METADATA
- AUDIT_ID: AUDIT-20260304-PHASE1-CODE-VS-SPEC
- DATE_UTC: 2026-03-04T13:46:43Z
- AUDITOR: ORCHESTRATOR (CodexCLI)
- SPEC_CURRENT_POINTER: .GOV/roles_shared/SPEC_CURRENT.md
- SPEC_TARGET_RESOLVED: Handshake_Master_Spec_v02.139.md
- CODE_TARGET:
  - worktree: ../handshake_main
  - branch: main
  - commit_sha: 5f966720915cd188cd8b3891d0a4984bd26d7ef5
- SCOPE_SUMMARY: Cross-cutting audit to reconcile "done" reality (code + tests) against current Master Spec v02.139 and identify remediation needs.
- FOCUS_AREAS:
  - LLM-friendly data representation and auditability
  - PostgreSQL readiness (interfaces, migrations posture, portability)
  - Locus work tracking system (Phase 1)
  - Loom and media pipeline integration posture
  - Microtasking / executor correctness
  - Calendar drift / out-of-date lens behavior
- RELATED_WP_IDS:
  - WP-1-AI-Ready-Data-Architecture-v1
  - WP-1-Micro-Task-Executor-v1
  - WP-1-Loom-MVP-v1
  - WP-1-Media-Downloader-v2
  - WP-1-Migration-Framework-v2
  - WP-1-Dual-Backend-Tests-v2
  - WP-1-Storage-Abstraction-Layer-v3
  - WP-1-Storage-Foundation-v3
- OUT_OF_SCOPE:
  - None (audit-only; remediation captured as stubs, not implemented here)

## METHOD (EVIDENCE-BASED)

Rules:
- "Done" is defined by current Master Spec v02.139 requirements (not prior spec baselines).
- For each audited WP or subsystem: map MUST/SHOULD -> code evidence (path:line) + tests where feasible.
- If current spec moved, record delta and propose remediation stubs (do not rewrite history).

## INVENTORY (WHAT EXISTS)

This audit starts from the Operator-visible Task Board `## Done` list, then inspects each corresponding official task packet in `.GOV/task_packets/`.

<!-- WP_INVENTORY:BEGIN -->
| WP_ID | Packet | Packet **Status** | Has Verdict | Has PASS |
|---|---|---|---|---|
| WP-1-Spec-Router-SpecPromptCompiler-v1 | `.GOV/task_packets/WP-1-Spec-Router-SpecPromptCompiler-v1.md` | Validated (PASS) | YES | YES |
| WP-1-Front-End-Memory-System-v1 | `.GOV/task_packets/WP-1-Front-End-Memory-System-v1.md` | Validated (PASS) | YES | YES |
| WP-1-Unified-Tool-Surface-Contract-v1 | `.GOV/task_packets/WP-1-Unified-Tool-Surface-Contract-v1.md` | In Progress | YES | YES |
| WP-1-Lens-ViewMode-v1 | `.GOV/task_packets/WP-1-Lens-ViewMode-v1.md` | In Progress | YES | YES |
| WP-1-Cloud-Escalation-Consent-v2 | `.GOV/task_packets/WP-1-Cloud-Escalation-Consent-v2.md` | In Progress | YES | YES |
| WP-1-Autonomous-Governance-Protocol-v2 | `.GOV/task_packets/WP-1-Autonomous-Governance-Protocol-v2.md` | In Progress | YES | YES |
| WP-1-Model-Onboarding-ContextPacks-v1 | `.GOV/task_packets/WP-1-Model-Onboarding-ContextPacks-v1.md` | Done | YES | YES |
| WP-1-Spec-Enrichment-Product-Governance-Consistency-v1 | `.GOV/task_packets/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md` | Done | YES | YES |
| WP-1-LLM-Provider-Registry-v1 | `.GOV/task_packets/WP-1-LLM-Provider-Registry-v1.md` | Done | YES | YES |
| WP-1-Runtime-Governance-NoExpect-v1 | `.GOV/task_packets/WP-1-Runtime-Governance-NoExpect-v1.md` | Ready for Dev | YES | YES |
| WP-1-Product-Governance-Snapshot-v4 | `.GOV/task_packets/WP-1-Product-Governance-Snapshot-v4.md` | Done | YES | YES |
| WP-1-Flight-Recorder-v4 | `.GOV/task_packets/WP-1-Flight-Recorder-v4.md` | Done | YES | YES |
| WP-1-Supply-Chain-Cargo-Deny-Clean-v1 | `.GOV/task_packets/WP-1-Supply-Chain-Cargo-Deny-Clean-v1.md` | Done | YES | YES |
| WP-1-Artifact-System-Foundations-v1 | `.GOV/task_packets/WP-1-Artifact-System-Foundations-v1.md` | Done | YES | YES |
| WP-1-Model-Swap-Protocol-v1 | `.GOV/task_packets/WP-1-Model-Swap-Protocol-v1.md` | In Progress | YES | YES |
| WP-1-ModelSession-Core-Scheduler-v1 | `.GOV/task_packets/WP-1-ModelSession-Core-Scheduler-v1.md` | Done | YES | YES |
| WP-1-AI-UX-Summarize-Display-v2 | `.GOV/task_packets/WP-1-AI-UX-Summarize-Display-v2.md` | In Progress | YES | YES |
| WP-1-Atelier-Collaboration-Panel-v1 | `.GOV/task_packets/WP-1-Atelier-Collaboration-Panel-v1.md` | Validated (PASS) | YES | YES |
| WP-1-Response-Behavior-ANS-001 | `.GOV/task_packets/WP-1-Response-Behavior-ANS-001.md` | Done | YES | YES |
| WP-1-Global-Silent-Edit-Guard | `.GOV/task_packets/WP-1-Global-Silent-Edit-Guard.md` | In Progress | YES | YES |
| WP-1-AI-Ready-Data-Architecture-v1 | `.GOV/task_packets/WP-1-AI-Ready-Data-Architecture-v1.md` | Done | YES | YES |
| WP-1-Micro-Task-Executor-v1 | `.GOV/task_packets/WP-1-Micro-Task-Executor-v1.md` | Done | YES | YES |
| WP-1-AI-UX-Actions-v2 | `.GOV/task_packets/WP-1-AI-UX-Actions-v2.md` | Done | YES | YES |
| WP-1-Dev-Experience-ADRs-v1 | `.GOV/task_packets/WP-1-Dev-Experience-ADRs-v1.md` | Done | YES | YES |
| WP-1-Editor-Hardening-v2 | `.GOV/task_packets/WP-1-Editor-Hardening-v2.md` | Done | YES | YES |
| WP-1-Governance-Kernel-Conformance-v1 | `.GOV/task_packets/WP-1-Governance-Kernel-Conformance-v1.md` | Done | YES | YES |
| WP-1-Governance-Template-Volume-v1 | `.GOV/task_packets/WP-1-Governance-Template-Volume-v1.md` | Done | YES | YES |
| WP-1-Role-Mailbox-v1 | `.GOV/task_packets/WP-1-Role-Mailbox-v1.md` | Done | YES | YES |
| WP-1-Role-Registry-AppendOnly-v1 | `.GOV/task_packets/WP-1-Role-Registry-AppendOnly-v1.md` | Done | YES | YES |
| WP-1-Loom-MVP-v1 | `.GOV/task_packets/WP-1-Loom-MVP-v1.md` | Validated (PASS) | YES | YES |
| WP-1-Media-Downloader-v2 | `.GOV/task_packets/WP-1-Media-Downloader-v2.md` | Validated (PASS) | YES | YES |
| WP-1-Migration-Framework-v2 | `.GOV/task_packets/WP-1-Migration-Framework-v2.md` | Done | YES | YES |
| WP-1-ACE-Validators-v4 | `.GOV/task_packets/WP-1-ACE-Validators-v4.md` | In Progress | YES | YES |
| WP-1-LLM-Core-v3 | `.GOV/task_packets/WP-1-LLM-Core-v3.md` | Done | NO | NO |
| WP-1-OSS-Register-Enforcement-v1 | `.GOV/task_packets/WP-1-OSS-Register-Enforcement-v1.md` | Done | YES | YES |
| WP-1-Tokenization-Service-v3 | `.GOV/task_packets/WP-1-Tokenization-Service-v3.md` | Done | YES | YES |
| WP-1-Security-Gates-v3 | `.GOV/task_packets/WP-1-Security-Gates-v3.md` | Done | YES | YES |
| WP-1-Gate-Check-Tool-v2 | `.GOV/task_packets/WP-1-Gate-Check-Tool-v2.md` | Done | YES | YES |
| WP-1-Workflow-Engine-v4 | `.GOV/task_packets/WP-1-Workflow-Engine-v4.md` | Done | YES | YES |
| WP-1-Debug-Bundle-v3 | `.GOV/task_packets/WP-1-Debug-Bundle-v3.md` | Done | YES | YES |
| WP-1-Validator-Error-Codes-v1 | `.GOV/task_packets/WP-1-Validator-Error-Codes-v1.md` | Done | YES | YES |
| WP-1-Storage-Foundation-v3 | `.GOV/task_packets/WP-1-Storage-Foundation-v3.md` | Done | YES | YES |
| WP-1-Storage-Abstraction-Layer-v3 | `.GOV/task_packets/WP-1-Storage-Abstraction-Layer-v3.md` | Done | YES | YES |
| WP-1-AppState-Refactoring-v3 | `.GOV/task_packets/WP-1-AppState-Refactoring-v3.md` | Done | YES | YES |
| WP-1-Dual-Backend-Tests-v2 | `.GOV/task_packets/WP-1-Dual-Backend-Tests-v2.md` | Done | YES | YES |
| WP-1-Terminal-LAW-v3 | `.GOV/task_packets/WP-1-Terminal-LAW-v3.md` | Done | YES | YES |
| WP-1-MEX-v1.2-Runtime-v3 | `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v3.md` | In Progress | YES | YES |
| WP-1-Operator-Consoles-v3 | `.GOV/task_packets/WP-1-Operator-Consoles-v3.md` | In Progress | YES | YES |
| WP-1-Flight-Recorder-UI-v3 | `.GOV/task_packets/WP-1-Flight-Recorder-UI-v3.md` | Done | YES | YES |
| WP-1-Supply-Chain-MEX-v2 | `.GOV/task_packets/WP-1-Supply-Chain-MEX-v2.md` | Done | YES | YES |
| WP-1-Mutation-Traceability-v2 | `.GOV/task_packets/WP-1-Mutation-Traceability-v2.md` | Done | YES | YES |
| WP-1-Metrics-Mock-Tokens | `.GOV/task_packets/WP-1-Metrics-Mock-Tokens.md` | Done | YES | YES |
| WP-1-Canvas-Typography-v2 | `.GOV/task_packets/WP-1-Canvas-Typography-v2.md` | Done | YES | YES |
| WP-1-ACE-Runtime-v2 | `.GOV/task_packets/WP-1-ACE-Runtime-v2.md` | In Progress | YES | YES |
| WP-1-OSS-Governance-v2 | `.GOV/task_packets/WP-1-OSS-Governance-v2.md` | Done | YES | YES |
| WP-1-Capability-SSoT-v2 | `.GOV/task_packets/WP-1-Capability-SSoT-v2.md` | In Progress | YES | YES |
| WP-1-AI-Job-Model-v4 | `.GOV/task_packets/WP-1-AI-Job-Model-v4.md` | Done | YES | YES |
| WP-1-Cross-Tool-Interaction-Conformance-v1 | `.GOV/task_packets/WP-1-Cross-Tool-Interaction-Conformance-v1.md` | In Progress | YES | YES |
| WP-1-MCP-Skeleton-Gate-v2 | `.GOV/task_packets/WP-1-MCP-Skeleton-Gate-v2.md` | In Progress | YES | YES |
| WP-1-MCP-End-to-End-v2 | `.GOV/task_packets/WP-1-MCP-End-to-End-v2.md` | Done | YES | YES |
| WP-1-Session-Scoped-Capabilities-Consent-Gate-v1 | `.GOV/task_packets/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md` | Done | YES | YES |
<!-- WP_INVENTORY:END -->

## AUDIT RESULTS (CODE VS SPEC)

### Summary Table (by focus area)

| Focus Area | Spec Alignment (v02.139) | Test Evidence | Notes | Action |
|---|---|---|---|---|
| Calendar | FAIL (missing) | NOT_RUN | Calendar capability IDs exist, but no Calendar lens surface / storage / sync engine. | Create remediation stubs (Calendar v3 family). |
| PostgreSQL readiness | WARN (blocker present) | PARTIAL (sqlite tests + postgres CI matrix exists) | Postgres path exists, but MCP durable progress mapping is not implemented on Postgres (runtime protocol failure risk). Runtime schema drift outside migrations. | Create remediation stubs (MCP fields portability + schema migration cleanup). |
| Locus Phase 1 | WARN (partial impl) | NOT_RUN | SQLite tables + core ops exist; Spec Router + Micro-Task Executor integration is missing; medallion (Bronze/Silver) absent; query contract partial. | Split into targeted Locus remediation stubs (integration/query/medallion). |
| Loom + Media | WARN (vertical slice incomplete) | NOT_RUN | Loom import pipeline and Media Downloader exist, but Tier-1 preview for video poster frames is missing; no MD->Loom promotion bridge; ASR job kind not wired. | Keep existing validated WPs; create stubs for missing bridges/previews/ASR. |
| Micro-Task Executor | WARN (drift) | PASS (targeted tests exist) | Core loop/escalation/recovery are implemented and tested, but spec deltas exist (summaries, drop-back smart logic, LoRA wiring, blocked decisioning, caps, Locus wiring). | Create remediation stubs (do not reopen validated WP). |
| LLM-friendly data | WARN (partial) | NOT_RUN | Bronze/Silver/Gold pipeline exists, but CoreMetadata shape is incomplete, relationship_id missing, graph retrieval candidates stubbed, doc jobs don't persist ACE artifacts, FR lacks model_session_id column. | Create remediation stubs (metadata/graph/ACE persistence/FR model_session_id). |

### Detailed Findings

#### Calendar (current-spec missing)

- Spec requirements (v02.139):
  - Calendar lens surface and Calendar entities (CalendarEvent/CalendarSource) with persistence.
  - Calendar sync mechanical engine + policy integration + law compliance tests.
- Evidence (code inspection):
  - No calendar UI surface deps in `../handshake_main/app/package.json:15`.
  - No calendar storage tables in `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs:342`.
  - No mechanical engine registration in `../handshake_main/src/backend/handshake_core/mechanical_engines.json:1`.
  - Partial overlap exists via Timeline/Flight Recorder viewing primitives, but no Calendar-specific model.
- Verdict (current-spec): FAIL (missing deliverables).
- Action: Create remediation stubs (Calendar v3 family) rather than reopening the older Calendar stub.

#### PostgreSQL readiness (blockers + portability drift)

- Spec requirements (v02.139):
  - DB boundary through `Database` trait (no pool leakage), portable migrations, dual-backend CI, trait purity.
- Evidence (present / good):
  - `AppState` holds `Arc<dyn Database>`: `../handshake_main/src/backend/handshake_core/src/lib.rs:32`.
  - Backend chosen by `DATABASE_URL` prefix: `../handshake_main/src/backend/handshake_core/src/storage/mod.rs:1566`.
  - `sqlx` built with sqlite + postgres: `../handshake_main/src/backend/handshake_core/Cargo.toml:13`.
  - CI matrix includes sqlite + postgres: `../handshake_main/.github/workflows/ci.yml:100`.
  - Migration replay/down tests exist: `../handshake_main/src/backend/handshake_core/src/storage/tests.rs:1055`.
- Evidence (gaps / risks):
  - Hard Postgres blocker: MCP durable progress mapping is `NotImplemented` on Postgres:
    - Defaults: `../handshake_main/src/backend/handshake_core/src/storage/mod.rs:1766`.
    - MCP gate requires updating durable fields when job_id exists: `../handshake_main/src/backend/handshake_core/src/mcp/gate.rs:1489`.
    - SQLite implements via `ai_jobs.*` columns: `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs:4350`.
  - Schema drift outside migrations:
    - SQLite runtime schema mutation during run_migrations: `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs:806`.
    - MCP columns added via runtime ALTER TABLE on SQLite: `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs:770`.
    - Model session schema ensured at runtime (both backends): `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs:342`, `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs:67`.
  - Trait purity escape hatch via downcast:
    - `as_any()` in Database: `../handshake_main/src/backend/handshake_core/src/storage/mod.rs:1842`.
    - API uses downcast checks: `../handshake_main/src/backend/handshake_core/src/api/loom.rs:880`.
  - Locus is SQLite-only enforced at runtime:
    - `ensure_locus_sqlite`: `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs:15`.
    - Workflow requires SQLite: `../handshake_main/src/backend/handshake_core/src/workflows.rs:1188`.
- Verdict (current-spec): WARN (dual-backend posture exists but key Postgres path is incomplete; portability hygiene drift).
- Action: Create targeted remediation stubs (MCP durable fields portability + move runtime DDL into migrations + trait-purity cleanup).

#### Locus Phase 1 (partial implementation; missing integration + medallion)

- Spec requirements (v02.139 Phase 1 bullets):
  - SQLite tables + core operations + dependency/cycle detection + query contract.
  - Integration: Spec Router auto-invoke locus_create_wp; Micro-Task Executor auto updates Locus; auto task-board sync; Bronze/Silver/Gold integration.
- Evidence (present):
  - SQLite schema created: `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs:692`.
  - Core ops dispatcher exists (job_kind == LocusOperation): `../handshake_main/src/backend/handshake_core/src/workflows.rs:5063`.
  - Cycle detection: `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs:219`.
  - Task-board sync job exists (invoked manually): `../handshake_main/src/backend/handshake_core/src/workflows.rs:5070`.
- Evidence (missing / partial):
  - Query contract partial: current query supports only limit and returns wp_ids, not rich filtering/full objects:
    - Types: `../handshake_main/src/backend/handshake_core/src/locus/types.rs:425`.
    - Storage returns wp_ids: `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs:757`.
  - Spec Router integration absent: no locus_* calls in spec_router paths (`../handshake_main/src/backend/handshake_core/src/spec_router/`).
  - Micro-Task Executor integration absent: no locus_start/record/complete calls in MT executor loop.
  - Medallion objects absent: no WPBronze/WPSilver (search-confirmed by agent).
  - Flight Recorder payload conformance gap: spec requires vector clocks; emitted payload shape lacks them and validator disallows extra keys:
    - Emission: `../handshake_main/src/backend/handshake_core/src/workflows.rs:2052`.
    - Validator restrictions: `../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs:3198`.
- Verdict (current-spec): WARN (partial impl exists; spec deliverables missing).
- Action: Create split remediation stubs (integration+occupancy, medallion+search, query contract+autosync).

#### Micro-Task Executor (validated WP; current-spec drift)

- Evidence (present):
  - Dispatch path exists: `../handshake_main/src/backend/handshake_core/src/workflows.rs:4528`.
  - Job contract invariants + tests exist: `../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs:1090`.
  - Bounded loop + gates + recovery + event emission have coverage: `../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs:169`, `../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs:1007`.
- Drift against v02.139 (examples from agent):
  - MT summaries required but not generated:
    - `summary_ref` defined but never populated: `../handshake_main/src/backend/handshake_core/src/workflows.rs:6067`.
  - Drop-back "smart" logic not implemented:
    - Always drops after success: `../handshake_main/src/backend/handshake_core/src/workflows.rs:11585`.
  - LoRA selection is effectively no-op (request lacks lora field):
    - Request type: `../handshake_main/src/backend/handshake_core/src/llm/mod.rs:64`.
  - Resource exhaustion caps incomplete: `../handshake_main/src/backend/handshake_core/src/workflows.rs:5661`.
  - Locus integration missing in MT loop (spec expects it).
- Verdict (current-spec): OUTDATED_ONLY (do not reopen validated WP; create remediation stubs for deltas).
- Action: Create remediation stubs (summaries, drop-back smart, LoRA wiring, blocked decisioning, caps, Locus wiring, conformance tests).

#### Loom + Media (validated WPs; current-spec deltas)

- Evidence (present):
  - Loom import pipeline: `../handshake_main/src/backend/handshake_core/src/api/loom.rs:467`.
  - Loom views + filters exist: `../handshake_main/src/backend/handshake_core/src/api/loom.rs:713`.
  - Loom storage views exist (sqlite): `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs:2222`.
  - Media Downloader unified UI: `../handshake_main/app/src/components/MediaDownloaderView.tsx:395`.
  - Media Downloader runner supports youtube/generic: `../handshake_main/src/backend/handshake_core/src/workflows.rs:14929`.
- Gaps against v02.139:
  - Loom Tier-1 video preview missing (preview job hard-fails non-image MIME):
    - `unsupported_mime`: `../handshake_main/src/backend/handshake_core/src/workflows.rs:13510`.
  - ThumbnailSpec mismatch (current 256 PNG, spec prefers 512 WebP):
    - Current settings: `../handshake_main/src/backend/handshake_core/src/workflows.rs:13543`.
  - No Media Downloader -> Loom promotion bridge (large-file base64 import is not viable):
    - Loom import expects base64 bytes: `../handshake_main/src/backend/handshake_core/src/api/loom.rs:441`.
  - `asr_transcribe` job kind exists but runner not wired:
    - Job kind mapping exists: `../handshake_main/src/backend/handshake_core/src/capabilities.rs:184`.
    - No workflow runner: `../handshake_main/src/backend/handshake_core/src/workflows.rs:5047`.
  - Captions ingestion not populating Loom searchable layer:
    - Artifacts exist: `../handshake_main/src/backend/handshake_core/src/workflows.rs:20600`.
    - Loom storage derived/full_text not populated: `../handshake_main/src/backend/handshake_core/src/storage/loom.rs:121`.
- Verdict (current-spec): OUTDATED_ONLY (keep validated base WPs; fix via remediation stubs).

#### LLM-friendly data + auditability (validated WP; current-spec deltas)

- Evidence (present):
  - Bronze/Silver tables exist: `../handshake_main/src/backend/handshake_core/migrations/0012_ai_ready_data_arch.sql:22`.
  - Deterministic IDs: `../handshake_main/src/backend/handshake_core/src/ai_ready_data/pipeline.rs:1293`.
  - Shadow Workspace layout + tree-sitter chunking: `../handshake_main/src/backend/handshake_core/src/ai_ready_data/paths.rs:32`.
  - ACE QueryPlan/RetrievalTrace types exist: `../handshake_main/src/backend/handshake_core/src/ace/mod.rs:634`.
- Gaps against v02.139:
  - metadata_json does not conform to CoreMetadata required shape:
    - Current write site: `../handshake_main/src/backend/handshake_core/src/ai_ready_data/pipeline.rs:677`.
  - relationship_id missing; graph retrieval candidates stubbed:
    - GraphEdge has no id: `../handshake_main/src/backend/handshake_core/src/ai_ready_data/indexing.rs:79`.
    - retrieval sets empty graph_candidates: `../handshake_main/src/backend/handshake_core/src/ai_ready_data/pipeline.rs:1080`.
  - DocSummarize/DocEdit do not persist ACE artifacts (QueryPlan/Trace/ContextSnapshot refs null):
    - Null refs: `../handshake_main/src/backend/handshake_core/src/workflows.rs:4909`.
  - Flight Recorder lacks model_session_id at envelope/sink level:
    - Event type lacks field: `../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs:341`.
    - DuckDB schema lacks column: `../handshake_main/src/backend/handshake_core/src/flight_recorder/duckdb.rs:236`.
- Verdict (current-spec): OUTDATED_ONLY (keep validated base WPs; fix via remediation stubs).

## REMEDIATION STUBS (PROPOSED)

Create new remediation stubs (do not reopen validated WPs):

| Base WP ID | Stub ID | Spec Anchor(s) | Problem | Suggested Fix | Risk |
|---|---|---|---|---|---|
| WP-1-Calendar-Lens | WP-1-Calendar-Lens-v3 | Master Spec v02.139 Calendar lens sections | Calendar surface missing | Implement Calendar UI lens + model integration. | HIGH |
| WP-1-Calendar-Lens | WP-1-Calendar-Storage-v1 | Master Spec v02.139 Calendar entities | No CalendarEvent/CalendarSource storage | Add storage tables + DAL + tests. | HIGH |
| WP-1-Calendar-Lens | WP-1-Calendar-Sync-Engine-v1 | Master Spec v02.139 calendar_sync | No calendar mechanical engine | Implement sync engine entry + workflow runner. | HIGH |
| WP-1-Storage-Abstraction-Layer-v3 | WP-1-Postgres-MCP-Durable-Progress-v1 | Master Spec v02.139 MCP/tool gate durability | Postgres MCP durable progress mapping unimplemented | Portable side-table `ai_job_mcp_fields` + conformance tests. | HIGH |
| WP-1-Migration-Framework-v2 | WP-1-Storage-No-Runtime-DDL-v1 | Master Spec v02.139 portable migrations | Runtime DDL drift outside migrations | Move ensure_* schema into migrations; delete runtime ALTER/CREATE. | MED |
| WP-1-Storage-Abstraction-Layer-v3 | WP-1-Storage-Trait-Purity-v1 | Master Spec v02.139 trait purity | `as_any()` downcasts break purity | Add `backend_kind()`/capability methods; remove downcast use. | MED |
| WP-1-Locus-Work-Tracking-System-Phase1 | WP-1-Locus-Phase1-Integration-Occupancy-v1 | Master Spec v02.139 Locus integration | Missing Spec Router + MT executor wiring; occupancy missing | Add active_session_ids + bind/unbind ops + wire locus_* calls. | HIGH |
| WP-1-Locus-Work-Tracking-System-Phase1 | WP-1-Locus-Phase1-QueryContract-Autosync-v1 | Master Spec v02.139 query examples | Query shapes/filters partial; autosync missing | Align API output + add autosync on WP state changes. | MED |
| WP-1-Locus-Work-Tracking-System-Phase1 | WP-1-Locus-Phase1-Medallion-Search-v1 | Master Spec v02.139 WPBronze/WPSilver | Medallion objects absent | Implement Bronze/Silver generation + embeddings + keyword index. | HIGH |
| WP-1-Micro-Task-Executor-v1 | WP-1-MTE-Summaries-v1 | Master Spec v02.139 MT summaries | summaries not generated | Persist per-MT + aggregate summaries; populate summary_ref; tests. | MED |
| WP-1-Micro-Task-Executor-v1 | WP-1-MTE-DropBack-Smart-v1 | Master Spec v02.139 drop-back | smart drop-back not implemented | Implement ShouldDropBack logic + tests. | LOW |
| WP-1-Micro-Task-Executor-v1 | WP-1-MTE-LoRA-Wiring-v1 | Master Spec v02.139 LoRA selection | selection no-op | Add lora_id field in request and plumb through providers; tests. | MED |
| WP-1-Micro-Task-Executor-v1 | WP-1-MTE-Blocked-Decisioning-v1 | Master Spec v02.139 blocked retry | blocked always escalates | Implement recoverable blocked retry decision tree + tests. | MED |
| WP-1-Micro-Task-Executor-v1 | WP-1-MTE-Resource-Caps-v1 | Master Spec v02.139 caps | token/storage caps incomplete | Add max_total_tokens + enforce; storage size checks; tests. | MED |
| WP-1-Loom-MVP-v1 | WP-1-Loom-Preview-VideoPosterFrames-v1 | Master Spec v02.139 ThumbnailSpec video | video Tier-1 preview missing | Add poster-frame generation + spec-aligned format/dimensions. | HIGH |
| WP-1-Media-Downloader-v2 | WP-1-Media-Downloader-Loom-Bridge-v1 | Master Spec v02.139 MD output routing + Loom | no promotion path | Add idempotent promotion of outputs into Loom assets/blocks. | HIGH |
| WP-1-Video-Archive-Loom-Integration | WP-1-ASR-Transcribe-Media-v1 | Master Spec v02.139 ASR | ASR job not runnable | Wire `asr_transcribe` as workflow runner; artifacts + search integration. | MED |
| WP-1-AI-Ready-Data-Architecture-v1 | WP-1-AIReady-CoreMetadata-v1 | Master Spec v02.139 CoreMetadata | metadata incomplete | Versioned CoreMetadata subset in metadata_json + validation. | MED |
| WP-1-AI-Ready-Data-Architecture-v1 | WP-1-AIReady-RelationshipIds-GraphRetrieval-v1 | Master Spec v02.139 relationship_id | graph edges missing IDs, retrieval stubbed | Add relationship_id + implement graph candidates in hybrid retrieval. | MED |
| WP-1-ACE-Runtime-v2 | WP-1-ACE-Persist-QueryPlan-Trace-v1 | Master Spec v02.139 RetrievalTrace persistence | Doc jobs not persisting ACE artifacts | Persist QueryPlan/Trace/ContextSnapshot and link in job records. | MED |
| WP-1-Flight-Recorder-v4 | WP-1-FR-ModelSessionId-v1 | Master Spec v02.139 model session correlation | FR events missing model_session_id | Extend envelope + DuckDB schema; populate in session-scope emitters. | MED |

## COMMAND LOG

- `cd ../handshake_main/src/backend/handshake_core; cargo test -j 1 --lib` -> PASS (202 tests; warnings: dead_code)
- `cd ../handshake_main/src/backend/handshake_core; cargo test` -> FAIL (os error 1455: paging file too small). Use `-j 1` and run targeted suites.

## DECISIONS / NOTES

- Audit kickoff created on 2026-03-04; first pass should prioritize cross-cutting spec deltas (LLM-friendly data, Postgres readiness, calendar drift).
