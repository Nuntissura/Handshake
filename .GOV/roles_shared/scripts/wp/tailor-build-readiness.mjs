#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const WP_ID = "WP-KERNEL-010-Tailor-Cloth-Garment-Engine-v1";
const ROOT = process.cwd();
const PACKET_DIR = path.join(ROOT, ".GOV", "task_packets", WP_ID);
const STUB_PATH = path.join(ROOT, ".GOV", "task_packets", "stubs", `${WP_ID}.contract.json`);
const INDEX_PATH = path.join(PACKET_DIR, "_MT_INDEX.json");
const UPDATED_AT = "2026-08-01T04:26:38.699Z";
const ORIGINAL_COUNT = 782;
const FINAL_COUNT = 842;
const ACTIVE_SPEC_VERSION = "v02.203";
const ACTIVE_SPEC_ENTRYPOINT = ".GOV/spec/master-spec-v02.203/indexed-spec-manifest.json";
const RECONCILIATION_STATUS = "SPEC_V02_203_RECONCILED_BODYKIT_V2_DAG_CANDIDATE";
const GENERATOR_PATH = ".GOV/roles_shared/scripts/wp/tailor-build-readiness.mjs";
const REVIEW_PATH = `.GOV/task_packets/${WP_ID}/_MULTI_LENS_REVIEW.json`;
const PARITY_V3_PATH = `.GOV/task_packets/${WP_ID}/_PARITY_REVIEW_V3.json`;
const BUILD_READY_PATH = `.GOV/task_packets/${WP_ID}/${WP_ID}.build-readiness-prework.json`;
const TECHNICAL_REFINEMENT_PATH = `.GOV/task_packets/${WP_ID}/${WP_ID}.technical-refinement-prework.json`;

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function idNumber(mtId) {
  return Number(mtId.slice(3));
}

function mtId(number) {
  return `MT-${String(number).padStart(3, "0")}`;
}

function unique(values) {
  return [...new Set(values.filter(Boolean))];
}

function addUnique(array, value) {
  if (!array.includes(value)) array.push(value);
}

function addCriteria(mt, criterion) {
  if (!mt.scope.acceptance_criteria.includes(criterion)) {
    mt.scope.acceptance_criteria.push(criterion);
  }
}

function replaceOrAddCriteria(mt, marker, criterion) {
  const index = mt.scope.acceptance_criteria.findIndex((item) => item.includes(marker));
  if (index >= 0) mt.scope.acceptance_criteria[index] = criterion;
  else mt.scope.acceptance_criteria.push(criterion);
}

const nonUiProducerIds = new Set([
  "MT-018", "MT-022", "MT-023", "MT-041", "MT-053", "MT-057", "MT-058", "MT-068", "MT-069",
  "MT-115", "MT-116", "MT-118", "MT-119", "MT-120", "MT-124", "MT-125", "MT-128", "MT-130",
  "MT-132", "MT-135", "MT-152", "MT-161", "MT-176", "MT-177", "MT-182", "MT-197", "MT-212",
  "MT-213", "MT-214", "MT-226", "MT-234", "MT-238", "MT-239", "MT-244", "MT-245", "MT-259",
  "MT-262", "MT-263", "MT-279", "MT-287", "MT-288", "MT-289", "MT-292", "MT-301", "MT-304",
  "MT-333", "MT-344", "MT-355", "MT-362", "MT-368", "MT-371", "MT-375", "MT-379", "MT-382",
  "MT-397", "MT-400", "MT-401", "MT-405", "MT-412", "MT-415", "MT-416", "MT-417", "MT-431",
  "MT-443", "MT-462", "MT-465", "MT-045", "MT-117", "MT-133", "MT-236"
]);

const guiNeedsNativeProof = new Set([
  "MT-335", "MT-336", "MT-338", "MT-339", "MT-340", "MT-341", "MT-342", "MT-343", "MT-345",
  "MT-347", "MT-348", "MT-349", "MT-353", "MT-356", "MT-357", "MT-358", "MT-361", "MT-364",
  "MT-365", "MT-372", "MT-383", "MT-388", "MT-389", "MT-408", "MT-409", "MT-413", "MT-414",
  "MT-420", "MT-424", "MT-432", "MT-433", "MT-434", "MT-436"
]);

const bodyKitJobProducers = new Set([
  "MT-513", "MT-530", "MT-531", "MT-542", "MT-543", "MT-544", "MT-573", "MT-586", "MT-591",
  "MT-592", "MT-599", "MT-602", "MT-619", "MT-624", "MT-625", "MT-641", "MT-642", "MT-643",
  "MT-644", "MT-645", "MT-653"
]);

const bodyKitGui = {
  "MT-626": {
    surfaces: ["BodyKit editor pane", "BodyKit region tree", "BodyKit viewport and inspector shell"],
    targets: ["tailor-bodykit-pane", "tailor-bodykit-tab", "tailor-bodykit-body-selector", "tailor-bodykit-region-tree", "tailor-bodykit-viewport", "tailor-bodykit-inspector", "tailor-bodykit-job-status", "tailor-bodykit-problems"]
  },
  "MT-627": {
    surfaces: ["BodyKit channel and region panels"],
    targets: ["tailor-bodykit-channels", "tailor-bodykit-region-{region_id}", "tailor-bodykit-channel-{channel_id}-slider", "tailor-bodykit-channel-{channel_id}-value", "tailor-bodykit-channel-{channel_id}-overrange", "tailor-bodykit-region-{region_id}-isolate"]
  },
  "MT-628": {
    surfaces: ["BodyKit recipe and archetype browser"],
    targets: ["tailor-bodykit-recipes", "tailor-bodykit-recipe-search", "tailor-bodykit-recipe-{recipe_id}-card", "tailor-bodykit-recipe-{recipe_id}-apply", "tailor-bodykit-recipe-partial-scope", "tailor-bodykit-recipe-import", "tailor-bodykit-recipe-compatibility-status"]
  },
  "MT-629": {
    surfaces: ["BodyKit measurement and target solver panel"],
    targets: ["tailor-bodykit-measurements", "tailor-bodykit-measurement-{measurement_id}-value", "tailor-bodykit-measurement-{measurement_id}-target", "tailor-bodykit-measurement-{measurement_id}-residual", "tailor-bodykit-measurement-solve-status", "tailor-bodykit-measurement-cancel"]
  },
  "MT-630": {
    surfaces: ["BodyKit coupling graph inspector"],
    targets: ["tailor-bodykit-coupling-graph", "tailor-bodykit-coupling-{link_id}-row", "tailor-bodykit-coupling-{link_id}-disable", "tailor-bodykit-coupling-create", "tailor-bodykit-coupling-source", "tailor-bodykit-coupling-target", "tailor-bodykit-coupling-parity-status"]
  },
  "MT-632": {
    surfaces: ["BodyKit genital configuration and arousal panel"],
    targets: ["tailor-bodykit-genitals", "tailor-bodykit-vulva-{channel_id}", "tailor-bodykit-penis-{channel_id}", "tailor-bodykit-scrotum-{channel_id}", "tailor-bodykit-anus-{channel_id}", "tailor-bodykit-erection-state", "tailor-bodykit-gape-state", "tailor-bodykit-arousal-state", "tailor-bodykit-genital-reference-compare"]
  },
  "MT-633": {
    surfaces: ["BodyKit shared wgpu viewport and validation overlays"],
    targets: ["tailor-bodykit-viewport", "tailor-bodykit-viewport-shading", "tailor-bodykit-overlay-mode", "tailor-bodykit-overlay-bone", "tailor-bodykit-overlay-legend", "tailor-bodykit-capture-view", "tailor-bodykit-viewport-status"]
  },
  "MT-634": {
    surfaces: ["BodyKit Argus visual matrix runner and evidence browser"],
    targets: ["tailor-bodykit-argus-matrix", "tailor-bodykit-argus-cell-{cell_id}", "tailor-bodykit-argus-run", "tailor-bodykit-argus-cancel", "tailor-bodykit-argus-result-{cell_id}", "tailor-bodykit-argus-console-status", "tailor-bodykit-argus-layout-status"]
  },
  "MT-635": {
    surfaces: ["BodyKit no-context GUI-operation reference flow"],
    targets: ["tailor-bodykit-pane", "tailor-bodykit-channel-breast-volume-slider", "tailor-bodykit-channel-breast-volume-value", "tailor-bodykit-viewport", "tailor-bodykit-capture-view", "tailor-bodykit-e2e-status"]
  }
};

const chapterByGroup = {
  SolverCore: "Cloth simulation and physics",
  Collision: "Cloth collision, contact, and repair",
  PhysicsHardening: "Cloth final-quality simulation and diagnostics",
  KernelIntegration: "Projects, authority, jobs, and recovery",
  GarmentAuthoring: "Pattern authoring, sewing, and arrangement",
  ValidationHBR: "Validation, diagnostics, and visual inspection",
  TrimRigid: "Trims, closures, and rigid attachments",
  Fabric: "Fabrics and physical calibration",
  UvTexture: "UVs, graphics, and materials",
  RenderViewportExport: "Viewport, look development, and export",
  ModelFirstApi: "Model and agent operation",
  Animation: "Animation, simulation caches, and recording",
  AutoFit: "Body fitting, sizing, and garment transfer",
  ProductionBridge: "Publishing and DCC interchange",
  BkFoundation: "BodyKit projects and base assets",
  BkChannels: "BodyKit shape channels and measurements",
  BkSkeleton: "BodyKit skeleton and rigging",
  BkCorrectives: "BodyKit morphs and correctives",
  BkSoftTissue: "BodyKit soft tissue and physical response",
  BkGenitals: "BodyKit genital anatomy and controls",
  BkSkin: "BodyKit skin, UVs, and materials",
  BkFace: "BodyKit face, expressions, and speech",
  BkClothBridge: "BodyKit, Cloth, and contact integration",
  BkModelApi: "BodyKit model and agent operation",
  BkExport: "BodyKit publishing and DCC interchange",
  BkGui: "BodyKit workspace and controls",
  BkBodiesQA: "BodyKit quality assurance and references",
  BkGovernance: "BodyKit jobs, diagnostics, and recovery",
  NativeProductionUX: "Tailor workspace and controls",
  PoseKitCharacterInterop: "PoseKit and character-sheet interlinking",
  ProfessionalProductionHardening: "Professional production workflows",
  BuildReadinessPlatform: "Model operation, diagnostics, pillars, and manual",
  ProfessionalClothUX: "Professional Cloth authoring workflows",
  BodyKitProduction: "Professional BodyKit production workflows",
  DccParityQualification: "DCC parity and qualification",
  NativeRuntimeSupply: "Offline dependencies and native runtime",
  BodyProviderRuntime: "Body providers, high-resolution heads, and rig evaluation",
  CinematicPerformance: "Performance capture, cinematic appearance, and conditioning",
  InterchangeDelivery: "Blender, Unreal, and canonical interchange",
  NativeOperatorSurfaces: "Professional Tailor workspaces and tools",
  ProjectPillars: "Projects, privacy, parallel work, and shared pillars",
  CinematicQualification: "Movie-production qualification and recovery"
};

function classify(mt) {
  const text = `${mt.scope.summary} ${(mt.scope.allowed_paths || []).join(" ")}`.toLowerCase();
  const frontend = text.includes("handshake_native") || text.includes("src/frontend/");
  const solverOnly = (mt.scope.allowed_paths || []).some((item) => item.startsWith("tailor-solver/")) &&
    !(mt.scope.allowed_paths || []).some((item) => item.includes("handshake_core") || item.includes("frontend"));
  const ui = Boolean(mt.gui_obligation?.gui_creation_required);
  const job = /\b(job|jobs|scheduler|queue|batch|bake|long-running|simulation run|render queue)\b/.test(text);
  const dcc = /\b(dcc|daz|marvelous|blender|unreal|usd|usdz|fbx|alembic|gltf|dson|sbsar|substance)\b/.test(text);
  const artifact = /\b(asset|artifact|export|capture|cache|texture|material|package|report|render|file|manifest|image)\b/.test(text);
  const command = /\b(api|command|tool|editor|panel|workspace|browser|create|edit|delete|apply|import|export|publish|run|configure|operation)\b/.test(text);
  const authority = !solverOnly && (/handshake_core|eventledger|authority|database|migration|persist|crdt|promotion|registry|publish/.test(text) || command || job);
  const visual = ui || /viewport|visual|capture|overlay|render|heatmap|reference image/.test(text);
  const diagnostics = job || dcc || frontend || /device|gpu|wgpu|diagnostic|recovery|fault|overflow|cancel|process|bridge/.test(text);
  return { ui, frontend, solverOnly, job, dcc, artifact, command, authority, visual, diagnostics };
}

function hbrObligations(classification) {
  const obligations = ["HBR-INT", "HBR-MAN", "HBR-PRIV"];
  if (classification.command || classification.authority || classification.job || classification.ui) obligations.push("HBR-SWARM");
  if (classification.visual || classification.artifact || classification.solverOnly) obligations.push("HBR-VIS");
  if (classification.ui || classification.job || classification.dcc) obligations.push("HBR-QUIET");
  if (classification.command || classification.job || classification.dcc || classification.frontend) obligations.push("HBR-STOP");
  return unique(obligations);
}

function diagnosticTiers(classification) {
  let flightRecorder = "NOT_APPLICABLE";
  if (classification.authority || classification.job || classification.dcc) flightRecorder = "DIRECT";
  else if (classification.ui || classification.command || classification.artifact) flightRecorder = "INHERITED";

  let internalDiagnostics = "NOT_APPLICABLE";
  if (classification.diagnostics) internalDiagnostics = "DIRECT";
  else if (classification.solverOnly || classification.command || classification.authority || classification.artifact) internalDiagnostics = "INHERITED";

  let palmistry = "NOT_APPLICABLE";
  if (classification.job || classification.dcc || classification.frontend) palmistry = "DIRECT";
  else if (classification.authority || classification.command || classification.ui) palmistry = "INHERITED";

  return [
    {
      tier: "flight_recorder",
      posture: flightRecorder,
      reason: flightRecorder === "DIRECT"
        ? "This MT owns authoritative, job, or external-tool lifecycle events and must project the same event_id, run_id, actor_id, and correlation_id from EventLedger into Flight Recorder."
        : flightRecorder === "INHERITED"
          ? "This MT is observed through the canonical command, job, or artifact owner; it must preserve correlation identifiers and must not create a second event authority."
          : "Pure computation or schema-only behavior has no independent lifecycle event; its owning run records inputs, outputs, diagnostics, and receipts."
    },
    {
      tier: "internal_diagnostics",
      posture: internalDiagnostics,
      reason: internalDiagnostics === "DIRECT"
        ? "This MT must wire the shared internal_diagnostics API for its native, job, device, solver-run, or DCC failure modes with bounded structured evidence."
        : internalDiagnostics === "INHERITED"
          ? "This MT emits typed diagnostics through its owning command or job and inherits retention, health, and projection behavior from the shared diagnostics substrate."
          : "No independent runtime or failure surface exists; invalid input is returned as a typed local error and the owning caller provides diagnostics."
    },
    {
      tier: "palmistry",
      posture: palmistry,
      reason: palmistry === "DIRECT"
        ? "Long-running, native, or external-process work must register bounded progress and child/process identity with the shared Palmistry watcher; Tailor must not fork a watcher."
        : palmistry === "INHERITED"
          ? "The owning native shell or scheduled job supplies Palmistry coverage; this MT must preserve operation identity and recovery handles."
          : "This bounded in-process computation has no application or child-process watch surface."
    }
  ];
}

function hardenAcceptance(mt, classification) {
  if (mt.scope.acceptance_criteria.length >= 3) return;

  if (classification.solverOnly) {
    addCriteria(mt, "Typed inputs, outputs, units, tolerances, invalid/non-finite behavior, and deterministic seed/order requirements are explicit; invalid input fails without mutating solver or authority state.");
    addCriteria(mt, "Boundary, degeneracy, cancellation, and repeatability tests prove the behavior through the owning SolverCapture or typed run receipt, including an evidence-bearing failure path.");
  } else if (classification.ui) {
    addCriteria(mt, "Blank, loading, populated, stale/conflicted, cancelled, failed, retry, and permission-denied states are either rendered distinctly or recorded as not applicable in the exact TailorSurfaceDescriptor row.");
    addCriteria(mt, "Keyboard-only focus order, AccessKit labels, non-color state, narrow/4K/DPI layouts, no overlap/clipping, and no focus steal are proven through the registry-derived Argus matrix.");
  } else if (classification.authority || classification.command || classification.job) {
    addCriteria(mt, "The typed operation declares actor/session/correlation ids, expected base version, lease scope, idempotency key, preview/diff/apply semantics, cancellation/retry handles, and stale/conflict/error envelopes where applicable.");
    addCriteria(mt, "Negative, concurrent, partial-failure, retry, and recovery tests prove EventLedger/ArtifactStore effects are atomic, attributable, replayable, and leave no silent authority or orphan-artifact drift.");
  } else {
    addCriteria(mt, "Typed inputs, outputs, preconditions, bounds, unsupported cases, and deterministic error behavior are specified so a no-context implementer does not infer hidden behavior.");
    addCriteria(mt, "A negative or boundary proof demonstrates fail-closed behavior, stable identifiers, and no unrelated state or artifact changes.");
  }

  while (mt.scope.acceptance_criteria.length < 3) {
    addCriteria(mt, "The proof target exercises both the successful path and one named failure path and emits enough structured evidence for a no-context reviewer to reproduce the result.");
  }
}

const nativeSupersessions = {
  "MT-249": "Qualify the Rust-owned deterministic GLB garment export through Khronos validation, independent Rust reload, typed structural comparison, and optional manual DCC inspection.",
  "MT-252": "Replace the legacy Alembic writer scope with Rust-owned time-varying cloth delivery through restricted USD mesh time samples or the canonical fixed-topology cache; unsupported Alembic export is explicit.",
  "MT-253": "Produce a deterministic Blender manual-qualification package containing canonical GLB/USD artifacts, manifest, hashes, operator checklist, expected observations, capture slots, and a structured receipt; do not automate Blender.",
  "MT-254": "Produce the Unreal delivery adapter using canonical GLB/USD artifacts plus an optional pinned native C++ Interchange editor commandlet that returns structured import and qualification receipts.",
  "MT-296": "Wire animation export to deterministic glTF animation, restricted USD time samples, or the canonical native cache for the selected frame range, with capability and loss receipts.",
  "MT-317": "Define the production interchange package—mesh, rig, morphs, UV/textures, OpenPBR/MaterialX projection, trims, animation/cache, camera/light state, manifest and losses—for canonical delivery and optional DCC qualification.",
  "MT-323": "Generate a Rust-owned canonical GLB/USD garment package and Blender manual-qualification checklist/receipt; executable DCC scripts are prohibited.",
  "MT-332": "Prove promoted-garment delivery by Rust structural reload, optional Unreal native commandlet qualification, optional manual Blender evidence, and one complete EventLedger/Flight Recorder trail.",
  "MT-388": "Implement local-only fabric texture generation through the canonical Handshake image-generation capability using pre-acquired hash-locked local models; cloud providers and runtime downloads are excluded.",
  "MT-394": "Export morph-channel animation through deterministic glTF weight tracks and qualified restricted USD/native-cache time samples with Rust reload and timing/name/delta comparison.",
  "MT-415": "Represent named morph-weight animation in restricted USD or the canonical native cache and implement only a separately qualified limited Alembic import adapter; do not claim Alembic export.",
  "MT-418": "Apply named frame-range and marker resolution consistently to GLB animation, restricted USD time samples and native caches, preserving exact sample times and provenance.",
  "MT-419": "Implement dense animated-garment delivery through GLB animation, restricted USD time samples and native caches; FBX export returns a typed unsupported capability until a no-Python writer qualifies.",
  "MT-421": "Enforce the direction-specific interchange capability matrix: validate canonical GLB/USD delivery, return typed unsupported FBX export, allow a future qualified Rust writer, and prohibit hidden DCC execution.",
  "MT-440": "Implement a limited pinned native Alembic/Ogawa import adapter for qualified fixture classes and convert accepted data to canonical GarmentFrame/native-cache state; unsupported properties fail typed.",
  "MT-442": "Implement Rust-owned MDD/PC2 cache export and independent Rust readback, with optional operator-driven DCC inspection recorded separately from automated proof.",
  "MT-536": "Emit a DCC-neutral corrective/driver graph sidecar; Unreal reconstruction uses the optional native C++ commandlet and Blender reconstruction remains a documented manual qualification step.",
  "MT-582": "Export canonical face controls, ARKit/FACS/viseme mappings and morph animation through GLB/restricted USD with Rust structural reload, Unreal native qualification and optional manual Blender evidence.",
  "MT-608": "Implement versioned TailorInterchangeProfile rows for deterministic GLB, restricted USD, native caches, optional Unreal commandlet, manual Blender qualification, import-only FBX/Alembic adapters and typed unsupported directions.",
  "MT-609": "Implement the deterministic Rust GLB skeletal-mesh writer and independent Rust reload for skeleton, weights, morphs, animation, materials, cameras and lights, with explicit target loss reports.",
  "MT-610": "Make deterministic GLB and restricted USD the required BodyKit exports; FBX export is an honest typed unsupported capability until a no-Python writer passes the qualification matrix.",
  "MT-611": "Package corrective shapes and the machine-readable neutral rig-driver graph with every skeletal export; executable DCC scripts are prohibited and reconstruction is adapter/manual-profile specific.",
  "MT-614": "Implement the Unreal profile over canonical GLB/USD plus the optional native C++ Interchange commandlet, including skeleton/IK mapping, units/axes, reference pose, materials, groom and structured losses.",
  "MT-624": "Split interchange proof into Rust structural round-trip, optional Unreal native C++ commandlet qualification, and operator-driven Blender import/inspection receipts; no generated scripts or Python execution.",
  "MT-625": "Run BodyKit archetype export integration over every applicable qualified interchange row, distinguishing required canonical success, optional qualification, manual evidence and typed unsupported directions.",
  "MT-644": "Run the export QA matrix across all archetypes and applicable TailorInterchangeProfile rows with structural tolerances, material/color losses, optional Unreal receipts, manual Blender evidence and honest unsupported cells.",
  "MT-649": "Bind every BodyKit core command to the local process sandbox and blocked-network profile; optional adapters are named local jobs and their removal never bypasses or disables core authoring, render, save or canonical export.",
  "MT-651": "Author the BodyKit UserManual for the v02.203 native runtime, GNM-v3 provider, dependency bundles, project/body/face/anatomy/performance workflows, GLB/restricted USD delivery, optional Unreal, manual Blender and typed unsupported recovery.",
  "MT-653": "Prove the provider-neutral BodyKit lifecycle through registered commands, canonical GLB/restricted USD delivery, EventLedger replay and adapter removal without requiring FBX, Blender automation or cloud services.",
  "MT-709": "Prove the professional Tailor workflow through native project/body/pattern/drape/material/animation/render and canonical interchange, with Rust reload, optional Unreal, manual Blender, recovery and no-context replay.",
  "MT-720": "Qualify BodyPose/interlink units, axes, skeleton, camera, contact and animation through Rust structural fixtures, optional Unreal native commandlet and separately recorded manual Blender inspection.",
  "MT-736": "Implement a restricted pure-Rust USD profile for only qualified layers, references, variants, UsdSkel, materials, curves, caches, cameras and lights, with explicit unsupported schemas and exact target qualification profiles.",
  "MT-737": "Prove the BodyKit creator lifecycle through native body/head/rig/morph/material/anatomy/contact/cloth/render and canonical interchange; optional DCC evidence cannot replace product-native proof.",
  "MT-773": "Create a Rust-only capability-scoped Tailor adapter/plugin SDK with manifests, typed action registration, permissions, ABI/version negotiation, sandboxing, reload/recovery, introspection and deterministic tests; Python adapters are prohibited.",
  "MT-774": "Implement production garment fur/fuzz with native guides/strands/cards, binding, dynamics, render and USD-curves-first interchange; unsupported DCC projections are explicit.",
  "MT-775": "Implement BodyKit strand/groom authoring, binding, dynamics, LOD, native render and USD-curves-first interchange with exact capability and loss receipts.",
  "MT-776": "Implement the provider-neutral BodyPerformanceTrack speech/viseme workflow with local Rust-owned processing and optional local Audio2Face adapter; preserve editable timing, confidence, rig mapping and provenance.",
  "MT-778": "Create the exact-version black-box DCC qualification harness with Rust structural fixtures, Unreal native commandlet automation and manual Blender checklists/captures; executable Python is prohibited.",
  "MT-779": "Execute the feature-by-feature professional qualification matrix over Rust-native workflows, structural interchange, optional Unreal automation, manual Blender evidence, failure/recovery, visual output and performance budgets.",
  "MT-781": "Prove the preserved v02.201 professional Tailor lifecycle without Python, cloud services or mandatory Blender/FBX success; BodyKit-v2 cinematic completion is extended by MT-842.",
  "MT-782": "Lock Tailor implementation and supply inputs: Rust/compiler/crate/model/SDK/config versions and hashes, offline dependency bundle, runtime precedence, profiles/tolerances/fixtures/hardware budgets, adapter identities, DCC profiles and rollback."
};

