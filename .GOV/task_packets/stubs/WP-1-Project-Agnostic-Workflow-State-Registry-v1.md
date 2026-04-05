# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.
- For any stub covering current-spec Phase 1 roadmap additions (`[ADD v<current>]`), `ROADMAP_ADD_COVERAGE` MUST enumerate the exact spec line numbers so governance checks can verify no additions were missed.

---

# Work Packet Stub: WP-1-Project-Agnostic-Workflow-State-Registry-v1

## STUB_METADATA
- WP_ID: WP-1-Project-Agnostic-Workflow-State-Registry-v1
- BASE_WP_ID: WP-1-Project-Agnostic-Workflow-State-Registry
- CREATED_AT: 2026-03-10T14:06:10.157Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry, WP-1-Project-Profile-Extension-Registry, WP-1-Governance-Workflow-Mirror
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.171.md 7.6.3 (Phase 1) -> [ADD v02.171] Project-agnostic workflow-state and governed-action contracts
- ROADMAP_ADD_COVERAGE: SPEC=v02.171; PHASE=7.6.3; LINES=46980
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.171.md 2.3.15.5 Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
  - Handshake_Master_Spec_v02.171.md 10.11.5.21 Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]
  - Handshake_Master_Spec_v02.171.md 10.11.5.17 Structured Work Records, Notes, and Collaboration Inbox [ADD v02.171]

## INTENT (DRAFT)
- What: Define and later implement the registry of shared workflow-state families, queue-reason codes, governed action descriptors, and project-profile workflow label overrides used by Work Packets, Micro-Tasks, Task Board rows, Role Mailbox-linked waits, and Dev Command Center queues.
- Why: Handshake now has structured collaboration records and typed views, but those surfaces still need one portable workflow vocabulary. Without a registry, each project kernel or board layout will invent incompatible status names, queue reasons, and allowed-action assumptions.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical registry entries for `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`.
  - Compatibility rules for project-profile relabeling without changing base workflow semantics.
  - Validation and linting expectations for Locus, Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox-linked queue state.
  - Dev Command Center projection requirements for queue grouping, label mapping, and governed next-action previews.
- OUT_OF_SCOPE:
  - Full implementation of future non-software project kernels.
  - Freeform board-layout experimentation that does not mutate canonical workflow state.
  - Replacing the broader structured-collaboration schema registry or Markdown mirror contracts.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center workflow-state inspector
  - Task Board lane-definition editor
  - Work Packet detail queue-state panel
  - Role Mailbox triage reason badges
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Workflow family badge | Type: status chip | Tooltip: Shows the portable base workflow state behind the current project label. | Notes: read-only in generic views
  - Control: Queue reason badge | Type: status chip | Tooltip: Explains why this item is waiting, blocked, routed, or ready. | Notes: visible before opening mirrors
  - Control: Allowed actions list | Type: expandable chip set | Tooltip: Shows governed actions currently legal for this record. | Notes: links to action preview
  - Control: Project-profile label mapping drawer | Type: drawer toggle | Tooltip: Explains how project-specific labels map onto base workflow semantics. | Notes: advanced operator view
- UI_STATES (empty/loading/error):
  - Unknown workflow family
  - Unknown queue reason
  - Action registry mismatch
  - Project-profile label override invalid
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Base workflow family
  - Queue reason
  - Governed next actions
  - Profile label override

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Jira workflow states and status categories
  - GitHub Projects field-driven workflows and automations
  - Linear issue states and triage vocabulary
  - OpenProject workflow transition matrices
  - multi-agent handoff and queue reason design
