---
file_id: ckc-code-agent-draft
file_kind: greenroom-extraction
updated_at: 2026-05-16
scope: CKC_main codebase only
---

# CKC Code Greenroom Draft: Atelier/Lens Consolidation

CKC is evidence only. This extraction treats CKC runtime choices as source evidence, not Handshake authority. Handshake must remain PostgreSQL-only; CKC SQLite compatibility, Electron, and localhost intake are rejected as Handshake runtime authority.

## 1. Top CKC Architecture Files

- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\package.json`: Electron/Vite/React app, Node test runner, release scripts, dependencies on `pg`, `sqlite3`, MediaPipe, Three, React Three Fiber, and Electron Builder. Build artifacts route to sibling `CKC_GOV/targets`.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\README.md`: Declares repo split: `CKC_main` product code, `CKC_GOV` governance/spec/artifacts. Development and packaging commands are here.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\app\main.js`: Electron main process, config defaults, PostgreSQL default config, portable library roots, CKC custom protocol, IPC handlers, automation capture/jobs, ComfyUI intake server startup, reference window, backup/restore jobs.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\app\preload.js`: Exposes the full `window.ckc` IPC surface: characters, images, collections, contact sheets, templates, PoseKit, identity profiles, ComfyUI workflow, docs, search, saved searches, exports, backup/restore, intake, diagnostics, and automation.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\app\backend\db.js`: Database adapter and schema. PostgreSQL provider exists, but SQLite is still the fallback/compat path. Defines core tables: Character, FieldValue, ImageAsset, Rig, IdentityProfile, Prompt, StoryBeat, SavedSearch, NoteDoc, StoryDoc, MoodboardDoc, Collection, IntakeBatch, ContactSheet, EntityRevision, ProductEvent, etc.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\app\backend\library.js`: Main product service layer. Owns character/sheet/image import, versions, exports, collections/contact sheets, intake, repair, similarity, PoseKit, identity, ComfyUI registration/replay, docs/search, audit/product events.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\src\ui\App.tsx`: Primary router and app state for library, character, exports, intake, pose, workflow, settings/managers, command palette, global search, renderer automation state.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\src\vite-env.d.ts`: Frontend type contract for CKC entities and IPC return shapes.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\app\backend\automationManual.js` and `automationCommandMap.js`: Machine-facing manual and wired command registry. Separates implemented commands from roadmap/preload-only surfaces.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\src\posekit\core.mjs`: PoseKit core: body-18, hand-21, face-70, yaw/head-pose math, calibration, OpenPose JSON rendering/export.
- `D:\Projects\LLM projects\CastKit-Codex\CKC_main\comfyui_node\castkit_codex_bridge.py`: Custom ComfyUI output node that saves images, packages workflow/identity metadata, and POSTs to CKC intake.

## 2. Capability Clusters

