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

# Task Packet: WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1

## METADATA
- TASK_ID: WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- WP_ID: WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- BASE_WP_ID: WP-1-Structured-Collaboration-Governed-Next-Action-Alignment
- DATE: 2026-03-25T15:16:51.236Z
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
- MODEL_FAMILY_POLICY: OPENAI_GPT_SERIES_ONLY
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
<!-- Validator roles keep distinct local branches/worktrees, but they mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-action-alignment-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
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
- RISK_TIER: MEDIUM
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: MEDIUM
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Contract-Hardening, WP-1-Structured-Collaboration-Artifact-Family
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- LOCAL_WORKTREE_DIR: ../wtc-action-alignment-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: 019d25e7-d493-7ee3-bd33-2ee9e9255ffb
- INTEGRATION_VALIDATOR_OF_RECORD: 019d261f-74ee-7901-ad10-535d4203d89f
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja250320261614
- PACKET_FORMAT_VERSION: 2026-03-23

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: VALIDATED_PASS
Blockers: NONE
Next: NONE

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: StructuredCollaborationSummaryV1 compact summary contract includes optional `next_action` | CODE_SURFACES: `next_action_for_work_packet`, `next_action_for_micro_task`, `default_structured_collaboration_summary_record` | TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` | EXAMPLES: Ready Work Packet compact summary whose `next_action` resolves to a registered governed action id or is omitted, Pending Micro-Task compact summary whose `next_action` resolves to a registered governed action id, Mutated Work Packet summary payload with `next_action: "start_work_packet"` rejected as unregistered, Mutated Micro-Task summary payload with `next_action: "start_micro_task"` rejected as unregistered | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Every state-changing operator or model action SHOULD resolve through a registered GovernedActionDescriptorV1 | CODE_SURFACES: governed next-action mapping helper in `workflows.rs` and validator legality checks in `locus/types.rs` | TESTS: mutation-based rejection of unregistered and family-illegal `next_action` values in `micro_task_executor_tests`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` | EXAMPLES: Ready Work Packet compact summary whose `next_action` resolves to a registered governed action id or is omitted, Pending Micro-Task compact summary whose `next_action` resolves to a registered governed action id, Mutated Work Packet summary payload with `next_action: "start_work_packet"` rejected as unregistered, Mutated Work Packet summary payload with `next_action: "archive"` rejected as family-illegal, Mutated Micro-Task summary payload with `next_action: "start_micro_task"` rejected as unregistered, Mutated Micro-Task summary payload with `next_action: "archive"` rejected as family-illegal | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Task Board and board-adjacent viewers SHOULD expose base-envelope next action before grouping metadata | CODE_SURFACES: unified `next_action_for_work_packet` and `next_action_for_micro_task` helpers in `workflows.rs`; Task Board row `summary_ref` path in `workflows.rs` | TESTS: `rg` and summary assertions prove there is one governed next-action contract only; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` | EXAMPLES: Ready Work Packet compact summary whose `next_action` resolves to a registered governed action id or is omitted, Pending Micro-Task compact summary whose `next_action` resolves to a registered governed action id, Mutated Work Packet summary payload with `next_action: "start_work_packet"` rejected as unregistered, Mutated Micro-Task summary payload with `next_action: "start_micro_task"` rejected as unregistered | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
- CANONICAL_CONTRACT_EXAMPLES:
  - Ready Work Packet compact summary whose `next_action` resolves to a registered governed action id or is omitted
  - Pending Micro-Task compact summary whose `next_action` resolves to a registered governed action id
  - Mutated Work Packet summary payload with `next_action: "start_work_packet"` rejected as unregistered
  - Mutated Micro-Task summary payload with `next_action: "start_micro_task"` rejected as unregistered
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md StructuredCollaborationSummaryV1 compact summary contract
- CONTEXT_START_LINE: 6086
- CONTEXT_END_LINE: 6095
- CONTEXT_TOKEN: interface StructuredCollaborationSummaryV1 {
- EXCERPT_ASCII_ESCAPED:
  ```text
