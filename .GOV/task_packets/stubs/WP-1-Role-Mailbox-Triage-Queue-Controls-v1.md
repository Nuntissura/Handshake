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

# Work Packet Stub: WP-1-Role-Mailbox-Triage-Queue-Controls-v1

## STUB_METADATA
- WP_ID: WP-1-Role-Mailbox-Triage-Queue-Controls-v1
- BASE_WP_ID: WP-1-Role-Mailbox-Triage-Queue-Controls
- CREATED_AT: 2026-03-11T00:08:00.000Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Role-Mailbox, WP-1-Role-Mailbox-Message-Thread-Contract, WP-1-Workflow-Transition-Automation-Registry, WP-1-Project-Agnostic-Workflow-State-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.175.md 7.6.3 (Phase 1) -> [ADD v02.175] Role Mailbox triage, queue aging, and remediation controls
- ROADMAP_ADD_COVERAGE: SPEC=v02.175; PHASE=7.6.3; LINES=47234
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.175.md 2.6.8.10.3.2 Role Mailbox Triage, Aging, Snooze, Expiry, and Dead-Letter Remediation (Normative) [ADD v02.175]
  - Handshake_Master_Spec_v02.175.md 10.11.5.25 Role Mailbox Triage Queues and Remediation Controls [ADD v02.175]
  - Handshake_Master_Spec_v02.175.md 2.3.15.5 Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]

## INTENT (DRAFT)
- What: Define and later implement the durable Role Mailbox triage queue, reminder, snooze, expiry, and dead-letter remediation contract across Role Mailbox, Dev Command Center, Task Board, Work Packet views, and Locus joins.
- Why: Handshake now has typed mailbox messages and Micro-Task loop control, but it still needs one portable queue-management layer so async collaboration pressure, stale work, and delivery failures do not collapse back into unread badges, thread order, or operator memory.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical `RoleMailboxTriageQueueState`, `RoleMailboxReminderScheduleV1`, and `RoleMailboxDeadLetterDisposition` contracts.
  - Queue-aging fields, reminder cadence, snooze posture, expiry posture, and dead-letter remediation metadata for Role Mailbox threads.
  - Dev Command Center remediation queue requirements for reminder, unsnooze, retry-delivery, reroute, archive, and transcription-aware controls.
  - Task Board pressure overlays and Work Packet follow-up summaries that project mailbox backlog without becoming authority.
  - Locus joins that make mailbox-derived waiting, snooze, expiry, and dead-letter posture queryable by stable identifiers.
- OUT_OF_SCOPE:
  - External email, Slack, or generic ticket-ingest channels.
  - Rich issue-tracker redesign beyond the queue and remediation fields needed for lawful triage.
  - Replacing the Role Mailbox export or Flight Recorder baseline contracts.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center mailbox remediation queue
  - Role Mailbox triage row and thread inspector
  - Task Board mailbox-pressure overlay
  - Work Packet follow-up drawer
  - Dead-letter remediation drawer
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Triage queue badge | Type: status chip | Tooltip: Shows whether the thread is active, snoozed, due soon, expired, or in dead-letter remediation. | Notes: opens queue-state history
  - Control: Reminder schedule badge | Type: due-date chip | Tooltip: Shows next reminder, expiry, and snooze posture. | Notes: visible in queue rows
  - Control: Dead-letter disposition badge | Type: status chip | Tooltip: Shows how a dead-lettered message should be remediated. | Notes: must stay separate from linked work state
  - Control: Remediation action strip | Type: segmented actions | Tooltip: Offers reminder, unsnooze, retry-delivery, reroute, transcription, and archive actions with previews. | Notes: must separate mailbox-local from governed actions
- UI_STATES (empty/loading/error):
  - No queued mailbox remediation
  - Snoozed but not expired
  - Expired and awaiting operator follow-up
  - Dead-letter remediation required
  - Reminder schedule invalid

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Linear inbox triage snooze reminders asks
  - Jira queue reminders follow up service management
  - GitHub Projects fields automations board queue
  - TinyClaw dead letter retry dashboard
  - MCP Agent Mail searchable inbox leases
  - Hugging Face smolagents summary-first delegation
  - agent routing queue state paper
- CANDIDATE_SOURCES:
  - Source: Linear Inbox | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T00:05:00Z | URL: https://linear.app/docs/inbox | Why: triage-first inbox posture, snooze or reminder expectations, and low-noise queue handling.
  - Source: Linear Asks | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T00:05:00Z | URL: https://linear.app/docs/linear-asks | Why: inbound request normalization, queue routing, and answer workflow patterns.
  - Source: GitHub Projects fields and automations | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T00:05:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: stable field model powering multiple queue and board views without making the view authoritative.
  - Source: Jira Service Management queues and reminders | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T00:05:00Z | URL: https://support.atlassian.com/jira-service-management-cloud/docs/set-up-queues/ | Why: queue aging, follow-up, and remediation posture for operator-driven work management.
  - Source: TinyClaw repository | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T00:05:00Z | URL: https://github.com/TinyAGI/tinyclaw | Why: queue-backed team chat, retries, dead-letter handling, and dashboarded async collaboration.
  - Source: MCP Agent Mail | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T00:05:00Z | URL: https://github.com/Dicklesworthstone/mcp_agent_mail | Why: searchable unified inbox, leases, thread metadata, and git-backed collaboration records.
  - Source: LangGraph Agent Inbox | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T00:05:00Z | URL: https://github.com/langchain-ai/agent-inbox | Why: explicit action envelopes and legal-response posture for inbox-like flows.
  - Source: smolagents managed agents and memory | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T00:05:00Z | URL: https://huggingface.co/docs/smolagents/en/reference/agents | Why: summary-first delegation and bounded context patterns for smaller models.
  - Source: Towards a Science of Scaling Agent Systems | Kind: BIG_TECH | Date: 2025-12-11 | Retrieved: 2026-03-11T00:05:00Z | URL: https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/ | Why: explicit warning that coordination overhead must stay bounded and stateful.
  - Source: Optimal-Agent-Selection | Kind: UNIVERSITY|PAPER | Date: 2025-11-13 | Retrieved: 2026-03-11T00:05:00Z | URL: https://arxiv.org/abs/2511.10137 | Why: routing and executor-selection ideas relevant to mailbox triage and remediation queues.

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: Linear Inbox | Pattern: explicit triage queue with reminder-friendly posture | Why: Handshake inbox work needs deliberate triage state instead of unread or newest-first heuristics.
  - Source: GitHub Projects fields and automations | Pattern: field-driven queues and regrouping | Why: mailbox pressure should be projected from canonical fields, not view-local state.
  - Source: TinyClaw | Pattern: dead-letter and retry posture | Why: mailbox delivery failures need visible remediation instead of disappearing into failed transport logs.
