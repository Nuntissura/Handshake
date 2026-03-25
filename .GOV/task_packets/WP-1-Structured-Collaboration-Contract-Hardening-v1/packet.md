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
- For `REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1`, this packet is auto-hydrated from the signed refinement; manual drift is forbidden and `just pre-work` enforces alignment.

---

# Task Packet: WP-1-Structured-Collaboration-Contract-Hardening-v1

## METADATA
- TASK_ID: WP-1-Structured-Collaboration-Contract-Hardening-v1
- WP_ID: WP-1-Structured-Collaboration-Contract-Hardening-v1
- BASE_WP_ID: WP-1-Structured-Collaboration-Contract-Hardening
- DATE: 2026-03-25T04:34:58.378Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
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
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: OPENAI_GPT_SERIES_ONLY
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Structured-Collaboration-Contract-Hardening-v1
<!-- Validator roles keep distinct local branches/worktrees, but they mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Contract-Hardening-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-contract-hardening-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Contract-Hardening-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Contract-Hardening-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Contract-Hardening-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Contract-Hardening-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Contract-Hardening-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Contract-Hardening-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Structured-Collaboration-Contract-Hardening-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Structured-Collaboration-Contract-Hardening-v1
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- GOVERNED_VALIDATOR_COMPLETION_FIELDS: WORKFLOW_VALIDITY | SCOPE_VALIDITY | PROOF_COMPLETENESS | INTEGRATION_READINESS | DOMAIN_GOAL_COMPLETION
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Validated (PASS)
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Role-Mailbox
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Contract-Hardening-v1
- LOCAL_WORKTREE_DIR: ../wtc-contract-hardening-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Contract-Hardening-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Contract-Hardening-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Contract-Hardening-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Contract-Hardening-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Contract-Hardening-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Contract-Hardening-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: 019d234a-da1a-76f1-8f9a-2da8df9ee610
- INTEGRATION_VALIDATOR_OF_RECORD: 019d23d3-8baa-7081-8e8d-cc94b137819b
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja250320260532
- PACKET_FORMAT_VERSION: 2026-03-23

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: VALIDATED_PASS
Blockers: NONE
Next: NONE

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.171] canonical Work Packet and Micro-Task records SHALL expose governed `allowed_action_ids` rather than ad hoc verbs | CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/storage/locus_sqlite.rs`, `src/backend/handshake_core/src/locus/types.rs` | TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` | EXAMPLES: Mutated Work Packet `packet.json` with an unregistered `allowed_action_ids[0]`, Mutated Micro-Task `packet.json` with an unregistered `allowed_action_ids[0]`, Mutated Task Board `index.json` row whose workflow-state triplet no longer matches linked backend truth, Mutated Role Mailbox `index.json` with multiline or oversized `subject_redacted`, Mutated Role Mailbox thread line with multiline or oversized `note_redacted` | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: [ADD v02.171] Task Board rows SHOULD expose portable workflow-state and queue-reason semantics rather than board-status heuristics | CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs` | TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board` | EXAMPLES: Mutated Work Packet `packet.json` with an unregistered `allowed_action_ids[0]`, Mutated Micro-Task `packet.json` with an unregistered `allowed_action_ids[0]`, Mutated Task Board `index.json` row whose workflow-state triplet no longer matches linked backend truth, Mutated Role Mailbox `index.json` with multiline or oversized `subject_redacted`, Mutated Role Mailbox thread line with multiline or oversized `note_redacted` | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 redacted fields MUST be bounded Secret-Redactor outputs | CODE_SURFACES: `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/role_mailbox.rs`, `src/backend/handshake_core/tests/role_mailbox_tests.rs` | TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | EXAMPLES: Mutated Work Packet `packet.json` with an unregistered `allowed_action_ids[0]`, Mutated Micro-Task `packet.json` with an unregistered `allowed_action_ids[0]`, Mutated Task Board `index.json` row whose workflow-state triplet no longer matches linked backend truth, Mutated Role Mailbox `index.json` with multiline or oversized `subject_redacted`, Mutated Role Mailbox thread line with multiline or oversized `note_redacted` | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Mechanical gate (HARD) RoleMailboxExportGate must reject malformed export thread-line field sets | CODE_SURFACES: `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/role_mailbox.rs`, `src/backend/handshake_core/tests/role_mailbox_tests.rs` | TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | EXAMPLES: Mutated Work Packet `packet.json` with an unregistered `allowed_action_ids[0]`, Mutated Micro-Task `packet.json` with an unregistered `allowed_action_ids[0]`, Mutated Task Board `index.json` row whose workflow-state triplet no longer matches linked backend truth, Mutated Role Mailbox `index.json` with multiline or oversized `subject_redacted`, Mutated Role Mailbox thread line with multiline or oversized `note_redacted` | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox`
- CANONICAL_CONTRACT_EXAMPLES:
  - Mutated Work Packet `packet.json` with an unregistered `allowed_action_ids[0]`
  - Mutated Micro-Task `packet.json` with an unregistered `allowed_action_ids[0]`
  - Mutated Task Board `index.json` row whose workflow-state triplet no longer matches linked backend truth
  - Mutated Role Mailbox `index.json` with multiline or oversized `subject_redacted`
  - Mutated Role Mailbox thread line with multiline or oversized `note_redacted`
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Structured-Collaboration-Contract-Hardening-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- CONTEXT_START_LINE: 6928
- CONTEXT_END_LINE: 6987
- CONTEXT_TOKEN: **Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]
- EXCERPT_ASCII_ESCAPED:
  ```text
**Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]

  - Every canonical Work Packet, Micro-Task, Task Board projection row, and Dev Command Center queue row SHALL expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
  - `workflow_state_family` MUST stay low-cardinality and project-agnostic. Phase 1 base families are:
    - `intake`
    - `ready`
    - `active`
    - `waiting`
    - `review`
    - `approval`
    - `validation`
    - `blocked`
    - `done`
    - `canceled`
    - `archived`
  - The families SHALL be interpreted as:
    - `intake`: known work that still requires triage or decomposition.
    - `ready`: executable work with enough context, dependencies, and permissions to begin.
    - `active`: work currently being executed by a human, local small model, cloud model, or workflow.
    - `waiting`: work expected to resume after an external response, dependency, or scheduled retry.
    - `review`: work awaiting human or model review rather than new execution.
    - `approval`: work awaiting an explicit governance or operator decision.
    - `validation`: work awaiting deterministic checks, rubric checks, or acceptance verification.
    - `blocked`: work that cannot progress safely until a blocker is cleared.
    - `done`: work completed but still visible to current operating views.
    - `canceled`: work explicitly stopped and not expected to resume automatically.
    - `archived`: closed work retained for history, evidence, or search only.
  - `queue_reason_code` MUST explain why the record is currently routed or grouped where it is. Phase 1 base reasons are:
    - `needs_triage`
    - `dependency_wait`
    - `mailbox_response_wait`
    - `mailbox_snoozed`
    - `human_review_wait`
    - `decision_wait`
    - `approval_wait`
    - `validation_wait`
    - `escalation_wait`
    - `mailbox_expired`
    - `dead_letter_remediation`
    - `operator_pause`
    - `policy_block`
    - `resource_unavailable`
    - `retry_scheduled`
    - `ready_for_local_small_model`
    - `ready_for_cloud_model`
    - `completed`
    - `rejected`
    - `canceled`
  - Board position, queue order, and mailbox thread order MUST NOT become substitutes for `workflow_state_family` or `queue_reason_code`.
  - Every state-changing operator or model action SHOULD resolve through a registered `GovernedActionDescriptorV1` so the system knows:
    - who may invoke the action
    - which base families it may start from
    - which family and reason it produces
    - whether approval or evidence is required
    - whether linked record kinds or workflow activation are mandatory
  - Project profiles MAY define `ProjectProfileWorkflowExtensionV1` mappings that rename visible state labels or narrow valid reasons and actions, but those mappings MUST NOT change the meaning of the base families.
  - Local-small-model routing MUST default to `workflow_state_family` plus `queue_reason_code` and only then consult project-profile extensions, note sidecars, or Markdown mirrors.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Task Board projection viewer workflow portability [ADD v02.171]
