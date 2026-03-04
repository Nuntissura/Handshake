# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Storage-No-Runtime-DDL-v1

## STUB_METADATA
- WP_ID: WP-1-Storage-No-Runtime-DDL-v1
- BASE_WP_ID: WP-1-Storage-No-Runtime-DDL
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: MEDIUM
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Migration-Framework, WP-1-Storage-Abstraction-Layer
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (portable migrations posture)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Portable migrations posture (no hidden runtime schema mutation)
  - Handshake_Master_Spec_v02.139.md Dual-backend correctness (SQLite + Postgres parity)

## INTENT (DRAFT)
- What: Remove runtime schema mutation (CREATE/ALTER at runtime) by moving all schema evolution into numbered migrations for both backends.
- Why: Runtime DDL creates schema drift, breaks portability auditing, and can hide Postgres/SQLite parity bugs.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Identify all runtime DDL sites (ensure_* schema functions, runtime ALTER TABLE) and migrate them into numbered migrations.
  - Eliminate (or temporarily gate) runtime schema mutation code paths once migrations cover the schema.
  - Add a portability test that asserts no runtime DDL is executed (at least for core tables).
- OUT_OF_SCOPE:
  - Rewriting the migration framework itself (use existing system).

## ACCEPTANCE_CRITERIA (DRAFT)
- run_migrations() does not execute ad-hoc CREATE/ALTER for core tables outside the migration framework.
- Both backends start cleanly from empty DB using only migrations.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Requires careful sequencing with any existing ensure_* compatibility shims.

## RISKS / UNKNOWNs (DRAFT)
- A hard cut-over might break existing deployments; may need staged migrations and a compatibility window.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Storage-No-Runtime-DDL-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Storage-No-Runtime-DDL-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
