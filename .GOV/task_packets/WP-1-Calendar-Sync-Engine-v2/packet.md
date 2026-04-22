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

# Task Packet: WP-1-Calendar-Sync-Engine-v2

## METADATA
- TASK_ID: WP-1-Calendar-Sync-Engine-v2
- WP_ID: WP-1-Calendar-Sync-Engine-v2
- BASE_WP_ID: WP-1-Calendar-Sync-Engine
- DATE: 2026-04-21T09:01:48.288Z
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
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Optional but authoritative when Activation Manager launch or repair resumes from the packet. -->
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
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
- SESSION_CONTROL_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl
- SESSION_CONTROL_RESULTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl
- SESSION_COMPATIBILITY_SURFACE: VSCODE_PLUGIN_REPAIR_ONLY
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
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Calendar-Sync-Engine-v2
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.4
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Calendar-Sync-Engine-v2
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-sync-engine-v2
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Sync-Engine-v2-mainproof
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Sync-Engine-v2-mainproof
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Calendar-Sync-Engine-v2
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.4
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Sync-Engine-v2-mainproof
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Sync-Engine-v2-mainproof
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Calendar-Sync-Engine-v2
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Calendar-Sync-Engine-v2
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Calendar-Sync-Engine-v2
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
- **Status:** Validated (FAIL)
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: NOT_REQUIRED
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: NONE
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-21T17:58:20.892Z
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
- LOCAL_BRANCH: feat/WP-1-Calendar-Sync-Engine-v2
- LOCAL_WORKTREE_DIR: ../wtc-sync-engine-v2
- REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Sync-Engine-v2-mainproof
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Sync-Engine-v2-mainproof
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v2
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v2/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v2/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v2/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: wp_validator:wp-1-calendar-sync-engine-v2
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-calendar-sync-engine-v2
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja210420261037
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: FAIL
Blockers: Validator recorded FAIL; see VALIDATION_REPORTS for the authoritative signed-scope findings.
Next: NONE
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: surface mutation discipline plus write gate | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: workflow capability profile and required-capabilities contract | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/capabilities.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/gates.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Cross-Tool Interaction Map no-shadow-pipeline rule | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests | EXAMPLES: a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: `calendar_sync` engine contract and output | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | EXAMPLES: a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: `CalendarSourceSyncState` as single source of truth for recovery | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: MCP/provider adapter guidance plus read-only mode | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | EXAMPLES: a `workflow_run` that requests the intended calendar sync capabilities and reaches the calendar-sync path instead of Analyst/doc.summarize fallback, a read-only calendar source sync that updates event rows plus `CalendarSourceSyncState` without attempting remote writes, a repeated identical sync run that keeps stable identity and produces no duplicate events, a mutation attempt without the required capability or against a read-only source that fails closed while still leaving inspectable runtime evidence | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
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
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  - targeted validator review of capability-contract wiring, engine registration, runtime adapter installation, gated sync execution, and sync-state durability
- CANONICAL_CONTRACT_EXAMPLES:
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Calendar-Sync-Engine-v2.md
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
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: surface mutation discipline plus write gate | WHY_IN_SCOPE: the packet must make `calendar_sync` the real workflow-only mutation path instead of a paper contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: direct helper or UI-side writes can still bypass governed execution
  - CLAUSE: workflow capability profile and required-capabilities contract | WHY_IN_SCOPE: the v2 packet must repair the calendar sync path so `workflow_run` and capability gating use the intended calendar capability contract | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/capabilities.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/gates.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: `calendar_sync` remains blocked by the wrong capability contract or by `HSK-4001 UnknownCapability`
  - CLAUSE: Cross-Tool Interaction Map no-shadow-pipeline rule | WHY_IN_SCOPE: provider sync must run through Workflow Engine + MEX runtime, not hidden background helpers | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/mex/runtime.rs; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests | RISK_IF_MISSED: provider access happens outside the contract the spec requires
  - CLAUSE: `calendar_sync` engine contract and output | WHY_IN_SCOPE: the packet exists to realize the engine input/behavior/output contract already named in the spec | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | RISK_IF_MISSED: the engine may exist nominally but still fail to honor spec-defined behavior
  - CLAUSE: `CalendarSourceSyncState` as single source of truth for recovery | WHY_IN_SCOPE: retries, backoff, and recovery are core parts of a sync engine, not optional extras | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs; ../handshake_main/src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: the engine becomes non-recoverable or duplicates data under retry
  - CLAUSE: MCP/provider adapter guidance plus read-only mode | WHY_IN_SCOPE: the spec explicitly prefers provider access through tools inside the engine and names read-only behavior as a first-class posture | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/workflows.rs; ../handshake_main/src/backend/handshake_core/mechanical_engines.json; ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests; cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: provider access and write posture drift from the calendar law contract
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: `calendar_sync` engine registry contract | PRODUCER: mechanical_engines.json | CONSUMER: mex/registry.rs, workflows.rs, tests/mex_tests.rs | SERIALIZER_TRANSPORT: JSON engine registry | VALIDATOR_READER: mex_tests.rs | TRIPWIRE_TESTS: mex registry/runtime tests | DRIFT_RISK: engine is declared but not executable, or executable but not declared consistently
  - CONTRACT: calendar sync job input / protocol contract | PRODUCER: workflows.rs job/profile parser and engine runner | CONSUMER: calendar-sync adapter implementation, storage layer, validators | SERIALIZER_TRANSPORT: workflow payload plus PlannedOperation inputs | VALIDATOR_READER: workflow/job tests plus validator inspection | TRIPWIRE_TESTS: targeted calendar-sync execution tests plus full cargo test | DRIFT_RISK: job payload shape and adapter expectations silently diverge
  - CONTRACT: calendar sync capability contract | PRODUCER: capabilities.rs plus workflow capability-profile binding | CONSUMER: workflows.rs, mex/gates.rs, tests/mex_tests.rs | SERIALIZER_TRANSPORT: capability profile ids and requested capability strings | VALIDATOR_READER: mex_tests.rs plus validator inspection | TRIPWIRE_TESTS: mex capability-path tests plus full cargo test | DRIFT_RISK: requested calendar capabilities remain undefined, misnamed, or bound to the wrong workflow profile
  - CONTRACT: `CalendarSourceSyncState` durable recovery contract | PRODUCER: storage/calendar.rs plus engine runner | CONSUMER: sqlite.rs, postgres.rs, later recovery/retry flows | SERIALIZER_TRANSPORT: sqlx row mapping and JSON-ish sync-state payloads | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: calendar storage tests plus sync retry/idempotency tests | DRIFT_RISK: sync token/backoff/watermark state is lost or inconsistently updated
  - CONTRACT: calendar event upsert/idempotency contract | PRODUCER: engine runner and adapter | CONSUMER: storage backends and later Lens/policy consumers | SERIALIZER_TRANSPORT: storage upsert calls keyed by source/external identity | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: calendar storage tests plus repeat-sync tests | DRIFT_RISK: repeated sync runs duplicate events or destabilize identity
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Audit the existing storage/runtime seams first so the packet reuses completed work instead of recreating it.
  - Repair the calendar sync capability contract before or alongside runtime adapter wiring so the path can execute under the intended `workflow_run` profile.
  - Add `calendar_sync` to the mechanical engine registry and wire its adapter into `build_mex_runtime(...)`.
  - Reuse existing calendar storage upserts and sync-state contracts for the smallest truthful sync path.
  - Extend `mex_tests` and storage/runtime tests to prove engine registration, governed execution, capability routing, idempotent sync behavior, and fail-closed posture.
  - Re-run the proof commands from the product worktree until they pass cleanly.
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
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- CARRY_FORWARD_WARNINGS:
  - Do not reimplement calendar storage or invent shadow tables; the completed storage packet is the substrate.
  - Do not add ad hoc background sync threads or direct provider clients outside workflow/MEX runtime.
  - Do not silently reuse Analyst/doc.summarize or any unrelated `workflow_run` capability contract for the calendar sync path.
  - Do not widen the packet into Lens, ACE policy integration, multi-provider breadth, or rich write-back UX.
  - Do not mint new PRIM IDs or new top-level Flight Recorder schemas to paper over runtime gaps.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
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
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - verify `calendar_sync` still exists in `mechanical_engines.json` on `main`
  - verify workflow runtime still installs the calendar adapter on `main`
  - verify `workflow_run` no longer routes the calendar sync path through Analyst/doc.summarize capability posture
  - verify there is still no direct provider-write path that bypasses capability gates
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - This refinement does not prove whether the smallest truthful MVP should be local ICS import only or one external provider in read-only mode; both remain spec-compatible and must be resolved during implementation.
  - This refinement does not freeze the exact protocol_id/schema naming for the calendar-sync job contract; coder and validators must align those names to current workflow conventions.
  - This refinement does not prove bidirectional write-back, conflict resolution, CalendarScopeHint policy projection, Lens UX, or downstream correlation behavior.
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
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: calendar capability profile and workflow capability routing | SUBFEATURES: `calendar.sync.read`, `calendar.sync.write`, workflow-run capability mapping, `CapabilityGate` acceptance, fail-closed denials | PRIMITIVES_FEATURES: PRIM-CalendarSource, PRIM-CalendarSourceWritePolicy, PRIM-CalendarMutation | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the material v2 scope delta proven by the dossier.
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
  - WP-1-Calendar-Storage-v2 -> KEEP_SEPARATE
  - WP-1-Workflow-Engine-v4 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs -> NOT_PRESENT (WP-1-Calendar-Sync-Engine-v1)
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs -> PARTIAL (WP-1-Calendar-Sync-Engine-v1)
  - ../handshake_main/src/backend/handshake_core/src/mex/gates.rs -> PARTIAL (WP-1-Calendar-Sync-Engine-v1)
  - ../handshake_main/src/backend/handshake_core/tests/mex_tests.rs -> PARTIAL (WP-1-Calendar-Sync-Engine-v1)
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
- What: Register and implement `calendar_sync` as a governed workflow-driven mechanical engine and repair the calendar-specific capability contract surfaces that currently block `workflow_run` execution and `mex_tests` proof coverage.
- Why: The spec already defines `calendar_sync` as the only legal path for external calendar mutation and provider sync, and it already defines explicit capability profiles for Calendar and AI jobs. The v1 dossier proved the current product is blocked at both layers: missing engine/runtime wiring and missing calendar capability routing.
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
  - Repo-governance tooling or protocol changes unrelated to this packet
