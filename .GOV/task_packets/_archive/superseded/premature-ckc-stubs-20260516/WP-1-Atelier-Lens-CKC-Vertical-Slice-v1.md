# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Atelier-Lens-CKC-Vertical-Slice-v1

## STUB_METADATA
- WP_ID: WP-1-Atelier-Lens-CKC-Vertical-Slice-v1
- BASE_WP_ID: WP-1-Atelier-Lens-CKC-Vertical-Slice
- CREATED_AT: 2026-05-16T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Atelier-Lens-CKC-Greenroom, WP-1-Atelier-Lens-CKC-Kernel
- BUILD_ORDER_BLOCKS: WP-1-Atelier-Lens, WP-1-Photo-Studio, WP-1-Studio-Runtime-Visibility
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: FEAT-ATELIER-LENS; FEAT-PHOTO-STUDIO; TOOL-COMFYUI; PRIM-Moodboard; CKC useful vertical slice
- ROADMAP_ADD_COVERAGE: SPEC=v02.185; PHASE=7.6.3; LINES=12-end-of-file-appendices:155,889,4376,5728; 10-product-surfaces:4872,5134,5391,5797
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - `.GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md` FEAT-ATELIER-LENS / FEAT-PHOTO-STUDIO / PRIM-Moodboard / TOOL-COMFYUI
  - `.GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md` 10.10 Photo Studio, 10.10.4.5 Library/DAM Functions, 10.10.5.2 ComfyUI Integration Scope, 10.10.8.3 Moodboard Workflows

## INTENT (DRAFT)
- What: Build a narrow useful CKC-in-Handshake vertical slice after the CKC kernel exists: character create/import, sheet parse/version, media attach, intake batch, collection/contact-sheet manifest, and ComfyUI lineage receipt.
- Why: The operator already used CKC and wants this core Atelier/Lens value back quickly before iterating on GUI polish or full feature parity.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Create/import one character.
  - Parse and version one character sheet.
  - Attach media with hash/provenance.
  - Run a small intake batch with accept/pending/reject transitions.
  - Create one collection and one contact-sheet manifest.
  - Ingest or register OpenPose/PoseKit metadata as sidecar lineage.
  - Record a ComfyUI recipe/run receipt with artifact refs.
  - Expose projection/API output sufficient for later UI work.
  - Validate no-data-loss, no-silent-rewrite, no-silent-delete, and replayability.
- OUT_OF_SCOPE:
  - Full production GUI.
  - Full CKC parity.
  - Advanced PoseKit calibration.
  - Batch ComfyUI orchestration.
  - Dockable window/tab shell.

## ACCEPTANCE_CRITERIA (DRAFT)
- A no-context tester can run the slice against fixtures and see character, sheet version, media, intake, collection, contact-sheet manifest, sidecar, and ComfyUI lineage records.
- The slice proves CKC core value without depending on CKC runtime code.
- Every created/updated artifact has traceable lineage and recoverable source references.
- User-authored sheet text is preserved without silent rewrite or field loss.
- SFW/NSFW ViewMode and LensExtractionTier requirements are not contradicted by projections.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: CKC Greenroom and CKC Kernel.
- Blocks: later CKC/Atelier UI, Photo Studio useful surface, Studio runtime visibility refinement.

## RISKS / UNKNOWNs (DRAFT)
- Risk: useful slice expands into full GUI parity; control by keeping API/projection-only scope.
- Risk: ComfyUI receipt becomes a fake integration; control by storing real workflow/run receipt shape even if generation is mocked.
- Risk: old CKC behavior not proven after translation; control by importing targeted CKC fixtures and parity tests.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Complete enough CKC Kernel contracts to support the slice.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Atelier-Lens-CKC-Vertical-Slice-v1.md`.
- [ ] Create official task packet via `just create-task-packet WP-1-Atelier-Lens-CKC-Vertical-Slice-v1`.
- [ ] Create 60-90 no-context implementation microtasks.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.

