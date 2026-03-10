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

# Work Packet Stub: WP-1-Workflow-Transition-Automation-Registry-v1

## STUB_METADATA
- WP_ID: WP-1-Workflow-Transition-Automation-Registry-v1
- BASE_WP_ID: WP-1-Workflow-Transition-Automation-Registry
- CREATED_AT: 2026-03-10T16:10:41.644Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Project-Agnostic-Workflow-State-Registry, WP-1-Dev-Command-Center-Layout-Projection-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.172.md 7.6.3 (Phase 1) -> [ADD v02.172] Workflow transition matrix, queue automation, and executor eligibility contracts
- ROADMAP_ADD_COVERAGE: SPEC=v02.172; PHASE=7.6.3; LINES=47071
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.172.md 2.3.15.6 Workflow transition matrix, queue automation, and executor eligibility [ADD v02.172]
  - Handshake_Master_Spec_v02.172.md 10.11.5.22 Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]
  - Handshake_Master_Spec_v02.172.md 10.11.5.17 Structured Work Records, Notes, and Collaboration Inbox [ADD v02.172]

## INTENT (DRAFT)
- What: Define and later implement the registry of `WorkflowTransitionRuleV1`, `QueueAutomationRuleV1`, and `ExecutorEligibilityPolicyV1` records that govern lawful state changes across Work Packets, Micro-Tasks, Task Board projections, Role Mailbox-linked waits, and Dev Command Center action previews.
- Why: Handshake now has canonical structured records, shared workflow-state families, and typed Dev Command Center views, but it still needs one portable transition-and-automation law so local small models, cloud models, reviewers, validators, and operators can mutate work only through inspectable, approval-aware transitions.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical registry entries for allowed transition edges, transition-triggering governed actions, queue automation triggers, and executor eligibility policies.
  - Portable automation rules for mailbox replies, dependency clearance, validation outcomes, retry scheduling, approval decisions, and escalation acknowledgements.
  - Dev Command Center action-preview requirements that explain whether an action is automatic, approval-bound, actor-ineligible, or ready.
  - Validation and linting expectations for Locus, Micro-Task Executor, Task Board projections, Work Packet views, and Role Mailbox-linked queue changes.
- OUT_OF_SCOPE:
  - Full project-profile-specific board or interface design beyond the transition metadata needed for previews.
  - Replacing the broader workflow-state family registry or the structured-collaboration schema registry.
  - Freeform heuristics that derive transition legality from lane names, thread order, or prose-only notes.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center transition preview drawer
  - Work Packet detail transition matrix preview
  - Micro-Task execution queue eligibility inspector
  - Task Board lane-move legality preview
  - Role Mailbox automation-trigger explainer
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Transition rule badge | Type: status chip | Tooltip: Shows the rule that makes the proposed move legal or illegal. | Notes: opens rule drilldown
  - Control: Automation trigger badge | Type: status chip | Tooltip: Explains which event can move the item automatically. | Notes: visible before auto-queue moves
  - Control: Eligible actor list | Type: expandable chip set | Tooltip: Shows which actor kinds may execute the next legal action. | Notes: distinguishes local small model, cloud model, reviewer, validator, and operator
  - Control: Approval boundary warning | Type: inline banner | Tooltip: Shows when automation stops because human approval is required. | Notes: blocks silent state changes
- UI_STATES (empty/loading/error):
  - No lawful transition available
  - Automation rule missing
  - Actor ineligible
  - Approval boundary reached
  - Transition registry mismatch
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Allowed transition
  - Automatic when trigger fires
  - Approval required before moving
  - Eligible executor kinds

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Jira workflow transitions actor permissions automation
  - GitHub Projects automation fields workflow routing
  - Prefect automations trigger deployment queue
  - OpenProject transition matrix roles work package
  - multi-agent handoffs explicit transition eligibility