- ADAPT:
  - Source: MCP Agent Mail | Pattern: unified inbox plus leases/searchable threads | Why: good fit for operator drilldown and later role-level claim semantics, but Locus must remain authority.
  - Source: Jira queues and reminders | Pattern: operator-managed queue follow-up and escalation | Why: useful for aging and reminder controls, but Handshake must preserve governed-action previews.
  - Source: smolagents | Pattern: summary-first delegation | Why: smaller models should ingest compact queue state before thread replay.
- REJECT:
  - Source: unread-count-only inbox patterns | Pattern: newest-first or unread-only priority | Why: unread state is not enough to drive governed remediation or linked work posture.

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - role mailbox queue snooze dead letter reminder github
  - searchable inbox lease dead letter retry dashboard github
- MATCHED_PROJECTS:
  - Repo: TinyAGI/tinyclaw | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: strongest queue/dead-letter collaboration fit.
  - Repo: Dicklesworthstone/mcp_agent_mail | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: strongest unified inbox and searchable-thread fit.
  - Repo: langchain-ai/agent-inbox | Intent: SPEC_PATTERN | Decision hint: ADOPT | Impact hint: SPEC_UPDATE | Notes: action-envelope fit for remediation controls.

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Human Collaboration | STATUS: TOUCHED | NOTES: durable queue posture turns async collaboration debt into explicit operator work instead of tribal memory | Stub follow-up: THIS_STUB
  - PILLAR: Local small model execution | STATUS: TOUCHED | NOTES: compact triage state lets smaller models understand whether to wait, respond, or escalate without replaying threads | Stub follow-up: THIS_STUB
  - PILLAR: Governance kernel | STATUS: TOUCHED | NOTES: queue and remediation controls must stay non-authoritative and project-agnostic | Stub follow-up: THIS_STUB

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: role_mailbox.triage_queue | ENGINE_ID: role_mailbox.triage_queue | STATUS: TOUCHED | NOTES: owns queue state, reminder schedule, snooze posture, expiry posture, and dead-letter disposition | Stub follow-up: THIS_STUB
  - ENGINE: dev_command_center.mailbox_remediation | ENGINE_ID: dev_command_center.mailbox_remediation | STATUS: TOUCHED | NOTES: projects remediation controls and previews mailbox-local versus governed follow-up | Stub follow-up: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1
  - ENGINE: locus.mailbox_queue_join | ENGINE_ID: locus.mailbox_queue_join | STATUS: TOUCHED | NOTES: joins mailbox pressure and remediation posture into linked work state without handing authority to mailbox threads | Stub follow-up: THIS_STUB

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-RoleMailboxTriageQueueState
  - PRIM-RoleMailboxReminderScheduleV1
  - PRIM-RoleMailboxDeadLetterDisposition
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-TASK-BOARD | ROI: H | Effort: M | Notes: mailbox pressure should project into planning views without making Task Board authoritative.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-DEV-COMMAND-CENTER | ROI: H | Effort: M | Notes: remediation queue and action previews need one shared queue-control contract.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-LOCUS-WORK-TRACKING | ROI: H | Effort: M | Notes: mailbox queue state must join to linked work posture through stable ids.

## ACCEPTANCE_CRITERIA (DRAFT)
- Role Mailbox triage queue state, reminder schedule, snooze posture, expiry posture, and dead-letter disposition are explicit, durable, and queryable without transcript parsing.
- Dev Command Center can distinguish mailbox-local reminder or archive actions from governed follow-up before execution.
- Task Board and Work Packet views can explain mailbox-derived waiting pressure and remediation posture without becoming authority for mailbox state.
- Local small models can ingest bounded triage state before replaying long-form mailbox threads.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on Role Mailbox thread-lifecycle work, workflow-state registry work, and workflow-transition automation law.
- Rich cross-channel intake remains deferred until the internal queue and remediation contract is stable.

## RISKS / UNKNOWNs (DRAFT)
- Too many queue states or remediation controls could create operator noise and make local-small-model routing harder.
- If reminder schedules are optional, stale collaboration debt may hide behind “active” state.
- If dead-letter disposition is not explicit, delivery failure can be mistaken for linked work failure.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Role-Mailbox-Triage-Queue-Controls-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Role-Mailbox-Triage-Queue-Controls-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
