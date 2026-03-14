# Task Packet: WP-1-Structured-Collaboration-Schema-Registry-v1

## METADATA
- TASK_ID: WP-1-Structured-Collaboration-Schema-Registry-v1
- WP_ID: WP-1-Structured-Collaboration-Schema-Registry-v1
- BASE_WP_ID: WP-1-Structured-Collaboration-Schema-Registry
- DATE: 2026-03-14T00:36:43.121Z
- MERGE_BASE_SHA: 1a2568b0842ecc7b9b9aca4efcc9911cc2ce8cc8 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
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
- CODER_MODEL: Coder-A
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
- SESSION_PLUGIN_REQUESTS_FILE: .GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: .GOV/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: WINDOWS_TERMINAL
- MODEL_FAMILY_POLICY: OPENAI_GPT_SERIES_ONLY
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Structured-Collaboration-Schema-Registry-v1
- WP_VALIDATOR_LOCAL_BRANCH: validate/WP-1-Structured-Collaboration-Schema-Registry-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wt-WPV-WP-1-Structured-Collaboration-Schema-Registry-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: validate/WP-1-Structured-Collaboration-Schema-Registry-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/validate/WP-1-Structured-Collaboration-Schema-Registry-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Schema-Registry-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: integrate/WP-1-Structured-Collaboration-Schema-Registry-v1
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../wt-INTV-WP-1-Structured-Collaboration-Schema-Registry-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: integrate/WP-1-Structured-Collaboration-Schema-Registry-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/integrate/WP-1-Structured-Collaboration-Schema-Registry-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Structured-Collaboration-Schema-Registry-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Structured-Collaboration-Schema-Registry-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Structured-Collaboration-Schema-Registry-v1
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
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
- LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v1
- LOCAL_WORKTREE_DIR: ../wt-WP-1-Structured-Collaboration-Schema-Registry-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Schema-Registry-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Schema-Registry-v1
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
- WP_COMMUNICATION_DIR: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v1
- WP_THREAD_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- USER_SIGNATURE: ilja140320260133
- PACKET_FORMAT_VERSION: 2026-03-12
## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Structured-Collaboration-Schema-Registry-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [BIG_TECH] Atlassian Jira Issue Fields docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-fields/ | Why: shows typed field authority reused by multiple issue and board views
  - [BIG_TECH] GitHub Projects fields docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://docs.github.com/en/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: shows stable project-item fields driving multiple projections and layouts
  - [OSS_DOC] Backstage descriptor format docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://backstage.io/docs/features/software-catalog/descriptor-format/ | Why: useful reference for a shared core envelope with bounded extension metadata
  - [GITHUB] Backstage repository | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://github.com/backstage/backstage | Why: concrete repository-scale example of descriptor-backed projections and extensibility pressure
  - [PAPER] FocusLLM paper | 2024-08-21 | Retrieved: 2026-03-13T22:38:08Z | https://arxiv.org/abs/2408.11745 | Why: supports compact-summary-first loading for smaller local models before detail hydration
