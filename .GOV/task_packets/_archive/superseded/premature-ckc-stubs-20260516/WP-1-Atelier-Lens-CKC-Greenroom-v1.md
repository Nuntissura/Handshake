# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Atelier-Lens-CKC-Greenroom-v1

## STUB_METADATA
- WP_ID: WP-1-Atelier-Lens-CKC-Greenroom-v1
- BASE_WP_ID: WP-1-Atelier-Lens-CKC-Greenroom
- CREATED_AT: 2026-05-16T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: GOV
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: NONE
- BUILD_ORDER_BLOCKS: WP-1-Atelier-Lens-CKC-Kernel, WP-1-Atelier-Lens-CKC-Vertical-Slice
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: FEAT-ATELIER-LENS; FEAT-PHOTO-STUDIO; TOOL-COMFYUI; PRIM-Moodboard; CKC consolidation source material
- ROADMAP_ADD_COVERAGE: SPEC=v02.185; PHASE=7.6.3; LINES=12-end-of-file-appendices:155,889,4376,5728; 10-product-surfaces:4872,5134,5391,5797
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - `.GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md` FEAT-ATELIER-LENS / FEAT-PHOTO-STUDIO / PRIM-Moodboard / TOOL-COMFYUI
  - `.GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md` 10.10 Photo Studio, 10.10.4.5 Library/DAM Functions, 10.10.5.2 ComfyUI Integration Scope, 10.10.8.3 Moodboard Workflows

## INTENT (DRAFT)
- What: Consolidate CastKit Codex code, spec, taskboard, and existing Handshake Atelier/Lens-adjacent stubs into a no-loss greenroom source register for a Handshake-native rebuild.
- Why: Speed the rebuild of a core Atelier/Lens feature without dropping CKC goals, prompt-diary lineage, or already-planned Handshake technical requirements.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Preserve CKC code/spec/taskboard intent as source requirements.
  - Preserve existing Handshake Atelier/Lens, Photo Studio, Lens tier/ViewMode, Studio runtime, Stage/media, ASR, Loom/media, screenshot, visual-debug, and artifact-foundation stub intent.
  - Produce the conflict matrix for SQLite, Electron, localhost intake, CKC product strings, product-output paths, and sparse legacy stubs.
  - Produce namespace migration, non-SQLite parity case register, acceptance gates, and official packet handoff material.
  - Use `.GOV/reference/ckc_atelier_lens_consolidation/consolidated-runway.md` and `.json` as source reference.
- OUT_OF_SCOPE:
  - Product runtime implementation.
  - UI rebuild.
  - CKC feature parity claims.
  - Marking downstream CKC WPs Ready for Dev.

## ACCEPTANCE_CRITERIA (DRAFT)
- The greenroom register preserves CKC capability clusters and existing Handshake stub goals without silent deletion.
- Each conflict has a Handshake-native resolution or a deferred decision.
- The downstream CKC Kernel and Vertical Slice packet scopes can be created from the register without rereading the full CKC app/spec/taskboard.
- Existing validated or superseded stubs are classified as baseline, source material, inherited requirement, or separate dependency.
- SQLite is explicitly rejected in every Handshake context: runtime, tests, fixtures, compatibility, imports, fallback, cache, examples, harnesses, and temporary adapters. Electron/localhost-intake assumptions are rejected for Kernel V1 runtime authority.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: NONE.
- Blocks: CKC Kernel and CKC Vertical Slice activation.
- Source reference: `.GOV/reference/ckc_atelier_lens_consolidation/`.

## RISKS / UNKNOWNs (DRAFT)
- Risk: consolidation compresses away old intents; control by keeping source-path-backed carry-forward rows.
- Risk: CKC speed goal causes runtime architecture shortcuts; control by rejecting SQLite absolutely and rejecting Electron/localhost authority before implementation.
- Risk: stubs with sparse contracts hide meaningful Markdown intent; control by preserving Markdown projection content before supersession.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body and Appendix feature/tool/primitive rows.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Atelier-Lens-CKC-Greenroom-v1.md`.
- [ ] Create official task packet via `just create-task-packet WP-1-Atelier-Lens-CKC-Greenroom-v1`.
- [ ] Copy relevant scope/acceptance notes from this stub and `.GOV/reference/ckc_atelier_lens_consolidation/`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
