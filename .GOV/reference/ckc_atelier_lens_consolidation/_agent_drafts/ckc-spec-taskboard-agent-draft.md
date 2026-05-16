---
file_id: ckc-spec-taskboard-agent-draft
file_kind: greenroom-extraction-draft
updated_at: 2026-05-16
source_spec: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/spec/CastKit_Codex_Spec_v00.075.md
source_taskboard: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/taskboard/TASK_BOARD.md
target_context: Handshake Atelier/Lens consolidation
---

# CKC Spec/Taskboard Greenroom Extraction For Handshake Atelier/Lens

## Extraction Boundary

This draft extracts source-backed CKC behavior from `CastKit_Codex_Spec_v00.075.md` and `TASK_BOARD.md` for Handshake Atelier/Lens consolidation. It is a greenroom planning artifact, not product output.

Hard rejection flags for Handshake:
- SQLite is forbidden in any form for Handshake. CKC SQLite references are evidence of CKC history only, not reusable architecture.
- Electron is not authority. CKC Electron implementation details can inspire UX or automation constraints but cannot define Handshake architecture.
- Localhost intake is not authority. CKC's localhost-only bridge is a product-specific implementation detail that must be redesigned or replaced for Handshake.
- CKC namespace must migrate. Names such as CKC, CastKit Codex, PoseKit, Character, Rig, Workflow, and CKC_GOV must be mapped into Handshake-owned Atelier/Lens vocabulary before adoption.
- `.GOV` is not product output. Governance artifacts may guide planning, validation, and agent workflows but must not be surfaced as shipped product content.

## Source Anchors

- Spec is the current working spec: spec lines 1-9.
- Current review-batch hardening scope: spec lines 11-46.
- Automation/manual source-of-truth and command surfaces: spec lines 331-358 and 521-553.
- Library data layout and PostgreSQL direction: spec lines 460-579.
- Product UX and image/library requirements: spec lines 655-725 and 1476-1668.
- Taskboard binding status contract and status legend: taskboard lines 1-16.
- Taskboard current focus and unresolved work: taskboard lines 157-184.

## Source-Backed CKC Requirement Clusters

### Image-First Library And Character Sheet Surface

CKC's recovered product direction is a "Character sheet + portfolio viewer" where images are the hero and sit side-by-side with the sheet. Source: spec lines 1491-1518.

Key requirements:
- Front page uses a global carousel across all images marked for carousel/frontpage use. Source: spec lines 1507-1512 and 439-457.
- Character page uses a character-specific carousel/photos toggle with the sheet editor beside it. Source: spec lines 1513-1517.
- Notes, stories, and moodboard mode expand to a three-panel layout without losing image context. Source: spec lines 1518-1523.
- Media chrome is intentionally minimal, with controls and filters hidden by default. Source: spec lines 1525-1535 and 655-676.
- Ratings are assignable, filterable, and slideshow-aware. Source: spec lines 1540-1545 and taskboard lines 24, 31.

Handshake overlap:
- Strong Lens overlap: image-first browsing, filtered galleries, rating/tag metadata, full-image thumbnails, slideshow/reference review.
- Strong Atelier overlap if Atelier owns character/reference-sheet authoring, notes, stories, moodboards, exports, and curated collections.

### Intake, Inbox, And Triage

CKC evolved toward a persistent intake pipeline. Source: spec lines 21-25, 556-569; taskboard lines 140-145.

Key requirements:
- Persistent intake batches with item selection, linked/loose classification, character linkage, optional sheet-version/collection linkage, pending Inbox parking, and derivative recognition. Source: spec lines 21-25.
- Inbox/watch-folder import and pending review exist as durable features. Source: taskboard lines 75, 114, 142-145.
- Direct image ingress is routed toward Intake instead of random product entrypoints. Source: taskboard line 141.
- Filesystem health checks cover images, thumbnails, pending Inbox items, untracked originals, and Pose/OpenPose sidecars. Source: taskboard line 145.

