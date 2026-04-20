# Repo Governance Capability Matrix

## Purpose

This document is the comparison spine for repo-governance research.
Its job is to give the current Handshake repo-governance kernel and external harnesses one shared evaluation frame.

This document is not:

- a product architecture document
- a freeform lessons log
- a protocol replacement
- a redesign decision record

It is the place where repo-governance capabilities are compared systematically.

## Scope Boundary

This matrix is about repo governance only:

- workflow authority
- ACP and session control
- validator flow
- work transfer
- artifact hygiene
- control-plane truth
- stall recovery
- swarm readiness

It is not about Handshake product features.
The product matters later only as the destination that may absorb these governance capabilities.

## Relationship to Other Research Documents

| Document | Role |
|---|---|
| `Harness_Lessons_Learned.md` | Failure-backed lessons and architecture implications |
| `Gov_Kernel_Technical_Map.md` | Whole-system map of the current kernel |
| `ACP_Broker_and_Session_Control.md` | Deep dive on broker and session-control mechanics |
| `Workflow_State_Packet_Truth_and_Range_Drift.md` | Deep dive on truth drift and false-failure mechanics |
| `Validator_Routing_Gates_and_Closeout_Repair.md` | Deep dive on validator and closeout mechanics |
| `Repo_Governance_Capability_Matrix.md` (this file) | Shared capability frame for current-kernel and external-harness comparison |
| `Technical_Implementation_Research.md` | External implementation research to be mapped back into this matrix |

## How to Use This Matrix

Use this file in four passes:

1. Record the current kernel capability shape.
2. Attach failure evidence from dossiers, patches, checks, and runtime artifacts.
3. Compare external harnesses against the same rows.
4. Convert the matrix into keep/adopt/drop decisions.

Each row should stay grounded in:

- capability intent
- current mechanism
- current weakness
- swarm relevance

## Rating Guide

Use these values consistently:

- `Strong` = capability exists and is usually reliable
- `Partial` = capability exists but is costly, brittle, or incomplete
- `Brittle` = capability exists but frequently misbehaves under ordinary workflow load
- `Missing` = capability is absent or only implied

For swarm readiness:

- `Ready` = can likely scale without redesign
- `Constrained` = usable only with significant operator oversight or strict limits
- `Blocked` = current shape will fail under swarm-style parallelism

## Capability Matrix

