# Orchestrator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: ORCHESTRATOR

## Use

Use this brief after reading the Codex, this protocol and the assigned MT, and before governed mutation. It is operational memory, not authority.

## Action Cards

### RAM-ORCHESTRATOR-SESSION_OPEN-001

Retired with the governance harness on 2026-09-23.

### RAM-ORCHESTRATOR-MECHANICAL_GOVERNANCE-001

Retired with the governance harness on 2026-09-23.

### RAM-ORCHESTRATOR-CLOSEOUT-001

- ACTION: CLOSEOUT_PREP
- TRIGGER: before launching Integration Validator
- FAILURE_PATTERN: launching final judgment with broken mechanical closeout truth, causing repair loops and stale-session drift
- DO: verify final `CODER_HANDOFF` committed target evidence (pushed `<base>..<head>` range) before launching Integration Validator
- DO_NOT: run terminal closeout or launch Integration Validator while committed handoff evidence is missing
- VERIFY: committed handoff validation passes before Integration Validator launch, and closeout runs only after Integration Validator resolves the final review/verdict response
- SOURCE: GOV-CHANGE-20260429-03, GOV-CHANGE-20260506-03, CX-218K

### RAM-ORCHESTRATOR-MECHANICAL_INTERVENTION-001

- ACTION: CX-218K_MECHANICAL_INTERVENTION
- TRIGGER: stall, handoff delay, relay miss, documentation/protocol drift, or session/ACP drift during orchestrator-managed work
- FAILURE_PATTERN: steering, relaying, or patching after reading one symptom and missing cheaper deterministic truth
- DO: classify 3-5 plausible causes first, including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read
- DO_NOT: manually broker ordinary role content, compensate with narrative relay, repeat broad rereads, or add extra prompts when packet/runtime/receipt truth can answer the next action
- VERIFY: the chosen repair names the cause class and updates the mechanical surface, typed receipt, or explicit no-patch rationale
- SOURCE: CX-218K, `.GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md`

### RAM-ORCHESTRATOR-MEMORY_PROPOSAL_REVIEW-001

Retired with the governance harness on 2026-09-23.
