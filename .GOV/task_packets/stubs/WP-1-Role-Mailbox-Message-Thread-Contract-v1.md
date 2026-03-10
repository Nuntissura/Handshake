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

# Work Packet Stub: WP-1-Role-Mailbox-Message-Thread-Contract-v1

## STUB_METADATA
- WP_ID: WP-1-Role-Mailbox-Message-Thread-Contract-v1
- BASE_WP_ID: WP-1-Role-Mailbox-Message-Thread-Contract
- CREATED_AT: 2026-03-10T17:08:16.615Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Role-Mailbox, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Project-Agnostic-Workflow-State-Registry, WP-1-Workflow-Transition-Automation-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.173.md 7.6.3 (Phase 1) -> [ADD v02.173] Role Mailbox message contract, thread lifecycle, and authority boundary
- ROADMAP_ADD_COVERAGE: SPEC=v02.173; PHASE=7.6.3; LINES=47158
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.173.md 2.3.15 Role Mailbox message contract, thread lifecycle, and authority boundary [ADD v02.173]
  - Handshake_Master_Spec_v02.173.md 2.6.8.10.3 Thread Lifecycle, Allowed Responses, and Authority Boundary (Normative) [ADD v02.173]
  - Handshake_Master_Spec_v02.173.md 10.11.5.23 Role Mailbox Thread Lifecycle and Action Requests [ADD v02.173]

## INTENT (DRAFT)
- What: Define and later implement the typed Role Mailbox thread-lifecycle, message-delivery, allowed-response, and action-request contract that governs asynchronous collaboration across Work Packets, Micro-Tasks, Locus Work Tracking, Task Board projections, and Dev Command Center inbox triage.
- Why: Handshake now has structured collaboration artifacts, portable workflow state, and transition law, but it still needs one durable mailbox contract so local small models, cloud models, operators, reviewers, and overseers can collaborate asynchronously without transcript order becoming authority.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical `RoleMailboxThreadLifecycleState`, `RoleMailboxMessageDeliveryState`, `RoleMailboxAllowedResponse`, and `RoleMailboxActionRequestV1` contracts.
  - Thread and message schemas for due posture, snooze posture, dead-letter posture, linked record identifiers, and governed action candidates.
  - Mailbox-local versus governed-action boundaries for acknowledge, snooze, reply, escalate, delegate, resolve, and transcription-request flows.
  - Typed Micro-Task collaboration message families for request, feedback, verification, escalation, and completion reporting.
  - Dev Command Center triage requirements for thread-lifecycle badges, action-request previews, and dead-letter handling.
- OUT_OF_SCOPE:
  - External email or generic task-intake mailbox systems.
  - Full cross-channel intake normalization from Slack, email, or Stage capture.
  - Rich board design beyond the inbox and queue fields needed for lawful triage.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center mailbox action-request queue
  - Role Mailbox triage row and thread inspector
  - Work Packet handoff thread drawer
  - Micro-Task verification and escalation inbox view
  - Dead-letter and expiry remediation queue
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Thread lifecycle badge | Type: status chip | Tooltip: Shows whether the thread is open, awaiting response, escalated, resolved, expired, or archived. | Notes: opens thread-state timeline
  - Control: Delivery state badge | Type: status chip | Tooltip: Shows whether the latest message is queued, delivered, acknowledged, replied, ignored, failed, or dead-lettered. | Notes: mailbox-local only
  - Control: Allowed response strip | Type: segmented actions | Tooltip: Shows which replies are legal for the current action request. | Notes: must separate mailbox-local from governed actions
  - Control: Due posture badge | Type: due-date chip | Tooltip: Shows due time, expiry, or snooze state. | Notes: visible in queue rows
  - Control: Governed action preview | Type: side panel | Tooltip: Explains whether the selected mailbox action is mailbox-local, transcription-only, or mutates a linked record. | Notes: required before execution
- UI_STATES (empty/loading/error):
  - No pending mailbox actions
  - Awaiting linked authority
  - Expired but not escalated
  - Dead-letter remediation required
  - Action request malformed
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Mailbox-only action
  - Requires governed action
  - Requires transcription
  - Dead-lettered; linked work unchanged
  - Awaiting response from linked role

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - OpenClaw session messaging gateway collaboration
  - TinyClaw team chat queue dead letter dashboard
  - Anthropic Claude team project sharing subagents
  - LangGraph agent inbox action request allowed responses
  - Slack message metadata workflow actions
  - Hugging Face smolagents managed agents memory collaboration
