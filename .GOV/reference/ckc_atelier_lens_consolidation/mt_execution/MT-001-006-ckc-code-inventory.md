---
file_id: MT-001-006-ckc-code-inventory
file_kind: ckc_source_inventory
updated_at: "2026-05-16"
work_packet: WP-1-Atelier-Lens-Consolidation-v1
source_root_observed: "D:/Projects/LLM projects/CastKit-Codex/CKC_main"
write_scope: ".GOV/reference/ckc_atelier_lens_consolidation/mt_execution"
---

<topic id="inventory-scope" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="Read-only CKC inventory scope." updated_at="2026-05-16">

# MT-001 through MT-006 CKC Code Inventory

This artifact records read-only source evidence from CKC for Handshake Atelier Lens consolidation. CKC remains source evidence only. This artifact does not authorize CKC rebuild stubs, copied CKC modules, or direct CKC product paths in Handshake.

Source root inspected: `D:/Projects/LLM projects/CastKit-Codex/CKC_main`.

Owned outputs:
- `.GOV/reference/ckc_atelier_lens_consolidation/mt_execution/MT-001-006-ckc-code-inventory.md`
- `.GOV/reference/ckc_atelier_lens_consolidation/mt_execution/MT-001-006-ckc-code-inventory.json`

</topic>

<topic id="sqlite-rejection" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="SQLite is rejected for Handshake." updated_at="2026-05-16">

# SQLite Rejection

SQLite is absolutely rejected for Handshake translation. No Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths may use SQLite.

CKC evidence contains SQLite in all of the following places and every one is evidence-only, not reusable architecture:
- `package.json`: dependency `sqlite3`.
- `app/backend/db.js`: `require('sqlite3')`, `normalizeProvider()` fallback to SQLite, `openSqliteDb()`, SQLite compatibility SQL translation, and `openDb()` selecting SQLite when provider is not Postgres.
- `app/main.js`: config normalization accepts `sqlite` and `sqlite3`; default library code references `db/codex.db`.
- `test/fixtures/legacy/wp-0091/db/codex.db`
- `test/fixtures/legacy/wp-0100/db/codex.db`
- `test/fixtures/legacy/wp-0103/db/codex.db`
- `test/fixtures/legacy/wp-0104/db/codex.db`
- `test/legacy_fixture_compatibility.test.js`: exercises frozen legacy SQLite fixture migration.

Handshake translation note: use durable Postgres-first or repository-approved storage contracts only. Do not port CKC SQLite fallback behavior, fixture compatibility behavior, SQLite migration tests, `.db` artifacts, or test helpers.

</topic>

<topic id="mt-001-package-runtime-dependencies" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="Package and runtime dependency inventory." updated_at="2026-05-16">

# MT-001 Package and Runtime Dependencies

Primary source paths:
- `package.json`: app metadata, npm scripts, runtime dependencies, dev dependencies, Electron builder config.
- `package-lock.json`: locked npm dependency graph.
- `vite.config.ts`: Vite React config, worker format, MediaPipe WASM copy/dev middleware.
- `tsconfig.json`: TypeScript project config.
- `index.html`: Vite renderer HTML entry.
- `app/main.js`: Electron main-process runtime, config defaults, IPC registration.
- `app/preload.js`: `contextBridge` surface exposed as `window.ckc`.
- `src/main.tsx`: React renderer mount.
- `src/vite-env.d.ts`: renderer/global CKC API and domain type declarations.
- `public/posekit_models/*.task`: MediaPipe model assets for pose, hand, and face detection.
- `scripts/*.ps1`, `scripts/*.sh`, `scripts/installer_custom.nsh`: packaging and release scripts.

Observed dependency/runtime signals:
- Electron desktop app: `main` points to `app/main.js`; scripts include `electron:dev`, `dev:electron`, and `electron:build`.
- Vite/React renderer: `vite`, `@vitejs/plugin-react`, `react`, `react-dom`, `typescript`.
- Pose/rendering stack: `@mediapipe/tasks-vision`, `three`, `@react-three/fiber`, `@react-three/drei`.
- Data/storage stack: `pg` is present and CKC defaults config to Postgres; `sqlite3` is present but rejected for Handshake.
- Test runner: `node --test`.
- Build outputs are configured outside CKC_main into sibling CKC_GOV target folders.