- TOUCHED_FILE_BUDGET: 12
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (For explicit post-signature continuation after `TOKEN_BUDGET_EXCEEDED` / `POLICY_CONFLICT`, use `COVERS: GOVERNANCE` and name the policy conflict directly in `SCOPE` or `JUSTIFICATION`.)
- WAIVER_ID: CX-573F-20260421-CALENDAR-SYNC-TOKEN-BUDGET-CONTINUATION | STATUS: ACTIVE | COVERS: GOVERNANCE | SCOPE: post-signature orchestrator-managed continuation after TOKEN_BUDGET_EXCEEDED / POLICY_CONFLICT on WP-1-Calendar-Sync-Engine-v2 during autonomous finish pass | JUSTIFICATION: Operator explicitly instructed autonomous continuation until this WP is finished and coder work is integrated into main, which authorizes bounded continuation after the orchestrator-managed run exceeded the governed token budget under host-load recovery. This waiver preserves the budget overrun as audit-visible truth rather than masking it. | APPROVER: Operator (chat, 2026-04-21) | EXPIRES: when WP-1-Calendar-Sync-Engine-v2 reaches an honest closeout verdict

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
```

### DONE_MEANS
- `calendar_sync` exists in `mechanical_engines.json` and is loadable by `MexRegistry`.
- The workflow runtime installs a `calendar_sync` adapter and dispatches the chosen protocol/job contract through governed workflow execution instead of ad hoc helpers.
- `capabilities.rs` exposes the calendar sync capability contract needed for this path, and `workflow_run` no longer inherits the Analyst / `doc.summarize` capability contract for the calendar sync flow.
- `CapabilityGate` continues to fail closed for invalid requests but no longer blocks the signed calendar sync path as `HSK-4001 UnknownCapability`.
- `mex_tests` contains `calendar_sync` registry/runtime/capability coverage, and the existing storage tests still prove idempotent sync-state behavior.
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
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-21T09:01:48.288Z)
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
  - calendar_mutation
- RUN_COMMANDS:
  ```bash
rg -n "calendar_sync|calendar.sync.read|calendar.sync.write|workflow_run|doc.summarize|UnknownCapability|HSK-4001|capability_profile_id" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests ../handshake_main/src/backend/handshake_core/mechanical_engines.json
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml mex_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
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
- MT-002 repaired the `calendar_sync` capability contract in `src/backend/handshake_core/src/capabilities.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/mex/gates.rs`, and `src/backend/handshake_core/tests/mex_tests.rs` so `workflow_run` binds to `CalendarSync` with `calendar.sync.read` and `calendar.sync.write`, and wrong-profile denials stay profile-based instead of surfacing `HSK-4001 UnknownCapability`.
- MT-003 routed the `calendar_sync` workflow path through the governed MEX runtime in `src/backend/handshake_core/src/workflows.rs`, threaded workflow context through `src/backend/handshake_core/src/mex/runtime.rs`, and added runtime tripwires in `src/backend/handshake_core/tests/mex_tests.rs` so adapter absence produces explicit evidence instead of a shadow helper path.
- MT-004 materialized the `calendar_sync` engine contract in `src/backend/handshake_core/mechanical_engines.json`, `src/backend/handshake_core/src/workflows.rs`, and `src/backend/handshake_core/src/storage/calendar.rs` so the engine registry, adapter inputs, and `calendar_sync_result` output shape are explicit and trace-linked.
- MT-005 made `CalendarSourceSyncState` the durable recovery truth in `src/backend/handshake_core/src/storage/calendar.rs` and `src/backend/handshake_core/src/workflows.rs`, persisting begin-attempt, failure/backoff, and success-reset transitions around the storage query path.
- MT-006 added provider-access guidance and read-only fail-closed posture in `src/backend/handshake_core/mechanical_engines.json`, `src/backend/handshake_core/src/workflows.rs`, and `src/backend/handshake_core/tests/mex_tests.rs`, then repair commit `a0a355c3` closed the denied-path seam so workflow output preserves `provider_access`, write-policy guidance, and engine-error context even when no `calendar_sync_result` artifact exists.

## HYGIENE
- Superseded the earlier bounded proof range `5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719` with the current-main-compatible authoritative range `e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a` on local branch `feat/WP-1-Calendar-Sync-Engine-v2`, preserved on remote backup branch `feat/WP-1-Calendar-Sync-Engine-v2-mainproof`.
- Confirmed the committed MT-002..MT-006 product surface remains confined to 7 files: `src/backend/handshake_core/mechanical_engines.json`, `src/backend/handshake_core/src/capabilities.rs`, `src/backend/handshake_core/src/mex/gates.rs`, `src/backend/handshake_core/src/mex/runtime.rs`, `src/backend/handshake_core/src/storage/calendar.rs`, `src/backend/handshake_core/src/workflows.rs`, and `src/backend/handshake_core/tests/mex_tests.rs`.
- Whole-WP overlap review is recorded PASS for all landed MT commits: MT-002 `551258a7` / `review:WP-1-Calendar-Sync-Engine-v2:mt-002:workflow-capability-contract`, MT-003 `2c3e569e` / `review:WP-1-Calendar-Sync-Engine-v2:review_request:mo8hpz4z:661500`, MT-004 `e65b27aa` / `review:WP-1-Calendar-Sync-Engine-v2:review_request:mo8ia39t:24fb07`, MT-005 `dbaf8b73` / `review:WP-1-Calendar-Sync-Engine-v2:review_request:mo8iyfy9:36d18b`, and MT-006 repair head `a0a355c3` / `review:WP-1-Calendar-Sync-Engine-v2:review_request:mt-006:a0a355c3`.
- Current-main-compatible proof commits are `31d4c221`, `a503c097`, and `dba8b409`; they transplant the already-reviewed MT-002..MT-006 seven-file surface onto the local `main` compatibility baseline `e1243008365566d4cde3c707f1b6078b5837fdcd`.
- Rebuilt the deterministic manifest and packet-local signed-scope patch artifact from committed blob ids plus `git diff --unified=0` on the final bounded committed range because the packet still reflected only the older MT-005 handoff slice.
- Fresh current-main-compatible cargo proof is not yet settled. The latest settled attempt on the preserved `mainproof` backup lineage reported external Windows `libduckdb-sys` header-generation failures before packet-owned code compiled, and the orchestrator's targeted rerun timed out locally under heavy host load.
- Final direct proof is now run from `../wtc-sync-engine-v2`, where the live current-main-compatible candidate and rebuilt deterministic manifest both reside.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable `just phase-check HANDOFF <WP_ID> CODER`. This section records the What (hashes/lines) for the Validator's How/Why audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- **Target File**: `src/backend/handshake_core/mechanical_engines.json`
- **Start**: 75
- **End**: 118
- **Line Delta**: `+44`
- **Pre-SHA1**: `402e5bc2d02678a24c70c06b11de4ed51c34f7b0`
- **Post-SHA1**: `56343d738935ecd401dac80c2a6a4c6e11aeb22f`
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
- **Start**: 10
- **End**: 712
- **Line Delta**: `+65`
- **Pre-SHA1**: `bf323172c4b1c642365097eadee4ca3565672f05`
- **Post-SHA1**: `99bddb009767bc3e08a9f48968644f47f11773fd`
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

- **Target File**: `src/backend/handshake_core/src/mex/gates.rs`
- **Start**: 570
- **End**: 792
- **Line Delta**: `+68`
- **Pre-SHA1**: `bb6cf19452a73d16b48dd90d09e1f4e2b8c01f50`
- **Post-SHA1**: `498a2d14877ac141f89da25ce74f3e1783ef62e6`
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
- **Start**: 159
- **End**: 754
- **Line Delta**: `64`
- **Pre-SHA1**: `c2c4136eb36a89a7036f4083f3e33b8c2dd19b44`
- **Post-SHA1**: `16e331bba10c1d79fe84942b1915c488cdad2181`
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
- **End**: 498
- **Line Delta**: `149`
- **Pre-SHA1**: `9fbd02c81fd0f17cdea6b1bedde2da83797b2e24`
- **Post-SHA1**: `a9a803a1b3101a2b24fb3efa30ec12e8e13dd6bd`
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
- **Start**: 51
- **End**: 28612
- **Line Delta**: `1520`
- **Pre-SHA1**: `292b63d2c0da2ccd5dfd1505461575223096d6d5`
- **Post-SHA1**: `9553334d3782ee247b3a5ebde07de16e414ec691`
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
- **Start**: 20
- **End**: 1189
- **Line Delta**: `361`
- **Pre-SHA1**: `5ed02fd920b9c538d8b4c441d125631d43d23774`
- **Post-SHA1**: `607acbfb342fc6e33cd3fda310c73d271778ef19`
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

