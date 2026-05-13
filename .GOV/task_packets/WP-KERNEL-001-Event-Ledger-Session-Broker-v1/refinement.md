<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.refinement_contract@1 source_file=.GOV/task_packets/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/refinement.json source_hash=ef96f50b1c6f056b projection_hash=b9281401d4409403 generated_at_utc=2026-05-13T19:45:38.368Z generator=master-spec-correction-sync.mjs -->
## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements:
- This refinement is blocked until the Operator provides the one-time activation signature.
- This refinement is also blocked until the indexed Master Spec contradiction around Kernel V1 storage authority is resolved or explicitly accepted by the Operator.
- This file is a no-context implementation/refinement record for `WP-KERNEL-001-Event-Ledger-Session-Broker-v1`.

### METADATA
- WP_ID: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- BASE_WP_ID: WP-KERNEL-001-Event-Ledger-Session-Broker
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: KERNEL_BUILDER_HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-05-13T14:50:53Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> .GOV/spec/master-spec-v02.183/indexed-spec-manifest.json (v02.183)
- USER_REVIEW_STATUS: PENDING_OPERATOR_SIGNATURE
- USER_SIGNATURE: PENDING_OPERATOR_SIGNATURE
- USER_APPROVAL_EVIDENCE: Operator requested creation of kernel packet/refinement/microtasks on 2026-05-13; activation signature still required.
- STUB_WP_IDS: WP-KERNEL-001-Event-Ledger-Session-Broker-v1, WP-1-Postgres-Control-Plane-Shift-Bundle-v1, WP-1-Postgres-Dev-Test-Container-Matrix-v1, WP-1-Postgres-Control-Plane-Leases-Backpressure-v1, WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-FEMS-Postgres-Memory-Store-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
- ENRICHMENT_NEEDED: YES
- CLEARLY_COVERS_VERDICT: PARTIAL_PENDING_KERNEL_SPEC_ENRICHMENT
- SPEC_IMPACT: YES

### GAPS_IDENTIFIED
- Current product code has Postgres storage and storage-mode labels, but no Postgres-backed kernel EventLedger.
- Current workflow/model session execution uses a process-local scheduler lock for dispatch, not durable product-owned claim/lease semantics.
- Current Flight Recorder and DCC surfaces are useful diagnostics/projections, but neither is the product authority ledger.
- Current Master Spec modules still contain SQLite-primary and SQLite-durable-execution language that conflicts with the reset brief's Kernel V1 no-SQLite rule.
- The old Postgres bundle stub is too broad for the first reset proof because it includes full FEMS memory store and full DCC projection scope; Kernel-001 must preserve those residuals without letting them block the first event-ledger proof.

### LANDSCAPE_SCAN
- TIMEBOX: current-source scan and local code inspection on 2026-05-13.
- SEARCH_SCOPE: PostgreSQL row locking and LISTEN/NOTIFY docs, LangGraph persistence docs, OpenAI Agents SDK human-in-loop/tracing docs, Temporal durable execution docs, Dapr workflow docs, PGMQ GitHub docs, current Handshake product code in `../handshake_main`.
- PRODUCT_CODE_SURFACES: storage modes and Postgres backend, ModelSession/SessionCheckpoint/SessionMessage, process-local session scheduler lock, ToolGate, Flight Recorder, DCC snapshot, LLM client trait, runtime governance check runner.
- PATTERNS_EXTRACTED: product-owned durable state, typed event ledger, persisted checkpoints, explicit human approval gates, trace projection, durable worker claims, and provider/framework traces as diagnostics only.
- DECISIONS_ADOPT: Postgres event/queue tables as authority; deterministic dummy adapter for first proof; typed promotion event for authority transition; ledger-driven trace replay.
- DECISIONS_ADAPT: Postgres notifications only as wakeups; framework traces only as observability references; CRDT library choice deferred until event/promotion authority exists.
- DECISIONS_REJECT: SQLite as Kernel V1 authority/cache/offline/fallback/test fixture; provider chat history as authority; Flight Recorder as authority; framework-first kernel ownership.