Handshake translation notes:
- Reuse only architectural signals: Electron-style IPC separation, explicit automation surface, model assets as static runtime inputs, and Postgres-oriented storage contracts.
- Do not reuse CKC package identity, CKC app naming, CKC_GOV output paths, or CKC packaging scripts as Handshake product scaffolding.
- Remove SQLite completely from any Handshake dependency graph, lockfile, config, test fixture, migration path, cache path, or fallback.

</topic>

<topic id="mt-002-backend-service-files" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="Backend service inventory." updated_at="2026-05-16">

# MT-002 Backend Service Files

Primary source paths and observed purpose:
- `app/main.js`: Electron main process; loads config, starts windows, initializes CKCLibrary, registers IPC handlers, starts ComfyUI intake server, handles automation capture and command dispatch.
- `app/preload.js`: bridge from renderer to backend IPC commands. Important as the observable API catalog.
- `app/backend/library.js`: central product service layer for characters, templates, images, rigs, PoseKit, identity profiles, workflow history, prompts, story beats, exports, collections, search, diagnostics, backup coordination, and intake classification.
- `app/backend/db.js`: database abstraction, schema initialization, SQL helpers, Postgres provider support, SQLite fallback support in CKC only.
- `app/backend/backup.js`: library backup and restore.
- `app/backend/automationControl.js`: automation sessions, leases, command queue/log behavior.
- `app/backend/automationManual.js`: model-facing automation manual.
- `app/backend/automationCommandMap.js`: command allowlist/catalog, including PoseKit and ComfyUI commands.
- `app/backend/automationStealth.js`: background/stealth automation constraints.
- `app/backend/intakeServer.js`: localhost intake HTTP server for ComfyUI output bundles.
- `app/backend/comfyuiClient.js`: ComfyUI client functions for stats, queue/prompt, and history retrieval.
- `app/backend/llm.js`: OpenAI-compatible chat/completion integration.
- `app/backend/imageSourcingAdapter.js` and `app/backend/imageSourcingHandlers/v00_19.js`: workflow-specific image sourcing/import handling.
- `app/backend/workflowSpecRegistry.js`: registry for workflow specs and ingestion handlers.
- `app/backend/templateParser.js`, `sheet.js`, `validation.js`, `text.js`: template/sheet parsing and validation utilities.
- `app/backend/dhash.js`, `palette.js`, `crypto.js`: image hashing, palette extraction, hashing helpers.
- `app/backend/resetModes.js`: installer/reset mode behavior.

Observed dependency/runtime signals:
- `app/main.js` default config sets database provider to Postgres with local host/port/user/password defaults.
- `app/main.js` also accepts CKC SQLite provider names; this must not translate to Handshake.
- `app/backend/library.js` imports `openDb`, `initSchema`, `run`, `get`, `all`, and `isPostgresDb` from `db.js`.
- `app/backend/library.js` carries a large domain-service surface rather than small bounded services.
- `app/preload.js` exposes many stable command names, making it useful as a source API inventory even if implementation is not copied.

Handshake translation notes:
- High-ROI reuse is not code copy; it is extracting service boundaries: automation control plane, intake server, workflow registry, PoseKit rig service, prompt/story workflow service, diagnostics, and export service.
- Split Handshake services by bounded context instead of preserving CKC's monolithic `library.js`.
- Preserve stable command-surface thinking: Handshake should expose explicit model-safe command contracts and receipts.
- Reject all SQLite abstractions and compatibility adapters from `db.js`.

</topic>

<topic id="mt-003-ui-views-reusable-behavior" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="UI view and reusable behavior inventory." updated_at="2026-05-16">

# MT-003 UI Views and Reusable Behavior

Primary view source paths and observed purpose:
- `src/ui/App.tsx`: top-level page routing, command palette/global search state, PoseKit workbench context, renderer automation command handling, drawer/page selection.
- `src/ui/views/LibraryView.tsx`: library browsing, filtering, image/character navigation.
- `src/ui/views/CharacterView.tsx`: character sheet editing, media panes, templates, tags, relationships, protected fields, version/merge flows.
- `src/ui/views/ExportHubView.tsx`: export workflows.
- `src/ui/views/IntakeSorterView.tsx`: intake triage and classification.
- `src/ui/views/PoseView.tsx`: PoseKit workbench, rig tabs, detection, OpenPose export, identity profile panel, calibration controls, sidecar previews, ComfyUI replay trigger.
- `src/ui/views/WorkflowView.tsx`: prompt/story beat/workflow history tabs and ComfyUI replay.
- `src/ui/views/SettingsView.tsx`: config/settings surface.
- `src/ui/views/ReferenceWindowView.tsx`: reference window UI.

