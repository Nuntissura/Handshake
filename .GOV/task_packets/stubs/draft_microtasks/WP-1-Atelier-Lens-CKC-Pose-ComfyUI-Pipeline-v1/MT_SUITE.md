---
file_id: mt-suite-wp-1-atelier-lens-ckc-pose-comfyui-pipeline-v1
file_kind: draft_microtask_suite
updated_at: 2026-05-16
status: draft_non_executable
wp_id: WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1
official_microtasks_generated: false
replaces_inline_twenty_task_draft: true
---

<topic id="draft-microtask-suite" status="draft" version="v1" wp="WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1" summary="Fresh no-context draft microtasks for Pose ComfyUI Pipeline" updated_at="2026-05-16">

# Draft Microtask Suite: Pose, OpenPose, Identity, ComfyUI

These microtasks replace the prior 20-item draft list. They are not executable until the stub is activated into an official signed work packet with refinement, packet contract, and generated `MT-*.json` / `MT-*.md` contracts.

Each MT is a single implementation slice. If a future implementer finds one MT touches unrelated files, split it during activation and preserve the original requirement text.

## Shared Execution Rules For Every MT

- Authority: planning draft only; no coder or validator may start from this file before activation.
- Runtime translation: CKC code is source evidence, not code to copy. Implement through Handshake domains, Workflow Engine, ArtifactStore, EventLedger, Flight Recorder, and governed external-tool adapters.
- Rejection rules: no CKC namespace, Electron IPC authority, SQLite, localhost intake authority as truth, direct LLM tool execution, hidden output writes, or product outputs under `.GOV`.
- Output-first rule: generated images or sidecars must not be lost when registration fails; failures produce retryable diagnostic records.
- Status rule: CKC DONE rows seed fixtures; REVIEW rows require evidence review; BLOCKED and PLANNED rows remain unresolved/deferred until explicitly activated.

### MT-001 - Pose Source Evidence And Status Matrix
- Objective: Build the source/evidence matrix for PoseKit, OpenPose, identity profiles, ComfyUI, workflow registry, and image-sourcing adapter.
- Inputs: repaired Pose stub; CKC taskboard `WP-0107` through `WP-0117`, `WP-0131`, `WP-0132`, `WP-0133`; CKC PoseKit/ComfyUI code inventory; greenroom reference files.
- Work slice: create/update activation evidence matrix only.
- Outputs: matrix with source path, behavior, CKC status, owner boundary, implementation/deferred decision, and fixture need.
- Acceptance: `WP-0133` is marked BLOCKED/unresolved; planned `WP-0112`, `WP-0114`, `WP-0116`, `WP-0117` are not counted as done.
- Verification: compare matrix rows to taskboard anchors and repaired stub EVOL rows.
- Depends on: none.

### MT-002 - Pose Product Anchor Verification
- Objective: Verify Handshake product anchors for pose, media, artifact, workflow, external tools, and diagnostics.
- Inputs: activated packet, product worktree, `.GOV/spec/SPEC_CURRENT.md`, Core/Data anchor map.
- Work slice: inspect files and produce verified path map or `BLOCKED_MISSING_ANCHOR`.
- Outputs: anchor map for pose domain, sidecars, identity profiles, workflow receipt storage, ComfyUI adapter, tests, and visual evidence hooks.
- Acceptance: no later MT names an unverified path without a blocker.
- Verification: `rg --files` evidence and anchor-map artifact.
- Depends on: MT-001.

### MT-003 - Pose Runtime Rejection Gates
- Objective: Add checks that prevent forbidden CKC runtime assumptions in pose pipeline code.
- Inputs: runtime rejection map in repaired stub.
- Work slice: create validation checks for SQLite, CKC namespace, localhost authority, Electron IPC, direct LLM execution, `.GOV` outputs, and machine-local paths.
- Outputs: tests/check script integrated into WP validation plan.
- Acceptance: historical references under `.GOV/reference` are allowed; runtime/product/test paths fail on forbidden assumptions.
- Verification: positive and negative checks run with expected pass/fail.
- Depends on: MT-002.

