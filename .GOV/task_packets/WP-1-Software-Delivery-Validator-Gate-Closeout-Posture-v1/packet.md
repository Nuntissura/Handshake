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

# Task Packet: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1

## METADATA
- TASK_ID: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- WP_ID: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- BASE_WP_ID: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture
- DATE: 2026-05-04T00:01:00.947Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just phase-check HANDOFF ... CODER --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: ACTIVATION_MANAGER
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: claude-opus-4-7
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-closeout-posture-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.5
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
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
- **Status:** Ready for Dev
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Check-Runner, WP-1-Governance-Workflow-Mirror, WP-1-Workflow-Projection-Correlation, WP-1-Dev-Command-Center-Control-Plane-Backend
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: YES
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- LOCAL_WORKTREE_DIR: ../wtc-closeout-posture-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja040520260128
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: v02.181 Validator-gate and closeout posture | CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs, locus/task_board.rs, role_mailbox.rs | TESTS: validator_gate_runtime_summary_links_check_evidence; closeout_posture_requires_gate_evidence_owner_and_action_truth | EXAMPLES: Example validator-gate summary for one work_packet_id showing gate_record_id, gate phase, CheckResult status, descriptor provenance, evidence refs, role/session proof, and current blockers., Example CheckResult PASS that remains insufficient for closeout until canonical gate materialization, evidence completeness, ownership/claim posture, and governed-action resolution are true., Example closeout posture row distinguishing not_ready, ready_for_validation, validator_cleared, integration_blocked, closeout_pending, and closeout_complete., Example Task Board row and Role Mailbox thread with stale/advisory closeout text while runtime unresolved-gate or missing-evidence blockers win., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, gate_record_id, and legal recover/close actions. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Governance Check Runner validator-gate convergence | CODE_SURFACES: runtime_governance.rs, workflows.rs, locus/types.rs | TESTS: check_result_pass_does_not_close_work_without_gate_materialization | EXAMPLES: Example validator-gate summary for one work_packet_id showing gate_record_id, gate phase, CheckResult status, descriptor provenance, evidence refs, role/session proof, and current blockers., Example CheckResult PASS that remains insufficient for closeout until canonical gate materialization, evidence completeness, ownership/claim posture, and governed-action resolution are true., Example closeout posture row distinguishing not_ready, ready_for_validation, validator_cleared, integration_blocked, closeout_pending, and closeout_complete., Example Task Board row and Role Mailbox thread with stale/advisory closeout text while runtime unresolved-gate or missing-evidence blockers win., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, gate_record_id, and legal recover/close actions. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Software-delivery closeout derivation | CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs | TESTS: closeout_posture_requires_gate_evidence_owner_and_action_truth | EXAMPLES: Example validator-gate summary for one work_packet_id showing gate_record_id, gate phase, CheckResult status, descriptor provenance, evidence refs, role/session proof, and current blockers., Example CheckResult PASS that remains insufficient for closeout until canonical gate materialization, evidence completeness, ownership/claim posture, and governed-action resolution are true., Example closeout posture row distinguishing not_ready, ready_for_validation, validator_cleared, integration_blocked, closeout_pending, and closeout_complete., Example Task Board row and Role Mailbox thread with stale/advisory closeout text while runtime unresolved-gate or missing-evidence blockers win., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, gate_record_id, and legal recover/close actions. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Software-delivery overlay lifecycle semantics | CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs | TESTS: validator_gate_final_pass_requires_committable_gate_and_authority_proof | EXAMPLES: Example validator-gate summary for one work_packet_id showing gate_record_id, gate phase, CheckResult status, descriptor provenance, evidence refs, role/session proof, and current blockers., Example CheckResult PASS that remains insufficient for closeout until canonical gate materialization, evidence completeness, ownership/claim posture, and governed-action resolution are true., Example closeout posture row distinguishing not_ready, ready_for_validation, validator_cleared, integration_blocked, closeout_pending, and closeout_complete., Example Task Board row and Role Mailbox thread with stale/advisory closeout text while runtime unresolved-gate or missing-evidence blockers win., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, gate_record_id, and legal recover/close actions. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Projection-only DCC/Task Board/Role Mailbox posture | CODE_SURFACES: locus/task_board.rs, role_mailbox.rs, workflows.rs, runtime_governance.rs | TESTS: task_board_and_mailbox_closeout_badges_remain_projection_only | EXAMPLES: Example validator-gate summary for one work_packet_id showing gate_record_id, gate phase, CheckResult status, descriptor provenance, evidence refs, role/session proof, and current blockers., Example CheckResult PASS that remains insufficient for closeout until canonical gate materialization, evidence completeness, ownership/claim posture, and governed-action resolution are true., Example closeout posture row distinguishing not_ready, ready_for_validation, validator_cleared, integration_blocked, closeout_pending, and closeout_complete., Example Task Board row and Role Mailbox thread with stale/advisory closeout text while runtime unresolved-gate or missing-evidence blockers win., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, gate_record_id, and legal recover/close actions. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## PACKET_ACCEPTANCE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the executable acceptance contract for packet closure. New packets must keep stable row IDs and move each required row to PROVED, CONFIRMED, or NOT_APPLICABLE with evidence before PASS.
- Rule: use STEER or BLOCKED for unresolved required rows instead of narrative closure.
- ACCEPTANCE_ROWS:
  - ID: AC-001 | REQUIREMENT: v02.181 Validator-gate and closeout posture | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: validator_gate_runtime_summary_links_check_evidence; closeout_posture_requires_gate_evidence_owner_and_action_truth | REASON: NONE
  - ID: AC-002 | REQUIREMENT: Governance Check Runner validator-gate convergence | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: check_result_pass_does_not_close_work_without_gate_materialization | REASON: NONE
  - ID: AC-003 | REQUIREMENT: Software-delivery closeout derivation | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: closeout_posture_requires_gate_evidence_owner_and_action_truth | REASON: NONE
  - ID: AC-004 | REQUIREMENT: Software-delivery overlay lifecycle semantics | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: validator_gate_final_pass_requires_committable_gate_and_authority_proof | REASON: NONE
  - ID: AC-005 | REQUIREMENT: Projection-only DCC/Task Board/Role Mailbox posture | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: task_board_and_mailbox_closeout_badges_remain_projection_only | REASON: NONE
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - `validator_gate_runtime_summary_links_check_evidence`
  - `check_result_pass_does_not_close_work_without_gate_materialization`
  - `closeout_posture_requires_gate_evidence_owner_and_action_truth`
  - `task_board_and_mailbox_closeout_badges_remain_projection_only`
  - `validator_gate_final_pass_requires_committable_gate_and_authority_proof`
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml validator_gate_runtime_summary_links_check_evidence -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml check_result_pass_does_not_close_work_without_gate_materialization -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_posture_requires_gate_evidence_owner_and_action_truth -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_and_mailbox_closeout_badges_remain_projection_only -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml validator_gate_final_pass_requires_committable_gate_and_authority_proof -- --exact`
- CANONICAL_CONTRACT_EXAMPLES:
  - Example validator-gate summary for one work_packet_id showing gate_record_id, gate phase, CheckResult status, descriptor provenance, evidence refs, role/session proof, and current blockers.
  - Example CheckResult PASS that remains insufficient for closeout until canonical gate materialization, evidence completeness, ownership/claim posture, and governed-action resolution are true.
  - Example closeout posture row distinguishing not_ready, ready_for_validation, validator_cleared, integration_blocked, closeout_pending, and closeout_complete.
  - Example Task Board row and Role Mailbox thread with stale/advisory closeout text while runtime unresolved-gate or missing-evidence blockers win.
  - Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, gate_record_id, and legal recover/close actions.
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Validator-gate runtime materialization | SUBFEATURES: gate_record_id, gate phase, CheckResult status, descriptor provenance, evidence refs, role/session proof, authority proof | PRIMITIVES_FEATURES: FEAT-WORKFLOW-ENGINE | MECHANICAL: engine.sovereign, engine.version, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Check outputs become evidence into canonical gate state rather than workflow truth by themselves.
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: Software-delivery closeout derivation substrate | SUBFEATURES: closeout posture, unresolved-gate reasons, missing-evidence reasons, owner/claim blockers, governed-action blockers, workflow binding state | PRIMITIVES_FEATURES: FEAT-LOCUS-WORK-TRACKING | MECHANICAL: engine.librarian, engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus-backed records remain the join target for DCC and Task Board closeout views.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact gate/closeout summary | SUBFEATURES: compact blocker reasons, stable ids, current posture, next eligible action, evidence completeness | PRIMITIVES_FEATURES: FEAT-LLM-FRIENDLY-DATA | MECHANICAL: engine.context, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Local-small-model routing should consume compact gate and closeout state before raw packet prose.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Validator-gate runtime summary | JobModel: WORKFLOW | Workflow: software_delivery_validator_gate_materialization | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: governance.check.completed, governance.check.blocked | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Gate summary records should carry CheckResult status, descriptor provenance, evidence refs, role/session proof, and current gate phase.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Evidence-linked gate execution record | JobModel: WORKFLOW | Workflow: governance_check_runner_to_validator_gate | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: governance.check.started, governance.check.completed, governance.check.blocked | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Raw check output contributes evidence but cannot become closeout truth without canonical gate materialization.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Derived closeout posture | JobModel: WORKFLOW | Workflow: software_delivery_closeout_derivation | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: work_packet_gated, work_packet_completed | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Closeout posture is computed from workflow state, gate state, governed-action resolution, ownership/claim posture, checkpoint lineage, and evidence completeness.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Projection-only gate/closeout display | JobModel: UI_ACTION | Workflow: dcc_task_board_mailbox_gate_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: task_board_synced, task_board_status_changed | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: DCC, Task Board, and Role Mailbox show the same gate and closeout truth while remaining non-authoritative projections.
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/workflows.rs -> IMPLEMENTED (WP-1-Workflow-Projection-Correlation-v1)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md Phase 1 validator-gate and closeout sweep
- CONTEXT_START_LINE: 47940
- CONTEXT_END_LINE: 47945
- CONTEXT_TOKEN: Validator-gate and closeout sweep
- EXCERPT_ASCII_ESCAPED:
  ```text