Handshake overlap:
- Lens should adopt the concept of a canonical intake/triage lane for external references and generated outputs.
- Atelier should reuse the assignment/linkage pattern: pending assets can be linked later to a person/character, project, shoot, board, collection, or version.
- Handshake should preserve CKC's split between accepted, pending, and rejected lanes, but rename the entities and avoid CKC paths/names.

### Collections, Contact Sheets, And Curated Sets

CKC collections carry notes, tags, optional character/sheet-version links, batch tag application, contact-sheet creation, and export pathways. Source: spec lines 21-25; taskboard lines 89, 146-147, 154.

Key requirements:
- Collections/playlists are cross-character image sets with CRUD, slideshow, and export. Source: spec lines 896-907 and taskboard line 89.
- Collections v2 adds notes, tags, optional character/sheet-version links, Library lane navigation, batch tag application, and contact-sheet creation. Source: taskboard line 146.
- Contact sheets currently produce SVG plus manifest with no-space artifact names, source image IDs/hashes, tags, layout metadata, and optional character/sheet-version linkage. Source: spec lines 21-25 and taskboard line 147.
- Raster PNG/JPG contact sheet export remains planned. Source: spec line 24 and taskboard lines 154, 173.

Handshake overlap:
- Lens can use curated sets/contact sheets as first-class review artifacts.
- Atelier can use contact sheets as planning, comparison, shot-list, and visual continuity artifacts.
- Manifest-backed contact sheets are high value for agent workflows because they preserve source IDs/hashes and layout metadata.

### Pose/OpenPose/Identity/Workflow

CKC's PoseKit surface is the densest production-technical cluster. Source: spec lines 27-30, 80-168, 108-129, 140-168; taskboard lines 125-133, 149-151.

Key requirements:
- Pose workbench can open blank, single-photo, or collection contexts; collection entry carries selected collection context. Source: spec lines 27-30 and taskboard line 149.
- OpenPose artifacts use deterministic no-space PNG/JSON sidecar paths; multiple sidecars can link to one source/rig. Source: spec line 30 and taskboard line 150.
- Pose data includes body-18, face-70, hand-21, zero-filled hand arrays when no hands pass gating, and canonical OpenPose JSON fields. Source: spec lines 80-90.
- Head pose uses yaw/pitch/roll with quaternion-backed storage and UI/automation controls. Source: spec lines 140-153.
- Multi-rig workspace tabs are session-scoped, accessible, automation-wired, and preserve rig/image rows on close. Source: spec lines 50-76 and taskboard line 133.
- Identity profiles store a 512x512 face crop, landmarks, measurements, pose metadata, and bridge payload for downstream IPAdapter/face-reference workflows. Source: spec lines 108-123 and taskboard line 129.
- Workflow replay can inject identity profile data into bridge inputs. Source: spec lines 118-123.

Handshake overlap:
- Lens can absorb the "source image plus derivative sidecars" model for pose/control/reference artifacts.
- Atelier can absorb identity profile concepts where reusable face/reference/pose assets need provenance, deterministic crops, and downstream generator payloads.
- The sidecar/manifest contract is portable and more valuable than the CKC UI implementation.

Handshake caution:
- CKC product names `PoseKit`, `Rig`, `Workflow`, and `CastKitCodexBridge` must migrate.
- CKC's ComfyUI localhost intake/replay mechanics are not authority for Handshake. Only the higher-level concepts of workflow capture, provenance, replay, and bridge payloads should be considered.

### LLM Automation, Manual, Visual Debugging, And Parallel Operation

CKC has a mature agent-facing operation surface. Source: spec lines 15-19, 331-358, 521-553; taskboard lines 113, 115, 119, 138-140, 155, 158, 171-174.

