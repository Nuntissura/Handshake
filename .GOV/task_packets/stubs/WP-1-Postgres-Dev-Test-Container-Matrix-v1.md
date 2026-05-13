# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Postgres-Dev-Test-Container-Matrix-v1

## STUB_METADATA
- WP_ID: WP-1-Postgres-Dev-Test-Container-Matrix-v1
- BASE_WP_ID: WP-1-Postgres-Dev-Test-Container-Matrix
- CREATED_AT: 2026-05-05T17:55:00Z
- STUB_STATUS: SUPERSEDED (FOLDED INTO WP-1-Postgres-Control-Plane-Shift-Bundle-v1)
- FOLDED_INTO: WP-1-Postgres-Control-Plane-Shift-Bundle-v1
- KERNEL_RESET_TRANSFERRED_TO: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- KERNEL_RESET_TRANSFER_SCOPE: Postgres dev/test profile and migration proof requirements for Kernel V1 EventLedger/SessionBroker; use Kernel001 packet/refinement/MTs for activation.
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-Migration-Framework, WP-1-Dual-Backend-Tests
- BUILD_ORDER_BLOCKS: WP-1-Postgres-Control-Plane-Leases-Backpressure, WP-1-ModelSession-Postgres-Queue-Workers, WP-1-FEMS-Postgres-Memory-Store, WP-1-Workflow-Engine-Postgres-Durable-Execution, WP-1-DCC-Postgres-Control-Plane-Projections
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ROADMAP_POINTER: SPEC_CURRENT storage portability, dual-backend test, and PostgreSQL readiness anchors
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Storage portability and dual-backend testing anchors around Master Spec lines 3248-3520.
  - Runtime dependency anchors around Master Spec lines 3737-3738.

## INTENT (DRAFT)
- What: Establish a reproducible PostgreSQL developer/test container matrix for the PostgreSQL-primary pivot, including local startup, migration reset, seeded fixtures, and CI-ready smoke profiles.
- Why: Follow-on PostgreSQL control-plane packets need a reliable database target under high script load; otherwise each WP re-discovers container setup, connection strings, migration order, and backend parity failure modes.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Docker or equivalent local PostgreSQL test service with stable environment variables and reset helpers.
  - Test profiles for migration-only, storage-conformance, ModelSession, FEMS, workflow, and DCC projection slices.
  - Documentation for operator/developer startup that keeps generated data outside the repo artifact roots.
  - Clear skip semantics when PostgreSQL is unavailable, with CI treating PostgreSQL-required lanes as required.
- OUT_OF_SCOPE:
  - Product schema design beyond test harness tables and fixtures.
  - Replacing SQLite cache/index tests.
  - Production deployment hardening.

## ACCEPTANCE_CRITERIA (DRAFT)
- A clean checkout can start the PostgreSQL test service, run migrations, seed fixtures, run targeted storage tests, and reset state mechanically.
- Test commands clearly separate required PostgreSQL-primary checks from optional SQLite cache/offline checks.
- Runtime artifact output stays under the external `../Handshake_Artifacts/` roots.
- Failure messages identify service-down, migration-failed, schema-drift, and fixture-drift states separately.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: foundation PostgreSQL-primary posture, existing migration framework, and dual-backend test precedent.
- Blocks: PostgreSQL lease/backpressure, ModelSession queue, FEMS store, workflow durable execution, and DCC projection follow-ups.

## RISKS / UNKNOWNs (DRAFT)
- Risk: container startup under heavy host load may be slow; activation should define generous but bounded timeouts.
- Unknown: whether current CI should run PostgreSQL tests always or as a gated required job until host capacity is stable.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body or create/approve spec enrichment first.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create the signed refinement.
- [ ] Create the official Work Packet via `just create-task-packet WP-1-Postgres-Dev-Test-Container-Matrix-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
