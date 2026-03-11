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

# Work Packet Stub: WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1

## STUB_METADATA
- WP_ID: WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1
- BASE_WP_ID: WP-1-Role-Mailbox-Executor-Routing-Claim-Lease
- CREATED_AT: 2026-03-11T00:45:00.000Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Role-Mailbox, WP-1-Role-Mailbox-Message-Thread-Contract, WP-1-Role-Mailbox-Triage-Queue-Controls, WP-1-Workflow-Transition-Automation-Registry, WP-1-Project-Agnostic-Workflow-State-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.176.md 7.6.3 (Phase 1) -> [ADD v02.176] Role Mailbox executor routing, claim-lease, and response authority
- ROADMAP_ADD_COVERAGE: SPEC=v02.176; PHASE=7.6.3; LINES=47268
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.176.md 2.3.15 Role Mailbox executor routing, claim-or-lease semantics, and response authority [ADD v02.176]
  - Handshake_Master_Spec_v02.176.md 2.6.8.10.3.3 Executor Routing, Claim-or-Lease Semantics, and Response Authority (Normative) [ADD v02.176]
  - Handshake_Master_Spec_v02.176.md 10.11.5.26 Role Mailbox Executor Routing, Claim-Lease, and Response Authority [ADD v02.176]

## INTENT (DRAFT)
- What: Define and later implement the durable Role Mailbox executor-routing, claim or lease, takeover, and response-authority contract across Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Task Board, and Work Packet follow-up views.
- Why: Handshake now has typed mailbox messages, verifier-loop control, and queue-remediation posture, but it still needs one lawful ownership layer so parallel local models, cloud models, reviewers, validators, operators, and workflow automation do not race to answer the same thread or silently seize work.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical `RoleMailboxExecutorKind`, `RoleMailboxClaimMode`, `RoleMailboxClaimLeaseV1`, and `RoleMailboxResponseAuthorityV1` contracts.
  - Single-owner versus shared-observer mailbox semantics, claim acquisition, renewal, release, expiry, and takeover posture.
  - Response-authority boundaries for local small models, cloud models, reviewers, validators, operators, and workflow automation.
  - Locus joins that make claimant identity, lease age, lease expiry, takeover legality, and handback reasons queryable by stable identifiers.
  - Dev Command Center, Task Board, Work Packet, and Micro-Task projections for claimant visibility and actor-ineligible posture.
- OUT_OF_SCOPE:
  - Full external email, chat, or ticket-ingest systems.
  - Replacing underlying Locus authority, Work Packet authority, or Micro-Task execution law.
  - Rich board redesign beyond the claimant, lease, and takeover fields needed for lawful projection.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center mailbox claim and takeover panel
  - Role Mailbox claim and lease takeover inspector
  - Task Board mailbox claimant overlay
  - Work Packet mailbox claimant and lease follow-up drawer
  - Micro-Task execution queue claimant badge
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Claim mode badge | Type: status chip | Tooltip: Shows whether the thread is exclusive, shared, broadcast, or handoff-reserved. | Notes: visible in queue rows and thread header
  - Control: Claimant badge | Type: identity chip | Tooltip: Shows the current claimant and actor kind. | Notes: opens claimant history
  - Control: Lease expiry badge | Type: due-date chip | Tooltip: Shows lease age, expiry, and stale-lease risk. | Notes: projection only
  - Control: Takeover preview | Type: side panel | Tooltip: Explains whether takeover is legal, who can approve it, and whether linked work changes are mailbox-local or governed. | Notes: required before takeover
  - Control: Response authority strip | Type: segmented actions | Tooltip: Shows which reply kinds are legal for the current actor. | Notes: must mark actor-ineligible actions explicitly
- UI_STATES (empty/loading/error):
  - No current claimant
  - Exclusive lease active
  - Lease expired and needs triage
  - Takeover blocked by authority scope
  - Claim metadata malformed

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - mcp agent mail advisory locks leases unified inbox
  - TinyClaw queue ownership team chat dead letter dashboard
  - OpenClaw session routing agent gateway ownership
  - Anthropic agent teams subagents shared project tasks
  - Hugging Face smolagents managed agents delegation authority
  - AutoGen handoffs event-driven routing observability
  - Google scaling agent systems routing coordination overhead
  - MasRouter multi-agent routing paper
- CANDIDATE_SOURCES:
  - Source: MCP Agent Mail | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T00:25:00Z | URL: https://github.com/Dicklesworthstone/mcp_agent_mail | Why: advisory reservations, unified inbox, searchable threads, and cross-agent routing posture.
  - Source: TinyClaw repository | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T00:25:00Z | URL: https://github.com/TinyAGI/tinyclaw | Why: queue-backed team chat, isolated agents, retries, and dashboarded async collaboration.
  - Source: OpenClaw repository | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T00:25:00Z | URL: https://github.com/openclaw/openclaw | Why: multi-agent gateway, channel routing, and session-scoped collaboration posture.
  - Source: Anthropic agent teams | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T00:25:00Z | URL: https://code.claude.com/docs/en/agent-teams | Why: task lists, shared team planning, and project-scoped agent coordination.
  - Source: Anthropic subagents | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T00:25:00Z | URL: https://code.claude.com/docs/en/sub-agents | Why: source-controlled subagents and scoped shared behavior.
  - Source: smolagents reference | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T00:25:00Z | URL: https://huggingface.co/docs/smolagents/en/reference/agents | Why: managed agents, delegation, step callbacks, and bounded-step control.
  - Source: AutoGen Studio | Kind: BIG_TECH | Date: 2025-10-14 | Retrieved: 2026-03-11T00:25:00Z | URL: https://www.microsoft.com/en-us/research/publication/autogen-studio-a-no-code-developer-tool-for-building-and-debugging-multi-agent-systems/ | Why: event-driven multi-agent workflows, observability, and debugging posture.
  - Source: Towards a Science of Scaling Agent Systems | Kind: BIG_TECH | Date: 2025-12-11 | Retrieved: 2026-03-11T00:25:00Z | URL: https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/ | Why: warns that coordination and routing overhead must stay explicit and bounded.
  - Source: MasRouter | Kind: UNIVERSITY|PAPER | Date: 2025-01-01 | Retrieved: 2026-03-11T00:25:00Z | URL: https://aclanthology.org/2025.acl-long.757/ | Why: specialist routing and adaptive multi-agent selection relevant to responder eligibility and claim posture.

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: MCP Agent Mail | Pattern: advisory reservations and unified inbox | Why: Handshake needs explicit temporary ownership instead of implicit “who replied last” state.
  - Source: Anthropic agent teams and subagents | Pattern: project-scoped shared collaborators and explicit task sharing | Why: claimant rules and allowed responders should be source-controlled and scoped, not informal.
  - Source: smolagents | Pattern: managed delegation and bounded-step control | Why: local small models should only claim mailbox work they are eligible to answer.
