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

# Task Packet: WP-1-Calendar-Sync-Engine-v3

## METADATA
- TASK_ID: WP-1-Calendar-Sync-Engine-v3
- WP_ID: WP-1-Calendar-Sync-Engine-v3
- BASE_WP_ID: WP-1-Calendar-Sync-Engine
- DATE: 2026-04-25T06:50:59.143Z
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
- CLI_CURRENT_HOST_STATUS: DISABLED_HEADLESS_ONLY
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.5
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.4
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Calendar-Sync-Engine-v3
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.5
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Calendar-Sync-Engine-v3
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-sync-engine-v3
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Sync-Engine-v3
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Sync-Engine-v3
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Calendar-Sync-Engine-v3
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.5
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Sync-Engine-v3
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Sync-Engine-v3
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Calendar-Sync-Engine-v3
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Calendar-Sync-Engine-v3
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Calendar-Sync-Engine-v3
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
- **Status:** Done
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: MERGE_PENDING
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: NONE
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-25T23:48:55.900Z
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Calendar-Storage, WP-1-MEX-v1-2-Runtime, WP-1-Workflow-Engine
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Calendar-Law-Compliance-Tests
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Calendar-Sync-Engine-v3
- LOCAL_WORKTREE_DIR: ../wtc-sync-engine-v3
- REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Sync-Engine-v3
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Sync-Engine-v3
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v3
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v3/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v3/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v3/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: wp_validator:wp-1-calendar-sync-engine-v3
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-calendar-sync-engine-v3
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja250420260848
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: Awaiting local main containment verification for the approved PASS closure.
Next: INTEGRATION_VALIDATOR verifies main containment once the approved merge lands in local main.
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: v2 Integration Validator compile blockers | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs; ../handshake_main/src/backend/handshake_core/tests/model_session_scheduler_tests.rs; ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs | TESTS: cargo check --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests | EXAMPLES: the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant, `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`, a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: deterministic final-lane handoff reproducibility | CODE_SURFACES: .GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/packet.md; product candidate commit metadata | TESTS: just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range recorded_base..recorded_candidate | EXAMPLES: the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant, `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`, a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: surface mutation discipline plus write gate | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant, `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`, a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: workflow capability profile and required-capabilities contract | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/capabilities.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/gates.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant, `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`, a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Cross-Tool Interaction Map no-shadow-pipeline rule | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests | EXAMPLES: the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant, `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`, a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: `calendar_sync` engine contract and output | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | EXAMPLES: the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant, `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`, a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: `CalendarSourceSyncState` as single source of truth for recovery | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant, `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`, a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: MCP/provider adapter guidance plus read-only mode | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant, `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`, a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo check --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range recorded_base..recorded_candidate
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo check --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range recorded_base..recorded_candidate
  - targeted validator review of compile repair, capability-contract wiring, engine registration, runtime adapter installation, gated sync execution, sync-state durability, and proof reproducibility
- CANONICAL_CONTRACT_EXAMPLES:
  - the unchanged checkpoint creation path in `workflows.rs` still resolves `SessionCheckpoint` after the calendar-sync transplant
  - `run_calendar_sync_job` owns or clones job-input fields before params assembly so it does not move borrowed `inputs`
  - a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback
  - a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes
  - a repeated identical sync run that keeps stable identity and produces no duplicate events
  - a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: portable sync behavior across both supported backends | SUBFEATURES: storage reuse, sync-state persistence, repeat-run consistency, backend parity | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The engine must reuse the portable storage substrate instead of introducing backend-specific sync behavior.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: canonical synced calendar substrate for downstream routing and projection | SUBFEATURES: durable source sync posture, stable event identity, queryable event windows | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet should produce the canonical data shape that downstream routing and projection packets consume.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: policy and routing consumers read sync-backed posture through a dedicated integration packet | SUBFEATURES: policy-profile selection, scope-hint routing, downstream projections | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Policy-Integration-v1 | NOTES: This packet produces the routing inputs without also owning the routing layer.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar source sync execution | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_sync_result | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: The packet must make the sync run as a governed workflow job rather than a helper thread.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar mutation apply discipline | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_mutation | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Successful patch application must remain trace-linked and workflow-governed.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar capability contract evaluation | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: capability allow/deny evidence | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: The `calendar_sync` path cannot truthfully retain Analyst/doc.summarize capability routing.
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Calendar-Sync-Engine-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a72.6.6 AI Job Model surface mutation discipline
- CONTEXT_START_LINE: 9408
- CONTEXT_END_LINE: 9410
- CONTEXT_TOKEN: Any external calendar mutation is executed only by the mechanical engine `calendar_sync`
- EXCERPT_ASCII_ESCAPED:
  ```text
Surface mutation discipline (non-negotiable)
  - Calendar UI remains view-only; all calendar mutations are expressed as validated patch-sets and applied by the host after Gate checks.
  - Any external calendar mutation is executed only by the mechanical engine `calendar_sync` under explicit capability+consent.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a75.4.6.4 Calendar Law compliance tests
- CONTEXT_START_LINE: 23624
- CONTEXT_END_LINE: 23628
- CONTEXT_TOKEN: Outbox is idempotent
- EXCERPT_ASCII_ESCAPED:
  ```text
Key invariants covered:
  - RBC is view-only: UI may render calendar state, but MUST NOT write to calendar tables directly.
  - All mutations are patch-sets: changes flow through the AI Job Model + Workflow Engine, then `calendar_sync` applies them.
  - External writes are gated: any provider-side mutation requires explicit capabilities + consent prompts.
  - Outbox is idempotent: every outbound change has a stable idempotency key; retries must not duplicate events.
  - Full observability: every calendar mutation emits Flight Recorder spans and links back to `job_id`.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 Mutation governance (Hard Invariant) [ilja251220250127]
- CONTEXT_START_LINE: 55965
- CONTEXT_END_LINE: 55969
- CONTEXT_TOKEN: calendar_mutation
- EXCERPT_ASCII_ESCAPED:
  ```text
- Direct database writes to `calendar_events` are PROHIBITED from the API layer or UI components.
  - All mutations MUST be submitted as `CalendarMutation` patches via a `WorkflowRun` targeting the `calendar_sync` mechanical engine.
  - Every successful mutation MUST emit a Flight Recorder span of type `calendar_mutation` with a back-link to the `job_id`.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 Mutation and governance rules
- CONTEXT_START_LINE: 55983
- CONTEXT_END_LINE: 55990
- CONTEXT_TOKEN: Patch-sets are the only write primitive
- EXCERPT_ASCII_ESCAPED:
  ```text
- No direct UI writes: UI gestures emit jobs; only the host applies patches after validation and gates.
  - Patch-sets are the only write primitive: all calendar writes (local or external) are expressed as validated patch-sets with preconditions, effect, and provenance.
  - External writes are explicitly gated: any write that leaves the device requires capability + user confirmation unless the source is configured as `auto_export=true`.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 Mechanical engine: `calendar_sync`
- CONTEXT_START_LINE: 56205
- CONTEXT_END_LINE: 56227
- CONTEXT_TOKEN: calendar_sync_result
- EXCERPT_ASCII_ESCAPED:
  ```text
Mechanical engine: `calendar_sync`
  Engine input includes `CalendarSource.id`, direction, and time_window.
  Behavior includes pulling from provider sources, pushing mirrored events when allowed, and always recording sync activity in Flight Recorder.
  Output includes `calendar_sync_result` plus updated CalendarEvent rows.
  All external writes are capability-gated and must go through the Workflow Engine, not ad hoc helpers.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 `CalendarSourceSyncState`
- CONTEXT_START_LINE: 56316
- CONTEXT_END_LINE: 56323
- CONTEXT_TOKEN: CalendarSourceSyncState
- EXCERPT_ASCII_ESCAPED:
  ```text
Each `CalendarSource` persists a sync state record.
  This is the single source of truth for incremental sync and recovery.
  `CalendarSourceSyncState` carries stage, sync token, last-ok timestamps, and recovery fields.
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 MCP client for external calendars
- CONTEXT_START_LINE: 56838
- CONTEXT_END_LINE: 56842
- CONTEXT_TOKEN: Use these tools **inside** the `calendar_sync` engine
- EXCERPT_ASCII_ESCAPED:
  ```text
Implement MCP tools that wrap Google Calendar, Outlook/Exchange, and generic CalDAV.
  Use these tools inside the `calendar_sync` engine instead of hardcoding clients.
  This lets the orchestrator call provider operations uniformly regardless of provider.
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4.1 read-only source posture
- CONTEXT_START_LINE: 57016
- CONTEXT_END_LINE: 57019
- CONTEXT_TOKEN: write_back=false
- EXCERPT_ASCII_ESCAPED:
  ```text
Some sources may be used in read-only mode.
  `CalendarSource` has a flag `write_back=false`.
  `calendar_sync_google` for that source only pulls; it never calls insert/update/delete.
  ```

#### ANCHOR 9
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a710.4 Calendar backend force-multiplier capability contract
- CONTEXT_START_LINE: 55877
- CONTEXT_END_LINE: 55877
- CONTEXT_TOKEN: capability_profile_id
- EXCERPT_ASCII_ESCAPED:
  ```text
[ADD v02.155] In Phase 1, Calendar is also a backend force multiplier: `CalendarSourceSyncState`, `CalendarSource.write_policy`, `CalendarEvent.export_mode`, `capability_profile_id`, and `CalendarScopeHint` are canonical backend contracts for sync recovery, consent posture, AI-job mutation discipline, and scope-hint routing.
  ```

#### ANCHOR 10
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a72.6.6.5.2 AI Job capability profiles
- CONTEXT_START_LINE: 9763
- CONTEXT_END_LINE: 9777
- CONTEXT_TOKEN: capability_profile_id
- EXCERPT_ASCII_ESCAPED:
  ```text
Jobs are evaluated under capability profiles:

  | Field | Role |
  |-------|------|
  | `capability_profile_id` | Determines what the job can read/write in the workspace |
  | `access_mode` | Read-only, preview-only, or scoped-apply |
  | `layer_scope` | Which layers (raw/derived/display) are writable |

  Enforcement Points:
  1. Before `queued`: Basic capability check
  2. At `awaiting_validation`: Full capability and policy check
  3. On commit: Final verification that only allowed entities were modified
  ```

#### ANCHOR 11
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md \u00a74.3.9.19 SessionCheckpoint primitive
- CONTEXT_START_LINE: 32844
- CONTEXT_END_LINE: 32857
- CONTEXT_TOKEN: SessionCheckpoint
- EXCERPT_ASCII_ESCAPED:
  ```text
SessionCheckpoint:
    checkpoint_id: string
    session_id: string
    timestamp: string
    session_state: ModelSession
    message_thread_tail_id: string
    pending_tool_calls: ToolCall[]
    checkpoint_artifact_id: string
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: v2 Integration Validator compile blockers | WHY_IN_SCOPE: the latest final-lane FAIL names signed-surface compile defects that prevent packet proof commands from running; Orchestrator repair widened MT-001 in-place to include the delimiter repair in `flight_recorder/mod.rs` and the follow-on integration-test compile blockers in `tests/model_session_scheduler_tests.rs` and `tests/micro_task_executor_tests.rs` | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs; ../handshake_main/src/backend/handshake_core/tests/model_session_scheduler_tests.rs; ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs | EXPECTED_TESTS: cargo check --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests | RISK_IF_MISSED: the candidate remains unbuildable and all semantic proof is void
  - CLAUSE: deterministic final-lane handoff reproducibility | WHY_IN_SCOPE: the v2 FAIL showed stale manifest hashes and an unreachable candidate commit from `../handshake_main` | EXPECTED_CODE_SURFACES: .GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/packet.md; product candidate commit metadata | EXPECTED_TESTS: just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range recorded_base..recorded_candidate | RISK_IF_MISSED: the Integration Validator cannot reproduce the candidate even if local coder tests pass
  - CLAUSE: surface mutation discipline plus write gate | WHY_IN_SCOPE: the packet must make `calendar_sync` the real workflow-only mutation path instead of a paper contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: direct helper or UI-side writes can still bypass governed execution
  - CLAUSE: workflow capability profile and required-capabilities contract | WHY_IN_SCOPE: the v3 packet must preserve the v2 calendar sync path so `workflow_run` and capability gating use the intended calendar capability contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/capabilities.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/gates.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: `calendar_sync` regresses to the wrong capability contract or to `HSK-4001 UnknownCapability`
  - CLAUSE: Cross-Tool Interaction Map no-shadow-pipeline rule | WHY_IN_SCOPE: provider sync must run through Workflow Engine + MEX runtime, not hidden background helpers | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests | RISK_IF_MISSED: provider access happens outside the contract the spec requires
  - CLAUSE: `calendar_sync` engine contract and output | WHY_IN_SCOPE: the packet exists to realize the engine input/behavior/output contract already named in the spec | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | RISK_IF_MISSED: the engine may exist nominally but still fail to honor spec-defined behavior
  - CLAUSE: `CalendarSourceSyncState` as single source of truth for recovery | WHY_IN_SCOPE: retries, backoff, and recovery are core parts of a sync engine, not optional extras | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: the engine becomes non-recoverable or duplicates data under retry
  - CLAUSE: MCP/provider adapter guidance plus read-only mode | WHY_IN_SCOPE: the spec explicitly prefers provider access through tools inside the engine and names read-only behavior as a first-class posture | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: provider access and write posture drift from the calendar law contract
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: workflow compile/regression contract | PRODUCER: workflows.rs imports and `run_calendar_sync_job` params assembly | CONSUMER: Rust compiler, mex_tests, Integration Validator | SERIALIZER_TRANSPORT: Rust module imports and owned job-input values | VALIDATOR_READER: cargo check/test output | TRIPWIRE_TESTS: cargo check plus mex_tests | DRIFT_RISK: compile-only regressions prevent every calendar semantic proof command from running
  - CONTRACT: deterministic handoff manifest contract | PRODUCER: coder post-work manifest and candidate commit | CONSUMER: phase-check HANDOFF and Integration Validator | SERIALIZER_TRANSPORT: packet validation manifest plus git commit range | VALIDATOR_READER: phase-check HANDOFF output from final-lane repo | TRIPWIRE_TESTS: just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range recorded_base..recorded_candidate | DRIFT_RISK: a stale manifest or unreachable candidate repeats the v2 final-lane proof failure
  - CONTRACT: `calendar_sync` engine registry contract | PRODUCER: mechanical_engines.json | CONSUMER: mex/registry.rs, workflows.rs, tests/mex_tests.rs | SERIALIZER_TRANSPORT: JSON engine registry | VALIDATOR_READER: mex_tests.rs | TRIPWIRE_TESTS: mex registry/runtime tests | DRIFT_RISK: engine is declared but not executable, or executable but not declared consistently
  - CONTRACT: calendar sync job input / protocol contract | PRODUCER: workflows.rs job/profile parser and engine runner | CONSUMER: calendar-sync adapter implementation, storage layer, validators | SERIALIZER_TRANSPORT: workflow payload plus PlannedOperation inputs | VALIDATOR_READER: workflow/job tests plus validator inspection | TRIPWIRE_TESTS: targeted calendar-sync execution tests plus full cargo test | DRIFT_RISK: job payload shape and adapter expectations silently diverge
  - CONTRACT: calendar sync capability contract | PRODUCER: capabilities.rs plus workflow capability-profile binding | CONSUMER: workflows.rs, mex/gates.rs, tests/mex_tests.rs | SERIALIZER_TRANSPORT: capability profile ids and requested capability strings | VALIDATOR_READER: mex_tests.rs plus validator inspection | TRIPWIRE_TESTS: mex capability-path tests plus full cargo test | DRIFT_RISK: requested calendar capabilities remain undefined, misnamed, or bound to the wrong workflow profile
  - CONTRACT: `CalendarSourceSyncState` durable recovery contract | PRODUCER: storage/calendar.rs plus engine runner | CONSUMER: sqlite.rs, postgres.rs, later recovery/retry flows | SERIALIZER_TRANSPORT: sqlx row mapping and JSON-ish sync-state payloads | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: calendar storage tests plus sync retry/idempotency tests | DRIFT_RISK: sync token/backoff/watermark state is lost or inconsistently updated
  - CONTRACT: calendar event upsert/idempotency contract | PRODUCER: engine runner and adapter | CONSUMER: storage backends and later Lens/policy consumers | SERIALIZER_TRANSPORT: storage upsert calls keyed by source/external identity | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: calendar storage tests plus repeat-sync tests | DRIFT_RISK: repeated sync runs duplicate events or destabilize identity
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Start from the latest v2 failed candidate and inspect the exact Integration Validator findings before editing.
  - Repair `src/backend/handshake_core/src/workflows.rs` so the existing `SessionCheckpoint` constructor resolves again.
  - Refactor `run_calendar_sync_job` params assembly so borrowed fields from `inputs` are owned or cloned before params are built; do not move `inputs` after borrowing from it.
  - Preserve the reviewed calendar sync registry/runtime/capability semantics unless a compile fix requires the smallest local adjustment.
  - Run `cargo check`, the targeted wrong-profile test, `mex_tests`, `calendar_storage_tests`, and full `cargo test` from the product worktree with external cargo artifacts until they pass cleanly.
  - Rebuild deterministic handoff proof for the v3 candidate and append fresh evidence before requesting validator review.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- TRIPWIRE_TESTS:
  - cargo check --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range recorded_base..recorded_candidate
