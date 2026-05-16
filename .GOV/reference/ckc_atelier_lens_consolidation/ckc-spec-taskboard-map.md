# CKC Spec And Taskboard Preservation Map

file_id: ckc-spec-taskboard-map
file_kind: ckc_preservation_map
updated_at: 2026-05-16
source_spec: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/spec/CastKit_Codex_Spec_v00.075.md
source_taskboard: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/taskboard/TASK_BOARD.md
target_context: Rebuild CastKit Codex capability intent inside Handshake without carrying stale CKC architecture as binding implementation.

## Preservation Stance

- Preserve CKC product intent, user workflows, data-safety expectations, and no-rewrite/no-censorship guarantees.
- Do not preserve CKC-specific repo layout, product strings, Electron-only assumptions, SQLite examples, CKC_GOV runtime write paths, or standalone packaging rules as Handshake requirements. SQLite is not accepted in Handshake in any form.
- Treat CKC taskboard DONE and REVIEW as evidence of operator intent and prior validation shape, not as proof that Handshake already has the behavior.
- Treat WP-0133 as blocked interaction debt that must be rebuilt deliberately, not carried forward as complete PoseKit behavior.
- Treat WP-0136, WP-0112, WP-0114, WP-0116, WP-0117, WP-0118, and WP-0119 as future/planned intent.

## Capability Clusters

### 1. Media Viewer / DAM

Source anchors: spec sections 4.2-7, Appendix A layout/photo goals, WP-0003, WP-0004, WP-0007, WP-0014, WP-0025, WP-0026, WP-0030, WP-0045, WP-0047, WP-0060, WP-0061, WP-0088, WP-0098, WP-0127, WP-0130.

Preserved requirements:
- Image-first portfolio viewer is the default experience.
- Library front page uses a global carousel/media surface beside a character list/grid.
- Character view uses a character-specific carousel/photos surface beside the sheet.
- Full images must display without unintended cropping.
- Thumbnails are horizontal, scrollable, full-image, hideable, and large enough for 4K/TV use.
- Viewer supports slideshow, fullscreen, arrow navigation, visible empty states, and filters that never trap the user.
- Favorite, rating, notes, tags, carousel/frontpage toggles, and provenance are visible in the bottom metadata area.
- Missing media must produce explicit UI states and repair paths.
- Media rows must distinguish normal images, deleted/archive rows, pending rows, and OpenPose sidecars.
- Original media bytes are never modified by annotations, contact sheets, OpenPose exports, or cleanup flows.

### 2. Character Sheets

Source anchors: spec sections 1-3, 4.3.1, 8-10, Appendix A hard rules/sheet editor/exports, WP-0005, WP-0006, WP-0032, WP-0033, WP-0037, WP-0046, WP-0085, WP-0090, WP-0103, WP-0104, WP-0125, WP-0135.

Preserved requirements:
- Character sheet and portfolio viewer remain the product core.
- User text is stored and exported byte-for-byte, with no censorship, rewriting, euphemizing, or in-editor field hiding.
- Adult/explicit fields are first-class.
- Template integrity gates matter: no silent field drops and descriptors stay on the same line as their field ID.
- Public Character ID is system-managed, copyable, protected from import/merge/revert overwrite, and distinct from the internal stable storage ID.
- Enum fields allow free text while offering canonical suggestions.
- Per-field suggestions are field-scoped and never auto-rewrite user input.
- Ingest/merge is selective, diffed, append-only through sheet versions, and non-destructive.
- Sheet version archive/delete must expose impact and avoid dangling references.
- Block/list fields need structured inline editing, recursive validation, and path-style validation errors.
- Templates, cloning, batch character operations, bulk field edits, manual character tags, and soft delete remain first-class productivity flows.

### 3. Inbox / Intake

Source anchors: spec sections 4.1 Image Intake Sorter, 11.2, 11.3, v00.075 intake summary, WP-0055, WP-0056, WP-0094, WP-0100, WP-0123, WP-0124, WP-0125, WP-0126.

Preserved requirements:
- Image ingress routes toward Intake rather than scattered direct imports.
- Intake supports persistent batches, restart/route-switch resume, item selection, image viewer, linked and loose classification, and explicit accepted/pending/rejected lanes.
- Inbox/pending is a safe holding area until assignment to characters, sheet versions, and collections.
- Pending rows carry review status and tags and are surfaced through navigation and automation.
- Clipboard paste and URL import are explicit user actions only.
- Folder-only sorting may move files into pass/reject/pending under the source folder; linked mode imports accepted/pending into the managed library.
- Full batch sheet-field editing remains follow-up debt, not complete behavior.
- Per-batch Inbox folder UI was intentionally deferred to avoid unsafe filesystem reshuffling.