- Media viewer / DAM: `src\ui\components\MediaPane.tsx`, `src\ui\views\LibraryView.tsx`, `app\backend\library.js`, `app\backend\dhash.js`, `app\backend\palette.js`, tests `backend_duplicates.test.js`, `backend_similarity_search.test.js`, `backend_image_annotations.test.js`, `backend_ratings_filters.test.js`. Features include carousel/thumb viewer, fullscreen/slideshow, ratings/favorites/notes/tags/source note, multi-select, archive/delete impact, pins/annotations, palettes/color filter, dHash exact/near duplicate search, AI tag suggestions.
- Character sheets/templates: `app\templates\CHARACTER_SHEET__v2.00.txt`, `app\templates\character_templates\*.json`, `app\backend\templateParser.js`, `sheet.js`, `validation.js`, `src\ui\components\SheetEditor.tsx`, `SheetIngestMergeTools.tsx`, `SheetVersionTools.tsx`, tests `backend_sheet.test.js`, `backend_sheet_ingest_versions.test.js`, `backend_character_templates.test.js`, `template_parser_unions.test.js`, `template_field_id_immutability.test.js`. Features include structured field typing, block/block-list fields, protected fields, ingest preview/apply, diff/revert, template-derived characters, clone sheet-only or with images.
- Intake / inbox: `src\ui\views\IntakeSorterView.tsx`, `app\backend\intakeServer.js`, `app\backend\imageSourcingAdapter.js`, `imageSourcingHandlers\v00_19.js`, `app\backend\library.js`, tests `intake_server.test.js`, `backend_ingestion_batches.test.js`, `ingestion_idempotency.test.js`, `ingestion_handler_routing.test.js`, `backend_web_import_url_capture.test.js`. Features include folder scan, atomic/loose batch modes, selected/rejected/pending classification, assignment to character/sheet version/collection, notes/tags defaults, source URL/clipboard/web import, task ingestion audit/rejections.
- Collections/contact sheets: `src\ui\views\LibraryView.tsx`, `src\ui\views\IntakeSorterView.tsx`, `src\ui\views\SettingsView.tsx`, `app\backend\library.js`, tests `backend_collections.test.js`. Features include cross-character image collections, collection tags, collection image listing, contact sheet creation/listing, intake contact sheets.
- Sidecars/versioning/recovery: `app\backend\library.js`, `backup.js`, `resetModes.js`, `src\ui\components\SheetVersionTools.tsx`, tests `backend_sheet_ingest_versions.test.js`, `backup_version_traceability.test.js`, `backend_backup_restore.test.js`, `adopt_orphan_images.test.js`, `backend_repair_missing_images.test.js`, `backend_diagnostics.test.js`. Features include SheetVersion, EntityRevision, ProductEvent, audit log, orphan manifest/adoption, missing image repair by hash, backup manifest/SHA256SUMS, version-gated restore.
- PoseKit/OpenPose/identity: `src\posekit\core.mjs`, `poseDetectionClient.ts`, `poseDetection.worker.ts`, `public\posekit_models\*.task`, `src\ui\views\PoseView.tsx`, `src\ui\components\Pose3DViewport.tsx`, `app\backend\library.js`, tests `posekit_core.test.js`, `hand_openpose_export.test.js`, `identity_profile_crud.test.js`, `identity_profile_replay_injection.test.js`, `backend_posekit_workflow.test.js`. Features include local MediaPipe body/face/hand detection, deterministic fallback rig, head pose quaternion/YXZ order, multi-rig workspaces, OpenPose PNG sidecars, identity profile crop bundle and replay injection.
- ComfyUI pipeline/receipts: `app\backend\comfyuiClient.js`, `comfyui_node\castkit_codex_bridge.py`, `app\backend\intakeServer.js`, `src\ui\views\WorkflowView.tsx`, tests `backend_posekit_workflow.test.js`, `comfyui_node_contract.test.js`. Features include prompt submission, history polling, output download, workflow JSON and prompt extraction, lineage metadata, dedupe, replay, identity/rig/openpose references.
- Automation/manual/debug: `app\backend\automationManual.js`, `automationCommandMap.js`, `automationControl.js`, `automationStealth.js`, `src\ui\App.tsx`, tests `automation_manual_consistency.test.js`, `automation_input_injection_invariants.test.js`, `automation_background_stealth_invariants.test.js`, `backend_diagnostics.test.js`. Features include in-app model manual, command registry, sessions/leases/heartbeats/logs, renderer state, DOM/synthetic input, screenshots/capture, background stealth.
- Search/tags/similarity: `app\backend\library.js`, `src\ui\components\GlobalSearchModal.tsx`, `src\ui\views\LibraryView.tsx`, `MediaPane.tsx`, tests `backend_global_search.test.js`, `backend_saved_searches.test.js`, `backend_tag_manager.test.js`, `backend_similarity_search.test.js`, `backend_ai_tagging_suggestions.test.js`. Features include multi-surface search across sheets/docs/moodboards/images, saved searches with scope/tag/gallery filters, tag stats/merge/rename/templates/rules, dHash similarity.
- Exports/backups/share packs: `src\ui\views\ExportHubView.tsx`, `src\ui\components\BatchExportDialog.tsx`, `app\backend\library.js`, `backup.js`, `app\templates\web-portfolio\*`, tests `backend_exports.test.js`, `backend_export_hub.test.js`, `backend_backup_restore.test.js`. Features include empty template, field pack, image set, share pack, web portfolio, moodboard PNG/PDF, backup/restore jobs.

## 3. Entities/Data Models

- Character: `Character` table and `CKCCharacter`/`CKCCharacterListItem`; fields include internal ID, public ID, display name, template metadata/hash, search blobs, icon image/focus, system/deleted flags.
- FieldValue/Template/TemplateSpinOff/ProtectedField: structured sheet values, canonical template AST/raw text, spinoff field packs, global/character protected fields.
- ImageAsset: original/thumb asset records with hash, path, dimensions, favorite, rating, notes, tags, suggested tags, storage mode, source path/url/note, palette, dHash, review status, media role, source image, sheet version, intake refs, deletion/revision metadata.
- Rig/RigTag/Openpose sidecar convention: pose JSON, calibration JSON, portrait image, status, session workspace state; OpenPose exports become `ImageAsset` rows with `mediaRole=openpose`.
- IdentityProfile: profile ID, character/source image/source rig, cropped face image, crop/manifest paths, landmarks, measurements, pose metadata, bridge payload, soft delete timestamps.
- Prompt/StoryBeat/WorkflowHistory: prompts and story beats in DB; workflow history inferred from ImageAsset workflow metadata registered by ComfyUI.
- NoteDoc/StoryDoc/MoodboardDoc/StoryBoard/LinkIndex: DB-first docs, structured moodboard JSON, corkboard/story board, backlink index via `[[token]]`.
- SavedSearch/Tag/CharacterTag/TagTemplate/TagRule/TagStats: saved search filters, manual/derived tags, reusable tag templates, derived tag rules, tag manager stats.
- Collection/CollectionItem/ContactSheet: named image collections and generated contact sheet records, with optional character/sheet version linkage added by migrations.
- IntakeBatch/IntakeBatchItem/IngestionBatch/IngestionRejection: UI intake batches, selected/status metadata, external task ingestion audit, task rejection records.
- AuditLog/EntityRevision/ProductEvent/CkcMeta/CkcDbMigration: audit trail, optimistic-ish revision surfaces, product event log, metadata, migration cursor.

