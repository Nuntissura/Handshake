# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Stage-Media-Artifact-Portability-v1

## STUB_METADATA
- WP_ID: WP-1-Stage-Media-Artifact-Portability-v1
- BASE_WP_ID: WP-1-Stage-Media-Artifact-Portability
- CREATED_AT: 2026-03-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Handshake-Stage-MVP, WP-1-Media-Downloader, WP-1-Artifact-System-Foundations, WP-1-Storage-Trait-Purity
- BUILD_ORDER_BLOCKS: WP-1-Media-Downloader-Loom-Bridge, WP-1-Video-Archive-Loom-Integration
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.150.md 7.6.3 (Phase 1) -> backend combo expansion: stage/media artifact portability
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.150.md 10.13 Handshake Stage (Normative)
  - Handshake_Master_Spec_v02.150.md 10.14 Media Downloader (Normative)
  - Handshake_Master_Spec_v02.150.md 2.3.10 Debug Bundle export (Normative)
  - Handshake_Master_Spec_v02.150.md 2.3.12 Storage Backend Portability Architecture (Normative)

## INTENT (DRAFT)
- What: Unify Stage capture/import sessions and Media Downloader session/auth/materialization outputs under portable artifact manifest, bundle-index, and retention semantics.
- Why: Backend evidence from Stage and Media Downloader should survive export, replay, and storage/backend changes without semantic drift.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define the shared artifact portability contract across Stage session records, media capture sessions, debug bundle export, and storage portability rules.
  - Make session/auth/materialization outputs valid bounded export anchors with portable manifests and retention evidence.
  - Clarify what later Loom/archive integrations may assume about Stage/media artifacts.
- OUT_OF_SCOPE:
  - Full Stage product UX.
  - Media-to-Loom feature work beyond the backend portability contract.

## ACCEPTANCE_CRITERIA (DRAFT)
- Stage and Media Downloader are explicitly linked through portable artifact and retention contracts in the Main Body and Appendix matrix.
- Later implementation packets can reuse one bounded export/manifest contract across Stage and media capture flows.
- Storage backend swaps do not require redefining Stage/media evidence semantics.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Handshake Stage MVP, Media Downloader, Artifact System foundations, and Storage Trait Purity.
- Research seed: adapt asset-lineage and portability patterns from systems such as Dagster and evidence-manifest approaches rather than inventing Stage/media-specific export dialects.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Stage and media artifacts fork into incompatible manifest/index semantics.
- Risk: portability rules cover storage backends generally but omit capture-session provenance needed for replay and audit.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Stage-Media-Artifact-Portability-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Stage-Media-Artifact-Portability-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