Key requirements:
- UI-affecting work must capture affected routes/panels/windows at normal and constrained/resized viewports, record capture paths, and report console/runtime errors. Source: spec lines 15-19.
- Automation manual, command map, dispatcher/preload wiring, and tests must remain consistent. Source: spec lines 37-40 and 335-343.
- Automation exposes sessions, heartbeats, leases, logs, commands, captures, renderer/backend state, and safe synthetic input. Source: spec lines 339-358 and 521-553.
- Background automation must not steal focus or attention. Source: spec lines 349-355 and taskboard lines 113, 115, 119.
- Parallel editing policy is hybrid: PostgreSQL source of truth, sessions/leases for coordination, optimistic revisions and append-only product events for durability, and CRDT-style merges only for safe shapes. Source: spec lines 32-36 and taskboard line 174.

Handshake overlap:
- Direct governance/agent-workflow overlap. Handshake should adopt the LLM manual, command consistency tests, state inspection, capture evidence, session/lease logs, and background-safe operation concepts.
- For Lens/Atelier, this is likely as important as product UX: it makes parallel model work observable and recoverable.

### Storage, Portability, And Governance Split

CKC moved from SQLite history toward PostgreSQL-first operation. Source: spec lines 439-440, 505-517, 571-579; taskboard lines 112, 137, 169, 174.

Key requirements:
- PostgreSQL is default/current provider for new CKC runs and parallel operator/model work. Source: spec lines 439-440 and taskboard line 112.
- SQLite is retained only as a CKC legacy/test fallback in lower-level DB boundaries. Source: spec lines 516-517.
- Product behavior tests should target PostgreSQL first; SQLite-only tests are insufficient except legacy fixture/import compatibility. Source: taskboard lines 137 and 169.
- Generated artifacts use no-space paths. Source: spec lines 21-25 and taskboard line 116.
- Governance lives in CKC_GOV and must not be mirrored into product docs. Source: taskboard lines 1-10 and 111.

Handshake overlap:
- PostgreSQL-first, no-space artifact naming, manifest-backed outputs, and governance/product-code split are directly reusable.

Handshake rejection:
- Handshake should strengthen CKC's partial direction: SQLite is forbidden entirely, including fallback, fixtures, migrations, docs, tests, and examples unless the operator explicitly creates a one-off external import research artifact outside product authority.

## Data Entities Found In CKC

Source-backed CKC entities and likely Handshake mappings:

- `ImageAsset`: image file, metadata, tags, ratings, hashes, palette, dHash, sidecar role, provenance. Handshake candidate: Lens asset/reference/media item.
- `Character`: CKC person/character record with public ID, sheet values, icon, images. Handshake candidate: Atelier subject/model/character/project entity depending on domain taxonomy.
- `FieldValue` and sheet versions: structured character sheet values with versioning, validation, diff, merge, revert. Handshake candidate: structured profile/spec/version records.
- Notes, Stories, Moodboards: separate libraries with smart tags and document-type metadata. Handshake candidate: Atelier creative notebooks/boards.
- `Collection` and `CollectionItem`: curated image sets/playlists. Handshake candidate: Lens collection/contact set/shot set.
- Intake batch / ingestion batch / ingestion rejection: accepted, pending, rejected, deduped intake records. Handshake candidate: import batch and triage audit.
- `Rig`: pose/control representation derived from an image. Handshake candidate: pose/control artifact or Lens derivative.
- `IdentityProfile`: deterministic face crop plus landmarks/measurements/bridge payload. Handshake candidate: identity/reference profile.
- Workflow specs, workflow history, prompt/story beats, ComfyUI outputs: generation workflow provenance and replay. Handshake candidate: generator workflow run/capture/replay records.
- Automation session, lease, event log, entity revision, product event: model/operator coordination and audit. Handshake candidate: agent session, operation lease, event journal, revision ledger.
- Contact sheet manifest: generated visual artifact plus source IDs/hashes/layout metadata. Handshake candidate: review sheet manifest.