- CONTEXT_START_LINE: 60910
- CONTEXT_END_LINE: 60922
- CONTEXT_TOKEN: [ADD v02.171] Board rows SHOULD expose `workflow_state_family` and `queue_reason_code` separately from any project-specific display label so queue semantics remain portable across project kernels.
- EXCERPT_ASCII_ESCAPED:
  ```text
- **Task Board projection viewer**
    - Show structured board rows keyed by stable `task_board_id` and `work_packet_id`, plus freshness, manual-edit detection, and sync status.
    - Any Markdown board is read-only by default from this view unless a governed sync or status-update workflow is being invoked.
    - [ADD v02.168] Board rows SHOULD expose the base-envelope status, next action, blockers, and project-profile kind before board-specific grouping metadata.
    - [ADD v02.170] Board, list, queue, and roadmap layouts SHOULD read from the same row set and declare which lane definitions, grouping keys, and action bindings are active for the current preset.
    - [ADD v02.171] Board rows SHOULD expose `workflow_state_family` and `queue_reason_code` separately from any project-specific display label so queue semantics remain portable across project kernels.
  - **Role Mailbox triage**
    - Show message type, expected response, expiry, evidence references, linked Work Packet or Micro-Task identifiers, and handoff completeness.
    - Role Mailbox remains non-authoritative, but Dev Command Center MUST make collaboration state queryable without reading transcript blobs line by line.
    - [ADD v02.168] Thread and message views SHOULD expose the shared base-envelope fields and any mailbox-specific profile extensions separately.
    - [ADD v02.170] Inbox-triage presets SHOULD group by expected response, expiry, linked work identifier, or escalation posture, and MUST keep any reply or escalation mutation visibly separate from non-authoritative message text.
    - [ADD v02.171] Mailbox rows SHOULD show when expected-response or escalation posture contributes to a linked record's `queue_reason_code`, without turning the mailbox thread into the authority for the linked record's `workflow_state_family`.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Required state contract and governed action behavior [ADD v02.171]
- CONTEXT_START_LINE: 61025
- CONTEXT_END_LINE: 61054
- CONTEXT_TOKEN: **Required state contract**
- EXCERPT_ASCII_ESCAPED:
  ```text
