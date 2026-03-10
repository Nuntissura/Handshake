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

# Work Packet Stub: WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1

## STUB_METADATA
- WP_ID: WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1
- BASE_WP_ID: WP-1-Role-Mailbox-Micro-Task-Loop-Control
- CREATED_AT: 2026-03-10T22:45:00.000Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Role-Mailbox, WP-1-Micro-Task-Executor, WP-1-Role-Mailbox-Message-Thread-Contract, WP-1-Workflow-Transition-Automation-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.174.md 7.6.3 (Phase 1) -> [ADD v02.174] Role Mailbox and Micro-Task loop control
- ROADMAP_ADD_COVERAGE: SPEC=v02.174; PHASE=7.6.3; LINES=47216
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.174.md 2.6.6.8.6.3 Mailbox Loop Checkpoint and Verifier Outcome Contracts (Normative) [ADD v02.174]
  - Handshake_Master_Spec_v02.174.md 2.6.8.10.3.1 Micro-Task Loop Control, Verifier Feedback, and Escalation (Normative) [ADD v02.174]
  - Handshake_Master_Spec_v02.174.md 10.11.5.24 Role Mailbox Micro-Task Loop Control [ADD v02.174]

## INTENT (DRAFT)
- What: Define and later implement the bounded mailbox-loop-control contract that lets Role Mailbox coordinate Micro-Task retries, verifier feedback, verification-needed waits, escalation, and completion reports through structured checkpoints instead of transcript replay.
- Why: Handshake now has typed Role Mailbox threads and portable workflow law, but it still needs one durable loop-control contract so small local models, cloud models, reviewers, and operators can resume or inspect Micro-Task loops from compact state rather than chat chronology.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical `MicroTaskLoopCheckpointV1` and `MicroTaskVerifierOutcomeV1` artifact contracts.
  - Structured Role Mailbox payload requirements for `MicroTaskFeedback`, `MicroTaskVerificationNeeded`, `MicroTaskEscalation`, and `MicroTaskCompletionReport`.
  - Retry-budget, escalation-target, and completion-report transcription posture across Role Mailbox, Locus Work Tracking, Work Packet notes, Task Board projections, and Dev Command Center.
  - Compact-summary-first loop inspection for local small models and operator triage.
  - Dead-letter, expiry, and ignored-message handling for mailbox-linked Micro-Task loops.
- OUT_OF_SCOPE:
  - External email, chat, or Slack intake.
  - Generic Jira or board redesign beyond loop-state fields needed for lawful projection.
  - Replacing the underlying Micro-Task execution engine, validator engine, or Work Profile model-routing law.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center verifier loop inspector
  - Role Mailbox thread drawer with loop checkpoint timeline
  - Micro-Task queue row with retry-budget and verifier badges
  - Work Packet note timeline with completion-report transcription state
  - Escalation remediation drawer
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Loop checkpoint badge | Type: status chip | Tooltip: Shows the latest bounded checkpoint for the linked Micro-Task loop. | Notes: opens checkpoint timeline
  - Control: Verifier outcome badge | Type: status chip | Tooltip: Shows whether the latest verifier outcome passed, failed, needs evidence, or requires escalation. | Notes: visible in inbox and execution queues
  - Control: Retry budget meter | Type: inline meter | Tooltip: Shows remaining retries before escalation or hard stop. | Notes: projection only; not editable directly
  - Control: Completion transcription badge | Type: status chip | Tooltip: Shows whether the latest completion report has been transcribed into linked Work Packet or Locus authority. | Notes: must differentiate mailbox-only from authoritative state
- UI_STATES (empty/loading/error):
  - No loop checkpoint yet
  - Verifier outcome missing
  - Retry budget exhausted
  - Completion reported but transcription pending
  - Dead-letter loop remediation required

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Ralph loop agent verifier retry checkpoint github
  - Get Shit Done atomic task verification github
  - LangGraph agent inbox action request retry verification
  - TinyClaw team chat retry dead letter queue
  - Anthropic shared subagents project collaboration
  - Google scaling agent systems coordination overhead
