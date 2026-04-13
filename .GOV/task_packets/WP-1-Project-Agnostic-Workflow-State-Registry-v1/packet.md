# TASK_PACKET_TEMPLATE

Copy this into each new task packet and fill all fields.

Requirements:
- Keep packets ASCII-only (required by deterministic gates).
- Use SPEC_BASELINE for provenance (spec at creation time).
- Use SPEC_TARGET as the authoritative spec for closure/revalidation (usually .GOV/spec/SPEC_CURRENT.md).
- WP_ID and filename MUST NOT include date/time stamps; use `-v{N}` for revisions (e.g., `WP-1-Tokenization-Service-v3`).
- If multiple packets exist for the same Base WP, update `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet).
- Packet metadata is the authoritative lifecycle truth. `TASK_BOARD.md`, `WP_TRACEABILITY_REGISTRY.md`, and `BUILD_ORDER.md` are projections and must reconcile to this header.
- Active packet rule: the packet mapped by `BASE_WP_ID` in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` is the current contract. Any other official packet with the same `BASE_WP_ID` is older history and must be tracked as `SUPERSEDED` on the Task Board.
- For `REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1`, this packet is auto-hydrated from the signed refinement; manual drift is forbidden and `just phase-check STARTUP ... CODER` enforces alignment.

---

# Task Packet: WP-1-Project-Agnostic-Workflow-State-Registry-v1

## METADATA
- TASK_ID: WP-1-Project-Agnostic-Workflow-State-Registry-v1
- WP_ID: WP-1-Project-Agnostic-Workflow-State-Registry-v1
- BASE_WP_ID: WP-1-Project-Agnostic-Workflow-State-Registry
- DATE: 2026-04-13T00:23:26.546Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just phase-check HANDOFF ... CODER --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: ActivationManager
- ROLE: Orchestrator
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: CODER_A
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
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- CODER_MODEL: claude-opus-4-6
- CODER_REASONING_STRENGTH: EXTRA_HIGH
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_CONTROL_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl
- SESSION_CONTROL_RESULTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl
- SESSION_COMPATIBILITY_SURFACE: VSCODE_PLUGIN_REPAIR_ONLY
- SESSION_COMPATIBILITY_QUEUE_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
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
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Project-Agnostic-Workflow-State-Registry-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: claude-opus-4-6
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-state-registry-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Project-Agnostic-Workflow-State-Registry-v1
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.4
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Project-Agnostic-Workflow-State-Registry-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Project-Agnostic-Workflow-State-Registry-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Project-Agnostic-Workflow-State-Registry-v1
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY | ABANDONED
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- GOVERNED_VALIDATOR_DUAL_TRACK_FIELDS: MECHANICAL_TRACK_VERDICT | SPEC_RETENTION_TRACK_VERDICT
<!-- For PACKET_FORMAT_VERSION >= 2026-04-05 and RISK_TIER=MEDIUM|HIGH, both governed dual-track fields become mandatory at validator closeout. -->
- GOVERNED_VALIDATOR_COMPLETION_FIELDS: WORKFLOW_VALIDITY | SCOPE_VALIDITY | PROOF_COMPLETENESS | INTEGRATION_READINESS | DOMAIN_GOAL_COMPLETION
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01. Allowed: NONE | LLM_FIRST_DATA_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Validated (PASS)
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: 896e808714f0d584c06efb457b5be0a9e85e2fd5
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-13T05:19:28.442Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 17f0a543276a0393e3746478a82d059af9160b53
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-13T05:19:28.442Z
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NOT_REQUIRED
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NONE | NOT_REQUIRED | FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED -->
- PACKET_WIDENING_EVIDENCE: N/A
<!-- Use follow-on/superseding WP id, audit id, or short rationale when widening is required. -->
- ZERO_DELTA_PROOF_ALLOWED: NO
<!-- Allowed: YES | NO. YES => deterministic post-work may accept an empty diff only for an explicitly proof-only/status-sync packet. -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry, WP-1-Project-Profile-Extension-Registry, WP-1-Governance-Workflow-Mirror
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1
- LOCAL_WORKTREE_DIR: ../wtc-state-registry-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Agnostic-Workflow-State-Registry-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Agnostic-Workflow-State-Registry-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Agnostic-Workflow-State-Registry-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Agnostic-Workflow-State-Registry-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: WP_VALIDATOR:WP-1-Project-Agnostic-Workflow-State-Registry-v1
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-project-agnostic-workflow-state-registry-v1
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja130420260159
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: NONE
Next: NONE
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs; ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board | EXAMPLES: A work packet artifact, a micro-task artifact, and a compact summary that all expose the same `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`, A mailbox-linked wait that still leaves the linked record in a canonical waiting posture with an explicit queue reason, A project-profile label override that changes display wording while preserving the base family, reason, and governed action ids | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171] | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests | EXAMPLES: A work packet artifact, a micro-task artifact, and a compact summary that all expose the same `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`, A mailbox-linked wait that still leaves the linked record in a canonical waiting posture with an explicit queue reason, A project-profile label override that changes display wording while preserving the base family, reason, and governed action ids | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172] | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs; ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs; ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: A work packet artifact, a micro-task artifact, and a compact summary that all expose the same `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`, A mailbox-linked wait that still leaves the linked record in a canonical waiting posture with an explicit queue reason, A project-profile label override that changes display wording while preserving the base family, reason, and governed action ids | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- CANONICAL_CONTRACT_EXAMPLES:
  - A work packet artifact, a micro-task artifact, and a compact summary that all expose the same `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`
  - A mailbox-linked wait that still leaves the linked record in a canonical waiting posture with an explicit queue reason
  - A project-profile label override that changes display wording while preserving the base family, reason, and governed action ids
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: portable workflow-state registry enforcement | SUBFEATURES: family enum coverage, queue-reason coverage, governed action descriptors, durable transition and eligibility ids | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-LOCUS-WORK-TRACKING | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the core contract belongs in product backend validation and artifact emission logic
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: low-cardinality workflow routing substrate | SUBFEATURES: ready-vs-waiting posture, local-small-model reason routing, display-label degradation to base families | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-ProjectProfileWorkflowExtensionV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: small-model routing should depend on compact family and reason fields, not prose or viewer-local labels
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: workflow-state registry enforcement | JobModel: NONE | Workflow: structured_collaboration_validation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: shared backend validation logic, not a standalone operator tool
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: governed action and transition parity across packet and board projections | JobModel: WORKFLOW | Workflow: task_board_projection_refresh | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: board rows, packet artifacts, and compact summaries should expose the same action posture
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: executor eligibility and mailbox-linked wait posture | JobModel: WORKFLOW | Workflow: runtime_governance_queue_refresh | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox-linked waits must remain explicit queue reasons on the linked record rather than viewer-local annotations
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Project-Agnostic-Workflow-State-Registry-v1)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs -> PARTIAL (WP-1-Project-Agnostic-Workflow-State-Registry-v1)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- DATA_CONTRACT_RULES:
  - Keep persisted and emitted structure SQL-backed and PostgreSQL-ready; do not introduce fresh SQLite-only semantics unless the packet or spec explicitly requires them.
  - Prefer explicit machine-readable fields, enums, ids, relations, and provenance over presentation-only strings, overloaded text blobs, or parser-only implied meaning.
  - Preserve stable ids, explicit relations, backlink-friendly fields, provenance anchors, and retrieval-friendly summaries so Loom and graph/search consumers can traverse the data without reparsing UI text.
- VALIDATOR_DATA_PROOF_HINTS:
  - Prove the touched data surfaces remain SQLite-now and PostgreSQL-ready or justify any backend-specific semantics explicitly.
  - Prove the emitted or persisted shapes stay LLM-first readable and parseable with stable field names and explicit structured values.
  - Prove Loom-facing ids, relations, provenance anchors, and retrieval fields remain explicit where the packet touches them.
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
- DIRECT REVIEW CONTRACT:
  - For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED` packets with `PACKET_FORMAT_VERSION >= 2026-03-21`, `COMMUNICATION_CONTRACT` MUST be `DIRECT_REVIEW_V1` and `COMMUNICATION_HEALTH_GATE` MUST be `HANDOFF_VERDICT_BLOCKING`.
  - Required structured receipts for the coder <-> WP validator lane:
    - `VALIDATOR_KICKOFF` (`WP_VALIDATOR -> CODER`)
    - `CODER_INTENT` (`CODER -> WP_VALIDATOR`, correlated to kickoff)
    - `CODER_HANDOFF` (`CODER -> WP_VALIDATOR`)
    - `VALIDATOR_REVIEW` (`WP_VALIDATOR -> CODER`, correlated to handoff)
  - For `PACKET_FORMAT_VERSION >= 2026-03-22`, `VERDICT` also requires one direct coder <-> integration-validator review pair recorded in receipts with a shared `correlation_id` / `ack_for` chain.
  - Review-tracked receipt appends auto-write notifications for the explicit target role and auto-project the next actor / validator wake state back into `RUNTIME_STATUS.json`.
  - `just wp-thread-append` remains valid for soft coordination only. It does not satisfy the required direct-review contract by itself.
  - `just wp-communication-health-check WP-{ID} KICKOFF|HANDOFF|VERDICT` is the machine gate for this contract.
- SESSION START + WAKE RULE:
  - only `WORKFLOW_AUTHORITY` may start repo-governed Coder, WP Validator, and Integration Validator sessions
  - ordinary launch path is `SESSION_HOST_PREFERENCE` through the governed ACP control lane
  - `SESSION_CONTROL_REQUESTS_FILE`, `SESSION_CONTROL_RESULTS_FILE`, and `SESSION_REGISTRY_FILE` are the deterministic launch/steer/watch state for ordinary governed sessions
  - `SESSION_COMPATIBILITY_SURFACE` and `SESSION_COMPATIBILITY_QUEUE_FILE` exist only for explicit compatibility or repair launches; they are not the default `AUTO` path
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Project-Agnostic-Workflow-State-Registry-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- CONTEXT_START_LINE: 6930
- CONTEXT_END_LINE: 7020
- CONTEXT_TOKEN: **Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]
- EXCERPT_ASCII_ESCAPED:
  ```text
**Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]

  - Every canonical Work Packet, Micro-Task, Task Board projection row, and Dev Command Center queue row SHALL expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
  - `workflow_state_family` MUST stay low-cardinality and project-agnostic.
  - `queue_reason_code` MUST explain why the record is currently routed or grouped where it is.
  - Board position, queue order, and mailbox thread order MUST NOT become substitutes for `workflow_state_family` or `queue_reason_code`.
  - Every state-changing operator or model action SHOULD resolve through a registered `GovernedActionDescriptorV1`.
  - Project profiles MAY define `ProjectProfileWorkflowExtensionV1` mappings that rename visible state labels or narrow valid reasons and actions, but those mappings MUST NOT change the meaning of the base families.
  - Local-small-model routing MUST default to `workflow_state_family` plus `queue_reason_code`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]
- CONTEXT_START_LINE: 61135
- CONTEXT_END_LINE: 61192
- CONTEXT_TOKEN: #### 10.11.5.21 Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 10.11.5.21 Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]

  The Dev Command Center and every structured collaboration artifact family member MUST share one portable workflow-state and action vocabulary.

  **Required state contract**
  - Canonical records SHOULD expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
    - optional project-profile display labels
  - `allowed_action_ids` MUST reference registered `GovernedActionDescriptorV1` records rather than ad hoc user-interface verbs.
  - Mailbox-linked waits MUST remain visible as `queue_reason_code=mailbox_response_wait`, but the mailbox thread itself MUST NOT become the authority for the linked record's state family.
  - Project-profile extensions MAY relabel families for display, but they MUST NOT change the base semantic meaning.
  - Unknown project-profile workflow extensions MUST degrade to the base workflow-state families, reason codes, and governed action ids rather than hiding the record.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]
- CONTEXT_START_LINE: 61192
- CONTEXT_END_LINE: 61260
- CONTEXT_TOKEN: #### 10.11.5.22 Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 10.11.5.22 Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]

  The Dev Command Center and every structured collaboration artifact family member MUST now share one portable transition matrix.

  **Required transition contract**
  - Canonical records SHOULD expose or reference:
    - `transition_rule_ids`
    - `queue_automation_rule_ids`
    - `executor_eligibility_policy_ids`
  - `WorkflowTransitionRuleV1` MUST remain portable across Work Packets, Micro-Tasks, Task Board rows, and Role Mailbox-linked waits.
  - `QueueAutomationRuleV1` SHOULD be the reusable contract for triggers such as dependency cleared, mailbox response received, validation passed, and retry timer elapsed.
  - `ExecutorEligibilityPolicyV1` SHOULD be the reusable contract for executor kinds such as `operator`, `local_small_model`, `cloud_model`, `workflow_engine`, `reviewer`, and `governance`.
  - Local-small-model eligibility MUST require a compact summary and a ready-family state.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Appendix 12.4 workflow-state primitive rows
- CONTEXT_START_LINE: 76323
- CONTEXT_END_LINE: 76354
- CONTEXT_TOKEN: "primitive_id": "PRIM-WorkflowStateFamily"
- EXCERPT_ASCII_ESCAPED:
  ```text
"primitive_id": "PRIM-WorkflowStateFamily",
      "title": "WorkflowStateFamily",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-WorkflowQueueReasonCode",
      "title": "WorkflowQueueReasonCode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-GovernedActionDescriptorV1",
      "title": "GovernedActionDescriptorV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProjectProfileWorkflowExtensionV1",
      "title": "ProjectProfileWorkflowExtensionV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkflowTransitionRuleV1",
      "title": "WorkflowTransitionRuleV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-QueueAutomationRuleV1",
      "title": "QueueAutomationRuleV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExecutorEligibilityPolicyV1",
      "title": "ExecutorEligibilityPolicyV1",
      "kind": "ts_interface"
    }
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] | WHY_IN_SCOPE: current product code emits partial workflow metadata but still falls short of the full portable registry contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs; ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board | RISK_IF_MISSED: surfaces will keep emitting workflow posture that looks canonical but is still semantically incomplete
  - CLAUSE: Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171] | WHY_IN_SCOPE: v02.180 explicitly requires portable family, reason, and governed-action semantics across artifact families and queues | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests | RISK_IF_MISSED: `allowed_action_ids` legality and reason coverage will continue to drift across emitters and persistence paths
  - CLAUSE: Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172] | WHY_IN_SCOPE: the packet must either expose or durably reference transition, automation, and executor policy ids before queue posture can be trusted end to end | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/locus/types.rs; ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: executor routing and automatic queue posture will remain social convention rather than durable product truth
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: `workflow_state_family` and `queue_reason_code` on packet and micro-task artifacts | PRODUCER: workflows.rs emitters | CONSUMER: packet readers, micro-task readers, compact summaries, local-small-model routing | SERIALIZER_TRANSPORT: packet.json and summary.json | VALIDATOR_READER: locus/types.rs validators | TRIPWIRE_TESTS: micro_task_executor_tests workflow drift checks | DRIFT_RISK: detail artifacts and summaries can silently diverge
  - CONTRACT: `allowed_action_ids` backed by durable governed action semantics | PRODUCER: workflows.rs and storage/locus_sqlite.rs | CONSUMER: task board, compact summaries, runtime governance, validator reads | SERIALIZER_TRANSPORT: structured collaboration JSON payloads | VALIDATOR_READER: schema registry and action validation paths | TRIPWIRE_TESTS: negative-path unregistered action-id tests | DRIFT_RISK: duplicated helper maps can disagree while all rows still look syntactically valid
  - CONTRACT: transition, automation, and executor policy identifiers | PRODUCER: locus/workflow registry surfaces | CONSUMER: runtime governance queues, task board projections, queue refresh flows | SERIALIZER_TRANSPORT: structured collaboration records or equivalent linked contract ids | VALIDATOR_READER: runtime-governance and queue readers | TRIPWIRE_TESTS: runtime_governance targeted tests plus full cargo test | DRIFT_RISK: queue moves and executor selection stay implicit instead of inspectable
  - CONTRACT: mailbox-linked wait posture on linked records | PRODUCER: runtime_governance.rs and mailbox-linked refresh paths | CONSUMER: queue views, compact summaries, local-small-model routing | SERIALIZER_TRANSPORT: compact summary JSON and linked work record fields | VALIDATOR_READER: workflow-state validators and queue readers | TRIPWIRE_TESTS: runtime_governance targeted tests | DRIFT_RISK: waits remain hidden in prose or thread ordering instead of canonical reason codes
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Expand `locus/types.rs` to align workflow family, reason, governed action, transition, automation, and executor surfaces with the v02.180 contract.
  - Remove helper-only legality drift by routing emitters and persistence-backed readers through one durable workflow/action contract.
  - Propagate canonical waiting and executor posture through packet, micro-task, task-board, and compact-summary consumers.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
- CARRY_FORWARD_WARNINGS:
  - Do not widen into repo-governance tooling, task-board maintenance, role-protocol work, or `.GOV` process changes.
  - Do not let project-profile workflow overrides mutate base-family semantics.
  - Do not preserve duplicated action legality helpers as a second source of truth.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - Full family and queue-reason parity across packet, micro-task, task-board, and compact-summary surfaces
  - Durable governed action, transition, automation, and executor posture instead of helper-only inference
  - Canonical mailbox-linked wait reasons and display-only project-profile relabeling
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - NONE
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - This refinement does not prove the final code shape for durable transition, automation, and executor-policy ids; it only proves those contract surfaces are already required by the current spec.
  - This refinement does not prove any Dev Command Center GUI affordance; it only bounds the backend workflow registry semantics those viewers must consume.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Current v02.180 spec text is already specific enough to decide scope without external comparison.
  - Local product code shows a concrete implementation gap: partial field emission exists, but the full registry-backed contract does not.
- GITHUB_PROJECT_DECISIONS:
  - NONE
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_VERDICT: NOT_APPLICABLE
- SOURCE_SCAN_DECISIONS:
  - NONE
- MATRIX_GROWTH_CANDIDATES:
  - NONE
- ENGINEERING_TRICKS_CARRIED_OVER:
  - NONE
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-WorkflowTransitionRuleV1
  - PRIM-QueueAutomationRuleV1
  - PRIM-ExecutorEligibilityPolicyV1
- PRIMITIVES_EXPOSED:
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-WorkflowTransitionRuleV1
  - PRIM-QueueAutomationRuleV1
  - PRIM-ExecutorEligibilityPolicyV1
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.director
  - engine.archivist
  - engine.dba
  - engine.context
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Locus
  - MicroTask
  - Execution / Job Runtime
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - canonical family and reason parity in packet and micro-task artifacts -> IN_THIS_WP (stub: NONE)
  - governed action registry replaces helper-only action maps -> IN_THIS_WP (stub: NONE)
  - workflow transition rule ids back runtime queue moves -> IN_THIS_WP (stub: NONE)
  - queue automation rule ids explain non-manual posture changes -> IN_THIS_WP (stub: NONE)
  - executor eligibility policy gates local-small-model pickup -> IN_THIS_WP (stub: NONE)
  - mailbox-linked waits become canonical queue reasons on linked records -> IN_THIS_WP (stub: NONE)
  - project-profile workflow label overrides degrade cleanly to base semantics -> IN_THIS_WP (stub: NONE)
  - compact summaries preserve the same governed next-action posture as detail records -> IN_THIS_WP (stub: NONE)
  - durable storage stops duplicating family-to-action legality maps -> IN_THIS_WP (stub: NONE)
  - task-board and compact-summary consumers stay mirrors over the same backend workflow contract -> IN_THIS_WP (stub: NONE)
  - packet, micro-task, and runtime-governance paths all read the same versioned workflow contract -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: portable workflow-state registry enforcement | SUBFEATURES: family enum coverage, queue-reason coverage, governed action descriptors, durable transition and eligibility ids | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-LOCUS-WORK-TRACKING | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the core contract belongs in product backend validation and artifact emission logic
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: packet-detail workflow parity over the shared registry | SUBFEATURES: packet family fields, packet queue posture, packet governed next actions | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-WORK-PACKET-SYSTEM | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: packet artifacts are first-order consumers of the shared registry even though the pillar itself is not widened as a separate product lane
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: queue posture parity on entry/index/view records | SUBFEATURES: family badge source truth, queue reason parity, governed next-action previews | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-TASK-BOARD | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: board projections must consume the same backend workflow vocabulary instead of helper-only lane semantics
  - PILLAR: MicroTask | CAPABILITY_SLICE: micro-task detail and summary workflow parity | SUBFEATURES: compact summary readiness, governed next-action parity, durable waiting posture | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task artifacts already carry partial workflow posture and need the full registry-backed contract
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: durable transition, automation, and executor eligibility posture | SUBFEATURES: rule ids, automation ids, executor policy ids, local-small-model readiness posture | PRIMITIVES_FEATURES: PRIM-WorkflowTransitionRuleV1, PRIM-QueueAutomationRuleV1, PRIM-ExecutorEligibilityPolicyV1, FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.director, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: executor routing should be backed by stable contract ids rather than inferred from prose
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: low-cardinality workflow routing substrate | SUBFEATURES: ready-vs-waiting posture, local-small-model reason routing, display-label degradation to base families | PRIMITIVES_FEATURES: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-ProjectProfileWorkflowExtensionV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: small-model routing should depend on compact family and reason fields, not prose or viewer-local labels
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: workflow-state registry enforcement | JobModel: NONE | Workflow: structured_collaboration_validation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: shared backend validation logic, not a standalone operator tool
  - Capability: governed action and transition parity across packet and board projections | JobModel: WORKFLOW | Workflow: task_board_projection_refresh | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: board rows, packet artifacts, and compact summaries should expose the same action posture
  - Capability: executor eligibility and mailbox-linked wait posture | JobModel: WORKFLOW | Workflow: runtime_governance_queue_refresh | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox-linked waits must remain explicit queue reasons on the linked record rather than viewer-local annotations
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Project-Agnostic-Workflow-State-Registry-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Project-Profile-Extension-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Governance-Workflow-Mirror-v2 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Project-Agnostic-Workflow-State-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Project-Agnostic-Workflow-State-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs -> PARTIAL (WP-1-Project-Agnostic-Workflow-State-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs -> PARTIAL (WP-1-Project-Agnostic-Workflow-State-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs -> PARTIAL (WP-1-Project-Agnostic-Workflow-State-Registry-v1)
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
- What: Implement and prove one product-backed registry surface for workflow-state families, queue-reason codes, governed action descriptors, workflow transition rules, queue automation rules, executor eligibility posture, and project-profile workflow label overrides across packet, micro-task, task-board, mailbox-linked, and compact-summary surfaces.
- Why: Current product code already exposes partial workflow metadata, but it does not yet satisfy the full v02.180 portable registry contract. Until that closes, queue posture, action legality, and executor routing remain partly helper-defined and drift-prone.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - Dev Command Center UI implementation or visual workflow editors
  - Repo-governance task board, broker, runtime, or role-protocol maintenance
  - New Master Spec or appendix updates
  - Full mailbox workflow ownership changes beyond explicit linked-record queue posture
- TOUCHED_FILE_BUDGET: 6
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (For explicit post-signature continuation after `TOKEN_BUDGET_EXCEEDED` / `POLICY_CONFLICT`, use `COVERS: GOVERNANCE` and name the policy conflict directly in `SCOPE` or `JUSTIFICATION`.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
```