**Required state contract**
  - Canonical records SHOULD expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
    - optional project-profile display labels
  - `workflow_state_family` MUST remain portable across record kinds.
  - `queue_reason_code` MUST explain why the record is currently grouped, queued, or blocked.
  - `allowed_action_ids` MUST reference registered `GovernedActionDescriptorV1` records rather than ad hoc user-interface verbs.

  **Required queue and routing behavior**
  - Local-small-model queues SHOULD prefer records where:
    - `workflow_state_family=ready`
    - `queue_reason_code=ready_for_local_small_model`
  - Cloud-model routing SHOULD prefer records where:
    - `workflow_state_family=ready`
    - `queue_reason_code=ready_for_cloud_model`
      or
    - `workflow_state_family=waiting`
    - `queue_reason_code=escalation_wait`
  - Review and approval queues MUST distinguish:
    - `workflow_state_family=review`
    - `queue_reason_code=human_review_wait`
    - `workflow_state_family=approval`
    - `queue_reason_code=approval_wait`
  - Validation queues MUST use `workflow_state_family=validation` plus explicit validation reasons rather than generic blocked state.
  - Mailbox-linked waits MUST remain visible as `queue_reason_code=mailbox_response_wait`, but the mailbox thread itself MUST NOT become the authority for the linked record's state family.

  **Required action behavior**
  - `GovernedActionDescriptorV1` SHOULD be the reusable contract for verbs such as:
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md RoleMailboxIndexV1 and RoleMailboxThreadLineV1 base-envelope contract
- CONTEXT_START_LINE: 11023
- CONTEXT_END_LINE: 11084
- CONTEXT_TOKEN: interface RoleMailboxIndexV1 {
- EXCERPT_ASCII_ESCAPED:
  ```text
interface RoleMailboxIndexV1 {
    schema_id: 'hsk.role_mailbox_index@1';
    schema_version: 'role_mailbox_export_v1';
    record_id: 'role_mailbox_index';
    record_kind: 'generic';
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    updated_at: string; // RFC3339; equals generated_at for full export snapshots
    generated_at: string; // RFC3339
    authority_refs: string[];
    evidence_refs: string[];
    mirror_contract?: MarkdownMirrorContractV1;
    threads: Array<{
      thread_id: string;
      created_at: string; // RFC3339
      closed_at?: string | null; // RFC3339
      participants: string[]; // RoleId rendered as strings
      context: {
        spec_id?: string | null;
        work_packet_id?: string | null;
        task_board_id?: string | null;
        governance_mode: 'gov_strict' | 'gov_standard' | 'gov_light';
        project_id?: string | null;
      };
      subject_redacted: string; // MUST be Secret-Redactor output; bounded
      subject_sha256: string;   // sha256 of original subject bytes (UTF-8)
      message_count: number;
      thread_file: string; // "threads/<thread_id>.jsonl"
    }>;
  }

  // docs/ROLE_MAILBOX/threads/<thread_id>.jsonl (one JSON object per line)
  // This is a canonical JSON encoding of RoleMailboxMessage, but MUST NOT include any inline body.
  type RoleMailboxThreadLineV1 = {
    schema_id: 'hsk.role_mailbox_thread_line@1';
    schema_version: 'role_mailbox_export_v1';
    record_id: string;
    record_kind: 'role_mailbox_message';
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    updated_at: string; // RFC3339; equals created_at unless a mailbox export rewraps the same canonical message
    message_id: string;
    thread_id: string;
    created_at: string; // RFC3339
    from_role: string;
    to_roles: string[];
    message_type: string;
    authority_refs: string[];
    evidence_refs: string[];
    mirror_contract?: MarkdownMirrorContractV1;
    body_ref: string;    // artifact handle string
    body_sha256: string; // sha256
    attachments: string[];
    relates_to_message_id?: string | null;
    transcription_links: Array<{
      target_kind: string;
      target_ref: string;
      target_sha256: string;
      note_redacted: string; // MUST be Secret-Redactor output; bounded
      note_sha256: string;   // sha256 of original note bytes (UTF-8)
    }>;
    idempotency_key: string;
  };
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Mechanical gate (HARD): RoleMailboxExportGate
- CONTEXT_START_LINE: 11115
- CONTEXT_END_LINE: 11125
- CONTEXT_TOKEN: Mechanical gate (HARD): RoleMailboxExportGate
- EXCERPT_ASCII_ESCAPED:
  ```text
Mechanical gate (HARD): RoleMailboxExportGate
  - The runtime MUST provide a mechanical gate that verifies the export is in sync and leak-safe.
  - The gate MUST fail if:
    - `export_manifest.json` hashes do not match current `index.json` / thread files,
    - any thread JSONL line is not valid JSON or violates the RoleMailboxThreadLineV1 field set,
    - any governance-critical message lacks required `transcription_links`,
    - any export file contains forbidden inline body fields (e.g., `body`, `body_text`, `raw_body`).
  - The repo MUST expose the gate as a deterministic command and integrate it into the standard workflow gates:
    - Script: `scripts/validation/role_mailbox_export_check.mjs`
    - Command: `just role-mailbox-export-check`
    - Inclusion: `just post-work {WP_ID}` MUST run this gate in GOV_STANDARD/GOV_STRICT workflows.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.171] canonical Work Packet and Micro-Task records SHALL expose governed `allowed_action_ids` rather than ad hoc verbs | WHY_IN_SCOPE: current emitters still synthesize action ids from workflow families and the spec explicitly forbids ad hoc UI verbs | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/storage/locus_sqlite.rs`, `src/backend/handshake_core/src/locus/types.rs` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` | RISK_IF_MISSED: downstream consumers continue reading the wrong contract even though the field exists
  - CLAUSE: [ADD v02.171] Task Board rows SHOULD expose portable workflow-state and queue-reason semantics rather than board-status heuristics | WHY_IN_SCOPE: current row emission still derives `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` from `TaskBoardStatus` | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board` | RISK_IF_MISSED: Task Board remains a lossy projection that hides true routing semantics
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 redacted fields MUST be bounded Secret-Redactor outputs | WHY_IN_SCOPE: current validator still accepts `subject_redacted` and `note_redacted` as generic non-empty strings | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/role_mailbox.rs`, `src/backend/handshake_core/tests/role_mailbox_tests.rs` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | RISK_IF_MISSED: export gate can pass leak-unsafe payloads as long as they are syntactically non-empty
  - CLAUSE: Mechanical gate (HARD) RoleMailboxExportGate must reject malformed export thread-line field sets | WHY_IN_SCOPE: the spec requires leak-safe rejection and the current negative-path proof does not cover malformed redacted outputs | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/role_mailbox.rs`, `src/backend/handshake_core/tests/role_mailbox_tests.rs` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | RISK_IF_MISSED: the mechanical gate remains weaker than the contract it claims to enforce
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: Work Packet `packet.json` and `summary.json` workflow-state triplet | PRODUCER: `src/backend/handshake_core/src/workflows.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and downstream viewers | SERIALIZER_TRANSPORT: serde JSON files | VALIDATOR_READER: `validate_structured_collaboration_record` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` | DRIFT_RISK: action ids remain ad hoc strings and the validator still accepts them
  - CONTRACT: Micro-Task packet and summary workflow-state triplet plus SQLite progress metadata | PRODUCER: `src/backend/handshake_core/src/workflows.rs` and `src/backend/handshake_core/src/storage/locus_sqlite.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and progress readers | SERIALIZER_TRANSPORT: serde JSON and SQLite-backed metadata JSON | VALIDATOR_READER: `validate_structured_collaboration_record` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` | DRIFT_RISK: alternate emitters stay weaker than the canonical artifact writer
  - CONTRACT: TaskBoardIndexV1 and TaskBoardViewV1 row workflow-state triplets | PRODUCER: `src/backend/handshake_core/src/workflows.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and Task Board viewers | SERIALIZER_TRANSPORT: serde JSON `index.json` and `views/<view_id>.json` | VALIDATOR_READER: `validate_structured_collaboration_record` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board` | DRIFT_RISK: row semantics are flattened from lane status instead of preserved from linked backend truth
  - CONTRACT: RoleMailboxIndexV1 thread metadata | PRODUCER: `src/backend/handshake_core/src/role_mailbox.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and mailbox viewers | SERIALIZER_TRANSPORT: JSON `index.json` | VALIDATOR_READER: `validate_structured_collaboration_record` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | DRIFT_RISK: bounded redacted subject fields remain syntactically valid but semantically weak
  - CONTRACT: RoleMailboxThreadLineV1 redacted notes and transcription links | PRODUCER: `src/backend/handshake_core/src/role_mailbox.rs` | CONSUMER: `src/backend/handshake_core/src/locus/types.rs` and `validate_runtime_mailbox_record` | SERIALIZER_TRANSPORT: JSONL thread files | VALIDATOR_READER: `validate_structured_collaboration_record` and mailbox export gate | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` | DRIFT_RISK: malformed redacted fields can still pass because the validator only requires non-empty strings
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Add one canonical governed-action registry helper in `src/backend/handshake_core/src/locus/types.rs` or an adjacent Locus module, using `GovernedActionDescriptorV1` ids as the emitted and validated contract.
  - Route `src/backend/handshake_core/src/workflows.rs` Work Packet, Micro-Task, and Task Board emitters through that registry helper instead of family-default ad hoc verbs.
  - Route `src/backend/handshake_core/src/storage/locus_sqlite.rs` micro-task progress metadata through the same registry helper or retire the weaker emitter path.
  - Replace Task Board row workflow-state derivation from `TaskBoardStatus` with preservation of authoritative linked backend workflow semantics.
  - Harden mailbox export validation in `src/backend/handshake_core/src/locus/types.rs` so `subject_redacted` and `note_redacted` prove bounded, single-line redacted form.
  - Add mutation-based negative tests in `micro_task_executor_tests.rs` and `role_mailbox_tests.rs` for unregistered action ids, Task Board projection drift, and malformed redacted export fields.
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- CARRY_FORWARD_WARNINGS:
  - Do not reopen the already-closed v4 scope around workflow-state field presence, nested row or thread validation, or typed timestamp and sha field enforcement unless the current code proves a concrete regression.
  - Keep the change centered on governed action ids, Task Board workflow fidelity, mailbox redacted-field validation, and negative-path proof. Do not widen into Loom portability or repo governance.
  - Avoid inventing a speculative generic workflow engine; one explicit governed-action registry helper is enough for this pass.
  - Do not let the SQLite-backed progress path remain a second-class producer with weaker semantics after the canonical artifact writer is fixed.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - [ADD v02.171] `allowed_action_ids` resolves to governed action descriptors rather than ad hoc verbs
  - [ADD v02.171] Task Board rows preserve explicit workflow-state and queue-reason semantics from authoritative backend records
  - RoleMailboxIndexV1 and RoleMailboxThreadLineV1 redacted fields are mechanically validated as bounded redacted outputs
  - Mechanical gate (HARD) RoleMailboxExportGate rejects malformed redacted export payloads
- FILES_TO_READ:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  - rg -n "GovernedActionDescriptorV1|allowed_action_ids|task_board_workflow_state|subject_redacted|note_redacted|workflow_state_family|queue_reason_code" src/backend/handshake_core
- POST_MERGE_SPOTCHECKS:
  - Verify no producer path still emits ad hoc action verbs after the main emitters are fixed.
  - Verify Task Board row workflow semantics are preserved from linked backend truth rather than only renamed heuristics.
  - Verify malformed `subject_redacted` and `note_redacted` payloads fail at the shared validation and export-gate boundary.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Whether the cleanest governed-action registry implementation should live fully in `locus/types.rs` or in a nearby Locus helper module
  - Whether preserving Task Board row semantics will require additional durable metadata beyond what the current status-driven projection path exposes
  - Whether any existing mailbox export fixtures already contain tolerated redacted-field shapes that the stricter validator will surface immediately
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Internal sources are sufficient because the unresolved work is explicit in the current Master Spec and in the current local producers and validators.
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
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_EXPOSED:
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
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
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - Locus governed-action registry foundation -> IN_THIS_WP (stub: NONE)
  - Locus action registry wired into Work Packet artifacts -> IN_THIS_WP (stub: NONE)
  - MicroTask action registry wired into canonical artifacts -> IN_THIS_WP (stub: NONE)
  - MicroTask SQLite progress parity -> IN_THIS_WP (stub: NONE)
  - Locus validator action-id legality checks -> IN_THIS_WP (stub: NONE)
  - Locus Task Board workflow truth preservation -> IN_THIS_WP (stub: NONE)
  - LLM-friendly mailbox export leak-safety -> IN_THIS_WP (stub: NONE)
  - Mailbox export gate plus typed redaction validation -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: Governed action registry-backed action ids on canonical structured collaboration records | SUBFEATURES: Work Packet packet and summary records, Micro-Task packet and summary records, shared validator legality checks | PRIMITIVES_FEATURES: PRIM-GovernedActionDescriptorV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask | MECHANICAL: engine.director, engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: current producers still emit ad hoc verbs and the shared validator still accepts any string array
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Preserve authoritative workflow-state triplets into Task Board rows | SUBFEATURES: row `workflow_state_family`, `queue_reason_code`, `allowed_action_ids`, lane projection from linked backend truth rather than lane heuristic defaults | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-TrackedWorkPacket | MECHANICAL: engine.director, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: current row emission derives semantics from `TaskBoardStatus`
  - PILLAR: MicroTask | CAPABILITY_SLICE: SQLite-backed progress metadata contract alignment | SUBFEATURES: micro-task progress metadata `allowed_action_ids`, shared action-registry helper reuse, parity with canonical artifact writers | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-GovernedActionDescriptorV1 | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: alternate producer paths must not preserve weaker semantics after the main artifact writer is fixed
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Leak-safe Role Mailbox export validation | SUBFEATURES: bounded `subject_redacted`, bounded `note_redacted`, shared export gate rejection of malformed redacted fields | PRIMITIVES_FEATURES: PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1 | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: current emitter is stronger than the validator and gate proof must close that mismatch
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Negative-path proof of structured collaboration law | SUBFEATURES: rejection of unregistered action ids, rejection of workflow-projection drift, rejection of malformed redacted export text | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry, PRIM-RoleMailboxThreadLineV1 | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this packet should close semantic proof, not only implementation wiring
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Registry-backed `allowed_action_ids` for Work Packet and Micro-Task artifacts | JobModel: WORKFLOW | Workflow: Locus artifact emission | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: emitted action ids must stop being ad hoc verbs and resolve through one canonical runtime registry helper
  - Capability: Task Board projection fidelity from authoritative workflow-state triplets | JobModel: WORKFLOW | Workflow: Task Board sync and projection export | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: row semantics must come from linked backend truth before lane aliasing or display grouping
  - Capability: SQLite-backed micro-task progress metadata parity | JobModel: WORKFLOW | Workflow: micro-task progress persistence and export | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: alternate emitters must align to the same action registry contract
  - Capability: Leak-safe mailbox export validation | JobModel: MECHANICAL_TOOL | Workflow: RoleMailbox export gate | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: FR-EVT-GOV-MAILBOX-002 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: malformed bounded-redaction fields must fail the same mechanical gate that validates thread-line field sets
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Structured-Collaboration-Contract-Hardening-v1 -> EXPAND_IN_THIS_WP
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/tests/role_mailbox_tests.rs -> PARTIAL (NONE)
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
- What: Close the remaining Master Spec product gaps left after Schema Registry v4 by replacing ad hoc action ids with governed action descriptors, preserving authoritative Task Board workflow semantics, and hardening leak-safe mailbox export validation.
- Why: The 2026-03-25 smoketest review proved that v4 fixed the original shallow-validator defects but did not deliver full Master Spec correctness for governed actions, Task Board projection fidelity, or mailbox leak-safe validation.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- OUT_OF_SCOPE:
  - Broad Loom portability work
  - Repo-governance workflow-harness remediation
  - Broad Dev Command Center or Task Board UI redesign beyond the backend contract surfaces needed to prove correctness
  - New spec text or appendix version bumps unless coding proves a real Main Body gap
- TOUCHED_FILE_BUDGET: 7
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
```

