# REPO_GOVERNANCE_REFACTOR_ROADMAP

**Status:** Draft  
**Scope:** Governance-only refactor in `/.GOV/`  
**Explicit non-goals:** no product-code changes, no Master Spec edits, no Work Packets, no signature flow

## Purpose

This roadmap documents the governance refactor before implementation starts.

The immediate problem is not that the repo lacks governance artifacts. The repo already has packet structure, split validator verdict fields, communication helpers, session ledgers, and multiple gate checks. The problem is that those surfaces can still be satisfied in a way that looks complete while leaving proof, authority, and workflow truth too weak.

This roadmap therefore focuses on closing the specific escape path that was just observed:

- visible completion can still outrun real proof
- validator PASS language can still overstate what was actually defended
- workflow truth can still fragment across packet, runtime, task board, session, and worktree state
- the Orchestrator still absorbs repair and routing work that should be enforced by workflow law

## Source Inputs

- Research basis: `.GOV/reference/research_and_papers/repo-governance-combined-version.md`
- Triggering audit: `.GOV/Audits/audits/AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`
- Current orchestration law: `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- Current validator law: `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- Current packet contract: `.GOV/templates/TASK_PACKET_TEMPLATE.md`

## Placement Rules

This refactor must stay inside the existing governance kernel structure. Do not introduce a new top-level `governance/` tree.

- Shared docs go in `.GOV/roles_shared/docs/`
- Shared records and ledgers go in `.GOV/roles_shared/records/`
- Shared schemas go in `.GOV/roles_shared/schemas/`
- Shared scripts and libraries go in `.GOV/roles_shared/scripts/`
- Shared checks go in `.GOV/roles_shared/checks/`
- Role-specific wrappers stay under the owning role directory only when the logic is genuinely role-local

## Design Rules

1. Build on the current split-verdict and `NOT_PROVEN` machinery instead of replacing it with a second parallel system.
2. Prefer computed closure over narrated closure.
3. Prefer one authoritative workflow truth over mirrored interpretation.
4. Prefer machine-checked direct role exchange over Orchestrator relay.
5. Treat repeated audit escape shapes as prevention-ladder inputs, not one-off commentary.
6. Keep the first rollout governance-only; do not mix this refactor with product remediation.

## Board Model

The matching governance-only task board for this roadmap lives at:

- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- Implementation closeout lives at `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_CLOSEOUT.md`

Status tokens used there:

- `PLANNED` = defined but not implementation-ready
- `READY` = scoped and sequenced; safe to implement
- `IN_PROGRESS` = actively being implemented
- `BLOCKED` = cannot advance until an upstream roadmap item lands
- `DONE` = implemented and verified on the governance surface
- `HOLD` = intentionally deferred

This is not a Work Packet board and does not replace `.GOV/roles_shared/records/TASK_BOARD.md`.

## Refactor Sequence

| ID | Workstream | Why First | Primary Surfaces |
|---|---|---|---|
| RGR-01 | Workflow Truth and Startup Gate | Prevent false-ready state before any more governance or product execution | `roles_shared/checks`, `roles/orchestrator/scripts`, runtime ledgers |
| RGR-02 | Transactional Prepare and Status Sync | Stop partial packet/runtime/task-board writes | `roles/orchestrator/scripts`, `roles_shared/scripts/lib`, records/runtime |
| RGR-03 | Direct Review Boundary Enforcement | Remove manual routing and prove coder-validator exchange | WP communication helpers, notification routing, communication checks |
| RGR-04 | Scope and Tool Spill Enforcement | Block vibecoding-style broad edits and formatter spill | coder/validator checks, packet helpers, scope ledgers |
| RGR-05 | Computed Policy and Evidence Gate | Make final closure computed rather than narrated | shared schemas, records, policy checks, validator structure checks |
| RGR-06 | Prevention Ladder, Legacy Cleanup, and Audit Capture | Convert escapes into prevention and reduce future governance drift | mistake/anti-pattern ledgers, deprecation surfaces, audit generators |

## Workstreams

## RGR-01 - Workflow Truth and Startup Gate

**Goal:** make false-ready and split-truth states fail before execution starts.

**Why this exists:** startup currently verifies a lot, but the audit showed that a run can still look governed while packet truth, proof truth, and closure truth are softer than they appear.

**Feature scope**

- Add one explicit startup hard gate for governed execution readiness.
- Fail startup when packet truth, runtime truth, task-board truth, session truth, and worktree truth disagree in material ways.
- Fail startup when authoritative communication artifacts are missing, duplicated, or rooted outside the packet-declared authority path.
- Keep gate output machine-readable and operator-readable.

**Technical detail**