Reusable behavior/components:
- `src/ui/components/CommandPalette.tsx`, `CommandBar.tsx`, `Drawer.tsx`, `LibraryDrawer.tsx`: navigation and command surfaces.
- `src/ui/components/MediaPane.tsx`, `MoodboardCanvas.tsx`: large reusable media and canvas interaction surfaces.
- `src/ui/components/Pose3DViewport.tsx`: Three.js/React Three Fiber PoseKit preview.
- `src/ui/components/SheetEditor.tsx`, `SheetField.tsx`, `SheetVersionTools.tsx`, `SheetIngestMergeTools.tsx`, `SheetFieldChangePicker.tsx`: sheet editing and version/merge behavior.
- `src/ui/components/BlockEditor.tsx`, `BlockListEditor.tsx`, `blockListSerialize.ts`: structured block editing.
- `src/ui/components/*Dialog*.tsx`, `*Modal*.tsx`: bulk edit, tag, template, search, help modal behavior.
- `src/ui/hooks/useHotkeys.ts`, `useElementWidth.ts`: reusable keyboard and layout helpers.
- `src/ui/styles/*.css`, `src/ui/views/*.module.css`, `src/ui/components/*.module.css`: CSS module styling and global styles.

Observed dependency/runtime signals:
- Renderer uses `window.ckc` from preload as the backend boundary.
- Pose and workflow routes include stable selectors such as `data-testid="pose-view"`, `data-action="pose-detect"`, `data-action="pose-export-openpose"`, and `data-action="workflow-replay-comfyui"`.
- UI tests statically assert ARIA tab semantics and stable automation selectors.

Handshake translation notes:
- Reuse the concept of stable model-facing selectors and command surfaces for Atelier Lens.
- Do not copy CKC's visual layout wholesale; treat CKC UI as evidence for workflows: library, character, intake, pose, workflow, exports, settings.
- Carry over the expectation that GUI behavior has automation-observable state, not hidden session-only behavior.

</topic>

<topic id="mt-004-posekit-files" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="PoseKit file inventory." updated_at="2026-05-16">

# MT-004 PoseKit Files

Primary source paths and observed purpose:
- `src/posekit/core.mjs`: PoseKit domain math and serialization; fallback rig construction, MediaPipe-to-rig fitting, head-pose math, OpenPose JSON export, OpenPose canvas rendering.
- `src/posekit/core.d.mts`: type declarations for PoseKit core module.
- `src/posekit/poseDetectionClient.ts`: renderer-facing pose detection wrapper; image bitmap creation; worker invocation; fallback rig behavior.
- `src/posekit/poseDetection.worker.ts`: MediaPipe worker using `@mediapipe/tasks-vision`; loads pose, face, and hand landmarkers; CPU delegate; emits rig with OpenPose.
- `public/posekit_models/pose_landmarker_lite.task`: pose model asset.
- `public/posekit_models/face_landmarker.task`: face model asset.
- `public/posekit_models/hand_landmarker.task`: hand model asset.
- `src/ui/views/PoseView.tsx`: UI and workflow orchestration for detection, calibration, rig workspaces, OpenPose export, identity profiles, and replay.
- `src/ui/components/Pose3DViewport.tsx`: 3D pose visualization.
- `app/backend/library.js`: backend rig CRUD, open rig workspace state, calibration, OpenPose sidecar export, identity profile CRUD, workflow replay injection.
- `app/backend/db.js`: `Rig`, `IdentityProfile`, `RigTag`, `ImageAsset.openpose_png_path`, product event schema.
- `app/preload.js` and `app/main.js`: IPC for `listRigs`, `getRig`, `createRig`, `updateRigCalibration`, `setRigHeadPose`, `setRigPortrait`, `updateRigPose`, `exportOpenposePng`, `listOpenposeSidecars`, rig workspace commands, and identity profile commands.