### DONE_MEANS
- Every in-scope canonical structured-collaboration producer emits `allowed_action_ids` as registered `GovernedActionDescriptorV1.action_id` values only.
- The shared validator rejects unregistered or malformed `allowed_action_ids`.
- Task Board rows preserve authoritative `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` from linked backend truth instead of recomputing them from board-status heuristics.
- RoleMailbox export validation rejects malformed, unbounded, or non-redacted `subject_redacted` and `note_redacted` values.
- Negative-path tests prove all of the above failures are mechanically blocked.

- PRIMITIVES_EXPOSED:
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.178.md (recorded_at: 2026-03-25T04:34:58.378Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.171]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md workflow-state, governed-action, Task Board projection, and RoleMailbox export-gate contracts [ADD v02.171]
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
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- SEARCH_TERMS:
  - GovernedActionDescriptorV1
  - allowed_action_ids
  - task_board_workflow_state
  - workflow_state_family
  - queue_reason_code
  - subject_redacted
  - note_redacted
  - RoleMailboxExportGate
- RUN_COMMANDS:
  ```bash
rg -n "GovernedActionDescriptorV1|allowed_action_ids|task_board_workflow_state|subject_redacted|note_redacted|workflow_state_family|queue_reason_code" src/backend/handshake_core
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox
  ```
- RISK_MAP:
  - "Action-id registry hardening exposes hidden coupling between UI verbs and backend mutation routes" -> "emitters, validators, or consumers may fail until they all use one canonical registry helper"
  - "Task Board projection preservation needs a durable backend truth source" -> "row semantics can stay flattened if the implementation only renames heuristics"
  - "Mailbox redaction proof can look green on happy-path data while malformed fields still pass" -> "export gate remains weaker than the spec requires"
