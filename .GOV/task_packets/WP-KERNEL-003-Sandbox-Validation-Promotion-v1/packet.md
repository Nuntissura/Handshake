<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.work_packet_contract@1 source_file=.GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/packet.json source_hash=bd48f15f8f96e894 projection_hash=1683294690083c18 generated_at_utc=2026-05-17T16:19:22.497Z generator=ensure-wp-communications.mjs -->
# TASK_PACKET_TEMPLATE

Generated projection template for `packet.md` during the contract migration. Do not hand-copy this Markdown into future work as authority; author or update `packet.json` / `WORK_PACKET_CONTRACT_TEMPLATE.json` first, then generate a projection only when a current contract or explicit Operator request requires one.

Requirements:
- Keep packets ASCII-only (required by deterministic gates).
- Use SPEC_BASELINE for provenance (spec at creation time).
- Use SPEC_TARGET as the authoritative spec for closure/revalidation (usually .GOV/spec/SPEC_CURRENT.md).
- WP_ID and filename MUST NOT include date/time stamps; use `-v{N}` for revisions (e.g., `WP-1-Tokenization-Service-v3`).
- If multiple packets exist for the same Base WP, update `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet).
- `packet.json` is the authoritative lifecycle truth; this generated packet metadata is a compatibility projection. `TASK_BOARD.md`, `WP_TRACEABILITY_REGISTRY.md`, and `BUILD_ORDER.md` are projections and must reconcile to the contract.
- Legacy Markdown packets remain migration safety rails only. They must not be copied forward as the model-created artifact pattern for new packets, refinements, or microtasks.
- Active packet rule: the packet mapped by `BASE_WP_ID` in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` is the current contract. Any other official packet with the same `BASE_WP_ID` is older history and must be tracked as `SUPERSEDED` on the Task Board.
- For `REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1`, this packet is auto-hydrated from the signed refinement; manual drift is forbidden and `just phase-check STARTUP ... CODER` enforces alignment.

---

# Task Packet: WP-KERNEL-003-Sandbox-Validation-Promotion-v1