Observed dependency/runtime signals:
- PoseKit depends on MediaPipe Tasks Vision WASM/model assets and Web Worker execution.
- Vite config copies/serves MediaPipe WASM files under `/wasm/`.
- `poseDetection.worker.ts` uses CPU delegate only for deterministic Electron automation behavior.
- PoseKit exports canonical OpenPose body/face/hand arrays and sidecar PNG/JSON artifacts.
- Pose UI exposes stable automation selectors for detection, export, calibration, hand visibility, workspace tabs, identity panel, and ComfyUI replay.

Handshake translation notes:
- Treat PoseKit as an evidence bundle for an Atelier Lens pose subsystem: source image, rig, calibration, OpenPose sidecar, identity profile, replay into generation workflow.
- Keep the model-asset and WASM loading pattern as research evidence, not as a CKC path copy.
- If Handshake builds a pose subsystem, define independent Handshake domain contracts and storage schemas first.
- SQLite-backed rig or fixture behavior is rejected.

</topic>

<topic id="mt-005-comfyui-bridge-files" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="ComfyUI bridge inventory." updated_at="2026-05-16">

# MT-005 ComfyUI Bridge Files

Primary source paths and observed purpose:
- `comfyui_node/castkit_codex_bridge.py`: ComfyUI custom output node; saves images to ComfyUI output; optionally POSTs base64 image, workflow JSON, metadata, character, rig, OpenPose ref, and identity profile refs to CKC intake.
- `comfyui_node/__init__.py`: ComfyUI node package entry.
- `comfyui_node/README.md`: install/env var instructions.
- `app/backend/intakeServer.js`: local intake server endpoint for ComfyUI output.
- `app/backend/comfyuiClient.js`: ComfyUI client-side queue/history/stats integration.
- `app/backend/library.js`: `registerComfyUIOutput`, workflow prompt extraction, workflow replay, bridge-node input injection, workflow history, identity profile injection.
- `app/backend/workflowSpecRegistry.js`: workflow spec and ingestion routing registry.
- `app/backend/imageSourcingAdapter.js`, `app/backend/imageSourcingHandlers/v00_19.js`, `app/backend/imageSourcingHandlers/_pinned.json`: ingestion routing/handler evidence.
- `src/ui/views/WorkflowView.tsx`: UI for prompt/story/workflow tabs and replay.
- `src/ui/views/PoseView.tsx`: replay latest workflow using rig/openpose/identity context.
- `app/preload.js`, `app/main.js`, `app/backend/automationCommandMap.js`: IPC/automation commands for ComfyUI registration, replay, stats, and workflow history.

Observed dependency/runtime signals:
- Bridge env vars include `CKC_INTAKE_URL`, `CKC_INTAKE_CHARACTER`, `CKC_INTAKE_TOKEN`, `CKC_INTAKE_RIG`, `CKC_INTAKE_OPENPOSE_REF`, and `CKC_INTAKE_SESSION`.
- Default CKC intake URL in docs is `http://127.0.0.1:52319/intake/comfyui_output`.
- CKC app config defaults ComfyUI host to `http://127.0.0.1:8188` and intake port `52319`.
- The Python node treats CKC POST failures as non-fatal after saving to ComfyUI output.

Handshake translation notes:
- Reuse the bridge pattern: generation tool saves its own output first, then performs best-effort registration into the product via a local authenticated intake endpoint.
- Handshake should define its own schema name, auth/receipt model, and product event/audit model.
- Do not reuse `CastKitCodexBridge` naming or CKC schema identifiers as Handshake product paths.
- No SQLite-backed intake, workflow history, bridge cache, fixture, or compatibility path is allowed.

</topic>

<topic id="mt-006-tests-by-behavior-area" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="Tests grouped by behavior area." updated_at="2026-05-16">

# MT-006 Tests by Behavior Area

Test runtime:
- `package.json`: `npm test` runs `node --test`.
- `test/*.test.js`: Node test files.

