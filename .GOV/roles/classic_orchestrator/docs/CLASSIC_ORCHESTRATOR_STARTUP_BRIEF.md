# Classic Orchestrator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: CLASSIC_ORCHESTRATOR

## Use

Use this brief after startup: read the Codex, the Classic Orchestrator protocol and the assigned WP. It is operational memory for the manual relay lane.

## Action Cards

### RAM-CLASSIC_ORCHESTRATOR-MECHANICAL_INTERVENTION-001

- ACTION: MECHANICAL_INTERVENTION
- TRIGGER: before patching, steering, relaying, declaring a stall, or treating handoff/documentation/protocol drift as blocked
- FAILURE_PATTERN: spending manual-relay turns on transcript reconstruction instead of classifying route and artifact drift mechanically
- DO: classify 3-5 plausible causes including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read or repair before relaying content
- DO_NOT: manually broker ordinary role content when a packet artifact, receipt, or runtime status can prove the next action
- VERIFY: the chosen relay or repair cites the cause class and current packet/runtime authority
- SOURCE: CX-218K, CLASSIC_ORCHESTRATOR_PROTOCOL

### RAM-CLASSIC_ORCHESTRATOR-LANE-001

- ACTION: LANE_BOUNDARY
- TRIGGER: operator deliberately chooses `MANUAL_RELAY`
- FAILURE_PATTERN: continuing under orchestrator-managed ACP assumptions after manual relay was selected
- DO: keep the Operator as active relay for explicit brokered hops
- DO_NOT: convert the lane into autonomous ORCHESTRATOR_MANAGED control
- VERIFY: startup and relay output identify `WORKFLOW_LANE=MANUAL_RELAY`
- SOURCE: CLASSIC_ORCHESTRATOR_PROTOCOL

### RAM-CLASSIC_ORCHESTRATOR-PRELAUNCH-001

- ACTION: PRELAUNCH
- TRIGGER: manual-relay refinement, signature, packet, microtask, or worktree prep
- FAILURE_PATTERN: splitting old pre-launch authority between Classic Orchestrator and Activation Manager
- DO: own the combined pre-launch flow in this role unless the Operator explicitly assigns bounded repair/reference work elsewhere
- DO_NOT: create a second manual Activation Manager authority lane
- VERIFY: packet/readiness handoff names Classic Orchestrator as manual-lane owner
- SOURCE: CLASSIC_ORCHESTRATOR_PROTOCOL

### RAM-CLASSIC_ORCHESTRATOR-MEMORY_PROPOSAL_REVIEW-001

Retired with the governance harness on 2026-09-23.
