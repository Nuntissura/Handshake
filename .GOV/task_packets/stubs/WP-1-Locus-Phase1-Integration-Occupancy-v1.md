# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Locus-Phase1-Integration-Occupancy-v1

## STUB_METADATA
- WP_ID: WP-1-Locus-Phase1-Integration-Occupancy-v1
- BASE_WP_ID: WP-1-Locus-Phase1-Integration-Occupancy
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (Locus integration points + MT occupancy)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md 2.3.15.4 Integration Points (Spec Router, MT Executor, Task Board)
  - Handshake_Master_Spec_v02.139.md MT occupancy: TrackedMicroTask.active_session_ids + bind/unbind ops

## INTENT (DRAFT)
- What: Wire Locus into Spec Router and Micro-Task Executor and implement MT occupancy tracking for parallel sessions.
- Why: Without integration, Locus is an unused side subsystem; occupancy is required to make parallel sessions safe and observable.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Add `active_session_ids: string[]` to tracked micro-task state.
  - Add mechanical ops: locus_bind_session_v1 / locus_unbind_session_v1.
  - Spec Router integration: auto locus_create_wp (and updates) for created/updated WPs.
  - Micro-Task Executor integration: locus_start_mt / locus_record_iteration / locus_complete_mt calls in the loop.
- OUT_OF_SCOPE:
  - Postgres Locus parity (separate decision/strategy stub).
  - Medallion and hybrid search (separate stub).

## ACCEPTANCE_CRITERIA (DRAFT)
- Running a WP through Spec Router + MT Executor produces Locus storage rows for WP/MT/iterations.
- active_session_ids reflects reality for parallel sessions (bind/unbind on session start/end).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Locus SQLite store exists.
- Session IDs and MT execution lifecycle hooks are stable.

## RISKS / UNKNOWNs (DRAFT)
- Event ordering and idempotency across crashes; must be designed as replay-safe operations.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Locus-Phase1-Integration-Occupancy-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Locus-Phase1-Integration-Occupancy-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

