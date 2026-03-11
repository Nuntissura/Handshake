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

# Work Packet Stub: WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1

## STUB_METADATA
- WP_ID: WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1
- BASE_WP_ID: WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back
- CREATED_AT: 2026-03-11T08:20:00.000Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Role-Mailbox, WP-1-Role-Mailbox-Message-Thread-Contract, WP-1-Role-Mailbox-Micro-Task-Loop-Control, WP-1-Role-Mailbox-Executor-Routing-Claim-Lease, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Project-Agnostic-Workflow-State-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.177.md 7.6.3 (Phase 1) -> [ADD v02.177] Role Mailbox handoff bundle, note transcription, and announce-back provenance
- ROADMAP_ADD_COVERAGE: SPEC=v02.177; PHASE=7.6.3; LINES=47289
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.177.md Role Mailbox handoff bundle, note transcription, and announce-back provenance [ADD v02.177]
  - Handshake_Master_Spec_v02.177.md 2.6.8.10.3.4 Handoff Bundle, Note Transcription, and Announce-Back Provenance (Normative) [ADD v02.177]
  - Handshake_Master_Spec_v02.177.md 10.11.5.27 Role Mailbox Handoff Bundle and Announce-Back Provenance [ADD v02.177]

## INTENT (DRAFT)
- What: Define and later implement the structured handoff-bundle, note-transcription, and announce-back provenance contract that lets Role Mailbox hand work across actors without forcing Work Packet, Micro-Task, Task Board, Locus, or Dev Command Center consumers to replay full threads.
- Why: Handshake now has mailbox message law, verifier-loop control, triage queues, and claim-or-lease posture, but it still needs one durable handoff artifact so parallel actors can resume safely and so advisory announce-back summaries do not get mistaken for authoritative completion.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical `RoleMailboxHandoffBundleV1` and `RoleMailboxAnnounceBackProvenanceV1` contracts.
  - Required handoff fields for remaining work, unresolved blockers, changed scope, evidence refs, recommended next actor, risk, confidence, and transcription targets.
  - Announce-back provenance kinds that distinguish advisory status, completion notice, escalation summary, scope-change notice, handoff-ready posture, and transcription-confirmed outcomes.
  - Normalized Work Packet note transcription and Locus join posture so handoff state remains queryable without transcript replay.
  - Dev Command Center and Task Board projection rules for handoff-ready, transcription-pending, and advisory announce-back posture.
- OUT_OF_SCOPE:
  - External email or chat intake.
  - Replacing Locus authority, Work Packet authority, or generic workflow-state law.
  - Final export or audit portability of mailbox threads beyond the handoff fields required for Phase 1.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center mailbox handoff and announce-back inspector
  - Role Mailbox provenance drawer
  - Work Packet handoff-note timeline
  - Task Board handoff-ready overlay
  - Micro-Task resume card with compact handoff summary
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Handoff bundle badge | Type: status chip | Tooltip: Shows whether a structured handoff bundle exists for the latest actionable mailbox message. | Notes: opens compact summary first
  - Control: Announce-back provenance badge | Type: status chip | Tooltip: Shows whether the latest announce-back is advisory, completion, escalation, or transcription-confirmed. | Notes: visible in thread rows and DCC
  - Control: Transcription status badge | Type: status chip | Tooltip: Shows whether linked Work Packet, Locus, or Micro-Task records already reflect the mailbox handoff. | Notes: must never imply completion on its own
  - Control: Recommended next actor chip | Type: identity chip | Tooltip: Shows the actor or executor that the current handoff bundle points to. | Notes: opens routing context
  - Control: Source-thread drilldown | Type: side panel | Tooltip: Shows the source mailbox lines and authoritative note refs behind the current compact handoff bundle. | Notes: drilldown only
- UI_STATES (empty/loading/error):
  - No handoff bundle yet
  - Advisory announce-back only
  - Transcription pending
  - Handoff summary stale
  - Provenance missing or malformed

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - LangChain handoffs state transfer structured summaries
  - AutoGen handoffs event driven collaboration provenance
  - Anthropic agent teams shared task handoff source control
  - MCP Agent Mail handback summary announce back threads
  - OpenHands plan execute handoff summary agent collaboration
  - Google Chain of Agents short context relay handoff
