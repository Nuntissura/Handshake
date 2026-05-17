# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- This stub is planning-only. It authorizes no product code changes.

---

# Work Packet Stub: WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1

## STUB_METADATA
- WP_ID: WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1
- BASE_WP_ID: WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline
- CREATED_AT: 2026-05-16T05:05:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- STUB_FORMAT_VERSION: 2026-04-06
- BUILD_ORDER_DOMAIN: ATELIER_LENS
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: CRITICAL
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1, WP-1-Studio-Runtime-Visibility-v1, WP-1-Artifact-System-Foundations-v1
- BUILD_ORDER_BLOCKS: WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- PRODUCT_REFERENCE: .GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md
- SOURCE_GREENROOM_ROOT: .GOV/reference/ckc_atelier_lens_consolidation
- SOURCE_CKC_CODE: D:/Projects/LLM projects/CastKit-Codex/CKC_main
- SOURCE_CKC_SPEC: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/spec/CastKit_Codex_Spec_v00.075.md
- SOURCE_CKC_TASKBOARD: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/taskboard/TASK_BOARD.md
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_CONTROL_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl
- SESSION_CONTROL_RESULTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl
- SESSION_COMPATIBILITY_SURFACE: VSCODE_PLUGIN_REPAIR_ONLY
- SESSION_COMPATIBILITY_QUEUE_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.5
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.4
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- PLANNED_EXECUTION_OWNER_RANGE: Coder-A..Coder-Z
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH

## INTENT (DRAFT)
- What: Plan the Handshake-native PoseKit/OpenPose/identity-profile/ComfyUI lineage pipeline for Atelier/Lens. This folds CKC PoseKit, OpenPose sidecars, identity profiles, ComfyUI output registration, workflow receipts, replay, and image-sourcing registry into Handshake primitives.
- Why: CKC evolved the production loop from character/media to pose/identity to ComfyUI output and back into media lineage. Handshake needs that loop, but implemented through governed Workflow Engine jobs, ArtifactStore receipts, EventLedger lineage, and Tauri/Rust/Python boundaries.
- No-code stance: This stub creates only future work scope and microtasks. It does not implement PoseKit, ComfyUI bridge, model assets, Python adapters, Rust services, tests, or GUI.
- Model/execution separation: LLMs must not directly run ComfyUI, write bridge files, mutate pose sidecars, or execute local endpoints. LLMs propose or invoke governed AI Jobs/tool calls; Handshake executes via Workflow Engine/mechanical tool adapters with Flight Recorder and DCC evidence.

## SOURCE_COVERAGE_STATUS (DRAFT)
- Coverage audit: `.GOV/reference/ckc_atelier_lens_consolidation/wp-stub-coverage-audit-20260516.md`.
- This stub owns pose, OpenPose, identity-profile, ComfyUI workflow lineage, workflow registry, image-sourcing adapter, and generation replay requirements.
- This stub depends on the Core/Data stub for character identity, media asset identity, sidecar projection law, artifact manifests, collections/contact sheets, search, and export/backup contracts.
- This stub must remain planning-only. It does not implement PoseKit, ComfyUI bridge code, custom nodes, Python adapters, Rust services, tests, or GUI.
- Repair status: this stub was audited before microtask generation and needed more concrete contract detail. Activation must preserve the payload below before any official MT files are created; blocked/planned CKC PoseKit work must stay marked unresolved or deferred instead of being represented as delivered parity.

## FOLDED_SOURCE_STUBS_FULL_PAYLOAD (DRAFT)
- `WP-1-Photo-Studio-v2`
  - Handling here: source dependency for image production workflows, thumbnails, recipes, media review, and identity/pose surfaces.
  - Preserved payload consumed by this stub: recipe persistence/use, image viewer context, thumbnail/preview expectations, skeleton surface pressure, and media-to-generation loop pressure. If the prior `WP-1-Photo-Studio` packet is available, activation must inspect it for recipe record/apply/replay intent and thumbnail/proxy lifecycle before defining this pipeline.
  - Owner boundary: Core owns DAM/media records; this stub owns pose/identity/ComfyUI lineage that consumes those records.
- `WP-1-Stage-Media-Artifact-Portability-v1`
  - Handling here: foundational dependency for generated output materialization and replayable provenance.
  - Preserved payload consumed by this stub: portable artifact manifests, bundle indexes, bounded export anchors, retention, source hashes, and storage-portable evidence.
  - Owner boundary: Core owns the shared manifest contract; this stub requires generated images, OpenPose sidecars, workflow JSON, identity crops, and replay receipts to use it.