### 4. Collections / Contact Sheets

Source anchors: spec sections 11.10, 11.16, v00.075 contact sheets, WP-0063, WP-0069, WP-0128, WP-0129, WP-0136.

Preserved requirements:
- Collections are cross-character curated image sets with CRUD, slideshow, export, notes, tags, optional character links, and optional sheet-version links.
- Collection batch tag application preserves per-photo tags.
- Contact sheets are generated artifacts with manifests, source image IDs/hashes, tags, layout metadata, and optional character/sheet-version linkage.
- Intake and Collections can create contact sheets.
- Managers/listing surfaces expose generated contact sheets.
- Contact sheet generated paths must be no-space and deterministic enough for provenance.
- Raster PNG/JPG contact sheet export is planned in WP-0136 and must preserve SVG/manifest provenance.

### 5. Sidecars / Versioning

Source anchors: spec sections 4.3, 10, v00.070 compatibility, v00.075 sidecars/recoverable deletion, WP-0033, WP-0100, WP-0105, WP-0106, WP-0127, WP-0130, WP-0132, WP-0134, WP-0135.

Preserved requirements:
- Sheet versions are append-only; revert creates a new version.
- OpenPose PNG/JSON artifacts are sidecars, not normal gallery images.
- Multiple sidecars can link to one source image and rig.
- Normal galleries hide sidecars and archived/deleted rows unless explicitly requested.
- Image archive/restore is recoverable and does not delete image bytes.
- Full reset/orphan recovery intent preserves images and metadata manifests, but CKC installer implementation is not portable to Handshake.
- Product event logs, entity revisions, optimistic concurrency, and hybrid parallel-editing policy must be rebuilt as Handshake-native revision/event surfaces.
- Compatibility checks must pin migration invariants, template field IDs, handler routing, idempotency, backup manifests, and indexes.

### 6. PoseKit / OpenPose

Source anchors: spec v00.071-v00.074, v00.075 PoseKit Workbench, WP-0107, WP-0108, WP-0110, WP-0111, WP-0112, WP-0113, WP-0114, WP-0115, WP-0131, WP-0132, WP-0133.

Preserved requirements:
- PoseKit is a CKC-native workbench for rig data, OpenPose exports, workflow context, source images, and sidecars.
- Pose tab supports blank, single-photo, and collection contexts.
- Workbench state reports context, visible source counts, visible sidecar counts, and active rig state for LLM verification.
- Rig rows are durable; workspace tabs are session-scoped and closing a tab must not delete rig rows, source images, OpenPose exports, workflow history, or identity profiles.
- Pipeline detects body-18, face-70, hand-left-21, and hand-right-21 with canonical OpenPose arrays.
- Hand arrays are 63 floats per side, zero-filled when no confident detection exists.
- Head pose supports yaw, pitch, and roll with quaternion-backed math and UI reset paths.
- Identity export profiles store a deterministic face crop, landmarks, measurements, pose metadata, and bridge payload.
- Future/planned PoseKit intent includes multi-subject scenes, multi-file import, multi-angle export, clear workspace, synchronized zoom, import-existing-OpenPose-JSON, extended shortcuts, stylized detector research, and prompt-response matrix.
- Blocked PoseKit debt: draggable calibration overlay, missing-marker placement, 3D/live split editing, and forked history require a dedicated interaction pass.

### 7. ComfyUI Bridge

Source anchors: WP-0109, WP-0111, WP-0112, WP-0118, spec workflow storage/replay sections.

Preserved requirements:
- ComfyUI integration is localhost-only intake/replay with bearer token support where applicable.
- Workflow history, extracted prompts, replay, stats, and registered outputs are queryable.
- Replay can poll ComfyUI history, fetch view images, and register vanilla SaveImage outputs if the bridge node is absent.
- Workflow tab includes recent run, JSON, extract, and replay UI.
- Pose tab can replay the newest stored workflow.
- Bridge identity payloads should accept identity profile references and image references for IPAdapter/face-reference workflows.
- Future prompt-response matrix stores per-model generation behavior based on operator-owned history, ratings, notes, paired tests, and enough real generation volume.

