# Validator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: VALIDATOR

## Use

Use this brief after `just validator-startup VALIDATOR`. It is operational memory for classical manual-relay validation.

## Action Cards

### RAM-VALIDATOR-MECHANICAL_INTERVENTION-001

- ACTION: MECHANICAL_INTERVENTION
- TRIGGER: before manual-relay validation response, final verdict, declaring a stall, or treating handoff/documentation/protocol drift as blocked
- FAILURE_PATTERN: combining manual-relay review and final verdict work through ad hoc prose instead of the packet, receipts, and validator helpers
- DO: classify 3-5 plausible causes including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read, validation helper, or typed response before relaying verdict content
- DO_NOT: manually relay ordinary review content when a packet check, validator-next output, notification ack, or review-response helper owns the state transition
- VERIFY: validation output names the cause class, packet/runtime authority, helper result, and evidence needed for PASS/FAIL
- SOURCE: CX-218K, VALIDATOR_PROTOCOL

### RAM-VALIDATOR-MANUAL_RELAY-001

- ACTION: MANUAL_RELAY_VALIDATION
- TRIGGER: validating a manual-relay packet
- FAILURE_PATTERN: assuming split WP Validator / Integration Validator behavior when the packet is classical
- DO: combine early-review discipline and final verdict rigor in the classical Validator lane
- DO_NOT: ask for autonomous orchestrator-managed steering unless the packet explicitly declares it
- VERIFY: `just validator-next VALIDATOR WP-{ID}` and `just external-validator-brief WP-{ID}` provide the active route
- SOURCE: VALIDATOR_PROTOCOL

### RAM-VALIDATOR-EVIDENCE-001

- ACTION: EVIDENCE
- TRIGGER: preparing a validation report
- FAILURE_PATTERN: accepting test output or coder summaries without independent file:line proof
- DO: map packet/spec requirements to code, tests, and negative/counterfactual evidence
- DO_NOT: pass with debt for hard invariants, security, traceability, or spec alignment
- VERIFY: report includes a spec clause map with concrete citations
- SOURCE: VALIDATOR_PROTOCOL