- RESEARCH_SYNTHESIS:
  - Handshake should keep one field-authoritative collaboration record family and let board, queue, mailbox, and viewer surfaces remain projections over that family.
  - The shared envelope should stay intentionally small and stable while project-specific payloads move behind explicit extension schemas and compatibility checks.
  - Summary artifacts should be first-read surfaces for smaller local models and operator triage, with canonical detail loaded only when required.
  - Strong registry behavior is not just about naming schema ids; it also needs deterministic incompatibility reporting so future kernels do not guess across unknown profile extensions.
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
  - Shared base envelope plus compact summary pairing -> IN_THIS_WP (stub: NONE)
  - Base descriptor plus bounded extension compatibility -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep schema id/version constants and compatibility policy in one registry surface instead of scattering them across emitters.
  - Validate summary/detail shared identity and authority refs mechanically before allowing summary-first reads.
  - Separate base-envelope validation from extension validation so unknown extensions never force parser forks or silent fallback.
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
  - Locus tracked record families plus one shared registry -> IN_THIS_WP (stub: NONE)
  - Work Packet detail plus compact summary join validation -> IN_THIS_WP (stub: NONE)
  - Micro-Task detail plus compact summary join validation -> IN_THIS_WP (stub: NONE)
  - Task Board row, index, and view validation against the same envelope -> IN_THIS_WP (stub: NONE)
  - Command Center diagnostics over registry mismatch results -> IN_THIS_WP (stub: NONE)
  - Role Mailbox export validation with strict runtime/governance boundary -> IN_THIS_WP (stub: NONE)
  - Schema-version mismatch diagnostics at the parser boundary -> IN_THIS_WP (stub: NONE)
  - Profile-extension compatibility gating over canonical records -> IN_THIS_WP (stub: NONE)
  - Summary/detail authority-ref validation across all collaboration families -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: Shared collaboration base-envelope validation | SUBFEATURES: Work Packet, Micro-Task, Task Board record identity and compatibility checks | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the registry must validate one field-equivalent envelope across the main Locus-owned record families
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Canonical packet and summary schema registration | SUBFEATURES: `packet.json`, `summary.json`, compatibility readers, profile-extension enforcement | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: packet and summary validation must share ids, authorities, and extension policy
  - PILLAR: MicroTask | CAPABILITY_SLICE: Canonical micro-task packet and summary schema registration | SUBFEATURES: `packet.json`, `summary.json`, compatibility readers, machine-readable mismatch results | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task routing and bounded execution depend on the same registry guarantees as work packets
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Structured projection schema registration | SUBFEATURES: `index.json`, `views/{view_id}.json`, row validation, shared summary joins | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-StructuredCollaborationEnvelopeV1, PRIM-MirrorSyncState | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: board layouts need one parser boundary instead of board-only schema forks
  - PILLAR: Command Center | CAPABILITY_SLICE: Registry-driven validation diagnostics | SUBFEATURES: unknown-schema, incompatible-extension, and summary-drift outputs consumable by generic viewers | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-StructuredCollaborationSummaryV1, FEAT-DEV-COMMAND-CENTER | MECHANICAL: engine.context, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the packet should emit deterministic validator outputs that the viewer packet can consume later
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact-summary-first compatibility enforcement | SUBFEATURES: shared identity and authority refs across detail and summary records | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationSummaryV1, PRIM-MirrorSyncState, PRIM-ProjectProfileExtensionV1 | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: small-model reads must not guess across mismatched summaries or unknown extensions
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Shared collaboration schema registry | JobModel: WORKFLOW | Workflow: locus_structured_artifact_publish | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: registry outputs should be visible to both generic viewers and runtime artifact producers without adding a new database coupling
  - Capability: Compact summary compatibility validation | JobModel: WORKFLOW | Workflow: compact_summary_emit | ToolSurface: COMMAND_CENTER | ModelExposure: LOCAL | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: summary-first local-model routing depends on deterministic summary/detail compatibility checks
  - Capability: Task Board structured projection validation | JobModel: WORKFLOW | Workflow: task_board_projection_publish | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: board projections need one validator path that explains unknown schema, drift, and missing envelope fields
  - Capability: Role Mailbox export schema validation | JobModel: WORKFLOW | Workflow: role_mailbox_export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox export validation must stay scoped to product-runtime collaboration artifacts and not collapse into `.GOV` control-plane validation
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Project-Profile-Extension-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Markdown-Mirror-Sync-Drift-Guard-v1 -> KEEP_SEPARATE
  - WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Artifact-Family-v1 -> KEEP_SEPARATE
  - WP-1-Role-Mailbox-v1 -> KEEP_SEPARATE
  - WP-1-Locus-Phase1-Integration-Occupancy-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Structured-Collaboration-Artifact-Family-v1)
  - src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Structured-Collaboration-Artifact-Family-v1)
  - src/backend/handshake_core/src/runtime_governance.rs -> PARTIAL (NONE)
  - src/backend/handshake_core/src/role_mailbox.rs -> PARTIAL (WP-1-Role-Mailbox-v1)
  - .GOV/scripts/validation/role_mailbox_export_check.mjs -> PARTIAL (NONE)
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
- What: Implement the canonical schema registry, compatibility-reader policy, and deterministic validation outputs for the shared structured-collaboration envelope and compact summary contract used by Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports.
- Why: The spec already defines the shared record law, but runtime code still spreads schema ids and validation assumptions across emitters. This packet centralizes schema authority so downstream profile-extension, mirror-sync, and viewer work can build on one deterministic parser boundary.
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
- One canonical registry owns schema ids, schema versions, and compatibility-reader policy for packet, summary, task-board, and mailbox collaboration artifacts.
- Unknown or incompatible schema versions produce deterministic machine-readable validation results instead of silent fallback.
- Summary/detail joins validate shared identity, authority refs, and project-profile posture consistently across the collaboration artifact family.
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
- SPEC_BASELINE: Handshake_Master_Spec_v02.178.md (recorded_at: 2026-03-14T00:36:43.121Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.168]
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Base structured schema and project-profile extension contract [ADD v02.168]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - Handshake_Master_Spec_v02.178.md
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
- RUN_COMMANDS:
  ```bash
rg -n "schema_id|schema_version|project_profile_kind|mirror_state|authority_refs|evidence_refs|summary.json|profile_extension|role_mailbox_index|role_mailbox_thread_line" src/backend/handshake_core
  cargo test -p handshake_core
  just gov-check
  ```