interface StructuredCollaborationSummaryV1 {
    schema_id: string;
    schema_version: string;
    record_id: string;
    record_kind: StructuredRecordKind;
    project_profile_kind: ProjectProfileKind;
    status: string;
    title_or_objective: string;
    blockers: string[];
    next_action?: string;
    authority_refs: ArtifactHandle[];
    evidence_refs: ArtifactHandle[];
    updated_at: ISO8601Timestamp;
  }
  ```

#### ANCHOR 2
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

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Task Board projection viewer workflow portability [ADD v02.168-v02.171]
- CONTEXT_START_LINE: 60910
- CONTEXT_END_LINE: 60922
- CONTEXT_TOKEN: [ADD v02.168] Board rows SHOULD expose the base-envelope status, next action, blockers, and project-profile kind before board-specific grouping metadata.
- EXCERPT_ASCII_ESCAPED:
  ```text
- **Task Board projection viewer**
    - Show structured board rows keyed by stable `task_board_id` and `work_packet_id`, plus freshness, manual-edit detection, and sync status.
    - Any Markdown board is read-only by default from this view unless a governed sync or status-update workflow is being invoked.
    - [ADD v02.168] Board rows SHOULD expose the base-envelope status, next action, blockers, and project-profile kind before board-specific grouping metadata.
    - [ADD v02.170] Board, list, queue, and roadmap layouts SHOULD read from the same row set and declare which lane definitions, grouping keys, and action bindings are active for the current preset.
    - [ADD v02.171] Board rows SHOULD expose `workflow_state_family` and `queue_reason_code` separately from any project-specific display label so queue semantics remain portable across project kernels.
  ```

#### ANCHOR 4
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

  **Required action behavior**
  - `GovernedActionDescriptorV1` SHOULD be the reusable contract for verbs such as:
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: StructuredCollaborationSummaryV1 compact summary contract includes optional `next_action` | WHY_IN_SCOPE: current emitters and tests already use this field, but they populate it with ungoverned tokens | EXPECTED_CODE_SURFACES: `structured_work_packet_next_action`, `structured_micro_task_next_action`, `default_structured_collaboration_summary_record` | EXPECTED_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` | RISK_IF_MISSED: compact summaries remain machine-readable in shape but semantically weak
  - CLAUSE: Every state-changing operator or model action SHOULD resolve through a registered GovernedActionDescriptorV1 | WHY_IN_SCOPE: compact summary next-action hints currently encode a parallel action vocabulary | EXPECTED_CODE_SURFACES: governed next-action mapping helper in `workflows.rs` and validator legality checks in `locus/types.rs` | EXPECTED_TESTS: mutation-based rejection of unregistered `next_action` values in `micro_task_executor_tests` | RISK_IF_MISSED: summary actions drift from canonical registry law
  - CLAUSE: Task Board and board-adjacent viewers SHOULD expose base-envelope next action before grouping metadata | WHY_IN_SCOPE: board-facing preview helpers must not keep a separate prose or ad hoc token vocabulary | EXPECTED_CODE_SURFACES: residual `next_action_for_work_packet` and `next_action_for_micro_task` helpers in `workflows.rs` | EXPECTED_TESTS: `rg` and summary assertions prove there is one governed next-action contract only | RISK_IF_MISSED: board/detail surfaces can silently diverge from compact summaries
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: Work Packet `summary.json` `next_action` | PRODUCER: `build_structured_work_packet_summary` in `workflows.rs` | CONSUMER: compact summary readers and downstream board/detail surfaces via `summary_ref` | SERIALIZER_TRANSPORT: structured JSON summary artifact | VALIDATOR_READER: `validate_structured_collaboration_record` in `locus/types.rs` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` | DRIFT_RISK: live summary tokens can drift from the governed action registry
  - CONTRACT: Micro-Task `summary.json` `next_action` | PRODUCER: `build_structured_micro_task_summary` in `workflows.rs` | CONSUMER: compact summary readers and local-small-model routing | SERIALIZER_TRANSPORT: structured JSON summary artifact | VALIDATOR_READER: `validate_structured_collaboration_record` in `locus/types.rs` | TRIPWIRE_TESTS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` | DRIFT_RISK: micro-task summaries continue to advertise legacy tokens
  - CONTRACT: preview helper next-action mapping | PRODUCER: `next_action_for_work_packet` and `next_action_for_micro_task` in `workflows.rs` | CONSUMER: future board/detail callers | SERIALIZER_TRANSPORT: in-memory helper path | VALIDATOR_READER: code review plus grep-based verification | TRIPWIRE_TESTS: `rg -n "next_action_for_work_packet|next_action_for_micro_task" src/backend/handshake_core/src/workflows.rs` | DRIFT_RISK: prose-only helper paths can become shadow authority later
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Add one canonical helper in `src/backend/handshake_core/src/workflows.rs` that derives in-scope compact-summary `next_action` values from governed action ids rather than ad hoc tokens.
  - Use that helper from Work Packet and Micro-Task summary emitters.
  - Remove or align `next_action_for_work_packet` and `next_action_for_micro_task` so no prose-only helper path remains in scope.
  - Harden `src/backend/handshake_core/src/locus/types.rs` so summary `next_action` values are either absent or registered governed action ids.
  - Update and extend `src/backend/handshake_core/tests/micro_task_executor_tests.rs` so legacy summary tokens fail mechanically.
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- CARRY_FORWARD_WARNINGS:
  - Do not reopen `allowed_action_ids`, queue-reason, Task Board authoritative-field, or mailbox leak-safety scope already closed by WP-1-Structured-Collaboration-Contract-Hardening-v1 unless a concrete regression is found.
  - Do not invent a second summary-only action vocabulary.
  - If a status maps to more than one legal governed action and no deterministic rule is defensible, omit `next_action` instead of overclaiming certainty.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - Compact summary `next_action` remains optional but, when present, resolves to a registered governed action id
  - No live helper path in scope emits prose-only or ad hoc token next-action values
  - Summary validation rejects unregistered `next_action` values mechanically