## 4. Workflows/User Flows

- Library navigation: `App.tsx` routes to Library, Character, Export, Intake, Pose, Workflow, Settings/Managers; command palette/global search can open characters/docs/tags.
- Character management: Library creates characters from templates, bulk edits fields/tags, soft-deletes/restores/purges; Character view edits sheet, tags, icon focus, docs, relations, images.
- Sheet ingest/version workflow: paste/load text, preview mapped field changes, select changes, apply with validation/protected fields, create SheetVersion, diff versions, revert selected fields.
- Media workflow: import images, URL/clipboard/inbox intake, edit metadata, use hotkeys for ratings, run AI tag suggestions, color/palette/similarity filters, pin annotations, inspect backlinks/OpenPose sidecars.
- Intake sorter: choose source folder, scan into batch, inspect/compare, select items, classify accepted/pending/rejected, assign character/sheet/collection, create contact sheet.
- Collection workflow: create/rename/update/delete collections, add/remove images, apply tags to collection images, open PoseKit on collection context, create contact sheets.
- Pose workflow: choose character/collection/single image, detect pose, edit calibration/markers/reframe/head pose, create/update rig, manage rig workspace tabs, export OpenPose PNG sidecar.
- Identity workflow: create/list/update/delete identity profile from source image/rig; create cropped face bundle and bridge metadata; workflow replay can inject identity inputs.
- ComfyUI workflow: register bridge output via localhost intake or replay workflow via `/prompt`, poll `/history`, fetch `/view`, register output with metadata and prompt extraction.
- Export/backup workflow: choose export/backup roots, run backup/restore jobs with polling, export image sets/share packs/web portfolios/moodboards/templates/field packs.
- Automation workflow: model creates session, acquires lease, navigates via command map, reads renderer state, runs backend commands, records logs/captures, avoids foreground behavior in background mode.

## 5. Tests/Fixtures That Prove Behavior

- Automation/manual: `automation_manual_consistency.test.js`, `automation_input_injection_invariants.test.js`, `automation_background_stealth_invariants.test.js`.
- PostgreSQL/SQLite compatibility evidence: `backend_postgres_provider.test.js`, `migration_invariants.test.js`, `legacy_fixture_compatibility.test.js`, legacy SQLite fixtures under `test\fixtures\legacy\wp-0091` through `wp-0104`.
- Character/sheets/templates: `backend_sheet.test.js`, `backend_sheet_ingest_versions.test.js`, `backend_character_templates.test.js`, `template_parser_unions.test.js`, `validation_field_types.test.js`, `template_field_id_immutability.test.js`, fixture `test\fixtures\template_v2_00_field_ids.json`.
- Media/DAM: `backend_duplicates.test.js`, `backend_similarity_search.test.js`, `backend_image_annotations.test.js`, `backend_ratings_filters.test.js`, `backend_image_meta_batch.test.js`, `backend_ai_tagging_suggestions.test.js`, `backend_repair_missing_images.test.js`, `adopt_orphan_images.test.js`.
- Intake/ingestion: `intake_server.test.js`, `backend_ingestion_batches.test.js`, `ingestion_idempotency.test.js`, `ingestion_handler_routing.test.js`, `spec_canon_consistency.test.js`.
- Collections/contact sheets: `backend_collections.test.js`.
- PoseKit/identity/ComfyUI: `posekit_core.test.js`, `hand_openpose_export.test.js`, `backend_posekit_crud.test.js`, `backend_posekit_schema.test.js`, `backend_posekit_workflow.test.js`, `identity_profile_crud.test.js`, `identity_profile_replay_injection.test.js`, `comfyui_node_contract.test.js`.
- Search/docs/export/backup: `backend_global_search.test.js`, `backend_saved_searches.test.js`, `backend_docs.test.js`, `backend_links_backlinks.test.js`, `backend_story_board.test.js`, `backend_exports.test.js`, `backend_export_hub.test.js`, `backend_backup_restore.test.js`, `backup_version_traceability.test.js`.

