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

# Task Packet: WP-1-Project-Profile-Extension-Registry-v1

## METADATA
- TASK_ID: WP-1-Project-Profile-Extension-Registry-v1
- WP_ID: WP-1-Project-Profile-Extension-Registry-v1
- BASE_WP_ID: WP-1-Project-Profile-Extension-Registry
- DATE: 2026-03-31T17:20:44.648Z
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Project-Profile-Extension-Registry-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: validate/WP-1-Project-Profile-Extension-Registry-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtv-extension-registry-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Project-Profile-Extension-Registry-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Project-Profile-Extension-Registry-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Project-Profile-Extension-Registry-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Project-Profile-Extension-Registry-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Project-Profile-Extension-Registry-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Project-Profile-Extension-Registry-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Project-Profile-Extension-Registry-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Project-Profile-Extension-Registry-v1
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY | ABANDONED
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
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: 26e85bbfdebfa5b19044420ced816ee3c3501f5d
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-01T11:40:08.254Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 26e85bbfdebfa5b19044420ced816ee3c3501f5d
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-01T11:40:08.254Z
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Project-Profile-Extension-Registry-v1
- LOCAL_WORKTREE_DIR: ../wtc-extension-registry-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Project-Profile-Extension-Registry-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Project-Profile-Extension-Registry-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Profile-Extension-Registry-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Profile-Extension-Registry-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Profile-Extension-Registry-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Profile-Extension-Registry-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: wp_validator:wp-1-project-profile-extension-registry-v1
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-project-profile-extension-registry-v1
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja310320261913
- PACKET_FORMAT_VERSION: 2026-03-29

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: NONE
Next: CLOSED; packet is contained in local main at 26e85bbfdebfa5b19044420ced816ee3c3501f5d and governance truth is synchronized.
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Base structured schema and project-profile extension contract [ADD v02.168] | CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/locus/task_board.rs; src/backend/handshake_core/src/role_mailbox.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension; cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board; cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox | EXAMPLES: Work Packet detail plus summary artifacts with `project_profile_kind=software_delivery` and a valid software-delivery `profile_extension`, Work Packet or Micro-Task detail plus summary artifacts with `project_profile_kind=research` and a valid non-software `profile_extension`, Task Board and Role Mailbox exported artifacts that preserve base-envelope fields and remain parseable when `profile_extension` is unknown or omitted | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: `profile_extension` payloads declare schema id, schema version, and compatibility and keep the base envelope valid when absent | CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/tests/micro_task_executor_tests.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension | EXAMPLES: Work Packet detail plus summary artifacts with `project_profile_kind=software_delivery` and a valid software-delivery `profile_extension`, Work Packet or Micro-Task detail plus summary artifacts with `project_profile_kind=research` and a valid non-software `profile_extension`, Task Board and Role Mailbox exported artifacts that preserve base-envelope fields and remain parseable when `profile_extension` is unknown or omitted | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 share the base envelope and project-profile extension boundary | CODE_SURFACES: src/backend/handshake_core/src/role_mailbox.rs; src/backend/handshake_core/tests/role_mailbox_tests.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox | EXAMPLES: Work Packet detail plus summary artifacts with `project_profile_kind=software_delivery` and a valid software-delivery `profile_extension`, Work Packet or Micro-Task detail plus summary artifacts with `project_profile_kind=research` and a valid non-software `profile_extension`, Task Board and Role Mailbox exported artifacts that preserve base-envelope fields and remain parseable when `profile_extension` is unknown or omitted | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
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
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - Work Packet detail plus summary artifacts with `project_profile_kind=software_delivery` and a valid software-delivery `profile_extension`
  - Work Packet or Micro-Task detail plus summary artifacts with `project_profile_kind=research` and a valid non-software `profile_extension`
  - Task Board and Role Mailbox exported artifacts that preserve base-envelope fields and remain parseable when `profile_extension` is unknown or omitted
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Project-Profile-Extension-Registry-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md Base structured schema and project-profile extension contract [ADD v02.168]
- CONTEXT_START_LINE: 6840
- CONTEXT_END_LINE: 6861
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
  - Implementations MAY denormalize base-envelope fields into top-level record keys, but Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST remain field-equivalent at the base-envelope level so shared viewers, validators, and local-small-model ingestion can reuse the same parser.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md RoleMailboxIndexV1 and RoleMailboxThreadLineV1 export schemas