- CARRY_FORWARD_WARNINGS:
  - Do not treat this as an environment-only failure; the latest v2 blocker is inside signed `workflows.rs`.
  - Do not remove or bypass `SessionCheckpoint` behavior to make calendar sync compile.
  - Do not move borrowed `inputs` in `run_calendar_sync_job`; build params from owned data.
  - Do not claim closure until the candidate commit is reachable from the final-lane repo and the handoff manifest reproduces.
  - Do not reimplement calendar storage or invent shadow tables; the completed storage packet is the substrate.
  - Do not add ad hoc background sync threads or direct provider clients outside workflow/MEX runtime.
  - Do not silently reuse Analyst/doc.summarize or any unrelated `workflow_run` capability contract for the calendar sync path.
  - Do not widen the packet into Lens, ACE policy integration, multi-provider breadth, or rich write-back UX.
  - Do not mint new PRIM IDs or new top-level Flight Recorder schemas to paper over runtime gaps.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - `workflows.rs` compile repair for `SessionCheckpoint` import/reference retention
  - `run_calendar_sync_job` ownership repair for borrowed `inputs`
  - deterministic handoff manifest and candidate commit reachability from final-lane repo
  - engine registration and runtime adapter installation
  - workflow dispatch and governed execution path
  - workflow capability profile binding and requested-capability routing
  - capability posture plus read-only/write-policy fail-closed behavior
  - sync-state durability and repeat-run idempotency
  - trace/result evidence linkage
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- COMMANDS_TO_RUN:
  - cargo check --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range recorded_base..recorded_candidate
- POST_MERGE_SPOTCHECKS:
  - verify `src/backend/handshake_core/src/workflows.rs` still compiles with `SessionCheckpoint` and the borrow-safe calendar params assembly on `main`
  - verify `calendar_sync` still exists in `mechanical_engines.json` on `main`
  - verify workflow runtime still installs the calendar adapter on `main`
  - verify `workflow_run` no longer routes the calendar sync path through Analyst/doc.summarize capability posture
  - verify there is still no direct provider-write path that bypasses capability gates
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - This refinement does not prove the product code compile repair; the coder must implement and run the proof commands in the product worktree.
  - This refinement does not prove the v3 candidate commit is reachable from final-lane validation; the coder/orchestrator must rebuild and verify deterministic handoff proof after implementation.
  - This refinement does not prove bidirectional write-back, conflict resolution, CalendarScopeHint policy projection, Lens UX, MCP provider breadth, or downstream correlation behavior.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE
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
  - PRIM-CalendarSyncInput
  - PRIM-CalendarMutation
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
  - PRIM-SessionCheckpoint
- PRIMITIVES_EXPOSED:
  - PRIM-CalendarSyncInput
  - PRIM-CalendarMutation
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.dba
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: NEEDS_STUBS
- PILLARS_TOUCHED:
  - Flight Recorder
  - Calendar
  - Command Center
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - Flight Recorder: WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Law-Compliance-Tests-v1
  - Calendar: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
  - LLM-friendly data: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1
  - ACE: WP-1-Calendar-Policy-Integration-v1
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_RESOLUTIONS:
  - existing calendar storage plus governed sync-engine registration removes the shadow-pipeline gap -> IN_THIS_WP (stub: NONE)
  - workflow capability routing plus calendar capability identifiers removes the `HSK-4001 UnknownCapability` hard-stop -> IN_THIS_WP (stub: NONE)
  - compile-safe workflow transplant plus deterministic handoff proof converts reviewed semantics into final-lane actionable evidence -> IN_THIS_WP (stub: NONE)
  - provider-safe result evidence keeps sync inspectable in existing workflow consoles -> IN_THIS_WP (stub: NONE)
  - portable storage reuse keeps SQLite and Postgres sync behavior aligned -> IN_THIS_WP (stub: NONE)
  - idempotent synced rows become the canonical calendar substrate for downstream consumers -> IN_THIS_WP (stub: NONE)
  - read-only and write-policy enforcement keeps remote provider access fail-closed -> IN_THIS_WP (stub: NONE)
  - Lens projections consume the same synced storage truth rather than bespoke provider fetches -> NEW_STUB (stub: WP-1-Calendar-Lens-v3)
  - policy and routing consumers read sync-backed posture through a dedicated integration packet -> NEW_STUB (stub: WP-1-Calendar-Policy-Integration-v1)
  - law-compliance validation can finally exercise a real governed sync path -> NEW_STUB (stub: WP-1-Calendar-Law-Compliance-Tests-v1)
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS
- DECOMPOSITION_ROWS:
  - PILLAR: Calendar | CAPABILITY_SLICE: governed sync-engine registration and execution | SUBFEATURES: engine registry row, runtime adapter install, workflow dispatch contract, idempotent source/event upserts | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation, PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This remains the direct activation target.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: calendar capability profile, workflow capability routing, and compile-safe current-main transplant | SUBFEATURES: `calendar.sync.read`, `calendar.sync.write`, workflow-run capability mapping, `CapabilityGate` acceptance, fail-closed denials, `SessionCheckpoint` import retention, borrow-safe `run_calendar_sync_job` params | PRIMITIVES_FEATURES: PRIM-CalendarSource, PRIM-CalendarSourceWritePolicy, PRIM-CalendarMutation, PRIM-SessionCheckpoint | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the material v3 remediation slice proven by the Integration Validator FAIL.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: auditable sync and mutation evidence | SUBFEATURES: result artifact, trace linkage, provider-safe diagnostics, retry visibility | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation, PRIM-CalendarSourceSyncState | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The engine must be inspectable through existing runtime evidence surfaces rather than helper-local logs.
  - PILLAR: Command Center | CAPABILITY_SLICE: inspectable workflow and capability outcomes for calendar sync | SUBFEATURES: job visibility, workflow-run status, operator-visible denial posture, failure summaries | PRIMITIVES_FEATURES: PRIM-CalendarSyncInput, PRIM-CalendarMutation | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Existing runtime inspection surfaces should expose the sync job without a packet-owned UI.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: portable sync behavior across both supported backends | SUBFEATURES: storage reuse, sync-state persistence, repeat-run consistency, backend parity | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The engine must reuse the portable storage substrate instead of introducing backend-specific sync behavior.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: canonical synced calendar substrate for downstream routing and projection | SUBFEATURES: durable source sync posture, stable event identity, queryable event windows | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet should produce the canonical data shape that downstream routing and projection packets consume.
  - PILLAR: Calendar | CAPABILITY_SLICE: user-facing Lens projection and filters | SUBFEATURES: agenda/timeline rendering, diagnostics display, user controls | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Lens-v3 | NOTES: Lens remains a downstream UI consumer packet.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: policy and routing consumers read sync-backed posture through a dedicated integration packet | SUBFEATURES: policy-profile selection, scope-hint routing, downstream projections | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Policy-Integration-v1 | NOTES: This packet produces the routing inputs without also owning the routing layer.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: calendar source sync execution | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_sync_result | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: The packet must make the sync run as a governed workflow job rather than a helper thread.
  - Capability: calendar mutation apply discipline | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_mutation | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Successful patch application must remain trace-linked and workflow-governed.
  - Capability: calendar capability contract evaluation | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: capability allow/deny evidence | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: The `calendar_sync` path cannot truthfully retain Analyst/doc.summarize capability routing.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Calendar-Lens-v3 -> KEEP_SEPARATE
  - WP-1-Calendar-Policy-Integration-v1 -> KEEP_SEPARATE
  - WP-1-Calendar-Law-Compliance-Tests-v1 -> KEEP_SEPARATE
  - WP-1-Calendar-Sync-Engine-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Calendar-Sync-Engine-v2 -> EXPAND_IN_THIS_WP
  - WP-1-Calendar-Storage-v2 -> KEEP_SEPARATE
  - WP-1-Workflow-Engine-v4 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Calendar-Sync-Engine-v2)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Calendar-Sync-Engine-v2)
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs -> PARTIAL (WP-1-Calendar-Sync-Engine-v2)
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs -> PARTIAL (WP-1-Calendar-Sync-Engine-v2)
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
- What: Remediate WP-1-Calendar-Sync-Engine-v2 after Integration Validator FAIL by repairing the signed `workflows.rs` compile regressions, preserving the reviewed calendar sync runtime/capability semantics, rebuilding deterministic handoff proof, and appending fresh cargo-backed evidence.
- Why: The spec already defines `calendar_sync` as the only legal path for external calendar mutation and provider sync. The v2 Integration Validator found the intended semantic shape mostly coherent, but merge cannot proceed because the candidate does not compile and the final-lane proof chain is not reproducible.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- OUT_OF_SCOPE:
  - Multi-provider breadth beyond one truthful MVP adapter/import path
  - Rich bidirectional write-back UX, conflict-resolution policy, and multi-source reconciliation
  - Calendar Lens UI implementation
  - CalendarScopeHint / ACE policy-routing implementation
  - Calendar correlation export and mailbox correlation product logic
  - New provider MCP wrapper implementation beyond preserving the existing guidance and fail-closed posture
  - Repo-governance protocol changes except the Operator-authorized same-WP headless ACP launch remediation recorded on 2026-04-25; deterministic handoff proof must be rebuilt through existing gates
  - Repo-governance tooling or protocol changes unrelated to this packet
- TOUCHED_FILE_BUDGET: 12
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
cargo check --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
```

### DONE_MEANS
- `src/backend/handshake_core/src/workflows.rs` compiles after restoring the missing `SessionCheckpoint` import or equivalent in-scope reference.
- `run_calendar_sync_job` builds `params` without moving `inputs` after borrowing from it, eliminating the `E0505` failure reported at the v2 final-lane review.
- The reviewed v2 calendar sync semantics remain intact: `calendar_sync` registry/runtime dispatch, `CalendarSync` capability routing, denied output parity, sync-state durability, and provider-safe evidence.
- The targeted `calendar_sync_runtime_denies_wrong_profile_without_unknown_capability` test, `mex_tests`, `calendar_storage_tests`, and full `cargo test` produce fresh passing evidence from the active product worktree with external cargo artifacts.
- Deterministic handoff proof is rebuilt for the actual v3 candidate so final-lane validation can resolve the candidate commit and `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 ...` succeeds.
- WP Validator and Integration Validator pass, and the validated code is integrated into `main`.

- PRIMITIVES_EXPOSED:
  - PRIM-CalendarSyncInput
  - PRIM-CalendarMutation
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-25T06:50:59.143Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.181]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.181.md \u00a72.6.6 capability profiles + \u00a710.4 Calendar capability contracts + \u00a76.0.1 workflow capability checks
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
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/registry.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- SEARCH_TERMS:
  - calendar_sync
  - calendar.sync.read
  - calendar.sync.write
  - workflow_run
  - doc.summarize
  - UnknownCapability
  - HSK-4001
  - capability_profile_id
  - CalendarSourceSyncState
  - SessionCheckpoint
  - run_calendar_sync_job
  - E0505
  - E0422
  - calendar_mutation
- RUN_COMMANDS:
  ```bash
rg -n "calendar_sync|calendar.sync.read|calendar.sync.write|workflow_run|doc.summarize|UnknownCapability|HSK-4001|capability_profile_id|SessionCheckpoint|run_calendar_sync_job" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  cargo check --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "compile repair only restores imports but skips cargo proof" -> "the packet repeats the v2 false closure pattern"
  - "params still moves borrowed `inputs`" -> "the workflow entrypoint remains unbuildable and no semantic test can execute"
  - "handoff manifest or candidate commit remains unreachable from final-lane validation" -> "Integration Validator cannot lawfully prove or merge the candidate"
  - "calendar_sync is added as a one-off helper instead of a MEX/workflow contract" -> "capability gates, provenance, and replay guarantees drift immediately"
  - "workflow_run keeps the wrong capability contract" -> "calendar sync remains blocked or runs under unrelated authority semantics"
  - "calendar capabilities stay undefined while runtime code lands" -> "the path still fails as `HSK-4001 UnknownCapability` and the packet reports false progress"
  - "tests cover registry happy-path only and skip capability/routing execution" -> "the packet looks landed while real governed execution still fails"
  - "read-only or import-only sources can still push remote mutations" -> "consent and capability law is violated"
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
- MT-001 scoped implementation commit: `85673ab5` (`fix: restore workflow checkpoint import [WP-1-Calendar-Sync-Engine-v3]`).
- MT-001 repair implementation commit: `7b5519e13339ef06fafc6ec63d1e768068d038a0` (`fix: repair MT-001 compile blockers [WP-1-Calendar-Sync-Engine-v3]`).
- MT-001 widened proof-blocker commit: `c7c5b6d8` (`Fix MT-001 proof test compile blockers`).
- MT-001 validator-steer remediation commit: `d31da029` (`Fix MT-001 micro task executor validation`).
- Changed only `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs` in the assigned `feat/WP-1-Calendar-Sync-Engine-v3` worktree.
- Restored the `SessionCheckpoint` import used by the unchanged checkpoint construction path in `workflows.rs`.
- Repaired narrow delimiter defects in `flight_recorder/mod.rs` so cargo can reach the checkpoint path.
- Repaired in-scope `workflows.rs` compile errors exposed after the parser fix: mapped checkpoint serialization errors to `WorkflowError::Terminal`, closed the malformed test function, and updated the in-file test `ModelSession` initializer for checkpoint fields.
- Added backward-compatible `FlightRecorderEvent` builder aliases (`with_*_id`) in `flight_recorder/mod.rs` to satisfy existing in-tree test callers without editing out-of-scope test files.
- Updated the widened test fixtures so `NewModelSession` includes checkpoint fields and `micro_task_executor_tests.rs` uses the current `locus::task_board` validation export plus local governed-action assertions for generated summary next actions.
- Repaired the WP Validator STEER finding by enforcing registered/family-legal governed action ids and profile-extension schema compatibility in runtime structured work-packet and micro-task validation, aligning generated `next_action` values to the generic governed action registry, and allowing MEX validation commands to run from the repo root while retaining the in-scope path list.
- Did not edit `../handshake_main`.
- Did not implement a `run_calendar_sync_job` borrow repair because that symbol/path is not present in the assigned worktree `workflows.rs` under the corrected MT-001 route.
- ORCHESTRATOR_SCOPE_UPDATE 2026-04-25T11:08:50Z: Operator rejected a new WP for the remaining proof blockers. MT-001 is widened in place to include `src/backend/handshake_core/tests/model_session_scheduler_tests.rs` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs`; Coder must continue same-WP remediation until `cargo check` and `cargo test ... mex_tests` are honestly proven or an in-scope hard blocker is recorded.
- ORCHESTRATOR_SCOPE_UPDATE 2026-04-25T11:43:00Z: Operator rejected a separate WP for the ACP/headless remediation. Same-WP governance remediation is authorized to disable `CURRENT` governed role launches, keep `AUTO` on direct headless ACP, keep `SYSTEM_TERMINAL` hidden repair-only, and mark `VSCODE_PLUGIN` disabled so governed launch/steer cannot focus terminals or capture Operator keyboard input.

## HYGIENE
- Checked CODER notifications before implementation: none pending.
- Used `node .GOV/roles_shared/scripts/session/role-command-compat.mjs active-lane-brief CODER WP-1-Calendar-Sync-Engine-v3` to confirm MT-001 is active.
- Regenerated `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch` from the MT-001 product range.
- Ran the exact required evidence commands from the assigned worktree after the product repair.
- `cargo check` passes with existing warnings.
- `cargo test ... mex_tests` passes after compiling the integration-test targets.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` passes with all 27 tests.
- Worktree product range for the committed repair is limited to the four widened MT-001 product files.

