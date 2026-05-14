<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.work_packet_contract@1 source_file=.GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json source_hash=03a234ef2c7b0932 projection_hash=1cfcce1dfdae379f generated_at_utc=2026-05-14T10:27:06.028Z generator=ensure-wp-communications.mjs -->
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

# Task Packet: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1

## METADATA
- TASK_ID: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- WP_ID: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- BASE_WP_ID: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening
- DATE: 2026-05-14T04:54:49.177Z
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
- CODER_RESUME_COMMAND: just coder-next WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
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
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
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
- **Status:** In Progress
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: NOT_STARTED
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: NONE
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
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
- BUILD_ORDER_DEPENDS_ON: WP-KERNEL-001-Event-Ledger-Session-Broker, WP-1-Global-Silent-Edit-Guard, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Artifact-System-Foundations, WP-1-Role-Mailbox, WP-1-Micro-Task-Executor, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Postgres-Primary-Control-Plane-Foundation
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-KERNEL-003-Sandbox-Validation-Promotion, WP-KERNEL-004-Local-Model-Memory-Runtime, WP-1-Software-Delivery-Runtime-Truth, WP-1-Workflow-Transition-Automation-Registry, WP-1-Dev-Command-Center-MVP, WP-1-FEMS-Working-Memory-Checkpoint-Schema, WP-1-Session-Spawn-Tree-DCC-Visualization, WP-1-Session-Spawn-Conversation-Distillation, WP-1-Visual-Debugging-Loop
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: YES
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- LOCAL_WORKTREE_DIR: ../wtc-preuse-hardening-v1
- REMOTE_BACKUP_BRANCH: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: NONE
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: INTEGRATION_BATCH_REVIEW_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja140520260455
- PACKET_FORMAT_VERSION: 2026-04-06
- AUTHORITATIVE_CONTRACT_SCHEMA_ID: hsk.work_packet_contract@1
- AUTHORITATIVE_CONTRACT_FILE: .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json
- GENERATED_MARKDOWN_PROJECTION_FILE: .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.md
- REFINEMENT_CONTRACT_SCHEMA_ID: hsk.refinement_contract@1
- REFINEMENT_CONTRACT_FILE: .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/refinement.json
- MICROTASK_CONTRACT_SCHEMA_ID: hsk.microtask_contract@1
- MICROTASK_CONTRACT_GLOB: .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-*.json
- MARKDOWN_PROJECTION_STATUS: GENERATED_IN_SYNC
<!-- Allowed: GENERATED_PENDING | GENERATED_IN_SYNC | LEGACY_AUTHORITY | BLOCKED. New packets use packet.json/refinement.json/MT-*.json as authoritative deterministic contracts; packet.md/refinement.md/MT-*.md are generated projections or frozen legacy references, not sidecar authority. -->
- RED_TEAM_REQUIRED: YES
- RED_TEAM_PROFILE: DETERMINISTIC_CONTRACT_MIGRATION_V1
<!-- Assume stale projections, shadow prose authority, schema omissions, round-trip loss, lifecycle split drift, and Activation Manager / Classic Orchestrator divergence until machine checks prove otherwise. -->

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: Implementation is in progress; awaiting coder handoff to WP validator.
Next: CODER completes in-scope work and records CODER_HANDOFF with proof.
## WORKTREE_CLEANUP_STATUS (STATUS-SYNC APPENDIX; PRODUCT-CODE ONLY)
- CHECK_TYPE: PRODUCT_CODE_ONLY_WORKTREE_CONTAINMENT
- CHECKED_AT_UTC: 2026-05-14T20:52:00Z
- CHECKED_BY: INTEGRATION_VALIDATOR
- MAIN_HEAD: c5fa320e18ef9e1f13993811df77d30c3a25a538
- WORKTREE_DIR: ../wtc-preuse-hardening-v1
- WORK_BRANCH: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- WORKTREE_HEAD: 55cedf7459298f1e52ced8d89a37602b47e31731
- BRANCH_HEAD_ANCESTOR_OF_MAIN: YES
- COMMITTED_PRODUCT_DIFF_VS_MAIN_COUNT: 0
- TRACKED_DIRTY_PRODUCT_COUNT: 1
- UNTRACKED_PRODUCT_COUNT: 97
- CLEANUP_RECOMMENDATION: NOT_DELETE_SAFE_UNCOMMITTED_PRODUCT_WORK
- SUMMARY: The branch HEAD itself is contained in main, but the worktree carries uncommitted product work that is not contained in main.
- EVIDENCE:
  - tracked_dirty_product: src/backend/handshake_core/src/lib.rs
  - untracked_product_count: 97
  - examples: src/backend/handshake_core/src/kernel/action_catalog.rs, src/backend/handshake_core/src/kernel/crdt/mod.rs, src/backend/handshake_core/tests/kernel_action_catalog_tests.rs

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: MT-001 Fold Preservation Manifest and Source Import: materialize the complete folded-source manifest in the official packet/refinement. Acceptance: every listed source stub has path, pre-fold hash, direct/transitive fold classification, and source-scope import instructions. Activation cannot proceed if any source file is missing or hash mismatch is unexplained. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-002 Reset Invariant Reconciliation: reconcile folded legacy assumptions with reset invariants. Acceptance: every source obligation that mentions SQLite, Markdown authority, mailbox chronology, or UI-local truth is explicitly converted to Postgres authority, projection/advisory status, or promotion-gated action semantics. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-003 CRDT Library and Storage ADR: compare Yjs, Loro, Automerge, and existing product dependencies against Handshake runtime needs. Acceptance: ADR selects a CRDT approach, rejected options, sync/storage model, Rust/TypeScript integration boundary, schema compatibility, and validation plan. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-004 Kernel Action Envelope: define `KernelActionRequestV1`, `KernelActionResultV1`, `KernelActionDenialV1`, and receipt/event mappings. Acceptance: action requests carry actor/session/profile, target ids, input schema id, expected write boxes, authority effect, approval posture, validation requirements, and trace id. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-005 Action Catalog Registry: implement the durable `KernelActionCatalogV1` registry. Acceptance: every model-facing action has stable id, schemas, role eligibility, capability requirements, write boxes, promotion path, validation hooks, and DCC preview metadata. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-006 Write Box Schema Family: define `DraftBox`, `CRDTWorkspaceBox`, `ProposalBox`, `PatchBox`, `ArtifactBox`, `MirrorAdvisoryBox`, `MemoryBox`, `ExecutionBox`, and `PromotionBox`. Acceptance: each write box has lifecycle state, owner, allowed transitions, authority effect, evidence refs, validation status, and projection rules. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-007 Direct Edit Denial Path: route model/tool attempts to mutate authority artifacts through ToolGate denial or proposal wrapping. Acceptance: tests prove raw authority-file edit attempts fail with actionable denial evidence and lawful replacement action ids. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-008 Advisory Edit Normalization: convert manual/model edits against generated mirrors into `MirrorAdvisoryBox` records. Acceptance: advisory edits do not mutate authority until a registered normalization/promotion action validates and accepts them. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-009 No-Context Model Manual: create durable model-facing instructions for using Handshake mechanically. Acceptance: the manual explains purpose, startup, action catalog, write boxes, DCC paths, CRDT workflow, safety constraints, failure modes, denial recovery, and validation evidence for a model with no conversation history. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-010 CRDT Document Identity and Workspace Model: define document/workspace ids, actor ids, site/client ids, schema ids, and authority links. Acceptance: CRDT records can be linked to work item, action request, artifact proposal, Role Mailbox thread, DCC projection, and EventLedger ids. | CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-011 CRDT Update Persistence: persist CRDT updates in Postgres with ordering, hash, actor/session attribution, and replay metadata. Acceptance: a workspace can be reconstructed from persisted updates after restart without file-system authority assumptions. | CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-012 CRDT Snapshot and Compaction: add snapshot/state-vector or equivalent sync cursor support. Acceptance: update replay is bounded by snapshots, old updates remain auditable or compacted according to policy, and compaction never drops promotion evidence. | CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-013 CRDT Context Slicing for Models: expose summaries, selected ranges, field digests, and operation deltas. Acceptance: model prompts can request bounded CRDT context without loading entire documents, and extract outputs cite workspace/version/source ids. | CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-014 CRDT Schema and Validity Guard: validate CRDT materialized state before promotion. Acceptance: structurally invalid, unauthorized, or schema-drifted CRDT state cannot be promoted into authority. | CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-015 CRDT Promotion Bridge: convert CRDT edits/drafts into ArtifactProposal and PromotionGate inputs. Acceptance: accepted promotions emit EventLedger authority events; rejected promotions keep CRDT/draft state as non-authoritative evidence. | CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-016 Conflict and Presence Projection: expose presence, pending conflicts, actor attribution, and merge/proposal state. Acceptance: DCC can show who changed what, which edits are merely merged CRDT state, and which changes are pending promotion. | CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-017 Software-Delivery Runtime Truth Records: fold `WP-1-Software-Delivery-Runtime-Truth-v1`. Acceptance: current software-delivery posture is queryable from product-owned stable records and governed actions, not packet prose, mailbox order, or Markdown freshness. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-018 Workflow Transition Automation Registry: fold `WP-1-Workflow-Transition-Automation-Registry-v1`. Acceptance: every workflow mutation has a registered transition rule, eligible actor, action trigger, approval boundary, and DCC preview. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-019 Governance Overlay Boundary: fold `WP-1-Software-Delivery-Governance-Overlay-Boundary-v1`. Acceptance: imported repo `.GOV/**` artifacts are evidence/source overlays, not runtime truth, and import/export cannot bypass gates. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-020 Overlay Coordination Records: fold `WP-1-Software-Delivery-Overlay-Coordination-Records-v1`. Acceptance: claim/lease, queued steering, follow-up, takeover, and actor eligibility are queryable by stable ids without mailbox chronology. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-021 Overlay Lifecycle and Recovery Control Plane: fold `WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1`. Acceptance: start, steer, cancel, close, recover, checkpoint replay, partial failure, and restart posture are record-backed and projection-safe. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-022 Postgres Control-Plane Residual Scope: fold `WP-1-Postgres-Control-Plane-Shift-Bundle-v1` plus its transitive folded stubs. Acceptance: residual live Postgres service proof, leases/backpressure, ModelSession queues, FEMS memory store, durable workflow execution, DCC projections, and SQLite boundary obligations are carried into Kernel002 or explicitly mapped to Kernel003/Kernel004 without reopening the old bundle. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-023 Locus Work Tracking Reset Migration: fold `WP-1-Locus-Work-Tracking-System-Phase1-v1`. Acceptance: WP/MT tracking, dependencies, occupancy, query, Task Board projection, and Flight Recorder obligations are preserved, but SQLite authority is replaced with Postgres/EventLedger/CRDT-compatible authority. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-024 DCC MVP Runtime Surface: fold `WP-1-Dev-Command-Center-MVP-v1`. Acceptance: DCC can select work, view worktree/session/action/proposal state, inspect diffs/evidence, preview approvals, and trigger governed actions through the catalog. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-025 DCC Structured Artifact Viewer: fold `WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1`. Acceptance: DCC renders canonical fields before mirrors, exposes mirror state, and provides raw structured drilldown as advanced view. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-026 DCC Layout Projection Registry: fold `WP-1-Dev-Command-Center-Layout-Projection-Registry-v1`. Acceptance: board, queue, list, roadmap, inbox-triage, and execution-queue views derive from registered presets and action bindings. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-027 Role Mailbox Message and Action Request Contract: fold `WP-1-Role-Mailbox-Message-Thread-Contract-v1`. Acceptance: mailbox lifecycle, delivery state, allowed responses, due/dead-letter posture, and action requests are typed and authority-bounded. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-028 Role Mailbox Micro-Task Loop Control: fold `WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1`. Acceptance: retry budget, verifier outcome, escalation, completion report, dead-letter, and loop checkpoint state are compact and replayable. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-029 Role Mailbox Triage Queue Controls: fold `WP-1-Role-Mailbox-Triage-Queue-Controls-v1`. Acceptance: reminder, snooze, expiry, dead-letter, retry/reroute/archive, and Task Board pressure overlays are field-backed projections. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-030 Role Mailbox Claim and Lease: fold `WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1`. Acceptance: claimant, claim mode, lease age/expiry, takeover legality, and responder eligibility are explicit and queryable. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-031 Role Mailbox Handoff and Announce-Back: fold `WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1`. Acceptance: handoff bundles, transcription targets, recommended next actor, announce-back provenance, and advisory/completion distinction are typed. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-032 Role Mailbox Inbox Alignment and Evidence Bridge: fold `WP-1-Inbox-Role-Mailbox-Alignment-v1` and `WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1`. Acceptance: Inbox labels map to Role Mailbox only, mailbox telemetry is leak-safe, and debug bundle exports preserve stable evidence/provenance. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-033 FEMS Working-Memory Checkpoints: fold `WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1`. Acceptance: SESSION_OPEN, PRE_TASK, INSIGHT, TASK_COMPLETE, SESSION_CLOSE, memory extract, repeated insight promotion, and GC are typed and quality-gated. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-034 FEMS Write-Time Safeguards: fold `WP-1-FEMS-Write-Time-Safeguards-v1`. Acceptance: novelty scoring, supersession, contradiction detection, dedup, state validation, and audit trail run mechanically; SQLite/FTS5 references are reworked to reset-approved storage/search primitives. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-035 FEMS Memory Poisoning and Drift Guardrails: fold `WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1`. Acceptance: trust gates, pack budget, deterministic reduction, proposal/approval/denial events, and effective pack hashes prevent untrusted long-lived memory drift. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-036 FEMS MT Handoff Memory Context: fold `WP-1-FEMS-MT-Handoff-Memory-Context-v1`. Acceptance: escalated or handed-off MTs carry typed memory context with source/target sessions, failed attempts, recommended items, provenance, and bounded scoring. | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-037 Role Turn Isolation: fold `WP-1-Role-Turn-Isolation-v1`. Acceptance: role turns default to isolated context, replay pins are recorded, and cross-role bleed is mechanically prevented. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-038 Work Profiles: fold `WP-1-Work-Profiles-v1`. Acceptance: profile storage, selection, immutable profile ids, per-role routing, autonomy knobs, and profile receipts are wired into action requests. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-039 Local-First Agentic MCP Posture: fold `WP-1-LocalFirst-Agentic-MCP-Posture-v1`. Acceptance: local-first execution remains default; MCP/cloud paths are capability-gated adapters with cached artifacts and fallback behavior. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-040 Git Engine Decision Gate: fold `WP-1-Git-Engine-Decision-Gate-v1`. Acceptance: one repo engine path is recorded/enforced, dangerous git actions remain gated, and DCC/action catalog expose only lawful git affordances. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-041 Session Anti-Pattern Registry: fold `WP-1-Session-Anti-Pattern-Registry-v1`. Acceptance: scheduler/trust/capability/session orchestration anti-patterns have machine-readable detections and deny/downgrade/consent/stop outcomes. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-042 Governance Pack Instantiation: fold `WP-1-Governance-Pack-v1`. Acceptance: project identity, pack manifest, instantiation, naming/path policy, conformance harness, and imported-overlay boundaries are compatible with Kernel002 action/write-box law. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-043 Session Spawn Tree DCC Visualization: fold `WP-1-Session-Spawn-Tree-DCC-Visualization-v1`. Acceptance: DCC shows spawn hierarchy, child counts, depth, cascade cancel, spawn mode, and announce-back badges from runtime records. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-044 Session Spawn Conversation Distillation: fold `WP-1-Session-Spawn-Conversation-Distillation-v1`. Acceptance: parent-child request/summary pairs and spawn metadata feed distillation artifacts without making conversation text authority. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-045 Product Screenshot Capture: fold `WP-1-Product-Screenshot-Visual-Validation-v1`. Acceptance: governed sessions can capture full app, panel, and module screenshots with metadata and artifact refs. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-046 Visual Debugging Loop: fold `WP-1-Visual-Debugging-Loop-v1`. Acceptance: post-commit or post-action screenshot capture, baseline comparison, visual evidence storage, threshold config, and validator steering are available for GUI work. | CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-047 Markdown Mirror Sync Drift Guard: fold `WP-1-Markdown-Mirror-Sync-Drift-Guard-v1`. Acceptance: deterministic mirror regeneration, drift states, manual advisory handling, reconciliation, DCC mirror queue, and projection banners are implemented. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-048 Direct-Edit Regression Harness: prove future models cannot bypass write boxes through common edit paths. Acceptance: tests simulate model raw patch, generated file write, mirror edit, CRDT edit, mailbox reply, DCC quick action, and git action; each path either uses registered action/write box or fails with evidence. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-049 Projection Rebuild and Task Board Sync: regenerate projections and sync Task Board, traceability registry, build order, and stub contracts. Acceptance: `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check` pass or produce a concrete blocker. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-050 Pre-Use Kernel Acceptance Run: prove Kernel001 + Kernel002 are usable before real kernel operation. Acceptance: a no-context model follows the manual to draft in CRDT, submit a proposal, trigger validation, receive a promotion/denial, view DCC projections, and inspect evidence without direct authority-file edits. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-051 Stub, Work Packet, and Microtask Contract Lifecycle: define the machine-readable lifecycle from inactive stub to active work packet to generated microtask contracts. Acceptance: `StubContractV1`, `WorkPacketContractV1`, and `MicroTaskContractV1` schemas define states, authority rules, required fields, provenance hashes, source imports, lifecycle transitions, receipt events, projection hooks, validation hooks, and failure states. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-052 Work Packet Full-Detail Authority and Microtask Source Plan: ensure the activated work packet itself carries full implementation detail while also containing a structured MT source plan. Acceptance: a no-context strong model can execute from the work packet alone; the same packet can regenerate MT contracts/files without relying on manually maintained sidecars or hidden chat context. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-053 Mechanical Stub Promotion and Microtask Extraction: implement deterministic commands or action-catalog entries for stub-to-WP promotion and WP-to-MT extraction. Acceptance: promotion/extraction preserves operator intent, source hashes, folded details, dependencies, constraints, acceptance criteria, verification, and status provenance; every generated artifact records its source contract id and hash. | CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-054 Local-Model Fresh-Context Microtask Loop Contract: define the Locus-compatible execution loop for smaller/local models working one MT at a time. Acceptance: the loop contract covers fresh-context input bundle, allowed actions, write boxes, retry budget, verifier handoff, failure requeue, memory checkpoint input, receipt emission, and final MT outcome without requiring the model to inspect unrelated WP scope. | CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-055 Generated Documentation and Status Projection: replace manual status/docs maintenance with projections from contracts, receipts, runtime state, and validation outputs. Acceptance: packet status, MT status, Task Board rows, traceability rows, DCC work views, mirror docs, and operator summaries regenerate from machine-readable authority; direct manual status edits are denied or captured as advisory normalization input. | CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-056 Coder Handoff and Validation Request Contract: define the structured handoff from coder execution to Handshake-owned validation. Acceptance: `CoderHandoffContractV1` records MT id, parent WP id, actor/session, claimed scope, touched files/actions, receipts, tests, evidence, known blockers, and requested review; Handshake can generate a validator review request from it without a model editing status fields. | CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-057 Validator Verdict and Mediation Contract: define structured pass/fail/mediation verdicts from Integration Validator batch review for this packet. Acceptance: `ValidatorVerdictContractV1` and `MediationInstructionContractV1` encode verdict, failed acceptance criteria, evidence refs, severity, reproducibility, exact remediation instructions, dependency impact, and whether the MT may advance, must loop back, or must escalate. | CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-058 Mechanical Issue, Bug, Gap, and Out-of-Scope Reports: define machine-readable reports for validator findings that are not simple pass/fail. Acceptance: `IssueReportContractV1`, `BugReportContractV1`, `GapReportContractV1`, and `OutOfScopeReportContractV1` preserve validator reasoning, source refs, affected surfaces, reproduction or proof, proposed destination, and routing outcome without becoming manual prose-only reports. | CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-059 Remediation Microtask and Packet Generation: generate follow-up work from failed verdicts and reports. Acceptance: Handshake can create `RemediationMicroTaskContractV1` or a remediation packet/stub from verdict/report contracts, preserving parent WP/MT links, dependency state, acceptance criteria, allowed actions, write boxes, evidence refs, retry budget, and validator recheck requirements. | CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-060 Loop Scheduler and Next-Coder Dispatch: define the mechanical loop that dispatches coders after validation outcomes. Acceptance: Handshake only dispatches a new coder when leases, current coder completion, dependency state, retry budget, and verdict state allow it; failed prerequisites loop to remediation before dependent MTs can advance. | CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: MT-061 Locus Work Graph Projection for MT Validation Loops: connect the validation/remediation loop to Locus work tracking semantics from the Master Spec. Acceptance: Locus can project MT nodes, validator verdicts, remediation edges, blocked/escalated states, actor leases, and pass/fail history without treating prose reports or chat messages as truth. | CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## PACKET_ACCEPTANCE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the executable acceptance contract for packet closure. New packets must keep stable row IDs and move each required row to PROVED, CONFIRMED, or NOT_APPLICABLE with evidence before PASS.
- Rule: use STEER or BLOCKED for unresolved required rows instead of narrative closure.
- ACCEPTANCE_ROWS:
  - ID: AC-001 | REQUIREMENT: MT-001 Fold Preservation Manifest and Source Import: materialize the complete folded-source manifest in the official packet/refinement. Acceptance: every listed source stub has path, pre-fold hash, direct/transitive fold classification, and source-scope import instructions. Activation cannot proceed if any source file is missing or hash mismatch is unexplained. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-002 | REQUIREMENT: MT-002 Reset Invariant Reconciliation: reconcile folded legacy assumptions with reset invariants. Acceptance: every source obligation that mentions SQLite, Markdown authority, mailbox chronology, or UI-local truth is explicitly converted to Postgres authority, projection/advisory status, or promotion-gated action semantics. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-003 | REQUIREMENT: MT-003 CRDT Library and Storage ADR: compare Yjs, Loro, Automerge, and existing product dependencies against Handshake runtime needs. Acceptance: ADR selects a CRDT approach, rejected options, sync/storage model, Rust/TypeScript integration boundary, schema compatibility, and validation plan. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-004 | REQUIREMENT: MT-004 Kernel Action Envelope: define `KernelActionRequestV1`, `KernelActionResultV1`, `KernelActionDenialV1`, and receipt/event mappings. Acceptance: action requests carry actor/session/profile, target ids, input schema id, expected write boxes, authority effect, approval posture, validation requirements, and trace id. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-005 | REQUIREMENT: MT-005 Action Catalog Registry: implement the durable `KernelActionCatalogV1` registry. Acceptance: every model-facing action has stable id, schemas, role eligibility, capability requirements, write boxes, promotion path, validation hooks, and DCC preview metadata. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-006 | REQUIREMENT: MT-006 Write Box Schema Family: define `DraftBox`, `CRDTWorkspaceBox`, `ProposalBox`, `PatchBox`, `ArtifactBox`, `MirrorAdvisoryBox`, `MemoryBox`, `ExecutionBox`, and `PromotionBox`. Acceptance: each write box has lifecycle state, owner, allowed transitions, authority effect, evidence refs, validation status, and projection rules. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-007 | REQUIREMENT: MT-007 Direct Edit Denial Path: route model/tool attempts to mutate authority artifacts through ToolGate denial or proposal wrapping. Acceptance: tests prove raw authority-file edit attempts fail with actionable denial evidence and lawful replacement action ids. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-008 | REQUIREMENT: MT-008 Advisory Edit Normalization: convert manual/model edits against generated mirrors into `MirrorAdvisoryBox` records. Acceptance: advisory edits do not mutate authority until a registered normalization/promotion action validates and accepts them. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-009 | REQUIREMENT: MT-009 No-Context Model Manual: create durable model-facing instructions for using Handshake mechanically. Acceptance: the manual explains purpose, startup, action catalog, write boxes, DCC paths, CRDT workflow, safety constraints, failure modes, denial recovery, and validation evidence for a model with no conversation history. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-010 | REQUIREMENT: MT-010 CRDT Document Identity and Workspace Model: define document/workspace ids, actor ids, site/client ids, schema ids, and authority links. Acceptance: CRDT records can be linked to work item, action request, artifact proposal, Role Mailbox thread, DCC projection, and EventLedger ids. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-011 | REQUIREMENT: MT-011 CRDT Update Persistence: persist CRDT updates in Postgres with ordering, hash, actor/session attribution, and replay metadata. Acceptance: a workspace can be reconstructed from persisted updates after restart without file-system authority assumptions. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-012 | REQUIREMENT: MT-012 CRDT Snapshot and Compaction: add snapshot/state-vector or equivalent sync cursor support. Acceptance: update replay is bounded by snapshots, old updates remain auditable or compacted according to policy, and compaction never drops promotion evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-013 | REQUIREMENT: MT-013 CRDT Context Slicing for Models: expose summaries, selected ranges, field digests, and operation deltas. Acceptance: model prompts can request bounded CRDT context without loading entire documents, and extract outputs cite workspace/version/source ids. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-014 | REQUIREMENT: MT-014 CRDT Schema and Validity Guard: validate CRDT materialized state before promotion. Acceptance: structurally invalid, unauthorized, or schema-drifted CRDT state cannot be promoted into authority. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-015 | REQUIREMENT: MT-015 CRDT Promotion Bridge: convert CRDT edits/drafts into ArtifactProposal and PromotionGate inputs. Acceptance: accepted promotions emit EventLedger authority events; rejected promotions keep CRDT/draft state as non-authoritative evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-016 | REQUIREMENT: MT-016 Conflict and Presence Projection: expose presence, pending conflicts, actor attribution, and merge/proposal state. Acceptance: DCC can show who changed what, which edits are merely merged CRDT state, and which changes are pending promotion. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-017 | REQUIREMENT: MT-017 Software-Delivery Runtime Truth Records: fold `WP-1-Software-Delivery-Runtime-Truth-v1`. Acceptance: current software-delivery posture is queryable from product-owned stable records and governed actions, not packet prose, mailbox order, or Markdown freshness. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-018 | REQUIREMENT: MT-018 Workflow Transition Automation Registry: fold `WP-1-Workflow-Transition-Automation-Registry-v1`. Acceptance: every workflow mutation has a registered transition rule, eligible actor, action trigger, approval boundary, and DCC preview. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-019 | REQUIREMENT: MT-019 Governance Overlay Boundary: fold `WP-1-Software-Delivery-Governance-Overlay-Boundary-v1`. Acceptance: imported repo `.GOV/**` artifacts are evidence/source overlays, not runtime truth, and import/export cannot bypass gates. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-020 | REQUIREMENT: MT-020 Overlay Coordination Records: fold `WP-1-Software-Delivery-Overlay-Coordination-Records-v1`. Acceptance: claim/lease, queued steering, follow-up, takeover, and actor eligibility are queryable by stable ids without mailbox chronology. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-021 | REQUIREMENT: MT-021 Overlay Lifecycle and Recovery Control Plane: fold `WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1`. Acceptance: start, steer, cancel, close, recover, checkpoint replay, partial failure, and restart posture are record-backed and projection-safe. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-022 | REQUIREMENT: MT-022 Postgres Control-Plane Residual Scope: fold `WP-1-Postgres-Control-Plane-Shift-Bundle-v1` plus its transitive folded stubs. Acceptance: residual live Postgres service proof, leases/backpressure, ModelSession queues, FEMS memory store, durable workflow execution, DCC projections, and SQLite boundary obligations are carried into Kernel002 or explicitly mapped to Kernel003/Kernel004 without reopening the old bundle. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-023 | REQUIREMENT: MT-023 Locus Work Tracking Reset Migration: fold `WP-1-Locus-Work-Tracking-System-Phase1-v1`. Acceptance: WP/MT tracking, dependencies, occupancy, query, Task Board projection, and Flight Recorder obligations are preserved, but SQLite authority is replaced with Postgres/EventLedger/CRDT-compatible authority. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-024 | REQUIREMENT: MT-024 DCC MVP Runtime Surface: fold `WP-1-Dev-Command-Center-MVP-v1`. Acceptance: DCC can select work, view worktree/session/action/proposal state, inspect diffs/evidence, preview approvals, and trigger governed actions through the catalog. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | REASON: NONE
  - ID: AC-025 | REQUIREMENT: MT-025 DCC Structured Artifact Viewer: fold `WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1`. Acceptance: DCC renders canonical fields before mirrors, exposes mirror state, and provides raw structured drilldown as advanced view. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | REASON: NONE
  - ID: AC-026 | REQUIREMENT: MT-026 DCC Layout Projection Registry: fold `WP-1-Dev-Command-Center-Layout-Projection-Registry-v1`. Acceptance: board, queue, list, roadmap, inbox-triage, and execution-queue views derive from registered presets and action bindings. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | REASON: NONE
  - ID: AC-027 | REQUIREMENT: MT-027 Role Mailbox Message and Action Request Contract: fold `WP-1-Role-Mailbox-Message-Thread-Contract-v1`. Acceptance: mailbox lifecycle, delivery state, allowed responses, due/dead-letter posture, and action requests are typed and authority-bounded. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-028 | REQUIREMENT: MT-028 Role Mailbox Micro-Task Loop Control: fold `WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1`. Acceptance: retry budget, verifier outcome, escalation, completion report, dead-letter, and loop checkpoint state are compact and replayable. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-029 | REQUIREMENT: MT-029 Role Mailbox Triage Queue Controls: fold `WP-1-Role-Mailbox-Triage-Queue-Controls-v1`. Acceptance: reminder, snooze, expiry, dead-letter, retry/reroute/archive, and Task Board pressure overlays are field-backed projections. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-030 | REQUIREMENT: MT-030 Role Mailbox Claim and Lease: fold `WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1`. Acceptance: claimant, claim mode, lease age/expiry, takeover legality, and responder eligibility are explicit and queryable. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-031 | REQUIREMENT: MT-031 Role Mailbox Handoff and Announce-Back: fold `WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1`. Acceptance: handoff bundles, transcription targets, recommended next actor, announce-back provenance, and advisory/completion distinction are typed. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-032 | REQUIREMENT: MT-032 Role Mailbox Inbox Alignment and Evidence Bridge: fold `WP-1-Inbox-Role-Mailbox-Alignment-v1` and `WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1`. Acceptance: Inbox labels map to Role Mailbox only, mailbox telemetry is leak-safe, and debug bundle exports preserve stable evidence/provenance. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-033 | REQUIREMENT: MT-033 FEMS Working-Memory Checkpoints: fold `WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1`. Acceptance: SESSION_OPEN, PRE_TASK, INSIGHT, TASK_COMPLETE, SESSION_CLOSE, memory extract, repeated insight promotion, and GC are typed and quality-gated. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-034 | REQUIREMENT: MT-034 FEMS Write-Time Safeguards: fold `WP-1-FEMS-Write-Time-Safeguards-v1`. Acceptance: novelty scoring, supersession, contradiction detection, dedup, state validation, and audit trail run mechanically; SQLite/FTS5 references are reworked to reset-approved storage/search primitives. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-035 | REQUIREMENT: MT-035 FEMS Memory Poisoning and Drift Guardrails: fold `WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1`. Acceptance: trust gates, pack budget, deterministic reduction, proposal/approval/denial events, and effective pack hashes prevent untrusted long-lived memory drift. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-036 | REQUIREMENT: MT-036 FEMS MT Handoff Memory Context: fold `WP-1-FEMS-MT-Handoff-Memory-Context-v1`. Acceptance: escalated or handed-off MTs carry typed memory context with source/target sessions, failed attempts, recommended items, provenance, and bounded scoring. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-037 | REQUIREMENT: MT-037 Role Turn Isolation: fold `WP-1-Role-Turn-Isolation-v1`. Acceptance: role turns default to isolated context, replay pins are recorded, and cross-role bleed is mechanically prevented. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-038 | REQUIREMENT: MT-038 Work Profiles: fold `WP-1-Work-Profiles-v1`. Acceptance: profile storage, selection, immutable profile ids, per-role routing, autonomy knobs, and profile receipts are wired into action requests. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-039 | REQUIREMENT: MT-039 Local-First Agentic MCP Posture: fold `WP-1-LocalFirst-Agentic-MCP-Posture-v1`. Acceptance: local-first execution remains default; MCP/cloud paths are capability-gated adapters with cached artifacts and fallback behavior. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-040 | REQUIREMENT: MT-040 Git Engine Decision Gate: fold `WP-1-Git-Engine-Decision-Gate-v1`. Acceptance: one repo engine path is recorded/enforced, dangerous git actions remain gated, and DCC/action catalog expose only lawful git affordances. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-041 | REQUIREMENT: MT-041 Session Anti-Pattern Registry: fold `WP-1-Session-Anti-Pattern-Registry-v1`. Acceptance: scheduler/trust/capability/session orchestration anti-patterns have machine-readable detections and deny/downgrade/consent/stop outcomes. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-042 | REQUIREMENT: MT-042 Governance Pack Instantiation: fold `WP-1-Governance-Pack-v1`. Acceptance: project identity, pack manifest, instantiation, naming/path policy, conformance harness, and imported-overlay boundaries are compatible with Kernel002 action/write-box law. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-043 | REQUIREMENT: MT-043 Session Spawn Tree DCC Visualization: fold `WP-1-Session-Spawn-Tree-DCC-Visualization-v1`. Acceptance: DCC shows spawn hierarchy, child counts, depth, cascade cancel, spawn mode, and announce-back badges from runtime records. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | REASON: NONE
  - ID: AC-044 | REQUIREMENT: MT-044 Session Spawn Conversation Distillation: fold `WP-1-Session-Spawn-Conversation-Distillation-v1`. Acceptance: parent-child request/summary pairs and spawn metadata feed distillation artifacts without making conversation text authority. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-045 | REQUIREMENT: MT-045 Product Screenshot Capture: fold `WP-1-Product-Screenshot-Visual-Validation-v1`. Acceptance: governed sessions can capture full app, panel, and module screenshots with metadata and artifact refs. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | REASON: NONE
  - ID: AC-046 | REQUIREMENT: MT-046 Visual Debugging Loop: fold `WP-1-Visual-Debugging-Loop-v1`. Acceptance: post-commit or post-action screenshot capture, baseline comparison, visual evidence storage, threshold config, and validator steering are available for GUI work. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | REASON: NONE
  - ID: AC-047 | REQUIREMENT: MT-047 Markdown Mirror Sync Drift Guard: fold `WP-1-Markdown-Mirror-Sync-Drift-Guard-v1`. Acceptance: deterministic mirror regeneration, drift states, manual advisory handling, reconciliation, DCC mirror queue, and projection banners are implemented. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-048 | REQUIREMENT: MT-048 Direct-Edit Regression Harness: prove future models cannot bypass write boxes through common edit paths. Acceptance: tests simulate model raw patch, generated file write, mirror edit, CRDT edit, mailbox reply, DCC quick action, and git action; each path either uses registered action/write box or fails with evidence. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-049 | REQUIREMENT: MT-049 Projection Rebuild and Task Board Sync: regenerate projections and sync Task Board, traceability registry, build order, and stub contracts. Acceptance: `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check` pass or produce a concrete blocker. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-050 | REQUIREMENT: MT-050 Pre-Use Kernel Acceptance Run: prove Kernel001 + Kernel002 are usable before real kernel operation. Acceptance: a no-context model follows the manual to draft in CRDT, submit a proposal, trigger validation, receive a promotion/denial, view DCC projections, and inspect evidence without direct authority-file edits. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-051 | REQUIREMENT: MT-051 Stub, Work Packet, and Microtask Contract Lifecycle: define the machine-readable lifecycle from inactive stub to active work packet to generated microtask contracts. Acceptance: `StubContractV1`, `WorkPacketContractV1`, and `MicroTaskContractV1` schemas define states, authority rules, required fields, provenance hashes, source imports, lifecycle transitions, receipt events, projection hooks, validation hooks, and failure states. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-052 | REQUIREMENT: MT-052 Work Packet Full-Detail Authority and Microtask Source Plan: ensure the activated work packet itself carries full implementation detail while also containing a structured MT source plan. Acceptance: a no-context strong model can execute from the work packet alone; the same packet can regenerate MT contracts/files without relying on manually maintained sidecars or hidden chat context. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-053 | REQUIREMENT: MT-053 Mechanical Stub Promotion and Microtask Extraction: implement deterministic commands or action-catalog entries for stub-to-WP promotion and WP-to-MT extraction. Acceptance: promotion/extraction preserves operator intent, source hashes, folded details, dependencies, constraints, acceptance criteria, verification, and status provenance; every generated artifact records its source contract id and hash. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-054 | REQUIREMENT: MT-054 Local-Model Fresh-Context Microtask Loop Contract: define the Locus-compatible execution loop for smaller/local models working one MT at a time. Acceptance: the loop contract covers fresh-context input bundle, allowed actions, write boxes, retry budget, verifier handoff, failure requeue, memory checkpoint input, receipt emission, and final MT outcome without requiring the model to inspect unrelated WP scope. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-055 | REQUIREMENT: MT-055 Generated Documentation and Status Projection: replace manual status/docs maintenance with projections from contracts, receipts, runtime state, and validation outputs. Acceptance: packet status, MT status, Task Board rows, traceability rows, DCC work views, mirror docs, and operator summaries regenerate from machine-readable authority; direct manual status edits are denied or captured as advisory normalization input. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-056 | REQUIREMENT: MT-056 Coder Handoff and Validation Request Contract: define the structured handoff from coder execution to Handshake-owned validation. Acceptance: `CoderHandoffContractV1` records MT id, parent WP id, actor/session, claimed scope, touched files/actions, receipts, tests, evidence, known blockers, and requested review; Handshake can generate a validator review request from it without a model editing status fields. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-057 | REQUIREMENT: MT-057 Validator Verdict and Mediation Contract: define structured pass/fail/mediation verdicts from Integration Validator batch review for this packet. Acceptance: `ValidatorVerdictContractV1` and `MediationInstructionContractV1` encode verdict, failed acceptance criteria, evidence refs, severity, reproducibility, exact remediation instructions, dependency impact, and whether the MT may advance, must loop back, or must escalate. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-058 | REQUIREMENT: MT-058 Mechanical Issue, Bug, Gap, and Out-of-Scope Reports: define machine-readable reports for validator findings that are not simple pass/fail. Acceptance: `IssueReportContractV1`, `BugReportContractV1`, `GapReportContractV1`, and `OutOfScopeReportContractV1` preserve validator reasoning, source refs, affected surfaces, reproduction or proof, proposed destination, and routing outcome without becoming manual prose-only reports. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-059 | REQUIREMENT: MT-059 Remediation Microtask and Packet Generation: generate follow-up work from failed verdicts and reports. Acceptance: Handshake can create `RemediationMicroTaskContractV1` or a remediation packet/stub from verdict/report contracts, preserving parent WP/MT links, dependency state, acceptance criteria, allowed actions, write boxes, evidence refs, retry budget, and validator recheck requirements. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-060 | REQUIREMENT: MT-060 Loop Scheduler and Next-Coder Dispatch: define the mechanical loop that dispatches coders after validation outcomes. Acceptance: Handshake only dispatches a new coder when leases, current coder completion, dependency state, retry budget, and verdict state allow it; failed prerequisites loop to remediation before dependent MTs can advance. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
  - ID: AC-061 | REQUIREMENT: MT-061 Locus Work Graph Projection for MT Validation Loops: connect the validation/remediation loop to Locus work tracking semantics from the Master Spec. Acceptance: Locus can project MT nodes, validator verdicts, remediation edges, blocked/escalated states, actor leases, and pass/fail history without treating prose reports or chat messages as truth. | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: INTEGRATION_VALIDATOR | STATUS: PENDING | EVIDENCE: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | REASON: NONE
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
  - src/backend/handshake_core/src/role_mailbox/**
  - src/backend/handshake_core/src/fems/**
  - src/backend/handshake_core/src/locus/**
  - src/backend/handshake_core/tests/**
  - app/**
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_write_box --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target; cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target; cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target; just gov-check; just spec-eof-appendices-check [LEGACY_REFINEMENT_BRIDGE]
- CANONICAL_CONTRACT_EXAMPLES:
  - NONE
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/** (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/role_mailbox/** (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/locus/** (backend data surface)
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: MT validation loop graph | SUBFEATURES: MT nodes, remediation edges, leases, blocked/escalated states | PRIMITIVES_FEATURES: PRIM-WriteBoxPromotionReceiptV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Postgres authority reset | SUBFEATURES: CRDT updates, snapshots, no SQLite authority | PRIMITIVES_FEATURES: PRIM-CrdtWorkspaceSnapshotV1 | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: Typed model-operable surfaces | SUBFEATURES: catalog schemas, write boxes, CRDT slices, mailbox state, FEMS checkpoints | PRIMITIVES_FEATURES: PRIM-WriteBoxV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Action catalog and denial | JobModel: MECHANICAL_TOOL | Workflow: registered action schemas and denial/proposal receipts | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: action_request/action_denial | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: CRDT workspace promotion | JobModel: WORKFLOW | Workflow: CRDT updates promote through ArtifactProposal and PromotionGate | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: crdt_update/promotion_receipt | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: MT remediation loop | JobModel: WORKFLOW | Workflow: handoff and Integration Validator verdicts create loop records | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: handoff/verdict/remediation | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: DCC visual evidence | JobModel: UI_ACTION | Workflow: catalog/write-box/screenshot/stale-state projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: screenshot/projection_rebuilt | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
  - FORCE_MULTIPLIER_EXPANSION: Locus remediation graph -> IN_THIS_WP (stub: NONE)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/role_mailbox/**
  - src/backend/handshake_core/src/fems/**
  - src/backend/handshake_core/src/locus/**
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
- REFINEMENT_FILE: .GOV/refinements/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.10
- CONTEXT_START_LINE: 3708
- CONTEXT_END_LINE: 3720
- CONTEXT_TOKEN: Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
The Kernel V1 EventLedger is the product authority for the first kernel slice. Postgres notifications, process-local locks, UI state, and provider or framework traces MAY accelerate or display work, but recovery and validation MUST be possible from durable product rows alone.
  
  #### 2.3.13.10 Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]
  
  Kernel V1 CRDT workspace state is pre-promotion working state. It MAY hold concurrent human and model drafts, advisory edits, review notes, and normalized proposed operations, but it is not authority until promotion commits through the Postgres EventLedger defined in Section 2.3.13.9.
  
  The Kernel V1 implementation MUST provide a KernelActionCatalogV1 contract that enumerates every write-capable kernel action before it can mutate a draft or request promotion. Each catalog entry MUST declare a stable action id, target authority class, input schema version, actor eligibility, required capability or approval posture, preview behavior, validation checks, idempotency key policy, and resulting event or receipt type. Ad hoc direct writes that do not resolve through this catalog are forbidden.
  
  The Kernel V1 implementation MUST provide a WriteBoxV1 family for draft mutations. A write box MUST carry a stable write_box_id, workspace_id, actor_id, actor kind, CRDT site id, target record refs, base snapshot or state vector refs, intent summary, operation payload refs, schema version, validation state, denial or promotion receipt refs, and replay metadata. Write boxes MAY normalize advisory text, diffs, CRDT transactions, or model proposals into a common envelope, but they MUST preserve actor provenance and source evidence.
  
  Direct edits to authoritative Kernel V1 records MUST be denied unless they enter through an allowed catalog action and write box path. Denials MUST produce durable WriteBoxDirectEditDeniedV1 evidence with actor, target, attempted action, denial reason, recovery instruction, and linked UI or API response. Denial handling is part of product behavior, not only a validation test.
  
  The CRDT-to-EventLedger promotion bridge MUST be explicit. Promotion MUST read a validated write box, confirm actor eligibility and target authority class, verify schema and CRDT state-vector freshness, reject stale or duplicate promotion requests by idempotency key, and append promotion-request, promotion-accepted, or promotion-rejected events to the Postgres EventLedger. A CRDT merge MUST NOT directly mutate EventLedger authority; it can only make a promotion candidate visible for validation.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#KernelActionCatalogV1
- CONTEXT_START_LINE: 65
- CONTEXT_END_LINE: 77
- CONTEXT_TOKEN: KernelActionCatalogV1
- EXCERPT_ASCII_ESCAPED:
  ```text
| Version | Date | Author | Changes | Approval |
  |---------|------|--------|---------|----------|
  | v02.185 | 2026-05-14 | Kernel Builder | Added Kernel002 authority law: KernelActionCatalogV1, WriteBoxV1, direct-edit denial, advisory edit normalization, CRDT workspace draft persistence, and CRDT-to-EventLedger promotion bridge with DCC projection requirements. [ADD v02.185] | ilja140520260455 |
  | v02.184 | 2026-05-13 | Kernel Builder | Added Kernel V1 authority law: Postgres EventLedger as product runtime authority, SessionBroker/ContextBundle/ModelAdapter boundary, no-SQLite Kernel V1 authority/cache/offline/fallback/test permission, and projection-only Flight Recorder/DCC/diagnostics posture for kernel replay and promotion. [ADD v02.184] | Operator approval in chat, 2026-05-13; WP activation signature pending |
  | v02.183 | 2026-05-13 | Orchestrator | Migrated the active indexed Master Spec into the copy-first versioned bundle `.GOV/spec/master-spec-v02.183/`, moved the previous indexed bundle to `.GOV/spec/spec_archive/master-spec-v02.182/`, added uniform module `spec_version` metadata, added a manifest-declared machine-readable changelog module, updated `.GOV/spec/SPEC_CURRENT.md`, and refreshed internal references away from latest-monolith/version-file wording. [ADD v02.183] | Operator approval in chat, 2026-05-13 |
  | v02.182 | 2026-05-05 | Activation Manager | Added PostgreSQL-primary control-plane foundation law: explicit storage modes, PostgreSQL-authoritative self-hosted runtime records, fail-closed behavior when PostgreSQL is required, SQLite cache/offline boundaries, downstream split for queue workers, leases/backpressure, FEMS memory store, workflow durable execution, DCC projections, SQLite fallback boundaries, and developer/test container setup; updated Appendix 12 feature, primitive, and interaction metadata for the pivot. [ADD v02.182] | APPROVE REFINEMENT WP-1-Postgres-Primary-Control-Plane-Foundation-v1 |
  | v02.181 | 2026-04-17 | Orchestrator | Added software-delivery governance overlay law: product-owned runtime truth over imported repo `/.GOV/**`, validator-gate convergence on top of Governance Check Runner, projection-only Dev Command Center / Task Board / Role Mailbox posture, derived closeout semantics, overlay claim/lease and queued-instruction extension records, explicit overlay lifecycle constraints, and workflow-backed start/steer/cancel/close/recover control-plane law; updated Appendix 12 / roadmap follow-through for the affected feature families. [ADD v02.181] | pending |
  | v02.180 | 2026-04-07 | Orchestrator | Added 7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD) -- typed CheckResult contract (PASS/FAIL/BLOCKED/ADVISORY_ONLY/UNSUPPORTED), tool surface governance.check.run, FR events FR-EVT-GOV-CHECK-001..003, additive overlay rule. [ADD v02.180] | pending |
  | v02.179 | 2026-03-28 | Orchestrator | **Workflow-correlation bundle-scope pass:** patched Debug Bundle export law so `workflow_run` and `workflow_node_execution` become first-class bounded scopes, added workflow-node execution inventory plus manifest-count rules, extended exporter and exportable-inventory posture, deepened FEAT-DEBUG-BUNDLE UI guidance for workflow-scoped export, and kept roadmap/cov-matrix scheduling aligned with the existing Workflow Projection Correlation backlog. | ilja280320262308 |
  | v02.178 | 2026-03-11 | Orchestrator | **RAG mode and no-RAG cross-pillar pass:** clarified that RAG is one governed retrieval mode rather than the default context strategy; added retrieval-mode and non-hybrid-reason law across AI-Ready Data, ACE Runtime, Project Brain, Prompt-to-Spec Router, Loom, Work Packets, and Micro-Task Executor; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for authoritative direct-load, graph-first, and bounded local-model retrieval posture; and materialized a dedicated retrieval-mode policy stub. | ilja110320261228 |
  | v02.177 | 2026-03-11 | Orchestrator | **Role Mailbox handoff-bundle and announce-back provenance pass:** defined structured handoff bundles, announce-back provenance, note-transcription duties, and compact handoff summaries across Role Mailbox, Work Packet System, Locus Work Tracking, Micro-Task Executor, Task Board, and Dev Command Center; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for durable handoffs; and materialized a dedicated mailbox handoff/transcription/announce-back stub. | ilja110320260813 |
  | v02.176 | 2026-03-11 | Orchestrator | **Role Mailbox executor-routing and claim-lease pass:** defined mailbox executor kinds, claim or lease modes, response-authority scope, claimant visibility, takeover policy, and lease-expiry posture across Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Work Packet System, and Task Board; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for claimant-aware parallel work; and materialized a dedicated mailbox executor-routing and claim-lease stub. | ilja110320260021 |
  | v02.175 | 2026-03-11 | Orchestrator | **Role Mailbox triage and queue-control pass:** defined mailbox triage queue state, reminder schedules, snooze and expiry posture, dead-letter remediation, and operator-facing remediation controls across Role Mailbox, Dev Command Center, Task Board, Work Packet System, and Locus Work Tracking; deepened Appendix 12 ownership, coverage, UI guidance, and interaction edges for queue aging and recovery; and materialized a dedicated mailbox-triage-and-queue-controls stub. | ilja110320260002 |
  ```

#### ANCHOR 3
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#WriteBoxV1
- CONTEXT_START_LINE: 65
- CONTEXT_END_LINE: 77
- CONTEXT_TOKEN: WriteBoxV1
- EXCERPT_ASCII_ESCAPED:
  ```text
| Version | Date | Author | Changes | Approval |
  |---------|------|--------|---------|----------|
  | v02.185 | 2026-05-14 | Kernel Builder | Added Kernel002 authority law: KernelActionCatalogV1, WriteBoxV1, direct-edit denial, advisory edit normalization, CRDT workspace draft persistence, and CRDT-to-EventLedger promotion bridge with DCC projection requirements. [ADD v02.185] | ilja140520260455 |
  | v02.184 | 2026-05-13 | Kernel Builder | Added Kernel V1 authority law: Postgres EventLedger as product runtime authority, SessionBroker/ContextBundle/ModelAdapter boundary, no-SQLite Kernel V1 authority/cache/offline/fallback/test permission, and projection-only Flight Recorder/DCC/diagnostics posture for kernel replay and promotion. [ADD v02.184] | Operator approval in chat, 2026-05-13; WP activation signature pending |
  | v02.183 | 2026-05-13 | Orchestrator | Migrated the active indexed Master Spec into the copy-first versioned bundle `.GOV/spec/master-spec-v02.183/`, moved the previous indexed bundle to `.GOV/spec/spec_archive/master-spec-v02.182/`, added uniform module `spec_version` metadata, added a manifest-declared machine-readable changelog module, updated `.GOV/spec/SPEC_CURRENT.md`, and refreshed internal references away from latest-monolith/version-file wording. [ADD v02.183] | Operator approval in chat, 2026-05-13 |
  | v02.182 | 2026-05-05 | Activation Manager | Added PostgreSQL-primary control-plane foundation law: explicit storage modes, PostgreSQL-authoritative self-hosted runtime records, fail-closed behavior when PostgreSQL is required, SQLite cache/offline boundaries, downstream split for queue workers, leases/backpressure, FEMS memory store, workflow durable execution, DCC projections, SQLite fallback boundaries, and developer/test container setup; updated Appendix 12 feature, primitive, and interaction metadata for the pivot. [ADD v02.182] | APPROVE REFINEMENT WP-1-Postgres-Primary-Control-Plane-Foundation-v1 |
  | v02.181 | 2026-04-17 | Orchestrator | Added software-delivery governance overlay law: product-owned runtime truth over imported repo `/.GOV/**`, validator-gate convergence on top of Governance Check Runner, projection-only Dev Command Center / Task Board / Role Mailbox posture, derived closeout semantics, overlay claim/lease and queued-instruction extension records, explicit overlay lifecycle constraints, and workflow-backed start/steer/cancel/close/recover control-plane law; updated Appendix 12 / roadmap follow-through for the affected feature families. [ADD v02.181] | pending |
  | v02.180 | 2026-04-07 | Orchestrator | Added 7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD) -- typed CheckResult contract (PASS/FAIL/BLOCKED/ADVISORY_ONLY/UNSUPPORTED), tool surface governance.check.run, FR events FR-EVT-GOV-CHECK-001..003, additive overlay rule. [ADD v02.180] | pending |
  | v02.179 | 2026-03-28 | Orchestrator | **Workflow-correlation bundle-scope pass:** patched Debug Bundle export law so `workflow_run` and `workflow_node_execution` become first-class bounded scopes, added workflow-node execution inventory plus manifest-count rules, extended exporter and exportable-inventory posture, deepened FEAT-DEBUG-BUNDLE UI guidance for workflow-scoped export, and kept roadmap/cov-matrix scheduling aligned with the existing Workflow Projection Correlation backlog. | ilja280320262308 |
  | v02.178 | 2026-03-11 | Orchestrator | **RAG mode and no-RAG cross-pillar pass:** clarified that RAG is one governed retrieval mode rather than the default context strategy; added retrieval-mode and non-hybrid-reason law across AI-Ready Data, ACE Runtime, Project Brain, Prompt-to-Spec Router, Loom, Work Packets, and Micro-Task Executor; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for authoritative direct-load, graph-first, and bounded local-model retrieval posture; and materialized a dedicated retrieval-mode policy stub. | ilja110320261228 |
  | v02.177 | 2026-03-11 | Orchestrator | **Role Mailbox handoff-bundle and announce-back provenance pass:** defined structured handoff bundles, announce-back provenance, note-transcription duties, and compact handoff summaries across Role Mailbox, Work Packet System, Locus Work Tracking, Micro-Task Executor, Task Board, and Dev Command Center; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for durable handoffs; and materialized a dedicated mailbox handoff/transcription/announce-back stub. | ilja110320260813 |
  | v02.176 | 2026-03-11 | Orchestrator | **Role Mailbox executor-routing and claim-lease pass:** defined mailbox executor kinds, claim or lease modes, response-authority scope, claimant visibility, takeover policy, and lease-expiry posture across Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Work Packet System, and Task Board; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for claimant-aware parallel work; and materialized a dedicated mailbox executor-routing and claim-lease stub. | ilja110320260021 |
  | v02.175 | 2026-03-11 | Orchestrator | **Role Mailbox triage and queue-control pass:** defined mailbox triage queue state, reminder schedules, snooze and expiry posture, dead-letter remediation, and operator-facing remediation controls across Role Mailbox, Dev Command Center, Task Board, Work Packet System, and Locus Work Tracking; deepened Appendix 12 ownership, coverage, UI guidance, and interaction edges for queue aging and recovery; and materialized a dedicated mailbox-triage-and-queue-controls stub. | ilja110320260002 |
  ```

#### ANCHOR 4
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/03-local-first-infrastructure.md#kernel-v1-crdt-workspace-addendum
- CONTEXT_START_LINE: 19876
- CONTEXT_END_LINE: 19888
- CONTEXT_TOKEN: Kernel V1 CRDT workspace addendum [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
Kernel V1 is excluded from the SQLite local-first recommendation: its first authority path is Postgres EventLedger only.
  
  Kernel V1 CRDT workspace addendum [ADD v02.185]: Kernel V1 may use CRDT updates, snapshots, and state vectors for draft workspace collaboration, but those records remain pre-promotion working state. PostgreSQL remains mandatory for EventLedger authority, promotion receipts, replay, and validation. CRDT storage MUST be restart-replayable, snapshot-safe, and joinable to write-box and promotion ids; it MUST NOT become a hidden SQLite authority path.
  
  
  ---
  
  ## 3.4 Conflict Resolution UX
  
  **Why**  
  Even with CRDTs, users sometimes need to understand what changed. Good conflict UX builds trust.
  
  **What**
  ```

#### ANCHOR 5
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md#10.11.5.28
- CONTEXT_START_LINE: 61701
- CONTEXT_END_LINE: 61713
- CONTEXT_TOKEN: Kernel Action Catalog and Write Box Projections [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
- Bulk handoff or announce-back actions MUST preview whether they only normalize mailbox summaries or also request governed note transcription or linked work mutation.
  
  ### 10.11.5.28 Kernel Action Catalog and Write Box Projections [ADD v02.185]
  
  The Dev Command Center MUST expose Kernel V1 action-catalog and write-box state as typed product projections, not as raw transcript or repo-governance mirrors.
  
  **Required projections**
  - Action catalog viewer: list KernelActionCatalogV1 entries by stable action id, target authority class, input schema version, actor eligibility, approval or capability requirements, preview behavior, and allowed output receipt types.
  - Write box queue: show draft write boxes by write_box_id, actor, CRDT site id, target refs, validation state, stale-state-vector posture, denial receipt, promotion receipt, and linked EventLedger events when promoted.
  - Direct-edit denial view: show attempted actor, target, action, denial reason, recovery instruction, and whether the blocked edit can be normalized into an advisory write box.
  - Promotion preview: before promotion, show affected target refs, current state vector, validation checks, idempotency key, expected EventLedger event types, and stale or duplicate risk.
  - Projection freshness badges: distinguish live CRDT draft state, compacted snapshot state, pending promotion, accepted promotion, rejected promotion, and stale projection.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#FEAT-KERNEL-WORKSPACE-WRITE-BOX
- CONTEXT_START_LINE: 73924
- CONTEXT_END_LINE: 73936
- CONTEXT_TOKEN: FEAT-KERNEL-WORKSPACE-WRITE-BOX
- EXCERPT_ASCII_ESCAPED:
  ```text
},
      {
        "feature_id": "FEAT-KERNEL-WORKSPACE-WRITE-BOX",
        "title": "Kernel Workspace Write Box and Promotion Bridge",
        "spec_anchor": "#231310-kernel-v1-crdt-workspace-write-box-and-promotion-bridge-add-v02185",
        "surfaces": [
          "backend",
          "ui"
        ],
        "primitives": [
          "PRIM-KernelActionCatalogV1",
          "PRIM-KernelActionDescriptorV1",
          "PRIM-WriteBoxV1",
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: MT-001 Fold Preservation Manifest and Source Import: materialize the complete folded-source manifest in the official packet/refinement. Acceptance: every listed source stub has path, pre-fold hash, direct/transitive fold classification, and source-scope import instructions. Activation cannot proceed if any source file is missing or hash mismatch is unexplained. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-001 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-002 Reset Invariant Reconciliation: reconcile folded legacy assumptions with reset invariants. Acceptance: every source obligation that mentions SQLite, Markdown authority, mailbox chronology, or UI-local truth is explicitly converted to Postgres authority, projection/advisory status, or promotion-gated action semantics. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-002 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-003 CRDT Library and Storage ADR: compare Yjs, Loro, Automerge, and existing product dependencies against Handshake runtime needs. Acceptance: ADR selects a CRDT approach, rejected options, sync/storage model, Rust/TypeScript integration boundary, schema compatibility, and validation plan. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-003 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-004 Kernel Action Envelope: define `KernelActionRequestV1`, `KernelActionResultV1`, `KernelActionDenialV1`, and receipt/event mappings. Acceptance: action requests carry actor/session/profile, target ids, input schema id, expected write boxes, authority effect, approval posture, validation requirements, and trace id. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-004 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-005 Action Catalog Registry: implement the durable `KernelActionCatalogV1` registry. Acceptance: every model-facing action has stable id, schemas, role eligibility, capability requirements, write boxes, promotion path, validation hooks, and DCC preview metadata. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-005 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-006 Write Box Schema Family: define `DraftBox`, `CRDTWorkspaceBox`, `ProposalBox`, `PatchBox`, `ArtifactBox`, `MirrorAdvisoryBox`, `MemoryBox`, `ExecutionBox`, and `PromotionBox`. Acceptance: each write box has lifecycle state, owner, allowed transitions, authority effect, evidence refs, validation status, and projection rules. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-006 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-007 Direct Edit Denial Path: route model/tool attempts to mutate authority artifacts through ToolGate denial or proposal wrapping. Acceptance: tests prove raw authority-file edit attempts fail with actionable denial evidence and lawful replacement action ids. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-007 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-008 Advisory Edit Normalization: convert manual/model edits against generated mirrors into `MirrorAdvisoryBox` records. Acceptance: advisory edits do not mutate authority until a registered normalization/promotion action validates and accepts them. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-008 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-009 No-Context Model Manual: create durable model-facing instructions for using Handshake mechanically. Acceptance: the manual explains purpose, startup, action catalog, write boxes, DCC paths, CRDT workflow, safety constraints, failure modes, denial recovery, and validation evidence for a model with no conversation history. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-009 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-010 CRDT Document Identity and Workspace Model: define document/workspace ids, actor ids, site/client ids, schema ids, and authority links. Acceptance: CRDT records can be linked to work item, action request, artifact proposal, Role Mailbox thread, DCC projection, and EventLedger ids. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-010 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-011 CRDT Update Persistence: persist CRDT updates in Postgres with ordering, hash, actor/session attribution, and replay metadata. Acceptance: a workspace can be reconstructed from persisted updates after restart without file-system authority assumptions. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-011 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-012 CRDT Snapshot and Compaction: add snapshot/state-vector or equivalent sync cursor support. Acceptance: update replay is bounded by snapshots, old updates remain auditable or compacted according to policy, and compaction never drops promotion evidence. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-012 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-013 CRDT Context Slicing for Models: expose summaries, selected ranges, field digests, and operation deltas. Acceptance: model prompts can request bounded CRDT context without loading entire documents, and extract outputs cite workspace/version/source ids. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-013 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-014 CRDT Schema and Validity Guard: validate CRDT materialized state before promotion. Acceptance: structurally invalid, unauthorized, or schema-drifted CRDT state cannot be promoted into authority. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-014 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-015 CRDT Promotion Bridge: convert CRDT edits/drafts into ArtifactProposal and PromotionGate inputs. Acceptance: accepted promotions emit EventLedger authority events; rejected promotions keep CRDT/draft state as non-authoritative evidence. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-015 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-016 Conflict and Presence Projection: expose presence, pending conflicts, actor attribution, and merge/proposal state. Acceptance: DCC can show who changed what, which edits are merely merged CRDT state, and which changes are pending promotion. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-016 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-017 Software-Delivery Runtime Truth Records: fold `WP-1-Software-Delivery-Runtime-Truth-v1`. Acceptance: current software-delivery posture is queryable from product-owned stable records and governed actions, not packet prose, mailbox order, or Markdown freshness. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-017 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-018 Workflow Transition Automation Registry: fold `WP-1-Workflow-Transition-Automation-Registry-v1`. Acceptance: every workflow mutation has a registered transition rule, eligible actor, action trigger, approval boundary, and DCC preview. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-018 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-019 Governance Overlay Boundary: fold `WP-1-Software-Delivery-Governance-Overlay-Boundary-v1`. Acceptance: imported repo `.GOV/**` artifacts are evidence/source overlays, not runtime truth, and import/export cannot bypass gates. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-019 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-020 Overlay Coordination Records: fold `WP-1-Software-Delivery-Overlay-Coordination-Records-v1`. Acceptance: claim/lease, queued steering, follow-up, takeover, and actor eligibility are queryable by stable ids without mailbox chronology. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-020 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-021 Overlay Lifecycle and Recovery Control Plane: fold `WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1`. Acceptance: start, steer, cancel, close, recover, checkpoint replay, partial failure, and restart posture are record-backed and projection-safe. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-021 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-022 Postgres Control-Plane Residual Scope: fold `WP-1-Postgres-Control-Plane-Shift-Bundle-v1` plus its transitive folded stubs. Acceptance: residual live Postgres service proof, leases/backpressure, ModelSession queues, FEMS memory store, durable workflow execution, DCC projections, and SQLite boundary obligations are carried into Kernel002 or explicitly mapped to Kernel003/Kernel004 without reopening the old bundle. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-022 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-023 Locus Work Tracking Reset Migration: fold `WP-1-Locus-Work-Tracking-System-Phase1-v1`. Acceptance: WP/MT tracking, dependencies, occupancy, query, Task Board projection, and Flight Recorder obligations are preserved, but SQLite authority is replaced with Postgres/EventLedger/CRDT-compatible authority. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-023 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-024 DCC MVP Runtime Surface: fold `WP-1-Dev-Command-Center-MVP-v1`. Acceptance: DCC can select work, view worktree/session/action/proposal state, inspect diffs/evidence, preview approvals, and trigger governed actions through the catalog. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-024 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-025 DCC Structured Artifact Viewer: fold `WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1`. Acceptance: DCC renders canonical fields before mirrors, exposes mirror state, and provides raw structured drilldown as advanced view. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-025 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-026 DCC Layout Projection Registry: fold `WP-1-Dev-Command-Center-Layout-Projection-Registry-v1`. Acceptance: board, queue, list, roadmap, inbox-triage, and execution-queue views derive from registered presets and action bindings. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-026 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-027 Role Mailbox Message and Action Request Contract: fold `WP-1-Role-Mailbox-Message-Thread-Contract-v1`. Acceptance: mailbox lifecycle, delivery state, allowed responses, due/dead-letter posture, and action requests are typed and authority-bounded. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-027 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-028 Role Mailbox Micro-Task Loop Control: fold `WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1`. Acceptance: retry budget, verifier outcome, escalation, completion report, dead-letter, and loop checkpoint state are compact and replayable. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-028 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-029 Role Mailbox Triage Queue Controls: fold `WP-1-Role-Mailbox-Triage-Queue-Controls-v1`. Acceptance: reminder, snooze, expiry, dead-letter, retry/reroute/archive, and Task Board pressure overlays are field-backed projections. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-029 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-030 Role Mailbox Claim and Lease: fold `WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1`. Acceptance: claimant, claim mode, lease age/expiry, takeover legality, and responder eligibility are explicit and queryable. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-030 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-031 Role Mailbox Handoff and Announce-Back: fold `WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1`. Acceptance: handoff bundles, transcription targets, recommended next actor, announce-back provenance, and advisory/completion distinction are typed. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-031 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-032 Role Mailbox Inbox Alignment and Evidence Bridge: fold `WP-1-Inbox-Role-Mailbox-Alignment-v1` and `WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1`. Acceptance: Inbox labels map to Role Mailbox only, mailbox telemetry is leak-safe, and debug bundle exports preserve stable evidence/provenance. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-032 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-033 FEMS Working-Memory Checkpoints: fold `WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1`. Acceptance: SESSION_OPEN, PRE_TASK, INSIGHT, TASK_COMPLETE, SESSION_CLOSE, memory extract, repeated insight promotion, and GC are typed and quality-gated. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-033 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-034 FEMS Write-Time Safeguards: fold `WP-1-FEMS-Write-Time-Safeguards-v1`. Acceptance: novelty scoring, supersession, contradiction detection, dedup, state validation, and audit trail run mechanically; SQLite/FTS5 references are reworked to reset-approved storage/search primitives. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-034 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-035 FEMS Memory Poisoning and Drift Guardrails: fold `WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1`. Acceptance: trust gates, pack budget, deterministic reduction, proposal/approval/denial events, and effective pack hashes prevent untrusted long-lived memory drift. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-035 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-036 FEMS MT Handoff Memory Context: fold `WP-1-FEMS-MT-Handoff-Memory-Context-v1`. Acceptance: escalated or handed-off MTs carry typed memory context with source/target sessions, failed attempts, recommended items, provenance, and bounded scoring. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-036 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-037 Role Turn Isolation: fold `WP-1-Role-Turn-Isolation-v1`. Acceptance: role turns default to isolated context, replay pins are recorded, and cross-role bleed is mechanically prevented. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-037 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-038 Work Profiles: fold `WP-1-Work-Profiles-v1`. Acceptance: profile storage, selection, immutable profile ids, per-role routing, autonomy knobs, and profile receipts are wired into action requests. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-038 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-039 Local-First Agentic MCP Posture: fold `WP-1-LocalFirst-Agentic-MCP-Posture-v1`. Acceptance: local-first execution remains default; MCP/cloud paths are capability-gated adapters with cached artifacts and fallback behavior. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-039 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-040 Git Engine Decision Gate: fold `WP-1-Git-Engine-Decision-Gate-v1`. Acceptance: one repo engine path is recorded/enforced, dangerous git actions remain gated, and DCC/action catalog expose only lawful git affordances. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-040 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-041 Session Anti-Pattern Registry: fold `WP-1-Session-Anti-Pattern-Registry-v1`. Acceptance: scheduler/trust/capability/session orchestration anti-patterns have machine-readable detections and deny/downgrade/consent/stop outcomes. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-041 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-042 Governance Pack Instantiation: fold `WP-1-Governance-Pack-v1`. Acceptance: project identity, pack manifest, instantiation, naming/path policy, conformance harness, and imported-overlay boundaries are compatible with Kernel002 action/write-box law. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-042 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-043 Session Spawn Tree DCC Visualization: fold `WP-1-Session-Spawn-Tree-DCC-Visualization-v1`. Acceptance: DCC shows spawn hierarchy, child counts, depth, cascade cancel, spawn mode, and announce-back badges from runtime records. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-043 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-044 Session Spawn Conversation Distillation: fold `WP-1-Session-Spawn-Conversation-Distillation-v1`. Acceptance: parent-child request/summary pairs and spawn metadata feed distillation artifacts without making conversation text authority. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-044 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-045 Product Screenshot Capture: fold `WP-1-Product-Screenshot-Visual-Validation-v1`. Acceptance: governed sessions can capture full app, panel, and module screenshots with metadata and artifact refs. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-045 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-046 Visual Debugging Loop: fold `WP-1-Visual-Debugging-Loop-v1`. Acceptance: post-commit or post-action screenshot capture, baseline comparison, visual evidence storage, threshold config, and validator steering are available for GUI work. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-046 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-047 Markdown Mirror Sync Drift Guard: fold `WP-1-Markdown-Mirror-Sync-Drift-Guard-v1`. Acceptance: deterministic mirror regeneration, drift states, manual advisory handling, reconciliation, DCC mirror queue, and projection banners are implemented. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-047 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-048 Direct-Edit Regression Harness: prove future models cannot bypass write boxes through common edit paths. Acceptance: tests simulate model raw patch, generated file write, mirror edit, CRDT edit, mailbox reply, DCC quick action, and git action; each path either uses registered action/write box or fails with evidence. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-048 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-049 Projection Rebuild and Task Board Sync: regenerate projections and sync Task Board, traceability registry, build order, and stub contracts. Acceptance: `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check` pass or produce a concrete blocker. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-049 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-050 Pre-Use Kernel Acceptance Run: prove Kernel001 + Kernel002 are usable before real kernel operation. Acceptance: a no-context model follows the manual to draft in CRDT, submit a proposal, trigger validation, receive a promotion/denial, view DCC projections, and inspect evidence without direct authority-file edits. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-050 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-051 Stub, Work Packet, and Microtask Contract Lifecycle: define the machine-readable lifecycle from inactive stub to active work packet to generated microtask contracts. Acceptance: `StubContractV1`, `WorkPacketContractV1`, and `MicroTaskContractV1` schemas define states, authority rules, required fields, provenance hashes, source imports, lifecycle transitions, receipt events, projection hooks, validation hooks, and failure states. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-051 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-052 Work Packet Full-Detail Authority and Microtask Source Plan: ensure the activated work packet itself carries full implementation detail while also containing a structured MT source plan. Acceptance: a no-context strong model can execute from the work packet alone; the same packet can regenerate MT contracts/files without relying on manually maintained sidecars or hidden chat context. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-052 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-053 Mechanical Stub Promotion and Microtask Extraction: implement deterministic commands or action-catalog entries for stub-to-WP promotion and WP-to-MT extraction. Acceptance: promotion/extraction preserves operator intent, source hashes, folded details, dependencies, constraints, acceptance criteria, verification, and status provenance; every generated artifact records its source contract id and hash. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-053 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-054 Local-Model Fresh-Context Microtask Loop Contract: define the Locus-compatible execution loop for smaller/local models working one MT at a time. Acceptance: the loop contract covers fresh-context input bundle, allowed actions, write boxes, retry budget, verifier handoff, failure requeue, memory checkpoint input, receipt emission, and final MT outcome without requiring the model to inspect unrelated WP scope. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-054 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-055 Generated Documentation and Status Projection: replace manual status/docs maintenance with projections from contracts, receipts, runtime state, and validation outputs. Acceptance: packet status, MT status, Task Board rows, traceability rows, DCC work views, mirror docs, and operator summaries regenerate from machine-readable authority; direct manual status edits are denied or captured as advisory normalization input. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-055 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-056 Coder Handoff and Validation Request Contract: define the structured handoff from coder execution to Handshake-owned validation. Acceptance: `CoderHandoffContractV1` records MT id, parent WP id, actor/session, claimed scope, touched files/actions, receipts, tests, evidence, known blockers, and requested review; Handshake can generate a validator review request from it without a model editing status fields. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-056 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-057 Validator Verdict and Mediation Contract: define structured pass/fail/mediation verdicts from Integration Validator batch review for this packet. Acceptance: `ValidatorVerdictContractV1` and `MediationInstructionContractV1` encode verdict, failed acceptance criteria, evidence refs, severity, reproducibility, exact remediation instructions, dependency impact, and whether the MT may advance, must loop back, or must escalate. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-057 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-058 Mechanical Issue, Bug, Gap, and Out-of-Scope Reports: define machine-readable reports for validator findings that are not simple pass/fail. Acceptance: `IssueReportContractV1`, `BugReportContractV1`, `GapReportContractV1`, and `OutOfScopeReportContractV1` preserve validator reasoning, source refs, affected surfaces, reproduction or proof, proposed destination, and routing outcome without becoming manual prose-only reports. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-058 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-059 Remediation Microtask and Packet Generation: generate follow-up work from failed verdicts and reports. Acceptance: Handshake can create `RemediationMicroTaskContractV1` or a remediation packet/stub from verdict/report contracts, preserving parent WP/MT links, dependency state, acceptance criteria, allowed actions, write boxes, evidence refs, retry budget, and validator recheck requirements. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-059 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-060 Loop Scheduler and Next-Coder Dispatch: define the mechanical loop that dispatches coders after validation outcomes. Acceptance: Handshake only dispatches a new coder when leases, current coder completion, dependency state, retry budget, and verdict state allow it; failed prerequisites loop to remediation before dependent MTs can advance. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-060 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-061 Locus Work Graph Projection for MT Validation Loops: connect the validation/remediation loop to Locus work tracking semantics from the Master Spec. Acceptance: Locus can project MT nodes, validator verdicts, remediation edges, blocked/escalated states, actor leases, and pass/fail history without treating prose reports or chat messages as truth. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-061 scope is lost and Kernel002 can reintroduce drift or direct mutation.
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: KernelActionCatalogV1 | PRODUCER: kernel action registry | CONSUMER: ToolGate, DCC, model manual, Integration Validator | SERIALIZER_TRANSPORT: JSON schema over product authority records | VALIDATOR_READER: catalog tests and projection checks | TRIPWIRE_TESTS: each action has ids, schemas, actors, write boxes, hooks, receipts | DRIFT_RISK: undocumented action path bypasses law
  - CONTRACT: WriteBoxV1 family | PRODUCER: CRDT, advisory, patch, artifact, memory, execution, promotion surfaces | CONSUMER: PromotionGate, validators, DCC, Locus | SERIALIZER_TRANSPORT: typed JSON plus artifact hashes | VALIDATOR_READER: write-box transition tests | TRIPWIRE_TESTS: every box records state, owner, authority effect, evidence, validation, projection | DRIFT_RISK: draft/advisory state becomes authority
  - CONTRACT: CRDT workspace records | PRODUCER: CRDT storage layer | CONSUMER: context slicer, promotion bridge, DCC, replay tests | SERIALIZER_TRANSPORT: Postgres update stream and snapshots | VALIDATOR_READER: restart replay and compaction tests | TRIPWIRE_TESTS: reconstruct workspace after restart without SQLite/file authority | DRIFT_RISK: CRDT state is non-replayable
  - CONTRACT: WorkPacketContractV1 and MicroTaskContractV1 | PRODUCER: work graph generator | CONSUMER: Locus, DCC, packet projections, MT loop | SERIALIZER_TRANSPORT: JSON contracts with generated Markdown projections | VALIDATOR_READER: contract import and projection drift checks | TRIPWIRE_TESTS: 61 MT contracts regenerate one-to-one | DRIFT_RISK: packet detail collapses into stale prose
  - CONTRACT: CoderHandoffContractV1 and ValidatorVerdictContractV1 | PRODUCER: Kernel Builder and Integration Validator | CONSUMER: loop scheduler, Locus, DCC | SERIALIZER_TRANSPORT: JSON receipts and review records | VALIDATOR_READER: handoff/verdict tests | TRIPWIRE_TESTS: failed MT creates remediation or block before dependency advances | DRIFT_RISK: validator prose manually drives status
  - CONTRACT: DCC projection and visual evidence payloads | PRODUCER: projection registry and visual debug loop | CONSUMER: operator, models, Integration Validator | SERIALIZER_TRANSPORT: structured projection JSON plus screenshot refs | VALIDATOR_READER: visual-debug checks | TRIPWIRE_TESTS: rows expose stable ids, stale badges, and promotion previews | DRIFT_RISK: UI hides authority or stale state
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Implement MT-001 through MT-061 back to back in declared order unless dependency proof requires local reordering without dropping any MT.
- HOT_FILES:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/role_mailbox/**
  - src/backend/handshake_core/src/fems/**
  - src/backend/handshake_core/src/locus/**
  - src/backend/handshake_core/tests/**
  - app/**
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_write_box --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
- CARRY_FORWARD_WARNINGS:
  - Do not condense or remove any MT.
  - Do not introduce WP Validator gate.
  - Do not treat Kernel Builder checks as validation.
  - Do not use SQLite authority or prose/mirror authority.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - All 61 MT clauses and contracts.
  - CRDT is non-authority until promotion.
  - Action catalog/write boxes deny or normalize direct edits.
  - No WP Validator gate; Integration Validator batch/spec review is separate.
- FILES_TO_READ:
  - .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json
  - .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-*.json
  - .GOV/spec/SPEC_CURRENT.md
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/tests/**
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
  - just spec-eof-appendices-check
- POST_MERGE_SPOTCHECKS:
  - No-context manual path, direct-edit denial harness, CRDT promotion/restart proof, and visual DCC evidence.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Product implementation has not started.
  - MT-003 ADR has not selected CRDT library.
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
  - Spec basis already selects action catalog/write boxes over direct edits and EventLedger/PromotionGate over CRDT convergence.
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
  - PRIM-CrdtWorkspaceDraftV1
  - PRIM-CrdtWorkspaceSnapshotV1
- PRIMITIVES_EXPOSED:
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-WriteBoxDirectEditDeniedV1
  - PRIM-WriteBoxPromotionRequestV1
  - PRIM-WriteBoxPromotionReceiptV1
  - PRIM-CrdtWorkspaceDraftV1
  - PRIM-CrdtWorkspaceSnapshotV1
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
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
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - Catalog denial evidence -> IN_THIS_WP (stub: NONE)
  - Locus remediation graph -> IN_THIS_WP (stub: NONE)
  - Packet regeneration -> IN_THIS_WP (stub: NONE)
  - Board projection freshness -> IN_THIS_WP (stub: NONE)
  - Fresh-context MTs -> IN_THIS_WP (stub: NONE)
  - DCC write-box queue -> IN_THIS_WP (stub: NONE)
  - Runtime transition law -> IN_THIS_WP (stub: NONE)
  - Spec prompt slices -> IN_THIS_WP (stub: NONE)
  - Postgres CRDT replay -> IN_THIS_WP (stub: NONE)
  - LLM structured writes -> IN_THIS_WP (stub: NONE)
  - Sandbox-ready hooks -> IN_THIS_WP (stub: NONE)
  - Guide manual to catalog -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: Action and denial evidence | SUBFEATURES: action, denial, promotion, screenshot, validation, and replay receipts | PRIMITIVES_FEATURES: PRIM-WriteBoxDirectEditDeniedV1 | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: Locus | CAPABILITY_SLICE: MT validation loop graph | SUBFEATURES: MT nodes, remediation edges, leases, blocked/escalated states | PRIMITIVES_FEATURES: PRIM-WriteBoxPromotionReceiptV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Contract-first work authority | SUBFEATURES: StubContractV1, WorkPacketContractV1, MicroTaskContractV1 | PRIMITIVES_FEATURES: FEAT-KERNEL-WORKSPACE-WRITE-BOX | MECHANICAL: engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Projection sync | SUBFEATURES: status projection, drift queue, advisory mirror normalization | PRIMITIVES_FEATURES: PRIM-WriteBoxDirectEditDeniedV1 | MECHANICAL: engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: MicroTask | CAPABILITY_SLICE: Fresh-context MT loop | SUBFEATURES: one-MT bundle, retry budget, handoff, verdict, remediation | PRIMITIVES_FEATURES: PRIM-WriteBoxV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: Command Center | CAPABILITY_SLICE: Kernel workbench projections | SUBFEATURES: catalog viewer, write-box queue, artifacts, layouts, visual debug | PRIMITIVES_FEATURES: PRIM-KernelActionCatalogV1 | MECHANICAL: engine.guide | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Registered transitions | SUBFEATURES: action envelope, transition registry, queues, leases, recovery | PRIMITIVES_FEATURES: PRIM-KernelActionDescriptorV1 | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: Spec to prompt | CAPABILITY_SLICE: Bounded model context | SUBFEATURES: manual, CRDT slices, anchors, prompt-safe extracts | PRIMITIVES_FEATURES: PRIM-CrdtWorkspaceDraftV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Postgres authority reset | SUBFEATURES: CRDT updates, snapshots, no SQLite authority | PRIMITIVES_FEATURES: PRIM-CrdtWorkspaceSnapshotV1 | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Typed model-operable surfaces | SUBFEATURES: catalog schemas, write boxes, CRDT slices, mailbox state, FEMS checkpoints | PRIMITIVES_FEATURES: PRIM-WriteBoxV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Action catalog and denial | JobModel: MECHANICAL_TOOL | Workflow: registered action schemas and denial/proposal receipts | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: action_request/action_denial | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
  - Capability: CRDT workspace promotion | JobModel: WORKFLOW | Workflow: CRDT updates promote through ArtifactProposal and PromotionGate | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: crdt_update/promotion_receipt | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
  - Capability: MT remediation loop | JobModel: WORKFLOW | Workflow: handoff and Integration Validator verdicts create loop records | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: handoff/verdict/remediation | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
  - Capability: DCC visual evidence | JobModel: UI_ACTION | Workflow: catalog/write-box/screenshot/stale-state projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: screenshot/projection_rebuilt | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1 -> EXPAND_IN_THIS_WP
- CODE_REALITY_SUMMARY:
  - NONE
## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: YES
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PASS
- GUI_REFERENCE_DECISIONS:
  - Dev Command Center kernel write box projection <- NONE (IN_THIS_WP)
- HANDSHAKE_GUI_ADVICE:
  - Surface: DCC action catalog | Control: Preview action | Type: icon button | Why: inspect authority effect before mutation | Microcopy: Preview action | Tooltip: Show schemas and promotion path.
- HIDDEN_GUI_REQUIREMENTS:
  - Denial, stale projection, and promotion-blocked states remain visible.
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - Key rows by action_id, write_box_id, target_authority_id, projection_hash, and evidence_ref.
## SCOPE
- What: Activate Kernel002 as the CRDT workspace, action catalog, write-box, direct-edit denial, DCC projection, Role Mailbox, FEMS, Locus, generated-doc, and MT validation-loop hardening packet with all 61 preserved MTs.
- Why: Kernel001 supplies authority substrate, but safe no-context model use needs registered actions/write boxes and CRDT drafts promoted through EventLedger gates.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/role_mailbox/**
  - src/backend/handshake_core/src/fems/**
  - src/backend/handshake_core/src/locus/**
  - src/backend/handshake_core/tests/**
  - app/**
  - tests/**
  - README.md
- OUT_OF_SCOPE:
  - No product implementation in this activation session.
  - No WP Validator gate/session.
  - No Integration Validator launch/verdict/merge/pass-fail claim in this activation session.
  - No condensing or reducing the 61 MTs.
  - No SQLite authority or fallback store.
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
  cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  just spec-eof-appendices-check
```

### DONE_MEANS
- Active packet/refinement and exactly 61 MT contracts/projections exist.
- Action catalog and WriteBoxV1 family prevent or normalize direct edits.
- CRDT updates/snapshots/promotions persist and remain non-authoritative until EventLedger acceptance.
- DCC, Role Mailbox, FEMS, Locus, docs/status, visual-debug, and MT loops consume typed authority.
- No WP Validator gate exists; Integration Validator batch review remains separate.

- PRIMITIVES_EXPOSED:
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-WriteBoxDirectEditDeniedV1
  - PRIM-WriteBoxPromotionRequestV1
  - PRIM-WriteBoxPromotionReceiptV1
  - PRIM-CrdtWorkspaceDraftV1
  - PRIM-CrdtWorkspaceSnapshotV1
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: .GOV/spec/master-spec-v02.185/indexed-spec-manifest.json (recorded_at: 2026-05-14T04:54:49.177Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.185]
- SPEC_ANCHOR_PRIMARY: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.10
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
  - .GOV/task_packets/stubs/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1.md
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/tests/**
- SEARCH_TERMS:
  - KernelActionCatalogV1
  - WriteBoxV1
  - WriteBoxDirectEditDeniedV1
  - CRDT workspace
  - PromotionGate
  - EventLedger
  - Role Mailbox
  - FEMS
  - Locus
  - visual debugging
  - MicroTaskContractV1
- RUN_COMMANDS:
  ```bash
just pre-work WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
  cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  ```
- RISK_MAP:
  - "Direct edit bypass" -> "authority mutation outside action law"
  - "CRDT equals authority" -> "invalid promotions"
  - "Projection drift" -> "stale operator/model state"
  - "MT condensation" -> "lost folded obligations"
## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- For `PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1`, this section is copied from the signed refinement and should not drift.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - DCC action catalog viewer.
  - DCC write-box queue.
  - Direct-edit denial/recovery panel.
  - CRDT promotion preview and stale projection badges.
  - Visual debugging evidence viewer.
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: action preview | Type: icon button | Tooltip: Show schemas, write boxes, validation hooks, and promotion path | Notes: disabled until actor/action eligibility resolves.
  - Control: write-box filter | Type: segmented control | Tooltip: Filter boxes by lifecycle and authority effect | Notes: stable row dimensions.
  - Control: denial recovery | Type: action button | Tooltip: Convert denied edit into advisory proposal | Notes: denial evidence remains visible.
- UI_STATES (empty/loading/error):
  - Empty catalog, loading queue, denied edit, stale projection, and promotion blocked states.
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Use Action, Write box, Authority effect, Promotion preview, Denied edit, Stale projection, and Evidence.
- UI_ACCESSIBILITY_NOTES:
  - Tooltips work on hover and keyboard focus and do not obscure queue rows.
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
  - LOG_PATH: `.handshake/logs/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/<name>.log` (recommended; not committed)
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
