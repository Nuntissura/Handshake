# TASK_PACKET_STUB_TEMPLATE
## MACHINE_CONTRACT
- AUTHORITATIVE_CONTRACT_SCHEMA_ID: hsk.work_packet_stub_contract@1
- AUTHORITATIVE_CONTRACT_FILE: .GOV/task_packets/stubs/WP-KERNEL-001-Event-Ledger-Session-Broker-v1.contract.json
- MARKDOWN_PROJECTION_FILE: .GOV/task_packets/stubs/WP-KERNEL-001-Event-Ledger-Session-Broker-v1.md
- MARKDOWN_PROJECTION_STATUS: GENERATED_IN_SYNC
- RED_TEAM_REQUIRED: YES
- RED_TEAM_PROFILE: DETERMINISTIC_CONTRACT_MIGRATION_V1
<!-- Assume stale projections, shadow prose authority, schema omissions, round-trip loss, lifecycle split drift, and role-duty divergence until machine checks prove otherwise. -->
- RULE: ACP, apps, and checks consume machine contracts when present; this stub Markdown is the human/operator projection.

This is a BACKLOG STUB. It is NOT an executable Work Packet.

## MACHINE_READABLE_GOVERNANCE_ARTIFACT_STANCE
- This stub inherits the Handshake Governance Pack stance from `.GOV/spec/SPEC_CURRENT.md` -> active indexed Master Spec module `07-user-experience-and-development.md` section `7.5.4.10.0`.
- For this stub, `.GOV/task_packets/stubs/WP-KERNEL-001-Event-Ledger-Session-Broker-v1.contract.json` is the machine-readable authority; this Markdown file is a projection/safety view.
- Future projects instantiated from Handshake governance MUST treat model-created Work Packets, Work Packet stubs, refinements, Micro-Tasks, Task Board state, validation records, workflow state, role/session state, receipts, handoffs, topology records, startup capsules, runtime dossiers, and governance registries as typed machine contracts first.
- Markdown created by models or tooling MAY exist only as an on-demand projection, generated report/dashboard, frozen legacy migration reference, or compatibility bridge with provenance to the machine contract.
- Existing Markdown-heavy artifacts in this repo are migration safety nets and must not be copied forward as the future pattern for Handshake-governed projects.
- Operator-created notes, research, and audits remain allowed prose artifacts, but they do not override machine governance contracts unless a contract explicitly imports them as evidence.

