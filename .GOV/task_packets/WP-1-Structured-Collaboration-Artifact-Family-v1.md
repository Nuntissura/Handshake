# Task Packet: WP-1-Structured-Collaboration-Artifact-Family-v1

## METADATA
- TASK_ID: WP-1-Structured-Collaboration-Artifact-Family-v1
- WP_ID: WP-1-Structured-Collaboration-Artifact-Family-v1
- BASE_WP_ID: WP-1-Structured-Collaboration-Artifact-Family
- DATE: 2026-03-12T19:23:27.829Z
- MERGE_BASE_SHA: 3068595fa5c194ffa09a87de60daa9e5c3b7d052 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: CODER_A
<!-- Required before packet creation: CODER_A | CODER_B -->
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
- **Status:** Done
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: YES
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Locus-Phase1-Integration-Occupancy, WP-1-Role-Mailbox, WP-1-Micro-Task-Executor
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Markdown-Mirror-Sync-Drift-Guard, WP-1-Dev-Command-Center-Structured-Artifact-Viewer
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Structured-Collaboration-Artifact-Family-v1
- LOCAL_WORKTREE_DIR: ../wt-WP-1-Structured-Collaboration-Artifact-Family-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Structured-Collaboration-Artifact-Family-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Structured-Collaboration-Artifact-Family-v1
- REMOTE_BACKUP_LIFECYCLE: TEMPORARY
<!-- WP backup branches may be deleted after Operator-approved cleanup; later dead links are non-blocking. -->
- BACKUP_PUSH_STATUS: REQUIRED_BEFORE_DESTRUCTIVE_OPS
- HEARTBEAT_INTERVAL_MINUTES: 15
<!-- Integer minutes; update runtime status/receipts on event boundaries and at this interval only while actively working. -->
- STALE_AFTER_MINUTES: 45
<!-- Integer minutes; heartbeat older than this threshold is stale. -->
- MAX_CODER_REVISION_CYCLES: 3
- MAX_VALIDATOR_REVIEW_CYCLES: 3
- MAX_RELAY_ESCALATION_CYCLES: 2
- WP_COMMUNICATION_DIR: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Artifact-Family-v1
- WP_THREAD_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Artifact-Family-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Artifact-Family-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: .GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Artifact-Family-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- USER_SIGNATURE: ilja120320262021
- PACKET_FORMAT_VERSION: 2026-03-11

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: WP Validator review

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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Structured-Collaboration-Artifact-Family-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [BIG_TECH] Atlassian Jira Issue Fields docs | 2026-03-12 | Retrieved: 2026-03-12T19:00:55Z | https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-fields/ | Why: validates the value of typed authority fields that drive multiple board and issue views without treating the board layout as the source of truth
  - [BIG_TECH] GitHub Projects roadmap layout docs | 2026-03-12 | Retrieved: 2026-03-12T19:00:55Z | https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-roadmap-layout | Why: shows multiple governed layouts rendered from the same underlying project records
  - [OSS_DOC] Backstage descriptor format docs | 2026-03-12 | Retrieved: 2026-03-12T19:00:55Z | https://backstage.io/docs/features/software-catalog/descriptor-format/ | Why: useful reference for stable base descriptors plus extensible metadata without overfitting everything into one universal schema
  - [GITHUB] Backstage repository | 2026-03-12 | Retrieved: 2026-03-12T19:00:55Z | https://github.com/backstage/backstage | Why: concrete OSS reference for catalog and projection architecture at repository scale
  - [PAPER] FocusLLM paper | 2024-08-21 | Retrieved: 2026-03-12T19:00:55Z | https://arxiv.org/abs/2408.11745 | Why: supports the summary-first, compaction-before-detail pattern that maps well to Handshake local-small-model ingestion
- RESEARCH_SYNTHESIS:
  - Handshake should keep one authoritative structured record family and let board, queue, and roadmap surfaces remain projections over that family.
  - The shared base envelope should stay intentionally small while project-specific fields move behind explicit extension boundaries.
  - Compact summaries should be treated as first-read artifacts for smaller models and operator triage, with canonical detail records and long note streams loaded only when needed.
  - External systems generally succeed when layout state is derived from typed records instead of turning UI position or prose into authority.
- GITHUB_PROJECT_DECISIONS:
  - backstage/backstage -> ADAPT (NONE)

## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_VERDICT: PASS
- SOURCE_SCAN_DECISIONS:
  - Atlassian Jira Issue Fields docs -> ADOPT (IN_THIS_WP)
  - GitHub Projects roadmap layout docs -> ADOPT (IN_THIS_WP)
  - Backstage descriptor format docs -> ADAPT (IN_THIS_WP)
  - Backstage repository -> REJECT (REJECT_DUPLICATE)
  - FocusLLM paper -> ADAPT (IN_THIS_WP)
- MATRIX_GROWTH_CANDIDATES:
  - Shared base envelope plus per-record bounded summary -> IN_THIS_WP (stub: NONE)
  - Canonical records plus view-specific projection files -> IN_THIS_WP (stub: NONE)
  - Base descriptor plus bounded extension surface -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep record identity, summary identity, and authority refs mechanically joinable so readers never reconstruct state from transcript order or Markdown prose.
  - Separate projection layout logic from canonical record mutation so board or queue re-layouts stay cheap and non-authoritative.
  - Bound summary payloads deliberately for local-small-model first reads and only hydrate longer detail when a workflow step truly requires it.

## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-MirrorSyncState
  - PRIM-MarkdownMirrorContractV1
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
- PRIMITIVES_EXPOSED:
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-MirrorSyncState
  - PRIM-MarkdownMirrorContractV1
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
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
  - Locus canonical packet emission plus durable summary pairing -> IN_THIS_WP (stub: NONE)
  - Locus canonical micro-task emission plus bounded execution summary -> IN_THIS_WP (stub: NONE)
  - Task Board projection files over canonical records -> IN_THIS_WP (stub: NONE)
  - Portable workflow-state fields on canonical work-packet records -> IN_THIS_WP (stub: NONE)
  - Portable workflow-state fields on canonical micro-task records -> IN_THIS_WP (stub: NONE)
  - Mirror-state plus authority refs on packet and task summaries -> IN_THIS_WP (stub: NONE)
  - Mailbox export convergence with shared collaboration envelope -> IN_THIS_WP (stub: NONE)
  - One parser surface across work packets, micro-tasks, and task-board rows -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE

## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: Canonical collaboration record persistence | SUBFEATURES: shared base-envelope alignment for work packets, micro-tasks, task-board rows, and mailbox exports | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry, PRIM-RoleMailboxIndexV1, PRIM-RoleMailboxThreadLineV1 | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus already owns tracked work state and is the right backend surface for the canonical artifact family rollout
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Canonical packet detail plus bounded summary pair | SUBFEATURES: `packet.json`, `summary.json`, note refs, mirror refs, and portable workflow-state fields | PRIMITIVES_FEATURES: PRIM-TrackedWorkPacket, PRIM-StructuredCollaborationSummaryV1, PRIM-MirrorSyncState, PRIM-MarkdownMirrorContractV1, PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is a first-class deliverable of the WP, not a downstream consumer concern
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Structured projection inventory and views | SUBFEATURES: `task_board/index.json`, `task_board/views/{view_id}.json`, stable row identity, and projection-safe mirror posture | PRIMITIVES_FEATURES: PRIM-TaskBoardEntry, PRIM-MirrorSyncState, PRIM-MarkdownMirrorContractV1, PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode | MECHANICAL: engine.archivist, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this WP should supply the backend projection artifacts that later viewer work depends on
  - PILLAR: MicroTask | CAPABILITY_SLICE: Canonical micro-task detail plus bounded summary pair | SUBFEATURES: per-micro-task `packet.json`, `summary.json`, and shared envelope alignment with work packets | PRIMITIVES_FEATURES: PRIM-TrackedMicroTask, PRIM-StructuredCollaborationSummaryV1, PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode | MECHANICAL: engine.archivist, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: micro-task state already exists in runtime structs and should be promoted into the canonical artifact family in this pass
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Summary-first local-small-model ingestion | SUBFEATURES: bounded summary payloads, stable references, blockers, next action, and explicit workflow-state fields | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationSummaryV1, PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: smaller models should be able to operate from summaries without reopening long Markdown or replaying mailbox threads

## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Work Packet canonical artifact emission | JobModel: WORKFLOW | Workflow: Locus work-packet persistence and sync | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-WP-001, FR-EVT-WP-002 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: authoritative work-packet records should be emitted from the existing backend work-tracking flow rather than from a UI-only export
  - Capability: Micro-Task canonical artifact emission | JobModel: WORKFLOW | Workflow: Locus micro-task registration and update flow | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-MT-001..006, FR-EVT-MT-001..017 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: micro-task records should align to the same base envelope and summary contract as work packets while preserving existing executor telemetry
  - Capability: Task Board structured projection export | JobModel: WORKFLOW | Workflow: task-board sync and projection rebuild | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-TB-001..003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: projection rows and view files should be generated from canonical state and remain queryable outside Markdown parsing
  - Capability: Role Mailbox structured export convergence | JobModel: WORKFLOW | Workflow: mailbox export and transcription-safe manifest update | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-GOV-MAILBOX-002, FR-EVT-GOV-MAILBOX-003 | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: mailbox export already exists and should be aligned to the shared envelope without regressing leak-safe export guarantees

## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Structured-Collaboration-Schema-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Markdown-Mirror-Sync-Drift-Guard-v1 -> KEEP_SEPARATE
  - WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1 -> KEEP_SEPARATE
  - WP-1-Project-Profile-Extension-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Project-Agnostic-Workflow-State-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Role-Mailbox-v1 -> KEEP_SEPARATE
  - WP-1-Locus-Phase1-Integration-Occupancy-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Locus-Phase1-Integration-Occupancy-v1)
  - src/backend/handshake_core/src/locus/task_board.rs -> PARTIAL (WP-1-Locus-Phase1-Integration-Occupancy-v1)
  - src/backend/handshake_core/src/role_mailbox.rs -> PARTIAL (WP-1-Role-Mailbox-v1)
  - src/backend/handshake_core/src/runtime_governance.rs -> PARTIAL (NONE)

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
- What: Implement the canonical structured collaboration artifact family for Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports using the v02.178 base envelope, bounded summary contract, and mirror-governance fields.
- Why: Current spec coverage is already strong, but the runtime still relies on partial structs, Markdown-only board sync, and mailbox-specific exports. This WP converts that gap into one reusable backend record family that later schema-registry, mirror-sync, and viewer work can build on safely.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
- OUT_OF_SCOPE:
  - frontend Dev Command Center viewer and layout work
  - schema-registry and project-profile extension registry hardening beyond the runtime fields needed for this implementation
  - standalone mirror-reconciliation controllers and overwrite-safe normalization policy
  - non-software project-profile packs beyond the shared base envelope and extension boundary

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
- Work Packet, Micro-Task, Task Board, and Role Mailbox runtime artifacts are emitted in a canonical structured family aligned to the v02.178 base envelope.
- Each canonical collaboration artifact family member exposes a bounded summary path or summary payload that smaller local models can consume first.
- Runtime artifact paths and serialization stay deterministic and preserve mirror-state plus authoritative-reference semantics.
- Existing mailbox leak-safety and current Locus/Task Board behavior do not regress while the new artifact family is added.

- PRIMITIVES_EXPOSED:
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxIndexV1
  - PRIM-RoleMailboxThreadLineV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-MirrorSyncState
  - PRIM-MarkdownMirrorContractV1
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.178.md (recorded_at: 2026-03-12T19:23:27.829Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.167]
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.15.5 Canonical structured collaboration artifact family [ADD v02.167]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.
- Prior packet history: NONE.
- Activation note: this official packet activates the prior stub `.GOV/task_packets/stubs/WP-1-Structured-Collaboration-Artifact-Family-v1.md` without changing scope.

## BOOTSTRAP
- FILES_TO_OPEN:
  - Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - TrackedWorkPacket
  - TrackedMicroTask
  - role_mailbox_export_v1
  - summary.json
  - packet.json
  - workflow_state_family
  - queue_reason_code
  - mirror_state
- RUN_COMMANDS:
  ```bash
rg -n "TrackedWorkPacket|TrackedMicroTask|role_mailbox_export_v1|workflow_state_family|queue_reason_code|mirror_state|summary.json|packet.json" src/backend/handshake_core
  cargo test -p handshake_core
  just gov-check
  ```
- RISK_MAP:
  - "base-envelope drift between record families" -> "shared readers and validators become unreliable"
  - "summary artifacts diverge from canonical detail" -> "local-small-model routing and operator triage become unsafe"
  - "mailbox export convergence regresses leak-safe behavior" -> "governance-critical data could be exposed incorrectly"
  - "task-board projection generation stays Markdown-only" -> "downstream viewer and layout packets remain blocked"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: server->storage
- SERVER_SOURCES_OF_TRUTH:
  - Locus tracked work state and Role Mailbox export source records remain authoritative; Markdown mirrors and board projections are derived views only.