- `WP-1-Stage-ASR-Transcript-Lineage-v1`
  - Handling here: pattern dependency for media-to-derived-artifact lineage.
  - Preserved payload consumed by this stub: source media -> derived artifact -> searchable consumer lineage, stable source hash, timing/probe/provenance facts.
  - Owner boundary: this stub applies the same lineage discipline to source image -> pose/identity/workflow -> output image.
- `WP-1-Studio-Runtime-Visibility-v1`
  - Handling here: runtime visibility dependency.
  - Preserved payload consumed by this stub: Studio jobs, workflow nodes, tool surfaces, DCC/operator projection, Flight Recorder linkage, Locus/task-board/WP linkage, and PostgreSQL-only state.
  - Owner boundary: Diagnostics owns model-facing projections; this stub requires pose/ComfyUI workflow state to be visible through those projections.
- `WP-1-Artifact-System-Foundations-v1`
  - Handling here: foundational dependency.
  - Preserved payload consumed by this stub: ArtifactStore manifests, SHA-256, atomic Materialize API, retention/pinning/GC, no random filesystem side effects.
  - Owner boundary: all pose sidecars, workflow receipts, generated outputs, replay bundles, and identity profile media references must materialize through ArtifactStore contracts.
- `WP-1-Lens-ViewMode-v1`
  - Handling here: projection baseline.
  - Preserved payload consumed by this stub: sidecars and generation-support files are hidden from normal galleries unless a projection explicitly asks for them; toggling projections must not mutate raw/derived artifacts.
- `WP-1-Lens-Extraction-Tier-v1`
  - Handling here: extraction/search control baseline.
  - Preserved payload consumed by this stub: generated and pose-derived content must expose requested/effective extraction tier where it affects search or model extraction.
- `WP-1-Product-Screenshot-Visual-Validation-v1` and `WP-1-Visual-Debugging-Loop-v1`
  - Handling here: inherited validation requirements through Diagnostics/Kernel visual evidence.
  - Preserved payload consumed by this stub: future pose overlays, sidecar previews, ComfyUI output review, and replay projections need quiet screenshot/visual evidence when GUI exists.

## CKC_GREENROOM_PAYLOAD_OWNED (DRAFT)
- `EVOL-008` PoseKit blank/single/collection workbench contexts.
  - Source evidence: `App.tsx` pose context state, `PoseView.tsx`, `getPoseKitState`, CKC taskboard `WP-0131`.
  - Requirement: preserve blank, single-image, character-linked, and collection-linked contexts; context switching must not delete stored rigs, source media, or collection links.
  - Handling: folded as a domain/state requirement. Full UI workbench implementation can be deferred, but the data contract must preserve context kind, source image selection, collection reference, active rig IDs, sidecar strip contents, route-state persistence, and independent source/OpenPose strips from CKC `WP-0131`.
- `EVOL-009` Body-18, face-70, hand-21 OpenPose-compatible rig contract.
  - Source evidence: `src/posekit/core.mjs`, `core.d.mts`, `poseDetection.worker.ts`, `posekit_core.test.js`, `hand_openpose_export.test.js`.
  - Requirement: preserve typed OpenPose body/face/hand shape, confidence/provenance needs, serialization behavior, zero-filled/missing-keypoint rules, detection-provider metadata, source-image dimensions, image bitmap failure fallback behavior, and compatibility test cases; CKC JS code is evidence, not runtime code to copy.
  - Minimum data shape to carry into activation: rig id, character id, source image id, optional collection id, provider, provider model/version, body-18 array, face-70 array, handLeft-21 array, handRight-21 array, confidence per point when available, source image width/height, calibration JSON, created/updated timestamps, and provenance refs.
- `EVOL-010` quaternion-backed yaw/pitch/roll head pose.
  - Source evidence: `src/posekit/core.mjs`, `setRigHeadPose`, CKC taskboard `WP-0110`.
  - Requirement: preserve intrinsic YXZ order, angle ranges, quaternion-backed intent, legacy yaw compatibility where applicable, reset/default behavior, and OpenPose serialization/rendering effects.
- `EVOL-011` identity profiles for face-reference workflows.
  - Source evidence: `IdentityProfile` table/CRUD and identity crop/manifest logic in `library.js`, CKC taskboard `WP-0111`.
  - Requirement: preserve stable identity references, face/reference media links, 512x512 crop intent, landmarks/measurements/pose metadata when available, versioning, artifact refs, lineage, workflow payload references, and privacy/path-leak rules.