- RISK_MAP:
  - "schema ids remain hardcoded in multiple emitters" -> "future readers drift and compatibility fixes become multi-file repair work"
  - "summary/detail joins stay implicit" -> "local-small-model routing and operator triage trust mismatched summaries"
  - "runtime and governance mailbox paths remain conflated" -> "the packet validates the wrong authority surface and hides real product regressions"
  - "profile extensions are not compatibility-gated" -> "future project kernels silently fork the shared record contract"
## SKELETON
- Proposed interfaces/types/contracts:
  - `src/backend/handshake_core/src/locus/types.rs`
    - Add a canonical `StructuredCollaborationSchemaRegistry` surface plus typed descriptors for `tracked_work_packet`, `tracked_micro_task`, `task_board_entry`, `role_mailbox_index`, and `role_mailbox_thread_line`.
    - Add machine-readable validation payloads for `unknown_schema`, `schema_version_mismatch`, `incompatible_profile_extension`, `summary_join_mismatch`, and `authority_scope_mismatch`.
    - Add shared envelope and compact-summary helper types so workflow code validates `record_id`, `record_kind`, `project_profile_kind`, `mirror_state`, `authority_refs`, `evidence_refs`, and summary/detail linkage through one path.
  - `src/backend/handshake_core/src/runtime_governance.rs`
    - Add runtime artifact path and display helpers for work-packet `packet.json` and `summary.json`, micro-task `packet.json` and `summary.json`, task-board `index.json` and `views/{view_id}.json`, and mailbox `index.json` plus `threads/*.jsonl`.
    - Add a strict runtime-authority helper that treats `.handshake/gov/**` as valid product-runtime schema authority and rejects `.GOV/**` when registry validation is checking collaboration artifact ownership.
  - `src/backend/handshake_core/src/workflows.rs`
    - Replace file-local schema literals in the micro-task executor and task-board sync paths with registry lookups from `locus/types.rs`.
    - Emit deterministic validation objects into workflow results and persisted metadata when a summary/detail pair, schema version, or profile-extension posture is incompatible instead of silently accepting or skipping the check.
  - `src/backend/handshake_core/src/locus/task_board.rs`
    - Extend the task-board model from markdown tokens only to registry-backed projection row, index, and view builders and validators while keeping markdown rewrite behavior as a derived view.
    - Validate task-board row identity and shared authority refs against the same registry contract used by work-packet and micro-task artifacts.
  - `src/backend/handshake_core/src/role_mailbox.rs` and `src/backend/handshake_core/src/api/role_mailbox.rs`
    - Replace mailbox-local export shaping (`schema_version` only today) with registry-backed `index.json` and `threads/*.jsonl` records that carry full base-envelope identity and version fields plus deterministic validation failures.
    - Make the API `read_index` path validate the exported runtime `index.json` through the same registry before returning it.
  - Tests
    - `src/backend/handshake_core/tests/role_mailbox_tests.rs`: extend deterministic export coverage to assert registry fields on `index.json` and thread lines, shared authority/evidence posture, and rejection/reporting when mailbox export authority drifts toward `.GOV/**` or an incompatible schema id/version is injected.
    - `src/backend/handshake_core/tests/micro_task_executor_tests.rs`: add executor/Locus coverage proving registry-backed micro-task validation returns machine-readable mismatch payloads for summary/detail drift, unknown schema versions, or incompatible profile-extension posture instead of silent fallback.
- Open questions:
  - Prefer to keep the existing persisted `TrackedWorkPacket` and `TrackedMicroTask` storage layout stable and derive the shared envelope through registry helpers first; only widen persisted structs if the current serialization path cannot express the required fields without breaking existing compatibility.
  - Confirm the concrete runtime location for structured task-board projection JSON (`.handshake/gov/task_board/...` sibling directory versus the current `TASK_BOARD.md`-only layout) before implementation, because `runtime_governance.rs` does not expose those paths yet.
  - Keep `export_manifest.json` as mailbox-export plumbing unless implementation proves it must join the canonical registry; the registry authority in this WP is `index.json` plus `threads/*.jsonl`, not governance-side `.GOV` ledgers.
- Notes:
  - No Loom portability files, `.GOV` control-plane mailbox validation surfaces, or viewer/UI work will be touched in this WP.
  - The implementation will stay inside the packet-listed backend files plus the two listed test files, and the explicit verification target remains `cargo test -p handshake_core` plus `just gov-check`.

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- REASON_NO: This packet is a backend registry and validation activation pass; no separate bootstrap-time end-to-end closure plan is required beyond the signed scope, DONE_MEANS, and validator checks.

## IMPLEMENTATION
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <target-file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `N/A until implementation begins`
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
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
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
  - EVIDENCE: `N/A until implementation begins`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Structured-Collaboration-Schema-Registry-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
