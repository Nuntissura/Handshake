# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1

## STUB_METADATA
- WP_ID: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1
- BASE_WP_ID: WP-1-Workflow-Engine-Postgres-Durable-Execution
- CREATED_AT: 2026-05-05T17:55:00Z
- STUB_STATUS: SUPERSEDED (FOLDED INTO WP-1-Postgres-Control-Plane-Shift-Bundle-v1)
- FOLDED_INTO: WP-1-Postgres-Control-Plane-Shift-Bundle-v1
- KERNEL002_TRANSITIVE_FOLDED_INTO: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- KERNEL002_FOLD_STATUS: FULL_STUB_FOLDED_TRANSITIVE
- KERNEL_RESET_TRANSFERRED_TO: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- KERNEL_RESET_TRANSFER_SCOPE: Kernel run-state, replay, restart, validation, and promotion-gate semantics moved into Kernel001; full generic workflow durable execution remains downstream residual scope.
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-Postgres-Dev-Test-Container-Matrix, WP-1-Postgres-Control-Plane-Leases-Backpressure, WP-1-Workflow-Engine, WP-1-Migration-Framework
- BUILD_ORDER_BLOCKS: WP-1-DCC-Postgres-Control-Plane-Projections, WP-1-Workflow-Transition-Automation-Registry, WP-1-Workflow-Projection-Correlation
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ROADMAP_POINTER: SPEC_CURRENT workflow durable execution and storage portability anchors
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Workflow durable execution anchors around Master Spec lines 9191-9340.
  - Storage portability anchors around Master Spec lines 3248-3520.

## INTENT (DRAFT)
- What: Move workflow-engine durable execution state, checkpoints, node outcomes, retries, and resumability metadata onto PostgreSQL-primary storage.
- Why: The self-hosting control plane needs workflows and AI jobs to survive process restarts and coordinate with ModelSession queues through the same authoritative database.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - PostgreSQL persistence for workflow instance state, node execution state, checkpoint payloads, retry counters, and terminal outcomes.
  - Integration with shared lease/backpressure primitives for runnable workflow work.
  - Migration-backed schema and tests for crash recovery, resume idempotence, retry bounds, and stale-lease recovery.
  - Explicit retention and artifact-link fields for Flight Recorder and DCC projections.
- OUT_OF_SCOPE:
  - New workflow node types.
  - Operator UI redesign.
  - Replacing structured collaboration artifact mirrors.

## ACCEPTANCE_CRITERIA (DRAFT)
- Workflow execution can stop and resume from PostgreSQL state without replaying completed nodes incorrectly.
- Parallel workflow workers cannot execute the same node claim concurrently.
- Retry and terminal failure states are queryable for DCC/runtime truth projection.
- SQLite use, if any, is cache/offline/index scoped and not the workflow runtime authority.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the foundation WP, test matrix, lease/backpressure primitives, validated workflow engine, and migration framework.
- Blocks workflow transition automation, workflow projection correlation, and DCC runtime truth work.

## RISKS / UNKNOWNs (DRAFT)
- Risk: existing workflow contracts may still say SQLite durable execution; activation should include spec enrichment before code work if the Master Spec is not yet updated.
- Unknown: checkpoint payload size and retention policy may require artifact indirection rather than storing all payload bytes in PostgreSQL.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body or create/approve spec enrichment first.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create the signed refinement.
- [ ] Create the official Work Packet via `just create-task-packet WP-1-Workflow-Engine-Postgres-Durable-Execution-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