- Consolidate startup truth checks behind a shared readiness check instead of scattering partial truth decisions across multiple helpers.
- Require these predicates at minimum:
  - `just gov-check` clean
  - `just orchestrator-startup` clean
  - packet/refinement/spec pointer hashes current where applicable
  - PREPARE target exists on disk where applicable
  - packet-declared worktree/runtime paths exist and match live topology
  - packet/task-board/traceability/runtime/session/worktree state agree on the active status
  - communication authority exists only at the packet-declared root
- Emit one settled gate result rather than a mix of optimistic informational output plus later correction.

**Primary target surfaces**

- `.GOV/roles_shared/checks/`
- `.GOV/roles/orchestrator/scripts/`
- `.GOV/roles_shared/runtime/`
- `.GOV/roles_shared/records/`

**Done when**

- governed startup cannot report a ready state if workflow truth is split
- the new hard gate becomes part of normal Orchestrator startup/resume flow
- the gate output is stable enough to be reused by later workflow-proof testing

## RGR-02 - Transactional Prepare and Status Sync

**Goal:** make governance state changes atomic enough that packet creation and activation cannot partially succeed.

**Why this exists:** partial updates create fake progress and make the Orchestrator clean up drift by hand.

**Feature scope**

- Make `orchestrator-prepare-and-packet` transactional in practice.
- Update packet state, micro-task state, task-board state, traceability state, runtime state, and declared communication state together or fail without partial truth drift.
- Record workflow-state change receipts when execution mode changes materially.

**Technical detail**

- Introduce a staged write plan in shared helpers before mutating multiple governance artifacts.
- Apply writes through temp-file or staged-buffer flow, then promote them as one settled state change.
- Reject partial success where some of these surfaces move and others do not:
  - packet `CURRENT_STATE`
  - packet closure monitors
  - task board projection
  - traceability registry projection
  - runtime status projection
  - communication folder bootstrap
- Add a deterministic receipt for mode changes such as selective integration vs direct branch closure.

**Primary target surfaces**