- CANDIDATE_SOURCES:
  - Source: Jira project statuses, categories, and workflows | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T13:50:00Z | URL: https://support.atlassian.com/jira-cloud-administration/docs/what-are-project-statuses-categories-and-workflows/ | Why: status-family layering and project-specific workflow labels
  - Source: Jira workflow rules | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T13:50:00Z | URL: https://support.atlassian.com/jira-cloud-administration/docs/use-workflow-rules/ | Why: explicit action transitions and rule-bound state changes
  - Source: GitHub Projects fields | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T13:50:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: field-driven records with multiple projections
  - Source: GitHub Projects built-in automations | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T13:50:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/automating-your-project/using-the-built-in-automations | Why: action semantics tied to explicit field changes
  - Source: Linear issue tracking | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T13:50:00Z | URL: https://linear.app/docs/issue-tracking | Why: compact workflow vocabulary and low-noise triage posture
  - Source: OpenProject work package workflows | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T13:50:00Z | URL: https://www.openproject.org/docs/system-admin-guide/work-packages/work-package-workflows/ | Why: role and type specific transition matrix without losing shared workflow structure
  - Source: LangChain handoffs | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T13:50:00Z | URL: https://docs.langchain.com/oss/python/langchain/multi-agent/handoffs | Why: explicit handoff and responsibility transfer semantics
  - Source: AutoGen handoffs | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T13:50:00Z | URL: https://microsoft.github.io/autogen/dev/user-guide/core-user-guide/design-patterns/handoffs.html | Why: typed transition posture for multi-agent collaboration

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: GitHub Projects fields | Pattern: one stable field model consumed by multiple layouts | Why: Handshake should keep workflow state in canonical records and let board or queue layouts remain downstream projections
  - Source: OpenProject workflows | Pattern: explicit transition matrix by role and work-item type | Why: aligns with governed action ids and project-profile-specific label mapping without losing shared semantics
- ADAPT:
  - Source: Jira status categories | Pattern: low-cardinality status families with project-specific labels | Why: Handshake needs portable workflow families but richer project-profile wording
  - Source: Linear issue tracking | Pattern: compact visible triage states | Why: Handshake should keep the base vocabulary small enough for local small models and operators
- REJECT:
  - Source: board-column-as-authority patterns | Pattern: treating lane position or board column name as canonical state | Why: conflicts with structured state, local-small-model routing, and project-agnostic governance

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - workflow state registry board projection json task system
  - issue workflow status reasons queue actions registry
- MATCHED_PROJECTS:
  - Repo: plane-so/plane | Intent: UI_PATTERN | Decision hint: ADAPT | Impact hint: UI_ENRICHMENT | Notes: useful for lane and issue-state presentation, but canonical workflow semantics should remain record-first
  - Repo: OpenHands/OpenHands | Intent: ADJACENT | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: useful for agent handoff and session-state projection, but needs stronger state registry discipline

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Governance kernel | STATUS: TOUCHED | NOTES: one portable workflow vocabulary reduces repository-specific assumptions in later project kernels | Stub follow-up: THIS_STUB
  - PILLAR: Dev Command Center | STATUS: TOUCHED | NOTES: queue grouping and action previews now need a stable backend vocabulary | Stub follow-up: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1
  - PILLAR: Local small model execution | STATUS: TOUCHED | NOTES: smaller models need bounded workflow families and queue reasons instead of prose or board labels | Stub follow-up: THIS_STUB

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: Locus work-tracking queue projection | ENGINE_ID: locus.queue_projection | STATUS: TOUCHED | NOTES: publishes base workflow-state family, queue reason, and allowed action ids | Stub follow-up: THIS_STUB
  - ENGINE: Dev Command Center layout projection | ENGINE_ID: dev_command_center.layout_projection | STATUS: TOUCHED | NOTES: consumes portable workflow state and profile label overrides | Stub follow-up: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1
  - ENGINE: Role Mailbox triage | ENGINE_ID: role_mailbox.triage | STATUS: TOUCHED | NOTES: contributes mailbox-linked queue reasons without becoming state authority | Stub follow-up: THIS_STUB

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-WorkflowStateFamily
  - PRIM-WorkflowQueueReasonCode
  - PRIM-GovernedActionDescriptorV1
  - PRIM-ProjectProfileWorkflowExtensionV1
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-LOCUS-WORK-TRACKING -> FEAT-DEV-COMMAND-CENTER | ROI: H | Effort: M | Notes: shared workflow-state registry powers operator queues and project-profile relabeling
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-MICRO-TASK-EXECUTOR | ROI: H | Effort: M | Notes: mailbox-linked waits become explicit queue reasons instead of generic blocked state
  - Edge: FEAT-TASK-BOARD -> FEAT-WORK-PACKET-SYSTEM | ROI: H | Effort: M | Notes: board labels stay derived from packet workflow semantics