## SKELETON
- Proposed interfaces/types/contracts:
  - `src/backend/handshake_core/src/locus/types.rs`: add one shared governed-action registry helper that resolves `WorkflowStateFamily` to emitted `GovernedActionDescriptorV1.action_id` values, and use the same helper path to validate `allowed_action_ids` on Work Packet, Micro-Task, and Task Board record families.
  - `src/backend/handshake_core/src/locus/types.rs`: harden `validate_structured_collaboration_record()` so `allowed_action_ids` fail when malformed or unregistered, and so `RoleMailboxIndexV1.threads[].subject_redacted` plus `RoleMailboxThreadLineV1.transcription_links[].note_redacted` must be bounded single-line Secret-Redactor outputs rather than only non-empty strings.
  - `src/backend/handshake_core/src/workflows.rs`: replace the local ad hoc `allowed_action_ids()` mapper and status-only Task Board projection path so `TrackedWorkPacketArtifactV1`, `TrackedMicroTaskArtifactV1`, and `TaskBoardEntryRecordV1` all emit authoritative `workflow_state_family`, `queue_reason_code`, and governed `allowed_action_ids`.
  - `src/backend/handshake_core/src/storage/locus_sqlite.rs`: remove the duplicate SQLite-only ad hoc `allowed_action_ids()` emitter and route `tracked_mt_progress_metadata()` through the same shared governed-action helper used by the main workflow emitters.
  - `src/backend/handshake_core/src/locus/task_board.rs`: keep `TaskBoardEntryRecordV1` as the typed contract surface for preserved workflow semantics; only touch this file if a narrow helper or serde contract adjustment is required to keep projection rows authoritative instead of heuristic.
  - `src/backend/handshake_core/src/role_mailbox.rs`: keep export emission on the existing redactor path, but harden `validate_runtime_mailbox_record()` so malformed redacted fields are rejected before Role Mailbox index/thread exports are accepted as canonical.
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs`: add mutation-driven negative tests for unregistered `allowed_action_ids` on Work Packet and Micro-Task artifacts plus Task Board workflow-state drift against linked backend truth.
  - `src/backend/handshake_core/tests/role_mailbox_tests.rs`: add negative-path export tests for multiline, oversized, or otherwise malformed `subject_redacted` and `note_redacted` values at the shared mailbox validation boundary.
- Open questions:
  - Whether Task Board sync can lift authoritative workflow semantics directly from linked structured artifacts/metadata, or whether a narrow carry-forward field must be added to avoid recomputing from `TaskBoardStatus`.
  - Whether the shared governed-action helper can live fully in `locus/types.rs` without introducing an unwanted dependency edge into `locus/task_board.rs`.
  - Whether any current mailbox fixtures rely on tolerated redacted-field shapes that will need explicit negative-path coverage rather than emitter changes.
- Notes:
  - Stay within the 7-file touched budget and keep the change limited to governed action id emission/validation, Task Board workflow fidelity, mailbox export-gate hardening, and negative-path proof.
  - Do not reopen the already-closed schema-registry-v4 scope around timestamp/hash typing, nested object presence, or unrelated Loom portability work unless the in-scope code proves a direct regression.
  - Planned implementation order: shared helper/validator hardening first, producer adoption second, Task Board authority preservation third, negative-path tests last, then the packet test plan.

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.
## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: authoritative workflow/runtime records and mailbox export rows -> canonical structured-collaboration JSON artifacts consumed by Task Board, mailbox, and model-routing surfaces
- SERVER_SOURCES_OF_TRUTH:
  - `TrackedWorkPacket` and `TrackedMicroTask` artifact builders in `src/backend/handshake_core/src/workflows.rs`
  - SQLite-backed `tracked_mt_progress_metadata()` in `src/backend/handshake_core/src/storage/locus_sqlite.rs`
  - Task Board projection rows emitted during `locus_sync_task_board_v1`
  - Role Mailbox thread/index export assembly in `src/backend/handshake_core/src/role_mailbox.rs`
- REQUIRED_PROVENANCE_FIELDS:
  - `workflow_state_family`
  - `queue_reason_code`
  - `allowed_action_ids`
  - `authority_refs`
  - `subject_redacted` plus `subject_sha256`
  - `note_redacted` plus `note_sha256`
- VERIFICATION_PLAN:
  - Prove Work Packet, Micro-Task, Task Board, and SQLite-backed progress metadata outputs all source `allowed_action_ids` from one shared governed-action registry helper.
  - Add mutation tests that make `allowed_action_ids` unregistered and assert shared validation fails on Work Packet, Micro-Task, and Task Board records.
  - Add a Task Board drift proof that mismatched `workflow_state_family`, `queue_reason_code`, or `allowed_action_ids` values fail against linked backend truth.
  - Add mailbox mutation tests that introduce multiline, oversized, or otherwise malformed `subject_redacted` and `note_redacted` values and assert the mailbox export validation boundary fails.
  - Run the packet test plan after implementation: schema registry tripwire, task board tripwire, mailbox tripwire, then the full `handshake_core` test suite.
- ERROR_TAXONOMY_PLAN:
  - Treat unregistered governed actions as structured-collaboration validation failures, not UI-only warnings.
  - Treat Task Board projection drift as a record-validation failure at the consumer boundary.
  - Treat malformed redacted mailbox fields as RoleMailbox export-gate failures before file acceptance.
- UI_GUARDRAILS:
  - Backend-only WP; no UI contract changes are planned, and board lane labels must remain downstream display concerns rather than new sources of truth.
- VALIDATOR_ASSERTIONS:
  - Confirm no local ad hoc `allowed_action_ids()` helper survives in `workflows.rs` or `locus_sqlite.rs`.
  - Confirm Task Board rows preserve authoritative workflow semantics rather than recomputing them from `TaskBoardStatus` heuristics alone.
  - Confirm mailbox validation rejects malformed `subject_redacted` and `note_redacted` values while happy-path exports still validate.
## IMPLEMENTATION
- Added governed action registry helpers and shared validation for `workflow_state_family`, `queue_reason_code`, and registered `allowed_action_ids`.
- Preserved authoritative Task Board workflow-state truth at the projection boundary and added explicit row-vs-packet validation.
- Emitted structured Work Packet and Micro-Task packet artifacts instead of writing the raw tracked structs directly, while preserving `profile_extension`.
- Hardened mailbox redaction validation so existing `[REDACTED...]` markers are preserved only when the surrounding single-line text is still secret-free and the marker shape itself is valid.
- Added negative-path tests for unregistered governed action ids, Task Board authoritative drift, malformed mailbox redacted fields, and single-line leak drift around valid redaction markers.

## HYGIENE
- Reproduced the initial packet-emitter mismatch, fixed it, then fixed the follow-on `profile_extension` regression on the Micro-Task artifact path.
- Ran the three scoped tripwire slices first, then reran the full `micro_task_executor_tests` file serially, then reran the full `handshake_core` crate serially with `CARGO_INCREMENTAL=0`.
- Direct-review receipts now include validator kickoff and coder intent on the active kickoff correlation.
- Reproduced the WP-validator mailbox finding against `canonical_redacted_secret_output()`, patched the redaction-marker handling, reran the scoped slices, and reran the full serialized crate proof.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Implementation WP**: Shared structured-collaboration contract hardening across governed action ids, Task Board authoritative workflow truth, mailbox redaction validation, and negative-path proof.

- **Target File**: `src/backend/handshake_core/src/locus/task_board.rs`
- **Start**: 1
- **End**: 336
- **Line Delta**: 74
- **Pre-SHA1**: `bd4a8b681d5fb0793b3e01aedfd7e90082035488`
- **Post-SHA1**: `399a261540d9b90b712a19985d1e6ef874d8f370`
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
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md

- **Target File**: `src/backend/handshake_core/src/locus/types.rs`
- **Start**: 1
- **End**: 2355
- **Line Delta**: 604
- **Pre-SHA1**: `ce48d67cf815ac8bfb8c11184b5b4f301f4750b2`
- **Post-SHA1**: `84fe208cbcd233e0014e1e691309d210f964d3a5`
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
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md

- **Target File**: `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Start**: 1
- **End**: 1055
- **Line Delta**: -16
- **Pre-SHA1**: `a3bdbe81c302f8fdbefd260bff808c12b2181ee8`
- **Post-SHA1**: `fdbd874d806902db2138621902d784f689c17818`
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
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 22660
- **Line Delta**: -11
- **Pre-SHA1**: `a77b1a14aad10787f4fb5b4c8347de5b8ec484e2`
- **Post-SHA1**: `68d88fa6e602b5f6440a14b2e0d98b3769a91713`
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
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md