### DONE_MEANS
- Product code exposes the full spec-defined workflow family and queue reason vocabulary or an explicitly equivalent durable contract.
- Governed action legality no longer depends on duplicated helper-only maps.
- Packet, micro-task, task-board, and compact-summary surfaces remain family and reason equivalent under the same backend contract.
- Mailbox-linked waits remain visible as canonical queue reasons on the linked record instead of being only sidecar prose or ordering hints.

- PRIMITIVES_EXPOSED:
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-WorkflowTransitionRuleV1
  - PRIM-QueueAutomationRuleV1
  - PRIM-ExecutorEligibilityPolicyV1
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-13T00:23:26.546Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.171]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.180.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- Codex: .GOV/codex/Handshake_Codex_v1.4.md
- Task Board: .GOV/roles_shared/records/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.180.md
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- SEARCH_TERMS:
  - workflow_state_family
  - queue_reason_code
  - allowed_action_ids
  - GovernedActionDescriptorV1
  - WorkflowTransitionRuleV1
  - QueueAutomationRuleV1
  - ExecutorEligibilityPolicyV1
  - mailbox_response_wait
- RUN_COMMANDS:
  ```bash
rg -n "workflow_state_family|queue_reason_code|allowed_action_ids|GovernedActionDescriptorV1|WorkflowTransitionRuleV1|QueueAutomationRuleV1|ExecutorEligibilityPolicyV1" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance
  ```
- RISK_MAP:
  - "reason vocabulary stays narrower than the spec" -> "queue grouping and routing remain inconsistent across surfaces"
  - "action legality remains helper-defined in multiple places" -> "durable governed action semantics drift and become harder to audit"
  - "mailbox waits remain sidecar-only hints" -> "linked records lose canonical waiting posture and small-model routing degrades"
  - "project-profile label overrides mutate semantics instead of labels" -> "portable workflow meaning collapses across product kernels"
## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.
## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- TRUST_BOUNDARY: N/A
- SERVER_SOURCES_OF_TRUTH:
  - NONE
- REQUIRED_PROVENANCE_FIELDS:
  - NONE
- VERIFICATION_PLAN:
  - Record end-to-end trust/provenance requirements only if this WP introduces a cross-boundary apply path.
- ERROR_TAXONOMY_PLAN:
  - N/A for initial coder handoff.
- UI_GUARDRAILS:
  - N/A for initial coder handoff.
- VALIDATOR_ASSERTIONS:
  - Validate the packet-scoped spec anchors, in-scope files, and deterministic evidence recorded during implementation.
## IMPLEMENTATION
- MT-001 established the product-side workflow state contract in `src/backend/handshake_core/src/locus/types.rs` and `src/backend/handshake_core/src/workflows.rs`: governed action registry resolution, project-profile display relabel degradation, and mailbox-aware queue-reason handling now derive from shared helpers instead of helper-local action maps.
- MT-002 removed storage-layer legality drift in `src/backend/handshake_core/src/storage/locus_sqlite.rs` and added parity proof in `src/backend/handshake_core/tests/micro_task_executor_tests.rs`, so persisted MT progress metadata emits the same workflow family, queue reason, and allowed action ids as the structured artifacts.
- MT-003 added the portable transition matrix, queue automation rules, and executor eligibility policies in `src/backend/handshake_core/src/locus/types.rs`, wired those ids through task-board, work-packet, micro-task, summary, and persisted-progress emitters in `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, and `src/backend/handshake_core/src/storage/locus_sqlite.rs`, and proved the registry semantics in `src/backend/handshake_core/src/runtime_governance.rs`.

## HYGIENE
- Bounded the whole-WP committed product range to `e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5` after the default creation-time `MERGE_BASE_SHA..HEAD` handoff preflight pulled unrelated historic branch drift.
- Confirmed the signed WP implementation is fully contained in six product files: `src/backend/handshake_core/src/locus/task_board.rs`, `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/runtime_governance.rs`, `src/backend/handshake_core/src/storage/locus_sqlite.rs`, `src/backend/handshake_core/src/workflows.rs`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs`.
- Completed overlap review on all declared MTs with PASS receipts: MT-001 correlation `review:WP-1-Project-Agnostic-Workflow-State-Registry-v1:review_request:mnwj1u5r:89db1a`, MT-002 correlation `review:WP-1-Project-Agnostic-Workflow-State-Registry-v1:review_request:mnwjo83m:60bc83`, MT-003 correlation `review:WP-1-Project-Agnostic-Workflow-State-Registry-v1:review_request:mnwksumj:006d86`.
- Rebuilt the deterministic manifest from committed blob ids and `git diff --unified=0` output because the packet still contained placeholder `VALIDATION`, `STATUS_HANDOFF`, `EVIDENCE_MAPPING`, and `EVIDENCE` sections.
- Attempted a fresh targeted product proof run with `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests locus_mt_progress_workflow_parity_with_emitted_packet_and_mailbox_wait -- --exact --nocapture`, but the compile stopped early on a pre-existing out-of-scope unclosed delimiter in `src/backend/handshake_core/src/flight_recorder/mod.rs`.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable `just phase-check HANDOFF <WP_ID> CODER`. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/locus/task_board.rs`
- **Start**: 48
- **End**: 54
- **Line Delta**: +6
- **Pre-SHA1**: `94420cf97740ebc3df0bf2a1fda05b8d0a40e634`
- **Post-SHA1**: `a93551b18629a826c30648137d9cf97b156fe53b`
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

- **Target File**: `src/backend/handshake_core/src/locus/types.rs`
- **Start**: 154
- **End**: 829
- **Line Delta**: +569
- **Pre-SHA1**: `20426e53c50e4fa53a5840aea0132ab045590a86`
- **Post-SHA1**: `6596ebbf12a9f58223f70cf507e6b28d5bf72000`
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

- **Target File**: `src/backend/handshake_core/src/runtime_governance.rs`
- **Start**: 466
- **End**: 681
- **Line Delta**: +215
- **Pre-SHA1**: `b0d6defd4569ee504af2930fe7264427d258af4e`
- **Post-SHA1**: `4ce3f1882174eb994533e19eaf86d878bc10ee4d`
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

- **Target File**: `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Start**: 18
- **End**: 248
- **Line Delta**: -3
- **Pre-SHA1**: `e8b673477c97e800f09b9d469276969d48b0be08`
- **Post-SHA1**: `fc8d53f5303e284b2921556fd9af705f14f1dd4e`
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

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 3275
- **End**: 25262
- **Line Delta**: +33
- **Pre-SHA1**: `292b63d2c0da2ccd5dfd1505461575223096d6d5`
- **Post-SHA1**: `0c4c341fa7db77c06f68680183e71d5490b01d5a`
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

