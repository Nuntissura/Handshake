# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1

## STUB_METADATA
- WP_ID: WP-1-DCC-Postgres-Control-Plane-Projections-v1
- BASE_WP_ID: WP-1-DCC-Postgres-Control-Plane-Projections
- CREATED_AT: 2026-05-05T17:55:00Z
- STUB_STATUS: SUPERSEDED (FOLDED INTO WP-1-Postgres-Control-Plane-Shift-Bundle-v1)
- FOLDED_INTO: WP-1-Postgres-Control-Plane-Shift-Bundle-v1
- KERNEL_RESET_TRANSFERRED_TO: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- KERNEL_RESET_TRANSFER_SCOPE: Minimal TraceProjection/DCC-inspector proof moved into Kernel001; full DCC PostgreSQL projection UI/backend scope remains downstream residual scope.
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-Postgres-Control-Plane-Leases-Backpressure, WP-1-ModelSession-Postgres-Queue-Workers, WP-1-Workflow-Engine-Postgres-Durable-Execution, WP-1-Dev-Command-Center-Control-Plane-Backend
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-MVP, WP-1-Session-Spawn-Tree-DCC-Visualization, WP-1-Dev-Command-Center-Layout-Projection-Registry
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ROADMAP_POINTER: SPEC_CURRENT DCC, work-tracking, and storage portability anchors
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Locus/work-tracking PostgreSQL and multi-user anchors around Master Spec lines 6742-7443.
  - Storage portability anchors around Master Spec lines 3248-3520.

## INTENT (DRAFT)
- What: Project PostgreSQL-primary runtime truth into Dev Command Center views for sessions, queues, leases, workflows, memory jobs, and blocked/stalled control-plane work.
- Why: Self-hosting is not operational if the Operator cannot see the actual database-backed runtime state and intervene without reading logs or role terminals.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Backend projection endpoints or query services that read authoritative PostgreSQL runtime/control-plane state.
  - DCC-ready state models for queue depth, active leases, stalled work, model sessions, workflow instances, memory jobs, and dead-letter items.
  - Action affordance contracts for retry, cancel, release lease, requeue, and inspect evidence, even if some actions remain disabled.
  - Tests that verify UI projections do not infer truth from stale mirrors or local process state.
- OUT_OF_SCOPE:
  - Full DCC visual redesign.
  - Implementing every operator action mutation.
  - Replacing Flight Recorder evidence storage.

## ACCEPTANCE_CRITERIA (DRAFT)
- DCC projection data comes from PostgreSQL authority or a declared derived projection with source/version metadata.
- Stale, missing, and conflicting runtime states are represented explicitly.
- Projection tests cover at least empty, healthy, saturated/backpressured, stalled, and dead-letter states.
- The UI contract distinguishes read-only evidence from governed mutation actions.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on foundation, leases/backpressure, ModelSession queue workers, workflow durable execution, and existing DCC backend.
- Blocks DCC MVP and session visualization packets that need live control-plane truth.

## RISKS / UNKNOWNs (DRAFT)
- Risk: mixing mirrors with canonical PostgreSQL state can create false operator confidence; activation should require source labels and projection freshness.
- Unknown: whether DCC should poll, subscribe, or consume NOTIFY-backed projection updates in Phase 1.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body or create/approve spec enrichment first.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create the signed refinement.
- [ ] Create the official Work Packet via `just create-task-packet WP-1-DCC-Postgres-Control-Plane-Projections-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