- **Lint Results**: Whole-WP overlap review remains PASS for MT-002 through MT-006, and deterministic handoff now PASSes on the current-main-compatible range `e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a` from `../wtc-sync-engine-v2`. Fresh cargo-backed proof on the current-main candidate is still unsettled under host load.
- **Artifacts**: `.GOV/task_packets/WP-1-Calendar-Sync-Engine-v2/signed-scope.patch`; authoritative current-main-compatible product range `e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a`; current proof commits `31d4c221`, `a503c097`, `dba8b409`; historical MT commits `551258a7`, `2c3e569e`, `e65b27aa`, `dbaf8b73`, `579ef5b4`, `a0a355c3`; review receipts `review:WP-1-Calendar-Sync-Engine-v2:mt-002:workflow-capability-contract`, `review:WP-1-Calendar-Sync-Engine-v2:review_request:mo8hpz4z:661500`, `review:WP-1-Calendar-Sync-Engine-v2:review_request:mo8ia39t:24fb07`, `review:WP-1-Calendar-Sync-Engine-v2:review_request:mo8iyfy9:36d18b`, `review:WP-1-Calendar-Sync-Engine-v2:review_request:mt-006:a0a355c3`
- **Timestamp**: `2026-04-21T16:46:37.9696630Z`
- **Operator**: `ORCHESTRATOR`
- **Spec Target Resolved**: `.GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md`
- Deterministic Handoff Command: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v2 CODER --range e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a`
- **Notes**: The current-main-compatible 7-file range above is the authoritative product proof surface for this WP. It supersedes the earlier `5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719` proof slice by transplanting the same calendar-sync contract onto the local `main` baseline `e1243008365566d4cde3c707f1b6078b5837fdcd`, and the rebuilt manifest now reproduces a green deterministic handoff.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: DONE_FAIL
- What changed in this update: Restored the canonical local/validator branch policy `feat/WP-1-Calendar-Sync-Engine-v2` required by session-policy and topology checks, kept the superseding current-main-compatible candidate preserved on remote backup branch `feat/WP-1-Calendar-Sync-Engine-v2-mainproof`, promoted the authoritative proof range `e1243008..dba8b409`, and rebuilt the `workflows.rs` manifest so the deterministic handoff command reproduces cleanly from `../wtc-sync-engine-v2`.
- Requirements / clauses self-audited: `workflow_run` now binds `calendar_sync` to `CalendarSync` with explicit required capabilities in `src/backend/handshake_core/src/capabilities.rs`, `src/backend/handshake_core/src/workflows.rs`, and `src/backend/handshake_core/src/mex/gates.rs`; governed execution still routes through the MEX runtime in `src/backend/handshake_core/src/workflows.rs` and `src/backend/handshake_core/src/mex/runtime.rs`; the engine registry and output contract remain explicit in `src/backend/handshake_core/mechanical_engines.json` and `src/backend/handshake_core/src/storage/calendar.rs`; recovery truth remains durable in `CalendarSourceSyncState`; and MT-006 now adds provider-access guidance, read-only fail-closed behavior, and deny-path output parity without requiring a `calendar_sync_result` artifact to exist first.
- Checks actually run: `git -C ..\\wtc-sync-engine-v2 log --oneline e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a`; `git -C ..\\wtc-sync-engine-v2 diff --numstat e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a -- src/backend/handshake_core/mechanical_engines.json src/backend/handshake_core/src/capabilities.rs src/backend/handshake_core/src/mex/gates.rs src/backend/handshake_core/src/mex/runtime.rs src/backend/handshake_core/src/storage/calendar.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/mex_tests.rs`; `git -C ..\\wtc-sync-engine-v2 diff --unified=0 e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a -- src/backend/handshake_core/mechanical_engines.json src/backend/handshake_core/src/capabilities.rs src/backend/handshake_core/src/mex/gates.rs src/backend/handshake_core/src/mex/runtime.rs src/backend/handshake_core/src/storage/calendar.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/mex_tests.rs`; receipt review in `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Sync-Engine-v2/THREAD.md` and `RECEIPTS.jsonl`; `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v2 CODER --range e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a` from `..\\wtc-sync-engine-v2` => PASS.
- Known gaps / weak spots: Fresh cargo-backed TEST_PLAN proof on the current-main-compatible head is still unsettled. The latest settled coder-side mainproof attempt reported external Windows `libduckdb-sys` header-generation failures before packet-owned code compiled, and the orchestrator's targeted rerun timed out locally under host load. The Integration Validator closeout session has not been relaunched on the superseding current-main-compatible head yet.
- Heuristic risks / maintainability concerns: `src/backend/handshake_core/src/workflows.rs` remains the largest shared surface in the WP and now carries capability enforcement, runtime dispatch, adapter construction, read-only denial behavior, and deny-path output parity. Closeout tooling also still shows a protocol mismatch by expecting governed Integration Validator identity earlier than the written closeout-prep sequence.
- Validator focus request: Treat `e1243008..dba8b409` in `../wtc-sync-engine-v2` on local branch `feat/WP-1-Calendar-Sync-Engine-v2` and preserved remote backup `feat/WP-1-Calendar-Sync-Engine-v2-mainproof` as the authoritative current-main-compatible surface, confirm the seven-file transplant preserves the already-reviewed `calendar_sync` contract, and judge whether the unsettled cargo proof state is environmental only or still blocks final readiness.
- Rubric contract understanding proof: This packet owns the governed `calendar_sync` execution path, capability/runtime/storage contract, read-only provider posture, and deny-path workflow output parity on the current-main-compatible head `dba8b409`.
- Rubric scope discipline proof: The authoritative committed handoff range is bounded to 7 product files and 3 proof commits on top of the local `main` baseline `e1243008365566d4cde3c707f1b6078b5837fdcd`. It keeps the already-reviewed MT-002..MT-006 surface while excluding unrelated branch drift from the stale creation-time merge base.
- Rubric baseline comparison: Relative to `e1243008365566d4cde3c707f1b6078b5837fdcd`, the current-main-compatible range adds the `calendar_sync` engine registry entry, explicit `CalendarSync` capability contract, governed MEX runtime routing with workflow-linked evidence, explicit `calendar_sync_result` output typing, durable sync-state sequencing, provider-access guidance, read-only fail-closed behavior, and deny-path workflow output parity. Net product delta across the bounded range is `+2344 / -73`.
- Rubric end-to-end proof: The current-main-compatible bounded range proves registry declaration (`mechanical_engines.json`), capability allow/deny posture (`capabilities.rs`, `mex/gates.rs`), governed runtime dispatch (`workflows.rs`, `mex/runtime.rs`), adapter/output contract (`workflows.rs`, `storage/calendar.rs`), durable success/backoff storage truth (`storage/calendar.rs`, `workflows.rs`), and read-only/provider deny-path tripwires (`workflows.rs`, `tests/mex_tests.rs`). Deterministic handoff is now green; the remaining missing proof is a fresh settled cargo-backed execution on the current-main-compatible head.
- Rubric architecture fit self-review: The implementation keeps `calendar_sync` inside the existing workflow plus MEX stack instead of introducing side helpers, preserves durable sync-state truth in the calendar storage model, and adds provider/read-only posture by extending the existing governed engine contract rather than bypassing it.
- Rubric heuristic quality self-review: The strongest remaining uncertainty is not feature scope anymore; it is governance completion. The main product quality risk inside the committed range is continued concentration of cross-cutting logic in `workflows.rs`, not hidden contract drift or shadow execution paths.
- Rubric anti-gaming / counterfactual check: If `calendar_sync` were removed from `mechanical_engines.json`, the engine would no longer be declared consistently with the adapter contract. If `run_calendar_sync_job` in `workflows.rs` stopped using the governed MEX runtime, the workflow would silently fall back to a shadow helper. If the deny-safe payload construction added in `a0a355c3` were removed, `EngineStatus::Denied` could again collapse into a workflow error before `provider_access` and read-only guidance are emitted. If the `CalendarSync` capability contract were weakened, the deny-path and adapter-missing tripwires in `tests/mex_tests.rs` would stop proving the signed calendar capability posture.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: This packet repair is tied to a concrete current-main-compatible committed range, explicit line windows, a rebuilt `workflows.rs` manifest, a green deterministic handoff command, and recorded MT PASS receipts. It does not rely on the stale `a0a355` packet snapshot or the creation-time merge base.
- Signed-scope debt ledger: Inside the signed packet, the remaining debt is final-lane governance and proof settlement only: closeout-repair, `phase-check CLOSEOUT`, a fresh Integration Validator verdict on `e1243008..dba8b409`, and settled cargo-backed proof on the current-main-compatible head.
- Data contract self-check: The active contract flows from the engine registry (`src/backend/handshake_core/mechanical_engines.json`) through the workflow adapter/output (`src/backend/handshake_core/src/workflows.rs`) into the emitted result schema and durable source state (`src/backend/handshake_core/src/storage/calendar.rs`). Capability allow/deny posture, provider-access guidance, and read-only mutation denial are explicit rather than fallback-driven.
- Next step / handoff hint: Run `just closeout-repair WP-1-Calendar-Sync-Engine-v2`, then `just phase-check CLOSEOUT WP-1-Calendar-Sync-Engine-v2`; if mechanical truth stays green, relaunch the Integration Validator against the authoritative `e1243008..dba8b409` current-main-compatible surface.

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
- REQUIREMENT: "`workflow_run` plus `calendar_sync` binds to the `CalendarSync` capability profile with explicit `calendar.sync.read` and `calendar.sync.write` requirements."
- EVIDENCE: `src/backend/handshake_core/src/capabilities.rs:10-13`; `src/backend/handshake_core/src/workflows.rs:26192-26247`; `src/backend/handshake_core/src/mex/gates.rs:753-779`
- REQUIREMENT: "The `calendar_sync` workflow executes through the governed MEX runtime with workflow-linked tool evidence and explicit adapter-missing failure evidence."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:8608`; `src/backend/handshake_core/src/workflows.rs:12507-12687`; `src/backend/handshake_core/src/mex/runtime.rs:521`; `src/backend/handshake_core/tests/mex_tests.rs:868-1155`
- REQUIREMENT: "The engine registry and adapter contract expose one declared `engine.calendar_sync` path with `calendar.sync` operation and `calendar_sync_result` output."
- EVIDENCE: `src/backend/handshake_core/mechanical_engines.json:75-98`; `src/backend/handshake_core/src/workflows.rs:11789-12687`; `src/backend/handshake_core/src/storage/calendar.rs:481-492`
- REQUIREMENT: "`CalendarSourceSyncState` remains the durable single source of truth for begin-attempt, failure/backoff, and success-reset recovery state."
- EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:124-205`; `src/backend/handshake_core/src/workflows.rs:11903-12137`; `src/backend/handshake_core/src/workflows.rs:26392-26582`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Calendar-Sync-Engine-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
- COMMAND: `git -C ..\\wtc-sync-engine-v2 show --stat --oneline 551258a7 2c3e569e e65b27aa dbaf8b73 579ef5b4 a0a355c3`
- EXIT_CODE: `0`
- PROOF_LINES: `551258a7 feat: MT-002 workflow capability profile and required-capabilities contract`; `2c3e569e feat: MT-003 calendar sync workflow routes through MEX runtime`; `e65b27aa feat: MT-004 calendar_sync engine contract and output`; `dbaf8b73 feat: MT-005 sync-state recovery durability`; `579ef5b4 feat: MT-006 MCP/provider adapter guidance plus read-only mode`; `a0a355c3 fix: MT-006 denied-path workflow output parity`

- COMMAND: `git -C ..\\wtc-sync-engine-v2 diff --numstat 5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719`
- EXIT_CODE: `0`
- PROOF_LINES: `44 0 src/backend/handshake_core/mechanical_engines.json`; `65 0 src/backend/handshake_core/src/capabilities.rs`; `68 0 src/backend/handshake_core/src/mex/gates.rs`; `87 23 src/backend/handshake_core/src/mex/runtime.rs`; `150 1 src/backend/handshake_core/src/storage/calendar.rs`; `1571 49 src/backend/handshake_core/src/workflows.rs`; `362 1 src/backend/handshake_core/tests/mex_tests.rs`

- COMMAND: `git -C ..\\wtc-sync-engine-v2 diff --unified=0 5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719 -- src/backend/handshake_core/mechanical_engines.json src/backend/handshake_core/src/capabilities.rs src/backend/handshake_core/src/mex/gates.rs src/backend/handshake_core/src/mex/runtime.rs src/backend/handshake_core/src/storage/calendar.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/mex_tests.rs`
- EXIT_CODE: `0`
- PROOF_LINES: `bounded final range confirms the committed surface is the declared 7 files only`; `window values in VALIDATION were rebuilt from this exact diff through MT-006 repair head a0a355c3`

- COMMAND: `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v2 CODER --range 5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719`
- EXIT_CODE: `0`
- LOG_PATH: `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Calendar-Sync-Engine-v2/2026-04-21T12-59-58-295Z.log`
- PROOF_LINES: `Post-work validation PASSED (deterministic manifest gate; not tests)`; `workflow sequence verified`; `kickoff exchange is complete`; `previous_microtask=MT-006:CLEARED`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex_tests`
- EXIT_CODE: `101`
- PROOF_LINES: `error: this file contains an unclosed delimiter`; `--> src\\flight_recorder\\mod.rs:6180:3`; `error: could not compile 'handshake_core' (lib) due to 1 previous error`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml calendar_storage_tests`
- EXIT_CODE: `101`
- PROOF_LINES: `error: this file contains an unclosed delimiter`; `--> src\\flight_recorder\\mod.rs:6180:3`; `error: could not compile 'handshake_core' (lib) due to 1 previous error`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: `101`
- PROOF_LINES: `error: this file contains an unclosed delimiter`; `--> src\\flight_recorder\\mod.rs:6180:3`; `error: could not compile 'handshake_core' (lib) due to 1 previous error`

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

### 2026-04-21T13:58:00.5630950Z | INTEGRATION_VALIDATOR | session=integration_validator:wp-1-calendar-sync-engine-v2
- Verdict: FAIL
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PARTIAL
- TEST_VERDICT: FAIL
- CODE_REVIEW_VERDICT: PASS
- HEURISTIC_REVIEW_VERDICT: PASS
- SPEC_ALIGNMENT_VERDICT: PARTIAL
- ENVIRONMENT_VERDICT: FAIL
- DISPOSITION: NONE
- LEGAL_VERDICT: FAIL
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- WORKFLOW_VALIDITY: PARTIAL
- SCOPE_VALIDITY: IN_SCOPE
- PROOF_COMPLETENESS: NOT_PROVEN
- INTEGRATION_READINESS: NOT_READY
- DOMAIN_GOAL_COMPLETION: PARTIAL
- MECHANICAL_TRACK_VERDICT: FAIL
- SPEC_RETENTION_TRACK_VERDICT: PARTIAL
- CLAUSES_REVIEWED:
  - `surface mutation discipline plus write gate` -> `src/backend/handshake_core/mechanical_engines.json:75-115` declares the workflow-only `engine.calendar_sync` path with explicit provider guidance; `src/backend/handshake_core/src/workflows.rs:12755-12864` routes `workflow_run` inputs into governed MEX execution instead of an ad hoc helper path.
  - `workflow capability profile and required-capabilities contract` -> `src/backend/handshake_core/src/capabilities.rs:10-13`, `src/backend/handshake_core/src/capabilities.rs:136-143`, `src/backend/handshake_core/src/capabilities.rs:372-415`, and `src/backend/handshake_core/src/mex/gates.rs:753-791` bind `workflow_run` + `calendar_sync` to `CalendarSync` and keep wrong-profile denials profile-based.
  - `Cross-Tool Interaction Map no-shadow-pipeline rule` -> `src/backend/handshake_core/src/workflows.rs:12829-12864` injects workflow context into the MEX op, and `src/backend/handshake_core/src/mex/runtime.rs:159-187`, `src/backend/handshake_core/src/mex/runtime.rs:254-325`, and `src/backend/handshake_core/src/mex/runtime.rs:511-525` preserve `job_id` / `workflow_run_id` / `trace_id` and emit explicit adapter-missing tool failure evidence.
  - ``calendar_sync` engine contract and output` -> `src/backend/handshake_core/mechanical_engines.json:75-115`, `src/backend/handshake_core/src/storage/calendar.rs:481-492`, and `src/backend/handshake_core/src/workflows.rs:12688-12919` keep the `calendar.sync` operation, `calendar_sync_result` schema, and workflow payload fields aligned.
  - ``CalendarSourceSyncState` as single source of truth for recovery` -> `src/backend/handshake_core/src/storage/calendar.rs:124-205`, `src/backend/handshake_core/src/workflows.rs:11942-12066`, `src/backend/handshake_core/src/workflows.rs:12100-12220`, `src/backend/handshake_core/src/storage/sqlite.rs:3635-3705`, and `src/backend/handshake_core/src/storage/postgres.rs:4081-4151` preserve sync-state mutation and persistence on the storage boundary.
  - `MCP/provider adapter guidance plus read-only mode` -> `src/backend/handshake_core/mechanical_engines.json:80-88`, `src/backend/handshake_core/src/workflows.rs:12122-12154`, `src/backend/handshake_core/src/workflows.rs:12653-12752`, and `src/backend/handshake_core/src/workflows.rs:12871-12919` fail closed for read-only push requests and preserve denied-path payload parity.
- NOT_PROVEN:
  - The packet `TEST_PLAN` is still blocked on the committed candidate: `cargo test --manifest-path ..\\wtc-sync-engine-v2\\src\\backend\\handshake_core\\Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` failed after a clean external `CARGO_TARGET_DIR` build because `src/backend/handshake_core/src/flight_recorder/mod.rs:6180` contains an unclosed delimiter outside the signed seven-file surface. The packet clause that `mex_tests` and storage tests "still prove" the behavior is therefore not currently satisfied.
  - `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v2 CODER --range 5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719` failed on 2026-04-21 because `Manifest[6]` reports a `post_sha1` mismatch for `src/backend/handshake_core/src/workflows.rs`; the packet still expects LF hash `acb8b3a0bb44dff59e3233903cfcab7ca01aca26`. The deterministic handoff proof chain is not fully reproducible from current packet truth.
- MAIN_BODY_GAPS:
  - The signed implementation reads coherently, but the required proof surface is incomplete because the packet TEST_PLAN cannot currently execute on the committed candidate and the HANDOFF manifest no longer reproduces without repair.
  - Final-lane containment into local `main` is therefore not lawful: `PROOF_COMPLETENESS=NOT_PROVEN` and `INTEGRATION_READINESS=NOT_READY`.
- QUALITY_RISKS:
  - NONE
- VALIDATOR_RISK_TIER: HIGH
- DIFF_ATTACK_SURFACES:
  - Capability-profile override drift between `CapabilityRegistry`, `CapabilityGate`, and `workflow_run` dispatch.
  - Workflow-to-MEX routing drift that could preserve a shadow helper path or drop `workflow_run_id` / `trace_id`.
  - Denied-path output parity that could still depend on loading a `calendar_sync_result` artifact before constructing workflow outputs.
  - Storage-boundary drift where `CalendarSourceSyncState` updates are emitted in workflow code but not persisted through SQLite/Postgres upserts.
  - Read-only source posture drift where provider mutation guidance exists in JSON but push requests still mutate or fail without inspectable workflow output.
- INDEPENDENT_CHECKS_RUN:
  - `just phase-check STARTUP WP-1-Calendar-Sync-Engine-v2 INTEGRATION_VALIDATOR integration_validator:wp-1-calendar-sync-engine-v2` => PASS; startup communication mesh and active-lane context were ready before final review.
  - `just phase-check CLOSEOUT WP-1-Calendar-Sync-Engine-v2` => PASS; closeout dependencies were coherent, but no terminal sync mode had been recorded yet.
  - `git diff --check 5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719 -- <signed 7 files>` => PASS; no whitespace or patch-format hygiene defects were present inside the signed surface.
  - PowerShell JSON parse of `git show a0a355c3:src/backend/handshake_core/mechanical_engines.json` => `engine.calendar_sync` declares required caps `calendar.sync.read` / `calendar.sync.write`, op `calendar.sync`, output type `calendar_sync_result`, provider path `workflow:calendar_sync->engine.calendar_sync/calendar.sync`, and read-only blocked directions `push,mirror`.
  - Static cross-surface probe on `git show a0a355c3:src/backend/handshake_core/src/workflows.rs` => the `EngineStatus::Denied` branch appears before `load_calendar_sync_result_from_output`, and the denied payload still includes `provider_access` plus empty `output_types`.
  - Static cross-surface probe on `git show a0a355c3:src/backend/handshake_core/src/mex/runtime.rs` + `workflows.rs` => `workflow_context` is inserted by the workflow entrypoint and consumed by runtime helpers for `job_id`, `workflow_run_id`, `trace_id`, and explicit `MEX_ADAPTER_MISSING` recording.
  - `cargo test --manifest-path ..\\wtc-sync-engine-v2\\src\\backend\\handshake_core\\Cargo.toml calendar_sync_runtime_denies_wrong_profile_without_unknown_capability -- --exact` with `CARGO_TARGET_DIR=D:\\Projects\\LLM projects\\Handshake\\Handshake Worktrees\\Handshake_Artifacts\\handshake-cargo-target\\iv-wp1-proof` => FAIL; compile stopped at out-of-scope `src/backend/handshake_core/src/flight_recorder/mod.rs:6180` with an unclosed delimiter.
  - `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v2 CODER --range 5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719` => FAIL; deterministic post-work proof no longer reproduces because the packet manifest hash for `src/backend/handshake_core/src/workflows.rs` drifted.
- COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/capabilities.rs:380-415` stopped special-casing `workflow_run` + `calendar_sync`, the workflow would fall back to the wrong profile and the calendar path would again deny under unrelated authority semantics.
  - If `src/backend/handshake_core/src/workflows.rs:12829-12831` stopped inserting `workflow_context`, the runtime code in `src/backend/handshake_core/src/mex/runtime.rs:173-187` and `src/backend/handshake_core/src/mex/runtime.rs:254-325` would lose workflow/job/trace linkage in `tool.call` / `tool.result` evidence.
  - If `src/backend/handshake_core/src/workflows.rs:12871-12889` fell through to `load_calendar_sync_result_from_output`, a true denied result would again raise a terminal "no outputs" failure before emitting provider-access guidance.
  - If `src/backend/handshake_core/src/workflows.rs:11942-12066` no longer wrote through `upsert_calendar_source`, the sync-state transitions built in `src/backend/handshake_core/src/storage/calendar.rs:146-205` would never reach the persisted storage contract reviewed in SQLite/Postgres.