- **Target File**: `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- **Start**: 1
- **End**: 2505
- **Line Delta**: 212
- **Pre-SHA1**: `0c396ecceeec0e74dc726aaa887e95c9d74d8af5`
- **Post-SHA1**: `d309f54551798fc5e0bf30d833caef96749cd9f8`
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
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md

- **Target File**: `src/backend/handshake_core/tests/role_mailbox_tests.rs`
- **Start**: 1
- **End**: 753
- **Line Delta**: 134
- **Pre-SHA1**: `96adf0cc0f9bb09cd622996d1036772af84c3f99`
- **Post-SHA1**: `5c04a1778e88ff809211050e8fc05f853a8f6ea8`
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
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: Validated (PASS); repair commit `92d9032` cleared the WP-validator finding and the integration-validator merged the final product diff to `main` at `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`.
- What changed in this update: Preserved the earlier governed action registry and Task Board contract hardening, then tightened `canonical_redacted_secret_output()` so valid redaction markers no longer whitelist surrounding leaked single-line text, and extended `role_mailbox_tests.rs` to prove both single-line leak drift and multiline drift are rejected.
- Requirements / clauses self-audited: [ADD v02.171] governed `allowed_action_ids` on canonical Work Packet and Micro-Task records; [ADD v02.171] Task Board workflow-state and queue-reason semantics must remain authoritative; RoleMailbox redacted fields must be bounded Secret-Redactor outputs; malformed mailbox export thread-line field sets must be rejected deterministically.
- Checks actually run: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests -- --test-threads=1` PASS; `CARGO_INCREMENTAL=0 cargo test -j 1 --manifest-path src/backend/handshake_core/Cargo.toml` PASS; prior repair cycle also cleared `just wp-communication-health-check WP-1-Structured-Collaboration-Contract-Hardening-v1 KICKOFF` and `just post-work WP-1-Structured-Collaboration-Contract-Hardening-v1`, and both will be rerun on this repair diff before commit.
- Known gaps / weak spots: No remaining product gap is known inside the signed WP scope. Non-blocking quality debt remains in pre-existing dead-code warnings inside `src/backend/handshake_core/src/workflows.rs`.
- Heuristic risks / maintainability concerns: `locus/types.rs` now carries more contract-specific validation logic; `workflows.rs` and `locus_sqlite.rs` now share more responsibility for keeping emitted packet artifacts aligned with the validator; Task Board truth now depends on preserving packet-derived state rather than recomputing from display heuristics.
- Validator focus request: Verify that governed action ids still come only from the shared registry path, that Task Board rows still fail if their workflow-state triplet drifts from the linked packet truth, and that mailbox redacted fields now reject single-line leaked text wrapped around valid `[REDACTED...]` markers in addition to multiline drift.
- Rubric contract understanding proof: This WP is not satisfied by happy-path emitter success alone; it requires shared validator enforcement, authoritative producer-consumer alignment, and negative-path proof for the exact contract gaps identified in the smoke-test audit.
- Rubric scope discipline proof: No Loom work, no new schema family, no UI changes, and no unrelated runtime/governance redesign were introduced. The diff stayed inside the six approved product files for this packet.
- Rubric baseline comparison: Before the first implementation diff, runtime packet emitters still exposed ad hoc action semantics, Task Board workflow semantics could drift from packet truth, and mailbox redacted-field validation was too weak. The advisory review then found one remaining single-line mailbox leak hole around pre-existing `[REDACTED...]` markers. This repair closes that exact hole without widening scope.
- Rubric end-to-end proof: Scoped tests prove rejected unregistered action ids, rejected Task Board authoritative drift, rejected single-line mailbox leak drift, rejected multiline mailbox drift, and green happy-path emission through the shared validator. The serialized full crate run also passed after the mailbox repair.
- Rubric architecture fit self-review: The contract is enforced at the shared validator boundary and shared producer paths instead of by adding one-off checks only in tests or UI-facing helpers.
- Rubric heuristic quality self-review: The strongest part is the producer-consumer alignment and mutation-based proof. The weakest part is that the packet/handoff governance surface lagged the finished code and had to be repaired after the product proof was already green.
- Rubric anti-gaming / counterfactual check: If `validate_allowed_action_ids` were relaxed, `locus_schema_registry_rejects_unregistered_allowed_action_ids` would stop proving the registry-backed contract. If `validate_task_board_entry_authoritative_fields` or the projection merge were removed, `locus_task_board_validation_reports_authoritative_row_drift` would stop proving Task Board truth preservation. If the new redaction-marker masking and validation were removed, `role_mailbox_validation_reports_redacted_field_drift` would stop proving rejection of single-line mailbox leaks wrapped around valid markers.
- Next step / handoff hint: CLOSED. WP-validator PASS is recorded on correlation `review:WP-1-Structured-Collaboration-Contract-Hardening-v1:validator_kickoff:mn5mpq3g:bd59af`, and the integration-validator PASS + merge is recorded on correlation `review:WP-1-Structured-Collaboration-Contract-Hardening-v1:integration_final:20260325t0709z`.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "[ADD v02.171] canonical Work Packet and Micro-Task records SHALL expose governed `allowed_action_ids` rather than ad hoc verbs"
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:285`, `src/backend/handshake_core/src/locus/types.rs:298`, `src/backend/handshake_core/src/locus/types.rs:1843`, `src/backend/handshake_core/src/workflows.rs:4671`, `src/backend/handshake_core/src/workflows.rs:4734`, `src/backend/handshake_core/src/storage/locus_sqlite.rs:154`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:940`
  - REQUIREMENT: "[ADD v02.171] Task Board rows SHOULD expose portable workflow-state and queue-reason semantics rather than board-status heuristics"
  - EVIDENCE: `src/backend/handshake_core/src/locus/task_board.rs:104`, `src/backend/handshake_core/src/workflows.rs:3441`, `src/backend/handshake_core/src/workflows.rs:3635`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:298`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1026`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1235`
  - REQUIREMENT: "RoleMailboxIndexV1 and RoleMailboxThreadLineV1 redacted fields MUST be bounded Secret-Redactor outputs"
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:2034`, `src/backend/handshake_core/src/locus/types.rs:2062`, `src/backend/handshake_core/src/locus/types.rs:2070`, `src/backend/handshake_core/src/locus/types.rs:2107`, `src/backend/handshake_core/src/locus/types.rs:2132`, `src/backend/handshake_core/src/locus/types.rs:2156`, `src/backend/handshake_core/src/locus/types.rs:2188`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:499`
  - REQUIREMENT: "Mechanical gate (HARD) RoleMailboxExportGate must reject malformed export thread-line field sets"
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1306`, `src/backend/handshake_core/src/locus/types.rs:1323`, `src/backend/handshake_core/src/locus/types.rs:2014`, `src/backend/handshake_core/src/locus/types.rs:2042`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:261`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:406`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:499`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Structured-Collaboration-Contract-Hardening-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry`
- EXIT_CODE: 0
- PROOF_LINES: `test locus_schema_registry_rejects_unregistered_allowed_action_ids ... ok`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board`
- EXIT_CODE: 0
- PROOF_LINES: `test locus_sync_task_board_emits_structured_index_and_view ... ok`; `test locus_task_board_validation_reports_authoritative_row_drift ... ok`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox`
- EXIT_CODE: 0
- PROOF_LINES: `test role_mailbox_index_api_returns_valid_structured_export ... ok`; `test role_mailbox_validation_reports_redacted_field_drift ... ok`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests -- --test-threads=1`
- EXIT_CODE: 0
- PROOF_LINES: `test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`

- COMMAND: `CARGO_INCREMENTAL=0 cargo test -j 1 --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: 0
- PROOF_LINES: `Finished test profile [unoptimized + debuginfo] target(s) in 8m 57s`; `Doc-tests handshake_core`

- COMMAND: `just post-work WP-1-Structured-Collaboration-Contract-Hardening-v1`
- EXIT_CODE: 0
- PROOF_LINES: `Post-work validation PASSED (deterministic manifest gate; not tests) with warnings`; `You may proceed with commit.`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox`
- EXIT_CODE: 0
- PROOF_LINES: `test role_mailbox_validation_reports_redacted_field_drift ... ok`; `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry`
- EXIT_CODE: 0
- PROOF_LINES: `test locus_schema_registry_rejects_unregistered_allowed_action_ids ... ok`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board`
- EXIT_CODE: 0
- PROOF_LINES: `test locus_sync_task_board_emits_structured_index_and_view ... ok`; `test locus_task_board_validation_reports_authoritative_row_drift ... ok`