## VALIDATION
- (Mechanical handoff manifest for audit. This section records the current remediation candidate range for `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c`. It is NOT a Validator verdict.)
- Candidate range: `d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c`.
- Candidate commit: `05df783915b340efc8b6f5b180483e340710f04c` (`fix: isolate calendar sync storage imports`).
- Product main compatibility baseline used by Coder: `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`.
- **Target File**: `src/backend/handshake_core/mechanical_engines.json`
- **Start**: 1
- **End**: 239
- **Line Delta**: 28
- **Pre-SHA1**: `402e5bc2d02678a24c70c06b11de4ed51c34f7b0`
- **Post-SHA1**: `c8270ea928fd55fcc850aa82a346dfd36df37c1f`
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
- **Target File**: `src/backend/handshake_core/src/capabilities.rs`
- **Start**: 1
- **End**: 723
- **Line Delta**: 29
- **Pre-SHA1**: `bf323172c4b1c642365097eadee4ca3565672f05`
- **Post-SHA1**: `c1e375a4346fdabc3d70a0218db83fb2ae1802f4`
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
- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 6201
- **Line Delta**: 21
- **Pre-SHA1**: `6a4c6aedddd68fdb1cf78ec56acab3e4b81906c0`
- **Post-SHA1**: `2de5401ebed400796300774de0cfda19e9e7b333`
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
- **Target File**: `src/backend/handshake_core/src/locus/mod.rs`
- **Start**: 1
- **End**: 6
- **Line Delta**: 1
- **Pre-SHA1**: `e3a47c5cea8bba4bdb60865fe229a2ddcd7da9eb`
- **Post-SHA1**: `9bc4663719c8e2062f9bbe5da286a6f6c0a2e75b`
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
- **Target File**: `src/backend/handshake_core/src/locus/task_board.rs`
- **Start**: 1
- **End**: 472
- **Line Delta**: 92
- **Pre-SHA1**: `94420cf97740ebc3df0bf2a1fda05b8d0a40e634`
- **Post-SHA1**: `8f25d550bccace4b3682a321117b7ae7d0c0513d`
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
- **Start**: 1
- **End**: 2732
- **Line Delta**: 849
- **Pre-SHA1**: `20426e53c50e4fa53a5840aea0132ab045590a86`
- **Post-SHA1**: `b496ef6c7c3800cf974440af1c60c2dc9f7641da`
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
- **Target File**: `src/backend/handshake_core/src/mex/runtime.rs`
- **Start**: 1
- **End**: 1023
- **Line Delta**: 4
- **Pre-SHA1**: `c2c4136eb36a89a7036f4083f3e33b8c2dd19b44`
- **Post-SHA1**: `a2c47e5afa93cd84007473e557762c25231b3ffa`
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
- **Target File**: `src/backend/handshake_core/src/storage/calendar.rs`
- **Start**: 1
- **End**: 436
- **Line Delta**: 87
- **Pre-SHA1**: `9fbd02c81fd0f17cdea6b1bedde2da83797b2e24`
- **Post-SHA1**: `d885dbf7d4dd8e4641ff8bd8615996fd7c3e49a3`
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
- **Start**: 1
- **End**: 1113
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
- **Start**: 1
- **End**: 26849
- **Line Delta**: 495
- **Pre-SHA1**: `292b63d2c0da2ccd5dfd1505461575223096d6d5`
- **Post-SHA1**: `1a38079e9515b1c19ca010c5cc4f074d8a058261`
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
- **Target File**: `src/backend/handshake_core/tests/mex_tests.rs`
- **Start**: 1
- **End**: 1640
- **Line Delta**: 385
- **Pre-SHA1**: `5ed02fd920b9c538d8b4c441d125631d43d23774`
- **Post-SHA1**: `a8075672cc7ee52cd5e11acddc2653dee09c7440`
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
- **Start**: 1
- **End**: 3343
- **Line Delta**: 16
- **Pre-SHA1**: `d0d8c79a208ac5f9152ff28769f02f04d5dd0af7`
- **Post-SHA1**: `fb8d239ae7a61d99cab16dff444c2ddd76884f99`
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
- **Target File**: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`
- **Start**: 1
- **End**: 1648
- **Line Delta**: 3
- **Pre-SHA1**: `de64f7856de440a59c00fea2fd4ca33445bf3ce1`
- **Post-SHA1**: `23aba22fd39bd51e7af2975dd67720441b689807`
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
- **Lint Results**:
  - `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e HEAD` -> EXIT_CODE 0, tree `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..HEAD` -> EXIT_CODE 0.
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` -> EXIT_CODE 0, running 1 test, 1 passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` -> EXIT_CODE 0.
- **Artifacts**: `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Calendar-Sync-Engine-v3/2026-04-25T20-14-21-863Z.log`
  - Initial phase-check artifact showed stale manifest coverage before this coder-owned manifest refresh.
- **Timestamp**: 2026-04-25T20:14:21Z
- **Operator**: CODER:CODER-20260425-183551
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**:
  - Remediation implements signed-packet calendar-sync runtime semantics after WP Validator NOT_PROVEN finding.
  - Product work remains in the Coder worktree; this update repairs only the packet manifest required for deterministic handoff.
## VALIDATION_HISTORY_MT001
- Previous MT-001 mechanical manifest retained for audit; not current MT-002 handoff evidence.
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 50
- **End**: 25498
- **Line Delta**: 170
- **Pre-SHA1**: `292b63d2c0da2ccd5dfd1505461575223096d6d5`
- **Post-SHA1**: `dad7eb04f522b1e407a95862dd473c9c9558cad5`
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
- **Lint Results**:
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` -> EXIT_CODE 0.
- **Artifacts**: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch`
  - Product commits: `85673ab5`, `7b5519e13339ef06fafc6ec63d1e768068d038a0`, `c7c5b6d8`, `d31da029`
  - Signed patch artifact: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch`
- **Timestamp**: 2026-04-25T12:50:53Z
- **Operator**: CODER:coder:wp-1-calendar-sync-engine-v3
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**:
  - Full MT-001 product range: `d104c127e258a027fb51bc28cd3ed52e53874c92..d31da029`.
- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 378
- **End**: 4384
- **Line Delta**: 21
- **Pre-SHA1**: `6a4c6aedddd68fdb1cf78ec56acab3e4b81906c0`
- **Post-SHA1**: `2de5401ebed400796300774de0cfda19e9e7b333`
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
- **Lint Results**:
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` -> EXIT_CODE 0.
- **Artifacts**: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch`
  - Product commit: `7b5519e13339ef06fafc6ec63d1e768068d038a0`
  - Signed patch artifact: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch`
- **Timestamp**: 2026-04-25T12:50:53Z
- **Operator**: CODER:coder:wp-1-calendar-sync-engine-v3
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**:
  - Orchestrator direct same-WP repair widened MT-001 to include this file; no broad unrelated flight recorder transplant was applied.
- **Target File**: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`
- **Start**: 189
- **End**: 191
- **Line Delta**: 3
- **Pre-SHA1**: `de64f7856de440a59c00fea2fd4ca33445bf3ce1`
- **Post-SHA1**: `23aba22fd39bd51e7af2975dd67720441b689807`
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
- **Lint Results**:
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` -> EXIT_CODE 0.
- **Artifacts**: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch`
  - Product commit: `c7c5b6d8`
  - Signed patch artifact: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch`
- **Timestamp**: 2026-04-25T12:50:53Z
- **Operator**: CODER:coder:wp-1-calendar-sync-engine-v3
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**:
  - Added the checkpoint fields required by the widened MT-001 proof build.
- **Target File**: `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- **Start**: 21
- **End**: 2023
- **Line Delta**: 173
- **Pre-SHA1**: `d0d8c79a208ac5f9152ff28769f02f04d5dd0af7`
- **Post-SHA1**: `30d4fdcc452eb49343a0f350d980ad1c1625c653`
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
- **Lint Results**:
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests` -> EXIT_CODE 0.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` -> EXIT_CODE 0.
- **Artifacts**: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch`
  - Product commits: `c7c5b6d8`, `d31da029`
  - Signed patch artifact: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch`
- **Timestamp**: 2026-04-25T12:50:53Z
- **Operator**: CODER:coder:wp-1-calendar-sync-engine-v3
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**:
  - Repaired stale locus imports without adding production API surface outside MT-001 scope; direct changed-test binary now passes after adding runtime-action validation coverage and aligning profile-extension expectations to the in-scope create-WP data shape.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: DONE_MERGE_PENDING
- What changed in this update: preserved commits `85673ab5`, `7b5519e13339ef06fafc6ec63d1e768068d038a0`, and `c7c5b6d8`; repaired the direct changed-test failure by validating generated governed action ids/profile-extension schemas in `workflows.rs`, aligning `next_action` generation to the registered governed action vocabulary, allowing MEX validation commands to run from repo-root cwd, and updating the direct integration-test expectations to the current in-scope data shape; regenerated `signed-scope.patch`.
- Requirements / clauses self-audited: `v2 Integration Validator compile blockers`; `workflow compile/regression contract`; unchanged checkpoint creation path must resolve `SessionCheckpoint`; repaired flight-recorder delimiter so cargo can reach the workflow compile proof; direct changed-test binary must pass after WP Validator STEER.
- Checks actually run: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` (EXIT_CODE 0); `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests` (EXIT_CODE 0, Coder filter-form evidence); `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests` (EXIT_CODE 0, Orchestrator cross-check: 9 passed, 4 ignored); `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` (EXIT_CODE 0).
- Known gaps / weak spots: least-proven requirement is still broader calendar-sync semantic coverage because MT-001 is a compile/proof-blocker and direct-test remediation slice; riskiest boundary is the duplicated governed-action/profile-extension validation vocabulary in `micro_task_executor_tests.rs`, because the production helpers remain private and the test mirrors the registry to preserve coverage without broadening production API surface.
- WEAK_SPOTS: least-proven requirement is still broader calendar-sync semantic coverage because MT-001 is a compile/proof-blocker and direct-test remediation slice; riskiest boundary is the duplicated governed-action/profile-extension validation vocabulary in `micro_task_executor_tests.rs`, because the production helpers remain private and the test mirrors the registry to preserve coverage without broadening production API surface.
- Heuristic risks / maintainability concerns: builder alias methods in `flight_recorder/mod.rs` are backward-compatible shims for existing test callers; they avoid editing out-of-scope test files but should be reviewed for API naming consistency.
- Validator focus request: after Coder completes the widened same-WP repair, verify the full MT-001 range including the two integration-test proof-blocker files.
- Rubric contract understanding proof: MT-001 repair followed the Orchestrator same-WP correction and remained in the assigned feature worktree.
- Rubric scope discipline proof: product edits were limited to `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs`; `../handshake_main` was not edited.
- Rubric baseline comparison: candidate range `d104c127e258a027fb51bc28cd3ed52e53874c92..d31da029`.
- Rubric end-to-end proof: `cargo check`, Coder's filter-form `mex_tests`, Orchestrator's precise `--test mex_tests` cross-check, and direct `micro_task_executor_tests` proof commands pass.
- Rubric architecture fit self-review: delimiter fixes restore existing event/session checkpoint behavior; serialization errors are mapped into existing `WorkflowError::Terminal` handling.
- Rubric heuristic quality self-review: repair is intentionally narrow; no broad flight-recorder transplant was applied and no new production locus API was invented to satisfy stale test imports.
- Rubric anti-gaming / counterfactual check: without the delimiter repairs cargo cannot parse `flight_recorder/mod.rs`; without checkpoint serialization mapping, cargo fails in `workflows.rs`.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: handoff records exact passing commands and does not claim the broader calendar-sync semantics beyond this MT-001 compile/direct-test repair.
- Signed-scope debt ledger: NONE for the widened MT-001 compile-proof and direct-test blockers.
- Data contract self-check: runtime structured summary `next_action` values now use registered governed action ids; create-WP/task-board tests confirm profile extensions remain absent where the in-scope input type does not carry those fields.
- Next step / handoff hint: WP Validator should review commit range `d104c127e258a027fb51bc28cd3ed52e53874c92..d31da029` and the refreshed signed-scope manifest.

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
- REQUIREMENT: "`src/backend/handshake_core/src/workflows.rs` compiles after restoring the missing `SessionCheckpoint` import or equivalent in-scope reference."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:53` restores the `SessionCheckpoint` import used by `src/backend/handshake_core/src/workflows.rs:5305`.
- REQUIREMENT: "workflow compile/regression contract"
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:53`, `src/backend/handshake_core/src/workflows.rs:5293`, `src/backend/handshake_core/src/workflows.rs:5305`, and exact cargo command evidence below.
- REQUIREMENT: "Orchestrator direct same-WP repair: fix the unclosed delimiter reported at `src/flight_recorder/mod.rs:6180` so cargo can reach/prove the existing workflows.rs SessionCheckpoint import repair."
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:381`, `src/backend/handshake_core/src/flight_recorder/mod.rs:931`, and `src/backend/handshake_core/src/flight_recorder/mod.rs:4382` close the malformed delimiter paths; `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` returns EXIT_CODE 0.
- REQUIREMENT: "Widened MT-001 proof blocker: `NewModelSession` initializer missing checkpoint fields."
- EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:189` through `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:191` add `checkpoint_artifact_id`, `last_checkpoint_at`, and `checkpoint_count`.
- REQUIREMENT: "Widened MT-001 proof blocker: unresolved `workflows::locus` imports in `micro_task_executor_tests.rs`."
- EVIDENCE: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:22`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:320`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:339` use the current task-board export and preserve governed-action assertions locally.
- REQUIREMENT: "WP Validator STEER: direct `micro_task_executor_tests` binary must pass and continue rejecting unregistered or workflow-family-illegal `next_action` values."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:3525`, `src/backend/handshake_core/src/workflows.rs:5196`, `src/backend/handshake_core/src/workflows.rs:12989`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1299`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:2175`.
- REQUIREMENT: "MEX validation commands in MT executor tests must execute from the assigned worktree rather than hard-gating on terminal cwd before running."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:12336` preserves normalized in-scope paths while adding repo-root cwd allowance for the shell validation operation; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` returns EXIT_CODE 0.
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Calendar-Sync-Engine-v3/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
- COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: `1`
- PROOF_LINES: `error: this file contains an unclosed delimiter`; `--> src\flight_recorder\mod.rs:6180:3`; `error: could not compile handshake_core (lib) due to 1 previous error`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests`
- EXIT_CODE: `1`
- PROOF_LINES: `error: this file contains an unclosed delimiter`; `--> src\flight_recorder\mod.rs:6180:3`; `warning: build failed, waiting for other jobs to finish...`
- COMMAND: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range facce56f879d4ee990f62566b12a8b26d8bc61d7..HEAD`
- EXIT_CODE: `1`
- PROOF_LINES: `post-work-check: FAIL`; `EVIDENCE_MAPPING has no file:line evidence`; `EVIDENCE must include at least one COMMAND + EXIT_CODE entry`; `STATUS_HANDOFF missing concrete field: Next step / handoff hint`
- COMMAND: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range d104c127e258a027fb51bc28cd3ed52e53874c92..85673ab51a29a26eb6d7f1a8fd76bb5aeccd1551`
- EXIT_CODE: `0`
- PROOF_LINES: `post-work-check: PASS`; `role-mailbox-export-check: PASS`; `wp-communication-health-check: PASS`; `RESULT: PASS`
- COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: `0`
- PROOF_LINES: `Finished dev profile`; `handshake_core (lib) generated 39 warnings`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests`
- EXIT_CODE: `1`
- PROOF_LINES: `error[E0063]: missing fields checkpoint_artifact_id, checkpoint_count and last_checkpoint_at in initializer of NewModelSession`; `error[E0432]: unresolved imports handshake_core::workflows::locus::is_governed_action_id_allowed_for_workflow_family, is_registered_governed_action_id, validate_task_board_entry_authoritative_fields`; `error: unable to delete old work product index ... os error 32`
- COMMAND: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range d104c127e258a027fb51bc28cd3ed52e53874c92..7b5519e13339ef06fafc6ec63d1e768068d038a0`
- EXIT_CODE: `0`
- PROOF_LINES: `post-work-check: PASS`; `role-mailbox-export-check: PASS`; `wp-communication-health-check: PASS`; `RESULT: PASS`
- COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: `0`
- PROOF_LINES: `Finished dev profile`; `handshake_core (lib) generated 39 warnings`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests`
- EXIT_CODE: `0`
- PROOF_LINES: `Finished test profile`; `Running tests\mex_tests.rs`; `test result: ok`
- COMMAND: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range d104c127e258a027fb51bc28cd3ed52e53874c92..HEAD`
- EXIT_CODE: `0`
- PROOF_LINES: `post-work-check: PASS`; `role-mailbox-export-check: PASS`; `wp-communication-health-check: PASS`; `RESULT: PASS`
- COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: `0`
- PROOF_LINES: `Finished dev profile`; `handshake_core (lib) generated 39 warnings`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests`
- EXIT_CODE: `0`
- PROOF_LINES: `Finished test profile`; `Running tests\mex_tests.rs`; `test result: ok`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests`
- EXIT_CODE: `0`
- PROOF_LINES: `running 27 tests`; `test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests`
- EXIT_CODE: `0`
- PROOF_LINES: `running 13 tests`; `test result: ok. 9 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out`
- COMMAND: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range d104c127e258a027fb51bc28cd3ed52e53874c92..HEAD`
- EXIT_CODE: `0`
- PROOF_LINES: `post-work-check: PASS`; `role-mailbox-export-check: PASS`; `wp-communication-health-check: PASS`; `RESULT: PASS`
- COMMAND: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range d104c127e258a027fb51bc28cd3ed52e53874c92..d31da029ecf682ca1b94b2794f20c212c543195f`
- EXIT_CODE: `0`
- PROOF_LINES: `post-work-check: PASS`; `role-mailbox-export-check: PASS`; `wp-communication-health-check: PASS`; `RESULT: PASS`