- `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
- `.GOV/roles/orchestrator/scripts/`
- `.GOV/roles_shared/scripts/lib/`
- `.GOV/roles_shared/records/`
- `.GOV/roles_shared/runtime/`

**Done when**

- activation either lands coherently across all required governance surfaces or does not land
- startup no longer needs to compensate routinely for packet-creation drift

## RGR-03 - Direct Review Boundary Enforcement

**Goal:** make direct coder-validator review a hard workflow boundary instead of a preference.

**Why this exists:** the audit and current doctrine both require direct review, but the workflow still leaves too much room for Orchestrator-centered steering and narrative relay.

**Feature scope**

- Make pre-edit, handoff, and verdict review boundaries machine-checked.
- Auto-route the next expected actor from receipts and notifications.
- Make missing direct review a real blocker at the boundary where it matters.

**Technical detail**

- Require structured direct-review receipts with stable `correlation_id` and `ack_for` chains for:
  - coder intent / pre-edit proof statement
  - validator checklist / tripwire receipt
  - coder handoff
  - validator review response
  - integration-validator review pair where the packet format requires it
- Tighten `wp-communication-health-check` so it verifies the required pairs, actor routing, and acknowledgment behavior at each stop point.
- Project `next_expected_actor`, `waiting_on`, and review wake state from governed receipts instead of manual narrative steering.
- Treat unacknowledged required review notifications as boundary health defects when they cross the configured threshold.

**Primary target surfaces**

- `.GOV/roles_shared/scripts/`
- `.GOV/roles_shared/checks/`
- `.GOV/roles_shared/tests/`
- `../gov_runtime/roles_shared/` runtime ledgers and communication projections

**Done when**

- handoff and verdict cannot clear if the required direct-review chain is missing
- the Orchestrator no longer needs to act as a message broker for ordinary coder-validator traffic

## RGR-04 - Scope and Tool Spill Enforcement

**Goal:** block broad, low-discipline edit patterns that create governance-approved technical debt.

**Why this exists:** one way models game the system is by satisfying the visible packet shape while using broad tools or broad edits that spill outside the intended proof surface.

**Feature scope**

- Treat touched-file budgets and in-scope paths as enforceable, not decorative.
- Add explicit tool allowlisting for broad-scope tools inside narrow-scope work.
- Promote formatter spill and out-of-scope file creation into hard correction events.

**Technical detail**

- Expand packet and checker support for explicit broad-tool allowlists such as:
  - formatter runs
  - code generation
  - repo-wide search/replace
  - migration rewrites
- Detect when final touched files exceed declared packet scope without a matching allowlist or waiver.
- Make broad spill a hard stop in governance checks, not a warning to be explained away later.
- Record the exact spill set and governing correction path in machine-readable evidence when it occurs.

**Primary target surfaces**

- `.GOV/roles/coder/checks/`
- `.GOV/roles/validator/checks/`
- `.GOV/templates/TASK_PACKET_TEMPLATE.md`
- `.GOV/roles_shared/checks/`
- `.GOV/roles_shared/schemas/`

**Done when**

- a narrow packet cannot quietly pass after broad write spill
- formatter or tool spill becomes visible, attributable, and blockable

## RGR-05 - Computed Policy and Evidence Gate

**Goal:** move final closure authority from narrative validator prose into a deterministic policy decision.

**Why this exists:** the repo already has split verdict fields, but the audit shows that narrative completion can still overstate actual proof depth.

**Feature scope**

- Add canonical shared governance artifacts for claims, witnesses, protected surfaces, and waivers.
- Compute final closure from those artifacts plus existing split verdicts.
- Keep `NOT_PROVEN` as a first-class proof state during migration instead of collapsing it back into PASS-oriented prose.

**Technical detail**

- Introduce or formalize shared records and schemas for:
  - requirement or constraint registry for diff-scoped closure
  - diff claims
  - witness matrix
  - protected-surface registry
  - waiver ledger
- Add a shared policy gate that computes closure classes from:
  - scope validity
  - proof completeness
  - integration readiness
  - domain-goal completion
  - protected-surface review state
  - waiver state
- Preserve current validator split fields where already present, but make them inputs to the computed gate rather than the final authority by themselves.
- Introduce computed outcomes that distinguish:
  - `PASS`
  - `FAIL`
  - `REVIEW_REQUIRED`
  - `WAIVED`
  - `BLOCKED`
- Seed the first prevention assets from the concrete audit escapes:
  - missing workflow contract field enforcement
  - shallow nested payload validation
  - non-typed timestamp validation

**Primary target surfaces**

- `.GOV/roles_shared/records/`
- `.GOV/roles_shared/schemas/`
- `.GOV/roles_shared/scripts/`
- `.GOV/roles_shared/checks/`
- `.GOV/roles/validator/checks/`
- `.GOV/roles/validator/docs/`

**Done when**

- validator prose alone cannot manufacture a clean PASS
- final closure has a deterministic computed basis
- the first audit-derived escape classes exist as tracked prevention assets

## RGR-06 - Prevention Ladder, Legacy Cleanup, and Audit Capture

**Goal:** reduce governance drift and make post-run scrutiny less dependent on reconstruction from memory.

**Why this exists:** governance is still carrying migration residue, legacy compatibility paths, and audit effort that is too manual.

**Feature scope**

- Create a governed prevention ladder and mistake or anti-pattern ledger for repo-governance escapes.
- Keep a compatibility-shim ledger with explicit sunset conditions.
- Auto-generate a stronger post-run audit skeleton from authoritative artifacts.

**Technical detail**

- Add a shared mistake or anti-pattern record for governance escapes and repeated workflow defects.
- Track each compatibility shim with:
  - why it exists
  - what supersedes it
  - deletion condition
  - sunset trigger
- Continue cleanup of legacy authority paths that can still poison live checks.
- Harden session-registry and runtime-ledger writes where concurrent rename/write behavior is still fragile.
- Expand the post-run audit generator so it pulls from:
  - packet state
  - runtime state
  - gate results
  - session ledgers
  - direct-review receipts
  - notification state

**Primary target surfaces**

- `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`
- `.GOV/roles_shared/records/`
- `.GOV/roles_shared/scripts/audit/`
- `.GOV/roles_shared/scripts/session/`
- `.GOV/roles_shared/checks/`

**Done when**

- repeated governance defects have a clear promotion path into prevention
- legacy compatibility surfaces are explicitly tracked instead of living as hidden residue
- a workflow-proof audit can be generated from authoritative artifacts with minimal manual reconstruction

## First Implementation Slice

The first implementation slice should stay narrow and land only the minimum control-plane hardening needed to stop known workflow misrepresentation.

Recommended first slice:

1. RGR-01 workflow truth and startup hard gate
2. RGR-02 transactional prepare and status sync
3. RGR-03 direct review boundary enforcement

Do not start RGR-04 through RGR-06 until the first three items are stable enough to support one honest workflow-proof run.

## Deferred Until After the First Stable Slice

- broad requirement-graph expansion beyond what the governance gate actually needs
- large canary-suite buildout
- product-specific workflow harness growth
- product remediation for the audited Schema Registry gaps
- any attempt to recast this roadmap as a Master Spec change

## Exit Condition for This Refactor

This roadmap is complete when the governance kernel can prove the following without leaning on narrative trust:

- startup blocks false-ready state
- packet activation is transactionally coherent
- direct coder-validator exchange is machine-checked at the real boundaries
- out-of-scope spill is visible and blockable
- final closure is computed from evidence and policy, not rounded up from prose
- repeated governance failures have a promotion path into prevention assets

At that point the repo can safely resume product-code remediation under a stronger governance baseline.