- CANDIDATE_SOURCES:
  - Source: Jira project statuses, categories, and workflows | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T16:00:00Z | URL: https://support.atlassian.com/jira-cloud-administration/docs/what-are-project-statuses-categories-and-workflows/ | Why: shared workflow families plus project-specific workflow labels
  - Source: Jira workflow rules | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T16:00:00Z | URL: https://support.atlassian.com/jira-cloud-administration/docs/use-workflow-rules/ | Why: explicit action-bound transitions, validators, and post-functions
  - Source: GitHub Projects fields | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T16:00:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: stable field model with multiple projections
  - Source: GitHub Projects built-in automations | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T16:00:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/automating-your-project/using-the-built-in-automations | Why: queue and state updates driven by explicit automation conditions
  - Source: Prefect automations | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T16:00:00Z | URL: https://docs.prefect.io/v3/concepts/automations | Why: trigger-action automation patterns with queue and run-state semantics
  - Source: OpenProject work package workflows | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T16:00:00Z | URL: https://www.openproject.org/docs/system-admin-guide/work-packages/work-package-workflows/ | Why: role-sensitive transition matrices that still preserve shared work-item structure
  - Source: LangChain handoffs | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T16:00:00Z | URL: https://docs.langchain.com/oss/python/langchain/multi-agent/handoffs | Why: explicit handoff semantics between actors
  - Source: AutoGen handoffs | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T16:00:00Z | URL: https://microsoft.github.io/autogen/dev/user-guide/core-user-guide/design-patterns/handoffs.html | Why: typed multi-agent transfer and control-boundary rules
  - Source: MetaGPT: Meta Programming for Multi-Agent Collaborative Framework | Kind: UNIVERSITY|PAPER | Date: 2023-08-01 | Retrieved: 2026-03-10T16:00:00Z | URL: https://arxiv.org/abs/2308.00352 | Why: explicit role workflow, handoff discipline, and structured collaborative state

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: OpenProject work package workflows | Pattern: explicit transition matrix by role and work-item type | Why: aligns with Handshake actor eligibility and project-profile extensions without letting board lanes become authority
  - Source: GitHub Projects built-in automations | Pattern: field-driven automation rules over canonical state | Why: Handshake queue moves should be explicit trigger-to-field mutations, not view heuristics
- ADAPT:
  - Source: Jira workflow rules | Pattern: rule-bound transitions with validators and post-functions | Why: Handshake needs lighter but explicit transition legality, automation triggers, and approval boundaries
  - Source: LangChain handoffs | Pattern: actor handoff as an explicit state change | Why: local small model, cloud model, reviewer, and operator handoff should reuse one governed transition contract
- REJECT:
  - Source: lane-order or thread-order authority patterns | Pattern: infer state change legality from board position or mailbox chronology | Why: conflicts with structured state, auditability, and portable project kernels

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - workflow transition matrix queue automation typed board state
  - agent task handoff executor eligibility registry
- MATCHED_PROJECTS:
  - Repo: OpenHands/OpenHands | Intent: ADJACENT | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: useful for explicit agent lifecycle and handoff posture, but needs stricter registry-backed transition law
  - Repo: plane-so/plane | Intent: UI_PATTERN | Decision hint: ADAPT | Impact hint: UI_ENRICHMENT | Notes: useful for queue and board presentation, but workflow legality must remain field-driven
  - Repo: prefecthq/prefect | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: useful for automation triggers and run-state transitions

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Governance kernel | STATUS: TOUCHED | NOTES: one portable transition law reduces repository-specific workflow assumptions in later project kernels | Stub follow-up: THIS_STUB
  - PILLAR: Dev Command Center | STATUS: TOUCHED | NOTES: action previews and queue regrouping need explicit legality and eligibility semantics | Stub follow-up: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1
  - PILLAR: Local small model execution | STATUS: TOUCHED | NOTES: smaller models need exact eligibility and automation posture rather than prose or lane inference | Stub follow-up: THIS_STUB

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: Locus transition registry | ENGINE_ID: locus.transition_registry | STATUS: TOUCHED | NOTES: publishes transition rules, queue automation posture, and executor eligibility into tracked work state | Stub follow-up: THIS_STUB
  - ENGINE: Dev Command Center action preview | ENGINE_ID: dev_command_center.action_preview | STATUS: TOUCHED | NOTES: consumes registry entries to explain legality before a state mutation | Stub follow-up: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1
  - ENGINE: Role Mailbox automation bridge | ENGINE_ID: role_mailbox.automation_bridge | STATUS: TOUCHED | NOTES: mailbox events can trigger queue changes only through explicit automation rules | Stub follow-up: THIS_STUB

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-WorkflowTransitionRuleV1
  - PRIM-QueueAutomationRuleV1
  - PRIM-ExecutorEligibilityPolicyV1
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-LOCUS-WORK-TRACKING -> FEAT-MICRO-TASK-EXECUTOR | ROI: H | Effort: M | Notes: transition rules and executor eligibility should gate local small model execution
  - Edge: FEAT-DEV-COMMAND-CENTER -> FEAT-WORK-PACKET-SYSTEM | ROI: H | Effort: M | Notes: action previews should show transition legality before routing, review, or approval actions
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-WORK-PACKET-SYSTEM | ROI: H | Effort: M | Notes: mailbox replies and expiries should drive queue posture only through explicit automation rules