### MT-004 - Pose Domain Skeleton
- Objective: Create skeleton modules/types for the pose pipeline without business logic.
- Inputs: MT-002 anchor map; Core/Data media and character contracts.
- Work slice: add module/type skeletons for rigs, keypoints, pose context, sidecars, identity profiles, workflow receipts, replay, and external adapter boundary.
- Outputs: compile-only skeleton with clear interfaces.
- Acceptance: signatures name all durable concepts; no hardcoded CKC paths or ComfyUI URLs.
- Verification: product typecheck/build selected at activation.
- Depends on: MT-002, Core MT-003.

### MT-005 - Rig Identity And Storage Contract
- Objective: Implement durable Rig identity linked to character, source image, optional collection, and artifact refs.
- Inputs: CKC `Rig` table, `openRigWorkspace`, `listOpenRigs`.
- Work slice: add Rig fields and CRUD/list behavior.
- Outputs: rig create/get/list/update APIs and tests.
- Acceptance: closing a workspace or changing context never deletes Rig rows or source media.
- Verification: tests for create rig, list by character/image, update calibration, close-workspace no-delete.
- Depends on: MT-004, Core media/character MTs.

### MT-006 - OpenPose Keypoint Taxonomy
- Objective: Preserve Body-18, face-70, handLeft-21, and handRight-21 keypoint shapes.
- Inputs: CKC `src/posekit/core.mjs`, `core.d.mts`, posekit tests.
- Work slice: implement typed keypoint arrays and validation/serialization rules.
- Outputs: keypoint types, validation function, and fixtures.
- Acceptance: arrays have canonical lengths; missing points are represented explicitly and never shifted.
- Verification: tests for body length 18, face length 70, left/right hand length 21, invalid length rejection.
- Depends on: MT-005.

### MT-007 - Detection Provider Provenance
- Objective: Record detector/provider metadata for every generated rig.
- Inputs: CKC MediaPipe provider strings, taskboard WP-0108/WP-0113.
- Work slice: add provider name, model/version, asset version/path ref, confidence availability, and generation timestamp fields.
- Outputs: provenance fields and event payload.
- Acceptance: every detected rig can answer which detector/model produced it.
- Verification: tests for provider recorded, missing provider rejected or marked manual, and event payload.
- Depends on: MT-006.

### MT-008 - Fallback And Zero-Fill Behavior
- Objective: Preserve deterministic fallback and zero-filled missing-marker behavior.
- Inputs: CKC worker fallback on model/image bitmap failure and hand-positive/zero-hand tests.
- Work slice: implement fallback result classification and zero-fill serialization rules.
- Outputs: fallback status, error reason, and serialized keypoints.
- Acceptance: failed detector does not crash pipeline; fallback is visibly marked and never mistaken for high-confidence detection.
- Verification: tests for missing model asset, image bitmap failure, no hands detected, fallback body rig.
- Depends on: MT-007.

### MT-009 - Head Pose Quaternion Contract
- Objective: Preserve yaw/pitch/roll intrinsic YXZ quaternion-backed head pose.
- Inputs: CKC `setRigHeadPose`, WP-0110.
- Work slice: add headPose fields, validation ranges, quaternion conversion/storage, and legacy yaw import if needed.
- Outputs: head pose update/read behavior and serialization into OpenPose-compatible output.
- Acceptance: yaw/pitch/roll round-trip; reset/default behavior is explicit.
- Verification: tests for `30/-15/10`, reset, invalid number, legacy yaw compatibility.
- Depends on: MT-006.

### MT-010 - Calibration JSON Contract
- Objective: Preserve calibration state as typed data.
- Inputs: CKC `Rig.calibration_json`, WP-0133 partial implementation.
- Work slice: define calibration schema for head pose, marker visibility, marker color/hand rows, history refs, and unresolved fields.
- Outputs: schema validation and storage tests.
- Acceptance: unsupported blocked fields remain represented as deferred/unresolved instead of dropped.
- Verification: tests for valid calibration, marker row, invalid JSON rejection, and unresolved blocked field marker.
- Depends on: MT-005, MT-009.

### MT-011 - OpenPose JSON Sidecar
- Objective: Materialize OpenPose JSON as a derived artifact.
- Inputs: CKC OpenPose export tests and WP-0132.
- Work slice: implement JSON sidecar creation with source image id, rig id, hash, role, artifact ref, and manifest link.
- Outputs: export JSON API/job and artifact manifest.
- Acceptance: sidecar filenames are no-space/content-addressed; normal galleries hide sidecars by default.
- Verification: tests for JSON shape, hash stability, artifact manifest, and Core sidecar projection exclusion.
- Depends on: MT-006, MT-016.