## Evolved And Convenience-Driven Features

The CKC taskboard shows repeated convenience evolution from core library to power-user workflow:

- Hidden/minimal controls, drawer navigation, command palette, global search, hotkeys, fullscreen, slideshow, and saved filters. Sources: taskboard lines 23-31, 50, 92, 103.
- Drag/drop, clipboard paste, URL import, Inbox/watch-folder import, and direct ingress cleanup. Sources: taskboard lines 75-76, 84, 141.
- Batch metadata edits, bulk character operations, tag manager, smart folders, duplicate/near-duplicate search, visual similarity, color tools, and performance caps. Sources: taskboard lines 77-87, 103, 107-110.
- Moodboard maturity: layers, transform, undo/redo, text/stickies, gradients, zoom/pan/grid, shapes, connectors, masks, selection tools, numeric inspector, rulers/guides, folders/search/tags, styling, export. Sources: taskboard lines 68-74, 94-102.
- Repair, diagnostics, backup/restore, reset modes, filesystem health, recoverable deletion, and orphan adoption. Sources: taskboard lines 45-48, 93, 145, 153; spec lines 202-222.
- Agent convenience: automation manual, state inspection, visual capture, sessions/leases/logs, background mode, manual consistency tests. Sources: taskboard lines 113, 115, 119, 138-140, 155.

Handshake relevance:
- These are not all base scope for Atelier/Lens, but they are strong ROI candidates when touching the same subsystem.
- Highest ROI for consolidation: intake/triage, collections/contact sheets, global search, batch metadata, visual similarity, repair/diagnostics, and agent automation surfaces.

## Taskboard Status Signals

- Large completed base: WP-0001 through WP-0115 are mostly DONE, including portfolio layout, media metadata, moodboards, exports, search, collections, PostgreSQL storage, automation, intake, PoseKit pipeline, workflow replay, identity profiles, hand detection, and multi-rig tabs. Source: taskboard lines 21-133.
- Review batch: WP-0122 through WP-0132 and WP-0134 through WP-0135 are REVIEW; WP-0137 hardening is REVIEW. Source: taskboard lines 140-155.
- Blocked: WP-0133 PoseKit calibration/markers/history is BLOCKED for draggable calibration overlay, missing-marker placement flow, 3D/live split editing, and forked history. Source: taskboard line 151.
- Planned: WP-0112 multi-subject scenes, WP-0114 Pose tab polish, WP-0116 keyboard shortcuts, WP-0117 stylized detector research, WP-0119 PostgreSQL-first testing/SQLite removal, WP-0136 raster contact sheet export. Source: taskboard lines 130, 132, 134-137, 154.
- Concept-only: WP-0118 model prompt-response matrix is parked as future concept. Source: taskboard lines 136 and 168.
- Current focus: WP-0137 hardening in REVIEW, full npm test passes, but review-batch GUI/sample-corpus captures remain before DONE promotion. Source: taskboard line 158.
- Deferred validation: packaged-build smoke for WP-0099/WP-0100/WP-0103/WP-0104; NAS mirror backup; DB-backed live PostgreSQL container verification. Source: taskboard line 182.

## Unresolved Work And Handshake Implications

- SQLite removal is unresolved in CKC. For Handshake, do not inherit this ambiguity; make PostgreSQL or another approved non-SQLite store explicit from the first contract. Source: taskboard lines 137 and 169.
- Raster contact-sheet export is planned, not shipped. Handshake can adopt SVG/manifest first and decide whether raster export belongs in Lens MVP. Source: spec line 24 and taskboard line 154.
- PoseKit calibration/history interaction is blocked. Handshake should not assume CKC solved deep pose-edit UX. Source: taskboard line 151.
- Multi-subject scenes are planned, not shipped. Handshake should treat multi-person/multi-subject pose/control as future scope unless already required. Source: taskboard line 130.
- Stylized portrait landmark research is planned, not productized. Handshake can reuse the research shape but should run its own current field pass before implementation. Source: taskboard line 135.
- Localhost ComfyUI intake/replay exists in CKC, but Handshake should treat it as non-authoritative implementation detail because the operator explicitly flagged localhost intake as not authority. Source for CKC behavior: taskboard line 127.