- REQUIRED_PROVENANCE_FIELDS:
  - `schema_id`, `schema_version`, `record_id`, `record_kind`, `project_profile_kind`, `updated_at`, `mirror_state`, `authority_refs`, and `evidence_refs`
- VERIFICATION_PLAN:
  - Prove canonical artifacts are emitted from server-side authority, preserve the shared envelope, and match the signed refinement via `cargo test -p handshake_core`, `just gov-check`, and `just pre-work WP-1-Structured-Collaboration-Artifact-Family-v1`.
- ERROR_TAXONOMY_PLAN:
  - Distinguish stale mirror/projection drift, malformed envelope/schema drift, missing authority refs, and mailbox export regression.
- UI_GUARDRAILS:
  - N/A; no direct UI surface is in scope for this packet.
- VALIDATOR_ASSERTIONS:
  - Validator must prove the canonical artifact family is emitted from authoritative runtime state, not reconstructed from Markdown or layout position, and that the shared envelope fields remain present across packet, summary, board, and mailbox outputs.

## IMPLEMENTATION
- Added the shared structured-collaboration envelope and canonical artifact structs in `locus/types.rs` for work packets, micro-tasks, summaries, workflow-state fields, governed actions, and Markdown mirror contracts.
- Added task-board projection record/index/view structs plus stable lane/view helpers in `locus/task_board.rs`.
- Extended `RuntimeGovernancePaths` with deterministic canonical artifact paths for work packets, micro-tasks, task-board projections, and view files, plus coverage for the new path helpers.
- Wired locus operations in `workflows.rs` to emit `packet.json`, `summary.json`, note sidecars, task-board projection files, and shared authority/evidence/mirror metadata from authoritative runtime state.
- Tightened the workflow implementation with deterministic micro-task `updated_at` derivation and a workflow-local atomic-write lock so parallel runtime tests do not race on the same canonical artifact paths.
- Aligned role mailbox export thread lines, index, and manifest records to the shared envelope with explicit `record_kind`, `authority_refs`, and `evidence_refs`.
- `src/backend/handshake_core/src/api/role_mailbox.rs` required no code change because it already serves the exported mailbox index as parsed JSON.

## HYGIENE
- `cargo fmt` in `src/backend/handshake_core`: PASS
- `CARGO_TARGET_DIR=D:\hctarget cargo test -p handshake_core --no-run`: PASS
- `CARGO_TARGET_DIR=D:\hctarget cargo test -p handshake_core --test micro_task_executor_tests`: PASS
- `CARGO_TARGET_DIR=D:\hctarget cargo test -p handshake_core`: PASS
- `just gov-check`: PASS

## VALIDATION
- Mechanical manifest for audit. Records the coder-side `What` only; no validation verdict is claimed here.

### Manifest Entry 1: runtime_governance.rs
- **Target File**: `src/backend/handshake_core/src/runtime_governance.rs`
- **Start**: 13
- **End**: 396
- **Line Delta**: 162
- **Pre-SHA1**: `d2341a20c372789500925ba19097871637512d06`
- **Post-SHA1**: `b348f715467840ed0068dbbeddaa1145399476c9`
- **Change Summary**: Added canonical path helpers for work-packet, micro-task, and task-board projection artifact families plus path coverage tests.
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
- **Lint Results**: `cargo fmt` PASS
- **Artifacts**: `git diff --cached --unified=0 -- src/backend/handshake_core/src/runtime_governance.rs`; `just cor701-sha src/backend/handshake_core/src/runtime_governance.rs`
- **Timestamp**: `2026-03-13T08:42:44.7758130Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Runtime artifact locations remain deterministic and rooted under `.handshake/gov/`.

### Manifest Entry 2: locus/types.rs
- **Target File**: `src/backend/handshake_core/src/locus/types.rs`
- **Start**: 9
- **End**: 236
- **Line Delta**: 228
- **Pre-SHA1**: `97c5a28506a9fa8cad69a8180fe2af808dc7e335`
- **Post-SHA1**: `944462d1c363c20a075662360f6f3a7fa9302c1f`
- **Change Summary**: Added the shared collaboration envelope enums and canonical record structs for work packets, micro-tasks, and summaries.
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
- **Lint Results**: `cargo fmt` PASS
- **Artifacts**: `git diff --cached --unified=0 -- src/backend/handshake_core/src/locus/types.rs`; `just cor701-sha src/backend/handshake_core/src/locus/types.rs`
- **Timestamp**: `2026-03-13T08:42:44.7758130Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: This block establishes the reusable base envelope required by the packet refinement.