- CONTEXT_START_LINE: 11020
- CONTEXT_END_LINE: 11086
- CONTEXT_TOKEN: interface RoleMailboxIndexV1 {
- EXCERPT_ASCII_ESCAPED:
  ```text
Export schemas (normative; role_mailbox_export_v1):

  // docs/ROLE_MAILBOX/index.json
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
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: Base structured schema and project-profile extension contract [ADD v02.168] | WHY_IN_SCOPE: current product code only partially implements the required profile-extension registry and parity across artifact families | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/locus/task_board.rs; src/backend/handshake_core/src/role_mailbox.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension; cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board; cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox | RISK_IF_MISSED: the registry will still be socially treated as done while consumers remain software-only
  - CLAUSE: `profile_extension` payloads declare schema id, schema version, and compatibility and keep the base envelope valid when absent | WHY_IN_SCOPE: current validation accepts extension-shaped metadata but does not prove a real registry-backed contract or fallback behavior end-to-end | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/tests/micro_task_executor_tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension | RISK_IF_MISSED: unknown or incompatible extensions will keep failing late or silently
  - CLAUSE: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 share the base envelope and project-profile extension boundary | WHY_IN_SCOPE: mailbox exports currently flatten to `software_delivery` and omit the explicit boundary | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox.rs; src/backend/handshake_core/tests/role_mailbox_tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox | RISK_IF_MISSED: mailbox exports will remain the easiest place for hidden repository assumptions to leak back into the shared artifact family
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: StructuredCollaborationSummaryV1 `profile_extension` payload | PRODUCER: locus summary emitters | CONSUMER: generic readers and small-model summary flows | SERIALIZER_TRANSPORT: packet summary JSON | VALIDATOR_READER: validate_profile_extension in locus/types.rs | TRIPWIRE_TESTS: profile_extension positive and negative cases in micro_task_executor_tests.rs | DRIFT_RISK: summary-level support can look complete even when downstream detail artifacts are not
  - CONTRACT: TrackedWorkPacketArtifactV1 and TrackedMicroTaskArtifactV1 project-profile boundary | PRODUCER: workflows.rs emitters | CONSUMER: Work Packet and Micro-Task detail readers | SERIALIZER_TRANSPORT: packet.json and summary.json | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: profile_extension filter tests plus emitted-artifact parity tests | DRIFT_RISK: packet and micro-task paths can diverge from Task Board or mailbox behavior silently
  - CONTRACT: TaskBoardEntryRecordV1, TaskBoardIndexV1, and TaskBoardViewV1 project-profile fields | PRODUCER: workflows.rs task-board projection builder | CONSUMER: Task Board and generic board viewers | SERIALIZER_TRANSPORT: task_board entry/index/view JSON | VALIDATOR_READER: task board record validators in locus/types.rs | TRIPWIRE_TESTS: task_board-targeted cargo tests with software and non-software examples | DRIFT_RISK: board views can stay software-only even when packet artifacts evolve
  - CONTRACT: RoleMailboxIndexV1 and RoleMailboxThreadLineV1 project-profile fields | PRODUCER: role_mailbox.rs export writers | CONSUMER: mailbox triage and generic export readers | SERIALIZER_TRANSPORT: index.json and thread.jsonl | VALIDATOR_READER: mailbox record validators in locus/types.rs and export checks | TRIPWIRE_TESTS: role_mailbox cargo tests with unknown-extension fallback and non-software example | DRIFT_RISK: mailbox export portability silently diverges from the rest of the artifact family
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Define the explicit registry and compatibility enforcement in `src/backend/handshake_core/src/locus/types.rs`.
  - Propagate `project_profile_kind` and the profile-extension boundary through Task Board and Role Mailbox emitted artifacts.
  - Add software-delivery and non-software proof cases plus unknown-extension fallback regression coverage.
- HOT_FILES:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
- CARRY_FORWARD_WARNINGS:
  - Do not hard-code `software_delivery` anywhere this packet touches.
  - Do not widen into workflow-state registry or transition-automation law.
  - Do not satisfy fallback behavior by dropping `profile_extension` while also dropping base-envelope parity.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - Base-envelope parity across Work Packet, Micro-Task, Task Board, and Role Mailbox records
  - Explicit extension schema id, version, and compatibility enforcement
  - Non-software emitted-artifact proof and unknown-extension fallback behavior
- FILES_TO_READ:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - NONE
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Current local `cargo test` runtime is slow and previous targeted commands timed out during audit, so runtime proof still depends on the actual coding and validation passes.
  - This packet does not prove a full Dev Command Center GUI implementation; it proves the backend contract and emitted-artifact fallback that current and future viewers rely on.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [BIG_TECH] GitHub Projects table layout docs | 2026-03-31 | Retrieved: 2026-03-31T15:20:00Z | https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-table-layout | Why: shows how configurable fields, grouping, slicing, and sorting can stay view-level behavior over shared items instead of redefining the canonical item contract
  - [OSS_DOC] Backstage descriptor format | 2026-03-31 | Retrieved: 2026-03-31T15:21:00Z | https://backstage.io/docs/features/software-catalog/descriptor-format | Why: demonstrates a stable shared envelope with kind-specific extension metadata and annotations
  - [GITHUB] backstage/backstage | 2026-03-31 | Retrieved: 2026-03-31T15:22:00Z | https://github.com/backstage/backstage | Why: provides a large OSS implementation surface for descriptor-based extensibility and catalog-style shared parsing
  - [GITHUB] open-metadata/OpenMetadata | 2026-03-30 | Retrieved: 2026-03-31T15:25:00Z | https://github.com/open-metadata/OpenMetadata | Why: reinforces registry-first schemas plus custom extensions/properties consumed by multiple surfaces from one central metadata repository
  - [PAPER] Validation of Modern JSON Schema: Formalization and Complexity | 2024-02-01 | Retrieved: 2026-03-31T15:25:30Z | https://arxiv.org/abs/2307.10034 | Why: warns that overly dynamic schema semantics become hard to reason about and validate, which argues for explicit low-cardinality compatibility handling in Handshake
- RESEARCH_SYNTHESIS:
  - The base envelope should stay small, typed, and portable while project-specific data moves into explicit versioned extensions.
  - Viewer/layout customization should operate over canonical shared fields rather than silently becoming the authority for workflow or portability semantics.
  - Extension compatibility needs to be explicit and simple, because dynamic or annotation-heavy schema behavior quickly becomes difficult to validate and explain.
- GITHUB_PROJECT_DECISIONS:
  - backstage/backstage -> ADAPT (NONE)
  - open-metadata/OpenMetadata -> ADOPT (NONE)
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_VERDICT: PASS
- SOURCE_SCAN_DECISIONS:
  - GitHub Projects table layout docs -> ADAPT (IN_THIS_WP)
  - Backstage descriptor format -> ADOPT (IN_THIS_WP)
  - open-metadata/OpenMetadata -> ADOPT (IN_THIS_WP)
  - Validation of Modern JSON Schema: Formalization and Complexity -> REJECT (IN_THIS_WP)
- MATRIX_GROWTH_CANDIDATES:
  - base-envelope registry plus view-level custom grouping -> IN_THIS_WP (stub: NONE)
  - registry-first schemas plus explicit compatibility semantics -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep canonical shared fields visible even when extensions are unknown or collapsed.
  - Separate viewer grouping and custom-field behavior from authoritative workflow and portability semantics.
  - Make extension compatibility explicit enough that validators can reject or degrade deterministically.
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-ProjectProfileExtensionV1
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_EXPOSED:
  - PRIM-ProjectProfileExtensionV1
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
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
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - base-envelope parity across work-packet and micro-task detail artifacts -> IN_THIS_WP (stub: NONE)
  - task-board projection parity from the Locus registry -> IN_THIS_WP (stub: NONE)
  - role-mailbox export parity from the Locus registry -> IN_THIS_WP (stub: NONE)
  - unknown-extension fallback on micro-task summaries -> IN_THIS_WP (stub: NONE)
  - non-software proof case across packet and micro-task artifact pairs -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: project-profile extension validation and compatibility | SUBFEATURES: shared base envelope, extension schema ids, compatibility semantics | PRIMITIVES_FEATURES: PRIM-ProjectProfileExtensionV1, PRIM-StructuredCollaborationEnvelopeV1, FEAT-WORK-PACKET-SYSTEM, FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.archivist, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: central registry and validation logic belong here
  - PILLAR: MicroTask | CAPABILITY_SLICE: canonical detail and compact summary boundary | SUBFEATURES: micro-task packet.json, summary.json, non-software proof case | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1, FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task artifacts already carry partial support and need the same registry-backed proof as work packets
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: profile-extension registry enforcement | JobModel: NONE | Workflow: structured_collaboration_validation | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: this is shared backend validation logic, not a standalone operator tool
  - Capability: Task Board projection parity for project-profile fields | JobModel: WORKFLOW | Workflow: task_board_projection_refresh | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: generic viewers depend on these projections preserving base-envelope semantics
  - Capability: Role Mailbox export parity for project-profile fields | JobModel: WORKFLOW | Workflow: role_mailbox_export | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: export consumers should parse mailbox records without software-only assumptions
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Project-Profile-Extension-Registry-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Project-Agnostic-Workflow-State-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Workflow-Transition-Automation-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Artifact-Family-v1 -> KEEP_SEPARATE
  - WP-1-Role-Mailbox-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Project-Profile-Extension-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs -> NOT_PRESENT (WP-1-Project-Profile-Extension-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Project-Profile-Extension-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs -> PARTIAL (WP-1-Project-Profile-Extension-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs -> PARTIAL (WP-1-Project-Profile-Extension-Registry-v1)
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
- What: Complete the explicit project-profile extension registry and propagate the base-envelope versus `profile_extension` boundary through Work Packet, Micro-Task, Task Board, Role Mailbox, and the coupled tracked micro-task progress payload needed to keep emitted progress artifacts consistent, including generic-fallback proof and one non-software example.
- Why: Current product code over-credits partial envelope plumbing as if the registry were done. Downstream portable workflow-law work remains unsafe until registry, projection, and export truth are aligned end-to-end.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- OUT_OF_SCOPE:
  - Project-agnostic workflow-state registry or transition automation work
  - Full Dev Command Center frontend or typed-viewer implementation
  - Main Body or appendix spec updates
  - Loom storage portability or runtime-backend refactors
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
cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
```

### DONE_MEANS
- Shared structured-collaboration validation enforces explicit profile-extension schema id, version, and compatibility semantics instead of accepting opaque partial metadata as if the registry were complete.
- Task Board and Role Mailbox emitted artifacts preserve `project_profile_kind` and the profile-extension boundary instead of hard-coding `software_delivery`.
- Base-envelope parsing still works when an extension is unknown or omitted, and that fallback is covered by tests.
- At least one software-delivery example and one non-software emitted-artifact example are produced and validated without breaking base-envelope validity.

- PRIMITIVES_EXPOSED:
  - PRIM-ProjectProfileExtensionV1
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
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
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-03-31T17:20:44.648Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.168]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md Base structured schema and project-profile extension contract [ADD v02.168]
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
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- SEARCH_TERMS:
  - project_profile_kind
  - profile_extension
  - extension_schema_id
  - extension_schema_version
  - compatibility
  - software_delivery