### 8. Automation / Debug / Manual

Source anchors: spec 4.1 Automation/debugger, v00.065, v00.075 Governance and Verification, WP-0093, WP-0095, WP-0099, WP-0120, WP-0121, WP-0122, WP-0137.

Preserved requirements:
- The app exposes model-facing automation commands, state inspection, session/heartbeat/lease/log surfaces, and file-based captures.
- Automation must be background-safe and must not use OS keyboard, cursor, focus, global shortcut, or attention-stealing APIs.
- UI-affecting work must capture affected routes/panels/windows at normal and constrained sizes and report console/runtime errors.
- Manual, command map, dispatcher, preload, and executable command paths must be consistency-tested.
- In-app manual is a first-class product feature for models with no prior context.
- State should expose config path, library root, DB provider, current route, selected entities, overlays, diagnostics, and command map.
- Handshake rebuild must adapt this to Handshake's product/manual/debug model and keep product runtime out of .GOV.

### 9. Search / Tags / Similarity

Source anchors: spec sections 11.1, 11.6, 11.12-11.14, 12.1, 12.2, 12.7, WP-0016, WP-0054, WP-0059, WP-0065, WP-0066, WP-0067, WP-0083, WP-0084, WP-0089.

Preserved requirements:
- Search spans sheets, notes, stories, moodboard text, image metadata, tags, and provenance.
- Results group by type, show snippets/context, and jump to source.
- Smart folders are rule-based saved searches, not snapshots.
- Tag manager supports rename, merge, counts, pinning, and structured tag-only mutation without touching free text.
- Links/backlinks connect characters, docs, and images without silent rewrite.
- AI-assisted tagging stores suggestions with confidence and requires review/apply before final tags.
- Palette and color search cache dominant colors and provide threshold filtering.
- Exact duplicate and perceptual near-duplicate workflows are safe, explicit, cancellable, and never auto-delete.
- SQLite FTS5 examples are rejected for Handshake. Port search to Handshake-native PostgreSQL/search-index behavior only.

### 10. Exports

Source anchors: Appendix A exports, spec sections 11.10, 11.20, 11.29, 12.5, 12.8, WP-0006, WP-0063, WP-0073, WP-0082, WP-0087, WP-0090, WP-0129, WP-0136.

Preserved requirements:
- Export hub consolidates empty templates, LLM-friendly packs, filled character exports, selected image sets, moodboards, collections, share packs, batch exports, static web portfolio, contact sheets, backups, and high-resolution moodboard/PDF export.
- Exports support field/section selection and reusable dropdown presets.
- Empty template export must preserve canonical template bytes/layout and descriptor-on-same-line rule.
- Export destinations default near the library root/export root unless user chooses otherwise.
- Generated names are Windows-safe, disk-agnostic, and no-space.
- Backups include manifest and checksums, verify before writes, and require explicit restore confirmation.
- Handshake must replace CKC's D-drive refusal and CKC_GOV artifact paths with Handshake's external artifact root and path-safety rules.

## CKC Taskboard Status Preservation

### Done

The taskboard marks WP-0001 through WP-0111, WP-0113, WP-0115, and WP-0120 through WP-0121 as DONE. These done items cover the base rebuild, spec, portfolio viewer, ratings, notes/stories/moodboards, exports, thumbnails, build targets, workflow gates, IDs, diagnostics, repair, backup, layouts, sheet merge/versioning, security, local models, style, tags/search, moodboard tools, links/backlinks, inbox, clipboard, batch metadata, duplicate cleanup, tag manager, reference window, annotations, story corkboard, export hub, URL import, smart folders, color tools, near-duplicates, collections, relationship map, command palette, backup/restore, moodboard power tools, full-text search, AI tagging, templates/cloning, mac build, web export, performance, visual similarity, batch character operations, PostgreSQL storage, automation/debugger/manual, image intake sorter, no-space artifacts, image-sourcing ingestion, sheet validator, block editor, reset modes, compatibility checks, PoseKit schema/pipeline/ComfyUI/head pose/identity profiles/hand keypoints/workspace tabs, and build-rule governance.

### In Review

The taskboard marks WP-0122 through WP-0132, WP-0134, WP-0135, and WP-0137 as REVIEW. These are not final DONE but provide useful implemented intent and evidence for LLM visibility, navigation naming, persistent Intake, Intake linking, Inbox lifecycle, filesystem hardening, Collections v2, contact sheets, MediaPane sidecars, PoseKit workbench routing, OpenPose artifact contract, CRDT/event/revision policy, recoverable deletion, and manual/dispatcher/PostgreSQL regression hardening.