- COMMAND: `CARGO_INCREMENTAL=0 cargo test -j 1 --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: 0
- PROOF_LINES: `test role_mailbox_validation_reports_redacted_field_drift ... ok`; `test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`; `Doc-tests handshake_core`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
  - `SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED`
- For `PACKET_FORMAT_VERSION >= 2026-03-22`, every appended governed validation report MUST also include these completion-layer fields:
  - `WORKFLOW_VALIDITY: VALID | INVALID | PARTIAL | BLOCKED | NOT_RUN`
  - `SCOPE_VALIDITY: IN_SCOPE | OUT_OF_SCOPE | PARTIAL | BLOCKED | NOT_RUN`
  - `PROOF_COMPLETENESS: PROVEN | NOT_PROVEN | PARTIAL | BLOCKED | NOT_RUN`
  - `INTEGRATION_READINESS: READY | NOT_READY | PARTIAL | BLOCKED | NOT_RUN`
  - `DOMAIN_GOAL_COMPLETION: COMPLETE | INCOMPLETE | PARTIAL | BLOCKED | NOT_RUN`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST also include:
  - `CLAUSES_REVIEWED:`
    - one bullet per in-scope MUST/SHOULD clause reviewed, each with file:line evidence or an explicit proof note
    - when `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`, reuse the exact clause text from `CLAUSE_CLOSURE_MATRIX`
  - `NOT_PROVEN:`
    - `- NONE` only when nothing remains unproven
    - otherwise list each unresolved clause/gap explicitly
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`, every appended governed validation report MUST also include:
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
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`
  - `NEGATIVE_PROOF:`
    - list at least one spec requirement the validator verified is NOT fully implemented
    - this proves the validator independently read the code rather than trusting coder summaries
    - `- NONE` is illegal; every codebase has at least one gap or partial implementation
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Rule: `HEURISTIC_REVIEW_VERDICT=PASS` is only valid when `QUALITY_RISKS` is exactly `- NONE`.
- Rule: `LEGAL_VERDICT=PASS` is only valid when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, `COUNTERFACTUAL_CHECKS`, and `SPEC_CLAUSE_MAP` are all present and non-empty, and `SPEC_CLAUSE_MAP` entries include file:line evidence.
- Rule: `Verdict: PASS` is legal only when `VALIDATION_CONTEXT=OK`, `WORKFLOW_VALIDITY=VALID`, `SCOPE_VALIDITY=IN_SCOPE`, `PROOF_COMPLETENESS=PROVEN`, `INTEGRATION_READINESS=READY`, `DOMAIN_GOAL_COMPLETION=COMPLETE`, and `LEGAL_VERDICT=PASS`.
- Rule: if `PROOF_COMPLETENESS` is anything other than `PROVEN`, the top-line `Verdict` MUST NOT be `PASS`; use `NOT_PROVEN`, `FAIL`, `BLOCKED`, or `OUTDATED_ONLY` honestly.
- Rule: `PROOF_COMPLETENESS=PROVEN` is legal only when `NOT_PROVEN` is exactly `- NONE`.
- Rule: `WORKFLOW_VALIDITY=VALID` is legal only when `VALIDATION_CONTEXT=OK` and `GOVERNANCE_VERDICT=PASS`.
- Rule: `LEGAL_VERDICT=PASS` is legal only when `PROOF_COMPLETENESS=PROVEN`.
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.

VALIDATION REPORT - WP-1-Structured-Collaboration-Contract-Hardening-v1
Verdict: PASS
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PASS
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PASS
HEURISTIC_REVIEW_VERDICT: PASS
SPEC_ALIGNMENT_VERDICT: PASS
ENVIRONMENT_VERDICT: PASS
DISPOSITION: NONE
LEGAL_VERDICT: PASS
SPEC_CONFIDENCE: POST_MERGE_RECHECKED
WORKFLOW_VALIDITY: VALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: PROVEN
INTEGRATION_READINESS: READY
DOMAIN_GOAL_COMPLETION: COMPLETE
VALIDATOR_RISK_TIER: HIGH

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Structured-Collaboration-Contract-Hardening-v1/packet.md` (status at review time: `In Progress`)
- Spec: `Handshake_Master_Spec_v02.178.md`
- Reviewed diff: `e65516eec1063383a2f268e82fd22b53a4bc49ae..92d9032f497aa47cd0a8cb56df57e21d86a96c7f`
- Merged main commit: `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`
- Review receipts: `review:WP-1-Structured-Collaboration-Contract-Hardening-v1:validator_kickoff:mn5mpq3g:bd59af`; `review:WP-1-Structured-Collaboration-Contract-Hardening-v1:integration_final:20260325t0709z`

CLAUSES_REVIEWED:
- `[ADD v02.171] canonical Work Packet and Micro-Task records SHALL expose governed `allowed_action_ids` rather than ad hoc verbs` -> `src/backend/handshake_core/src/locus/types.rs:227`; `src/backend/handshake_core/src/locus/types.rs:285`; `src/backend/handshake_core/src/locus/types.rs:299`; `src/backend/handshake_core/src/locus/types.rs:1243`; `src/backend/handshake_core/src/locus/types.rs:1843`; `src/backend/handshake_core/src/locus/types.rs:1891`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:155`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:182`; `src/backend/handshake_core/src/workflows.rs:4676`; `src/backend/handshake_core/src/workflows.rs:4691`; `src/backend/handshake_core/src/workflows.rs:4738`; `src/backend/handshake_core/src/workflows.rs:4753`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:940`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:976`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1019`
- `[ADD v02.171] Task Board rows SHOULD expose portable workflow-state and queue-reason semantics rather than board-status heuristics` -> `src/backend/handshake_core/src/locus/task_board.rs:104`; `src/backend/handshake_core/src/locus/task_board.rs:137`; `src/backend/handshake_core/src/locus/task_board.rs:153`; `src/backend/handshake_core/src/locus/task_board.rs:169`; `src/backend/handshake_core/src/workflows.rs:3531`; `src/backend/handshake_core/src/workflows.rs:3547`; `src/backend/handshake_core/src/workflows.rs:3561`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1177`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1181`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1185`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1275`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1283`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1287`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1291`
- `RoleMailboxIndexV1 and RoleMailboxThreadLineV1 redacted fields MUST be bounded Secret-Redactor outputs` -> `src/backend/handshake_core/src/role_mailbox.rs:290`; `src/backend/handshake_core/src/role_mailbox.rs:727`; `src/backend/handshake_core/src/role_mailbox.rs:1003`; `src/backend/handshake_core/src/role_mailbox.rs:1101`; `src/backend/handshake_core/src/locus/types.rs:2034`; `src/backend/handshake_core/src/locus/types.rs:2062`; `src/backend/handshake_core/src/locus/types.rs:2070`; `src/backend/handshake_core/src/locus/types.rs:2107`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:499`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:553`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:567`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:590`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:604`
- `Mechanical gate (HARD) RoleMailboxExportGate must reject malformed export thread-line field sets` -> `src/backend/handshake_core/src/role_mailbox.rs:177`; `src/backend/handshake_core/src/role_mailbox.rs:533`; `src/backend/handshake_core/src/locus/types.rs:1319`; `src/backend/handshake_core/src/locus/types.rs:1323`; `src/backend/handshake_core/src/locus/types.rs:2042`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:261`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:406`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:499`