- RUN_COMMANDS:
  ```bash
rg -n "project_profile_kind|profile_extension|extension_schema_id|software_delivery" src/backend/handshake_core/src src/backend/handshake_core/tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml profile_extension
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  just gov-check
  ```
- RISK_MAP:
  - "Task Board or mailbox exports still flatten to software_delivery" -> "future non-software kernels inherit hidden repository assumptions"
  - "Unknown extensions silently disappear from generic readers" -> "operators and small models over-trust partial records"
  - "Packet widens into workflow-state or transition law" -> "remediation becomes too broad and loses proof quality"
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
- Committed validation target: `1b03c2cdf22b968368d651c1656e79082b8b25e0`
- Diff authority: `MERGE_BASE_SHA facce56f879d4ee990f62566b12a8b26d8bc61d7 .. HEAD`
- Spec Target Resolved: `.GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md`
- **Artifacts**: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Profile-Extension-Registry-v1/signed_scope.patch`

- **Target File**: `src/backend/handshake_core/src/locus/task_board.rs`
- **Start**: 6
- **End**: 175
- **Line Delta**: 7
- **Pre-SHA1**: `bd4a8b681d5fb0793b3e01aedfd7e90082035488`
- **Post-SHA1**: `932ba6a2a68c9c5c11a4280a3e898c9f3cf59e90`
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
- **Start**: 46
- **End**: 1888
- **Line Delta**: 150
- **Pre-SHA1**: `ce48d67cf815ac8bfb8c11184b5b4f301f4750b2`
- **Post-SHA1**: `0ba8f856fa29413dddd5a65a08fca9b9c83c4c73`
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

- **Target File**: `src/backend/handshake_core/src/role_mailbox.rs`
- **Start**: 21
- **End**: 1543
- **Line Delta**: 4
- **Pre-SHA1**: `0546b534dfc11d5ab263a2124805f65dc675d817`
- **Post-SHA1**: `84ac581ef5e179cacedf2d62de022e57315e6b0b`
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
- **Start**: 14
- **End**: 1135
- **Line Delta**: 1
- **Pre-SHA1**: `a3bdbe81c302f8fdbefd260bff808c12b2181ee8`
- **Post-SHA1**: `8f30d13dd18734fc89f5b72e1bd9c7e0294cb4cd`
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
- **Start**: 1480
- **End**: 11939
- **Line Delta**: 148
- **Pre-SHA1**: `a77b1a14aad10787f4fb5b4c8347de5b8ec484e2`
- **Post-SHA1**: `fce12ab1f35f0506cb6b0661780093c9be697403`
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
- **End**: 1752
- **Line Delta**: 195
- **Pre-SHA1**: `0c396ecceeec0e74dc726aaa887e95c9d74d8af5`
- **Post-SHA1**: `770432342415b5ee1cce58290fc975e0ede94437`
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

- **Target File**: `src/backend/handshake_core/tests/role_mailbox_tests.rs`
- **Start**: 240
- **End**: 541
- **Line Delta**: 59
- **Pre-SHA1**: `96adf0cc0f9bb09cd622996d1036772af84c3f99`
- **Post-SHA1**: `2304d6d1fff3d0854f196a4d3b20a730ea9f9b43`
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
## STATUS_HANDOFF
- Current WP_STATUS: In Progress (committed handoff proved on `1b03c2cdf22b968368d651c1656e79082b8b25e0`; final-lane closeout sync is the remaining completion step)
- What changed in this update: Replaced placeholder manifest/evidence sections with the exact 7-file committed proof surface and synchronized the packet to the recorded WP validator PASS plus Integration Validator review receipts.
- Requirements / clauses self-audited: The explicit profile-extension registry and compatibility checks in `src/backend/handshake_core/src/locus/types.rs`, the Task Board `profile_extension` boundary propagation in `src/backend/handshake_core/src/workflows.rs` plus `src/backend/handshake_core/src/locus/task_board.rs`, and the Role Mailbox generic/null boundary in `src/backend/handshake_core/src/role_mailbox.rs` all match the signed clause set.
- Checks actually run: `just pre-work WP-1-Project-Profile-Extension-Registry-v1`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_register_mts_returns_machine_readable_validation_for_unknown_profile_extension_schema -- --exact`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_sync_task_board_emits_structured_index_and_view -- --exact`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox_validation_reports_schema_and_authority_drift -- --exact`; `just gov-check`
- Known gaps / weak spots: No unresolved signed-scope product-code gaps remain in the committed diff; the only pre-closeout weakness was missing packet proof material, which this sync repairs.
- Heuristic risks / maintainability concerns: `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/locus/task_board.rs`, and `src/backend/handshake_core/src/role_mailbox.rs` remain shared cross-boundary surfaces, so future profile-kind additions must update registry seeds, emitted artifacts, and validators together.
- Validator focus request: Re-run final-lane closeout against committed target `1b03c2cdf22b968368d651c1656e79082b8b25e0` and confirm current-main compatibility plus merge-progression truth before projecting the packet to Done.
- Rubric contract understanding proof: This packet closes only the Phase 1 base-envelope and `profile_extension` contract; it does not widen into workflow-state automation, richer mailbox round-tripping, or unrelated collaboration-schema work.
- Rubric scope discipline proof: The committed diff now matches the signed 7-file product surface exactly, including the explicit scope widening that already brought `src/backend/handshake_core/src/storage/locus_sqlite.rs` into packet authority.
- Rubric baseline comparison: Compared against merge base `facce56f879d4ee990f62566b12a8b26d8bc61d7` and current local `main` head `4bc9aa76aa4469beede96a403e1aaf32e357bbbc`; no extra signed-scope files or hidden branch-local product deltas remain.
- Rubric end-to-end proof: Full `handshake_core` tests, exact unknown-schema rejection, task-board projection, mailbox validation, and `just gov-check` all passed on committed branch state `1b03c2cdf22b968368d651c1656e79082b8b25e0`.
- Rubric architecture fit self-review: The change keeps the shared envelope law centralized in `src/backend/handshake_core/src/locus/types.rs` and only projects boundary data through task-board and mailbox emitters, which fits the existing locus-centered design.
- Rubric heuristic quality self-review: The touched emitters no longer flatten everything to `software_delivery`; the packet now uses explicit registry-backed behavior and deterministic rejection rather than silent nullable drift.
- Rubric anti-gaming / counterfactual check: If `PROFILE_EXTENSION_REGISTRY` or `validate_profile_extension` lost registry enforcement, the unknown-schema exact probe would fail; if task-board or mailbox emitters dropped the boundary fields again, the exact task-board or mailbox probes would fail.
- Next step / handoff hint: Run final authority closeout sync and closeout check from `handshake_main` with kernel governance injected, then project the packet to `Done` / `MERGE_PENDING`.

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
- REQUIREMENT: "Base structured schema and project-profile extension contract [ADD v02.168]"
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:53`, `src/backend/handshake_core/src/locus/types.rs:1126`, `src/backend/handshake_core/src/locus/types.rs:1684`, `src/backend/handshake_core/src/workflows.rs:2633`, `src/backend/handshake_core/src/locus/task_board.rs:34`
- REQUIREMENT: "`profile_extension` payloads declare schema id, schema version, and compatibility and keep the base envelope valid when absent"
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1684`, `src/backend/handshake_core/src/locus/types.rs:1799`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1698`
- REQUIREMENT: "RoleMailboxIndexV1 and RoleMailboxThreadLineV1 share the base envelope and project-profile extension boundary"
  - EVIDENCE: `src/backend/handshake_core/src/role_mailbox.rs:1310`, `src/backend/handshake_core/src/role_mailbox.rs:1422`, `src/backend/handshake_core/src/role_mailbox.rs:1493`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:426`
