<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.work_packet_contract@1 source_file=.GOV/task_packets/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/packet.json source_hash=c87a0d1796984add projection_hash=84462b518bf69a86 generated_at_utc=2026-05-14T19:55:48.693Z generator=wp-contract-import.mjs -->
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
- **Status:** Validated (PASS)
- CURRENT_WP_STATUS: READY_FOR_INTEGRATION_VALIDATOR_RECHECK
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja140520260015
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
- MERGED_MAIN_COMMIT: c5fa320e18ef9e1f13993811df77d30c3a25a538
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-05-14T20:52:00Z
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: c5fa320e18ef9e1f13993811df77d30c3a25a538
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-05-14T20:52:00Z
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
Verdict: PASS
Blockers: NONE
Next: Product code is contained in main at c5fa320e18ef9e1f13993811df77d30c3a25a538; wtc-session-broker-v1 is cleanup-safe after operator-approved worktree cleanup.
## WORKTREE_CLEANUP_STATUS (STATUS-SYNC APPENDIX; PRODUCT-CODE ONLY)
- CHECK_TYPE: PRODUCT_CODE_ONLY_WORKTREE_CONTAINMENT
- CHECKED_AT_UTC: 2026-05-14T20:52:00Z
- CHECKED_BY: INTEGRATION_VALIDATOR
- MAIN_HEAD: c5fa320e18ef9e1f13993811df77d30c3a25a538
- WORKTREE_DIR: ../wtc-session-broker-v1
- WORK_BRANCH: feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- WORKTREE_HEAD: 064fa4c8177413f40d658325a34733d4050ad7a3
- BRANCH_HEAD_ANCESTOR_OF_MAIN: YES
- COMMITTED_PRODUCT_DIFF_VS_MAIN_COUNT: 0
- TRACKED_DIRTY_PRODUCT_COUNT: 0
- UNTRACKED_PRODUCT_COUNT: 0
- CLEANUP_RECOMMENDATION: READY_FOR_OPERATOR_APPROVED_WORKTREE_DELETE
- SUMMARY: Branch product commits are contained in main through merge commit c5fa320e18ef9e1f13993811df77d30c3a25a538 and no local product drift was found.
- EVIDENCE:
  - no_committed_product_diff_vs_main
  - no_tracked_dirty_product_paths
  - no_untracked_product_paths

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Kernel V1 product authority is a Postgres EventLedger and must not use SQLite authority, cache, offline, fallback, or test authority for the first kernel slice | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/postgres.rs, src/backend/handshake_core/migrations/** | TESTS: cargo test -p handshake_core kernel_event_ledger --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: EventLedger row links KernelTaskRun, SessionRun, event_type, actor, causation_id, correlation_id, payload, created_at | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: SessionBroker state must be durable, replayable, claim-safe, cancellable, and restart-reconstructable from the ledger | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/storage/mod.rs | TESTS: cargo test -p handshake_core kernel_session_broker --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: one SessionRun transitions queued, claimed, running, completed, cancelled, retryable, dead_lettered without process-local authority | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: ContextBundle and replaceable ModelAdapter must record the exact context allowed to a dummy/echo local adapter without binding kernel semantics to a provider trace | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/llm/**, src/backend/handshake_core/src/storage/mod.rs | TESTS: cargo test -p handshake_core kernel_context_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: dummy adapter emits response, tool request, and artifact proposal linked to a ContextBundle hash | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: ToolGate, ArtifactStore, ValidationRunner, and PromotionGate decisions must be ledger-linked and operator-reviewable before authority transition | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/mcp/gate.rs, src/backend/handshake_core/src/runtime_governance.rs, src/backend/handshake_core/src/flight_recorder/** | TESTS: cargo test -p handshake_core kernel_gate_artifact_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: allow or deny tool request, artifact proposal, validation pass or fail, and approve or reject promotion all share ledger correlation | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: TraceProjection must reconstruct the complete proof run after restart from product authority, with Flight Recorder only mirroring diagnostics | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/flight_recorder/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_trace_projection_restart --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: replay shows task intent, context, adapter output, tool decision, artifact evidence, validation evidence, and promotion decision after process restart | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED

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

## INTEGRATION_VALIDATOR_MT_APPENDIX_2026-05-14

Session: `INTEGRATION_VALIDATOR-20260514-081109`.

Candidate reviewed: `../wtc-session-broker-v1` on `feat/WP-KERNEL-001-Event-Ledger-Session-Broker-v1`.

Operator scope for this appendix: code correctness, clean implementation, and Master Spec readiness. Governance paperwork was explicitly waived for this review pass.

Batch outcome: **MT gate FAIL**. The scoped product-code-vs-Master-Spec PASS/FAIL judgment was not performed because the MT batch did not pass.

Independent checks run:
- `git -C ../wtc-session-broker-v1 status --short --branch`.
- `cargo test -p handshake_core kernel_event_taxonomy --target-dir ../Handshake_Artifacts/handshake-cargo-target` from the worktree root. Result: invalid command context, no `Cargo.toml` at the root.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml kernel_event_taxonomy --target-dir ../Handshake_Artifacts/handshake-cargo-target` from the worktree root. Result: timed out after roughly four minutes with no conclusive test output.
- Static source review of the untracked kernel implementation, migrations, Postgres/SQLite storage changes, and kernel test files.

Environment and proof limits:
- `POSTGRES_TEST_URL` was not set. The new Postgres-dependent kernel tests are written to return early when Postgres is absent, so those tests can appear non-blocking without proving durable Postgres behavior.
- The candidate worktree contains repo-local `Handshake_Artifacts/`, which should not be part of the product worktree state.

Failed or not-proven MTs:
- MT-001 FAIL/NOT_PROVEN: the requested evidence map and proof commands were not available as passing evidence for this batch. This is not treated as a product-code blocker by itself, but it prevents MT-batch PASS.
- MT-002 FAIL: the event taxonomy exists, but the durable event schema does not carry the required event version, aggregate type, aggregate ID, payload hash, or source component fields.
- MT-003 FAIL: Postgres migrations exist, but the EventLedger table is missing the same required authority fields and no successful live Postgres migration proof was available.
- MT-004 FAIL/NOT_PROVEN: append/list APIs exist, but the EventLedger record shape is incomplete and the proof test did not pass in this session.
- MT-006 FAIL: KernelTaskRun and SessionRun IDs are generated and repeated on events, but KernelTaskRun is not proven as a durable reconstructable record independent of the event list.
- MT-008 FAIL: claim/lease storage exists, but there is no generic claim-next worker path, no successful live concurrency proof, and queue state changes are not automatically tied to ledger events.
- MT-009 FAIL/PARTIAL: ContextBundle hashing is present, but durable persistence is only embedded in a happy-path event payload and was not proven as a reconstructable product authority surface.
- MT-011 FAIL/PARTIAL: the proof runner dispatches to the dummy adapter, but this is a happy-path orchestration helper rather than a durable SessionBroker dispatch loop.
- MT-012 FAIL: session messages store `content_artifact_id` hashes, but the actual message body is not persisted by this WP and session message rows do not carry direct kernel event linkage.
- MT-014 FAIL: the existing ToolGate enforcement surface is not bridged. The proof path records a hard-coded allow decision event.
- MT-016 FAIL: ArtifactStore linkage is represented as a ledger event only. There is no product artifact persistence/readback integration proving reconstructable artifacts.
- MT-017 FAIL/PARTIAL: ValidationRunner records a local validation shape and event, but it is not integrated with the existing product validation/check-running surfaces.
- MT-018 FAIL/PARTIAL: PromotionGate requires validation, but operator approval is synthetic in code and not persisted as an operator-reviewable authority transition path.
- MT-019 FAIL: TraceProjection accepts any non-empty same-run event list and does not enforce a complete required event chain, artifact/message readback, or promotion proof.
- MT-020 FAIL/PARTIAL: the kernel trace inspector is kernel-local only; no existing DCC/product inspection path integration was found.
- MT-021 FAIL: restart reconstruction is not proven. The test uses the same database handle and is skipped without `POSTGRES_TEST_URL`.
- MT-022 FAIL/NOT_PROVEN: adapter replaceability exists at the trait/test shape level, but the durable proof depends on skipped Postgres tests.
- MT-023 FAIL/PARTIAL: cancellation, backpressure, retry, and dead-letter states exist, but durable behavior and ledger event recording are not implemented end to end.
- MT-024 FAIL/PARTIAL: Flight Recorder mirror helper exists and is marked diagnostic, but it is not integrated into the proof run.
- MT-025 FAIL/PARTIAL: SQLite authority guards are present, but the tripwire only scans the new kernel source and does not prove absence of SQLite kernel authority through storage/test fixture paths.
- MT-026 FAIL: the end-to-end kernel proof is not valid yet because it relies on skipped Postgres tests and happy-path-only substitutes for ToolGate, ArtifactStore, ValidationRunner, PromotionGate, and TraceProjection completeness.
- MT-027 FAIL: this appendix is the first consolidated debt map; the implementation still needs a corrected handoff after remediation.

MTs with acceptable static shape but still lacking a conclusive batch proof:
- MT-005: SQLite kernel authority methods fail closed and authority mode guards reject SQLite/test modes.
- MT-007: SessionBroker legal transition table exists.
- MT-010: deterministic dummy echo ModelAdapter exists.
- MT-013: tool request event shape exists.
- MT-015: ArtifactProposal shape exists.

Combined remediation plan for Kernel Builder:
1. Expand EventLedger authority schema and Rust types to include event version, aggregate type, aggregate ID, causation parent, actor/session IDs, timestamp, payload hash, and source component; store payload as validated JSON/JSONB where practical.
2. Make SessionBroker the durable path, not only a happy-path proof helper: enqueue, claim-next, lease expiry, running, retry, backpressure, cancellation, completion, failure, and dead-letter transitions should append typed EventLedger events atomically with state changes.
3. Persist reconstructable ContextBundle and session messages. Message rows or linked artifacts must allow a no-context replay to recover the exact prompt/context/response body and cite kernel event IDs.
4. Bridge the real ToolGate surface instead of hard-coding `allow`; ledger events should record actual request, decision, rationale, actor, and linked run IDs without weakening existing checks.
5. Bridge artifact proposal/storage to real product artifact persistence and readback. Ledger rows should reference stored artifacts whose content/hash can be verified during replay.
6. Bridge ValidationRunner and PromotionGate to product validation and operator-reviewable approval surfaces. Synthetic approval is acceptable only inside a test fixture, not as the product authority path.
7. Harden TraceProjection so it rejects incomplete chains and reconstructs the run from durable product rows alone, including context, messages, tool decisions, artifacts, validations, promotion decisions, and event IDs.
8. Add a real restart proof using a fresh Postgres connection/process boundary. Tests must fail or emit an explicit environment-blocked result when `POSTGRES_TEST_URL` is absent; they must not silently skip proof-critical MTs.
9. Widen the SQLite tripwire to cover kernel authority call paths, storage implementations, and test fixtures; preserve the current fail-closed SQLite methods.
10. Move/remove repo-local `Handshake_Artifacts/` from the candidate worktree before handoff, using the project artifact hygiene path rather than manual destructive cleanup.

Residual uncertainty:
- Because the build/test run timed out before producing compile output, additional compiler or test failures may exist behind the static findings above.
- Because no live Postgres proof ran, durable concurrency, migrations, and replay behavior remain unproven even where the code shape is directionally correct.

## KERNEL_BUILDER_REMEDIATION_HANDOFF_2026-05-14

Status: `READY_FOR_INTEGRATION_VALIDATOR_RECHECK`. This is Kernel Builder evidence only; it is not a validator PASS.

Remediated failed MTs: MT-014, MT-020, MT-021, MT-022, MT-023, MT-024, MT-026, MT-027.

Feature/build blockers addressed:
- Default feature build now compiles the existing public module surface because `default` resolves to `runtime-full`.
- `runtime-full` now excludes `duckdb-flight-recorder`, so kernel Postgres/API proof does not compile DuckDB or `libduckdb-sys`.
- DuckDB-backed binaries and tests require explicit `app-runtime` or `duckdb-flight-recorder` opt-in.
- Live Postgres proof was run locally with `POSTGRES_TEST_URL=postgres://postgres:postgres@127.0.0.1:5432/handshake_test`; missing Postgres remains `ENVIRONMENT_BLOCKED`, not a PASS.

Proof commands:
- `cargo test kernel_event_taxonomy --target-dir ..\..\..\..\Handshake_Artifacts\handshake-cargo-target-remediation`: PASS, default filtered cargo path compiled the public module surface and ran `kernel_event_taxonomy_covers_first_slice_families`; 1 passed.
- `cargo test --features runtime-full --test kernel_runtime_tests mcp_toolgate_bridge_records_allow_and_deny_decisions_from_explicit_grants --target-dir ..\..\..\..\Handshake_Artifacts\handshake-cargo-target-remediation`: PASS, 1 runtime-full ToolGate bridge proof test passed without compiling DuckDB/`libduckdb-sys`.
- `cargo test --test kernel_runtime_tests --target-dir ..\..\..\..\Handshake_Artifacts\handshake-cargo-target-remediation`: PASS, 7 default kernel runtime tests passed.
- `cargo tree --features runtime-full -i duckdb`: expected absence evidence, Cargo reported no `duckdb` package in the `runtime-full` dependency tree.
- `cargo test --features runtime-full --test kernel_runtime_tests --test kernel_event_ledger_tests --test kernel_flight_recorder_tests --test kernel_promotion_trace_tests --target-dir ..\..\..\..\Handshake_Artifacts\handshake-cargo-target-remediation`: PASS, 24 focused runtime-full non-Postgres tests passed.
- `POSTGRES_TEST_URL=postgres://postgres:postgres@127.0.0.1:5432/handshake_test cargo test --features runtime-full --test kernel_end_to_end_tests -- --test-threads=1`: PASS, 7 live Postgres end-to-end tests passed.
- `POSTGRES_TEST_URL=postgres://postgres:postgres@127.0.0.1:5432/handshake_test cargo test --features runtime-full --test kernel_postgres_event_ledger_tests -- --test-threads=1`: PASS, 7 live Postgres EventLedger tests passed.
- `POSTGRES_TEST_URL=postgres://postgres:postgres@127.0.0.1:5432/handshake_test KERNEL_TRACE_PROOF_OUTPUT_DIR=D:\Projects\LLM projects\Handshake\Handshake Worktrees\Handshake_Artifacts\kernel-trace-evidence-20260514T194500Z cargo test --features runtime-full --test kernel_end_to_end_tests end_to_end_kernel_proof -- --test-threads=1`: PASS, 1 live Postgres proof test passed and wrote trace evidence.
- `git diff --check`: PASS in the product worktree with CRLF warnings only.

Trace evidence:
- Trace output: `../Handshake_Artifacts/kernel-trace-evidence-20260514T194500Z/end-to-end-kernel-proof-trace-projection.json`.
- KernelTaskRun: `KTR-13e06e2e-1418-4e30-8d67-c7ce14d95548`.
- SessionRun: `SR-121bfa3b-6d6f-4f3c-9c14-299c90f5abf0`.
- Artifact ID: `ART-8b85883c846ded7b`.
- Event IDs: `KE-e4c7241e-3273-417b-ab69-f1a4df0d6751`, `KE-5d28f99b-b09f-45ea-986f-c6220fae7853`, `KE-ee0a0757-7629-4738-9a55-b4b541da892c`, `KE-9baa49be-b28e-4eca-9939-ded9a1ac2ad0`, `KE-b36e229e-a492-4a7c-b6cb-7e12cd3bd94b`, `KE-ac4912eb-4c09-4eee-9c63-a08ef7f77238`, `KE-b0d0823c-b76c-4c2a-8f9f-06292252d386`, `KE-f59cb56b-13c4-47ee-a11e-9900bb410436`, `KE-ef327fd5-96c6-4a38-9b84-ebdf6391f01a`, `KE-96d1ee8b-fa31-437e-83d0-3a4bec7531b9`, `KE-372603fd-c2e0-4819-9431-b44f15bd5425`, `KE-89f77bd6-96d6-42f1-a745-7c2fd61fc915`, `KE-150b0075-335e-4888-b6e9-5dd219e1292a`, `KE-39bb7975-61e3-4ccc-971f-c9843e617ea0`.

Residual debt and follow-up ownership:
- `KERNEL001-DEFER-CRDT-WORKSPACE`: full CRDT workspace/write-box/action catalog work remains outside Kernel001 and is preserved in `WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`.
- `KERNEL001-DEFER-DCC-UI`: Kernel001 adds the backend trace inspection route only; full DCC UI/projection work remains in Kernel002 and DCC follow-up stubs.
- `KERNEL001-DEFER-FEMS-MEMORY`: Kernel001 records context/message/artifact/promotion primitives but does not implement the local model memory runtime; FEMS follow-up stubs and Kernel002 folded scope retain that work.
- `KERNEL001-DEFER-GENERIC-WORKFLOW`: Kernel001 proves the kernel run ledger and SessionBroker path; generic workflow durable execution migration remains outside this WP.

Artifact hygiene:
- External artifact root: `../Handshake_Artifacts`.
- The pre-existing candidate-worktree `Handshake_Artifacts/` directory was moved intact to `../Handshake_Artifacts/repo-local-moved-root-Handshake_Artifacts-20260514T164500Z`.
- Generated trace proof output that initially landed under `src/backend/Handshake_Artifacts` was moved intact to `../Handshake_Artifacts/repo-local-moved-src-backend-Handshake_Artifacts-20260514T153101Z`.
- Feature-split verification recreated `src/backend/Handshake_Artifacts`; that directory was moved intact to `../Handshake_Artifacts/repo-local-moved-src-backend-Handshake_Artifacts-20260514T194040`.
- The candidate product worktree root and `src/backend` now have no repo-local `Handshake_Artifacts/` directory.

Next validator action:
- Integration Validator should re-run the MT batch review from the candidate worktree, then perform scoped Master Spec validation only if the MT batch passes.