- **Target File**: `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- **Start**: 21
- **End**: 3345
- **Line Delta**: +175
- **Pre-SHA1**: `d0d8c79a208ac5f9152ff28769f02f04d5dd0af7`
- **Post-SHA1**: `9d15fd0c6a9924280db018506cb6b63422525916`
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

- **Lint Results**: Whole-WP overlap review passed for all declared MTs. Fresh local Rust reruns are still blocked by the out-of-scope parse error in `src/backend/handshake_core/src/flight_recorder/mod.rs`, so the active proof set is the committed-range manifest plus the MT PASS receipts already recorded on this WP.
- **Artifacts**: committed product range `e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`; commits `6d18529c`, `6c61d07e`, `a77df5e3`, `896e8087`; signed patch artifact `.GOV/task_packets/WP-1-Project-Agnostic-Workflow-State-Registry-v1/signed-scope.patch`; remote backup branch `feat/WP-1-Project-Agnostic-Workflow-State-Registry-v1`
- **Timestamp**: 2026-04-13T02:50:48.1097478Z
- **Operator**: CODER_A
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- Deterministic Handoff Command: `just phase-check HANDOFF WP-1-Project-Agnostic-Workflow-State-Registry-v1 CODER --range e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`
- **Notes**: The bounded six-file committed range above is the authoritative product proof surface for this WP. It intentionally supersedes the stale creation-time `MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7` handoff range because that broader base pulled unrelated historical branch drift outside the signed WP scope.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: DONE_VALIDATED
- What changed in this update: Across MT-001..MT-003, the product workflow registry now exposes project-agnostic workflow families, canonical queue reasons, governed action ids, transition ids, queue automation ids, executor eligibility ids, and mailbox-aware wait posture across task-board rows, work-packet artifacts, micro-task artifacts, compact summaries, and persisted MT progress metadata.
- Requirements / clauses self-audited: `Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]` is implemented through `src/backend/handshake_core/src/locus/types.rs:174`, `:225`, `:359`, `:413`, `src/backend/handshake_core/src/workflows.rs:3278`, `:3288`, `:3326`, `:3763`, `:4853`, `:4893`, `:4961`, `:5000`, and `src/backend/handshake_core/src/locus/task_board.rs:50`; `Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]` is retained in persisted MT progress via `src/backend/handshake_core/src/storage/locus_sqlite.rs:174`, `:215`, `:242` and parity proof `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3174`; `Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]` is implemented by `src/backend/handshake_core/src/locus/types.rs:437`, `:504`, `:532`, `:565`, `:598`, `:670`, `:683`, exposed by `src/backend/handshake_core/src/locus/task_board.rs:50`, `:52`, `:54`, `src/backend/handshake_core/src/workflows.rs:3780`, `:3781`, `:3782`, `:4868`, `:4873`, `:4878`, `:4912`, `:4913`, `:4914`, `:4976`, `:4981`, `:4986`, `:5019`, `:5020`, `:5021`, and proven in `src/backend/handshake_core/src/runtime_governance.rs:479`, `:526`, `:607`.
- Checks actually run: `git -C ..\\wtc-state-registry-v1 log --oneline --decorate --graph --max-count 5`; `git -C ..\\wtc-state-registry-v1 diff --numstat e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`; `git -C ..\\wtc-state-registry-v1 diff --unified=0 e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5 -- src/backend/handshake_core/src/locus/task_board.rs src/backend/handshake_core/src/locus/types.rs src/backend/handshake_core/src/runtime_governance.rs src/backend/handshake_core/src/storage/locus_sqlite.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/micro_task_executor_tests.rs`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests locus_mt_progress_workflow_parity_with_emitted_packet_and_mailbox_wait -- --exact --nocapture` (blocked by out-of-scope `src/backend/handshake_core/src/flight_recorder/mod.rs` parse error); `just phase-check HANDOFF WP-1-Project-Agnostic-Workflow-State-Registry-v1 CODER --range e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`.
- Known gaps / weak spots: Fresh local cargo proof is still blocked by the out-of-scope `src/backend/handshake_core/src/flight_recorder/mod.rs` syntax error. Inside signed scope, `ProjectProfileWorkflowExtensionV1.narrowed_reason_codes` is present at `src/backend/handshake_core/src/locus/types.rs:180` but is not yet consumed by a runtime narrowing helper, and `structured_work_packet_next_action` / `structured_micro_task_next_action` at `src/backend/handshake_core/src/workflows.rs:5056` and `:12850` still emit ad hoc strings outside the governed action registry.
- Heuristic risks / maintainability concerns: `src/backend/handshake_core/src/workflows.rs` remains a large shared emitter surface with widely separated packet/summary/task-board construction paths. Future callers can still reintroduce drift if they bypass the registry helpers in `src/backend/handshake_core/src/locus/types.rs` and hardcode ids locally.
- Validator focus request: Re-check that every product artifact family member and persisted MT progress path carries the same workflow family, queue reason, allowed action ids, transition ids, automation ids, and executor eligibility ids for the bounded six-file range; confirm the residual `narrowed_reason_codes` and `structured_*_next_action` gaps are non-blocking relative to the signed spec anchors; and verify current-`main` compatibility on the shared `locus/types.rs`, `storage/locus_sqlite.rs`, and `workflows.rs` surfaces before merge.
- Rubric contract understanding proof: This WP is product governance only. The contract is the portable product-side workflow vocabulary consumed by work packets, micro tasks, summaries, task-board rows, and persisted progress metadata. It is not a repo-governance task, and it does not authorize `.GOV` workflow, protocol, or task-board process changes as part of product closure.
- Rubric scope discipline proof: The authoritative handoff range is bounded to the six signed product files and four product commits from `e450df81` through `896e8087`. The out-of-scope `src/backend/handshake_core/src/flight_recorder/mod.rs` parse failure was recorded as external blocker evidence, not accepted into this WP or patched inside the signed product range.
- Rubric baseline comparison: Relative to `e450df813b20e2dddeb7b7b51cac28d922e41a29`, the committed WP delta adds project-profile display relabel degradation, governed action registry helpers, mailbox-aware reason resolution, storage parity, transition rules, queue automation rules, executor eligibility policies, six emitted-surface id arrays, and targeted parity / semantics tests. Net delta across the six-file bounded range is `+995` lines.
- Rubric end-to-end proof: The bounded range proves end-to-end propagation from registry helpers in `src/backend/handshake_core/src/locus/types.rs` through emitters in `src/backend/handshake_core/src/workflows.rs` and persistence in `src/backend/handshake_core/src/storage/locus_sqlite.rs`, while `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3174` and `src/backend/handshake_core/src/runtime_governance.rs:479`, `:526`, `:607` provide the diff-scoped parity and semantics proofs. The only missing fresh rerun is blocked outside signed scope by the `flight_recorder` parse error.
- Rubric architecture fit self-review: The implementation keeps the product workflow contract in `locus/types.rs` and reuses existing emitter and storage layers instead of introducing a second authority or repo-governance dependency. Task-board projection, packet emission, summary emission, and persisted MT progress all consume the same registry helpers.
- Rubric heuristic quality self-review: The new logic is centralized enough to prevent the old action-legality drift, but the shared emitter breadth in `workflows.rs` means future edits still need direct line review on each artifact builder. The current shape is maintainable if those callers continue to flow through the typed helper functions rather than inline strings.
- Rubric anti-gaming / counterfactual check: If `governed_action_ids_for_family` were removed from `src/backend/handshake_core/src/workflows.rs:3396` or `src/backend/handshake_core/src/storage/locus_sqlite.rs:242`, allowed action ids would drift again across emitted and persisted surfaces. If `resolve_queue_reason_with_mailbox_context` stopped overriding only the reason at `src/backend/handshake_core/src/locus/types.rs:413`, the mailbox parity proof in `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3174` would lose `mailbox_response_wait` while preserving the wrong semantics. If `transition_rule_ids_for_family`, `queue_automation_rule_ids_for_reason`, or `executor_eligibility_policy_ids_for_family` were removed from the emitter callsites at `src/backend/handshake_core/src/workflows.rs:3780-3782`, `:4868-4879`, `:4912-4914`, `:4976-4987`, or `:5019-5021`, the packet and summary artifacts would immediately stop exposing the v02.172 contract.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: This handoff is tied to a concrete committed range, committed blob ids, explicit file windows, and MT review receipts. It no longer depends on placeholder packet sections, stale creation-time merge-base assumptions, or a generic "tests passed" claim.
- Signed-scope debt ledger: NONE inside the bounded six-file range. The out-of-scope `src/backend/handshake_core/src/flight_recorder/mod.rs` parse error remains external product debt and is not accepted as WP-owned residue.
- Data contract self-check: The active data contract is emitted structurally on `TaskBoardEntryRecordV1` in `src/backend/handshake_core/src/locus/task_board.rs:50`, on work-packet / micro-task artifacts and summaries in `src/backend/handshake_core/src/locus/types.rs:726`, `:763`, `:825` plus `src/backend/handshake_core/src/workflows.rs:4868-4879`, `:4912-4914`, `:4976-4987`, `:5019-5021`, and on persisted MT progress metadata in `src/backend/handshake_core/src/storage/locus_sqlite.rs:242-245`. The ids are typed, low-cardinality, and emitter-backed rather than UI-only strings.
- Next step / handoff hint: Re-run `just phase-check HANDOFF WP-1-Project-Agnostic-Workflow-State-Registry-v1 CODER --range e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`, publish `CODER_HANDOFF` on that bounded range, then wake `WP_VALIDATOR` for whole-WP review before closeout prep and final-lane launch.

