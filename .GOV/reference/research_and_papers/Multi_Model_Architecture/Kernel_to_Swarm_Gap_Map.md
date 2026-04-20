# Kernel to Swarm Gap Map

## Purpose

This document translates current repo-governance capability and failure evidence into swarm-readiness blockers.

It is not trying to redesign the kernel in one pass.
It is trying to answer a narrower question:

- what prevents the current repo-governance kernel from scaling into reliable swarm-style coordination

This is the synthesis layer above:

- `Repo_Governance_Capability_Matrix.md`
- `Repo_Governance_Failure_Taxonomy.md`

## Scope Boundary

This document is about repo governance only:

- orchestrator-managed workflow
- session lifecycle
- validator layering
- handoff and work transfer
- authoritative completion
- evidence and artifact hygiene
- swarm-readiness blockers

It is not about Handshake product features.

## Relationship to Other Research Documents

| Document | Role |
|---|---|
| `Repo_Governance_Capability_Matrix.md` | Capability comparison spine |
| `Repo_Governance_Failure_Taxonomy.md` | Failure classification spine |
| `Harness_Lessons_Learned.md` | Cross-cutting lessons and implications |
| `Gov_Kernel_Technical_Map.md` | Whole-system map of the current kernel |
| `ACP_Broker_and_Session_Control.md` | Session-control mechanics |
| `Workflow_State_Packet_Truth_and_Range_Drift.md` | Truth drift deep dive |
| `Validator_Routing_Gates_and_Closeout_Repair.md` | Validator and closeout deep dive |
| `Kernel_to_Swarm_Gap_Map.md` (this file) | Prioritized blocker map from current kernel to swarm-capable governance |

## How to Read This Document

Each row answers:

- what the kernel can do now
- why that is not enough for swarm-style coordination
- what failure classes prove the gap is real
- what direction the next harness likely needs

The point is not to decide implementation details yet.
The point is to identify which gaps are true blockers versus tolerable constraints.

## Severity Guide

- `Tier 1` = hard swarm blocker; current shape is not viable without redesign
- `Tier 2` = serious constraint; small-scale autonomy possible, swarm scale still unsafe or too costly
- `Tier 3` = useful but non-blocking improvement; can wait until the core blockers move

## Gap Map