- `EVOL-012` multi-rig workspace tabs.
  - Source evidence: `openRigWorkspace`, `listOpenRigs`, `reorderOpenRigWorkspaces`, CKC taskboard `WP-0115`.
  - Requirement: preserve session/open-tab semantics, active rig state, close-without-delete behavior, dirty calibration indicators, per-tab panel state, save-before-switch/close behavior, keyboard navigation expectation, and backend reorder support. Activation may defer GUI tabs, but the Handshake state model must be able to represent this behavior without deleting Rig rows or image assets.
- `EVOL-013` ComfyUI output registration and replay.
  - Source evidence: `comfyui_node/castkit_codex_bridge.py`, `registerComfyUIOutput`, `getWorkflowHistory`, `replayWorkflow`, CKC taskboard `WP-0109`.
  - Requirement: preserve workflow receipt, prompt/workflow JSON, extracted prompt text, output image registration, workflow history, replay, stats, vanilla `SaveImage` output discovery fallback, identity reference payloads, pose/openpose refs, wait-for-completion polling behavior, and non-fatal bridge failure posture.
  - Runtime adaptation: reject CKC localhost intake as authority; registration becomes a typed Handshake integration/event/artifact proposal path. Local HTTP may exist only as a capability-gated adapter detail, never as product truth or LLM bypass.
  - Minimum receipt fields to carry into activation: receipt id, external system id, external prompt/run id, workflow spec version, workflow JSON artifact ref, extracted prompt artifact/ref, source character id, sheet version id when available, source image ids, rig ids, OpenPose sidecar refs, identity profile refs, output image artifact refs, registration status, error code/message, replayable inputs, created/imported timestamps, and EventLedger/Flight Recorder refs.
- `EVOL-014` workflow spec registry and image-sourcing adapter.
  - Source evidence: `workflowSpecRegistry.js`, `imageSourcingAdapter.js`, `imageSourcingHandlers/v00_19.js`, CKC taskboard `WP-0100`.
  - Requirement: preserve versioned workflow specs, read-only external spec roots, handler routing by `spec_version`, pinned compatibility concepts, idempotency, source-to-output mapping, accepted/pending/rejected lanes, ingestion audit rows, and future handler slot semantics.
  - Runtime adaptation: route through Handshake Workflow Engine registry and non-SQLite parity fixtures.
- `EVOL-015` identity-decoupled media filenames.
  - Source evidence: `backend_identity_decoupling.test.js`, `ingestImageSourcingTask`, content-hash naming.
  - Requirement here: identity profile payloads and ComfyUI output paths must not leak character names or sensitive sheet fields; use no-space content-addressed artifact names.
- `EVOL-026` blocked PoseKit calibration/history debt.
  - Source evidence: CKC taskboard `WP-0133`.
  - Requirement: preserve as real future requirement evidence, not solved scope: draggable calibration overlay, missing-marker placement flow, 3D/live split editing, forked history, History tab semantics, marker color/hand rows, and sidecar/history visibility.
  - Handling: explicit deferred/decision row; implementation must not claim parity until activated.
- `EVOL-026A` planned PoseKit carry-forward items.
  - Source evidence: CKC taskboard `WP-0112`, `WP-0114`, `WP-0116`, `WP-0117`.
  - Requirement: preserve planned but not delivered CKC work as future requirements: multi-subject rigs and OpenPose `people[]`, multi-file drag-drop import, multi-angle batch export, clear workspace command, synchronized viewport zoom, import existing OpenPose JSON, extended keyboard shortcuts, and stylized/anime/painted portrait landmark-detector research/router. These rows must not block the first pipeline unless the operator explicitly activates them, but they must remain visible so later parity work does not rediscover them.

## GREENROOM_OVERLAP_ROWS_OWNED (DRAFT)
- `OVR-005` sidecars, versioning, recovery: this stub owns production/schema of OpenPose sidecars and generated workflow sidecars; Core owns projection/recovery law.
- `OVR-006` PoseKit / OpenPose / identity: preserve blank/single/collection modes, identity profile lineage, deterministic OpenPose sidecars, multi-rig tabs, and calibration/history debt.
- `OVR-007` ComfyUI workflow lineage: preserve workflow receipts, prompt extraction, replay, stats, identity reference payloads, output image registration, non-fatal bridge failure posture; replace localhost authority with typed Handshake integration/event flow.
- `OVR-011` exports/backups/share packs: this stub consumes export/share contracts for generated output handoff and replay bundles; Core owns export system contract.
- `OVR-012` parallel editing/event log/revisions: this stub emits pose/workflow events and optimistic revision requirements; Diagnostics owns parallel model coordination and model session projection.