## STUB_EXECUTION_RULES
Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating this stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` or the current Kernel Builder activation rule approved by the Operator.
- The official packet must preserve the microtask boundaries or split them finer. Do not compress this bundle into broad implementation turns.
- This stub exists to make the Kernel V1 reset visible to Task Board, Build Order, and no-context models.

---

# Work Packet Stub: WP-KERNEL-001-Event-Ledger-Session-Broker-v1

## STUB_METADATA
- WP_ID: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- BASE_WP_ID: WP-KERNEL-001-Event-Ledger-Session-Broker
- CREATED_AT: 2026-05-13T14:19:26Z
- STUB_FORMAT_VERSION: 2026-04-06
- STUB_STATUS: SUPERSEDED_BY_OFFICIAL_PACKET (READY_FOR_DEV)
- ACTIVATED_PACKET_FILE: .GOV/task_packets/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/packet.md
- ACTIVATED_REFINEMENT_FILE: .GOV/task_packets/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/refinement.md
- ACTIVATION_STATE: OFFICIAL_PACKET_SIGNED_READY_FOR_DEV
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-ModelSession-Core-Scheduler, WP-1-Workflow-Engine, WP-1-Flight-Recorder, WP-1-Artifact-System-Foundations, WP-1-Unified-Tool-Surface-Contract, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Product-Governance-Check-Runner, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Session-Crash-Recovery-Checkpointing
- BUILD_ORDER_BLOCKS: WP-1-Postgres-Control-Plane-Shift-Bundle, WP-1-Software-Delivery-Runtime-Truth, WP-1-Workflow-Transition-Automation-Registry, WP-1-Dev-Command-Center-MVP, WP-1-FEMS-Working-Memory-Checkpoint-Schema, WP-1-Session-Spawn-Tree-DCC-Visualization, WP-1-Session-Spawn-Conversation-Distillation, WP-1-Visual-Debugging-Loop, WP-KERNEL-002-CRDT-Workspace-Promotion, WP-KERNEL-003-Sandbox-Validation-Promotion
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.5
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.4
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- PLANNED_EXECUTION_OWNER_RANGE: Coder-A..Coder-Z
- ROADMAP_POINTER: RESET_BRIEF HSK-KERNEL-001 Event Ledger + Session Broker Proof; topical anchors must come from indexed Master Spec modules 02, 03, 04, 05, 10, and 11.
- ROADMAP_ADD_COVERAGE: RESET_BRIEF=handshake-v2-kernel-reset-brief.md; HSK_KERNEL_ID=HSK-KERNEL-001; SPEC_MODULES=02,03,04,05,10,11
- RESET_BRIEF_TARGET: .GOV/operator/docs_local/handshake-v2-kernel-reset-brief.md
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - `.GOV/spec/master-spec-v02.183/spec-modules/02-system-architecture.md`: product-owned runtime architecture, storage authority, workflow/runtime state, and control-plane contracts.
  - `.GOV/spec/master-spec-v02.183/spec-modules/03-local-first-infrastructure.md`: local-first, CRDT, and durable sync posture; this WP only lays the event authority needed by later CRDT workspace work.
  - `.GOV/spec/master-spec-v02.183/spec-modules/04-llm-infrastructure.md`: LLM client/adapter boundary, local-first model posture, provider replaceability, and model run state.
  - `.GOV/spec/master-spec-v02.183/spec-modules/05-security-and-observability.md`: auditability, Flight Recorder, traceability, and evidence surfaces.
  - `.GOV/spec/master-spec-v02.183/spec-modules/10-product-surfaces.md`: Dev Command Center as a projection/control surface over runtime truth, not authority itself.
  - `.GOV/spec/master-spec-v02.183/spec-modules/11-shared-dev-platform-and-oss-foundations.md`: sandbox, tool/capability boundary, reproducible dev platform, and OSS/runtime constraints.

## INTENT (DRAFT)
- What: Build the first Kernel V1 proof as one bundled product WP: Postgres-backed EventLedger, SessionBroker, ContextBundle, replaceable dummy ModelAdapter, ToolGate ledger bridge, ArtifactProposal/ArtifactStore linkage, ValidationRunner, PromotionGate, and TraceProjection replay.
- Why: The reset brief's first useful slice is not a full IDE or UI. It is the deterministic kernel path where an operator task becomes durable typed state that can survive restart, be replayed, be validated, and be inspected by a no-context model.
- Why one WP: The old backlog splits the same substrate across storage, session scheduling, workflow transition, DCC, FEMS, artifact, and validation stubs. Activating those separately would repeat schema decisions and create drift before the kernel event authority exists.
- Build stance: reuse existing Handshake product code when it already implements a valid substrate. Replace only the parts that conflict with Kernel V1 authority, especially SQLite fallback/test authority and process-local scheduling claims.

## KERNEL_FIRST_ACCEPTANCE_SLICE
The official packet must prove this exact flow end to end:
1. Operator creates or imports a task intent.
2. Product assigns durable `KernelTaskRun` and `SessionRun` IDs.
3. `SessionBroker` dispatches a run to a local dummy/echo `ModelAdapter`.
4. `ContextBundle` records exactly what the adapter was allowed to see.
5. Adapter emits a visible response, a tool request, and an artifact proposal.
6. `ToolGate` records allow/deny decisions as ledger events.
7. `ArtifactStore` records output/log/evidence artifacts linked to ledger events.
8. `ValidationRunner` records pass/fail evidence for the proposed result.
9. `PromotionGate` records operator approve/reject and authority transition.
10. `TraceProjection` reconstructs the full run from the ledger after process restart.

## SOURCE_STUB_FOLD_MAP
This is the single activation target for the Kernel V1 first slice. The source stubs below remain historical/source material unless the Operator later splits this bundle again.

### Folded As Foundation
- `WP-1-Postgres-Control-Plane-Shift-Bundle-v1`: reuse Postgres-primary migration/dev-test thinking, lease/backpressure concerns, ModelSession queue shape, workflow durable-execution concerns, and DCC projection concerns. Override its SQLite cache/offline/test-fixture acceptance: Kernel V1 authority must not use SQLite.
- `WP-1-ModelSession-Core-Scheduler-v1`: reuse validated session identity, scheduler vocabulary, profile state, messages, and checkpoint concepts as product anchors.
- `WP-1-Workflow-Engine-v4`: reuse validated workflow run/node execution concepts, but move kernel run authority to typed ledger events instead of in-memory dispatcher state.
- `WP-1-Flight-Recorder-v4`: reuse diagnostic event vocabulary and evidence practice. Do not treat Flight Recorder or DuckDB as the authority ledger.
- `WP-1-Artifact-System-Foundations-v1`: reuse artifact metadata/path/evidence foundations for ledger-linked artifact proposals and outputs.
- `WP-1-Unified-Tool-Surface-Contract-v1` and `WP-1-Session-Scoped-Capabilities-Consent-Gate-v1`: reuse tool/capability gates for ToolGate decisions.
- `WP-1-Product-Governance-Check-Runner-v1`: reuse proof-command/check-run concepts for ValidationRunner.
- `WP-1-Dev-Command-Center-Control-Plane-Backend-v1`: reuse read-only projection discipline for TraceProjection and later DCC views.

### Partially Folded
- `WP-1-Software-Delivery-Runtime-Truth-v1`: adapt stable runtime truth and governed actions, but keep the first kernel proof product-native instead of repo-governance-overlay-first.
- `WP-1-Workflow-Transition-Automation-Registry-v1`: adapt transition-rule thinking for PromotionGate and state transitions only.
- `WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1`: adapt session lifecycle checkpoint vocabulary only; do not implement full FEMS memory in this first WP.
- `WP-1-Session-Spawn-Contract-v1`, `WP-1-Session-Crash-Recovery-Checkpointing-v1`, `WP-1-Session-Observability-Spans-FR-v1`, and `WP-1-Workspace-Safety-Parallel-Sessions-v1`: reuse as session safety context, not as separate activation blockers.

### Explicitly Deferred
- Full Dev Command Center MVP UI.
- Full CRDT workspace/promotion state. Reserve for `WP-KERNEL-002-CRDT-Workspace-Promotion`.
- Full sandboxed patch runner and container/VM execution. Reserve for `WP-KERNEL-003-Sandbox-Validation-Promotion`.
- Full local model memory/FEMS/personalization runtime. Reserve for `WP-KERNEL-004-Local-Model-Memory-Runtime`.
- Visual debugging/screenshot loop.
- Creative/media/calendar/document/lens/presentation runtime backfills.

## PRODUCT_CODE_REALITY_CHECK
Existing product code that should be reused:
- `src/backend/handshake_core/src/storage/mod.rs`: `ModelSession`, `SessionCheckpoint`, `SessionMessage`, workflow run APIs, storage capability labels, and storage initialization.
- `src/backend/handshake_core/src/storage/postgres.rs`: Postgres product storage backend and migration path.
- `src/backend/handshake_core/src/storage/sqlite.rs` and `src/backend/handshake_core/src/storage/locus_sqlite.rs`: legacy/local storage surfaces to audit and contain, not kernel authority.
- `src/backend/handshake_core/src/workflows.rs`: current model-run scheduler, session checkpoint/recovery, workflow run/node execution handling, and DCC control-plane snapshot.
- `src/backend/handshake_core/src/mcp/gate.rs`: capability-aware ToolGate substrate with session scoped grants, consent checks, path policy, and Flight Recorder events.
- `src/backend/handshake_core/src/llm/`: existing `LlmClient` trait, OpenAI-compatible/Ollama/disabled/in-memory adapters.
- `src/backend/handshake_core/src/flight_recorder/`: diagnostic recording substrate and evidence pattern.
- `src/backend/handshake_core/src/runtime_governance.rs`: governance check run concepts that can back ValidationRunner evidence.
- `app/package.json`: existing `yjs` dependency proves CRDT work has a starting point, but CRDT workspace state is deferred.

Product gaps this WP must close:
- No product `EventLedger` authority API/table exists yet.
- Current `FlightRecorder` records diagnostics but is not an append-only kernel authority ledger.
- Current model-run scheduling is process-local around queued jobs and a static dispatcher lock; it is not a durable Postgres claim/lease worker queue.
- Legacy storage may still expose names such as `SqliteCache`, `SqliteOffline`, or `Test`; Kernel V1 and Handshake product tests must reject SQLite for authority, cache, offline, fallback, fixture, compatibility, harness, example, temporary-adapter, and test roles.
- Generic structured-collaboration storage paths still route through `locus_sqlite` in some code paths; kernel work must add tripwires so this cannot drive kernel authority.
- There is no product `SessionBroker` abstraction with a durable event state machine.
- There is no ledger-recorded `ContextBundle` contract for what a model saw.
- There is no kernel `ModelAdapter` layer with a local dummy/echo proof and replaceability test.
- ToolGate records Flight Recorder events but does not yet write typed allow/deny ledger events.
- Artifact store foundations exist, but there is no ledger-linked `ArtifactProposal` to promotion path.
- Governance check runner exists, but there is no kernel `ValidationRunner` event contract.
- There is no `PromotionGate` event that is the sole authority transition point.
- DCC snapshot/projection exists, but no trace replay projection from the kernel ledger.

## SPEC_GAPS_AND_ENRICHMENT_DECISION
- ENRICHMENT_NEEDED_BEFORE_STUB_ACTIVATION: CHECK_DURING_REFINEMENT
- Current research did not find a better architecture that should replace the reset brief. The reset direction is field-aligned: product-owned durable event/checkpoint state, explicit human approval, gate evidence, and replayable traces.
- The better implementation detail found during research is narrower: Postgres notifications may wake workers, but the authority must stay in durable Postgres event/queue tables. Do not treat `LISTEN/NOTIFY`, framework traces, provider chat history, terminal transcripts, or Flight Recorder mirrors as authority.
- If topical Master Spec modules still authorize SQLite cache/offline/test-fixture behavior for Kernel V1 authority, activation must stop for minimal spec enrichment before USER_SIGNATURE.
- If topical Master Spec modules do not contain enough normative authority for EventLedger, PromotionGate, or TraceProjection as product-owned surfaces, activation must stop for minimal spec enrichment before USER_SIGNATURE.
- Do not patch the old monolithic spec. Any approved enrichment must patch the indexed modules resolved through `.GOV/spec/SPEC_CURRENT.md`, update manifest/index metadata, and run spec-current/spec-regression/spec-eof/gov-check.

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- RETRIEVED_AT: 2026-05-13T14:19:26Z
- TARGET_BUCKETS:
  - OFFICIAL_DOC
  - BIG_TECH_DOC
  - GITHUB_OSS_DOC
  - CURRENT_AGENT_FRAMEWORK_DOC
- SEARCH_SEEDS:
  - Postgres durable worker claims with `FOR UPDATE SKIP LOCKED`.
  - Postgres `LISTEN/NOTIFY` reliability boundaries.
  - LangGraph persistence, checkpointing, and human-in-the-loop interrupts.
  - OpenAI Agents SDK tracing, handoffs, and human approvals.
  - Temporal/Dapr durable execution and external approval events.
  - Yjs/Yrs/Loro/Automerge CRDT state/update patterns.
  - PGMQ Postgres queue extension tradeoffs.
- CANDIDATE_SOURCES:
  - Source: PostgreSQL SELECT documentation | Kind: OFFICIAL_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://www.postgresql.org/docs/current/sql-select.html | Why: row locking and `SKIP LOCKED` semantics for durable worker claims.
  - Source: PostgreSQL NOTIFY documentation | Kind: OFFICIAL_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://www.postgresql.org/docs/current/sql-notify.html | Why: notification semantics and why notifications are wakeups, not durable authority.
  - Source: LangGraph human-in-the-loop and persistence docs | Kind: OSS_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://docs.langchain.com/oss/python/langgraph/human-in-the-loop | Why: current agent runtimes persist checkpoints and pause for human approval.
  - Source: OpenAI Agents SDK docs | Kind: BIG_TECH_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://openai.github.io/openai-agents-python/ | Why: current agent frameworks expose tracing/handoffs/tool approval, but app authority still needs product storage.
  - Source: Temporal durable execution docs | Kind: BIG_TECH_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://docs.temporal.io/workflows | Why: durable execution, signals, and external human approval are established patterns.
  - Source: Dapr Workflow docs | Kind: OSS_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://docs.dapr.io/developing-applications/building-blocks/workflow/ | Why: durable task orchestration and wait-for-event patterns.
  - Source: Yjs documentation | Kind: OSS_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://docs.yjs.dev/ | Why: mature CRDT update/state-vector model for later workspace sync.
  - Source: Loro documentation | Kind: OSS_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://www.loro.dev/docs/ | Why: current Rust-friendly CRDT candidate for hierarchical/versioned data in a later kernel WP.
  - Source: Automerge documentation | Kind: OSS_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://automerge.org/docs/ | Why: mature CRDT sync/storage reference for later comparison.
  - Source: PGMQ GitHub repository | Kind: GITHUB_OSS_DOC | Date: current | Retrieved: 2026-05-13T14:19:26Z | URL: https://github.com/tembo-io/pgmq | Why: Postgres queue extension to track but not require for the first portable kernel WP.

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: PostgreSQL SELECT documentation | Pattern: durable claims from Postgres tables using transactionally guarded row selection/locking. | Why: Kernel authority stays in product-owned tables and survives process restart.
  - Source: LangGraph/Temporal/Dapr docs | Pattern: persisted execution checkpoints and explicit human approval points. | Why: matches reset requirement for restartable sessions and promotion gates.
  - Source: OpenAI Agents SDK docs | Pattern: traces/tool events/handoffs as inspectable execution surfaces. | Why: use the pattern, but store Handshake authority in product events instead of provider/runtime history.
- ADAPT:
  - Source: PostgreSQL NOTIFY documentation | Pattern: use `LISTEN/NOTIFY` only as an optional wakeup. | Why: notifications are not the ledger; missed/restarted workers must recover by polling durable rows.
  - Source: Yjs/Loro/Automerge docs | Pattern: CRDT updates and state vectors are current. | Why: keep CRDT choice for `WP-KERNEL-002`; this WP only creates event authority and promotion points needed by CRDT integration.
  - Source: PGMQ GitHub repository | Pattern: Postgres-backed queue extension. | Why: track as a later optimization only if operational setup accepts an extension; first WP should not require extra Postgres extensions.
- REJECT:
  - Framework-first kernel authority. | Why: Handshake must remain product-owned and inspectable without hidden framework state.
  - Provider chat history or terminal transcript as authority. | Why: reset requires no dependency on provider chat or terminal history.
  - SQLite cache/offline/fallback/test-fixture for Kernel V1 authority. | Why: reset brief explicitly forbids SQLite in any authority/cache/offline/fallback/test role for the kernel.
  - Flight Recorder/DuckDB as the authority ledger. | Why: Flight Recorder remains a diagnostic/evidence mirror; EventLedger is the source of truth.

## MICRO_TASK_PLAN (DRAFT MINIMUM DECOMPOSITION)
Activation must create official microtask files under the packet folder. Each MT below is deliberately small enough for a no-context coder model to implement and a validator to review independently. Split further if a single MT touches unrelated authority boundaries.

### MT-001 Reset-Authority-And-Code-Reality-Map
- Goal: produce a packet-local evidence map from reset brief, indexed Master Spec modules, and current product code.
- Owned files/modules: packet/refinement only; no product code.
- Dependencies: none.
- Implementation notes: list exact product paths and spec modules used; confirm no Product Reference as authority.
- Proof: inspection evidence in packet plus `just task-packet-stub-contracts --check` after activation contract generation.
- Risk if missed: coder implements from reset prose without product anchors.
- Validator focus: evidence completeness and no old monolithic spec reliance.

### MT-002 Kernel-Event-Taxonomy
- Goal: define typed event families for task, session, context, adapter, tool, artifact, validation, promotion, and trace replay.
- Owned files/modules: `src/backend/handshake_core/src/kernel/*` or equivalent new module; schema docs embedded in Rust types/tests.
- Dependencies: MT-001.
- Implementation notes: include event type, event version, aggregate ID, causal parent, actor, timestamp, payload hash, and source component.
- Proof: unit test for event serialization and stable event type names.
- Risk if missed: later MTs invent incompatible event payloads.
- Validator focus: event names are stable and no prose-only event authority.

### MT-003 Postgres-EventLedger-Migration
- Goal: add Postgres tables/indexes for append-only kernel events and event payload metadata.
- Owned files/modules: Postgres migrations and `storage/postgres.rs`.
- Dependencies: MT-002.
- Implementation notes: include monotonic sequence, UUID/event ID, aggregate type/id, idempotency key, event type/version, JSON payload, payload hash, actor/session IDs, and created timestamp.
- Proof: migration test or targeted Postgres proof that creates an empty DB and appends/reads an event.
- Risk if missed: ledger remains an in-memory or diagnostic-only concept.
- Validator focus: no SQLite migration or fallback path for kernel events.

### MT-004 EventLedger-Storage-API
- Goal: implement Rust API/trait for appending and querying kernel events through the Postgres backend.
- Owned files/modules: storage trait and Postgres implementation; optional `kernel/event_ledger.rs`.
- Dependencies: MT-003.
- Implementation notes: append must be transactional, idempotent when the same idempotency key repeats, and queryable by aggregate/run ID.
- Proof: unit/integration test for append, duplicate idempotency, and ordered replay query.
- Risk if missed: later broker/promotion code bypasses durable event authority.
- Validator focus: call sites cannot write authority events outside the API.

### MT-005 No-SQLite-Kernel-Authority-Guard
- Goal: fail closed if kernel event/session/promotion authority is attempted under SQLite modes.
- Owned files/modules: `storage/mod.rs`, SQLite storage implementations, tests.
- Dependencies: MT-004.
- Implementation notes: any legacy `SqliteCache`, `SqliteOffline`, or `Test` surface is migration/removal debt. Kernel authority APIs and tests must return explicit errors rather than accepting SQLite in any form.
- Proof: tests proving kernel authority calls fail in SQLite-backed modes.
- Risk if missed: split-brain kernel state survives the reset.
- Validator focus: no hidden test fixture writes through SQLite for kernel APIs.

### MT-006 Durable-KernelRun-Identifiers
- Goal: introduce durable IDs and state records for `KernelTaskRun` and `SessionRun`.
- Owned files/modules: kernel types, storage/Postgres tables, workflow/session integration points.
- Dependencies: MT-004.
- Implementation notes: IDs must be stable across restart and appear in every subsequent event.
- Proof: create/read test and event linkage test.
- Risk if missed: trace replay cannot tie events into one run.
- Validator focus: stable IDs, no timestamp-only or path-position identity.

### MT-007 SessionBroker-State-Machine
- Goal: define SessionBroker states and legal transitions from requested to dispatched, waiting_on_tool, waiting_on_validation, waiting_on_promotion, completed, rejected, failed, or cancelled.
- Owned files/modules: `kernel/session_broker.rs` or equivalent.
- Dependencies: MT-006.
- Implementation notes: transition attempts must append ledger events and reject illegal transitions.
- Proof: state-machine unit tests for allowed and rejected transitions.
- Risk if missed: promotion and validation transitions become ad hoc.
- Validator focus: no authority transition without event append.

### MT-008 Durable-Claim-And-Lease-Worker
- Goal: implement minimal Postgres claim/lease mechanics for broker work.
- Owned files/modules: Postgres migration/storage worker helper.
- Dependencies: MT-003, MT-007.
- Implementation notes: use durable rows and transactionally guarded claims; optional notifications are wakeups only.
- Proof: concurrent claim test proves one owner per work item and reclaim after expiry.
- Risk if missed: current process-local dispatcher remains the only scheduler.
- Validator focus: one-winner semantics and restart-safe polling.

### MT-009 ContextBundle-Contract
- Goal: build and persist a context bundle describing exactly what the adapter was allowed to see.
- Owned files/modules: kernel context bundle types/storage; existing session message integration.
- Dependencies: MT-006.
- Implementation notes: include task intent, spec/module anchors, product file anchors, capability grants, artifact refs, and redacted/excluded fields.
- Proof: serialization test and event linkage to `SessionRun`.
- Risk if missed: no-context inspection cannot reconstruct model inputs.
- Validator focus: context bundle is stored in product artifacts/ledger, not chat history.

### MT-010 Dummy-Echo-ModelAdapter
- Goal: add a local deterministic adapter that emits response text, a tool request, and an artifact proposal without provider dependency.
- Owned files/modules: `llm` or new `kernel/model_adapter.rs`; tests.
- Dependencies: MT-009.
- Implementation notes: keep adapter replaceable behind a trait; echo adapter is proof-only, not product intelligence.
- Proof: adapter test runs without network, provider credentials, or terminal history.
- Risk if missed: first kernel proof depends on external provider behavior.
- Validator focus: adapter replaceability and deterministic outputs.

### MT-011 Broker-Dispatch-To-Adapter
- Goal: connect SessionBroker to ContextBundle and dummy ModelAdapter with ledger events for requested, dispatched, adapter_started, adapter_response, and adapter_failed.
- Owned files/modules: broker module and workflow integration point.
- Dependencies: MT-007, MT-009, MT-010.
- Implementation notes: each phase records event IDs and keeps failure events inspectable.
- Proof: integration test dispatches one run and queries all expected events.
- Risk if missed: adapter activity remains invisible or transient.
- Validator focus: event order and restart-visible state.

### MT-012 Session-Messages-Ledger-Link
- Goal: link existing `SessionMessage` persistence to kernel ledger events.
- Owned files/modules: storage/session message code and broker dispatch.
- Dependencies: MT-011.
- Implementation notes: do not duplicate message truth; add event references/provenance where needed.
- Proof: response message has a ledger event ID and run ID.
- Risk if missed: messages and ledger drift.
- Validator focus: a no-context model can navigate from trace event to stored message.

### MT-013 ToolRequest-Event-Contract
- Goal: define typed tool request events emitted by the adapter.
- Owned files/modules: kernel event types and adapter output.
- Dependencies: MT-010, MT-011.
- Implementation notes: capture tool name, capability intent, args hash, redaction status, and requested actor/session.
- Proof: dummy adapter emits a valid tool request event.
- Risk if missed: ToolGate cannot be audited from the ledger.
- Validator focus: no raw sensitive args in event where redaction is required.

### MT-014 ToolGate-Ledger-Bridge
- Goal: bridge existing ToolGate decisions into kernel allow/deny events.
- Owned files/modules: `mcp/gate.rs` plus kernel ledger API calls.
- Dependencies: MT-013, MT-004.
- Implementation notes: preserve existing capability checks and Flight Recorder events; add ledger writes as authority evidence.
- Proof: allowed and denied tool requests each create typed ledger events.
- Risk if missed: tool execution authority is split between diagnostics and runtime state.
- Validator focus: denied requests are as visible as allowed requests.

### MT-015 ArtifactProposal-Contract
- Goal: define artifact proposal event/payload shape before artifact persistence or promotion.
- Owned files/modules: kernel artifact types and adapter output.
- Dependencies: MT-010, MT-004.
- Implementation notes: include artifact kind, intended target, content hash/ref, producer event ID, and whether operator promotion is required.
- Proof: dummy adapter emits an artifact proposal linked to a response event.
- Risk if missed: artifacts become outputs without explicit proposal authority.
- Validator focus: artifact proposals cannot silently modify authoritative state.

### MT-016 ArtifactStore-Ledger-Link
- Goal: store adapter output/log/evidence artifacts with ledger event provenance.
- Owned files/modules: artifact system foundations and kernel event integration.
- Dependencies: MT-015.
- Implementation notes: artifact write may create an event or record the event ID that authorized it.
- Proof: artifact lookup by run ID and event ID.
- Risk if missed: trace replay cannot recover evidence.
- Validator focus: artifact paths remain portable and outside repo-local build output where required.

### MT-017 ValidationRunner-Contract
- Goal: create a kernel validation runner event contract using existing product governance check runner concepts.
- Owned files/modules: kernel validation module, runtime governance/check run integration.
- Dependencies: MT-016.
- Implementation notes: validation records command/check ID, status, evidence artifact refs, stdout/stderr refs, and failure class.
- Proof: deterministic validation pass and fail fixture recorded as events.
- Risk if missed: promotion decisions lack machine-checkable evidence.
- Validator focus: validation is evidence, not final PASS authority.

### MT-018 PromotionGate-Contract
- Goal: implement the sole authority transition for approve/reject/promote decisions.
- Owned files/modules: kernel promotion module, storage API, tests.
- Dependencies: MT-017.
- Implementation notes: promotion must require prior artifact proposal and validation event unless explicitly marked operator override with reason.
- Proof: approve/reject tests and blocked promotion without validation.
- Risk if missed: authority changes happen without a promotion event.
- Validator focus: no authority transition outside `PromotionGate`.

### MT-019 TraceProjection-Replay
- Goal: implement a trace projection that reconstructs a run from ledger events and linked artifacts/messages.
- Owned files/modules: kernel trace projection module and optional backend command/API.
- Dependencies: MT-012, MT-014, MT-016, MT-018.
- Implementation notes: projection must not read provider chat history or terminal transcripts.
- Proof: replay after process restart reconstructs task, context bundle, adapter response, tool decision, artifact, validation, and promotion.
- Risk if missed: no-context inspection remains impossible.
- Validator focus: replay is ledger-driven and deterministic.

### MT-020 Minimal-DCC-Or-CLI-Inspector
- Goal: expose trace projection through the smallest existing product inspection path.
- Owned files/modules: backend command/API or DCC backend projection; no full UI.
- Dependencies: MT-019.
- Implementation notes: a CLI/test helper is acceptable if it returns structured JSON for no-context models.
- Proof: command/API returns a structured trace for a known run ID.
- Risk if missed: kernel proof exists but is not inspectable.
- Validator focus: structured output and stable fields.

### MT-021 Restart-Reconstruction-Proof
- Goal: prove reconstruction after process/storage restart.
- Owned files/modules: integration test harness and targeted proof command.
- Dependencies: MT-019, MT-020.
- Implementation notes: run proof must avoid relying on memory, provider chat, terminal scrollback, or non-durable fixtures.
- Proof: targeted test stops/reopens storage and reconstructs the trace.
- Risk if missed: reset's restartability goal is unproven.
- Validator focus: actual restart/reopen, not just replay in one process.

### MT-022 Adapter-Replaceability-Proof
- Goal: prove SessionBroker can swap dummy adapter for existing in-memory or disabled/client-compatible adapter without changing ledger contract.
- Owned files/modules: adapter trait/tests.
- Dependencies: MT-010, MT-011.
- Implementation notes: no external provider required; this is trait/contract proof.
- Proof: two adapters pass the same broker contract test.
- Risk if missed: first kernel hardcodes echo behavior.
- Validator focus: adapter boundary stability.

### MT-023 Cancellation-Backpressure-DeadLetter
- Goal: add minimal cancellation, pressure, retry, and terminal failure states for broker work.
- Owned files/modules: claim/lease schema and SessionBroker state machine.
- Dependencies: MT-008.
- Implementation notes: keep scope minimal; enough to prevent infinite loops and invisible stuck work.
- Proof: tests for cancelled, retry exhausted, dead-letter, and backpressure-visible queue state.
- Risk if missed: failed runs become stuck or silently retried.
- Validator focus: bounded retry and operator-visible stuck state.

### MT-024 FlightRecorder-Diagnostic-Mirror
- Goal: clarify and test that Flight Recorder mirrors key kernel events for diagnostics without becoming authority.
- Owned files/modules: Flight Recorder integration.
- Dependencies: MT-004, MT-011, MT-014, MT-018.
- Implementation notes: mirror event IDs into diagnostics; do not read diagnostics for authority decisions.
- Proof: diagnostic events contain kernel event IDs and ledger remains source of replay.
- Risk if missed: validator confuses diagnostic log with authority ledger.
- Validator focus: one-way mirror from ledger to diagnostics.

### MT-025 Product-SQLite-Leakage-Tripwire
- Goal: add targeted tests/search guards for kernel APIs accidentally using SQLite/locus_sqlite.
- Owned files/modules: tests and storage guard helpers.
- Dependencies: MT-005.
- Implementation notes: cover authority, cache, offline, fallback, and test fixture paths called out by reset brief.
- Proof: tests fail if kernel authority uses SQLite storage mode.
- Risk if missed: future MTs reintroduce SQLite authority while tests still pass.
- Validator focus: explicit failure messages and no broad unrelated refactor.

### MT-026 End-To-End-Kernel-Proof
- Goal: run the full Kernel V1 first-slice flow from task intent through replay.
- Owned files/modules: integration test/proof command and any fixture setup.
- Dependencies: MT-001 through MT-025.
- Implementation notes: use Postgres authority, dummy adapter, tool allow/deny, artifact proposal, validation result, promotion decision, and trace replay.
- Proof: one documented targeted command returns PASS or deterministic environment blocker.
- Risk if missed: individual pieces pass but the kernel is not proven.
- Validator focus: evidence covers every reset-brief component.

### MT-027 Validator-Handoff-And-Debt-Map
- Goal: record unresolved product/spec gaps, environment blockers, and follow-up kernel WPs after the first proof.
- Owned files/modules: packet closeout/handoff only.
- Dependencies: MT-026.
- Implementation notes: include explicit follow-up stubs for CRDT workspace, sandbox validation, and local model/memory runtime if still needed.
- Proof: handoff section cites evidence paths, commands, run IDs, and open risks.
- Risk if missed: next kernel session restarts discovery from scratch.
- Validator focus: no hidden debt and no self-issued final PASS.

## ACCEPTANCE_CRITERIA (DRAFT)
- Official activation creates this as one large WP with microtask files at least as fine-grained as the MT list above.
- Kernel EventLedger is Postgres-backed and is the authority for Kernel V1 run state.
- SQLite is rejected for kernel authority, cache, offline, fallback, and test-fixture roles.
- A local dummy/echo adapter proves the first flow without provider credentials or network.
- Tool decisions, artifact proposals, validation results, and promotion decisions are typed ledger events.
- No authority transition happens without a PromotionGate event.
- TraceProjection reconstructs the full run after restart from durable product state.
- A no-context model can inspect a run using stored events, context bundle, messages, artifacts, validation evidence, and trace projection.
- Product code changes stay in product worktrees/branches; `.GOV/` kernel planning stays in `wt-gov-kernel`.
- Any spec underspecification discovered during activation stops for minimal indexed-spec enrichment before signature.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Requires a working Postgres-primary development/test path or a deterministic environment-blocked proof.
- Requires current indexed Master Spec resolver to remain valid.
- Requires product code access from a product worktree for implementation; do not edit product code through `.GOV/` junctions or stale worktrees.
- Requires validator to judge product correctness; Kernel Builder self-tests are evidence only.

## RISKS / UNKNOWNs (DRAFT)
- Risk: the bundle is large enough for MT bleed. Mitigation: one MT per coder turn, explicit owned files, and per-MT validator review.
- Risk: legacy SQLite paths may still exist in product storage. Mitigation: land the No-SQLite kernel guard before broker/promotion work and prove no SQLite runtime, cache, fixture, fallback, compatibility, harness, example, temporary-adapter, or test path is accepted.
- Risk: in-process scheduler code is attractive to reuse incorrectly. Mitigation: reuse vocabulary/state only; add durable claim/lease for kernel work.
- Risk: DCC scope creep turns first kernel WP into a UI project. Mitigation: expose only structured trace projection or minimal backend inspector.
- Risk: Flight Recorder is mistaken for authority. Mitigation: event IDs mirror to FR, but ledger is read for replay and promotion.
- Risk: spec modules may not yet say EventLedger/PromotionGate strongly enough. Mitigation: activation must stop for minimal indexed-spec enrichment if topical authority is missing.
- Unknown: whether PGMQ or another Postgres queue extension should be adopted later. Current decision: do not require extensions for first kernel proof.
- Unknown: final CRDT library choice. Current decision: defer to `WP-KERNEL-002` after event/promotion authority exists.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [x] Confirm topical Master Spec anchors from indexed modules, not Product Reference and not the old monolithic spec.
- [x] Confirm the reset brief target is still `HSK-KERNEL-001`.
- [x] If research finds a better approach than the reset direction, perform minimal indexed-spec enrichment before packet signature.
- [x] Produce an in-chat Technical Refinement Block with source-stub fold map, MT list, spec anchors, product anchors, and out-of-scope list.
- [x] Obtain USER_SIGNATURE for the official WP.
- [x] Create the official packet and microtask files.
- [x] Ensure the official packet declares a product worktree/branch strategy before product code edits.
- [ ] Keep `.GOV/` changes on `gov_kernel`; keep product implementation commits off `gov_kernel`.
- [x] Move the Task Board item from STUB to Ready for Dev only after official activation.