## VALIDATION_REPORTS
### INTEGRATION_VALIDATOR_VALIDATION_REPORT_CANONICAL_2026-04-25T23:34:59Z
- Reviewer: INTEGRATION_VALIDATOR
- Session: integration_validator:wp-1-calendar-sync-engine-v3 / repomem `INTEGRATION_VALIDATOR-20260425-231207`
- Review type: Final whole-WP Integration Validator report for candidate `05df783915b340efc8b6f5b180483e340710f04c`.
- Range: `d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c`
- Main baseline reviewed: `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`
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
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
WORKFLOW_VALIDITY: VALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: PROVEN
INTEGRATION_READINESS: READY
DOMAIN_GOAL_COMPLETION: COMPLETE
MECHANICAL_TRACK_VERDICT: PASS
SPEC_RETENTION_TRACK_VERDICT: PASS
CLAUSES_REVIEWED:
  - v2 Integration Validator compile blockers: PASS via `cargo check`, `src/backend/handshake_core/src/workflows.rs:54`, and `src/backend/handshake_core/src/workflows.rs:5298`.
  - deterministic final-lane handoff reproducibility: PASS via HANDOFF artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Calendar-Sync-Engine-v3/2026-04-25T23-17-26-244Z.log` and merge-tree result `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - surface mutation discipline plus write gate: PASS via `src/backend/handshake_core/mechanical_engines.json:2`, `src/backend/handshake_core/src/workflows.rs:12295`, and `src/backend/handshake_core/src/workflows.rs:12441`.
  - workflow capability profile and required-capabilities contract: PASS via `src/backend/handshake_core/src/capabilities.rs:13-14`, `:146-151`, `:382`, and `:411`.
  - Cross-Tool Interaction Map no-shadow-pipeline rule: PASS via `src/backend/handshake_core/src/workflows.rs:8593`, `:12555`, and `:12585`.
  - `calendar_sync` engine contract and output: PASS via `src/backend/handshake_core/mechanical_engines.json:22-26`, `src/backend/handshake_core/src/workflows.rs:12261`, and `:12585`.
  - `CalendarSourceSyncState` as single source of truth for recovery: PASS via `src/backend/handshake_core/src/storage/calendar.rs:124`, `src/backend/handshake_core/src/workflows.rs:12410`, and `:12441`.
  - MCP/provider adapter guidance plus read-only mode: PASS via `src/backend/handshake_core/src/workflows.rs:12295`, `:12470-12475`, and `src/backend/handshake_core/tests/mex_tests.rs:870`.
NOT_PROVEN:
  - NONE
MAIN_BODY_GAPS:
  - NONE
QUALITY_RISKS:
  - NONE
VALIDATOR_RISK_TIER: HIGH
DIFF_ATTACK_SURFACES:
  - Current-main containment boundary between baseline `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e` and candidate `05df783915b340efc8b6f5b180483e340710f04c`.
  - Registry/runtime boundary between `mechanical_engines.json`, workflow dispatch, adapter output, and MEX result extraction.
  - Capability/runtime boundary for `CalendarSync` and `calendar.sync.read/write`.
  - Storage/recovery boundary for `CalendarSourceSyncState` and calendar source/event upserts.
INDEPENDENT_CHECKS_RUN:
  - `git rev-parse HEAD` in `../wtc-sync-engine-v3` confirmed `05df783915b340efc8b6f5b180483e340710f04c`.
  - `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e 05df783915b340efc8b6f5b180483e340710f04c` returned `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c` passed.
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` passed with `CARGO_TARGET_DIR=..\Handshake_Artifacts\handshake-cargo-target`.
  - Direct exact `mex_tests` binary probes passed for `calendar_sync_runtime_denies_wrong_profile_without_unknown_capability` and `calendar_sync_workflow_imports_read_only_source_and_updates_sync_state`.
  - Direct artifact script `node "$HANDSHAKE_GOV_ROOT/roles_shared/scripts/topology/artifact-hygiene-check.mjs"` passed after local active-topology `.cargo/config.toml` hygiene repair.
COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/workflows.rs:54` stopped importing `SessionCheckpoint`, construction at `src/backend/handshake_core/src/workflows.rs:5298` would break.
  - If `src/backend/handshake_core/src/workflows.rs:8593` stopped dispatching `calendar_sync`, workflow jobs would fall through instead of running the MEX-backed calendar path.
  - If `src/backend/handshake_core/src/capabilities.rs:382` or `:411` changed, the `CalendarSync` profile and required `calendar.sync.read` contract could regress.
  - If `src/backend/handshake_core/src/workflows.rs:12295` changed, read-only provider mutation denial could regress.
BOUNDARY_PROBES:
  - Main/candidate boundary: merge-tree against current `main` succeeded without conflict.
  - Registry/runtime boundary: `calendar_sync.run` registry rows match dispatch and adapter output paths.
  - Capability/runtime boundary: exact wrong-profile denial behavior passed.
  - Storage/output boundary: exact read-only import test passed and code persists sync state through `upsert_calendar_source`.
NEGATIVE_PATH_CHECKS:
  - Wrong-profile runtime denial passed at `src/backend/handshake_core/tests/mex_tests.rs:707`.
  - Read-only source import passed while asserting `remote_write_attempted=false` and `read_only_source=true` at `src/backend/handshake_core/tests/mex_tests.rs:870`, `:960`, and `:966`.
  - Static negative proof confirmed multi-provider breadth and new provider MCP wrapper implementation remain outside this packet at `packet.md:761-767`.
INDEPENDENT_FINDINGS:
  - No product blocker remains for the packet scope.
  - The signed candidate preserves calendar-sync registry, dispatch, capability routing, denied-output, sync-state durability, and provider-safe evidence.
  - The active candidate worktree inherited stale `.cargo/config.toml`; local repair to `../Handshake_Artifacts/handshake-cargo-target` made artifact hygiene pass without changing the reviewed candidate commit.
RESIDUAL_UNCERTAINTY:
  - Canonical-target `cargo test` exact rebuild hit native `libduckdb-sys` build pressure and one reduced-jobs retry timed out; direct prebuilt exact behavior probes and `cargo check` still passed.
  - Real external provider/MCP breadth remains intentionally outside this WP.
SPEC_CLAUSE_MAP:
  - packet.md:791 compile after `SessionCheckpoint` repair: PASS, `src/backend/handshake_core/src/workflows.rs:54`, `src/backend/handshake_core/src/workflows.rs:5298`, and `cargo check`.
  - packet.md:792 `run_calendar_sync_job` params borrow fix: PASS, `src/backend/handshake_core/src/workflows.rs:12518`.
  - packet.md:793 registry/runtime dispatch: PASS, `src/backend/handshake_core/mechanical_engines.json:2`, `:22-26`, `src/backend/handshake_core/src/workflows.rs:8593`, `:12261`, and `:12585`.
  - packet.md:793 capability routing: PASS, `src/backend/handshake_core/src/capabilities.rs:13-14`, `:146-151`, `:382`, and `:411`.
  - packet.md:793 denied output parity: PASS, `src/backend/handshake_core/src/workflows.rs:12147`, `:12566`, and `src/backend/handshake_core/tests/mex_tests.rs:707`.
  - packet.md:793 sync-state durability: PASS, `src/backend/handshake_core/src/workflows.rs:12410`, `:12441`, and `src/backend/handshake_core/tests/mex_tests.rs:870`.
  - packet.md:793 provider-safe evidence: PASS, `src/backend/handshake_core/src/workflows.rs:12470-12475` and `src/backend/handshake_core/tests/mex_tests.rs:1024`.
  - packet.md:794 targeted calendar runtime tests: PASS, `src/backend/handshake_core/tests/mex_tests.rs:707` and `:870` behavior probes passed.
  - packet.md:795 deterministic handoff proof: PASS, `packet.md:2055` records closeout-prep candidate metadata, HANDOFF phase-check artifact `2026-04-25T23-17-26-244Z.log`, and merge-tree result `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - packet.md:796 WP Validator and Integration Validator pass / main integration: PASS_FOR_VERDICT, `src/backend/handshake_core/src/workflows.rs:8593` remains merge-tree-compatible with current `main`; main containment is the next governed step.
  - packet.md:798-813 primitive exposure: PASS, `src/backend/handshake_core/src/storage/calendar.rs:47`, `:80`, `:124`, `:158`, `:222`, `:247`, `:398`, and `:431`.
NEGATIVE_PROOF:
  - Multi-provider breadth, rich bidirectional write-back UX, Calendar Lens UI, CalendarScopeHint / ACE policy-routing, correlation export, mailbox correlation, and a new provider MCP wrapper are not implemented in this WP; `packet.md:761-767` keeps them out of scope.
ANTI_VIBE_FINDINGS:
  - NONE
SIGNED_SCOPE_DEBT:
  - NONE
PRIMITIVE_RETENTION_PROOF:
  - Calendar source/write/sync/input/mutation/event/query primitives remain in `src/backend/handshake_core/src/storage/calendar.rs:47`, `:80`, `:124`, `:158`, `:222`, `:247`, `:398`, and `:431`.
  - Runtime imports consume calendar sync primitives at `src/backend/handshake_core/src/workflows.rs:72-73`.
PRIMITIVE_RETENTION_GAPS:
  - NONE
SHARED_SURFACE_INTERACTION_CHECKS:
  - `workflows.rs` import split preserves `SessionCheckpoint` at `src/backend/handshake_core/src/workflows.rs:54` while calendar storage imports remain at `:72-73`.
  - MEX registry/runtime/capability surfaces remain connected by `src/backend/handshake_core/mechanical_engines.json:22-26`, `src/backend/handshake_core/src/workflows.rs:8593`, and `src/backend/handshake_core/src/capabilities.rs:382`.
CURRENT_MAIN_INTERACTION_CHECKS:
  - Merge-tree against current `main` returned `bb37a5debab88b8ec1cee78d8c89696fa56636a6` while preserving `src/backend/handshake_core/src/workflows.rs:54` and `src/backend/handshake_core/src/workflows.rs:8593`.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c` passed for the signed product surface including `src/backend/handshake_core/src/workflows.rs:12518`.
DATA_CONTRACT_PROOF:
  - `calendar_sync_result` is emitted/extracted at `src/backend/handshake_core/src/workflows.rs:12261` and `:12585`.
  - Read-only/provider-safe evidence is emitted at `src/backend/handshake_core/src/workflows.rs:12470-12475` and asserted at `src/backend/handshake_core/tests/mex_tests.rs:960`, `:966`, and `:1024`.
DATA_CONTRACT_GAPS:
  - NONE

### VALIDATION_REPORT_REQUIREMENTS_AND_HISTORY
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

### WP_VALIDATOR_FINAL_WP_REVIEW_2026-04-25T18:23:40Z
- Reviewer: WP_VALIDATOR
- Session: wp_validator:wp-1-calendar-sync-engine-v3
- Review type: Final whole-WP review, not MT-only review.
- Range: `d104c127e258a027fb51bc28cd3ed52e53874c92..b3a05b6a7611a9e59c108d4b01ec128483d1c2f8`
- Candidate: `b3a05b6a7611a9e59c108d4b01ec128483d1c2f8`
- Main baseline reviewed: `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`
- Verdict: NOT_PROVEN
- Disposition: RETURN_TO_ORCHESTRATOR. Do not launch Integration Validator from this WP Validator report.

- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS - full-range WP_VALIDATOR HANDOFF phase-check passed with artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Calendar-Sync-Engine-v3/2026-04-25T18-22-39-372Z.log`.
- TEST_VERDICT: NOT_PROVEN - `cargo check` and `--test calendar_storage_tests` passed, but the packet-required exact `calendar_sync_runtime_denies_wrong_profile_without_unknown_capability` test ran zero tests.
- CODE_REVIEW_VERDICT: NOT_PROVEN - MT-001/MT-002 repairs are in scope, but the final packet's `calendar_sync` runtime/engine path is absent or unproven.
- HEURISTIC_REVIEW_VERDICT: FAIL - green filtered tests can hide a missing test; storage-only proof does not prove workflow runtime dispatch.
- SPEC_ALIGNMENT_VERDICT: FAIL - packet `DONE_MEANS` at packet.md:790-795 requires `calendar_sync` registry/runtime dispatch, `CalendarSync` capability routing, denied output parity, sync-state durability, and provider-safe evidence.
- ENVIRONMENT_VERDICT: PASS - no environment failure blocked review.
- DISPOSITION: RETURN_TO_ORCHESTRATOR
- LEGAL_VERDICT: NOT_PROVEN
- SPEC_CONFIDENCE: HIGH_FOR_NEGATIVE_PROOF; NOT_CONFIDENT_FOR_COMPLETION

- WORKFLOW_VALIDITY: VALID
- SCOPE_VALIDITY: IN_SCOPE
- PROOF_COMPLETENESS: NOT_PROVEN
- INTEGRATION_READINESS: NOT_READY
- DOMAIN_GOAL_COMPLETION: INCOMPLETE
- MAIN_CONTAINMENT_STATUS: NOT_REQUIRED
- MERGED_MAIN_COMMIT: NONE
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
- INTEGRATION_FINAL_VERDICT: NOT_READY

- MECHANICAL_TRACK_VERDICT: PARTIAL_PASS
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..b3a05b6a7611a9e59c108d4b01ec128483d1c2f8` passed.
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` passed from the product worktree using external cargo artifacts.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests` passed, 2 passed.
  - Full-range HANDOFF phase-check passed for WP_VALIDATOR.
  - The exact calendar-sync runtime test required by packet.md:794 ran zero tests, so mechanical proof is incomplete.
- SPEC_RETENTION_TRACK_VERDICT: FAIL
  - Calendar storage primitives are retained, but the packet-required calendar sync engine/runtime contract is absent or unproven.

- CLAUSES_REVIEWED:
  - packet.md:517 v2 Integration Validator compile blockers: PASS. `workflows.rs` constructs the checkpoint with `crate::storage::SessionCheckpoint` at src/backend/handshake_core/src/workflows.rs:5291, emits `FrEvtSessionCheckpointCreated` at src/backend/handshake_core/src/workflows.rs:5329, and the payload type exists at src/backend/handshake_core/src/flight_recorder/mod.rs:5508.
  - packet.md:518 deterministic final-lane handoff reproducibility: PASS. Full-range HANDOFF phase-check passed with artifact `2026-04-25T18-22-39-372Z.log`.
  - packet.md:519 surface mutation discipline plus write gate: NOT_PROVEN. Calendar storage write APIs exist at src/backend/handshake_core/src/storage/sqlite.rs:3614 and src/backend/handshake_core/src/storage/postgres.rs:4062, but no reviewed `calendar_sync` workflow-only mutation path exists in `src/backend/handshake_core/mechanical_engines.json` or src/backend/handshake_core/src/workflows.rs.
  - packet.md:520 workflow capability profile and required-capabilities contract: NOT_PROVEN. Calendar capability IDs exist at src/backend/handshake_core/src/capabilities.rs:13-23 and generic HSK-4001 denial exists at src/backend/handshake_core/src/mex/gates.rs:257-295, but no `CalendarSync` workflow binding or calendar-specific denied-output parity test exists.
  - packet.md:521 Cross-Tool Interaction Map no-shadow-pipeline rule: NOT_PROVEN. No `calendar_sync` MEX runtime path was found, so provider access cannot be verified as flowing through Workflow Engine plus MEX runtime.
  - packet.md:522 `calendar_sync` engine contract and output: NOT_PROVEN. Independent search found no `calendar_sync`, `CalendarSync`, or `run_calendar_sync_job` symbol in `src/backend/handshake_core/mechanical_engines.json`, src/backend/handshake_core/src/workflows.rs, or src/backend/handshake_core/tests/mex_tests.rs.
  - packet.md:523 `CalendarSourceSyncState` as single source of truth for recovery: PARTIAL. The primitive exists at src/backend/handshake_core/src/storage/calendar.rs:124 and sqlite/postgres persistence paths exist at src/backend/handshake_core/src/storage/sqlite.rs:3614 and src/backend/handshake_core/src/storage/postgres.rs:4062, but runtime engine recovery behavior is not proven.
  - packet.md:524 MCP/provider adapter guidance plus read-only mode: NOT_PROVEN. `CalendarSourceWritePolicy::ReadOnlyImport` exists at src/backend/handshake_core/src/storage/calendar.rs:47-68, but no provider adapter/runtime policy enforcement path exists in the reviewed calendar-sync workflow surface.