## HANDSHAKE_TRANSLATION (DRAFT)
- Module boundary candidates: `atelier_posekit`, `atelier_comfy`, `atelier_sidecars`, `atelier_media`, `atelier_exports`, `atelier_automation`, and `kernel_event_bridge`.
- CKC `comfyui_node/castkit_codex_bridge.py` becomes evidence for a Handshake-named integration schema, not a copied product namespace.
- CKC localhost intake authority becomes a governed Handshake integration/job receipt path. Local HTTP may be an adapter detail only if capability-gated, not product authority, not the source of truth, and not used as an LLM bypass.
- CKC Electron IPC becomes Tauri command facade plus Rust backend service/API contracts.
- CKC JS PoseKit core becomes source evidence for Rust/Python/TypeScript domain contracts selected during refinement; CKC source is not copied as runtime authority.
- ComfyUI execution belongs behind Workflow Engine and mechanical tool adapter boundaries with capability gates, consent where required, Flight Recorder logging, result capture, and ArtifactStore materialization.
- Python orchestration may drive model/tool workflows but cannot hold durable product authority or persistent source of truth.
- PostgreSQL/EventLedger authority owns pose, identity, workflow receipt, output registration, replay, and sidecar lineage facts.
- ArtifactStore owns generated images, OpenPose JSON/PNG, workflow JSON, identity crop/reference media, replay bundles, and materialized handoff outputs.
- SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Product outputs must not be written under `.GOV`; product artifacts must use product artifact roots with manifests and retention.
- Runtime namespace must be Handshake/Atelier/Lens, not CKC/CastKit.
- External tool posture: ComfyUI, MediaPipe/DWPose or other pose detectors, ffmpeg/image tools, and model assets are mechanical/external dependencies. Activation must define capability gates, version discovery/pinning, failure recording, timeout/cancel behavior, and offline/no-model fallback behavior before implementation.
- Generated output preservation: output images are saved/materialized first; registration failure must never delete or orphan the output silently. Failed registration creates a recoverable diagnostic record with output path/artifact ref, attempted receipt payload, error code, and retry instructions.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Pose artifact contract: source image, rig, body/face/hand keypoints, head pose, calibration, sidecar visibility, provenance.
  - Pose workbench contexts: blank, single image, character-linked, collection-linked.
  - Identity profile contract: face/reference media, source links, versioning, provenance, privacy/path rules.
  - OpenPose JSON/PNG sidecar manifest and projection rules.
  - ComfyUI workflow receipt: prompt/workflow JSON, output images, identity/pose/media refs, non-fatal registration failure handling.
  - Workflow spec registry and image-sourcing adapter translation into Handshake Workflow Engine.
  - Replay and lineage requirements for generation outputs.
  - Mechanical tool adapter boundary for ComfyUI.
  - Concrete Rig, OpenPose sidecar, IdentityProfile, WorkflowReceipt, WorkflowSpec, ImageSourcingTask, and replay receipt field requirements sufficient for no-context implementation planning.
  - Explicit evidence maturity for CKC PoseKit statuses: DONE rows seed parity fixtures, REVIEW rows need evidence review, BLOCKED rows remain unresolved, PLANNED rows remain future/deferred.
- OUT_OF_SCOPE:
  - Full GUI workbench or dockable shell.
  - Full CKC parity.
  - Direct CKC bridge namespace or CastKit product naming.
  - CKC SQLite, Electron, localhost-intake authority, or process-local session authority.
  - Advanced blocked PoseKit calibration/history implementation unless explicitly activated as a later packet.
  - Treating CKC's localhost intake server, Electron IPC, JavaScript PoseKit implementation, or CKC namespace as runtime authority.

