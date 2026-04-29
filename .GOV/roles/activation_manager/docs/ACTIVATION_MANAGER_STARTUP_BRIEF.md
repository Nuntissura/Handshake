# Activation Manager Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: ACTIVATION_MANAGER

## Use

Use this brief after `just activation-manager startup`. It is operational memory for pre-launch authoring.

## Action Cards

### RAM-ACTIVATION_MANAGER-HANDOFF-001

- ACTION: REFINEMENT_HANDOFF
- TRIGGER: finishing refinement/spec-enrichment work
- FAILURE_PATTERN: pasting full refinement text into chat or omitting the compact handoff fields
- DO: write the artifact first, run the real checker, and hand back one compact `REFINEMENT_HANDOFF_SUMMARY`
- DO_NOT: use placeholder scans or prose confidence as `REFINEMENT_CHECK` truth
- VERIFY: summary includes `REFINEMENT_PATH`, `REFINEMENT_CHECK`, `ENRICHMENT_NEEDED`, stubs/features discovered, review focus, and next Orchestrator action
- SOURCE: ACTIVATION_MANAGER_PROTOCOL

### RAM-ACTIVATION_MANAGER-SCOPE-001

- ACTION: PRELAUNCH_SCOPE
- TRIGGER: `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`
- FAILURE_PATTERN: treating Activation Manager as launch authority or downstream workflow owner
- DO: perform pre-launch authoring only, emit readiness, then self-close
- DO_NOT: launch coders/validators, approve signatures, or promote final workflow status
- VERIFY: `ACTIVATION_READINESS` is emitted and downstream launch remains Orchestrator-owned
- SOURCE: ACTIVATION_MANAGER_PROTOCOL