- NOT_PROVEN:
  - `run_calendar_sync_job` required by packet.md:792 is absent from the candidate.
  - `calendar_sync` registry/runtime dispatch required by packet.md:793 is absent from `mechanical_engines.json`, `workflows.rs`, and `mex_tests.rs`.
  - `CalendarSync` capability routing and calendar-specific denied output parity required by packet.md:793 are absent or unproven.
  - The exact test required by packet.md:794, `calendar_sync_runtime_denies_wrong_profile_without_unknown_capability`, ran zero tests.
  - Full `cargo test` evidence from the final WP review is not present and cannot substitute for the missing named runtime test.

- MAIN_BODY_GAPS:
  - Missing `calendar_sync` engine registration.
  - Missing workflow runtime implementation for `CalendarSync` / `run_calendar_sync_job`.
  - Missing calendar-specific MEX capability denial parity test.
  - Missing provider-safe calendar-sync runtime evidence and output contract proof.

- QUALITY_RISKS:
  - A filtered cargo command can report exit code 0 while running zero tests.
  - Storage conformance can pass while the workflow engine contract is not implemented.
  - Current-main merge compatibility does not prove domain completion.

- VALIDATOR_RISK_TIER: HIGH
- DIFF_ATTACK_SURFACES:
  - MT-001 compile repair in `workflows.rs` and `flight_recorder/mod.rs`.
  - MT-002 current-main compatibility repair in `locus/*`, `storage/locus_sqlite.rs`, `workflows.rs`, and `micro_task_executor_tests.rs`.
  - Calendar storage persistence versus calendar workflow runtime dispatch.
  - Mechanical engine registry, capability gate, and runtime denial behavior.
  - Packet evidence versus actually executable product symbols/tests.

- INDEPENDENT_CHECKS_RUN:
  - `git rev-parse HEAD` confirmed candidate `b3a05b6a7611a9e59c108d4b01ec128483d1c2f8`.
  - `git diff --name-status d104c127e258a027fb51bc28cd3ed52e53874c92..b3a05b6a7611a9e59c108d4b01ec128483d1c2f8` showed only expected product files.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..b3a05b6a7611a9e59c108d4b01ec128483d1c2f8` passed.
  - Full-range WP_VALIDATOR HANDOFF phase-check passed.
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests` passed, 2 passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` exited 0 but ran zero tests.
  - `rg` searches for `calendar_sync`, `CalendarSync`, `run_calendar_sync_job`, and the exact target test in `mechanical_engines.json`, `workflows.rs`, and `mex_tests.rs` found no matches.

- COUNTERFACTUAL_CHECKS:
  - If `crate::storage::SessionCheckpoint` at src/backend/handshake_core/src/workflows.rs:5291 were invalid, `cargo check` would fail; it passed.
  - If repo-root validation cwd were still rejected, the full-range HANDOFF phase-check would fail; it passed.
  - If `calendar_sync` runtime dispatch existed, a literal search should find an engine ID, runtime symbol, or test in `mechanical_engines.json`, `workflows.rs`, or `mex_tests.rs`; it found none.
  - If the packet-named calendar denial test existed, the exact filtered cargo command should run at least one test; it ran zero.

- BOUNDARY_PROBES:
  - Storage boundary: sqlite/postgres calendar storage conformance passed through `calendar_storage_tests`.
  - Runtime boundary: no calendar-sync workflow boundary found in `mechanical_engines.json` or `workflows.rs`.
  - Capability boundary: generic capability denial exists, but calendar-specific routing is not proven.
  - Current-main boundary: compatibility was recorded against product main `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e` and the validator full-range HANDOFF gate passed.

- NEGATIVE_PATH_CHECKS:
  - Exact missing-test probe: zero tests ran for `calendar_sync_runtime_denies_wrong_profile_without_unknown_capability`.
  - Missing-symbol probe: no `calendar_sync`, `CalendarSync`, or `run_calendar_sync_job` match in the registry/runtime/test surfaces.
  - Wrong-profile denial parity remains generic only; no calendar-specific negative path exists.

- INDEPENDENT_FINDINGS:
  - MT-001 and MT-002 are acceptable as scoped repair steps, but they do not satisfy the complete WP packet.
  - The final WP is not Integration Validator-ready because required calendar sync runtime semantics are absent or unproven.

- RESIDUAL_UNCERTAINTY:
  - Whether v3 was intended to narrow to only final-lane repair blockers is an authority question. The signed packet currently requires broader calendar-sync semantics, so WP_VALIDATOR cannot clear PASS without implementation or packet scope correction.

- SPEC_CLAUSE_MAP:
  - packet.md:791 compile after `SessionCheckpoint` repair: SATISFIED by src/backend/handshake_core/src/workflows.rs:5291 and `cargo check`.
  - packet.md:792 `run_calendar_sync_job` params borrow fix: NOT_IMPLEMENTED_OR_NOT_PRESENT; no `run_calendar_sync_job` symbol exists in the reviewed workflow surface.
  - packet.md:793 `calendar_sync` registry/runtime dispatch: NOT_IMPLEMENTED_OR_NOT_PRESENT; no `calendar_sync` entry or runtime path exists in `mechanical_engines.json` or `workflows.rs`.
  - packet.md:793 `CalendarSync` capability routing: NOT_PROVEN; calendar capability IDs exist at src/backend/handshake_core/src/capabilities.rs:13-23, but no `CalendarSync` engine binding is present.
  - packet.md:793 denied output parity: NOT_PROVEN; generic HSK-4001 checks exist at src/backend/handshake_core/src/mex/gates.rs:257-295 and src/backend/handshake_core/tests/mex_tests.rs:453, but no calendar-specific denied-output parity test exists.
  - packet.md:793 sync-state durability: PARTIAL; storage primitives and conformance exist at src/backend/handshake_core/src/storage/calendar.rs:124 and src/backend/handshake_core/src/storage/tests.rs:2123, but runtime recovery is not proven.
  - packet.md:793 provider-safe evidence: NOT_PROVEN; no provider adapter/runtime evidence path exists.
  - packet.md:794 targeted calendar runtime test: NOT_PROVEN; exact command ran zero tests.
  - packet.md:794 `calendar_storage_tests`: SATISFIED; `--test calendar_storage_tests` passed, 2 tests.
  - packet.md:795 deterministic handoff proof: SATISFIED; full-range WP_VALIDATOR HANDOFF phase-check passed.
  - packet.md:796 WP Validator/Integration Validator/main integration: NOT_SATISFIED; this report is NOT_PROVEN and Integration Validator must not launch from it.
  - packet.md:798-813 primitive exposure: PARTIAL; storage primitives exist at src/backend/handshake_core/src/storage/calendar.rs:8, :47, :80, :124, :160, :177, :210, :243, :275, :311, and :344, but `CalendarSyncInput` and `CalendarMutation` were not found in product source.

- NEGATIVE_PROOF:
  - Independent search found no `calendar_sync`, `CalendarSync`, or `run_calendar_sync_job` implementation in the reviewed registry/runtime/test surfaces.
  - The named exact calendar-sync runtime test command ran zero tests.

- ANTI_VIBE_FINDINGS:
  - Passing `cargo check`, `mex_tests`, `micro_task_executor_tests`, and `calendar_storage_tests` is not sufficient because they do not exercise a missing `calendar_sync` workflow.
  - Current-main merge compatibility is not evidence of domain completion.

- SIGNED_SCOPE_DEBT:
  - Not accepted for PASS: calendar-sync runtime, engine registration, capability routing, denied parity, provider-safe output evidence, and full packet test plan proof remain unresolved under the signed packet.

- PRIMITIVE_RETENTION_PROOF:
  - `CalendarSourceProviderType`: src/backend/handshake_core/src/storage/calendar.rs:8.
  - `CalendarSourceWritePolicy` including read-only import posture: src/backend/handshake_core/src/storage/calendar.rs:47.
  - `CalendarSyncStateStage`: src/backend/handshake_core/src/storage/calendar.rs:80.
  - `CalendarSourceSyncState`: src/backend/handshake_core/src/storage/calendar.rs:124.
  - `CalendarSourceUpsert`: src/backend/handshake_core/src/storage/calendar.rs:160.
  - `CalendarEventStatus`, `CalendarEventVisibility`, `CalendarEventExportMode`, `CalendarEvent`, `CalendarEventUpsert`, `CalendarEventWindowQuery`: src/backend/handshake_core/src/storage/calendar.rs:177, :210, :243, :275, :311, :344.
  - Storage conformance harness: src/backend/handshake_core/src/storage/tests.rs:2123.

- PRIMITIVE_RETENTION_GAPS:
  - `CalendarSyncInput` not found in product source.
  - `CalendarMutation` not found in product source.
  - Storage primitive retention does not prove engine/runtime primitive use.

- SHARED_SURFACE_INTERACTION_CHECKS:
  - Calendar storage shared surfaces pass sqlite/postgres conformance.
  - Governance/locus MT-002 shared-surface repair remains compile-clean.
  - MEX registry/runtime shared surface does not contain the packet-required calendar-sync engine path.

- CURRENT_MAIN_INTERACTION_CHECKS:
  - Candidate was reviewed at `b3a05b6a7611a9e59c108d4b01ec128483d1c2f8` and current-main compatibility had already been recorded against product main `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`.
  - Full-range HANDOFF phase-check passed after signed-scope regeneration.
  - Current-main compatibility does not clear missing domain behavior.

- DATA_CONTRACT_PROOF:
  - Calendar source/event storage schema contracts are exercised by `calendar_storage_tests`, which passed two conformance tests.
  - sqlite/postgres calendar source upsert and event query surfaces exist at src/backend/handshake_core/src/storage/sqlite.rs:3614, src/backend/handshake_core/src/storage/sqlite.rs:4215, src/backend/handshake_core/src/storage/postgres.rs:4062, and src/backend/handshake_core/src/storage/postgres.rs:4656.

- DATA_CONTRACT_GAPS:
  - No LLM-parseable `calendar_sync` engine output contract is emitted or tested because no calendar-sync runtime path is present.
  - No provider-safe evidence contract is implemented/proven.

- REMEDIATION_INSTRUCTIONS:
  - If the signed packet remains authoritative, route to Coder to implement the missing `calendar_sync` engine/runtime path, `run_calendar_sync_job`, capability routing, denied-output parity, provider/read-only behavior, sync-state recovery behavior, and the named exact runtime test.
  - If the intended scope is only MT compile/current-main repair, route to Orchestrator for explicit authority-level packet scope correction before another final PASS review.
  - After remediation or scope correction, rerun `cargo check`, the exact calendar-sync runtime test with nonzero test count, `--test mex_tests`, `--test calendar_storage_tests`, full `cargo test`, current-main merge-tree compatibility, and full-range HANDOFF phase-check before requesting another final WP review.

### WP_VALIDATOR_FINAL_WP_REVIEW_2026-04-25T21:01:40Z
- Reviewer: WP_VALIDATOR
- Session: wp_validator:wp-1-calendar-sync-engine-v3
- Review type: Final whole-WP remediation review after calendar-sync runtime implementation.
- Range: `d104c127e258a027fb51bc28cd3ed52e53874c92..ed73bab0f30e0329b308564ab24b94eb901f12db`
- Candidate: `ed73bab0f30e0329b308564ab24b94eb901f12db`
- Main baseline reviewed: `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`
- Verdict: NOT_PROVEN
- BLOCKER_CLASS: product compatibility
- Coder relaunch required: YES, unless Orchestrator corrects the product-main baseline or merge-tree procedure.
- Disposition: RETURN_TO_ORCHESTRATOR_FOR_CODER_REPAIR. Do not launch Integration Validator from this WP Validator report.

- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PARTIAL_PASS - full-range WP_VALIDATOR HANDOFF phase-check passed with artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Calendar-Sync-Engine-v3/2026-04-25T20-45-06-751Z.log`, but current-main compatibility failed independently.
- TEST_VERDICT: PASS_WITH_RESIDUAL_FULL_SUITE_FAILURES - WP-relevant targeted tests passed; broad `cargo test` remains red on residual non-calendar tests and therefore cannot be used as final clean-suite proof.
- CODE_REVIEW_VERDICT: NOT_PROVEN - calendar-sync obligations are implemented/proven at the feature-branch level, but the candidate does not merge cleanly with product main in `workflows.rs`.
- HEURISTIC_REVIEW_VERDICT: FAIL - the handoff claimed merge-tree compatibility, but independent merge-tree produced a content conflict.
- SPEC_ALIGNMENT_VERDICT: NOT_PROVEN - packet calendar-sync clauses are now substantially satisfied, but deterministic final-lane integration is not proven.
- ENVIRONMENT_VERDICT: PASS - no environment failure blocked review; the merge-tree result is deterministic product evidence.
- DISPOSITION: RETURN_TO_ORCHESTRATOR_FOR_CODER_REPAIR
- LEGAL_VERDICT: NOT_PROVEN
- SPEC_CONFIDENCE: HIGH_FOR_CALENDAR_RUNTIME; LOW_FOR_INTEGRATION_READINESS

- WORKFLOW_VALIDITY: VALID
- SCOPE_VALIDITY: IN_SCOPE
- PROOF_COMPLETENESS: NOT_PROVEN
- INTEGRATION_READINESS: NOT_READY
- DOMAIN_GOAL_COMPLETION: PARTIAL
- MAIN_CONTAINMENT_STATUS: MERGE_PENDING
- MERGED_MAIN_COMMIT: NONE
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
- INTEGRATION_FINAL_VERDICT: NOT_READY