## FORCE_MULTIPLIER_HYPOTHESES (DRAFT)
- CANDIDATES:
  - Combo: portable transition-and-automation law | Pillars: Governance kernel, Dev Command Center, local small model execution | Mechanical: locus.transition_registry, dev_command_center.action_preview, role_mailbox.automation_bridge | Primitives/Features: PRIM-WorkflowTransitionRuleV1, PRIM-QueueAutomationRuleV1, PRIM-ExecutorEligibilityPolicyV1, FEAT-LOCUS-WORK-TRACKING, FEAT-MICRO-TASK-EXECUTOR, FEAT-DEV-COMMAND-CENTER | Resolution hint: IN_THIS_STUB | Notes: gives every later board, queue, and handoff surface one lawful action model

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: WP-1-Project-Agnostic-Workflow-State-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: owns shared workflow-state families and allowed actions, but not transition legality or automation triggers
  - Artifact: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: layout registry should consume transition metadata rather than define it
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: WP-1-Dev-Command-Center-MVP-v1 | Status: VALIDATED | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | CodeReality: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: Dev Command Center exists, but transition legality and executor eligibility are not yet codified as one reusable registry
- CODE_REALITY_HINTS:
  - Path: src/backend/handshake_core/src/locus/types.rs | Covers: primitive | Notes: likely future home for persisted transition and eligibility enums or structs
  - Path: src/backend/handshake_core/src/locus/task_board.rs | Covers: combo | Notes: lane regrouping should consume transition legality instead of defining it
  - Path: src/backend/handshake_core/src/role_mailbox.rs | Covers: execution | Notes: mailbox replies and expiries need explicit automation-trigger mapping

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - FEAT-LOCUS-WORK-TRACKING, FEAT-MICRO-TASK-EXECUTOR, FEAT-ROLE-MAILBOX, FEAT-TASK-BOARD, FEAT-WORK-PACKET-SYSTEM, and FEAT-DEV-COMMAND-CENTER align on transition rules, queue automation rules, and executor eligibility policies
- PRIMITIVE_INDEX:
  - Add PRIM-WorkflowTransitionRuleV1, PRIM-QueueAutomationRuleV1, and PRIM-ExecutorEligibilityPolicyV1
- UI_GUIDANCE:
  - Dev Command Center, Role Mailbox, Task Board, and Work Packet viewers expose transition legality, automation triggers, and actor eligibility before state changes
- INTERACTION_MATRIX:
  - Deepen Dev Command Center, Task Board, Work Packet, Role Mailbox, and Locus interactions for transition and automation previews; add Locus Work Tracking -> Micro-Task Executor executor-eligibility edge

## ACCEPTANCE_CRITERIA (DRAFT)
- Every canonical workflow state mutation has an explicit transition rule, governed action source, and eligible actor set.
- Queue automation triggers are explicit, machine-readable, and cannot silently cross approval boundaries.
- Role Mailbox replies, expiries, and escalation acknowledgements can influence queues only through explicit automation rules tied to stable identifiers.
- Dev Command Center previews explain whether a move is view-only, automatic, actor-ineligible, or lawful before authoritative state changes occur.
- Local small model routing derives from workflow family, queue reason, transition legality, and executor eligibility without parsing long-form Markdown.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the project-agnostic workflow-state registry and Dev Command Center layout projection registry.
- Portable queue automation remains weaker until Locus, Role Mailbox, Task Board, and Micro-Task execution all consume one shared transition-and-eligibility registry.

## RISKS / UNKNOWNs (DRAFT)
- Too many transition rules or automation triggers would make the registry noisy for operators and smaller models.
- If approval boundaries are not encoded directly in transition rules, automation may silently overreach.
- If mailbox or board views are allowed to invent legal moves, the registry becomes advisory instead of authoritative.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Workflow-Transition-Automation-Registry-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Workflow-Transition-Automation-Registry-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
