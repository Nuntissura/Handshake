# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Atelier-Lens-CKC-Kernel-v1

## STUB_METADATA
- WP_ID: WP-1-Atelier-Lens-CKC-Kernel-v1
- BASE_WP_ID: WP-1-Atelier-Lens-CKC-Kernel
- CREATED_AT: 2026-05-16T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Atelier-Lens-CKC-Greenroom, WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening, WP-KERNEL-003-Sandbox-Validation-Promotion, WP-1-Stage-Media-Artifact-Portability, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-Atelier-Lens-CKC-Vertical-Slice, WP-1-Studio-Runtime-Visibility, WP-1-Photo-Studio, WP-1-Atelier-Lens
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: FEAT-ATELIER-LENS; FEAT-PHOTO-STUDIO; TOOL-COMFYUI; PRIM-Moodboard; CKC Handshake-native kernel
- ROADMAP_ADD_COVERAGE: SPEC=v02.185; PHASE=7.6.3; LINES=12-end-of-file-appendices:155,889,4376,5728; 10-product-surfaces:4872,5134,5391,5797
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - `.GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md` FEAT-ATELIER-LENS / FEAT-PHOTO-STUDIO / PRIM-Moodboard / TOOL-COMFYUI
  - `.GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md` 10.10 Photo Studio, 10.10.4.5 Library/DAM Functions, 10.10.5.2 ComfyUI Integration Scope, 10.10.8.3 Moodboard Workflows

## INTENT (DRAFT)
- What: Rebuild the CastKit Codex domain as a Handshake-native Atelier/Lens kernel using Rust/Tauri boundaries, PostgreSQL authority, EventLedger lineage, artifact store integration, and typed validation/promotion hooks.
- Why: CKC was always a core Atelier/Lens feature path; rebuilding it natively avoids a separate-product detour while preserving character, media, ComfyUI, PoseKit, search, export, and automation goals.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Rust domain modules for character profiles, sheets/templates, media assets, intake, collections, sidecars, PoseKit metadata, ComfyUI receipts, search projections, exports, and automation commands.
  - PostgreSQL migrations and repository/service boundaries.
  - EventLedger events for imports, mutations, artifact proposals, ComfyUI lineage, validation, promotion, export, and recovery.
  - Artifact store integration and portable manifests.
  - Tauri command facade for thin future React projections.
  - PostgreSQL-only CKC parity test harness adapted from greenroom source maps. No SQLite fixture, compatibility path, cache, fallback, harness, or temporary adapter is allowed.
- OUT_OF_SCOPE:
  - Full GUI rebuild.
  - Direct CKC runtime dependency.
  - Electron IPC, localhost intake as authority, and SQLite in any form. SQLite is not accepted in runtime, tests, fixtures, compatibility, imports, fallback, cache, examples, harnesses, or temporary adapters.
  - Full PoseKit calibration UI and batch ComfyUI generation UI.

## ACCEPTANCE_CRITERIA (DRAFT)
- Character, sheet, media, intake, collection, sidecar, ComfyUI, and export domain contracts exist in Handshake-native modules.
- PostgreSQL is the only runtime authority path for Kernel V1 data.
- Every significant mutation emits or can emit an EventLedger lineage event.
- Artifact outputs use configured artifact roots and manifests, not `.GOV` or CKC app folders.
- CKC PostgreSQL-only parity cases prove no silent rewrite, no silent delete, stable IDs, version append-only behavior, and recoverable media lineage.
- Existing LensExtractionTier, ViewMode, Studio runtime visibility, Stage media artifact portability, and Photo Studio gap intents are explicitly carried forward or linked.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: CKC Greenroom; active Kernel002/Kernel003 substrate; artifact-system and Stage/media portability decisions.
- Blocks: CKC Vertical Slice, later Atelier/Lens UI, Photo Studio runtime rebuild, Studio runtime visibility.

## RISKS / UNKNOWNs (DRAFT)
- Risk: product domain grows too wide; control by implementing backend contracts before UI.
- Risk: CKC code is ported instead of translated; control by using preserve/adapt/reject decisions from greenroom.
- Risk: old SQLite paths leak into tests; control by no-SQLite tripwires and PostgreSQL-only proof.
- Risk: ComfyUI lineage becomes opaque; control by requiring typed workflow/run receipts and artifact refs.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Complete or accept CKC Greenroom output.
- [ ] Confirm current Kernel substrate dependencies and active worktree target.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Atelier-Lens-CKC-Kernel-v1.md`.
- [ ] Create official task packet via `just create-task-packet WP-1-Atelier-Lens-CKC-Kernel-v1`.
- [ ] Create 80-120 no-context implementation microtasks.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
