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

# Task Packet: WP-1-Structured-Collaboration-Schema-Registry-v4

## METADATA
- TASK_ID: WP-1-Structured-Collaboration-Schema-Registry-v4
- WP_ID: WP-1-Structured-Collaboration-Schema-Registry-v4
- BASE_WP_ID: WP-1-Structured-Collaboration-Schema-Registry
- DATE: 2026-03-24T22:39:44.996Z
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Structured-Collaboration-Schema-Registry-v4
<!-- Validator roles keep distinct local branches/worktrees, but they mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v4
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-schema-registry-v4
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v4
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v4
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Schema-Registry-v4
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v4
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v4
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Schema-Registry-v4
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Structured-Collaboration-Schema-Registry-v4
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Structured-Collaboration-Schema-Registry-v4
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
- **Status:** In Progress
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: YES
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Artifact-Family
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Project-Profile-Extension-Registry, WP-1-Markdown-Mirror-Sync-Drift-Guard, WP-1-Dev-Command-Center-Structured-Artifact-Viewer
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v4
- LOCAL_WORKTREE_DIR: ../wtc-schema-registry-v4
- REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v4
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v4
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v4
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v4/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v4/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v4/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja240320262335
- PACKET_FORMAT_VERSION: 2026-03-23

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.171] canonical Work Packet, Micro-Task, and Task Board records SHALL expose workflow_state_family, queue_reason_code, allowed_action_ids | CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/locus/task_board.rs | TESTS: cargo test -p handshake_core schema_registry workflow_state | EXAMPLES: Mutated Work Packet `packet.json` missing `workflow_state_family`, Mutated Micro-Task `packet.json` missing `allowed_action_ids`, Mutated Task Board `index.json` with malformed `rows[0]`, Mutated Role Mailbox `index.json` with malformed `threads[0].created_at`, Mutated Role Mailbox thread line with invalid `body_ref`, `body_sha256`, or `transcription_links[0].target_sha256` | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: [ADD v02.168] Task Board projections and Role Mailbox exports MUST remain field-equivalent at the base-envelope level and obey their nested structured record contracts | CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/role_mailbox.rs | TESTS: cargo test -p handshake_core task_board nested_validation; cargo test -p handshake_core role_mailbox nested_validation | EXAMPLES: Mutated Work Packet `packet.json` missing `workflow_state_family`, Mutated Micro-Task `packet.json` missing `allowed_action_ids`, Mutated Task Board `index.json` with malformed `rows[0]`, Mutated Role Mailbox `index.json` with malformed `threads[0].created_at`, Mutated Role Mailbox thread line with invalid `body_ref`, `body_sha256`, or `transcription_links[0].target_sha256` | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 typed timestamp, artifact-handle, and sha256 fields | CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/role_mailbox.rs | TESTS: cargo test -p handshake_core role_mailbox structured_field_formats | EXAMPLES: Mutated Work Packet `packet.json` missing `workflow_state_family`, Mutated Micro-Task `packet.json` missing `allowed_action_ids`, Mutated Task Board `index.json` with malformed `rows[0]`, Mutated Role Mailbox `index.json` with malformed `threads[0].created_at`, Mutated Role Mailbox thread line with invalid `body_ref`, `body_sha256`, or `transcription_links[0].target_sha256` | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Mechanical gate (HARD) RoleMailboxExportGate requires valid thread-line field sets and required transcription links | CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/tests/role_mailbox_tests.rs | TESTS: cargo test -p handshake_core role_mailbox export_gate_inputs | EXAMPLES: Mutated Work Packet `packet.json` missing `workflow_state_family`, Mutated Micro-Task `packet.json` missing `allowed_action_ids`, Mutated Task Board `index.json` with malformed `rows[0]`, Mutated Role Mailbox `index.json` with malformed `threads[0].created_at`, Mutated Role Mailbox thread line with invalid `body_ref`, `body_sha256`, or `transcription_links[0].target_sha256` | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test -p handshake_core schema_registry
  - cargo test -p handshake_core task_board
  - cargo test -p handshake_core role_mailbox
  - cargo test -p handshake_core
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test -p handshake_core schema_registry_workflow_state_fields_required
  - cargo test -p handshake_core task_board_nested_record_validation
  - cargo test -p handshake_core role_mailbox_typed_structured_validation
  - cargo test -p handshake_core role_mailbox_export_nested_validation