- CANDIDATE_SOURCES:
  - Source: Vercel Labs Ralph Loop Agent | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T22:30:00Z | URL: https://github.com/vercel-labs/ralph-loop-agent | Why: verifier-driven loop control, bounded stop conditions, and iteration summaries.
  - Source: Ralph Orchestrator | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T22:30:00Z | URL: https://github.com/mikeyobrien/ralph-orchestrator | Why: human interrupt path and loop-routing posture.
  - Source: Get Shit Done | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T22:30:00Z | URL: https://github.com/gsd-build/get-shit-done | Why: atomic task units, fresh context, and verification after execution.
  - Source: LangGraph Agent Inbox | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T22:30:00Z | URL: https://github.com/langchain-ai/agent-inbox | Why: typed action requests and bounded response envelopes.
  - Source: TinyClaw repository | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T22:30:00Z | URL: https://github.com/TinyAGI/tinyclaw | Why: retries, dead-letter posture, persistent team chat, and queue-backed collaboration.
  - Source: MCP Agent Mail | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T22:30:00Z | URL: https://github.com/Dicklesworthstone/mcp_agent_mail | Why: searchable threads, unified inbox, and git-backed collaboration artifacts.
  - Source: Claude Code subagents | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T22:30:00Z | URL: https://code.claude.com/docs/en/sub-agents | Why: shared team subagents and source-controlled collaborator definitions.
  - Source: Towards a Science of Scaling Agent Systems | Kind: BIG_TECH | Date: 2025-12-11 | Retrieved: 2026-03-10T22:30:00Z | URL: https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/ | Why: warns against unbounded coordination overhead and supports explicit bounded loops.

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: Ralph Loop Agent | Pattern: verifier-driven retry with hard stop conditions | Why: Handshake mailbox-linked Micro-Task loops need bounded retry and explicit verifier outcomes.
  - Source: Get Shit Done | Pattern: atomic execution units with fresh context and post-execution verification | Why: Handshake Micro-Tasks should stay small and resumable from compact checkpoints.
  - Source: LangGraph Agent Inbox | Pattern: typed action envelopes | Why: retry, escalate, and complete actions need explicit contracts instead of prose parsing.
- ADAPT:
  - Source: TinyClaw | Pattern: dead-letter and queue-backed collaboration posture | Why: useful for mailbox loop expiry and remediation, but Handshake must keep Locus authoritative.
  - Source: MCP Agent Mail | Pattern: searchable unified inbox plus artifact-backed thread state | Why: good fit for compact checkpoint recovery and operator drilldown.
- REJECT:
  - Source: open-ended retry loops | Pattern: “keep trying until it feels done” | Why: conflicts with Handshake hard-stop, verifier, and authority-boundary rules.

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - verifier driven retry checkpoint mailbox loop agent github
  - micro task feedback escalation completion report thread repo