## ACCEPTANCE_CRITERIA (DRAFT)
- Every CKC PoseKit/ComfyUI feature is assigned to implemented-now, deferred, dependency, or operator-decision-needed scope.
- Every owned CKC EVOL row from `EVOL-008` through `EVOL-014` plus `EVOL-015` and `EVOL-026` is represented with source evidence, preserved behavior, and Handshake adaptation.
- Every owned overlap row has an owner boundary and required handling.
- Source dependencies on Core Data/Intake, Studio Runtime Visibility, Artifact System Foundations, Lens ViewMode, Lens Extraction Tier, and visual validation are visible without reopening the greenroom.
- Future implementation path uses Handshake AI Job/Workflow Engine/mechanical tool adapter boundaries, not direct LLM execution.
- Every generated or imported output has ArtifactStore and EventLedger lineage.
- Sidecars and identity refs are preserved without cluttering normal media projections.
- ComfyUI registration failures preserve source output and create diagnosable evidence.
- Rig/OpenPose/identity/workflow contracts are concrete enough for a no-context model to draft tables/types and fixtures: body-18, face-70, handLeft/handRight-21, headPose YXZ/quaternion data, calibration JSON, provider/version metadata, source image refs, identity crop/profile refs, OpenPose PNG/JSON artifacts, workflow JSON, extracted prompt, replayable inputs, output artifact refs, status, errors, timestamps, and evidence refs are all named.
- The pipeline preserves both CKC DONE and unresolved/deferred PoseKit work: `WP-0115` multi-rig semantics are represented, `WP-0133` calibration/history remains blocked/unresolved, and planned rows `WP-0112`, `WP-0114`, `WP-0116`, `WP-0117` are visible as future/deferred requirements.
- Workflow spec registry and image-sourcing adapter behavior is preserved as versioned, idempotent, handler-routed, non-SQLite Workflow Engine requirements with accepted/pending/rejected lanes and audit evidence.
- External tool and model-asset risks have controls: capability gates, allowlists, version discovery/pinning, timeout/cancel, no secret leakage, output-first registration, and recoverable failure records.
- SQLite is rejected in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Electron authority, CKC namespace authority, localhost intake authority, `.GOV` product outputs, machine-local runtime paths, and direct LLM execution are explicitly rejected.

## MICROTASKS (DRAFT)

- Draft MT authority: non-executable planning only; official MT files/contracts are still not generated.
- Replacement rule: the earlier 20-item draft MT list is retired and must not be reused as source for activation.
- DRAFT_MICROTASK_SUITE_PATH: .GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1/MT_SUITE.md
- DRAFT_MICROTASK_COUNT: 51
- OFFICIAL_MICROTASKS_GENERATED: false
- DRAFT_MICROTASK_ACTIVATION_DESTINATION_PATTERN: .GOV/task_packets/<WP_ID>/MT-*.{md,json}
- Fresh no-context MT suite: `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1/MT_SUITE.md`.
- Draft MT count: 51.
- Granularity rule: Activation Manager must split any MT further if it touches unrelated files, crosses owner boundaries, or cannot be executed by a no-context local/small cloud model from that MT alone.
- Activation rule: convert these draft MTs into official `.GOV/task_packets/<WP_ID>/MT-*.json` and `.md` only after refinement, USER_SIGNATURE, and official packet creation.

## RISKS / UNKNOWNs (DRAFT)
- Risk: CKC ComfyUI localhost bridge is copied as authority. Mitigation: MT-010 and MT-011 force Workflow Engine/Tauri/Rust boundaries.
- Risk: LLM executes ComfyUI directly. Mitigation: MT-010 requires capability-gated tool adapter and FR evidence.
- Risk: blocked PoseKit calibration is represented as done. Mitigation: MT-006 and MT-020 preserve it as deferred/decision work.
- Risk: generated outputs lose lineage. Mitigation: MT-013 through MT-015 require ArtifactStore/EventLedger lineage.
- Risk: output image exists but registration fails and later models cannot recover it. Mitigation: output-first preservation plus failed-registration diagnostic record with retryable receipt payload.
- Risk: concrete pose schema gets hand-waved as "OpenPose-compatible". Mitigation: activation acceptance names body-18, face-70, handLeft/handRight-21, headPose, calibration, provider metadata, source refs, and sidecar artifacts.
- Risk: CKC DONE/REVIEW/BLOCKED/PLANNED PoseKit rows collapse into a single parity claim. Mitigation: status maturity is acceptance-critical and blocked/planned rows cannot be counted as implemented.
- Risk: external model/tool versions drift. Mitigation: activation must decide version discovery/pinning, local model asset strategy, fallback behavior, and capability policy.
- Risk: UI scope expands. Mitigation: visual validation requirements are projections only, full GUI out of scope.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm operator accepts this as one of no more than three CKC fold-in WP stubs.
- [ ] Verify product worktree anchors for Studio, ArtifactStore, Workflow Engine, Tauri, Rust backend, Python orchestration, and media surfaces.
- [ ] Produce Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE.
- [ ] Create official task packet and MT files.
- [ ] Keep SQLite forbidden everywhere.