Behavior-area inventory:
- Automation/model workflow: `automation_background_stealth_invariants.test.js`, `automation_input_injection_invariants.test.js`, `automation_manual_consistency.test.js`, `identity_profile_replay_injection.test.js`.
- Backend library CRUD and domain services: `backend_batch_character_operations.test.js`, `backend_character_icons.test.js`, `backend_character_scripts.test.js`, `backend_character_templates.test.js`, `backend_collections.test.js`, `backend_docs.test.js`, `backend_field_value_suggestions.test.js`, `backend_global_search.test.js`, `backend_identity_decoupling.test.js`, `backend_links_backlinks.test.js`, `backend_public_character_id.test.js`, `backend_ratings_filters.test.js`, `backend_relations.test.js`, `backend_saved_searches.test.js`, `backend_sheet.test.js`, `backend_story_board.test.js`, `backend_tag_manager.test.js`, `backend_validation.test.js`.
- Image/media/intake: `adopt_orphan_images.test.js`, `backend_ai_tagging_suggestions.test.js`, `backend_duplicates.test.js`, `backend_image_annotations.test.js`, `backend_image_meta_batch.test.js`, `backend_ingestion_batches.test.js`, `backend_repair_missing_images.test.js`, `backend_similarity_search.test.js`, `backend_web_import_url_capture.test.js`, `dhash.test.js`, `ingestion_handler_routing.test.js`, `ingestion_idempotency.test.js`, `palette_extraction.test.js`.
- Backup/export/install/reset: `backend_backup_restore.test.js`, `backend_exports.test.js`, `backend_export_hub.test.js`, `backup_version_traceability.test.js`, `full_reset_marker.test.js`, `installer_modes_invariants.test.js`.
- PoseKit/OpenPose/rigs: `backend_posekit_crud.test.js`, `backend_posekit_schema.test.js`, `backend_posekit_workflow.test.js`, `hand_detection_taxonomy.test.js`, `hand_openpose_export.test.js`, `identity_profile_crud.test.js`, `pose_head_pose_math.test.js`, `posekit_core.test.js`, `posekit_ui_static.test.js`, `rig_workspace_tabs.test.js`.
- ComfyUI/workflow bridge: `comfyui_node_contract.test.js`, `intake_server.test.js`, `backend_workflow_spec_registry.test.js`, `spec_canon_consistency.test.js`.
- Template/parser/validation/static contracts: `backend_templateParser.test.js`, `block_list_editor_serialize.test.js`, `block_list_validation.test.js`, `canonical_template.test.js`, `template_field_id_immutability.test.js`, `template_parser_unions.test.js`, `validation_field_types.test.js`.
- Database/provider/migration invariants: `backend_postgres_provider.test.js`, `db_index_invariants.test.js`, `migration_invariants.test.js`, `sql_alias_regression.test.js`, `legacy_fixture_compatibility.test.js`.
- Regression suites: `backend_startup_guards.test.js`, `backend_wp0122_0135_regression.test.js`.

Observed dependency/runtime signals:
- Tests instantiate `CKCLibrary` and exercise backend behavior through direct service calls.
- Static UI tests inspect source files for selectors and ARIA semantics.
- Pose tests import `src/posekit/core.mjs` directly.
- Legacy fixture tests include SQLite `.db` files and compatibility migration behavior.

Handshake translation notes:
- Reuse behavior-area coverage ideas, not CKC test fixtures or CKC DB files.
- Add Handshake tests around explicit command contracts, receipts, Postgres storage, PoseKit-like data contracts, ComfyUI intake registration, and GUI automation selectors.
- Reject `legacy_fixture_compatibility.test.js` and all `test/fixtures/legacy/**/db/codex.db` usage as Handshake inputs.

</topic>

<topic id="key-findings" status="complete" version="v1" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="Key findings for translation." updated_at="2026-05-16">

# Key Findings

1. CKC has useful source evidence for Atelier Lens workflows: library/character UI, PoseKit workbench, OpenPose sidecars, identity profiles, ComfyUI replay/intake, and model-facing automation.
2. CKC backend service code is centralized in `app/backend/library.js`; Handshake should split this into bounded services instead of cloning the monolith.
3. CKC already has stable automation selectors and command surfaces; this is directly relevant to Handshake's model-facing usability requirements.
4. CKC contains both Postgres and SQLite support. SQLite must be treated as a negative example and hard rejection for Handshake.
5. CKC ComfyUI bridge has a strong product pattern: generation output remains in ComfyUI first, while product registration is best-effort and local-intake based.
6. CKC tests provide useful behavior categories, but SQLite fixtures and compatibility tests are not portable into Handshake.

</topic>
