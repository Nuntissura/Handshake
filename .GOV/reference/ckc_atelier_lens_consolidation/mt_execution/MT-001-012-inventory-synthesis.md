---
file_id: mt-001-012-inventory-synthesis
file_kind: inventory_synthesis
updated_at: 2026-05-16
wp_id: WP-1-Atelier-Lens-Consolidation-v1
status: complete
---

# MT-001..012 Inventory Synthesis

<topic id="scope" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

This synthesis integrates the first execution tranche of `WP-1-Atelier-Lens-Consolidation-v1`.

Covered microtasks:
- `MT-001` package/runtime dependencies
- `MT-002` backend service files
- `MT-003` UI views and reusable behavior
- `MT-004` PoseKit files
- `MT-005` ComfyUI bridge files
- `MT-006` tests by behavior area
- `MT-007` CKC spec headings and requirement sections
- `MT-008` CKC taskboard statuses
- `MT-009` existing Handshake Atelier/Lens stubs
- `MT-010` adjacent Photo/Studio/media/Loom/ASR/artifact stubs
- `MT-011` prior packets for supersession risk
- `MT-012` current Handshake product code anchors relevant to Atelier/Lens

SQLite remains absolutely rejected for Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.

</topic>

<topic id="source-artifacts" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Source Artifacts

- `MT-001-006-ckc-code-inventory.md/json`: CKC code/runtime/backend/UI/PoseKit/ComfyUI/test evidence.
- `MT-007-008-ckc-spec-taskboard-inventory.md/json`: CKC spec and taskboard evidence.
- `MT-009-012-handshake-anchor-inventory.md/json`: Handshake stub, adjacent packet, supersession, and product-anchor evidence.

All three lanes were read-only against CKC and did not create CKC rebuild stubs.

</topic>

<topic id="ckc-code-findings" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## CKC Code Findings

- CKC is useful source evidence for library, character, intake, PoseKit, workflow, ComfyUI, export, settings, and automation surfaces.
- CKC runtime shape is Electron/Vite/React with an Electron main process, preload command bridge, React renderer, and model-visible automation/diagnostic command surface.
- CKC backend is concentrated in `app/backend/library.js`; Handshake should split bounded services rather than copy CKC's monolith.
- CKC PoseKit evidence includes MediaPipe model assets, worker-based detection, OpenPose JSON/PNG export, rig calibration, identity profiles, and ComfyUI replay.
- CKC ComfyUI bridge shows a strong pattern: generation output is saved by ComfyUI first, then registered into the product through a local intake endpoint. Handshake should preserve the receipt pattern but define its own schema, auth, ArtifactStore, and EventLedger contract.
- CKC behavior tests are useful coverage inspiration, but CKC SQLite fixtures and compatibility tests are rejected for Handshake.

</topic>

<topic id="ckc-governance-findings" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## CKC Spec And Taskboard Findings

- CKC evidence clusters around governance/verification, automation/manual consistency, Intake/Inbox, Collections/contact sheets, PoseKit/OpenPose, parallel editing, recoverable deletion, storage/runtime, and product UX preservation.
- CKC taskboard inventory found 135 work-packet rows: 113 `DONE`, 14 `REVIEW`, 1 `BLOCKED`, 1 `CONCEPT`, and 6 planned variants.
- `WP-0133` PoseKit calibration/history remains blocked and must not be silently imported as solved Handshake behavior.
- `WP-0136` raster contact sheet export is planned, not shipped evidence.
- `WP-0119` SQLite removal/quarantine remains planned in CKC; Handshake must go further and forbid SQLite everywhere.
- CKC review-batch evidence is valuable but not final Handshake proof. Future Handshake implementation packets must cite CKC source anchors and re-verify against Handshake stack boundaries.

</topic>

<topic id="handshake-anchor-findings" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Handshake Anchor Findings

- The current worktree is governance-kernel-only. Product code anchors are candidate paths from packet evidence until verified in the product worktree.
- Existing Atelier/Lens intent is split across the active consolidation packet, `WP-1-Atelier-Lens-v2`, validated `WP-1-Atelier-Collaboration-Panel-v1`, `WP-1-Lens-Extraction-Tier-v1`, validated `WP-1-Lens-ViewMode-v1`, and adjacent Lens-pattern stubs.
- Adjacent Photo/Studio/media/Loom/ASR/artifact work supplies the reusable implementation runway: Photo Studio, Studio Runtime Visibility, Stage Media Artifact Portability, Stage ASR Transcript Lineage, ASR Transcribe Media, Video Archive Loom Integration, Media Downloader Loom Bridge, Loom Preview Poster Frames, Loom MVP, Loom Storage Portability v4, and Artifact System Foundations.
- Supersession risk is real around old Atelier/Lens, Photo Studio, Media Downloader v1, Loom portability v1-v3, superseded visual-debug stubs, and archived premature CKC rebuild stubs.
- Archived premature CKC stubs remain evidence only. They must not be revived directly.

</topic>

<topic id="translation-decisions" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Translation Decisions

- Preserve CKC capability intent, not CKC implementation shape.
- Translate storage and collaboration to PostgreSQL, EventLedger, ArtifactStore, CRDT/workspace contracts, and governed Handshake runtime boundaries.
- Translate CKC automation/manual/debugger concepts into Handshake diagnostics, command receipts, model-facing manuals, and visual verification surfaces.
- Translate CKC UI evidence into workflows and state requirements, not a GUI copy.
- Treat CKC `DONE` rows as historical evidence, `REVIEW` rows as implemented-but-not-final evidence, and `BLOCKED`/`PLANNED`/`CONCEPT` rows as risk or backlog signals.
- Product code anchors must be re-verified in the product worktree before implementation.

</topic>

<topic id="next-tranche" status="ready" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Next Tranche

Proceed to `MT-013` through `MT-026`: extract media/DAM, character sheet, template/parser, intake/inbox, collection/contact-sheet, sidecar/versioning, PoseKit/OpenPose, identity profile, ComfyUI workflow, automation/manual/debug, search/tag/similarity, export/backup/share-pack, no-rewrite/no-censorship preservation, and path/naming portability requirements.

Do not create CKC rebuild stubs yet. Requirement extraction must finish before future Handshake-native rebuild packets are authored.

</topic>