- ADAPT:
  - Source: TinyClaw | Pattern: queue-backed team chat and isolated workers | Why: good fit for per-actor routing and stale-lease visibility, but Handshake must keep Locus authoritative.
  - Source: OpenClaw | Pattern: multi-channel gateway and session routing | Why: useful for actor routing and announce-back posture, but not as a work-state authority.
  - Source: AutoGen Studio | Pattern: event-driven multi-agent observability | Why: claimant and takeover transitions should be inspectable and replay-safe.
- REJECT:
  - Source: implicit latest-responder ownership | Pattern: treat the most recent reply as owner | Why: conflicts with Handshake governed action, Locus authority, and safe parallel work.

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - agent mailbox lease claim takeover github
  - multi agent team inbox reservation ownership github
- MATCHED_PROJECTS:
  - Repo: Dicklesworthstone/mcp_agent_mail | Intent: ARCH_PATTERN | Decision hint: ADOPT | Impact hint: SPEC_UPDATE | Notes: best direct analog for advisory leases and unified inbox semantics.
  - Repo: TinyAGI/tinyclaw | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: queue-backed team routing and isolation are a strong fit for claimant-aware parallel work.
  - Repo: openclaw/openclaw | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: useful for gateway and routing posture, but not for authority.

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Human Collaboration | STATUS: TOUCHED | NOTES: explicit claimant and takeover rules reduce double-handling and silent ownership drift | Stub follow-up: THIS_STUB
  - PILLAR: Local small model execution | STATUS: TOUCHED | NOTES: response-authority scope keeps smaller models inside safe work classes | Stub follow-up: THIS_STUB
  - PILLAR: Governance kernel | STATUS: TOUCHED | NOTES: temporary mailbox ownership stays visible without replacing governed work authority | Stub follow-up: THIS_STUB

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: role_mailbox.claim_registry | ENGINE_ID: role_mailbox.claim_registry | STATUS: TOUCHED | NOTES: owns claimant identity, claim mode, lease age, expiry, and takeover history | Stub follow-up: THIS_STUB
  - ENGINE: role_mailbox.response_authority | ENGINE_ID: role_mailbox.response_authority | STATUS: TOUCHED | NOTES: decides which actor kinds may answer or take over a thread | Stub follow-up: THIS_STUB
  - ENGINE: locus.mailbox_claim_join | ENGINE_ID: locus.mailbox_claim_join | STATUS: TOUCHED | NOTES: joins mailbox claimant posture back to authoritative work state without making claims authoritative | Stub follow-up: THIS_STUB
  - ENGINE: dev_command_center.mailbox_claim_panel | ENGINE_ID: dev_command_center.mailbox_claim_panel | STATUS: TOUCHED | NOTES: projects claimant, lease-expiry, and takeover preview state | Stub follow-up: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-RoleMailboxExecutorKind
  - PRIM-RoleMailboxClaimMode
  - PRIM-RoleMailboxClaimLeaseV1
  - PRIM-RoleMailboxResponseAuthorityV1
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-DEV-COMMAND-CENTER | ROI: H | Effort: M | Notes: claim and takeover previews must remain operator-visible before any reply or reroute action.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-LOCUS-WORK-TRACKING | ROI: H | Effort: M | Notes: claimant and lease posture must join back to authoritative work state without overtaking it.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-MICRO-TASK-EXECUTOR | ROI: H | Effort: M | Notes: mailbox claim state must gate safe executor pickup and reroute behavior.

## ACCEPTANCE_CRITERIA (DRAFT)
- Actionable mailbox threads expose executor-kind allowlists, claim mode, current claimant, lease expiry, takeover policy, and response-authority scope through structured fields.
- Dev Command Center can preview whether claim, release, renew, takeover, or reply actions are mailbox-local, automation-triggering, or governed.
- Locus, Micro-Task, Task Board, and Work Packet views can explain claimant and stale-lease posture without turning mailbox claims into work-state authority.
- Local small models, cloud models, reviewers, validators, and operators are visibly separated by response-authority scope before a reply or takeover is offered.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on Role Mailbox thread-lifecycle, queue-remediation, workflow-state, and transition-automation law.
- Full cross-channel intake remains deferred until internal claim and authority rules are stable.

## RISKS / UNKNOWNs (DRAFT)
- Overly granular claimant states could create operator noise and make local-small-model routing harder.
- If takeover policy is vague, actors may assume stale leases are safe to seize and create hidden duplicate work.
- If response-authority scope is optional, implementations may fall back to prompt-only heuristics and lose determinism.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