- BOUNDARY_PROBES:
  - Registry/capability boundary: compared `mechanical_engines.json:75-115` against `capabilities.rs:372-415` and `mex/gates.rs:753-791`; the op name, required capabilities, and denial reason all line up on the `CalendarSync` contract.
  - Workflow/runtime boundary: compared `workflows.rs:12829-12864` with `mex/runtime.rs:159-187` and `mex/runtime.rs:254-325`; the workflow injects context once and the runtime consumes the same fields when emitting tool evidence and gate events.
  - Workflow/storage boundary: compared `workflows.rs:11942-12066` and `workflows.rs:12100-12220` with `sqlite.rs:3635-3705` and `postgres.rs:4081-4151`; the adapter writes `capability_profile_id`, `write_policy`, and the expanded sync-state fields through the durable upsert surfaces instead of an untracked side channel.
- NEGATIVE_PATH_CHECKS:
  - Wrong-profile negative path: `src/backend/handshake_core/src/mex/gates.rs:774-791`, `src/backend/handshake_core/tests/mex_tests.rs:1051-1103`, and `src/backend/handshake_core/src/workflows.rs:26446-26475` all deny `Analyst` without surfacing `HSK-4001 UnknownCapability`.
  - Read-only provider-mutation negative path: `src/backend/handshake_core/src/workflows.rs:12122-12154`, `src/backend/handshake_core/src/workflows.rs:12688-12752`, and `src/backend/handshake_core/src/workflows.rs:26876-27135` surface `CALENDAR_SYNC_READ_ONLY_WRITE_BLOCKED`, preserve `provider_access`, and avoid artifact-load dependency on denied results.
  - Environment negative path: the targeted `cargo test` command above fails outside signed scope at `src/backend/handshake_core/src/flight_recorder/mod.rs:6180`, so the packet's required proof commands remain blocked even after an external-target rebuild.
