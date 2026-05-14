<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.work_packet_contract@1 source_file=.GOV/task_packets/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/packet.json source_hash=0c41b167950aaf67 projection_hash=58669ca77e969e29 generated_at_utc=2026-05-14T00:07:20.330Z generator=ensure-wp-communications.mjs -->
# TASK_PACKET_TEMPLATE

This is an official product Work Packet projection. It is blocked until the pending operator signature and spec-enrichment blocker are resolved.

---

# Task Packet: WP-KERNEL-001-Event-Ledger-Session-Broker-v1

## METADATA
- TASK_ID: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- WP_ID: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- BASE_WP_ID: WP-KERNEL-001-Event-Ledger-Session-Broker
- DATE: 2026-05-13T14:50:53Z
- REQUESTOR: Operator
- AGENT_ID: Kernel Builder
- ROLE: Kernel Builder
- ROLE_CONSOLIDATION_MODE: KERNEL_BUILDER_CONSOLIDATED_PRE_INTEGRATION
- KERNEL_BUILDER_CONSOLIDATED_ROLE_PROFILE: OPENAI_GPT_5_5_XHIGH
- INTEGRATION_VALIDATOR_SESSION_POLICY: SEPARATE_PASS_SESSION
- INTEGRATION_VALIDATOR_LAUNCH_STATE: DEFERRED_NOT_STARTED
- REFINEMENT_ENFORCEMENT_PROFILE: KERNEL_BUILDER_ACTIVATION_MODE_V1
- PACKET_HYDRATION_PROFILE: KERNEL_BUILDER_ACTIVATION_MODE_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- EXECUTION_OWNER: CODER_A
- WORKFLOW_AUTHORITY: ORCHESTRATOR
- TECHNICAL_ADVISOR: NONE
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
- VALIDATION_TOPOLOGY: INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1
- PER_MT_WP_VALIDATOR_REVIEW: DISABLED
- MT_REVIEW_AUTHORITY: INTEGRATION_VALIDATOR_BATCH
- SPEC_REVIEW_AUTHORITY: INTEGRATION_VALIDATOR_SCOPED_MASTER_SPEC
- KERNEL_BUILDER_RECEIPT_COMPAT_ROLE: CODER
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL: gpt-5.5
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL: gpt-5.5
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- INTEGRATION_VALIDATOR_MODEL: gpt-5.5
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
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
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-session-broker-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- GOVERNED_VALIDATOR_DUAL_TRACK_FIELDS: MECHANICAL_TRACK_VERDICT | SPEC_RETENTION_TRACK_VERDICT
- GOVERNED_VALIDATOR_COMPLETION_FIELDS: WORKFLOW_VALIDITY | SCOPE_VALIDITY | PROOF_COMPLETENESS | INTEGRATION_READINESS | DOMAIN_GOAL_COMPLETION
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1
- TOUCHED_FILE_BUDGET: 15
- BROAD_TOOL_ALLOWLIST: NONE
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** In Progress
- CURRENT_WP_STATUS: READY_FOR_DEV
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja140520260015
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- MAIN_CONTAINMENT_STATUS: NOT_STARTED
- MERGED_MAIN_COMMIT: NONE
- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A
- PACKET_WIDENING_DECISION: NONE
- PACKET_WIDENING_EVIDENCE: N/A
- ZERO_DELTA_PROOF_ALLOWED: NO
- RISK_TIER: HIGH
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-ModelSession-Core-Scheduler, WP-1-Workflow-Engine, WP-1-Flight-Recorder, WP-1-Artifact-System-Foundations, WP-1-Unified-Tool-Surface-Contract, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Product-Governance-Check-Runner, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Session-Crash-Recovery-Checkpointing
- BUILD_ORDER_BLOCKS: WP-1-Software-Delivery-Runtime-Truth, WP-1-Workflow-Transition-Automation-Registry, WP-1-Dev-Command-Center-MVP, WP-1-FEMS-Working-Memory-Checkpoint-Schema, WP-1-Session-Spawn-Tree-DCC-Visualization, WP-1-Session-Spawn-Conversation-Distillation, WP-1-Visual-Debugging-Loop, WP-KERNEL-002-CRDT-Workspace-Promotion, WP-KERNEL-003-Sandbox-Validation-Promotion, WP-KERNEL-004-Local-Model-Memory-Runtime
- STUB_WP_IDS: WP-KERNEL-001-Event-Ledger-Session-Broker-v1, WP-1-Postgres-Control-Plane-Shift-Bundle-v1, WP-1-Postgres-Dev-Test-Container-Matrix-v1, WP-1-Postgres-Control-Plane-Leases-Backpressure-v1, WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-FEMS-Postgres-Memory-Store-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> .GOV/spec/master-spec-v02.184/indexed-spec-manifest.json
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.184/spec-modules/02-system-architecture.md, .GOV/spec/master-spec-v02.184/spec-modules/03-local-first-infrastructure.md, .GOV/spec/master-spec-v02.184/spec-modules/04-llm-infrastructure.md, .GOV/spec/master-spec-v02.184/spec-modules/05-security-and-observability.md, .GOV/spec/master-spec-v02.184/spec-modules/10-product-surfaces.md, .GOV/spec/master-spec-v02.184/spec-modules/11-shared-dev-platform-and-oss-foundations.md
- IN_SCOPE_PATHS: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/mod.rs, src/backend/handshake_core/src/storage/postgres.rs, src/backend/handshake_core/migrations/**, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/mcp/gate.rs, src/backend/handshake_core/src/llm/**, src/backend/handshake_core/src/flight_recorder/**, src/backend/handshake_core/src/runtime_governance.rs, src/backend/handshake_core/tests/**, app/src-tauri/src/**, app/package.json
- OUT_OF_SCOPE: full Dev Command Center UI, full CRDT workspace implementation, sandboxed patch runner, full FEMS/local memory runtime, creative/media/calendar/document/lens/presentation runtime backfills, repo-governance implementation changes outside this packet's governance artifacts
- LOCAL_BRANCH: feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- LOCAL_WORKTREE_DIR: ../wtc-session-broker-v1
- REMOTE_BACKUP_BRANCH: feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- REMOTE_BACKUP_LIFECYCLE: TEMPORARY
- BACKUP_PUSH_STATUS: PUSHED_TO_ORIGIN
- HEARTBEAT_INTERVAL_MINUTES: 15
- STALE_AFTER_MINUTES: 45
- MAX_CODER_REVISION_CYCLES: 3
- MAX_VALIDATOR_REVIEW_CYCLES: 3
- MAX_RELAY_ESCALATION_CYCLES: 2
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
- COMMUNICATION_HEALTH_GATE: INTEGRATION_BATCH_REVIEW_BLOCKING
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: NONE
- INTEGRATION_VALIDATOR_OF_RECORD: INTEGRATION_VALIDATOR:WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: Implementation is in progress; awaiting coder handoff to WP validator.
Next: CODER completes in-scope work and records CODER_HANDOFF with proof.
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Kernel V1 product authority is a Postgres EventLedger and must not use SQLite authority, cache, offline, fallback, or test authority for the first kernel slice | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/postgres.rs, src/backend/handshake_core/migrations/** | TESTS: cargo test -p handshake_core kernel_event_ledger --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: EventLedger row links KernelTaskRun, SessionRun, event_type, actor, causation_id, correlation_id, payload, created_at | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: SessionBroker state must be durable, replayable, claim-safe, cancellable, and restart-reconstructable from the ledger | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/storage/mod.rs | TESTS: cargo test -p handshake_core kernel_session_broker --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: one SessionRun transitions queued, claimed, running, completed, cancelled, retryable, dead_lettered without process-local authority | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: ContextBundle and replaceable ModelAdapter must record the exact context allowed to a dummy/echo local adapter without binding kernel semantics to a provider trace | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/llm/**, src/backend/handshake_core/src/storage/mod.rs | TESTS: cargo test -p handshake_core kernel_context_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: dummy adapter emits response, tool request, and artifact proposal linked to a ContextBundle hash | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: ToolGate, ArtifactStore, ValidationRunner, and PromotionGate decisions must be ledger-linked and operator-reviewable before authority transition | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/mcp/gate.rs, src/backend/handshake_core/src/runtime_governance.rs, src/backend/handshake_core/src/flight_recorder/** | TESTS: cargo test -p handshake_core kernel_gate_artifact_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: allow or deny tool request, artifact proposal, validation pass or fail, and approve or reject promotion all share ledger correlation | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: TraceProjection must reconstruct the complete proof run after restart from product authority, with Flight Recorder only mirroring diagnostics | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/flight_recorder/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_trace_projection_restart --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: replay shows task intent, context, adapter output, tool decision, artifact evidence, validation evidence, and promotion decision after process restart | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING

## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.

## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/migrations/**
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/src/runtime_governance.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test -p handshake_core kernel_event_ledger --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_session_broker --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_trace_projection_restart --target-dir ../Handshake_Artifacts/handshake-cargo-target
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.

## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - Verify SQLite authority is rejected for every Kernel V1 EventLedger write path while Postgres authority records the same task/session event chain.
  - Kill and restart the kernel proof process after dummy adapter output and prove TraceProjection reconstructs the run from Postgres ledger rows, not Flight Recorder or memory.
  - Swap the dummy/echo adapter implementation behind the ModelAdapter boundary and prove ledger, ContextBundle, ToolGate, ArtifactStore, ValidationRunner, and PromotionGate semantics stay unchanged.
- CANONICAL_CONTRACT_EXAMPLES:
  - KernelTaskRun KTR-EXAMPLE with SessionRun SR-EXAMPLE emits TASK_INTENT_RECORDED, SESSION_CLAIMED, CONTEXT_BUNDLE_RECORDED, MODEL_RESPONSE_RECORDED, TOOL_DECISION_RECORDED, ARTIFACT_PROPOSED, VALIDATION_RECORDED, and PROMOTION_DECIDED events.
  - PromotionGate approve moves an artifact proposal to promoted only after validation evidence and operator approval are ledger-linked; reject records a terminal decision without deleting evidence.
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical contract example.

## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: KERNEL001 touches product-owned durable runtime authority, typed event storage, model/session/gate/promotion events, and trace reconstruction surfaces, so the LLM-first data contract is active.
- EVIDENCE:
  - IN_SCOPE_PATH: src/backend/handshake_core/src/kernel/** (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/postgres.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/migrations/** (migration/sql surface)
  - SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> .GOV/spec/master-spec-v02.184/indexed-spec-manifest.json

## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/migrations/**
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/llm/**
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/tests/**
- DATA_CONTRACT_RULES:
  - Kernel V1 runtime truth must be reconstructable from product-owned Postgres EventLedger events.
  - SQLite must not be used as Kernel V1 authority, cache, offline, fallback, or test fixture.
  - ContextBundle, ModelAdapter, ToolGate, ArtifactProposal, ValidationRunner, PromotionGate, and TraceProjection records must preserve stable ids, causation/correlation links, provenance, and parseable typed payloads.
  - Flight Recorder and Dev Command Center diagnostics are projections unless linked to durable EventLedger event ids.
- VALIDATOR_DATA_PROOF_HINTS:
  - Prove the Postgres EventLedger schema and migrations support the KERNEL001 event chain with stable ids, actor, causation, correlation, payload, and timestamp fields.
  - Prove SessionBroker replay, claim, cancellation, retry, and restart reconstruction are driven by EventLedger state rather than process memory or SQLite.
  - Prove TraceProjection can reconstruct a complete dummy-adapter proof run after restart from product authority and ledger-linked gate/artifact/validation/promotion evidence.

## WHAT
Build the first Kernel V1 product proof as one large but MT-sliced WP: a Postgres-backed EventLedger, SessionBroker, ContextBundle, deterministic dummy ModelAdapter, ToolGate ledger bridge, ArtifactProposal and ArtifactStore linkage, ValidationRunner, PromotionGate, and TraceProjection replay.

The exact proof flow is:
1. Operator creates or imports a task intent.
2. Product assigns durable `KernelTaskRun` and `SessionRun` IDs.
3. `SessionBroker` dispatches the run to a local dummy/echo `ModelAdapter`.
4. `ContextBundle` records exactly what the adapter was allowed to see.
5. Adapter emits a visible response, a tool request, and an artifact proposal.
6. `ToolGate` records allow/deny decisions as typed ledger events.
7. `ArtifactStore` records output/log/evidence artifacts linked to ledger events.
8. `ValidationRunner` records pass/fail evidence for the proposed result.
9. `PromotionGate` records operator approve/reject and authority transition.
10. `TraceProjection` reconstructs the full run from durable product state after restart.

## WHY
The reset brief makes the first useful slice a deterministic product kernel, not a full IDE, UI rebuild, or repo-governance expansion. This WP creates the product-owned authority path that lets later CRDT, sandbox, memory, DCC, and creative modules build on typed state instead of provider chat history, terminal scrollback, repo-governance paperwork, or diagnostic-only mirrors.

## SOURCE_STUB_COVERAGE_MATRIX
The old `WP-1-Postgres-Control-Plane-Shift-Bundle-v1` is superseded as an activation vehicle, not erased as historical source. Kernel-first scope is moved here; residual product families remain attached to active/follow-up stubs.

| Source stub | Kernel-001 coverage | Preservation decision |
|-------------|---------------------|-----------------------|
| WP-1-Postgres-Dev-Test-Container-Matrix-v1 | Covered by MT-001, MT-003, MT-026 for the kernel proof path. | Fully moved for Kernel V1. |
| WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | Covered by MT-008 and MT-023 for minimal broker claims, expiry, retry, and backpressure. | Fully moved for Kernel V1; broader subsystem queues remain later optimization. |
| WP-1-ModelSession-Postgres-Queue-Workers-v1 | Covered by MT-006, MT-007, MT-008, MT-011, MT-012, MT-021, MT-022. | Moved as SessionBroker kernel equivalent. |
| WP-1-FEMS-Postgres-Memory-Store-v1 | Not implemented in this WP except checkpoint vocabulary and event/promotion primitives. | Preserved for `WP-KERNEL-004-Local-Model-Memory-Runtime` and existing FEMS stubs. |
| WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | Kernel run durability covered by MT-006, MT-007, MT-008, MT-019, MT-021, MT-023, MT-026. | Generic workflow-engine migration remains downstream of the kernel proof and workflow-transition stub. |
| WP-1-DCC-Postgres-Control-Plane-Projections-v1 | Minimal no-context trace inspector covered by MT-020. | Full DCC UI/projection expansion remains under `WP-1-Dev-Command-Center-MVP-v1` and later DCC stubs. |
| WP-1-SQLite-Cache-Offline-Boundaries-v1 | Kernel no-SQLite authority/cache/offline/fallback/test guard covered by MT-005 and MT-025. | Kernel portion fully moved; non-kernel cache/offline product policy remains storage follow-up. |

## PRODUCT_CODE_EVIDENCE
Read-only inspection from `../handshake_main` found reusable product surfaces and gaps:
- `src/backend/handshake_core/src/storage/mod.rs:47` already defines `ControlPlaneStorageMode` with `PostgresPrimary`, `SqliteCache`, `SqliteOffline`, and `Test`; Kernel V1 must fail closed for all non-Postgres authority calls.
- `src/backend/handshake_core/src/storage/mod.rs:1475`, `:1526`, and `:1537` already define `ModelSession`, `SessionCheckpoint`, and `SessionMessage`; reuse identity/message/checkpoint concepts rather than inventing a parallel session model.
- `src/backend/handshake_core/src/storage/postgres.rs:39` and `:1701` show a product Postgres backend and migration path; build EventLedger there instead of adding a new database stack.
- `src/backend/handshake_core/src/workflows.rs:235` and `:8177` show process-local scheduler locking; replace kernel authority scheduling with durable Postgres claims.
- `src/backend/handshake_core/src/runtime_governance.rs:54` and `src/backend/handshake_core/src/workflows.rs:6416` show DCC snapshot/projection surfaces; use them as read-only projections, not authority.
- `src/backend/handshake_core/src/llm/mod.rs:34` and `:240` show a replaceable `LlmClient` trait and disabled client; use this pattern for a deterministic dummy ModelAdapter.
- `src/backend/handshake_core/src/mcp/gate.rs:663` through deny/allow branches shows ToolGate enforcement and Flight Recorder logging; add ledger writes without weakening existing checks.
- `src/backend/handshake_core/src/flight_recorder/mod.rs:38` and `:5837` show typed diagnostic events and recorder trait; mirror kernel event IDs to diagnostics, but never replay from Flight Recorder as authority.

## RESEARCH_BASIS
Current research confirmed the reset architecture. Field practice favors a durable controller outside the LLM, persisted checkpoints/state, explicit human approval, traceable tools, and replayable execution. No source found a better architecture than product-owned durable state plus model adapters.

Adopt:
- PostgreSQL `FOR UPDATE SKIP LOCKED` for portable durable worker claims where the worker queue is product-owned.
- Durable checkpoint and human-in-loop patterns from LangGraph, Temporal, Dapr, and OpenAI Agents SDK.
- Trace/event surfaces as observability, while keeping Handshake product storage as authority.

Adapt:
- PostgreSQL `LISTEN/NOTIFY` only as an optional wakeup; polling durable rows remains the recovery path.
- PGMQ as a later optimization candidate; do not require a Postgres extension for Kernel V1.
- CRDT library research remains for `WP-KERNEL-002`; this WP only creates event and promotion authority needed by CRDT integration.

Reject:
- Framework-first product authority.
- Provider chat history, terminal transcript, Flight Recorder, or DuckDB as authority.
- SQLite authority, cache, offline, fallback, or test-fixture use for Kernel V1.

## SPEC_ENRICHMENT_STATUS
Resolved. SPEC_CURRENT now resolves to `.GOV/spec/master-spec-v02.184/indexed-spec-manifest.json`, whose active indexed modules define Kernel V1 Postgres EventLedger authority, no SQLite Kernel V1 authority/cache/offline/fallback/test permission, and projection-only Flight Recorder/DCC/diagnostic posture. Operator signature has been recorded, the v1 packet is activation-ready, and no blocking spec debt remains.

## MICROTASK_INDEX
- MT-001: Reset authority and code reality map.
- MT-002: Kernel event taxonomy.
- MT-003: Postgres EventLedger migration.
- MT-004: EventLedger storage API.
- MT-005: No-SQLite kernel authority guard.
- MT-006: Durable KernelTaskRun and SessionRun identifiers.
- MT-007: SessionBroker state machine.
- MT-008: Durable claim and lease worker.
- MT-009: ContextBundle contract.
- MT-010: Dummy echo ModelAdapter.
- MT-011: Broker dispatch to adapter.
- MT-012: Session messages ledger link.
- MT-013: Tool request event contract.
- MT-014: ToolGate ledger bridge.
- MT-015: ArtifactProposal contract.
- MT-016: ArtifactStore ledger link.
- MT-017: ValidationRunner contract.
- MT-018: PromotionGate contract.
- MT-019: TraceProjection replay.
- MT-020: Minimal DCC or CLI inspector.
- MT-021: Restart reconstruction proof.
- MT-022: Adapter replaceability proof.
- MT-023: Cancellation, backpressure, and dead-letter handling.
- MT-024: Flight Recorder diagnostic mirror.
- MT-025: Product SQLite leakage tripwire.
- MT-026: End-to-end kernel proof.
- MT-027: Validator handoff and debt map.

## ACCEPTANCE_CRITERIA
- The active product implementation exposes a Postgres-backed append-only kernel EventLedger.
- Kernel authority APIs reject SQLite cache, SQLite offline, fallback, and test-fixture authority.
- SessionBroker creates durable KernelTaskRun and SessionRun IDs and persists legal state transitions.
- A deterministic dummy/echo adapter proves the run flow without provider credentials or network.
- Tool decisions, artifact proposals, validation results, and promotion decisions are typed ledger events.
- PromotionGate is the only authority transition path for promoted artifacts/results.
- TraceProjection reconstructs the full proof run from durable product state after restart.
- A no-context model can inspect a run using stable IDs, stored events, ContextBundle, messages, artifacts, validation evidence, and trace projection.
- Product commits happen on a WP feature branch/worktree, not on `gov_kernel`.
- Integration Validator can review the full MT evidence batch before the scoped product-code-vs-Master-Spec review; failed MTs return to Kernel Builder as per-MT mitigation work.

## VALIDATION_PLAN
- Run packet/governance checks from `wt-gov-kernel`: `just wp-contract-import WP-KERNEL-001-Event-Ledger-Session-Broker-v1`, `just task-packet-stub-contracts --all`, `just build-order-sync`, `just gov-check`.
- Product implementation self-checks must run from the product WP worktree after signature and worktree creation.
- Product proof command must distinguish PASS, PRODUCT_FAIL, ENVIRONMENT_BLOCKED, TIMEOUT_INCONCLUSIVE, and unrelated bare cargo failures.
- Kernel Builder records MT implementation evidence and blocker truth, but does not issue validator PASS/FAIL.
- Integration Validator first reviews the full MT evidence batch and returns any failed MTs as per-MT mitigation work.
- After the MT batch passes, Integration Validator performs the WP-scoped product-code-vs-Master-Spec review and must cite run IDs, event IDs, artifact IDs, command output refs, and trace projection output at closeout.

## RISKS_AND_MITIGATIONS
- Risk: this WP is large enough for MT bleed. Mitigation: one MT per Kernel Builder turn, explicit dependencies, typed implementation evidence, and Integration Validator batch MT review before scoped Master Spec review.
- Risk: current SQLite paths are reused because they are convenient. Mitigation: land MT-005 before broker/promotion work and MT-025 before final proof.
- Risk: Flight Recorder is mistaken for authority. Mitigation: ledger event IDs may mirror to Flight Recorder, but replay and promotion must read EventLedger.
- Risk: DCC scope expands into UI. Mitigation: MT-020 allows a minimal structured CLI/API inspector and forbids full DCC UI work.
- Risk: live Postgres is unavailable. Mitigation: proof commands must emit deterministic environment-blocked evidence rather than silently substituting SQLite.
