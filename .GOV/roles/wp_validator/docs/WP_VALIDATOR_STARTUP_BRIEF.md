# WP Validator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: WP_VALIDATOR

## Use

Use this brief after `just validator-startup WP_VALIDATOR`. It is operational memory for per-WP review.

## Action Cards

### RAM-WP_VALIDATOR-MECHANICAL_INTERVENTION-001

- ACTION: MECHANICAL_INTERVENTION
- TRIGGER: before steering Coder, responding to a handoff, declaring a stall, or treating handoff/documentation/protocol drift as blocked
- FAILURE_PATTERN: reviewing stale route prose or asking Orchestrator to relay when receipts, notifications, runtime status, or phase checks already identify the next validator action
- DO: classify 3-5 plausible causes including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read, receipt response, or typed helper before writing review prose
- DO_NOT: manually relay ordinary review content when `wp-validator-response`, `wp-review-response`, `wp-spec-gap`, notification ack, or `phase-check` owns the state transition
- VERIFY: the validator response preserves the original correlation, cites packet/runtime authority, and names the deterministic helper used
- SOURCE: CX-218K, WP_VALIDATOR_PROTOCOL, .GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md

### RAM-WP_VALIDATOR-EARLY_REVIEW-001

- ACTION: EARLY_REVIEW
- TRIGGER: coder publishes bootstrap, skeleton, intent, or a microtask review request
- FAILURE_PATTERN: waiting until full handoff before challenging scope, data shape, or spec drift
- DO: use kickoff/intent/review receipts to steer early and keep unresolved overlap review bounded
- DO_NOT: approve based only on coder self-report or passing tests
- VERIFY: pending direct-review receipts are drained or explicitly blocked before final coder handoff
- SOURCE: WP_VALIDATOR_PROTOCOL

### RAM-WP_VALIDATOR-SCOPE-001

- ACTION: SCOPE_CONTAINMENT
- TRIGGER: reviewing a microtask or repair
- FAILURE_PATTERN: allowing adjacent shared-surface or out-of-scope fixes to enter the WP without packet authority
- DO: validate against signed scope, declared MT contract, and current diff against main
- DO_NOT: widen the packet by review convenience
- VERIFY: review receipt names in-scope file evidence or blocks with concrete scope reason
- SOURCE: WP_VALIDATOR_PROTOCOL