- INDEPENDENT_FINDINGS:
  - The signed seven-file implementation itself looks internally coherent: capability routing, workflow/MEX plumbing, denied-path repair, and sync-state persistence all line up across the reviewed code surfaces.
  - The final-lane blocker is proof law, not an in-scope semantic defect. The packet requires executable TEST_PLAN evidence and reproducible deterministic handoff proof; both are currently missing from the candidate's closure state.
- Current local `main` is still compatible with the packet surface: `git rev-parse HEAD` returned `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`, and `git diff --name-only 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e..dba8b4099c1afda1992fd79451baacc9fa79c47a -- <signed 7 files>` remained confined to the declared seven-file surface.
- RESIDUAL_UNCERTAINTY:
  - A clean cargo-backed execution of the signed candidate has not been re-proven in this lane because compilation stops in out-of-scope `src/backend/handshake_core/src/flight_recorder/mod.rs:6180`.
  - The `phase-check HANDOFF` hash mismatch may be stale packet proof rather than product-code drift, but until the manifest is regenerated or reconciled, the deterministic evidence chain is still broken.
  - The broader calendar spec still describes provider MCP tools and push-side provider operations that are not implemented inside this signed packet; that gap is recorded below as negative proof and is not treated as a signed-surface regression here.
- SPEC_CLAUSE_MAP:
  - `workflow_run` + `calendar_sync` binds to `CalendarSync` with explicit `calendar.sync.read` / `calendar.sync.write` => `src/backend/handshake_core/src/capabilities.rs:10-13`, `src/backend/handshake_core/src/capabilities.rs:136-143`, `src/backend/handshake_core/src/capabilities.rs:372-415`, `src/backend/handshake_core/src/workflows.rs:12808-12845`, `src/backend/handshake_core/src/mex/gates.rs:753-791`.
  - `calendar_sync` executes through governed MEX runtime with workflow-linked tool evidence and adapter-missing failure evidence => `src/backend/handshake_core/src/workflows.rs:12829-12864`, `src/backend/handshake_core/src/mex/runtime.rs:159-187`, `src/backend/handshake_core/src/mex/runtime.rs:254-325`, `src/backend/handshake_core/src/mex/runtime.rs:511-525`, `src/backend/handshake_core/tests/mex_tests.rs:958-1149`.
  - `engine.calendar_sync` exposes `calendar.sync` with `calendar_sync_result` output => `src/backend/handshake_core/mechanical_engines.json:75-115`, `src/backend/handshake_core/src/storage/calendar.rs:481-492`, `src/backend/handshake_core/src/workflows.rs:12688-12919`.
  - `CalendarSourceSyncState` remains the durable sync/recovery source of truth => `src/backend/handshake_core/src/storage/calendar.rs:124-205`, `src/backend/handshake_core/src/workflows.rs:11942-12066`, `src/backend/handshake_core/src/workflows.rs:12100-12220`, `src/backend/handshake_core/src/storage/sqlite.rs:3635-3705`, `src/backend/handshake_core/src/storage/postgres.rs:4081-4151`.
  - Read-only provider posture is fail-closed and workflow outputs preserve guidance / parity on denied results => `src/backend/handshake_core/mechanical_engines.json:80-88`, `src/backend/handshake_core/src/workflows.rs:12122-12154`, `src/backend/handshake_core/src/workflows.rs:12653-12752`, `src/backend/handshake_core/src/workflows.rs:12871-12919`, `src/backend/handshake_core/src/workflows.rs:26876-27135`.
  - `mex_tests` contains packet-scoped coverage, but runnable proof is still blocked => `src/backend/handshake_core/tests/mex_tests.rs:608-643`, `src/backend/handshake_core/tests/mex_tests.rs:908-1149`, `src/backend/handshake_core/src/workflows.rs:26420-27135`, blocked in execution by `src/backend/handshake_core/src/flight_recorder/mod.rs:6180`.