NOT_PROVEN:
- NONE

MAIN_BODY_GAPS:
- NONE

QUALITY_RISKS:
- NONE

DIFF_ATTACK_SURFACES:
- Producer/validator drift between workflow emitters, SQLite micro-task progress metadata, and the shared `allowed_action_ids` legality checks.
- Task Board row drift where projected `workflow_state_family`, `queue_reason_code`, or `allowed_action_ids` diverge from authoritative work-packet truth.
- Mailbox redaction acceptance of leak-after-token or multiline malformed values that still look superficially redacted.
- Thread-line export gate acceptance of malformed `transcription_links` or missing required transcription-link payloads.

INDEPENDENT_CHECKS_RUN:
- `git -C ../handshake_main rev-parse HEAD` => `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe` on `main`, matching the integration-validator receipt.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests locus_schema_registry_rejects_unregistered_allowed_action_ids` from `../handshake_main` => `test locus_schema_registry_rejects_unregistered_allowed_action_ids ... ok`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests locus_task_board_validation_reports_authoritative_row_drift` from `../handshake_main` => `test locus_task_board_validation_reports_authoritative_row_drift ... ok`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox_validation_reports_redacted_field_drift` from `../handshake_main` => `test role_mailbox_validation_reports_redacted_field_drift ... ok`

COUNTERFACTUAL_CHECKS:
- If `validate_allowed_action_ids()` in `src/backend/handshake_core/src/locus/types.rs:1843` stopped deriving its accepted set from `governed_action_descriptors_for_workflow_family()` in `src/backend/handshake_core/src/locus/types.rs:285`, the unregistered action-id mutations in `src/backend/handshake_core/tests/micro_task_executor_tests.rs:967` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1009` would stop failing.
- If `validate_task_board_entry_authoritative_fields()` in `src/backend/handshake_core/src/locus/task_board.rs:104` stopped checking `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`, the authoritative-row drift mutations at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1275-1291` would no longer prove Task Board truth preservation.
- If `canonical_redacted_secret_output()` in `src/backend/handshake_core/src/locus/types.rs:2107` stopped masking and restoring valid redaction markers before redaction, the single-line leak-after-token mutations at `src/backend/handshake_core/tests/role_mailbox_tests.rs:553` and `src/backend/handshake_core/tests/role_mailbox_tests.rs:590` would pass again.

BOUNDARY_PROBES:
- Storage/producer probe: `src/backend/handshake_core/src/workflows.rs:4676-4753` and `src/backend/handshake_core/src/storage/locus_sqlite.rs:155-182` both now emit `allowed_action_ids` through the same governed-action registry path that `src/backend/handshake_core/src/locus/types.rs:1843-1908` validates.
- Projection probe: `src/backend/handshake_core/src/workflows.rs:3531-3561` populates Task Board rows from authoritative workflow-state triplets, and `src/backend/handshake_core/src/locus/task_board.rs:104-169` rejects drift at the row boundary.
- Mailbox boundary probe: `src/backend/handshake_core/src/role_mailbox.rs:290`, `src/backend/handshake_core/src/role_mailbox.rs:727`, `src/backend/handshake_core/src/role_mailbox.rs:1003`, and `src/backend/handshake_core/src/role_mailbox.rs:1101` emit bounded redacted fields that `src/backend/handshake_core/src/locus/types.rs:2034-2188` re-canonicalizes and validates.

NEGATIVE_PATH_CHECKS:
- Unregistered `allowed_action_ids` mutations are rejected for both Work Packet and Micro-Task artifacts at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:967-1019`.
- Task Board row mutations that drift `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` are rejected at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1275-1291`.
- Role Mailbox single-line leak-after-token and multiline redacted-field mutations are rejected at `src/backend/handshake_core/tests/role_mailbox_tests.rs:553-604`.
- Role Mailbox export-gate malformed thread-line payloads and missing required transcription links are rejected at `src/backend/handshake_core/tests/role_mailbox_tests.rs:261-420`.

INDEPENDENT_FINDINGS:
- The merged `main` code already closes the refinement-era governed-action gap: canonical emitters, the SQLite progress path, and the shared validator all use the registry-backed action-id contract.
- The repaired mailbox path does not merely reject multiline drift; it now also rejects leaked single-line text wrapped around valid `[REDACTED...]` markers.
- The final integrated tree on `main` matches the intended WP contract surface, and no extra product-file drift beyond the approved contract-hardening slice was needed for the targeted proof.

RESIDUAL_UNCERTAINTY:
- This closeout reran the three contract-hardening tripwire tests on `main`, but it did not rerun the entire `handshake_core` test suite from the merge-authority tree during status sync.
- Dead-code warnings in `src/backend/handshake_core/src/workflows.rs:2573-3271` remain outside this WP's correctness scope and should be cleaned up separately.

SPEC_CLAUSE_MAP:
- `[ADD v02.171] canonical Work Packet and Micro-Task records SHALL expose governed `allowed_action_ids` rather than ad hoc verbs` -> `src/backend/handshake_core/src/locus/types.rs:227`; `src/backend/handshake_core/src/locus/types.rs:285`; `src/backend/handshake_core/src/locus/types.rs:299`; `src/backend/handshake_core/src/locus/types.rs:1843`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:155`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:182`; `src/backend/handshake_core/src/workflows.rs:4676`; `src/backend/handshake_core/src/workflows.rs:4738`
- `[ADD v02.171] Task Board rows SHOULD expose portable workflow-state and queue-reason semantics rather than board-status heuristics` -> `src/backend/handshake_core/src/locus/task_board.rs:104`; `src/backend/handshake_core/src/locus/task_board.rs:169`; `src/backend/handshake_core/src/workflows.rs:3531`; `src/backend/handshake_core/src/workflows.rs:3561`
- `RoleMailboxIndexV1 and RoleMailboxThreadLineV1 redacted fields MUST be bounded Secret-Redactor outputs` -> `src/backend/handshake_core/src/role_mailbox.rs:290`; `src/backend/handshake_core/src/role_mailbox.rs:1003`; `src/backend/handshake_core/src/role_mailbox.rs:1101`; `src/backend/handshake_core/src/locus/types.rs:2034`; `src/backend/handshake_core/src/locus/types.rs:2107`
- `Mechanical gate (HARD) RoleMailboxExportGate must reject malformed export thread-line field sets` -> `src/backend/handshake_core/src/role_mailbox.rs:177`; `src/backend/handshake_core/src/role_mailbox.rs:533`; `src/backend/handshake_core/src/locus/types.rs:1319`; `src/backend/handshake_core/src/locus/types.rs:2042`

NEGATIVE_PROOF:
- The broader spec SHOULD that every state-changing operator or model action resolve through a registered `GovernedActionDescriptorV1` is still not fully implemented. `src/backend/handshake_core/src/workflows.rs:3172` and `src/backend/handshake_core/src/workflows.rs:3194` still synthesize prose `next_action` strings (`"triage work packet"`, `"start the next iteration"`, etc.) instead of resolving through the governed-action descriptor registry rooted at `src/backend/handshake_core/src/locus/types.rs:227-299`. That gap is outside this packet's signed scope, but it is a real remaining spec delta.

REASON FOR PASS:
- The signed WP scope is closed on the reviewed diff `e65516eec1063383a2f268e82fd22b53a4bc49ae..92d9032f497aa47cd0a8cb56df57e21d86a96c7f`, the integration-validator merged the final product change to `main` at `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`, and the post-merge tripwire tests for governed action-id legality, Task Board authoritative-row fidelity, and leak-safe mailbox export validation all passed from `main`.