### MT-012 - OpenPose PNG Sidecar
- Objective: Materialize OpenPose PNG preview/conditioning sidecar.
- Inputs: CKC `exportOpenposePng`, WP-0108, WP-0132.
- Work slice: implement PNG sidecar job with source image, rig, dimensions, output artifact, and error handling.
- Outputs: PNG sidecar API/job and manifest.
- Acceptance: PNG generation is deterministic for the same rig/source inputs.
- Verification: tests for generated PNG metadata, source link, no-space path, failure record.
- Depends on: MT-011.

### MT-013 - Sidecar Relation Query
- Objective: Preserve lookup of OpenPose sidecars by source image and rig.
- Inputs: CKC `listOpenposeSidecars`, Core sidecar visibility matrix.
- Work slice: implement relation query and projection contract consumed by Core/Data galleries.
- Outputs: list sidecars API with source image id, rig id, media role, artifact refs.
- Acceptance: sidecars are visible in explicit OpenPose projection but not normal gallery.
- Verification: tests for list by image, list by rig, normal gallery exclusion, missing artifact health.
- Depends on: MT-011, MT-012, Core sidecar MT.

### MT-014 - Pose Context State Contract
- Objective: Preserve blank, single-image, character-linked, and collection-linked pose contexts.
- Inputs: CKC `PoseView.tsx`, `getPoseKitState`, WP-0131.
- Work slice: implement context state type with context kind, selected image, character, collection, active rig, source strip, sidecar strip.
- Outputs: context state API/projection.
- Acceptance: context switching does not delete rigs, source media, collection links, or sidecars.
- Verification: tests for blank -> single, character -> collection, reload state, invalid context ref.
- Depends on: MT-005, Core collection/media MTs.

### MT-015 - Source And OpenPose Strip State
- Objective: Preserve independent source image and OpenPose sidecar strips.
- Inputs: CKC WP-0131 and MediaPane sidecar behavior.
- Work slice: add projection/query fields for source thumbnails and sidecar thumbnails separately.
- Outputs: structured state projection consumed by Diagnostics.
- Acceptance: a model can inspect both source images and derived sidecars without screen scraping.
- Verification: tests for strip counts, selected source id, selected sidecar id, and empty state.
- Depends on: MT-014, MT-013.

### MT-016 - Multi-Rig Workspace State
- Objective: Preserve session-scoped multi-rig workspace tabs as data/state.
- Inputs: CKC WP-0115, `openRigWorkspace`, `setActiveRig`, `closeRigWorkspace`, `reorderOpenRigWorkspaces`.
- Work slice: implement workspace state records or session projection for open rigs, active rig, order, dirty calibration, panel state.
- Outputs: list/open/set-active/close/reorder behavior.
- Acceptance: closing a workspace never deletes Rig rows; save-before-switch/close policy is explicit.
- Verification: tests for two rigs open, reorder, close inactive, close active, dirty indicator.
- Depends on: MT-005, Diagnostics session MTs.

### MT-017 - Multi-Rig Keyboard And Route Semantics
- Objective: Preserve keyboard/navigation expectations without full UI build.
- Inputs: CKC WP-0115 and WP-0116 planned shortcuts.
- Work slice: define command/action catalog rows for active rig switching, tab navigation, close, save, and route persistence.
- Outputs: command rows for Diagnostics manual/action catalog.
- Acceptance: commands are typed and no shortcut is treated as implemented UI unless a later UI MT wires it.
- Verification: command catalog consistency check and deferred shortcut status.
- Depends on: MT-016, Diagnostics command catalog MT.

### MT-018 - Identity Profile Record
- Objective: Preserve identity profiles for face/reference workflows.
- Inputs: CKC `IdentityProfile` table/CRUD and WP-0111.
- Work slice: implement identity profile fields: id, character id, name, description, source media refs, crop refs, artifact refs, version, created/updated.
- Outputs: CRUD/list APIs and tests.
- Acceptance: identity profile references are stable and do not leak character names in artifact paths.
- Verification: tests for create/list/update/delete, source media validation, privacy path check.
- Depends on: MT-004, Core media/identity MTs.