### Blocked

WP-0133 is BLOCKED. Implemented pieces include slider values/resets, marker list color/hand rows, OpenPose/history sidecar visibility, and History tab. Remaining blockers are draggable calibration overlay, missing-marker placement flow, 3D/live split editing, and forked history. This must become a dedicated Handshake interaction packet before claiming PoseKit complete.

### Future / Planned

WP-0112, WP-0114, WP-0116, WP-0117, WP-0118, WP-0119, and WP-0136 are planned/concept/research. They preserve intent for multi-subject scenes, Pose tab polish, extended shortcuts, stylized detector research, prompt-response matrix, PostgreSQL-only testing, explicit SQLite rejection, and raster contact sheet export.

### Useful Test Evidence

Useful evidence to preserve as verification shape:
- WP-0099: automation manual consistency tests, synthetic input invariant tests, background stealth invariant tests.
- WP-0103 and WP-0104: sheet parser/validator/block editor recursive validation tests.
- WP-0105: reset/orphan manifest tests plus live before/after captures.
- WP-0106: PostgreSQL-only parity cases, migration invariants, handler routing, template field ID immutability, ingestion idempotency, backup traceability, spec consistency, index invariants.
- WP-0108 through WP-0115: PoseKit core, UI static, math, hand taxonomy/export, automation manual consistency, tsc/build, live captures.
- WP-0122 through WP-0137: normal/constrained GUI captures, PostgreSQL-compatible regression tests, manual-command-dispatcher consistency, full npm test evidence.

## Conflicts With Handshake Reset Rules

- SQLite conflict: CKC spec and older roadmap examples use SQLite tables, SQLite FTS5, triggers, better-sqlite3, and legacy db/codex.db layout. Handshake must reject SQLite in every form, including runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. New rebuild work targets PostgreSQL only.
- Standalone app conflict: CKC assumes an Electron standalone app, portable exe, NSIS installer modes, app.getPath userData, product-specific package scripts, CKC_main, and CKC_GOV/targets. Handshake should rebuild capabilities as Handshake product modules and external artifacts, not as a resurrected CKC standalone packaging stack unless explicitly chosen.
- Product string conflict: CastKit Codex, CKC, CKC_main, CKC_GOV, CastKit-Codex-Library, ckc-config.json, ckc IPC names, and CastKitCodexBridge are CKC identifiers. Handshake should rename product-facing strings, config keys, API namespaces, bridge names, manual labels, and generated artifact labels to Handshake/Atelier Lens equivalents.
- Governance boundary conflict: CKC product writes automation captures and build outputs under CKC_GOV/targets in dev mode. Handshake product runtime must not read or write .GOV; generated files belong under the configured external artifact/runtime roots.
- Path portability conflict: CKC evidence and examples include absolute D:/ paths and old OpenRepose sample paths. Handshake rebuild must store repo-relative, runtime-root-relative, or operator-configured paths and keep absolute paths as evidence-only historical anchors.
- Reset-mode conflict: CKC installer reset modes are tied to NSIS and CKC library layout. Handshake reset behavior must be product-native, reversible where possible, scoped by Handshake storage boundaries, and aligned with no destructive work without explicit operator approval.
- Stale architecture conflict: CKC stored some docs/gov mirrors in product docs and later removed them. Handshake should preserve the lesson: one authority surface, machine-readable governance contracts, generated projections, and no parallel manual sidecars.
- Search architecture conflict: CKC FTS5 examples cannot be copied into Handshake. Preserve search behavior, not the SQLite/FTS5 implementation.
- Automation focus conflict: CKC Electron hidden-window automation should be adapted to Handshake's runtime. Preserve non-focus-stealing, file capture, stable selectors, and command/manual consistency.
- Build artifact conflict: CKC put artifacts under CKC_GOV/targets; Handshake requires external Handshake_Artifacts or configured runtime artifact roots.

## Preserved Goals