| Capability | Why it matters | Current kernel mechanism | Current status | Swarm readiness | Primary weaknesses seen so far | Evidence anchors | External comparison questions |
|---|---|---|---|---|---|---|---|
| Workflow authority and lane boundaries | Parallel execution fails quickly if authority is ambiguous | Codex law, role protocols, lane split between `ORCHESTRATOR`, `CLASSIC_ORCHESTRATOR`, `ACTIVATION_MANAGER`, `WP_VALIDATOR`, and `INTEGRATION_VALIDATOR` | `Strong` | `Constrained` | authority is well-defined in law, but enforcement still depends on many runtime surfaces staying aligned | `Handshake_Codex_v1.4.md`; role protocols; validator closeout library | How do other harnesses encode authority so runtime state drift cannot silently blur lanes? |
| Pre-launch governance authoring | Swarm-safe execution needs a clear packet and launch contract before agents act | Activation Manager lane, packet hydration, refinement, signature capture, packet prep | `Partial` | `Constrained` | pre-launch structure exists, but packet quality and hydration burden are still high-cost | Activation Manager protocol; task packets; startup flows | Which harnesses minimize pre-launch authoring while preserving execution clarity? |
| Session launch, steer, cancel, and close | Swarm work needs reliable lifecycle control over many parallel sessions | ACP broker, session-control request/result ledgers, session registry, session-control command flow | `Partial` | `Blocked` | broker and ledgers exist, but residue, unsettled results, and recovery cost are still major failure sources | `ACP_Broker_and_Session_Control.md`; session-control scripts; dossiers | Which harnesses make session lifecycle mechanically convergent after failure or timeout? |
| Session identity and runtime registry truth | Parallel work needs unambiguous actor identity and ownership | `ROLE_SESSION_REGISTRY.json`, broker state, session policy, terminal ownership tracking | `Partial` | `Blocked` | registry is durable, but it can disagree with results, active runs, and actual runtime state | session-registry lib; closeout library; session governance checks | How do other systems keep session identity authoritative without multi-ledger disagreement? |
| Workflow truth authority versus mirrors | Swarm work collapses when projections become accidental authorities | packet, task board, runtime status, receipts, traceability registry, mirror contracts, projection checks | `Brittle` | `Blocked` | too many overlapping truth surfaces; packet and mirrors drift; gates read stale truth and create false failures | `Workflow_State_Packet_Truth_and_Range_Drift.md`; spec mirror clauses; calendar-storage dossier | Which harnesses reduce workflow truth to fewer canonical records while keeping readable views? |
| Work transfer, handoff, and announce-back | Swarm coordination depends on replay-safe transfer of state between actors | WP communications, receipts, thread history, packet notes, handoff views, role-boundary rules | `Partial` | `Constrained` | current repo kernel has several handoff surfaces, but handoff durability and transcription still cost manual effort | WP communications law; dossier evidence; spec handoff clauses | How do other harnesses make handoff compact, durable, and safe without transcript replay? |
| Validator layering and final authority | Multi-model workflows need advisory review separated from final authority | `WP_VALIDATOR` versus `INTEGRATION_VALIDATOR`, validator-governance lib, gate model | `Strong` | `Constrained` | split is good, but routing and lane identity drift can still block the whole path | `Validator_Routing_Gates_and_Closeout_Repair.md`; validator protocols | How do other harnesses preserve layered review without adding heavy routing overhead? |
| Closeout, finalization, and authoritative completion | Swarm systems need one durable convergence step before declaring work done | closeout checks, integration-validator closeout library, closeout repair, gate commit clearance | `Partial` | `Blocked` | semantics are important, but current closeout is burdened by truth repair and control-plane residue | closeout library; `closeout-repair.mjs`; spec handoff/completion clauses | Which harnesses preserve authoritative completion while reducing pre-closeout repair burden? |
| Mechanical governance checks and repair bundles | Deterministic checks are required to keep AI agents inside bounds | `phase-check`, packet-truth checks, communication checks, closeout-repair, validator-gate ops | `Strong` | `Constrained` | mechanical check coverage is strong, but the bundle can become expensive when too many upstream surfaces drift | `phase-check.mjs`; shared checks; closeout-repair | Which harnesses keep deterministic governance without creating a large operator repair tax? |
| Stall detection and recovery | Swarm execution needs fast detection of stuck runs, stale claims, and bad waits | orchestrator-next, steer-next, communication health, self-settle logic, terminal reclaim, fail capture | `Partial` | `Constrained` | recovery tools exist, but ordinary runs still depend heavily on orchestrator intervention skill | orchestrator commands; self-settle lib; dossiers | Which harnesses convert recovery from operator craft into normal runtime behavior? |
| Artifact hygiene and audit surfaces | Swarm work creates many artifacts; without hygiene, evidence becomes noise | live dossiers, audits, fail capture, memory, artifact hygiene checks, retention manifests | `Partial` | `Constrained` | the kernel records a lot, but some artifacts remain expensive to keep live and truthy | workflow dossiers; memory system; artifact hygiene checks | Which harnesses keep evidence rich without creating a second bureaucracy of audit upkeep? |
| Token, time, and retry control | Swarm coordination fails economically before it fails technically if retries are unbounded | per-MT stop pattern, validator loops, closeout repair, model profile catalog, escalation policy | `Partial` | `Blocked` | explicit cost awareness exists, but ordinary workflows still burn large budgets on repair and retries | lessons learned; codex clauses; dossiers | Which harnesses make retry, escalation, and provider selection runtime-native and budget-aware? |
| Parallel work coordination | The whole goal is safe multi-session and later swarm execution | worktree rules, session orchestration, lane split, validator layering, shared communications | `Partial` | `Blocked` | current kernel can coordinate some parallel roles, but it is not yet robust enough for swarm-style concurrency | Gov kernel map; session-control docs; validator docs | What mechanisms do other harnesses use for concurrent claiming, transfer, and conflict containment? |
| Manual relay and degraded-mode fallback | A serious harness needs a fast fallback when autonomy is unhealthy | `CLASSIC_ORCHESTRATOR`, manual relay lane, operator relay practices | `Strong` | `Ready` | fallback exists, but it is not yet normalized as a first-class design target in the research set | codex lane-boundary clauses; operator observations | Which harnesses preserve one state model across autonomous and manual relay modes? |

## Current Pattern Summary

Three early conclusions stand out:

- the current kernel is strongest at explicit authority definition and deterministic checks
- the current kernel is weakest where many ledgers must converge before action is safe
- the current kernel is not swarm-ready until workflow truth, session lifecycle, and closeout convergence become cheaper and more canonical

## Immediate Follow-On Documents

This matrix should drive the next research wave:

- `Repo_Governance_Failure_Taxonomy.md`
- `Kernel_to_Swarm_Gap_Map.md`
- `External_Harness_Comparison_Matrix.md`
- `Keep_Adopt_Drop_Decisions.md`

Those documents should reuse these capability rows rather than inventing new comparison categories ad hoc.