## EVIDENCE
- COMMAND: `just pre-work WP-1-Project-Profile-Extension-Registry-v1`
- EXIT_CODE: 0
- PROOF_LINES: `PASS; packet/refinement scope hydration aligned; signed in-scope files match the committed review target`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: 0
- PROOF_LINES: `PASS on committed branch state 1b03c2cdf22b968368d651c1656e79082b8b25e0; full handshake_core suite green per coder handoff receipt 2026-04-01T02:04:33.086Z and WP validator PASS receipt 2026-04-01T02:11:17.238Z`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_register_mts_returns_machine_readable_validation_for_unknown_profile_extension_schema -- --exact`
- EXIT_CODE: 0
- PROOF_LINES: `PASS; unknown profile_extension schema ids are rejected deterministically`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_sync_task_board_emits_structured_index_and_view -- --exact`
- EXIT_CODE: 0
- PROOF_LINES: `PASS; Task Board row/index/view projection now carries the signed base-envelope/profile_extension boundary`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox_validation_reports_schema_and_authority_drift -- --exact`
- EXIT_CODE: 0
- PROOF_LINES: `PASS; mailbox export validation reports schema and authority drift on the updated generic/null boundary`

- COMMAND: `just gov-check`
- EXIT_CODE: 0
- PROOF_LINES: `PASS; governance checks green on the committed reviewable state`

## VALIDATION_REPORTS
### 2026-04-01 - VALIDATION REPORT - WP-1-Project-Profile-Extension-Registry-v1 (FINAL AUTHORITY)
Verdict: PASS

Scope Inputs:
- Reviewed commit: `1b03c2cdf22b968368d651c1656e79082b8b25e0`
- Task Packet: `.GOV/task_packets/WP-1-Project-Profile-Extension-Registry-v1/packet.md` (`**Status:** In Progress` at review time)
- Spec: `.GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md`
- Final review receipts:
  - `WP_VALIDATOR VALIDATOR_REVIEW PASS` at `2026-04-01T02:11:17.238Z`
  - `INTEGRATION_VALIDATOR REVIEW_RESPONSE` at `2026-04-01T02:46:32.499Z`

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
VALIDATOR_RISK_TIER: HIGH

CLAUSES_REVIEWED:
- Base structured schema and project-profile extension contract [ADD v02.168] -> `src/backend/handshake_core/src/locus/types.rs:53`, `src/backend/handshake_core/src/locus/types.rs:1684`, `src/backend/handshake_core/src/workflows.rs:2633`, `src/backend/handshake_core/src/locus/task_board.rs:34`
- `profile_extension` payloads declare schema id, schema version, and compatibility and keep the base envelope valid when absent -> `src/backend/handshake_core/src/locus/types.rs:1684`, `src/backend/handshake_core/src/locus/types.rs:1799`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1698`
- RoleMailboxIndexV1 and RoleMailboxThreadLineV1 share the base envelope and project-profile extension boundary -> `src/backend/handshake_core/src/role_mailbox.rs:1310`, `src/backend/handshake_core/src/role_mailbox.rs:1422`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:426`

NOT_PROVEN:
- NONE

MAIN_BODY_GAPS:
- NONE

QUALITY_RISKS:
- NONE

DIFF_ATTACK_SURFACES:
- Registry drift between `PROFILE_EXTENSION_REGISTRY` and `validate_profile_extension` in `src/backend/handshake_core/src/locus/types.rs`
- Projection drift between workflow emitters and Task Board row/index/view serializers in `src/backend/handshake_core/src/workflows.rs` and `src/backend/handshake_core/src/locus/task_board.rs`
- Mailbox portability drift if `project_profile_kind` or the `profile_extension` boundary regressed in `src/backend/handshake_core/src/role_mailbox.rs`

INDEPENDENT_CHECKS_RUN:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_register_mts_returns_machine_readable_validation_for_unknown_profile_extension_schema -- --exact` => PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_sync_task_board_emits_structured_index_and_view -- --exact` => PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml role_mailbox_validation_reports_schema_and_authority_drift -- --exact` => PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` => PASS
- `just gov-check` => PASS

COUNTERFACTUAL_CHECKS:
- If `validate_profile_extension` in `src/backend/handshake_core/src/locus/types.rs:1684` stopped enforcing registered schema ids or versions, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1698` would fail.
- If Task Board projection dropped the boundary fields in `src/backend/handshake_core/src/workflows.rs:1614` or `src/backend/handshake_core/src/locus/task_board.rs:34`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:959` would fail.
- If mailbox export stopped emitting the generic/null boundary in `src/backend/handshake_core/src/role_mailbox.rs:1310` or `src/backend/handshake_core/src/role_mailbox.rs:1422`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:426` would fail.