- [ADD v02.181] Software-delivery governance overlay boundary sweep: Phase 1 MUST keep repository `/.GOV/**` artifacts as imported overlay source material and evidence while live software-delivery authority moves through product-owned runtime records and workflow-backed governed actions.
  - [ADD v02.181] Software-delivery runtime-truth sweep: Phase 1 MUST expose software-delivery work through stable-id-linked runtime records, linked governed actions, and workflow-backed state rather than packet text, mailbox order, or Markdown mirrors acting as operational truth.
  - [ADD v02.181] Validator-gate and closeout sweep: Phase 1 MUST converge validator posture into runtime-visible gate summaries and evidence-linked gate executions, and MUST derive closeout posture from canonical runtime and gate state rather than packet surgery.
  - [ADD v02.181] Projection-surface sweep: Phase 1 MUST keep Dev Command Center, Task Board, and Role Mailbox as projection or control surfaces over the same runtime truth, with no planning lane, inbox thread, or readable mirror becoming authority by chronology alone.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md Phase 1 validator-gate and closeout acceptance
- CONTEXT_START_LINE: 48043
- CONTEXT_END_LINE: 48048
- CONTEXT_TOKEN: Validator-gate and closeout posture
- EXCERPT_ASCII_ESCAPED:
  ```text
- [ADD v02.181] Software-delivery runtime truth: at least one workflow-backed software-delivery work item exposes product-owned runtime state and linked governed actions by stable identifiers instead of relying on packet prose, mailbox order, or Markdown mirrors as the operational authority surface.
  - [ADD v02.181] Validator-gate and closeout posture: at least one workflow-backed software-delivery work item exposes validator-gate summaries, evidence-linked gate posture, and derived closeout posture by stable identifiers without requiring packet surgery to explain why the item may proceed or close.
  - [ADD v02.181] Projection-surface discipline: Dev Command Center, Task Board, and Role Mailbox projections for at least one software-delivery work item explain the same underlying state without turning repo `/.GOV/**`, Markdown mirrors, or mailbox chronology into authority.
  - [ADD v02.181] Overlay coordination records: at least one software-delivery work item exposes overlay claim/lease state and queued steering or follow-up state by stable identifiers so actor ownership, takeover legality, and deferred steering are visible without transcript reconstruction.
  - [ADD v02.181] Overlay lifecycle and recovery posture: at least one software-delivery work item exposes checkpoint-backed recovery posture plus workflow-backed start/steer/cancel/close/recover semantics by stable identifiers so restart-safe replay and control decisions remain inspectable.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md Governance Check Runner validator-gate convergence
- CONTEXT_START_LINE: 31993
- CONTEXT_END_LINE: 31998
- CONTEXT_TOKEN: Validator-gate convergence
- EXCERPT_ASCII_ESCAPED:
  ```text
**Validator-gate convergence (HARD)** [ADD v02.181]
  - Software-delivery validation posture MUST resolve through a dedicated product-owned validator-gate runtime record family or an equivalent canonical runtime record keyed by stable work and gate identifiers.
  - `CheckResult` executions MAY contribute evidence and status updates to that canonical gate state, but a raw check result MUST NOT become workflow truth or closeout truth by itself.
  - `PASS`, `FAIL`, `BLOCKED`, `ADVISORY_ONLY`, and `UNSUPPORTED` outcomes MUST remain queryable through canonical gate state together with evidence references and the originating descriptor provenance.
  - When validator posture participates in workflow progression, closeout, cancellation, or recovery, the canonical gate view MUST also preserve any required authority proof, claim/lease posture, checkpoint lineage, and queued follow-up state that explains why work may or may not advance.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md software-delivery runtime truth specialization
- CONTEXT_START_LINE: 6915
- CONTEXT_END_LINE: 6920
- CONTEXT_TOKEN: validator-gate posture
- EXCERPT_ASCII_ESCAPED:
  ```text
**Software-delivery overlay runtime truth specialization** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, authoritative work meaning MUST resolve through canonical structured records instead of packet prose, board ordering, mailbox chronology, or side-ledger files.
  - Software-delivery structured collaboration state MUST preserve, at minimum, canonical truth for scoped work contract semantics, workflow binding semantics, governed action request/resolution posture, validator-gate posture, and checkpoint/evidence references.
  - Readable task-packet Markdown, Task Board mirrors, and mailbox summaries MAY remain source artifacts and human-readable projections, but they MUST NOT act as the mutable operational ledger for software-delivery execution.
  - Software-delivery-specific fields SHOULD remain profile extensions or profile-specialized records over the shared base envelope so the shared parser, compact summary contract, and validator surface stay reusable across project kinds.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md software-delivery closeout derivation and gate lifecycle
- CONTEXT_START_LINE: 7032
- CONTEXT_END_LINE: 7048
- CONTEXT_TOKEN: committable
- EXCERPT_ASCII_ESCAPED:
  ```text
**Software-delivery closeout derivation** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, authoritative closeout MUST be derived from canonical workflow state, validator-gate posture, governed action resolutions, and evidence references rather than from packet-local checklist surgery, board reshuffling, or manual side-ledger convergence.
  - Human-readable closeout sections, packets, and board badges MAY be synchronized after authoritative closeout becomes true, but they MUST NOT define closeout legality on their own.
  - When closeout remains invalid, the canonical runtime view SHOULD preserve explicit unresolved-gate, missing-evidence, missing-owner, or equivalent blocking reasons so resume and review do not require transcript replay.

  **Software-delivery overlay extension records and lifecycle semantics** [ADD v02.181]

  - Software-delivery validator-gate records SHOULD preserve explicit phases `pending`, `presented`, `acknowledged`, `appending`, `committable`, `committed`, and `archived`. Final PASS authority requires a committable or committed gate plus any required evidence, role/session proof, and claim/lease posture.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md software-delivery close and recover control-plane semantics
- CONTEXT_START_LINE: 7050
- CONTEXT_END_LINE: 7059
- CONTEXT_TOKEN: close
- EXCERPT_ASCII_ESCAPED:
  ```text
**Software-delivery overlay control-plane behaviors** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, start, steer, cancel, close, and recover MUST resolve through workflow-backed governed actions and canonical runtime records instead of repo ledgers, mailbox chronology, or transcript-only intent.
  - `close` MUST remain derived from canonical gate, evidence, governed-action, and ownership truth. A close sequence MAY synchronize readable packet or board artifacts afterward, but it MUST NOT let those artifacts authorize closeout.
  - `recover` MUST resolve through explicit reattach, replay, or checkpoint-restore posture. Recovery MAY reuse queued instructions or claim/lease state where valid, but stale bindings MUST remain visible until authority is re-established.
  - Software-delivery control-plane state SHOULD preserve health posture, stale-detection posture, backpressure posture, and operator-alert posture by stable runtime identifiers.
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md Dev Command Center software-delivery projection law
- CONTEXT_START_LINE: 6678
- CONTEXT_END_LINE: 6680
- CONTEXT_TOKEN: validator-gate posture
- EXCERPT_ASCII_ESCAPED:
  ```text
- DCC is the canonical operator/developer surface to **view** Locus WPs/MTs and bind a **worktree-backed workspace** to a `wp_id`/`mt_id`/`session_id` context.
  - DCC MUST NOT become an alternate authority for work status; it MUST read/write via `locus_*` operations and treat `.handshake/gov/TASK_BOARD.md` as the human-readable mirror.
  - [ADD v02.181] For `project_profile_kind=software_delivery`, Dev Command Center SHOULD project work contract state, workflow-binding state, pending governed actions, validator-gate posture, checkpoint lineage, evidence readiness, claim/lease posture, queued follow-up instructions, binding health, stale detection, and backpressure posture from canonical runtime records.
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md Role Mailbox software-delivery authority boundary
- CONTEXT_START_LINE: 10659
- CONTEXT_END_LINE: 10661
- CONTEXT_TOKEN: closeout state
- EXCERPT_ASCII_ESCAPED:
  ```text
[ADD v02.176] Role Mailbox SHOULD also act as the executor-routing and temporary-claim surface for asynchronous collaboration. When a thread expects action, Handshake MUST preserve who may respond, whether one actor may hold an exclusive lease, when that lease expires, whether takeover is legal, and which reply kinds remain mailbox-local versus linked-authority-triggering so parallel actors do not double-handle or silently steal work.

  [ADD v02.181] For `project_profile_kind=software_delivery`, mailbox summaries, handoff bundles, announce-back traffic, and escalation threads MAY inform linked work, but they MUST NOT directly mutate authoritative workflow meaning, validator posture, accepted evidence, claim/lease posture, queued follow-up state, or closeout state. Any such change MUST resolve through a governed action, a workflow-backed authoritative artifact, or explicit transcription into canonical runtime records.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: v02.181 Validator-gate and closeout posture | WHY_IN_SCOPE: This is the exact stub target and Phase 1 acceptance bullet | EXPECTED_CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs, locus/task_board.rs, role_mailbox.rs | EXPECTED_TESTS: validator_gate_runtime_summary_links_check_evidence; closeout_posture_requires_gate_evidence_owner_and_action_truth | RISK_IF_MISSED: PASS/FAIL/blocked/ready-to-close state remains explainable only by packet prose or transcript replay.
  - CLAUSE: Governance Check Runner validator-gate convergence | WHY_IN_SCOPE: CheckResult executions must contribute to canonical gate state without becoming workflow or closeout truth by themselves | EXPECTED_CODE_SURFACES: runtime_governance.rs, workflows.rs, locus/types.rs | EXPECTED_TESTS: check_result_pass_does_not_close_work_without_gate_materialization | RISK_IF_MISSED: Raw tool output substitutes for product-owned validator posture.
  - CLAUSE: Software-delivery closeout derivation | WHY_IN_SCOPE: Closeout must derive from workflow, validator-gate, governed-action, owner/claim, checkpoint, and evidence truth | EXPECTED_CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs | EXPECTED_TESTS: closeout_posture_requires_gate_evidence_owner_and_action_truth | RISK_IF_MISSED: Work can look complete while gates or evidence are unresolved.
  - CLAUSE: Software-delivery overlay lifecycle semantics | WHY_IN_SCOPE: Final PASS and closeout may depend on claim/lease posture, queued follow-up state, checkpoint lineage, and gate phase | EXPECTED_CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs | EXPECTED_TESTS: validator_gate_final_pass_requires_committable_gate_and_authority_proof | RISK_IF_MISSED: Ownership, recovery, or queued follow-up state can be inferred from comments or mailbox order.
  - CLAUSE: Projection-only DCC/Task Board/Role Mailbox posture | WHY_IN_SCOPE: Gate and closeout posture must be inspectable across operator surfaces while runtime truth remains authoritative | EXPECTED_CODE_SURFACES: locus/task_board.rs, role_mailbox.rs, workflows.rs, runtime_governance.rs | EXPECTED_TESTS: task_board_and_mailbox_closeout_badges_remain_projection_only | RISK_IF_MISSED: Display surfaces become hidden validation or closeout authorities.
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: Validator-gate summary record | PRODUCER: workflow/runtime-governance gate materialization | CONSUMER: DCC, Task Board, Role Mailbox triage, validators, local-small-model summaries | SERIALIZER_TRANSPORT: structured runtime record keyed by work_packet_id, workflow_run_id, and gate_record_id | VALIDATOR_READER: validator-gate runtime tests | TRIPWIRE_TESTS: validator_gate_runtime_summary_links_check_evidence | DRIFT_RISK: Surfaces disagree about PASS/FAIL/BLOCKED gate posture.
  - CONTRACT: CheckResult evidence linkage | PRODUCER: Governance Check Runner | CONSUMER: validator-gate materializer, Flight Recorder, DCC evidence drilldown | SERIALIZER_TRANSPORT: CheckResult plus descriptor_ref, check_descriptor_hash, evidence_artifact_id, and status | VALIDATOR_READER: CheckRunner/gate convergence tests | TRIPWIRE_TESTS: check_result_pass_does_not_close_work_without_gate_materialization | DRIFT_RISK: Raw check output becomes truth without descriptor/evidence provenance.
  - CONTRACT: Closeout derivation payload | PRODUCER: workflow runtime, validator-gate records, governed action registry, claim/lease records, checkpoint lineage | CONSUMER: DCC close/recover controls, Task Board badges, Role Mailbox follow-up, validators | SERIALIZER_TRANSPORT: structured closeout summary with closeout_derivation_id, blocker reasons, and authority refs | VALIDATOR_READER: closeout derivation tests | TRIPWIRE_TESTS: closeout_posture_requires_gate_evidence_owner_and_action_truth | DRIFT_RISK: Closeout depends on packet surgery or transcript reconstruction.
  - CONTRACT: Projection-only gate/closeout badges | PRODUCER: DCC/Task Board/Role Mailbox projection builders | CONSUMER: operator UI and validators | SERIALIZER_TRANSPORT: projection row fields carrying source_record_id, source_kind, mirror posture, and authoritative refs | VALIDATOR_READER: projection conflict tests | TRIPWIRE_TESTS: task_board_and_mailbox_closeout_badges_remain_projection_only | DRIFT_RISK: Board lane, mailbox reply, or mirror status can clear validation or closeout.
  - CONTRACT: Final PASS authority proof | PRODUCER: validator-gate runtime and workflow closeout path | CONSUMER: close action, commit/promotion gates, integration validator | SERIALIZER_TRANSPORT: gate phase plus required evidence, role/session proof, claim/lease posture, and governed-action refs | VALIDATOR_READER: final PASS proof tests | TRIPWIRE_TESTS: validator_gate_final_pass_requires_committable_gate_and_authority_proof | DRIFT_RISK: PASS is accepted without committable/committed gate and required authority proof.
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Inspect current CheckRunner result persistence, runtime_governance, workflow closeout, DCC compact summary, Task Board projection, Role Mailbox export, workflow-state-family, queue-reason, and allowed-action code before adding fields.
  - Define the minimal software-delivery validator-gate summary and closeout derivation payload needed to carry gate phase, CheckResult status, descriptor provenance, evidence refs, authority proof, claim/lease posture, checkpoint lineage, and blocker reasons.
  - Wire DCC, Task Board, and Role Mailbox projections to read the same gate/closeout runtime-backed fields by stable identifiers.
  - Add tests where raw check PASS, packet prose, board lane, and mailbox announce-back disagree with runtime truth and runtime truth wins.
  - Keep repo /.GOV mirrors, Markdown packet prose, board lanes, and mailbox chronology as readable/advisory inputs only.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - `validator_gate_runtime_summary_links_check_evidence`
  - `check_result_pass_does_not_close_work_without_gate_materialization`
  - `closeout_posture_requires_gate_evidence_owner_and_action_truth`
  - `task_board_and_mailbox_closeout_badges_remain_projection_only`
  - `validator_gate_final_pass_requires_committable_gate_and_authority_proof`
- CARRY_FORWARD_WARNINGS:
  - Do not create a second packet-local, DCC-only, board-only, or mailbox-only gate truth store.
  - Do not treat raw CheckResult output, packet prose, unread badges, transcript order, lane position, or mirror freshness as authority.
  - Keep stable identifiers and authority_refs/evidence_refs visible enough for validators to inspect.
  - If implementation discovers missing base schema support, report bounded spec/stub need rather than silently broadening scope.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - v02.181 validator-gate and closeout posture.
  - v02.181 Governance Check Runner validator-gate convergence.
  - v02.181 software-delivery closeout derivation.
  - v02.181 software-delivery overlay lifecycle semantics.
  - v02.181 projection-only DCC/Task Board/Role Mailbox posture.
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- COMMANDS_TO_RUN:
  - `rg -n "validator_gate|CheckResult|governance\\.check|evidence_artifact_id|gate_record_id|closeout|closeout_pending|queue_reason_code|allowed_action_ids|role_mailbox" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml validator_gate_runtime_summary_links_check_evidence -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml check_result_pass_does_not_close_work_without_gate_materialization -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_posture_requires_gate_evidence_owner_and_action_truth -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_and_mailbox_closeout_badges_remain_projection_only -- --exact`
  - `just gov-check`
- POST_MERGE_SPOTCHECKS:
  - Verify DCC, Task Board, and Role Mailbox projection rows expose the same canonical work_packet_id, workflow_run_id, and gate_record_id.
  - Verify a raw CheckResult PASS cannot close work without canonical gate materialization, evidence completeness, owner/claim proof, and governed-action resolution.
  - Verify stale packet prose, Task Board mirrors, and mailbox announce-back text cannot override validator-gate or closeout blockers.
  - Verify closeout blockers remain explicit for unresolved gate, missing evidence, unsupported check, blocked check, missing owner, pending governed action, and recovery/checkpoint gaps.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Exact final field names for gate summary, closeout derivation, gate evidence, and closeout blocker payloads are not proven until implementation inspects current struct boundaries.
  - Whether the best landing surface is a software-delivery profile extension, DccCompactSummaryV1 extension, runtime_governance helper, or workflow projection helper split is not proven yet.
  - Product tests were not run in this Activation Manager refinement-writing pass.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Local spec evidence shows the target is already covered by v02.181 main-body clauses. The implementation should narrow around durable gate summaries, evidence-linked gate executions, closeout derivation inputs, and projection fields rather than importing external workflow patterns.
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
  - NONE
- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.archivist
  - engine.librarian
  - engine.dba
  - engine.sovereign
  - engine.guide
  - engine.context
  - engine.version
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
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - CheckRunner evidence plus validator-gate runtime state -> IN_THIS_WP (stub: NONE)
  - Validator-gate posture plus closeout derivation -> IN_THIS_WP (stub: NONE)
  - DCC closeout explanation plus governed action preview -> IN_THIS_WP (stub: NONE)
  - Task Board rows plus gate blocker projection -> IN_THIS_WP (stub: NONE)
  - Work Packet proof refs plus linked gate evidence -> IN_THIS_WP (stub: NONE)
  - Compact summary plus local model routing -> IN_THIS_WP (stub: NONE)
  - Claim/lease posture plus final PASS authority -> IN_THIS_WP (stub: NONE)
  - Checkpoint lineage plus closeout recovery posture -> IN_THIS_WP (stub: NONE)
  - MicroTask validation wait plus gate evidence refs -> IN_THIS_WP (stub: NONE)
  - Runtime evidence catalog plus compact summary -> IN_THIS_WP (stub: NONE)
  - Projection conflict proof plus stale board mirror -> IN_THIS_WP (stub: NONE)
  - Closeout blocker taxonomy plus owner/claim truth -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Validator-gate runtime materialization | SUBFEATURES: gate_record_id, gate phase, CheckResult status, descriptor provenance, evidence refs, role/session proof, authority proof | PRIMITIVES_FEATURES: FEAT-WORKFLOW-ENGINE | MECHANICAL: engine.sovereign, engine.version, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Check outputs become evidence into canonical gate state rather than workflow truth by themselves.
  - PILLAR: Locus | CAPABILITY_SLICE: Software-delivery closeout derivation substrate | SUBFEATURES: closeout posture, unresolved-gate reasons, missing-evidence reasons, owner/claim blockers, governed-action blockers, workflow binding state | PRIMITIVES_FEATURES: FEAT-LOCUS-WORK-TRACKING | MECHANICAL: engine.librarian, engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus-backed records remain the join target for DCC and Task Board closeout views.
  - PILLAR: Command Center | CAPABILITY_SLICE: Gate and closeout explanation surface | SUBFEATURES: gate summary row, evidence drilldown, ready-to-validate, validator-cleared, integration-blocked, closeout-complete, closeout-blocked action preview | PRIMITIVES_FEATURES: FEAT-DEV-COMMAND-CENTER | MECHANICAL: engine.guide, engine.context, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: DCC explains and requests governed actions; it does not authorize closeout by display state.
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Closeout posture projection | SUBFEATURES: gate badge, blocker reason, closeout eligibility badge, mirror stale marker, authority refs | PRIMITIVES_FEATURES: FEAT-TASK-BOARD | MECHANICAL: engine.context, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Board summaries mirror runtime truth without becoming runtime truth.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Contract mirror and closeout proof boundary | SUBFEATURES: signed scope, validator proof refs, closeout summary refs, packet-prose non-authority marker | PRIMITIVES_FEATURES: FEAT-WORK-PACKET-SYSTEM | MECHANICAL: engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Work Packet records remain contract/evidence carriers while runtime gate and closeout records decide posture.
  - PILLAR: MicroTask | CAPABILITY_SLICE: Validation wait and gate evidence projection | SUBFEATURES: active microtask, validation wait reason, gate_record_id, evidence_ref, closeout blocker summary | PRIMITIVES_FEATURES: FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Micro-task waits and evidence refs must be visible without transcript replay or packet surgery.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: Evidence-linked gate execution | SUBFEATURES: governance.check.started, governance.check.completed, governance.check.blocked, evidence_artifact_id, descriptor hash, duration, blocked reason | PRIMITIVES_FEATURES: FEAT-FLIGHT-RECORDER | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Gate records must join back to durable check evidence.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact gate/closeout summary | SUBFEATURES: compact blocker reasons, stable ids, current posture, next eligible action, evidence completeness | PRIMITIVES_FEATURES: FEAT-LLM-FRIENDLY-DATA | MECHANICAL: engine.context, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Local-small-model routing should consume compact gate and closeout state before raw packet prose.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Validator-gate runtime summary | JobModel: WORKFLOW | Workflow: software_delivery_validator_gate_materialization | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: governance.check.completed, governance.check.blocked | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Gate summary records should carry CheckResult status, descriptor provenance, evidence refs, role/session proof, and current gate phase.
  - Capability: Evidence-linked gate execution record | JobModel: WORKFLOW | Workflow: governance_check_runner_to_validator_gate | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: governance.check.started, governance.check.completed, governance.check.blocked | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Raw check output contributes evidence but cannot become closeout truth without canonical gate materialization.
  - Capability: Derived closeout posture | JobModel: WORKFLOW | Workflow: software_delivery_closeout_derivation | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: work_packet_gated, work_packet_completed | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Closeout posture is computed from workflow state, gate state, governed-action resolution, ownership/claim posture, checkpoint lineage, and evidence completeness.
  - Capability: Projection-only gate/closeout display | JobModel: UI_ACTION | Workflow: dcc_task_board_mailbox_gate_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: task_board_synced, task_board_status_changed | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: DCC, Task Board, and Role Mailbox show the same gate and closeout truth while remaining non-authoritative projections.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Software-Delivery-Runtime-Truth-v1 -> KEEP_SEPARATE
  - WP-1-Software-Delivery-Projection-Surface-Discipline-v1 -> KEEP_SEPARATE
  - WP-1-Product-Governance-Check-Runner-v1 -> KEEP_SEPARATE
  - WP-1-Governance-Workflow-Mirror-v2 -> KEEP_SEPARATE
  - WP-1-Workflow-Projection-Correlation-v1 -> KEEP_SEPARATE
  - WP-1-Dev-Command-Center-Control-Plane-Backend-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - .GOV/task_packets/stubs/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1.md -> NOT_PRESENT (WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1)
  - .GOV/spec/Handshake_Master_Spec_v02.181.md -> IMPLEMENTED (NONE)
  - .GOV/refinements/WP-1-Software-Delivery-Projection-Surface-Discipline-v1.md -> PARTIAL (WP-1-Software-Delivery-Projection-Surface-Discipline-v1)
  - ../handshake_main/src/backend/handshake_core/src/governance_check_runner.rs -> IMPLEMENTED (WP-1-Product-Governance-Check-Runner-v1)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> IMPLEMENTED (WP-1-Workflow-Projection-Correlation-v1)
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs -> PARTIAL (WP-1-Governance-Workflow-Mirror-v2)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Dev-Command-Center-Control-Plane-Backend-v1)
## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: YES
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PASS
- GUI_REFERENCE_DECISIONS:
  - DCC gate evidence drilldown <- NONE (IN_THIS_WP)
  - Task Board and Role Mailbox closeout projection <- NONE (IN_THIS_WP)
- HANDSHAKE_GUI_ADVICE:
  - Surface: Dev Command Center | Control: Gate evidence inspector | Type: icon button | Why: Operators and validators need the gate record, CheckResult provenance, and evidence refs behind each status | Microcopy: Gate evidence | Tooltip: Show canonical gate ids, check descriptor, evidence refs, and blockers
  - Surface: Dev Command Center | Control: Close request preview | Type: icon button | Why: Closeout must preview the governed action, transition rule, and blocker/evidence state before mutation | Microcopy: Preview close | Tooltip: Show why this work can or cannot close
  - Surface: Task Board | Control: Closeout eligibility badge | Type: status chip | Why: Board users need compact posture without board lane becoming authority | Microcopy: Closeout blocked or Ready to close | Tooltip: Explain runtime closeout posture and source ids
  - Surface: Role Mailbox | Control: Advisory gate context | Type: status chip | Why: Mailbox announce-back may inform linked work but cannot clear validation alone | Microcopy: Advisory | Tooltip: Linked gate state must change through runtime records
- HIDDEN_GUI_REQUIREMENTS:
  - Mutation controls remain disabled when canonical gate or closeout derivation fields are absent or stale even if a visible mirror suggests work is ready.
  - Cross-surface conflict state must name DCC, Task Board, packet, and Role Mailbox values while marking canonical runtime gate state as winning.
  - Closeout controls must show unresolved gate/evidence/owner/action/checkpoint blockers before allowing a close request.
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - Keep gate/closeout rows compact but include expandable gate_record_id, check_result_id, evidence_artifact_id, workflow_run_id, and closeout_derivation_id.
  - Store closeout preview payloads as structured data so validators can assert legal transitions without screenshot inspection.
  - Emit one test fixture where raw check PASS exists but closeout remains blocked until canonical gate/evidence/owner truth is satisfied.
## SCOPE
- What: Implement and prove runtime-visible validator-gate summaries, evidence-linked gate executions, and derived closeout posture for one workflow-backed software-delivery work item.
- Why: v02.181 forbids packet surgery, raw check output, board reshuffling, mailbox chronology, or mirror freshness from deciding validator posture or closeout legality without canonical runtime gate and evidence state.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - Rewriting historical packet reports or packet-local validator narratives.
  - Making raw CheckResult output authoritative workflow or closeout truth by itself.
  - Cosmetic UI redesign or broad layout registry work.
  - Non-software-delivery closeout policy.
  - Official packet creation, signature recording, coder launch, or validator launch during this refinement-writing turn.
- TOUCHED_FILE_BUDGET: 7
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (Do not use `## WAIVERS GRANTED` to continue after token-cost overrun. Token budget and token-ledger drift are diagnostic-only cost telemetry and must be surfaced mechanically in audits/dossiers instead of requiring a continuation waiver.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml validator_gate_runtime_summary_links_check_evidence -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml check_result_pass_does_not_close_work_without_gate_materialization -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_posture_requires_gate_evidence_owner_and_action_truth -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_and_mailbox_closeout_badges_remain_projection_only -- --exact
  just gov-check
```

### DONE_MEANS
- At least one workflow-backed software-delivery work item exposes validator-gate summaries by stable identifiers.
- Gate execution posture links to CheckResult status, descriptor provenance, evidence artifact ids, role/session proof, and gate phase.
- Closeout posture is derived from workflow state, validator-gate posture, governed-action resolutions, ownership/claim posture, checkpoint lineage, and evidence completeness.
- Runtime closeout blockers distinguish unresolved gate, missing evidence, missing owner/claim, pending governed action, checkpoint/recovery gap, unsupported check, and blocked check cases.
- DCC, Task Board, and Role Mailbox projections can display the same gate and closeout posture without becoming authority.
- Tests include a negative case where raw check PASS, packet prose, board lane, or mailbox announce-back exists but closeout remains blocked until canonical runtime truth is satisfied.

- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-05-04T00:01:00.947Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.181]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.181.md v02.181 validator-gate convergence and software-delivery closeout derivation
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
  - .GOV/task_packets/stubs/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1.md
  - .GOV/spec/Handshake_Master_Spec_v02.181.md
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- SEARCH_TERMS:
  - validator_gate
  - CheckResult
  - governance.check.completed
  - evidence_artifact_id
  - gate_record_id
  - closeout
  - closeout_pending
  - queue_reason_code
  - allowed_action_ids
  - role_mailbox
- RUN_COMMANDS:
  ```bash
rg -n "validator_gate|CheckResult|governance\\.check|evidence_artifact_id|gate_record_id|closeout|closeout_pending|queue_reason_code|allowed_action_ids|role_mailbox" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml validator_gate_runtime_summary_links_check_evidence -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml check_result_pass_does_not_close_work_without_gate_materialization -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_posture_requires_gate_evidence_owner_and_action_truth -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_and_mailbox_closeout_badges_remain_projection_only -- --exact
  just gov-check
  ```
- RISK_MAP:
  - "Raw CheckResult PASS becomes closeout truth" -> "Work closes while evidence, role proof, ownership, or governed-action state is incomplete."
  - "Packet validation note outranks gate record" -> "Packet surgery can hide pending, blocked, or unsupported validator posture."
  - "Task Board or DCC badge becomes authority" -> "Operators can close or validate from stale display state."
  - "Mailbox announce-back is treated as completion" -> "A reply or handoff can substitute for workflow-backed gate/closeout records."
  - "Gate records lack stable ids" -> "DCC, Task Board, mailbox, validators, and local models cannot prove they describe the same work item."
## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- For `PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1`, this section is copied from the signed refinement and should not drift.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center work item detail, validation queue, closeout queue, and gate evidence drilldown.
  - Task Board derived software-delivery planning rows and closeout badges.
  - Role Mailbox triage rows linked to review, validation, escalation, or announce-back threads.
  - Runtime-gate and closeout derivation inspector panels.
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Gate evidence inspector | Type: icon button | Tooltip: Show check result, descriptor, evidence, and role/session proof for this gate | Notes: Read-only drilldown unless a governed action is invoked.
  - Control: Closeout eligibility badge | Type: status chip | Tooltip: Explain unresolved gate, evidence, owner, action, or checkpoint blockers | Notes: Badge is derived from runtime state.
  - Control: Validator-gate phase filter | Type: menu | Tooltip: Filter by pending, presented, acknowledged, appending, committable, committed, or archived gate phase | Notes: Reads gate records only.
  - Control: Close request preview | Type: icon button | Tooltip: Preview target close action, transition rule, evidence refs, and blockers before request | Notes: Must not close directly.
  - Control: Evidence completeness indicator | Type: status chip | Tooltip: Show whether required evidence artifacts and hashes are attached | Notes: Include non-color accessible label.
  - Control: Runtime versus mirror compare | Type: segmented control | Tooltip: Compare runtime gate truth with packet, board, and mailbox projections | Notes: Runtime wins on conflicts.
- UI_STATES (empty/loading/error):
  - Empty state says no runtime validator-gate record exists yet and offers no fake validation status.
  - Loading state preserves the last verified timestamp and disables close controls until authority refs load.
  - Error state distinguishes missing gate record, missing evidence, unsupported check, blocked check, stale mirror, and mailbox-only advisory state.
  - Conflict state shows packet, board, mailbox, and runtime values with runtime gate/closeout state winning.
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Use labels such as Gate pending, Gate blocked, Evidence missing, Validator cleared, Integration blocked, Closeout pending, Ready to close, Runtime truth, Projection only, and Mailbox advisory.
  - Avoid wording that implies raw check output, board lane, packet note, or mailbox reply is itself authority.
  - Every close, recover, validate, or acknowledge action should name the governed action id and target records before confirmation.
- UI_ACCESSIBILITY_NOTES:
  - Tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
  - Status chips must not rely on color alone; include accessible labels for pending, blocked, missing evidence, cleared, and ready-to-close states.
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
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
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
  - LOG_PATH: `.handshake/logs/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1/<name>.log` (recommended; not committed)
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
