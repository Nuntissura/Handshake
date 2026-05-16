---
file_id: ckc-code-greenroom
file_kind: reference_greenroom_inventory
updated_at: 2026-05-16T00:00:00Z
source_project: CastKit-Codex
target_project: Handshake
status: draft
---

<topic id="summary" status="draft" version="v1" summary="CKC code inventory for Handshake-native rebuild." updated_at="2026-05-16T00:00:00Z">

# CKC Code Greenroom Inventory

Scope: read-only review of `D:/Projects/LLM projects/CastKit-Codex/CKC_main`, prioritizing `package.json`, `src/`, `app/backend/`, `comfyui_node/`, and `test/`.

Purpose: preserve CastKit-Codex intent while identifying what to adapt into Handshake Kernel V1. The current CKC implementation is valuable product work, but the exact JS/Electron/SQLite shape must not become Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths.

Top findings:
- Preserve CKC as a character/media/pose/workflow atelier: character sheets, image libraries, template field semantics, PoseKit/OpenPose exports, ComfyUI lineage, docs/moodboards, collections, reference windows, and export packs are product concepts worth carrying forward.
- Adapt the CKC automation surface: `automationManual.js`, `automationCommandMap.js`, `automationControl.js`, renderer automation arms, and stealth/background invariants are directly aligned with model-facing Handshake workflow requirements.
- Reject Electron IPC/main-window shell, process-local automation sessions, SQLite schema-upgrade-at-runtime pattern, library-root file authority, SQLite FTS triggers, and ComfyUI localhost intake. SQLite is not accepted in Handshake in any form. These should become Rust/Tauri commands, PostgreSQL/EventLedger authority, object/file attachment services, and typed integration events.
- CKC already contains a Postgres provider, but it translates SQLite-shaped SQL and still initializes schema at runtime. Handshake Kernel V1 should treat this as migration evidence, not as the target storage architecture.

</topic>

<topic id="research-basis" status="draft" version="v1" summary="Local source evidence used." updated_at="2026-05-16T00:00:00Z">

## Research Basis

Primary CKC sources inspected:
- `package.json`: Electron/Vite/React 19, sqlite3, pg, Three.js, MediaPipe dependencies, packaging targets.
- `app/main.js` and `app/preload.js`: Electron IPC bridge, BrowserWindow ownership, command exposure, dialogs, native image usage, automation capture and synthetic input.
- `app/backend/db.js`: CKC source evidence for SQLite and Postgres adapters, runtime schema creation/upgrades, FTS5 tables/triggers, table inventory, and Postgres SQL translation helper. These are rejected for Handshake except as translation evidence toward PostgreSQL-only design.
- `app/backend/library.js`: central CKC domain service for characters, sheets, images, docs, collections, pose/rigs, identity profiles, ingestion, exports, diagnostics, and filesystem layout.
- `app/backend/sheet.js`, `templateParser.js`, `validation.js`: durable parsing/validation semantics for field IDs, typed values, block schemas, and canonical sheet text.
- `app/backend/imageSourcingAdapter.js`, `workflowSpecRegistry.js`, `intakeServer.js`, `comfyuiClient.js`: image-sourcing workflows, versioned handler registry, localhost intake, and ComfyUI API client.
- `src/posekit/*`: MediaPipe-to-rig mapping, deterministic fallback rigs, OpenPose JSON export, hand/face/head-pose math.
- `src/ui/*`: React views for Library, Character, Pose, Workflow, Export Hub, Intake Sorter, Settings, reference windows, command palette, drawers, moodboard canvas, and sheet/version tools.
- `comfyui_node/castkit_codex_bridge.py`: ComfyUI output node posting generated images and workflow metadata into CKC intake.
- `test/`: broad node test suite covering backend CRUD, schema/migrations, Postgres translator, automation invariants, PoseKit, ComfyUI contract, ingestion idempotency, exports, backups, legacy fixtures, and UI static wiring.

Handshake authority context checked:
- `.GOV/spec/SPEC_CURRENT.md` points to active bundle `v02.185`.
- Kernel V1 direction is PostgreSQL-backed EventLedger only and rejects SQLite in every form, including runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Current spec language treats Rust/Tauri as the coordinator shell, Postgres EventLedger as kernel authority, CRDT/work-box state as pre-promotion state, and DCC/Flight Recorder/UI/provider traces as projections or diagnostics.

</topic>

<topic id="reusable-capability-clusters" status="draft" version="v1" summary="Reusable CKC domain concepts and patterns." updated_at="2026-05-16T00:00:00Z">

## Reusable Capability Clusters

1. Character atelier domain.
   Preserve: `Character`, `FieldValue`, `Template`, `TemplateSpinOff`, `Tag`, `ProtectedField`, `public_id`, safe subset packs, character templates, display names, soft delete, clone, bulk field/tag operations, and character relations.
   Adapt: Rust domain types plus Postgres tables with explicit migrations and EventLedger writes for all mutations.

