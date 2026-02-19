# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Loom-MVP-v1

## STUB_METADATA
- WP_ID: WP-1-Loom-MVP-v1
- BASE_WP_ID: WP-1-Loom-MVP
- CREATED_AT: 2026-02-19T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.131.md 7.6.3 (Phase 1) -> Loom MVP (Heaper-style library surface) [ADD v02.130]
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.131.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]
  - Handshake_Master_Spec_v02.131.md 2.2.1.14 LoomBlock Entity (Heaper-style Unit of Meaning) [ADD v02.130]
  - Handshake_Master_Spec_v02.131.md 2.3.7.1 Loom Relational Edges (Mentions, Tags, Backlinks) [ADD v02.130]
  - Handshake_Master_Spec_v02.131.md 11.5.12 FR-EVT-LOOM-* (Flight Recorder)

## INTENT (DRAFT)
- What: Deliver the Phase 1 Loom MVP: a local-first library surface powered by LoomBlocks (note/file/context) and LoomEdges (mentions/tags/backlinks), with fast browsing views and evidence-grade provenance via Flight Recorder.
- Why: Provide a unified "block-as-unit-of-meaning" surface that ties together notes and files, unlocking link/tag organization, backlinks, and quick recall without importing external app constraints.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - LoomBlock + LoomEdge foundations:
    - Storage model + API plumbing for LoomBlocks and LoomEdges (UUID-stable; rename-stable identity independent of filenames).
    - Inline token semantics for @mentions + #tags that create LoomEdges with stable UUID references (no text-based links).
  - Loom views (Phase 1 subset):
    - All / Unlinked / Sorted / Pins browsing projections over LoomBlocks.
    - LoomBlock detail view with backlinks panel (with context snippets).
  - Import + dedup:
    - File import (folder drag/drop + file picker).
    - SHA-256-based dedup: importing the same file twice must route to the existing LoomBlock (no duplicates).
  - Media previews:
    - Tier-1 preview generation (thumbnails, lightweight proxies where needed) implemented as governed Mechanical jobs.
    - Attach thumbnail/proxy asset refs to LoomBlocks; ensure outputs survive restart.
  - Search (Tier-1):
    - Basic Loom search with facets sufficient to support the MVP browse loops.
  - Observability:
    - Emit FR-EVT-LOOM-* events and surface them in Operator Consoles / Job History.
- OUT_OF_SCOPE:
  - AI auto-tagging / auto-caption and semantic/hybrid retrieval (Phase 2+ / Phase 4 per Master Spec).
  - Multi-user Loom collaboration/sync and Postgres-backed Loom query engines (Phase 3+ / Phase 4).

## ACCEPTANCE_CRITERIA (DRAFT)
- Importing files creates LoomBlocks; dedup prevents duplicates on re-import.
- All/Unlinked/Sorted/Pins views operate correctly on the same LoomBlock dataset.
- @mentions and #tags create LoomEdges with UUID-stable references; tag targets are TAG_HUB LoomBlocks.
- Backlinks panel updates correctly (with snippets) based on LoomEdges.
- Tier-1 search works with basic facets for LoomBlocks.
- FR-EVT-LOOM-* events are emitted and visible in Operator Consoles / Job History for the key Loom flows.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on Asset ingest + hashing + artifact/proxy generation primitives (Photo/Asset plumbing).
- Depends on Knowledge Graph / LoomEdge persistence primitives and Flight Recorder event surfacing.
- Depends on Mechanical Extension v1.2 gates for preview generation jobs (no bypass).

## RISKS / UNKNOWNs (DRAFT)
- Risk: anchor drift and inline token robustness during edits (must remain UUID-stable and not text-based).
- Risk: dedup false positives/negatives and identity coupling to filenames.
- Risk: background preview generation throughput degrading core UI responsiveness.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Loom-MVP-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Loom-MVP-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