### MT-019 - Identity Crop Artifact
- Objective: Preserve 512x512 content-hash face crop artifact behavior.
- Inputs: CKC identity crop/manifest logic.
- Work slice: implement crop job contract with source image, crop box/landmarks, output artifact, hash, manifest.
- Outputs: identity crop artifact and manifest tests.
- Acceptance: crop output is content-addressed and linked to identity profile version.
- Verification: tests for valid crop, invalid source image, hash/no-space path, manifest link.
- Depends on: MT-018, Core ArtifactStore MT.

### MT-020 - Identity Metadata For Workflows
- Objective: Preserve landmarks/measurements/pose metadata for identity payloads.
- Inputs: CKC identity profile metadata and ComfyUI identity inputs.
- Work slice: implement optional metadata fields and serialization into workflow receipt inputs.
- Outputs: metadata schema and validation.
- Acceptance: missing optional metadata is allowed; present metadata is validated and traceable.
- Verification: tests for landmarks, measurements, pose metadata, absent metadata, invalid payload.
- Depends on: MT-018.

### MT-021 - Workflow Receipt Schema
- Objective: Implement durable ComfyUI workflow receipt schema.
- Inputs: CKC `registerComfyUIOutput`, `getWorkflowHistory`, repaired stub minimum receipt fields.
- Work slice: add receipt fields for external system/run id, workflow spec, workflow JSON artifact, extracted prompt, character/sheet/media/rig/openpose/identity refs, output refs, status, error, timestamps, evidence refs.
- Outputs: receipt create/get/list APIs and tests.
- Acceptance: every output registration creates a receipt or failed receipt; no output is lost silently.
- Verification: tests for successful receipt, failed receipt, missing output artifact, and list by character.
- Depends on: MT-005, MT-011, MT-018, Core media MTs.

### MT-022 - Output-First Registration Failure Recovery
- Objective: Preserve generated output before registration and create retryable failure evidence.
- Inputs: CKC non-fatal bridge failure posture, repaired stub output-first rule.
- Work slice: implement failure record with output artifact/path ref, attempted payload, error code/message, retry action.
- Outputs: failed registration state and retry API/contract.
- Acceptance: a saved image can be registered later after a transient error.
- Verification: tests for output saved then registration failure, retry success, retry duplicate idempotency.
- Depends on: MT-021.

### MT-023 - Workflow History And Stats
- Objective: Preserve workflow history and stats queries.
- Inputs: CKC `getWorkflowHistory`, `getComfyUIStats`.
- Work slice: implement history query and aggregate stats over receipts by character, workflow spec, status, time range.
- Outputs: history/stats APIs.
- Acceptance: history includes success and failure receipts with source refs and replay availability.
- Verification: tests for list recent, filter by character, failed included, stats counts.
- Depends on: MT-021.

### MT-024 - Replay Input Contract
- Objective: Preserve replayable workflow inputs.
- Inputs: CKC `replayWorkflow`, workflow JSON/prompt extraction behavior.
- Work slice: define replay request schema and resolve all artifact refs needed for replay.
- Outputs: replay request/result types and validation.
- Acceptance: replay can be attempted only when required workflow JSON and output/conditioning refs are available.
- Verification: tests for valid replay request, missing workflow JSON, missing source sidecar, and validation error.
- Depends on: MT-021.

### MT-025 - SaveImage Output Discovery Fallback
- Objective: Preserve vanilla ComfyUI `SaveImage` output discovery when bridge node is absent.
- Inputs: CKC WP-0109 live verification notes.
- Work slice: implement adapter logic contract for polling `/history` and discovering `/view` outputs through a capability-gated external adapter.
- Outputs: adapter interface and fake-adapter tests.
- Acceptance: bridge-node absence is not fatal when vanilla output discovery succeeds.
- Verification: fake history response test, fake view image registration, timeout failure, malformed response.
- Depends on: MT-024, MT-029.

### MT-026 - Workflow Spec Registry
- Objective: Preserve versioned workflow specs and handler routing.
- Inputs: CKC `workflowSpecRegistry.js`, external spec root, WP-0100.
- Work slice: implement registry model with spec version, read-only source refs, handler id, compatibility pin, and validation.
- Outputs: registry load/list/get APIs and tests.
- Acceptance: unknown spec version fails with structured error; known versions route deterministically.
- Verification: tests for v00.19 handler, unknown v00.20, invalid spec, read-only root.
- Depends on: MT-004.