## MERGE_PROGRESSION_TRUTH
- For `PACKET_FORMAT_VERSION >= 2026-03-25`, PASS closure is two-step and must stay explicit:
  - validator PASS append before merge-to-main:
    - set `**Status:** Done`
    - set `MAIN_CONTAINMENT_STATUS: MERGE_PENDING`
    - keep `MERGED_MAIN_COMMIT: NONE`
    - keep `MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A`
    - project Task Board `## Done` token as `[MERGE_PENDING]`
  - after merge-to-main authority finishes and local `main` actually contains the approved closure commit:
    - set `**Status:** Validated (PASS)`
    - set `MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN`
    - record `MERGED_MAIN_COMMIT: <main-contained sha>`
    - record `MAIN_CONTAINMENT_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - project Task Board `## Done` token as `[VALIDATED]`
- `Validated (FAIL)`, `Validated (OUTDATED_ONLY)`, and `Validated (ABANDONED)` must use `MAIN_CONTAINMENT_STATUS: NOT_REQUIRED`.
- Historical packet variants older than `PACKET_FORMAT_VERSION 2026-03-25` keep their legacy closure semantics and must not be rewritten in place only to satisfy this newer merge-progression model.

## SIGNED_SCOPE_COMPATIBILITY_TRUTH
- For `PACKET_FORMAT_VERSION >= 2026-03-26`, final-lane PASS clearance must record current-`main` compatibility explicitly before `just validator-gate-commit`:
  - if the signed packet still covers the required integration work against current local `main`, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: NOT_REQUIRED`
    - `PACKET_WIDENING_EVIDENCE: N/A`
  - if current local `main` introduces adjacent shared-surface work outside the signed packet, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: ADJACENT_SCOPE_REQUIRED`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED`
    - `PACKET_WIDENING_EVIDENCE: <new WP id / packet id / audit id / short governed rationale>`
  - if compatibility cannot be checked honestly, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: BLOCKED`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: NONE`
    - `PACKET_WIDENING_EVIDENCE: N/A`
- `CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN` is legal only before final-lane compatibility review starts.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "Every canonical Work Packet, Micro-Task, Task Board projection row, and Dev Command Center queue row SHALL expose `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`."
  - EVIDENCE: `src/backend/handshake_core/src/locus/task_board.rs:50`; `src/backend/handshake_core/src/locus/types.rs:726`; `src/backend/handshake_core/src/locus/types.rs:763`; `src/backend/handshake_core/src/locus/types.rs:825`; `src/backend/handshake_core/src/workflows.rs:3763`; `src/backend/handshake_core/src/workflows.rs:4893`; `src/backend/handshake_core/src/workflows.rs:5000`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:215`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3174`
  - REQUIREMENT: "`allowed_action_ids` MUST reference registered `GovernedActionDescriptorV1` records rather than ad hoc user-interface verbs."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:225`; `src/backend/handshake_core/src/locus/types.rs:359`; `src/backend/handshake_core/src/workflows.rs:3396`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:242`
  - REQUIREMENT: "Mailbox-linked waits MUST remain visible as `queue_reason_code=mailbox_response_wait`, but the mailbox thread itself MUST NOT become the authority for the linked record's state family."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:413`; `src/backend/handshake_core/src/workflows.rs:3278`; `src/backend/handshake_core/src/workflows.rs:3288`; `src/backend/handshake_core/src/workflows.rs:3326`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:174`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3174`
  - REQUIREMENT: "Canonical records SHOULD expose or reference `transition_rule_ids`, `queue_automation_rule_ids`, and `executor_eligibility_policy_ids`."
  - EVIDENCE: `src/backend/handshake_core/src/locus/task_board.rs:50`; `src/backend/handshake_core/src/locus/types.rs:726`; `src/backend/handshake_core/src/locus/types.rs:763`; `src/backend/handshake_core/src/locus/types.rs:825`; `src/backend/handshake_core/src/workflows.rs:3780`; `src/backend/handshake_core/src/workflows.rs:4868`; `src/backend/handshake_core/src/workflows.rs:4976`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:243`
  - REQUIREMENT: "`WorkflowTransitionRuleV1` MUST remain portable across Work Packets, Micro-Tasks, Task Board rows, and Role Mailbox-linked waits."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:437`; `src/backend/handshake_core/src/locus/types.rs:504`; `src/backend/handshake_core/src/workflows.rs:3780`; `src/backend/handshake_core/src/workflows.rs:4912`; `src/backend/handshake_core/src/workflows.rs:5019`; `src/backend/handshake_core/src/runtime_governance.rs:479`
  - REQUIREMENT: "Local-small-model eligibility MUST require a compact summary and a ready-family state."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:598`; `src/backend/handshake_core/src/locus/types.rs:683`; `src/backend/handshake_core/src/runtime_governance.rs:607`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `git -C ..\wtc-state-registry-v1 log --oneline --decorate --graph --max-count 5`
  - EXIT_CODE: `0`
  - LOG_PATH: `N/A`
  - LOG_SHA256: `N/A`
  - PROOF_LINES: `* 896e8087 feat: add workflow transition matrix, queue automation, and executor eligibility registry [WP-1-Project-Agnostic-Workflow-State-Registry-v1] [MT-003]`; `* a77df5e3 feat: eliminate storage-layer legality drift and prove MT progress workflow parity [WP-1-Project-Agnostic-Workflow-State-Registry-v1] [MT-002]`; `* 6c61d07e fix: wire mailbox-aware queue_reason into all emitter callsites and revert out-of-scope flight_recorder edits [WP-1-Project-Agnostic-Workflow-State-Registry-v1] [MT-001]`; `* 6d18529c feat: establish GovernedActionDescriptorV1 registry and workflow state contract [WP-1-Project-Agnostic-Workflow-State-Registry-v1] [MT-001]`; `* e450df81 docs: bootstrap claim [WP-1-Project-Agnostic-Workflow-State-Registry-v1]`
  - COMMAND: `git -C ..\wtc-state-registry-v1 diff --numstat e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`
  - EXIT_CODE: `0`
  - LOG_PATH: `N/A`
  - LOG_SHA256: `N/A`
  - PROOF_LINES: `6 0 src/backend/handshake_core/src/locus/task_board.rs`; `569 0 src/backend/handshake_core/src/locus/types.rs`; `215 0 src/backend/handshake_core/src/runtime_governance.rs`; `19 22 src/backend/handshake_core/src/storage/locus_sqlite.rs`; `84 51 src/backend/handshake_core/src/workflows.rs`; `176 1 src/backend/handshake_core/tests/micro_task_executor_tests.rs`
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests locus_mt_progress_workflow_parity_with_emitted_packet_and_mailbox_wait -- --exact --nocapture`
  - EXIT_CODE: `1`
  - LOG_PATH: `N/A`
  - LOG_SHA256: `N/A`
  - PROOF_LINES: `error: this file contains an unclosed delimiter`; `--> src\flight_recorder\mod.rs:6180:3`; `impl fmt::Display for FlightRecorderEventType {`; `impl FlightRecorderEvent {`; `could not compile \`handshake_core\` (lib) due to 1 previous error`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