| Capability area | Current kernel posture | Main failure classes | Why this blocks swarm use | Likely direction | Severity | Evidence anchors |
|---|---|---|---|---|---|---|
| Workflow truth authority | many overlapping truth surfaces with packet, projections, runtime, and gate outputs all participating | truth authority drift; communication routing residue; operator skill dependence | swarm coordination cannot rely on humans deciding which truth surface is stale on each failure | reduce to fewer canonical records with explicit projection layers | `Tier 1` | capability matrix rows on workflow truth; truth-drift deep dive; calendar-storage dossier |
| Session lifecycle convergence | ACP/session-control is real, but settled versus active state still drifts | session-control residue; token-burn amplification | swarm work needs cheap, reliable convergence after timeouts, cancellation, and partial failure | make session state mechanically convergent with one authoritative lifecycle object | `Tier 1` | ACP deep dive; repomem 2026-04-10 stale-run recovery insight; closeout bundle checks |
| Final authority and closeout convergence | final-lane semantics are defensible, but current convergence requires many surfaces to align | closeout convergence failure; lane identity ambiguity; truth authority drift | a swarm cannot pay repeated closeout repair cost just to publish authoritative completion | preserve final authority while shrinking pre-closeout reconciliation burden | `Tier 1` | validator/closeout deep dive; closeout-repair; spec-aligned handoff/completion anchors |
| Command-surface stability | command family is documented, but stale forms and wrappers still leak into active runs | command-surface drift and wrong command usage | in a swarm, stale commands become systemic misroutes rather than isolated slips | move toward tighter canonical command families with stronger drift detection | `Tier 1` | repomem 2026-04-13 stale validator command forms; command surface reference; workflow-mirror evidence |
| Handoff and work transfer | several governed handoff surfaces exist, but transfer still needs too much runtime interpretation | truth authority drift; communication routing residue; audit burden | swarm work depends on compact transfer of remaining work and next actor without transcript replay or manual transcription repair | converge on compact, structured transfer artifacts with authoritative transcription state | `Tier 2` | spec handoff anchors; WP communications law; dossiers; truth-drift deep dive |
| Validator layering | authority split is one of the kernel’s strongest design decisions | lane identity ambiguity; command-surface drift | the split itself is not the blocker, but the cost of proving which validator may act still scales badly | keep layered validator authority, reduce routing and identity burden around it | `Tier 2` | capability matrix validator row; validator protocols; validator-governance lib |
| Mechanical governance bundles | checks are strong and often correctly strict | truth authority drift; closeout convergence failure; token-burn amplification | swarm work still needs deterministic bounds, but the current bundle becomes expensive when upstream truth is fragmented | keep deterministic checks, but bundle them around fewer canonical truth surfaces | `Tier 2` | phase-check bundle; closeout-repair; lessons learned |
| Audit and evidence upkeep | dossiers, fail capture, and memory create rich evidence, but upkeep is high-friction | audit and artifact upkeep burden; operator skill dependence | swarm work increases evidence volume; if evidence upkeep remains manual-heavy, the control plane becomes self-defeating | keep rich evidence, but make live views projection-first and mechanically fed | `Tier 2` | distillation dossier; DCC dossier; workflow-dossier sync; repomem injection |
| Token and time discipline | the kernel knows cost matters, but ordinary repair still burns large budgets | token-burn amplification; session-control residue; closeout convergence failure | a swarm harness that is economically unstable is not viable even if semantically correct | make retry budgets, escalation, and degraded-mode fallbacks runtime-native | `Tier 1` | operator report; lessons learned; closeout-repair rationale |
| Parallel coordination semantics | some parallel role coordination exists today | truth authority drift; session-control residue; lane identity ambiguity | current coordination is enough for narrow governed parallelism, not enough for many simultaneous agents | add explicit claim/ownership/transfer primitives that survive partial failure | `Tier 1` | capability matrix parallel row; session docs; validator layering; role boundary law |
| Manual relay and degraded mode | manual relay already exists and is faster in some failure conditions | operator skill dependence | this is not a blocker; it is an asset that should survive into the next harness | preserve one state model across autonomous and manual relay modes | `Tier 2` | lane-boundary clauses; operator report; capability matrix fallback row |

## Priority Stack

The current priority order is:

1. workflow truth authority
2. session lifecycle convergence
3. closeout convergence
4. command-surface stability
5. token and time discipline
6. parallel coordination semantics

These are the gaps most likely to make external harness comparisons meaningful.
If an external project solves one of these well, it is immediately relevant.
If it only improves a lower-tier concern, it is interesting but not decisive.

## What Should Be Preserved

The gap map is not a teardown list.
Several current-kernel choices look worth preserving:

- explicit authority and lane boundaries
- layered validator split
- deterministic governance checks
- manual relay as a real degraded mode
- strong emphasis on durable evidence instead of transcript trust

The redesign pressure is mostly around:

- how many truth surfaces participate
- how session state converges
- how closeout publishes authoritative completion
- how much operator craft is required for routine recovery

## What External Harness Research Should Answer Next

When comparing other harnesses, use these questions:

- How do they keep one authoritative workflow state while still offering readable mirrors and dashboards?
- How do they converge session lifecycle after partial failure or timeout?
- How do they publish authoritative completion or handoff without making closeout repair-heavy?
- How do they keep command surfaces small and drift-resistant?
- How do they represent claim, ownership, handoff, and takeover for many concurrent actors?
- How do they preserve degraded manual fallback without forking the state model?

## Immediate Follow-On Use

This gap map should directly feed:

- `External_Harness_Comparison_Matrix.md`
- `Keep_Adopt_Drop_Decisions.md`

Those documents should justify comparisons and later decisions against the blocker tiers here instead of relying on intuition.