function applyNativeSupersession(mt) {
  const summary = nativeSupersessions[mt.mt_id];
  if (!summary) return;
  mt.scope.summary = summary;
  const conflict = /python|pytorch|cloud api|blender[- ]bridge|headless blender|--python|generated .*script|fbx[-_ ]bridge|alembic (?:write|export|conversion)|via blender/i;
  mt.scope.acceptance_criteria = (mt.scope.acceptance_criteria || []).filter((item) => !conflict.test(item));
  mt.scope.proof_targets = (mt.scope.proof_targets || []).filter((item) => !/python|\.py\b|blender|fbx_bridge|alembic.*(?:export|roundtrip)/i.test(item));
  mt.scope.allowed_paths = (mt.scope.allowed_paths || []).filter((item) => !/\.py$/i.test(item)).map((item) => item.replace(/bridge_blender\.rs$/i, "dcc/blender_manual_qualification.rs").replace(/fbx_bridge\.rs$/i, "interchange/fbx_capability.rs"));
  addCriteria(mt, "The product path is Rust-owned, local and offline: Python/PyTorch, generated executable scripts, cloud services, implicit network access and runtime downloads are prohibited; optional adapters are pinned and removable.");
  addCriteria(mt, "The active TailorInterchangeProfile or runtime capability row records exact direction, versions, hashes, supported payloads, losses, unsupported cases, privacy behavior, artifacts and qualification receipts before the action is exposed.");
  addCriteria(mt, "Proof includes successful canonical Rust behavior plus one missing/removed-adapter or unsupported-direction case and demonstrates no partial authority mutation, silent fallback, data loss or false qualification.");
  if (mt.scope.proof_targets.length === 0) mt.scope.proof_targets.push(`cargo test -p handshake_core tailor_v02_203_${mt.mt_id.toLowerCase().replace("-", "_")}`);
}

function applySpecificAmendments(mt) {
  const id = mt.mt_id;
  applyNativeSupersession(mt);
  if (["MT-432", "MT-433", "MT-434"].includes(id)) {
    addCriteria(mt, "Fit-map behavior distinguishes Global and Fabric modes, uses each fabric's tensile limits, exposes eight configurable bands, dominant warp/weft/bias direction, actual stretch and force ratio, and supports point probing with numeric non-color output.");
  }
  if (id === "MT-467") {
    addCriteria(mt, "An atomic Swap Front/Back material operation preserves side identity, UV orientation, normals/tangents, undo/replay, export, and command/GUI parity without silently duplicating material authority.");
  }
  if (id === "MT-342") {
    addCriteria(mt, "The 3D Pencil workflow includes freehand eraser, symmetry, front/back constrained conversion, editable sampled points, preview/apply/revert, and exact command and author_id coverage.");
  }
  if (id === "MT-401") {
    addCriteria(mt, "Pattern Drafter edit mode exposes source image, scale/calibration, editable points/curves, constraints, uncertainty, preview, accept/reject, and recovery through the shared command and surface registries.");
  }
  if (id === "MT-381") {
    addCriteria(mt, "Graphic visibility, side, layer, opacity, masking, solo/hide, and export state are explicit in backend and native projections and survive undo, reload, and round-trip export.");
  }
  if (id === "MT-359") {
    addCriteria(mt, "The library exposes docked preview/detail, compatibility and dependency state, favorites, recents, background hydration progress, missing-asset recovery, and exact stable record IDs.");
  }
  if (id === "MT-360" || id === "MT-706") {
    addCriteria(mt, "The shared scene tree includes garments, BodyKit bodies, skeletons, props, cameras, lights, wind actors, contacts, animation layers, material stacks, linked assets, jobs, and validation findings without duplicating their authority.");
  }
  if (id === "MT-632") {
    addUnique(mt.lifecycle.depends_on, "MT-730");
    addUnique(mt.lifecycle.depends_on, "MT-731");
    addCriteria(mt, "The panel exposes every MT-730/MT-731 vulva, penis, scrotum, erection, foreskin, asymmetry, pigmentation, wetness, and interior channel with searchable grouping, authored/effective values, shape-family presets, reference comparison, and exact stable IDs.");
  }
  if (id === "MT-689") {
    mt.scope.summary = "Register Tailor commands as extensions of the canonical KernelActionCatalogV1 and PRIM-CommandCorpusEntryV1, with one descriptor governing native GUI, shortcuts, palette, Argus, MCP/REST, replay, semantic undo, receipts, UserManual generation, and extension commands; no Tailor-only command authority is allowed.";
    addCriteria(mt, "Every mutating descriptor carries actor/session/correlation ids, target and acknowledgement routing, expected base version, lease scope, idempotency key, preview/diff/apply semantics, cancellation/retry handles, typed stale/conflict/error envelopes, EventLedger event ids, and receipt ids.");
  }
  if (["MT-722", "MT-723", "MT-724", "MT-725", "MT-726"].includes(id)) {
    addCriteria(mt, "Every creator operation is registered in the canonical action catalog and is executable identically through typed backend command, MCP, native GUI, replay, and semantic undo with attribution, leases, stale-state rejection, atomic failure, and EventLedger receipts.");
    addCriteria(mt, "Proof includes a backend-only no-context operation test in addition to native/Argus coverage; GUI-only success is insufficient.");
  }
  if (id === "MT-650") {
    mt.lifecycle.depends_on = ["MT-483", "MT-490", "MT-741", "MT-742", "MT-743"];
    mt.scope.summary = "Establish the early shared BodyKit job adapter before any job-producing BodyKit work: generation, correctives, soft-tissue settles, skin bakes, imports, exports, and QA inherit one scheduler contract for leases, backpressure, cancellation, retry, orphan recovery, progress, attribution, EventLedger authority, Flight Recorder projection, and Palmistry watch coverage.";
  }
  if (id === "MT-066" || id === "MT-319") {
    addCriteria(mt, "EventLedger remains the replay authority; Flight Recorder is an idempotent projection carrying the same event/run/correlation ids, projection outage cannot block authority replay, and catch-up cannot duplicate state.");
  }
  if (id === "MT-483") {
    addCriteria(mt, "The BodyKit event registry enumerates every planned later event family before downstream implementation; ad hoc event names outside the registry fail the build-readiness gate.");
  }
  if (id === "MT-708") {
    addCriteria(mt, "The matrix is generated from TailorSurfaceDescriptor rows and fails on wildcard-only targets, duplicate/orphan/unreachable controls, missing commands or help anchors, row-position IDs, and controls without permission, undo, receipt, and state coverage.");
  }
  if (id === "MT-699") {
    addCriteria(mt, "Context help and the compatibility ModelManual projection are generated from the canonical command/surface corpus; a control, Problem entry, command, or manual-anchor drift is a build error.");
  }
  if (id === "MT-709") {
    for (const dependency of ["MT-748", "MT-751", "MT-752", "MT-753", "MT-754", "MT-755"]) addUnique(mt.lifecycle.depends_on, dependency);
  }
  if (id === "MT-719") addUnique(mt.lifecycle.depends_on, "MT-749");
  if (id === "MT-737") {
    for (const dependency of ["MT-635", "MT-760", "MT-761", "MT-762", "MT-763"]) addUnique(mt.lifecycle.depends_on, dependency);
  }
  if (id === "MT-653") {
    for (const dependency of ["MT-760", "MT-761", "MT-762", "MT-763"]) addUnique(mt.lifecycle.depends_on, dependency);
  }
  if (bodyKitJobProducers.has(id)) addUnique(mt.lifecycle.depends_on, "MT-650");
}

function newMt({ number, group, summary, deps, paths, criteria, proof, risk, gui = null, manualChapter = null }) {
  const id = mtId(number);
  const guiRequired = Boolean(gui);
  const targets = gui?.targets || [];
  return {
    schema_id: "hsk.microtask_contract@1",
    schema_version: "microtask_contract_v1",
    contract_authority: "PRIMARY_MACHINE_READABLE",
    artifact_policy: {
      authority_surface: "MACHINE_CONTRACT",
      legacy_markdown_policy: "SAFETY_NET_ONLY_DO_NOT_COPY_FORWARD",
      projection_creation: "ON_OPERATOR_REQUEST_ONLY",
      operator_facing_authority: false,
      model_created_markdown_authority_allowed: false,
      allowed_markdown_exceptions: ["operator_created_notes", "operator_created_research", "operator_created_audits", "explicit_on_demand_projection", "explicit_report_projection_contract", "frozen_legacy_migration_reference"]
    },
    wp_id: WP_ID,
    mt_id: id,
    created_at_utc: UPDATED_AT,
    updated_at_utc: UPDATED_AT,
    authority_files: {
      microtask_contract: `.GOV/task_packets/${WP_ID}/${id}.json`,
      microtask_projection: `.GOV/task_packets/${WP_ID}/${id}.md`,
      packet_contract: `.GOV/task_packets/${WP_ID}/packet.json`,
      stub_contract: `.GOV/task_packets/stubs/${WP_ID}.contract.json`,
      build_readiness_prework: BUILD_READY_PATH,
      technical_refinement_prework: TECHNICAL_REFINEMENT_PATH,
      multi_lens_review: REVIEW_PATH
    },
    markdown_projection: {
      path: "NOT_GENERATED_BY_DEFAULT",
      status: "NOT_GENERATED_BY_DEFAULT",
      source_hash: "NOT_GENERATED_BY_DEFAULT",
      projection_hash: "NOT_GENERATED_BY_DEFAULT",
      generated_at_utc: null,
      generator: "PENDING_DEMAND"
    },
    lifecycle: {
      status: "PENDING",
      depends_on: unique(deps),
      blocks: [],
      active: false,
      validator_verdict: "PENDING"
    },
    scope: {
      summary,
      allowed_paths: paths,
      forbidden_paths: [],
      acceptance_criteria: criteria,
      proof_targets: proof,
      risk_if_missed: risk
    },
    gui_obligation: {
      operator_surface_required: guiRequired,
      gui_creation_required: guiRequired,
      argus_required: guiRequired,
      trace_projection_required_when_non_ui: !guiRequired,
      surfaces: gui?.surfaces || [],
      argus_targets: targets,
      expected_evidence: ["argus_inspect_or_accesskit_tree", "argus_screenshot_or_snapshot_reference", "renderer_console_error_scan", "layout_overlap_state_check", "capture_matrix_or_recorded_not_applicable_reason"],
      not_applicable_reason: guiRequired ? null : "Non-UI contract: typed commands, state, receipts, diagnostics, and TraceProjection provide model/operator observability; the owning native projection is separately contracted."
    },
    user_manual_obligation: {
      required: true,
      same_change_update_required: true,
      manual_version_bump_required: false,
      target_entries: [`Tailor UserManual > ${manualChapter || chapterByGroup[group]}`],
      expected_evidence: ["user_manual_diff", "manual_self_consistency_test", "manual_inspection_or_no_context_operation_test", "hbr_int_009_diagnostic_posture_recorded"],
      not_applicable_reason: null
    },
    hbr_obligations: [],
    resource_privacy_obligation: {
      required: true,
      hbr_rows: ["HBR-PRIV-001", "HBR-PRIV-002", "HBR-PRIV-003", "HBR-PRIV-004", "HBR-PRIV-005", "HBR-PRIV-006", "HBR-PRIV-007", "HBR-PRIV-008"],
      requirements: ["Resolve Principal, AuthenticatedSession and AccessSpace before every resource read or mutation.", "Use PostgreSQL RLS/FORCE RLS and ResourceBroker-backed artifact/file handles; UI filtering is never the security boundary.", "Derived resources preserve the intersection of source access scopes and must not leak existence through counts, search, previews, logs, traces, caches, diagnostics, exports or model context."],
      expected_evidence: ["cross_account_and_cross_space_negative_tests", "derived_scope_non_widening_test", "revocation_cache_context_handle_invalidation_test", "log_trace_preview_non_disclosure_test"]
    },
    hbr_int_009_tier_obligations: [],
    handoff: {
      coder_session: null,
      wp_validator_session: null,
      review_request_receipt_id: null,
      review_response_receipt_id: null
    },
    red_team: {
      required: true,
      profile: "TAILOR_PROFESSIONAL_PRODUCTION_V1",
      risks: [risk, "GUI, backend, agent, replay, and manual projections may drift if they do not consume the same typed authority."],
      minimum_controls: ["One typed owner and exact stable ids are required.", "Negative, concurrent, cancellation, recovery, and no-context proofs are required where applicable.", "No validator PASS/FAIL is inferred from this pre-activation contract."]
    },
    pre_activation_reconciliation: {
      status: "NEW_BUILD_READINESS_CONTRACT",
      original_scope_preserved: true,
      reason_codes: ["MULTI_LENS_BUILD_READINESS_GAP"],
      required_changes_before_execution: [],
      dependency_graph_status: RECONCILIATION_STATUS,
      authority: TECHNICAL_REFINEMENT_PATH
    },
    _group: group
  };
}

