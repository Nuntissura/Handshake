# WP Validator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: WP_VALIDATOR

## Use

Use this brief after `just validator-startup WP_VALIDATOR`. It is operational memory for per-WP review.

## Action Cards

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