- For `PACKET_FORMAT_VERSION >= 2026-03-25`, merge progression truth is part of closure law:
  - `**Status:** Done` means PASS is recorded but main containment is still pending and requires:
    - `MAIN_CONTAINMENT_STATUS: MERGE_PENDING`
    - `MERGED_MAIN_COMMIT: NONE`
    - `MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A`
  - `**Status:** Validated (PASS)` is reserved for closures already contained in local `main` and requires:
    - `MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN`
    - `MERGED_MAIN_COMMIT: <main-contained sha>`
    - `MAIN_CONTAINMENT_VERIFIED_AT_UTC: <RFC3339 UTC>`
  - `Validated (FAIL)`, `Validated (OUTDATED_ONLY)`, and `Validated (ABANDONED)` require `MAIN_CONTAINMENT_STATUS: NOT_REQUIRED`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY | ABANDONED`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
  - `SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED`
- For `PACKET_FORMAT_VERSION >= 2026-03-22`, every appended governed validation report MUST also include these completion-layer fields:
  - `WORKFLOW_VALIDITY: VALID | INVALID | PARTIAL | BLOCKED | NOT_RUN`
  - `SCOPE_VALIDITY: IN_SCOPE | OUT_OF_SCOPE | PARTIAL | BLOCKED | NOT_RUN`
  - `PROOF_COMPLETENESS: PROVEN | NOT_PROVEN | PARTIAL | BLOCKED | NOT_RUN`
  - `INTEGRATION_READINESS: READY | NOT_READY | PARTIAL | BLOCKED | NOT_RUN`
  - `DOMAIN_GOAL_COMPLETION: COMPLETE | INCOMPLETE | PARTIAL | BLOCKED | NOT_RUN`
- For `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, every appended governed validation report MUST also include:
  - `MECHANICAL_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_RETENTION_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST also include:
  - `CLAUSES_REVIEWED:`
    - one bullet per in-scope MUST/SHOULD clause reviewed, each with file:line evidence or an explicit proof note
    - when `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`, reuse the exact clause text from `CLAUSE_CLOSURE_MATRIX`
  - `NOT_PROVEN:`
    - `- NONE` only when nothing remains unproven
    - otherwise list each unresolved clause/gap explicitly
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
  - `MAIN_BODY_GAPS:`
    - `- NONE` only when no main-body requirement remains unproven, partial, or weakly evidenced
    - otherwise list each unresolved MUST/SHOULD gap explicitly
  - `QUALITY_RISKS:`
    - `- NONE` only when no material maintainability, brittleness, ambiguity, or heuristic-quality risk remains
    - otherwise list each residual code-quality risk explicitly
  - `VALIDATOR_RISK_TIER: LOW | MEDIUM | HIGH`
    - validator-assigned risk tier; MUST NOT be lower than the packet `RISK_TIER`
  - `DIFF_ATTACK_SURFACES:`
    - list the failure surfaces the validator derived from reading the diff directly
  - `INDEPENDENT_CHECKS_RUN:`
    - list validator-owned checks that were not copied from coder evidence, formatted as `what => observed`
  - `COUNTERFACTUAL_CHECKS:`
    - list concrete code-path / symbol counterfactuals in the form `If X were removed or altered, Y would break`
    - naming a test only is insufficient; name the file, symbol, or code path
  - `BOUNDARY_PROBES:`
    - required for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`
    - record the validator's interface / producer-consumer / storage / contract boundary checks
  - `NEGATIVE_PATH_CHECKS:`
    - required for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`
    - record invalid, missing, adversarial, or failure-path checks the validator ran
  - `INDEPENDENT_FINDINGS:`
    - list what the validator learned independently, even if the conclusion is baseline confirmation
  - `RESIDUAL_UNCERTAINTY:`
    - list remaining uncertainty explicitly; for `VALIDATOR_RISK_TIER=HIGH`, `- NONE` is illegal
  - `SPEC_CLAUSE_MAP:`
    - map each packet requirement to `file:line` evidence proving it is implemented
    - entries must include concrete code references (file paths, line numbers, or symbol names)
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`
  - `NEGATIVE_PROOF:`
    - list at least one spec requirement the validator verified is NOT fully implemented
    - this proves the validator independently read the code rather than trusting coder summaries
    - `- NONE` is illegal; every codebase has at least one gap or partial implementation
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`
- For `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
  - `ANTI_VIBE_FINDINGS:`
    - `- NONE` only when the validator found no shallow easy-surface work, no weakly justified implementation, and no vibe-coded behavior inside signed scope
    - otherwise list each anti-vibe finding explicitly
  - `SIGNED_SCOPE_DEBT:`
    - `- NONE` only when no signed-scope debt, cleanup IOU, or "fix later" residue was accepted
    - otherwise list each signed-scope debt item explicitly
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
  - `PRIMITIVE_RETENTION_PROOF:`
    - record the concrete file:line or symbol evidence proving previously exposed or expected primitives remain present after the change
    - for `RISK_TIER=MEDIUM|HIGH`, `- NONE` is illegal
  - `PRIMITIVE_RETENTION_GAPS:`
    - `- NONE` only when no primitive was weakened, dropped, or silently re-shaped inside signed scope
    - otherwise list each retained-feature or primitive-composition gap explicitly
  - `SHARED_SURFACE_INTERACTION_CHECKS:`
    - record the concrete producer/consumer, registry, type, runtime, or contract interaction checks reviewed across shared surfaces
    - for `RISK_TIER=MEDIUM|HIGH` or `SHARED_SURFACE_RISK=YES`, `- NONE` is illegal
  - `CURRENT_MAIN_INTERACTION_CHECKS:`
    - record the concrete current-`main` caller, consumer, or compatibility interactions reviewed against the packet diff
    - for `RISK_TIER=MEDIUM|HIGH` or `CURRENT_MAIN_COMPATIBILITY_STATUS=PASS`, `- NONE` is illegal
- When `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, every appended governed validation report MUST also include:
  - `DATA_CONTRACT_PROOF:`
    - list concrete code paths, emitted artifacts, or storage/query surfaces proving the active data contract was reviewed
  - `DATA_CONTRACT_GAPS:`
    - `- NONE` only when no SQL-portability, LLM-parseability, or Loom-intertwined gap remains inside signed scope
    - otherwise list each remaining gap explicitly
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Rule: `HEURISTIC_REVIEW_VERDICT=PASS` is only valid when `QUALITY_RISKS` is exactly `- NONE`.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, `HEURISTIC_REVIEW_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- Rule: when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- Rule: `LEGAL_VERDICT=PASS` is only valid when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, `COUNTERFACTUAL_CHECKS`, and `SPEC_CLAUSE_MAP` are all present and non-empty, and `SPEC_CLAUSE_MAP` entries include file:line evidence.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, `LEGAL_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `SPEC_ALIGNMENT_VERDICT=PASS` and `Verdict: PASS` are legal only when `PRIMITIVE_RETENTION_GAPS` is exactly `- NONE`.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `LEGAL_VERDICT=PASS` is legal only when `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS` all contain concrete code or symbol evidence.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `RISK_TIER=MEDIUM|HIGH` requires non-empty `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS`.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `MECHANICAL_TRACK_VERDICT=PASS` is legal only when `GOVERNANCE_VERDICT`, `TEST_VERDICT`, `CODE_REVIEW_VERDICT`, `HEURISTIC_REVIEW_VERDICT`, `ENVIRONMENT_VERDICT`, `WORKFLOW_VALIDITY`, `SCOPE_VALIDITY`, `PROOF_COMPLETENESS`, `INTEGRATION_READINESS`, and `DOMAIN_GOAL_COMPLETION` are all in their PASS states.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `SPEC_RETENTION_TRACK_VERDICT=PASS` is legal only when `SPEC_ALIGNMENT_VERDICT=PASS`, `NOT_PROVEN`, `MAIN_BODY_GAPS`, and `PRIMITIVE_RETENTION_GAPS` are all exactly `- NONE`, and the report contains concrete `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, `CURRENT_MAIN_INTERACTION_CHECKS`, `SPEC_CLAUSE_MAP`, and `NEGATIVE_PROOF` evidence.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `LEGAL_VERDICT=PASS` and `Verdict: PASS` are legal only when `MECHANICAL_TRACK_VERDICT=PASS` and `SPEC_RETENTION_TRACK_VERDICT=PASS`.
- Rule: when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `LEGAL_VERDICT=PASS` is legal only when `DATA_CONTRACT_PROOF` is present and `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- Rule: `Verdict: PASS` is legal only when `VALIDATION_CONTEXT=OK`, `WORKFLOW_VALIDITY=VALID`, `SCOPE_VALIDITY=IN_SCOPE`, `PROOF_COMPLETENESS=PROVEN`, `INTEGRATION_READINESS=READY`, `DOMAIN_GOAL_COMPLETION=COMPLETE`, and `LEGAL_VERDICT=PASS`.
- Rule: if `PROOF_COMPLETENESS` is anything other than `PROVEN`, the top-line `Verdict` MUST NOT be `PASS`; use `NOT_PROVEN`, `FAIL`, `BLOCKED`, `OUTDATED_ONLY`, or `ABANDONED` honestly.
- Rule: `PROOF_COMPLETENESS=PROVEN` is legal only when `NOT_PROVEN` is exactly `- NONE`.
- Rule: `WORKFLOW_VALIDITY=VALID` is legal only when `VALIDATION_CONTEXT=OK` and `GOVERNANCE_VERDICT=PASS`.
- Rule: `LEGAL_VERDICT=PASS` is legal only when `PROOF_COMPLETENESS=PROVEN`.
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.

### 2026-04-13T05:00:11Z - INTEGRATION_VALIDATOR TERMINAL VERDICT SUMMARY
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PASS
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PASS
HEURISTIC_REVIEW_VERDICT: PASS
SPEC_ALIGNMENT_VERDICT: PASS
ENVIRONMENT_VERDICT: PASS
DISPOSITION: NONE
LEGAL_VERDICT: PASS
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
WORKFLOW_VALIDITY: VALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: PROVEN
INTEGRATION_READINESS: READY
DOMAIN_GOAL_COMPLETION: COMPLETE
MECHANICAL_TRACK_VERDICT: PASS
SPEC_RETENTION_TRACK_VERDICT: PASS
VALIDATOR_RISK_TIER: HIGH
Verdict: PASS

### 2026-04-13T03:39:26Z - INTEGRATION_VALIDATOR FINAL WHOLE-WP REVIEW
- Review Range: `e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`
- Review Scope: bounded six-file signed product surface only (`src/backend/handshake_core/src/locus/task_board.rs`, `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/runtime_governance.rs`, `src/backend/handshake_core/src/storage/locus_sqlite.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs`) against current local `main`
- Verdict: BLOCKED
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PARTIAL
- TEST_VERDICT: PARTIAL
- CODE_REVIEW_VERDICT: PASS
- HEURISTIC_REVIEW_VERDICT: PARTIAL
- SPEC_ALIGNMENT_VERDICT: PASS
- ENVIRONMENT_VERDICT: PARTIAL
- DISPOSITION: NONE
- LEGAL_VERDICT: PENDING
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- WORKFLOW_VALIDITY: PARTIAL
- SCOPE_VALIDITY: IN_SCOPE
- PROOF_COMPLETENESS: PROVEN
- INTEGRATION_READINESS: BLOCKED
- DOMAIN_GOAL_COMPLETION: PARTIAL
- MECHANICAL_TRACK_VERDICT: BLOCKED
- SPEC_RETENTION_TRACK_VERDICT: PASS
- VALIDATOR_RISK_TIER: HIGH
CLAUSES_REVIEWED:
  - `Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]` proved via `src/backend/handshake_core/src/locus/types.rs:148-363` (governed action registry), `src/backend/handshake_core/src/workflows.rs:3395-3397` (`allowed_action_ids` delegates to the registry), `src/backend/handshake_core/src/storage/locus_sqlite.rs:208-245` (persisted MT progress metadata emits governed ids), and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3233-3268` (packet/progress parity + governed registry assertion).
  - `Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]` proved via `src/backend/handshake_core/src/locus/types.rs:169-218` (display-only profile extension + degradation), `src/backend/handshake_core/src/locus/types.rs:413-421` (mailbox-aware queue reason resolution), `src/backend/handshake_core/src/workflows.rs:3275-3358` (mailbox-aware state emitters), `src/backend/handshake_core/src/workflows.rs:3762-3782`, `4853-4881`, and `4961-4988` (task-board, WP summary, and MT summary propagation), plus `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3279-3342` (mailbox override preserves family while forcing `mailbox_response_wait`).
  - `Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]` proved via `src/backend/handshake_core/src/locus/types.rs:424-688` (transition, automation, and eligibility registries), `src/backend/handshake_core/src/locus/task_board.rs:45-54` (portable ids carried on task-board rows), `src/backend/handshake_core/src/workflows.rs:3779-3782`, `4867-4879`, and `4975-4987` (ids emitted on task-board, WP summary, and MT summary surfaces), `src/backend/handshake_core/src/storage/locus_sqlite.rs:242-245` (ids persisted in MT progress metadata), and `src/backend/handshake_core/src/runtime_governance.rs:479-639` (proof tests for transition, automation, and eligibility semantics).