## METADATA
- TASK_ID: WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- WP_ID: WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- BASE_WP_ID: WP-KERNEL-003-Sandbox-Validation-Promotion
- DATE: 2026-05-15T00:40:02.917Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just phase-check HANDOFF ... CODER --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: Kernel Builder
- ROLE: Kernel Builder
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: CODER_A
- KERNEL_BUILDER_CONSOLIDATION: YES
- IMPLEMENTATION_ROLE: KERNEL_BUILDER
- CODER_COMPATIBILITY_LANE: CODER_A
- WP_VALIDATOR_GATE: DISABLED
- VALIDATION_TOPOLOGY: INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1
<!-- Required before packet creation: CODER_A .. CODER_Z -->
- WORKFLOW_AUTHORITY: ORCHESTRATOR
<!-- Current repo-governance owner for workflow steering and hard-gate progression. -->
- TECHNICAL_ADVISOR: NONE
<!-- WP Validator gate is disabled for this packet; Integration Validator batch review remains the separate technical authority. -->
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
<!-- Final technical verdict authority across the WP batch. -->
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
<!-- Final merge-to-main authority. -->
- AGENTIC_MODE: NO
<!-- Allowed: YES | NO. Default NO; set to YES only with explicit operator-authorized sub-agent use. -->
- ORCHESTRATOR_MODEL: N/A
<!-- Required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Optional but authoritative when Activation Manager launch or repair resumes from the packet. -->
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- CODER_MODEL: gpt-5.5
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
- SESSION_COMPATIBILITY_SURFACE: VSCODE_PLUGIN_DISABLED
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
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.5
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.4
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-KERNEL-003-Sandbox-Validation-Promotion-v1
<!-- WP Validator session/worktree fields are intentionally N/A for this packet. Kernel Builder implementation uses the coder-compatible lane; Integration Validator stays on handshake_main/main for separate batch review. -->
- WP_VALIDATOR_MODEL_PROFILE: N/A
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: N/A
- WP_VALIDATOR_REASONING_STRENGTH: N/A
- WP_VALIDATOR_LOCAL_BRANCH: N/A
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: N/A
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: N/A
- WP_VALIDATOR_REMOTE_BACKUP_URL: N/A
- WP_VALIDATOR_STARTUP_COMMAND: N/A
- WP_VALIDATOR_RESUME_COMMAND: N/A
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.5
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-KERNEL-003-Sandbox-Validation-Promotion-v1
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
- MERGED_MAIN_COMMIT: a813b7615f4e29d77ed8ee0b3ea8845ad5fd406f
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-05-17T15:45:00Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NONE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NONE | NOT_REQUIRED | FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED -->
- PACKET_WIDENING_EVIDENCE: N/A
<!-- Use follow-on/superseding WP id, audit id, or short rationale when widening is required. -->
- ZERO_DELTA_PROOF_ALLOWED: NO
<!-- Allowed: YES | NO. YES => deterministic post-work may accept an empty diff only for an explicitly proof-only/status-sync packet. -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: YES
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-KERNEL-001-Event-Ledger-Session-Broker, WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening, WP-1-Product-Governance-Check-Runner, WP-1-Product-Governance-Artifact-Registry, WP-1-Unified-Tool-Surface-Contract, WP-1-Capability-SSoT, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Workflow-Engine, WP-1-Flight-Recorder, WP-1-Artifact-System-Foundations, WP-1-Micro-Task-Executor, WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Postgres-Primary-Control-Plane-Foundation
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-KERNEL-004-Local-Model-Memory-Runtime, WP-1-MTE-Resource-Caps, WP-1-MTE-Blocked-Decisioning, WP-1-MTE-Summaries, WP-1-MTE-DropBack-Smart, WP-1-MCP-MEX-Evidence-Export, WP-1-Diagnostics-Debug-Bundle-Bridge
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: YES
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: WP-KERNEL-003-Sandbox-Validation-Promotion-v1
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- LOCAL_WORKTREE_DIR: ../wtc-validation-promotion-v1
- REMOTE_BACKUP_BRANCH: feat/WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-KERNEL-003-Sandbox-Validation-Promotion-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: NONE
- INTEGRATION_VALIDATOR_OF_RECORD: INTEGRATION_VALIDATOR-20260517-1545Z
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: INTEGRATION_BATCH_REVIEW_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja150520260208
- PACKET_FORMAT_VERSION: 2026-04-06
- AUTHORITATIVE_CONTRACT_SCHEMA_ID: hsk.work_packet_contract@1
- AUTHORITATIVE_CONTRACT_FILE: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/packet.json
- GENERATED_MARKDOWN_PROJECTION_FILE: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/packet.md
- REFINEMENT_CONTRACT_SCHEMA_ID: hsk.refinement_contract@1
- REFINEMENT_CONTRACT_FILE: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/refinement.json
- MICROTASK_CONTRACT_SCHEMA_ID: hsk.microtask_contract@1
- MICROTASK_CONTRACT_GLOB: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/MT-*.json
- MARKDOWN_PROJECTION_STATUS: GENERATED_IN_SYNC
<!-- Allowed: GENERATED_PENDING | GENERATED_IN_SYNC | LEGACY_AUTHORITY | BLOCKED. New packets use packet.json/refinement.json/MT-*.json as authoritative deterministic contracts; packet.md/refinement.md/MT-*.md are generated projections or frozen legacy references, not sidecar authority. -->
- RED_TEAM_REQUIRED: YES
- RED_TEAM_PROFILE: DETERMINISTIC_CONTRACT_MIGRATION_V1
<!-- Assume stale projections, shadow prose authority, schema omissions, round-trip loss, lifecycle split drift, and Activation Manager / Classic Orchestrator divergence until machine checks prove otherwise. -->

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: Implementation is in progress; awaiting coder handoff to WP validator.
Next: CODER completes in-scope work and records CODER_HANDOFF with proof.
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: MT-001 Activation Source Inventory: re-scan stubs, Task Board, Build Order, and traceability for every KB003-related source. Acceptance: packet contains a source fold table at least as detailed as the stub. | CODE_SURFACES: .GOV/task_packets/**, .GOV/roles_shared/records/** | TESTS: just gov-check | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-002 Conflict Deliberation Record: convert conflict register into signed decisions for raw shell, direct mutation, container-only, SQLite, and domain-evidence bloat. Acceptance: conflicts are approved, rejected, or parked, none silently removed. | CODE_SURFACES: .GOV/refinements/**, .GOV/task_packets/** | TESTS: just gov-check | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-003 Current Product API Inventory: inspect product modules for KB001/KB002 and existing check/artifact/governance APIs. Acceptance: packet targets real files or declares missing upstream blockers. | CODE_SURFACES: src/backend/handshake_core/src/**, src/backend/handshake_core/tests/** | TESTS: rg EventLedger src/backend/handshake_core; rg WriteBox src/backend/handshake_core | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-004 Research Basis Update: compare current sandbox adapter and validation evidence options before implementation. Acceptance: selected adapter sequence and rejected options are documented. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, README.md | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-005 Official Packet Contract Generation: promote stub into signed official packet with contracts and 80 MTs. Acceptance: packet is ready but not self-validated by Kernel Builder. | CODE_SURFACES: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/** | TESTS: just gov-check | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-006 Product Module Placement Decision: decide where sandbox, validation, and promotion modules live. Acceptance: module topology is documented before scaffolding. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-007 Kernel003 Schema Namespace: define stable schema IDs for KernelSandboxRunV1, SandboxPolicyV1, SandboxWorkspaceV1, SandboxArtifactBundleV1, ValidationRunV1, PromotionDecisionV1, and PromotionReceiptV1. Acceptance: schema names are versioned and referenced by EventLedger events. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-008 EventLedger Event Type Plan: add Kernel003 event type names and payload expectations. Acceptance: every event carries run ID, actor, session, task, schema version, timestamp, and artifact refs. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_event_ledger --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-009 Artifact Type Plan: define sandbox and validation artifact classes for logs, diffs, manifests, screenshots, reports, redaction, and receipts. Acceptance: each artifact class has content type, hash policy, exportability default, and retention/default root. | CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_artifact --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-010 DCC Projection Contract: define minimum operator projection for sandbox and promotion state. Acceptance: no-context model can inspect state without terminal logs. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-011 Postgres Migration for Sandbox Runs: add authority tables for sandbox runs. Acceptance: records persist and replay after backend restart. | CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-012 Postgres Migration for Sandbox Policies: persist versioned sandbox policies. Acceptance: policy changes are versioned and traceable. | CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_policy --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-013 Postgres Migration for Validation Runs: persist validation run metadata and summaries. Acceptance: validation results reconstruct without file-system-only state. | CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_validation_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-014 Postgres Migration for Promotion Receipts: persist decisions and receipts. Acceptance: duplicate idempotency keys are rejected or idempotently resolved. | CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_promotion_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-015 No SQLite Authority Tripwire: prevent Kernel003 authority from using SQLite in production or tests. Acceptance: Kernel003 authority fails closed without Postgres/EventLedger authority. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_no_sqlite --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-016 Replay Projection Storage Query: reconstruct a run from durable rows/events. Acceptance: replay does not read provider chat, terminal scrollback, or transient logs. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-017 Legacy Compatibility Blocker Check: detect prerequisite API gaps. Acceptance: missing APIs produce BLOCKED with evidence, not parallel implementations. | CODE_SURFACES: src/backend/handshake_core/src/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_compat --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-018 SandboxAdapter Trait: define adapter boundary independent of Docker, WSL, Deno, or WASM. Acceptance: at least one adapter can be implemented without changing caller code. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-019 PolicyScopedLocal Adapter: implement minimum local proof adapter with strict policy checks. Acceptance: policy mode is explicitly not hard isolation and denies sensitive capabilities by default. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_policy_local --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-020 HardIsolation Adapter Stub: add non-executing adapter slot for hard isolation. Acceptance: hard isolation absence is typed BLOCKED/UNSUPPORTED, not success. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_hard_isolation --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-021 SandboxPolicy Default Deny: implement default-deny policy construction. Acceptance: omitted policy fields deny access. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_policy --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-022 Filesystem Scope Guard: enforce read/write roots and prevent path escape. Acceptance: all path escape attempts return typed denial evidence. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_fs_guard --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-023 Network Capability Gate: deny network unless policy grants it. Acceptance: network grants require approval/provenance refs. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_network --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-024 Process Execution Allowlist: permit only registered commands/checks. Acceptance: raw shell strings without descriptors are rejected. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_command_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-025 Environment and Secret Redaction: prevent env/secret leakage. Acceptance: secret-looking values are not emitted in stored logs or reports. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_redaction --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-026 Resource Cap Policy: fold MTE resource caps into sandbox policy. Acceptance: overage halts or gates deterministically with evidence. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_resource_caps --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-027 Cancellation and Timeout: add cancellation and timeout handling. Acceptance: cancelled runs cannot promote and have typed terminal state. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_timeout --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-028 Sandbox Workspace Materializer: materialize candidate inputs into isolated root. Acceptance: no undeclared project files appear in sandbox input manifest. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_workspace --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-029 Sandbox Cleanup and Retention: clean temp roots while preserving artifacts. Acceptance: cleanup never deletes project files or authority rows. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_cleanup --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-030 Sandbox Adapter Health Projection: expose adapter health/preflight state. Acceptance: unsupported isolation is visible before run. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, app/**, tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_health --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-031 PatchProposal Contract: define candidate patch envelope. Acceptance: proposals without base refs or target ranges cannot enter validation. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_patch_proposal --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-032 Candidate Range Truth: validate changed paths/ranges against declared targets. Acceptance: unexpected file edits are rejected before promotion. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_candidate_range --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-033 Diff Capture: capture candidate diffs as stable artifacts. Acceptance: identical candidate produces identical diff artifact hash. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_diff_capture --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-034 Artifact Bundle Manifest: create canonical bundle format. Acceptance: bundle hash is deterministic for same inputs. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_artifact_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-035 Stdout/Stderr Log Capture: store bounded command logs. Acceptance: logs never live only in terminal output. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_log_capture --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-036 Environment Manifest: record non-sensitive runtime environment identifiers. Acceptance: manifest explains run context without exposing secrets. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_environment_manifest --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-037 Command Manifest: record exactly what commands/checks ran. Acceptance: validators can replay or reason about command intent. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_command_manifest --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-038 Visual Evidence Attachment: attach KB002 screenshot/visual artifacts to validation reports. Acceptance: GUI reports can reference screenshots and DOM/log evidence. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_visual_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-039 Redaction Report: add redaction report to exportable bundles. Acceptance: default export is redacted and denied artifacts are listed. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_redaction_report --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-040 Artifact Store Integration: store sandbox artifacts through validated artifact system. Acceptance: every artifact has stable handle and hash. | CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_artifact_store --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-041 ValidationDescriptor Contract: define validation command/check descriptors. Acceptance: validation runner rejects undeclared raw commands. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_validation_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-042 Check Runner Adapter: reuse Product Governance Check Runner. Acceptance: no duplicate check runner is created. | CODE_SURFACES: src/backend/handshake_core/src/governance_check_runner.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_check_runner_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-043 Validation Result Schema: define result states and finding shapes. Acceptance: every non-PASS has typed reason and evidence refs. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_validation_result --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-044 Validation Preflight: preflight descriptors, tools, capabilities, policy mode, paths, and budget. Acceptance: missing tools produce BLOCKED/UNSUPPORTED, not silent skip. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_validation_preflight --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-045 Deterministic Check Batch: run deterministic validation before model review. Acceptance: blocking check failure prevents promotion. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_validation_batch --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-046 Validation Evidence Bundle: store validation outputs canonically. Acceptance: validation report can be inspected offline. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_validation_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-047 Finding Normalization: normalize check output into findings. Acceptance: raw logs are not the only finding source. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_finding_normalization --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-048 Advisory vs Blocking Rules: make blocking posture explicit. Acceptance: advisory failure is visible but does not block unless configured. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_validation_posture --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-049 Validation Replay: re-run descriptor set against same candidate. Acceptance: replay records new run ID linked to original. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_validation_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-050 Validation Report Projection: expose validation summaries to DCC/projection layer. Acceptance: operator/model can inspect validation without reading raw files first. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_validation_projection --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-051 PromotionCandidate Contract: define promotion candidate shape from patch proposal or write box. Acceptance: missing validation refs block promotion. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_promotion_candidate --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-052 Promotion Eligibility Check: implement promotion preconditions. Acceptance: ineligible candidate produces typed rejection receipt. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_promotion_eligibility --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-053 Promotion Accept Path: append accepted promotion events to EventLedger. Acceptance: accepted promotion is replayable from durable events. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_promotion_accept --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-054 Promotion Reject Path: record rejected promotion attempts. Acceptance: reject path creates receipt and does not mutate authority. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_promotion_reject --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-055 Idempotency Key Enforcement: prevent duplicate promotion effects. Acceptance: duplicate accept returns prior receipt or typed duplicate rejection without second mutation. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_promotion_idempotency --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-056 Approval Ref Binding: bind approval evidence to promotion decisions. Acceptance: promotion cannot accept without required approval posture. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_promotion_approval --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-057 Authority Mutation Boundary: sandbox and validation cannot mutate authority except through PromotionGate. Acceptance: direct mutation attempt produces denial evidence. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_authority_boundary --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-058 Promotion Closeout Bundle: implement canonical closeout bundle. Acceptance: Integration Validator can review one bundle for promotion. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_closeout_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-059 MTE Run Cap Integration: wire resource caps into sandboxed microtask execution. Acceptance: cap overage halts bounded run and writes evidence. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mte_caps --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-060 Blocked Reason Taxonomy: implement blocked decisioning for sandbox/validation. Acceptance: each blocked reason has retry/escalate/gate semantics. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_blocked_taxonomy --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-061 Retry Budget: bound retry behavior. Acceptance: retry exhaustion becomes typed BLOCKED/FAILED. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_retry_budget --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-062 Smart DropBack: implement smart drop-back semantics. Acceptance: smart/always/never modes have test coverage. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_dropback --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-063 Per-MT Summary Artifact: persist per-microtask summaries. Acceptance: every completed/blocked MT attempt has summary ref. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_summary --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-064 Aggregate Run Summary: persist aggregate summary across attempts. Acceptance: no-context reviewer can inspect aggregate before raw artifacts. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_aggregate_summary --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-065 Lane Wake Receipt: implement receipt-driven lane wake/settlement. Acceptance: wake/settlement event includes receipt refs and reason. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_lane_wake --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-066 Bootstrap Skeleton Receipt Projection: first skeleton sandbox run creates restartable receipts. Acceptance: all receipts visible after restart. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_sandbox_skeleton --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-067 DCC Sandbox Run List: add projection/API for sandbox run list. Acceptance: operator can find current and past sandbox runs. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc_sandbox_list --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-068 DCC Run Detail: add projection/API for sandbox run detail. Acceptance: detail view has no hidden dependency on terminal scrollback. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc_run_detail --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-069 DCC Promotion Control State: expose promotion eligibility and approval state. Acceptance: UI/API cannot promote when eligibility is false. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc_promotion_state --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-070 Debug Bundle Bridge: fold diagnostics debug bundle into evidence output. Acceptance: diagnostics evidence is bounded and portable. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_debug_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-071 MCP/MEX Evidence Export Bridge: fold tool/mechanical engine evidence into sandbox evidence. Acceptance: MCP/MEX evidence does not use ad hoc bundle schema. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mex_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-072 Capability Audit Evidence Link: link grants/denials to capability evidence. Acceptance: every sensitive grant has provenance. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_capability_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-073 Visual Validation Gate Descriptor: define visual evidence validation mapping. Acceptance: visual evidence can block or advise according to descriptor posture. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_visual_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-074 Console and Network Evidence: capture browser/app console and network evidence for GUI checks. Acceptance: GUI validation failures are diagnosable. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_gui_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-075 Security Denial Test Matrix: add negative tests for sandbox boundaries. Acceptance: every denial writes typed evidence. | CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | TESTS: cargo test -p handshake_core kernel_security_denial --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-076 Promotion Failure Test Matrix: test each promotion failure scenario. Acceptance: no failure mutates authority. | CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | TESTS: cargo test -p handshake_core kernel_promotion_failure --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-077 Restart Replay Test: prove state survives restart. Acceptance: replay is complete from durable product state. | CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | TESTS: cargo test -p handshake_core kernel_restart_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-078 Disk-Agnostic Path Test: prove paths remain repo-root/config relative. Acceptance: moving workspace root does not break path resolution. | CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | TESTS: cargo test -p handshake_core kernel_disk_agnostic --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-079 Documentation and Model Manual Update: update product-local model-facing manual. Acceptance: no-context model can run/inspect sandbox workflow from durable docs. | CODE_SURFACES: README.md, src/backend/handshake_core/src/kernel/**, app/** | TESTS: just gov-check | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-080 Integration Validator Handoff: prepare final validation bundle without self-validating. Acceptance: Kernel Builder/Coder does not claim PASS/FAIL and validator has sufficient evidence. | CODE_SURFACES: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/**, src/backend/handshake_core/tests/** | TESTS: just gov-check; cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## PACKET_ACCEPTANCE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the executable acceptance contract for packet closure. New packets must keep stable row IDs and move each required row to PROVED, CONFIRMED, or NOT_APPLICABLE with evidence before PASS.
- Rule: use STEER or BLOCKED for unresolved required rows instead of narrative closure.
- ACCEPTANCE_ROWS:
  - ID: AC-001 | REQUIREMENT: MT-001 Activation Source Inventory: re-scan stubs, Task Board, Build Order, and traceability for every KB003-related source. Acceptance: packet contains a source fold table at least as detailed as the stub. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: just gov-check | REASON: NONE
  - ID: AC-002 | REQUIREMENT: MT-002 Conflict Deliberation Record: convert conflict register into signed decisions for raw shell, direct mutation, container-only, SQLite, and domain-evidence bloat. Acceptance: conflicts are approved, rejected, or parked, none silently removed. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: just gov-check | REASON: NONE
  - ID: AC-003 | REQUIREMENT: MT-003 Current Product API Inventory: inspect product modules for KB001/KB002 and existing check/artifact/governance APIs. Acceptance: packet targets real files or declares missing upstream blockers. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: rg EventLedger src/backend/handshake_core; rg WriteBox src/backend/handshake_core | REASON: NONE
  - ID: AC-004 | REQUIREMENT: MT-004 Research Basis Update: compare current sandbox adapter and validation evidence options before implementation. Acceptance: selected adapter sequence and rejected options are documented. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-005 | REQUIREMENT: MT-005 Official Packet Contract Generation: promote stub into signed official packet with contracts and 80 MTs. Acceptance: packet is ready but not self-validated by Kernel Builder. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: just gov-check | REASON: NONE
  - ID: AC-006 | REQUIREMENT: MT-006 Product Module Placement Decision: decide where sandbox, validation, and promotion modules live. Acceptance: module topology is documented before scaffolding. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-007 | REQUIREMENT: MT-007 Kernel003 Schema Namespace: define stable schema IDs for KernelSandboxRunV1, SandboxPolicyV1, SandboxWorkspaceV1, SandboxArtifactBundleV1, ValidationRunV1, PromotionDecisionV1, and PromotionReceiptV1. Acceptance: schema names are versioned and referenced by EventLedger events. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-008 | REQUIREMENT: MT-008 EventLedger Event Type Plan: add Kernel003 event type names and payload expectations. Acceptance: every event carries run ID, actor, session, task, schema version, timestamp, and artifact refs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_event_ledger --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-009 | REQUIREMENT: MT-009 Artifact Type Plan: define sandbox and validation artifact classes for logs, diffs, manifests, screenshots, reports, redaction, and receipts. Acceptance: each artifact class has content type, hash policy, exportability default, and retention/default root. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_artifact --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-010 | REQUIREMENT: MT-010 DCC Projection Contract: define minimum operator projection for sandbox and promotion state. Acceptance: no-context model can inspect state without terminal logs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-011 | REQUIREMENT: MT-011 Postgres Migration for Sandbox Runs: add authority tables for sandbox runs. Acceptance: records persist and replay after backend restart. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-012 | REQUIREMENT: MT-012 Postgres Migration for Sandbox Policies: persist versioned sandbox policies. Acceptance: policy changes are versioned and traceable. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_policy --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-013 | REQUIREMENT: MT-013 Postgres Migration for Validation Runs: persist validation run metadata and summaries. Acceptance: validation results reconstruct without file-system-only state. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-014 | REQUIREMENT: MT-014 Postgres Migration for Promotion Receipts: persist decisions and receipts. Acceptance: duplicate idempotency keys are rejected or idempotently resolved. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_promotion_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-015 | REQUIREMENT: MT-015 No SQLite Authority Tripwire: prevent Kernel003 authority from using SQLite in production or tests. Acceptance: Kernel003 authority fails closed without Postgres/EventLedger authority. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_no_sqlite --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-016 | REQUIREMENT: MT-016 Replay Projection Storage Query: reconstruct a run from durable rows/events. Acceptance: replay does not read provider chat, terminal scrollback, or transient logs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-017 | REQUIREMENT: MT-017 Legacy Compatibility Blocker Check: detect prerequisite API gaps. Acceptance: missing APIs produce BLOCKED with evidence, not parallel implementations. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_compat --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-018 | REQUIREMENT: MT-018 SandboxAdapter Trait: define adapter boundary independent of Docker, WSL, Deno, or WASM. Acceptance: at least one adapter can be implemented without changing caller code. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-019 | REQUIREMENT: MT-019 PolicyScopedLocal Adapter: implement minimum local proof adapter with strict policy checks. Acceptance: policy mode is explicitly not hard isolation and denies sensitive capabilities by default. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_policy_local --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-020 | REQUIREMENT: MT-020 HardIsolation Adapter Stub: add non-executing adapter slot for hard isolation. Acceptance: hard isolation absence is typed BLOCKED/UNSUPPORTED, not success. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_hard_isolation --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-021 | REQUIREMENT: MT-021 SandboxPolicy Default Deny: implement default-deny policy construction. Acceptance: omitted policy fields deny access. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_policy --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-022 | REQUIREMENT: MT-022 Filesystem Scope Guard: enforce read/write roots and prevent path escape. Acceptance: all path escape attempts return typed denial evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_fs_guard --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-023 | REQUIREMENT: MT-023 Network Capability Gate: deny network unless policy grants it. Acceptance: network grants require approval/provenance refs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_network --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-024 | REQUIREMENT: MT-024 Process Execution Allowlist: permit only registered commands/checks. Acceptance: raw shell strings without descriptors are rejected. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_command_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-025 | REQUIREMENT: MT-025 Environment and Secret Redaction: prevent env/secret leakage. Acceptance: secret-looking values are not emitted in stored logs or reports. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_redaction --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-026 | REQUIREMENT: MT-026 Resource Cap Policy: fold MTE resource caps into sandbox policy. Acceptance: overage halts or gates deterministically with evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_resource_caps --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-027 | REQUIREMENT: MT-027 Cancellation and Timeout: add cancellation and timeout handling. Acceptance: cancelled runs cannot promote and have typed terminal state. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_timeout --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-028 | REQUIREMENT: MT-028 Sandbox Workspace Materializer: materialize candidate inputs into isolated root. Acceptance: no undeclared project files appear in sandbox input manifest. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_workspace --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-029 | REQUIREMENT: MT-029 Sandbox Cleanup and Retention: clean temp roots while preserving artifacts. Acceptance: cleanup never deletes project files or authority rows. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_cleanup --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-030 | REQUIREMENT: MT-030 Sandbox Adapter Health Projection: expose adapter health/preflight state. Acceptance: unsupported isolation is visible before run. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_health --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-031 | REQUIREMENT: MT-031 PatchProposal Contract: define candidate patch envelope. Acceptance: proposals without base refs or target ranges cannot enter validation. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_patch_proposal --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-032 | REQUIREMENT: MT-032 Candidate Range Truth: validate changed paths/ranges against declared targets. Acceptance: unexpected file edits are rejected before promotion. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_candidate_range --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-033 | REQUIREMENT: MT-033 Diff Capture: capture candidate diffs as stable artifacts. Acceptance: identical candidate produces identical diff artifact hash. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_diff_capture --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-034 | REQUIREMENT: MT-034 Artifact Bundle Manifest: create canonical bundle format. Acceptance: bundle hash is deterministic for same inputs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_artifact_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-035 | REQUIREMENT: MT-035 Stdout/Stderr Log Capture: store bounded command logs. Acceptance: logs never live only in terminal output. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_log_capture --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-036 | REQUIREMENT: MT-036 Environment Manifest: record non-sensitive runtime environment identifiers. Acceptance: manifest explains run context without exposing secrets. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_environment_manifest --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-037 | REQUIREMENT: MT-037 Command Manifest: record exactly what commands/checks ran. Acceptance: validators can replay or reason about command intent. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_command_manifest --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-038 | REQUIREMENT: MT-038 Visual Evidence Attachment: attach KB002 screenshot/visual artifacts to validation reports. Acceptance: GUI reports can reference screenshots and DOM/log evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_visual_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-039 | REQUIREMENT: MT-039 Redaction Report: add redaction report to exportable bundles. Acceptance: default export is redacted and denied artifacts are listed. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_redaction_report --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-040 | REQUIREMENT: MT-040 Artifact Store Integration: store sandbox artifacts through validated artifact system. Acceptance: every artifact has stable handle and hash. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_artifact_store --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-041 | REQUIREMENT: MT-041 ValidationDescriptor Contract: define validation command/check descriptors. Acceptance: validation runner rejects undeclared raw commands. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-042 | REQUIREMENT: MT-042 Check Runner Adapter: reuse Product Governance Check Runner. Acceptance: no duplicate check runner is created. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_check_runner_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-043 | REQUIREMENT: MT-043 Validation Result Schema: define result states and finding shapes. Acceptance: every non-PASS has typed reason and evidence refs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_result --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-044 | REQUIREMENT: MT-044 Validation Preflight: preflight descriptors, tools, capabilities, policy mode, paths, and budget. Acceptance: missing tools produce BLOCKED/UNSUPPORTED, not silent skip. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_preflight --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-045 | REQUIREMENT: MT-045 Deterministic Check Batch: run deterministic validation before model review. Acceptance: blocking check failure prevents promotion. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_batch --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-046 | REQUIREMENT: MT-046 Validation Evidence Bundle: store validation outputs canonically. Acceptance: validation report can be inspected offline. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-047 | REQUIREMENT: MT-047 Finding Normalization: normalize check output into findings. Acceptance: raw logs are not the only finding source. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_finding_normalization --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-048 | REQUIREMENT: MT-048 Advisory vs Blocking Rules: make blocking posture explicit. Acceptance: advisory failure is visible but does not block unless configured. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_posture --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-049 | REQUIREMENT: MT-049 Validation Replay: re-run descriptor set against same candidate. Acceptance: replay records new run ID linked to original. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-050 | REQUIREMENT: MT-050 Validation Report Projection: expose validation summaries to DCC/projection layer. Acceptance: operator/model can inspect validation without reading raw files first. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_validation_projection --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-051 | REQUIREMENT: MT-051 PromotionCandidate Contract: define promotion candidate shape from patch proposal or write box. Acceptance: missing validation refs block promotion. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_promotion_candidate --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-052 | REQUIREMENT: MT-052 Promotion Eligibility Check: implement promotion preconditions. Acceptance: ineligible candidate produces typed rejection receipt. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_promotion_eligibility --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-053 | REQUIREMENT: MT-053 Promotion Accept Path: append accepted promotion events to EventLedger. Acceptance: accepted promotion is replayable from durable events. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_promotion_accept --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-054 | REQUIREMENT: MT-054 Promotion Reject Path: record rejected promotion attempts. Acceptance: reject path creates receipt and does not mutate authority. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_promotion_reject --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-055 | REQUIREMENT: MT-055 Idempotency Key Enforcement: prevent duplicate promotion effects. Acceptance: duplicate accept returns prior receipt or typed duplicate rejection without second mutation. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_promotion_idempotency --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-056 | REQUIREMENT: MT-056 Approval Ref Binding: bind approval evidence to promotion decisions. Acceptance: promotion cannot accept without required approval posture. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_promotion_approval --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-057 | REQUIREMENT: MT-057 Authority Mutation Boundary: sandbox and validation cannot mutate authority except through PromotionGate. Acceptance: direct mutation attempt produces denial evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_authority_boundary --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-058 | REQUIREMENT: MT-058 Promotion Closeout Bundle: implement canonical closeout bundle. Acceptance: Integration Validator can review one bundle for promotion. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_closeout_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-059 | REQUIREMENT: MT-059 MTE Run Cap Integration: wire resource caps into sandboxed microtask execution. Acceptance: cap overage halts bounded run and writes evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mte_caps --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-060 | REQUIREMENT: MT-060 Blocked Reason Taxonomy: implement blocked decisioning for sandbox/validation. Acceptance: each blocked reason has retry/escalate/gate semantics. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_blocked_taxonomy --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-061 | REQUIREMENT: MT-061 Retry Budget: bound retry behavior. Acceptance: retry exhaustion becomes typed BLOCKED/FAILED. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_retry_budget --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-062 | REQUIREMENT: MT-062 Smart DropBack: implement smart drop-back semantics. Acceptance: smart/always/never modes have test coverage. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dropback --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-063 | REQUIREMENT: MT-063 Per-MT Summary Artifact: persist per-microtask summaries. Acceptance: every completed/blocked MT attempt has summary ref. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_summary --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-064 | REQUIREMENT: MT-064 Aggregate Run Summary: persist aggregate summary across attempts. Acceptance: no-context reviewer can inspect aggregate before raw artifacts. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_aggregate_summary --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-065 | REQUIREMENT: MT-065 Lane Wake Receipt: implement receipt-driven lane wake/settlement. Acceptance: wake/settlement event includes receipt refs and reason. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_lane_wake --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-066 | REQUIREMENT: MT-066 Bootstrap Skeleton Receipt Projection: first skeleton sandbox run creates restartable receipts. Acceptance: all receipts visible after restart. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_sandbox_skeleton --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-067 | REQUIREMENT: MT-067 DCC Sandbox Run List: add projection/API for sandbox run list. Acceptance: operator can find current and past sandbox runs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc_sandbox_list --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-068 | REQUIREMENT: MT-068 DCC Run Detail: add projection/API for sandbox run detail. Acceptance: detail view has no hidden dependency on terminal scrollback. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc_run_detail --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-069 | REQUIREMENT: MT-069 DCC Promotion Control State: expose promotion eligibility and approval state. Acceptance: UI/API cannot promote when eligibility is false. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc_promotion_state --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-070 | REQUIREMENT: MT-070 Debug Bundle Bridge: fold diagnostics debug bundle into evidence output. Acceptance: diagnostics evidence is bounded and portable. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_debug_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-071 | REQUIREMENT: MT-071 MCP/MEX Evidence Export Bridge: fold tool/mechanical engine evidence into sandbox evidence. Acceptance: MCP/MEX evidence does not use ad hoc bundle schema. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mex_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-072 | REQUIREMENT: MT-072 Capability Audit Evidence Link: link grants/denials to capability evidence. Acceptance: every sensitive grant has provenance. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_capability_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-073 | REQUIREMENT: MT-073 Visual Validation Gate Descriptor: define visual evidence validation mapping. Acceptance: visual evidence can block or advise according to descriptor posture. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_visual_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-074 | REQUIREMENT: MT-074 Console and Network Evidence: capture browser/app console and network evidence for GUI checks. Acceptance: GUI validation failures are diagnosable. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_gui_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-075 | REQUIREMENT: MT-075 Security Denial Test Matrix: add negative tests for sandbox boundaries. Acceptance: every denial writes typed evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_security_denial --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-076 | REQUIREMENT: MT-076 Promotion Failure Test Matrix: test each promotion failure scenario. Acceptance: no failure mutates authority. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_promotion_failure --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-077 | REQUIREMENT: MT-077 Restart Replay Test: prove state survives restart. Acceptance: replay is complete from durable product state. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_restart_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-078 | REQUIREMENT: MT-078 Disk-Agnostic Path Test: prove paths remain repo-root/config relative. Acceptance: moving workspace root does not break path resolution. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_disk_agnostic --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-079 | REQUIREMENT: MT-079 Documentation and Model Manual Update: update product-local model-facing manual. Acceptance: no-context model can run/inspect sandbox workflow from durable docs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: just gov-check | REASON: NONE
  - ID: AC-080 | REQUIREMENT: MT-080 Integration Validator Handoff: prepare final validation bundle without self-validating. Acceptance: Kernel Builder/Coder does not claim PASS/FAIL and validator has sufficient evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: just gov-check; cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/governance_check_runner.rs
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/tests/**
  - app/**
  - tests/**
  - README.md
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mte --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target; cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target; cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target; cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target; cargo test -p handshake_core kernel_mte --target-dir ../Handshake_Artifacts/handshake-cargo-target; just gov-check; just spec-eof-appendices-check [LEGACY_REFINEMENT_BRIDGE]
- CANONICAL_CONTRACT_EXAMPLES:
  - NONE
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/** (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/governance_artifact_registry.rs (backend data surface)
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: MT summaries and blocked work graph | SUBFEATURES: blocked taxonomy, retries, summaries, lane settlement | PRIMITIVES_FEATURES: PRIM-ValidationRecord | MECHANICAL: engine.wrangler | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Postgres-only kernel authority | SUBFEATURES: run/policy/validation/promotion rows, replay query, no SQLite tripwire | PRIMITIVES_FEATURES: PRIM-PromotionGates | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: No-context evidence bundles | SUBFEATURES: manifests, findings, summaries, redaction report, closeout bundle | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR_DECOMPOSITION: PILLAR: ACE | CAPABILITY_SLICE: Optional ACE evidence reference passthrough | SUBFEATURES: trace refs, query-plan refs, validation evidence links | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Kernel003 preserves references without owning ACE persistence.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Sandbox run execution | JobModel: MECHANICAL_TOOL | Workflow: policy-scoped sandbox run through ToolGate and adapter | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: sandbox_run/requested/started/blocked/completed | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Authority owns state.
  - FORCE_MULTIPLIER_EXPANSION: Flight Recorder run correlation -> IN_THIS_WP (stub: NONE)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/governance_check_runner.rs
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/tests/**
  - app/**
  - tests/**
  - README.md
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
  - `TECHNICAL_ADVISOR` is `NONE` for this packet because WP Validator is disabled
  - `TECHNICAL_AUTHORITY` owns final technical verdict authority
  - `MERGE_AUTHORITY` owns merge-to-main authority
  - `WP_VALIDATOR_OF_RECORD` remains `NONE`; `INTEGRATION_VALIDATOR_OF_RECORD` names the active batch-review validator session once assigned
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
  - For this packet, `COMMUNICATION_CONTRACT` remains `DIRECT_REVIEW_V1` and `COMMUNICATION_HEALTH_GATE` is `INTEGRATION_BATCH_REVIEW_BLOCKING`.
  - WP Validator kickoff is intentionally disabled. Kernel Builder implementation uses the coder-compatible lane until the final Integration Validator batch handoff.
  - Required structured receipts for this topology are Kernel Builder/Coder-compatible implementation handoff receipts and one final coder-compatible <-> Integration Validator review pair.
  - For `PACKET_FORMAT_VERSION >= 2026-03-22`, `VERDICT` also requires one direct coder <-> integration-validator review pair recorded in receipts with a shared `correlation_id` / `ack_for` chain.
  - Review-tracked receipt appends auto-write notifications for the explicit target role and auto-project the next actor / validator wake state back into `RUNTIME_STATUS.json`.
  - `just wp-thread-append` remains valid for soft coordination only. It does not satisfy the required direct-review contract by itself.
  - `just wp-communication-health-check WP-{ID} KICKOFF|HANDOFF|VERDICT` is the machine gate for this contract.
- SESSION START + WAKE RULE:
  - only `WORKFLOW_AUTHORITY` may start the coder-compatible Kernel Builder implementation session and the Integration Validator session; WP Validator is disabled for this packet
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
  - Sub-agents produce draft code only; Primary Coder verifies against resolved SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-KERNEL-003-Sandbox-Validation-Promotion-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.9
- CONTEXT_START_LINE: 3688
- CONTEXT_END_LINE: 3724
- CONTEXT_TOKEN: Kernel V1 Authority State [ADD v02.184]
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 2.3.13.9 Kernel V1 Authority State [ADD v02.184]
  Kernel V1 runtime authority MUST be product-owned durable state, not provider chat history, terminal transcripts, repo-governance artifacts, or diagnostic mirrors.
  - A Postgres-backed append-only EventLedger for kernel task, session, tool, artifact, validation, and promotion events.
  - ToolGate, ArtifactProposal, ValidationRunner, and PromotionGate events linked to the same run IDs.
  Kernel V1 MUST NOT use SQLite for authority, cache, offline mode, compatibility mode, local fallback, bootstrap convenience, or test fixtures.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.10
- CONTEXT_START_LINE: 3710
- CONTEXT_END_LINE: 3724
- CONTEXT_TOKEN: Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 2.3.13.10 Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]
  Kernel V1 CRDT workspace state is pre-promotion working state.
  The Kernel V1 implementation MUST provide a KernelActionCatalogV1 contract that enumerates every write-capable kernel action before it can mutate a draft or request promotion.
  Direct edits to authoritative Kernel V1 records MUST be denied unless they enter through an allowed catalog action and write box path.
  The CRDT-to-EventLedger promotion bridge MUST be explicit.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/05-security-and-observability.md#5.2.5
- CONTEXT_START_LINE: 22958
- CONTEXT_END_LINE: 22967
- CONTEXT_TOKEN: Mechanical Runner Sandbox
- EXCERPT_ASCII_ESCAPED:
  ```text
### 5.2.5 Mechanical Runner Sandbox
  - Mechanical engines run via a constrained runner: explicit allowlist per engine, resource limits (CPU/GPU/mem/time), and capability gates (file/process/network/device).
  - Log command, params, cwd, exit code, stdout/stderr, artifact hashes; refuse/abort when capability is missing or bounds exceeded.
  - Provide refusal paths and tests to ensure engines cannot bypass Workflow/Flight Recorder or capabilities.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/05-security-and-observability.md#5.4.5
- CONTEXT_START_LINE: 23481
- CONTEXT_END_LINE: 23500
- CONTEXT_TOKEN: Kernel V1 Authority Observability Boundary [ADD v02.184]
- EXCERPT_ASCII_ESCAPED:
  ```text
### 5.4.5 Kernel V1 Authority Observability Boundary [ADD v02.184]
  Flight Recorder remains mandatory append-only observability, but Kernel V1 replay and promotion authority MUST come from the Postgres EventLedger defined in Section 2.3.13.9.
  Kernel V1 observability MUST expose enough structured fields for no-context debugging: kernel_task_run_id, session_run_id, event_ledger_id, artifact_proposal_id, validation_run_id, and promotion_gate_id.
  Security and observability tests for Kernel V1 MUST prove that replay still works when Flight Recorder or provider trace history is unavailable and EventLedger rows remain intact.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md#10.11.5.28
- CONTEXT_START_LINE: 61703
- CONTEXT_END_LINE: 61719
- CONTEXT_TOKEN: Kernel Action Catalog and Write Box Projections [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
### 10.11.5.28 Kernel Action Catalog and Write Box Projections [ADD v02.185]
  The Dev Command Center MUST expose Kernel V1 action-catalog and write-box state as typed product projections, not as raw transcript or repo-governance mirrors.
  Dev Command Center controls MAY request catalog-backed write-box actions or promotion, but they MUST NOT directly mutate EventLedger authority or silently apply CRDT updates as authority.
  Visual debugging and acceptance proof MUST include stable element identifiers for action catalog rows, write-box rows, denial receipts, promotion previews, and stale projection badges.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/11-shared-dev-platform-and-oss-foundations.md#sandbox-minimum
- CONTEXT_START_LINE: 68955
- CONTEXT_END_LINE: 68970
- CONTEXT_TOKEN: Sandbox MUST prevent filesystem escape
- EXCERPT_ASCII_ESCAPED:
  ```text
Any device.*, net.http, or secrets.use:* MUST require policy approval and be recorded in provenance.
  Sandbox MUST prevent filesystem escape, deny network unless granted, deny exec unless allowlisted, and record environment identifiers.
  Required global gates: G-SCHEMA, G-CAP, G-INTEGRITY, G-BUDGET, G-PROVENANCE, G-DET.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: MT-001 Activation Source Inventory: re-scan stubs, Task Board, Build Order, and traceability for every KB003-related source. Acceptance: packet contains a source fold table at least as detailed as the stub. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: .GOV/task_packets/**, .GOV/roles_shared/records/** | EXPECTED_TESTS: just gov-check | RISK_IF_MISSED: folded source scope can be lost before implementation.
  - CLAUSE: MT-002 Conflict Deliberation Record: convert conflict register into signed decisions for raw shell, direct mutation, container-only, SQLite, and domain-evidence bloat. Acceptance: conflicts are approved, rejected, or parked, none silently removed. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: .GOV/refinements/**, .GOV/task_packets/** | EXPECTED_TESTS: just gov-check | RISK_IF_MISSED: implementation reopens unsafe assumptions.
  - CLAUSE: MT-003 Current Product API Inventory: inspect product modules for KB001/KB002 and existing check/artifact/governance APIs. Acceptance: packet targets real files or declares missing upstream blockers. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: rg EventLedger src/backend/handshake_core; rg WriteBox src/backend/handshake_core | RISK_IF_MISSED: coder may invent duplicate authority.
  - CLAUSE: MT-004 Research Basis Update: compare current sandbox adapter and validation evidence options before implementation. Acceptance: selected adapter sequence and rejected options are documented. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, README.md | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: adapter design may be stale or host-incompatible.
  - CLAUSE: MT-005 Official Packet Contract Generation: promote stub into signed official packet with contracts and 80 MTs. Acceptance: packet is ready but not self-validated by Kernel Builder. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/** | EXPECTED_TESTS: just gov-check | RISK_IF_MISSED: downstream session lacks executable contract.
  - CLAUSE: MT-006 Product Module Placement Decision: decide where sandbox, validation, and promotion modules live. Acceptance: module topology is documented before scaffolding. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: implementation creates duplicate or misplaced modules.
  - CLAUSE: MT-007 Kernel003 Schema Namespace: define stable schema IDs for KernelSandboxRunV1, SandboxPolicyV1, SandboxWorkspaceV1, SandboxArtifactBundleV1, ValidationRunV1, PromotionDecisionV1, and PromotionReceiptV1. Acceptance: schema names are versioned and referenced by EventLedger events. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: contracts drift across producer and validator.
  - CLAUSE: MT-008 EventLedger Event Type Plan: add Kernel003 event type names and payload expectations. Acceptance: every event carries run ID, actor, session, task, schema version, timestamp, and artifact refs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_event_ledger --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: replay and promotion evidence cannot connect.
  - CLAUSE: MT-009 Artifact Type Plan: define sandbox and validation artifact classes for logs, diffs, manifests, screenshots, reports, redaction, and receipts. Acceptance: each artifact class has content type, hash policy, exportability default, and retention/default root. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_artifact --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence becomes ad hoc and non-replayable.
  - CLAUSE: MT-010 DCC Projection Contract: define minimum operator projection for sandbox and promotion state. Acceptance: no-context model can inspect state without terminal logs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: operator/model cannot inspect state.
  - CLAUSE: MT-011 Postgres Migration for Sandbox Runs: add authority tables for sandbox runs. Acceptance: records persist and replay after backend restart. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: run state is not durable.
  - CLAUSE: MT-012 Postgres Migration for Sandbox Policies: persist versioned sandbox policies. Acceptance: policy changes are versioned and traceable. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_policy --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: policy provenance is missing.
  - CLAUSE: MT-013 Postgres Migration for Validation Runs: persist validation run metadata and summaries. Acceptance: validation results reconstruct without file-system-only state. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation evidence is not replayable.
  - CLAUSE: MT-014 Postgres Migration for Promotion Receipts: persist decisions and receipts. Acceptance: duplicate idempotency keys are rejected or idempotently resolved. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: duplicate promotion corrupts authority.
  - CLAUSE: MT-015 No SQLite Authority Tripwire: prevent Kernel003 authority from using SQLite in production or tests. Acceptance: Kernel003 authority fails closed without Postgres/EventLedger authority. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_no_sqlite --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: Kernel V1 no-SQLite law regresses.
  - CLAUSE: MT-016 Replay Projection Storage Query: reconstruct a run from durable rows/events. Acceptance: replay does not read provider chat, terminal scrollback, or transient logs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: restart proof is weak.
  - CLAUSE: MT-017 Legacy Compatibility Blocker Check: detect prerequisite API gaps. Acceptance: missing APIs produce BLOCKED with evidence, not parallel implementations. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_compat --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: implementation forks around prerequisites.
  - CLAUSE: MT-018 SandboxAdapter Trait: define adapter boundary independent of Docker, WSL, Deno, or WASM. Acceptance: at least one adapter can be implemented without changing caller code. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: isolation choice leaks through callers.
  - CLAUSE: MT-019 PolicyScopedLocal Adapter: implement minimum local proof adapter with strict policy checks. Acceptance: policy mode is explicitly not hard isolation and denies sensitive capabilities by default. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_policy_local --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: local proof becomes unsafe or misleading.
  - CLAUSE: MT-020 HardIsolation Adapter Stub: add non-executing adapter slot for hard isolation. Acceptance: hard isolation absence is typed BLOCKED/UNSUPPORTED, not success. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_hard_isolation --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: host capability absence is hidden.
  - CLAUSE: MT-021 SandboxPolicy Default Deny: implement default-deny policy construction. Acceptance: omitted policy fields deny access. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_policy --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: unsafe defaults allow access.
  - CLAUSE: MT-022 Filesystem Scope Guard: enforce read/write roots and prevent path escape. Acceptance: all path escape attempts return typed denial evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_fs_guard --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox can access project or host paths.
  - CLAUSE: MT-023 Network Capability Gate: deny network unless policy grants it. Acceptance: network grants require approval/provenance refs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_network --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox can leak or fetch untracked data.
  - CLAUSE: MT-024 Process Execution Allowlist: permit only registered commands/checks. Acceptance: raw shell strings without descriptors are rejected. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_command_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation becomes raw shell execution.
  - CLAUSE: MT-025 Environment and Secret Redaction: prevent env/secret leakage. Acceptance: secret-looking values are not emitted in stored logs or reports. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_redaction --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence leaks sensitive data.
  - CLAUSE: MT-026 Resource Cap Policy: fold MTE resource caps into sandbox policy. Acceptance: overage halts or gates deterministically with evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_resource_caps --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: execution is unbounded.
  - CLAUSE: MT-027 Cancellation and Timeout: add cancellation and timeout handling. Acceptance: cancelled runs cannot promote and have typed terminal state. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_timeout --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: hung/cancelled work can leak into promotion.
  - CLAUSE: MT-028 Sandbox Workspace Materializer: materialize candidate inputs into isolated root. Acceptance: no undeclared project files appear in sandbox input manifest. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_workspace --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox input is not auditable.
  - CLAUSE: MT-029 Sandbox Cleanup and Retention: clean temp roots while preserving artifacts. Acceptance: cleanup never deletes project files or authority rows. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_cleanup --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: cleanup can destroy data or evidence.
  - CLAUSE: MT-030 Sandbox Adapter Health Projection: expose adapter health/preflight state. Acceptance: unsupported isolation is visible before run. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, app/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_health --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: operators/models cannot diagnose adapter blockers.
  - CLAUSE: MT-031 PatchProposal Contract: define candidate patch envelope. Acceptance: proposals without base refs or target ranges cannot enter validation. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_patch_proposal --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: candidate identity is incomplete.
  - CLAUSE: MT-032 Candidate Range Truth: validate changed paths/ranges against declared targets. Acceptance: unexpected file edits are rejected before promotion. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_candidate_range --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: promotion can include out-of-scope changes.
  - CLAUSE: MT-033 Diff Capture: capture candidate diffs as stable artifacts. Acceptance: identical candidate produces identical diff artifact hash. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_diff_capture --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence hashes drift.
  - CLAUSE: MT-034 Artifact Bundle Manifest: create canonical bundle format. Acceptance: bundle hash is deterministic for same inputs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_artifact_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: artifact identity is unstable.
  - CLAUSE: MT-035 Stdout/Stderr Log Capture: store bounded command logs. Acceptance: logs never live only in terminal output. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_log_capture --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence disappears with terminal scrollback.
  - CLAUSE: MT-036 Environment Manifest: record non-sensitive runtime environment identifiers. Acceptance: manifest explains run context without exposing secrets. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_environment_manifest --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: run context cannot be reconstructed.
  - CLAUSE: MT-037 Command Manifest: record exactly what commands/checks ran. Acceptance: validators can replay or reason about command intent. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_command_manifest --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation cannot be audited.
  - CLAUSE: MT-038 Visual Evidence Attachment: attach KB002 screenshot/visual artifacts to validation reports. Acceptance: GUI reports can reference screenshots and DOM/log evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_visual_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: GUI validation loses evidence.
  - CLAUSE: MT-039 Redaction Report: add redaction report to exportable bundles. Acceptance: default export is redacted and denied artifacts are listed. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_redaction_report --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: exported bundles leak or omit policy state.
  - CLAUSE: MT-040 Artifact Store Integration: store sandbox artifacts through validated artifact system. Acceptance: every artifact has stable handle and hash. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_artifact_store --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence bypasses artifact authority.
  - CLAUSE: MT-041 ValidationDescriptor Contract: define validation command/check descriptors. Acceptance: validation runner rejects undeclared raw commands. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation cannot be policy checked.
  - CLAUSE: MT-042 Check Runner Adapter: reuse Product Governance Check Runner. Acceptance: no duplicate check runner is created. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_check_runner.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_check_runner_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: duplicate validation engines drift.
  - CLAUSE: MT-043 Validation Result Schema: define result states and finding shapes. Acceptance: every non-PASS has typed reason and evidence refs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_result --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: blockers are ambiguous.
  - CLAUSE: MT-044 Validation Preflight: preflight descriptors, tools, capabilities, policy mode, paths, and budget. Acceptance: missing tools produce BLOCKED/UNSUPPORTED, not silent skip. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_preflight --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation can silently under-run.
  - CLAUSE: MT-045 Deterministic Check Batch: run deterministic validation before model review. Acceptance: blocking check failure prevents promotion. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_batch --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: unvalidated candidate can reach promotion.
  - CLAUSE: MT-046 Validation Evidence Bundle: store validation outputs canonically. Acceptance: validation report can be inspected offline. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validator lacks portable evidence.
  - CLAUSE: MT-047 Finding Normalization: normalize check output into findings. Acceptance: raw logs are not the only finding source. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_finding_normalization --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: findings cannot drive remediation.
  - CLAUSE: MT-048 Advisory vs Blocking Rules: make blocking posture explicit. Acceptance: advisory failure is visible but does not block unless configured. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_posture --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: advisory/blocking semantics drift.
  - CLAUSE: MT-049 Validation Replay: re-run descriptor set against same candidate. Acceptance: replay records new run ID linked to original. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation cannot be reproduced.
  - CLAUSE: MT-050 Validation Report Projection: expose validation summaries to DCC/projection layer. Acceptance: operator/model can inspect validation without reading raw files first. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_projection --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation state is hidden.
  - CLAUSE: MT-051 PromotionCandidate Contract: define promotion candidate shape from patch proposal or write box. Acceptance: missing validation refs block promotion. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_candidate --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: promotion input is incomplete.
  - CLAUSE: MT-052 Promotion Eligibility Check: implement promotion preconditions. Acceptance: ineligible candidate produces typed rejection receipt. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_eligibility --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: invalid candidate can promote.
  - CLAUSE: MT-053 Promotion Accept Path: append accepted promotion events to EventLedger. Acceptance: accepted promotion is replayable from durable events. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_accept --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: authority mutation is not durable.
  - CLAUSE: MT-054 Promotion Reject Path: record rejected promotion attempts. Acceptance: reject path creates receipt and does not mutate authority. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_reject --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: failed promotion is invisible.
  - CLAUSE: MT-055 Idempotency Key Enforcement: prevent duplicate promotion effects. Acceptance: duplicate accept returns prior receipt or typed duplicate rejection without second mutation. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_idempotency --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: duplicates corrupt authority.
  - CLAUSE: MT-056 Approval Ref Binding: bind approval evidence to promotion decisions. Acceptance: promotion cannot accept without required approval posture. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_approval --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: promotion can bypass approval.
  - CLAUSE: MT-057 Authority Mutation Boundary: sandbox and validation cannot mutate authority except through PromotionGate. Acceptance: direct mutation attempt produces denial evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_authority_boundary --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox bypasses promotion law.
  - CLAUSE: MT-058 Promotion Closeout Bundle: implement canonical closeout bundle. Acceptance: Integration Validator can review one bundle for promotion. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_closeout_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: review evidence remains scattered.
  - CLAUSE: MT-059 MTE Run Cap Integration: wire resource caps into sandboxed microtask execution. Acceptance: cap overage halts bounded run and writes evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mte_caps --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MTE folded cap scope is lost.
  - CLAUSE: MT-060 Blocked Reason Taxonomy: implement blocked decisioning for sandbox/validation. Acceptance: each blocked reason has retry/escalate/gate semantics. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_blocked_taxonomy --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: blocked work cannot be routed.
  - CLAUSE: MT-061 Retry Budget: bound retry behavior. Acceptance: retry exhaustion becomes typed BLOCKED/FAILED. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_retry_budget --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: loops become unbounded.
  - CLAUSE: MT-062 Smart DropBack: implement smart drop-back semantics. Acceptance: smart/always/never modes have test coverage. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dropback --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: escalation/retry recovery is ad hoc.
  - CLAUSE: MT-063 Per-MT Summary Artifact: persist per-microtask summaries. Acceptance: every completed/blocked MT attempt has summary ref. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_summary --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: no-context handoff loses MT evidence.
  - CLAUSE: MT-064 Aggregate Run Summary: persist aggregate summary across attempts. Acceptance: no-context reviewer can inspect aggregate before raw artifacts. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_aggregate_summary --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: packet review is too expensive and brittle.
  - CLAUSE: MT-065 Lane Wake Receipt: implement receipt-driven lane wake/settlement. Acceptance: wake/settlement event includes receipt refs and reason. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_lane_wake --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: lane state depends on chat.
  - CLAUSE: MT-066 Bootstrap Skeleton Receipt Projection: first skeleton sandbox run creates restartable receipts. Acceptance: all receipts visible after restart. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_skeleton --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: integration path is unproven.
  - CLAUSE: MT-067 DCC Sandbox Run List: add projection/API for sandbox run list. Acceptance: operator can find current and past sandbox runs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc_sandbox_list --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: run discovery is missing.
  - CLAUSE: MT-068 DCC Run Detail: add projection/API for sandbox run detail. Acceptance: detail view has no hidden dependency on terminal scrollback. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc_run_detail --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence inspection remains manual.
  - CLAUSE: MT-069 DCC Promotion Control State: expose promotion eligibility and approval state. Acceptance: UI/API cannot promote when eligibility is false. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc_promotion_state --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: UI may imply unsafe promotion.
  - CLAUSE: MT-070 Debug Bundle Bridge: fold diagnostics debug bundle into evidence output. Acceptance: diagnostics evidence is bounded and portable. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_debug_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: debug evidence cannot support validation.
  - CLAUSE: MT-071 MCP/MEX Evidence Export Bridge: fold tool/mechanical engine evidence into sandbox evidence. Acceptance: MCP/MEX evidence does not use ad hoc bundle schema. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mex_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: tool evidence is non-portable.
  - CLAUSE: MT-072 Capability Audit Evidence Link: link grants/denials to capability evidence. Acceptance: every sensitive grant has provenance. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_capability_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: grants cannot be audited.
  - CLAUSE: MT-073 Visual Validation Gate Descriptor: define visual evidence validation mapping. Acceptance: visual evidence can block or advise according to descriptor posture. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_visual_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: visual evidence cannot influence validation safely.
  - CLAUSE: MT-074 Console and Network Evidence: capture browser/app console and network evidence for GUI checks. Acceptance: GUI validation failures are diagnosable. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_gui_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: UI failures lack actionable evidence.
  - CLAUSE: MT-075 Security Denial Test Matrix: add negative tests for sandbox boundaries. Acceptance: every denial writes typed evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_security_denial --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox security claims are unproven.
  - CLAUSE: MT-076 Promotion Failure Test Matrix: test each promotion failure scenario. Acceptance: no failure mutates authority. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_failure --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: rejection paths can mutate or disappear.
  - CLAUSE: MT-077 Restart Replay Test: prove state survives restart. Acceptance: replay is complete from durable product state. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_restart_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: hidden session state remains required.
  - CLAUSE: MT-078 Disk-Agnostic Path Test: prove paths remain repo-root/config relative. Acceptance: moving workspace root does not break path resolution. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_disk_agnostic --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: local drive paths leak into product.
  - CLAUSE: MT-079 Documentation and Model Manual Update: update product-local model-facing manual. Acceptance: no-context model can run/inspect sandbox workflow from durable docs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: README.md, src/backend/handshake_core/src/kernel/**, app/** | EXPECTED_TESTS: just gov-check | RISK_IF_MISSED: future sessions depend on chat history.
  - CLAUSE: MT-080 Integration Validator Handoff: prepare final validation bundle without self-validating. Acceptance: Kernel Builder/Coder does not claim PASS/FAIL and validator has sufficient evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: just gov-check; cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: final review lacks batch evidence.
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: KernelSandboxRunV1 and SandboxPolicyV1 | PRODUCER: sandbox runner and policy builder | CONSUMER: ToolGate, ValidationRunner, DCC, Integration Validator | SERIALIZER_TRANSPORT: Postgres records plus EventLedger event payloads | VALIDATOR_READER: sandbox policy and run replay tests | TRIPWIRE_TESTS: omitted grants deny, unsupported adapters block, runs replay after restart | DRIFT_RISK: sandbox grants become implicit.
  - CONTRACT: SandboxWorkspaceV1 and SandboxArtifactBundleV1 | PRODUCER: workspace materializer and artifact store | CONSUMER: validators, DCC, promotion gate, debug bundle exporter | SERIALIZER_TRANSPORT: artifact handles, manifest JSON, hashes, redaction report | VALIDATOR_READER: bundle hash and path guard tests | TRIPWIRE_TESTS: path escape denial, deterministic bundle hash, no terminal-only logs | DRIFT_RISK: evidence becomes file-only.
  - CONTRACT: ValidationDescriptorV1, ValidationRunV1, and ValidationFindingV1 | PRODUCER: validation descriptor registry and check runner adapter | CONSUMER: PromotionGate, DCC, Integration Validator | SERIALIZER_TRANSPORT: JSON descriptors/results and Postgres rows | VALIDATOR_READER: validation preflight, result, replay tests | TRIPWIRE_TESTS: raw command denial, blocked tool, advisory/blocking mix | DRIFT_RISK: raw shell silently runs.
  - CONTRACT: PromotionCandidateV1, PromotionDecisionV1, and PromotionReceiptV1 | PRODUCER: promotion gate | CONSUMER: EventLedger, DCC, Locus, Integration Validator | SERIALIZER_TRANSPORT: EventLedger events, Postgres rows, receipts | VALIDATOR_READER: accept/reject/idempotency tests | TRIPWIRE_TESTS: stale, duplicate, missing approval, validation failure, projection failure | DRIFT_RISK: invalid candidate mutates authority.
  - CONTRACT: MTE bounded-run controls | PRODUCER: sandbox/MTE runtime | CONSUMER: Locus, DCC, summaries, lane settlement | SERIALIZER_TRANSPORT: run cap policy, blocked reason, summary artifact, wake receipt | VALIDATOR_READER: cap, blocked, retry, drop-back, summary tests | TRIPWIRE_TESTS: cap overage, retry exhaustion, settlement by receipt | DRIFT_RISK: MT execution loops become unbounded.
  - CONTRACT: DCC sandbox validation projections | PRODUCER: projection registry | CONSUMER: operator, no-context model, Integration Validator | SERIALIZER_TRANSPORT: structured projection JSON plus stable artifact refs | VALIDATOR_READER: API/projection and visual checks | TRIPWIRE_TESTS: disabled promotion when ineligible, stable row ids, stale badges | DRIFT_RISK: UI hides unsafe state.
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Implement MT-001 through MT-080 back to back in declared order unless dependency proof requires local reordering without dropping, merging, renumbering, or condensing any MT.
- HOT_FILES:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/governance_check_runner.rs
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/tests/**
  - app/**
  - tests/**
  - README.md
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mte --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
- CARRY_FORWARD_WARNINGS:
  - Do not condense or remove any MT.
  - Do not introduce WP Validator gate.
  - Do not treat Kernel Builder checks as validation.
  - Do not use SQLite authority, cache, offline, fallback, compatibility, or test fixtures for Kernel003.
  - Do not create parallel EventLedger, CheckRunner, ArtifactStore, WriteBox, or PromotionGate systems when KB001/KB002 surfaces exist.
  - PolicyScopedLocal is not hard isolation; label it honestly and expose hard isolation unsupported state.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - All 80 MT clauses and contracts.
  - Fully folded source-stub goals remain present and traceable.
  - Sandbox and validation cannot mutate authority except through PromotionGate.
  - Default-deny policy, path guardrails, caps, blocked taxonomy, and redaction are enforced.
  - Promotion accept/reject/idempotency/replay paths are durable EventLedger/Postgres authority.
  - No SQLite authority, fallback, compatibility, offline, or fixture path exists for Kernel003.
  - No WP Validator gate; Integration Validator batch/spec review is separate.
- FILES_TO_READ:
  - .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/packet.json
  - .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/MT-*.json
  - .GOV/spec/SPEC_CURRENT.md
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/tests/**
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mte --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
  - just spec-eof-appendices-check
- POST_MERGE_SPOTCHECKS:
  - No-context manual path, sandbox denial harness, validation replay proof, promotion rejection proof, and DCC evidence visibility.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Product implementation has not started.
  - KB001 and KB002 product APIs may not be fully landed in the implementation worktree.
  - MT-004 current external research has not run yet.
  - Host hard-isolation availability is unknown.
  - Integration Validator verdict does not exist yet.
  - DCC screenshot evidence waits for GUI implementation.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Local authority already selects default-deny policy, durable EventLedger evidence, generated artifact bundles, validation descriptors, and PromotionGate receipts.
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
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-WriteBoxDirectEditDeniedV1
  - PRIM-WriteBoxPromotionRequestV1
  - PRIM-WriteBoxPromotionReceiptV1
  - PRIM-ArtifactManifest
  - PRIM-ValidationExecution
  - PRIM-ValidationFinding
  - PRIM-ValidationRecord
  - PRIM-ValidationResult
  - PRIM-ValidationStatus
  - PRIM-PromotionGates
  - PRIM-PromotionGateSnapshot
  - PRIM-PromotionPath
- PRIMITIVES_EXPOSED:
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-ArtifactManifest
  - PRIM-ValidationExecution
  - PRIM-ValidationFinding
  - PRIM-ValidationRecord
  - PRIM-ValidationResult
  - PRIM-ValidationStatus
  - PRIM-PromotionGates
  - PRIM-PromotionGateSnapshot
  - PRIM-PromotionPath
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.archivist
  - engine.librarian
  - engine.analyst
  - engine.wrangler
  - engine.dba
  - engine.sovereign
  - engine.guide
  - engine.context
  - engine.version
  - engine.sandbox
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NO_CHANGE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Flight Recorder
  - Locus
  - Work packets (product, not repo)
  - Task board (product, not repo)
  - MicroTask
  - Command Center
  - Execution / Job Runtime
  - Spec to prompt
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
  - ACE
  - RAG
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - Sandbox doctor/preflight -> IN_THIS_WP (stub: NONE)
  - Canonical bundle hashing -> IN_THIS_WP (stub: NONE)
  - No-op skeleton sandbox run -> IN_THIS_WP (stub: NONE)
  - Receipt-driven lane settlement -> IN_THIS_WP (stub: NONE)
  - DCC evidence drawer -> IN_THIS_WP (stub: NONE)
  - Flight Recorder run correlation -> IN_THIS_WP (stub: NONE)
  - Task Board activation projection -> IN_THIS_WP (stub: NONE)
  - MicroTask bounded attempt envelope -> IN_THIS_WP (stub: NONE)
  - Spec prompt context windows -> IN_THIS_WP (stub: NONE)
  - Postgres replay query -> IN_THIS_WP (stub: NONE)
  - ACE trace reference bridge -> IN_THIS_WP (stub: NONE)
  - Retrieval trace reference bridge -> IN_THIS_WP (stub: NONE)
  - Candidate range truth -> IN_THIS_WP (stub: NONE)
  - Finding normalization -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: WP-KERNEL-003-Sandbox-Validation-Promotion-v1
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: Sandbox and promotion event evidence | SUBFEATURES: run events, validation events, promotion receipts, replay verification | PRIMITIVES_FEATURES: PRIM-ValidationExecution | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: Locus | CAPABILITY_SLICE: MT summaries and blocked work graph | SUBFEATURES: blocked taxonomy, retries, summaries, lane settlement | PRIMITIVES_FEATURES: PRIM-ValidationRecord | MECHANICAL: engine.wrangler | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Contract-first active packet | SUBFEATURES: 80 MT contracts, packet contract, closeout bundle | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Stub-to-active projection | SUBFEATURES: Ready-for-dev status, traceability registry update, build-order row update | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: MicroTask | CAPABILITY_SLICE: Bounded sandboxed MT attempts | SUBFEATURES: caps, blocked reasons, retry budget, summaries, drop-back | PRIMITIVES_FEATURES: PRIM-ValidationRecord | MECHANICAL: engine.analyst | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: Command Center | CAPABILITY_SLICE: Sandbox/validation/promotion projection | SUBFEATURES: run list, run detail, promotion eligibility, evidence links | PRIMITIVES_FEATURES: PRIM-KernelActionCatalogV1 | MECHANICAL: engine.guide | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Sandboxed execution and validation | SUBFEATURES: adapter, policy, command descriptor, resource caps, preflight | PRIMITIVES_FEATURES: PRIM-ValidationExecution | MECHANICAL: engine.sandbox | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: Spec to prompt | CAPABILITY_SLICE: No-context bounded implementation context | SUBFEATURES: spec anchors, packet context windows, model manual, evidence refs | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Postgres-only kernel authority | SUBFEATURES: run/policy/validation/promotion rows, replay query, no SQLite tripwire | PRIMITIVES_FEATURES: PRIM-PromotionGates | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: No-context evidence bundles | SUBFEATURES: manifests, findings, summaries, redaction report, closeout bundle | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
  - PILLAR: ACE | CAPABILITY_SLICE: Optional ACE evidence reference passthrough | SUBFEATURES: trace refs, query-plan refs, validation evidence links | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Kernel003 preserves references without owning ACE persistence.
  - PILLAR: RAG | CAPABILITY_SLICE: Optional retrieval evidence reference passthrough | SUBFEATURES: retrieval trace refs, export-compatible handles, validation evidence links | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Kernel003 preserves references without implementing retrieval export.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Sandbox run execution | JobModel: MECHANICAL_TOOL | Workflow: policy-scoped sandbox run through ToolGate and adapter | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: sandbox_run/requested/started/blocked/completed | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Authority owns state.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-KERNEL-003-Sandbox-Validation-Promotion-v1 -> EXPAND_IN_THIS_WP
  - WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1 -> REUSE_EXISTING
- CODE_REALITY_SUMMARY:
  - NONE
## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: YES
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PASS
- GUI_REFERENCE_DECISIONS:
  - DCC sandbox validation projection <- NONE (IN_THIS_WP)
- HANDSHAKE_GUI_ADVICE:
  - Surface: DCC sandbox run list | Control: Open run evidence | Type: icon button | Why: inspect evidence before promotion | Microcopy: Open evidence | Tooltip: Show manifests, validation, promotion, and replay refs.
  - Surface: DCC promotion state | Control: Request promotion | Type: action button | Why: avoid direct authority mutation | Microcopy: Request promotion | Tooltip: Requires validated candidate and approval refs.
- HIDDEN_GUI_REQUIREMENTS:
  - Unsupported adapter, denied capability, blocked validation, stale candidate, and missing approval states remain visible.
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - Key rows by sandbox_run_id, validation_run_id, promotion_candidate_id, artifact_bundle_id, policy_id, and projection_hash.
## SCOPE
- What: Activate Kernel003 as the sandbox, validation, evidence, promotion, MTE controls, DCC projection, debug bundle bridge, candidate-range truth, closeout bundle, and receipt-driven settlement packet with all 80 preserved MTs.
- Why: Kernel001 supplies durable authority and Kernel002 supplies write-box/pre-promotion surfaces; Kernel003 makes model-produced work safe by running it in bounded sandbox/validation paths before promotion to product authority.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/governance_check_runner.rs
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/tests/**
  - app/**
  - tests/**
  - README.md
- OUT_OF_SCOPE:
  - No product implementation in this activation session.
  - No WP Validator gate/session.
  - No Integration Validator launch/verdict/merge/pass-fail claim in this activation session.
  - No condensing, merging, dropping, or renumbering the 80 MTs.
  - No Kernel004 local model memory implementation.
  - No full CRDT workspace beyond KB002 dependencies.
  - No broad legacy SQLite replacement unrelated to Kernel V1 authority.
  - No mandatory production-grade VM/container stack as the only supported adapter.
  - No domain-specific retrieval, Spec Router, AI-ready, cloud-consent, calendar, or mail exporters beyond generic evidence interfaces.
- TOUCHED_FILE_BUDGET: 10
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (Host-load/test deferral example: `- WAIVER_ID: CX-ENV-HOST-LOAD-... | STATUS: ACTIVE | COVERS: TEST, ENVIRONMENT | SCOPE: <heavy commands> while operator-owned host load is active | JUSTIFICATION: Operator waived heavy tests during load; roles must not touch external processes | APPROVER: Operator | EXPIRES: <condition requiring fresh proof or closure>`.)
- (Do not use `## WAIVERS GRANTED` to continue after token-cost overrun. Token budget and token-ledger drift are diagnostic-only cost telemetry and must be surfaced mechanically in audits/dossiers instead of requiring a continuation waiver.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_mte --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  just spec-eof-appendices-check
```

### DONE_MEANS
- Active packet/refinement and exactly 80 MT contracts/projections exist.
- All fully folded source-stub goals are preserved in packet, MTs, and traceability.
- Sandbox jobs cannot write authority state directly or escape declared sandbox/output roots.
- Sandbox policy default-denies filesystem escape, network, process, device, env, and secret access unless explicitly granted and recorded.
- Sandbox outputs include canonical hashed artifact bundles, manifests, logs, environment metadata, and redaction state.
- Validation descriptors run deterministic checks and store typed PASS, FAIL, BLOCKED, ADVISORY_ONLY, UNSUPPORTED, SKIPPED_WITH_REASON, or ERROR results.
- PromotionGate accepts only validated candidates and appends durable EventLedger events linked to validation and approval evidence.
- Promotion rejection receipts cover stale candidate, duplicate idempotency key, validation failure, policy denial, missing approval, missing artifact, Postgres failure, and projection rebuild failure.
- MTE resource caps, blocked taxonomy, retry budget, smart drop-back, per-MT summaries, aggregate summaries, and lane settlement are typed and durable.
- DCC or equivalent projection shows sandbox runs, blocked reasons, validation reports, promotion decisions, and evidence links.
- Visual validation evidence can be attached when GUI/browser checks are in scope.
- Kernel003 authority uses Postgres/EventLedger and does not introduce SQLite authority, fallback, fixture, compatibility, or offline path.
- Validation and promotion evidence remains reconstructable after restart without provider chat history, terminal scrollback, or hidden session context.
- Generated artifacts, logs, and external tool outputs remain under configured artifact roots and disk-agnostic paths.
- Implementation closeout requests Integration Validator batch review and does not self-claim PASS/FAIL.

- PRIMITIVES_EXPOSED:
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-ArtifactManifest
  - PRIM-ValidationExecution
  - PRIM-ValidationFinding
  - PRIM-ValidationRecord
  - PRIM-ValidationResult
  - PRIM-ValidationStatus
  - PRIM-PromotionGates
  - PRIM-PromotionGateSnapshot
  - PRIM-PromotionPath
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: .GOV/spec/master-spec-v02.185/indexed-spec-manifest.json (recorded_at: 2026-05-15T00:40:02.917Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.185]
- SPEC_ANCHOR_PRIMARY: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.9
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
  - .GOV/task_packets/stubs/WP-KERNEL-003-Sandbox-Validation-Promotion-v1.md
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md
  - .GOV/spec/master-spec-v02.185/spec-modules/05-security-and-observability.md
  - .GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md
  - .GOV/spec/master-spec-v02.185/spec-modules/11-shared-dev-platform-and-oss-foundations.md
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/tests/**
- SEARCH_TERMS:
  - KernelSandboxRunV1
  - SandboxPolicyV1
  - SandboxAdapter
  - ValidationDescriptor
  - ValidationRun
  - PromotionCandidate
  - PromotionReceipt
  - EventLedger
  - ToolGate
  - ArtifactManifest
  - WriteBoxV1
  - Sandbox MUST prevent filesystem escape
  - no SQLite
- RUN_COMMANDS:
  ```bash
just pre-work WP-KERNEL-003-Sandbox-Validation-Promotion-v1
  cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  ```
- RISK_MAP:
  - "Policy sandbox mistaken for hard isolation" -> "unsafe operator/model trust"
  - "Raw validation command passthrough" -> "repo mutation or unbounded execution"
  - "Path escape" -> "sandbox can read/write outside declared roots"
  - "Secret leakage in evidence" -> "unsafe artifact export"
  - "Stale or duplicate promotion" -> "authority corruption"
  - "SQLite authority reintroduced" -> "Kernel V1 replay/promotion drift"
  - "MT condensation" -> "folded obligations lost"
## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- For `PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1`, this section is copied from the signed refinement and should not drift.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - DCC sandbox run list.
  - DCC sandbox run detail.
  - Validation report and findings projection.
  - Promotion eligibility/control state.
  - Evidence/debug bundle drawer.
  - Adapter health/preflight panel.
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: sandbox run detail | Type: icon button | Tooltip: Open manifests, logs, validation, promotion, and replay evidence | Notes: disabled only when run ID is missing.
  - Control: adapter mode filter | Type: segmented control | Tooltip: Filter local policy, hard isolation, unsupported, and blocked adapter states | Notes: stable row dimensions.
  - Control: promotion request | Type: action button | Tooltip: Request catalog-backed promotion for an eligible validated candidate | Notes: disabled with visible missing requirements.
  - Control: evidence export | Type: icon button | Tooltip: Export redacted canonical bundle | Notes: exportability flags drive availability.
- UI_STATES (empty/loading/error):
  - Empty run list, adapter unsupported, policy denied, validation blocked, promotion ineligible, artifact missing, stale projection, and replay unavailable states.
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Use Sandbox run, Adapter, Policy, Validation, Promotion, Evidence, Blocked reason, Replay, Redacted export, and Unsupported isolation.
- UI_ACCESSIBILITY_NOTES:
  - Stable row IDs, focusable tooltips, visible disabled reasons, and no reliance on color alone for verdicts.
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
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable `just phase-check HANDOFF <WP_ID> CODER`. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `N/A (fill after implementation)`
- **Start**: N/A
- **End**: N/A
- **Line Delta**: N/A
- **Pre-SHA1**: `N/A`
- **Post-SHA1**: `N/A`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> .GOV/spec/master-spec-v02.185/indexed-spec-manifest.json
- **Notes**:
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS:
- What changed in this update:
- Requirements / clauses self-audited:
- Checks actually run:
- Known gaps / weak spots:
- Heuristic risks / maintainability concerns:
- Validator focus request:
- Rubric contract understanding proof:
- Rubric scope discipline proof:
- Rubric baseline comparison:
- Rubric end-to-end proof:
- Rubric architecture fit self-review:
- Rubric heuristic quality self-review:
- Rubric anti-gaming / counterfactual check:
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check:
- Signed-scope debt ledger:
- Data contract self-check:
- Next step / handoff hint:

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
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `N/A (fill during implementation)`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

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