const newMts = [
  newMt({ number: 740, group: "BuildReadinessPlatform", summary: "Create the versioned Tailor vendor-capability registry and parity-claim gate covering the pinned Marvelous Designer and Daz Studio baselines; every capability is REQUIRED, QUALIFIED, ACCEPTED_EXCLUSION, or UNSUPPORTED, and full-parity wording fails while a required row is not QUALIFIED.", deps: ["MT-001"], paths: ["src/backend/handshake_core/src/tailor/capabilities.rs", "src/backend/handshake_core/tests/tailor_capability_claim_gate.rs"], criteria: ["Rows carry vendor/version/source, owning MT, command/UI/manual/proof ids, qualification artifacts, limitations, and supersession history.", "Claim generation fails closed on missing, deferred, unknown, excluded-required, stale-version, or uninspected rows and never treats an MT name as runtime proof.", "The registry distinguishes core product parity from third-party ecosystems and records unsupported legacy formats explicitly rather than through omission."], proof: ["cargo test -p handshake_core --test tailor_capability_claim_gate"], risk: "Marketing and planning could claim full parity while required workflows remain deferred, absent, or unqualified." }),
  newMt({ number: 741, group: "BuildReadinessPlatform", summary: "Extend the canonical KernelActionCatalogV1 and PRIM-CommandCorpusEntryV1 for Tailor so GUI, MCP/REST, shortcuts, Argus, replay, semantic undo, receipts, UserManual, and parallel-agent operation share one typed command authority.", deps: ["MT-001"], paths: ["src/backend/handshake_core/src/tailor/commands/", "src/backend/handshake_core/tests/tailor_command_parity.rs"], criteria: ["Every mutating action declares actor/session/correlation ids, target/ack routing, expected base version, lease scope, idempotency key, preview/diff/apply, cancellation/retry, stale/conflict/error envelopes, event ids, and receipt ids.", "Pure queries and mutations are distinguishable; parallel unrelated entities proceed, conflicting edits fail or merge through the declared policy, and retry is idempotent.", "A parity audit rejects GUI-only actions, MCP-only mutations, duplicate ids, undocumented permissions, non-replayable changes, or a Tailor-specific parallel command authority."], proof: ["cargo test -p handshake_core --test tailor_command_parity"], risk: "Parallel agents and operators could invoke divergent implementations, lose attribution, or corrupt authority under concurrent edits." }),
  newMt({ number: 742, group: "BuildReadinessPlatform", summary: "Define and enforce Tailor operation classification and EventLedger/ArtifactStore/Flight-Recorder conformance across pure computation, authority mutation, derived artifact, job lifecycle, UI projection, and DCC operations.", deps: ["MT-741"], paths: ["src/backend/handshake_core/src/tailor/governance/operation_contract.rs", "src/backend/handshake_core/tests/tailor_operation_conformance.rs"], criteria: ["Authority mutations obey event-before-row or one atomic transaction and EventLedger remains the replay authority; Flight Recorder is an idempotent projection with matching ids.", "Derived/exported artifacts require content hash, manifest, owner, source refs, retention, tombstone/cleanup, and partial/cancelled behavior before publication.", "A mechanical gate rejects unclassified operations, authority writes without event conformance, artifacts without manifests, and projection outages that block replay."], proof: ["cargo test -p handshake_core --test tailor_operation_conformance"], risk: "Tailor could fork authority across rows, files, traces, and DCC outputs, making recovery and attribution unreliable." }),
  newMt({ number: 743, group: "BuildReadinessPlatform", summary: "Integrate Tailor with the shared three-tier diagnostics contract: Flight Recorder projection, internal_diagnostics, and Palmistry watcher coverage classified DIRECT, INHERITED, or NOT_APPLICABLE for every MT and runtime operation.", deps: ["MT-742"], paths: ["src/backend/handshake_core/src/tailor/diagnostics.rs", "src/frontend/handshake_native/src/tailor/diagnostics.rs", "src/backend/handshake_core/tests/tailor_diagnostics_tiers.rs"], criteria: ["Covers backend unavailable, device loss/VRAM pressure, solver/contact overflow, native frame stall, cancellation/orphan recovery, DCC hang/survivor, application freeze/crash, forwarding, bounded retention, and typed allowlist behavior.", "Pure kernels inherit diagnostics through owning jobs; native, job, device, bridge, and DCC surfaces wire directly; Tailor creates no watcher or diagnostic store parallel to shared primitives.", "Fault injection proves content-redacted structured evidence, correlation continuity, Palmistry survivor recovery, and operation continuity when non-authoritative projections are unavailable."], proof: ["cargo test -p handshake_core --test tailor_diagnostics_tiers", "cargo test -p handshake-native tailor_diagnostics_projection"], risk: "Blanket deferral would make professional failures silent and would violate the shared diagnostics architecture." }),
  newMt({ number: 744, group: "BuildReadinessPlatform", summary: "Integrate Tailor with Locus as an optional production-work projection for QA assignments, repair queues, production plans, claim/occupancy/resume state, and stable tailor:// references without making Locus product or job authority.", deps: ["MT-742"], paths: ["src/backend/handshake_core/src/tailor/interops/locus.rs", "src/backend/handshake_core/tests/tailor_locus_interop.rs"], criteria: ["Uses only the canonical Postgres/EventLedger Locus API; absent canonical routes return the typed LocusReadApiUnavailable blocker and never create a Tailor-private Locus backend.", "Round-trip references cover Tailor jobs, entities, findings, actors, and receipts with idempotent requests, attribution, rename/tombstone behavior, and no duplicated Cloth/BodyKit authority.", "Tailor jobs and editing continue when Locus is unavailable; reconnect catch-up cannot duplicate work items or change product state."], proof: ["cargo test -p handshake_core --test tailor_locus_interop"], risk: "A private or authoritative Locus integration would fork work state and block Tailor when the optional work projection is unavailable." }),
  newMt({ number: 745, group: "BuildReadinessPlatform", summary: "Integrate Tailor garments, bodies, materials, recipes, captures, references, DCC reports, and mix graphs with Loom/Notes/ProjectKnowledgeIndex through stable shared references, backlinks, tags, search, and navigation.", deps: ["MT-742"], paths: ["src/backend/handshake_core/src/tailor/interops/loom.rs", "src/backend/handshake_core/tests/tailor_loom_interop.rs"], criteria: ["Loom stores links and knowledge metadata, not duplicate Tailor payload authority; direct-load by stable ref precedes semantic search.", "Backlinks, tags, notes, navigation, rename/version/tombstone/missing-asset, capability-denied, and no-silent-auto-index behavior are typed and tested.", "Uses the shared navigation and primitive/index schemas; bounded Tailor domain graphs declare their ownership and remain cross-navigable."], proof: ["cargo test -p handshake_core --test tailor_loom_interop"], risk: "Duplicated assets or a Tailor-only knowledge browser would drift from Loom and strand production context." }),
  newMt({ number: 746, group: "BuildReadinessPlatform", summary: "Integrate Tailor jobs with Calendar ActivitySpans/SessionSpans and optional production-block association, using CalendarMutation through WorkflowRun/calendar-sync for mutations and treating Calendar policy only as a scheduling hint.", deps: ["MT-742"], paths: ["src/backend/handshake_core/src/tailor/interops/calendar.rs", "src/backend/handshake_core/tests/tailor_calendar_interop.rs"], criteria: ["Tailor never writes Calendar tables directly; missing canonical routes return a typed blocker and do not prevent job execution.", "Tests cover timezone/DST, overlaps, reschedule/cancel, busy-only redaction, retry idempotency, content non-leakage, and correlation to job/session/event ids.", "Calendar unavailability, denial, or stale data cannot mutate Tailor authority or stall simulation, bake, render, or export work."], proof: ["cargo test -p handshake_core --test tailor_calendar_interop"], risk: "Direct calendar coupling could leak content, duplicate scheduling state, or block production jobs on an optional pillar." }),
  newMt({ number: 747, group: "BuildReadinessPlatform", summary: "Build the canonical task-oriented in-product Tailor UserManual and no-context operation corpus; keep per-MT evidence as a generated technical appendix and project legacy ModelManual content as a compatibility shim only.", deps: ["MT-741"], paths: ["src/backend/handshake_core/src/user_manual/tailor.rs", "src/backend/handshake_core/tests/tailor_user_manual.rs"], criteria: ["Chapters cover opening/navigating, project lifecycle, garment authoring, sewing/arrangement, simulation/repair, BodyKit, anatomy/contact, PoseKit/sheets, materials, animation, publishing/DCC, diagnostics/recovery, and parallel-agent operation.", "Exact commands, inputs, outputs, permissions, preconditions, errors, receipts, recovery actions, stable control ids, and context-help links are generated from the canonical command/surface corpus.", "Manual-only no-context tests complete representative Cloth, BodyKit, interlink, contact, conflict recovery, and export workflows through backend-only and GUI-only paths; drift is a build error."], proof: ["cargo test -p handshake_core --test tailor_user_manual"], risk: "Hundreds of MT-number pages would be unusable to operators and no-context models during real production or failure recovery." }),
  newMt({ number: 748, group: "BuildReadinessPlatform", summary: "Create the versioned TailorSurfaceDescriptor registry and generated Argus/AccessKit audit for exact routes, panes, controls, dynamic entity-key rules, actions, state projections, help anchors, permissions, undo policies, receipts, and command bindings.", deps: ["MT-741", "MT-750"], paths: ["src/frontend/handshake_native/src/tailor/surface_registry.rs", "src/frontend/handshake_native/tests/tailor_surface_manifest.rs"], criteria: ["Rejects duplicate ids, wildcard-only evidence, orphan/unreachable visible controls, controls without commands/help/permissions/undo/receipts, commands without projections, and dynamic ids derived from labels or row position.", "Argus targets and AccessKit assertions are generated from exact registry rows; blank/loading/populated/stale/conflict/cancelled/failed/retry states carry stable non-color semantics.", "Docked, floating, narrow, 4K, DPI, keyboard/focus, no-overlap/clipping, no-focus-steal, and screenshot/capture evidence are reproducible from the manifest."], proof: ["cargo test -p handshake-native --test tailor_surface_manifest"], risk: "Generic GUI targets cannot prove reachability, stable automation, accessibility, or safe parallel model interaction." }),
  newMt({ number: 749, group: "BuildReadinessPlatform", summary: "Implement typed pose/body/garment mix primitives over immutable PoseKit evidence, BodyPose assets, BodyKit recipes/channels, garment presets, and material/animation layers with region/layer weights, conflict previews, selected apply, detach, repair, and provenance.", deps: ["MT-714", "MT-715", "MT-717", "MT-741", "MT-742"], paths: ["src/backend/handshake_core/src/tailor/interlink/mix.rs", "src/backend/handshake_core/tests/tailor_mix_primitives.rs"], criteria: ["Mix operations are read/derive proposals until explicit selected apply through the owning API; 2D PoseKit evidence never becomes direct 3D authority.", "Supports weighted pose blending, symmetry, N-pose proximity blending, region-scoped body recipe/channel mixing, garment/preset composition, layer conflict rules, compare/revert, and content-addressed provenance.", "Concurrent mixes use expected versions, leases, idempotency, immutable proposals, deterministic conflict diagnostics, EventLedger receipts, and Loom-navigable refs."], proof: ["cargo test -p handshake_core --test tailor_mix_primitives"], risk: "Unbounded or UI-only mixing could silently overwrite body, pose, garment, or source authority and destroy reproducibility." }),
  newMt({ number: 750, group: "BuildReadinessPlatform", summary: "Activate the first real Tailor native surface integration seam against the merged WP-KERNEL-012 shell contract, replacing or graduating the design-only surface_extension_seam without assuming unused scaffold code is live.", deps: ["MT-001"], paths: ["src/frontend/handshake_native/src/surface_extension_seam.rs", "src/frontend/handshake_native/src/tailor/mod.rs", "src/frontend/handshake_native/tests/tailor_surface_extension.rs"], criteria: ["The seam is gated on merged-main WP-KERNEL-012 schema/receipt compatibility and registers Tailor panes, commands, state, navigation, jobs/problems/history, Argus, and UserManual anchors through canonical APIs.", "Missing/incompatible shell capabilities return typed blockers; no WebView/Tauri fallback, hidden feature fork, or unused design-only registration can claim success.", "Headless startup, multiple parallel sessions, layout restore, crash/restart, backend unavailable, and no-focus-steal behavior are proven."], proof: ["cargo test -p handshake-native --test tailor_surface_extension"], risk: "Tailor could be planned against a design-only seam and reach implementation with no real native host integration." }),
  newMt({ number: 751, group: "ProfessionalClothUX", summary: "Build the native professional 2D sewing and arrangement workspace for segment/free/1:N/M:N sewing, direction/range editing, length and gather diagnostics, arrangement points/bounds, wrap/curvature/offset/orientation, presets, preview, and correction.", deps: ["MT-118", "MT-119", "MT-120", "MT-350", "MT-351", "MT-352", "MT-354", "MT-461", "MT-741", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/commands/sewing_arrangement.rs", "src/frontend/handshake_native/src/tailor/sewing_arrangement.rs", "src/frontend/handshake_native/tests/tailor_sewing_arrangement.rs"], criteria: ["Directional arrows, handles, edge ranges, live mismatch/gather, reverse/delete/repair, collision-aware placement, saved presets, and preview/apply/revert are explicit and command-backed.", "Segment/free/1:N/M:N and arrangement operations have backend-only parity, stable entity IDs, semantic undo, EventLedger receipts, concurrent conflict handling, and no row-position identity.", "Argus proves keyboard-only authoring, error correction, saved/reloaded arrangement, drape, narrow/4K layouts, and no focus steal."], proof: ["cargo test -p handshake_core tailor_sewing_arrangement_commands", "cargo test -p handshake-native --test tailor_sewing_arrangement"], risk: "Schema-only sewing and arrangement cannot replace the professional Marvelous Designer workflow." , gui: { surfaces: ["Tailor 2D sewing and arrangement workspace"], targets: ["tailor-sewing-toolbar", "tailor-sewing-mode", "tailor-sewing-edge-{edge_id}", "tailor-sewing-direction-{seam_id}", "tailor-sewing-mismatch-{seam_id}", "tailor-arrangement-point-{point_id}", "tailor-arrangement-bounds", "tailor-arrangement-preset", "tailor-arrangement-preview", "tailor-arrangement-apply"] } }),
  newMt({ number: 752, group: "ProfessionalClothUX", summary: "Build the native simulation control and performance workspace for quality/use-case/backend profiles, CPU/GPU selection, start/instant-stop/reset/freeze, collision and resolution controls, residual/contact/VRAM state, fallback, and device recovery.", deps: ["MT-462", "MT-658", "MT-677", "MT-678", "MT-679", "MT-680", "MT-681", "MT-682", "MT-741", "MT-743", "MT-748", "MT-782"], paths: ["src/backend/handshake_core/src/tailor/commands/simulation_control.rs", "src/frontend/handshake_native/src/tailor/simulation_control.rs", "src/frontend/handshake_native/tests/tailor_simulation_control.rs"], criteria: ["Profiles keep quality, use-case, and backend as separate axes; exact effective parameters, unit conversions, solver/backend selection, fallback reason, and certificate status are inspectable.", "Instant stop, cancellation latency, reset/freeze, overflow, device loss, VRAM pressure, contention, checkpoint/resume, and failed fallback are bounded and recoverable without partial authority writes.", "Backend-only and Argus flows prove start/progress/stop/retry, accessible non-color performance state, no unbounded polling, and no focus steal."], proof: ["cargo test -p handshake_core tailor_simulation_control", "cargo test -p handshake-native --test tailor_simulation_control"], risk: "A strong solver without professional controls and failure visibility is unusable in production." , gui: { surfaces: ["Tailor simulation control and performance workspace"], targets: ["tailor-sim-profile", "tailor-sim-use-case", "tailor-sim-backend", "tailor-sim-start", "tailor-sim-stop", "tailor-sim-reset", "tailor-sim-freeze", "tailor-sim-residual", "tailor-sim-contact-pressure", "tailor-sim-vram", "tailor-sim-fallback", "tailor-sim-device-status"] } }),
  newMt({ number: 753, group: "ProfessionalClothUX", summary: "Build the professional fabric calibration and material-authoring lab for measured coupons, calibration curves, paired drape comparison, preset provenance/versioning, per-face assignment, front/back swap, prints/graphics, loss diagnostics, and model-command parity.", deps: ["MT-191", "MT-212", "MT-375", "MT-388", "MT-455", "MT-466", "MT-467", "MT-468", "MT-685", "MT-741", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/commands/fabric_lab.rs", "src/frontend/handshake_native/src/tailor/fabric_lab.rs", "src/frontend/handshake_native/tests/tailor_fabric_lab.rs"], criteria: ["Coupon inputs are unit-bearing and versioned; calibration curves, uncertainty, preview/final parameters, paired drape residuals, accepted/rejected results, and source provenance are inspectable.", "Per-face and front/back materials, atomic swap, graphics/prints, presets, dependency invalidation, unsupported/loss reports, compare, apply, revert, and batch replacement share typed commands.", "Backend-only and Argus tests cover invalid measurements, stale presets, missing textures, cancellation, partial bake, reference compare, accessibility, and recovery."], proof: ["cargo test -p handshake_core tailor_fabric_lab", "cargo test -p handshake-native --test tailor_fabric_lab"], risk: "Scattered physics and material primitives do not form a usable professional fabric workflow." , gui: { surfaces: ["Tailor fabric calibration and material lab"], targets: ["tailor-fabric-coupon", "tailor-fabric-curve", "tailor-fabric-drape-reference", "tailor-fabric-drape-result", "tailor-fabric-residual", "tailor-fabric-preset", "tailor-material-face-assignment", "tailor-material-front-back-swap", "tailor-material-loss-report"] } }),
  newMt({ number: 754, group: "ProfessionalClothUX", summary: "Define and build Tailor project/workspace lifecycle: create, open, duplicate, archive, recent projects, dirty/pending drafts, checkpoints, close with active jobs, recovery, layout, and multi-agent occupancy.", deps: ["MT-077", "MT-357", "MT-447", "MT-741", "MT-742", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/projects.rs", "src/frontend/handshake_native/src/tailor/project_lifecycle.rs", "src/frontend/handshake_native/tests/tailor_project_lifecycle.rs"], criteria: ["Project identity, version, authority/draft state, dirty causes, active jobs/agents, last checkpoint, archive state, and recent metadata are typed and inspectable.", "Close/archive/duplicate with active jobs, stale drafts, conflicts, missing assets, backend loss, and crash/restart use explicit preview, checkpoint, cancel/wait/detach, recovery, and receipt behavior.", "Backend-only and Argus tests prove no data loss, concurrent occupancy, deterministic restore, keyboard navigation, and no modal/focus-stealing automation."], proof: ["cargo test -p handshake_core tailor_project_lifecycle", "cargo test -p handshake-native --test tailor_project_lifecycle"], risk: "Professional users could lose drafts or jobs because opening, closing, duplicating, and recovering projects are undefined." , gui: { surfaces: ["Tailor project and workspace lifecycle"], targets: ["tailor-project-new", "tailor-project-open", "tailor-project-duplicate", "tailor-project-archive", "tailor-project-recent-{project_id}", "tailor-project-dirty-state", "tailor-project-active-jobs", "tailor-project-checkpoint", "tailor-project-close-preview", "tailor-project-recover"] } }),
  newMt({ number: 755, group: "ProfessionalClothUX", summary: "Build the professional export/publish workspace for object selection, target profiles, units/axes, topology/weld/thickness, UV/material/texture, animation range, LOD, dependency/preflight, manifest diff, destination, atomic output, retry, and round-trip qualification.", deps: ["MT-317", "MT-332", "MT-435", "MT-448", "MT-473", "MT-476", "MT-477", "MT-608", "MT-625", "MT-733", "MT-735", "MT-736", "MT-741", "MT-742", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/publish/workspace.rs", "src/frontend/handshake_native/src/tailor/publish_workspace.rs", "src/frontend/handshake_native/tests/tailor_publish_workspace.rs"], criteria: ["Effective target settings, selected objects, dependencies, conversions/losses, preflight findings, artifact paths/hashes, manifest diff, and promotion state are visible before apply.", "Export is a typed cancellable job with leases, bounded child processes, atomic staging/promotion, retry-failed-only, survivor cleanup, and exact DCC round-trip report.", "Backend-only and Argus tests cover missing dependency, unsupported feature, overwrite conflict, partial export, DCC hang, cancellation, recovery, permission denial, and no focus steal."], proof: ["cargo test -p handshake_core tailor_publish_workspace", "cargo test -p handshake-native --test tailor_publish_workspace"], risk: "Broad exporters without one complete configuration and recovery surface remain error-prone and unusable for production delivery." , gui: { surfaces: ["Tailor professional export and publish workspace"], targets: ["tailor-publish-objects", "tailor-publish-profile", "tailor-publish-units-axes", "tailor-publish-topology", "tailor-publish-materials", "tailor-publish-animation-range", "tailor-publish-lod", "tailor-publish-preflight", "tailor-publish-manifest-diff", "tailor-publish-destination", "tailor-publish-run", "tailor-publish-retry"] } }),
  newMt({ number: 756, group: "ProfessionalClothUX", summary: "Build the visual modular garment composer and block library over the existing typed modular block/swap contracts, including compatibility, preview, attachment diagnostics, atomic replacement, saved compositions, and backend/headless parity.", deps: ["MT-358", "MT-359", "MT-741", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/garment/modular_composer.rs", "src/frontend/handshake_native/src/tailor/modular_composer.rs", "src/frontend/handshake_native/tests/tailor_modular_composer.rs"], criteria: ["Block boxes, attachment interfaces, compatible replacements, mismatch diagnostics, preview, saved composition variants, dependency manifests, and library refs have stable typed ids.", "Swap is atomic, undoable, replayable, idempotent, conflict-aware, and preserves/remaps seams, materials, trims, measurements, and arrangement or reports exact losses.", "GUI and backend-only flows produce identical composition receipts and prove missing, stale, incompatible, cancelled, and recovery states."], proof: ["cargo test -p handshake_core tailor_modular_composer", "cargo test -p handshake-native --test tailor_modular_composer"], risk: "A block schema without a visual and model-operable composer does not deliver the Marvelous modular workflow." , gui: { surfaces: ["Tailor modular garment composer"], targets: ["tailor-modular-composer", "tailor-modular-block-{block_id}", "tailor-modular-slot-{slot_id}", "tailor-modular-compatible-{block_id}", "tailor-modular-preview", "tailor-modular-diagnostics", "tailor-modular-apply", "tailor-modular-revert"] } }),
  newMt({ number: 757, group: "ProfessionalClothUX", summary: "Implement Marvelous-compatible Pattern Archive semantics: active/archived panel state, dotted 2D retained outline, exclusion from 3D and simulation, restore, dependencies, undo/replay, API, and exact native UI behavior.", deps: ["MT-347", "MT-360", "MT-741", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/garment/pattern_archive.rs", "src/frontend/handshake_native/src/tailor/pattern_archive.rs", "src/frontend/handshake_native/tests/tailor_pattern_archive.rs"], criteria: ["Archived panels retain stable ids, geometry, seams, materials, notes, provenance, and dotted 2D projection while being excluded from tessellation, 3D scene, simulation, selection defaults, and export unless explicitly requested.", "Archive/unarchive previews dependency effects, is atomic/undoable/replayable/idempotent, handles concurrent edits and stale versions, and never aliases project archive or snapshot restore.", "Backend and Argus tests prove active/archive/restore, selection/filtering, dependent seam diagnostics, restart, and no hidden simulation participation."], proof: ["cargo test -p handshake_core tailor_pattern_archive", "cargo test -p handshake-native --test tailor_pattern_archive"], risk: "Project snapshots cannot substitute for Pattern Archive and would delete or simulate panels with the wrong semantics." , gui: { surfaces: ["Tailor Pattern Archive in 2D pattern workspace"], targets: ["tailor-pattern-{panel_id}-archive", "tailor-pattern-{panel_id}-archived-outline", "tailor-pattern-archive-filter", "tailor-pattern-{panel_id}-restore", "tailor-pattern-archive-dependencies"] } }),
  newMt({ number: 758, group: "ProfessionalClothUX", summary: "Build professional manual 2D/3D retopology over automatic draft generation: topology points, edges, triangles/quads, cross-seam ghosts, loops, symmetry, locks, projection, deformation preview, validation, and export.", deps: ["MT-385", "MT-741", "MT-748"], paths: ["tailor-solver/src/remesh/manual_topology.rs", "src/backend/handshake_core/src/tailor/garment/retopology.rs", "src/frontend/handshake_native/src/tailor/retopology.rs", "src/frontend/handshake_native/tests/tailor_retopology.rs"], criteria: ["Automatic MT-385 output is only a starting draft; users and agents can add/move/delete points and edges, form 3/4-sided faces, manage loops/poles, mirror, lock, and inspect cross-seam correspondence.", "Projection error, seam continuity, UV/material transfer, deformation quality, skin/cloth binding, LOD, invalid topology, compare, apply, and revert are explicit and versioned.", "Backend-only and Argus tests cover non-manifold edits, crossing edges, stale source, symmetry conflicts, cancellation, rollback, pose/deformation sweep, and DCC round trip."], proof: ["cargo test -p handshake_core tailor_retopology", "cargo test -p handshake-native --test tailor_retopology"], risk: "Shortest-diagonal triangle pairing is not professional retopology and cannot support production deformation or DCC delivery." , gui: { surfaces: ["Tailor 2D and 3D retopology workspace"], targets: ["tailor-retopo-mode", "tailor-retopo-point-{point_id}", "tailor-retopo-edge-{edge_id}", "tailor-retopo-face-{face_id}", "tailor-retopo-cross-seam", "tailor-retopo-loop", "tailor-retopo-symmetry", "tailor-retopo-lock", "tailor-retopo-error", "tailor-retopo-apply"] } }),
  newMt({ number: 759, group: "ProfessionalClothUX", summary: "Create reusable trim, button, buttonhole, zipper, topstitch, puckering, graphic, and multi-material colorway style assets with inheritance, previews, search, versioning, batch replacement, dependency invalidation, render variants, and export proof.", deps: ["MT-167", "MT-190", "MT-213", "MT-234", "MT-359", "MT-383", "MT-741", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/styles/", "src/frontend/handshake_native/src/tailor/style_browser.rs", "src/frontend/handshake_native/tests/tailor_style_browser.rs"], criteria: ["Style assets separate mechanics, geometry, material, graphic/layer, puckering, preview, compatibility, and export parameters with stable ids and explicit inheritance/override rules.", "Named colorways span multiple materials/graphics, carry thumbnails and dependency manifests, support compare/batch render/export, and never reduce to one-off HSV mutation.", "Backend and Argus tests cover inheritance cycles, missing dependencies, stale versions, batch replacement rollback, visibility/side/layer state, variant export, and recovery."], proof: ["cargo test -p handshake_core tailor_style_assets", "cargo test -p handshake-native --test tailor_style_browser"], risk: "Isolated trim mechanics and one-off recolor operations cannot support reusable production styling or colorway delivery." , gui: { surfaces: ["Tailor style and colorway browser/editor"], targets: ["tailor-style-browser", "tailor-style-{style_id}", "tailor-style-inheritance", "tailor-style-preview", "tailor-style-dependencies", "tailor-style-batch-replace", "tailor-colorway-{colorway_id}", "tailor-colorway-compare", "tailor-colorway-export"] } }),
  newMt({ number: 760, group: "BodyKitProduction", summary: "Integrate BodyKit geometry, joint, weight/rigidity, transfer, skin, and anatomy creator operations with canonical command/backend-agent parity, EventLedger, semantic undo, leases, cancellation, stale-state rejection, recovery, and native surfaces.", deps: ["MT-489", "MT-594", "MT-607", "MT-722", "MT-723", "MT-724", "MT-725", "MT-726", "MT-741", "MT-742", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/bodykit/content_tools/commands.rs", "src/backend/handshake_core/tests/bodykit_creator_command_parity.rs"], criteria: ["Every creator action has one descriptor and identical GUI/MCP/backend/replay/undo semantics with exact inputs, outputs, permissions, side effects, artifacts, progress, and receipts.", "Concurrent unrelated edits proceed; topology/rig/weight/material conflicts use declared leases/base versions, and cancellation/partial failure leaves atomic authority and artifact state.", "A no-context model completes representative creator flows through backend only and reproduces them through GUI without hidden state or manual intervention."], proof: ["cargo test -p handshake_core --test bodykit_creator_command_parity"], risk: "Native-only creator tools would exclude parallel agents and create unreplayable UI authority." }),
  newMt({ number: 761, group: "BodyKitProduction", summary: "Build the professional adult contact-staging workspace for penetrator/receiver selection, entry axis, insertion depth/girth sweeps, erection/gape, pair-collision policy, wetness, feasibility, strain/inversion diagnostics, timeline recording, preview/apply/revert, and headless parity.", deps: ["MT-521", "MT-532", "MT-540", "MT-549", "MT-559", "MT-631", "MT-632", "MT-673", "MT-697", "MT-730", "MT-731", "MT-732", "MT-741", "MT-743", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/bodykit/contact_staging.rs", "src/frontend/handshake_native/src/tailor/bodykit/contact_staging.rs", "src/frontend/handshake_native/tests/bodykit_contact_staging.rs"], criteria: ["Typed staging supports vulva, penis, scrotum, anus, oral, props, futa combinations, multi-contact, cloth/tissue coupling, insertion path/depth/girth, erection/gape, wetness, contact layers, and timeline keys.", "Preview exposes feasibility, initial intersection, strain, inversion, contact pressure, thickness, collision policy, corrective state, and accepted-step certificate before explicit apply.", "Backend and Argus tests cover impossible entry, stale body/pose, topology inversion, conflicting agent edit, device loss, cancellation, partial settle/bake, retry, revert, and export continuity."], proof: ["cargo test -p handshake_core bodykit_contact_staging", "cargo test -p handshake-native --test bodykit_contact_staging"], risk: "Detailed genital anatomy and contact solvers remain unusable for professional porn staging without one safe, inspectable workflow." , gui: { surfaces: ["BodyKit adult contact-staging workspace"], targets: ["tailor-contact-penetrator", "tailor-contact-receiver", "tailor-contact-entry-axis", "tailor-contact-depth", "tailor-contact-girth", "tailor-contact-erection", "tailor-contact-gape", "tailor-contact-wetness", "tailor-contact-feasibility", "tailor-contact-strain", "tailor-contact-preview", "tailor-contact-apply", "tailor-contact-revert"] } }),
  newMt({ number: 762, group: "BodyKitProduction", summary: "Build the BodyKit face, FACS/ARKit, expression, eye, corrective, and viseme creator with typed backend commands, graph/timeline editing, symmetry/asymmetry, reference comparison, and native UI parity.", deps: ["MT-576", "MT-583", "MT-627", "MT-633", "MT-722", "MT-726", "MT-741", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/bodykit/face_creator.rs", "src/frontend/handshake_native/src/tailor/bodykit/face_creator.rs", "src/frontend/handshake_native/tests/bodykit_face_creator.rs"], criteria: ["FACS/ARKit units, custom expressions, correctives, eyelid/gaze, jaw/tongue/lip, visemes, symmetry/asymmetry, ranges, graph drivers, timeline curves, and export mappings have stable typed ids.", "Edits support preview/compare/apply/revert, reference captures, authored/effective values, leases, stale conflicts, semantic undo, and EventLedger receipts.", "Backend-only and Argus tests cover extremes, eye/oral contacts, invalid drivers, missing correctives, conflicting edits, cancellation, recovery, and DCC export."], proof: ["cargo test -p handshake_core bodykit_face_creator", "cargo test -p handshake-native --test bodykit_face_creator"], risk: "Face primitives without a creator workflow cannot replace Daz expression, PowerPose, and viseme authoring." , gui: { surfaces: ["BodyKit face and expression creator"], targets: ["tailor-face-unit-{unit_id}", "tailor-face-expression-{expression_id}", "tailor-face-corrective-{corrective_id}", "tailor-face-eye-gaze", "tailor-face-jaw", "tailor-face-tongue", "tailor-face-viseme-{viseme_id}", "tailor-face-curve", "tailor-face-reference-compare", "tailor-face-apply"] } }),
  newMt({ number: 763, group: "BodyKitProduction", summary: "Build the unified professional Tailor asset browser with Daz Smart Content-compatible metadata: product/content type/category/tag/compatibility base, scene selection filtering, dependency resolution, package authoring, previews, search, favorites, recents, uncategorized/lost-and-found, and progressive hydration.", deps: ["MT-357", "MT-359", "MT-360", "MT-520", "MT-605", "MT-606", "MT-628", "MT-702", "MT-721", "MT-733", "MT-741", "MT-745", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/assets/smart_content.rs", "src/frontend/handshake_native/src/tailor/asset_browser.rs", "src/frontend/handshake_native/tests/tailor_asset_browser.rs"], criteria: ["Typed metadata and compatibility filters cover garments, modular blocks, bodies, morphs, poses, materials, trims, styles, hair, scenes, lights/cameras, recipes, and DCC packages without copying payload authority.", "Selection-aware compatibility, dependency status, uncategorized/lost-and-found, previews, favorites/recents, background loading, package authoring, rename/version/tombstone, and missing-asset recovery are explicit.", "Backend and Argus tests cover large libraries, denied capability, offline index, stale metadata, dependency cycles, progressive hydration, concurrent tagging, and deterministic search/direct-load."], proof: ["cargo test -p handshake_core tailor_smart_content", "cargo test -p handshake-native --test tailor_asset_browser"], risk: "A generic flat library cannot replace Daz Smart Content or support professional dependency-heavy production." , gui: { surfaces: ["Unified Tailor asset and Smart Content browser"], targets: ["tailor-assets-browser", "tailor-assets-search", "tailor-assets-category", "tailor-assets-compatible", "tailor-assets-selection-filter", "tailor-asset-{asset_id}", "tailor-asset-dependencies", "tailor-asset-preview", "tailor-asset-favorite", "tailor-assets-recents", "tailor-assets-lost-found", "tailor-assets-hydration"] } }),
  newMt({ number: 764, group: "BodyKitProduction", summary: "Implement the typed BodyKit/Cloth shader graph authoring system with nodes, sockets, parameters, subgraphs, preview compile, version diff, validation, MaterialX/OpenPBR translation, unsupported-node diagnostics, and agent/native parity.", deps: ["MT-735", "MT-741", "MT-742", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/materials/shader_graph.rs", "src/frontend/handshake_native/src/tailor/shader_graph.rs", "src/frontend/handshake_native/tests/tailor_shader_graph.rs"], criteria: ["Graph authority is typed, acyclic where required, versioned, content-addressed, diffable, and supports reusable subgraphs, parameters, textures, normals/displacement, SSS, wetness, hair, and garment/body contexts.", "Preview compile, MaterialX/OpenPBR translation, target capability, unsupported/loss diagnostics, caches, cancellation, and recovery are explicit and deterministic.", "Backend and Argus tests cover invalid cycles/types, missing textures, stale graphs, concurrent edits, compile failure, compare/revert, and DCC round trip."], proof: ["cargo test -p handshake_core tailor_shader_graph", "cargo test -p handshake-native --test tailor_shader_graph"], risk: "Material interchange alone cannot replace Daz Shader Mixer or provide inspectable production look development." , gui: { surfaces: ["Tailor shader graph editor"], targets: ["tailor-shader-graph", "tailor-shader-node-{node_id}", "tailor-shader-socket-{socket_id}", "tailor-shader-link-{link_id}", "tailor-shader-parameter-{parameter_id}", "tailor-shader-preview", "tailor-shader-diagnostics", "tailor-shader-diff"] } }),
  newMt({ number: 765, group: "BodyKitProduction", summary: "Implement layered image and layered material authoring with image/paint/procedural groups, masks, blend modes, transforms, channel routing, front/back and region assignment, non-destructive versioning, preview/bake, and shader-graph integration.", deps: ["MT-726", "MT-735", "MT-764", "MT-741", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/materials/layer_stack.rs", "src/frontend/handshake_native/src/tailor/layer_stack.rs", "src/frontend/handshake_native/tests/tailor_layer_stack.rs"], criteria: ["Layers, groups, masks, blend modes, opacity, transforms, channel packing, color management, side/region assignment, dependencies, and bake outputs have stable ids and explicit authority.", "Preview/bake is cancellable and atomic with hashes, loss reports, compare/revert, cache invalidation, unsupported-target diagnostics, and MaterialX/shader-graph translation.", "Backend and Argus tests cover missing sources, invalid masks, deep stacks, stale edits, concurrent reorder, partial bake, recovery, and DCC round trip."], proof: ["cargo test -p handshake_core tailor_layer_stack", "cargo test -p handshake-native --test tailor_layer_stack"], risk: "A shader graph without layered image/material authoring cannot replace Daz Layered Image Editor workflows." , gui: { surfaces: ["Tailor layered image and material editor"], targets: ["tailor-layer-stack", "tailor-layer-{layer_id}", "tailor-layer-group-{group_id}", "tailor-layer-mask-{mask_id}", "tailor-layer-blend", "tailor-layer-opacity", "tailor-layer-transform", "tailor-layer-preview", "tailor-layer-bake", "tailor-layer-diagnostics"] } }),
  newMt({ number: 766, group: "BodyKitProduction", summary: "Implement the native final-render and Photo Studio lane with path tracing, render presets, physical cameras/lights, environment, denoise, tone mapping/color management, material/hair/volume support, deterministic reference scenes, and bounded GPU/CPU scheduling.", deps: ["MT-235", "MT-241", "MT-390", "MT-633", "MT-735", "MT-764", "MT-765", "MT-741", "MT-743", "MT-782"], paths: ["src/backend/handshake_core/src/tailor/render/final_renderer.rs", "src/frontend/handshake_native/src/tailor/render_studio.rs", "src/frontend/handshake_native/tests/tailor_render_studio.rs"], criteria: ["Render scenes, cameras, lights, environments, integrator/sampling, motion/depth of field, denoise, tone/color, materials, hair, volumes, device budgets, and presets are versioned and inspectable.", "Rendering is a cancellable leased job with progress, checkpoints, deterministic seeds, device-loss/VRAM recovery, atomic artifacts, content hashes, and no focus steal.", "Backend and Argus tests cover reference scenes, skin/genital/cloth/hair close-ups, dark/light tones, wetness, unsupported nodes, device loss, cancellation, resume, and reproducibility."], proof: ["cargo test -p handshake_core tailor_final_renderer", "cargo test -p handshake-native --test tailor_render_studio"], risk: "External viewport captures cannot satisfy literal Daz final-render parity or professional close-up qualification." , gui: { surfaces: ["Tailor Photo Studio and final-render workspace"], targets: ["tailor-render-studio", "tailor-render-camera", "tailor-render-light-{light_id}", "tailor-render-environment", "tailor-render-preset", "tailor-render-device", "tailor-render-start", "tailor-render-stop", "tailor-render-progress", "tailor-render-preview", "tailor-render-diagnostics"] } }),
  newMt({ number: 767, group: "BodyKitProduction", summary: "Implement final-render AOV/canvas outputs, region/spot render, render queue, retry/priority/cancellation, output templates, comparison, history, and Render Library with artifact manifests and no-context operation.", deps: ["MT-697", "MT-702", "MT-733", "MT-766", "MT-741", "MT-743", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/render/queue.rs", "src/frontend/handshake_native/src/tailor/render_queue.rs", "src/frontend/handshake_native/tests/tailor_render_queue.rs"], criteria: ["AOV/canvas definitions, crop/spot regions, queue priority, dependencies, retries, output templates, formats/color spaces, compare/history, artifact hashes, and source scene versions are explicit.", "Queue work inherits scheduler leases/backpressure/cancellation/orphan recovery; retry-failed-only and app restart cannot duplicate completed artifacts or leave survivor processes.", "Backend and Argus tests cover multi-agent enqueue/reorder, stale scenes, partial AOVs, disk error, cancellation, device loss, retry, comparison, and direct-load Render Library navigation."], proof: ["cargo test -p handshake_core tailor_render_queue", "cargo test -p handshake-native --test tailor_render_queue"], risk: "A renderer without AOVs, spot render, queue, recovery, and library cannot support professional Daz-class production." , gui: { surfaces: ["Tailor render queue and Render Library"], targets: ["tailor-render-spot-region", "tailor-render-aov-{aov_id}", "tailor-render-queue", "tailor-render-job-{job_id}", "tailor-render-priority", "tailor-render-cancel", "tailor-render-retry", "tailor-render-output-template", "tailor-render-library", "tailor-render-artifact-{artifact_id}", "tailor-render-compare"] } }),
  newMt({ number: 768, group: "BodyKitProduction", summary: "Implement reference-image-to-face/body transfer as an immutable proposal workflow separating landmarks, camera, lighting, shape, texture, and identity evidence, with manual correction, similarity matrices, lineage, and no silent authority writes.", deps: ["MT-576", "MT-583", "MT-710", "MT-713", "MT-715", "MT-721", "MT-741", "MT-742", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/bodykit/reference_transfer.rs", "src/frontend/handshake_native/src/tailor/bodykit/reference_transfer.rs", "src/frontend/handshake_native/tests/bodykit_reference_transfer.rs"], criteria: ["Inputs and derived landmarks, segmentation, camera, lighting, shape, texture, confidence, similarity, uncertainty, source hashes, and model/tool versions are immutable and separately inspectable.", "Results remain proposals until explicit selected apply; manual landmark/camera/shape/texture correction, compare, reject, revert, and rederive preserve lineage and source authority.", "Backend and Argus tests cover multi-view/single-view, occlusion, mismatched camera, lighting leakage, stale body, conflicting edits, partial model failure, cancellation, recovery, and identity-reference matrices."], proof: ["cargo test -p handshake_core bodykit_reference_transfer", "cargo test -p handshake-native --test bodykit_reference_transfer"], risk: "Generic face controls do not provide Daz Face Transfer-equivalent workflow or evidence separation." , gui: { surfaces: ["BodyKit reference face/body transfer workspace"], targets: ["tailor-reference-source-{source_id}", "tailor-reference-landmark-{landmark_id}", "tailor-reference-camera", "tailor-reference-lighting", "tailor-reference-shape-proposal", "tailor-reference-texture-proposal", "tailor-reference-similarity", "tailor-reference-diff", "tailor-reference-apply", "tailor-reference-revert"] } }),
  newMt({ number: 769, group: "BodyKitProduction", summary: "Implement reusable Daz-class deformation fields with point/line/plane/spherical/cylindrical influences, falloff curves, weights, masks, transforms, stacks, animation, visualization, baking, and BodyKit/Cloth application.", deps: ["MT-478", "MT-722", "MT-741", "MT-742", "MT-748"], paths: ["tailor-solver/src/body/deformers.rs", "src/backend/handshake_core/src/tailor/bodykit/deformers.rs", "src/frontend/handshake_native/src/tailor/bodykit/deformers.rs", "src/frontend/handshake_native/tests/bodykit_deformers.rs"], criteria: ["Field shapes, transforms, falloff, weights, masks, affected nodes/regions, stack order, animation, authored/effective values, and bake targets have stable typed ids.", "Preview and heatmaps expose influence and bounds; apply/bake is versioned, undoable, conflict-aware, atomic, and preserves or reports rig/morph/UV/material effects.", "Backend and Argus tests cover zero/extreme falloff, overlapping fields, invalid targets, stale geometry, concurrent edits, animation, cancellation, rollback, and export."], proof: ["cargo test -p handshake_core bodykit_deformers", "cargo test -p handshake-native --test bodykit_deformers"], risk: "Body-specific morph tools do not replace reusable Daz D-Former workflows." , gui: { surfaces: ["BodyKit deformation-field editor"], targets: ["tailor-deformer-{deformer_id}", "tailor-deformer-shape", "tailor-deformer-transform", "tailor-deformer-falloff", "tailor-deformer-mask", "tailor-deformer-target", "tailor-deformer-heatmap", "tailor-deformer-bake"] } }),
  newMt({ number: 770, group: "BodyKitProduction", summary: "Implement the Figure Setup workflow for node hierarchy, geometry/face groups, skeleton association, orientation/limits, labels, region mapping, weight initialization, validation, presets, and versioned figure construction.", deps: ["MT-514", "MT-527", "MT-721", "MT-723", "MT-724", "MT-741", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/bodykit/figure_setup.rs", "src/frontend/handshake_native/src/tailor/bodykit/figure_setup.rs", "src/frontend/handshake_native/tests/bodykit_figure_setup.rs"], criteria: ["Figure nodes, hierarchy, geometry groups, bones, orientations, limits, labels, regions, compatibility bases, weight seeds, and export mappings are typed, versioned, and inspectable.", "Construction supports preview/validate/apply/revert, reusable presets, missing/duplicate/orphan group diagnostics, cycle rejection, symmetry, and atomic authority updates.", "Backend and Argus tests cover invalid hierarchy, missing geometry group, bad orientation, stale geometry, concurrent edits, rollback, and DCC round trip."], proof: ["cargo test -p handshake_core bodykit_figure_setup", "cargo test -p handshake-native --test bodykit_figure_setup"], risk: "Individual joint and weight editors do not provide a complete Daz Figure Setup workflow." , gui: { surfaces: ["BodyKit Figure Setup workspace"], targets: ["tailor-figure-tree", "tailor-figure-node-{node_id}", "tailor-figure-geometry-group-{group_id}", "tailor-figure-bone-{bone_id}", "tailor-figure-orientation", "tailor-figure-limits", "tailor-figure-weight-seed", "tailor-figure-validation", "tailor-figure-apply"] } }),
  newMt({ number: 771, group: "BodyKitProduction", summary: "Implement UV, texture, weight, mask, and generic map transfer across compatible or changed topology/UV variants with cage/raycast/barycentric methods, projection-error inspection, seam policy, batch jobs, and atomic apply.", deps: ["MT-570", "MT-724", "MT-725", "MT-726", "MT-741", "MT-742", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/bodykit/map_transfer.rs", "src/frontend/handshake_native/src/tailor/bodykit/map_transfer.rs", "src/frontend/handshake_native/tests/bodykit_map_transfer.rs"], criteria: ["Source/target meshes, UV sets, map/channel type, method, cage/ray settings, seams, missing coverage, projection error, artifacts, and versions are explicit.", "Preview heatmaps, thresholded acceptance, manual exclusions, batch operation, cancellation, retry, atomic apply, rollback, and loss report are shared by backend and GUI.", "Tests cover topology/UV mismatch, holes/overlap, mirrored seams, dark/light skin maps, weights/masks, stale source/target, cancellation, recovery, and DCC round trip."], proof: ["cargo test -p handshake_core bodykit_map_transfer", "cargo test -p handshake-native --test bodykit_map_transfer"], risk: "Transfer Utility does not cover Daz Map Transfer semantics or production inspection across UV/topology variants." , gui: { surfaces: ["BodyKit map-transfer workspace"], targets: ["tailor-map-source", "tailor-map-target", "tailor-map-channel", "tailor-map-method", "tailor-map-cage", "tailor-map-seam-policy", "tailor-map-error-heatmap", "tailor-map-threshold", "tailor-map-preview", "tailor-map-apply"] } }),
  newMt({ number: 772, group: "BodyKitProduction", summary: "Expand read-only user-owned DSON migration beyond morph dials to supported scene/preset nodes, transforms, hierarchy, materials, poses, cameras/lights, animation, and compatibility metadata with exact unsupported-feature reports and no Daz asset redistribution.", deps: ["MT-623", "MT-733", "MT-740", "MT-741", "MT-742"], paths: ["src/backend/handshake_core/src/tailor/bodykit/import/dson.rs", "src/backend/handshake_core/tests/bodykit_dson_migration.rs"], criteria: ["Supported DSON entities map through versioned typed adapters with source hashes, ownership/provenance, units/axes, stable ids, dependency refs, conversion losses, and unsupported-node reports.", "Import is read-only, sandboxed, cancellable, idempotent, and proposal-first; missing/unsupported content does not silently substitute or write authority.", "Fixtures cover scene, pose, material, camera/light, animation, compatibility metadata, unsupported scripts/plugins, dependency loss, cancellation, retry, and round-trip reporting."], proof: ["cargo test -p handshake_core --test bodykit_dson_migration"], risk: "Morph-only DSON ingest cannot support a credible Daz migration workflow and may silently lose scene/preset intent." }),
  newMt({ number: 773, group: "DccParityQualification", summary: "Create the capability-scoped Tailor extension/plugin SDK for Rust and documented Python/Daz/Marvelous bridge adapters with manifests, typed command registration, permissions, sandboxing, version compatibility, reload/recovery, API introspection, and deterministic testing.", deps: ["MT-689", "MT-740", "MT-741", "MT-742", "MT-743"], paths: ["src/backend/handshake_core/src/tailor/extensions/", "src/backend/handshake_core/tests/tailor_extension_sdk.rs"], criteria: ["Manifests declare id/version/API range/capabilities/commands/assets/config/dependencies/signature/hash and fail closed on incompatibility or denied capability.", "Extensions register through the canonical action catalog, run in bounded sandbox/process contexts, cannot bypass authority or artifact policies, and expose health/reload/disable/recovery state.", "The harness proves install/load/call/reload/crash/survivor/upgrade/downgrade/conflict/permission denial and API coverage introspection without foreground focus or hidden state."], proof: ["cargo test -p handshake_core --test tailor_extension_sdk"], risk: "REST/MCP alone does not provide Marvelous Python or Daz Script-style extension lifecycle and could encourage unsafe private integrations." }),
  newMt({ number: 774, group: "ProfessionalClothUX", summary: "Replace the placeholder fur contract with production garment fur/fuzz: guides or procedural fibers, density/length/clump/curl/noise, masks, seam/graphic preservation, cloth binding, collisions, grooming, LOD/cards/strands, viewport/final render, export, and performance qualification.", deps: ["MT-384", "MT-633", "MT-755", "MT-764", "MT-766", "MT-741", "MT-743", "MT-748"], paths: ["tailor-solver/src/groom/garment_fur.rs", "src/backend/handshake_core/src/tailor/groom/garment_fur.rs", "src/frontend/handshake_native/src/tailor/groom/garment_fur.rs", "src/frontend/handshake_native/tests/tailor_garment_fur.rs"], criteria: ["Fur assets carry guides/procedural settings, masks, density/length/clump/curl/noise, direction, material, binding, collision, LOD/card/strand outputs, and stable provenance.", "Grooming preserves seams/graphics and follows cloth deformation without detachment, explosive motion, hidden intersections, or unsupported export silence.", "Backend/Argus/reference tests cover short/long fur, trims/seams, animation, wetness, extreme deformation, LOD, renderer, DCC export, device budget, cancellation, and recovery."], proof: ["cargo test -p handshake_core tailor_garment_fur", "cargo test -p handshake-native --test tailor_garment_fur"], risk: "A deferred placeholder invalidates literal Marvelous parity and blocks furry garment production." , gui: { surfaces: ["Tailor garment fur and fuzz groom workspace"], targets: ["tailor-fur-asset-{groom_id}", "tailor-fur-guides", "tailor-fur-density", "tailor-fur-length", "tailor-fur-clump", "tailor-fur-curl", "tailor-fur-mask", "tailor-fur-lod", "tailor-fur-preview", "tailor-fur-export"] } }),
  newMt({ number: 775, group: "BodyKitProduction", summary: "Implement BodyKit strand-hair/groom authoring and export: scalp/eyebrow/eyelash/body-hair guides, density/length/clump/curl/noise, surface binding under morphs, grooming tools, dynamics/collision, LOD/cards/strands, materials, render, and DCC qualification.", deps: ["MT-574", "MT-721", "MT-764", "MT-766", "MT-741", "MT-743", "MT-748"], paths: ["tailor-solver/src/groom/body_hair.rs", "src/backend/handshake_core/src/tailor/bodykit/groom.rs", "src/frontend/handshake_native/src/tailor/bodykit/groom.rs", "src/frontend/handshake_native/tests/bodykit_groom.rs"], criteria: ["Hair assets separate scalp/region binding, guides, procedural settings, masks, material, dynamics, collisions, LOD/card/strand outputs, attachment, and versioned provenance.", "Groom tools support comb/cut/grow/curl/clump/mirror/mask/preview/apply/revert with canonical command parity, leases, semantic undo, and atomic artifacts.", "Backend/Argus/reference tests cover head/body morph extremes, animation, collisions, wetness, eyelashes/brows, dark/light hair, render, LOD, export, device loss, cancellation, and recovery."], proof: ["cargo test -p handshake_core bodykit_groom", "cargo test -p handshake-native --test bodykit_groom"], risk: "Hooks without authoring, dynamics, render, and export cannot replace Daz strand-based hair workflows." , gui: { surfaces: ["BodyKit strand-hair and groom workspace"], targets: ["tailor-groom-asset-{groom_id}", "tailor-groom-region", "tailor-groom-guides", "tailor-groom-density", "tailor-groom-length", "tailor-groom-clump", "tailor-groom-curl", "tailor-groom-mask", "tailor-groom-dynamics", "tailor-groom-lod", "tailor-groom-preview"] } }),
  newMt({ number: 776, group: "BodyKitProduction", summary: "Implement the version-pinned speech-to-viseme/lip-sync pipeline with audio/transcript ingest, phoneme timing, co-articulation, emotion/intensity, editable curves, face/tongue/jaw integration, preview, bake, export, and exact connector/model provenance.", deps: ["MT-581", "MT-762", "MT-741", "MT-742", "MT-743", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/bodykit/lipsync.rs", "src/frontend/handshake_native/src/tailor/bodykit/lipsync.rs", "src/frontend/handshake_native/tests/bodykit_lipsync.rs"], criteria: ["Audio/transcript/model/tool versions, language, phonemes, visemes, timings, confidence, co-articulation, intensity/emotion, curves, manual edits, and source hashes are explicit.", "Generation is a cancellable proposal job; preview/compare/edit/apply/revert, stale face rigs, missing visemes, silence/noise, model unavailability, and unsupported language are typed.", "Backend/Argus tests cover timing/alignment, speech extremes, jaw/tongue/lip contacts, manual correction, cancellation, retry, replay, animation export, and DCC round trip."], proof: ["cargo test -p handshake_core bodykit_lipsync", "cargo test -p handshake-native --test bodykit_lipsync"], risk: "Accepting external tracks while deferring the solver cannot satisfy Daz lip-sync workflow parity." , gui: { surfaces: ["BodyKit speech-to-viseme and lip-sync editor"], targets: ["tailor-lipsync-audio", "tailor-lipsync-transcript", "tailor-lipsync-language", "tailor-lipsync-generate", "tailor-lipsync-phoneme-{phoneme_id}", "tailor-lipsync-viseme-{viseme_id}", "tailor-lipsync-curve", "tailor-lipsync-preview", "tailor-lipsync-apply"] } }),
  newMt({ number: 777, group: "ProfessionalClothUX", summary: "Implement native PBR map generation and a version-pinned optional Substance/SBSAR connector with parameter introspection, previews, texture bakes, manifests, color management, loss reporting, fail-closed availability, and material/layer/shader integration.", deps: ["MT-226", "MT-228", "MT-388", "MT-735", "MT-753", "MT-764", "MT-765", "MT-741", "MT-743", "MT-748"], paths: ["src/backend/handshake_core/src/tailor/materials/pbr_generator.rs", "src/backend/handshake_core/src/tailor/materials/sbsar_connector.rs", "src/frontend/handshake_native/src/tailor/pbr_generator.rs", "src/frontend/handshake_native/tests/tailor_pbr_generator.rs"], criteria: ["Native generation covers base color, normal, height/displacement, roughness, metalness, AO, opacity, masks, channel packing, resolution/tiling, seed, and dependency manifests.", "The optional SBSAR connector pins executable/API/version/hash, exposes parameters and outputs, runs bounded/cancellable, records unsupported/loss state, and fails closed without licensing/tool assumptions.", "Backend/Argus tests cover native-only operation, parameter sweeps, missing connector, incompatible version, DCC hang, partial bake, color management, stale sources, recovery, and round trip."], proof: ["cargo test -p handshake_core tailor_pbr_generator", "cargo test -p handshake-native --test tailor_pbr_generator"], risk: "Fragmented texture generation and an implicit SBSAR exclusion leave a major professional material workflow unowned." , gui: { surfaces: ["Tailor PBR generator and SBSAR connector workspace"], targets: ["tailor-pbr-source", "tailor-pbr-output-{map_id}", "tailor-pbr-resolution", "tailor-pbr-seed", "tailor-pbr-generate", "tailor-sbsar-status", "tailor-sbsar-parameter-{parameter_id}", "tailor-sbsar-bake", "tailor-pbr-loss-report"] } }),
  newMt({ number: 778, group: "DccParityQualification", summary: "Create the version/path/hash-locked black-box DCC reference harness and legal fixture workflow for local Daz Studio and Marvelous Designer, official APIs/docs/examples, exported scenes, screenshots, behavior captures, and version-delta reports without decompilation or runtime dependence.", deps: ["MT-734", "MT-740", "MT-742", "MT-755", "MT-782"], paths: ["src/backend/handshake_core/src/tailor/dcc/reference_harness.rs", "src/backend/handshake_core/tests/tailor_dcc_reference_harness.rs", ".GOV/fixtures/tailor/dcc_reference/"], criteria: ["Pins executable path, version, SHA-256, API/plugin versions, source URL/doc version, hardware/driver, fixture hashes, capture method, and observed capability; missing Marvelous path remains a typed activation input, not inspected fact.", "Uses documented APIs, local metadata/config/examples, black-box actions, exports, and screenshots only; no proprietary binary decompilation, copying, or runtime product dependency.", "Produces reproducible capability deltas and exact unsupported/loss reports across upgrades, missing tools, API changes, timeout/hang, export/import, and reference requalification."], proof: ["cargo test -p handshake_core --test tailor_dcc_reference_harness"], risk: "Unpinned or uninspected DCC references make parity claims non-reproducible and can bake version-specific guesses into implementation." }),
  newMt({ number: 779, group: "DccParityQualification", summary: "Execute the feature-by-feature Marvelous Designer and Daz Studio parity qualification matrix over every required capability, GUI/backend/manual path, black-box reference fixture, export/import, visual artifact, failure path, and performance budget.", deps: ["MT-687", "MT-720", "MT-740", "MT-755", "MT-756", "MT-757", "MT-758", "MT-759", "MT-760", "MT-761", "MT-762", "MT-763", "MT-764", "MT-765", "MT-766", "MT-767", "MT-768", "MT-769", "MT-770", "MT-771", "MT-772", "MT-773", "MT-774", "MT-775", "MT-776", "MT-777", "MT-778"], paths: ["src/backend/handshake_core/tests/tailor_vendor_parity_matrix.rs", ".GOV/fixtures/tailor/vendor_parity/"], criteria: ["Every required capability row references exact MTs, commands, surfaces, manual anchors, reference sources, fixtures, artifacts, metric thresholds, DCC versions, and inspected reviewer evidence.", "Qualification covers success, boundary, unsupported, cancellation, recovery, concurrency, accessibility, no-context backend/GUI operation, round-trip, visual comparison, performance, and claim-state transitions.", "No cargo-only, changed-hash-only, MT-name-only, or uninspected artifact evidence can set QUALIFIED; the claim gate remains fail-closed until direct proof exists."], proof: ["cargo test -p handshake_core --test tailor_vendor_parity_matrix"], risk: "Feature ownership without witnessed qualification cannot establish professional usability or full-parity claims." }),
  newMt({ number: 780, group: "BuildReadinessPlatform", summary: "Run the machine-readable every-MT multi-lens semantic gate over all Tailor contracts, classifying product role, GUI ownership, backend/agent parity, HBR tiers, manual target, pillar links, acceptance depth, paths, proofs, and DAG integrity before activation and at every contract change.", deps: ["MT-740"], paths: [".GOV/roles_shared/checks/tailor-mt-preactivation-check.mjs", `.GOV/task_packets/${WP_ID}/_MULTI_LENS_REVIEW.json`], criteria: ["Every MT is uniquely classified as pure computation, authority mutation, artifact creation, job lifecycle, UI projection, tool/API, DCC, or a justified combination with one primary review lens.", "The gate rejects missing/duplicate ids, cycles, missing dependencies, inactive-state drift before activation, sparse acceptance, generic GUI targets, GUI/path mismatches, missing native proof, synthetic MT-number manual navigation, empty HBR obligations, deferred diagnostics, and unowned pillar/capability requirements.", "The generated review accounts for every MT exactly once and is advisory pre-activation evidence only; it never emits a WP validator PASS/FAIL verdict."], proof: ["node .GOV/roles_shared/checks/tailor-mt-preactivation-check.mjs"], risk: "A large MT inventory can hide omissions and contradictory obligations unless every contract is mechanically reviewed through the required lenses." , manualChapter: "Model operation, diagnostics, pillars, and manual" }),
  newMt({ number: 781, group: "DccParityQualification", summary: "Prove the final professional Tailor lifecycle end to end across project creation, Cloth authoring/sewing/arrangement, simulation/repair, BodyKit creation/anatomy/contact, PoseKit/sheets/mix primitives, materials/lookdev, animation, render, publish/DCC, pillars, parallel agents, diagnostics, manual-only operation, recovery, and parity claim gating.", deps: ["MT-653", "MT-709", "MT-719", "MT-737", "MT-743", "MT-744", "MT-745", "MT-746", "MT-747", "MT-748", "MT-749", "MT-750", "MT-751", "MT-752", "MT-753", "MT-754", "MT-755", "MT-756", "MT-757", "MT-758", "MT-759", "MT-760", "MT-761", "MT-762", "MT-763", "MT-764", "MT-765", "MT-766", "MT-767", "MT-768", "MT-769", "MT-770", "MT-771", "MT-772", "MT-773", "MT-774", "MT-775", "MT-776", "MT-777", "MT-778", "MT-779", "MT-780", "MT-782"], paths: ["src/backend/handshake_core/tests/tailor_professional_e2e.rs", "src/frontend/handshake_native/tests/tailor_professional_e2e.rs", ".GOV/fixtures/tailor/professional_e2e/"], criteria: ["A no-context model completes representative professional workflows through backend only and GUI only using the built-in UserManual, exact commands/ids, and structured state; outputs and receipts are equivalent.", "Multiple agents edit unrelated Cloth/BodyKit entities concurrently, conflict on one entity, recover from stale state, cancellation, device loss, DCC hang, partial artifact, app restart, and retry-failed-only without data loss, focus steal, or authority drift.", "Every required vendor capability and professional visual/metric artifact is directly inspected against pinned references; parity claim remains unavailable until the independent validator later accepts the implemented evidence."], proof: ["cargo test -p handshake_core --test tailor_professional_e2e", "cargo test -p handshake-native --test tailor_professional_e2e"], risk: "Independent feature tests can pass while the real production lifecycle, recovery, manual, parallel-agent, or parity claim fails." }),
  newMt({ number: 782, group: "BuildReadinessPlatform", summary: "Lock Tailor implementation inputs before dependent solver, renderer, DCC, and performance work: crate/compiler versions, quality-profile numbers, tolerances, seeds/repeats/timeouts, fixture ids/hashes, hardware/backend/driver matrix, memory/performance/cancellation budgets, sparse solver decision, and DCC executable/version/hash configuration.", deps: ["MT-001"], paths: ["tailor-solver/src/implementation_profile.rs", "src/backend/handshake_core/src/tailor/implementation_profile.rs", ".GOV/fixtures/tailor/implementation_profile/"], criteria: ["Values that require measurement are owned by bounded benchmark/calibration procedures with acceptance rules; the contract does not invent constants before evidence and freezes accepted outputs as versioned profiles.", "Profiles separate reference/interactive/final quality, use case, backend, hardware, compiler, driver, asset corpus, units, tolerances, seeds, repeats, timeouts, memory, performance, cancellation, and jitter with content hashes.", "Missing, stale, incompatible, or unmeasured inputs fail dependent work with typed blockers; profile changes invalidate affected baselines and trigger explicit requalification."], proof: ["cargo test -p tailor-solver implementation_profile", "cargo test -p handshake_core tailor_implementation_profile"], risk: "Architecture-ready physics can still fail implementation because numerical, hardware, asset, and DCC decisions remain implicit or guessed." })
];

function bodyKitV2Mt({ number, group, summary, deps, paths, acceptance, proof, gui = null }) {
  return newMt({
    number,
    group,
    summary,
    deps,
    paths,
    criteria: [
      acceptance,
      "Inputs, outputs, schemas, units, version/hash identities, capability state, authority/artifact ownership, privacy scope, diagnostics, cancellation/retry and unsupported behavior are explicit; a no-context implementer must not infer a provider, download, path, threshold or fallback.",
      "Automated proof covers a successful offline Rust path plus missing/corrupt/stale/incompatible input, cancellation or adapter-removal as applicable; it verifies EventLedger/ArtifactStore/Flight Recorder consistency, no partial mutation, no hidden network/Python process and reproducible receipts."
    ],
    proof: [proof],
    risk: `Without ${summary.toLowerCase()}, BodyKit-v2 could ship an ambiguous, non-reproducible or non-professional production path.`,
    gui,
    manualChapter: chapterByGroup[group]
  });
}

const bodyKitV2Mts = [
  bodyKitV2Mt({ number: 783, group: "NativeRuntimeSupply", summary: "Implement the Rust-owned Tailor dependency acquisition tool and immutable TailorDependencyBundleV1 for crates, SDKs, models, weights, configs, shaders and fixtures.", deps: ["MT-782"], paths: ["src/backend/handshake_core/src/tailor/dependencies/", "src/backend/handshake_core/tests/tailor_dependency_bundle.rs"], acceptance: "An explicit networked acquisition job verifies published signatures/hashes where available, computes SHA-256+BLAKE3, captures license/SBOM/transitives/features/ABI/build scripts, writes a content-addressed portable bundle, and supports immutable update plus rollback.", proof: "cargo test -p handshake_core --test tailor_dependency_bundle" }),
  bodyKitV2Mt({ number: 784, group: "NativeRuntimeSupply", summary: "Enforce the Tailor Python, cloud, runtime-download and arbitrary-execution firewall across repository source, dependency graph, build, process tree and shipped bundle.", deps: ["MT-783"], paths: ["src/backend/handshake_core/src/tailor/runtime/firewall.rs", "src/backend/handshake_core/tests/tailor_runtime_firewall.rs"], acceptance: "The gate rejects .py execution, python/py/pip/conda, PyO3/libpython/PyTorch, generated executable scripts, cloud endpoints, implicit telemetry/license calls, unpinned git content and runtime downloads while allowing quarantined non-executed research evidence only.", proof: "cargo test -p handshake_core --test tailor_runtime_firewall" }),
  bodyKitV2Mt({ number: 785, group: "NativeRuntimeSupply", summary: "Implement the versioned local Tailor ML/runtime registry with precedence Rust, Candle, qualified Mojo AOT, ONNX Runtime C API, then last-resort TensorFlow C API.", deps: ["MT-783", "MT-784"], paths: ["src/backend/handshake_core/src/tailor/runtime/registry.rs", "src/backend/handshake_core/tests/tailor_runtime_registry.rs"], acceptance: "Each capability row declares graph/operators/dtypes/devices/determinism/tolerances/ABI/license/bundle hash and removal behavior; selection is explicit and fail-closed and never probes into an undeclared fallback.", proof: "cargo test -p handshake_core --test tailor_runtime_registry" }),
  bodyKitV2Mt({ number: 786, group: "NativeRuntimeSupply", summary: "Qualify Candle as BodyKit's preferred local Rust model runtime with the smallest pinned feature/device surface and no model download behavior.", deps: ["MT-785"], paths: ["src/backend/handshake_core/src/tailor/runtime/candle.rs", "src/backend/handshake_core/tests/tailor_candle_runtime.rs"], acceptance: "CPU and configured accelerator fixtures prove required tensor/operator coverage, model load by dependency-bundle hash, deterministic/tolerance behavior, device loss recovery and core build success with Candle disabled.", proof: "cargo test -p handshake_core --test tailor_candle_runtime" }),
  bodyKitV2Mt({ number: 787, group: "NativeRuntimeSupply", summary: "Qualify optional Mojo ahead-of-time kernels behind an owned stable C ABI without making Mojo the BodyKit application runtime.", deps: ["MT-785"], paths: ["src/backend/handshake_core/src/tailor/runtime/mojo_aot.rs", "src/backend/handshake_core/tests/tailor_mojo_aot.rs"], acceptance: "The adapter pins compiler/binary/ABI hashes, proves numerical parity and cancellation, scans imports/processes for Python, loads only requested qualified kernels, and leaves the Rust/Candle path usable when removed.", proof: "cargo test -p handshake_core --test tailor_mojo_aot" }),
  bodyKitV2Mt({ number: 788, group: "NativeRuntimeSupply", summary: "Implement the owned minimal Rust FFI over the official ONNX Runtime C API for locally bundled provider graphs.", deps: ["MT-785"], paths: ["src/backend/handshake_core/src/tailor/runtime/onnx_c.rs", "src/backend/handshake_core/tests/tailor_onnx_c.rs"], acceptance: "Exact runtime/execution-provider DLL identities, operator/device capability, allocator ownership, thread/cancel policy and graph I/O are pinned; missing providers fail typed and never download or widen DLL search paths.", proof: "cargo test -p handshake_core --test tailor_onnx_c" }),
  bodyKitV2Mt({ number: 789, group: "NativeRuntimeSupply", summary: "Implement a last-resort TensorFlow C API adapter that can qualify only a model proven impossible on Rust, Candle, Mojo AOT and ONNX lanes.", deps: ["MT-785"], paths: ["src/backend/handshake_core/src/tailor/runtime/tensorflow_c.rs", "src/backend/handshake_core/tests/tailor_tensorflow_c.rs"], acceptance: "Activation requires a recorded earlier-lane incapability decision, exact library/model hashes, local offline execution, a narrow ABI and adapter-removal success; absent platform packages remain a typed unsupported capability.", proof: "cargo test -p handshake_core --test tailor_tensorflow_c" }),
  bodyKitV2Mt({ number: 790, group: "NativeRuntimeSupply", summary: "Implement the optional local NVIDIA Audio2Face adapter as a Rust FFI producer of canonical BodyPerformanceTrackV1.", deps: ["MT-776", "MT-783", "MT-785"], paths: ["src/backend/handshake_core/src/tailor/adapters/audio2face.rs", "src/backend/handshake_core/tests/tailor_audio2face_adapter.rs"], acceptance: "A pre-staged hash-locked SDK/model bundle produces timestamped canonical controls with model/device/audio hashes, timing/confidence/map/loss receipts; NIM/cloud/bootstrap downloads and direct rig mutation are prohibited.", proof: "cargo test -p handshake_core --test tailor_audio2face_adapter" }),
  bodyKitV2Mt({ number: 791, group: "NativeRuntimeSupply", summary: "Define the specialist capture adapter family and provider-neutral recorded color/depth/audio/inertial/calibration stream.", deps: ["MT-783", "MT-785"], paths: ["src/backend/handshake_core/src/tailor/capture/", "src/backend/handshake_core/tests/tailor_capture_stream.rs"], acceptance: "Frames preserve device/version, intrinsics/extrinsics, lens/exposure, clocks/timecode, dropped samples and content hashes; every live adapter records a deterministic replay fixture usable with hardware disconnected.", proof: "cargo test -p handshake_core --test tailor_capture_stream" }),
  bodyKitV2Mt({ number: 792, group: "NativeRuntimeSupply", summary: "Create the neutral RigLogic/DNA fixture corpus and optional pinned OpenRigLogic C++ oracle used only to qualify the owned Rust evaluator.", deps: ["MT-429", "MT-535", "MT-783"], paths: ["src/backend/handshake_core/testdata/tailor/riglogic/", "src/backend/handshake_core/tests/tailor_riglogic_oracle.rs"], acceptance: "Fixtures cover raw/GUI controls, PSD, joint groups, blend shapes, animated maps and LODs; oracle outputs carry exact source/build hashes and may be removed after Rust parity without changing project schemas or runtime authority.", proof: "cargo test -p handshake_core --test tailor_riglogic_oracle" }),
  bodyKitV2Mt({ number: 793, group: "NativeRuntimeSupply", summary: "Enforce the canonical Rust adapter lifecycle, capability discovery and adapter-removal build matrix for every vendor SDK, runtime, device and DCC boundary.", deps: ["MT-741", "MT-742", "MT-743", "MT-783", "MT-784"], paths: ["src/backend/handshake_core/src/tailor/adapters/registry.rs", "src/backend/handshake_core/tests/tailor_adapter_removal.rs"], acceptance: "Rows distinguish core/optional-native/optional-adapter/manual/unsupported/blocked, declare ABI/file boundary/network/privacy/loss/recovery, and prove core create-edit-render-save-GLB/restricted-USD behavior with each adapter independently removed.", proof: "cargo test -p handshake_core --test tailor_adapter_removal" }),

  bodyKitV2Mt({ number: 794, group: "BodyProviderRuntime", summary: "Implement BodyProviderPackageV1 and the provider-neutral BodyEvaluatorV1 Rust trait, persistence, events, capability rows and action schemas.", deps: ["MT-478", "MT-741", "MT-742", "MT-783"], paths: ["src/backend/handshake_core/src/tailor/bodykit/providers/", "src/backend/handshake_core/tests/bodykit_provider_contract.rs"], acceptance: "Packages map provider parameters/topology/regions/landmarks/measurements/LODs/rig/correctives/materials into canonical BodyRecipe/BodySpec outputs without leaking provider names into project authority; PostgreSQL rows use full resource envelopes and RLS.", proof: "cargo test -p handshake_core --test bodykit_provider_contract" }),
  bodyKitV2Mt({ number: 795, group: "BodyProviderRuntime", summary: "Acquire, pin, inspect and register Google GNM v3 as the default local BodyKit-v2 provider package.", deps: ["MT-783", "MT-785", "MT-794"], paths: ["src/backend/handshake_core/src/tailor/bodykit/providers/gnm_v3/package.rs", "src/backend/handshake_core/testdata/tailor/gnm_v3/"], acceptance: "Weights/assets/config/license/source/model identities are content-addressed; fixtures enumerate identity/expression/pose plus eyes/teeth/tongue/landmarks/topology expectations and provider discovery selects this exact profile by default.", proof: "cargo test -p handshake_core bodykit_gnm_v3_package" }),
  bodyKitV2Mt({ number: 796, group: "BodyProviderRuntime", summary: "Port the GNM-v3 evaluation graph and required preprocessing/postprocessing to the qualified Rust runtime ladder.", deps: ["MT-786", "MT-795"], paths: ["src/backend/handshake_core/src/tailor/bodykit/providers/gnm_v3/evaluator.rs", "src/backend/handshake_core/tests/bodykit_gnm_v3_evaluator.rs"], acceptance: "Pinned reference vectors prove canonical identity/expression/pose, eye/teeth/tongue geometry, landmarks, topology, repeatability, device parity and typed unsupported operators without Python or provider-side mutable state.", proof: "cargo test -p handshake_core --test bodykit_gnm_v3_evaluator" }),
  bodyKitV2Mt({ number: 797, group: "BodyProviderRuntime", summary: "Evaluate and conditionally port an MHR provider as an optional full-body/LOD/corrective source behind BodyEvaluatorV1.", deps: ["MT-785", "MT-794"], paths: ["src/backend/handshake_core/src/tailor/bodykit/providers/mhr/", "src/backend/handshake_core/tests/bodykit_mhr_provider.rs"], acceptance: "The MT records exact license/source/assets/graph, replaces Python/PyTorch, maps identity/pose/expression/correctives/LODs explicitly, reports unsupported constructs, and cannot displace GNM-v3 default or canonical topology authority.", proof: "cargo test -p handshake_core --test bodykit_mhr_provider" }),
  bodyKitV2Mt({ number: 798, group: "BodyProviderRuntime", summary: "Implement BodyHeadPackageV1 as the canonical high-resolution head artifact family with topology, LOD, UV, landmarks, materials, oral/eye assets, rig maps and lineage.", deps: ["MT-576", "MT-794", "MT-796"], paths: ["src/backend/handshake_core/src/tailor/bodykit/head/package.rs", "src/backend/handshake_core/tests/bodykit_head_package.rs"], acceptance: "BodySpec remains parametric authority while generated meshes/textures are immutable ArtifactStore outputs; package creation/reopen/replace validates every source/provider/seed/config/profile/artifact hash and privacy scope.", proof: "cargo test -p handshake_core --test bodykit_head_package" }),
  bodyKitV2Mt({ number: 799, group: "BodyProviderRuntime", summary: "Implement HeadBodyAssemblyV1 for replaceable source head/body, neck transition, skeleton, materials, grooms, attachments and LOD correspondence.", deps: ["MT-584", "MT-798"], paths: ["src/backend/handshake_core/src/tailor/bodykit/assembly.rs", "src/backend/handshake_core/tests/bodykit_head_body_assembly.rs"], acceptance: "Assembly records seam mapping, scale/axis/unit transforms, joint/expression remaps, normals/tangents, texture continuity, attachment rules and losses; preview/replace/rebase never destructively flattens sources.", proof: "cargo test -p handshake_core --test bodykit_head_body_assembly" }),
  bodyKitV2Mt({ number: 800, group: "BodyProviderRuntime", summary: "Implement reversible region-masked multiresolution BodyResidualLayerV1 for qualified detail outside the parametric basis.", deps: ["MT-722", "MT-799"], paths: ["src/backend/handshake_core/src/tailor/bodykit/residual.rs", "src/backend/handshake_core/tests/bodykit_residual_layer.rs"], acceptance: "Residuals carry topology/provider/rig/input hashes, sparse deltas, symmetry policy, masks, author/effective values, compare/revert and LOD transfer; incompatible changes invalidate visibly and untracked sculpt overwrites fail.", proof: "cargo test -p handshake_core --test bodykit_residual_layer" }),
  bodyKitV2Mt({ number: 801, group: "BodyProviderRuntime", summary: "Implement the measured corrective-regeneration versus residual-authoring decision engine and proposal report.", deps: ["MT-531", "MT-535", "MT-800"], paths: ["src/backend/handshake_core/src/tailor/bodykit/corrective_policy.rs", "src/backend/handshake_core/tests/bodykit_corrective_policy.rs"], acceptance: "Qualified thresholds cover deformation domain, joint/skin change, topology/seam/contact/LOD invalidation and locked-region leakage; previews show parametric/residual deltas and affected correctives, and unknown thresholds block apply rather than guessing.", proof: "cargo test -p handshake_core --test bodykit_corrective_policy" }),
  bodyKitV2Mt({ number: 802, group: "BodyProviderRuntime", summary: "Implement calibrated scan/reference fitting as an immutable-observation proposal pipeline split into parametric and residual contributions.", deps: ["MT-768", "MT-798", "MT-800"], paths: ["src/backend/handshake_core/src/tailor/bodykit/scan_fit.rs", "src/backend/handshake_core/tests/bodykit_scan_fit.rs"], acceptance: "Camera/calibration/landmarks/dense correspondence/occlusion/scale/stages/confidence/error/locks/rejected observations and deterministic seed are persisted; single-view ambiguity and infeasible locked measurements remain visible.", proof: "cargo test -p handshake_core --test bodykit_scan_fit" }),
  bodyKitV2Mt({ number: 803, group: "BodyProviderRuntime", summary: "Implement native high-resolution face identity/expression geometry layers, regional controls, asymmetry, pore/wrinkle detail and corrective authoring.", deps: ["MT-762", "MT-798"], paths: ["src/backend/handshake_core/src/tailor/bodykit/face/high_resolution.rs", "src/backend/handshake_core/tests/bodykit_high_resolution_face.rs"], acceptance: "Identity and expression spaces remain separate; global-to-local regions, symmetry/asymmetry, FACS/ARKit/viseme mapping, detail/residual/corrective order and locked-region behavior are deterministic and exportable.", proof: "cargo test -p handshake_core --test bodykit_high_resolution_face" }),
  bodyKitV2Mt({ number: 804, group: "BodyProviderRuntime", summary: "Implement the coupled cinematic eye, eyelid, lash, brow, lip, teeth, gum, tongue and oral-cavity system.", deps: ["MT-803"], paths: ["src/backend/handshake_core/src/tailor/bodykit/face/eye_oral.rs", "src/backend/handshake_core/tests/bodykit_eye_oral_system.rs"], acceptance: "Geometry/rig/material/contact checks cover gaze convergence, cornea/iris/pupil, lid-globe contact and blink closure, tear line, lip seal/roll, teeth/gum/tongue collision and phoneme closure through representative poses.", proof: "cargo test -p handshake_core --test bodykit_eye_oral_system" }),
  bodyKitV2Mt({ number: 805, group: "BodyProviderRuntime", summary: "Implement the owned Rust OpenRigLogic-compatible evaluator over a versioned neutral rig-logic graph.", deps: ["MT-723", "MT-792", "MT-798"], paths: ["src/backend/handshake_core/src/tailor/bodykit/riglogic/", "src/backend/handshake_core/tests/bodykit_riglogic_rust.rs"], acceptance: "Raw/GUI controls, PSD, joint groups, blend shapes, animated maps and LOD semantics match pinned oracle fixtures within declared tolerances; import returns graph+losses and unsupported DNA constructs fail typed.", proof: "cargo test -p handshake_core --test bodykit_riglogic_rust" }),
  bodyKitV2Mt({ number: 806, group: "BodyProviderRuntime", summary: "Implement the canonical face/figure rig solve graph, driver mapping, execution stack and debug state consumed by native tools and exports.", deps: ["MT-723", "MT-805"], paths: ["src/backend/handshake_core/src/tailor/bodykit/rig/solve_graph.rs", "src/backend/handshake_core/tests/bodykit_rig_solve_graph.rs"], acceptance: "Stable node/socket/edge IDs, typed sockets, ordering, groups, bypass/lock, build state, curves, failing-node focus, version diff, replay and neutral export sidecars use one authority and reject cycles/missing dependencies.", proof: "cargo test -p handshake_core --test bodykit_rig_solve_graph" }),
  bodyKitV2Mt({ number: 807, group: "BodyProviderRuntime", summary: "Implement topology/rig/corrective/material/groom transfer across the declared BodyKit LOD family and source replacements.", deps: ["MT-703", "MT-799", "MT-805"], paths: ["src/backend/handshake_core/src/tailor/bodykit/lod_transfer.rs", "src/backend/handshake_core/tests/bodykit_lod_transfer.rs"], acceptance: "Every LOD preserves stable correspondence, joints/weights, morph/residual/corrective intent, UV/material regions, groom binding and measurement tolerances; unresolved regions and quality losses block qualification.", proof: "cargo test -p handshake_core --test bodykit_lod_transfer" }),

  bodyKitV2Mt({ number: 808, group: "CinematicPerformance", summary: "Implement BodyPerformanceTrackV1 as the canonical timestamped rig-neutral performance, expression, gaze, viseme, contact and confidence representation.", deps: ["MT-581", "MT-776", "MT-805"], paths: ["src/backend/handshake_core/src/tailor/bodykit/performance/track.rs", "src/backend/handshake_core/tests/bodykit_performance_track.rs"], acceptance: "Manual keys, capture, audio solvers and imported animation map into versioned canonical controls with source timecode/calibration/discontinuities/provenance; producers never mutate the rig or overwrite immutable originals.", proof: "cargo test -p handshake_core --test bodykit_performance_track" }),
  bodyKitV2Mt({ number: 809, group: "CinematicPerformance", summary: "Implement capture synchronization, calibration, dropped-frame accounting and record/replay into BodyPerformanceTrack observations.", deps: ["MT-791", "MT-808"], paths: ["src/backend/handshake_core/src/tailor/bodykit/performance/capture_sync.rs", "src/backend/handshake_core/tests/bodykit_capture_sync.rs"], acceptance: "Color/depth/audio/inertial clocks align to canonical timecode with uncertainty and discontinuity records; live and replayed fixtures produce equivalent bounded observations after device removal.", proof: "cargo test -p handshake_core --test bodykit_capture_sync" }),
  bodyKitV2Mt({ number: 810, group: "CinematicPerformance", summary: "Implement the staged facial/body performance solve from synchronized observations through identity stabilization, expression/body solve, temporal filtering, retarget and correctives.", deps: ["MT-805", "MT-808", "MT-809"], paths: ["src/backend/handshake_core/src/tailor/bodykit/performance/solve.rs", "src/backend/handshake_core/tests/bodykit_performance_solve.rs"], acceptance: "Every stage is checkpointed and replayable with parameters/confidence/errors; filtering preserves blinks, saccades, closures and intentional high frequency motion and surfaces contact/occlusion failures.", proof: "cargo test -p handshake_core --test bodykit_performance_solve" }),
  bodyKitV2Mt({ number: 811, group: "CinematicPerformance", summary: "Implement performance takes, clips, layers, markers, timecode, dope-sheet/curve state, retiming and selective bake commands.", deps: ["MT-705", "MT-808", "MT-810"], paths: ["src/backend/handshake_core/src/tailor/bodykit/performance/edit.rs", "src/backend/handshake_core/tests/bodykit_performance_edit.rs"], acceptance: "Exact values/interpolation/tangents, loop/range, live/cached/final/stale state, before-after compare and reversible edits preserve immutable source tracks and semantic undo.", proof: "cargo test -p handshake_core --test bodykit_performance_edit" }),
  bodyKitV2Mt({ number: 812, group: "CinematicPerformance", summary: "Implement shot-aware cinematic appearance layers for wrinkles, compression/stretch, flush, sweat, tears, saliva, eye/mouth wetness and peach fuzz.", deps: ["MT-726", "MT-808"], paths: ["src/backend/handshake_core/src/tailor/bodykit/appearance/dynamic.rs", "src/backend/handshake_core/tests/bodykit_dynamic_appearance.rs"], acceptance: "Each layer has authored/effective values, region mask, source/provenance, temporal response, quality tier, renderer support and interchange losses and remains independently previewable/reversible.", proof: "cargo test -p handshake_core --test bodykit_dynamic_appearance" }),
  bodyKitV2Mt({ number: 813, group: "CinematicPerformance", summary: "Qualify eye, mouth, tongue, teeth, wetline and speech-contact behavior in static and animated cinematic fixtures.", deps: ["MT-804", "MT-810", "MT-812"], paths: ["src/backend/handshake_core/testdata/tailor/cinematic/eye_mouth/", "src/backend/handshake_core/tests/bodykit_eye_mouth_cinematic.rs"], acceptance: "Named fixtures measure gaze, blink seal, lid contact, lip seal/roll, phoneme closures, collision, wetline/specular continuity and temporal pops and attach exact frames/captures/findings to verdicts.", proof: "cargo test -p handshake_core --test bodykit_eye_mouth_cinematic" }),
  bodyKitV2Mt({ number: 814, group: "CinematicPerformance", summary: "Implement coupled neck, shoulder, clavicle, jaw, hairline, collar and groom-attachment deformation qualification.", deps: ["MT-775", "MT-799", "MT-810"], paths: ["src/backend/handshake_core/src/tailor/bodykit/deformation/waist_up.rs", "src/backend/handshake_core/tests/bodykit_waist_up_deformation.rs"], acceptance: "Pose/speech/gaze/cloth fixtures detect seam slide, volume loss, twist, texture discontinuity, groom float, collar penetration and corrective pops across named ranges and LODs.", proof: "cargo test -p handshake_core --test bodykit_waist_up_deformation" }),
  bodyKitV2Mt({ number: 815, group: "CinematicPerformance", summary: "Harden cinematic scalp, hair, brow, lash and body-groom authoring, dynamics, wetness, LOD and render behavior.", deps: ["MT-775", "MT-814"], paths: ["src/backend/handshake_core/src/tailor/bodykit/groom/cinematic.rs", "src/backend/handshake_core/tests/bodykit_cinematic_groom.rs"], acceptance: "Guide/strand/card assets, masks, collision/dynamics and attachment survive morph, performance, cloth contact, wet/dry states, render tiers and USD-curves-first export without float, explosion or silent loss.", proof: "cargo test -p handshake_core --test bodykit_cinematic_groom" }),
  bodyKitV2Mt({ number: 816, group: "CinematicPerformance", summary: "Harden cinematic skin/material evaluation using OpenPBR-aligned layers, UDIM detail, displacement, wetness, transmission and color provenance.", deps: ["MT-726", "MT-765", "MT-812"], paths: ["src/backend/handshake_core/src/tailor/bodykit/materials/cinematic_skin.rs", "src/backend/handshake_core/tests/bodykit_cinematic_skin.rs"], acceptance: "Hero close-up fixtures cover light/dark skin, pore/wrinkle detail, seams, displacement, roughness/specular/wetness/transmission response, color transforms and explicit projection losses across quality tiers.", proof: "cargo test -p handshake_core --test bodykit_cinematic_skin" }),
  bodyKitV2Mt({ number: 817, group: "CinematicPerformance", summary: "Define cinematic waist-up camera, lens, light, environment and native render qualification profiles.", deps: ["MT-766", "MT-813", "MT-814", "MT-816"], paths: ["src/backend/handshake_core/src/tailor/bodykit/render/cinematic_profile.rs", "src/backend/handshake_core/tests/bodykit_cinematic_render.rs"], acceptance: "Profiles pin camera/lens/aperture/focus, lights/environment, renderer/device/color config, resolution/sampling/denoise, safe frame and interactive/final tiers with exact reproducible project/artifact identities.", proof: "cargo test -p handshake_core --test bodykit_cinematic_render" }),
  bodyKitV2Mt({ number: 818, group: "CinematicPerformance", summary: "Implement BodyConditioningBundleV1 for local image/scene generation with beauty, typed AOVs, camera/light state, timing, IDs and provenance.", deps: ["MT-767", "MT-817"], paths: ["src/backend/handshake_core/src/tailor/bodykit/conditioning/bundle.rs", "src/backend/handshake_core/tests/bodykit_conditioning_bundle.rs"], acceptance: "Bundles enumerate depth/normals/position/motion/UV/material/part/landmark/skeleton/mask/albedo/roughness/metalness/emission/contact channels, units/color/frame semantics, source versions and hashes; omissions/approximations are explicit.", proof: "cargo test -p handshake_core --test bodykit_conditioning_bundle" }),
  bodyKitV2Mt({ number: 819, group: "CinematicPerformance", summary: "Implement Rust EXR multipart/arbitrary-channel write and reload for BodyConditioningBundle sequences.", deps: ["MT-818"], paths: ["src/backend/handshake_core/src/tailor/bodykit/conditioning/exr.rs", "src/backend/handshake_core/tests/bodykit_conditioning_exr.rs"], acceptance: "Channel names/types, parts, dimensions/data windows, timing, camera/light metadata, color spaces and hashes survive reload; deep/DWA or unqualified features return typed unsupported without flattening.", proof: "cargo test -p handshake_core --test bodykit_conditioning_exr" }),
  bodyKitV2Mt({ number: 820, group: "CinematicPerformance", summary: "Create the immutable cinematic waist-up frame and sequence corpus plus direct-inspection sampling contract.", deps: ["MT-813", "MT-814", "MT-817", "MT-819"], paths: ["src/backend/handshake_core/testdata/tailor/cinematic/corpus/", "src/backend/handshake_core/tests/bodykit_cinematic_corpus.rs"], acceptance: "The corpus covers close/medium/profile, neutral/expression/speech/blink/gaze/head-neck-shoulder motion, skin/groom/collar/contact variation, interactive/final renders, exact references, artifact hashes and required frame+temporal findings.", proof: "cargo test -p handshake_core --test bodykit_cinematic_corpus" }),

  bodyKitV2Mt({ number: 821, group: "InterchangeDelivery", summary: "Implement TailorInterchangeProfileV1 as the single direction-specific payload, capability, loss and DCC qualification matrix.", deps: ["MT-703", "MT-740", "MT-783"], paths: ["src/backend/handshake_core/src/tailor/interchange/profile.rs", "src/backend/handshake_core/tests/tailor_interchange_profile.rs"], acceptance: "Rows pin format/version/direction, schemas/extensions, units/axes, payload classes, materials/color, animation/cache, importer/writer/adapter identity, automation class, losses, unsupported cases and fixture receipts; menus/actions derive from rows.", proof: "cargo test -p handshake_core --test tailor_interchange_profile" }),
  bodyKitV2Mt({ number: 822, group: "InterchangeDelivery", summary: "Harden the deterministic Rust GLB writer and independent Rust reload for BodyKit and Cloth production packages.", deps: ["MT-609", "MT-821"], paths: ["src/backend/handshake_core/src/tailor/interchange/glb.rs", "src/backend/handshake_core/tests/tailor_glb_roundtrip.rs"], acceptance: "Meshes/topology/normals/tangents/UVs/materials/textures/joints/weights/morphs/animation/cameras/lights and manifest semantics round-trip within typed tolerances and pass the pinned Khronos validator where available.", proof: "cargo test -p handshake_core --test tailor_glb_roundtrip" }),
  bodyKitV2Mt({ number: 823, group: "InterchangeDelivery", summary: "Qualify a restricted pure-Rust USD profile for USDA, UsdSkel, mesh time samples, curves, LOD variants, cameras, lights and material bindings.", deps: ["MT-616", "MT-821"], paths: ["src/backend/handshake_core/src/tailor/interchange/usd.rs", "src/backend/handshake_core/tests/tailor_usd_restricted.rs"], acceptance: "Only exact passing schemas are exposed; USDA may qualify first, while USDC/USDZ/broad schema claims fail typed until proven; write/read structural fixtures record crate/source/version/hash and losses.", proof: "cargo test -p handshake_core --test tailor_usd_restricted" }),
  bodyKitV2Mt({ number: 824, group: "InterchangeDelivery", summary: "Implement the canonical OpenPBR material vocabulary and qualified MaterialX/glTF/USD projections with machine-readable loss reports.", deps: ["MT-764", "MT-765", "MT-821"], paths: ["src/backend/handshake_core/src/tailor/interchange/materials.rs", "src/backend/handshake_core/tests/tailor_material_interchange.rs"], acceptance: "Every supported, approximated, baked, dropped or transformed field and texture carries source/color provenance; MaterialX version/subset and target translations are pinned and reload-tested.", proof: "cargo test -p handshake_core --test tailor_material_interchange" }),
  bodyKitV2Mt({ number: 825, group: "InterchangeDelivery", summary: "Implement a qualified Rust-owned OCIO transform subset and optional pinned OCIO C adapter only for unsupported required transforms.", deps: ["MT-824"], paths: ["src/backend/handshake_core/src/tailor/color/ocio.rs", "src/backend/handshake_core/tests/tailor_ocio.rs"], acceptance: "Config/display/view/look/space identities and hashes, transform direction, precision/error tolerances and adapter use are recorded; missing transforms fail typed and adapter removal preserves qualified core transforms.", proof: "cargo test -p handshake_core --test tailor_ocio" }),
  bodyKitV2Mt({ number: 826, group: "InterchangeDelivery", summary: "Implement the optional local Unreal native C++ Interchange editor plugin/commandlet adapter and structured qualification receipt.", deps: ["MT-778", "MT-821", "MT-823"], paths: ["src/backend/handshake_core/src/tailor/adapters/unreal.rs", "src/backend/handshake_core/testdata/tailor/unreal_commandlet/", "src/backend/handshake_core/tests/tailor_unreal_commandlet.rs"], acceptance: "Exact engine/plugin/binary hashes and disabled-Python state are recorded; offline import verifies only declared skeletal/morph/animation/material/groom/LOD/cloth/camera/light capabilities and returns JSON logs/artifacts/captures/losses.", proof: "cargo test -p handshake_core --test tailor_unreal_commandlet" }),
  bodyKitV2Mt({ number: 827, group: "InterchangeDelivery", summary: "Implement the operator-driven Blender manual qualification profile, checklist, capture intake and structured evidence receipt.", deps: ["MT-778", "MT-821", "MT-822", "MT-823"], paths: ["src/backend/handshake_core/src/tailor/adapters/blender_manual.rs", "src/backend/handshake_core/tests/tailor_blender_manual_receipt.rs"], acceptance: "The receipt pins executable/version/hash, hardened startup options, package hashes, required open/import/inspect steps, screenshots/observations/losses and reviewer; it never labels manual evidence automated or launches Blender foreground.", proof: "cargo test -p handshake_core --test tailor_blender_manual_receipt" }),
  bodyKitV2Mt({ number: 828, group: "InterchangeDelivery", summary: "Implement optional pinned FBX import through a small Rust-owned loader FFI and typed unsupported FBX export behavior.", deps: ["MT-366", "MT-410", "MT-438", "MT-821"], paths: ["src/backend/handshake_core/src/tailor/interchange/fbx.rs", "src/backend/handshake_core/tests/tailor_fbx_capability.rs"], acceptance: "Import declares supported meshes/rigs/animation/materials and loader ABI/hash with hostile-input bounds; export returns unsupported_capability with alternatives until a separately qualified no-Python writer exists.", proof: "cargo test -p handshake_core --test tailor_fbx_capability" }),
  bodyKitV2Mt({ number: 829, group: "InterchangeDelivery", summary: "Implement optional limited Alembic/Ogawa import capability and typed unsupported export while canonical caches use USD/native forms.", deps: ["MT-440", "MT-821", "MT-823"], paths: ["src/backend/handshake_core/src/tailor/interchange/alembic.rs", "src/backend/handshake_core/tests/tailor_alembic_capability.rs"], acceptance: "Qualified object/property/sample classes, time bases, topology constraints, reader ABI/hash and hostile-input limits are explicit; unqualified properties and every export request fail typed without DCC conversion.", proof: "cargo test -p handshake_core --test tailor_alembic_capability" }),
  bodyKitV2Mt({ number: 830, group: "InterchangeDelivery", summary: "Implement the canonical structural round-trip comparator and interchange fixture matrix for geometry, rig, animation, materials, grooms, cameras, lights, caches and conditioning.", deps: ["MT-822", "MT-823", "MT-824", "MT-825", "MT-826", "MT-827", "MT-828", "MT-829"], paths: ["src/backend/handshake_core/src/tailor/interchange/compare.rs", "src/backend/handshake_core/tests/tailor_interchange_matrix.rs"], acceptance: "Absolute/relative tolerances and stable correspondences cover topology/positions/normals/UVs/joints/weights/morph deltas/timing/LODs/groom/material/color/camera/light/AOV data; visual evidence cannot erase a structural mismatch.", proof: "cargo test -p handshake_core --test tailor_interchange_matrix" }),

  bodyKitV2Mt({ number: 831, group: "NativeOperatorSurfaces", summary: "Register TAILOR in the WP-KERNEL-012 native module switcher and contribute all Tailor actions to FILE EDIT VIEW GO RUN HELP without a private shell.", deps: ["MT-741", "MT-748", "MT-750"], paths: ["src/frontend/handshake_native/src/tailor/module_registration.rs", "src/frontend/handshake_native/tests/tailor_module_registration.rs"], acceptance: "module_id Tailor, label TAILOR, author_id module-tailor and default route tailor.project support keyboard/AccessKit/backend navigation, layout restore and capability blockers; menus/palette/shortcuts/model/replay share one descriptor.", proof: "cargo test -p handshake-native --test tailor_module_registration", gui: { surfaces: ["Handshake native shell module switcher and top-bar menus"], targets: ["module-tailor", "tailor-project", "tailor-menu-file", "tailor-menu-edit", "tailor-menu-view", "tailor-menu-go", "tailor-menu-run", "tailor-menu-help"] } }),
  bodyKitV2Mt({ number: 832, group: "NativeOperatorSurfaces", summary: "Implement the professional Tailor workspace shell, pane layout and Cloth/BodyKit task-tab registry.", deps: ["MT-692", "MT-831"], paths: ["src/frontend/handshake_native/src/tailor/workspace.rs", "src/frontend/handshake_native/tests/tailor_workspace.rs"], acceptance: "Project tabs, module selection and pane-local task tabs remain distinct; tool options, tool rail, central editors, outliner/assets, Property Inspector, timeline/jobs/problems/history and status strip compose recoverable per-project layouts.", proof: "cargo test -p handshake-native --test tailor_workspace", gui: { surfaces: ["Tailor professional multi-pane workspace"], targets: ["tailor-task-tabs", "tailor-tool-options", "tailor-tool-rail", "tailor-center-editor", "tailor-outliner", "tailor-properties", "tailor-bottom-drawer", "tailor-status-strip"] } }),
  bodyKitV2Mt({ number: 833, group: "NativeOperatorSurfaces", summary: "Implement TailorActiveToolStateV1 and shared 2D/3D selection, transforms, gizmos, snapping, measurement, brush, mask and preview/apply tools.", deps: ["MT-690", "MT-693", "MT-748", "MT-832"], paths: ["src/backend/handshake_core/src/tailor/tools/active_tool.rs", "src/frontend/handshake_native/src/tailor/tools/", "src/frontend/handshake_native/tests/tailor_active_tools.rs"], acceptance: "Axis/plane/orientation/pivot/snap/fine-coarse constraints, slider+numeric authored/effective units, keyboard/model equivalents, stable entity IDs and cancel-before-one-semantic-commit are proven across pattern, body, rig, weight, UV and contact fixtures.", proof: "cargo test -p handshake-native --test tailor_active_tools", gui: { surfaces: ["Tailor shared tool rail, contextual options and viewport gizmos"], targets: ["tailor-active-tool", "tailor-gizmo", "tailor-snap", "tailor-pivot", "tailor-numeric-entry", "tailor-measure", "tailor-brush", "tailor-mask", "tailor-preview-apply"] } }),
  bodyKitV2Mt({ number: 834, group: "NativeOperatorSurfaces", summary: "Build the BodyKit Creator provider/preset/recipe browser and searchable regional parameter editor with slider-plus-exact-numeric controls.", deps: ["MT-627", "MT-628", "MT-633", "MT-798", "MT-833"], paths: ["src/frontend/handshake_native/src/tailor/bodykit/creator.rs", "src/frontend/handshake_native/tests/bodykit_creator_v2.rs"], acceptance: "Favorites/recent/locked/pinned channels, region accordions, units/authored/effective/reset/range/over-range, deterministic scoped randomization, partial recipe apply, provider/package/profile state and preview/diff/apply/revert use stable IDs and backend parity.", proof: "cargo test -p handshake-native --test bodykit_creator_v2", gui: { surfaces: ["BodyKit Creator shape workspace"], targets: ["tailor-bodykit-provider", "tailor-bodykit-recipe-browser", "tailor-bodykit-parameter-search", "tailor-bodykit-region", "tailor-bodykit-channel-slider", "tailor-bodykit-channel-numeric", "tailor-bodykit-preview-bar"] } }),
  bodyKitV2Mt({ number: 835, group: "NativeOperatorSurfaces", summary: "Build the BodyKit measurements and scan-fit workspace with pinned viewport lines, targets, residuals, locks, confidence and source comparison.", deps: ["MT-629", "MT-802", "MT-834"], paths: ["src/frontend/handshake_native/src/tailor/bodykit/measurements.rs", "src/frontend/handshake_native/tests/bodykit_measurements_v2.rs"], acceptance: "Pin/unpin, exact units/targets, infeasible residuals, locked-region leakage, observation confidence/error, synchronized reference compare and selective parametric/residual apply are command-backed and persist through reload.", proof: "cargo test -p handshake-native --test bodykit_measurements_v2", gui: { surfaces: ["BodyKit measurements and scan-fit workspace"], targets: ["tailor-bodykit-measurement-list", "tailor-bodykit-measurement-pin", "tailor-bodykit-measurement-target", "tailor-bodykit-measurement-residual", "tailor-bodykit-scan-source", "tailor-bodykit-fit-error", "tailor-bodykit-fit-apply"] } }),
  bodyKitV2Mt({ number: 836, group: "NativeOperatorSurfaces", summary: "Build the BodyKit geometry, morph, residual and corrective authoring workspace with topology outliner, sculpt tools, stacks, samples and RBF graph.", deps: ["MT-722", "MT-800", "MT-801", "MT-833"], paths: ["src/frontend/handshake_native/src/tailor/bodykit/geometry.rs", "src/frontend/handshake_native/tests/bodykit_geometry_v2.rs"], acceptance: "The UI distinguishes parameter/morph/residual/corrective layers, shows invalidation and locked leakage, supports before-after/version diff and exposes stable graph/mesh/entity controls with backend-only equivalents.", proof: "cargo test -p handshake-native --test bodykit_geometry_v2", gui: { surfaces: ["BodyKit geometry and corrective workspace"], targets: ["tailor-bodykit-topology-outliner", "tailor-bodykit-sculpt-tool", "tailor-bodykit-morph-stack", "tailor-bodykit-residual-stack", "tailor-bodykit-corrective-samples", "tailor-bodykit-rbf-graph", "tailor-bodykit-geometry-compare"] } }),
  bodyKitV2Mt({ number: 837, group: "NativeOperatorSurfaces", summary: "Build the BodyKit rig, solve-graph, weight/rigidity-paint and transfer workspaces with hierarchy, execution debugging and error overlays.", deps: ["MT-723", "MT-724", "MT-725", "MT-805", "MT-807", "MT-833"], paths: ["src/frontend/handshake_native/src/tailor/bodykit/rig.rs", "src/frontend/handshake_native/tests/bodykit_rig_v2.rs"], acceptance: "Hierarchy/joint limits/orientation/roll, solve direction/build/failing node, influence locks/normalization/symmetry/heatmap/probe and transfer correspondence/error/unresolved regions support preview/diff/apply/rollback and stable navigation.", proof: "cargo test -p handshake-native --test bodykit_rig_v2", gui: { surfaces: ["BodyKit rig, weights and transfer workspaces"], targets: ["tailor-bodykit-rig-hierarchy", "tailor-bodykit-rig-viewport", "tailor-bodykit-rig-graph", "tailor-bodykit-rig-execution", "tailor-bodykit-weight-brush", "tailor-bodykit-influences", "tailor-bodykit-transfer-error"] } }),
  bodyKitV2Mt({ number: 838, group: "NativeOperatorSurfaces", summary: "Build the BodyKit face and performance workspace for identity/expression, FACS/ARKit/visemes, eyes/jaw/tongue, correctives, takes, timeline and curves.", deps: ["MT-762", "MT-808", "MT-811", "MT-833"], paths: ["src/frontend/handshake_native/src/tailor/bodykit/face_performance.rs", "src/frontend/handshake_native/tests/bodykit_face_performance.rs"], acceptance: "Identity versus expression, symmetry/asymmetry, rig/control mapping, confidence/findings, source/edit layers, dope sheet, curve editor, exact values/tangents, retime/bake and synchronized viewport/timeline/Problems selection are explicit.", proof: "cargo test -p handshake-native --test bodykit_face_performance", gui: { surfaces: ["BodyKit face and performance workspace"], targets: ["tailor-bodykit-face-identity", "tailor-bodykit-face-expression", "tailor-bodykit-face-rig-graph", "tailor-bodykit-performance-takes", "tailor-bodykit-dope-sheet", "tailor-bodykit-curve-editor", "tailor-bodykit-performance-findings"] } }),
  bodyKitV2Mt({ number: 839, group: "NativeOperatorSurfaces", summary: "Build integrated BodyKit skin/material, groom, cinematic render and conditioning workspaces over shared native services.", deps: ["MT-726", "MT-764", "MT-765", "MT-775", "MT-817", "MT-818", "MT-833"], paths: ["src/frontend/handshake_native/src/tailor/bodykit/lookdev.rs", "src/frontend/handshake_native/tests/bodykit_lookdev_v2.rs"], acceptance: "UDIM/layers/channels/masks/OpenPBR losses, guide/strand/card groom tools, camera/light/quality/AOV/region render, queue/history/filmstrip/compare and conditioning pass/channel/hash inspection remain one action/job/artifact authority.", proof: "cargo test -p handshake-native --test bodykit_lookdev_v2", gui: { surfaces: ["BodyKit skin, groom, render and conditioning workspaces"], targets: ["tailor-bodykit-udim-browser", "tailor-bodykit-material-layers", "tailor-bodykit-groom-tools", "tailor-bodykit-render-viewport", "tailor-bodykit-render-queue", "tailor-bodykit-render-library", "tailor-bodykit-conditioning-channels"] } }),
  bodyKitV2Mt({ number: 840, group: "ProjectPillars", summary: "Harden Tailor multi-project save/recovery, semantic history/compare, scoped CRDT/leases and account-resource privacy across every BodyKit-v2 resource.", deps: ["MT-694", "MT-695", "MT-702", "MT-742", "MT-754", "MT-783", "MT-831"], paths: ["src/backend/handshake_core/src/tailor/projects_v2.rs", "src/frontend/handshake_native/src/tailor/project_tabs.rs", "src/backend/handshake_core/tests/tailor_parallel_projects_privacy.rs"], acceptance: "Every command/job/layout/selection/cache/receipt carries project_id; two projects run concurrently without retargeting, CRDT applies only to documented convergent data, semantic conflicts use revisions/leases/proposals, and RLS/ResourceBroker negative tests cover all source/derived resources.", proof: "cargo test -p handshake_core --test tailor_parallel_projects_privacy", gui: { surfaces: ["Tailor project tabs, save/recovery, history and compare"], targets: ["tailor-project-tabs", "tailor-project-save-state", "tailor-project-checkpoint", "tailor-project-recover", "tailor-history", "tailor-compare", "tailor-actor-lease", "tailor-access-space"] } }),
  bodyKitV2Mt({ number: 841, group: "ProjectPillars", summary: "Complete Tailor Loom/Locus/Flight Recorder/internal-diagnostics/Palmistry integration, Jobs/Problems/history surfaces and model navigation/manual discovery for BodyKit-v2.", deps: ["MT-743", "MT-744", "MT-745", "MT-747", "MT-748", "MT-831", "MT-832", "MT-840"], paths: ["src/backend/handshake_core/src/tailor/interops/bodykit_v2.rs", "src/frontend/handshake_native/src/tailor/operations_drawer.rs", "src/backend/handshake_core/tests/tailor_bodykit_v2_pillars.rs"], acceptance: "Locus is optional production work tracking, Loom/PKI links knowledge, Flight Recorder projects EventLedger, Palmistry watches declared jobs, and stable surface/scene/selection/job/problem/history/manual/viewport routes expose canonical totals, actors, receipts, artifacts and recovery without screen scraping.", proof: "cargo test -p handshake_core --test tailor_bodykit_v2_pillars", gui: { surfaces: ["Tailor Jobs, Problems, history, diagnostics and model-navigation surfaces"], targets: ["tailor-jobs", "tailor-problems", "tailor-history-drawer", "tailor-diagnostics", "tailor-pillar-status", "tailor-manual-anchor", "tailor-model-navigation", "tailor-canonical-total"] } }),
  bodyKitV2Mt({ number: 842, group: "CinematicQualification", summary: "Prove the complete offline no-context Tailor/BodyKit-v2 movie-production lifecycle with two parallel projects and directly inspected lifelike waist-up still and sequence outputs.", deps: ["MT-653", "MT-709", "MT-781", "MT-820", "MT-830", "MT-831", "MT-832", "MT-833", "MT-834", "MT-835", "MT-836", "MT-837", "MT-838", "MT-839", "MT-840", "MT-841"], paths: ["src/backend/handshake_core/tests/tailor_bodykit_v2_e2e.rs", "src/frontend/handshake_native/tests/tailor_bodykit_v2_e2e.rs", "src/backend/handshake_core/testdata/tailor/bodykit_v2_e2e/"], acceptance: "From a staged offline bundle, create/recover two projects; build a GNM-v3 assembled character; edit face/anatomy/rig/weights/material/groom/performance and Cloth; render/inspect frame+sequence+AOVs; reload GLB/USD; qualify Unreal/manual Blender; inject dependency/device/solver/adapter/conflict failures; recover/replay; verify PostgreSQL privacy, artifacts, EventLedger/Flight Recorder, Loom/Locus, diagnostics, Argus/AccessKit and UserManual consistency.", proof: "cargo test -p handshake-native --test tailor_bodykit_v2_e2e", gui: { surfaces: ["Tailor BodyKit-v2 final professional lifecycle"], targets: ["module-tailor", "tailor-bodykit-creator", "tailor-bodykit-face-performance", "tailor-bodykit-lookdev", "tailor-bodykit-render-library", "tailor-bodykit-interchange", "tailor-jobs", "tailor-problems", "tailor-bodykit-v2-e2e-verdict"] } })
];

function processExistingMt(mt, group) {
  mt.updated_at_utc = UPDATED_AT;
  mt.authority_files.build_readiness_prework = BUILD_READY_PATH;
  mt.authority_files.technical_refinement_prework = TECHNICAL_REFINEMENT_PATH;
  mt.authority_files.multi_lens_review = REVIEW_PATH;
  mt.artifact_policy.projection_creation = "ON_OPERATOR_REQUEST_ONLY";
  mt.markdown_projection = {
    path: "NOT_GENERATED_BY_DEFAULT",
    status: "NOT_GENERATED_BY_DEFAULT",
    source_hash: "NOT_GENERATED_BY_DEFAULT",
    projection_hash: "NOT_GENERATED_BY_DEFAULT",
    generated_at_utc: null,
    generator: "PENDING_DEMAND"
  };

  if (nonUiProducerIds.has(mt.mt_id)) {
    mt.gui_obligation.operator_surface_required = false;
    mt.gui_obligation.gui_creation_required = false;
    mt.gui_obligation.argus_required = false;
    mt.gui_obligation.trace_projection_required_when_non_ui = true;
    mt.gui_obligation.surfaces = [];
    mt.gui_obligation.argus_targets = [];
    mt.gui_obligation.not_applicable_reason = "Solver/backend/schema producer only. It exposes typed state, diagnostics, receipts, and TraceProjection; the named native workspace owner consumes it without widening this MT's allowed paths.";
  }

  if (bodyKitGui[mt.mt_id]) {
    mt.gui_obligation.surfaces = bodyKitGui[mt.mt_id].surfaces;
    mt.gui_obligation.argus_targets = bodyKitGui[mt.mt_id].targets;
    mt.gui_obligation.trace_projection_required_when_non_ui = false;
  }

  const classification = classify(mt);
  const number = idNumber(mt.mt_id);

  if (!classification.ui) mt.gui_obligation.trace_projection_required_when_non_ui = true;

  if (classification.command && number > 41) addUnique(mt.lifecycle.depends_on, "MT-741");
  if ((classification.authority || classification.artifact || classification.job) && number > 66 && mt.mt_id !== "MT-741") addUnique(mt.lifecycle.depends_on, "MT-742");
  if (classification.diagnostics && !["MT-742", "MT-743"].includes(mt.mt_id)) addUnique(mt.lifecycle.depends_on, "MT-743");
  if (classification.ui && !["MT-748", "MT-750"].includes(mt.mt_id)) {
    addUnique(mt.lifecycle.depends_on, "MT-747");
    addUnique(mt.lifecycle.depends_on, "MT-748");
    addUnique(mt.lifecycle.depends_on, "MT-750");
  }

  if (classification.ui) {
    const generic = "every control and visible state introduced or changed by this MT, using stable native author_id values";
    mt.gui_obligation.argus_targets = (mt.gui_obligation.argus_targets || []).filter((item) => item !== generic);
    if (mt.gui_obligation.argus_targets.length === 0) {
      mt.gui_obligation.argus_targets = [`tailor-surface-manifest-${mt.mt_id.toLowerCase()}`];
    }
    addCriteria(mt, `The exact TailorSurfaceDescriptor row ${mt.mt_id} enumerates every route, pane, control author_id, dynamic entity-key rule, action, state, help anchor, permission, undo policy, receipt, and command binding; wildcard-only evidence is rejected.`);
    addCriteria(mt, "Applicable blank, loading, populated, stale/conflict, cancelled, failed, retry, permission, keyboard/focus, narrow/4K/DPI, overlap/clipping, and no-focus-steal states are covered by the registry-derived Argus matrix.");
    if (guiNeedsNativeProof.has(mt.mt_id) || !(mt.scope.proof_targets || []).some((item) => item.includes("handshake-native") || item.includes("argus") || item.includes("Argus"))) {
      addUnique(mt.scope.proof_targets, `cargo test -p handshake-native tailor_surface_manifest_${mt.mt_id.toLowerCase().replace("-", "_")}`);
    }
  }

  const chapter = chapterByGroup[group] || "Professional production workflows";
  if (mt.user_manual_obligation.required) {
    mt.user_manual_obligation.target_entries = [`Tailor UserManual > ${chapter}`];
  }

  mt.hbr_obligations = hbrObligations(classification);
  mt.resource_privacy_obligation = {
    required: true,
    hbr_rows: ["HBR-PRIV-001", "HBR-PRIV-002", "HBR-PRIV-003", "HBR-PRIV-004", "HBR-PRIV-005", "HBR-PRIV-006", "HBR-PRIV-007", "HBR-PRIV-008"],
    requirements: ["Resolve Principal, AuthenticatedSession and AccessSpace before every resource read or mutation.", "Use PostgreSQL RLS/FORCE RLS and ResourceBroker-backed artifact/file handles; UI filtering is never the security boundary.", "Derived resources preserve the intersection of source access scopes and must not leak existence through counts, search, previews, logs, traces, caches, diagnostics, exports or model context."],
    expected_evidence: ["cross_account_and_cross_space_negative_tests", "derived_scope_non_widening_test", "revocation_cache_context_handle_invalidation_test", "log_trace_preview_non_disclosure_test"]
  };
  mt.hbr_int_009_tier_obligations = diagnosticTiers(classification);
  hardenAcceptance(mt, classification);
  applySpecificAmendments(mt);

  // Keep the platform bootstrap DAG acyclic when reprocessing MT-740..MT-750.
  if (mt.mt_id === "MT-741") mt.lifecycle.depends_on = mt.lifecycle.depends_on.filter((dependency) => dependency !== "MT-742");
  if (mt.mt_id === "MT-742") mt.lifecycle.depends_on = mt.lifecycle.depends_on.filter((dependency) => dependency !== "MT-743");
  if (mt.mt_id === "MT-748") mt.lifecycle.depends_on = mt.lifecycle.depends_on.filter((dependency) => dependency !== "MT-750");

  if (number >= 654 && number <= 687) addUnique(mt.lifecycle.depends_on, "MT-782");
  mt.lifecycle.depends_on = unique(mt.lifecycle.depends_on).filter((dependency) => dependency !== mt.mt_id);
  mt.pre_activation_reconciliation.required_changes_before_execution = unique([
    ...(mt.pre_activation_reconciliation.required_changes_before_execution || []),
    "Consume the signed technical refinement, canonical command/surface registries, classified HBR tiers, and task-oriented UserManual contract before execution."
  ]);
  mt.pre_activation_reconciliation.dependency_graph_status = RECONCILIATION_STATUS;
  mt.pre_activation_reconciliation.authority = TECHNICAL_REFINEMENT_PATH;
  return { mt, group, classification: classify(mt) };
}

const priorIndex = readJson(INDEX_PATH);
const groupById = new Map(priorIndex.microtasks.map((item) => [item.mt_id, item.group]));
const records = [];

for (let number = 1; number <= ORIGINAL_COUNT; number += 1) {
  const id = mtId(number);
  const filePath = path.join(PACKET_DIR, `${id}.json`);
  const group = groupById.get(id);
  if (!group) throw new Error(`Missing group for ${id}`);
  const result = processExistingMt(readJson(filePath), group);
  writeJson(filePath, result.mt);
  records.push(result);
}

for (const mt of bodyKitV2Mts) {
  const group = mt._group;
  delete mt._group;
  const classification = classify(mt);
  mt.hbr_obligations = hbrObligations(classification);
  mt.hbr_int_009_tier_obligations = diagnosticTiers(classification);
  hardenAcceptance(mt, classification);
  if (mt.gui_obligation.gui_creation_required) {
    addUnique(mt.scope.proof_targets, `cargo test -p handshake-native tailor_surface_manifest_${mt.mt_id.toLowerCase().replace("-", "_")}`);
  }
  if (mt.mt_id === "MT-842") {
    addCriteria(mt, "Direct final evidence includes Argus and AccessKit inspection, accessibility/layout/no-focus checks, renderer-console scan, GUI-only and backend-only no-context runs, injected recovery, and direct opening of full-resolution still and sequence artifacts against pinned references.");
  }
  writeJson(path.join(PACKET_DIR, `${mt.mt_id}.json`), mt);
  records.push({ mt, group, classification });
}

const groupCounts = {};
for (const record of records) groupCounts[record.group] = (groupCounts[record.group] || 0) + 1;

const index = {
  schema: "tailor.mt_index@1",
  wp_id: WP_ID,
  generated_at_utc: UPDATED_AT,
  total: FINAL_COUNT,
  range: "MT-001..MT-842",
  core_section_13_range: "MT-001..MT-332",
  md_parity_additions_range: "MT-333..MT-448",
  second_pass_parity_range: "MT-449..MT-477",
  bodykit_submodule_range: "MT-478..MT-653",
  physics_hardening_range: "MT-654..MT-687",
  native_production_ux_range: "MT-688..MT-709",
  posekit_character_interop_range: "MT-710..MT-720",
  professional_production_hardening_range: "MT-721..MT-739",
  build_readiness_platform_range: "MT-740..MT-750, MT-780, MT-782",
  professional_cloth_ux_range: "MT-751..MT-759, MT-774, MT-777",
  bodykit_production_range: "MT-760..MT-772, MT-775..MT-776",
  dcc_parity_qualification_range: "MT-773, MT-778..MT-779, MT-781",
  bodykit_v2_native_runtime_range: "MT-783..MT-793",
  bodykit_v2_provider_runtime_range: "MT-794..MT-807",
  bodykit_v2_cinematic_performance_range: "MT-808..MT-820",
  bodykit_v2_interchange_range: "MT-821..MT-830",
  bodykit_v2_native_operator_and_pillars_range: "MT-831..MT-841",
  bodykit_v2_final_qualification: "MT-842",
  by_group: groupCounts,
  parity_review: PARITY_V3_PATH,
  multi_lens_review: REVIEW_PATH,
  build_readiness_prework: BUILD_READY_PATH,
  technical_refinement_prework: TECHNICAL_REFINEMENT_PATH,
  generator: GENERATOR_PATH,
  note: "Pre-activation projection of all 842 inactive machine-readable MT contracts under active Master Spec v02.203. MT-001..782 are preserved and reconciled; MT-783..842 add BodyKit-v2 native runtime, providers, cinematic production, interchange, professional UI, privacy/pillars and final proof. This index is not execution or validator authority.",
  microtasks: records.map(({ mt, group }) => ({
    mt_id: mt.mt_id,
    group,
    depends_on: mt.lifecycle.depends_on,
    status: mt.lifecycle.status,
    summary: mt.scope.summary
  }))
};
writeJson(INDEX_PATH, index);

const allReviewLenses = ["feature_scope", "cloth_physics", "bodykit_physics", "professional_ui_gui", "backend_parallel_agents", "authority_artifacts_events", "diagnostics_palmistry", "user_manual_no_context", "posekit_character_sheet_mix", "locus", "loom", "calendar", "flight_recorder", "dcc_parity", "adult_contact_production", "native_runtime_supply", "python_cloud_firewall", "provider_evaluator", "cinematic_frame_sequence", "interchange_loss", "resource_privacy", "parallel_projects", "recovery_accessibility_quiet", "dependency_dag"];
const universalReviewLenses = ["feature_scope", "backend_parallel_agents", "authority_artifacts_events", "diagnostics_palmistry", "user_manual_no_context", "flight_recorder", "resource_privacy", "recovery_accessibility_quiet", "dependency_dag"];

function reviewLensesFor(mt, group) {
  const applied = new Set(universalReviewLenses);
  const contractText = JSON.stringify(mt);
  if (["SolverCore", "Collision", "PhysicsHardening", "GarmentAuthoring", "Fabric", "TrimRigid"].includes(group)) applied.add("cloth_physics");
  if (group.startsWith("Bk") || group === "BodyKitProduction") applied.add("bodykit_physics");
  if (mt.gui_obligation.gui_creation_required || mt.gui_obligation.operator_surface_required) applied.add("professional_ui_gui");
  if (group === "PoseKitCharacterInterop" || /PoseKit|character.sheet|mix primitive/i.test(contractText)) applied.add("posekit_character_sheet_mix");
  if (/\bLocus\b/.test(contractText)) applied.add("locus");
  if (/\bLoom\b|ProjectKnowledgeIndex/.test(contractText)) applied.add("loom");
  if (/\bCalendar\b|CalendarMutation|ActivitySpan/.test(contractText)) applied.add("calendar");
  if (group === "DccParityQualification" || /Daz Studio|Marvelous Designer|DCC/.test(contractText)) applied.add("dcc_parity");
  if (group === "BkGenitals" || /genital|penis|vulva|vagina|contact.stag|penetrat|gape/i.test(contractText)) applied.add("adult_contact_production");
  if (group === "NativeRuntimeSupply") { applied.add("native_runtime_supply"); applied.add("python_cloud_firewall"); }
  if (group === "BodyProviderRuntime") applied.add("provider_evaluator");
  if (group === "CinematicPerformance" || group === "CinematicQualification") applied.add("cinematic_frame_sequence");
  if (group === "InterchangeDelivery") applied.add("interchange_loss");
  if (group === "ProjectPillars") applied.add("parallel_projects");
  return allReviewLenses.filter((lens) => applied.has(lens));
}

const review = {
  schema: "tailor.multi_lens_review@1",
  wp_id: WP_ID,
  reviewed_at_utc: UPDATED_AT,
  review_authority: "ADVISORY_PRE_ACTIVATION_ONLY_NO_VALIDATOR_VERDICT",
  generator: GENERATOR_PATH,
  source_range_reviewed: "MT-001..MT-782",
  hardened_result_range: "MT-001..MT-842",
  lenses: allReviewLenses,
  source_audit_accounting: {
    unique_source_mts: ORIGINAL_COUNT,
    physics_solver: 117,
    cloth_authoring_materials: 163,
    motion_delivery: 130,
    kernel_model_validation: 101,
    bodykit: 176,
    native_production_ux: 22,
    posekit_character_interop: 11,
    professional_hardening: 19,
    bodykit_v2: 60
  },
  hardening_summary: {
    original_mts_preserved_and_reviewed: ORIGINAL_COUNT,
    new_gap_closure_mts: FINAL_COUNT - ORIGINAL_COUNT,
    final_total: FINAL_COUNT,
    gui_backend_producers_reclassified: nonUiProducerIds.size,
    bodykit_blank_gui_contracts_populated: Object.keys(bodyKitGui).length,
    bodykit_job_producers_rebased_to_early_scheduler: bodyKitJobProducers.size,
    diagnostic_deferrals_remaining: 0,
    synthetic_mt_number_manual_targets_remaining: 0,
    empty_hbr_obligations_remaining: 0,
    minimum_acceptance_criteria: Math.min(...records.map(({ mt }) => mt.scope.acceptance_criteria.length))
  },
  unresolved_external_inputs: [
    "MARVELOUS_DESIGNER_EXECUTABLE_PATH_VERSION_SHA256_NOT_INSPECTED",
    "WP_KERNEL_012_MERGED_MAIN_SURFACE_SEAM_AND_POSEKIT_SCHEMA_RECEIPT_REQUIRED",
    "CANONICAL_LOCUS_READ_API_CURRENTLY_TYPED_UNAVAILABLE",
    "CANONICAL_CALENDAR_NATIVE_ROUTES_CURRENTLY_TYPED_UNAVAILABLE"
  ],
  per_mt: records.map(({ mt, group, classification }) => ({
    mt_id: mt.mt_id,
    group,
    primary_lens: group.startsWith("Bk") || group === "BodyKitProduction" ? "bodykit" : group === "PhysicsHardening" || group === "SolverCore" || group === "Collision" ? "physics" : group === "NativeProductionUX" || group === "ProfessionalClothUX" ? "ui_workflow" : group === "DccParityQualification" ? "dcc_parity" : group === "PoseKitCharacterInterop" ? "posekit_character_interop" : "feature_and_platform",
    lenses_applied: reviewLensesFor(mt, group),
    classifications: Object.entries(classification).filter(([, value]) => value).map(([key]) => key),
    acceptance_criteria_count: mt.scope.acceptance_criteria.length,
    proof_target_count: mt.scope.proof_targets.length,
    gui_required: mt.gui_obligation.gui_creation_required,
    gui_targets: mt.gui_obligation.argus_targets,
    manual_targets: mt.user_manual_obligation.target_entries,
    hbr_obligations: mt.hbr_obligations,
    diagnostic_postures: Object.fromEntries(mt.hbr_int_009_tier_obligations.map((item) => [item.tier, item.posture])),
    dependencies: mt.lifecycle.depends_on,
    outcome: idNumber(mt.mt_id) <= ORIGINAL_COUNT ? "PRESERVED_RECONCILED_V02_203_CANDIDATE" : "NEW_BODYKIT_V2_GAP_CLOSURE_CANDIDATE"
  }))
};
writeJson(path.join(PACKET_DIR, "_MULTI_LENS_REVIEW.json"), review);

const parityV3 = {
  schema: "tailor.parity_review@3",
  wp_id: WP_ID,
  reviewed_at_utc: UPDATED_AT,
  review_authority: "ADVISORY_PRE_ACTIVATION_ONLY_NO_RUNTIME_PARITY_VERDICT",
  supersedes_planning_conclusions_from: ["_PARITY_REVIEW.json", "_PARITY_REVIEW_V2.json"],
  baseline: {
    marvelous_designer: "Official Marvelous Designer 2026.0 documentation and API behavior as black-box parity evidence; local executable NOT_INSPECTED pending exact path/version/hash. Python surfaces are research evidence only and prohibited from the Handshake path.",
    daz_studio: "Locally verified Daz Studio 6.25.2026.14722 General Release; Qt 6.10.3; OpenSubdiv 3.5.0; Iray 2025.0.3; RTX 3090 24 GiB reference.",
    daz_evidence: "PREP_SESSION_VERIFIED_LOCAL_LOG; activation must resolve the Daz executable/log roots through operator-controlled configuration and record path/version/hash in the DCC fixture manifest."
  },
  claim_law: "Every required vendor capability must be QUALIFIED by runtime and inspected artifact evidence before full-parity language is available. MT ownership is planning coverage, not parity proof.",
  gaps_closed_by_candidate_contracts: {
    capability_claim_gate: "MT-740",
    pattern_archive: "MT-757",
    professional_retopology: "MT-758",
    modular_garment_composer: "MT-756",
    trim_graphic_styles_and_colorways: "MT-759",
    fit_map_semantics: "MT-432..MT-434 amended",
    front_back_material_swap: "MT-467 amended",
    substance_pbr: "MT-777",
    garment_fur: "MT-774",
    smart_content: "MT-763",
    shader_mixer: "MT-764",
    layered_image_material: "MT-765",
    native_final_render: "MT-766",
    aov_spot_render_queue_library: "MT-767",
    face_transfer: "MT-768",
    strand_hair: "MT-775",
    generic_deformers: "MT-769",
    figure_setup: "MT-770",
    map_transfer: "MT-771",
    dson_migration: "MT-772",
    lip_sync: "MT-776",
    extension_plugin_api: "MT-773",
    vendor_fixture_harness: "MT-778",
    full_qualification_matrix: "MT-779",
    final_professional_e2e: "MT-781"
    ,bodykit_v2_native_runtime: "MT-783..793"
    ,gnm_v3_and_provider_neutral_bodykit: "MT-794..807"
    ,cinematic_performance_and_conditioning: "MT-808..820"
    ,canonical_interchange_and_dcc_qualification: "MT-821..830"
    ,professional_native_operator_and_model_surfaces: "MT-831..841"
    ,movie_production_e2e: "MT-842"
  },
  explicit_low_roi_legacy_state: {
    poser_cr2_pz3: "UNSUPPORTED_CANDIDATE_PENDING_OPERATOR_REFINEMENT_DECISION",
    third_party_plugin_ecosystem: "NOT_PART_OF_CORE_PARITY_UNLESS_REGISTERED_AS_REQUIRED_CAPABILITY"
  },
  primary_sources: [
    "https://support.marvelousdesigner.com/hc/en-us/articles/55837641308313-Marvelous-Designer-2026-0-New-Feature-List",
    "https://support.marvelousdesigner.com/hc/en-us/categories/51985515993625-Manual",
    "https://developer.marvelousdesigner.com/",
    "https://developer.marvelousdesigner.com/list.html",
    "https://docs.daz3d.com/public/software/dazstudio/4/referenceguide/scripting/start",
    "https://docs.daz3d.com/public/software/dazstudio/4/referenceguide/interface/panes/content_library/start",
    "https://docs.daz3d.com/public/software/dazstudio/4/referenceguide/terms/metadata/start",
    "https://docs.daz3d.com/public/dson_spec/start",
    "https://github.com/daz3d/DazBridgeUtils",
    "https://github.com/daz3d/DazToBlender",
    "https://github.com/daz3d/DazToUnreal",
    "https://github.com/google/GNM",
    "https://github.com/facebookresearch/MHR",
    "https://github.com/EpicGames/OpenRigLogic",
    "https://github.com/NVIDIA/Audio2Face-3D-SDK",
    "https://github.com/huggingface/candle",
    "https://docs.modular.com/mojo/cli/build",
    "https://onnxruntime.ai/docs/get-started/with-c.html",
    "https://www.tensorflow.org/install/lang_c",
    "https://github.com/KhronosGroup/glTF",
    "https://openusd.org/release/api/usd_skel_page_front.html",
    "https://academysoftwarefoundation.github.io/OpenPBR/",
    "https://github.com/AcademySoftwareFoundation/MaterialX",
    "https://docs.rs/exr/latest/exr/",
    "https://dev.epicgames.com/documentation/unreal-engine/importing-assets-using-interchange-in-unreal-engine",
    "https://docs.blender.org/manual/en/latest/files/import_export/index.html",
    "https://doc.rust-lang.org/cargo/commands/cargo-vendor.html"
  ]
};
writeJson(path.join(PACKET_DIR, "_PARITY_REVIEW_V3.json"), parityV3);

function specAnchorsForGroup(group) {
  if (group === "NativeRuntimeSupply") return ["TAI-V2-RUN-003", "TAI-V2-RUN-005", "TAI-V2-RUN-006", "TAI-V2-RUN-008", "TAI-V2-QA-005"];
  if (group === "BodyProviderRuntime") return ["TAI-V2-BDY-001", "TAI-V2-BDY-002", "TAI-V2-BDY-002A", "TAI-V2-BDY-003", "TAI-V2-BDY-007", "TAI-V2-BDY-012"];
  if (group === "CinematicPerformance") return ["TAI-V2-PERF-001", "TAI-V2-PERF-003", "TAI-V2-PERF-006", "TAI-V2-PERF-009", "TAI-V2-QA-002", "TAI-V2-QA-004"];
  if (group === "InterchangeDelivery") return ["TAI-V2-IO-001", "TAI-V2-IO-002", "TAI-V2-IO-003", "TAI-V2-IO-007", "TAI-V2-IO-008", "TAI-V2-IO-009"];
  if (group === "NativeOperatorSurfaces") return ["TAI-V2-ACT-003", "TAI-V2-ACT-005", "TAI-V2-UX-001", "TAI-V2-UX-003", "TAI-V2-UX-005", "TAI-V2-UX-012"];
  if (group === "ProjectPillars") return ["TAI-V2-PRJ-001", "TAI-V2-PRJ-003", "TAI-V2-PRJ-004", "TAI-V2-PRJ-006", "TAI-V2-PRJ-007", "TAI-V2-PRJ-010"];
  if (group === "CinematicQualification") return ["TAI-V2-QA-001", "TAI-V2-QA-002", "TAI-V2-QA-008", "TAI-V2-QA-009"];
  if (group === "DccParityQualification") return ["TAI-ACT-010", "TAI-QA-001", "TAI-QA-003", "TAI-V2-IO-001", "TAI-V2-QA-007"];
  if (group === "BodyKitProduction") return ["TAI-ACT-010", "TAI-BKP-004", "TAI-BKP-008", "TAI-V2-BDY-010", "TAI-V2-UX-004"];
  if (group === "ProfessionalClothUX") return ["TAI-ACT-010", "TAI-CLP-001", "TAI-CLP-006", "TAI-V2-ACT-005", "TAI-V2-UX-004"];
  return ["TAI-ACT-010", "TAI-GATE-002", "TAI-V2-ACT-003", "TAI-V2-QA-009"];
}

const specDeltaMap = Object.fromEntries(index.microtasks.filter((row) => idNumber(row.mt_id) >= 740).map((row) => [row.mt_id, {
  anchors: specAnchorsForGroup(row.group),
  manual_target: `Tailor UserManual > ${chapterByGroup[row.group] || "Professional production workflows"}`,
  status: RECONCILIATION_STATUS
}]));

const technicalRefinement = {
  schema: "tailor.technical_refinement_prework@1",
  wp_id: WP_ID,
  created_at_utc: UPDATED_AT,
  status: "OPERATOR_APPROVED_SPEC_V02_203_RECORDED_NON_EXECUTION_REFINEMENT",
  execution_authority: false,
  operator_request: "Preserve and update the full Tailor Cloth+BodyKit WP under Master Spec v02.203: Rust frontend/backend and offline runtime, Python ban, GNM-v3 default, Candle/Mojo/ONNX/TensorFlow precedence, professional high-resolution human/face/performance/conditioning tools, Photoshop/Lightroom/ComfyUI-informed native workspaces, parallel projects, PostgreSQL/CRDT/privacy, Loom/Locus/Flight Recorder, and production Blender/Unreal interchange without product coding or activation.",
  approved_spec_enrichment: [
    "Master Spec v02.203 Sections 13.37..13.45 and Appendix 12 are active and hash-validated.",
    "Rust-owned product/UI, no Python/PyTorch/cloud/runtime downloads, immutable dependency bundles, Candle-first local ML with qualified Mojo AOT and ONNX/TensorFlow fallbacks.",
    "GNM-v3 default provider-neutral evaluation, optional MHR lane, high-resolution head/source/assembly/residual/corrective/OpenRigLogic-compatible Rust contracts.",
    "Canonical performance/capture/dynamic appearance/conditioning and frame-plus-sequence cinematic waist-up qualification.",
    "Deterministic GLB, restricted pure-Rust USD, OpenPBR/MaterialX/OCIO/EXR, Unreal native commandlet, manual Blender qualification, import-only FBX/Alembic capability rows.",
    "Professional native top-bar, workspace, tool, slider-plus-numeric, viewport, graph, timeline, jobs, problems, history and model-navigation contracts; PostgreSQL authority, parallel projects, scoped CRDT, HBR-PRIV, Loom/Locus/Flight Recorder."
  ],
  proposed_spec_enrichment: [],
  scope_edges: [
    "One Tailor product module with Cloth and BodyKit submodules; shared rendering, materials, pose/contact, jobs, commands, diagnostics, assets, manual, and DCC qualification.",
    "Native Rust/wgpu/egui shell and standalone tailor-solver crate; no WebView/Tauri Tailor surface.",
    "EventLedger and PostgreSQL remain authority; ArtifactStore owns derived artifacts; Flight Recorder is a projection; optional pillars never become Tailor authority.",
    "PoseKit evidence stays 2D/immutable; lift/camera/contact/mix operations remain proposals until explicit selected apply through owning APIs.",
    "Adapters are optional typed Rust-owned compatibility boundaries. Unreal automation is a pinned native C++ commandlet; Blender evidence is operator-driven manual qualification; neither is core authority or required for native completion."
  ],
  assumptions: [
    "WP-KERNEL-001..004 remain validated and merged as previously recorded, subject to activation-time recheck.",
    "WP-KERNEL-012 and CKC PoseKit work are not treated as merged until main contains the pinned commits/schemas and compatibility receipts.",
    "Daz Studio local evidence is version 6.25.2026.14722; Marvelous Designer remains NOT_INSPECTED until an exact local executable path/version/hash is configured.",
    "Numerical, performance, hardware, fixture, model/SDK and DCC values that require measurement are locked by MT-782/783/778 procedures rather than invented during refinement.",
    "The Master Spec approval signature was consumed only for v02.203 enrichment; WP activation still requires the Classic Orchestrator/Activation Manager lane and separate activation authority."
  ],
  non_goals: [
    "No Tailor product code, WP activation, Coder/Validator launch, official packet hydration, product branch/worktree, or runtime parity claim in this prep change.",
    "No reuse of Genesis topology/assets, no SMPL-family prohibited-license dependency, and no proprietary DCC binary decompilation.",
    "No private Locus, Loom, Calendar, Flight Recorder, diagnostics, watcher, command, asset, manual, or scheduler authority.",
    "Third-party vendor plugin ecosystems are not automatically core parity; each capability must be explicitly registered and qualified.",
    "No Python/PyTorch product path, cloud service, runtime download, generated DCC script, mandatory FBX/Alembic export, private database/pillar/shell, or false automated Blender claim."
  ],
  spec_anchors: ["Master Spec v02.203 Section 13.1..13.45", "TAI-V2-RUN-001..010", "TAI-V2-BDY-001..012", "TAI-V2-PERF-001..010", "TAI-V2-IO-001..010", "TAI-V2-PRJ-001..010", "TAI-V2-ACT-001..007", "TAI-V2-UX-001..012", "TAI-V2-QA-001..009", "TAI-V2-RES-001..003", "HANDSHAKE_BUILD_RULES HBR-INT/HBR-SWARM/HBR-VIS/HBR-QUIET/HBR-MAN/HBR-STOP/HBR-PRIV", "WP-KERNEL-012 native shell and Atelier/PoseKit contracts", "PRIM-UserManual", "KernelActionCatalogV1", "EventLedger", "ArtifactStore", "Flight Recorder", "Palmistry", "Locus", "Loom/ProjectKnowledgeIndex"],
  capability_dimension_contract: {
    owner_mt: "MT-740",
    requirement_field: "requirement_status",
    requirement_values: ["required", "accepted_exclusion", "optional", "deliberate_exceedance"],
    qualification_field: "qualification_status",
    qualification_values: ["unimplemented", "implemented_unqualified", "qualified", "stale", "failed", "unsupported"],
    independence_rule: "requirement_status and qualification_status are independent and neither may be inferred from the other",
    rejected_legacy_forms: ["requirement_class", "uppercase enum values"]
  },
  acceptance_criteria: [
    "All 842 MT contracts are unique, inactive/PENDING, template-complete, no-context implementable, path/proof bounded, and form an acyclic dependency graph with no missing targets.",
    "Every MT has at least three acceptance criteria, task-oriented UserManual ownership, non-empty HBR obligations including HBR-PRIV, a resource_privacy_obligation, and DIRECT/INHERITED/NOT_APPLICABLE diagnostics classifications with no blanket deferral.",
    "GUI-producing MTs have native paths, exact surface-manifest ownership, stable action/control ids, Argus/AccessKit/native proof, failure/recovery/accessibility/layout/quiet coverage; backend-only producers do not falsely own GUI creation.",
    "All mutating/model-operable workflows consume the canonical action catalog and expose typed actor/session/correlation, versions, leases, idempotency, preview/diff/apply, cancellation/retry, conflict/error, events, receipts, and recovery.",
    "Cloth physics preserves the non-production ReferenceCpuF64 oracle and the two production tiers InteractiveGpuXpbd and FinalCpuBarrierF64 with finite-thickness VF/EE CCD/contact, strict strain, checkpoints, captures, fault injection, calibration, reliability, and performance gates.",
    "Professional Cloth, BodyKit, high-resolution head/body, performance/capture, cinematic render/conditioning, native UI, interchange, adult contact, PoseKit/sheets/mix, privacy/pillars, diagnostics and manual workflows each have bounded MT ownership and final E2E/qualification gates.",
    "The parity claim gate fails until every required pinned-vendor capability has runtime and directly inspected artifact proof accepted by the future independent validator.",
    "The active v02.203 bundle is the sole product authority; this refinement and MT set remain non-execution until official packet hydration, dependency containment, product worktree/state synchronization and explicit Activation Manager readiness."
  ],
  hbr_pillar_review: {
    INT: { applicable: true, evidence_path: REVIEW_PATH },
    SWARM: { applicable: true, evidence_path: REVIEW_PATH },
    VIS: { applicable: true, evidence_path: REVIEW_PATH },
    QUIET: { applicable: true, evidence_path: REVIEW_PATH },
    MAN: { applicable: true, evidence_path: REVIEW_PATH },
    STOP: { applicable: true, evidence_path: REVIEW_PATH },
    PRIV: { applicable: true, evidence_path: REVIEW_PATH }
  },
  hbr_int_009_three_tier_diagnostic: {
    applicable: true,
    verdict: "CANDIDATE_CONTRACTED_NOT_IMPLEMENTED",
    tier_flight_recorder: { status: "CLASSIFIED", notes: "EventLedger authority and idempotent Flight Recorder projection are explicit in MT-742 and per-MT classifications." },
    tier_internal_diagnostics: { status: "CLASSIFIED", notes: "Direct/inherited/not-applicable posture replaces blanket deferral; MT-743 owns shared integration." },
    tier_palmistry: { status: "CLASSIFIED", notes: "Long jobs/native/DCC processes use shared Palmistry; no Tailor watcher fork." }
  },
  dependency_gates: {
    wp_kernel_012_native_shell: {
      status: "BLOCKED_UNTIL_CONTAINED_IN_MAIN_AND_CURRENT_MAIN_COMPATIBILITY_PASS",
      authority_path: ".GOV/task_packets/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/packet.json",
      observed_state: "packet lifecycle is In Progress; main containment is NOT_STARTED; current-main compatibility is NOT_RUN",
      applies_to: ["MT-748", "MT-750", "MT-831..MT-842", "all Tailor GUI-producing consumers"],
      proof_required: ["non-NONE merged_main_commit", "main-containment verdict PASS or CONTAINED", "current-main compatibility verdict PASS", "canonical shell interface/schema/action/surface symbols exist on main", "Tailor module/top-bar/workspace compile probe passes against contained symbols"]
    },
    wp_ckc_posekit: {
      status: "TECHNICAL_PASS_MERGE_WITHHELD_BLOCKED_UNTIL_CONTAINED_IN_MAIN_AND_COMPATIBILITY_PASS",
      authority_path: ".GOV/task_packets/WP-CKC-posekit-overhaul/packet.json",
      validated_source_commit: "a2abba07af8e0446d5cd988f5aaabc1c7f48cbfe",
      validator_evidence: "INTEGRATION_VALIDATOR-20260708-ckc-v2",
      observed_state: "technical validation PASS with merge withheld; merged_main_commit is NONE",
      applies_to: ["MT-710..MT-720", "MT-749", "MT-781", "MT-842"],
      proof_required: ["non-NONE merged_main_commit", "main-containment verdict PASS or CONTAINED", "current-main compatibility verdict PASS", "canonical PoseKit/character-sheet/artifact schemas and actions exist on main", "Tailor lift/mix/contact compile and schema-compatibility probes pass"]
    }
  },
  absorbed_preworks: {
    status: "ABSORBED_AND_SUPERSEDED_FOR_CURRENT_BUILD_READINESS_PLANNING_RETAINED_AS_PROVENANCE",
    canonical_current_context: [ACTIVE_SPEC_ENTRYPOINT, TECHNICAL_REFINEMENT_PATH],
    absorbed_retained_files: [`.GOV/task_packets/${WP_ID}/${WP_ID}.physics-refinement-prework.json`, `.GOV/task_packets/${WP_ID}/${WP_ID}.professional-production-refinement-prework.json`, `.GOV/task_packets/${WP_ID}/${WP_ID}.mt-reconciliation-prework.json`, BUILD_READY_PATH],
    advisory_evidence_not_authority: [PARITY_V3_PATH, REVIEW_PATH],
    preservation_rule: "No prior MT intent or research is deleted. Active v02.203 wins on conflict; retained preworks preserve provenance only."
  },
  spec_delta_map: specDeltaMap,
  legacy_law_reconciliation: {
    "TAI-PHY-007": { owner_mts: ["MT-683", "MT-684"], contract: "Remesh/topology changes remain checkpointed transactions with complete state transfer, validation, atomic commit or exact rollback.", status: RECONCILIATION_STATUS },
    "python_blender_fbx_alembic": { owner_mts: Object.keys(nativeSupersessions), contract: "The listed legacy MTs are preservation-first rewritten to active v02.203 Rust/no-Python/canonical-interchange law.", status: RECONCILIATION_STATUS }
  },
  research_basis: {
    sources_checked: parityV3.primary_sources,
    local_reference_evidence: parityV3.baseline,
    selected_patterns: ["Rust-owned offline runtime and UI", "immutable content-addressed dependency bundles", "GNM-v3 default provider-neutral evaluator", "source/assembly/residual/corrective separation", "canonical performance and conditioning contracts", "one canonical action/surface/manual corpus", "GLB plus restricted USD", "OpenPBR/MaterialX with explicit losses", "native Unreal and manual Blender qualification", "frame plus sequence direct inspection"],
    reuse_opportunities: ["KernelActionCatalogV1", "EventLedger", "PostgreSQL RLS", "ResourceBroker", "ArtifactStore", "Flight Recorder", "Palmistry", "UserManual", "native shell/Argus/AccessKit", "Locus", "Loom", "Atelier/PoseKit/character sheets", "shared renderer/scheduler/leases/jobs"],
    rejected_options: ["Python/PyTorch product paths", "cloud services and runtime downloads", "generated Blender scripts", "mandatory Blender-mediated FBX/Alembic export", "provider schema authority", "unqualified broad USD claims", "generic everything-node graph", "GUI-only creator tools", "invented thresholds"],
    risks: ["upstream Python/C++ assumptions", "large dependency DAG", "provider/model/version drift", "immature pure-Rust USD coverage", "DCC loss/version drift", "cinematic temporal false positives", "privacy leakage through derived resources", "cross-project routing", "GUI/backend/manual drift"],
    mitigations: ["MT-783/784 dependency and firewall", "MT-793 adapter-removal matrix", "MT-794..807 provider/rig contracts", "MT-820 frame/sequence corpus", "MT-821..830 restricted interchange", "MT-831..841 native UI/projects/privacy/pillars", "MT-842 final E2E", "MT-780 semantic gate"],
    validation_plan: ["governance JSON/DAG/schema checks", "Python/process-tree and airgap proof", "dependency hash/license/SBOM proof", "provider/runtime fixture parity", "Argus visual matrix", "backend-only and GUI-only no-context operation", "multi-project conflict/privacy/recovery", "structural interchange and DCC qualification", "direct frame and sequence inspection", "independent WP validation after implementation"]
  },
  red_team: {
    risks: ["An MT can exist without professional workflow parity.", "An upstream model can silently reintroduce Python or downloads.", "A provider mesh can become hidden authority.", "A polished frame can hide temporal failures.", "A DCC can open a file while losing rig/material/scale data.", "A GUI can appear complete while agents cannot operate it.", "Derived resources can leak across accounts or projects.", "Optional pillar outages can block core production.", "Numerical thresholds can be guessed."],
    minimum_controls: ["Python/cloud/firewall and adapter-removal gates.", "Immutable dependency/provider/interchange profiles.", "Exact action/surface/manual registries.", "PostgreSQL/EventLedger/ArtifactStore authority and HBR-PRIV negative tests.", "Direct/inherited/not-applicable diagnostics.", "Measured profiles and typed unsupported behavior.", "Frame plus sequence direct inspection.", "No-context, fault, concurrency, recovery, accessibility and DCC qualification evidence."]
  },
  microtask_plan: index.microtasks
};
writeJson(path.join(PACKET_DIR, `${WP_ID}.technical-refinement-prework.json`), technicalRefinement);

const buildReadiness = {
  schema: "tailor.build_readiness_prework@1",
  wp_id: WP_ID,
  updated_at_utc: UPDATED_AT,
  status: "SPEC_V02_203_RECONCILED_CONTRACT_CANDIDATE_PENDING_PACKET_AND_ACTIVATION",
  implementation_authority: false,
  activation_changed: false,
  active_spec_changed: true,
  active_spec_version: ACTIVE_SPEC_VERSION,
  active_spec_entrypoint: ACTIVE_SPEC_ENTRYPOINT,
  source_inventory_reviewed: "MT-001..MT-782",
  hardened_inventory: "MT-001..MT-842",
  generated_by: GENERATOR_PATH,
  evidence: {
    index: `.GOV/task_packets/${WP_ID}/_MT_INDEX.json`,
    every_mt_review: REVIEW_PATH,
    parity_review: PARITY_V3_PATH,
    technical_refinement: TECHNICAL_REFINEMENT_PATH,
    physics_prework: `.GOV/task_packets/${WP_ID}/${WP_ID}.physics-refinement-prework.json`,
    professional_prework: `.GOV/task_packets/${WP_ID}/${WP_ID}.professional-production-refinement-prework.json`,
    historical_reconciliation: `.GOV/task_packets/${WP_ID}/${WP_ID}.mt-reconciliation-prework.json`
  },
  resolved_contract_gaps: ["every-MT acceptance depth", "GUI/backend ownership mismatch", "exact surface registry", "canonical action/backend-agent parity", "task-oriented UserManual", "HBR-PRIV and three-tier diagnostics", "Rust-native offline dependency acquisition", "Python/cloud/runtime-download firewall", "GNM-v3 provider-neutral evaluation", "high-resolution head/source/assembly/residual/corrective policy", "OpenRigLogic-compatible Rust evaluation", "performance/capture/cinematic appearance/conditioning", "professional native workspaces and top-bar actions", "parallel projects/PostgreSQL/CRDT/privacy", "Locus/Loom/Flight Recorder integration", "GLB/restricted USD/OpenPBR/MaterialX/EXR", "Unreal native and manual Blender qualification", "final cinematic waist-up frame/sequence E2E"],
  remaining_authority_gates: ["OFFICIAL_PACKET_JSON", "SEPARATE_WP_ACTIVATION_AUTHORITY", "DEPENDENCY_MAIN_CONTAINMENT_AND_COMPATIBILITY", "ACTIVATION_MANAGER_RECORDS", "PRODUCT_BRANCH_AND_WTC_WORKTREE", "TASK_BOARD_BUILD_ORDER_TRACEABILITY_SYNC", "ACTIVATION_READINESS"],
  remaining_external_input: "Marvelous Designer executable path/version/SHA-256 is NOT_INSPECTED and must be supplied or discovered before MT-778/activation input lock can close.",
  truthful_verdict: "Master Spec v02.203 and the 842 inactive MT contracts are authored candidates. No Tailor product code was implemented and the WP is not activated, executable or validator-ready until official packet hydration, separate WP activation authority, dependency containment, product worktree/state synchronization and Activation Manager readiness complete."
};
writeJson(path.join(PACKET_DIR, `${WP_ID}.build-readiness-prework.json`), buildReadiness);

const stub = readJson(STUB_PATH);
stub.lifecycle.status = "SPEC_V02_203_APPLIED_BODYKIT_V2_REFINEMENT_APPROVED_HELD_FOR_PACKET_ACTIVATION_AND_DEPENDENCY_GATES";
stub.lifecycle.spec_status = "Master Spec v02.203 Sections 13.1..13.45 and Appendix 12 are active and validated; execution remains blocked on official packet hydration, separate WP activation authority, dependency containment/current-main compatibility, product worktree/state synchronization and Activation Manager readiness.";
stub.microtasks.status = "PRE_CREATED_MULTI_LENS_BUILD_READY_AND_HELD_NOT_PACKET_HYDRATED";
stub.microtasks.total = FINAL_COUNT;
stub.microtasks.range = "MT-001..MT-842";
stub.microtasks.parity_review = `${PARITY_V3_PATH} (current advisory planning coverage) + historical _PARITY_REVIEW.json/_PARITY_REVIEW_V2.json`;
stub.microtasks.composition.build_readiness_platform = "MT-740..750, MT-780, MT-782 (capability claims, commands, authority/artifacts/events, diagnostics, pillars, UserManual, surface registry, mix primitives, native seam, semantic gate, implementation input lock)";
stub.microtasks.composition.professional_cloth_ux = "MT-751..759, MT-774, MT-777 (professional Cloth workspaces, modular/archive/retopo/styles, garment fur, PBR/SBSAR)";
stub.microtasks.composition.bodykit_production = "MT-760..772, MT-775..776 (creator parity, adult contact, face, Smart Content, shader/layers/render, transfer/deformers/setup/maps/DSON, hair, lip-sync)";
stub.microtasks.composition.dcc_parity_qualification = "MT-773, MT-778..779, MT-781 (extension SDK, DCC reference harness, vendor qualification matrix, final professional E2E)";
stub.microtasks.composition.bodykit_v2_native_runtime = "MT-783..793 (dependency acquisition, Python/cloud firewall, Candle/Mojo/ONNX/TensorFlow runtime ladder, Audio2Face/capture/OpenRigLogic oracle, adapter-removal matrix)";
stub.microtasks.composition.bodykit_v2_provider_runtime = "MT-794..807 (provider-neutral evaluator, GNM-v3 default, optional MHR, high-resolution head, source assembly, residual/corrective/scan fit, cinematic face, Rust RigLogic and LOD transfer)";
stub.microtasks.composition.bodykit_v2_cinematic_performance = "MT-808..820 (performance tracks, capture sync/solve/editing, dynamic appearance, eye/mouth/neck/groom/skin, cinematic render, conditioning EXR, frame/sequence corpus)";
stub.microtasks.composition.bodykit_v2_interchange = "MT-821..830 (capability/loss matrix, GLB, restricted USD, OpenPBR/MaterialX/OCIO, Unreal native commandlet, manual Blender, import-only FBX/Alembic, structural round trip)";
stub.microtasks.composition.bodykit_v2_operator_projects_pillars = "MT-831..841 (module/top-bar registration, professional workspace, shared tools, detailed BodyKit workspaces, parallel projects/CRDT/privacy, Loom/Locus/Flight Recorder/diagnostics/model navigation)";
stub.microtasks.composition.bodykit_v2_final_qualification = "MT-842 (offline no-context two-project movie-production waist-up still+sequence+conditioning+interchange+failure/recovery proof)";
stub.microtasks.shape = "All MT-001..842 follow hsk.microtask_contract@1; all remain PENDING and active=false. MT-001..782 are preserved/reconciled and MT-783..842 implement v02.203 BodyKit-v2. Every MT has at least three acceptance criteria, task-oriented UserManual targets, HBR-PRIV/resource privacy, classified diagnostics and an acyclic candidate DAG. This is non-execution refinement pending official packet hydration and activation.";
stub.microtasks.verified = "PREP_GENERATED_PENDING_CHECK: 842/842 JSON contracts, full index, multi-lens review, parity V3, build-readiness prework and approved technical refinement exist; semantic and repository gates must pass before this advances.";
stub.microtasks.reconciliation = `${REVIEW_PATH} reviews MT-001..842; prior MT-001..782 scope is preserved and incompatible Python/Blender/FBX/Alembic assumptions are explicitly superseded by active v02.203. This remains non-execution authority.`;
stub.refinement_ready.mt_group_plan.total_estimate = FINAL_COUNT;
stub.refinement_ready.mt_group_plan.groups = Object.entries(groupCounts).map(([group, count]) => `${group}(${count})`).join(", ");
stub.refinement_ready.mt_group_plan.source = "active Master Spec v02.203, preserved MT-001..782, research and independent runtime/UI/pillar audits, plus MT-783..842 BodyKit-v2 closure lanes";
stub.refinement_ready.spec_authority = "Master Spec v02.203 is normative: Section 13.1..13.36 preserves Cloth/BodyKit professional law and Sections 13.37..13.45 supersede incompatible runtime/interchange details with the approved BodyKit-v2 authority.";
stub.refinement_ready.gaps_status = "SPEC_V02_203_BODYKIT_V2_GAPS_CONTRACTED; SEMANTIC_GATE_PACKET_HYDRATION_DEPENDENCY_CONTAINMENT_AND_ACTIVATION_REMAIN_OPEN";
stub.refinement_ready.next_session_steps = [
  "1. Pass the v02.203 Tailor semantic/preactivation and repository governance gates over MT-001..842.",
  "2. Prove required WP-KERNEL-012, CRDT, Loom/Locus/Flight Recorder and other dependency interfaces are contained in main and compatibility-qualified; unavailable optional pillars remain typed and non-blocking.",
  "3. If the operator requests activation, obtain separate WP activation authority and let Activation Manager create packet.json plus official state records.",
  "4. Only after activation, prepare the product branch/worktree/backup and synchronize Task Board, Build Order and traceability.",
  "5. Emit ACTIVATION_READINESS before any implementation session."
];
stub.refinement_ready.hbr_directive_MUST.HBR_MAN = "Tailor uses canonical PRIM-UserManual task-oriented chapters generated from the shared command/surface corpus; legacy ModelManual is a compatibility projection, and MT-number pages are technical evidence only.";
stub.refinement_ready.hbr_directive_MUST.HBR_PRIV = "Every Tailor MT carries HBR-PRIV-001..008 resource obligations: Principal/Session/AccessSpace resolution, PostgreSQL RLS/FORCE RLS, ResourceBroker handles, derived-scope intersection, revocation and negative non-disclosure across source, generated, diagnostic, model-context and export resources.";
stub.native_shell_toolkit_integration.build_gating = "Creative module remains gated on kernel prerequisites and explicit activation. v02.203 registers Tailor through the WP-KERNEL-012 module, action and surface registries and requires top-bar, workspace, model and Argus parity; no private shell or foreground DCC automation.";
stub.activation_contract.required_activation_steps = unique([
  ...stub.activation_contract.required_activation_steps.filter((item) => !/MT_001_to_|v02_201|unique_operator_signature_for_v02_201/i.test(item)),
  "review_and_accept_TAILOR_V02_203_REFINEMENT_AND_MT_001_TO_842",
  "pin_or_configure_marvelous_designer_executable_version_and_sha256",
  "obtain_separate_operator_authority_for_WP_010_activation",
  "prove_dependency_main_containment_and_current_main_compatibility",
  "run_tailor_mt_pre_activation_semantic_gate"
]);
stub.activation_status.lifecycle = "REFINEMENT_APPROVED / NON_EXECUTION_STUB (HELD — v02.203 contract candidate, not packet-hydrated or activated)";
stub.activation_status.microtasks.status = "842 PRE_ACTIVATION CONTRACTS V02_203 RECONCILED (not packet-hydrated, not active)";
stub.activation_status.microtasks.total = FINAL_COUNT;
stub.activation_status.microtasks.note = "MT-001..782 preserve the prior Cloth/BodyKit plan with v02.203 supersessions; MT-783..842 add native runtime, providers, cinematic output, interchange, professional workspaces, privacy/pillars and final proof. All are PENDING and active=false.";
stub.activation_status.remaining_activation_steps_when_resumed = [
  "pin/configure the local Marvelous Designer executable path, version, and SHA-256",
  "prove dependency containment and current-main interface compatibility",
  "obtain separate WP activation authority through the Classic Orchestrator/Activation Manager lane",
  "create official packet.json with the accepted MT-001..842 architecture DAG",
  "prepare feat/WP-KERNEL-010 branch + wtc-* product worktree + backup branch",
  "synchronize Task Board, Build Order, traceability, and activation records",
  "emit ACTIVATION_READINESS before any implementation session"
];
stub.spec_trace.active_bundle_at_stub_time = ACTIVE_SPEC_ENTRYPOINT;
stub.spec_trace.roadmap_add_coverage = "Tailor is one Rust-native offline creative module with Cloth and BodyKit. BodyKit-v2 uses GNM-v3 default provider-neutral evaluation, high-resolution head/body/performance/conditioning primitives, professional native operator/model workspaces, PostgreSQL/EventLedger/privacy and canonical GLB/restricted USD delivery with optional Unreal native and manual Blender qualification.";
stub.draft_scope.historical_draft_scope_status = "RETAINED_FOR_PROVENANCE_ONLY_SUPERSEDED_BY_ACTIVE_v02_203";
stub.draft_scope.current_scope_correction = "Active v02.203 preserves native final-quality rendering and adds Rust-native offline BodyKit-v2 providers, cinematic performance/conditioning, professional UI/model routes, privacy and canonical interchange; legacy draft assumptions do not override it.";
stub.activation_status.spec_enrichment = {
  version: ACTIVE_SPEC_VERSION,
  entrypoint: ACTIVE_SPEC_ENTRYPOINT,
  status: "DONE_OPERATOR_APPROVED_ACTIVE_AND_VALIDATED",
  refinement_approval_evidence: "Operator approved the researched Tailor/BodyKit-v2 vision; v02.203 changelog and SIGNATURE_AUDIT record the governed enrichment.",
  master_spec_signature_recorded: true,
  activation_user_signature_consumed: false,
  activation_user_signature: null
};
stub.professional_production_hardening.verdict = "SPEC_V02_203_AND_MT_CONTRACTS_AUTHORED_PENDING_SEMANTIC_GATE_PACKET_DEPENDENCIES_AND_ACTIVATION";
stub.professional_production_hardening.build_readiness_prework = BUILD_READY_PATH;
stub.professional_production_hardening.technical_refinement_prework = TECHNICAL_REFINEMENT_PATH;
stub.professional_production_hardening.multi_lens_review = REVIEW_PATH;
stub.professional_production_hardening.parity_review_v3 = PARITY_V3_PATH;
stub.professional_production_hardening.resulting_mt_range = "MT-001..MT-842";
stub.professional_production_hardening.new_hardening_mts = FINAL_COUNT - ORIGINAL_COUNT;
stub.professional_production_hardening.dependency_shape = "ACYCLIC_ARCHITECTURAL_DAG_CANDIDATE_PENDING_SEMANTIC_CHECK";
stub.professional_production_hardening.implementation_authority = false;
stub.professional_production_hardening.activation_changed = false;
stub.professional_production_hardening.active_spec_changed = true;
stub.professional_production_hardening.active_spec_version = ACTIVE_SPEC_VERSION;
writeJson(STUB_PATH, stub);

console.log(JSON.stringify({ wp_id: WP_ID, active_spec: ACTIVE_SPEC_VERSION, source_reviewed: ORIGINAL_COUNT, final_count: FINAL_COUNT, new_mts: bodyKitV2Mts.length, group_counts: groupCounts, outputs: [INDEX_PATH, REVIEW_PATH, PARITY_V3_PATH, BUILD_READY_PATH, TECHNICAL_REFINEMENT_PATH, STUB_PATH] }, null, 2));