- CANONICAL_CONTRACT_EXAMPLES:
  - Mutated Work Packet `packet.json` missing `workflow_state_family`
  - Mutated Micro-Task `packet.json` missing `allowed_action_ids`
  - Mutated Task Board `index.json` with malformed `rows[0]`
  - Mutated Role Mailbox `index.json` with malformed `threads[0].created_at`
  - Mutated Role Mailbox thread line with invalid `body_ref`, `body_sha256`, or `transcription_links[0].target_sha256`
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Structured-Collaboration-Schema-Registry-v4.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Base structured schema and project-profile extension contract [ADD v02.168]
- CONTEXT_START_LINE: 6839
- CONTEXT_END_LINE: 6882
- CONTEXT_TOKEN: **Base structured schema and project-profile extension contract** [ADD v02.168]
- EXCERPT_ASCII_ESCAPED:
  ```text
**Base structured schema and project-profile extension contract** [ADD v02.168]

  - Every canonical collaboration artifact family member SHALL implement one shared base envelope before any profile-specific fields are applied. At minimum that base envelope MUST expose:
    - `schema_id`
    - `schema_version`
    - `record_id`
    - `record_kind`
    - `project_profile_kind`
    - `updated_at`
    - `mirror_state`
    - `authority_refs`
    - `evidence_refs`
  - The base envelope MUST remain valid even when no project-profile extension is present. Software-delivery fields such as repository branch names, worktree paths, coding-language hints, or continuous-integration gate identifiers SHALL move into `profile_extension` payloads rather than becoming universal required fields.
  - `project_profile_kind` SHALL be stable and low-cardinality. Phase 1 default kinds are `software_delivery`, `research`, `worldbuilding`, `design`, `generic`, and `custom`.
  - `profile_extension` payloads MUST declare `extension_schema_id`, `extension_schema_version`, and `compatibility` so migration and validation tooling can reject unknown breaking extensions deterministically.
  - `mirror_state` SHALL be one of:
    - `canonical_only`
    - `synchronized`
    - `stale`
    - `advisory_edit`
    - `normalization_required`
  - Implementations MAY denormalize base-envelope fields into top-level record keys, but Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST remain field-equivalent at the base-envelope level so shared viewers, validators, and local-small-model ingestion can reuse the same parser.

  **Compact summary contract for local small models** [ADD v02.168]

  - Every canonical `packet.json`, `index.json`, or `thread.jsonl` collaboration artifact family member SHOULD have a paired bounded summary view that smaller local models can ingest without loading the full long-form note history.
  - `summary.json` records SHOULD implement `StructuredCollaborationSummaryV1` and MUST preserve:
    - `record_id`
    - `record_kind`
    - `project_profile_kind`
    - `status`
    - `title_or_objective`
    - `blockers`
    - `next_action`
    - `authority_refs`
    - `evidence_refs`
    - `updated_at`
  - Dev Command Center, Task Board derived layouts, Role Mailbox triage, and local-small-model planning flows SHOULD default to the compact summary contract first and load canonical detail records or Markdown sidecars only on demand.

  **Task Board and Role Mailbox structured projections** [ADD v02.168]

  - Task Board projection rows SHOULD be serialized as `record_kind=task_board_entry` records that reuse the same base envelope and add only board-specific fields such as `task_board_id`, `work_packet_id`, `lane_id`, `display_order`, and optional `view_ids`.
  - Role Mailbox exports SHOULD reuse the same base envelope for thread indexes and append-only thread lines. Message-level records SHOULD add only mailbox-specific fields such as `thread_id`, `message_type`, `from_role`, `to_roles`, `expected_response`, and `expires_at`.
  - When a collaboration artifact supports both canonical detail and compact summary representations, both records MUST share the same `record_id`, `project_profile_kind`, and authoritative references so deterministic joins remain possible without transcript reconstruction or Markdown parsing.
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

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md RoleMailboxIndexV1 and RoleMailboxThreadLineV1 base-envelope contract
- CONTEXT_START_LINE: 11023
- CONTEXT_END_LINE: 11083
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
  ```

