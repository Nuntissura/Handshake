# Classic Orchestrator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: CLASSIC_ORCHESTRATOR

## Use

Use this brief after `just classic-orchestrator-startup`. It is operational memory for the manual relay lane.

## Action Cards

### RAM-CLASSIC_ORCHESTRATOR-LANE-001

- ACTION: LANE_BOUNDARY
- TRIGGER: operator deliberately chooses `MANUAL_RELAY`
- FAILURE_PATTERN: continuing under orchestrator-managed ACP assumptions after manual relay was selected
- DO: keep the Operator as active relay and use `just manual-relay-next` / `just manual-relay-dispatch` for explicit brokered hops
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

- ACTION: MEMORY_PROPOSAL_REVIEW
- TRIGGER: Memory Manager emits `MEMORY_PROPOSAL`, `MEMORY_FLAG`, `MEMORY_RGF_CANDIDATE`, or an Actionable Failure Candidate that affects `MANUAL_RELAY`
- FAILURE_PATTERN: applying Memory Manager suggestions as if they bypass manual-relay authority, or ignoring repeated-memory proposals because no ACP Orchestrator lane is active
- DO: review the typed receipt and backup proposal, decide whether to update startup brief guidance, create a governance refactor item, or make a manual-lane governance edit as Classic Orchestrator
- DO_NOT: let Memory Manager or Activation Manager become a parallel manual-lane authority for protocol, task-board, packet, or validator-truth changes
- VERIFY: accepted changes are made or queued by Classic Orchestrator, and rejected/deferred proposals record a reason
- SOURCE: STARTUP_BRIEF_SCHEMA, MEMORY_MANAGER_PROTOCOL, CLASSIC_ORCHESTRATOR_PROTOCOL