- CANDIDATE_SOURCES:
  - Source: LangChain handoffs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T08:00:00Z | URL: https://docs.langchain.com/oss/python/langchain/multi-agent/handoffs | Why: explicit handoff contract and state transfer patterns.
  - Source: AutoGen handoffs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T08:00:00Z | URL: https://microsoft.github.io/autogen/dev/user-guide/core-user-guide/design-patterns/handoffs.html | Why: event-driven handoff flows and structured next-actor transfer.
  - Source: Claude Code agent teams | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T08:00:00Z | URL: https://code.claude.com/docs/en/agent-teams | Why: scoped team task-sharing and structured collaboration.
  - Source: Claude Code subagents | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T08:00:00Z | URL: https://code.claude.com/docs/en/sub-agents | Why: source-controlled collaborators and reusable handoff definitions.
  - Source: MCP Agent Mail | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T08:00:00Z | URL: https://github.com/Dicklesworthstone/mcp_agent_mail | Why: git-backed inboxes, human overseer messages, searchable handback context, and advisory leases.
  - Source: OpenHands repository | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T08:00:00Z | URL: https://github.com/OpenHands/OpenHands | Why: plan-to-execute summaries, event-backed collaboration, and replay-safe agent state.
  - Source: Chain of Agents | Kind: BIG_TECH | Date: 2025-12-01 | Retrieved: 2026-03-11T08:00:00Z | URL: https://research.google/pubs/chain-of-agents-large-language-modelscollaborating-on-long-context-tasks/ | Why: summary-first relay patterns for long-context collaboration.
  - Source: smolagents reference | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T08:00:00Z | URL: https://huggingface.co/docs/smolagents/en/reference/agents | Why: summary-first agent routing and bounded memory posture.

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: LangChain handoffs | Pattern: structured next-actor state transfer | Why: Handshake handoff bundles should be explicit artifacts, not inferred from chat history.
  - Source: AutoGen handoffs | Pattern: event-driven provenance on handoff | Why: mailbox handoff summaries should retain source-thread, message, and transition lineage.
  - Source: Chain of Agents | Pattern: short relay summaries between agents | Why: local small models need compact handoff bundles before full-thread drilldown.
- ADAPT:
  - Source: MCP Agent Mail | Pattern: searchable handback context and overseer messaging | Why: useful for announce-back and handback visibility, but Handshake must keep Locus and Work Packets authoritative.
  - Source: OpenHands | Pattern: replay-safe collaboration summaries | Why: good fit for Dev Command Center drilldown, but Handshake needs stronger governed transcription boundaries.
  - Source: smolagents | Pattern: summary-first memory posture | Why: compact handoff summaries should be the default read path for smaller models.
- REJECT:
  - Source: latest-message-is-truth patterns | Pattern: infer completion or handoff from the newest reply | Why: conflicts with Handshake transcription and authoritative-state rules.

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - agent handoff bundle provenance summary github
  - multi agent announce back transcription thread repo
- MATCHED_PROJECTS:
  - Repo: Dicklesworthstone/mcp_agent_mail | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: closest mailbox-like handback and overseer posture.
  - Repo: OpenHands/OpenHands | Intent: SPEC_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: useful replay-safe summary and event-log patterns.
  - Repo: langchain-ai/langchain | Intent: SPEC_PATTERN | Decision hint: ADOPT | Impact hint: SPEC_UPDATE | Notes: strongest direct handoff contract inspiration.

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Role Mailbox | STATUS: TOUCHED | NOTES: structured handoff bundles prevent transcript-order authority | Stub follow-up: THIS_STUB
  - PILLAR: Locus Work Tracking | STATUS: TOUCHED | NOTES: authoritative joins can hold accepted handoff refs and provenance | Stub follow-up: THIS_STUB
  - PILLAR: Work Packet System | STATUS: TOUCHED | NOTES: note transcription becomes durable handoff context instead of prose drift | Stub follow-up: THIS_STUB
  - PILLAR: Local small model execution | STATUS: TOUCHED | NOTES: compact handoff summaries reduce thread replay cost | Stub follow-up: THIS_STUB
