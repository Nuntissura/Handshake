# Activation Manager Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: ACTIVATION_MANAGER

## Use

Use this brief after `just activation-manager startup`. It is operational memory for pre-launch authoring.

## Action Cards

### RAM-ACTIVATION_MANAGER-MECHANICAL_INTERVENTION-001

- ACTION: MECHANICAL_INTERVENTION
- TRIGGER: before patching, handing off refinement/readiness, declaring pre-launch blocked, or treating documentation/protocol drift as unresolved
- FAILURE_PATTERN: using prose confidence or repeated model turns when packet, readiness, runtime, or checker output can identify the real pre-launch blocker
- DO: classify 3-5 plausible causes including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read, readiness repair, or typed helper before asking Orchestrator to relay
- DO_NOT: invent a second downstream authority lane or manually relay ordinary handoff content when the packet/refinement/readiness helper can write the authority artifact
- VERIFY: the handoff names the cause class, artifact path, checker result, and next Orchestrator-owned action
- SOURCE: CX-218K, ACTIVATION_MANAGER_PROTOCOL, .GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md

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