- MECHANICAL_TRACK_VERDICT: FAIL
  - PASS: `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..ed73bab0f30e0329b308564ab24b94eb901f12db`.
  - PASS: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml`.
  - PASS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` ran 1 test and passed.
  - PASS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests` passed, 11 passed and 4 ignored.
  - PASS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests` passed, 2 passed.
  - PASS: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` passed, 27 passed.
  - FAIL: `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e ed73bab0f30e0329b308564ab24b94eb901f12db` exited 1 with `CONFLICT (content): Merge conflict in src/backend/handshake_core/src/workflows.rs`.
- SPEC_RETENTION_TRACK_VERDICT: PASS_FOR_CALENDAR_SYNC; FAIL_FOR_FINAL_LANE_COMPATIBILITY

- CLAUSES_REVIEWED:
  - packet.md:517 v2 Integration Validator compile blockers: PASS. `cargo check` passed; checkpoint construction remains at src/backend/handshake_core/src/workflows.rs:5270 and uses storage checkpoint creation at src/backend/handshake_core/src/workflows.rs:5304.
  - packet.md:518 deterministic final-lane handoff reproducibility: PARTIAL. HANDOFF phase-check passed, but independent current-main merge-tree failed against product main `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`.
  - packet.md:519 surface mutation discipline plus write gate: PASS. `calendar_sync` is registered in src/backend/handshake_core/mechanical_engines.json:2, the runtime adapter is registered at src/backend/handshake_core/src/workflows.rs:12647, and calendar writes flow through storage with `WriteContext::ai` at src/backend/handshake_core/src/workflows.rs:12365.
  - packet.md:520 workflow capability profile and required-capabilities contract: PASS. `calendar.sync.read/write` are canonical IDs at src/backend/handshake_core/src/capabilities.rs:13, `CalendarSync` profile is registered at src/backend/handshake_core/src/capabilities.rs:146, calendar workflow profile routing is at src/backend/handshake_core/src/capabilities.rs:381, and required read capability routing is at src/backend/handshake_core/src/capabilities.rs:410.
  - packet.md:521 Cross-Tool Interaction Map no-shadow-pipeline rule: PASS. `run_job` dispatches `calendar_sync` to `run_calendar_sync_job` at src/backend/handshake_core/src/workflows.rs:8589, which builds a MEX `PlannedOperation` and executes it at src/backend/handshake_core/src/workflows.rs:12560.
  - packet.md:522 `calendar_sync` engine contract and output: PASS. The engine operation exists in src/backend/handshake_core/mechanical_engines.json:22, `CalendarSyncEngineAdapter` validates engine/operation and emits `calendar_sync_result` at src/backend/handshake_core/src/workflows.rs:12224 and src/backend/handshake_core/src/workflows.rs:12257, and job outputs are extracted at src/backend/handshake_core/src/workflows.rs:12578.
  - packet.md:523 `CalendarSourceSyncState` as single source of truth for recovery: PASS_FOR_RUNTIME_SCOPE. Runtime sync-state update is built at src/backend/handshake_core/src/workflows.rs:12406 and persisted through `upsert_calendar_source` at src/backend/handshake_core/src/workflows.rs:12436; storage conformance passed.
  - packet.md:524 MCP/provider adapter guidance plus read-only mode: PASS_FOR_IN_SCOPE_STUB. Read-only mutation denial occurs at src/backend/handshake_core/src/workflows.rs:12292, provider-safe evidence is emitted at src/backend/handshake_core/src/workflows.rs:12471, and direct runtime tests cover read-only import plus denial parity at src/backend/handshake_core/tests/mex_tests.rs:707 and src/backend/handshake_core/tests/mex_tests.rs:870.

- NOT_PROVEN:
  - Current-main integration for `ed73bab0f30e0329b308564ab24b94eb901f12db` is not proven because merge-tree conflicts in `src/backend/handshake_core/src/workflows.rs`.
  - Full clean-suite proof is not available; broad `cargo test` still has residual failures in non-calendar tests.

- MAIN_BODY_GAPS:
  - `workflows.rs` must be repaired against product main `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e` while preserving both main's `SessionCheckpoint` import layout and the new calendar-sync storage imports/runtime path.
  - The handoff's no-conflict merge-tree claim is contradicted by WP Validator evidence and must be re-run after repair.

- QUALITY_RISKS:
  - Calendar-sync tests are now real and passing, but current-main conflict means Integration Validator cannot apply the candidate cleanly.
  - The exact calendar denial test takes over 60 seconds when run through the package-wide filter, so future proof should prefer `--test mex_tests` for targeted verification.
  - Residual full-suite failures are outside the calendar remediation path but remain suite debt for final integration visibility.

- VALIDATOR_RISK_TIER: HIGH
- DIFF_ATTACK_SURFACES:
  - `mechanical_engines.json` registry and MEX runtime adapter registration.
  - Capability profile/required-capability routing for `CalendarSync`.
  - `run_calendar_sync_job` input ownership, capability requests, output extraction, and denial handling.
  - Calendar storage durability and read-only/provider-safe posture.
  - Product-main merge boundary in `workflows.rs`.

- INDEPENDENT_CHECKS_RUN:
  - `git rev-parse HEAD` confirmed candidate `ed73bab0f30e0329b308564ab24b94eb901f12db`.
  - `rg` confirmed `calendar_sync`, `CalendarSync`, `run_calendar_sync_job`, `CalendarSyncInput`, and `CalendarMutation` symbols are now present.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..ed73bab0f30e0329b308564ab24b94eb901f12db` passed.
  - Full-range WP_VALIDATOR HANDOFF phase-check passed.
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` ran 1 test and passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests calendar_sync_workflow_imports_read_only_source_and_updates_sync_state -- --exact` ran 1 test and passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests` passed, 11 passed and 4 ignored.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests` passed, 2 passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests` passed, 27 passed.
  - `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e ed73bab0f30e0329b308564ab24b94eb901f12db` failed with a `workflows.rs` content conflict.
  - Exact residual probes: `flight_recorder_round_trip` reproduces as a non-calendar flight-recorder payload validation failure; `import_dedup_emits_fr_evt_loom_006` passes in isolation; `test_recover_session_from_checkpoint` and `test_mark_stalled_workflows_recovers_orphaned_active_session` reproduce as recovery-event lookup failures.

- COUNTERFACTUAL_CHECKS:
  - If src/backend/handshake_core/mechanical_engines.json:2 were removed, `calendar_sync.run` would not be registry-addressable by MEX.
  - If src/backend/handshake_core/src/workflows.rs:8589 were removed, workflow jobs with protocol `calendar_sync` would fall through to the prior fallback path instead of the calendar runtime.
  - If src/backend/handshake_core/src/capabilities.rs:410 were removed, `calendar_sync` could regress to UnknownCapability or wrong-profile behavior.
  - If src/backend/handshake_core/src/workflows.rs:12292 were removed, read-only source mutations could reach storage instead of failing closed.
  - If current-main merge conflict in `workflows.rs` is not resolved, Integration Validator cannot reproduce a clean merge regardless of local test success.

- BOUNDARY_PROBES:
  - Registry-to-runtime: `mechanical_engines.json` entry plus `build_mex_runtime` adapter registration reviewed.
  - Workflow-to-MEX: `run_job` dispatch and `run_calendar_sync_job` planned operation reviewed.
  - Capability producer/consumer: `CapabilityRegistry` profile/required-capability routing and MEX denial test reviewed.
  - Storage writer/reader: `calendar_sync_workflow_imports_read_only_source_and_updates_sync_state` and `calendar_storage_tests` reviewed.
  - Current-main boundary: merge-tree against `2ecd453c` failed.

- NEGATIVE_PATH_CHECKS:
  - Wrong profile denial test now runs one test and passes without `UnknownCapability`.
  - Read-only source import test passes and confirms `remote_write_attempted=false`, `read_only_source=true`, sync token/watermark persistence, and provider-safe flight-recorder evidence.
  - Residual full-suite failure probes confirm remaining failures are not calendar-sync tests, but they keep full-suite evidence red.

- INDEPENDENT_FINDINGS:
  - The prior missing-symbol/zero-test findings are resolved by `ed73bab0`.
  - Calendar-specific runtime, capability, denied-output, read-only/provider-safe, and storage durability proof is sufficient at the feature-branch level.
  - Final WP PASS is blocked by current-main product compatibility: `workflows.rs` conflicts when merging candidate `ed73bab0` with product main `2ecd453c`.

- RESIDUAL_UNCERTAINTY:
  - Real external provider adapter behavior remains stubbed/storage-backed, but this matches the packet's in-scope provider-safe evidence posture.
  - Full-suite residual failures are not caused by ed73's calendar code, but they should remain visible to Integration Validator/Orchestrator as suite debt.

- SPEC_CLAUSE_MAP:
  - packet.md:791 compile after `SessionCheckpoint` repair: SATISFIED by passing `cargo check`.
  - packet.md:792 `run_calendar_sync_job` params borrow fix: SATISFIED at src/backend/handshake_core/src/workflows.rs:12515, where `job.job_inputs` is cloned before params mutation/use.
  - packet.md:793 `calendar_sync` registry/runtime dispatch: SATISFIED by src/backend/handshake_core/mechanical_engines.json:2 and src/backend/handshake_core/src/workflows.rs:8589.
  - packet.md:793 `CalendarSync` capability routing: SATISFIED by src/backend/handshake_core/src/capabilities.rs:146, :381, and :410.
  - packet.md:793 denied output parity: SATISFIED by src/backend/handshake_core/src/workflows.rs:12144 and test coverage at src/backend/handshake_core/tests/mex_tests.rs:707.
  - packet.md:793 sync-state durability: SATISFIED by src/backend/handshake_core/src/workflows.rs:12406 and `calendar_storage_tests`.
  - packet.md:793 provider-safe evidence: SATISFIED by src/backend/handshake_core/src/workflows.rs:12471 and test coverage at src/backend/handshake_core/tests/mex_tests.rs:870.
  - packet.md:794 targeted calendar runtime test: SATISFIED; exact command ran 1 test and passed.
  - packet.md:794 `mex_tests`: SATISFIED; `--test mex_tests` passed 11/4 ignored.
  - packet.md:794 `calendar_storage_tests`: SATISFIED; 2 passed.
  - packet.md:794 full `cargo test`: RESIDUAL_NOT_CLEAN; failures are outside calendar-sync remediation but full clean-suite proof is unavailable.
  - packet.md:795 deterministic handoff proof: PARTIAL; HANDOFF gate passed but current-main merge-tree failed.
  - packet.md:796 WP Validator/Integration Validator/main integration: NOT_SATISFIED; this report is NOT_PROVEN and Integration Validator must not launch.
  - packet.md:798-813 primitive exposure: SATISFIED for reviewed source primitives; `CalendarSyncInput` exists at src/backend/handshake_core/src/storage/calendar.rs:158 and `CalendarMutation` exists at src/backend/handshake_core/src/storage/calendar.rs:222.

- NEGATIVE_PROOF:
  - Current-main compatibility is not fully implemented/proven: merge-tree reports `CONFLICT (content): Merge conflict in src/backend/handshake_core/src/workflows.rs`.
  - Full clean-suite proof is not fully implemented/proven: `flight_recorder_round_trip`, `test_recover_session_from_checkpoint`, and `test_mark_stalled_workflows_recovers_orphaned_active_session` still fail in targeted probes.

- ANTI_VIBE_FINDINGS:
  - A passing HANDOFF gate did not prove current-main compatibility; independent merge-tree contradicted the handoff claim.
  - Calendar-specific green tests are real now, but they cannot compensate for a reproducible main-merge conflict.

- SIGNED_SCOPE_DEBT:
  - Resolve `workflows.rs` conflict against product main `2ecd453c` while preserving `calendar_sync` imports, dispatch, adapter, and MT-001 root-cwd validation behavior.
  - Re-run no-conflict merge-tree after repair and regenerate signed-scope/manifest evidence as required by Orchestrator.

- PRIMITIVE_RETENTION_PROOF:
  - `CalendarSyncInput`: src/backend/handshake_core/src/storage/calendar.rs:158.
  - `CalendarMutation`: src/backend/handshake_core/src/storage/calendar.rs:222.
  - `CalendarSourceProviderType`, `CalendarSourceWritePolicy`, `CalendarSyncStateStage`, `CalendarSourceSyncState`, `CalendarSourceUpsert`, `CalendarEvent`, `CalendarEventUpsert`, and `CalendarEventWindowQuery` remain present in src/backend/handshake_core/src/storage/calendar.rs.

- PRIMITIVE_RETENTION_GAPS:
  - NONE for feature-branch calendar primitives.

- SHARED_SURFACE_INTERACTION_CHECKS:
  - MEX registry/runtime/capability surfaces now interact correctly for calendar denial and read-only import tests.
  - Locus/micro-task shared-surface regression test remains passing.
  - Current-main `workflows.rs` interaction fails merge-tree and requires repair.

- CURRENT_MAIN_INTERACTION_CHECKS:
  - FAIL: `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e ed73bab0f30e0329b308564ab24b94eb901f12db` produced a `workflows.rs` content conflict.
  - The old 3-way merge-tree also shows conflicting `workflows.rs` import layout around `SessionCheckpoint` versus the new calendar storage imports.

- DATA_CONTRACT_PROOF:
  - `calendar_sync_result` output is emitted by the adapter provenance environment at src/backend/handshake_core/src/workflows.rs:12257 and job output extraction at src/backend/handshake_core/src/workflows.rs:12578.
  - Calendar source/event storage contracts passed through `calendar_storage_tests`.
  - Provider-safe evidence fields are asserted by `calendar_sync_workflow_imports_read_only_source_and_updates_sync_state`.

- DATA_CONTRACT_GAPS:
  - NONE for the feature-branch calendar-sync data contract.
  - Current-main conflict prevents asserting the integrated data contract.

- REMEDIATION_INSTRUCTIONS:
  - Relaunch Coder for product compatibility repair on the same WP branch.
  - Resolve `src/backend/handshake_core/src/workflows.rs` against product main `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`, preserving main's current `SessionCheckpoint`/workflow layout, MT-001 repo-root validation behavior, and ed73's calendar-sync imports, dispatch, adapter registration, denied output, read-only/provider-safe behavior, and tests.
  - After repair, rerun: `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e <candidate>`; `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..<candidate>`; `cargo check`; exact calendar-sync denial test; `--test mex_tests`; `--test calendar_storage_tests`; `--test micro_task_executor_tests`; full-range HANDOFF phase-check with the official gov root.

### WP_VALIDATOR_FINAL_WP_REVIEW_2026-04-25T22:32:50Z
- Reviewer: WP_VALIDATOR
- Session: wp_validator:wp-1-calendar-sync-engine-v3
- Review type: Final whole-WP current-main compatibility re-review after Coder repair commits `ade74b4b` and `05df7839`.
- Range: `d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c`
- Candidate: `05df783915b340efc8b6f5b180483e340710f04c`
- Main baseline reviewed: `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`
- Verdict: PASS
- Disposition: RETURN_TO_ORCHESTRATOR_FOR_CLOSEOUT_PREP. This is WP Validator final review only; Integration Validator has not been launched by WP_VALIDATOR.

- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS - `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 WP_VALIDATOR --range d104c127e258a027fb51bc28cd3ed52e53874c92..HEAD` passed with artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Calendar-Sync-Engine-v3/2026-04-25T22-28-53-541Z.log`; `just phase-check VERDICT WP-1-Calendar-Sync-Engine-v3 WP_VALIDATOR` passed with artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-verdict/WP-1-Calendar-Sync-Engine-v3/2026-04-25T22-32-23-823Z.log`.
- TEST_VERDICT: PASS - WP Validator reran `cargo check`, the exact calendar denial test, and the read-only calendar workflow test. Coder handoff evidence also covers `--test mex_tests`, `--test calendar_storage_tests`, and `--test micro_task_executor_tests`. Broad full-suite residual failures remain classified as non-calendar/outside this compatibility repair per Orchestrator instruction and prior validator probing.
- CODE_REVIEW_VERDICT: PASS - the post-ed73 diff is limited to import isolation in src/backend/handshake_core/src/workflows.rs and preserves the accepted calendar-sync runtime path.
- HEURISTIC_REVIEW_VERDICT: PASS
- SPEC_ALIGNMENT_VERDICT: PASS
- ENVIRONMENT_VERDICT: PASS
- DISPOSITION: RETURN_TO_ORCHESTRATOR_FOR_CLOSEOUT_PREP
- LEGAL_VERDICT: PASS
- SPEC_CONFIDENCE: HIGH

- WORKFLOW_VALIDITY: VALID
- SCOPE_VALIDITY: IN_SCOPE
- PROOF_COMPLETENESS: PROVEN
- INTEGRATION_READINESS: READY
- DOMAIN_GOAL_COMPLETION: COMPLETE
- MAIN_CONTAINMENT_STATUS: MERGE_PENDING
- MERGED_MAIN_COMMIT: NONE
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
- INTEGRATION_FINAL_VERDICT: NOT_RUN_BY_WP_VALIDATOR

- MECHANICAL_TRACK_VERDICT: PASS
  - `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e HEAD` returned tree `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..HEAD` passed.
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` passed, 1 test.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests calendar_sync_workflow_imports_read_only_source_and_updates_sync_state -- --exact` passed, 1 test.
  - HANDOFF and VERDICT phase-checks passed for WP_VALIDATOR.
- SPEC_RETENTION_TRACK_VERDICT: PASS

- CLAUSES_REVIEWED:
  - packet.md:517 v2 Integration Validator compile blockers: PASS. `SessionCheckpoint` is restored into the grouped storage import at src/backend/handshake_core/src/workflows.rs:52-55 and checkpoint construction uses the imported type at src/backend/handshake_core/src/workflows.rs:5295; `cargo check` passed.
  - packet.md:518 deterministic final-lane handoff reproducibility: PASS. Full-range HANDOFF and VERDICT phase-checks passed, and current-main merge-tree now returns tree `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - packet.md:519 surface mutation discipline plus write gate: PASS. `calendar_sync` remains registered in src/backend/handshake_core/mechanical_engines.json:2 and the adapter remains registered at src/backend/handshake_core/src/workflows.rs:12650-12652.
  - packet.md:520 workflow capability profile and required-capabilities contract: PASS. `CalendarSync` profile/routing remains at src/backend/handshake_core/src/capabilities.rs:146, :381, and :410.
  - packet.md:521 Cross-Tool Interaction Map no-shadow-pipeline rule: PASS. `run_job` still dispatches `calendar_sync` through `run_calendar_sync_job` at src/backend/handshake_core/src/workflows.rs:8592-8593, and the job executes through MEX at src/backend/handshake_core/src/workflows.rs:12563.
  - packet.md:522 `calendar_sync` engine contract and output: PASS. `calendar_sync.run` remains in src/backend/handshake_core/mechanical_engines.json:22 and `calendar_sync_result` output remains emitted/extracted at src/backend/handshake_core/src/workflows.rs:12261 and src/backend/handshake_core/src/workflows.rs:12585.
  - packet.md:523 `CalendarSourceSyncState` as recovery source of truth: PASS. Sync-state update remains built at src/backend/handshake_core/src/workflows.rs:12410 and persisted by calendar source upsert at src/backend/handshake_core/src/workflows.rs:12439.
  - packet.md:524 MCP/provider adapter guidance plus read-only mode: PASS. Read-only mutation denial remains at src/backend/handshake_core/src/workflows.rs:12295, provider-safe evidence remains at src/backend/handshake_core/src/workflows.rs:12474, and the read-only import test passes.