### RESEARCH_CURRENCY
- RESEARCH_CURRENCY_REQUIRED: YES
- SOURCE_MAX_AGE_DAYS: 30 where current docs are available; official evergreen docs accepted when the page is the current version.
- SOURCE_LOG:
  - Source: PostgreSQL SELECT documentation | Kind: OFFICIAL_DOC | Retrieved: 2026-05-13 | URL: https://www.postgresql.org/docs/current/sql-select.html | Why: `FOR UPDATE SKIP LOCKED` row locking supports multi-consumer queue-like claims while warning that skipped rows create an inconsistent view.
  - Source: PostgreSQL LISTEN documentation | Kind: OFFICIAL_DOC | Retrieved: 2026-05-13 | URL: https://www.postgresql.org/docs/current/sql-listen.html | Why: listeners receive notifications only while registered; registrations clear when sessions end, so notifications are wakeups not authority.
  - Source: LangGraph persistence docs | Kind: OSS_DOC | Retrieved: 2026-05-13 | URL: https://langchain-5e9cc07a.mintlify.app/oss/python/langgraph/persistence | Why: current agent runtimes persist checkpoints for human-in-loop, replay, and fault tolerance.
  - Source: OpenAI Agents SDK human-in-loop docs | Kind: BIG_TECH_DOC | Retrieved: 2026-05-13 | URL: https://openai.github.io/openai-agents-python/human_in_the_loop/ | Why: approval interruptions can be serialized and resumed, supporting explicit approval state.
  - Source: OpenAI Agents SDK tracing docs | Kind: BIG_TECH_DOC | Retrieved: 2026-05-13 | URL: https://openai.github.io/openai-agents-js/guides/tracing/ | Why: tracing records LLM generations, tool calls, handoffs, guardrails, and custom events but remains observability.
  - Source: Temporal durable execution overview | Kind: BIG_TECH_DOC | Retrieved: 2026-05-13 | URL: https://temporal.io/ | Why: durable execution persists workflow state, retries, task queues, signals, timers, and visibility.
  - Source: Dapr Workflow overview | Kind: OSS_DOC | Retrieved: 2026-05-13 | URL: https://docs.dapr.io/developing-applications/building-blocks/workflow/workflow-overview/ | Why: durable workflow APIs support start/query/pause/resume/raise-event/terminate/purge.
  - Source: PGMQ GitHub repository | Kind: GITHUB_OSS_DOC | Retrieved: 2026-05-13 | URL: https://github.com/pgmq/pgmq | Why: Postgres queue extension is current and useful to track, but extension dependency is rejected for first proof portability.

### RESEARCH_DEPTH
- ADOPT_PATTERNS:
  - Durable Postgres tables and transactionally guarded claims for queue-like broker work.
  - Persisted checkpoint/state history for replay, restart recovery, and human approval.
  - Structured trace/event output for inspection by no-context models.
- ADAPT_PATTERNS:
  - Postgres `LISTEN/NOTIFY` as optional wakeup only; worker recovery must poll durable rows.
  - Provider/framework tracing as an observability mirror only; product EventLedger is authority.
  - CRDT state-vectors/updates later, after this WP creates promotion authority.
- REJECT_PATTERNS:
  - Framework-owned state as the Handshake kernel.
  - Raw terminal transcript or provider chat as replay source.
  - SQLite for any Kernel V1 authority/cache/offline/fallback/test role.
- RESEARCH_DEPTH_VERDICT: PASS

### SOURCE_STUB_COVERAGE_DECISION
- `WP-1-Postgres-Control-Plane-Shift-Bundle-v1` is superseded as the first activation vehicle because the kernel reset narrows the immediate proof to EventLedger + SessionBroker authority.
- Its source stubs were inspected. Kernel-relevant parts are moved into MT-001 through MT-027.
- Full FEMS memory-store scope is not implemented by Kernel-001 and must stay preserved for `WP-KERNEL-004-Local-Model-Memory-Runtime` plus existing FEMS stubs.
- Full DCC UI/projection scope is not implemented by Kernel-001 and remains in active DCC stubs. Kernel-001 only exposes a structured trace inspector.
- Generic workflow-engine migration is only implemented as far as needed for KernelTaskRun/SessionRun durability. Full workflow transition automation remains downstream.

