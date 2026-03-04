# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Locus-Phase1-Medallion-Search-v1

## STUB_METADATA
- WP_ID: WP-1-Locus-Phase1-Medallion-Search-v1
- BASE_WP_ID: WP-1-Locus-Phase1-Medallion-Search
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (Locus medallion objects + search)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Locus WPBronze/WPSilver requirements (embeddings + keyword search)
  - Handshake_Master_Spec_v02.139.md Hybrid retrieval integration posture

## INTENT (DRAFT)
- What: Implement medallion outputs for Locus Work Packets (WPBronze/WPSilver) and wire them into search/indexing.
- Why: Without medallion objects, Locus is not LLM-friendly and cannot support retrieval-backed orchestration or governance queries at scale.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define WPBronze (immutable ingest snapshot) and WPSilver (normalized/validated) schemas.
  - Generate embeddings and keyword index entries for Work Packets and microtasks.
  - Provide query surface for "search work packets" by text + facet filters.
- OUT_OF_SCOPE:
  - Gold-level optimization/index-only store (can follow once Bronze/Silver is stable).

## ACCEPTANCE_CRITERIA (DRAFT)
- Creating/updating a Work Packet results in Bronze/Silver records being materialized.
- Search finds WPs by title/body and can filter by status/labels.

## DEPENDENCIES / BLOCKERS (DRAFT)
- AI-ready data indexing pipeline primitives exist.
- Decide how Locus medallion integrates with existing Bronze/Silver stores (unify vs parallel).

## RISKS / UNKNOWNs (DRAFT)
- Data duplication risk if Locus and AI-ready data pipelines diverge; must unify identity semantics early.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Locus-Phase1-Medallion-Search-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Locus-Phase1-Medallion-Search-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