- CANDIDATE_SOURCES:
  - Source: OpenClaw repository | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://github.com/openclaw/openclaw | Why: multi-channel control plane, session routing, retries, and unified gateway posture.
  - Source: TinyClaw repository | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://github.com/TinyAGI/tinyclaw | Why: persistent team chat rooms, queue discipline, retries, dead-letter handling, and dashboarded async collaboration.
  - Source: Anthropic Team plan | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://support.anthropic.com/en/articles/9266767-what-is-the-team-plan | Why: team-shared workspace and collaboration boundary patterns.
  - Source: Anthropic project visibility and sharing | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://support.claude.com/en/articles/9519189-manage-project-visibility-and-sharing | Why: project-scoped sharing and explicit visibility boundaries.
  - Source: Claude Code subagents | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://code.claude.com/docs/en/sub-agents | Why: shared agent definitions checked into source control and reused by a team.
  - Source: LangGraph Agent Inbox | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://github.com/langchain-ai/agent-inbox | Why: typed `action_request` plus allowed responses instead of freeform reply parsing.
  - Source: MCP Agent Mail | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://github.com/Dicklesworthstone/mcp_agent_mail | Why: git-backed inboxes, searchable threads, leases, and unified inbox patterns.
  - Source: Agent Hub MCP | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://github.com/gilbarbara/agent-hub-mcp | Why: concise typed message taxonomy for collaboration between agents.
  - Source: Slack message metadata | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://docs.slack.dev/messaging/message-metadata/ | Why: structured metadata attached to human-readable messages.
  - Source: smolagents managed agents and memory | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T17:00:00Z | URL: https://huggingface.co/docs/smolagents/en/reference/agents | Why: managed-agent delegation, memory posture, and compact summaries for smaller models.
  - Source: Towards a Science of Scaling Agent Systems | Kind: BIG_TECH | Date: 2025-12-11 | Retrieved: 2026-03-10T17:00:00Z | URL: https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/ | Why: warning that multi-agent coordination must stay explicit and bounded to avoid overhead and degraded performance.

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: LangGraph Agent Inbox | Pattern: typed action request plus allowed responses | Why: Handshake Role Mailbox should expose legal mailbox responses explicitly instead of inferring them from prose.
  - Source: TinyClaw | Pattern: persistent chat rooms plus retry and dead-letter semantics | Why: thread lifecycle needs expiry, escalation, and dead-letter posture instead of only unread or archived.
  - Source: Anthropic project sharing and subagents | Pattern: project-scoped shared collaboration definitions | Why: shared mailbox handlers, prompts, or responder rules should remain source-controlled and scoped.
- ADAPT:
  - Source: OpenClaw | Pattern: unified gateway and session-routing posture | Why: useful for routing and announce-back flows, but Handshake must keep Locus and Work Packets authoritative.
  - Source: Slack metadata | Pattern: metadata attached to human-readable messages | Why: mailbox rows need typed action metadata while preserving readable note text.
  - Source: smolagents managed agents | Pattern: summary-first delegation for smaller models | Why: mailbox summaries should be ingestible without loading full threads.
- REJECT:
  - Source: transcript-order authority patterns | Pattern: infer work status from the latest visible reply | Why: conflicts with Handshake structured state and governed transitions.

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - role inbox action request thread lifecycle dead letter agent collaboration
  - agent mailbox git backed thread metadata queue retry dead letter
- MATCHED_PROJECTS:
  - Repo: openclaw/openclaw | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: useful for session and channel routing, but not as a source of work-state authority.
  - Repo: TinyAGI/tinyclaw | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: strongest queue, dead-letter, and team-chat pattern match for Handshake mailbox work.
  - Repo: langchain-ai/agent-inbox | Intent: SPEC_PATTERN | Decision hint: ADOPT | Impact hint: SPEC_UPDATE | Notes: best direct analog for typed action requests with allowed replies.
  - Repo: Dicklesworthstone/mcp_agent_mail | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: useful for git-backed searchable inboxes and thread inventory design.

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Human Collaboration | STATUS: TOUCHED | NOTES: one mailbox contract can unify handoff, escalation, announce-back, and reviewer/operator collaboration without turning chat into authority | Stub follow-up: THIS_STUB
  - PILLAR: Local small model execution | STATUS: TOUCHED | NOTES: compact mailbox summaries and explicit allowed responses reduce context cost and ambiguity for smaller models | Stub follow-up: THIS_STUB
  - PILLAR: Governance kernel | STATUS: TOUCHED | NOTES: mailbox-local versus governed-action boundaries keep project-agnostic workflow kernels portable | Stub follow-up: THIS_STUB

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: Role Mailbox thread registry | ENGINE_ID: role_mailbox.thread_registry | STATUS: TOUCHED | NOTES: owns thread lifecycle, delivery state, due posture, and dead-letter semantics | Stub follow-up: THIS_STUB
  - ENGINE: Role Mailbox action router | ENGINE_ID: role_mailbox.action_router | STATUS: TOUCHED | NOTES: distinguishes mailbox-local actions from governed linked-record actions | Stub follow-up: THIS_STUB
  - ENGINE: Dev Command Center inbox triage | ENGINE_ID: dev_command_center.mailbox_triage | STATUS: TOUCHED | NOTES: consumes typed mailbox state and previews mailbox-local versus governed actions | Stub follow-up: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1
  - ENGINE: Locus linked-action authority | ENGINE_ID: locus.linked_mailbox_action | STATUS: TOUCHED | NOTES: remains the authority for packet/task state even when the trigger originates in Role Mailbox | Stub follow-up: THIS_STUB

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-RoleMailboxThreadLifecycleState
  - PRIM-RoleMailboxMessageDeliveryState
  - PRIM-RoleMailboxAllowedResponse
  - PRIM-RoleMailboxActionRequestV1
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-LOCUS-WORK-TRACKING | ROI: H | Effort: M | Notes: mailbox action requests must resolve through Locus-backed authority rather than thread chronology.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-WORK-PACKET-SYSTEM | ROI: H | Effort: M | Notes: handoff and review threads should carry bounded action envelopes and explicit authority boundaries.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-MICRO-TASK-EXECUTOR | ROI: H | Effort: M | Notes: Micro-Task request, feedback, verification, escalation, and completion report messages should remain distinct message families.