- MATCHED_PROJECTS:
  - Repo: vercel-labs/ralph-loop-agent | Intent: SPEC_PATTERN | Decision hint: ADOPT | Impact hint: SPEC_UPDATE | Notes: strongest direct pattern for bounded verifier loops.
  - Repo: gsd-build/get-shit-done | Intent: EXEC_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: strongest direct pattern for atomic executable task units.
  - Repo: langchain-ai/agent-inbox | Intent: SPEC_PATTERN | Decision hint: ADOPT | Impact hint: SPEC_UPDATE | Notes: action request and allowed-response modeling.
  - Repo: TinyAGI/tinyclaw | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: dead-letter handling and queue-backed team collaboration.

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Local small model execution | STATUS: TOUCHED | NOTES: compact loop checkpoints let smaller models resume bounded work safely | Stub follow-up: THIS_STUB
  - PILLAR: Human Collaboration | STATUS: TOUCHED | NOTES: verifier-driven handoff reduces ambiguous “please continue” chat | Stub follow-up: THIS_STUB
  - PILLAR: Governance kernel | STATUS: TOUCHED | NOTES: mailbox loop control stays non-authoritative while still driving structured async collaboration | Stub follow-up: THIS_STUB

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: role_mailbox.loop_checkpoint_registry | ENGINE_ID: role_mailbox.loop_checkpoint_registry | STATUS: TOUCHED | NOTES: persists bounded checkpoint state for mailbox-linked Micro-Task loops | Stub follow-up: THIS_STUB
  - ENGINE: micro_task_executor.verifier_outcome_bridge | ENGINE_ID: micro_task_executor.verifier_outcome_bridge | STATUS: TOUCHED | NOTES: persists structured verifier outcomes for retry or escalation decisions | Stub follow-up: THIS_STUB
  - ENGINE: dev_command_center.loop_inspector | ENGINE_ID: dev_command_center.loop_inspector | STATUS: TOUCHED | NOTES: projects bounded loop state and quick-action previews | Stub follow-up: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1
  - ENGINE: locus.mailbox_loop_join | ENGINE_ID: locus.mailbox_loop_join | STATUS: TOUCHED | NOTES: ties mailbox loop artifacts back to authoritative packet and task state | Stub follow-up: THIS_STUB

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-MicroTaskLoopCheckpointV1
  - PRIM-MicroTaskVerifierOutcomeV1
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-MICRO-TASK-EXECUTOR | ROI: H | Effort: M | Notes: mailbox must coordinate verifier loops without becoming task authority.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-WORK-PACKET-SYSTEM | ROI: H | Effort: M | Notes: completion reports and escalation summaries should feed packet handoff posture.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-LOCUS-WORK-TRACKING | ROI: H | Effort: M | Notes: loop checkpoints and verifier outcomes should be joinable to authoritative tracked state.

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: WP-1-Role-Mailbox-Message-Thread-Contract-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: owns thread lifecycle and allowed responses, but not loop-checkpoint and verifier-outcome contracts.
  - Artifact: WP-1-Workflow-Transition-Automation-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: retry and escalation still resolve through transition law, but loop checkpoint state is distinct.
  - Artifact: WP-1-MTE-Summaries-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: summary work overlaps, but mailbox-linked checkpoint posture is a separate contract.
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: WP-1-Micro-Task-Executor-v1 | Status: VALIDATED | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | CodeReality: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: loop engine exists conceptually, but mailbox-linked checkpoint and verifier-outcome contracts still need dedicated treatment.

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - FEAT-ROLE-MAILBOX, FEAT-MICRO-TASK-EXECUTOR, FEAT-LOCUS-WORK-TRACKING, FEAT-WORK-PACKET-SYSTEM, FEAT-TASK-BOARD, and FEAT-DEV-COMMAND-CENTER align on loop checkpoints, verifier outcomes, retry-budget posture, and completion-report transcription.
- PRIMITIVE_INDEX:
  - Add PRIM-MicroTaskLoopCheckpointV1 and PRIM-MicroTaskVerifierOutcomeV1.
- UI_GUIDANCE:
  - Mailbox and Dev Command Center loop inspectors expose retry budget, verifier outcome, escalation target, and completion transcription posture before action execution.
- INTERACTION_MATRIX:
  - Deepen Role Mailbox -> Micro-Task Executor and Role Mailbox -> Locus Work Tracking notes; add Role Mailbox -> Work Packet System loop-report edge if it remains genuinely new.

## ACCEPTANCE_CRITERIA (DRAFT)
- Every mailbox-linked Micro-Task retry, escalation, verification request, or completion report can be understood from a bounded checkpoint plus verifier outcome without replaying the full thread.
- Remaining retry budget, escalation target, and completion transcription posture remain visible before a mailbox quick action can request a governed state change.
- Work Packet and Task Board projections can explain mailbox-linked waiting, retrying, escalated, and complete posture without becoming authoritative for loop state themselves.
- Local small models can ingest compact loop checkpoints before long-form thread or note replay.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on Role Mailbox thread-lifecycle work, Micro-Task Executor baseline loop law, and workflow-transition automation registry law.
- Full cross-channel collaboration remains deferred until the internal loop-control contract is stable.

## RISKS / UNKNOWNs (DRAFT)
- If checkpoints are too verbose, small models lose the intended compact-state benefit.
- If verifier outcomes are optional, retry loops may drift back toward prose-only explanations.
- If completion-report transcription is not explicit, Work Packet or Task Board views may falsely imply completion.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
