# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.
- For any stub covering current-spec Phase 1 roadmap additions (`[ADD v<current>]`), `ROADMAP_ADD_COVERAGE` MUST enumerate the exact spec line numbers so governance checks can verify no additions were missed.

---

# Work Packet Stub: WP-1-Loom-Storage-Portability-v1

## STUB_METADATA
- WP_ID: WP-1-Loom-Storage-Portability-v1
- BASE_WP_ID: WP-1-Loom-Storage-Portability
- CREATED_AT: 2026-03-09T05:28:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Loom-MVP, WP-1-Storage-Abstraction-Layer, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-Media-Downloader-Loom-Bridge, WP-1-Video-Archive-Loom-Integration
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.156.md 7.6.3 (Phase 1) -> [ADD v02.156] Knowledge/retrieval pillar backend sweep / Loom storage portability
- ROADMAP_ADD_COVERAGE: SPEC=v02.156; PHASE=7.6.3; LINES=46350,46828
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.156.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]
  - Handshake_Master_Spec_v02.156.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example)
  - Handshake_Master_Spec_v02.156.md 2.3.12 Storage Backend Portability Architecture (Normative)

## INTENT (DRAFT)
- What: Define the canonical portable backend contract for Loom block-edge records, search/view filters, source anchors, and derived asset bindings.
- Why: Loom already behaves like a backend library substrate, but its storage/export/replay semantics cannot stay implicit if later library, retrieval, and media bridges are expected to remain stable across backend changes.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Unify Loom block, edge, search, view, and source-anchor semantics under portable backend contracts.
  - Preserve stable meaning across SQLite-now / PostgreSQL-ready storage, export, replay, and bounded artifact transfer.
  - Clarify what downstream Loom-consuming packets may assume about portable block-edge/search/view/source-anchor behavior.
- OUT_OF_SCOPE:
  - Full Loom product expansion.
  - Media-to-Loom bridge feature work beyond the backend portability contract.

## ACCEPTANCE_CRITERIA (DRAFT)
- Loom storage/export/replay semantics are explicitly linked to Storage Portability in the Main Body and Appendix matrix.
- Loom block-edge/search/view/source-anchor contracts preserve stable meaning across backend swaps and evidence transfer.
- Downstream Loom bridge packets can reuse one canonical portability contract instead of redefining library/export semantics.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Loom MVP, Storage Abstraction Layer, and Artifact System foundations.
- Research seed: adapt typed ownership, graph-portability, and provenance/export patterns from systems such as Backstage, OpenLineage, and pgvector-backed artifact stores instead of inventing Loom-only portability rules.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Loom blocks, edges, search filters, and source anchors diverge into storage-specific behavior that breaks export or replay.
- Risk: downstream bridge packets assume stable library portability semantics that are not yet codified.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Loom-Storage-Portability-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Loom-Storage-Portability-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
