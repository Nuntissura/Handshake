---
file_id: ckc-greenroom-requirements-register
file_kind: greenroom_requirements_register
updated_at: 2026-05-16
status: reference_only_not_execution_authority
source_reports:
  - ckc-code-greenroom.json
  - ckc-spec-taskboard-map.json
  - handshake-stub-preservation-map.json
---

<topic id="purpose" status="active" version="v1" summary="Greenroom purpose and authority limits" updated_at="2026-05-16">

# Purpose

This register is the CKC Greenroom output. It preserves source requirements for rebuilding CastKit Codex inside Handshake as Atelier/Lens and adjacent Photo Studio capability.

This file is not an executable work packet. Official work starts only after refinement, signature, packet creation, and task-board transition.

The register exists so later packets can implement quickly without losing:

- CKC code intent.
- CKC spec and taskboard goals.
- Prompt-diary / Atelier-Lens original motivation.
- Existing Handshake Atelier/Lens, Photo Studio, Studio, Stage/media, Loom/media, ASR, ViewMode, extraction-tier, visual-debug, and artifact-system goals.

</topic>

<topic id="source-material" status="active" version="v1" summary="Preserved source material" updated_at="2026-05-16">

# Source Material

CKC sources:

- `D:/Projects/LLM projects/CastKit-Codex/CKC_main`
- `D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/spec/CastKit_Codex_Spec_v00.075.md`
- `D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/taskboard/TASK_BOARD.md`

Handshake sources:

- `.GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.md`
- `.GOV/task_packets/stubs/WP-1-Photo-Studio-v2.md`
- `.GOV/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.md`
- `.GOV/task_packets/stubs/WP-1-Lens-Extraction-Tier-v1.md`
- `.GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.md`
- `.GOV/task_packets/stubs/WP-1-Stage-Media-Artifact-Portability-v1.md`
- `.GOV/task_packets/stubs/WP-1-Stage-ASR-Transcript-Lineage-v1.md`
- `.GOV/task_packets/stubs/WP-1-Studio-Runtime-Visibility-v1.md`
- Adjacent media/Loom/ASR/artifact/visual-debug stubs listed in `handshake-stub-preservation-map.json`.

</topic>

<topic id="ckc-capability-clusters" status="active" version="v1" summary="CKC capability clusters to preserve" updated_at="2026-05-16">

# CKC Capability Clusters

1. Media viewer / DAM.
   - Preserve image-first portfolio browsing, character galleries, slideshow, fullscreen, metadata, missing-media states, repair flows, ratings, favorites, tags, notes, provenance, deleted/archive visibility controls, and OpenPose sidecar hiding.

2. Character sheets.
   - Preserve character profile plus sheet as core product shape, byte-preserved user text, explicit/adult fields as first-class, protected public IDs, internal IDs, append-only versions, templates, cloning, merge preview, and selective apply.

3. Inbox / intake.
   - Preserve direct image ingress, persistent batches, accept/reject/pending, linked/loose mode, character/sheet/collection linkage, source preservation, and no silent deletes.

4. Collections / contact sheets.
   - Preserve cross-character curated image sets, collection notes/tags, optional character/sheet-version links, slideshow, export, SVG contact sheet with manifest, source IDs/hashes, tags, layout metadata, and raster contact-sheet follow-up.

5. Sidecars / versioning.
   - Preserve OpenPose PNG/JSON sidecars, append-only sheet versions, reverts-as-new-version, archive/restore, orphan manifests, event logs, entity revisions, compatibility fixtures, migrations, and idempotency checks.

6. PoseKit / OpenPose.
   - Preserve blank/single-photo/collection workbench modes, body-18, face-70, left/right hand-21 arrays, zero-filled absent hands, yaw/pitch/roll, identity profiles, deterministic crops, landmarks, measurements, and blocked calibration/history debt.

7. ComfyUI bridge.
   - Preserve local workflow/run lineage, history, prompt extraction, replay, stats, registered outputs, workflow/Pose tab replay paths, identity reference payload intent, and prompt-response matrix as later scope.

8. Automation / debug / manual.
   - Preserve typed command map, in-app manual, automation sessions, leases, heartbeats, command log, captures, background/no-focus/no-OS-input invariants, visual captures, and command/manual consistency tests.

9. Search / tags / similarity.
   - Preserve global search across sheets, notes, stories, moodboards, image metadata, grouped results, snippets, jump targets, tag manager, links/backlinks, AI suggestions, palettes, duplicates, and perceptual similarity.

10. Exports.
    - Preserve empty templates, LLM packs, filled sheets, image sets, moodboards, collections, share packs, backups, web portfolios, contact sheets, batch exports, field/section selections, presets, no-space names, and provenance.

</topic>

<topic id="handshake-stub-goals" status="active" version="v1" summary="Existing Handshake goals carried forward" updated_at="2026-05-16">

# Handshake Stub Goals Carried Forward