- Rebuild CKC as a character portfolio, DAM, sheet, intake, collection, PoseKit, ComfyUI, automation, search, and export system inside Handshake.
- Preserve user-authored content byte-for-byte and keep adult/explicit fields first-class.
- Keep image-first workflows central: carousel, photos, fullscreen, metadata, tags, ratings, annotations, collections, and contact sheets.
- Make ingestion safe: no automatic clipboard/URL capture, no silent deletes, clear pending lifecycle, resumable intake batches, and explicit linked/loose modes.
- Maintain durable identity and versioning: internal IDs, public IDs, sheet versions, sidecars, provenance, event logs, revisions, manifests, and recovery reports.
- Support mass-parallel model/operator work through automation state, leases, manuals, captures, debug surfaces, and command consistency tests.
- Keep PoseKit and OpenPose production workflows integrated with image collections, workflow history, identity profiles, and ComfyUI replay.
- Make search, tags, similarity, palette, duplicate detection, and batch operations power-user-grade.
- Keep exports useful for humans, LLMs, sharing, backups, web portfolios, contact sheets, and production handoffs.
- Move from CKC-specific standalone assumptions to Handshake-native product modules, storage boundaries, and artifact hygiene.

## Proposed Massive WP Mapping

### WP-HS-CKC-001: Core Library, Character, Sheet, Media, And Governance-Compatible Storage

Goal: Establish the Handshake-native data model and UI foundation for the image-first character portfolio, sheet editor, DAM viewer, versioning, tags, ratings, metadata, docs/moodboards, and recoverable media behavior.

Dependencies: Handshake product storage boundary, product UI shell, external artifact/runtime root, current Handshake naming decision for the rebuilt product lane.

Microtask buckets:
1. MT-001 Source preservation registry: ingest CKC source anchors into machine-readable requirements without making CKC architecture binding.
2. MT-002 Naming migration matrix: map CKC product strings and namespaces to Handshake/Atelier Lens names.
3. MT-003 Storage contract: define Handshake-native entities for characters, images, sheets, docs, moodboards, tags, ratings, provenance, versions, and revisions.
4. MT-004 PostgreSQL-only migrations: reject SQLite examples and use PostgreSQL storage-interface design.
5. MT-005 Library root/runtime roots: define product data root, export root, cache root, and external artifact root without .GOV writes.
6. MT-006 Internal/public character ID model: implement stable internal IDs and protected public IDs.
7. MT-007 Canonical sheet template importer: preserve descriptor-on-same-line, field IDs, block/list schemas, and no field drops.
8. MT-008 Sheet parser and type validator: handle strings, enums, other descriptors, scores, blocks, and block lists.
9. MT-009 Structured sheet editor: free text enums, per-field suggestions, protected fields, copyable public ID.
10. MT-010 Sheet ingest/merge/version UI: paste/import, diff, selective apply, append-only versioning, non-destructive revert.
11. MT-011 Sheet archive/delete impact checks: prevent dangling references and preserve recoverability.
12. MT-012 Media asset model: storage modes, relative paths, hashes, dimensions, source provenance, review status, sidecar role.
13. MT-013 Media viewer shell: Library global carousel and Character carousel/photos layouts.
14. MT-014 Thumbnail strip: horizontal scroll, full-image thumbnails, hide/show, large-screen sizing.
15. MT-015 Media metadata bar: rating, favorite, notes, tags, carousel/frontpage toggles, provenance, AI suggestions entry.
16. MT-016 Viewer navigation: slideshow, fullscreen, arrows, filter empty states, no trapped filters.
17. MT-017 Missing media diagnostics: explicit missing states, library/root visibility, repair entrypoints.
18. MT-018 Repair-by-hash workflow: dry-run, copy restore, thumbnail regeneration, JSON report outside .GOV.
19. MT-019 Docs mode foundation: notes, stories, moodboards, autosave, drawer search/tags, persisted UI state.
20. MT-020 Moodboard baseline: image layers, ink, text/sticky notes, transform, undo/redo, zoom/pan, grid/snap.
21. MT-021 Moodboard power layers: shapes, fills, connectors, masks, selection tools, folders, styling, export hooks.
22. MT-022 Tag manager and saved searches baseline: structured tags only, counts, rename/merge, pinned filters.
23. MT-023 Character templates/cloning/batch operations: templates, clone, batch create, bulk edit, soft delete/trash.
24. MT-024 Core visual/debug verification: stable selectors, screenshots at normal/constrained sizes, no overlap/readability checks.

### WP-HS-CKC-002: Intake, Collections, Sidecars, PoseKit, OpenPose, And ComfyUI Production Workflows

Goal: Rebuild the production workflow layer that moves images from intake to curated collections, contact sheets, PoseKit rigs, OpenPose sidecars, identity profiles, workflow replay, and ComfyUI bridge outputs.