- NOT_PROVEN:
  - NONE

- MAIN_BODY_GAPS:
  - NONE

- QUALITY_RISKS:
  - NONE

- VALIDATOR_RISK_TIER: HIGH
- DIFF_ATTACK_SURFACES:
  - `workflows.rs` import merge boundary between product main's `SessionCheckpoint` layout and ed73 calendar storage imports.
  - Calendar-sync runtime symbol retention after import isolation.
  - Current-main merge-tree reproducibility against `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`.
  - MEX denial/read-only import tests as producer/consumer proof for capability and provider-safe output behavior.

- INDEPENDENT_CHECKS_RUN:
  - `git rev-parse HEAD` confirmed `05df783915b340efc8b6f5b180483e340710f04c`.
  - `git diff --name-status ed73bab0f30e0329b308564ab24b94eb901f12db..HEAD` showed only src/backend/handshake_core/src/workflows.rs.
  - `git diff ed73bab0f30e0329b308564ab24b94eb901f12db..HEAD -- src/backend/handshake_core/src/workflows.rs` confirmed the repair only restores `SessionCheckpoint` to the grouped storage import and moves calendar storage types to a separate import block.
  - `rg` confirmed `calendar_sync`, `CalendarSync`, `run_calendar_sync_job`, and `normalize_in_scope_paths_for_validation` symbols remain present.
  - `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e HEAD` returned tree `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..HEAD` passed.
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` passed.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mex_tests calendar_sync_workflow_imports_read_only_source_and_updates_sync_state -- --exact` passed.
  - `just phase-check HANDOFF ... WP_VALIDATOR --range d104c127e258a027fb51bc28cd3ed52e53874c92..HEAD` passed.
  - `just phase-check VERDICT WP-1-Calendar-Sync-Engine-v3 WP_VALIDATOR` passed.

- COUNTERFACTUAL_CHECKS:
  - If `SessionCheckpoint` were not restored in the grouped storage import at src/backend/handshake_core/src/workflows.rs:52-55, product-main merge would still conflict or checkpoint construction at src/backend/handshake_core/src/workflows.rs:5295 would lose the intended main-compatible path.
  - If the separate calendar storage import block at src/backend/handshake_core/src/workflows.rs:70-75 were removed, `CalendarSyncInput`, `CalendarMutationAction`, and sync-state types used by `run_calendar_sync_job` would fail compile or drift from the accepted runtime path.
  - If src/backend/handshake_core/src/workflows.rs:8592-8593 were removed, `workflow_run` jobs for `calendar_sync` would no longer dispatch to the MEX-backed calendar runtime.
  - If src/backend/handshake_core/src/workflows.rs:12295 were removed, read-only source mutation denial could regress.

- BOUNDARY_PROBES:
  - Main/candidate boundary: merge-tree against product main now succeeds.
  - Import producer/consumer boundary: `SessionCheckpoint` and calendar storage type consumers compile after import isolation.
  - Registry/runtime boundary: `calendar_sync` registry and adapter symbols remain present.
  - Capability/runtime boundary: exact wrong-profile calendar denial test passes.
  - Storage/output boundary: read-only import test passes and proves sync-state/provider-safe output preservation.

- NEGATIVE_PATH_CHECKS:
  - Wrong-profile calendar runtime denial passes without UnknownCapability.
  - Read-only source path proves provider import can complete with `remote_write_attempted=false`.
  - Negative proof confirms real multi-provider/MCP wrapper breadth is still not implemented and remains out of scope.

- INDEPENDENT_FINDINGS:
  - The only post-ed73 product diff is the intended import isolation in `workflows.rs`.
  - The prior current-main conflict in `workflows.rs` is cleared by merge-tree result `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - The accepted calendar-sync runtime/test proof is preserved.
  - No blocker remains for WP Validator final review.

- RESIDUAL_UNCERTAINTY:
  - Full external provider integration breadth is intentionally not proven because packet.md:761-767 excludes multi-provider breadth and new provider MCP wrapper implementation beyond preserving guidance/fail-closed posture.
  - Broad full-suite residual failures remain outside this WP Validator PASS per Orchestrator instruction and prior targeted classification; Integration Validator may choose how to handle them during final-lane policy.

- SPEC_CLAUSE_MAP:
  - packet.md:791 compile after `SessionCheckpoint` repair: PASS, src/backend/handshake_core/src/workflows.rs:52-55 and :5295 plus `cargo check`.
  - packet.md:792 `run_calendar_sync_job` params borrow fix: PASS, src/backend/handshake_core/src/workflows.rs:12518 clones job inputs before params mutation/use.
  - packet.md:793 `calendar_sync` registry/runtime dispatch: PASS, src/backend/handshake_core/mechanical_engines.json:2 and src/backend/handshake_core/src/workflows.rs:8592-8593.
  - packet.md:793 `CalendarSync` capability routing: PASS, src/backend/handshake_core/src/capabilities.rs:146, :381, and :410.
  - packet.md:793 denied output parity: PASS, src/backend/handshake_core/src/workflows.rs:12147 and test at src/backend/handshake_core/tests/mex_tests.rs:707.
  - packet.md:793 sync-state durability: PASS, src/backend/handshake_core/src/workflows.rs:12410 and read-only workflow test at src/backend/handshake_core/tests/mex_tests.rs:870.
  - packet.md:793 provider-safe evidence: PASS, src/backend/handshake_core/src/workflows.rs:12474 and src/backend/handshake_core/tests/mex_tests.rs:1024.
  - packet.md:794 targeted calendar runtime test: PASS, exact test ran and passed.
  - packet.md:794 `mex_tests` / `calendar_storage_tests` / `micro_task_executor_tests`: PASS by Coder proof, with WP Validator rerunning the calendar-specific mex tests directly.
  - packet.md:794 full cargo test: ACCEPTED_RESIDUAL_OUTSIDE_THIS_PASS per Orchestrator instruction; known failures are non-calendar and not introduced by the import-isolation repair.
  - packet.md:795 deterministic handoff proof: PASS, HANDOFF phase-check passed and current-main merge-tree succeeded.
  - packet.md:796 WP Validator pass: PASS. Integration Validator and main merge remain pending outside WP_VALIDATOR authority.
  - packet.md:798-813 primitive exposure: PASS, calendar primitives remain present and used by runtime imports at src/backend/handshake_core/src/workflows.rs:70-75.

- NEGATIVE_PROOF:
  - Real multi-provider breadth and new provider MCP wrapper implementation remain not implemented; this is verified as out-of-scope by packet.md:761-767, so it does not block PASS.

- ANTI_VIBE_FINDINGS:
  - NONE

- SIGNED_SCOPE_DEBT:
  - NONE

- PRIMITIVE_RETENTION_PROOF:
  - `CalendarSyncInput`, `CalendarMutationAction`, `CalendarSourceSyncState`, `CalendarSourceUpsert`, `CalendarSourceWritePolicy`, `CalendarSyncEventUpsert`, and `CalendarSyncStateStage` are imported at src/backend/handshake_core/src/workflows.rs:70-75 and consumed in the calendar runtime path.
  - `calendar_sync` engine and output primitive are present at src/backend/handshake_core/mechanical_engines.json:2 and :22-26.

- PRIMITIVE_RETENTION_GAPS:
  - NONE

- SHARED_SURFACE_INTERACTION_CHECKS:
  - `workflows.rs` import split preserves main-compatible `SessionCheckpoint` while keeping calendar storage type consumers compiling.
  - MEX registry/runtime/capability surfaces remain connected by code inspection and passing targeted tests.
  - Repo-root validation normalization remains present at src/backend/handshake_core/src/workflows.rs:12656.

- CURRENT_MAIN_INTERACTION_CHECKS:
  - `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e HEAD` succeeded with tree `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - Post-ed73 diff is limited to `workflows.rs` import isolation, the previous conflict site.

- DATA_CONTRACT_PROOF:
  - `calendar_sync_result` remains emitted/extracted at src/backend/handshake_core/src/workflows.rs:12261 and :12585.
  - Provider-safe output evidence remains at src/backend/handshake_core/src/workflows.rs:12474 and is tested at src/backend/handshake_core/tests/mex_tests.rs:870.

- DATA_CONTRACT_GAPS:
  - NONE

- REMEDIATION_INSTRUCTIONS:
  - NONE. Return to Orchestrator for closeout prep and Integration Validator launch decision.

### INTEGRATION_VALIDATOR_VALIDATION_REPORT_2026-04-25T23:34:59Z
- Reviewer: INTEGRATION_VALIDATOR
- Session: integration_validator:wp-1-calendar-sync-engine-v3 / repomem `INTEGRATION_VALIDATOR-20260425-231207`
- Review type: Final whole-WP Integration Validator report for candidate `05df783915b340efc8b6f5b180483e340710f04c`.
- Range: `d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c`
- Main baseline reviewed: `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`
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
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
WORKFLOW_VALIDITY: VALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: PROVEN
INTEGRATION_READINESS: READY
DOMAIN_GOAL_COMPLETION: COMPLETE
MECHANICAL_TRACK_VERDICT: PASS
SPEC_RETENTION_TRACK_VERDICT: PASS
CLAUSES_REVIEWED:
  - v2 Integration Validator compile blockers: PASS via `cargo check`, `src/backend/handshake_core/src/workflows.rs:54`, and `src/backend/handshake_core/src/workflows.rs:5298`.
  - deterministic final-lane handoff reproducibility: PASS via HANDOFF artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Calendar-Sync-Engine-v3/2026-04-25T23-17-26-244Z.log` and merge-tree result `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - surface mutation discipline plus write gate: PASS via `src/backend/handshake_core/mechanical_engines.json:2`, `src/backend/handshake_core/src/workflows.rs:12295`, and `src/backend/handshake_core/src/workflows.rs:12441`.
  - workflow capability profile and required-capabilities contract: PASS via `src/backend/handshake_core/src/capabilities.rs:13-14`, `:146-151`, `:382`, and `:411`.
  - Cross-Tool Interaction Map no-shadow-pipeline rule: PASS via `src/backend/handshake_core/src/workflows.rs:8593`, `:12555`, and `:12585`.
  - `calendar_sync` engine contract and output: PASS via `src/backend/handshake_core/mechanical_engines.json:22-26`, `src/backend/handshake_core/src/workflows.rs:12261`, and `:12585`.
  - `CalendarSourceSyncState` as single source of truth for recovery: PASS via `src/backend/handshake_core/src/storage/calendar.rs:124`, `src/backend/handshake_core/src/workflows.rs:12410`, and `:12441`.
  - MCP/provider adapter guidance plus read-only mode: PASS via `src/backend/handshake_core/src/workflows.rs:12295`, `:12470-12475`, and `src/backend/handshake_core/tests/mex_tests.rs:870`.
NOT_PROVEN:
  - NONE
MAIN_BODY_GAPS:
  - NONE
QUALITY_RISKS:
  - NONE
VALIDATOR_RISK_TIER: HIGH
DIFF_ATTACK_SURFACES:
  - Current-main containment boundary between baseline `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e` and candidate `05df783915b340efc8b6f5b180483e340710f04c`.
  - Registry/runtime boundary between `mechanical_engines.json`, workflow dispatch, adapter output, and MEX result extraction.
  - Capability/runtime boundary for `CalendarSync` and `calendar.sync.read/write`.
  - Storage/recovery boundary for `CalendarSourceSyncState` and calendar source/event upserts.
INDEPENDENT_CHECKS_RUN:
  - `git rev-parse HEAD` in `../wtc-sync-engine-v3` confirmed `05df783915b340efc8b6f5b180483e340710f04c`.
  - `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e 05df783915b340efc8b6f5b180483e340710f04c` returned `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c` passed.
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` passed with `CARGO_TARGET_DIR=..\Handshake_Artifacts\handshake-cargo-target`.
  - Direct exact `mex_tests` binary probes passed for `calendar_sync_runtime_denies_wrong_profile_without_unknown_capability` and `calendar_sync_workflow_imports_read_only_source_and_updates_sync_state`.
  - Direct artifact script `node "$HANDSHAKE_GOV_ROOT/roles_shared/scripts/topology/artifact-hygiene-check.mjs"` passed after local active-topology `.cargo/config.toml` hygiene repair.
COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/workflows.rs:54` stopped importing `SessionCheckpoint`, construction at `src/backend/handshake_core/src/workflows.rs:5298` would break.
  - If `src/backend/handshake_core/src/workflows.rs:8593` stopped dispatching `calendar_sync`, workflow jobs would fall through instead of running the MEX-backed calendar path.
  - If `src/backend/handshake_core/src/capabilities.rs:382` or `:411` changed, the `CalendarSync` profile and required `calendar.sync.read` contract could regress.
  - If `src/backend/handshake_core/src/workflows.rs:12295` changed, read-only provider mutation denial could regress.
BOUNDARY_PROBES:
  - Main/candidate boundary: merge-tree against current `main` succeeded without conflict.
  - Registry/runtime boundary: `calendar_sync.run` registry rows match dispatch and adapter output paths.
  - Capability/runtime boundary: exact wrong-profile denial behavior passed.
  - Storage/output boundary: exact read-only import test passed and code persists sync state through `upsert_calendar_source`.
NEGATIVE_PATH_CHECKS:
  - Wrong-profile runtime denial passed at `src/backend/handshake_core/tests/mex_tests.rs:707`.
  - Read-only source import passed while asserting `remote_write_attempted=false` and `read_only_source=true` at `src/backend/handshake_core/tests/mex_tests.rs:870`, `:960`, and `:966`.
  - Static negative proof confirmed multi-provider breadth and new provider MCP wrapper implementation remain outside this packet at `packet.md:761-767`.
INDEPENDENT_FINDINGS:
  - No product blocker remains for the packet scope.
  - The signed candidate preserves calendar-sync registry, dispatch, capability routing, denied-output, sync-state durability, and provider-safe evidence.
  - The active candidate worktree inherited stale `.cargo/config.toml`; local repair to `../Handshake_Artifacts/handshake-cargo-target` made artifact hygiene pass without changing the reviewed candidate commit.
RESIDUAL_UNCERTAINTY:
  - Canonical-target `cargo test` exact rebuild hit native `libduckdb-sys` build pressure and one reduced-jobs retry timed out; direct prebuilt exact behavior probes and `cargo check` still passed.
  - Real external provider/MCP breadth remains intentionally outside this WP.