### Manifest Entry 3: locus/task_board.rs
- **Target File**: `src/backend/handshake_core/src/locus/task_board.rs`
- **Start**: 1
- **End**: 175
- **Line Delta**: 102
- **Pre-SHA1**: `d0191f5ca5ca233afef59714dd8de131452c3bde`
- **Post-SHA1**: `28f94f596f1cc0ba89d8a5db0417a88dddcbdd00`
- **Change Summary**: Added structured task-board projection records and stable lane/view identifiers.
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
- **Lint Results**: `cargo fmt` PASS
- **Artifacts**: `git diff --cached --unified=0 -- src/backend/handshake_core/src/locus/task_board.rs`; `just cor701-sha src/backend/handshake_core/src/locus/task_board.rs`
- **Timestamp**: `2026-03-13T08:42:44.7758130Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Projection rows stay derived from canonical state instead of Markdown layout.

### Manifest Entry 4: role_mailbox.rs
- **Target File**: `src/backend/handshake_core/src/role_mailbox.rs`
- **Start**: 1396
- **End**: 1520
- **Line Delta**: 35
- **Pre-SHA1**: `4725d88f3c99d55073f35ad950546fd0533a6cd5`
- **Post-SHA1**: `019957403e03efdc4546fbd372349c8bd66b38c2`
- **Change Summary**: Aligned mailbox export thread lines, index, and manifest with the shared collaboration envelope.
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
- **Lint Results**: `cargo fmt` PASS
- **Artifacts**: `git diff --cached --unified=0 -- src/backend/handshake_core/src/role_mailbox.rs`; `just cor701-sha src/backend/handshake_core/src/role_mailbox.rs`
- **Timestamp**: `2026-03-13T08:42:44.7758130Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Existing leak-safe export behavior stays covered by the unchanged mailbox tests.

### Manifest Entry 5: workflows.rs
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 49
- **End**: 8446
- **Line Delta**: 1357
- **Pre-SHA1**: `399602f44739988443d68570eabde15a32f45498`
- **Post-SHA1**: `8644dcaee5ec289c3ad790baf2a1364a371bce33`
- **Change Summary**: Added authoritative artifact emission for work packets, micro-tasks, and task-board projections, plus deterministic micro-task timestamps and serialized atomic writes.
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
- **Lint Results**: `cargo fmt` PASS
- **Artifacts**: `git diff --cached --unified=0 -- src/backend/handshake_core/src/workflows.rs`; `just cor701-sha src/backend/handshake_core/src/workflows.rs`
- **Timestamp**: `2026-03-13T08:42:44.7758130Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: The repair for parallel test races is limited to the workflow-side write wrapper used by these new structured artifact emissions.

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete; post-work passed; ready for validator review.
- What changed in this update:
  - Implemented the canonical structured collaboration artifact family across work packets, micro-tasks, task-board projections, and role-mailbox exports.
  - Added deterministic runtime artifact path helpers and wired authoritative locus operations to materialize those artifacts.
  - Repaired a parallel-test race in workflow artifact writes and verified the targeted regression test binary before rerunning the full crate test plan.
- Next step / handoff hint:
  - Wake the WP validator now.
  - Review the staged packet manifest plus the cargo and governance logs, then audit the emitted artifact-family logic in the five scoped backend files.

## EVIDENCE_MAPPING
- REQUIREMENT: "Work Packet, Micro-Task, Task Board, and Role Mailbox runtime artifacts are emitted in a canonical structured family aligned to the v02.178 base envelope."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:106`, `src/backend/handshake_core/src/locus/types.rs:135`, `src/backend/handshake_core/src/locus/types.rs:189`, `src/backend/handshake_core/src/locus/task_board.rs:26`, `src/backend/handshake_core/src/locus/task_board.rs:56`, `src/backend/handshake_core/src/role_mailbox.rs:1402`, `src/backend/handshake_core/src/role_mailbox.rs:1465`, `src/backend/handshake_core/src/role_mailbox.rs:1510`, `src/backend/handshake_core/src/workflows.rs:3742`
- REQUIREMENT: "Each canonical collaboration artifact family member exposes a bounded summary path or summary payload that smaller local models can consume first."
  - EVIDENCE: `src/backend/handshake_core/src/runtime_governance.rs:137`, `src/backend/handshake_core/src/runtime_governance.rs:195`, `src/backend/handshake_core/src/workflows.rs:3339`, `src/backend/handshake_core/src/workflows.rs:3423`, `src/backend/handshake_core/src/workflows.rs:3660`
