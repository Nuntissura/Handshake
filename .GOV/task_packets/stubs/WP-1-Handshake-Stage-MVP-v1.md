# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Handshake-Stage-MVP-v1

## STUB_METADATA
- WP_ID: WP-1-Handshake-Stage-MVP-v1
- BASE_WP_ID: WP-1-Handshake-Stage-MVP
- CREATED_AT: 2026-02-19T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.131.md 7.6.3 (Phase 1) -> Handshake Stage MVP (governed browser + Stage Apps) [ADD v02.131]
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.131.md 10.13 Handshake Stage (Built-in Browser + Stage Apps) [ADD v02.131]
  - Handshake_Master_Spec_v02.131.md 2.6.6 (AI Job Model job profiles: Stage family)
  - Handshake_Master_Spec_v02.131.md 11.8 (Mechanical Extension v1.2: governed engines; no bypass)

## INTENT (DRAFT)
- What: Deliver the Phase 1 Handshake Stage MVP: a governed in-app browser surface with isolated sessions/tabs, Stage Apps, and a Stage Bridge API that can request privileged actions only via Workflow Engine + gates + Flight Recorder (no bypass).
- Why: Enable evidence-grade capture/import of external web/PDF/3D inputs into workspace artifacts while preventing session bleed, origin confusion, and unlogged side effects.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Stage surface (MVP):
    - Embedded browser UI for External Web navigation.
    - Stage Apps rendered in WebViews under a dedicated scheme (handshake-stage://), with bridge enabled only for Stage Apps.
  - Isolation + security model:
    - Multiple Stage sessions supported; no cookie/storage/cache bleed across sessions/tabs.
    - Stage Bridge injection only on handshake-stage://; External Web must never receive the bridge.
    - Every bridge call is capability-gated and emits allow/deny events into Flight Recorder / Operator Consoles.
  - Capture/import workflows (Phase 1 slice):
    - stage.capture_webpage.v1: evidence-grade web capture (Archivist) producing artifact.snapshot bundles (HTML + assets + screenshots) with manifests + SHA-256.
    - stage.clip_selection.v1: clip selected DOM ranges into artifact.clip (markdown + selectors), linked to the originating snapshot.
    - stage.import_pdf.v1: attach/import PDF bytes as artifacts and produce a document stub; structured Docling conversion remains Phase 2.
    - 3D Mechanical Assist Pack Phase 1 slice: stage.3d.import_gltf.v1 + stage.3d.validate_gltf.v1 producing artifact.scene_ir + validator/physics reports.
    - Minimal read-only 3D viewport/inspector to view 3D assets and reports (Stage Studio deferred).
  - Observability:
    - All Stage privileged actions are Jobs/Workflows with auditable artifacts and Flight Recorder trails; outputs survive restart and are discoverable.
- OUT_OF_SCOPE:
  - Docling-backed structured PDF conversion (Phase 2).
  - Browser-extension ecosystem, third-party Stage App marketplace/plugins, bulk crawling/mirroring.
  - Stage Studio authoring/editor UX and advanced 3D editing/collaboration (Phase 3+ / Phase 4).

## ACCEPTANCE_CRITERIA (DRAFT)
- External Web and Stage Apps run in isolated sessions/tabs with no cookie/storage bleed.
- Stage Bridge is injected only on handshake-stage:// and denies calls from External Web; every allow/deny is visible in Flight Recorder/Operator Consoles.
- stage.capture_webpage.v1 produces artifact.snapshot bundles with manifests + SHA-256 + provenance.
- stage.clip_selection.v1 produces artifact.clip linked to snapshot selectors.
- stage.import_pdf.v1 preserves original PDF bytes as artifacts and produces a document stub linked to the bytes.
- stage.3d.import_gltf.v1 + stage.3d.validate_gltf.v1 produce artifact.scene_ir + validator reports; Stage viewport refuses to render without a passing validation report.
- All outputs survive restart and are discoverable through Job History / artifact browsing surfaces.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on Mechanical Extension v1.2 engines/gates (no bypass) and artifact-first IO constraints.
- Depends on capability registry + approval UX for any network/capture operations.
- Depends on Flight Recorder + Operator Consoles surfacing for allow/deny and capture/import job traces.

## RISKS / UNKNOWNs (DRAFT)
- Risk: origin isolation bugs (External Web vs Stage Apps) and session bleed across profiles.
- Risk: private-network access and SSRF-style exposure if network policy is not enforced by capabilities/gates.
- Risk: evidence integrity drift (missing hashes/manifests/provenance) for capture outputs.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Handshake-Stage-MVP-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Handshake-Stage-MVP-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