- `WP-1-Atelier-Lens-v2`: preserve role claiming, SceneState, and ConflictSet gaps.
- `WP-1-Photo-Studio-v2`: preserve skeleton surface, thumbnails, and recipes gaps.
- `WP-1-Atelier-Collaboration-Panel-v1`: preserve selection-scoped role suggestions, range-bounded patching, and provenance.
- `WP-1-Lens-Extraction-Tier-v1`: preserve Tier0/Tier1/Tier2, Tier1 default, explicit override, requested/effective trace, and validation.
- `WP-1-Lens-ViewMode-v1`: preserve NSFW default, explicit SFW toggle, SFW hard-drop projection, immutable raw artifacts, and trace-visible filter.
- `WP-1-Stage-Media-Artifact-Portability-v1`: preserve portable artifact manifests, bundle index, retention evidence, storage portability, and replayable provenance.
- `WP-1-Stage-ASR-Transcript-Lineage-v1`: preserve source hashes, media probe facts, capture/session provenance, timing anchors, and searchable transcript linkage.
- `WP-1-Studio-Runtime-Visibility-v1`: preserve Studio job/tool/workflow mapping, DCC/operator projection, Flight Recorder linkage, Locus/WP linkage, and storage posture.
- Loom/media/ASR/video stubs: preserve LoomBlocks, captions sidecars, transcript documents, poster frames, tags, mentions, search facets, and local-first ASR fallback as adjacent media-intelligence path.
- Screenshot/visual-debug stubs: preserve screenshot capture and visual regression evidence as inherited Kernel validation requirements.
- Artifact-system and structured-collaboration stubs: preserve manifests, SHA-256, atomic materialization, retention/GC, structured packet summaries, and machine-readable collaboration artifacts as separate foundations.

</topic>

<topic id="non-negotiable-requirements" status="active" version="v1" summary="Non-negotiable requirements" updated_at="2026-05-16">

# Non-Negotiable Requirements

- No user-authored sheet or prompt text may be silently rewritten, censored, euphemized, hidden, normalized, or dropped.
- Adult/explicit fields remain first-class product data.
- Public character IDs are protected system-managed fields; internal IDs remain stable storage keys.
- Sheet changes are diffed, selective, append-only, and versioned.
- Import/intake never silently deletes sources.
- Sidecars are explicit artifact rows or refs, hidden from normal galleries unless requested.
- ViewMode is projection-only; it must not mutate raw or derived artifacts.
- LensExtractionTier is separate from content tier.
- ComfyUI outputs require typed workflow/run receipts and artifact lineage.
- Exports require provenance-preserving manifests and no-space portable names.
- Product runtime must not use `.GOV` as product output, cache, or artifact root.
- Handshake must not use SQLite in any form. This includes runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. Handshake uses PostgreSQL only.

</topic>

<topic id="minimum-greenroom-done-means" status="active" version="v1" summary="Greenroom completion criteria" updated_at="2026-05-16">

# Minimum Greenroom Done Means

The Greenroom is complete when downstream workers can implement CKC Kernel and Vertical Slice without reading the whole CKC repository.

Minimum outputs:

- Requirements register.
- Translation/conflict matrix.
- Stub preservation matrix.
- Capability-cluster map.
- Atelier/Lens versus CKC overlap matrix.
- CKC evolved-feature and convenience-driven requirement register.
- CKC taskboard status signal summary.
- Source fixture/test inventory, excluding all SQLite fixtures and SQLite-backed tests.
- Namespace migration notes.
- Microtask map with 60+ buckets.
- Corrected reference brief feeding `WP-1-Atelier-Lens-Consolidation-v1`.

This file plus the sibling Greenroom artifacts satisfy the reference-output side. Official packet activation still requires the project workflow gates.

</topic>

<topic id="ckc-taskboard-status-signals" status="active" version="v1" summary="CKC taskboard status signals that affect consolidation" updated_at="2026-05-16">

# CKC Taskboard Status Signals

Source: `D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/taskboard/TASK_BOARD.md`.

- CKC has a large completed base through `WP-0115`, including portfolio layout, media metadata, moodboards, exports, search, collections, PostgreSQL storage, automation, intake, PoseKit pipeline, workflow replay, identity profiles, hand detection, and multi-rig tabs.
- `WP-0122` through `WP-0132`, `WP-0134`, `WP-0135`, and `WP-0137` are in REVIEW. Treat these as implemented evidence that still needs review-batch GUI/sample-corpus evidence before being called fully accepted.
- `WP-0133` is BLOCKED. Preserve the debt explicitly: draggable calibration overlay, missing-marker placement flow, 3D/live split editing, and forked history.
- `WP-0112`, `WP-0114`, `WP-0116`, `WP-0117`, `WP-0119`, and `WP-0136` are planned. Treat them as backlog candidates, not already-shipped behavior.
- `WP-0118` is concept-only. Preserve model prompt-response matrix intent as later research, not current implementation scope.
- CKC's `WP-0119` PostgreSQL-first testing and SQLite removal remains unresolved in CKC. Handshake must not inherit ambiguity: SQLite remains forbidden in every Handshake runtime, test, fixture, mock, example, fallback, cache, compatibility adapter, temporary harness, import, export, or product path.

</topic>