- REQUIREMENT: "Runtime artifact paths and serialization stay deterministic and preserve mirror-state plus authoritative-reference semantics."
  - EVIDENCE: `src/backend/handshake_core/src/runtime_governance.rs:129`, `src/backend/handshake_core/src/runtime_governance.rs:213`, `src/backend/handshake_core/src/workflows.rs:3269`, `src/backend/handshake_core/src/workflows.rs:3497`, `src/backend/handshake_core/src/workflows.rs:3636`, `src/backend/handshake_core/src/workflows.rs:8439`, `src/backend/handshake_core/src/role_mailbox.rs:1415`
- REQUIREMENT: "Existing mailbox leak-safety and current Locus/Task Board behavior do not regress while the new artifact family is added."
  - EVIDENCE: `src/backend/handshake_core/tests/role_mailbox_tests.rs:32`, `src/backend/handshake_core/tests/role_mailbox_tests.rs:83`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:332`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:563`
- REQUIREMENT: "Handshake_Master_Spec_v02.178.md 2.3.15.5 Canonical structured collaboration artifact family [ADD v02.167]"
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:106`, `src/backend/handshake_core/src/locus/task_board.rs:26`, `src/backend/handshake_core/src/workflows.rs:3340`, `src/backend/handshake_core/src/workflows.rs:3664`

## EVIDENCE
- COMMAND: `CARGO_TARGET_DIR=D:\hctarget cargo test -p handshake_core --test micro_task_executor_tests`
- EXIT_CODE: 0
- LOG_PATH: `src/backend/handshake_core/.handshake/logs/WP-1-Structured-Collaboration-Artifact-Family-v1/cargo-test-micro-task-executor.log`
- LOG_SHA256: `3A6B819152AFEAAD9558DA2D666BEDB922A764BF9C5D164D81BFC5BCA53A35AA`
- PROOF_LINES:
  - `running 12 tests`
  - `test micro_task_executor_persists_locus_lifecycle_and_session_occupancy ... ok`
  - `test locus_bind_session_normalizes_and_deduplicates_session_ids ... ok`
  - `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.58s`

- COMMAND: `CARGO_TARGET_DIR=D:\hctarget cargo test -p handshake_core`
- EXIT_CODE: 0
- LOG_PATH: `src/backend/handshake_core/.handshake/logs/WP-1-Structured-Collaboration-Artifact-Family-v1/cargo-test-short-target.log`
- LOG_SHA256: `950DDF486025FCD460B0AF125A0120AE5BEC67113167032A0771CAF4BB825C48`
- PROOF_LINES:
  - `test result: ok. 202 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 56.65s`
  - `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.58s`
  - `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.87s`
  - `Doc-tests handshake_core`
  - `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

- COMMAND: `just gov-check`
- EXIT_CODE: 0
- LOG_PATH: `src/backend/handshake_core/.handshake/logs/WP-1-Structured-Collaboration-Artifact-Family-v1/gov-check.log`
- LOG_SHA256: `4EEA01DA12FD2BE04DA646F3EE9F77BC179C3B01A452E2574FF50938547983D5`
- PROOF_LINES:
  - `SPEC_CURRENT ok: Handshake_Master_Spec_v02.178.md`
  - `task-board-check ok`
  - `wp-communications-check ok`
  - `task-packet-claim-check ok`
  - `worktree-concurrency-check ok`

- COMMAND: `just post-work WP-1-Structured-Collaboration-Artifact-Family-v1`
- EXIT_CODE: 0
- LOG_PATH: `src/backend/handshake_core/.handshake/logs/WP-1-Structured-Collaboration-Artifact-Family-v1/post-work.log`
- LOG_SHA256: `6890C3E15F49C43B077AA00FDD721D17A2FC1C162F1393CF2E388108FEA0F2AE`
- PROOF_LINES:
  - `Post-work validation PASSED (deterministic manifest gate; not tests) with warnings`
  - `Warnings:`
  - `Working tree has unstaged changes; post-work validation uses STAGED changes only.`
  - `ROLE_MAILBOX_EXPORT_GATE PASS`
  - `RESULT: PASS`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