SPEC_CLAUSE_MAP:
  - packet.md:791 compile after `SessionCheckpoint` repair: PASS, `src/backend/handshake_core/src/workflows.rs:54`, `src/backend/handshake_core/src/workflows.rs:5298`, and `cargo check`.
  - packet.md:792 `run_calendar_sync_job` params borrow fix: PASS, `src/backend/handshake_core/src/workflows.rs:12518`.
  - packet.md:793 registry/runtime dispatch: PASS, `src/backend/handshake_core/mechanical_engines.json:2`, `:22-26`, `src/backend/handshake_core/src/workflows.rs:8593`, `:12261`, and `:12585`.
  - packet.md:793 capability routing: PASS, `src/backend/handshake_core/src/capabilities.rs:13-14`, `:146-151`, `:382`, and `:411`.
  - packet.md:793 denied output parity: PASS, `src/backend/handshake_core/src/workflows.rs:12147`, `:12566`, and `src/backend/handshake_core/tests/mex_tests.rs:707`.
  - packet.md:793 sync-state durability: PASS, `src/backend/handshake_core/src/workflows.rs:12410`, `:12441`, and `src/backend/handshake_core/tests/mex_tests.rs:870`.
  - packet.md:793 provider-safe evidence: PASS, `src/backend/handshake_core/src/workflows.rs:12470-12475` and `src/backend/handshake_core/tests/mex_tests.rs:1024`.
  - packet.md:794 targeted calendar runtime tests: PASS, `src/backend/handshake_core/tests/mex_tests.rs:707` and `:870` behavior probes passed.
  - packet.md:795 deterministic handoff proof: PASS, `packet.md:2055` records closeout-prep candidate metadata, HANDOFF phase-check artifact `2026-04-25T23-17-26-244Z.log`, and merge-tree result `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - packet.md:796 WP Validator and Integration Validator pass / main integration: PASS_FOR_VERDICT, `src/backend/handshake_core/src/workflows.rs:8593` remains merge-tree-compatible with current `main`; main containment is the next governed step.
  - packet.md:798-813 primitive exposure: PASS, `src/backend/handshake_core/src/storage/calendar.rs:47`, `:80`, `:124`, `:158`, `:222`, `:247`, `:398`, and `:431`.
NEGATIVE_PROOF:
  - Multi-provider breadth, rich bidirectional write-back UX, Calendar Lens UI, CalendarScopeHint / ACE policy-routing, correlation export, mailbox correlation, and a new provider MCP wrapper are not implemented in this WP; `packet.md:761-767` keeps them out of scope.
ANTI_VIBE_FINDINGS:
  - NONE
SIGNED_SCOPE_DEBT:
  - NONE
PRIMITIVE_RETENTION_PROOF:
  - Calendar source/write/sync/input/mutation/event/query primitives remain in `src/backend/handshake_core/src/storage/calendar.rs:47`, `:80`, `:124`, `:158`, `:222`, `:247`, `:398`, and `:431`.
  - Runtime imports consume calendar sync primitives at `src/backend/handshake_core/src/workflows.rs:72-73`.
PRIMITIVE_RETENTION_GAPS:
  - NONE
SHARED_SURFACE_INTERACTION_CHECKS:
  - `workflows.rs` import split preserves `SessionCheckpoint` at `src/backend/handshake_core/src/workflows.rs:54` while calendar storage imports remain at `:72-73`.
  - MEX registry/runtime/capability surfaces remain connected by `src/backend/handshake_core/mechanical_engines.json:22-26`, `src/backend/handshake_core/src/workflows.rs:8593`, and `src/backend/handshake_core/src/capabilities.rs:382`.
CURRENT_MAIN_INTERACTION_CHECKS:
  - Merge-tree against current `main` returned `bb37a5debab88b8ec1cee78d8c89696fa56636a6` while preserving `src/backend/handshake_core/src/workflows.rs:54` and `src/backend/handshake_core/src/workflows.rs:8593`.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c` passed for the signed product surface including `src/backend/handshake_core/src/workflows.rs:12518`.
DATA_CONTRACT_PROOF:
  - `calendar_sync_result` is emitted/extracted at `src/backend/handshake_core/src/workflows.rs:12261` and `:12585`.
  - Read-only/provider-safe evidence is emitted at `src/backend/handshake_core/src/workflows.rs:12470-12475` and asserted at `src/backend/handshake_core/tests/mex_tests.rs:960`, `:966`, and `:1024`.
DATA_CONTRACT_GAPS:
  - NONE

## ORCHESTRATOR_CLOSEOUT_PREP_SYNC
- **MERGE_BASE_SHA**: `d104c127e258a027fb51bc28cd3ed52e53874c92`
- **COMMITTED_TARGET_HEAD_SHA**: `05df783915b340efc8b6f5b180483e340710f04c`
- Current closeout candidate range: `d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c`.
- Candidate reachability repaired for final-lane proof by fetching `feat/WP-1-Calendar-Sync-Engine-v3` from `../wtc-sync-engine-v3` into `../handshake_main` as `refs/remotes/wp/WP-1-Calendar-Sync-Engine-v3`.
- Current-main compatibility proof: `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e 05df783915b340efc8b6f5b180483e340710f04c` returned tree `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
- COMMAND: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c`
- Artifacts: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v3/signed-scope.patch` regenerated from the full closeout candidate range.
- Timestamp: 2026-04-25T22:49:08Z
- Operator: ORCHESTRATOR:closeout-prep-manual-remediation

### INTEGRATION_VALIDATOR_FINAL_REVIEW_2026-04-25T23:34:59Z
- Reviewer: INTEGRATION_VALIDATOR
- Session: integration_validator:wp-1-calendar-sync-engine-v3 / repomem `INTEGRATION_VALIDATOR-20260425-231207`
- Review type: Final whole-WP Integration Validator judgment and merge-pending PASS for candidate `05df783915b340efc8b6f5b180483e340710f04c`.
- Range: `d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c`
- Main baseline reviewed: `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`
- Verdict: PASS
- Disposition: MERGE_PENDING; Integration Validator containment to `main` follows this report.

- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS
  - `just phase-check STARTUP WP-1-Calendar-Sync-Engine-v3 INTEGRATION_VALIDATOR integration_validator:wp-1-calendar-sync-engine-v3` passed with artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-startup/WP-1-Calendar-Sync-Engine-v3/2026-04-25T23-12-37-398Z.log`.
  - `just phase-check CLOSEOUT WP-1-Calendar-Sync-Engine-v3` passed with artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-closeout/WP-1-Calendar-Sync-Engine-v3/2026-04-25T23-13-36-247Z.log`.
  - `just phase-check VERDICT WP-1-Calendar-Sync-Engine-v3 INTEGRATION_VALIDATOR` passed with artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-verdict/WP-1-Calendar-Sync-Engine-v3/2026-04-25T23-34-10-896Z.log`.
  - Targeted notifications were checked and acknowledged; no pending Integration Validator notifications were present.
- TEST_VERDICT: PASS_WITH_ENVIRONMENT_CAVEAT
  - `cargo check --manifest-path src/backend/handshake_core/Cargo.toml` with `CARGO_TARGET_DIR=..\Handshake_Artifacts\handshake-cargo-target` passed.
  - `git merge-tree --write-tree 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e 05df783915b340efc8b6f5b180483e340710f04c` returned tree `bb37a5debab88b8ec1cee78d8c89696fa56636a6`.
  - `git diff --check d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c` passed.
  - Exact behavior probes passed by direct execution of the existing `mex_tests` binary: `calendar_sync_runtime_denies_wrong_profile_without_unknown_capability` and `calendar_sync_workflow_imports_read_only_source_and_updates_sync_state`.
  - Canonical-target `cargo test --test mex_tests calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` hit a native `libduckdb-sys`/`cl.exe` build failure; one reduced-jobs retry timed out under host load. This is environment confidence debt, not a Rust assertion failure.
- CODE_REVIEW_VERDICT: PASS
- HEURISTIC_REVIEW_VERDICT: PASS
- SPEC_ALIGNMENT_VERDICT: PASS
- ENVIRONMENT_VERDICT: PASS_WITH_CAVEAT
- DISPOSITION: MERGE_PENDING
- LEGAL_VERDICT: PASS
- SPEC_CONFIDENCE: HIGH

- WORKFLOW_VALIDITY: VALID
- SCOPE_VALIDITY: IN_SCOPE
- PROOF_COMPLETENESS: PROVEN_FOR_PACKET_SCOPE_WITH_ENVIRONMENT_CAVEAT
- INTEGRATION_READINESS: READY
- DOMAIN_GOAL_COMPLETION: COMPLETE
- MAIN_CONTAINMENT_STATUS: MERGE_PENDING
- MERGED_MAIN_COMMIT: NONE
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
- INTEGRATION_FINAL_VERDICT: PASS

- MECHANICAL_TRACK_VERDICT: PASS
- SPEC_RETENTION_TRACK_VERDICT: PASS
- VALIDATOR_RISK_TIER: HIGH

- DIFF_ATTACK_SURFACES:
  - Current-main containment boundary: the candidate branch is based before current `main`, so Integration Validator must use merge-tree/merge containment rather than a raw diff against `main`.
  - Runtime producer/consumer boundary: `mechanical_engines.json`, workflow dispatch, adapter registration, and MEX output extraction must agree on `calendar_sync.run` and `calendar_sync_result`.
  - Capability boundary: `CalendarSync` profile routing and `calendar.sync.read/write` required-capability behavior must avoid falling back to Analyst/doc.summarize or `UnknownCapability`.
  - Storage/recovery boundary: `CalendarSourceSyncState` and event upsert identity must remain the durable recovery substrate.
  - Artifact boundary: the active candidate worktree inherited stale `.cargo/config.toml` target-dir text, which blocked the documented artifact hygiene gate until locally repaired to the canonical sibling root.

- INDEPENDENT_CHECKS_RUN:
  - `git rev-parse HEAD` in `../wtc-sync-engine-v3` confirmed candidate `05df783915b340efc8b6f5b180483e340710f04c`.
  - `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v3 CODER --range d104c127e258a027fb51bc28cd3ed52e53874c92..05df783915b340efc8b6f5b180483e340710f04c` passed from the declared candidate worktree with artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Calendar-Sync-Engine-v3/2026-04-25T23-17-26-244Z.log`.
  - Direct artifact script `node "$HANDSHAKE_GOV_ROOT/roles_shared/scripts/topology/artifact-hygiene-check.mjs"` passed after the local active-topology `.cargo/config.toml` repair; `just artifact-hygiene-check` is documented but absent from the active root justfile.
  - `rg`/code inspection confirmed registry, capability, dispatch, adapter, storage, read-only denial, provider-safe evidence, and primitive-retention symbols at the file:line citations below.
  - Exact behavior probes from the existing `mex_tests` executable passed for wrong-profile denial and read-only import/sync-state persistence.

- COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/workflows.rs:54` did not import `SessionCheckpoint`, checkpoint construction at `src/backend/handshake_core/src/workflows.rs:5298` would fail to compile or regress the current-main merge repair.
  - If `src/backend/handshake_core/src/workflows.rs:8593` were removed, `workflow_run` jobs for protocol `calendar_sync` would no longer dispatch to `run_calendar_sync_job`.
  - If `src/backend/handshake_core/src/capabilities.rs:382` or `src/backend/handshake_core/src/capabilities.rs:411` were removed, `calendar_sync` would lose the `CalendarSync` profile / required `calendar.sync.read` contract.
  - If `src/backend/handshake_core/src/workflows.rs:12295` were removed, read-only calendar sources could accept mutations instead of failing closed.
  - If `src/backend/handshake_core/src/workflows.rs:12441` were removed, runtime sync-state updates would not be persisted through calendar source upsert.

- BOUNDARY_PROBES:
  - Main/candidate boundary: merge-tree against current `main` produced a tree without conflict.
  - Registry/runtime boundary: `calendar_sync` registry entries at `src/backend/handshake_core/mechanical_engines.json:2` and `:22-26` match workflow dispatch at `src/backend/handshake_core/src/workflows.rs:8593` and adapter output extraction at `src/backend/handshake_core/src/workflows.rs:12261` and `:12585`.
  - Capability/runtime boundary: `calendar.sync.read/write` exist at `src/backend/handshake_core/src/capabilities.rs:13-14`, `CalendarSync` exists at `src/backend/handshake_core/src/capabilities.rs:146-151`, and the exact wrong-profile denial test passed.
  - Storage/output boundary: read-only import test passed and code inspection confirmed sync-state build/persist at `src/backend/handshake_core/src/workflows.rs:12410` and `:12441`.

- NEGATIVE_PATH_CHECKS:
  - Wrong-profile runtime denial passed: `src/backend/handshake_core/tests/mex_tests.rs:707`.
  - Read-only source import passed while asserting `remote_write_attempted=false` and `read_only_source=true`: `src/backend/handshake_core/tests/mex_tests.rs:870`, `:960`, and `:966`.
  - Static negative proof: multi-provider breadth and a new provider MCP wrapper are not implemented in this WP and remain expressly out of scope at `packet.md:761-767`.

- INDEPENDENT_FINDINGS:
  - No product blocker remains for the packet scope.
  - The signed candidate preserves the accepted calendar-sync runtime, capability, output, and storage contracts.
  - The active topology had stale artifact config in the candidate worktree; local repair changed only `.cargo/config.toml` from `../Handshake Artifacts/...` to `../Handshake_Artifacts/...` so artifact hygiene could pass. This does not change the reviewed candidate commit or signed product range.
  - `just artifact-hygiene-check` is documented in the command surface but absent from the active root justfile; direct script execution was used as the mandatory proof path.

- RESIDUAL_UNCERTAINTY:
  - Full `cargo test` was not rerun in this final lane because the host is under heavy load and canonical-target test compilation stalled in native DuckDB build work; previous WP Validator/coder evidence covers broader cargo test surfaces, while this Integration Validator review supplies direct compile, merge, static, and exact behavior probes.
  - Real external provider/MCP breadth remains outside this packet and should be handled by the named follow-on stubs, not treated as implemented here.

- SPEC_CLAUSE_MAP:
  - packet.md:791 compile after `SessionCheckpoint` repair: PASS, `SessionCheckpoint` import at `src/backend/handshake_core/src/workflows.rs:54`, construction at `src/backend/handshake_core/src/workflows.rs:5298`, and `cargo check` passed.
  - packet.md:792 `run_calendar_sync_job` params borrow fix: PASS, `src/backend/handshake_core/src/workflows.rs:12518` clones `job.job_inputs` before params mutation/use.
  - packet.md:793 `calendar_sync` registry/runtime dispatch: PASS, registry at `src/backend/handshake_core/mechanical_engines.json:2` and `:22-26`, workflow dispatch at `src/backend/handshake_core/src/workflows.rs:8593`, adapter output at `src/backend/handshake_core/src/workflows.rs:12261`, and output extraction at `src/backend/handshake_core/src/workflows.rs:12585`.
  - packet.md:793 `CalendarSync` capability routing: PASS, capability IDs at `src/backend/handshake_core/src/capabilities.rs:13-14`, profile at `src/backend/handshake_core/src/capabilities.rs:146-151`, workflow profile lookup at `src/backend/handshake_core/src/capabilities.rs:382`, and required read capability at `src/backend/handshake_core/src/capabilities.rs:411`.
  - packet.md:793 denied output parity: PASS, denied output helper at `src/backend/handshake_core/src/workflows.rs:12147`, denial path at `src/backend/handshake_core/src/workflows.rs:12566`, and exact test at `src/backend/handshake_core/tests/mex_tests.rs:707`.
  - packet.md:793 sync-state durability: PASS, sync-state build at `src/backend/handshake_core/src/workflows.rs:12410`, persistence through `upsert_calendar_source` at `src/backend/handshake_core/src/workflows.rs:12441`, and read-only workflow test at `src/backend/handshake_core/tests/mex_tests.rs:870`.
  - packet.md:793 provider-safe evidence: PASS, provider-safe fields at `src/backend/handshake_core/src/workflows.rs:12470-12475` and evidence assertion at `src/backend/handshake_core/tests/mex_tests.rs:1024`.
  - packet.md:794 targeted calendar runtime tests: PASS, direct exact probes passed for `src/backend/handshake_core/tests/mex_tests.rs:707` and `:870`; canonical cargo test rebuild was environment-limited as noted above.
  - packet.md:794 full `cargo test`: NOT_FULLY_PROVEN_IN_THIS_FINAL_LANE; accepted as residual environment debt because packet-scope compile, current-main merge, exact behavior probes, and prior validator/coder broader evidence are coherent.
  - packet.md:795 deterministic handoff proof: PASS, HANDOFF phase-check passed from the declared candidate worktree and merge-tree against current `main` succeeded.
  - packet.md:796 WP Validator and Integration Validator pass / main integration: PASS_FOR_VERDICT; main containment follows this report and must be recorded by closeout sync.
  - packet.md:798-813 primitive exposure: PASS, primitives remain present in `src/backend/handshake_core/src/storage/calendar.rs:47`, `:80`, `:124`, `:158`, `:222`, `:247`, `:398`, and `:431`, and are consumed by runtime imports at `src/backend/handshake_core/src/workflows.rs:72-73`.

- NEGATIVE_PROOF:
  - Real multi-provider breadth, rich bidirectional write-back UX, conflict-resolution policy, Calendar Lens UI, CalendarScopeHint / ACE policy-routing, calendar correlation export, mailbox correlation, and a new provider MCP wrapper are not implemented in this WP. This is verified against `packet.md:761-767`, so it is a known non-implementation outside this PASS scope rather than a blocker.

- ANTI_VIBE_FINDINGS:
  - NONE

- SIGNED_SCOPE_DEBT:
  - NONE

- DATA_CONTRACT_PROOF:
  - `calendar_sync_result` is emitted and extracted at `src/backend/handshake_core/src/workflows.rs:12261` and `:12585`.
  - Read-only/provider-safe evidence is emitted at `src/backend/handshake_core/src/workflows.rs:12470-12475` and asserted at `src/backend/handshake_core/tests/mex_tests.rs:960`, `:966`, and `:1024`.

- DATA_CONTRACT_GAPS:
  - NONE for packet scope.

- REMEDIATION_INSTRUCTIONS:
  - NONE for product code. Proceed to validator gate append/commit, task-board merge-pending update, local `main` containment, contained-main closeout sync, and governance sync to `main`.