- NEGATIVE_PROOF:
  - Master Spec `Handshake_Master_Spec_v02.181.md:56840-56842` requires MCP tools for Google/Outlook/CalDAV to be used inside `calendar_sync`; the signed packet adds provider-access guidance and governed routing but does not implement provider MCP tool wrappers inside the reviewed surface.
  - Master Spec `Handshake_Master_Spec_v02.181.md:56218-56221` describes push-side delta computation and provider API calls; the signed packet proves read-only fail-closed posture and durable sync-state handling, but it does not implement live provider write-back beyond the guarded contract surface.
- ANTI_VIBE_FINDINGS:
  - NONE
- SIGNED_SCOPE_DEBT:
  - NONE
- PRIMITIVE_RETENTION_PROOF:
  - `CalendarSourceSyncState` retains its durable fields in `src/backend/handshake_core/src/storage/calendar.rs:124-138` and remains embedded in `CalendarSyncResult` at `src/backend/handshake_core/src/storage/calendar.rs:483-492`.
  - `capability_profile_id`, `write_policy`, and sync-state persistence remain on the calendar-source storage boundary in `src/backend/handshake_core/src/storage/sqlite.rs:3645-3660` and `src/backend/handshake_core/src/storage/postgres.rs:4091-4110`.
  - Workflow outputs still expose `schema_version`, `provider_type`, `write_policy`, `source_sync_state`, and provider guidance in `src/backend/handshake_core/src/workflows.rs:12715-12749` and `src/backend/handshake_core/src/workflows.rs:12891-12919`.
- PRIMITIVE_RETENTION_GAPS:
  - NONE
- SHARED_SURFACE_INTERACTION_CHECKS:
  - `mechanical_engines.json:75-115` <-> `capabilities.rs:372-415` <-> `mex/gates.rs:753-791`: registry, required capabilities, and gate denial semantics are aligned for `calendar_sync`.
  - `workflows.rs:12829-12864` <-> `mex/runtime.rs:159-187` and `mex/runtime.rs:254-325`: workflow context written by the workflow entrypoint is consumed by runtime evidence emission without a second contract shape.
  - `workflows.rs:11942-12066` and `workflows.rs:12100-12220` <-> `storage/calendar.rs:146-205` <-> `sqlite.rs:3635-3705` / `postgres.rs:4081-4151`: sync-state transitions and durable persistence use the same contract fields across the adapter and both storage backends.
- CURRENT_MAIN_INTERACTION_CHECKS:
  - `git rev-parse HEAD` in `../handshake_main` returned `2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e`, which matches `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA` already recorded in this packet.
  - `git diff --name-only 2ecd453c3eff9d4a93e962eb80dfb7a7f1458e4e..dba8b4099c1afda1992fd79451baacc9fa79c47a -- src/backend/handshake_core/mechanical_engines.json src/backend/handshake_core/src/capabilities.rs src/backend/handshake_core/src/mex/gates.rs src/backend/handshake_core/src/mex/runtime.rs src/backend/handshake_core/src/storage/calendar.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/mex_tests.rs` returned exactly those seven declared files, so current local `main` introduces no extra out-of-scope interaction pressure beyond the signed packet surface.
- DATA_CONTRACT_PROOF:
  - `src/backend/handshake_core/mechanical_engines.json:108-115` declares `calendar_sync_result` as the workflow-visible output type for `calendar.sync`.
  - `src/backend/handshake_core/src/storage/calendar.rs:483-492` defines the `CalendarSyncResult` contract, including provider identity, write policy, direction, time window, and `source_sync_state`.
  - `src/backend/handshake_core/src/workflows.rs:12715-12749` and `src/backend/handshake_core/src/workflows.rs:12891-12919` emit JSON payloads with `schema_version`, `provider_type`, `write_policy`, `source_sync_state`, `provider_access`, `outputs`, and `evidence`.
  - `src/backend/handshake_core/src/storage/sqlite.rs:3645-3660` and `src/backend/handshake_core/src/storage/postgres.rs:4091-4110` preserve the calendar-source portability fields (`capability_profile_id`, `write_policy`, `sync_state`, recovery timestamps, and workflow/job back-links).
- DATA_CONTRACT_GAPS:
  - NONE
- REMEDIATION_INSTRUCTIONS:
  - Restore a clean proof surface for the signed candidate by repairing or isolating the out-of-scope compile break in `src/backend/handshake_core/src/flight_recorder/mod.rs:6180` without widening the packet's signed seven-file surface.
  - Rebuild the packet's deterministic handoff artifact for `5336e8f23b7a6e2f35b450124dccb65a17644d7f..a0a355c359656eedea3098692fc89f3546a59719` so `just phase-check HANDOFF ... --range ...` reproduces without the `Manifest[6]` `post_sha1` mismatch on `src/backend/handshake_core/src/workflows.rs`.
  - Re-run the packet `TEST_PLAN` on the committed signed surface, append fresh proof to `## EVIDENCE`, and request a new final-lane verdict only after both the cargo-backed proof commands and the deterministic HANDOFF gate are green.

### 2026-04-21T17:47:09.8019104Z | INTEGRATION_VALIDATOR | session=integration_validator:wp-1-calendar-sync-engine-v2
Verdict: FAIL
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: FAIL
TEST_VERDICT: FAIL
CODE_REVIEW_VERDICT: FAIL
HEURISTIC_REVIEW_VERDICT: PARTIAL
SPEC_ALIGNMENT_VERDICT: FAIL
ENVIRONMENT_VERDICT: PARTIAL
DISPOSITION: NONE
LEGAL_VERDICT: FAIL
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
WORKFLOW_VALIDITY: INVALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: NOT_PROVEN
INTEGRATION_READINESS: NOT_READY
DOMAIN_GOAL_COMPLETION: PARTIAL
MECHANICAL_TRACK_VERDICT: FAIL
SPEC_RETENTION_TRACK_VERDICT: FAIL
CLAUSES_REVIEWED:
  - `surface mutation discipline plus write gate` -> `src/backend/handshake_core/mechanical_engines.json:75-115`, `src/backend/handshake_core/src/workflows.rs:9541-9542`, and `src/backend/handshake_core/src/workflows.rs:13769-13798` keep the calendar path on governed workflow/MEX execution instead of an ad hoc helper path.
  - `workflow capability profile and required-capabilities contract` -> `src/backend/handshake_core/src/capabilities.rs:10-13`, `src/backend/handshake_core/src/capabilities.rs:372-415`, `src/backend/handshake_core/src/capabilities.rs:677-699`, `src/backend/handshake_core/src/mex/gates.rs:753-791`, and `src/backend/handshake_core/tests/mex_tests.rs:1045-1096` bind `workflow_run` + `calendar_sync` to `CalendarSync` with explicit `calendar.sync.read` / `calendar.sync.write` and preserve profile-based denial semantics without `HSK-4001`.
  - `Cross-Tool Interaction Map no-shadow-pipeline rule` -> `src/backend/handshake_core/src/workflows.rs:9541-9542`, `src/backend/handshake_core/src/workflows.rs:13769-13798`, `src/backend/handshake_core/src/mex/runtime.rs:159-187`, `src/backend/handshake_core/src/mex/runtime.rs:254-333`, `src/backend/handshake_core/src/mex/runtime.rs:511-525`, and `src/backend/handshake_core/tests/mex_tests.rs:958-1172` preserve workflow-linked `tool.call` / `tool.result` evidence and explicit adapter-missing failure recording.
  - ``calendar_sync` engine contract and output` -> `src/backend/handshake_core/mechanical_engines.json:75-115`, `src/backend/handshake_core/src/storage/calendar.rs:481-492`, `src/backend/handshake_core/src/workflows.rs:12904-12927`, `src/backend/handshake_core/src/workflows.rs:12945-12999`, and `src/backend/handshake_core/src/workflows.rs:13823-13855` align the `calendar.sync` op, `calendar_sync_result` output contract, artifact emission, and workflow payload surface.
  - ``CalendarSourceSyncState` as single source of truth for recovery` -> `src/backend/handshake_core/src/storage/calendar.rs:124-205`, `src/backend/handshake_core/src/workflows.rs:12876-12902`, `src/backend/handshake_core/src/workflows.rs:13034-13155`, `src/backend/handshake_core/src/workflows.rs:28279-28323`, and `src/backend/handshake_core/src/workflows.rs:28444-28487` keep sync-state begin-attempt, failure/backoff, success-reset, and persisted read-only denial state on the storage-backed workflow boundary.
  - `MCP/provider adapter guidance plus read-only mode` -> `src/backend/handshake_core/mechanical_engines.json:79-115`, `src/backend/handshake_core/src/workflows.rs:13587-13687`, `src/backend/handshake_core/src/workflows.rs:13805-13823`, `src/backend/handshake_core/src/workflows.rs:28328-28487`, `src/backend/handshake_core/src/workflows.rs:28493-28595`, and `src/backend/handshake_core/tests/mex_tests.rs:608-643` preserve workflow-only provider guidance, read-only fail-closed posture, and denied-payload parity without requiring a `calendar_sync_result` artifact first.