### MT-027 - Image-Sourcing Adapter
- Objective: Preserve image-sourcing adapter dispatch and idempotent source-to-output mapping.
- Inputs: CKC `imageSourcingAdapter.js`, `imageSourcingHandlers/v00_19.js`, `IngestionBatch`, `IngestionRejection`.
- Work slice: implement adapter request/result schema and handler dispatch through Workflow Engine.
- Outputs: adapter interface, request validation, and result mapping tests.
- Acceptance: rerunning the same source task is idempotent and preserves accepted/pending/rejected lanes.
- Verification: tests for duplicate-only rerun, accepted output, pending output, rejected output.
- Depends on: MT-026, Core intake MTs.

### MT-028 - Accepted/Pending/Rejected Workflow Lanes
- Objective: Preserve image-sourcing lanes after generated/imported outputs.
- Inputs: CKC WP-0100 and Core intake lifecycle.
- Work slice: map workflow outputs into Core intake/media states with source task/run refs.
- Outputs: lane mapping function and events.
- Acceptance: pending/rejected items do not create ordinary accepted media silently.
- Verification: tests for accepted, pending, rejected, rejection reason, EventLedger payload.
- Depends on: MT-027, Core intake MTs.

### MT-029 - ComfyUI Capability And Endpoint Boundary
- Objective: Define ComfyUI as governed external tool execution.
- Inputs: Workflow Engine/tool policy Product Reference, CKC bridge behavior.
- Work slice: implement or skeleton only the adapter boundary contract, capability check, endpoint config validation, and direct-LLM-call rejection.
- Outputs: adapter interface, endpoint config schema, capability preflight result, and boundary tests.
- Acceptance: LLM cannot call ComfyUI directly; missing endpoint/capability is denied before any job is created.
- Verification: tests for missing endpoint, missing capability, direct-call rejection, valid endpoint config, and preflight receipt.
- Depends on: MT-003, Diagnostics tool-policy MTs.

### MT-030 - External Tool And Model Version Policy
- Objective: Capture version discovery/pinning for pose detectors, ComfyUI, image tools, and model assets.
- Inputs: CKC MediaPipe task assets, ComfyUI 0.20.1 live evidence, repaired stub external tool posture.
- Work slice: add version metadata fields and preflight checks.
- Outputs: capability/version preflight and evidence record.
- Acceptance: pipeline can report tool/model versions or block with actionable error.
- Verification: tests for missing model asset, unsupported version, allowed version, preflight evidence.
- Depends on: MT-029.

### MT-031 - Pose Detection Rig And Head-Pose Event Families
- Objective: Emit EventLedger/Flight Recorder events for pose detection, rig update, and head-pose actions only.
- Inputs: repaired stub event requirements for pose detection/rig/head pose and Core event baseline.
- Work slice: define/implement events for detector preflight, detection start/result/fail, rig create/update/delete, and head-pose compute/update.
- Outputs: pose/rig/head-pose event schemas and event-emitting tests.
- Acceptance: pose detection, rig mutation, and head-pose updates are observable and attributable.
- Verification: tests assert event kind and minimum payload for detector, rig, and head-pose events.
- Depends on: MT-005, MT-021, MT-029.

### MT-032 - Pose Diagnostic Bundle Hook
- Objective: Provide diagnostic bundle inputs for failed pose/ComfyUI operations.
- Inputs: Diagnostics diagnostic bundle contract.
- Work slice: expose structured failure data: request, source refs, tool versions, logs, artifacts, screenshots refs if relevant, error taxonomy.
- Outputs: diagnostic payload builder.
- Acceptance: failed detection/export/replay/registration can be investigated without chat history.
- Verification: tests for detection failure bundle, sidecar export failure bundle, ComfyUI failure bundle.
- Depends on: MT-031, Diagnostics bundle MT.

### MT-033 - Pose Fixture Corpus
- Objective: Create non-SQLite fixtures for rig/keypoint/headpose behavior.
- Inputs: CKC posekit tests and sample evidence.
- Work slice: add fixtures for body-18, face-70, hand-21, zero-hands, fallback body, head pose, calibration.
- Outputs: fixture files/builders under activated packet or product test fixtures.
- Acceptance: fixtures are portable and CKC namespace-free.
- Verification: fixture linter and tests consuming each fixture family.
- Depends on: MT-006, MT-009, MT-010.

