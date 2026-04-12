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
- CODER_RESUME_COMMAND: just coder-next WP-1-Structured-Collaboration-Schema-Registry-v3
<!-- Validator roles keep distinct local branches/worktrees, but they mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-schema-registry-v3
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Structured-Collaboration-Schema-Registry-v3
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v3
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Structured-Collaboration-Schema-Registry-v3
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Structured-Collaboration-Schema-Registry-v3
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Structured-Collaboration-Schema-Registry-v3
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
- **Status:** Blocked
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
- WP_VALIDATOR_OF_RECORD: wp_validator:wp-1-structured-collaboration-schema-registry-v3
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- USER_SIGNATURE: ilja190320260923
- PACKET_FORMAT_VERSION: 2026-03-18

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: FAIL
Blockers: LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED. The 2026-03-21 product-vs-spec audit found the v3 closure under-proved on shared validator enforcement, and current governance law treats this packet as blocked historical evidence rather than live validated closure.
Next: NONE. Do not resume or re-prepare this packet in place. Remediation, if required, moves to `WP-1-Structured-Collaboration-Schema-Registry-v4`.

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: [ADD v02.168] Base envelope MUST expose updated_at | CODE_SURFACES: locus/types.rs (ensure_schema_registry_fields_work_packet, ensure_schema_registry_fields_micro_task) | TESTS: cargo test -p handshake_core schema_registry updated_at | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: [ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids | CODE_SURFACES: locus/task_board.rs (TaskBoardEntryRecordV1), workflows.rs (board emission) | TESTS: cargo test -p handshake_core task_board_entry | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: [ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas | CODE_SURFACES: locus/types.rs (validate_structured_collaboration_record, validate_profile_extension) | TESTS: cargo test -p handshake_core validation_diagnostics | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: [ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs | CODE_SURFACES: workflows.rs (artifact emission), runtime_governance.rs, storage/locus_sqlite.rs (locus progress serialization) | TESTS: cargo test -p handshake_core summary_detail_integration | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: RoleMailboxThreadLineV1 field completeness including transcription_links | CODE_SURFACES: role_mailbox.rs, api/role_mailbox.rs | TESTS: cargo test -p handshake_core role_mailbox thread_line | EXAMPLES: Golden structured diagnostic payload JSON for schema version mismatch, Golden TaskBoardEntryRecordV1 JSON with all spec-required fields, Golden TrackedWorkPacket JSON with updated_at in base envelope, Golden RoleMailboxThreadLineV1 JSONL line with transcription_links array | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
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
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
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
- What: Implement structured diagnostic output for schema validation, add missing TaskBoardEntryRecordV1 fields (task_board_id, lane_id, display_order, view_ids), enforce updated_at in base envelope, prove validate_summary_detail_pair runtime integration, preserve legacy-compatible micro-task progress payload shape at the locus storage boundary, and verify RoleMailboxThreadLineV1 field completeness.
- Why: v2 passed validator but operator code inspection against spec v02.178 revealed string-only validation errors, missing board-projection fields, no updated_at enforcement, unproven summary/detail runtime integration, and incomplete ThreadLine fields. This v3 closes those concrete spec compliance gaps.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
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
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
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
- Proposed interfaces/types/contracts: Keep `validate_structured_collaboration_record()` as the shared base-envelope enforcement point, require `updated_at` there, and keep work-packet/micro-task producer parity in runtime artifact emission so both packet and summary/detail outputs expose the same base-envelope freshness field. Extend the test surface with a negative-path regression for missing `updated_at`, then continue later MTs through structured diagnostics, Task Board projection completeness, summary/detail shared-field parity, and RoleMailbox thread-line completeness.
- Open questions: The MT wording still references `ensure_schema_registry_fields_work_packet` / `ensure_schema_registry_fields_micro_task`, but the live enforcement surface appears to be `validate_structured_collaboration_record(...)` plus the runtime artifact emitters in `workflows.rs`. Keep the implementation anchored to the live enforcement path unless a stricter spec/code anchor emerges during implementation.
- Notes: First executable MT is `MT-001 [ADD v02.168] Base envelope MUST expose updated_at`. Validator tripwires for the first pass are producer parity, negative-path proof, and preventing `updated_at` from drifting into profile-extension-only handling.

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
- Added runtime structured-collaboration backfill on emitted work-packet and micro-task artifacts so the shared base envelope is normalized before summary/detail artifacts are written, including a guaranteed `updated_at` on tracked work packets and packet/summary parity validation on the live emission path.
- Reworked task-board structured projection output to the spec-facing contract: entry/index/view records now keep required arrays serialized, use the canonical `default` view id, emit `rows` and `lane_ids` instead of the old `entries`/`lanes` shape, and validate runtime authority refs before artifact write.
- Extended the micro-task executor regression surface with explicit positive/negative-path checks for work-packet `updated_at`, task-board artifact existence, and task-board row/view field coverage (`task_board_id`, `lane_id`, `display_order`, `view_ids`, `lane_ids`).
- Re-audited the Role Mailbox thread-line clause against the current branch head. No product diff was required there because the existing export/runtime path already preserves `transcription_links` and the current `role_mailbox_tests` still prove that contract.

## HYGIENE
- Captured deterministic COR-701 LF-blob SHA pairs for every staged product file with `just cor701-sha ...`.
- Re-ran targeted Rust tests from `src/backend/handshake_core` and stored full logs under `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v3/`.
- Kept the product diff scoped to the 4 packet-tracked files in `../wtc-schema-registry-v3`; governance edits remain in the shared kernel and are not part of the feature-branch product change set.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/locus/types.rs`
- **Start**: 164
- **End**: 249
- **Line Delta**: 0
- **Pre-SHA1**: `a6e4ffbdcee4d31ed15272ea03d99d8f5f88d3af`
- **Post-SHA1**: `ce48d67cf815ac8bfb8c11184b5b4f301f4750b2`
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
- **Lint Results**: Focused schema-registry integration tests plus role-mailbox exact checks
- **Artifacts**: `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v3/packet.md`; `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v3/micro_task_executor_tests.log`
- **Timestamp**: 2026-03-20T14:38:57.2258683+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Keeps empty `authority_refs` / `evidence_refs` materialized on structured summaries and legacy packet artifacts so validator-facing base-envelope fields stay explicit.
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1598
- **End**: 11768
- **Line Delta**: 97
- **Pre-SHA1**: `89b7c353688f43a33b4cb6c19c3ab7b370c56599`
- **Post-SHA1**: `a77b1a14aad10787f4fb5b4c8347de5b8ec484e2`
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
- **Lint Results**: Focused schema-registry integration tests plus role-mailbox exact checks
- **Artifacts**: `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v3/packet.md`; `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v3/micro_task_executor_tests.log`
- **Timestamp**: 2026-03-20T14:38:57.2258683+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Restores legacy packet-detail emission for work packets and micro-tasks while preserving the shared-envelope runtime backfill and validation path.
- **Target File**: `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- **Start**: 23
- **End**: 1388
- **Line Delta**: 113
- **Pre-SHA1**: `cf81e812af85938904cb23f6e1383098d546a9ad`
- **Post-SHA1**: `0c396ecceeec0e74dc726aaa887e95c9d74d8af5`
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
- **Lint Results**: Covered by focused schema-registry integration tests and machine-readable diagnostic exact tests
- **Artifacts**: `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v3/packet.md`; `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v3/micro_task_executor_tests.log`
- **Timestamp**: 2026-03-20T14:38:57.2258683+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Adds legacy-artifact deserialization assertions plus exact regression proof for summary/detail packet compatibility and structured validation diagnostics.
- **Target File**: `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Start**: 14
- **End**: 1134
- **Line Delta**: 119
- **Pre-SHA1**: `3cd7ba131365f6ba78462737cdabee571cda95d2`
- **Post-SHA1**: `a3bdbe81c302f8fdbefd260bff808c12b2181ee8`
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
- **Lint Results**: Covered by the full `micro_task_executor_tests` suite, which exercised the bind/unbind and lifecycle progress APIs against the restored legacy-compatible micro-task artifact view.
- **Artifacts**: `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v3/packet.md`; `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v3/micro_task_executor_tests.log`
- **Timestamp**: 2026-03-20T17:24:11.0000000+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Restores the locus progress/bind-session boundary to emit a legacy-compatible tracked micro-task artifact shape, including explicit `summary_ref`, workflow fields, and empty `active_session_ids` arrays when no session remains bound.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: Done; committed handoff validation passed at `23f4c9a`, validator gates are closed PASS, the branch is pushed to `origin/feat/WP-1-Structured-Collaboration-Schema-Registry-v3`, and the finalized Schema Registry product scope is integrated on `main` via selective commit `fe998e1`.
- What changed in this update: Implemented runtime structured-collaboration backfill for emitted work-packet artifacts, corrected task-board projection contract details (`rows`, `lane_ids`, canonical `default` view id), and added targeted negative/positive-path regression coverage for `updated_at` and task-board artifacts.
- Requirements / clauses self-audited: `[ADD v02.168] Base envelope MUST expose updated_at` -> staged diff plus negative-path proof added; `[ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids` -> staged struct/emission/test alignment added; `[ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas` -> current validation path re-used and task-board/work-packet runtime validation tightened on live emission; `[ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs` -> live emission path now builds packet/summary from the same normalized tracked record; `RoleMailboxThreadLineV1 field completeness including transcription_links` -> re-audited on branch head through existing runtime/tests, no product diff required.
- Checks actually run: `cargo test --test micro_task_executor_tests -- --nocapture`; `cargo test --test role_mailbox_tests -- --nocapture`
- Known gaps / weak spots: The packet does not add a new dedicated unit test named exactly after every clause; some proof is integration-style and the Role Mailbox clause still relies on existing branch-head coverage rather than a fresh diff in that surface.
- Heuristic risks / maintainability concerns: `workflows.rs` now owns more structured-artifact normalization and validation logic, so future emitters can drift again if they bypass `apply_runtime_structured_work_packet_registry(...)` / `apply_runtime_structured_micro_task_registry(...)`.
- Validator focus request: Adversarially test missing `updated_at`, task-board projection contract drift (`default` vs `by_status`, `rows`/`lane_ids` vs legacy `entries`/`lanes`), and summary/detail parity for `record_id`, `project_profile_kind`, and `authority_refs`; also confirm the Role Mailbox completeness clause is still live on current branch head.
- Rubric contract understanding proof: This packet is not closed by green tests alone; a pass requires file:line evidence for each diff-scoped clause plus at least one negative-path proof on the shared structured-collaboration contract.
- Rubric scope discipline proof: The staged product diff is limited to 4 packet-tracked files under `src/backend/handshake_core`; Role Mailbox proof references branch-head code/tests only and does not widen the implementation diff.
- Rubric baseline comparison: Before this diff, emitted tracked work packets could miss a fresh base-envelope `updated_at` on artifact write and task-board structured projections still serialized legacy `entries`/`lanes` with `by_status`; after this diff, emitted artifacts and tests align to `updated_at`, `rows`, `lane_ids`, `view_ids`, and `default`.
- Rubric end-to-end proof: `locus_create_and_close_wp_emit_structured_work_packet_packet_and_summary` now proves emitted packet artifacts carry RFC3339 `updated_at` and fail negative-path validation when that field is removed; `locus_sync_task_board_emits_structured_index_and_view` proves the generated task-board artifacts exist and expose the spec-required projection fields.
- Rubric architecture fit self-review: The change stays on the shared runtime artifact-emission and structured-validation path instead of adding packet-local schema exceptions or test-only shape shims.
- Rubric heuristic quality self-review: The strongest design risk is centralizing more contract logic in `workflows.rs`; the compensating guard is that the tests now assert both emitted artifact shape and validator rejection of missing required envelope data.
- Rubric anti-gaming / counterfactual check: If `tracked_wp.updated_at = Utc::now()` at `src/backend/handshake_core/src/locus/types.rs:1223` or `apply_runtime_structured_work_packet_registry(...)` in `src/backend/handshake_core/src/workflows.rs:11586` is removed, the `updated_at` assertions at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:749` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:791` fail. If task-board emission falls back to legacy `entries`/`lanes` or `by_status`, the assertions at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:934`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:942`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:957` fail.
- Next step / handoff hint: NONE. WP is closed; downstream structured-collaboration packets may now consume the completed schema-registry contract.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "[ADD v02.168] Base envelope MUST expose updated_at"
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1223; src/backend/handshake_core/src/workflows.rs:11586; src/backend/handshake_core/tests/micro_task_executor_tests.rs:749; src/backend/handshake_core/tests/micro_task_executor_tests.rs:791`
  - REQUIREMENT: "[ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids"
  - EVIDENCE: `src/backend/handshake_core/src/locus/task_board.rs:44; src/backend/handshake_core/src/locus/task_board.rs:46; src/backend/handshake_core/src/locus/task_board.rs:47; src/backend/handshake_core/src/locus/task_board.rs:49; src/backend/handshake_core/src/workflows.rs:3588; src/backend/handshake_core/src/workflows.rs:3590; src/backend/handshake_core/src/workflows.rs:3591; src/backend/handshake_core/src/workflows.rs:3592; src/backend/handshake_core/tests/micro_task_executor_tests.rs:934; src/backend/handshake_core/tests/micro_task_executor_tests.rs:938; src/backend/handshake_core/tests/micro_task_executor_tests.rs:942; src/backend/handshake_core/tests/micro_task_executor_tests.rs:945`
  - REQUIREMENT: "[ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas"
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1138; src/backend/handshake_core/src/workflows.rs:3647; src/backend/handshake_core/src/workflows.rs:3673; src/backend/handshake_core/src/workflows.rs:3692`
  - REQUIREMENT: "[ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs"
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11609; src/backend/handshake_core/src/workflows.rs:11630; src/backend/handshake_core/tests/micro_task_executor_tests.rs:765`
  - REQUIREMENT: "RoleMailboxThreadLineV1 field completeness including transcription_links"
  - EVIDENCE: `src/backend/handshake_core/src/role_mailbox.rs:252; src/backend/handshake_core/src/role_mailbox.rs:533; src/backend/handshake_core/src/role_mailbox.rs:933; src/backend/handshake_core/tests/role_mailbox_tests.rs:289; src/backend/handshake_core/tests/role_mailbox_tests.rs:359`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `cargo test --test micro_task_executor_tests -- --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v3/micro_task_executor_tests.log`
  - LOG_SHA256: `445FEB30D252E6578CBD0951EC9A5291D9707F935F472D7192FC6DB188E4E004`
  - PROOF_LINES: `test locus_register_mts_emits_structured_micro_task_packet_and_summary ... ok`; `test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 82.87s`
  - COMMAND: `cargo test --test role_mailbox_tests -- --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v3/role_mailbox_tests.log`
  - LOG_SHA256: `128D79962FEABB71F947CDBBEC755F04EA470A97AF572E076988B00DE9AEECCD`
  - PROOF_LINES: `test role_mailbox_index_api_returns_valid_structured_export ... ok`; `test role_mailbox_create_message_emits_events_and_export ... ok`; `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.74s`

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

### WP_VALIDATOR Advisory Report - 2026-03-20T13:56:53.0696714+01:00
- REVIEW_SCOPE: Advisory diff-scoped review of commit `59ae3393f6b37b9b2c3712ee8930ad73fe67d0de` versus first parent `317c09d7134ad86475e19732313ea8d285f45888`, with current packet as authority
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS
- TEST_VERDICT: PASS
- CODE_REVIEW_VERDICT: PARTIAL
- HEURISTIC_REVIEW_VERDICT: PARTIAL
- SPEC_ALIGNMENT_VERDICT: PARTIAL
- ENVIRONMENT_VERDICT: PASS
- DISPOSITION: NONE
- LEGAL_VERDICT: PASS
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- VALIDATOR_RISK_TIER: HIGH
- ADVISORY_VERDICT: PARTIAL
- FINDINGS_COUNT: 1
- CLAUSES_REVIEWED:
  - `[ADD v02.168] Base envelope MUST expose updated_at` -> `src/backend/handshake_core/src/locus/types.rs:1068`, `src/backend/handshake_core/src/locus/types.rs:1223`, `src/backend/handshake_core/src/workflows.rs:11586`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:780`
  - `[ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids` -> `src/backend/handshake_core/src/locus/task_board.rs:44`, `src/backend/handshake_core/src/locus/task_board.rs:70`, `src/backend/handshake_core/src/workflows.rs:3558`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:925`
  - `[ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas` -> `src/backend/handshake_core/src/locus/types.rs:1030`, `src/backend/handshake_core/src/locus/types.rs:1629`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1399`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1434`
  - `[ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs` -> `src/backend/handshake_core/src/locus/types.rs:1138`, `src/backend/handshake_core/src/workflows.rs:11624`, `src/backend/handshake_core/src/workflows.rs:11718`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:809`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1289`
  - `RoleMailboxThreadLineV1 field completeness including transcription_links` -> `src/backend/handshake_core/src/role_mailbox.rs:933`, `src/backend/handshake_core/src/api/role_mailbox.rs:35`, `src/backend/handshake_core/src/locus/types.rs:1128`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:261`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:541`
- NOT_PROVEN:
  - The commit does not fully prove packet-detail compatibility beyond the shared structured-collaboration envelope after replacing `TrackedWorkPacketArtifactV1` / `TrackedMicroTaskArtifactV1` serialization with raw `TrackedWorkPacket` / `TrackedMicroTask` writes in `src/backend/handshake_core/src/workflows.rs:3385` and `src/backend/handshake_core/src/workflows.rs:3412`; validator-owned coverage in this handoff does not pin the removed legacy packet fields.
- MAIN_BODY_GAPS:
  - Canonical detail artifact compatibility outside the shared envelope is still under-proven after the serializer swap in `src/backend/handshake_core/src/workflows.rs:3385` and `src/backend/handshake_core/src/workflows.rs:3412`; this packet closes the shared-envelope clauses, but not the broader historical packet-detail field surface.
- QUALITY_RISKS:
  - `src/backend/handshake_core/src/workflows.rs:3385-3443` changes the emitted packet JSON contract materially while legacy artifact types and helpers remain on disk at `src/backend/handshake_core/src/locus/types.rs:185-264` and `src/backend/handshake_core/src/workflows.rs:2573-3322`; the branch now has two competing packet-shape stories and no validator-owned golden/consumer test covering the dropped legacy fields.
- DIFF_ATTACK_SURFACES:
  - Work-packet packet emission changed from dedicated artifact struct serialization to raw tracked-record serialization.
  - Micro-task packet emission changed from dedicated artifact struct serialization to raw tracked-record serialization.
  - Task-board projection contract changed from legacy `entries` / `lanes` + `by_status` semantics to `rows` / `lane_ids` + `default` view semantics.
  - Runtime authority-scope enforcement now sits on emitted structured artifact refs for work packets, micro-tasks, and task-board projections.
  - Full-packet closure still depends on an unchanged Role Mailbox clause proven on current branch head rather than in this diff.
- INDEPENDENT_CHECKS_RUN:
  - `cargo test --test micro_task_executor_tests locus_create_and_close_wp_emit_structured_work_packet_packet_and_summary -- --exact --nocapture` => passed; emitted work-packet artifact carried RFC3339 `updated_at`, and the test's validator-owned negative path rejected a packet with `updated_at` removed.
  - `cargo test --test micro_task_executor_tests locus_sync_task_board_emits_structured_index_and_view -- --exact --nocapture` => passed; emitted task-board index/view artifacts validated with `rows`, `lane_ids`, `view_ids`, and canonical `default` view id.
  - `cargo test --test micro_task_executor_tests locus_register_mts_emits_structured_micro_task_packet_and_summary -- --exact --nocapture` => passed; micro-task packet and summary still form a valid structured-collaboration pair after the emitter rewrite.
  - `cargo test --test micro_task_executor_tests locus_register_mts_returns_machine_readable_validation_for_unknown_schema_version -- --exact --nocapture` => passed; machine-readable `schema_version_mismatch` diagnostics were observed on the micro-task registration path.
  - `cargo test --test micro_task_executor_tests locus_register_mts_returns_machine_readable_validation_for_incompatible_profile_extension -- --exact --nocapture` => passed; machine-readable `incompatible_profile_extension` diagnostics were observed on the micro-task registration path.
  - `cargo test --test role_mailbox_tests role_mailbox_create_message_emits_events_and_export -- --exact --nocapture` => passed; branch-head mailbox export still emitted thread lines with `transcription_links`.
  - `cargo test --test role_mailbox_tests role_mailbox_index_api_returns_valid_structured_export -- --exact --nocapture` => passed; branch-head mailbox index export still validated as structured output.
  - `rg -n 'by_status|default_view_id\\(|task_board_view:' src/backend/handshake_core` => only the new `default_view_id` producer path remained in-repo; no stale `by_status` consumer surfaced under `src/backend/handshake_core`.
- COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/workflows.rs:11586` `apply_runtime_structured_work_packet_registry(...)` were bypassed or removed, the emitted work-packet detail/summary pair would lose backfill and join-validation guarantees, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:780-815` would fail.
  - If `src/backend/handshake_core/src/workflows.rs:3490` `emit_task_board_projection_artifacts(...)` reverted to legacy `entries` / `lanes` output or `src/backend/handshake_core/src/locus/task_board.rs:167` changed view-id semantics inconsistently, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:911-965` would fail and `src/backend/handshake_core/src/locus/types.rs:1093-1111` would reject the emitted shape.
  - If `src/backend/handshake_core/src/role_mailbox.rs:933-957` stopped mapping `transcription_links` into exported thread lines, `src/backend/handshake_core/tests/role_mailbox_tests.rs:359` would fail and `src/backend/handshake_core/src/locus/types.rs:1128-1129` would reject the export.
- BOUNDARY_PROBES:
  - Producer/validator boundary: `src/backend/handshake_core/src/workflows.rs:3558-3707` emits task-board entry/index/view JSON and immediately validates it through `src/backend/handshake_core/src/locus/types.rs:1030-1111`; the focused exact task-board test confirmed both sides agree on `rows`, `lane_ids`, and `view_ids`.
  - Detail/summary boundary: `src/backend/handshake_core/src/workflows.rs:11604-11646` and `src/backend/handshake_core/src/workflows.rs:11718-11740` feed emitted detail + summary JSON through `src/backend/handshake_core/src/locus/types.rs:1138-1179`; the focused exact work-packet and micro-task tests confirmed join parity on `record_id`, `project_profile_kind`, and `authority_refs`.
- NEGATIVE_PATH_CHECKS:
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:786-800` removes `updated_at` from emitted work-packet JSON; validation fails with `MissingField` on `updated_at`.
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1399-1427` forces an unknown schema version; validation fails with machine-readable `schema_version_mismatch`.
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1434-1468` forces an incompatible profile extension; validation fails with machine-readable `incompatible_profile_extension`.
- INDEPENDENT_FINDINGS:
  - The packet's five named clauses are independently reproducible on current head with validator-owned focused probes; the handoff is not relying solely on the coder's broad binary-level test runs.
  - The task-board contract propagation looks internally clean under `src/backend/handshake_core`; no stale `by_status` consumer or `lanes` / `entries` task-board reader remained in the searched product code.
  - The diff also performs a broader packet-emitter contract pivot than the packet clauses themselves demand, and that broader compatibility surface is not pinned by dedicated validator-owned consumer or golden tests.
- RESIDUAL_UNCERTAINTY:
  - This is a `HIGH`-risk serialized-output packet, and the review did not find validator-owned coverage proving that any out-of-tree or historical packet readers tolerate the switch from legacy artifact structs to raw tracked-record serialization.
  - The exact tests here prove current in-repo producers and validators agree; they do not eliminate risk for previously emitted packet-detail fields that are no longer serialized.
- SPEC_CLAUSE_MAP:
  - `[ADD v02.168] Base envelope MUST expose updated_at` -> `src/backend/handshake_core/src/locus/types.rs:1068`; `src/backend/handshake_core/src/locus/types.rs:1223`; `src/backend/handshake_core/src/workflows.rs:11586-11646`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:780-800`
  - `[ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids` -> `src/backend/handshake_core/src/locus/task_board.rs:44-49`; `src/backend/handshake_core/src/locus/task_board.rs:70-99`; `src/backend/handshake_core/src/workflows.rs:3558-3644`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:925-965`
  - `[ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas` -> `src/backend/handshake_core/src/locus/types.rs:1030-1072`; `src/backend/handshake_core/src/locus/types.rs:1629-1675`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1399-1468`
  - `[ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs` -> `src/backend/handshake_core/src/locus/types.rs:1138-1179`; `src/backend/handshake_core/src/workflows.rs:11624-11646`; `src/backend/handshake_core/src/workflows.rs:11718-11740`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:809-815`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1289-1295`
  - `RoleMailboxThreadLineV1 field completeness including transcription_links` -> `src/backend/handshake_core/src/role_mailbox.rs:933-957`; `src/backend/handshake_core/src/api/role_mailbox.rs:35-114`; `src/backend/handshake_core/src/locus/types.rs:1128-1129`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:261-359`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:541-565`
- NEGATIVE_PROOF:
  - The packet does not fully prove canonical detail-artifact compatibility beyond the shared structured-collaboration envelope after `src/backend/handshake_core/src/workflows.rs:3385-3443` and `src/backend/handshake_core/src/workflows.rs:3412-3443` switched packet writes away from `TrackedWorkPacketArtifactV1` / `TrackedMicroTaskArtifactV1`; the legacy packet artifact types and related helper surface remain declared at `src/backend/handshake_core/src/locus/types.rs:185-264` and `src/backend/handshake_core/src/workflows.rs:2573-3322`, but no validator-owned compatibility test in this handoff proves the dropped legacy packet fields are safe to remove from emitted JSON.

### WP_VALIDATOR Advisory Report - 2026-03-20T15:20:42.5401475+01:00 - commit 23f4c9a
- REVIEW_SCOPE: Advisory diff-scoped review of commit `23f4c9ae148780e7c1a5728091fa41af7eb58325` versus first parent `59ae3393f6b37b9b2c3712ee8930ad73fe67d0de`, with the current packet, signed refinement, and committed PREPARE worktree as authority.
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS
- TEST_VERDICT: PASS
- CODE_REVIEW_VERDICT: PASS
- HEURISTIC_REVIEW_VERDICT: PASS
- SPEC_ALIGNMENT_VERDICT: PASS
- ENVIRONMENT_VERDICT: PASS
- DISPOSITION: NONE
- LEGAL_VERDICT: PASS
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- VALIDATOR_RISK_TIER: HIGH
- ADVISORY_VERDICT: PASS
- FINDINGS_COUNT: 0
- CLAUSES_REVIEWED:
  - `[ADD v02.168] Base envelope MUST expose updated_at` -> `src/backend/handshake_core/src/locus/types.rs:1200`; `src/backend/handshake_core/src/locus/types.rs:1223`; `src/backend/handshake_core/src/locus/types.rs:1294`; `src/backend/handshake_core/src/workflows.rs:3391-3428`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:756`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1192-1198`
  - `[ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids` -> `src/backend/handshake_core/src/locus/task_board.rs:44-49`; `src/backend/handshake_core/src/locus/task_board.rs:70-97`; `src/backend/handshake_core/src/locus/task_board.rs:167`; `src/backend/handshake_core/src/locus/types.rs:1093-1110`; `src/backend/handshake_core/src/workflows.rs:3508-3646`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:984-1011`
  - `[ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas` -> `src/backend/handshake_core/src/locus/types.rs:1055-1057`; `src/backend/handshake_core/src/locus/types.rs:1422`; `src/backend/handshake_core/src/locus/types.rs:1664-1672`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1539`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1580`
  - `[ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs` -> `src/backend/handshake_core/src/workflows.rs:3391-3428`; `src/backend/handshake_core/src/workflows.rs:4686-4778`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:171-238`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:1132`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:763-769`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1198-1204`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1356-1361`
  - `RoleMailboxThreadLineV1 field completeness including transcription_links` -> `src/backend/handshake_core/src/role_mailbox.rs:252`; `src/backend/handshake_core/src/role_mailbox.rs:533-535`; `src/backend/handshake_core/src/role_mailbox.rs:933-957`; `src/backend/handshake_core/src/api/role_mailbox.rs:35-114`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:289`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:359`
- NOT_PROVEN:
  - NONE
- MAIN_BODY_GAPS:
  - NONE
- QUALITY_RISKS:
  - NONE
- DIFF_ATTACK_SURFACES:
  - Legacy-compatible packet/detail emission for work packets and micro-tasks after the emitter rewrite.
  - Locus storage progress serialization for micro-task bind/unbind and lifecycle views.
  - Task-board projection contract (`rows`, `lane_ids`, `view_ids`, `default`) across emitters and validators.
  - Role Mailbox structured export completeness for `transcription_links`.
- INDEPENDENT_CHECKS_RUN:
  - `just validator-handoff-check WP-1-Structured-Collaboration-Schema-Registry-v3` => PASS; reran committed `pre-work` and `post-work --rev HEAD` against `23f4c9a`, confirming the refined scope and final four-file repair diff.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests -- --nocapture` => PASS; 20 tests passed, including the bind/unbind, lifecycle occupancy, task-board, summary/detail, and machine-readable diagnostic surfaces touched by the repair.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests -- --nocapture` => PASS; 6 tests passed, including structured export and message creation with `transcription_links`.
  - `rg -n "tracked_mt_progress_metadata|build_structured_work_packet_packet|build_structured_micro_task_packet|default_view_id|transcription_links" src/backend/handshake_core/src/storage/locus_sqlite.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/locus/task_board.rs src/backend/handshake_core/src/role_mailbox.rs src/backend/handshake_core/src/api/role_mailbox.rs` => confirmed the restored packet/detail compatibility lives on both the artifact-emission boundary and the locus progress boundary, with no stale task-board or mailbox field names in the touched surfaces.
- COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/storage/locus_sqlite.rs:171-238` `tracked_mt_progress_metadata(...)` were removed or stopped reinserting empty `active_session_ids`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1198-1256` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1722-1728` would fail on the progress payload contract.
  - If `src/backend/handshake_core/src/workflows.rs:3400` or `src/backend/handshake_core/src/workflows.rs:3428` reverted back to raw tracked-record serialization, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:763-769`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1198-1204`; and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1356-1361` would lose the legacy artifact deserialization proof.
  - If `src/backend/handshake_core/src/locus/task_board.rs:44-49` or `src/backend/handshake_core/src/workflows.rs:3591-3646` drifted back to legacy task-board fields, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:984-1011` would fail and `src/backend/handshake_core/src/locus/types.rs:1093-1110` would reject the emitted projection.
  - If `src/backend/handshake_core/src/role_mailbox.rs:933-957` stopped exporting `transcription_links`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:359` would fail and `src/backend/handshake_core/src/locus/types.rs:1128-1129` would reject the export.
- BOUNDARY_PROBES:
  - Artifact emitter -> validator boundary: `src/backend/handshake_core/src/workflows.rs:3391-3428` now emits legacy-compatible packet detail JSON that still carries the shared structured-collaboration envelope, and the full `micro_task_executor_tests` suite proved those payloads are accepted by the in-repo validators and readers.
  - Locus storage -> consumer boundary: `src/backend/handshake_core/src/storage/locus_sqlite.rs:171-238` and `src/backend/handshake_core/src/storage/locus_sqlite.rs:1132` project tracked micro-task state back into the legacy-compatible artifact shape consumed by `locus_get_mt_progress_v1`, including explicit empty `active_session_ids`.
- NEGATIVE_PATH_CHECKS:
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:850` confirms that removing `updated_at` from emitted work-packet JSON yields a `MissingField` validation failure.
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1539` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1580` confirm machine-readable `schema_version_mismatch` and `incompatible_profile_extension` failures on the repaired registration path.
- INDEPENDENT_FINDINGS:
  - The earlier packet-detail compatibility gap is closed on current head: emitted work-packet and micro-task packets remain deserializable as `TrackedWorkPacketArtifactV1` / `TrackedMicroTaskArtifactV1`, and the locus progress API now projects the same micro-task artifact shape.
  - The widened repair stayed inside the structured-collaboration runtime boundary; no Loom, frontend, or unrelated runtime family surface had to change to recover the contract.
  - Task-board projection and Role Mailbox completeness remain green on current head after the compatibility repair.
- RESIDUAL_UNCERTAINTY:
  - This remains a `HIGH`-risk shared serialization surface, so post-merge spotchecks across downstream artifact readers are still prudent even after the committed diff-scoped PASS.
  - This review is in-repo and diff-scoped; it does not prove behavior for out-of-tree consumers beyond the current repository surface.
- SPEC_CLAUSE_MAP:
  - `[ADD v02.168] Base envelope MUST expose updated_at` -> `src/backend/handshake_core/src/locus/types.rs:1200`; `src/backend/handshake_core/src/locus/types.rs:1223`; `src/backend/handshake_core/src/locus/types.rs:1294`; `src/backend/handshake_core/src/workflows.rs:3391-3428`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:756`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1192-1198`
  - `[ADD v02.168] Task Board projection rows SHOULD include task_board_id, lane_id, display_order, view_ids` -> `src/backend/handshake_core/src/locus/task_board.rs:44-49`; `src/backend/handshake_core/src/locus/task_board.rs:70-97`; `src/backend/handshake_core/src/locus/task_board.rs:167`; `src/backend/handshake_core/src/locus/types.rs:1093-1110`; `src/backend/handshake_core/src/workflows.rs:3508-3646`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:984-1011`
  - `[ADD v02.168] Deterministic machine-readable validation for unknown/incompatible schemas` -> `src/backend/handshake_core/src/locus/types.rs:1055-1057`; `src/backend/handshake_core/src/locus/types.rs:1422`; `src/backend/handshake_core/src/locus/types.rs:1664-1672`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1539`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1580`
  - `[ADD v02.168] Summary/detail shared record_id, project_profile_kind, authority_refs` -> `src/backend/handshake_core/src/workflows.rs:3391-3428`; `src/backend/handshake_core/src/workflows.rs:4686-4778`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:171-238`; `src/backend/handshake_core/src/storage/locus_sqlite.rs:1132`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:763-769`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1198-1204`; `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1356-1361`
  - `RoleMailboxThreadLineV1 field completeness including transcription_links` -> `src/backend/handshake_core/src/role_mailbox.rs:252`; `src/backend/handshake_core/src/role_mailbox.rs:533-535`; `src/backend/handshake_core/src/role_mailbox.rs:933-957`; `src/backend/handshake_core/src/api/role_mailbox.rs:35-114`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:289`; `src/backend/handshake_core/tests/role_mailbox_tests.rs:359`
- NEGATIVE_PROOF:
  - Out-of-scope project-profile registry publication remains open: the broader registry/product contract tracked under `WP-1-Project-Profile-Extension-Registry` is not closed by this packet, which only proves compatibility hooks, structured diagnostics, and current artifact/runtime boundaries.

VALIDATION REPORT - WP-1-Structured-Collaboration-Schema-Registry-v3
Verdict: FAIL
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: FAIL
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: FAIL
HEURISTIC_REVIEW_VERDICT: FAIL
SPEC_ALIGNMENT_VERDICT: FAIL
ENVIRONMENT_VERDICT: PASS
DISPOSITION: NONE
LEGAL_VERDICT: FAIL
SPEC_CONFIDENCE: POST_MERGE_RECHECKED
WORKFLOW_VALIDITY: INVALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: NOT_PROVEN
INTEGRATION_READINESS: NOT_READY
DOMAIN_GOAL_COMPLETION: PARTIAL
VALIDATOR_RISK_TIER: HIGH

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v3/packet.md` (status: `Blocked`)
- Spec: `Handshake_Master_Spec_v02.178.md`
- Governance basis: `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`

CLAUSES_REVIEWED:
- `[ADD v02.171/v02.178] canonical workflow-state fields on Work Packet, Micro-Task, and Task Board records` -> `src/backend/handshake_core/src/locus/types.rs`; `src/backend/handshake_core/src/locus/task_board.rs`; `src/backend/handshake_core/src/workflows.rs`
- `Role Mailbox export contract and nested payload shape` -> `src/backend/handshake_core/src/role_mailbox.rs`; `src/backend/handshake_core/src/api/role_mailbox.rs`; `src/backend/handshake_core/src/locus/types.rs`
- `machine-readable structured validation path` -> `src/backend/handshake_core/tests/micro_task_executor_tests.rs`; `src/backend/handshake_core/tests/role_mailbox_tests.rs`

NOT_PROVEN:
- `validate_structured_collaboration_record(...)` does not fully enforce `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` for every canonical record family claimed by the packet.
- Nested Task Board `rows[]`, Role Mailbox `threads[]`, and `transcription_links[]` are not validated deeply enough to justify spec-tight closure.
- Timestamp and other constrained string contracts remain weaker than the typed guarantees claimed by the spec.

MAIN_BODY_GAPS:
- Shared validator enforcement is shallower than the current Master Spec for the workflow-state contract.
- Nested structured payload conformance is not fully defended against malformed stored state.

QUALITY_RISKS:
- Happy-path emitter correctness still outruns validator hardness on a shared serialization surface.
- Future drift can remove workflow-state fields or nested payload shape without immediate validator rejection.

DIFF_ATTACK_SURFACES:
- Shared validator enforcement in `locus/types.rs`
- Task Board projection emission and validation
- Role Mailbox export validation and nested payload handling

INDEPENDENT_CHECKS_RUN:
- `cargo test --test role_mailbox_tests role_mailbox_validation_reports_schema_and_authority_drift -- --exact --nocapture` => PASS during the audit, but it did not cover the missing nested-shape enforcement gaps
- `cargo test --test micro_task_executor_tests locus_register_mts_returns_machine_readable_validation_for_unknown_schema_version -- --exact --nocapture` => PASS during the audit, but it did not prove the missing workflow-field enforcement
- Direct code inspection against `Handshake_Master_Spec_v02.178.md` => found the unresolved validator-enforcement gaps recorded above

COUNTERFACTUAL_CHECKS:
- If `workflow_state_family` is removed from a Work Packet, Micro-Task, or Task Board artifact, the current shared validator path can still accept the drifted record.
- If mailbox export contains malformed `transcription_links[]` items, the current export gate can still accept array-shaped but spec-invalid payloads.

BOUNDARY_PROBES:
- Emitted Task Board and mailbox payloads were compared against the shared validator branches in `src/backend/handshake_core/src/locus/types.rs` and showed outer-envelope checks that stop short of full nested-contract enforcement.

NEGATIVE_PATH_CHECKS:
- Existing negative-path tests for schema-version mismatch and incompatible profile extension passed, but the audit found no validator-owned negative-path proof for missing workflow-state fields or malformed nested mailbox/task-board payload items.

INDEPENDENT_FINDINGS:
- The v3 work materially improved emitted artifacts on the happy path.
- The remaining failure shape is contract-enforcement weakness, not complete feature absence.

RESIDUAL_UNCERTAINTY:
- This was a scoped audit of the v3 integrated product surfaces, not a full repo-wide structured-collaboration revalidation.
- Shared-surface downstream readers may still hide additional compatibility risk beyond the audited in-repo code paths.

SPEC_CLAUSE_MAP:
- `workflow-state fields on canonical records` -> `src/backend/handshake_core/src/locus/types.rs`; `src/backend/handshake_core/src/locus/task_board.rs`; `src/backend/handshake_core/src/workflows.rs`
- `Role Mailbox nested export contract` -> `src/backend/handshake_core/src/role_mailbox.rs`; `src/backend/handshake_core/src/api/role_mailbox.rs`; `src/backend/handshake_core/src/locus/types.rs`

NEGATIVE_PROOF:
- The packet does not fully prove the current-spec validator contract for required workflow-state fields and nested payload conformance, so the historical PASS-shaped closure is not valid under current governance law.

REASON FOR FAIL:
- Governance reclassification on 2026-03-24: this packet is retained as historical audit evidence only. It must not be resumed or treated as validated closure because the 2026-03-21 audit found unresolved proof gaps on the shared validator/export surface, and current governance law requires a newer remediation packet revision for any further work.
