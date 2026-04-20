# Repo Governance Failure Taxonomy

## Purpose

This document classifies how the current repo-governance kernel fails.
It is the failure-side companion to `Repo_Governance_Capability_Matrix.md`.

The matrix says what capabilities matter.
This taxonomy says how those capabilities currently break in practice.

## Scope Boundary

This document covers repo governance only:

- workflow authority
- session control
- packet and projection truth
- validator routing
- closeout convergence
- audit and artifact upkeep
- token and time burn caused by the control plane

It does not classify Handshake product defects.

## Relationship to Other Research Documents

| Document | Role |
|---|---|
| `Repo_Governance_Capability_Matrix.md` | Shared capability frame |
| `Harness_Lessons_Learned.md` | Cross-cutting lessons and design implications |
| `Gov_Kernel_Technical_Map.md` | Whole-system technical map |
| `ACP_Broker_and_Session_Control.md` | Session-control and broker mechanics |
| `Workflow_State_Packet_Truth_and_Range_Drift.md` | Truth drift deep dive |
| `Validator_Routing_Gates_and_Closeout_Repair.md` | Validator and closeout deep dive |
| `Repo_Governance_Failure_Taxonomy.md` (this file) | Classification of observed failure modes in the current kernel |

## Evidence Sources

This taxonomy should be fed from four evidence types:

- workflow dossiers and postmortems
- gate/check output and repair scripts
- repomem conversation checkpoints
- governance memory fail captures

Important note about repomem:

- repomem is useful for seeing steering context, stale-command-surface discoveries, and recovery reasoning
- repomem is not a complete failure ledger
- many low-level tool and script failures land in procedural memory or audit artifacts instead

So repomem should be treated as one evidence stream, not as the authoritative failure database.

## How to Use This Taxonomy

For each failure class, keep the following distinct:

- what fails
- which surfaces are involved
- whether the failure is mechanical, judgment-bound, or mixed
- what the current repair path is
- whether the failure blocks swarm-style coordination

## Failure Classes

| Failure class | What breaks | Typical symptom | Primary surfaces involved | Current repair mode | Swarm impact |
|---|---|---|---|---|---|
| Truth authority drift | Canonical and projected workflow state disagree | false gate failures, stale blockers, wrong next actor, misleading closeout posture | packet, task board, runtime status, traceability registry, validator gate files | manual or semi-mechanical resync | `Blocked` |
| Command-surface drift and wrong command usage | roles use stale, wrong, or partially superseded commands | startup misroutes, validator lanes wake incorrectly, repair loops start from the wrong surface | protocols, command reference, role prompts, runtime wrappers | manual correction plus sometimes code repair | `Blocked` |
| Session-control residue | broker, request/result ledgers, and registry disagree about what is running or settled | BUSY or RUNNING states persist after real work is done; cancel/close/restart loops | ACP broker, request ledger, result ledger, session registry, broker active runs | self-settle, cancel, restart, registry repair | `Blocked` |
| Lane identity and authority ambiguity | system cannot cleanly prove which role or session may act | wrong-lane writes blocked, validator PASS authority unclear, final-lane preflight failures | actor context, packet lane fields, validator gate state, closeout preflight | manual recovery plus lane-specific re-entry | `Blocked` |
| Communication routing residue | review and handoff state exist, but route state does not converge | pending notifications, stale open-review items, next actor misprojected | receipts, thread, runtime status, communication checks | manual receipt/routing cleanup | `Constrained` |
| Closeout convergence failure | finalization requires too many truths to align at once | repeated closeout retries, pre-repair loops, blocked done/PASS publication | packet, validator gate, session control, signed-scope truth, containment truth | mechanical pre-repair plus manual fixes | `Blocked` |
| Audit and artifact upkeep burden | evidence exists but is expensive to keep live and trustworthy | dossiers lag reality, placeholders accumulate, humans reconstruct runs by hand | workflow dossiers, audits, memory, receipts, patch artifacts | partial sync plus manual patching | `Constrained` |
| Token-burn and retry amplification | control-plane failures multiply cost and time | small diffs consume large token budgets; models spend time on governance archaeology | validator loops, closeout loops, repair retries, session recovery | manual intervention and rerouting | `Blocked` |
| Operator skill dependence | ordinary workflow success depends on orchestrator craft rather than stable runtime behavior | runs recover only because the operator or orchestrator notices the right fix path | all of the above | expert intervention | `Blocked` |

## Taxonomy Details

### 1. Truth Authority Drift

This is the dominant failure family in the current kernel.

It happens when:

- the packet says one thing
- the task board or runtime projection says another
- a gate reads stale upstream truth
- the verdict looks like a product failure even though the underlying product diff is fine

Representative evidence:

- `DOSSIER_20260413_CALENDAR_STORAGE_WORKFLOW_DOSSIER.md`
- `Workflow_State_Packet_Truth_and_Range_Drift.md`
- repomem entries on 2026-04-13 describing repaired packet truth, stale merge-base truth, and resynced handoff truth

Why it matters:

- this is the cleanest example of governance burning time on false failure interpretation instead of product work

### 2. Command-Surface Drift and Wrong Command Usage

This class covers stale launch, startup, or review commands that no longer match the live command surface.

Representative evidence:

- repomem on 2026-04-13: "WP_VALIDATOR startup previously used stale validator command forms"
- operator-observed wrong tool calls and stale wrapper assumptions
- command-surface drift findings already reflected in the calendar-storage and workflow-mirror runs

Why it matters:

- in a multi-role harness, command drift is a coordination bug, not a cosmetic bug
- a wrong command does not just fail locally; it can misroute the whole lane

### 3. Session-Control Residue

This class covers the gap between actual session state and recorded session state.

Typical examples:

- stale active runs
- requests without results
- sessions marked RUNNING after work is effectively done
- broker refusal paths that are technically correct relative to stale ledgers but wrong relative to reality

Representative evidence:

- repomem on 2026-04-10 capturing broker-side stale-run recovery before `BUSY_ACTIVE_RUN`
- `ACP_Broker_and_Session_Control.md`
- `integration-validator-closeout-lib.mjs` closeout bundle checks

Why it matters:

- swarm coordination is impossible if settled and active work cannot be distinguished cheaply

### 4. Lane Identity and Authority Ambiguity

This class appears when the kernel cannot deterministically prove:

- which lane is acting
- whether that actor is allowed to write
- whether final authority is attached to the right validator session

Representative evidence:

- validator gate wrong-lane protections
- final-lane invalidity checks in closeout preflight
- packet and session identity drift during validation and closeout

Why it matters:

- the current kernel is strong on authority law, but expensive when proving lane identity from many surfaces

### 5. Communication Routing Residue

This class is narrower than truth drift.
The work may be technically ready, but the governed communications state is still stale.

Examples:

- open review items linger after the actual response exists
- pending route state blocks the next actor
- direct review boundary state is not reflected in the runtime projection

Representative evidence:

- repomem on 2026-04-14 and 2026-04-10 about stale open review items and misprojected next actor
- `wp-communication-health-check`

Why it matters:

- parallel review loops need routing residue to clear automatically, not by operator memory

### 6. Closeout Convergence Failure

This is not "closeout is bad."
It is the failure class where too many truths must converge before finalization can safely happen.

Current subfamilies include:

- baseline mismatch
- signed-scope artifact drift
- missing verdict projection
- communication health blockage
- final-lane topology and session-control mismatch

Representative evidence:

- `Validator_Routing_Gates_and_Closeout_Repair.md`
- `closeout-repair.mjs`
- integration-validator closeout preflight checks

Why it matters:

- this is where the kernel currently pays a large share of its final token and time tax

### 7. Audit and Artifact Upkeep Burden

This class covers failures where the evidence exists, but maintaining it as live trustworthy state is too expensive.

Examples:

- workflow dossiers with stale placeholders
- evidence spread across too many files
- human reconstruction needed after the fact

Representative evidence:

- distillation and DCC dossiers
- workflow-dossier sync and repomem injection behavior

Why it matters:

- swarm systems need richer evidence, but not if evidence upkeep becomes another control plane to govern

### 8. Token-Burn and Retry Amplification

This class is the economic symptom of the others.

Typical pattern:

- a small product diff is ready or nearly ready
- control-plane drift triggers retries, restarts, re-wakes, or repair loops
- the harness burns time and model budget on governance archaeology

Representative evidence:

- the operator’s stated pain
- calendar-storage recovery
- closeout-repair rationale

Why it matters:

- if the repo kernel is the main time sink, it is not yet fit to serve as the prototype control plane for swarm work

### 9. Operator Skill Dependence

This is the meta-failure class.

The current kernel can often be recovered, but:

- only if the orchestrator notices the right truth surface
- only if the operator understands the topology
- only if someone remembers the correct repair sequence

Representative evidence:

- repeated orchestrator intervention patterns in dossiers and repomem
- the need for closeout-repair and self-settle helpers at all

Why it matters:

- a swarm-ready harness cannot depend on operator craft for routine success

## Early Ranking

These look like the highest-cost classes right now:

1. truth authority drift
2. closeout convergence failure
3. session-control residue
4. command-surface drift
5. operator skill dependence

## Immediate Follow-On Use

This taxonomy should feed:

- `External_Harness_Comparison_Matrix.md`
- `Kernel_to_Swarm_Gap_Map.md`
- `Keep_Adopt_Drop_Decisions.md`

Those documents should reuse these failure classes instead of inventing new categories each time.