BOUNDARY_PROBES:
- Work Packet and Micro-Task writers -> locus validator boundary through `validate_structured_collaboration_record` in `src/backend/handshake_core/src/locus/types.rs:1126`
- Workflow task-board emitter -> Task Board row/index/view consumer boundary in `src/backend/handshake_core/src/workflows.rs:1614` and `src/backend/handshake_core/src/locus/task_board.rs:34`
- Role Mailbox export writer -> mailbox validator boundary in `src/backend/handshake_core/src/role_mailbox.rs:1422` and `src/backend/handshake_core/tests/role_mailbox_tests.rs:426`

NEGATIVE_PATH_CHECKS:
- Unknown extension schema ids are rejected at `src/backend/handshake_core/src/locus/types.rs:1684` and proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1698`
- Mailbox schema and authority drift is rejected by `src/backend/handshake_core/tests/role_mailbox_tests.rs:426`

INDEPENDENT_FINDINGS:
- The committed diff now matches the signed 7-file product surface exactly, including the previously widened `src/backend/handshake_core/src/storage/locus_sqlite.rs` touch.
- No remaining blocking `software_delivery` flattening was found in the touched Task Board or mailbox projection paths outside intended enum, registry, or test literals.
- Final authority review found no new blocking product-code findings beyond the packet-proof incompleteness that this sync resolves.

RESIDUAL_UNCERTAINTY:
- Review is diff-scoped against the signed clause set; broader downstream readers outside the exercised task-board and mailbox export paths were not exhaustively revalidated for every future custom profile kind.

SPEC_CLAUSE_MAP:
- Base structured schema and project-profile extension contract [ADD v02.168] -> `src/backend/handshake_core/src/locus/types.rs:53`, `src/backend/handshake_core/src/locus/types.rs:232`, `src/backend/handshake_core/src/workflows.rs:2633`, `src/backend/handshake_core/src/locus/task_board.rs:34`
- `profile_extension` payloads declare schema id, schema version, and compatibility and keep the base envelope valid when absent -> `src/backend/handshake_core/src/locus/types.rs:1684`, `src/backend/handshake_core/src/locus/types.rs:1799`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1698`
- RoleMailboxIndexV1 and RoleMailboxThreadLineV1 share the base envelope and project-profile extension boundary -> `src/backend/handshake_core/src/role_mailbox.rs:1310`, `src/backend/handshake_core/src/role_mailbox.rs:1422`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:426`

NEGATIVE_PROOF:
- Role Mailbox export does not preserve caller-specific non-generic `profile_extension` payloads; it intentionally normalizes to `project_profile_kind=generic` with `profile_extension=null` in `src/backend/handshake_core/src/role_mailbox.rs:1310`, `src/backend/handshake_core/src/role_mailbox.rs:1422`, and `src/backend/handshake_core/src/role_mailbox.rs:1493`, so richer mailbox profile round-tripping remains unimplemented outside this packet's signed boundary contract.
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Rule: `HEURISTIC_REVIEW_VERDICT=PASS` is only valid when `QUALITY_RISKS` is exactly `- NONE`.
- Rule: `LEGAL_VERDICT=PASS` is only valid when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, `COUNTERFACTUAL_CHECKS`, and `SPEC_CLAUSE_MAP` are all present and non-empty, and `SPEC_CLAUSE_MAP` entries include file:line evidence.
- Rule: `Verdict: PASS` is legal only when `VALIDATION_CONTEXT=OK`, `WORKFLOW_VALIDITY=VALID`, `SCOPE_VALIDITY=IN_SCOPE`, `PROOF_COMPLETENESS=PROVEN`, `INTEGRATION_READINESS=READY`, `DOMAIN_GOAL_COMPLETION=COMPLETE`, and `LEGAL_VERDICT=PASS`.
- Rule: if `PROOF_COMPLETENESS` is anything other than `PROVEN`, the top-line `Verdict` MUST NOT be `PASS`; use `NOT_PROVEN`, `FAIL`, `BLOCKED`, `OUTDATED_ONLY`, or `ABANDONED` honestly.
- Rule: `PROOF_COMPLETENESS=PROVEN` is legal only when `NOT_PROVEN` is exactly `- NONE`.
- Rule: `WORKFLOW_VALIDITY=VALID` is legal only when `VALIDATION_CONTEXT=OK` and `GOVERNANCE_VERDICT=PASS`.
- Rule: `LEGAL_VERDICT=PASS` is legal only when `PROOF_COMPLETENESS=PROVEN`.
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.
