# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Locus-Phase1-QueryContract-Autosync-v1

## STUB_METADATA
- WP_ID: WP-1-Locus-Phase1-QueryContract-Autosync-v1
- BASE_WP_ID: WP-1-Locus-Phase1-QueryContract-Autosync
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (Locus query interface + autosync)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md 2.3.15.7 Query Interface (examples, filters, output contract)
  - Handshake_Master_Spec_v02.139.md Locus task-board sync requirements (auto-sync on state changes)

## INTENT (DRAFT)
- What: Align Locus query endpoints with the current spec contract and implement deterministic task-board autosync on relevant state changes.
- Why: Query shape drift breaks downstream tools and UIs; autosync prevents governance drift and manual "sync" toil.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Expand query filters (status, dependencies, facets) per spec examples.
  - Align output shape (full WP objects vs ids-only) as required by spec.
  - Auto-trigger locus_sync_task_board_v1 (or equivalent) on WP/MT state transitions.
  - Ensure Flight Recorder emits leak-safe events for query + sync paths.
- OUT_OF_SCOPE:
  - Realtime collaborative views and CRDT state (later phases).

## ACCEPTANCE_CRITERIA (DRAFT)
- Spec example queries execute and return the expected contract.
- Task Board stays consistent without manual sync calls after state changes.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Locus core operations exist.
- Decide Task Board canonical runtime path (`.handshake/gov/TASK_BOARD.md`) vs repo board for status reporting.

## RISKS / UNKNOWNs (DRAFT)
- Autosync must be bounded to avoid event storms; needs batching/throttling semantics.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Locus-Phase1-QueryContract-Autosync-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Locus-Phase1-QueryContract-Autosync-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