## 6. Convenience/Evolved Features To Preserve

- Model/operator automation manual with executable command registry and tests proving no aspirational command drift.
- Session/lease/log automation control plane for parallel model work.
- Background stealth contract for non-intrusive automation.
- Renderer state and PoseKit state exposed for no-context model debugging.
- Saved searches with scope flags, include/exclude tag filters, any/all tag mode, gallery filters.
- Relationship graph and character relations.
- Character template save/create-N/clone sheet-only-or-with-images.
- Sheet protected fields, selective ingest/revert, version archive/delete, field suggestions from existing values.
- Media annotations/pins, palette extraction/color filtering, dHash similarity, exact duplicate groups.
- Cross-character collections and contact sheet generation.
- Intake batches with selected item classification and contact sheet creation.
- Orphan manifest adoption, missing image repair by hash, diagnostics, backup version traceability.
- OpenPose sidecar assets as first-class ImageAsset rows.
- Identity profile bundles with replay injection metadata.
- Workflow prompts/story beats plus workflow replay/extraction history.
- Web portfolio/share pack/moodboard exports.

## 7. Handshake Conflicts/Rejections

- Reject SQLite entirely for Handshake runtime/tests/fixtures/mocks/examples/fallbacks/cache/import/export/product paths. CKC uses `sqlite3`, defaults `normalizeProvider` to SQLite when provider is not PostgreSQL-like, has SQLite schema paths, and ships legacy SQLite fixtures.
- Reject Electron as Handshake runtime authority. CKC’s IPC/main/preload/router are useful capability evidence, not architecture authority for Handshake.
- Reject localhost ComfyUI intake as Handshake authority. The local bridge and tokenized localhost server are evidence for pipeline receipts/metadata, not the production ingestion contract.
- Reject CKC file-library roots as canonical persistence. Handshake should make PostgreSQL canonical and treat external media storage paths as references/objects governed by DB records.
- Reject CKC compatibility translation adapter as a Handshake pattern. CKC translates SQLite-flavored SQL to Postgres; Handshake should use native PostgreSQL migrations/queries only.
- Reject CKC_GOV-specific governance workflow as direct product behavior. Preserve useful product capabilities, not CKC work-packet mechanics.

## 8. Recommended Overlap/Evolved-Feature Register Rows

| id | feature | CKC evidence | Handshake action |
|---|---|---|---|
| CKC-EVOLVED-001 | Model-facing automation manual and command registry | `automationManual.js`, `automationCommandMap.js`, automation tests | Preserve as Handshake agent manual/API command catalog with tests against implemented routes/services. |
| CKC-EVOLVED-002 | Media DAM metadata and viewer affordances | `MediaPane.tsx`, `ImageAsset`, media tests | Preserve ratings, favorites, notes, tags, source provenance, pins, palettes, similarity, duplicate review. |
| CKC-EVOLVED-003 | Selective sheet ingest/version/revert | `SheetIngestMergeTools.tsx`, `SheetVersionTools.tsx`, `SheetVersion`, tests | Preserve as PostgreSQL sheet revision workflow with protected fields and selective apply/revert. |
| CKC-EVOLVED-004 | Intake batches and classification | `IntakeSorterView.tsx`, `IntakeBatch*`, ingestion tests | Preserve batch intake model, selection/status, assignment, contact sheet output; reject localhost-only authority. |
| CKC-EVOLVED-005 | Collections/contact sheets | `Collection*`, `ContactSheet`, `backend_collections.test.js` | Preserve cross-character collections, bulk tag apply, contact sheets. |
| CKC-EVOLVED-006 | PoseKit/OpenPose sidecars | `posekit\core.mjs`, `PoseView.tsx`, PoseKit tests | Preserve rig/calibration/head-pose/openpose/hand/face model concepts; persist in PostgreSQL-native schema. |
| CKC-EVOLVED-007 | Identity profile replay metadata | `IdentityProfile`, identity tests, ComfyUI bridge | Preserve identity profile entity and workflow injection receipts; reject CKC local bridge as runtime authority. |
| CKC-EVOLVED-008 | ComfyUI workflow receipts/replay | `comfyuiClient.js`, `registerComfyUIOutput`, workflow tests | Preserve workflow JSON, prompts, seed/model/sampler metadata, output lineage, replay receipt model. |
| CKC-EVOLVED-009 | Recovery/diagnostics/backup | `backup.js`, `resetModes.js`, diagnostics/backup tests | Preserve diagnostics, repair by hash, versioned backup manifest; implement PostgreSQL-native dump/restore and object-store checks. |
| CKC-EVOLVED-010 | Search/tag intelligence | `globalSearch`, `SavedSearch`, tag manager tests | Preserve multi-surface search, saved filters, tag stats/merge/rename/templates/rules; use PostgreSQL indexes/search. |