## FORCE_MULTIPLIER_HYPOTHESES (DRAFT)
- CANDIDATES:
  - Combo: Portable workflow-state registry | Pillars: Governance kernel, Dev Command Center, local small model execution | Mechanical: locus.queue_projection, dev_command_center.layout_projection, role_mailbox.triage | Primitives/Features: PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, FEAT-LOCUS-WORK-TRACKING, FEAT-DEV-COMMAND-CENTER | Resolution hint: IN_THIS_STUB | Notes: establishes one reusable workflow vocabulary before future project-kernel expansion

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: NONE | Resolution hint: KEEP_SEPARATE | Notes: defines base record shape, but not workflow-state and action vocabulary
  - Artifact: WP-1-Project-Profile-Extension-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: profile extension registry owns specialization, while this stub owns the portable workflow semantics under those extensions
  - Artifact: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: layout registry consumes workflow-state vocabulary rather than defining it
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: WP-1-Dev-Command-Center-MVP-v1 | Status: VALIDATED | Intent: PARTIAL | PrimitiveIndex: MISSING | Matrix: PARTIAL | UI: PARTIAL | CodeReality: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: existing Dev Command Center work does not yet lock a portable workflow-state registry
- CODE_REALITY_HINTS:
  - Path: src/backend/handshake_core/src/locus/types.rs | Covers: execution | Notes: likely future home for shared workflow-state enums or normalized persisted values
  - Path: src/backend/handshake_core/src/locus/task_board.rs | Covers: combo | Notes: derived board projections should consume workflow-state registry values
  - Path: src/backend/handshake_core/src/role_mailbox.rs | Covers: combo | Notes: mailbox-linked waits need explicit queue-reason contribution rules

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - FEAT-LOCUS-WORK-TRACKING, FEAT-MICRO-TASK-EXECUTOR, FEAT-ROLE-MAILBOX, FEAT-TASK-BOARD, FEAT-WORK-PACKET-SYSTEM, and FEAT-DEV-COMMAND-CENTER stay aligned on shared workflow-state vocabulary ownership
- PRIMITIVE_INDEX:
  - Add PRIM-WorkflowStateFamily, PRIM-WorkflowQueueReasonCode, PRIM-GovernedActionDescriptorV1, and PRIM-ProjectProfileWorkflowExtensionV1
- UI_GUIDANCE:
  - Dev Command Center, Role Mailbox, Task Board, and Work Packet viewers expose base workflow-state families, queue-reason codes, and allowed actions before project-profile labels
- INTERACTION_MATRIX:
  - Add Locus Work Tracking -> Dev Command Center registry-driven queue projection and deepen mailbox/task-board workflow-state interactions

## ACCEPTANCE_CRITERIA (DRAFT)
- One shared workflow-state family exists for Work Packets, Micro-Tasks, Task Board rows, Role Mailbox-linked waits, and Dev Command Center queues.
- Queue-reason codes are explicit and machine-readable instead of implicit in board labels, thread order, or prose.
- Governed action descriptors explain which actions are legal for a record before a board move, review request, escalation, or reroute is offered.
- Project-profile label overrides can rename workflow states for a project without changing the underlying family, queue reason, or allowed action ids.
- Local small model routing can derive readiness from workflow-state family and queue-reason code without loading long-form Markdown mirrors.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the structured collaboration schema registry, project-profile extension registry, and Dev Command Center layout projection registry.
- Future project-kernel refresh, portable board layouts, and local-small-model task routing remain weaker until this registry exists.

## RISKS / UNKNOWNs (DRAFT)
- Too many workflow families or reason codes would make the registry too noisy for operators and smaller models.
- If project-profile label overrides are allowed to change semantics instead of labels only, portability collapses.
- If mailbox waits or board labels backdoor canonical state, the registry becomes advisory instead of authoritative.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Project-Agnostic-Workflow-State-Registry-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Project-Agnostic-Workflow-State-Registry-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