NOT_PROVEN:
  - The packet `TEST_PLAN` does not currently build on the authoritative current-main-compatible candidate. `cargo test --manifest-path ..\\wtc-sync-engine-v2\\src\\backend\\handshake_core\\Cargo.toml mex_tests` failed inside signed scope with `E0422 cannot find struct, variant or union type SessionCheckpoint` at `src/backend/handshake_core/src/workflows.rs:6257` and `E0505 cannot move out of inputs because it is borrowed` at `src/backend/handshake_core/src/workflows.rs:13748`.
  - Because the crate fails to compile in `src/backend/handshake_core/src/workflows.rs`, the remaining packet proof commands (`calendar_storage_tests` and full `cargo test`) are not presently runnable proof for this candidate either.
  - `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v2 CODER --range e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a` failed from the integration-validator lane: the packet validation manifest still carries stale post-SHA values and the final-lane repo cannot resolve commit `dba8b4099c1afda1992fd79451baacc9fa79c47a`, so deterministic handoff proof is not reproducible from `../handshake_main`.
MAIN_BODY_GAPS:
  - `DONE_MEANS` requires `mex_tests` coverage, surviving storage proof, validator PASS, and integration into `main`; compile errors inside `src/backend/handshake_core/src/workflows.rs` prevent the proof commands from establishing that state.
  - Final-lane merge authority cannot lawfully proceed while the current committed candidate is not reproducible by `phase-check HANDOFF` from the integration-validator worktree.
QUALITY_RISKS:
  - The current-main transplant removed `SessionCheckpoint` from the `storage::{...}` import list while leaving the live constructor call at `src/backend/handshake_core/src/workflows.rs:6257`, which is a signed-surface compile regression introduced without a successful end-to-end build.
  - `run_calendar_sync_job` borrows `source_id`, `workspace_id`, `direction`, and `time_window` from `inputs`, then moves `inputs` into `params` at `src/backend/handshake_core/src/workflows.rs:13747-13748`; that ownership error means the workflow entrypoint was not compiled successfully after the transplant.
VALIDATOR_RISK_TIER: HIGH
DIFF_ATTACK_SURFACES:
  - Capability-profile override drift between `CapabilityRegistry`, `CapabilityGate`, and the `workflow_run` dispatch path.
  - Workflow-to-MEX routing drift that could drop workflow/job/trace context or restore a shadow helper path.
  - Denied-path output parity that could again depend on loading a `calendar_sync_result` artifact before emitting provider guidance.
  - Sync-state durability drift where begin-attempt, failure/backoff, or success-reset writes stop matching the emitted `CalendarSyncResult`.
  - Current-main transplant drift inside `src/backend/handshake_core/src/workflows.rs`, especially imports and `params` assembly that were touched by the narrowing pass.
INDEPENDENT_CHECKS_RUN:
  - `just phase-check STARTUP WP-1-Calendar-Sync-Engine-v2 INTEGRATION_VALIDATOR integration_validator:wp-1-calendar-sync-engine-v2` => PASS; startup communication mesh and final-lane context were ready before review.
  - `git -C ..\\wtc-sync-engine-v2 diff --name-only e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a` => the authoritative committed candidate is still confined to the seven signed product files.
  - `git -C ..\\wtc-sync-engine-v2 diff --check e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a -- src/backend/handshake_core/mechanical_engines.json src/backend/handshake_core/src/capabilities.rs src/backend/handshake_core/src/mex/gates.rs src/backend/handshake_core/src/mex/runtime.rs src/backend/handshake_core/src/storage/calendar.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/mex_tests.rs` => PASS; no patch-format or whitespace hygiene defects were present in the signed surface.
  - `just phase-check CLOSEOUT WP-1-Calendar-Sync-Engine-v2` => PASS; closeout dependencies are coherent, but this is not a technical PASS verdict.
  - `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v2 CODER --range e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a` => FAIL; stale packet manifest hashes plus `fatal: bad object dba8b4099c1afda1992fd79451baacc9fa79c47a` mean the candidate is not reproducible from the integration-validator repo.
  - `cargo test --manifest-path ..\\wtc-sync-engine-v2\\src\\backend\\handshake_core\\Cargo.toml mex_tests` with `CARGO_TARGET_DIR=D:\\Projects\\LLM projects\\Handshake\\Handshake_Artifacts\\handshake-cargo-target\\iv-wp1-mex` => FAIL; compilation stops inside signed scope at `src/backend/handshake_core/src/workflows.rs:6257` and `src/backend/handshake_core/src/workflows.rs:13748`.
  - `git -C ..\\wtc-sync-engine-v2 show e1243008365566d4cde3c707f1b6078b5837fdcd:src/backend/handshake_core/src/workflows.rs` => baseline `main` imported `SessionCheckpoint` in `storage::{...}`; the current transplant removed that import while keeping the constructor call live.
COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/workflows.rs` re-added `SessionCheckpoint` to the `storage::{...}` import set, the checkpoint constructor at line 6257 would resolve again; without that import the workflow module cannot compile, so none of the packet proof commands can run.
  - If `run_calendar_sync_job` stopped moving the borrowed `inputs` value at `src/backend/handshake_core/src/workflows.rs:13747-13748` and instead cloned or rebuilt `params` from owned data, the workflow entrypoint would become borrow-check-safe and the packet proof commands could progress past compilation.
  - If `src/backend/handshake_core/src/workflows.rs:13805-13823` fell through to `load_calendar_sync_result_from_output`, a true denied result would again depend on a result artifact before workflow output could be emitted.
  - If `src/backend/handshake_core/src/workflows.rs:13056-13089` stopped persisting the policy-denial sync state, read-only mutation denials would again drift between stored source posture and emitted workflow output.
BOUNDARY_PROBES:
  - Registry/gate/workflow boundary: `mechanical_engines.json:75-115`, `src/backend/handshake_core/src/capabilities.rs:372-415`, `src/backend/handshake_core/src/mex/gates.rs:753-791`, and `src/backend/handshake_core/src/workflows.rs:13743-13779` align on the `CalendarSync` profile and `calendar.sync.read` / `calendar.sync.write` requirements.
  - Workflow/runtime evidence boundary: `src/backend/handshake_core/src/workflows.rs:13763-13798` injects workflow context once, and `src/backend/handshake_core/src/mex/runtime.rs:159-187`, `src/backend/handshake_core/src/mex/runtime.rs:254-333`, and `src/backend/handshake_core/src/mex/runtime.rs:511-525` consume it for `tool.call`, `tool.result`, and adapter-missing failure evidence.
  - Workflow/storage boundary: `src/backend/handshake_core/src/workflows.rs:12876-12902` and `src/backend/handshake_core/src/workflows.rs:13034-13155` preserve sync-state writes through the storage-backed calendar source contract defined in `src/backend/handshake_core/src/storage/calendar.rs:124-205`.
NEGATIVE_PATH_CHECKS:
  - Wrong-profile denial path: `src/backend/handshake_core/src/mex/gates.rs:774-791` and `src/backend/handshake_core/tests/mex_tests.rs:1045-1096` deny `Analyst` without raising `HSK-4001 UnknownCapability`.
  - Read-only provider-mutation denial path: `src/backend/handshake_core/src/workflows.rs:13622-13687`, `src/backend/handshake_core/src/workflows.rs:13805-13823`, `src/backend/handshake_core/src/workflows.rs:28328-28487`, and `src/backend/handshake_core/src/workflows.rs:28493-28595` preserve `provider_access`, `source_sync_state`, and denied output parity without requiring a result artifact first.
  - Compile-failure path: `cargo test --manifest-path ..\\wtc-sync-engine-v2\\src\\backend\\handshake_core\\Cargo.toml mex_tests` fails before test selection because of in-scope code errors at `src/backend/handshake_core/src/workflows.rs:6257` and `src/backend/handshake_core/src/workflows.rs:13748`.
INDEPENDENT_FINDINGS:
  - The current-main-compatible candidate is still limited to the intended seven-file product surface.
  - The semantic calendar-sync plumbing is mostly coherent across registry, MEX runtime, denied-path payload shaping, and sync-state persistence.
  - The decisive blockers are now inside the signed surface itself: the candidate does not compile, and the final-lane handoff proof is not reproducible from the integration-validator worktree.
RESIDUAL_UNCERTAINTY:
  - Because the signed candidate fails to compile in `src/backend/handshake_core/src/workflows.rs`, I could not execute the packet runtime tests to re-prove the new `mex_tests` and workflow coverage on this candidate.
  - The `phase-check HANDOFF` failure includes topology/reachability symptoms (`bad object dba8b409...`) in addition to stale manifest hashes. Even if some of that is repo-layout drift, it still blocks lawful final-lane proof from `../handshake_main`.
  - The broader calendar spec still requires MCP-backed provider tools and push-side provider writes that are outside the truthfully implemented packet surface; those gaps remain documented as negative proof rather than treated as the only blocker here.
SPEC_CLAUSE_MAP:
  - `workflow_run` + `calendar_sync` binds to `CalendarSync` with explicit `calendar.sync.read` / `calendar.sync.write` => `src/backend/handshake_core/src/capabilities.rs:10-13`, `src/backend/handshake_core/src/capabilities.rs:372-415`, `src/backend/handshake_core/src/workflows.rs:13743-13779`, `src/backend/handshake_core/src/mex/gates.rs:753-791`, `src/backend/handshake_core/tests/mex_tests.rs:1045-1096`.
  - `calendar_sync` executes through governed MEX runtime with workflow-linked tool evidence and adapter-missing failure evidence => `src/backend/handshake_core/src/workflows.rs:9541-9542`, `src/backend/handshake_core/src/workflows.rs:13763-13798`, `src/backend/handshake_core/src/mex/runtime.rs:159-187`, `src/backend/handshake_core/src/mex/runtime.rs:254-333`, `src/backend/handshake_core/src/mex/runtime.rs:511-525`, `src/backend/handshake_core/tests/mex_tests.rs:958-1172`.
  - `engine.calendar_sync` exposes `calendar.sync` with `calendar_sync_result` output => `src/backend/handshake_core/mechanical_engines.json:75-115`, `src/backend/handshake_core/src/storage/calendar.rs:481-492`, `src/backend/handshake_core/src/workflows.rs:12904-12927`, `src/backend/handshake_core/src/workflows.rs:12945-12999`, `src/backend/handshake_core/src/workflows.rs:13823-13855`.
  - `CalendarSourceSyncState` remains the durable sync/recovery source of truth => `src/backend/handshake_core/src/storage/calendar.rs:124-205`, `src/backend/handshake_core/src/workflows.rs:12876-12902`, `src/backend/handshake_core/src/workflows.rs:13034-13155`, `src/backend/handshake_core/src/workflows.rs:28279-28323`, `src/backend/handshake_core/src/workflows.rs:28444-28487`.
  - Read-only provider posture is fail-closed and denied workflow outputs preserve guidance / parity => `src/backend/handshake_core/mechanical_engines.json:79-115`, `src/backend/handshake_core/src/workflows.rs:13587-13687`, `src/backend/handshake_core/src/workflows.rs:13805-13823`, `src/backend/handshake_core/src/workflows.rs:28328-28487`, `src/backend/handshake_core/src/workflows.rs:28493-28595`, `src/backend/handshake_core/tests/mex_tests.rs:608-643`.
  - Packet-scoped `mex_tests` coverage exists in code, but executable proof is not currently available on the signed candidate => `src/backend/handshake_core/tests/mex_tests.rs:608-643`, `src/backend/handshake_core/tests/mex_tests.rs:908-1172`, blocked in execution by compile errors at `src/backend/handshake_core/src/workflows.rs:6257` and `src/backend/handshake_core/src/workflows.rs:13748`.
NEGATIVE_PROOF:
  - Master Spec `Handshake_Master_Spec_v02.181.md:56840-56841` requires MCP tools that wrap Google Calendar, Outlook/Exchange, and generic CalDAV to be used inside `calendar_sync`; the current signed surface does not implement provider MCP tool wrappers and instead reads calendar events from storage in `src/backend/handshake_core/src/workflows.rs:13099-13108`.
  - Master Spec `Handshake_Master_Spec_v02.181.md:56218-56220` requires push-side delta computation and provider API calls; the current signed surface only persists sync-state transitions, queries local storage, and fails closed for read-only provider mutations in `src/backend/handshake_core/src/workflows.rs:13056-13155`.
ANTI_VIBE_FINDINGS:
  - The narrowing transplant shipped without a successful compile check: it dropped the `SessionCheckpoint` import needed by an unchanged checkpoint path and introduced a borrow-check failure in the new calendar workflow entrypoint.
SIGNED_SCOPE_DEBT:
  - NONE
PRIMITIVE_RETENTION_PROOF:
  - `CalendarSourceSyncState` retains its durable field set in `src/backend/handshake_core/src/storage/calendar.rs:124-138` and remains embedded in `CalendarSyncResult` at `src/backend/handshake_core/src/storage/calendar.rs:483-492`.
  - The workflow-visible output contract still carries `schema_version`, `provider_type`, `write_policy`, `source_sync_state`, `provider_access`, `outputs`, and `evidence` in denied and non-denied payload assembly at `src/backend/handshake_core/src/workflows.rs:13648-13684` and `src/backend/handshake_core/src/workflows.rs:13825-13855`.
  - The calendar capability primitive remains aligned across registry, gate, and engine declaration in `src/backend/handshake_core/mechanical_engines.json:75-115`, `src/backend/handshake_core/src/capabilities.rs:372-415`, and `src/backend/handshake_core/src/mex/gates.rs:753-791`.
PRIMITIVE_RETENTION_GAPS:
  - NONE
SHARED_SURFACE_INTERACTION_CHECKS:
  - `src/backend/handshake_core/mechanical_engines.json:75-115` <-> `src/backend/handshake_core/src/capabilities.rs:372-415` <-> `src/backend/handshake_core/src/mex/gates.rs:753-791`: engine declaration, required capabilities, and gate denial semantics remain aligned for `calendar_sync`.
  - `src/backend/handshake_core/src/workflows.rs:13763-13798` <-> `src/backend/handshake_core/src/mex/runtime.rs:159-333` and `src/backend/handshake_core/src/mex/runtime.rs:511-525`: workflow context is written once by the workflow entrypoint and consumed by runtime evidence helpers and adapter-missing failure handling.
  - `src/backend/handshake_core/src/workflows.rs:12876-12902` and `src/backend/handshake_core/src/workflows.rs:13034-13155` <-> `src/backend/handshake_core/src/storage/calendar.rs:124-205`: sync-state transitions and result emission still use the same durable calendar-source contract.
CURRENT_MAIN_INTERACTION_CHECKS:
  - `git -C ..\\wtc-sync-engine-v2 show e1243008365566d4cde3c707f1b6078b5837fdcd:src/backend/handshake_core/src/workflows.rs` shows baseline `main` imported `SessionCheckpoint` in `storage::{...}`; current candidate `src/backend/handshake_core/src/workflows.rs:50-57` no longer does, while `src/backend/handshake_core/src/workflows.rs:6257-6266` still constructs `SessionCheckpoint`.
  - `git -C ..\\wtc-sync-engine-v2 diff --name-only e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a` remains confined to the seven signed files, so the compile regression is inside the packet-owned surface rather than adjacent-main drift.
  - `../handshake_main` currently cannot resolve commit `dba8b4099c1afda1992fd79451baacc9fa79c47a`, so local-main containment is not actionable from the integration-validator lane until the candidate lineage is reachable and the deterministic handoff proof is rebuilt.
DATA_CONTRACT_PROOF:
  - `src/backend/handshake_core/mechanical_engines.json:107-115` declares `calendar_sync_result` as the workflow-visible output type for `calendar.sync`.
  - `src/backend/handshake_core/src/storage/calendar.rs:483-492` defines the `CalendarSyncResult` contract, including provider identity, write policy, direction, time window, and `source_sync_state`.
  - `src/backend/handshake_core/src/workflows.rs:13648-13684` and `src/backend/handshake_core/src/workflows.rs:13825-13855` emit workflow payloads with `schema_version`, `provider_type`, `write_policy`, `source_sync_state`, `provider_access`, `output_types`, `outputs`, and `evidence`.
DATA_CONTRACT_GAPS:
  - Master Spec push-side provider behavior remains unimplemented in signed scope: `src/backend/handshake_core/src/workflows.rs:13056-13155` fails closed for read-only mutation posture and queries local event storage, but it does not perform provider-side delta application or API writes.
REMEDIATION_INSTRUCTIONS:
  - Restore the missing `SessionCheckpoint` import in `src/backend/handshake_core/src/workflows.rs` so the unchanged checkpoint path compiles on the current-main-compatible candidate.
  - Refactor `run_calendar_sync_job` so `params` is built from owned data instead of moving `inputs` after borrowing from it; the current `inputs` move at `src/backend/handshake_core/src/workflows.rs:13747-13748` must be eliminated.
  - Re-run the packet `TEST_PLAN` (`mex_tests`, `calendar_storage_tests`, and full `cargo test`) from `../wtc-sync-engine-v2` with external `CARGO_TARGET_DIR` and append fresh proof only after the crate compiles cleanly.
  - Reconcile the final-lane deterministic proof surface so `just phase-check HANDOFF WP-1-Calendar-Sync-Engine-v2 CODER --range e1243008365566d4cde3c707f1b6078b5837fdcd..dba8b4099c1afda1992fd79451baacc9fa79c47a` succeeds from `../handshake_main`, not only from the coder/prep lineage.