### SPEC_EXCERPTS
- Module 02 evidence: `.GOV/spec/master-spec-v02.183/spec-modules/02-system-architecture.md` includes the current Postgres bundle scope near lines 2642-2648, but also contains old SQLite durable-execution language near lines 8221, 8328, 8368, and 8775. Context tokens: `PostgreSQL-primary`, `SQLite`, `durable execution`.
- Module 03 evidence: `.GOV/spec/master-spec-v02.183/spec-modules/03-local-first-infrastructure.md` contains CRDT local-first patterns and SQLite query/index language near lines 411-636. Context tokens: `CRDT`, `SQLite`, `Postgres`.
- Module 04 evidence: `.GOV/spec/master-spec-v02.183/spec-modules/04-llm-infrastructure.md` contains model runtime and Flight Recorder correlation requirements, but does not define a product-owned Kernel EventLedger or replaceable kernel ModelAdapter boundary. Context tokens: `Flight Recorder`, `model`, `trace_id`.
- Module 05 evidence: `.GOV/spec/master-spec-v02.183/spec-modules/05-security-and-observability.md` contains append-only Flight Recorder and security/observability language; Kernel-001 must clarify that Flight Recorder is a diagnostic mirror, not authority. Context tokens: `Flight Recorder`, `append-only`, `observability`.
- Module 10 evidence: `.GOV/spec/master-spec-v02.183/spec-modules/10-product-surfaces.md` contains DCC/terminal/editor diagnostic projection requirements; Kernel-001 uses only a minimal structured trace inspector. Context tokens: `Flight Recorder`, `Diagnostics`, `DCC`.

### PROPOSED_SPEC_ENRICHMENT
Add the following minimal indexed-spec enrichment to the owning topical modules before coder launch unless the Operator explicitly accepts the reset brief as sufficient authority:

```markdown
#### Kernel V1 Authority State (ADD v02.183)

Kernel V1 runtime authority MUST be product-owned durable state, not provider chat history, terminal transcripts, repo-governance artifacts, or diagnostic mirrors.

The first kernel implementation MUST provide:
- A Postgres-backed append-only EventLedger for kernel task/session/tool/artifact/validation/promotion events.
- Stable `KernelTaskRun` and `SessionRun` identifiers that survive process restart.
- A SessionBroker state machine whose legal transitions append typed ledger events.
- A ContextBundle record for the exact input context exposed to a model adapter.
- A replaceable ModelAdapter boundary with a deterministic local dummy adapter for the first proof.
- ToolGate, ArtifactProposal, ValidationRunner, and PromotionGate events linked to the same run IDs.
- A TraceProjection that reconstructs a run from durable product state after restart.

Kernel V1 MUST NOT use SQLite for authority, cache, offline mode, compatibility mode, local fallback, bootstrap convenience, or test fixtures. Existing SQLite-backed product surfaces are legacy surfaces to contain or migrate when they intersect kernel authority. SQLite MAY remain in unrelated legacy product surfaces until their owning packets migrate them, but it MUST NOT drive kernel scheduling, promotion, replay, or validation authority.

Flight Recorder, DCC projections, provider traces, and generated Markdown are projections or diagnostics unless they explicitly reference EventLedger event IDs. They MUST NOT replace EventLedger as replay or promotion authority.
```

### MICRO_TASK_PLAN
- MT-001 through MT-027 are required. They may be split finer, but they must not be compressed into broader coding turns.
- MT-005 must land before broker/promotion implementation that could accidentally use SQLite.
- MT-020 must stay backend/CLI/API structured inspector only unless a later signed packet authorizes DCC UI work.
- MT-027 must preserve residual FEMS, DCC, workflow, CRDT, sandbox, and visual-debugging scope so no old stub scope disappears.

### ACCEPTANCE_CRITERIA
- Signature and spec-enrichment blockers are resolved before coder launch.
- Each MT has explicit owned files, dependencies, acceptance criteria, verification, and validator focus.
- The old Postgres bundle and folded source stubs are traceable to Kernel-001 or named residual follow-up scopes.
- Product code implementation produces durable EventLedger, SessionBroker, ContextBundle, dummy adapter, ToolGate bridge, artifact linkage, ValidationRunner, PromotionGate, TraceProjection, restart proof, and no-SQLite tripwires.

### RED_TEAM_ADVISORY
- Risk: old broad bundle semantics disappear during supersession. Mitigation: packet and MT-027 record residual scopes and follow-up owners.
- Risk: current spec contradiction lets coder choose SQLite. Mitigation: spec enrichment blocker and MT-005/MT-025 tripwires.
- Risk: no-context model treats diagnostics as authority. Mitigation: repeated packet/refinement/MT language says EventLedger is authority.
- Risk: huge WP overwhelms a single coder. Mitigation: one MT per turn and per-MT review.
