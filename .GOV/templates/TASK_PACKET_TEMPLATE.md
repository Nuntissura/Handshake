# TASK_PACKET_TEMPLATE

Copy this into each new task packet and fill all fields.

Requirements:
- Keep packets ASCII-only (required by deterministic gates).
- Use SPEC_BASELINE for provenance (spec at creation time).
- Use SPEC_TARGET as the authoritative spec for closure/revalidation (usually .GOV/roles_shared/records/SPEC_CURRENT.md).
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
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_V1
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- CLAUSE_CLOSURE_MONITOR_PROFILE: <pending>
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: <pending>
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Ready for Dev
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) -->
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
- WP_COMMUNICATION_DIR: .GOV/roles_shared/runtime/WP_COMMUNICATIONS/{{WP_ID}}
- WP_THREAD_FILE: .GOV/roles_shared/runtime/WP_COMMUNICATIONS/{{WP_ID}}/THREAD.md
- WP_RUNTIME_STATUS_FILE: .GOV/roles_shared/runtime/WP_COMMUNICATIONS/{{WP_ID}}/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: .GOV/roles_shared/runtime/WP_COMMUNICATIONS/{{WP_ID}}/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- USER_SIGNATURE: {{USER_SIGNATURE}}
- PACKET_FORMAT_VERSION: {{PACKET_FORMAT_VERSION}}

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature.
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

## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - <exact command/test/assertion target or NONE>
- CANONICAL_CONTRACT_EXAMPLES:
  - <fixture/example/golden payload/shape assertion target or NONE>
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
- IN_SCOPE_PATHS:
  - path/to/file
- OUT_OF_SCOPE:
  - out/of/scope/path

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
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
- SPEC_TARGET: .GOV/roles_shared/records/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: <pending>
- SPEC_ANCHOR: {{SPEC_ANCHOR}}
- Codex: Handshake Codex v1.4.md
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
  - .GOV/roles_shared/records/SPEC_CURRENT.md
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
- **Spec Target Resolved**: .GOV/roles_shared/records/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

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
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
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
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