### MT-034 - ComfyUI Receipt Fixture Corpus
- Objective: Create fixtures for workflow receipt, failed registration, replay, and SaveImage fallback.
- Inputs: CKC bridge examples and WP-0109 evidence.
- Work slice: add fake workflow JSON, extracted prompt, output artifact metadata, history response, view response, failed receipt.
- Outputs: fixture files/builders.
- Acceptance: fixtures prove success and failure paths without real ComfyUI or SQLite.
- Verification: tests consume success, failure, replay, fallback fixtures.
- Depends on: MT-021, MT-025.

### MT-035 - Blocked Calibration And History Guard
- Objective: Preserve `WP-0133` as blocked/unresolved and prevent false parity claims.
- Inputs: CKC taskboard `WP-0133`.
- Work slice: add feature flags/status rows for draggable calibration overlay, missing-marker placement, 3D/live split editing, forked history, History tab semantics.
- Outputs: deferred status records and validation guard.
- Acceptance: first pipeline can pass while these rows remain `BLOCKED_DEFERRED`, but cannot claim full `WP-0133` parity.
- Verification: test/status check confirms blocked rows are visible and not marked implemented.
- Depends on: MT-001, MT-010.

### MT-036 - Planned Multi-Subject Rig Carry-Forward
- Objective: Preserve planned multi-subject scenes without implementing them early.
- Inputs: CKC taskboard `WP-0112`.
- Work slice: add deferred schema note for RigData v2, `people[]`, per-subject calibration/head pose, per-subject masks.
- Outputs: deferred compatibility note and future migration hook.
- Acceptance: single-subject rigs are not blocked; future multi-subject migration is traceable.
- Verification: status check confirms `PLANNED_DEFERRED` and no fake `people[]` implementation.
- Depends on: MT-006.

### MT-037 - Planned Pose Polish Carry-Forward
- Objective: Preserve planned Pose tab polish items without bloating first pipeline.
- Inputs: CKC taskboard `WP-0114`, `WP-0116`, `WP-0117`.
- Work slice: add deferred rows for multi-file drag/drop, multi-angle export, clear workspace, synchronized zoom, import existing OpenPose JSON, keyboard shortcuts, stylized-landmark detector research/router.
- Outputs: deferred requirement records and future activation pointers.
- Acceptance: rows are visible and categorized as `PLANNED_DEFERRED` or `RESEARCH_DEFERRED`.
- Verification: status check and no implementation code for these rows unless explicitly activated.
- Depends on: MT-001.

### MT-038 - Pose Integration Smoke Path
- Objective: Prove one minimal pipeline from source image to rig to sidecar to workflow receipt.
- Inputs: completed earlier Pose MTs and Core media fixtures.
- Work slice: write integration test using fake detector and fake ComfyUI adapter.
- Outputs: integration test and evidence receipt.
- Acceptance: create media -> create rig -> export OpenPose JSON/PNG -> register fake workflow output -> query history.
- Verification: run focused integration test and inspect event/artifact refs.
- Depends on: MT-005, MT-011, MT-012, MT-021, MT-029.

### MT-039 - Pose Red-Team Runtime Authority And Secret Guards
- Objective: Convert runtime-authority and secret-handling risks into focused checks.
- Inputs: repaired Pose risks for direct localhost authority, direct ComfyUI execution, and secret leakage.
- Work slice: add negative checks that reject direct localhost authority, require governed Workflow Engine/tool adapters, and scrub secrets/cookies/tokens from workflow receipts and logs.
- Outputs: focused red-team checks for runtime authority and secret handling.
- Acceptance: each check fails when direct localhost execution or secret leakage is allowed.
- Verification: run focused red-team checks for authority path and secret redaction.
- Depends on: MT-003, MT-029, MT-030, MT-031.

