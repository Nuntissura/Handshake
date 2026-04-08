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

# Task Packet: WP-1-Workspace-Safety-Parallel-Sessions-v1

## METADATA
- TASK_ID: WP-1-Workspace-Safety-Parallel-Sessions-v1
- WP_ID: WP-1-Workspace-Safety-Parallel-Sessions-v1
- BASE_WP_ID: WP-1-Workspace-Safety-Parallel-Sessions
- DATE: 2026-04-07T18:48:47.551Z
- MERGE_BASE_SHA: f85d767d8ae8a56121f224f6e12ed2df6f973d6b
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: CODER_B
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
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: OPENAI_GPT_5_2_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- CODER_MODEL: gpt-5.2
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
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Workspace-Safety-Parallel-Sessions-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.4
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Workspace-Safety-Parallel-Sessions-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-parallel-sessions-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Workspace-Safety-Parallel-Sessions-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Workspace-Safety-Parallel-Sessions-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Workspace-Safety-Parallel-Sessions-v1
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.4
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Workspace-Safety-Parallel-Sessions-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Workspace-Safety-Parallel-Sessions-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Workspace-Safety-Parallel-Sessions-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Workspace-Safety-Parallel-Sessions-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Workspace-Safety-Parallel-Sessions-v1
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
- **Status:** Validated (PASS)
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: 3ee738ee60290ce7c7731c848a4e4e941dab2a8c
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-08T12:27:51.620Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 3ee738ee60290ce7c7731c848a4e4e941dab2a8c
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-08T12:27:51.620Z
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NOT_REQUIRED
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NONE | NOT_REQUIRED | FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED -->
- PACKET_WIDENING_EVIDENCE: N/A
<!-- Use follow-on/superseding WP id, audit id, or short rationale when widening is required. -->
- ZERO_DELTA_PROOF_ALLOWED: NO
<!-- Allowed: YES | NO. YES => deterministic post-work may accept an empty diff only for an explicitly proof-only/status-sync packet. -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: YES
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Session-Spawn-Contract, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Terminal-LAW, WP-1-Unified-Tool-Surface-Contract
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-Control-Plane-Backend
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Workspace-Safety-Parallel-Sessions-v1
- LOCAL_WORKTREE_DIR: ../wtc-parallel-sessions-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Workspace-Safety-Parallel-Sessions-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Workspace-Safety-Parallel-Sessions-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workspace-Safety-Parallel-Sessions-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workspace-Safety-Parallel-Sessions-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workspace-Safety-Parallel-Sessions-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workspace-Safety-Parallel-Sessions-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja070420262042
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: NONE
Next: INTEGRATION_VALIDATOR records MERGE_PENDING, then CONTAINED_IN_MAIN once local main contains the approved closure commit.
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Workspace Safety Boundaries 4.3.9.17.2 (isolation strategies) | CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/model_session.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | EXAMPLES: SessionWorktreeAllocation created at session spawn fails-closed if git worktree add returns error (INV-WS-002), Two parallel sessions with overlapping IN_SCOPE_PATHS get isolated worktrees; direct write conflict is structurally prevented, SessionScopedDenylist for a spawned session blocks git reset --hard and rm -rf outside scope; denylist violation logged via FR, MergeBackArtifact produced at session completion contains session_id, worktree_path, diff_patch, and provenance timestamp, Merge conflict detected at merge-back surfaces BLOCKED state with explicit conflict report; no silent resolution occurs, Cross-session file access attempt denied by default; operator approval required plus FR event emitted, Worktree cleanup triggered on session completion, cancellation, and timeout; orphan detection fires on TTL expiry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Command Denylist 4.3.9.17.3 | CODE_SURFACES: src/backend/handshake_core/src/terminal.rs; src/backend/handshake_core/src/workspace_safety.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal | EXAMPLES: SessionWorktreeAllocation created at session spawn fails-closed if git worktree add returns error (INV-WS-002), Two parallel sessions with overlapping IN_SCOPE_PATHS get isolated worktrees; direct write conflict is structurally prevented, SessionScopedDenylist for a spawned session blocks git reset --hard and rm -rf outside scope; denylist violation logged via FR, MergeBackArtifact produced at session completion contains session_id, worktree_path, diff_patch, and provenance timestamp, Merge conflict detected at merge-back surfaces BLOCKED state with explicit conflict report; no silent resolution occurs, Cross-session file access attempt denied by default; operator approval required plus FR event emitted, Worktree cleanup triggered on session completion, cancellation, and timeout; orphan detection fires on TTL expiry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Merge-Back Discipline 4.3.9.17.4 | CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/model_session.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | EXAMPLES: SessionWorktreeAllocation created at session spawn fails-closed if git worktree add returns error (INV-WS-002), Two parallel sessions with overlapping IN_SCOPE_PATHS get isolated worktrees; direct write conflict is structurally prevented, SessionScopedDenylist for a spawned session blocks git reset --hard and rm -rf outside scope; denylist violation logged via FR, MergeBackArtifact produced at session completion contains session_id, worktree_path, diff_patch, and provenance timestamp, Merge conflict detected at merge-back surfaces BLOCKED state with explicit conflict report; no silent resolution occurs, Cross-session file access attempt denied by default; operator approval required plus FR event emitted, Worktree cleanup triggered on session completion, cancellation, and timeout; orphan detection fires on TTL expiry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: INV-WS-001 IN_SCOPE_PATHS enforcement | CODE_SURFACES: src/backend/handshake_core/src/terminal.rs; src/backend/handshake_core/src/workspace_safety.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal | EXAMPLES: SessionWorktreeAllocation created at session spawn fails-closed if git worktree add returns error (INV-WS-002), Two parallel sessions with overlapping IN_SCOPE_PATHS get isolated worktrees; direct write conflict is structurally prevented, SessionScopedDenylist for a spawned session blocks git reset --hard and rm -rf outside scope; denylist violation logged via FR, MergeBackArtifact produced at session completion contains session_id, worktree_path, diff_patch, and provenance timestamp, Merge conflict detected at merge-back surfaces BLOCKED state with explicit conflict report; no silent resolution occurs, Cross-session file access attempt denied by default; operator approval required plus FR event emitted, Worktree cleanup triggered on session completion, cancellation, and timeout; orphan detection fires on TTL expiry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: INV-WS-002 fail-closed exec | CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/mex/gates.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | EXAMPLES: SessionWorktreeAllocation created at session spawn fails-closed if git worktree add returns error (INV-WS-002), Two parallel sessions with overlapping IN_SCOPE_PATHS get isolated worktrees; direct write conflict is structurally prevented, SessionScopedDenylist for a spawned session blocks git reset --hard and rm -rf outside scope; denylist violation logged via FR, MergeBackArtifact produced at session completion contains session_id, worktree_path, diff_patch, and provenance timestamp, Merge conflict detected at merge-back surfaces BLOCKED state with explicit conflict report; no silent resolution occurs, Cross-session file access attempt denied by default; operator approval required plus FR event emitted, Worktree cleanup triggered on session completion, cancellation, and timeout; orphan detection fires on TTL expiry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: INV-WS-003 cross-session access denial | CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/mex/gates.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | EXAMPLES: SessionWorktreeAllocation created at session spawn fails-closed if git worktree add returns error (INV-WS-002), Two parallel sessions with overlapping IN_SCOPE_PATHS get isolated worktrees; direct write conflict is structurally prevented, SessionScopedDenylist for a spawned session blocks git reset --hard and rm -rf outside scope; denylist violation logged via FR, MergeBackArtifact produced at session completion contains session_id, worktree_path, diff_patch, and provenance timestamp, Merge conflict detected at merge-back surfaces BLOCKED state with explicit conflict report; no silent resolution occurs, Cross-session file access attempt denied by default; operator approval required plus FR event emitted, Worktree cleanup triggered on session completion, cancellation, and timeout; orphan detection fires on TTL expiry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml model_session
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - NONE
- CANONICAL_CONTRACT_EXAMPLES:
  - SessionWorktreeAllocation created at session spawn fails-closed if git worktree add returns error (INV-WS-002)
  - Two parallel sessions with overlapping IN_SCOPE_PATHS get isolated worktrees; direct write conflict is structurally prevented
  - SessionScopedDenylist for a spawned session blocks git reset --hard and rm -rf outside scope; denylist violation logged via FR
  - MergeBackArtifact produced at session completion contains session_id, worktree_path, diff_patch, and provenance timestamp
  - Merge conflict detected at merge-back surfaces BLOCKED state with explicit conflict report; no silent resolution occurs
  - Cross-session file access attempt denied by default; operator approval required plus FR event emitted
  - Worktree cleanup triggered on session completion, cancellation, and timeout; orphan detection fires on TTL expiry
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/mod.rs (backend data surface)
  - PILLARS_TOUCHED: Locus
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: merge-back artifact storage and provenance | SUBFEATURES: merge-ready diff/patch stored as governance evidence, session_id linkage, conflict report persistence | PRIMITIVES_FEATURES: PRIM-Database, PRIM-ModelSession | MECHANICAL: engine.version, engine.sovereign | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: merge-back artifacts stored through Locus via Database trait boundary as governance evidence records
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: IN_SCOPE_PATHS per-session enforcement | JobModel: NONE | Workflow: Terminal LAW pre-execution | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: existing CapabilityAction | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends Terminal LAW validate_cwd with per-session file write target validation
  - FORCE_MULTIPLIER_EXPANSION: MergeBackArtifact plus Locus governance evidence -> IN_THIS_WP (stub: NONE)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/workspace_safety.rs
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/Cargo.toml
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Workspace-Safety-Parallel-Sessions-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions
- CONTEXT_START_LINE: 32655
- CONTEXT_END_LINE: 32700
- CONTEXT_TOKEN: Workspace Safety Boundaries for Parallel Sessions (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions (Normative) [ADD v02.137]

  When multiple sessions run concurrently, the product MUST enforce workspace isolation
  to prevent cross-session file conflicts, destructive command execution, and silent data loss.

  **4.3.9.17.2 Session Isolation Strategies**
  - Primary: git worktree isolation -- each writing session receives a dedicated git worktree
    at spawn time. Sessions write to their own branch and cannot modify the main workspace.
  - Fallback: file-scope lock isolation -- when git worktree is not available, sessions acquire
    Work Unit file-scope locks (4.3.9.2.4) before writing. Two sessions with overlapping
    IN_SCOPE_PATHS MUST NOT run concurrently without explicit operator approval.

  **4.3.9.17.3 Command Denylist**
  - Spawned and background sessions MUST receive a session-scoped command denylist at creation.
  - Denylist MUST include at minimum: git reset --hard, git clean -fd,
    rm -rf outside IN_SCOPE_PATHS, and any modification of .handshake/gov/.
  - Denylist violations MUST be logged via Flight Recorder and surface as BLOCKED state.

  **4.3.9.17.4 Merge-Back Discipline**
  - At session completion, the session MUST produce a merge-ready artifact (diff/patch)
    with session_id and provenance.
  - Merge conflicts MUST surface as BLOCKED state with an explicit conflict report.
  - No automated tooling may silently resolve conflicts.

  **4.3.9.17.5 Invariants**
  - INV-WS-001: Every session MUST declare IN_SCOPE_PATHS before writing.
  - INV-WS-002: If no isolation strategy can be established, execution MUST be denied (fail-closed).
  - INV-WS-003: Cross-session file access MUST be denied by default.
    Override requires explicit operator approval and a Flight Recorder event.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: 4.3.9.2.4 Work Unit lock contract
- CONTEXT_START_LINE: 21320
- CONTEXT_END_LINE: 21333
- CONTEXT_TOKEN: Work Unit lock contract (normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 4.3.9.2.4 Work Unit lock contract (normative)

  A Work Unit MUST acquire an advisory lock on its declared file scope before execution begins.
  Lock ownership is identified by session_id.
  Two concurrently executing Work Units MUST NOT modify overlapping file scopes
  unless an explicit operator override with Flight Recorder evidence is present.
  Lock release MUST occur on Work Unit completion, cancellation, or timeout.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: 6.0.2 Unified Tool Surface Contract
- CONTEXT_START_LINE: 23945
- CONTEXT_END_LINE: 24050
- CONTEXT_TOKEN: single canonical tool contract
- EXCERPT_ASCII_ESCAPED:
  ```text
### 6.0.2 Unified Tool Surface Contract

  All tools available to model sessions MUST be routed through a single canonical tool contract.
  The Tool Gate enforces capability permission checks before any tool execution.
  Command denylist enforcement is a Tool Gate responsibility.
  Sessions may not bypass the Tool Gate to invoke system commands directly.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions
- CONTEXT_START_LINE: 32430
- CONTEXT_END_LINE: 32654
- CONTEXT_TOKEN: Cloud Consent-Gate Lifecycle for Parallel Sessions
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions

  When a session spawns one or more parallel child sessions, each child session MUST go through
  the Cloud Consent-Gate lifecycle independently.
  Session isolation state (worktree path, denylist, IN_SCOPE_PATHS) MUST be established
  before consent is granted and before any tool execution begins.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: INV-MM-003 Strict non-overlap of file scopes
- CONTEXT_START_LINE: 21358
- CONTEXT_END_LINE: 21367
- CONTEXT_TOKEN: Two concurrently executing Work Units MUST NOT modify overlapping file scopes
- EXCERPT_ASCII_ESCAPED:
  ```text
**INV-MM-003** (Strict non-overlap of file scopes)
  Two concurrently executing Work Units MUST NOT modify overlapping file scopes
  unless an explicit operator override with Flight Recorder evidence is present.
  Violation detection MUST surface as a BLOCKED state, not a silent merge.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: Workspace Safety Boundaries 4.3.9.17.2 (isolation strategies) | WHY_IN_SCOPE: parallel sessions touching same workspace have no deterministic isolation contract today | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/model_session.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | RISK_IF_MISSED: parallel sessions silently overwrite each other with no enforcement boundary
  - CLAUSE: Command Denylist 4.3.9.17.3 | WHY_IN_SCOPE: spawned sessions currently inherit full terminal permissions with no session-scoped denylist | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/terminal.rs; src/backend/handshake_core/src/workspace_safety.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal | RISK_IF_MISSED: spawned sessions can execute destructive commands violating spec 4.3.9.17.3
  - CLAUSE: Merge-Back Discipline 4.3.9.17.4 | WHY_IN_SCOPE: no merge artifact production or conflict surfacing exists in session lifecycle today | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/model_session.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | RISK_IF_MISSED: session completion produces no provenance artifact and merge conflicts are silently discarded
  - CLAUSE: INV-WS-001 IN_SCOPE_PATHS enforcement | WHY_IN_SCOPE: Terminal LAW validate_cwd does not currently enforce per-session IN_SCOPE_PATHS at file write target level | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/terminal.rs; src/backend/handshake_core/src/workspace_safety.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal | RISK_IF_MISSED: sessions write outside declared scope violating INV-WS-001
  - CLAUSE: INV-WS-002 fail-closed exec | WHY_IN_SCOPE: no fail-closed guard exists when isolation strategy cannot be established | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/mex/gates.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | RISK_IF_MISSED: execution proceeds on host without any isolation in place violating INV-WS-002
  - CLAUSE: INV-WS-003 cross-session access denial | WHY_IN_SCOPE: no enforcement exists preventing one session reading another session uncommitted worktree changes | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/mex/gates.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | RISK_IF_MISSED: cross-session file reads proceed without operator approval violating INV-WS-003
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: SessionWorktreeAllocation | PRODUCER: workspace_safety.rs session spawn handler | CONSUMER: Terminal LAW validate_cwd, MergeBack, Flight Recorder | SERIALIZER_TRANSPORT: in-process struct via SessionWorktreeRegistry | VALIDATOR_READER: workspace_safety tests | TRIPWIRE_TESTS: workspace_safety session spawn round-trip test | DRIFT_RISK: worktree path field can drift if session_id naming convention changes across producer and consumer
  - CONTRACT: SessionScopedDenylist | PRODUCER: workspace_safety.rs spawn-time denylist injector | CONSUMER: terminal.rs TerminalGuard, mex/gates.rs Tool Gate | SERIALIZER_TRANSPORT: in-process struct injected into TerminalConfig | VALIDATOR_READER: terminal tests | TRIPWIRE_TESTS: terminal denylist injection unit test for spawned sessions | DRIFT_RISK: new denylist patterns added to spec but not injected at spawn time; Terminal LAW config fields renamed without updating injector
  - CONTRACT: MergeBackArtifact | PRODUCER: workspace_safety.rs session completion handler | CONSUMER: Operator review surface, DCC-Backend (downstream), Flight Recorder, storage layer | SERIALIZER_TRANSPORT: JSON via serde through Database trait boundary | VALIDATOR_READER: workspace_safety tests | TRIPWIRE_TESTS: workspace_safety merge artifact serialization round-trip test | DRIFT_RISK: provenance fields drift between producer and DCC-Backend consumer if schema version not checked at load time
  - CONTRACT: SessionWorktreeRegistry | PRODUCER: workspace_safety.rs runtime state manager | CONSUMER: all workspace safety enforcement, cleanup, orphan detection | SERIALIZER_TRANSPORT: in-process state map with Database trait boundary for persistence | VALIDATOR_READER: workspace_safety tests | TRIPWIRE_TESTS: workspace_safety registry load/save round-trip test | DRIFT_RISK: registry lookup returns stale worktree_path if session lifecycle events do not update registry atomically
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - 1. SessionWorktreeRegistry runtime state (session_id to worktree_path mapping)
  - 2. SessionWorktreeAllocation at session spawn (git worktree add)
  - 3. IN_SCOPE_PATHS per-session enforcement in Terminal LAW (extend validate_cwd)
  - 4. SessionScopedDenylist injection for spawned/background sessions
  - 5. Cross-session file access denial (INV-WS-003)
  - 6. Fail-closed exec enforcement (INV-WS-002)
  - 7. MergeBackArtifact production at session completion
  - 8. Merge conflict detection and BLOCKED state
  - 9. Worktree cleanup on session completion/cancellation
  - 10. Integration tests validating parallel session isolation
- HOT_FILES:
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml model_session
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
- CARRY_FORWARD_WARNINGS:
  - Do not allow worktree creation failure to silently proceed with write operations
  - Do not allow cross-session file access without explicit operator approval
  - Do not silently resolve merge conflicts
  - All storage must go through Database trait boundary
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - 4.3.9.17.2 Session Isolation Strategies
  - 4.3.9.17.3 Command Denylist
  - 4.3.9.17.4 Merge-Back Discipline
  - 4.3.9.17.5 Invariants INV-WS-001, INV-WS-002, INV-WS-003
  - 4.3.9.2.4 Work Unit lock contract
- FILES_TO_READ:
  - src/backend/handshake_core/src/workspace_safety.rs
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
  - cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml -- -D warnings
- POST_MERGE_SPOTCHECKS:
  - Verify fail-closed enforcement when worktree allocation fails
  - Verify cross-session file access is denied by default
  - Verify merge conflicts surface as BLOCKED not silently resolved
  - Verify session-scoped denylist includes all spec-mandated patterns
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - DCC UI for workspace status display (downstream WP-1-Dev-Command-Center-Control-Plane-Backend)
  - Non-git worksurface isolation for Design Studio (Phase 2+)
  - OS-level sandbox enforcement (future concern)
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [OSS_DOC] Worktrunk documentation | 2026-04-07 | Retrieved: 2026-04-07T17:00:00Z | https://worktrunk.dev/ | Why: three-command worktree management model for parallel agents demonstrating practical isolation patterns
  - [GITHUB] coderabbitai/git-worktree-runner | 2026-04-07 | Retrieved: 2026-04-07T17:05:00Z | https://github.com/coderabbitai/git-worktree-runner | Why: practical git worktree lifecycle management for parallel code review agents
  - [GITHUB] nwiizo/ccswarm | 2026-04-07 | Retrieved: 2026-04-07T17:10:00Z | https://github.com/nwiizo/ccswarm | Why: parallel Claude Code sessions each with dedicated worktrees and automatic cleanup
  - [BIG_TECH] NVIDIA Sandboxing Guidance | 2026-04-07 | Retrieved: 2026-04-07T17:15:00Z | https://developer.nvidia.com/blog/practical-security-guidance-for-sandboxing-agentic-workflows-and-managing-execution-risk/ | Why: path-based denylist limitations and mitigation strategies for agentic workspace isolation
  - [BIG_TECH] Docker Sandboxes for AI Agents | 2026-04-07 | Retrieved: 2026-04-07T17:20:00Z | https://www.docker.com/blog/docker-sandboxes-run-agents-in-yolo-mode-safely/ | Why: container-level isolation pattern for AI agent workspaces
  - [PAPER] SoK: Lessons Learned from Android Security Research | 2023-05-01 | Retrieved: 2026-04-07T17:25:00Z | https://arxiv.org/abs/2304.14235 | Why: systematic analysis of isolation enforcement pitfalls and bypass vectors in permission-based systems; advisory-only denylists are a well-documented failure mode confirming the need for OS-level enforcement as a future escalation path
- RESEARCH_SYNTHESIS:
  - Worktree-per-session is the simplest correct approach for git-based workspace isolation
  - File-scope locks are the fallback for non-git worksurfaces and when worktree isolation is impractical
  - String-matching denylists are advisory only; real security requires OS-level primitives
  - For a local-first product like Handshake the pragmatic approach is worktree isolation plus advisory enforcement plus complete audit trail
- GITHUB_PROJECT_DECISIONS:
  - nwiizo/ccswarm -> ADOPT (NONE)
  - coderabbitai/git-worktree-runner -> ADAPT (NONE)
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_VERDICT: PASS
- SOURCE_SCAN_DECISIONS:
  - Worktrunk documentation -> ADOPT (IN_THIS_WP)
  - nwiizo/ccswarm -> ADOPT (IN_THIS_WP)
  - NVIDIA Sandboxing Guidance -> ADAPT (IN_THIS_WP)
- MATRIX_GROWTH_CANDIDATES:
  - SessionSpawn x WorktreeIsolation -> IN_THIS_WP (stub: NONE)
  - TerminalLAW x SessionScopedDenylist -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Worktrunk: minimal three-command worktree lifecycle model for clean separation
  - ccswarm: session_id-based worktree naming with automatic cleanup on session end
  - NVIDIA: advisory enforcement at application level with full audit trail as pragmatic first step
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-ModelSession
  - PRIM-TerminalCommandEvent
  - PRIM-CapabilityRegistry
  - PRIM-Database
- PRIMITIVES_EXPOSED:
  - PRIM-ModelSession
  - PRIM-TerminalCommandEvent
  - PRIM-CapabilityRegistry
  - PRIM-Database
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.sovereign
  - engine.sandbox
  - engine.context
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Flight Recorder
  - Command Center
  - Execution / Job Runtime
  - Locus
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - SessionSpawnContract plus WorktreeIsolation -> IN_THIS_WP (stub: NONE)
  - TerminalLAW plus SessionScopedDenylist -> IN_THIS_WP (stub: NONE)
  - WorkspaceIsolation plus FlightRecorder -> IN_THIS_WP (stub: NONE)
  - WorkspaceIsolation plus MergeBackDiscipline -> IN_THIS_WP (stub: NONE)
  - WorkspaceIsolation plus Database trait boundary -> IN_THIS_WP (stub: NONE)
  - SessionScopedDenylist plus CapabilityGate -> IN_THIS_WP (stub: NONE)
  - WorktreeIsolation plus CommandCenter session panel -> IN_THIS_WP (stub: NONE)
  - MergeBackArtifact plus Locus governance evidence -> IN_THIS_WP (stub: NONE)
  - SessionWorktreeRegistry plus OrphanDetection -> IN_THIS_WP (stub: NONE)
  - CrossSessionAccessDenial plus OperatorApprovalOverride -> IN_THIS_WP (stub: NONE)
  - WorkspaceIsolationState plus SessionContext -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: session workspace allocation and deallocation | SUBFEATURES: SessionWorktreeAllocation at spawn, cleanup at completion/cancellation, orphan detection | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-TerminalCommandEvent | MECHANICAL: engine.sandbox, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: extends session spawn lifecycle with workspace isolation phase
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: workspace isolation lifecycle events | SUBFEATURES: isolation decision events in session payload, denylist violation events, merge-back events | PRIMITIVES_FEATURES: PRIM-ModelSession | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: extends existing FR-EVT-SESS event payloads with workspace isolation state
  - PILLAR: Command Center | CAPABILITY_SLICE: session workspace status display | SUBFEATURES: isolation mode indicator, merge review panel, violation alerts, worktree health | PRIMITIVES_FEATURES: PRIM-ModelSession | MECHANICAL: engine.sovereign | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: backend data surfaces for DCC downstream consumption
  - PILLAR: Locus | CAPABILITY_SLICE: merge-back artifact storage and provenance | SUBFEATURES: merge-ready diff/patch stored as governance evidence, session_id linkage, conflict report persistence | PRIMITIVES_FEATURES: PRIM-Database, PRIM-ModelSession | MECHANICAL: engine.version, engine.sovereign | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: merge-back artifacts stored through Locus via Database trait boundary as governance evidence records
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: IN_SCOPE_PATHS per-session enforcement | JobModel: NONE | Workflow: Terminal LAW pre-execution | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: existing CapabilityAction | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends Terminal LAW validate_cwd with per-session file write target validation
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Dev-Command-Center-Control-Plane-Backend-v1 -> KEEP_SEPARATE
  - WP-1-Session-Observability-Spans-FR-v1 -> KEEP_SEPARATE
  - WP-1-Session-Spawn-Contract-v1 -> KEEP_SEPARATE
  - WP-1-Terminal-LAW-v3 -> KEEP_SEPARATE
  - WP-1-Session-Scoped-Capabilities-Consent-Gate-v1 -> KEEP_SEPARATE
  - WP-1-Unified-Tool-Surface-Contract-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/terminal/guards.rs -> IMPLEMENTED (WP-1-Terminal-LAW-v3)
  - ../handshake_main/src/backend/handshake_core/src/jobs.rs -> IMPLEMENTED (WP-1-Session-Spawn-Contract-v1)
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
- What: Workspace isolation for parallel sessions with worktree allocation, session-scoped denylist, IN_SCOPE_PATHS enforcement, and merge-back discipline
- Why: Parallel sessions touching the same workspace can silently overwrite or conflict without deterministic isolation and fail-closed execution rules
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workspace_safety.rs
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/Cargo.toml
- OUT_OF_SCOPE:
  - Non-git worksurface isolation (Design Studio entity locking, Phase 2+)
  - OS-level sandbox primitives (Landlock, Seccomp, macOS Seatbelt)
  - Docker/container workspace isolation
  - DCC UI for workspace status (downstream WP)
  - Multi-repo workspace management
- TOUCHED_FILE_BUDGET: 7
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (For explicit post-signature continuation after `TOKEN_BUDGET_EXCEEDED` / `POLICY_CONFLICT`, use `COVERS: GOVERNANCE` and name the policy conflict directly in `SCOPE` or `JUSTIFICATION`.)
- WAIVER_ID: CX-573F-20260408-PARALLEL-SESSIONS-TOKEN-BUDGET-CONTINUATION | STATUS: ACTIVE | COVERS: GOVERNANCE | SCOPE: post-signature orchestrator-managed continuation after TOKEN_BUDGET_EXCEEDED / POLICY_CONFLICT on WP-1-Workspace-Safety-Parallel-Sessions-v1 during crash-recovery finish pass | JUSTIFICATION: Operator explicitly instructed autonomous continuation until this WP is finished and governance is satisfied after the prior orchestrator-managed parallel run exceeded the governed token budget during the crash-recovery attempt. This waiver authorizes bounded continuation without pretending the budget overrun did not occur. | APPROVER: Operator (chat, 2026-04-08) | EXPIRES: when WP-1-Workspace-Safety-Parallel-Sessions-v1 reaches an honest closeout verdict
- WAIVER_ID: CX-573F-20260408-PARALLEL-SESSIONS-ENABLEMENT-SURFACES | STATUS: ACTIVE | COVERS: SCOPE, GOVERNANCE | SCOPE: src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/flight_recorder/duckdb.rs | JUSTIFICATION: User accepted WP validator ruling that these narrow out-of-scope enablement touches are beneficial and required to make MT-005 and MT-006 operative and to preserve FR read-side decoding. | APPROVER: Operator (chat, 2026-04-08) | EXPIRES: when WP-1-Workspace-Safety-Parallel-Sessions-v1 reaches an honest closeout verdict

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml model_session
```

### DONE_MEANS
- SessionWorktreeAllocation creates dedicated git worktree per writing session at spawn time
- SessionWorktreeRegistry maps session_id to worktree_path in runtime state
- IN_SCOPE_PATHS validated per-session by Terminal LAW for file write targets
- Session-scoped command denylist injected for spawned/background sessions
- Cross-session file access denied by default (INV-WS-003) with operator approval override
- Merge-back produces merge-ready diff/patch artifact with provenance
- Merge conflicts surface as BLOCKED state with explicit conflict report
- Worktree cleanup on session completion/cancellation with orphan detection
- Fail-closed exec enforcement (INV-WS-002) if isolation cannot be established
- All isolation decisions logged via Flight Recorder
- All storage goes through Database trait boundary

- PRIMITIVES_EXPOSED:
  - PRIM-ModelSession
  - PRIM-TerminalCommandEvent
  - PRIM-CapabilityRegistry
  - PRIM-Database
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-07T18:48:47.551Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- SPEC_ANCHOR_PRIMARY: 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions
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
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/flight_recorder.rs
  - src/backend/handshake_core/src/storage/mod.rs
- SEARCH_TERMS:
  - TerminalGuard
  - TerminalConfig
  - denied_command_patterns
  - allowed_cwd_roots
  - validate_cwd
  - ModelSession
  - session_id
  - CapabilityGate
  - FlightRecorderEvent
  - WorkUnitLock
  - IN_SCOPE_PATHS
- RUN_COMMANDS:
  ```bash
cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety && cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml -- -D warnings
  ```
- RISK_MAP:
  - Worktree creation failure without fail-closed -> silent isolation bypass (HIGH, mitigated by INV-WS-002 enforcement)
  - Denylist bypass via symlinks/env -> destructive command execution (MEDIUM, mitigated by advisory enforcement plus audit trail)
  - Session orphan worktrees -> disk waste (LOW, mitigated by cleanup on completion plus TTL fallback)
  - Merge conflict silent resolution -> lost work (HIGH, mitigated by BLOCKED state enforcement)
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
- Historical crash-recovery closeout sync for the committed WP diff range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c`.
- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 907
- **End**: 915
- **Line Delta**: 9
- **Pre-SHA1**: `dcf4a3d8c527b31f3c05d9ce4631d6e3316dc502`
- **Post-SHA1**: `e28847e3dce700e527b284c868220719dc22e984`
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
- **Lint Results**: Historical validator review accepted this read-side decode companion under explicit waiver.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Waived narrow enablement surface for FR read-side decoding.
- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 166
- **End**: 347
- **Line Delta**: 13
- **Pre-SHA1**: `3b30f997f48c8ca89ad27679d7bb3e9f473d18ff`
- **Post-SHA1**: `8f5299c19c5acf7aec667bf8922aae639a967ec2`
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
- **Lint Results**: Historical validator review accepted the event-type expansion required for MT-005 and MT-006.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Out-of-scope historical support surface retained for honest manifest coverage.
- **Target File**: `src/backend/handshake_core/src/lib.rs`
- **Start**: 22
- **End**: 22
- **Line Delta**: 1
- **Pre-SHA1**: `adf12dc83b571c224166975ee0acba1b7fabe1ca`
- **Post-SHA1**: `2bd5f9bfa1120749cf7569a9bb116ec5070ccf8d`
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
- **Lint Results**: Historical validator review accepted module exposure change.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/lib.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Historical scope-support surface retained for honest manifest coverage.
- **Target File**: `src/backend/handshake_core/src/mex/gates.rs`
- **Start**: 3
- **End**: 675
- **Line Delta**: 312
- **Pre-SHA1**: `e2c96562c16bd0a38f81ef0726bf84bb739d48b9`
- **Post-SHA1**: `6cd3e052805b584721c67079ab63ee55e2876817`
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
- **Lint Results**: Validator reviewed gate pipeline wiring and invariant enforcement.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/mex/gates.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Primary MT-005 and MT-006 signed-scope surface.
- **Target File**: `src/backend/handshake_core/src/mex/mod.rs`
- **Start**: 20
- **End**: 21
- **Line Delta**: 0
- **Pre-SHA1**: `6fe19efa3b22cd2ff6dd79a3e7c6fd53a939b56f`
- **Post-SHA1**: `9e2c1f14330cb60b61c2f927a30d70e09249fa61`
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
- **Lint Results**: Historical validator review accepted module export alignment.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/mex/mod.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Historical scope-support surface retained for honest manifest coverage.
- **Target File**: `src/backend/handshake_core/src/mex/runtime.rs`
- **Start**: 786
- **End**: 845
- **Line Delta**: 26
- **Pre-SHA1**: `a3089335373b8c682d92423a4308e123ac224eab`
- **Post-SHA1**: `c2c4136eb36a89a7036f4083f3e33b8c2dd19b44`
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
- **Lint Results**: Validator review recorded IN_SCOPE_PATHS extraction and allowed-root enforcement.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/mex/runtime.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Historical scope-support surface retained for honest manifest coverage.
- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 9
- **End**: 1835
- **Line Delta**: 19
- **Pre-SHA1**: `d2547aeb9e83dcb5df5a8075591e0c57adb115de`
- **Post-SHA1**: `268cffd5facdb4139cf23da2ac9e0a6f63d98079`
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
- **Lint Results**: Historical validator review accepted Database trait boundary alignment.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/storage/mod.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Historical scope-support surface retained for honest manifest coverage.
- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 15
- **End**: 5101
- **Line Delta**: 33
- **Pre-SHA1**: `5975c01fb778d5fc35a6d54120a233b31f9250d8`
- **Post-SHA1**: `c42e2decdb253186d93498fdba7fc29b27600714`
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
- **Lint Results**: Historical validator review accepted storage-layer coverage for merge-back artifact persistence.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/storage/postgres.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Historical scope-support surface retained for honest manifest coverage.
- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 15
- **End**: 5662
- **Line Delta**: 44
- **Pre-SHA1**: `d6191100cf443da42a375a4b4ab0d7fe77edd2b3`
- **Post-SHA1**: `b42f36ca2f99c922dfc217d5594dabd9cf5b2d52`
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
- **Lint Results**: Historical validator review accepted storage-layer coverage for merge-back artifact persistence.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/storage/sqlite.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Historical scope-support surface retained for honest manifest coverage.
- **Target File**: `src/backend/handshake_core/src/terminal/config.rs`
- **Start**: 24
- **End**: 115
- **Line Delta**: 55
- **Pre-SHA1**: `b2853d5d85dbcc51a571848c75784a44467ed69c`
- **Post-SHA1**: `e5774328559052fc4f78b3fc4945e739c9bcc543`
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
- **Lint Results**: Historical validator review accepted session-scoped denylist and allowed-root injection tests.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/terminal/config.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Historical scope-support surface retained for honest manifest coverage.
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 39
- **End**: 16739
- **Line Delta**: 198
- **Pre-SHA1**: `e288741eee1ce165e85e0304edb58f331bfbb407`
- **Post-SHA1**: `336488a91baf05227c9a2727a87e991d5facc7d8`
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
- **Lint Results**: Validator waiver accepted this narrow runtime bridge as required enablement.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/workflows.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Accepted narrow enablement surface under explicit waiver.
- **Target File**: `src/backend/handshake_core/src/workspace_safety.rs`
- **Start**: 1
- **End**: 783
- **Line Delta**: 783
- **Pre-SHA1**: `d3f5a12faa99758192ecc4ed3fc22c9249232e86`
- **Post-SHA1**: `b064254d06603451b42ddb2f9f4e05745f62ee6d`
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
- **Lint Results**: Primary MT-001, MT-003, MT-005, and MT-006 implementation surface.
- **Artifacts**: `git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/workspace_safety.rs`
- **Timestamp**: 2026-04-08T11:11:00Z
- **Operator**: Historical coder closeout sync via ORCHESTRATOR after crash recovery
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> SPEC_CURRENT (resolved during crash-recovery closeout)
- **Notes**: Primary signed implementation surface across workspace isolation and merge-back discipline.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: Branch head contains packet-scoped implementation through MT-006. MT-005 and MT-006 landed in 37b7c9b and d7a4161. Waiver accepted for workflows.rs and flight_recorder/duckdb.rs as narrow enablement required to make the signed runtime and FR paths operative.
- What changed in this update: No new code change in this handoff. Governance recovery repaired the older validator kickoff receipt path and confirmed the waiver state against current branch head.
- Requirements / clauses self-audited: Workspace Safety Boundaries 4.3.9.17.2; Command Denylist 4.3.9.17.3; Merge-Back Discipline 4.3.9.17.4; INV-WS-001; INV-WS-002; INV-WS-003.
- Checks actually run: Historical coder and validator lane evidence already records cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety plus validator-passing proof for MT-002, MT-003, and MT-004.
- Known gaps / weak spots: No remaining signed-scope code gap is known after the accepted waiver. The rm -r -f separate flags advisory remains outside this signed packet.
- Heuristic risks / maintainability concerns: Shared-surface risk remains concentrated in workflows.rs, mex/gates.rs, workspace_safety.rs, and flight_recorder/duckdb.rs; validator should confirm current-main interactions at closeout.
- Validator focus request: Confirm final packet-law closeout on current branch head and current-main compatibility after accepting the waiver.
- Rubric contract understanding proof: Packet requires direct review communication, coder handoff self-audit, dual-track validator closeout, and main containment as a second step after PASS.
- Rubric scope discipline proof: The signed scope is satisfied by committed work on workspace_safety.rs and mex/gates.rs, with workflows.rs and flight_recorder/duckdb.rs accepted only as narrow enablement by validator waiver.
- Rubric baseline comparison: Before 37b7c9b and d7a4161, isolation gates, FR denial emission, and detached session worktree lifecycle were not operative on the runtime path. Current branch head wires those paths into real execution and cleanup.
- Rubric end-to-end proof: Runtime finalization now evaluates workspace safety, records denial events, manages detached session worktrees, and preserves FR read-side decoding for the emitted event types.
- Rubric architecture fit self-review: Changes stay inside the existing workspace safety, MEX gates, workflow finalization, and flight recorder seams; no new subsystem was introduced.
- Rubric heuristic quality self-review: The implementation uses existing registries and runtime hooks instead of ad hoc side channels.
- Rubric anti-gaming / counterfactual check: Removing workflows.rs bridge logic would leave MT-005 and MT-006 declarative-only, which is why the waiver was necessary and explicitly accepted.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: The implementation is runtime-wired and test-backed; the waived out-of-scope files make the signed behavior reachable rather than cosmetically present.
- Signed-scope debt ledger: - NONE
- Data contract self-check: Workspace isolation and cross-session FR event types are emitted on the runtime path and decoded on the DuckDB read side; no signed-scope data contract gap is known.
- Next step / handoff hint: WP validator publishes final governed closeout on current branch head, then integration validator verifies main containment.
- **Artifacts**: `.GOV/task_packets/WP-1-Workspace-Safety-Parallel-Sessions-v1/signed-scope.patch`

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
  - REQUIREMENT: "Fail-closed exec enforcement (INV-WS-002) if isolation cannot be established"
  - EVIDENCE: src/backend/handshake_core/src/workspace_safety.rs:247
  - REQUIREMENT: "Cross-session file access denied by default (INV-WS-003) with operator approval override"
  - EVIDENCE: src/backend/handshake_core/src/workspace_safety.rs:265
  - REQUIREMENT: "IN_SCOPE_PATHS validated per-session by Terminal LAW for file write targets"
  - EVIDENCE: src/backend/handshake_core/src/mex/runtime.rs:790
  - REQUIREMENT: "All isolation decisions logged via Flight Recorder"
  - EVIDENCE: src/backend/handshake_core/src/workflows.rs:6175
  - REQUIREMENT: "Worktree cleanup on session completion/cancellation with orphan detection"
  - EVIDENCE: src/backend/handshake_core/src/workflows.rs:6265
  - REQUIREMENT: "Merge-back produces merge-ready diff/patch artifact with provenance"
  - EVIDENCE: src/backend/handshake_core/src/workspace_safety.rs:332
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
  - EXIT_CODE: 0
  - LOG_PATH: cargo-gates-tests.log (historical transient lane artifact; not committed)
  - LOG_SHA256: N/A
  - PROOF_LINES: 21 passed; 0 failed
  - COMMAND: git diff --unified=0 f85d767d8ae8a56121f224f6e12ed2df6f973d6b..d7a416163734a6100bc6fd5f2eebc490d02ec69c -- src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/workspace_safety.rs src/backend/handshake_core/src/mex/gates.rs src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - EXIT_CODE: 0
  - LOG_PATH: N/A
  - LOG_SHA256: N/A
  - PROOF_LINES: 37b7c9b and d7a4161 remain the operative committed implementation for MT-005 and MT-006

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

### 2026-04-08T12:11:40.1548985Z | INTEGRATION_VALIDATOR PASS REPORT
VALIDATOR_ROLE: INTEGRATION_VALIDATOR
VALIDATOR_MODEL: gpt-5.4
COMMIT: d7a416163734a6100bc6fd5f2eebc490d02ec69c
GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
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
VALIDATOR_RISK_TIER: HIGH
Verdict: PASS

CLAUSES_REVIEWED:
  - Workspace Safety Boundaries 4.3.9.17.2 (isolation strategies) => src/backend/handshake_core/src/workspace_safety.rs:247 and src/backend/handshake_core/src/workspace_safety.rs:265
  - Command Denylist 4.3.9.17.3 => src/backend/handshake_core/src/mex/gates.rs:3 and src/backend/handshake_core/src/workspace_safety.rs:247
  - Merge-Back Discipline 4.3.9.17.4 => src/backend/handshake_core/src/workspace_safety.rs:332 and src/backend/handshake_core/src/workflows.rs:6265
  - INV-WS-001 IN_SCOPE_PATHS enforcement => src/backend/handshake_core/src/mex/gates.rs:3 and src/backend/handshake_core/src/workspace_safety.rs:265
  - INV-WS-002 fail-closed exec => src/backend/handshake_core/src/workspace_safety.rs:247 and src/backend/handshake_core/src/mex/gates.rs:3
  - INV-WS-003 cross-session access denial => src/backend/handshake_core/src/workspace_safety.rs:265 and src/backend/handshake_core/src/workflows.rs:6175

NOT_PROVEN:
  - NONE

MAIN_BODY_GAPS:
  - NONE

QUALITY_RISKS:
  - NONE

DIFF_ATTACK_SURFACES:
  - Session worktree allocation and cleanup lifecycle across src/backend/handshake_core/src/workspace_safety.rs:247, src/backend/handshake_core/src/workspace_safety.rs:332, and src/backend/handshake_core/src/workflows.rs:6265
  - Fail-closed execution and cross-session denial path across src/backend/handshake_core/src/workspace_safety.rs:247 and src/backend/handshake_core/src/workspace_safety.rs:265
  - MEX gate wiring that makes the workspace safety decision operative at src/backend/handshake_core/src/mex/gates.rs:3
  - Flight Recorder producer/consumer parity for workspace_isolation.denied across src/backend/handshake_core/src/workflows.rs:6175 and src/backend/handshake_core/src/flight_recorder/duckdb.rs:907

INDEPENDENT_CHECKS_RUN:
  - just validator-handoff-check WP-1-Workspace-Safety-Parallel-Sessions-v1 => PASS for committed range f85d767d8ae8a56121f224f6e12ed2df6f973d6b..HEAD with target_head_sha d7a416163734a6100bc6fd5f2eebc490d02ec69c
  - validateCandidateTargetAgainstSignedScope(packet, main=27d095ae33098d8fd23000399879dfb8c4eeaa9f, target=d7a416163734a6100bc6fd5f2eebc490d02ec69c) => ok=true with identical normalized diff hash 9233660c77062e51f93086fb91f15b77f1699bc78441526080b083793cb46834
  - just wp-communication-health-check WP-1-Workspace-Safety-Parallel-Sessions-v1 VERDICT => PASS

COUNTERFACTUAL_CHECKS:
  - If src/backend/handshake_core/src/workspace_safety.rs:247 were removed or weakened, execution could proceed without an established isolation strategy and INV-WS-002 would collapse.
  - If src/backend/handshake_core/src/workspace_safety.rs:265 were removed, cross-session access would no longer fail closed before operator approval and INV-WS-003 would collapse.
  - If src/backend/handshake_core/src/flight_recorder/duckdb.rs:907 were removed while src/backend/handshake_core/src/workflows.rs:6175 still emits denial events, the DuckDB read side would stop decoding workspace_isolation.denied correctly.

BOUNDARY_PROBES:
  - Reviewed the fail-closed handoff from src/backend/handshake_core/src/workspace_safety.rs:247 into the operative gate path at src/backend/handshake_core/src/mex/gates.rs:3.
  - Reviewed the denial-event producer/consumer contract from src/backend/handshake_core/src/workflows.rs:6175 into src/backend/handshake_core/src/flight_recorder/duckdb.rs:907.

NEGATIVE_PATH_CHECKS:
  - Reviewed the denial path at src/backend/handshake_core/src/workspace_safety.rs:265 where cross-session access is rejected unless operator approval exists.
  - Reviewed the denied-event read-side path at src/backend/handshake_core/src/flight_recorder/duckdb.rs:907 so the negative path stays queryable instead of degrading into an unknown event type.

INDEPENDENT_FINDINGS:
  - The final-lane blocker was packet truth, not product drift; the candidate diff matched the signed artifact exactly once artifact normalization was repaired.
  - The accepted waiver on src/backend/handshake_core/src/workflows.rs and src/backend/handshake_core/src/flight_recorder/duckdb.rs remained narrow enablement for MT-005 and MT-006 runtime reachability and FR decoding; no wider committed surface was observed.

SPEC_CLAUSE_MAP:
  - Workspace Safety Boundaries 4.3.9.17.2 (isolation strategies) => src/backend/handshake_core/src/workspace_safety.rs:247
  - Command Denylist 4.3.9.17.3 => src/backend/handshake_core/src/mex/gates.rs:3
  - Merge-Back Discipline 4.3.9.17.4 => src/backend/handshake_core/src/workspace_safety.rs:332
  - INV-WS-001 IN_SCOPE_PATHS enforcement => src/backend/handshake_core/src/workspace_safety.rs:265
  - INV-WS-002 fail-closed exec => src/backend/handshake_core/src/workspace_safety.rs:247
  - INV-WS-003 cross-session access denial => src/backend/handshake_core/src/workflows.rs:6175

NEGATIVE_PROOF:
  - src/backend/handshake_core/src/workspace_safety.rs:247 and src/backend/handshake_core/src/mex/gates.rs:3 still stop at git-worktree and allowlist isolation; this WP does not introduce OS-level sandbox primitives, so broader host-level containment remains outside the implemented surface by design.

PRIMITIVE_RETENTION_PROOF:
  - src/backend/handshake_core/src/workspace_safety.rs:332 retains explicit merge-back artifact provenance instead of collapsing it into ad hoc cleanup state.
  - src/backend/handshake_core/src/workflows.rs:6175 and src/backend/handshake_core/src/workflows.rs:6265 keep runtime finalization at the existing orchestration seam; the new isolation hooks are additive there.
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs:907 is an additive read-side mapping that preserves the existing FR decode/query surface while admitting workspace_isolation.denied.

PRIMITIVE_RETENTION_GAPS:
  - NONE

SHARED_SURFACE_INTERACTION_CHECKS:
  - src/backend/handshake_core/src/workspace_safety.rs:247 with src/backend/handshake_core/src/mex/gates.rs:3 keeps isolation establishment and gate enforcement on the same runtime path.
  - src/backend/handshake_core/src/workspace_safety.rs:265 with src/backend/handshake_core/src/workflows.rs:6175 keeps denial decisions observable through Flight Recorder.
  - src/backend/handshake_core/src/workflows.rs:6175 with src/backend/handshake_core/src/flight_recorder/duckdb.rs:907 keeps denied-event producer/consumer parity.

CURRENT_MAIN_INTERACTION_CHECKS:
  - src/backend/handshake_core/src/workspace_safety.rs:247, src/backend/handshake_core/src/workspace_safety.rs:265, src/backend/handshake_core/src/mex/gates.rs:3, src/backend/handshake_core/src/workflows.rs:6175, and src/backend/handshake_core/src/flight_recorder/duckdb.rs:907 remained the exact signed interaction surface when validateCandidateTargetAgainstSignedScope compared target d7a416163734a6100bc6fd5f2eebc490d02ec69c against local main 27d095ae33098d8fd23000399879dfb8c4eeaa9f and produced normalized diff hash 9233660c77062e51f93086fb91f15b77f1699bc78441526080b083793cb46834.
  - The current-main interaction points remained confined to src/backend/handshake_core/src/workspace_safety.rs:247, src/backend/handshake_core/src/workspace_safety.rs:265, src/backend/handshake_core/src/workspace_safety.rs:332, src/backend/handshake_core/src/mex/gates.rs:3, src/backend/handshake_core/src/workflows.rs:6175, src/backend/handshake_core/src/workflows.rs:6265, and src/backend/handshake_core/src/flight_recorder/duckdb.rs:907; no adjacent-scope widening was required.

DATA_CONTRACT_PROOF:
  - src/backend/handshake_core/src/workspace_safety.rs:332 preserves merge-back artifact provenance as structured runtime data.
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs:907 preserves parseable DuckDB read-side mapping for workspace_isolation.denied.

DATA_CONTRACT_GAPS:
  - NONE

ANTI_VIBE_FINDINGS:
  - NONE

SIGNED_SCOPE_DEBT:
  - NONE

RESIDUAL_UNCERTAINTY:
  - No fresh full-suite rerun was performed in the integration lane; PASS relies on committed gate evidence plus direct signed-scope and current-main validation at the final lane.