#### ANCHOR 4
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
  - CLAUSE: [ADD v02.171] canonical Work Packet, Micro-Task, and Task Board records SHALL expose workflow_state_family, queue_reason_code, allowed_action_ids | WHY_IN_SCOPE: current emitters largely populate these fields but the shared validator does not require them, so malformed records can still pass | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/locus/task_board.rs | EXPECTED_TESTS: cargo test -p handshake_core schema_registry workflow_state | RISK_IF_MISSED: routing falls back to lane order, mailbox chronology, or prose instead of portable state law
  - CLAUSE: [ADD v02.168] Task Board projections and Role Mailbox exports MUST remain field-equivalent at the base-envelope level and obey their nested structured record contracts | WHY_IN_SCOPE: current shared validation checks outer arrays but not nested `rows[]`, `threads[]`, or `transcription_links[]` element shape | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/role_mailbox.rs | EXPECTED_TESTS: cargo test -p handshake_core task_board nested_validation; cargo test -p handshake_core role_mailbox nested_validation | RISK_IF_MISSED: malformed nested payloads pass smoke tests and fail only in downstream consumers
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 typed timestamp, artifact-handle, and sha256 fields | WHY_IN_SCOPE: current shared validation treats these fields as non-empty strings even though the spec gives them typed semantics | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/src/role_mailbox.rs | EXPECTED_TESTS: cargo test -p handshake_core role_mailbox structured_field_formats | RISK_IF_MISSED: mailbox exports keep a split validation standard where mailbox-local code is stricter than the shared record validator
  - CLAUSE: Mechanical gate (HARD) RoleMailboxExportGate requires valid thread-line field sets and required transcription links | WHY_IN_SCOPE: the live smoke-test claim depends on malformed mailbox exports being caught deterministically before closure | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs, src/backend/handshake_core/tests/role_mailbox_tests.rs | EXPECTED_TESTS: cargo test -p handshake_core role_mailbox export_gate_inputs | RISK_IF_MISSED: mailbox smoke coverage remains optimistic and misses spec-grade export corruption
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: TrackedWorkPacket structured record validation | PRODUCER: src/backend/handshake_core/src/workflows.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and downstream viewers | SERIALIZER_TRANSPORT: serde JSON (packet.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core schema_registry workflow_state | DRIFT_RISK: emitted workflow-state fields exist but malformed or missing values still pass the shared validator
  - CONTRACT: TrackedMicroTask structured record validation | PRODUCER: src/backend/handshake_core/src/workflows.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and downstream runtimes | SERIALIZER_TRANSPORT: serde JSON (packet.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core schema_registry workflow_state | DRIFT_RISK: same portable workflow-state gap as work packets
  - CONTRACT: TaskBoardIndexV1 and TaskBoardViewV1 nested rows | PRODUCER: src/backend/handshake_core/src/workflows.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and task-board viewers | SERIALIZER_TRANSPORT: serde JSON (index.json / views/<view_id>.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core task_board nested_validation | DRIFT_RISK: row arrays remain well-typed only at the outer level while inner row objects silently drift
  - CONTRACT: RoleMailboxIndexV1 nested threads | PRODUCER: src/backend/handshake_core/src/role_mailbox.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and mailbox viewers | SERIALIZER_TRANSPORT: serde JSON (index.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core role_mailbox nested_validation | DRIFT_RISK: mailbox index threads keep their own informal schema instead of the shared typed contract
  - CONTRACT: RoleMailboxThreadLineV1 typed fields and transcription links | PRODUCER: src/backend/handshake_core/src/role_mailbox.rs | CONSUMER: src/backend/handshake_core/src/locus/types.rs and mailbox export consumers | SERIALIZER_TRANSPORT: JSONL (threads/<thread_id>.jsonl) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core role_mailbox structured_field_formats | DRIFT_RISK: shared validation accepts malformed timestamps, handle strings, or sha256 values that mailbox-local code would reject
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Add small shared validation helpers in `src/backend/handshake_core/src/locus/types.rs` for RFC3339 timestamps, lowercase hex sha256 strings, artifact-handle strings, required workflow-state fields, and nested object arrays.
  - Extend `validate_structured_collaboration_record()` to require `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` on `WorkPacketPacket`, `MicroTaskPacket`, and `TaskBoardEntry`.
  - Reuse the TaskBoard entry contract to validate nested `rows[]` for Task Board index and view records.
  - Validate nested Role Mailbox `threads[]` and `transcription_links[]` element shapes at the shared validator boundary, keeping the implementation narrow and spec-driven.
  - Reuse mailbox-local parsing helpers only if doing so does not create cross-module coupling or circular dependencies; otherwise add minimal equivalent shared-format validators in `locus/types.rs`.
  - Add mutation-based negative tests in `micro_task_executor_tests.rs` and `role_mailbox_tests.rs` that operate on emitted JSON artifacts before validation.
- HOT_FILES:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core schema_registry
  - cargo test -p handshake_core task_board
  - cargo test -p handshake_core role_mailbox
  - cargo test -p handshake_core
- CARRY_FORWARD_WARNINGS:
  - Do not reopen the already-closed v3 scope around emitter basics or structured-diagnostic introduction unless the current code proves a concrete regression.
  - Keep the change centered on the shared validator and tests; do not widen into Loom portability or `.GOV` governance scripts.
  - Recursive nested validation should stay shallow and explicit enough to audit; avoid a speculative generic schema engine.
  - Do not let mailbox-local stricter validation remain the only typed guard for exported thread-line fields.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - [ADD v02.171] required workflow-state fields on canonical Work Packet, Micro-Task, and Task Board records
  - [ADD v02.168] nested Task Board and Role Mailbox payload contracts
  - RoleMailboxIndexV1 and RoleMailboxThreadLineV1 typed timestamp, artifact-handle, and sha256 field semantics
  - Mechanical gate intent for mailbox exports as proven through malformed export-input tests
- FILES_TO_READ:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core schema_registry
  - cargo test -p handshake_core task_board
  - cargo test -p handshake_core role_mailbox
  - rg -n "workflow_state_family|queue_reason_code|allowed_action_ids|rows|threads|transcription_links|body_ref|body_sha256|target_sha256|created_at|generated_at|updated_at" src/backend/handshake_core
- POST_MERGE_SPOTCHECKS:
  - Verify emitted smoke-test artifacts still pass happy-path validation after the validator becomes stricter.
  - Verify malformed nested rows and transcription links fail at the shared validator boundary, not only in mailbox-local code.
  - Verify no scope drift into Loom files, `.GOV` files, or new schema ids.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Whether the cleanest implementation is shared helper reuse from `role_mailbox.rs` or new minimal typed validators inside `locus/types.rs`
  - How many existing happy-path test fixtures will need touch-ups once nested element validation becomes strict
  - Whether current emitted task-board and mailbox fixtures already contain any silently tolerated malformed fields that the new validator will surface immediately
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Internal sources are sufficient because the missing work is explicit in current spec clauses and current local code paths.
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
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
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
  - portable workflow-state fields on canonical packet and micro-task records -> IN_THIS_WP (stub: NONE)
  - portable workflow-state fields on canonical task-board entry records -> IN_THIS_WP (stub: NONE)
  - recursive validation of task-board `rows[]` payloads -> IN_THIS_WP (stub: NONE)
  - recursive validation of mailbox `threads[]` payloads -> IN_THIS_WP (stub: NONE)
  - recursive validation of mailbox `transcription_links[]` payloads -> IN_THIS_WP (stub: NONE)
  - typed RFC3339 validation for shared structured-collaboration timestamps -> IN_THIS_WP (stub: NONE)
  - typed artifact-handle validation for exported mailbox references -> IN_THIS_WP (stub: NONE)
  - mutation-based negative-path proof over emitted smoke-test artifacts -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: Shared validator enforcement of portable workflow-state fields | SUBFEATURES: `workflow_state_family`, `queue_reason_code`, `allowed_action_ids` on Work Packet, Micro-Task, and Task Board records | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: current emitters already populate most of these fields; the remaining gap is central validator enforcement and regression proof
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Canonical packet typed-field validation | SUBFEATURES: RFC3339 `updated_at`, portable workflow-state fields, negative-path record mutation tests | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationEnvelopeV1 | MECHANICAL: engine.archivist, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: work packet records are live smoke-test artifacts and need hard validator proof, not another happy-path-only closure
  - PILLAR: MicroTask | CAPABILITY_SLICE: Canonical micro-task typed-field validation | SUBFEATURES: RFC3339 `updated_at`, portable workflow-state fields, negative-path record mutation tests | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationEnvelopeV1 | MECHANICAL: engine.archivist, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task records share the same validator law and should fail on the same malformed workflow or timestamp drift
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Nested Task Board row validation | SUBFEATURES: index/view `rows[]` element validation, TaskBoardEntry workflow-state enforcement, row-shape negative tests | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1 | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the current validator checks outer arrays but not the nested row contract the spec expects consumers to trust
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Portable machine-readable record trust boundary | SUBFEATURES: typed timestamps, typed sha256 values, typed artifact handles, portable workflow-state vocabulary | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-RoleMailboxThreadLineV1, PRIM-RoleMailboxIndexV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: local models and operator tooling need records that fail deterministically before prose-only recovery paths are considered
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Shared structured-collaboration validator hardening | JobModel: WORKFLOW | Workflow: locus_structured_artifact_validation | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: validation results stay machine-readable and portable across packet, task-board, and mailbox records
  - Capability: Task Board nested row validation | JobModel: WORKFLOW | Workflow: task_board_projection_publish | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: board projections should fail deterministically when row payloads drift from the spec-defined nested contract
  - Capability: Role Mailbox typed export validation | JobModel: WORKFLOW | Workflow: role_mailbox_export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-MAILBOX-002, FR-EVT-GOV-MAILBOX-003 | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox export consumers should not need mailbox-local parsing rules to detect malformed thread lines or transcription links
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Structured-Collaboration-Schema-Registry-v4 -> EXPAND_IN_THIS_WP
  - WP-1-Loom-Storage-Portability-v4 -> KEEP_SEPARATE
  - WP-1-Project-Profile-Extension-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Artifact-Family-v1 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Schema-Registry-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Structured-Collaboration-Schema-Registry-v2 -> EXPAND_IN_THIS_WP
  - WP-1-Role-Mailbox-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Structured-Collaboration-Schema-Registry-v2)
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs -> IMPLEMENTED (WP-1-Structured-Collaboration-Schema-Registry-v2)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> IMPLEMENTED (WP-1-Structured-Collaboration-Schema-Registry-v2)
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs -> PARTIAL (WP-1-Structured-Collaboration-Schema-Registry-v2)
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs -> PARTIAL (WP-1-Structured-Collaboration-Schema-Registry-v2)
  - ../handshake_main/src/backend/handshake_core/tests/role_mailbox_tests.rs -> PARTIAL (WP-1-Structured-Collaboration-Schema-Registry-v2)
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
- What: Harden the shared structured-collaboration validator so Work Packet, Micro-Task, Task Board, and Role Mailbox records enforce portable workflow-state fields, recursively validate nested payload elements, and reject malformed RFC3339 timestamps, artifact-handle strings, and sha256 strings with validator-owned negative-path proof.
- Why: The v3 smoke-test lane already improved emitters and happy-path outputs, but audit against the current Master Spec proved closure incomplete. The remaining gap is central validator strictness and adversarial proof, not new record families.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- OUT_OF_SCOPE:
  - Loom storage portability and database-backend abstraction work
  - Dev Command Center viewer and layout UI work
  - New schema ids, new primitive families, or spec-version bumps
  - Governance-only `.GOV` ledger or gate redesign
- TOUCHED_FILE_BUDGET: 6
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
cargo test -p handshake_core schema_registry
  cargo test -p handshake_core task_board
  cargo test -p handshake_core role_mailbox
  cargo test -p handshake_core
  just gov-check
```

### DONE_MEANS
- `validate_structured_collaboration_record()` rejects missing `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` on canonical Work Packet, Micro-Task, and Task Board records.
- Task Board `rows[]`, Role Mailbox `threads[]`, and `transcription_links[]` are validated per element shape rather than only by outer-array presence.
- Shared structured validation rejects malformed RFC3339 timestamps, malformed artifact-handle strings, and malformed sha256 strings on the relevant record families.
- Regression tests prove the above failures on mutated exported JSON, not only on happy-path emitters.
- Existing happy-path smoke tests continue to pass after validator hardening.

- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
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
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-03-24T22:39:44.996Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.171]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
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
  - .GOV/spec/Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- SEARCH_TERMS:
  - workflow_state_family
  - queue_reason_code
  - allowed_action_ids
  - updated_at
  - generated_at
  - created_at
  - body_ref
  - body_sha256
  - transcription_links
  - threads
  - rows
  - validate_structured_collaboration_record
- RUN_COMMANDS:
  ```bash
rg -n "workflow_state_family|queue_reason_code|allowed_action_ids|updated_at|generated_at|created_at|body_ref|body_sha256|transcription_links|threads|rows|validate_structured_collaboration_record" src/backend/handshake_core
  cargo test -p handshake_core schema_registry
  cargo test -p handshake_core task_board
  cargo test -p handshake_core role_mailbox
  cargo test -p handshake_core
  just gov-check
  ```
- RISK_MAP:
  - "validator hardening rejects existing fixtures or exports" -> "current smoke tests were masking real malformed payload tolerance"
  - "nested validation pulls in too much emitter refactor" -> "scope creep away from the shared validator into unrelated record redesign"
  - "typed-field validation duplicates mailbox-local helpers poorly" -> "two conflicting validation dialects emerge for the same structured record"
  - "negative-path tests mutate the wrong layer" -> "green tests without proof that consumer-boundary payloads are actually protected"
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
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
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
- Next step / handoff hint:

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
  - LOG_PATH: `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v4/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

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
