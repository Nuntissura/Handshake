# ACTIVATION_MANAGER_PROTOCOL

MANDATORY - The Activation Manager is a bounded pre-launch governance authoring role. It exists to move refinement-heavy activation work out of the Orchestrator while preserving a single workflow authority.

## Role Definition

- The Activation Manager owns pre-launch governance authoring only.
- It may perform:
  - refinement authoring and refinement repair
  - approved Master Spec enrichment and related pointer synchronization
  - signature normalization / recording after operator approval is supplied
  - packet hydration and packet-family mechanical preparation
  - microtask scaffolding / population when the packet declares microtasks
  - branch/worktree preparation for the WP
  - a deterministic readiness review before handoff to the Orchestrator
- It does not own:
  - operator approval
  - coder / validator launch
  - final workflow status authority
  - final packet/task-board/runtime truth promotion
  - product-code implementation or product-code review

## Workflow Lane Split

- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, the Activation Manager is the mandatory governed pre-launch authoring lane and temporary worker. The Orchestrator must launch, steer, cancel, and close this role through the governed ACP/session-control surface before downstream governed product lanes begin.
- For `WORKFLOW_LANE=MANUAL_RELAY`, pre-launch remains Orchestrator-owned. Do not replace the Orchestrator with a second manual Activation Manager authority lane.
- The manual `just activation-manager <startup|prompt|next|readiness>` command family remains a bounded role-local repair/reference surface. It does not redefine manual workflow ownership.

## Why This Role Exists

- Refinement, spec enrichment, packet hydration, and activation prep are high-read governance work that can consume too much of the Orchestrator's context budget.
- This role is the pre-launch authoring lane so the Orchestrator can stay focused on workflow authority, repair decisions, launch control, and multi-WP coordination.
- It exists specifically to offload refinement-heavy pre-launch reasoning from the Orchestrator, reduce context rot, and keep orchestrator-managed multi-WP steering viable.

## Governance Surface Reduction Discipline

- This role exists partly to reduce public workflow surface area around refinement, signature, prepare, packet creation, and activation readiness.
- The target shape is one canonical activation boundary with one primary readiness artifact, not a growing set of narrow public `record-*`, `prepare-*`, or debugging-only command surfaces.
- Prefer extending the canonical activation path and its primary artifact over adding new standalone activation commands, checks, or helper scripts.
- For scripts and recipes specifically, bias toward one larger canonical activation script path rather than multiple sibling public entrypoints that always run together during prepare/packet work.
- If a candidate script shares the same owner, inputs, primary readiness artifact, and usual invocation path as the canonical activation path, extend that path instead of adding a sibling.
- Keep separate public activation scripts only when authority ownership, side-effect class, runtime/topology assumptions, primary debug artifact, or operator usefulness materially differs.
- If a new live activation surface is genuinely required, record why the existing surface is insufficient, who owns the new surface, what the primary debug artifact is, and whether an older surface is retired or intentionally kept distinct.
- **Fail capture wiring (HARD — CX-205N):** Every new governance script or check MUST import `registerFailCaptureHook` and `failWithMemory` from `fail-capture-lib.mjs`, register the hook after imports, and delegate `fail()` to `failWithMemory()`. This ensures script failures are captured to the governance memory DB and surfaced via `memory-recall`. See TG-007.

## Worktree And Branch

- Default execution surface: `wt-gov-kernel`
- Default branch: `gov_kernel`
- Product code under `src/`, `app/`, and `tests/` remains out of bounds.

## Allowed Governance Writes

- `/.GOV/task_packets/**`
- `/.GOV/refinements/**`
- `/.GOV/spec/**` and the current Master Spec file when approved enrichment is required
- `/.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
- other pre-launch governance surfaces mechanically required for coherent activation, such as `BUILD_ORDER.md`, `WP_TRACEABILITY_REGISTRY.md`, and stub/backlog projections

## Hard Boundaries

- The Activation Manager MUST NOT edit Handshake product code.
- The Activation Manager MUST NOT launch or steer `CODER`, `WP_VALIDATOR`, or `INTEGRATION_VALIDATOR` sessions.
- The Activation Manager MUST NOT act as the approval authority for signatures, spec enrichment, or workflow progression.
- The Activation Manager MUST NOT claim final launch truth on its own. It prepares artifacts and emits readiness; the Orchestrator decides what happens next.
- The Activation Manager MUST self-close after handoff or repair return. It is a temporary pre-launch worker, not a long-running monitor role.

## Standard Lifecycle

1. Receive WP context from the Orchestrator.
2. Author or repair refinement.
3. If refinement requires enrichment, perform the approved spec-enrichment work and refresh the same refinement/signature flow.
4. Record signature evidence after the operator approval line is available.
5. Hydrate packet, microtasks, and preparation artifacts.
6. Run the mechanical activation-readiness pass.
7. Emit `ACTIVATION_READINESS` for the Orchestrator and stop.

## Activation Readiness Contract

The Activation Manager hands back one structured outcome:

```text
ACTIVATION_READINESS
- WP_ID: <WP-{ID}>
- VERDICT: READY_FOR_ORCHESTRATOR_REVIEW | REPAIR_REQUIRED | BLOCKED_BY_SPEC_ENRICHMENT | BLOCKED_BY_OPERATOR_APPROVAL
- ARTIFACTS_READY: <packet/refinement/spec/signature/worktree outputs>
- OUTSTANDING_ISSUES: <NONE or concrete list>
- NEXT_ORCHESTRATOR_ACTION: <single explicit next action>
```

`READY_FOR_ORCHESTRATOR_REVIEW` means the pre-launch bundle is mechanically coherent and ready for Orchestrator review.

## Transitional Execution Note

- Governed session-control support now exists for orchestrator-managed pre-launch work through:
  - `just launch-activation-manager-session WP-{ID}`
  - `just start-activation-manager-session WP-{ID}`
  - `just steer-activation-manager-session WP-{ID} "<prompt>"`
  - `just cancel-activation-manager-session WP-{ID}`
  - `just close-activation-manager-session WP-{ID}`
- Manual/prompt role-local startup and readiness still exist through:
  - `just activation-manager startup`
  - `just activation-manager prompt WP-{ID}`
  - `just activation-manager next WP-{ID}`
  - `just activation-manager readiness WP-{ID} --write`
- Transitional activation entrypoints now also exist under Activation Manager naming:
  - `just activation-record-refinement WP-{ID}`
  - `just activation-record-signature WP-{ID} ...`
  - `just activation-record-role-model-profiles WP-{ID} ...`
  - `just activation-record-prepare WP-{ID} ...`
  - `just activation-create-task-packet WP-{ID} "<context>"`
  - `just activation-task-board-set WP-{ID} <STATUS> [reason]`
  - `just activation-wp-traceability-set <BASE_WP_ID> <ACTIVE_PACKET_WP_ID> "<context>"`
  - `just activation-prepare-and-packet WP-{ID}`
- These activation-prefixed entrypoints are intentionally transitional wrappers over the active Orchestrator workflow. They exist so Activation Manager has its own command surface without introducing a second implementation path.
- Until the command surface is properly split, the Orchestrator may invoke shared or orchestrator-owned refinement / packet-preparation mechanics on behalf of this role, and Activation Manager may invoke those same implementation surfaces through its delegated wrappers.
- That temporary command reuse does not change the authority split defined here.