Dependencies: WP-HS-CKC-001 media, character, storage, sheet versioning, tags, runtime roots, and automation selectors.

Microtask buckets:
25. MT-025 Intake navigation and import gate: route direct image ingress toward Intake and expose explicit entrypoints.
26. MT-026 Persistent intake batches: durable batches, item selection, resume after restart/route switch.
27. MT-027 Intake viewer and compare controls: source image review, checkmarked selection, compare/contact-sheet controls.
28. MT-028 Linked/loose classification: accepted/pending/rejected lanes, loose folder-only mode, linked character mode.
29. MT-029 Character/sheet/collection linking: optional character creation, sheet version links, collection links, version-agnostic defaults.
30. MT-030 Pending Inbox lifecycle: pending image listing, review status, tags, Inbox navigation, filesystem policy.
31. MT-031 Clipboard paste import: explicit action only, Character and Library/Inbox targets.
32. MT-032 URL import: explicit URL capture, provenance metadata, no automatic remote fetching.
33. MT-033 Image sourcing adapter: task-state ingestion, handler registry, lanes, sync events, rejection audit.
34. MT-034 Per-character scripts: copied script registry, hash dedup, identity-decoupled names.
35. MT-035 Intake filesystem hardening: source health checks, pending health checks, untracked original detection.
36. MT-036 Collections v2 model: notes, tags, item order, optional character/sheet-version links.
37. MT-037 Collections UI: CRUD, slideshow, add-to-collection, Library lane navigation, batch tag apply.
38. MT-038 Contact sheet SVG/manifest: deterministic no-space paths, source IDs/hashes, tags/layout metadata.
39. MT-039 Contact sheet actions: Intake compare, Collections action, manager listing, provenance view.
40. MT-040 Raster contact sheet export: PNG/JPG generation while preserving SVG/manifest provenance.
41. MT-041 Sidecar role model: OpenPose PNG/JSON sidecar links, hidden normal-gallery behavior, archive controls.
42. MT-042 PoseKit schema: rig, prompt, story beat, rig tags, source image links, workflow references.
43. MT-043 PoseKit workbench routing: blank/single/collection contexts, route-state persistence, source/OpenPose strips.
44. MT-044 PoseKit detection pipeline: body-18, face-70, hand-21 arrays, deterministic fallback, local model assets.
45. MT-045 PoseKit rendering/calibration: 2D canvas, 3D viewport, marker controls, yaw/pitch/roll quaternion head pose.
46. MT-046 PoseKit workspace tabs: session-scoped tabs, save-before-switch, close-without-delete, automation state.
47. MT-047 PoseKit blocked interaction pass: draggable calibration overlay, missing-marker placement, 3D/live split editing, forked history.
48. MT-048 Identity profiles: deterministic face crop, landmarks, measurements, pose metadata, bridge payload.
49. MT-049 ComfyUI bridge/workflow replay: localhost intake, token, workflow history, extract prompt, replay, register outputs, identity injection.

### WP-HS-CKC-003: Automation, Manual, Search, Similarity, Export, Backup, Parallel Editing, And Hardening

Goal: Add the model-facing operation layer, global search/tag/similarity intelligence, export/backup surfaces, parallel-editing controls, compatibility tests, and verification gates required for Handshake-scale rebuild quality.

Dependencies: WP-HS-CKC-001 and WP-HS-CKC-002 storage, UI routes, entities, sidecars, collections, PoseKit, and artifact roots.