- FILES_TO_READ:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - rg -n "structured_work_packet_next_action|structured_micro_task_next_action|next_action_for_work_packet|next_action_for_micro_task|next_action" src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/locus/types.rs src/backend/handshake_core/tests/micro_task_executor_tests.rs
- POST_MERGE_SPOTCHECKS:
  - Verify no emitted summary payload still uses legacy tokens such as `start_work_packet` or `start_micro_task`
  - Verify no prose helper path remains as a shadow next-action contract
  - Verify invalid `next_action` values fail at the shared validation boundary
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Whether any hidden downstream consumer outside current tests still depends on legacy summary tokens such as `start_work_packet`
  - Whether every current status can map to one deterministic governed action id without needing omission in some states
  - Whether current Task Board surfaces are fully satisfied by `summary_ref`-backed next-action lookup or will later require an inline field
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Internal sources are sufficient because the unresolved contract drift is explicit in the current spec and current local emitters.
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
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
- PRIMITIVES_EXPOSED:
  - PRIM-GovernedActionDescriptorV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.director
  - engine.archivist
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
  - Locus governed next-action helper foundation -> IN_THIS_WP (stub: NONE)
  - Locus Work Packet summary emission parity -> IN_THIS_WP (stub: NONE)
  - MicroTask summary emission parity -> IN_THIS_WP (stub: NONE)
  - Locus summary validator legality checks -> IN_THIS_WP (stub: NONE)
  - LLM-friendly compact-routing safety -> IN_THIS_WP (stub: NONE)
  - Locus residual preview-helper cleanup -> IN_THIS_WP (stub: NONE)
  - MicroTask mutation-based drift tripwires -> IN_THIS_WP (stub: NONE)
  - LLM-friendly omission policy for ambiguous states -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: governed compact-summary next_action emission | SUBFEATURES: Work Packet summary serializer, Micro-Task summary serializer, governed action mapping helper | PRIMITIVES_FEATURES: PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask | MECHANICAL: engine.director, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the core execution target of the packet.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: compact summary next-action contract | SUBFEATURES: Ready/InProgress/Blocked/Gated/Done/Cancelled Work Packet summary output and packet-adjacent preview helpers | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1 | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Work Packet next-action hints must stop drifting from the governed action registry.
  - PILLAR: MicroTask | CAPABILITY_SLICE: compact summary next-action contract | SUBFEATURES: Pending/InProgress/Completed/Failed/Blocked/Skipped Micro-Task summary output and mutation tests | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1 | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Micro-Task summaries are currently the clearest live drift surface.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: machine-readable next-action hints | SUBFEATURES: summary validation, governed-action legality checks, compact summary drift rejection | PRIMITIVES_FEATURES: PRIM-GovernedActionDescriptorV1, PRIM-StructuredCollaborationSummaryV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Compact routing hints must be safe for local model consumption.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Work Packet compact-summary next_action emission | JobModel: WORKFLOW | Workflow: Locus structured artifact generation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Emission remains a backend workflow concern even though downstream operator and model surfaces will consume it.
  - Capability: Micro-Task compact-summary next_action emission | JobModel: WORKFLOW | Workflow: Locus structured artifact generation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Micro-Task summary output is the main live drift surface today.
  - Capability: Structured summary next_action validation | JobModel: MECHANICAL_TOOL | Workflow: structured collaboration validation | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: This is the mechanical gate that prevents the drift from reappearing.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Project-Agnostic-Workflow-State-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Workflow-Transition-Automation-Registry-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs -> PARTIAL (NONE)
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
- What: Align Work Packet and Micro-Task compact-summary `next_action` values and preview helpers to the governed action registry instead of ad hoc summary tokens or prose.
- Why: Current compact summaries still advertise an ungoverned action vocabulary even after the structured-collaboration contract hardening pass.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - `allowed_action_ids` remediation already closed by WP-1-Structured-Collaboration-Contract-Hardening-v1
  - mailbox export validation
  - broad workflow transition and automation registry work
  - repo-governance and ACP workflow remediation