### MT-040 - Pose Red-Team Artifact And Recovery Guards
- Objective: Convert pose artifact visibility and output-recovery risks into focused checks.
- Inputs: repaired Pose risks for output loss after registration failure, sidecar gallery clutter, and missing artifact refs.
- Work slice: add checks that workflow outputs are saved before registration, failed registration creates recoverable artifacts, OpenPose sidecars stay hidden from normal galleries, and all sidecars carry source artifact refs.
- Outputs: focused red-team checks for artifact visibility and recovery.
- Acceptance: each check fails when output-first persistence, sidecar visibility, or artifact refs are broken.
- Verification: run focused red-team checks for output-first registration, recovery queue, sidecar hiding, and artifact refs.
- Depends on: MT-011, MT-012, MT-018, MT-021 through MT-025.

### MT-041 - Pose Red-Team Schema Status And Tool-Version Guards
- Objective: Convert schema/status/tool-version risks into focused checks.
- Inputs: repaired Pose risks for schema hand-waving, blocked/planned status flattening, missing tool versions, and unsupported external tool assumptions.
- Work slice: add checks that Body-18/face/hand keypoint schemas are explicit, deferred rows stay deferred, blocked rows stay blocked, detector/ComfyUI/tool versions are recorded, and unsupported capabilities surface as gates instead of fake success.
- Outputs: focused red-team checks for schema/status/tool-version integrity.
- Acceptance: each check fails when schemas are incomplete, statuses flatten, versions are missing, or capability gates are bypassed.
- Verification: run focused red-team checks for keypoint schema, status maturity, version metadata, and capability gates.
- Depends on: MT-006 through MT-010, MT-032, MT-034, MT-036.

### MT-042 - Pose Context And Rig Manual Source Rows
- Objective: Provide Diagnostics with command/manual rows for pose contexts and rig CRUD only.
- Inputs: MT-005 through MT-010 and MT-017.
- Work slice: produce manual rows for create/open pose context, inspect context metadata, create/update/delete rig, select rig, inspect Body-18/face/hand keypoint schema, and run calibration/capability checks.
- Outputs: manual rows with command ids, params, outputs, errors, recovery, and evidence for context/rig actions.
- Acceptance: no-context manual can describe context and rig operations without reading code.
- Verification: manual source coverage check for contexts, rig CRUD, keypoint schema, and calibration gates.
- Depends on: MT-005 through MT-010, MT-017.

### MT-043 - OpenPose Sidecar And Identity Manual Source Rows
- Objective: Provide Diagnostics with command/manual rows for OpenPose sidecars and identity profiles only.
- Inputs: MT-011 through MT-020.
- Work slice: produce manual rows for export OpenPose JSON, export control PNG, inspect sidecar links, hide sidecar from gallery, create/update identity profile, attach crop metadata, and inspect identity lineage.
- Outputs: manual rows with command ids, params, outputs, errors, recovery, and evidence for sidecar/identity actions.
- Acceptance: no-context manual can describe sidecar/identity behavior without reading implementation code.
- Verification: manual source coverage check for JSON sidecars, PNG controls, sidecar hiding, identity profile fields, crop metadata, and lineage.
- Depends on: MT-011 through MT-020.

### MT-044 - Workflow Receipt Replay And Failure Manual Source Rows
- Objective: Provide Diagnostics with command/manual rows for workflow receipts, replay, history, and registration failure recovery only.
- Inputs: MT-021 through MT-033.
- Work slice: produce manual rows for create workflow receipt, inspect prompt/seed/model metadata, register output, replay workflow, inspect history/stats, recover failed output registration, and use SaveImage fallback.
- Outputs: manual rows with command ids, params, outputs, errors, recovery, and evidence for workflow actions.
- Acceptance: no-context manual can explain output-first persistence and replay without reading code.
- Verification: manual source coverage check for receipt fields, replay, history, stats, failed registration, and SaveImage fallback.
- Depends on: MT-021 through MT-033.

### MT-045 - Deferred PoseKit And Adapter Manual Source Rows
- Objective: Provide Diagnostics with command/manual rows for deferred PoseKit rows and workflow adapter boundaries only.
- Inputs: MT-034 through MT-037 and deferred/blocked rows from MT-036.
- Work slice: produce manual rows for workflow registry inspection, image-sourcing adapter lanes, blocked WP-0133 status, planned PoseKit carry-forward rows, unsupported capability responses, and activation blockers.
- Outputs: manual rows with command ids or read-only inspection ids, params, outputs, status meanings, recovery/escalation, and evidence paths.
- Acceptance: blocked/planned PoseKit work is visible as deferred or blocked, never represented as implemented.
- Verification: manual source coverage check for registry, adapter lanes, blocked rows, planned rows, and unsupported capability gates.
- Depends on: MT-034 through MT-037.

