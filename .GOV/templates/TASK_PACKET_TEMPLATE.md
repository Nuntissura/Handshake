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

# Task Packet: {{WP_ID}}

## METADATA
- TASK_ID: {{WP_ID}}
- WP_ID: {{WP_ID}}
- BASE_WP_ID: {{WP_ID}} (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: {{DATE_ISO}}
- MERGE_BASE_SHA: {{MERGE_BASE_SHA}} (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: {{REQUESTOR}}
- AGENT_ID: {{AGENT_ID}}
- ROLE: Orchestrator
- REFINEMENT_ENFORCEMENT_PROFILE: <pending>
- PACKET_HYDRATION_PROFILE: <pending>
- WORKFLOW_LANE: <pending>
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: <pending>
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
- SESSION_START_AUTHORITY: {{SESSION_START_AUTHORITY}}
- SESSION_HOST_PREFERENCE: {{SESSION_HOST_PREFERENCE}}
- SESSION_HOST_FALLBACK: {{SESSION_HOST_FALLBACK}}
- SESSION_LAUNCH_POLICY: {{SESSION_LAUNCH_POLICY}}
- ROLE_SESSION_RUNTIME: {{ROLE_SESSION_RUNTIME}}
- CLI_SESSION_TOOL: {{CLI_SESSION_TOOL}}
- SESSION_PLUGIN_BRIDGE_ID: {{SESSION_PLUGIN_BRIDGE_ID}}
- SESSION_PLUGIN_BRIDGE_COMMAND: {{SESSION_PLUGIN_BRIDGE_COMMAND}}
- SESSION_PLUGIN_REQUESTS_FILE: {{SESSION_PLUGIN_REQUESTS_FILE}}
- SESSION_REGISTRY_FILE: {{SESSION_REGISTRY_FILE}}
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: {{SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION}}
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: {{SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS}}
- SESSION_WATCH_POLICY: {{SESSION_WATCH_POLICY}}
- SESSION_WAKE_CHANNEL_PRIMARY: {{SESSION_WAKE_CHANNEL_PRIMARY}}
- SESSION_WAKE_CHANNEL_FALLBACK: {{SESSION_WAKE_CHANNEL_FALLBACK}}
- CLI_ESCALATION_HOST_DEFAULT: {{CLI_ESCALATION_HOST_DEFAULT}}
- MODEL_FAMILY_POLICY: {{MODEL_FAMILY_POLICY}}
- CODEX_MODEL_ALIASES_ALLOWED: {{CODEX_MODEL_ALIASES_ALLOWED}}
- ROLE_SESSION_PRIMARY_MODEL: {{ROLE_SESSION_PRIMARY_MODEL}}
- ROLE_SESSION_FALLBACK_MODEL: {{ROLE_SESSION_FALLBACK_MODEL}}
- ROLE_SESSION_REASONING_REQUIRED: {{ROLE_SESSION_REASONING_REQUIRED}}
- ROLE_SESSION_REASONING_CONFIG_KEY: {{ROLE_SESSION_REASONING_CONFIG_KEY}}
- ROLE_SESSION_REASONING_CONFIG_VALUE: {{ROLE_SESSION_REASONING_CONFIG_VALUE}}
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next {{WP_ID}}
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: {{WP_VALIDATOR_LOCAL_BRANCH}}
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: {{WP_VALIDATOR_LOCAL_WORKTREE_DIR}}
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: {{WP_VALIDATOR_REMOTE_BACKUP_BRANCH}}
- WP_VALIDATOR_REMOTE_BACKUP_URL: {{WP_VALIDATOR_REMOTE_BACKUP_URL}}
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next {{WP_ID}}
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: {{INTEGRATION_VALIDATOR_LOCAL_BRANCH}}
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: {{INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR}}
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: {{INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH}}
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: {{INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL}}
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next {{WP_ID}}
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief {{WP_ID}}
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief {{WP_ID}}
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY | ABANDONED
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- GOVERNED_VALIDATOR_DUAL_TRACK_FIELDS: MECHANICAL_TRACK_VERDICT | SPEC_RETENTION_TRACK_VERDICT
<!-- For PACKET_FORMAT_VERSION >= 2026-04-05 and RISK_TIER=MEDIUM|HIGH, both governed dual-track fields become mandatory at validator closeout. -->
- GOVERNED_VALIDATOR_COMPLETION_FIELDS: WORKFLOW_VALIDITY | SCOPE_VALIDITY | PROOF_COMPLETENESS | INTEGRATION_READINESS | DOMAIN_GOAL_COMPLETION
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: <pending>
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: <pending>
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- DATA_CONTRACT_PROFILE: <pending>
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
- RISK_TIER: <pending>
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: <pending>
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: <pending>
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: <pending>
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: <pending>
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: <pending>
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: <pending>
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: <pending>
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: <pending>
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: <pending>
- LOCAL_WORKTREE_DIR: <pending>
- REMOTE_BACKUP_BRANCH: <pending>
- REMOTE_BACKUP_URL: <pending>
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
- WP_COMMUNICATION_DIR: {{WP_COMMUNICATION_DIR}}
- WP_THREAD_FILE: {{WP_THREAD_FILE}}
- WP_RUNTIME_STATUS_FILE: {{WP_RUNTIME_STATUS_FILE}}
- WP_RECEIPTS_FILE: {{WP_RECEIPTS_FILE}}
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: {{USER_SIGNATURE}}
- PACKET_FORMAT_VERSION: {{PACKET_FORMAT_VERSION}}

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature.
- Rule: if a clause depends on a portable/shared contract type, request shape, or filter consumed by multiple backends/adapters, do not collapse portable and backend-specific proof into one row unless every declared consumer/backend is named in `CODE_SURFACES` and proven by `TESTS`/`EXAMPLES` or explicit governed debt.
- CLAUSE_ROWS:
  - CLAUSE: <spec clause / anchor summary> | CODE_SURFACES: <paths/symbols> | TESTS: <tests/commands or NONE> | EXAMPLES: <fixtures/examples or NONE> | DEBT_IDS: <SPECDEBT-... or NONE> | CODER_STATUS: <UNPROVEN|PROVED|PARTIAL|DEFERRED|NOT_APPLICABLE> | VALIDATOR_STATUS: <PENDING|CONFIRMED|PARTIAL|REJECTED|NOT_APPLICABLE>

## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: <YES|NO>
- BLOCKING_SPEC_DEBT: <YES|NO>
- DEBT_IDS: <SPECDEBT-... | NONE>
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.

## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: <YES|NO>
- HOT_FILES:
  - path/to/file
- REQUIRED_TRIPWIRE_TESTS:
  - <test or NONE>
- POST_MERGE_SPOTCHECK_REQUIRED: <YES|NO>
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
- Rule: when shared contract fields are added/changed, `REQUIRED_TRIPWIRE_TESTS` must include at least one parity or field-consumption proof across every declared consumer/backend, or the packet must carry explicit governed debt.

## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - <exact command/test/assertion target or NONE>
- CANONICAL_CONTRACT_EXAMPLES:
  - <fixture/example/golden payload/shape assertion target or NONE>
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
- Rule: portable/shared contract fields must be exercised field-by-field across the declared consumers/backends; backend-specific tests may extend that proof, but they cannot stand in for portable parity by themselves.

## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED | WAIVED_NOT_DATA_BEARING
- REASON: <explicit activation or waiver rationale>
- EVIDENCE:
  - <reviewable scope/refinement evidence or NONE>
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01`, this section is mandatory and cannot be left implicit.
- Rule: `DECISION=ACTIVE_REQUIRED` requires `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`.
- Rule: `DECISION=WAIVED_NOT_DATA_BEARING` requires `DATA_CONTRACT_PROFILE=NONE` and is legal only when packet scope does not touch concrete data-bearing or structure-bearing surfaces.
- Rule: if packet scope later changes to include persisted, emitted, schema, registry, Loom-facing, or portability-relevant data surfaces, this decision must be updated before claim/handoff/closure proceeds.

## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES | NO
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY | BACKEND_NEUTRAL | NOT_APPLICABLE
- LLM_READABILITY_POSTURE: REQUIRED | NOT_APPLICABLE
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE | NOT_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - <paths/surfaces or NONE>
- DATA_CONTRACT_RULES:
  - <explicit data rule or NONE>
- VALIDATOR_DATA_PROOF_HINTS:
  - <proof target or NONE>
- Rule: `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1` requires `DATA_CONTRACT_ACTIVE=YES` and concrete non-empty lists in this section.
- Rule: `SQL_POSTURE=SQLITE_NOW_POSTGRES_READY` means new persisted or emitted structure must stay portable to PostgreSQL unless the packet/spec explicitly authorizes a backend-specific exception.
- Rule: `LLM_READABILITY_POSTURE=REQUIRED` means meaning must live in explicit structured fields first; presentation-only strings or opaque blobs are insufficient unless the packet/spec says otherwise.
- Rule: `LOOM_INTERTWINED_POSTURE=REQUIRED_WHEN_APPLICABLE` means stable ids, explicit relations, provenance anchors, and retrieval-friendly summaries must remain explicit wherever the packet touches Loom-facing data.

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
- REFINEMENT_FILE: .GOV/refinements/{{WP_ID}}.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: <fill>
- CONTEXT_START_LINE: <fill integer>
- CONTEXT_END_LINE: <fill integer>
- CONTEXT_TOKEN: <fill>
- EXCERPT_ASCII_ESCAPED:
  ```text
  <paste excerpt>
  ```

## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: <spec clause / anchor summary> | WHY_IN_SCOPE: <fill> | EXPECTED_CODE_SURFACES: <paths/symbols> | EXPECTED_TESTS: <tests/commands> | RISK_IF_MISSED: <fill>

## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: <artifact or payload> | PRODUCER: <fill> | CONSUMER: <fill> | SERIALIZER_TRANSPORT: <fill> | VALIDATOR_READER: <fill> | TRIPWIRE_TESTS: <fill> | DRIFT_RISK: <fill>

## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - <fill>
- HOT_FILES:
  - path/to/file
- TRIPWIRE_TESTS:
  - <fill>
- CARRY_FORWARD_WARNINGS:
  - <fill>

## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - <fill>
- FILES_TO_READ:
  - path/to/file
- COMMANDS_TO_RUN:
  - <exact command>
- POST_MERGE_SPOTCHECKS:
  - <fill or NONE>

## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - <fill or NONE>

## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: <fill> (YES | NO)
- RESEARCH_CURRENCY_VERDICT: <fill> (CURRENT | STALE | NOT_APPLICABLE)
- RESEARCH_DEPTH_VERDICT: <fill> (PASS | NOT_APPLICABLE)
- GITHUB_PROJECT_SCOUTING_VERDICT: <fill> (PASS | NOT_APPLICABLE)
- SOURCE_LOG:
  - [<KIND>] <title> | <YYYY-MM-DD> | Retrieved: <YYYY-MM-DDTHH:MM:SSZ> | <https://...> | Why: <fill>
- RESEARCH_SYNTHESIS:
  - <fill>
- GITHUB_PROJECT_DECISIONS:
  - <owner/name> -> <ADOPT|ADAPT|REJECT|TRACK_ONLY> (<impact>)

## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: <fill> (YES | NO)
- MATRIX_RESEARCH_VERDICT: <fill> (PASS | NOT_APPLICABLE | NEEDS_STUBS | NEEDS_SPEC_UPDATE)
- SOURCE_SCAN_DECISIONS:
  - <source> -> <ADOPT|ADAPT|REJECT> (<resolution>)
- MATRIX_GROWTH_CANDIDATES:
  - <combo> -> <resolution> (stub: <WP-... | NONE>)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - <fill>

## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-<fill> (or NONE)
- PRIMITIVES_EXPOSED:
  - PRIM-<fill> (or NONE)
- PRIMITIVES_CREATED:
  - PRIM-<fill> (or NONE)
- MECHANICAL_ENGINES_TOUCHED:
  - engine.<fill> (or NONE)
- PRIMITIVE_INDEX_ACTION: <fill> (UPDATED | NO_CHANGE)
- FEATURE_REGISTRY_ACTION: <fill> (UPDATED | NO_CHANGE)
- UI_GUIDANCE_ACTION: <fill> (UPDATED | NO_CHANGE | NOT_APPLICABLE)
- INTERACTION_MATRIX_ACTION: <fill> (UPDATED | NO_CHANGE)
- APPENDIX_MAINTENANCE_VERDICT: <fill> (OK | NEEDS_SPEC_UPDATE | NEEDS_STUBS)
- PILLAR_ALIGNMENT_VERDICT: <fill> (OK | NEEDS_SPEC_UPDATE | NEEDS_STUBS)
- PILLARS_TOUCHED:
  - <fill> (or NONE)
- PILLARS_REQUIRING_STUBS:
  - <fill> (or NONE)
- PRIMITIVE_MATRIX_VERDICT: <fill> (OK | NEEDS_STUBS | NONE_FOUND)
- FORCE_MULTIPLIER_VERDICT: <fill> (OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE)
- FORCE_MULTIPLIER_RESOLUTIONS:
  - <combo> -> <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW> (stub: <WP-... | NONE>)
- STUB_WP_IDS: <fill> (comma-separated WP-... IDs | NONE)

## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: <fill> (OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE)
- DECOMPOSITION_ROWS:
  - PILLAR: <fill> | CAPABILITY_SLICE: <fill> | SUBFEATURES: <fill> | PRIMITIVES_FEATURES: <ids> | MECHANICAL: <engine ids> | ROI: <HIGH|MEDIUM|LOW> | RESOLUTION: <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW> | STUB: <WP-... | NONE> | NOTES: <fill>

## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: <fill> (OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE)
- ALIGNMENT_ROWS:
  - Capability: <fill> | JobModel: <AI_JOB|WORKFLOW|MECHANICAL_TOOL|UI_ACTION|NONE> | Workflow: <fill> | ToolSurface: <UNIFIED_TOOL_SURFACE|MCP|COMMAND_CENTER|UI_ONLY|NONE> | ModelExposure: <LOCAL|CLOUD|BOTH|OPERATOR_ONLY> | CommandCenter: <VISIBLE|PLANNED|NONE> | FlightRecorder: <event ids | NONE> | Locus: <VISIBLE|PLANNED|NONE> | StoragePosture: <SQLITE_NOW_POSTGRES_READY|POSTGRES_ONLY|N/A> | Resolution: <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW> | Stub: <WP-... | NONE> | Notes: <fill>

## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: <fill> (OK | REUSE_EXISTING | NEEDS_SCOPE_EXPANSION | NEEDS_STUBS | NEEDS_SPEC_UPDATE)
- MATCHED_ARTIFACT_RESOLUTIONS:
  - <artifact> -> <REUSE_EXISTING|EXPAND_IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW|KEEP_SEPARATE>
- CODE_REALITY_SUMMARY:
  - <path> -> <IMPLEMENTED|PARTIAL|NOT_PRESENT> (<artifact>)

## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: <fill> (YES | NO)
- GUI_IMPLEMENTATION_ADVICE_VERDICT: <fill> (PASS | NOT_APPLICABLE | NEEDS_STUBS | NEEDS_SPEC_UPDATE)
- GUI_REFERENCE_DECISIONS:
  - <surface> <- <source> (<resolution>)
- HANDSHAKE_GUI_ADVICE:
  - <fill>
- HIDDEN_GUI_REQUIREMENTS:
  - <fill>
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - <fill>

## SCOPE
- What:
- Why:
- Rule: `IN_SCOPE_PATHS` must list the concrete write surfaces for this WP (exact files or narrow directories).
- Rule: root governance files (`justfile`, `AGENTS.md`, `.github/**`) count as ordinary scope surfaces and must be listed explicitly if touched.
- Rule: general `.GOV/**` drift is not WP implementation scope on feature branches; governance edits are committed on `gov_kernel`.
- IN_SCOPE_PATHS:
  - path/to/file
- OUT_OF_SCOPE:
  - out/of/scope/path
- TOUCHED_FILE_BUDGET: 1
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (For explicit post-signature continuation after `TOKEN_BUDGET_EXCEEDED` / `POLICY_CONFLICT`, use `COVERS: GOVERNANCE` and name the policy conflict directly in `SCOPE` or `JUSTIFICATION`.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work {{WP_ID}}
# ...task-specific commands...
just cargo-clean
just post-work {{WP_ID}} --range {{MERGE_BASE_SHA}}..HEAD
```

### DONE_MEANS
- measurable criterion 1
- measurable criterion 2

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: {{SPEC_BASELINE}} (recorded_at: {{DATE_ISO}})
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: <pending>
- SPEC_ANCHOR_PRIMARY: {{SPEC_ANCHOR}}
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
  - .GOV/roles_shared/docs/START_HERE.md
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/roles_shared/docs/ARCHITECTURE.md
  - path/to/file
- SEARCH_TERMS:
  - "exact symbol"
  - "error code"
- RUN_COMMANDS:
  ```bash
  # task-specific commands
  ```
- RISK_MAP:
  - "risk name" -> "impact"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- For `PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1`, this section is copied from the signed refinement and should not drift.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - <fill; screens/panels/dialogs/menus>
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: <fill> | Type: <fill> | Tooltip: <fill> | Notes: <fill>
- UI_STATES (empty/loading/error):
  - <fill>
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - <fill>
- UI_ACCESSIBILITY_NOTES:
  - <fill>

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES | NO
- TRUST_BOUNDARY: <fill> (examples: client->server, server->storage, job->apply)
- SERVER_SOURCES_OF_TRUTH:
  - <fill> (what the server loads/verifies instead of trusting the client)
- REQUIRED_PROVENANCE_FIELDS:
  - <fill> (role_id, contract_id, model_id/tool_id, evidence refs, before/after spans, etc.)
- VERIFICATION_PLAN:
  - <fill> (how provenance/audit is verified and recorded; include non-spoofable checks when required)
- ERROR_TAXONOMY_PLAN:
  - <fill> (distinct error classes: stale/mismatch vs spoof attempt vs true scope violation)
- UI_GUARDRAILS:
  - <fill> (prevent stale apply; preview before apply; disable conditions)
- VALIDATOR_ASSERTIONS:
  - <fill> (what the validator must prove; spec anchors; fields present; trust boundary enforced)

## IMPLEMENTATION
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
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
  - EVIDENCE: `path/to/file:line`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/{{WP_ID}}/<name>.log` (recommended; not committed)
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