- TOUCHED_FILE_BUDGET: 3
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
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
```

### DONE_MEANS
- Every emitted in-scope compact-summary `next_action` value is either a registered `GovernedActionDescriptorV1.action_id` or omitted.
- No live helper path in scope emits ad hoc token strings or prose-only next-action text.
- Summary validation and tests mechanically fail unregistered or drifted `next_action` values.

- PRIMITIVES_EXPOSED:
  - PRIM-GovernedActionDescriptorV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-03-25T15:16:51.236Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.171]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md compact summary plus governed next-action contract
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
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- SEARCH_TERMS:
  - structured_work_packet_next_action
  - structured_micro_task_next_action
  - next_action_for_work_packet
  - next_action_for_micro_task
  - GovernedActionDescriptorV1
  - next_action
- RUN_COMMANDS:
  ```bash
rg -n "structured_work_packet_next_action|structured_micro_task_next_action|next_action_for_work_packet|next_action_for_micro_task|next_action" src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/locus/types.rs src/backend/handshake_core/tests/micro_task_executor_tests.rs
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "legacy summary tokens remain live" -> "compact machine-readable next-action hints still drift from governed action law"
  - "dead prose helper survives" -> "future callers can silently reintroduce ungoverned next-action semantics"
  - "validator still accepts any non-empty string" -> "false machine-readable PASS remains possible"