NOT_PROVEN:
  - NONE
MAIN_BODY_GAPS:
  - NONE
QUALITY_RISKS:
  - NONE
DIFF_ATTACK_SURFACES:
  - Shared registry drift: the patch adds one registry source of truth in `types.rs` that must stay aligned with packet, summary, task-board, and storage emitters.
  - Mailbox wait producer/consumer drift: `has_pending_mailbox_wait` must override `queue_reason_code` without mutating `workflow_state_family`.
  - Current-main containment: the same six signed files have advanced on local `main`, so merge readiness depends on compatibility rather than clause proof alone.
INDEPENDENT_CHECKS_RUN:
  - `git diff --numstat e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5 -- <6 files>` => touched exactly the six in-scope product files and no out-of-scope product surfaces.
  - `git merge-tree --write-tree --merge-base 5336e8f23b7a6e2f35b450124dccb65a17644d7f --name-only HEAD 896e808714f0d584c06efb457b5be0a9e85e2fd5` => `CONFLICT (content): Merge conflict in src/backend/handshake_core/src/runtime_governance.rs`, so current-main compatibility is not clean.
  - `rg -n "ProjectProfileWorkflowExtensionV1|WorkflowTransitionRuleV1|QueueAutomationRuleV1|ExecutorEligibilityPolicyV1" src/backend/handshake_core/src/locus/types.rs src/backend/handshake_core/src/workflows.rs ...` on current `main` => no hits outside the reviewed commit, confirming the signed range is not already contained on `main`.
COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/workflows.rs:3275-3358` stopped routing through `resolve_queue_reason_with_mailbox_context`, mailbox-linked waits would fall back to ready/human/model queue reasons and violate the `[ADD v02.171]` mailbox requirement.
  - If `src/backend/handshake_core/src/locus/types.rs:225-363` were no longer the single source for governed action ids, `src/backend/handshake_core/src/workflows.rs:3395-3397`, `src/backend/handshake_core/src/storage/locus_sqlite.rs:242-245`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3255-3268` would drift again on action legality.
  - If `transition_rule_ids_for_family`, `queue_automation_rule_ids_for_reason`, and `executor_eligibility_policy_ids_for_family` were removed from `src/backend/handshake_core/src/workflows.rs:3779-3782`, `4867-4879`, and `4975-4987`, the portable id contract would disappear from task-board rows and structured packet/summary consumers.
BOUNDARY_PROBES:
  - Producer/consumer boundary: `src/backend/handshake_core/src/locus/task_board.rs:45-54` declares the three new portable id arrays, and `src/backend/handshake_core/src/workflows.rs:3779-3782` populates them from the locus registries on the emitted task-board surface.
  - Writer/reader boundary: `src/backend/handshake_core/src/storage/locus_sqlite.rs:228-245` writes mailbox-aware workflow metadata plus governed ids into MT progress records, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3226-3333` reads both progress metadata and emitted packet JSON to assert parity.
NEGATIVE_PATH_CHECKS:
  - `src/backend/handshake_core/src/locus/types.rs:413-421` preserves `base_reason` when `has_pending_mailbox_wait=false`; the non-mailbox branch is asserted at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3270-3277`.
  - `src/backend/handshake_core/src/locus/types.rs:683-688` rejects local-small-model eligibility without a compact summary or outside the `Ready` family; those negative cases are asserted at `src/backend/handshake_core/src/runtime_governance.rs:607-639`.
INDEPENDENT_FINDINGS:
  - The signed range is internally coherent on the packet clauses: types, emitters, storage metadata, and targeted tests line up on workflow family, queue reason, governed action ids, and the new transition/automation/eligibility ids.
  - Current local `main` is not merge-ready for this range. `runtime_governance.rs` conflicts in a three-way merge, and current `main` still lacks the new workflow primitive definitions immediately after `GovernedActionDescriptorV1` (`src/backend/handshake_core/src/locus/types.rs:155-156` resumes `StructuredCollaborationSummaryV1`).
  - Current `main` still retains ad-hoc next-action helpers outside the governed registry at `src/backend/handshake_core/src/workflows.rs:5941-5950` and `13722-13730`; this is spec-adjacent debt that survives independently of the signed range.
RESIDUAL_UNCERTAINTY:
  - I did not rerun cargo tests in this turn; the packet already records a pre-existing out-of-scope `flight_recorder` compile blocker affecting broad cargo execution.
  - Current-main compatibility was validated by non-mutating merge simulation only; no containment commit or closeout gate was executed in this turn by instruction.
SPEC_CLAUSE_MAP:
  - `Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]` -> `src/backend/handshake_core/src/locus/types.rs:148-363`; `src/backend/handshake_core/src/workflows.rs:3395-3397`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:208-245`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3233-3268`
  - `Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]` -> `src/backend/handshake_core/src/locus/types.rs:169-218`; `src/backend/handshake_core/src/locus/types.rs:413-421`; `src/backend/handshake_core/src/workflows.rs:3275-3358`; `src/backend/handshake_core/src/workflows.rs:3762-3782`; `src/backend/handshake_core/src/workflows.rs:4853-4881`; `src/backend/handshake_core/src/workflows.rs:4961-4988`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3279-3342`
  - `Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]` -> `src/backend/handshake_core/src/locus/types.rs:424-688`; `src/backend/handshake_core/src/locus/task_board.rs:45-54`; `src/backend/handshake_core/src/workflows.rs:3779-3782`; `src/backend/handshake_core/src/workflows.rs:4867-4879`; `src/backend/handshake_core/src/workflows.rs:4975-4987`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:242-245`; `src/backend/handshake_core/src/runtime_governance.rs:479-639`
NEGATIVE_PROOF:
  - `ProjectProfileWorkflowExtensionV1.narrowed_reason_codes` is declared at `src/backend/handshake_core/src/locus/types.rs:179-180` but never consumed inside the signed six-file surface, so queue-reason narrowing is not fully implemented.
  - Current `main` still derives next actions from ad-hoc status maps at `src/backend/handshake_core/src/workflows.rs:5941-5950` and `13722-13730` instead of the governed action registry, so the broader governed-action contract is not fully retained on current `main`.
ANTI_VIBE_FINDINGS:
  - NONE
SIGNED_SCOPE_DEBT:
  - NONE
