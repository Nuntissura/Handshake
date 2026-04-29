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
- DO: run `just closeout-repair WP-{ID}` then `just phase-check CLOSEOUT WP-{ID}` before Integration Validator launch
- DO_NOT: launch Integration Validator while mechanical truth is broken
- VERIFY: closeout repair and closeout phase check both pass or a single manual remediation/escalation is recorded
- SOURCE: GOV-CHANGE-20260429-03
