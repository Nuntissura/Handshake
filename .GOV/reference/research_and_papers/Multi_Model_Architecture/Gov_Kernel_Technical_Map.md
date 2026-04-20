# Gov Kernel Technical Map

## Purpose

This document is the technical map of the current Handshake repo-governance kernel.
Its job is to explain what the kernel actually is, what surfaces it uses, where truth lives,
how governed workflow execution moves through the system, and where the control plane is fragile.

This is not a README and not a protocol replacement.
It is the system map that should let a future redesign answer:

- what exists now
- what the critical paths are
- what the failure hotspots are
- what should be retained, replaced, or deleted

## Relationship to Other Research Documents

| Document | Role |
|---|---|
| `Harness_Lessons_Learned.md` | Cross-cutting lessons and architecture implications |
| `Repo_Governance_Capability_Matrix.md` | Shared capability frame for current-kernel and external-harness comparison |
| `Repo_Governance_Failure_Taxonomy.md` | Shared classification of how the current kernel fails |
| `Kernel_to_Swarm_Gap_Map.md` | Prioritized blocker map between the current kernel and swarm-capable governance |
| `Gov_Kernel_Technical_Map.md` (this file) | Whole-system map of the current governance kernel |
| `ACP_Broker_and_Session_Control.md` | Deep dive on launch, session control, broker, and health |
| `Workflow_State_Packet_Truth_and_Range_Drift.md` | Deep dive on workflow truth, packet truth, and false gate failures |
| `Validator_Routing_Gates_and_Closeout_Repair.md` | Deep dive on validator routing, gate enforcement, and closeout preparation |
| `Technical_Implementation_Research.md` | External harness implementation research |

## Current System Boundary

The current gov kernel is not one script. It is a control plane spread across:

- repo-local governance sources under `.GOV/`
- governed command entrypoints in `justfile`
- role protocols under `.GOV/roles/*`
- shared checks and runtime libraries under `.GOV/roles_shared/*`
- runtime state and lane artifacts under `../gov_runtime`
- governance memory and audit surfaces

At a high level, the kernel is trying to do five jobs at once:

1. define authority and allowed workflow behavior
2. create and route governed work
3. launch and steer role sessions
4. mechanically verify workflow truth
5. preserve an audit trail of what happened

## Directory-Level Map

### `.GOV/`

This is the authored governance source tree in the worktree.

| Path | Role in the kernel |
|---|---|
| `.GOV/Audits/` | Live dossiers, smoketest reviews, patch evidence, and post-run audit artifacts |
| `.GOV/codex/` | Codex authority and governance operating rules |
| `.GOV/docs/` and `.GOV/docs_repo/` | Governance and repo-facing documentation surfaces |
| `.GOV/operator/` | Operator-facing prompts, scratchpads, and local operating material |
| `.GOV/reference/` | Research, architectural references, and long-form study material |
| `.GOV/refinements/` | Refinement-stage artifacts and planning surfaces |
| `.GOV/roles/` | Role-specific protocols, scripts, tests, and checks |
| `.GOV/roles_shared/` | Shared checks, libraries, schemas, runtime helpers, and records |
| `.GOV/spec/` | Current spec references and spec-linked governance anchors |
| `.GOV/task_packets/` | Signed work packets and packet folders |
| `.GOV/templates/` | Reusable templates for governance artifacts |
| `.GOV/tools/` | Supporting governance tools and utilities |

### `.GOV/roles_shared/`

This is the shared governance platform layer inside the repo.

| Path | Role in the kernel |
|---|---|
| `.GOV/roles_shared/checks/` | Deterministic governance checks such as `phase-check`, packet truth, session runtime, and merge truth |
| `.GOV/roles_shared/docs/` | Shared operational doctrine, invariants, and guardrails |
| `.GOV/roles_shared/exports/` | Exported/generated governance outputs |
| `.GOV/roles_shared/fixtures/` | Test and development fixtures |
| `.GOV/roles_shared/records/` | Canonical records such as task board and build-order surfaces |
| `.GOV/roles_shared/runtime/` | Shared runtime-related repo-local code and helpers |
| `.GOV/roles_shared/schemas/` | Schema definitions for structured artifacts |
| `.GOV/roles_shared/scripts/` | Audit, session, topology, WP, and library scripts |
| `.GOV/roles_shared/tests/` | Regression tests for shared governance logic |