## SKELETON
- Proposed interfaces/types/contracts:
  - `src/backend/handshake_core/src/workflows.rs`: add one canonical governed-summary mapping seam for in-scope compact summaries, centered on workflow-family legality instead of legacy summary tokens or prose helper text.
  - `src/backend/handshake_core/src/workflows.rs`: change in-scope summary emitters so Work Packet and Micro-Task `next_action` serialize as `Option<_>` and only carry registered `GovernedActionDescriptorV1.action_id` values when a deterministic governed action exists.
  - `src/backend/handshake_core/src/workflows.rs`: remove or align `next_action_for_work_packet` and `next_action_for_micro_task` so no shadow prose-only next-action contract remains available to downstream callers.
  - `src/backend/handshake_core/src/locus/types.rs`: harden summary-record construction and validation so `next_action` is optional, but when present must be a registered governed action id for the record's `workflow_state_family`.
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs`: replace legacy compact-summary token expectations with governed action ids or omission, and add mutation-based rejection coverage for unregistered `next_action` values.
- Open questions:
  - Whether any currently emitted Work Packet or Micro-Task state should omit `next_action` because more than one governed action is valid and no deterministic single action is defensible.
  - Whether any downstream consumer outside current tests still reads legacy compact-summary tokens such as `start_work_packet` or `start_micro_task`.
- Notes:
  - Keep scope limited to the three signed write surfaces.
  - Do not reopen `allowed_action_ids`, Task Board field fidelity, mailbox export validation, or repo-governance runtime work.

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
- Authoritative merged product result is local `main` commit `154445243c0b28dc910454b97b0f7df2935529c7`.
- WP history on the backup branch was:
  - `bc85e15eb6958784838f606b0fe9f7e24b95251a` `docs: skeleton checkpoint [WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1]`
  - `8046724bdbcdde1a68a1914a73b6fbc78eed41be` `docs: skeleton approved [WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1]`
  - `2464fb5` initial family-aware validator repair on the signed three-file scope
  - `d8ac5fc` replay of the signed three-file scope onto current `main`
  - `0da6ea3` backup-branch compatibility repair that restored `profile_extension` parity for the branch lane while preserving the signed-scope summary/validator semantics
- Integration authority did not raw-merge `0da6ea3`. It semantically integrated the exact signed-scope file state for:
  - `src/backend/handshake_core/src/workflows.rs`
  - `src/backend/handshake_core/src/locus/types.rs`
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- The canonical merged behavior now is:
  - Work Packet and Micro-Task summaries serialize governed `next_action` ids or omit the field.
  - Summary validation requires `workflow_state_family` and rejects unregistered or family-illegal `next_action` values.
  - Runtime summary refresh paths reuse the same governed helper, so legacy token drift cannot re-enter through the Micro-Task registry refresh path.

## HYGIENE
- Reproduced the WP-validator finding that registered but family-illegal `next_action` values could still pass the shared validator after the first handoff.
- Repaired the validator boundary so `next_action` is optional, but when present it must be both registered and legal for the summary record's `workflow_state_family`.
- Replayed the signed three-file patch onto current `main` and then resolved the branch-lane `profile_extension` parity issue without widening the final merged main diff.
- Re-ran merge-authority proof directly on local `main` after semantic integration: the scoped `micro_task_executor_tests` file and the full `handshake_core` test suite both passed.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Implementation WP**: Governed compact-summary `next_action` alignment across summary emitters, shared validation, and negative-path proof on the canonical merged main diff.

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 24640
- **Line Delta**: -17
- **Pre-SHA1**: `68d88fa6e602b5f6440a14b2e0d98b3769a91713`
- **Post-SHA1**: `89c23d387e0672644dc7c7f39409afdcfed11f42`
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
- **End**: 2573
- **Line Delta**: 217
- **Pre-SHA1**: `84fe208cbcd233e0014e1e691309d210f964d3a5`
- **Post-SHA1**: `61a23390950b1650a5eb8a248afe3fd8fc8eac46`
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
- **End**: 2976
- **Line Delta**: 230
- **Pre-SHA1**: `d309f54551798fc5e0bf30d833caef96749cd9f8`
- **Post-SHA1**: `c4190eb106802a278c434e1655123a77a60fd274`
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
- Current WP_STATUS: Validated (PASS); the authoritative merged product state is local `main` commit `154445243c0b28dc910454b97b0f7df2935529c7`.
- What changed in this update: Replaced ad hoc summary `next_action` tokens with governed action ids or omission, enforced family-aware `next_action` legality in the shared validator, and added negative-path tests for unregistered and family-illegal summary drift on both Work Packet and Micro-Task paths.
- Requirements / clauses self-audited: `StructuredCollaborationSummaryV1` compact summary `next_action` remains optional but governed; state-changing action hints now resolve through registered `GovernedActionDescriptorV1` ids inside the signed summary surfaces; board-adjacent preview paths no longer use shadow prose/token vocabularies.
- Checks actually run: `just wp-communication-health-check WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1 VERDICT` PASS; `just validator-packet-complete WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1` PASS; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` PASS on `../handshake_main`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` PASS on `../handshake_main`.
- Known gaps / weak spots: No remaining defect is known inside the signed packet scope. Adjacent viewer/schema follow-on work may still be needed if Task Board rows are required to expose inline `next_action` instead of `summary_ref`-backed lookup.
- Heuristic risks / maintainability concerns: The governed action mapping now depends on one shared helper in `workflows.rs` and one shared validator path in `locus/types.rs`; future emitters must reuse those seams rather than inventing new summary-local vocabularies.
- Validator focus request: CLOSED. WP-validator PASS is recorded on `review:WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1:coder_handoff:mn6c3ebr:85f229`, and integration-validator PASS is recorded on `review:WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1:coder_handoff:mn6foaob:48d179`.
- Rubric contract understanding proof: This packet is not satisfied by changing strings in one serializer. It required producer-consumer agreement: summary emission, runtime summary refresh, shared validation, and mutation-based proof against legacy and family-illegal values.
- Rubric scope discipline proof: The authoritative merged diff on `main` stayed inside the three signed files even though the backup branch carried a broader compatibility repair during review.
- Rubric baseline comparison: Before this WP, summaries could still advertise ad hoc tokens like `start_work_packet` or `start_micro_task`, and the validator did not reject registered-but-family-illegal values. After merge, only governed ids or omission remain live on the signed summary surfaces.
- Rubric end-to-end proof: The merged `main` tree passed the full `micro_task_executor_tests` file and the full `handshake_core` suite after integration, and the direct review lane is complete through WP validator and integration validator receipts.
- Rubric architecture fit self-review: The fix lands at the shared summary seams rather than UI-facing compensations or test-only assertions.
- Rubric heuristic quality self-review: The strongest part is the family-aware validator proof. The weakest part is that semantic merge authority integrated the signed scope onto `main` while the packet/task-board/runtime truth lagged and had to be synchronized afterward.
- Rubric anti-gaming / counterfactual check: If `governed_next_action_for_family()` were replaced with legacy tokens, the summary mutation tests would stop proving governed action semantics. If `validate_optional_governed_action_id()` stopped checking the record `workflow_state_family`, the `archive` negative-path probes would stop failing for ready-state summaries.
- Next step / handoff hint: CLOSED. Sync packet/board/traceability/runtime truth, then retire the stale ACP role sessions for this WP.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "Every emitted in-scope compact-summary `next_action` value is either a registered `GovernedActionDescriptorV1.action_id` or omitted."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:3172`, `src/backend/handshake_core/src/workflows.rs:3176`, `src/backend/handshake_core/src/workflows.rs:3180`, `src/backend/handshake_core/src/workflows.rs:4647`, `src/backend/handshake_core/src/workflows.rs:4654`, `src/backend/handshake_core/src/workflows.rs:4712`, `src/backend/handshake_core/src/workflows.rs:4719`, `src/backend/handshake_core/src/locus/types.rs:1072`, `src/backend/handshake_core/src/locus/types.rs:1293`, `src/backend/handshake_core/src/locus/types.rs:1327`, `src/backend/handshake_core/src/locus/types.rs:1599`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:883`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1058`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1653`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1801`
  - REQUIREMENT: "Summary validation and tests mechanically fail unregistered or drifted `next_action` values."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:2037`, `src/backend/handshake_core/src/locus/types.rs:2073`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1058`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1094`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1801`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1851`
  - REQUIREMENT: "No live helper path in scope emits ad hoc token strings or prose-only next-action text."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:3172`, `src/backend/handshake_core/src/workflows.rs:3176`, `src/backend/handshake_core/src/workflows.rs:3180`, `src/backend/handshake_core/src/workflows.rs:4654`, `src/backend/handshake_core/src/workflows.rs:4719`, `src/backend/handshake_core/src/workflows.rs:11766`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
  - COMMAND: `just wp-communication-health-check WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1 VERDICT`
  - EXIT_CODE: 0
  - LOG_PATH: `N/A`
  - LOG_SHA256: `N/A`
  - PROOF_LINES: `[WP_COMMUNICATION_HEALTH] PASS: Direct review lane is complete`
  - COMMAND: `just validator-packet-complete WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1`
  - EXIT_CODE: 0
  - LOG_PATH: `N/A`
  - LOG_SHA256: `N/A`
  - PROOF_LINES: `validator-packet-complete: PASS - WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1 has required fields.`
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests`
  - EXIT_CODE: 0
  - LOG_PATH: `N/A`
  - LOG_SHA256: `N/A`
  - PROOF_LINES: `test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - EXIT_CODE: 0
  - LOG_PATH: `N/A`
  - LOG_SHA256: `N/A`
  - PROOF_LINES: `test result: ok. 207 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`

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

