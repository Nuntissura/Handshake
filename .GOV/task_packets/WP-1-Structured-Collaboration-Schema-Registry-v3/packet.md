# Task Packet: WP-1-Structured-Collaboration-Schema-Registry-v3

## METADATA
- TASK_ID: WP-1-Structured-Collaboration-Schema-Registry-v3
- WP_ID: WP-1-Structured-Collaboration-Schema-Registry-v3
- BASE_WP_ID: WP-1-Structured-Collaboration-Schema-Registry
- DATE: 2026-03-19T08:32:11.929Z
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
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed>
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Structured-Collaboration-Schema-Registry-v3
<!-- Validator roles keep distinct local branches/worktrees, but they mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-schema-registry-v3
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Schema-Registry-v3
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Schema-Registry-v3
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Structured-Collaboration-Schema-Registry-v3
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Structured-Collaboration-Schema-Registry-v3
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Ready for Dev
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
- LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- LOCAL_WORKTREE_DIR: ../wtc-schema-registry-v3
- REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v3
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v3
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v3/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v3/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v3/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- USER_SIGNATURE: ilja190320260923
- PACKET_FORMAT_VERSION: 2026-03-18

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: Prior v3 ACP bootstrap was abandoned after the assigned feature branch/worktree were torn down; no live coder or validator session remains. Fresh PREPARE plus canonical worktree recreation is required before delegation can restart.
Next: Wait for Operator restart instruction, then recreate the canonical worktree and re-run PREPARE before coder launch.

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.168] Base envelope MUST expose updated_at | CODE_SURFACES: locus/types.rs (ensure_schema_registry_fields_work_packet, ensure_schema_registry_fields_micro_task) | TESTS: cargo test -p handshake_core schema_registry updated_at | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: [ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids | CODE_SURFACES: locus/task_board.rs (TaskBoardEntryRecordV1), workflows.rs (board emission) | TESTS: cargo test -p handshake_core task_board_entry | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: [ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas | CODE_SURFACES: locus/types.rs (validate_structured_collaboration_record, validate_profile_extension) | TESTS: cargo test -p handshake_core validation_diagnostics | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: [ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs | CODE_SURFACES: workflows.rs (artifact emission), runtime_governance.rs | TESTS: cargo test -p handshake_core summary_detail_integration | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: RoleMailboxThreadLineV1 field completeness including transcription_links | CODE_SURFACES: role_mailbox.rs, api/role_mailbox.rs | TESTS: cargo test -p handshake_core role_mailbox thread_line | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
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
  - src/backend/handshake_core/src/api/role_mailbox.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test -p handshake_core
  - cargo test -p handshake_core schema_registry
  - cargo test -p handshake_core task_board
  - cargo test -p handshake_core role_mailbox
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test -p handshake_core schema_registry_updated_at -- proves updated_at is enforced in base envelope validation
  - cargo test -p handshake_core task_board_entry_spec_fields -- proves TaskBoardEntryRecordV1 includes task_board_id, lane_id, display_order, view_ids
  - cargo test -p handshake_core validation_diagnostics_structured -- proves validation returns structured payloads, not string errors
  - cargo test -p handshake_core summary_detail_integration -- proves validate_summary_detail_pair runs on actual emission paths
  - cargo test -p handshake_core role_mailbox_thread_line_completeness -- proves ThreadLineV1 includes all spec-required fields
- CANONICAL_CONTRACT_EXAMPLES:
  - Golden structured diagnostic payload JSON for schema version mismatch
  - Golden TaskBoardEntryRecordV1 JSON with all spec-required fields
  - Golden TrackedWorkPacket JSON with updated_at in base envelope
  - Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array
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
- REFINEMENT_FILE: .GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v3/refinement.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md interface TrackedWorkPacket shared structured-collaboration envelope
- CONTEXT_START_LINE: 6101
- CONTEXT_END_LINE: 6113
- CONTEXT_TOKEN: interface TrackedWorkPacket {
- EXCERPT_ASCII_ESCAPED:
  ```text
interface TrackedWorkPacket {
    // Shared structured-collaboration envelope
    schema_id: "hsk.tracked_work_packet@1";
    schema_version: "1";
    record_id: string;                   // Stable alias of wp_id
    record_kind: "work_packet";
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    authority_refs: ArtifactHandle[];
    evidence_refs: ArtifactHandle[];
    summary_record_path?: string;        // ".handshake/gov/work_packets/WP-1-Auth-System/summary.json"
    mirror_contract?: MarkdownMirrorContractV1;
    profile_extension?: ProjectProfileExtensionV1;
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md interface TrackedMicroTask shared structured-collaboration envelope
- CONTEXT_START_LINE: 6230
- CONTEXT_END_LINE: 6242
- CONTEXT_TOKEN: interface TrackedMicroTask {
- EXCERPT_ASCII_ESCAPED:
  ```text
interface TrackedMicroTask {
    // Shared structured-collaboration envelope
    schema_id: "hsk.tracked_micro_task@1";
    schema_version: "1";
    record_id: string;                   // Stable alias of mt_id
    record_kind: "micro_task";
    project_profile_kind: ProjectProfileKind;
    mirror_state: MirrorSyncState;
    authority_refs: ArtifactHandle[];
    evidence_refs: ArtifactHandle[];
    summary_record_path?: string;        // ".handshake/gov/micro_tasks/WP-1-Auth-System/MT-001/summary.json"
    mirror_contract?: MarkdownMirrorContractV1;
    profile_extension?: ProjectProfileExtensionV1;
  ```

#### ANCHOR 3
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

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md RoleMailboxIndexV1 and RoleMailboxThreadLineV1 base envelope
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
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.168] Base envelope MUST expose updated_at | WHY_IN_SCOPE: spec requires updated_at as minimum base envelope field; current ensure_schema_registry_fields_* functions do not check it | EXPECTED_CODE_SURFACES: locus/types.rs (ensure_schema_registry_fields_work_packet, ensure_schema_registry_fields_micro_task) | EXPECTED_TESTS: cargo test -p handshake_core schema_registry updated_at | RISK_IF_MISSED: base envelope validation accepts records without freshness tracking, breaking summary-first routing
  - CLAUSE: [ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids | WHY_IN_SCOPE: spec requires board-specific fields; current TaskBoardEntryRecordV1 lacks them | EXPECTED_CODE_SURFACES: locus/task_board.rs (TaskBoardEntryRecordV1), workflows.rs (board emission) | EXPECTED_TESTS: cargo test -p handshake_core task_board_entry | RISK_IF_MISSED: board projections are structurally incomplete vs spec and downstream viewers cannot sort or filter by lane/position
  - CLAUSE: [ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas | WHY_IN_SCOPE: spec requires structured mismatch output; current validation returns string errors | EXPECTED_CODE_SURFACES: locus/types.rs (validate_structured_collaboration_record, validate_profile_extension) | EXPECTED_TESTS: cargo test -p handshake_core validation_diagnostics | RISK_IF_MISSED: Command Center and downstream viewers cannot programmatically act on validation failures
  - CLAUSE: [ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs | WHY_IN_SCOPE: spec requires deterministic joins between summary and detail; no runtime integration test proves this | EXPECTED_CODE_SURFACES: workflows.rs (artifact emission), runtime_governance.rs | EXPECTED_TESTS: cargo test -p handshake_core summary_detail_integration | RISK_IF_MISSED: summary-first reads may consume mismatched records without detection
  - CLAUSE: RoleMailboxThreadLineV1 field completeness including transcription_links | WHY_IN_SCOPE: spec defines full ThreadLineV1 shape; current emit path needs verification | EXPECTED_CODE_SURFACES: role_mailbox.rs, api/role_mailbox.rs | EXPECTED_TESTS: cargo test -p handshake_core role_mailbox thread_line | RISK_IF_MISSED: mailbox exports silently drop fields required by spec
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: Structured validation diagnostic payload | PRODUCER: locus/types.rs validation functions | CONSUMER: Command Center viewer, runtime_governance.rs | SERIALIZER_TRANSPORT: serde JSON | VALIDATOR_READER: downstream viewer tests | TRIPWIRE_TESTS: cargo test -p handshake_core validation_diagnostics | DRIFT_RISK: diagnostic payload shape not stabilized; consumers expect different fields
  - CONTRACT: TaskBoardEntryRecordV1 struct | PRODUCER: workflows.rs (board emission) | CONSUMER: task_board.rs validators, Command Center viewer | SERIALIZER_TRANSPORT: serde JSON (packet.json / index.json) | VALIDATOR_READER: governance validator, storage conformance | TRIPWIRE_TESTS: cargo test -p handshake_core task_board_entry | DRIFT_RISK: field additions in struct not reflected in emission or validation
  - CONTRACT: TrackedWorkPacket base envelope | PRODUCER: workflows.rs | CONSUMER: locus/types.rs validators, summary emitter | SERIALIZER_TRANSPORT: serde JSON (packet.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core schema_registry | DRIFT_RISK: updated_at enforcement added but emission paths do not set the field
  - CONTRACT: TrackedMicroTask base envelope | PRODUCER: workflows.rs | CONSUMER: locus/types.rs validators, summary emitter | SERIALIZER_TRANSPORT: serde JSON (packet.json) | VALIDATOR_READER: validate_structured_collaboration_record | TRIPWIRE_TESTS: cargo test -p handshake_core micro_task schema_registry | DRIFT_RISK: same as TrackedWorkPacket
  - CONTRACT: StructuredCollaborationSummaryV1 | PRODUCER: workflows.rs (summary.json emission) | CONSUMER: compact_summary validator, local-model routing | SERIALIZER_TRANSPORT: serde JSON (summary.json) | VALIDATOR_READER: validate_summary_detail_pair | TRIPWIRE_TESTS: cargo test -p handshake_core summary_detail_integration | DRIFT_RISK: summary emitter silently drops fields that detail includes
  - CONTRACT: RoleMailboxThreadLineV1 | PRODUCER: role_mailbox.rs | CONSUMER: api/role_mailbox.rs, governance export | SERIALIZER_TRANSPORT: JSONL (threads/<id>.jsonl) | VALIDATOR_READER: role_mailbox_tests | TRIPWIRE_TESTS: cargo test -p handshake_core role_mailbox thread_line | DRIFT_RISK: transcription_links array omitted or typed incorrectly
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Define structured validation diagnostic type (e.g., SchemaValidationDiagnostic with field, expected, actual, severity) in locus/types.rs
  - Refactor validate_structured_collaboration_record and validate_profile_extension to return Vec<SchemaValidationDiagnostic> instead of Result<()>
  - Add updated_at check to ensure_schema_registry_fields_work_packet and ensure_schema_registry_fields_micro_task
  - Add task_board_id, lane_id, display_order, view_ids fields to TaskBoardEntryRecordV1 in task_board.rs
  - Update task board emission paths in workflows.rs to populate new fields
  - Verify and fix RoleMailboxThreadLineV1 field completeness in role_mailbox.rs including transcription_links
  - Write runtime integration test proving validate_summary_detail_pair runs on actual artifact emission paths
  - Write tests for structured diagnostic output, updated_at enforcement, TaskBoard fields, and ThreadLine completeness
- HOT_FILES:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core
  - cargo test -p handshake_core schema_registry
  - cargo test -p handshake_core task_board
  - cargo test -p handshake_core role_mailbox
- CARRY_FORWARD_WARNINGS:
  - v2 passed validator but failed operator code inspection; v3 must close gaps with actual implementation, not narrative
  - Do not widen scope beyond schema registry/validation surface; keep file-lock isolation from Loom storage
  - Structured diagnostic type should be simple and flat; do not over-engineer into a full error framework
  - TaskBoardEntryRecordV1 field additions must be backward-compatible (Optional/nullable for existing records)
  - RoleMailboxThreadLineV1 transcription_links must match spec shape exactly (Array of objects with target_kind, target_ref, target_sha256, note_redacted, note_sha256)
  - Keep product-runtime validation separate from .GOV governance-side validation
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - [ADD v02.168] updated_at enforced in base envelope validation (ensure_schema_registry_fields_*)
  - [ADD v02.168] TaskBoardEntryRecordV1 includes task_board_id, lane_id, display_order, view_ids
  - [ADD v02.168] Validation returns structured diagnostics, not string errors
  - [ADD v02.168] Summary/detail join validation runs on actual emission paths (runtime integration)
  - RoleMailboxThreadLineV1 field completeness including transcription_links
- FILES_TO_READ:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core
  - rg -n "updated_at|task_board_id|lane_id|display_order|view_ids|transcription_links|SchemaValidationDiagnostic|validate_summary_detail_pair" src/backend/handshake_core
- POST_MERGE_SPOTCHECKS:
  - Verify structured diagnostic payload is used in all validation paths, not just new ones
  - Verify TaskBoardEntryRecordV1 new fields are populated by emission paths, not just declared
  - Verify RoleMailboxThreadLineV1 transcription_links matches spec type shape
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Exact structured diagnostic payload shape has not been designed; the coder must choose a representation that is both machine-readable and stable
  - Whether existing TaskBoard emission paths set task_board_id/lane_id/display_order at all, or whether those values need to be threaded from upstream context
  - Whether RoleMailboxThreadLineV1 transcription_links is currently emitted as an empty array, partially populated, or completely absent
  - How many existing test fixtures will need updating after TaskBoardEntryRecordV1 gains new fields
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [BIG_TECH] Atlassian Jira Issue Fields docs | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-fields/ | Why: shows typed field authority reused by multiple issue and board views
  - [BIG_TECH] GitHub Projects fields docs | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://docs.github.com/en/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: shows stable project-item fields driving multiple projections and layouts
  - [OSS_DOC] Backstage descriptor format docs | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://backstage.io/docs/features/software-catalog/descriptor-format/ | Why: useful reference for a shared core envelope with bounded extension metadata
  - [GITHUB] Backstage repository | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://github.com/backstage/backstage | Why: concrete repository-scale example of descriptor-backed projections and extensibility pressure
  - [PAPER] FocusLLM paper | 2024-08-21 | Retrieved: 2026-03-19T05:55:23Z | https://arxiv.org/abs/2408.11745 | Why: supports compact-summary-first loading for smaller local models before detail hydration
- RESEARCH_SYNTHESIS:
  - Handshake should keep one field-authoritative collaboration record family and let board, queue, mailbox, and viewer surfaces remain projections over that family.
  - The shared envelope should stay intentionally small and stable while project-specific payloads move behind explicit extension schemas and compatibility checks.
  - Summary artifacts should be first-read surfaces for smaller local models and operator triage, with canonical detail loaded only when required.
  - Strong registry behavior is not just about naming schema ids; it also needs deterministic incompatibility reporting so future kernels do not guess across unknown profile extensions.
  - Structured validation diagnostics (not just string errors) are what make downstream Command Center and viewer surfaces actionable.
- GITHUB_PROJECT_DECISIONS:
  - backstage/backstage -> ADAPT (NONE)
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_VERDICT: PASS
- SOURCE_SCAN_DECISIONS:
  - Atlassian Jira Issue Fields docs -> ADOPT (IN_THIS_WP)
  - GitHub Projects fields docs -> ADOPT (IN_THIS_WP)
  - Backstage descriptor format docs -> ADAPT (IN_THIS_WP)
  - Backstage repository -> REJECT (REJECT_DUPLICATE)
  - FocusLLM paper -> ADAPT (IN_THIS_WP)
- MATRIX_GROWTH_CANDIDATES:
  - Shared base envelope plus compact summary pairing with structured diagnostics -> IN_THIS_WP (stub: NONE)
  - Base descriptor plus bounded extension compatibility with updated_at -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep schema id/version constants and compatibility policy in one registry surface instead of scattering them across emitters.
  - Validate summary/detail shared identity and authority refs mechanically before allowing summary-first reads.
  - Separate base-envelope validation from extension validation so unknown extensions never force parser forks or silent fallback.
  - Return structured validation diagnostic payloads (not just Result<()> with strings) so downstream surfaces can act on specific failures.
  - Enforce updated_at in base envelope validation alongside schema_id, record_id, record_kind, project_profile_kind, mirror_state.
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-MirrorSyncState
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-MirrorSyncState
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
  - Locus tracked record families plus one shared registry with structured diagnostics -> IN_THIS_WP (stub: NONE)
  - Work Packet detail plus compact summary join validation with updated_at -> IN_THIS_WP (stub: NONE)
  - Micro-Task detail plus compact summary join validation -> IN_THIS_WP (stub: NONE)
  - Task Board row, index, and view validation with missing spec fields -> IN_THIS_WP (stub: NONE)
  - Command Center diagnostics over structured registry mismatch results -> IN_THIS_WP (stub: NONE)
  - Role Mailbox export validation with complete ThreadLineV1 fields -> IN_THIS_WP (stub: NONE)
  - Schema-version mismatch structured diagnostics at the parser boundary -> IN_THIS_WP (stub: NONE)
  - Profile-extension compatibility gating over canonical records -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: Shared collaboration base-envelope validation with updated_at | SUBFEATURES: Work Packet, Micro-Task, Task Board record identity and compatibility checks; updated_at enforcement in base envelope | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the registry must validate one field-equivalent envelope including updated_at across the main Locus-owned record families
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Canonical packet and summary schema registration with structured diagnostics | SUBFEATURES: `packet.json`, `summary.json`, compatibility readers, profile-extension enforcement, structured mismatch output | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: packet and summary validation must share ids, authorities, and extension policy; mismatch output must be structured, not string-only
  - PILLAR: MicroTask | CAPABILITY_SLICE: Canonical micro-task packet and summary schema registration | SUBFEATURES: `packet.json`, `summary.json`, compatibility readers, machine-readable mismatch results | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task routing and bounded execution depend on the same registry guarantees as work packets
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Structured projection schema registration with missing fields | SUBFEATURES: `index.json`, `views/{view_id}.json`, row validation, task_board_id/lane_id/display_order/view_ids fields, shared summary joins | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1, PRIM-MirrorSyncState | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: TaskBoardEntryRecordV1 must gain spec-required board-specific fields
  - PILLAR: Command Center | CAPABILITY_SLICE: Registry-driven structured validation diagnostics | SUBFEATURES: unknown-schema, incompatible-extension, updated_at-missing, and summary-drift outputs as structured payloads consumable by generic viewers | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-StructuredCollaborationSummaryV1, FEAT-DEV-COMMAND-CENTER | MECHANICAL: engine.context, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the packet should emit deterministic structured validator outputs that the viewer packet can consume later
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact-summary-first compatibility enforcement with updated_at | SUBFEATURES: shared identity and authority refs across detail and summary records; updated_at as base envelope requirement | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationSummaryV1, PRIM-MirrorSyncState, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: small-model reads must not guess across mismatched summaries or unknown extensions; updated_at enables freshness-based routing
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Shared collaboration schema registry with structured diagnostics | JobModel: WORKFLOW | Workflow: locus_structured_artifact_publish | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: registry outputs should be visible as structured diagnostic payloads to both generic viewers and runtime artifact producers
  - Capability: Compact summary compatibility validation | JobModel: WORKFLOW | Workflow: compact_summary_emit | ToolSurface: COMMAND_CENTER | ModelExposure: LOCAL | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: summary-first local-model routing depends on deterministic summary/detail compatibility checks with runtime integration proof
  - Capability: Task Board structured projection validation with missing fields | JobModel: WORKFLOW | Workflow: task_board_projection_publish | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: board projections need spec-required fields (task_board_id, lane_id, display_order, view_ids) and structured validation diagnostics
  - Capability: Role Mailbox export schema validation | JobModel: WORKFLOW | Workflow: role_mailbox_export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox export validation must verify RoleMailboxThreadLineV1 field completeness including transcription_links
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Project-Profile-Extension-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Markdown-Mirror-Sync-Drift-Guard-v1 -> KEEP_SEPARATE
  - WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Artifact-Family-v1 -> KEEP_SEPARATE
  - WP-1-Role-Mailbox-v1 -> KEEP_SEPARATE
  - WP-1-Locus-Phase1-Integration-Occupancy-v1 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Schema-Registry-v2 -> EXPAND_IN_THIS_WP
- CODE_REALITY_SUMMARY:
  - src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Structured-Collaboration-Schema-Registry-v2)
  - src/backend/handshake_core/src/locus/types.rs -> IMPLEMENTED (WP-1-Structured-Collaboration-Schema-Registry-v2)
  - src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Structured-Collaboration-Artifact-Family-v1)
  - src/backend/handshake_core/src/locus/task_board.rs -> PARTIAL (WP-1-Structured-Collaboration-Artifact-Family-v1)
  - src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Structured-Collaboration-Artifact-Family-v1)
  - src/backend/handshake_core/src/runtime_governance.rs -> PARTIAL (NONE)
  - src/backend/handshake_core/src/role_mailbox.rs -> PARTIAL (WP-1-Role-Mailbox-v1)
  - src/backend/handshake_core/src/api/role_mailbox.rs -> PARTIAL (WP-1-Role-Mailbox-v1)
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
- What: Implement structured diagnostic output for schema validation, add missing TaskBoardEntryRecordV1 fields (task_board_id, lane_id, display_order, view_ids), enforce updated_at in base envelope, prove validate_summary_detail_pair runtime integration, and verify RoleMailboxThreadLineV1 field completeness.
- Why: v2 passed validator but operator code inspection against spec v02.178 revealed string-only validation errors, missing board-projection fields, no updated_at enforcement, unproven summary/detail runtime integration, and incomplete ThreadLine fields. This v3 closes those concrete spec compliance gaps.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - Loom storage, search, source-anchor, and asset portability
  - frontend Command Center viewer implementation and layout UX
  - project-profile-specific extension payload design beyond compatibility hooks and validation boundaries
  - Markdown mirror reconciliation controllers and overwrite policy
  - governance-only `.GOV` mailbox ledgers or session-control schemas
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test -p handshake_core
  just gov-check
```

### DONE_MEANS
- `validate_structured_collaboration_record()` and related validation functions return structured diagnostic payloads (with field name, expected value, actual value, severity) instead of string-only errors.
- `TaskBoardEntryRecordV1` includes `task_board_id`, `lane_id`, `display_order`, and `view_ids` fields matching spec [ADD v02.168].
- `ensure_schema_registry_fields_work_packet` and `ensure_schema_registry_fields_micro_task` enforce `updated_at` as a required base envelope field.
- Runtime integration test proves `validate_summary_detail_pair` runs on actual artifact emission paths in `workflows.rs`.
- `RoleMailboxThreadLineV1` emit path includes all spec-required fields including `transcription_links` array.
- The packet keeps product-runtime artifact authority distinct from governance-side `.GOV` control-plane ledgers and validators.

- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-MirrorSyncState
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
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-03-19T08:32:11.929Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.168]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md Base structured schema and project-profile extension contract [ADD v02.168]
- Codex: .GOV/codex/Handshake_Codex_v1.4.md
- Task Board: .GOV/roles_shared/records/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Roadmap/spec/repo audit basis:
  - Base WP traceability anchor: `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` marks `WP-1-Structured-Collaboration-Schema-Registry-v3` as the active remediation packet for `WP-1-Structured-Collaboration-Schema-Registry`.
  - Governing Main Body scope is unchanged across variants: shared structured-collaboration envelope, project-profile extension compatibility, summary/detail identity joins, task-board projection schema, and role-mailbox thread/index schema obligations remain mandatory.
  - Current repo-code audit baseline for v3 is the signed refinement plus direct inspection of current `main`/merge-base state; prior validator PASS claims are treated as non-authoritative where code/spec inspection disproved closure.
- Prior packet: `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v1.md`
  - Preserved into v3: the base WP intent to make the shared structured-collaboration registry authoritative across Work Packet, Micro-Task, Task Board, and Role Mailbox artifact families.
  - Preserved into v3: the requirement that summary/detail artifacts, profile extensions, and mailbox exports validate against one shared contract rather than ad hoc emitter-specific rules.
  - Changed in v3: v1 closure claims are no longer trusted as sufficient proof because later direct code inspection found missing structured diagnostics, missing `updated_at` enforcement, missing task-board fields, missing summary/detail runtime integration proof, and incomplete mailbox thread-line proof.
- Prior packet: `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v2.md`
  - Preserved into v3: the remediation posture against the v1 false-closeout and the shared-surface focus on `locus/types.rs`, task-board projections, workflow/runtime artifact emission, and mailbox artifacts.
  - Changed in v3: the missing requirements are promoted into explicit clause-closure rows and semantic-proof assets so the coder/validator cannot rely on narrative pass claims alone.
  - Changed in v3: the signed refinement narrows the active repo-code gap set to five concrete unclosed obligations: machine-readable schema diagnostics, `TaskBoardEntryRecordV1` spec fields, base-envelope `updated_at`, summary/detail integration proof on real emission paths, and `RoleMailboxThreadLineV1` completeness including `transcription_links`.
- Carry-forward verdict:
  - No prior governing requirement is dropped in v3.
  - v3 supersedes v1/v2 closure claims but preserves their valid scope while replacing under-proven or incorrect completion assumptions with explicit current-main gap statements and test/example obligations.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
- SEARCH_TERMS:
  - schema_id
  - schema_version
  - project_profile_kind
  - mirror_state
  - authority_refs
  - evidence_refs
  - summary.json
  - profile_extension
  - role_mailbox_index
  - role_mailbox_thread_line
  - updated_at
  - task_board_id
  - lane_id
  - display_order
  - view_ids
  - transcription_links
  - validate_structured_collaboration_record
  - validate_summary_detail_pair
- RUN_COMMANDS:
  ```bash
rg -n "schema_id|schema_version|project_profile_kind|mirror_state|updated_at|task_board_id|lane_id|display_order|view_ids|transcription_links|validate_structured_collaboration_record|validate_summary_detail_pair" src/backend/handshake_core
  cargo test -p handshake_core
  just gov-check
  ```
- RISK_MAP:
  - "structured diagnostics widen into a full error-framework redesign" -> "scope creep beyond the spec-required machine-readable mismatch output"
  - "TaskBoardEntryRecordV1 field additions break existing board emitters" -> "current task-board emission paths produce invalid records"
  - "updated_at enforcement fails existing records that lack the field" -> "migration or data-repair needed before validation can pass"
  - "summary/detail validation integration test is fragile" -> "false confidence in runtime coverage if test is too narrow"
  - "runtime and governance mailbox paths remain conflated" -> "the packet validates the wrong authority surface and hides real product regressions"
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
  - LOG_PATH: `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v3/<name>.log` (recommended; not committed)
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
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.