### `../gov_runtime/roles_shared/`

This is the live runtime state and ledger area outside the worktree.

| Path | Role in the kernel |
|---|---|
| `GATE_OUTPUTS/` | Persisted outputs from deterministic governance gates |
| `SESSION_CONTROL_OUTPUTS/` | Session stdout/stderr or structured output captures |
| `SESSION_MONITORS/` | Monitor state and activity snapshots |
| `validator_gates/` | Validator-side gate outputs and verdict-side artifacts |
| `WP_COMMUNICATIONS/` | Per-WP receipts, thread logs, status projections, and governed mailboxes |
| `WP_COMMUNICATIONS_ARCHIVE/` | Archived WP communication trails |
| `WP_TOKEN_USAGE/` | Token usage ledgers and budget tracking artifacts |
| `ROLE_SESSION_REGISTRY.json` | Live role-session registry |
| `SESSION_CONTROL_BROKER_STATE.json` | Broker state snapshot |
| `SESSION_CONTROL_REQUESTS.jsonl` | Session control requests ledger |
| `SESSION_CONTROL_RESULTS.jsonl` | Session control results ledger |
| `SESSION_LAUNCH_REQUESTS.jsonl` | Launch queue / host handoff ledger |
| `OPERATOR_ALERT_QUEUE.jsonl` | Operator alert queue surface |
| `GOVERNANCE_MEMORY.db` | Durable governance memory store |
| `ORCHESTRATOR_GATES.json` | Orchestrator gate state summary |
| `broker_stdout.log` and `broker_stderr.log` | Broker process logs |

## Kernel Layers

### 1. Authority Layer

These files tell a role what it is allowed to do:

- `.GOV/codex/Handshake_Codex_v1.4.md`
- `../handshake_main/AGENTS.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `.GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`
- `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md`

This layer is strong on declared authority.
It is weaker where runtime behavior depends on state that is not directly encoded in the protocol text.

### 2. Command Surface Layer

The `justfile` is the human and machine command surface for the kernel.
Important entrypoints include:

- `just orchestrator-startup`
- `just orchestrator-next`
- `just orchestrator-steer-next`
- `just orchestrator-prepare-and-packet`
- `just validator-startup <ROLE>`
- `just validator-next <ROLE> [WP-ID]`
- `just phase-check <PHASE> <WP-ID> ...`
- `just closeout-repair <WP-ID>`
- `just workflow-dossier-init <WP-ID>`
- `just workflow-dossier-note <WP-ID> ...`
- `just workflow-dossier-sync <WP-ID>`

This layer is critical because command-surface drift can create false failures even when product code is fine.

### Command-to-Script Map

The main governed entrypoints currently resolve like this:

| Command | Immediate behavior | Main script surface |
|---|---|---|
| `just workflow-dossier-init WP-{ID}` | Initializes live dossier output for a WP | `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs` (`init`) |
| `just workflow-dossier-note WP-{ID} ...` | Appends execution/governance notes into the live dossier | `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs` (`note`) |
| `just workflow-dossier-sync WP-{ID}` | Rebuilds or refreshes dossier-derived state from runtime artifacts | `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs` (`sync`) |
| `just orchestrator-startup` | Protocol ack, topology check, preflight, memory refresh/recall, memory-manager launch, and session-open reminder | `justfile` startup chain plus orchestrator protocol/check surfaces |
| `just orchestrator-next [WP-ID]` | Enforces repomem gate, recalls resume memory, then advances orchestrator logic | `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs` |
| `just orchestrator-steer-next WP-{ID} "<context>"` | Enforces repomem gate, records steering context, then resumes/steers the governed lane | `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs` |
| `just orchestrator-prepare-and-packet WP-{ID}` | Adds worktree, installs MT hook, recalls delegation memory, then prepares runtime packet state | `.GOV/roles/orchestrator/scripts/orchestrator-prepare-and-packet.mjs` |
| `just validator-startup <ROLE>` | Protocol ack, topology/preflight checks, memory refresh/recall, and session-open reminder for validator roles | `justfile` startup chain plus validator protocol/check surfaces |
| `just validator-next <ROLE> [WP-ID]` | Recalls validator resume memory then advances validator logic | `.GOV/roles/validator/scripts/validator-next.mjs` |
| `just closeout-repair WP-{ID}` | Repairs closeout-side workflow truth before final closeout checks | `.GOV/roles/orchestrator/scripts/closeout-repair.mjs` |
| `just phase-check <PHASE> WP-{ID} ...` | Runs the deterministic phase gate for startup/handoff/verdict/closeout | `.GOV/roles_shared/checks/phase-check.mjs` |

The important architectural point is that the human command surface is thin.
Most commands are wrappers that compose memory gates, policy expectations, and one or more node scripts.
That makes the command surface easy to use, but also makes drift between wrapper expectations and runtime scripts expensive.

### 3. Workflow Artifact Layer

These are the authored and derived artifacts that describe the intended work:

- `.GOV/refinements/*`
- `.GOV/task_packets/*`
- `.GOV/roles_shared/records/TASK_BOARD.md`
- packet and refinement metadata projected into runtime state
- merge progression truth and closure monitoring artifacts

This layer is where workflow intent, scope, and signed contract live.
It is also where stale or partially repaired truth can poison later checks.

### 4. Session and Runtime Layer

This is the operational layer that launches and steers governed sessions.
Key surfaces include:

- `.GOV/roles_shared/scripts/session/session-policy.mjs`
- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-broker.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-cancel.mjs`
- `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-stall-scan.mjs`
- `.GOV/roles_shared/scripts/session/session-governance-state-lib.mjs`
- `.GOV/roles_shared/scripts/session/terminal-ownership-lib.mjs`
- `.GOV/roles_shared/scripts/session/handshake-acp-client.mjs`

Runtime state is largely materialized in `../gov_runtime/roles_shared/`:

- `ROLE_SESSION_REGISTRY.json`
- `SESSION_CONTROL_BROKER_STATE.json`
- `SESSION_CONTROL_REQUESTS.jsonl`
- `SESSION_CONTROL_RESULTS.jsonl`
- `SESSION_CONTROL_OUTPUTS/`
- `WP_COMMUNICATIONS/<WP-ID>/...`

### 5. Mechanical Governance Layer

This is the check runner layer that tries to keep narrative drift from becoming control-plane drift.
Key checks include:

- `.GOV/roles_shared/checks/gov-check.mjs`
- `.GOV/roles_shared/checks/phase-check.mjs`
- `.GOV/roles_shared/checks/packet-truth-check.mjs`
- `.GOV/roles_shared/checks/packet-closure-monitor-check.mjs`
- `.GOV/roles_shared/checks/session-policy-check.mjs`
- `.GOV/roles_shared/checks/session-launch-runtime-check.mjs`
- `.GOV/roles_shared/checks/session-control-runtime-check.mjs`
- `.GOV/roles_shared/checks/merge-progression-truth-check.mjs`
- `.GOV/roles_shared/checks/wp-communications-check.mjs`

This is one of the strongest ideas in the current kernel.
It is also one of the biggest time and token multipliers when the truth surfaces feeding the checks are stale.

### 6. Audit and Memory Layer

The kernel also tries to preserve operator-visible and post-run truth:

- `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
- `.GOV/roles_shared/scripts/audit/workflow-dossier-lib.mjs`
- `.GOV/Audits/smoketest/*`
- governance memory via `repomem` and fail-capture hooks

This layer is valuable when it is mechanically fed.
It becomes overhead when large sections remain manual placeholder upkeep.

## Governed WP Critical Path

The current governed path is approximately:

1. Orchestrator startup establishes authority, topology, startup truth, and mandatory session-open memory checkpoint.
2. Work packet preparation and signature/model-profile capture define the governed execution context.
3. Activation or readiness work completes and packet/runtime surfaces are prepared.
4. The orchestrator launches or resumes the role lane through the ACP/session-control stack.
5. The coder and validators communicate through governed WP communications surfaces and runtime ledgers.
6. Mechanical checks evaluate packet truth, handoff truth, verdict truth, and closeout truth.
7. Dossier and memory surfaces record the run.
8. Final closeout converges packet state, merge truth, runtime truth, and validator outcome.

This path is correct in principle.
In practice, many delays come from disagreement between the authored workflow artifacts and the runtime/mechanical view of those artifacts.

## Critical Control Surfaces to Map in Later Passes

The next pass on this document should add script-by-script detail for:

### Work packet lifecycle

- packet creation
- refinement linkage
- packet hydration and prep
- handoff sections and evidence sections
- closeout synchronization

### Session lifecycle

- launch request generation
- broker dispatch
- registry updates
- output capture
- health checks
- cancel / resume / reclaim flows

### Communication lifecycle

- governed receipts
- thread updates
- runtime status updates
- validator handoff and response flow

### Audit lifecycle

- dossier init
- note sync
- metric autofill
- post-run truth preservation

## Known Failure Hotspots

Based on current dossiers and recovery artifacts, the highest-value failure areas are:

### 1. Workflow truth drift

The system can lose agreement about:

- current phase
- active MT
- intended diff range
- current merge containment state
- which packet fields are authoritative

### 2. Command-surface drift

The declared command shape can drift from the actual `just` surface, causing startup or resume failures that are governance-only.

### 3. Session-control brittleness

The ACP/session-control layer appears vulnerable to host load, stalled sessions, and weak operator visibility outside the terminal.

### 4. Audit labor inflation

The live dossier concept is good, but manual upkeep can become another workflow tax unless most fields are derived mechanically.

### 5. Truth repair cost

Once packet truth, range truth, or closeout truth drift, repair often requires multi-step orchestrator intervention and more governance work than product work.

## First Evidence Anchors

The first pass should repeatedly cross-reference these artifacts:

- `.GOV/Audits/smoketest/DOSSIER_20260413_CALENDAR_STORAGE_WORKFLOW_DOSSIER.md`
- `.GOV/Audits/smoketest/DOSSIER_20260414_DISTILLATION_WORKFLOW_DOSSIER.md`
- `.GOV/Audits/smoketest/DOSSIER_20260411_DEV_COMMAND_CENTER_CONTROL_PLANE_BACKEND_WORKFLOW_DOSSIER.md`
- `.GOV/Audits/smoketest/WP-1-Calendar-Storage-v2-MISROUTED_MAIN_DIFF-20260413T123133Z.patch`
- `.GOV/Audits/smoketest/WP-1-Calendar-Storage-v2-CANDIDATE_TARGET-066cc18d.patch`

## What the Redesign Should Preserve

The current kernel has several ideas worth preserving:

- explicit role authority and startup discipline
- mechanical checks for workflow truth
- dedicated runtime ledgers and communications artifacts
- a durable audit trail
- the distinction between workflow governance and product implementation

## What the Redesign Should Challenge

The next harness should challenge these assumptions:

- that the conversation is a reliable place to hold workflow state
- that all governance artifacts need equal manual authoring depth
- that autonomous orchestration should be the only first-class operating mode
- that more checks always improve throughput
- that repair-heavy recovery is acceptable for normal execution

## Open Questions

1. Which kernel truths should be authored by humans, and which should always be derived mechanically?
2. What is the minimum operator console needed so the operator is not reduced to terminal babysitting?
3. Which parts of the current kernel are reusable in a swarm harness, and which are testbed scaffolding only?
4. Can manual relay and autonomous orchestration share one durable state model?
5. Which checks are high-signal and which are governance noise?

## Next Deepening Pass

The next pass on this document should add:

- a lifecycle sequence for one governed WP from startup to closeout
- a "retain / replace / remove" table for major kernel components