### MT-046 - ComfyUI Job Creation And Queue Adapter
- Objective: Implement or specify only the governed ComfyUI job creation and queue phase.
- Inputs: MT-029 adapter boundary, Workflow Engine job contract, CKC bridge queue behavior.
- Work slice: add job request schema, queue request builder, queued job receipt, and fake-adapter tests for accepted and rejected queue requests.
- Outputs: job creation/queue adapter slice and tests.
- Acceptance: every queued ComfyUI run has a Workflow Engine job id, source refs, capability receipt, and no direct LLM execution path.
- Verification: tests for queue success, invalid request denial, missing source refs, and job receipt fields.
- Depends on: MT-029.

### MT-047 - ComfyUI Polling And Result Capture Adapter
- Objective: Implement or specify only ComfyUI polling and result capture.
- Inputs: MT-046 queued job receipts, CKC history/view response examples.
- Work slice: add polling request schema, status mapping, result artifact capture, output metadata extraction, and fake-adapter success tests.
- Outputs: polling/result adapter slice and tests.
- Acceptance: successful ComfyUI results are captured as ArtifactStore refs before registration.
- Verification: tests for queued/running/success states, result artifact refs, missing output handling, and output metadata fields.
- Depends on: MT-046, MT-021.

### MT-048 - ComfyUI Timeout Cancel And Sanitized Log Adapter
- Objective: Implement or specify only timeout, cancel, and log sanitization for ComfyUI jobs.
- Inputs: MT-046 queued jobs, MT-047 polling states, Diagnostics cancellation/log policy.
- Work slice: add timeout policy, cancel request/receipt, sanitized error/log capture, and fake-adapter tests for timeout/cancel/failure.
- Outputs: timeout/cancel/log adapter slice and tests.
- Acceptance: timeout and cancel preserve partial evidence and logs never expose secrets, cookies, tokens, or machine-local paths.
- Verification: tests for timeout, cancel, sanitized error, partial evidence retention, and secret/path redaction.
- Depends on: MT-046, MT-047, Diagnostics cancellation/log MTs.

### MT-049 - Sidecar And Identity Event Families
- Objective: Emit EventLedger/Flight Recorder events for sidecar export and identity profile actions only.
- Inputs: repaired stub event requirements for OpenPose sidecars and identity profiles.
- Work slice: define/implement events for OpenPose JSON export, control PNG export, sidecar link/update, identity profile create/update, crop attach/update, and identity lineage inspect.
- Outputs: sidecar/identity event schemas and event-emitting tests.
- Acceptance: sidecar and identity mutations are observable, attributable, and source-linked.
- Verification: tests assert event kind and minimum payload for sidecar export/link and identity profile/crop events.
- Depends on: MT-011 through MT-020.

### MT-050 - ComfyUI Replay And Registration Event Families
- Objective: Emit EventLedger/Flight Recorder events for ComfyUI workflow run, replay, result, and registration failure only.
- Inputs: repaired stub event requirements for workflow receipts, replay, output registration, and failure recovery.
- Work slice: define/implement events for ComfyUI queue, run start, run result, replay requested, replay completed/failed, output registration success, and output registration failure recovery.
- Outputs: ComfyUI/replay/registration event schemas and event-emitting tests.
- Acceptance: every external workflow action and recovery path is observable without chat history.
- Verification: tests assert event kind and minimum payload for queue/run/result/replay/registration-failure events.
- Depends on: MT-021 through MT-028, MT-046 through MT-048.

### MT-051 - Pose Activation Refinement Closure
- Objective: Convert the repaired stub plus this suite into activation-ready refinement content.
- Inputs: source matrix, anchor map, MT suite, fixtures, red-team controls.
- Work slice: write Pose/ComfyUI refinement section with scope, non-goals, dependencies, risks, acceptance, and MT plan.
- Outputs: refinement draft ready for operator signature.
- Acceptance: blocked/planned rows are explicit; no MT depends on unverified product anchors without a blocker.
- Verification: packet/refinement contract check selected at activation.
- Depends on: MT-001 through MT-050.

</topic>