## Overlap Candidates For Handshake Atelier/Lens

1. Lens asset intake and triage
   - Reuse CKC patterns: persistent batches, accepted/pending/rejected lanes, classification, pending Inbox, derivative recognition, filesystem health.
   - Rename entities and remove CKC paths/names.
   - Validate with state inspection, batch audit, and GUI capture evidence.

2. Lens collections and contact sheets
   - Reuse CKC patterns: collections with notes/tags, optional links, batch tagging, SVG plus manifest contact sheets, source hashes, no-space paths.
   - Defer raster export unless Lens explicitly needs it.
   - Validate manifest parseability and source ID/hash traceability.

3. Atelier authoring surface
   - Reuse CKC patterns: side-by-side image/reference and structured sheet/editor, notes/stories/moodboards, versioned field values, diff/merge/revert.
   - Replace CKC character-sheet taxonomy with Handshake Atelier domain model.
   - Validate no field loss, version recoverability, and no governance leakage into product UI.

4. Lens derived artifacts
   - Reuse CKC patterns: source image plus sidecar derivatives, deterministic paths, derivative hiding in normal galleries, explicit sidecar listing.
   - Map PoseKit/OpenPose only if Lens owns pose/control artifacts.
   - Validate source/derivative linkage and deletion/archive behavior.

5. Identity/reference profiles
   - Reuse CKC pattern: deterministic content-hash face/reference crop, landmarks/measurements metadata, downstream bridge payload.
   - Migrate naming away from `IdentityProfile` if Handshake has a stronger domain term.
   - Validate reproducible crop path and profile ownership.

6. Agent operation layer
   - Reuse CKC patterns: in-app/manual command map consistency, automation sessions, leases, logs, state JSON, screenshot capture, background-safe operation, visual verification gates.
   - Implement as Handshake authority, not CKC carryover text.
   - Validate manual-command-dispatch consistency and parallel-agent recoverability.

## Conflicts And Rejections

- SQLite forbidden: CKC still contains SQLite references for legacy fallback, indexes, fixtures, FTS5, snapshots, and history. Handshake must not copy those references into product specs, test plans, architecture, or governance except as a rejected CKC source note.
- Electron not authority: CKC uses Electron IPC, contextBridge, nativeImage, webContents capture, and electron-builder. Handshake may reuse constraints such as non-focus-stealing capture and narrow renderer bridges, but Electron mechanics do not set Handshake architecture.
- Localhost intake not authority: CKC's ComfyUI bridge uses localhost/127.0.0.1 and bearer token support. Handshake must not treat this as the canonical integration path.
- CKC namespace migration required: CKC, CastKit Codex, CKC_GOV, CKC_main, PoseKit, Rig, Character, Workflow, and CastKitCodexBridge are source namespaces only.
- `.GOV` not product output: Handshake `.GOV` can hold planning/governance/reference artifacts, but product exports, UI labels, shipped manuals, and user-facing docs must not expose `.GOV` as product content.

## Recommended Consolidation Notes

- Preserve CKC's proven workflow concepts at the behavior-contract level, not the implementation-label level.
- Promote manifest-backed artifacts, state inspection, and manual-command consistency into Handshake first-class acceptance criteria.
- Make Handshake storage/storage-test rules stricter than CKC: no SQLite fallback, no SQLite-only test evidence, no SQLite fixture dependency.
- Treat CKC's review rows as "implemented but not fully accepted" until GUI/sample-corpus evidence is reviewed.
- Treat CKC's blocked/planned rows as backlog candidates, not requirements.