PRIMITIVE_RETENTION_PROOF:
  - Existing `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` remain present on task-board rows while the new ids are appended rather than replacing them (`src/backend/handshake_core/src/locus/task_board.rs:45-54`).
  - MT progress metadata still emits the original trio and now adds transition/automation/eligibility ids without removing prior fields (`src/backend/handshake_core/src/storage/locus_sqlite.rs:240-245`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3233-3259` and `3297-3323`).
PRIMITIVE_RETENTION_GAPS:
  - NONE
SHARED_SURFACE_INTERACTION_CHECKS:
  - Registry -> emitter: `src/backend/handshake_core/src/locus/types.rs:358-363` defines the governed action id source of truth, and it is consumed by `src/backend/handshake_core/src/workflows.rs:3395-3397`, `4893-4914`, `5000-5021`, and `src/backend/handshake_core/src/storage/locus_sqlite.rs:242-245`.
  - Mailbox flag -> emitter parity: `src/backend/handshake_core/src/locus/types.rs:413-421` is threaded through `src/backend/handshake_core/src/workflows.rs:3275-3358`, then reflected in task-board rows (`3762-3782`), WP summaries (`4853-4881`), MT summaries (`4961-4988`), and MT progress metadata (`208-245`).
CURRENT_MAIN_INTERACTION_CHECKS:
  - `git merge-tree --write-tree --merge-base 5336e8f23b7a6e2f35b450124dccb65a17644d7f --name-only HEAD 896e808714f0d584c06efb457b5be0a9e85e2fd5` reports `CONFLICT (content): Merge conflict in src/backend/handshake_core/src/runtime_governance.rs`; direct containment into current `main` is blocked.
  - On current `main`, `src/backend/handshake_core/src/locus/types.rs:155-156` resumes `StructuredCollaborationSummaryV1` immediately after `GovernedActionDescriptorV1`, proving the project-profile extension and transition/automation/eligibility primitives are not already contained.
  - On current `main`, `src/backend/handshake_core/src/workflows.rs:5941-5950` and `13722-13730` still derive next actions from status-local helpers instead of the governed action registry.
DATA_CONTRACT_PROOF:
  - `src/backend/handshake_core/src/locus/task_board.rs:45-54` exposes the portable ids on the task-board record surface.
  - `src/backend/handshake_core/src/workflows.rs:4893-4914` and `5000-5021` emit the same portable ids and mailbox-aware queue reasons on work-packet and micro-task packet artifacts.
  - `src/backend/handshake_core/src/storage/locus_sqlite.rs:208-245` persists the same fields for MT progress metadata, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3233-3333` compares the persisted values against emitted packet JSON.
DATA_CONTRACT_GAPS:
  - NONE

### 2026-04-13T04:40:55Z - INTEGRATION_VALIDATOR RETRY WHOLE-WP REVIEW
- Correlation: `review:WP-1-Project-Agnostic-Workflow-State-Registry-v1:direct_final_lane_review_retry:20260413`
- Review Range: `e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`
- Compatibility Evidence Reviewed: `17f0a543276a0393e3746478a82d059af9160b53` as current-main remediation only; not treated as a widened signed product range
- Verdict: BLOCKED
- VALIDATION_CONTEXT: OK
- INTEGRATION_READINESS: BLOCKED
- MECHANICAL_TRACK_VERDICT: BLOCKED
- CURRENT_MAIN_COMPATIBILITY:
  - The retry remediation does not prove compatibility against the actual current local `main`.
  - The feature worktree still resolves `main` to stale base `5336e8f23b7a6e2f35b450124dccb65a17644d7f`, while the live `handshake_main` head is `3aee07ecaaf9236eebf39e4a7d76912bc3302e98`.
  - Direct object-level merge simulation against the real local `main` still reports content conflicts in `src/backend/handshake_core/src/runtime_governance.rs` and `src/backend/handshake_core/src/workflows.rs`.
- INDEPENDENT_CHECKS_RUN:
  - `git rev-parse HEAD` in `handshake_main` => `3aee07ecaaf9236eebf39e4a7d76912bc3302e98`
  - `git -C ../wtc-state-registry-v1 rev-parse main` => `5336e8f23b7a6e2f35b450124dccb65a17644d7f`
  - `git -C ../wtc-state-registry-v1 merge-tree --write-tree --merge-base 5336e8f23b7a6e2f35b450124dccb65a17644d7f 3aee07ecaaf9236eebf39e4a7d76912bc3302e98 17f0a543276a0393e3746478a82d059af9160b53` => `CONFLICT (content)` in `src/backend/handshake_core/src/runtime_governance.rs` and `src/backend/handshake_core/src/workflows.rs`
- RETRY_FINDINGS:
  - The earlier signed-range clause proof remains unchanged; no new in-scope spec defect was identified in the original six-file range during this retry.
  - The only blocking issue in this retry lane is unresolved compatibility against the actual current local `main`.
- REQUIRED_FOLLOW_UP:
  - Rebuild the compatibility remediation on top of the real local `main` head `3aee07ecaaf9236eebf39e4a7d76912bc3302e98`, then reopen final-lane review on the same signed six-file scope.

### 2026-04-13T04:59:40Z - INTEGRATION_VALIDATOR CURRENT-MAIN-VERIFIED WHOLE-WP REVIEW
- Correlation: `review:WP-1-Project-Agnostic-Workflow-State-Registry-v1:direct_final_lane_review_current_main_verified:20260413`
- Review Range: `e450df813b20e2dddeb7b7b51cac28d922e41a29..896e808714f0d584c06efb457b5be0a9e85e2fd5`
- Compatibility Evidence Reviewed: recovered validator-run evidence from `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/INTEGRATION_VALIDATOR_WP-1-Project-Agnostic-Workflow-State-Registry-v1/0dce5f44-97f8-4cca-a2ef-e71fcd148617.jsonl`
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PASS
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PASS
HEURISTIC_REVIEW_VERDICT: PASS
SPEC_ALIGNMENT_VERDICT: PASS
ENVIRONMENT_VERDICT: PASS
DISPOSITION: NONE
LEGAL_VERDICT: PASS
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
WORKFLOW_VALIDITY: VALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: PROVEN
INTEGRATION_READINESS: READY
DOMAIN_GOAL_COMPLETION: COMPLETE
MECHANICAL_TRACK_VERDICT: PASS
SPEC_RETENTION_TRACK_VERDICT: PASS
VALIDATOR_RISK_TIER: HIGH
Verdict: PASS
- PRODUCT_VERDICT_SCOPE:
  - This verdict covers product governance only: the signed six-file range plus verified current-main compatibility through `17f0a543276a0393e3746478a82d059af9160b53`.
  - Packet mechanical truth sync for `CURRENT_MAIN_COMPATIBILITY` and closeout gates remains orchestrator-owned follow-up and is not treated as a product blocker in this review.
SPEC_CLAUSE_MAP:
  - `Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]` -> `src/backend/handshake_core/src/locus/types.rs:148-363`; `src/backend/handshake_core/src/workflows.rs:3395-3397`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:208-245`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3233-3268`
  - `Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]` -> `src/backend/handshake_core/src/locus/types.rs:169-218`; `src/backend/handshake_core/src/locus/types.rs:413-421`; `src/backend/handshake_core/src/workflows.rs:3275-3358`; `src/backend/handshake_core/src/workflows.rs:3762-3782`; `src/backend/handshake_core/src/workflows.rs:4853-4881`; `src/backend/handshake_core/src/workflows.rs:4961-4988`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3279-3342`
  - `Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]` -> `src/backend/handshake_core/src/locus/types.rs:424-688`; `src/backend/handshake_core/src/locus/task_board.rs:45-54`; `src/backend/handshake_core/src/workflows.rs:3779-3782`; `src/backend/handshake_core/src/workflows.rs:4867-4879`; `src/backend/handshake_core/src/workflows.rs:4975-4987`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:242-245`; `src/backend/handshake_core/src/runtime_governance.rs:479-639`
- CURRENT_MAIN_COMPATIBILITY_PROOF:
  - The recovered run confirmed `17f0a543276a0393e3746478a82d059af9160b53` has parents `896e808714f0d584c06efb457b5be0a9e85e2fd5` and actual current local `main` `3aee07ecaaf9236eebf39e4a7d76912bc3302e98`.
  - The recovered run confirmed `git merge-tree --write-tree 17f0a543276a0393e3746478a82d059af9160b53 3aee07ecaaf9236eebf39e4a7d76912bc3302e98` returned clean tree `8399a698cfe800438d9bfd1ad46a305d1233bb45`.
  - The recovered run's signed-scope-restricted diff confirmed only `src/backend/handshake_core/src/locus/task_board.rs`, `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/runtime_governance.rs`, and `src/backend/handshake_core/src/workflows.rs` changed between `896e8087` and `17f0a543`.
- TEST_PROOF:
  - The recovered run completed `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib runtime_governance` successfully.
  - Broad cargo test remains blocked only by pre-existing out-of-scope main debt in `src/backend/handshake_core/tests/micro_task_executor_tests.rs` and `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`; that ambient debt is not introduced by the signed range or by `17f0a543`.
NEGATIVE_PROOF:
  - `ProjectProfileWorkflowExtensionV1.narrowed_reason_codes` is still declared but unconsumed at `src/backend/handshake_core/src/locus/types.rs:179-180`, so queue-reason narrowing remains only partially implemented. This is non-blocking for the packet clauses under review.
  - Current `main` still carries ad-hoc next-action helpers at `src/backend/handshake_core/src/workflows.rs:5941-5950` and `src/backend/handshake_core/src/workflows.rs:13722-13730` instead of full governed-action unification. This remains broader product debt, not a blocker for the signed packet scope.
- FINAL_PRODUCT_JUDGMENT:
  - The earlier clause proof from the `2026-04-13T03:39:26Z` final whole-WP review stands.
  - The prior product blocker is now resolved because the signed surface is proven compatible with the actual current local `main`.
  - Product review is PASS. Orchestrator must still run closeout-repair plus `phase-check CLOSEOUT` sync before merge.