Microtask buckets:
50. MT-050 Automation command registry: model-callable renderer/backend command map with typed inputs/outputs.
51. MT-051 Automation state inspection: expose route, selection, overlays, diagnostics, storage provider, command map, workbench state.
52. MT-052 Sessions/leases/logs: multi-agent sessions, heartbeats, leases for conflicting work, event log inspection.
53. MT-053 Background-safe capture: file captures, JSON sidecars, no focus/foreground/OS input behavior.
54. MT-054 Synthetic input invariants: window-scoped commands only, no robotjs/nut/AutoHotkey/SendInput equivalents.
55. MT-055 In-app model manual: purpose, workflows, commands, inputs/outputs, navigation, safety, failures, recovery.
56. MT-056 Manual/dispatcher consistency tests: fail when documented commands lack executable dispatch/preload/backend paths.
57. MT-057 GUI visual debug suite: route captures, constrained viewports, console/runtime errors, readable controls, no overlap.
58. MT-058 Global search backend: sheets, docs, moodboards, images, tags, provenance, snippets, jump targets.
59. MT-059 Search UI: grouped results, scope toggle, hotkey/palette entry, pagination, phrase/boolean support as backend allows.
60. MT-060 Smart folders: rule-based saved searches with include/exclude tags, tag mode, gallery filters, live counts.
61. MT-061 Links/backlinks: characters, docs, images, stable link primitives, no silent rewrite.
62. MT-062 AI-assisted tagging: OpenAI-compatible/local vision endpoint, suggested tags/confidence, review/apply, cancellable bulk jobs.
63. MT-063 Palette/color search: cached palettes, chips, threshold filter, computing state.
64. MT-064 Exact duplicate workflow: hash groups, context, safe redundant-row cleanup, no external file deletion.
65. MT-065 Perceptual similarity: dHash or Handshake-native equivalent, threshold, similar modal, safe jump-to.
66. MT-066 Export hub foundation: empty templates, LLM packs, filled sheets, selected image sets, share packs.
67. MT-067 Advanced exports: moodboard hi-res/selection/PDF, static web portfolio, batch character exports, collection exports.
68. MT-068 Backup/restore: manifest, checksums, pg dump/filesystem assets, integrity validation, explicit restore token.
69. MT-069 Parallel editing/event policy: entity revisions, optimistic updates, product events, safe merge shapes.
70. MT-070 Compatibility suite: additive migration lints, template ID immutability, handler pins, idempotency, backup version traceability, index pins.
71. MT-071 PostgreSQL-only proof: isolated Postgres harness, concurrency tests, and no-SQLite tripwires for runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
72. MT-072 Final hardening gate: full test suite, type checks, visual evidence, manual consistency, artifact hygiene, source requirement traceability.

## Dependency Summary

- WP-HS-CKC-001 must land first because it owns identity, storage, sheet, media, docs, metadata, and foundational UI surfaces.
- WP-HS-CKC-002 depends on WP-HS-CKC-001 because Intake, Collections, PoseKit, sidecars, identity profiles, and ComfyUI all require durable image/character/sheet entities.
- WP-HS-CKC-003 depends on both earlier WPs because automation, search, exports, backups, parallel editing, and compatibility tests need real routes, entities, commands, and artifact locations to verify.
- WP-0133-equivalent PoseKit interaction repair should be scheduled inside WP-HS-CKC-002 or split as a sub-WP if risk is too high.
- WP-0117 stylized landmark detector research can run in parallel with WP-HS-CKC-002 after PoseKit corpus and evaluation harness are defined.

## Gaps And Risks

- Risk: copying CKC's SQLite examples into Handshake will violate Kernel reset direction. Mitigation: express search/storage requirements through Handshake storage interfaces and PostgreSQL-only tests; add no-SQLite tripwires.
- Risk: CKC_GOV artifact writes would violate Handshake product/governance boundaries. Mitigation: all product outputs go to configured product runtime/artifact roots outside .GOV.
- Risk: CKC standalone labels leak into Handshake user-facing surfaces. Mitigation: require naming migration matrix before UI/API/manual work.
- Risk: PoseKit scope is large and could swallow the rebuild. Mitigation: keep PoseKit in WP-HS-CKC-002 with explicit blocked-interaction microtask and independent verification gates.
- Risk: preserving all CKC goals could produce a monolith. Mitigation: use three massive WPs with microtask buckets and traceability rather than one unstructured rebuild.
- Risk: DONE/REVIEW from CKC may be mistaken for Handshake proof. Mitigation: mark them as source evidence only; require fresh Handshake tests and visual/debug evidence.
- Risk: user-authored text could be normalized during import/export/search/tag operations. Mitigation: add byte-preservation tests and restrict tag operations to structured tag fields.
- Risk: automation manual drift returns. Mitigation: manual-command-dispatcher consistency tests are mandatory in WP-HS-CKC-003.

## Validation Needs

- Machine-readable JSON must keep arrays for capability_clusters, source_requirements, taskboard_status, conflicts, preserved_goals, and proposed_wp_mapping.
- Markdown and JSON must stay ASCII.
- Source traceability must cite CKC spec sections and taskboard WPs.
- Handshake conflicts must be explicit before implementation planning starts.
- Every proposed WP must link back to capability clusters and include 60+ total microtask buckets.
