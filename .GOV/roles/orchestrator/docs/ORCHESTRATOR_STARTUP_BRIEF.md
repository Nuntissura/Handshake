# Orchestrator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: ORCHESTRATOR

## Use

Use this brief after `just orchestrator-startup` and before governed mutation. It is operational memory, not authority.

## Action Cards

### RAM-ORCHESTRATOR-SESSION_OPEN-001

- ACTION: SESSION_OPEN
- TRIGGER: before any governed mutation, including push, closeout repair, packet prep, status sync, or deterministic governance edits
- FAILURE_PATTERN: repeatedly opening repomem with a terse under-80-character topic and then having to capture the same failure
- DO: write one sentence that names role action, target, reason, and expected outcome; use `just repomem open "<substantive purpose>" --role ORCHESTRATOR [--wp WP-ID]`
- DO_NOT: use fragments such as `push gov kernel` or `status sync`
- VERIFY: command prints `REPOMEM_SESSION_OPEN`
- SOURCE: memory-capture #5878, Operator correction 2026-04-30

### RAM-ORCHESTRATOR-MECHANICAL_GOVERNANCE-001

- ACTION: DETERMINISTIC_CHECKS
- TRIGGER: phase checks, closeout repair, validator-gate operations, or status projection
- FAILURE_PATTERN: routing deterministic governance checks through ACP prompts instead of direct local commands
- DO: run `just` or Node commands directly from `wt-gov-kernel`; reserve ACP for implementation and validator judgment lanes
- DO_NOT: use ACP `SEND_PROMPT` for phase-check, closeout-repair, or validator-gate mechanics
- VERIFY: command output comes from the direct local process and can be rerun without role-session state
- SOURCE: Orchestrator role lock

### RAM-ORCHESTRATOR-CLOSEOUT-001

- ACTION: CLOSEOUT_PREP
- TRIGGER: before launching Integration Validator
- FAILURE_PATTERN: launching final judgment with broken mechanical closeout truth, causing repair loops and stale-session drift
- DO: verify final `CODER_HANDOFF` committed target evidence, run `just phase-check HANDOFF WP-{ID} WP_VALIDATOR --range <base>..<head>`, and use `just closeout-repair WP-{ID}` only for pre-verdict prep drift; Integration Validator then starts with `just phase-check VERDICT WP-{ID} INTEGRATION_VALIDATOR <session>`
- DO_NOT: run terminal `phase-check CLOSEOUT` or launch Integration Validator while committed handoff evidence is missing
- VERIFY: committed handoff validation passes before Integration Validator launch, and closeout runs only after Integration Validator resolves the final review/verdict response
- SOURCE: GOV-CHANGE-20260429-03, GOV-CHANGE-20260506-03, CX-218K

### RAM-ORCHESTRATOR-MECHANICAL_INTERVENTION-001

- ACTION: CX-218K_MECHANICAL_INTERVENTION
- TRIGGER: stall, handoff delay, relay miss, documentation/protocol drift, or session/ACP drift during orchestrator-managed work
- FAILURE_PATTERN: steering, relaying, or patching after reading one symptom and missing cheaper deterministic truth
- DO: classify 3-5 plausible causes first, including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read or typed helper
- DO_NOT: manually broker ordinary role content, compensate with narrative relay, repeat broad rereads, or add extra prompts when packet/runtime/receipt truth can answer the next action
- VERIFY: the chosen repair names the cause class and updates the mechanical surface, typed receipt, or explicit no-patch rationale
- SOURCE: CX-218K, `.GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md`

### RAM-ORCHESTRATOR-MEMORY_PROPOSAL_REVIEW-001

- ACTION: MEMORY_PROPOSAL_REVIEW
- TRIGGER: Memory Manager emits `MEMORY_PROPOSAL`, `MEMORY_FLAG`, `MEMORY_RGF_CANDIDATE`, or an Actionable Failure Candidate during `ORCHESTRATOR_MANAGED` work
- FAILURE_PATTERN: treating Memory Manager output as self-executing governance authority or leaving repeated-memory proposals unactioned
- DO: inspect the typed receipt plus backup proposal, decide whether the change is a startup brief update, deterministic tooling repair, or governance refactor, then make approved governance edits directly as Orchestrator
- DO_NOT: ask Memory Manager to edit protocols, task boards, Codex law, packets, product code, or validator verdicts
- VERIFY: the chosen outcome is recorded as an Orchestrator decision, implemented by the active authority when accepted, or explicitly rejected/deferred with reason
- SOURCE: STARTUP_BRIEF_SCHEMA, MEMORY_MANAGER_PROTOCOL, Operator correction 2026-04-30