2. Canonical sheet and template semantics.
   Preserve: stable field IDs, multi-line field spans, canonical sheet header, diff/apply/revert flows, block/block_list JSON values, `score_10`, descriptor, enum, sentinel values, and parser tests.
   Adapt: port parser/validator to Rust with contract fixtures generated from CKC tests.

3. Media library and provenance.
   Preserve: `ImageAsset`, content hashes, storage mode concept, source URL/path/note, tags, suggested tags, palette, dhash similarity, annotations, review status, intake batches, pending images, orphan adoption, missing-image repair.
   Adapt: split metadata authority into Postgres/EventLedger; keep bytes as external attachments with content-addressed paths and no character-name leakage.

4. PoseKit and rig workflow.
   Preserve: body-18, face-70, hand-21 contracts, head-pose quaternion/YXZ order, yaw bins, deterministic fallback rig, OpenPose JSON export, render-to-canvas behavior, identity profile linkage, ComfyUI workflow lineage.
   Adapt: Rust types for stored rig contracts; frontend worker or WASM module for detection; EventLedger events for rig creation, calibration, export, and workflow replay.

5. ComfyUI integration.
   Preserve: bridge payload shape, workflow metadata capture, output image registration, non-fatal CKC intake failure posture, ComfyUI prompt/history/image client shape.
   Adapt: replace localhost-only intake authority with a Tauri command/API endpoint that records typed integration events and stores generated files through Handshake attachment services.

6. Docs, story, moodboard, and links.
   Preserve: NoteDoc, StoryDoc, MoodboardDoc, StoryBoard, LinkIndex, backlinks, global search, moodboard canvas model, corkboard/outliner.
   Adapt: PostgreSQL document tables plus derived search/projection indexes; no SQLite/FTS code or fixtures.

7. Collections and export hub.
   Preserve: collections/playlists, contact sheets, image sets, share packs, web portfolio templates, deterministic exports, no-space path tests, backup manifests.
   Adapt: export jobs as artifact proposals and generated artifacts with EventLedger lineage.

8. Automation and model-facing operation.
   Preserve: automation manual, command map, sessions, heartbeats, leases, command logs, renderer state, capture APIs, no OS-level input injection, non-focus-stealing background behavior.
   Adapt: typed Tauri command catalog, Postgres-backed leases, EventLedger command receipts, stable frontend selectors, visual debugging evidence surfaces.

</topic>

<topic id="runtime-conflicts" status="draft" version="v1" summary="CKC details that conflict with Handshake Kernel V1." updated_at="2026-05-16T00:00:00Z">

## Kernel V1 Conflicts

- SQLite rejection: CKC defaults to SQLite and stores `codex.db` under `libraryRoot/db`. Handshake requires PostgreSQL/EventLedger and rejects SQLite in every form, including runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Runtime DDL: CKC runs `CREATE TABLE IF NOT EXISTS`, `ensureColumn`, indexes, triggers, and provider translation at app startup. Handshake should use explicit migrations and fail closed on schema drift.
- SQL translation layer: CKC Postgres support translates SQLite-style placeholders, datatypes, `LIKE`, `COLLATE NOCASE`, and insert modes. Handshake should use native Postgres queries, typed repositories, and migration-tested SQL.
- Electron shell authority: CKC relies on `BrowserWindow`, `ipcMain`, preload `window.ckc`, dialogs, `nativeImage`, custom protocol behavior, and renderer commands. Handshake should expose equivalent behavior through Tauri commands, Rust state, and frontend API clients.
- Process-local sessions and leases: CKC automation sessions live in an in-memory `AutomationControlPlane`. Handshake requires durable Postgres claims/leases and replayable EventLedger events.
- Filesystem-as-authority: CKC writes sheets, scripts, images, identity profile bundles, exports, backups, and orphan manifests under `libraryRoot`. Handshake should treat files as artifacts/attachments linked to authoritative rows/events.
- SQLite FTS and triggers: CKC creates FTS5 virtual tables and triggers for search. Handshake must not copy them; use PostgreSQL search/projection tables or derived indexes, with rebuild receipts.
- Localhost intake as authority: CKC ComfyUI bridge posts to `127.0.0.1` intake and the backend imports directly. Handshake should ingest through typed integration events and artifact proposal flow.
- Electron-only visual verification: CKC screenshots/captures are tied to Electron APIs. Handshake should rebuild this as Tauri-safe visual diagnostics and DCC evidence.
- Governance split: CKC README says CKC_GOV is SOT and CKC_main product code; Handshake must keep product-code and repo-governance split, and avoid importing CKC governance assumptions as product runtime.

</topic>

<topic id="handshake-native-boundaries" status="draft" version="v1" summary="Suggested Handshake-native module boundaries." updated_at="2026-05-16T00:00:00Z">

## Handshake-Native Boundaries

Suggested Rust/Tauri/Postgres/EventLedger modules:

- `atelier_core`: character, template, sheet, tags, protected fields, relations, public IDs, validation contracts.
- `atelier_media`: image assets, content hashing, thumbnails, palettes, dhash, annotations, pending/accepted/rejected review state, attachment refs.
- `atelier_posekit`: rig schema, pose detection results, OpenPose export, identity profile metadata, yaw/head-pose calibration.
- `atelier_comfy`: ComfyUI bridge payloads, workflow lineage, generated image registration, integration token/session handling.
- `atelier_docs`: notes, stories, moodboards, corkboard cards, backlinks, search projection records.
- `atelier_exports`: field packs, safe subsets, share packs, image sets, contact sheets, web portfolio, backup/export artifact jobs.
- `atelier_automation`: command catalog, model manual, visual capture, renderer state, leases, stable selectors, command receipts.
- `kernel_event_bridge`: every create/update/delete/import/export/promotion emits append-only EventLedger events; projections rebuild from events or authoritative Postgres rows.
- `tauri_api`: frontend commands and events, replacing Electron IPC/preload with typed request/response structs.
- `db_migrations`: native Postgres migrations only for kernel authority; no startup DDL as normal operation.

Migration stance:
- Preserve domain vocabulary and tests wherever possible.
- Adapt JS algorithms into Rust contracts when they encode product semantics.
- Reject runtime-specific Electron/SQLite/localhost implementation details. SQLite is rejected even for runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.

</topic>

<topic id="microtask-outline" status="draft" version="v1" summary="Future CKC kernel WP microtask bucket outline." updated_at="2026-05-16T00:00:00Z">

## Future CKC Kernel WP Microtask Buckets

Character and sheet core:
1. Define `AtelierCharacter` Rust type and ID policy.
2. Define `AtelierTemplate` Rust type and version/hash fields.
3. Port CKC template header parsing fixtures.
4. Port field descriptor type inference.
5. Port enum/sentinel/allow-other validation.
6. Port canonical sheet header generation.
7. Port multi-line field span parser.
8. Port sheet field update/apply behavior.
9. Port sheet diff preview model.
10. Add protected-field mutation guard.
11. Add public character ID sequence migration.
12. Add character relation schema and tests.

Media and provenance:
13. Define `AtelierImageAsset` Postgres schema.
14. Add content hash and no-character-name path invariant.
15. Add attachment reference service for image bytes.
16. Port image import provenance fields.
17. Port duplicate content-hash behavior.
18. Port review status and pending image flow.
19. Port tags and suggested tags fields.
20. Port palette extraction contract.
21. Port dhash similarity contract.
22. Port image annotations JSON contract.
23. Add orphan-image adoption migration path.
24. Add missing-image repair diagnostics.

Pose, rig, and identity:
25. Define rig schema with body-18/face-70/hand-21.
26. Port yaw bin constants.
27. Port YXZ head-pose quaternion contract.
28. Port deterministic fallback rig fixture.
29. Port MediaPipe pose mapping fixtures.
30. Port face-70 mapping fixture.
31. Port hand-21 left/right confidence gating.
32. Add OpenPose JSON export service.
33. Add OpenPose PNG/render artifact job.
34. Define identity profile schema.
35. Add identity crop/manifest attachment refs.
36. Add rig workspace persistence model.

ComfyUI and ingestion:
37. Define ComfyUI bridge payload schema.
38. Add Tauri/API intake command for generated images.
39. Add bridge token/session validation.
40. Port ComfyUI workflow metadata capture.
41. Port prompt/history/image client contract.
42. Add generated-image lineage events.
43. Port workflow spec registry version sorting.
44. Port image-sourcing v00.19 handler contract.
45. Add ingestion batch schema.
46. Add ingestion rejection schema.
47. Add ingestion idempotency tests.
48. Add per-character script artifact import.

Docs, moodboards, and search:
49. Define notes/stories/moodboards schemas.
50. Add story board/corkboard schema.
51. Add moodboard canvas JSON contract.
52. Add link index and backlink resolver.
53. Add global search projection tables.
54. Add search projection rebuild receipts.
55. Add saved search schema.
56. Add tag template and tag rule schemas.

Exports and backups:
57. Define artifact proposal model for exports.
58. Port field-pack export contract.
59. Port safe subset export contract.
60. Port image set export contract.
61. Port share pack export contract.
62. Port web portfolio export contract.
63. Port contact sheet export contract.
64. Port backup manifest and checksum contract.
65. Add restore-version compatibility guard.

Automation and kernel bridge:
66. Define `AtelierActionCatalogV1`.
67. Port automation manual as generated model manual.
68. Port command map into typed Tauri command registry.
69. Add Postgres-backed automation session records.
70. Add claim/lease records for model agents.
71. Add command receipt EventLedger events.
72. Add non-focus-stealing capture command.
73. Add visual debug screenshot artifact records.
74. Add no OS-level input invariant tests.
75. Add stable selector/renderer state contract.
76. Add background stealth guard equivalent for Tauri.
77. Add Flight Recorder mirror from EventLedger IDs.
78. Add trace projection replay smoke test.
79. Add no-SQLite kernel authority tripwire.
80. Add end-to-end character image import replay proof.

</topic>