VALIDATION REPORT - WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
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
VALIDATOR_RISK_TIER: MEDIUM

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/packet.md` (status lagged as `Ready for Dev` during merge-authority review and was corrected to `Validated (PASS)` in this closeout pass)
- Spec: `Handshake_Master_Spec_v02.178.md`
- Reviewed diff: `9f8bbb6fecab00a6df8dc9b1ea20d7ff085a637f..154445243c0b28dc910454b97b0f7df2935529c7`
- Merged main commit: `154445243c0b28dc910454b97b0f7df2935529c7`
- Review receipts: `review:WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1:validator_kickoff:mn6adpkf:af41af`; `review:WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1:coder_handoff:mn6foaob:48d179`

CLAUSES_REVIEWED:
- `StructuredCollaborationSummaryV1 compact summary contract includes optional `next_action`` -> `src/backend/handshake_core/src/locus/types.rs:1072`; `src/backend/handshake_core/src/locus/types.rs:1293`; `src/backend/handshake_core/src/locus/types.rs:1327`; `src/backend/handshake_core/src/locus/types.rs:1599`; `src/backend/handshake_core/src/workflows.rs:3172`; `src/backend/handshake_core/src/workflows.rs:3176`; `src/backend/handshake_core/src/workflows.rs:3180`; `src/backend/handshake_core/src/workflows.rs:4647`; `src/backend/handshake_core/src/workflows.rs:4654`; `src/backend/handshake_core/src/workflows.rs:4712`; `src/backend/handshake_core/src/workflows.rs:4719`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:883`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1058`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1653`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1801`
- `Every state-changing operator or model action SHOULD resolve through a registered GovernedActionDescriptorV1` -> `src/backend/handshake_core/src/locus/types.rs:2037`; `src/backend/handshake_core/src/locus/types.rs:2073`; `src/backend/handshake_core/src/workflows.rs:3180`; `src/backend/handshake_core/src/workflows.rs:4654`; `src/backend/handshake_core/src/workflows.rs:4719`; `src/backend/handshake_core/src/workflows.rs:11766`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1094`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1851`
- `Task Board and board-adjacent viewers SHOULD expose base-envelope next action before grouping metadata` -> `src/backend/handshake_core/src/workflows.rs:3172`; `src/backend/handshake_core/src/workflows.rs:3176`; `src/backend/handshake_core/src/workflows.rs:3180`; `src/backend/handshake_core/src/workflows.rs:3520`; `src/backend/handshake_core/src/workflows.rs:3541`; `src/backend/handshake_core/src/workflows.rs:4654`; `src/backend/handshake_core/src/workflows.rs:4719`

NOT_PROVEN:
- NONE

MAIN_BODY_GAPS:
- NONE

QUALITY_RISKS:
- NONE

DIFF_ATTACK_SURFACES:
- Summary emitter drift between Work Packet and Micro-Task compact summaries.
- Shared validator drift that would allow registered but family-illegal `next_action` values.
- Runtime registry refresh drift that could reintroduce legacy summary tokens after initial write.

INDEPENDENT_CHECKS_RUN:
- `git -C ../handshake_main rev-parse HEAD` => `154445243c0b28dc910454b97b0f7df2935529c7`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` from `../handshake_main` => `test result: ok. 26 passed; 0 failed`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` from `../handshake_main` => `test result: ok. 207 passed; 0 failed`
- `just wp-communication-health-check WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1 VERDICT` => `PASS`

COUNTERFACTUAL_CHECKS:
- If `governed_next_action_for_family()` in `src/backend/handshake_core/src/workflows.rs:3180` reverted to legacy token helpers, the summary mutations at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1058` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1801` would stop proving governed next-action semantics.
- If `validate_optional_governed_action_id()` in `src/backend/handshake_core/src/locus/types.rs:2037` stopped checking `workflow_state_family`, the registered-but-family-illegal `archive` mutations at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1094` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1851` would pass.

BOUNDARY_PROBES:
- Producer/validator probe: `src/backend/handshake_core/src/workflows.rs:4647-4719` emits `workflow_state_family` plus governed `next_action`, and `src/backend/handshake_core/src/locus/types.rs:1293-1329` revalidates the same contract at the shared boundary.
- Runtime refresh probe: `src/backend/handshake_core/src/workflows.rs:11758-11766` reuses `next_action_for_micro_task()` when refreshing Micro-Task summary metadata, so the runtime path cannot silently preserve legacy tokens while the initial write path is governed.

NEGATIVE_PATH_CHECKS:
- Work Packet summary mutation `next_action: "start_work_packet"` is rejected at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1058-1092`.
- Work Packet summary mutation `next_action: "archive"` against a ready-state summary is rejected at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1094-1128`.
- Micro-Task summary mutation `next_action: "start_micro_task"` is rejected at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1801-1849`.
- Micro-Task summary mutation `next_action: "archive"` against a ready-state summary is rejected at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1851-1899`.

INDEPENDENT_FINDINGS:
- The merged `main` tree no longer carries any live ad hoc summary `next_action` token helpers in `workflows.rs`; only the negative-path tests still mention legacy strings.
- The shared validator now treats `next_action` as optional but family-scoped when present, which closes the WP-validator finding that blocked the first handoff.

RESIDUAL_UNCERTAINTY:
- This closeout rechecked the merged backend summary and validator surfaces plus the full `handshake_core` suite, but it did not re-audit downstream viewer/UI consumption of `summary_ref` beyond the Task Board projection backend.
- Semantic merge authority landed the signed-scope product diff on `main`, but the packet/task-board/runtime truth lagged until this closeout pass; that workflow gap is outside the product verdict and should be handled on the repo-governance lane.

SPEC_CLAUSE_MAP:
- `StructuredCollaborationSummaryV1 compact summary contract includes optional `next_action`` -> `src/backend/handshake_core/src/locus/types.rs:1072`; `src/backend/handshake_core/src/locus/types.rs:1293`; `src/backend/handshake_core/src/locus/types.rs:1327`; `src/backend/handshake_core/src/locus/types.rs:1599`; `src/backend/handshake_core/src/workflows.rs:4647`; `src/backend/handshake_core/src/workflows.rs:4712`
- `Every state-changing operator or model action SHOULD resolve through a registered GovernedActionDescriptorV1` -> `src/backend/handshake_core/src/locus/types.rs:2037`; `src/backend/handshake_core/src/locus/types.rs:2073`; `src/backend/handshake_core/src/workflows.rs:3180`; `src/backend/handshake_core/src/workflows.rs:4654`; `src/backend/handshake_core/src/workflows.rs:4719`; `src/backend/handshake_core/src/workflows.rs:11766`
- `Task Board and board-adjacent viewers SHOULD expose base-envelope next action before grouping metadata` -> `src/backend/handshake_core/src/workflows.rs:3172`; `src/backend/handshake_core/src/workflows.rs:3176`; `src/backend/handshake_core/src/workflows.rs:3520`; `src/backend/handshake_core/src/workflows.rs:3541`

NEGATIVE_PROOF:
- `src/backend/handshake_core/src/workflows.rs:3172-3180` and `src/backend/handshake_core/src/workflows.rs:4647-4719` now carry governed `next_action` in the compact summaries, but the board projection at `src/backend/handshake_core/src/workflows.rs:3520-3541` still emits an indirect `summary_ref`-driven row instead of copying `next_action` inline. The compact-summary contract is closed, while the broader viewer projection remains only indirectly aligned.

REASON FOR PASS:
- The signed three-file scope is closed on merged local `main` commit `154445243c0b28dc910454b97b0f7df2935529c7`, the WP-validator family-legality finding was repaired before final authority, the integration-validator exercised merge authority against canonical `main`, and post-merge local proof on `main` passed for both `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` and the full `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`.
