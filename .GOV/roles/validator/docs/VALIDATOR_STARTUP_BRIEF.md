# Validator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: VALIDATOR

## Use

Use this brief after `just validator-startup VALIDATOR`. It is operational memory for classical manual-relay validation.

## Action Cards

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