## FORCE_MULTIPLIER_HYPOTHESES (DRAFT)
- CANDIDATES:
  - Combo: typed mailbox contract plus Locus authority boundary | Pillars: Human Collaboration, Governance kernel, Local small model execution | Mechanical: role_mailbox.thread_registry, role_mailbox.action_router, locus.linked_mailbox_action | Primitives/Features: PRIM-RoleMailboxThreadLifecycleState, PRIM-RoleMailboxMessageDeliveryState, PRIM-RoleMailboxAllowedResponse, PRIM-RoleMailboxActionRequestV1, FEAT-ROLE-MAILBOX, FEAT-LOCUS-WORK-TRACKING, FEAT-WORK-PACKET-SYSTEM, FEAT-MICRO-TASK-EXECUTOR | Resolution hint: IN_THIS_STUB | Notes: gives every future inbox, board, and queue surface the same lawful async collaboration contract.

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: WP-1-Role-Mailbox-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: original mailbox packet owns storage and export, but not the newer typed thread-lifecycle and action-request contract.
  - Artifact: WP-1-Workflow-Transition-Automation-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: transition law stays separate, but mailbox actions must route through it.
  - Artifact: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: triage layout should consume mailbox contract metadata instead of defining it.
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: WP-1-Role-Mailbox-v1 | Status: VALIDATED | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | CodeReality: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: storage, export, and recorder behavior exist, but typed lifecycle and action routing still need a dedicated contract pass.
- CODE_REALITY_HINTS:
  - Path: src/backend/handshake_core/src/role_mailbox.rs | Covers: execution | Notes: likely future home for thread-lifecycle, delivery-state, and action-request structs.
  - Path: src/backend/handshake_core/src/api/role_mailbox.rs | Covers: combo | Notes: triage and quick actions should expose mailbox-local versus governed-action previews.
  - Path: src/backend/handshake_core/src/locus/types.rs | Covers: authority | Notes: linked mailbox requests must resolve to governed Locus state changes rather than direct mailbox mutation.

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - FEAT-ROLE-MAILBOX, FEAT-LOCUS-WORK-TRACKING, FEAT-MICRO-TASK-EXECUTOR, FEAT-TASK-BOARD, FEAT-WORK-PACKET-SYSTEM, and FEAT-DEV-COMMAND-CENTER align on typed mailbox thread state, allowed responses, and authority boundaries.
- PRIMITIVE_INDEX:
  - Add PRIM-RoleMailboxThreadLifecycleState, PRIM-RoleMailboxMessageDeliveryState, PRIM-RoleMailboxAllowedResponse, and PRIM-RoleMailboxActionRequestV1.
- UI_GUIDANCE:
  - Dev Command Center and Role Mailbox triage views expose mailbox-local versus governed actions, dead-letter posture, and due or snooze state.
- INTERACTION_MATRIX:
  - Deepen Dev Command Center -> Role Mailbox, Role Mailbox -> Work Packet System, and Role Mailbox -> Micro-Task Executor notes; add Role Mailbox -> Locus Work Tracking authority-boundary edge.

## ACCEPTANCE_CRITERIA (DRAFT)
- Role Mailbox thread lifecycle and message delivery state are explicit, durable, and queryable without transcript parsing.
- Allowed responses and action-request metadata are visible before a reply or quick action is offered.
- Mailbox-local actions such as acknowledge or snooze never mutate linked authoritative records implicitly.
- Micro-Task request, feedback, verification, escalation, and completion-report traffic can be queried by typed message family.
- Dev Command Center triage distinguishes mailbox-local actions, governed actions, and transcription-required actions before linked work changes.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the existing Role Mailbox packet plus structured collaboration, workflow-state, and transition-automation registries.
- Full executor routing remains weaker until the later Role Mailbox and Micro-Task loop pass deepens verifier-driven retry and escalation control.

## RISKS / UNKNOWNs (DRAFT)
- Too many message families or allowed responses could make the mailbox contract noisy for operators and smaller models.
- If action-request metadata is optional everywhere, implementations may fall back to prose parsing and lose determinism.
- If dead-letter posture is not visible, operators may misread mailbox failure as linked work failure.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Role-Mailbox-Message-Thread-Contract-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Role-Mailbox-Message-Thread-Contract-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
