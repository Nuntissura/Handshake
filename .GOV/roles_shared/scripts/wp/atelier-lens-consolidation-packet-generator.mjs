import fs from "node:fs";
import path from "node:path";
import {
  WORK_PACKET_CONTRACT_SCHEMA_ID,
  REFINEMENT_CONTRACT_SCHEMA_ID,
  MICRO_TASK_CONTRACT_SCHEMA_ID,
  DETERMINISTIC_CONTRACT_RED_TEAM_PROFILE,
  MACHINE_READABLE_ARTIFACT_POLICY,
  addOrReplaceGeneratedProjectionHeader,
  stableStringify,
  stampContractProjectionMetadata,
} from "../lib/packet-contract-lib.mjs";
import {
  formatClauseClosureMatrixSection,
  formatPacketAcceptanceMatrixSection,
  formatSharedSurfaceMonitoringSection,
  formatSpecDebtStatusSection,
} from "../lib/packet-closure-monitor-lib.mjs";
import { formatSemanticProofAssetsSection } from "../lib/semantic-proof-lib.mjs";
import {
  formatDataContractDecisionSection,
  formatDataContractMonitoringSection,
} from "../lib/data-contract-lib.mjs";

const WP_ID = "WP-1-Atelier-Lens-Consolidation-v1";
const BASE_WP_ID = "WP-1-Atelier-Lens-Consolidation";
const PACKET_FORMAT_VERSION = "2026-04-06";
const GENERATED_AT_UTC = "2026-05-16T03:39:00.000Z";
const USER_SIGNATURE = "ilja160520260339";
const GENERATOR = ".GOV/roles_shared/scripts/wp/atelier-lens-consolidation-packet-generator.mjs";
const REF_BASE = ".GOV/reference/ckc_atelier_lens_consolidation";
const PACKET_DIR = `.GOV/task_packets/${WP_ID}`;

function readJson(relPath) {
  return JSON.parse(fs.readFileSync(relPath, "utf8"));
}

const microtaskMap = readJson(`${REF_BASE}/greenroom-microtask-map.json`);
const preservationMap = readJson(`${REF_BASE}/handshake-stub-preservation-map.json`);
const overlapMatrix = readJson(`${REF_BASE}/greenroom-overlap-matrix.json`);
const evolvedRegister = readJson(`${REF_BASE}/greenroom-evolved-feature-register.json`);
const outputIndex = readJson(`${REF_BASE}/greenroom-output-index.json`);
const requirementsRegister = readJson(`${REF_BASE}/greenroom-requirements-register.json`);
const translationMatrix = readJson(`${REF_BASE}/greenroom-translation-matrix.json`);
const executionStatusPath = `${REF_BASE}/mt_execution/MT-001-012-status.json`;
const executionStatus = fs.existsSync(executionStatusPath) ? readJson(executionStatusPath) : { microtasks: [] };
const executionStatusByMt = new Map(
  (executionStatus.microtasks || []).map((entry) => [String(entry.mt_id || "").trim(), entry]),
);

const noSqliteRule = "SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. PostgreSQL/EventLedger/ArtifactStore are the only accepted storage and evidence direction.";

const inScopePaths = [
  ".GOV/reference/ckc_atelier_lens_consolidation/**",
  `${PACKET_DIR}/**`,
  ".GOV/task_packets/stubs/WP-1-Atelier-Lens-Consolidation-v1.md",
  ".GOV/task_packets/stubs/WP-1-Atelier-Lens-Consolidation-v1.contract.json",
  ".GOV/roles_shared/records/TASK_BOARD.md",
  ".GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md",
  ".GOV/roles_shared/records/FLAT_PACKET_LEGACY_INVENTORY.json",
  GENERATOR,
];

const outOfScopePaths = [
  "src/**",
  "app/**",
  "tests/**",
  ".GOV/spec/**",
  "../handshake_main/**",
  "D:/Projects/LLM projects/CastKit-Codex/**", // example: operator-local CastKit-Codex sibling project root (machine-local; replace with CASTKIT_CODEX_ROOT env-var indirection if relocated)
];

const specAnchors = [
  ".GOV/spec/SPEC_CURRENT.md",
  ".GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#FEAT-ATELIER-LENS",
  ".GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#FEAT-PHOTO-STUDIO",
  ".GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#PRIM-Moodboard",
  ".GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#TOOL-COMFYUI",
  ".GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md#Photo-Studio-and-Library-DAM-functions",
  ".GOV/operator/docs_local/handshake-v2-kernel-reset-brief.md#PostgreSQL-EventLedger-only-reset",
];

const acceptanceCriteria = [
  "Every source Atelier/Lens-adjacent stub is represented by a source-backed preservation row.",
  "CKC is preserved as an evolved sibling of Atelier/Lens and prompt-diary intent, not treated as a competing app.",
  "The overlap matrix maps CKC clusters to Handshake owners across Atelier/Lens, Photo Studio, Studio runtime, Loom/media/archive, artifact, and visual-debug surfaces.",
  "Every CKC evolved or convenience feature is classified as fold, dependency, defer, conflict, or operator-decision-needed.",
  "CKC runtime assumptions are translated to Handshake PostgreSQL/EventLedger/ArtifactStore/CRDT/promotion boundaries.",
  noSqliteRule,
  "Electron IPC, CKC localhost intake authority, .GOV product outputs, and CKC product namespace authority are rejected or translated.",
  "Future CKC rebuild stubs are deferred until this packet, CKC greenroom review, and CKC research basis are complete.",
  "The packet is detailed enough for no-context model execution and validator review without rereading all legacy stubs.",
  "Packet, refinement, microtask, taskboard, traceability, inventory, and projection contracts validate with the packet truth bundle.",
];

const clauseRows = [
  "CLAUSE: Source-stub no-loss preservation | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/handshake-stub-preservation-map.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: preservation stubs and carried-forward intent rows | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
  "CLAUSE: CKC/Atelier overlap matrix | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-overlap-matrix.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: OVR-001 through OVR-012 | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
  "CLAUSE: CKC evolved features | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-evolved-feature-register.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: EVOL-001 through EVOL-026 | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
  "CLAUSE: Runtime translation matrix | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-translation-matrix.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: module boundaries and conflict rows | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
  `CLAUSE: SQLite absolute rejection | CODE_SURFACES: ${REF_BASE}/greenroom-output-index.json, ${PACKET_DIR}/packet.md | TESTS: rg -n "SQLite" ${REF_BASE} ${PACKET_DIR} | EXAMPLES: no runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING`,
  "CLAUSE: Future CKC rebuild stubs gated | CODE_SURFACES: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.md, .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: deferred downstream WP register | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
  "CLAUSE: Model-facing manual diagnostics non-focus automation | CODE_SURFACES: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.md, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/MT-*.md | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: manual, visual debug, structured receipts, quiet model operation | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
  "CLAUSE: 75 MT coverage | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-microtask-map.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/MT-*.json | TESTS: node -e \"const fs=require('fs'); const c=fs.readdirSync('.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1').filter(f=>/^MT-\\\\d{3}\\\\.json$/.test(f)).length; if(c!==75) throw new Error(String(c));\" | EXAMPLES: MT-001 through MT-075 | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
  "CLAUSE: Task Board and registry point at official packet | CODE_SURFACES: .GOV/roles_shared/records/TASK_BOARD.md, .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: Ready for Dev entry and active registry row | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
  "CLAUSE: Contracts and projections stay in sync | CODE_SURFACES: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/*.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/*.md | TESTS: node .GOV/roles_shared/checks/packet-contract-projection-check.mjs | EXAMPLES: generated projection headers and source hashes | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
];

const tripwireTests = [
  "node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --check",
  "node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs",
  "node -e \"const fs=require('fs'); const c=fs.readdirSync('.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1').filter(f=>/^MT-\\\\d{3}\\\\.json$/.test(f)).length; if(c!==75) throw new Error(String(c));\"",
];

const researchSources = [
  {
    source: "ComfyUI SaveImage node documentation",
    url: "https://docs.comfy.org/built-in-nodes/SaveImage",
    pattern: "Image pipeline receipts need explicit output file, filename prefix, subfolder, and workflow prompt context; Handshake should map this into ArtifactStore/EventLedger receipts.",
  },
  {
    source: "ComfyUI V3 custom node migration documentation",
    url: "https://docs.comfy.org/custom-nodes/v3_migration",
    pattern: "ComfyUI integration must tolerate node API evolution and should use an adapter boundary instead of embedding CKC assumptions.",
  },
  {
    source: "ComfyUI-SaveImageWithMetaData GitHub project",
    url: "https://github.com/nkchocoai/ComfyUI-SaveImageWithMetaData",
    pattern: "Community workflows preserve generation metadata in output images; Handshake should preserve prompt/workflow metadata as explicit artifact provenance.",
  },
  {
    source: "OpenPose output documentation",
    url: "https://github.com/CMU-Perceptual-Computing-Lab/openpose/blob/master/doc/02_output.md",
    pattern: "Pose sidecars need stable schemas for body, face, and hand keypoints; CKC PoseKit should become a Handshake pose artifact contract.",
  },
  {
    source: "Playwright visual comparisons",
    url: "https://playwright.dev/docs/test-snapshots",
    pattern: "Visual/debug surfaces should be reproducible through deterministic snapshots and explicit evidence paths for model review.",
  },
  {
    source: "Automerge local-first CRDT overview",
    url: "https://automerge.org/",
    pattern: "Parallel model edits should flow through CRDT-friendly document and artifact boundaries instead of single-owner desktop state.",
  },
  {
    source: "PostgreSQL full text search documentation",
    url: "https://www.postgresql.org/docs/17/textsearch.html",
    pattern: "Search/tag/similarity features can start PostgreSQL-first with full-text search before specialized vector or media indexes.",
  },
  {
    source: "Tauri WebviewWindow API reference",
    url: "https://tauri.app/reference/javascript/api/namespacewebviewwindow/",
    pattern: "Window/tab/workspace behavior should translate to Handshake/Tauri-controlled surfaces rather than CKC Electron IPC authority.",
  },
];

function mdList(items, indent = "") {
  const normalized = (items || []).map((item) => String(item || "").trim()).filter(Boolean);
  if (normalized.length === 0) return `${indent}- NONE`;
  return normalized.map((item) => `${indent}- ${item}`).join("\n");
}

function pipeRows(rows, mapper) {
  return (rows || []).map(mapper).join("\n");
}

function slugFromTitle(title) {
  return String(title || "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80);
}

function classifyMicrotask(title, index) {
  if (/^Inventory/i.test(title)) return "Inventory";
  if (/^Extract/i.test(title)) return "Requirement extraction";
  if (/^Preserve/i.test(title)) return "Preservation";
  if (/^Build/i.test(title)) return "Translation";
  if (/^Select/i.test(title)) return "Fixture selection";
  if (/^Define/i.test(title)) return "Proof definition";
  if (/^Draft/i.test(title)) return "Packet authoring";
  return index > 68 ? "Runway closure" : "Consolidation";
}

function riskForMicrotask(title) {
  if (/SQLite/i.test(title)) return "SQLite could re-enter as a runtime, test, fixture, mock, fallback, cache, compatibility, import, export, or temporary harness assumption.";
  if (/Electron|Tauri|localhost|namespace|product-output/i.test(title)) return "CKC runtime authority could be copied into Handshake instead of translated to Handshake boundaries.";
  if (/Preserve/i.test(title)) return "Existing Atelier/Lens intent could be silently dropped during CKC folding.";
  if (/ComfyUI|PoseKit|OpenPose/i.test(title)) return "Media pipeline behavior could lose provenance, schema, or replay evidence.";
  if (/fixture/i.test(title)) return "Future implementation could lack source-backed acceptance examples.";
  return "The official consolidation packet could become incomplete and force later rework.";
}

function executionForMicrotask(mtId) {
  const status = executionStatusByMt.get(mtId);
  if (!status) {
    return {
      status: "PENDING",
      evidence_artifacts: [],
      completion_note: "",
      completed_at_utc: "",
    };
  }
  return {
    status: String(status.status || "PENDING").trim().toUpperCase(),
    evidence_artifacts: Array.isArray(status.evidence_artifacts) ? status.evidence_artifacts : [],
    completion_note: String(status.completion_note || "").trim(),
    completed_at_utc: String(status.completed_at_utc || executionStatus.completed_at_utc || "").trim(),
  };
}

function writeContractProjection({
  contractPath,
  projectionPath,
  contract,
  projectionBody,
}) {
  const stamped = stampContractProjectionMetadata(contract, {
    projectionPath,
    projectionBody,
    sourceFile: contractPath,
    generator: GENERATOR,
    generatedAtUtc: GENERATED_AT_UTC,
  });
  const projection = addOrReplaceGeneratedProjectionHeader(projectionBody, stamped, {
    sourceFile: contractPath,
  });
  fs.writeFileSync(contractPath, stableStringify(stamped));
  fs.writeFileSync(projectionPath, projection.endsWith("\n") ? projection : `${projection}\n`);
}

function packetMarkdownBody() {
  return `# ${WP_ID}: Atelier/Lens Consolidation and CKC Fold-In

## METADATA
- WP_ID: ${WP_ID}
- BASE_WP_ID: ${BASE_WP_ID}
- **Status:** Ready for Dev
- DATE: 2026-05-16
- USER_SIGNATURE: ${USER_SIGNATURE}
- PACKET_FORMAT_VERSION: ${PACKET_FORMAT_VERSION}
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
- RISK_TIER: HIGH
- CURRENT_WP_STATUS: READY_FOR_DEV
- MAIN_CONTAINMENT_STATUS: NOT_STARTED
- MERGED_MAIN_COMMIT: NONE
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- WORKFLOW_AUTHORITY: ORCHESTRATOR
- TECHNICAL_ADVISOR: WP_VALIDATOR
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
- AGENTIC_MODE: NO
- EXECUTION_OWNER: CODER_A
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- LOCAL_BRANCH: feat/WP-1-Atelier-Lens-Consolidation-v1
- LOCAL_WORKTREE_DIR: ../wtc-lens-consolidation-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Atelier-Lens-Consolidation-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Atelier-Lens-Consolidation-v1
- BACKUP_PUSH_STATUS: NOT_REQUIRED
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next ${WP_ID}
- WP_VALIDATOR_LOCAL_BRANCH: feat/${WP_ID}
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-lens-consolidation-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR ${WP_ID}
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/${WP_ID}
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/${WP_ID}
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR ${WP_ID}
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/${WP_ID}
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/${WP_ID}
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.5
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.4
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL: gpt-5.5
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL: gpt-5.5
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- INTEGRATION_VALIDATOR_MODEL: gpt-5.5
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}/RECEIPTS.jsonl
- WP_NOTIFICATIONS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}/NOTIFICATIONS.jsonl
- COMMUNICATION_CONTRACT: WP_COMMUNICATION_V1
- COMMUNICATION_HEALTH_GATE: REQUIRED_BEFORE_CLAIM
- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A
- PACKET_WIDENING_DECISION: NONE
- PACKET_WIDENING_EVIDENCE: N/A
- TOUCHED_FILE_BUDGET: 140
- BROAD_TOOL_ALLOWLIST: NONE
- DATA_CONTRACT_PROFILE: NONE
- REFINEMENT: ${PACKET_DIR}/refinement.md
- MICROTASK_GLOB: ${PACKET_DIR}/MT-*.md

## OPERATOR_REQUEST
The operator corrected the workflow: this is today's task. First consolidate all Atelier/Lens work packet stubs without losing their original intent, then fold CKC into that preserved Atelier/Lens runway, then only after greenroom and CKC research create CKC rebuild stubs. CKC is an evolved sibling of the same prompt-diary and Atelier/Lens goal, so its convenience features must be preserved and translated into Handshake instead of discarded.

## SCOPE_SUMMARY
This packet promotes the greenroom output into the official Ready for Dev consolidation packet. It is not product implementation. It turns CKC source evidence, CKC spec/taskboard evidence, and existing Handshake Atelier/Lens-adjacent stubs into a single execution contract, refinement, and 75 microtasks.

## IN_SCOPE_PATHS
${mdList(inScopePaths, "  ")}

## OUT_OF_SCOPE
${mdList(outOfScopePaths, "  ")}

## SPEC_ANCHORS
${mdList(specAnchors, "  ")}

## STORAGE_AND_RUNTIME_CONSTRAINTS
- ${noSqliteRule}
- CKC SQLite assumptions are rejected even for tests, fixtures, mocks, examples, fallback cache, temporary adapters, compatibility shims, imports, exports, or demo harnesses.
- CKC Electron IPC is source evidence only; Handshake implementation must use Handshake/Tauri command, window, event, and workspace contracts.
- CKC localhost intake authority is source evidence only; Handshake implementation must use governed ingestion endpoints, ArtifactStore receipts, EventLedger entries, and model-visible diagnostics.
- CKC product namespace and .GOV product-output habits are rejected; generated product data belongs in Handshake product surfaces, while repo governance stays under .GOV.

## SOURCE_EVIDENCE
- Greenroom output index: ${REF_BASE}/greenroom-output-index.json
- Overlap matrix: ${REF_BASE}/greenroom-overlap-matrix.json
- Evolved feature register: ${REF_BASE}/greenroom-evolved-feature-register.json
- Requirements register: ${REF_BASE}/greenroom-requirements-register.json
- Translation matrix: ${REF_BASE}/greenroom-translation-matrix.json
- Microtask map: ${REF_BASE}/greenroom-microtask-map.json
- Stub preservation map: ${REF_BASE}/handshake-stub-preservation-map.json

## PRESERVATION_REQUIREMENTS
${mdList((preservationMap.stubs || []).map((stub) => `${stub.wp_id}: ${stub.intent}`), "  ")}

## OVERLAP_MATRIX_SUMMARY
${pipeRows(overlapMatrix.rows || [], (row) => `- ${row.id}: ${row.overlap_area} -> ${row.decision}; preserve: ${row.preserve}`)}

## CKC_EVOLVED_FEATURES
${pipeRows(evolvedRegister.features || [], (feature) => `- ${feature.id}: ${feature.feature} -> ${feature.decision}; why: ${feature.why}`)}

## TRANSLATION_REQUIREMENTS
${mdList((translationMatrix.conflicts || []).map((row) => `${row.id}: ${row.ckc_assumption} => ${row.handshake_resolution}`), "  ")}

## ACCEPTANCE_CRITERIA
${mdList(acceptanceCriteria, "  ")}

${formatClauseClosureMatrixSection(clauseRows)}

${formatPacketAcceptanceMatrixSection(clauseRows)}

${formatSpecDebtStatusSection({ openSpecDebt: "NO", blockingSpecDebt: "NO", debtIds: [] })}

${formatSharedSurfaceMonitoringSection({
    sharedSurfaceRisk: "YES",
    hotFiles: [
      `${PACKET_DIR}/packet.md`,
      `${PACKET_DIR}/packet.json`,
      `${PACKET_DIR}/refinement.md`,
      `${PACKET_DIR}/refinement.json`,
      `${PACKET_DIR}/MT-*.md`,
      `${PACKET_DIR}/MT-*.json`,
      ".GOV/roles_shared/records/TASK_BOARD.md",
      ".GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md",
      ".GOV/roles_shared/records/FLAT_PACKET_LEGACY_INVENTORY.json",
      `${REF_BASE}/**`,
    ],
    requiredTripwireTests: tripwireTests,
    postMergeSpotcheckRequired: "YES",
  })}

${formatDataContractDecisionSection({
    decision: "WAIVED_NOT_DATA_BEARING",
    reason: "This activation packet writes governance packet/refinement/microtask surfaces only. It does not implement product data schemas, migrations, runtime persistence, or product code.",
    evidence: [
      `IN_SCOPE_PATHS reviewed: ${inScopePaths.join(", ")}`,
      "No src, app, tests, migration, backend storage, schema, DTO, or product runtime path is in scope.",
      noSqliteRule,
    ],
  })}

${formatDataContractMonitoringSection({ profile: "NONE", inScopePaths })}

## MICROTASK_PLAN
${mdList((microtaskMap.microtask_buckets || []).map((title, index) => `MT-${String(index + 1).padStart(3, "0")}: ${title}`), "  ")}

## VALIDATION_PLAN
${mdList([
    "Parse all generated JSON contracts and reference registers.",
    "Run node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --check.",
    "Run node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs.",
    "Confirm 75 MT JSON contracts exist and all generated projection headers are in sync.",
    "Search new packet/reference surfaces for weak SQLite language and stale Greenroom-WP phrasing.",
  ], "  ")}

## VALIDATION_REPORTS
- Status: PENDING
- CLAUSES_REVIEWED:
  - NONE
- NOT_PROVEN:
  - All clause rows pending coder execution and validator confirmation.
- SPEC_ALIGNMENT_VERDICT: PENDING

${formatSemanticProofAssetsSection({
    semanticTripwireTests: tripwireTests,
    canonicalContractExamples: [
      `${PACKET_DIR}/packet.json (work_packet_contract schema)`,
      `${PACKET_DIR}/refinement.json (refinement_contract schema)`,
      `${PACKET_DIR}/MT-001.json through MT-075.json (microtask_contract schema)`,
      `${REF_BASE}/greenroom-overlap-matrix.json (CKC/Atelier overlap rows OVR-001..OVR-012)`,
      `${REF_BASE}/greenroom-evolved-feature-register.json (CKC evolved features EVOL-001..EVOL-026)`,
      `${REF_BASE}/handshake-stub-preservation-map.json (Atelier/Lens stub preservation rows)`,
      `${REF_BASE}/greenroom-translation-matrix.json (Handshake runtime translation rows)`,
    ],
  })}

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: PREPARE assignment pending; WP communications will record live progress.
`;
}

function refinementMarkdownBody() {
  return `# ${WP_ID} Refinement: Atelier/Lens Consolidation and CKC Fold-In

## METADATA
- WP_ID: ${WP_ID}
- BASE_WP_ID: ${BASE_WP_ID}
- STATUS: READY_FOR_DEV
- USER_SIGNATURE: ${USER_SIGNATURE}
- PACKET_FORMAT_VERSION: ${PACKET_FORMAT_VERSION}
- SOURCE_PACKET: ${PACKET_DIR}/packet.md
- SOURCE_REFERENCE_ROOT: ${REF_BASE}

## OPERATOR_INTENT
The operator wants momentum on the original prompt-diary to Atelier/Lens goal: a media viewer, character sheet, media pipeline, and production companion folded into Handshake instead of maintained as a separate CKC app. Existing Atelier/Lens stubs must not be discarded. CKC is more evolved because it was built from unexpected need and convenience, so those extra behaviors must be preserved, classified, and translated into Handshake.

## REFINEMENT_STANCE
- Preserve-first consolidation: every existing Atelier/Lens-adjacent stub remains source material unless the operator explicitly supersedes it.
- CKC fold-in: CKC features become evidence for Atelier/Lens, Photo Studio, Studio runtime, media lineage, artifact, and visual-debug surfaces.
- No premature CKC rebuild stubs: downstream CKC implementation packets remain deferred until this consolidation and CKC research basis are complete.
- ${noSqliteRule}

## SOURCE_STUB_PRESERVATION
${mdList((preservationMap.stubs || []).map((stub) => `${stub.wp_id}: intent=${stub.intent}; handling=${stub.ckc_handling}; risks=${(stub.risks || []).join("; ") || "NONE"}`), "  ")}

## PRESERVED_INTENT_GROUPS
${mdList((preservationMap.preserved_intents || []).map((item) => String(item)), "  ")}

## CONFLICTS_AND_LAYERED_HANDLING
${mdList((preservationMap.conflicts || []).map((conflict) => `${conflict.id}: ${conflict.summary}; risk=${conflict.risk}; handling=${conflict.layered_handling}`), "  ")}

## CKC_CAPABILITY_CLUSTERS
${mdList((requirementsRegister.ckc_capability_clusters || []).map((cluster) => `${cluster.id}: ${cluster.preserve}`), "  ")}

## HANDSHAKE_STUB_GOALS
${mdList((requirementsRegister.handshake_stub_goals || []).map((goal) => `${goal.wp_id}: ${goal.preserve}`), "  ")}

## OVERLAP_ROWS
${pipeRows(overlapMatrix.rows || [], (row) => `- ${row.id}: area=${row.overlap_area}; CKC=${(row.ckc_sources || []).join(", ")}; Handshake=${(row.handshake_sources || []).join(", ")}; decision=${row.decision}; preserve=${row.preserve}`)}

## EVOLVED_FEATURE_ROWS
${pipeRows(evolvedRegister.features || [], (feature) => `- ${feature.id}: feature=${feature.feature}; decision=${feature.decision}; why=${feature.why}`)}

## TRANSLATION_MATRIX
${mdList((translationMatrix.module_boundaries || []).map((row) => String(row)), "  ")}

## RUNTIME_REJECTIONS
${mdList(outputIndex.runtime_rejections || [], "  ")}
- ${noSqliteRule}

## RESEARCH_BASIS
${mdList(researchSources.map((item) => `${item.source}: ${item.url} -> ${item.pattern}`), "  ")}

## REUSE_OPPORTUNITIES
- Handshake PostgreSQL/EventLedger/ArtifactStore boundaries replace CKC SQLite, file-local persistence, and localhost authority.
- Handshake CRDT/workspace surfaces carry parallel model editing and tab/window placement needs without copying CKC desktop state.
- Handshake visual-debug and screenshot validation work can absorb CKC media review and contact-sheet review evidence.
- Existing Photo Studio, Atelier/Lens, Lens ViewMode, Lens Extraction Tier, Stage media artifact portability, Loom archive, and artifact-system packets remain the runway for implementation.

## REJECTED_OPTIONS
- Keep CKC as a separate source of product authority: rejected because the operator wants CKC folded into Handshake Atelier/Lens.
- Create CKC rebuild stubs before consolidation: rejected because the operator made consolidation first the current task.
- Use SQLite for runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths: rejected absolutely.
- Copy Electron IPC, localhost intake, or CKC namespace authority into Handshake: rejected; translate to Handshake stack boundaries.

## RED_TEAM
- Risk: CKC convenience features are treated as optional extras and lost. Mitigation: evolved feature register is acceptance evidence, and MTs classify every extra as folded, dependency, deferred, conflict, or operator-decision-needed.
- Risk: Atelier/Lens stub intent is overwritten by CKC scope. Mitigation: preservation map is a required source and MT-027 through MT-038 preserve source stub intent before rebuild work.
- Risk: SQLite sneaks back through tests, fixtures, caches, import/export compatibility, or temporary harnesses. Mitigation: absolute rejection is a clause row, acceptance criterion, and MT-039.
- Risk: governance and product code blur. Mitigation: this packet is governance-only; future implementation packets must write product code under Handshake surfaces and keep .GOV for repo governance.
- Risk: future coders implement UI first. Mitigation: packet constrains the next step to language/tech-stack translation and source-backed CKC fold-in, not GUI redesign.

## MICROTASK_GROUPS
${mdList((microtaskMap.microtask_buckets || []).map((title, index) => `MT-${String(index + 1).padStart(3, "0")}: ${classifyMicrotask(title, index + 1)} - ${title}`), "  ")}

## ACCEPTANCE_CRITERIA
${mdList(acceptanceCriteria, "  ")}

## VALIDATION_REQUIREMENTS
${mdList(tripwireTests, "  ")}
`;
}

function packetContract() {
  return {
    schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID,
    schema_version: "work_packet_contract_v1",
    contract_authority: "PRIMARY_MACHINE_READABLE",
    artifact_policy: MACHINE_READABLE_ARTIFACT_POLICY,
    wp_id: WP_ID,
    base_wp_id: BASE_WP_ID,
    created_at_utc: GENERATED_AT_UTC,
    updated_at_utc: GENERATED_AT_UTC,
    source_control: {
      merge_base_sha: "NONE",
      canonical_branch: "main",
      work_branch: "feat/WP-1-Atelier-Lens-Consolidation-v1",
      worktree_dir: "../wtc-lens-consolidation-v1",
      remote_backup_branch: "feat/WP-1-Atelier-Lens-Consolidation-v1",
      remote_backup_url: "https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Atelier-Lens-Consolidation-v1",
      backup_push_status: "NOT_REQUIRED",
    },
    workflow: {
      lane: "ORCHESTRATOR_MANAGED",
      authority: "ORCHESTRATOR",
      technical_advisor: "WP_VALIDATOR",
      technical_authority: "INTEGRATION_VALIDATOR",
      merge_authority: "INTEGRATION_VALIDATOR",
      agentic_mode: "NO",
      execution_owner: "CODER_A",
      session_start_authority: "ORCHESTRATOR_ONLY",
      host_preference: "HANDSHAKE_ACP_BROKER",
      communication_dir: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}`,
      thread_file: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}/THREAD.md`,
      runtime_status_file: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}/RUNTIME_STATUS.json`,
      receipts_file: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}/RECEIPTS.jsonl`,
      notifications_file: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/${WP_ID}/NOTIFICATIONS.jsonl`,
      communication_contract: "WP_COMMUNICATION_V1",
      communication_health_gate: "REQUIRED_BEFORE_CLAIM",
    },
    lifecycle: {
      status: "Ready for Dev",
      main_containment_status: "NOT_STARTED",
      merged_main_commit: "NONE",
      main_containment_verified_at_utc: "N/A",
      current_main_compatibility_status: "NOT_RUN",
      current_main_compatibility_baseline_sha: "NONE",
      current_main_compatibility_verified_at_utc: "N/A",
      packet_widening_decision: "NONE",
      packet_widening_evidence: "N/A",
      current_wp_status: "READY_FOR_DEV",
      risk_tier: "HIGH",
      user_signature: USER_SIGNATURE,
      packet_format_version: PACKET_FORMAT_VERSION,
      clause_closure_monitor_profile: "CLAUSE_MONITOR_V1",
      wp_validator_of_record: "UNCLAIMED",
      integration_validator_of_record: "UNCLAIMED",
      orchestrator_model_profile: "OPENAI_GPT_5_5_XHIGH",
      orchestrator_model: "N/A",
      orchestrator_reasoning_strength: "EXTRA_HIGH",
      role_model_profile_policy: "ROLE_MODEL_PROFILE_CATALOG_V1",
      role_session_primary_model: "gpt-5.5",
      role_session_fallback_model: "gpt-5.4",
      semantic_proof_profile: "DIFF_SCOPED_SEMANTIC_V1",
      coder_model_profile: "OPENAI_GPT_5_5_XHIGH",
      coder_model: "gpt-5.5",
      coder_reasoning_strength: "EXTRA_HIGH",
      wp_validator_model_profile: "OPENAI_GPT_5_5_XHIGH",
      wp_validator_model: "gpt-5.5",
      wp_validator_reasoning_strength: "EXTRA_HIGH",
      integration_validator_model_profile: "OPENAI_GPT_5_5_XHIGH",
      integration_validator_model: "gpt-5.5",
      integration_validator_reasoning_strength: "EXTRA_HIGH",
    },
    authority_files: {
      packet_contract: `${PACKET_DIR}/packet.json`,
      packet_projection: `${PACKET_DIR}/packet.md`,
      refinement_contract: `${PACKET_DIR}/refinement.json`,
      microtask_contract_glob: `${PACKET_DIR}/MT-*.json`,
    },
    scope: {
      summary: "Promote CKC greenroom evidence and Atelier/Lens-adjacent stub preservation into one official Ready for Dev governance packet.",
      why: "The operator wants CKC folded into Handshake Atelier/Lens without losing pre-existing Atelier/Lens intent or CKC convenience-driven evolution.",
      allowed_paths: inScopePaths,
      forbidden_paths: outOfScopePaths,
      spec_anchors: specAnchors,
      touched_file_budget: 140,
      broad_tool_allowlist: ["NONE"],
      acceptance_criteria: acceptanceCriteria,
    },
    refinement: {
      contract_file: `${PACKET_DIR}/refinement.json`,
      projection_file: `${PACKET_DIR}/refinement.md`,
      status: "READY_FOR_DEV",
      activation_manager_required: true,
      enforcement_profile: "SIGNED_REFINEMENT_REQUIRED",
      hydration_profile: "GREENROOM_REFERENCE_AND_STUB_PRESERVATION",
    },
    microtasks: {
      contract_glob: `${PACKET_DIR}/MT-*.json`,
      declared_ids: (microtaskMap.microtask_buckets || []).map((_, index) => `MT-${String(index + 1).padStart(3, "0")}`),
      active_id: null,
      next_id: "MT-001",
      count: (microtaskMap.microtask_buckets || []).length,
    },
    data_contract: {
      profile: "NONE",
      decision: "WAIVED_NOT_DATA_BEARING",
      reason: "Governance-only activation packet; product data contracts must be handled by future implementation WPs.",
      no_sqlite_rule: noSqliteRule,
    },
    research_basis: researchSources,
    red_team: {
      required: true,
      profile: DETERMINISTIC_CONTRACT_RED_TEAM_PROFILE,
      assumptions_to_attack: [
        "CKC and Atelier/Lens are separate enough to split now.",
        "CKC convenience features can be deferred without recording their intent.",
        "SQLite can be harmless in test or fixture harnesses.",
        "Electron/localhost patterns can be copied into Handshake because they worked in CKC.",
      ],
      minimum_controls: [
        "Preservation map and evolved feature register are acceptance evidence.",
        noSqliteRule,
        "Future CKC rebuild stubs remain gated until this packet closes and CKC research is complete.",
      ],
    },
  };
}

function refinementContract() {
  return {
    schema_id: REFINEMENT_CONTRACT_SCHEMA_ID,
    schema_version: "refinement_contract_v1",
    contract_authority: "PRIMARY_MACHINE_READABLE",
    artifact_policy: MACHINE_READABLE_ARTIFACT_POLICY,
    wp_id: WP_ID,
    base_wp_id: BASE_WP_ID,
    status: "READY_FOR_DEV",
    created_at_utc: GENERATED_AT_UTC,
    updated_at_utc: GENERATED_AT_UTC,
    source_packet: `${PACKET_DIR}/packet.json`,
    source_references: outputIndex.outputs || [],
    source_agent_drafts: outputIndex.agent_drafts || [],
    source_stub_preservation: preservationMap.stubs || [],
    overlap_rows: overlapMatrix.rows || [],
    evolved_feature_rows: evolvedRegister.features || [],
    translation_conflicts: translationMatrix.conflicts || [],
    research_basis: researchSources,
    no_sqlite_rule: noSqliteRule,
    acceptance_criteria: acceptanceCriteria,
    red_team: {
      profile: DETERMINISTIC_CONTRACT_RED_TEAM_PROFILE,
      risks: [
        "Lost Atelier/Lens stub intent.",
        "Lost CKC convenience feature intent.",
        "SQLite backdoor through tests or fixtures.",
        "CKC runtime copied instead of translated.",
      ],
      controls: [
        "Preservation-first rows.",
        "Feature classification rows.",
        "Absolute no-SQLite clause.",
        "Handshake runtime translation matrix.",
      ],
    },
  };
}

function microtaskMarkdownBody({ id, title, index }) {
  const dependsOn = index <= 12 ? "NONE" : `MT-${String(Math.max(1, index - 1)).padStart(3, "0")}`;
  const execution = executionForMicrotask(id);
  const executionSection = execution.status === "COMPLETE"
    ? `
## EXECUTION_EVIDENCE
- COMPLETED_AT_UTC: ${execution.completed_at_utc || "2026-05-16T04:45:00.000Z"}
- COMPLETION_NOTE: ${execution.completion_note || "Completed by source-backed inventory execution."}
- EVIDENCE_ARTIFACTS:
${mdList(execution.evidence_artifacts, "  ")}
`
    : "";
  return `# ${id}: ${title}

## METADATA
- WP_ID: ${WP_ID}
- MT_ID: ${id}
- STATUS: ${execution.status}
- DEPENDS_ON: ${dependsOn}
- CLAUSE: ${title}
- CATEGORY: ${classifyMicrotask(title, index)}
- CODE_SURFACES: ${REF_BASE}/**, ${PACKET_DIR}/**
- EXPECTED_TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs
- RISK_IF_MISSED: ${riskForMicrotask(title)}

## INPUTS
- Greenroom output index: ${REF_BASE}/greenroom-output-index.json
- Stub preservation map: ${REF_BASE}/handshake-stub-preservation-map.json
- Requirements register: ${REF_BASE}/greenroom-requirements-register.json
- Translation matrix: ${REF_BASE}/greenroom-translation-matrix.json
- Microtask source bucket: ${title}

## WORK
- Execute this microtask as part of the Atelier/Lens consolidation packet, not as a separate CKC rebuild packet.
- Preserve original Atelier/Lens and adjacent stub intent first, then fold CKC evidence into the Handshake owner surface.
- Classify any CKC extra behavior as folded, dependency, deferred, conflict, or operator-decision-needed.
- Translate CKC runtime assumptions to Handshake PostgreSQL/EventLedger/ArtifactStore/CRDT/promotion boundaries.
- ${noSqliteRule}

## ACCEPTANCE
- The microtask output is source-backed by the greenroom registers or preserved Handshake stubs.
- No original source intent is discarded silently.
- No CKC-only runtime authority is copied into Handshake.
- SQLite remains rejected for runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- The packet/refinement/microtask projection stays aligned with ${PACKET_DIR}/packet.json and ${PACKET_DIR}/refinement.json.

## VERIFICATION
- Run node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs.
- Run node .GOV/roles_shared/checks/packet-contract-projection-check.mjs.
- Inspect this microtask for source-backed evidence and no-SQLite compliance.
${executionSection}
`;
}

function microtaskContract({ id, title, index }) {
  const execution = executionForMicrotask(id);
  return {
    schema_id: MICRO_TASK_CONTRACT_SCHEMA_ID,
    schema_version: "microtask_contract_v1",
    contract_authority: "PRIMARY_MACHINE_READABLE",
    artifact_policy: MACHINE_READABLE_ARTIFACT_POLICY,
    wp_id: WP_ID,
    mt_id: id,
    title,
    slug: slugFromTitle(title),
    status: execution.status,
    created_at_utc: GENERATED_AT_UTC,
    updated_at_utc: GENERATED_AT_UTC,
    depends_on: index <= 12 ? [] : [`MT-${String(Math.max(1, index - 1)).padStart(3, "0")}`],
    category: classifyMicrotask(title, index),
    clause: title,
    code_surfaces: [`${REF_BASE}/**`, `${PACKET_DIR}/**`],
    expected_tests: [
      "node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs",
      "node .GOV/roles_shared/checks/packet-contract-projection-check.mjs",
    ],
    execution_evidence: {
      completed_at_utc: execution.completed_at_utc || null,
      evidence_artifacts: execution.evidence_artifacts,
      completion_note: execution.completion_note || null,
    },
    risk_if_missed: riskForMicrotask(title),
    no_sqlite_rule: noSqliteRule,
    acceptance: [
      "Source-backed output.",
      "No silent loss of Atelier/Lens or CKC intent.",
      "No CKC-only runtime authority copied into Handshake.",
      noSqliteRule,
    ],
  };
}

function main() {
  const buckets = microtaskMap.microtask_buckets || [];
  if (buckets.length !== 75) {
    throw new Error(`Expected 75 microtask buckets, found ${buckets.length}`);
  }

  fs.mkdirSync(PACKET_DIR, { recursive: true });

  writeContractProjection({
    contractPath: `${PACKET_DIR}/packet.json`,
    projectionPath: `${PACKET_DIR}/packet.md`,
    contract: packetContract(),
    projectionBody: packetMarkdownBody(),
  });

  writeContractProjection({
    contractPath: `${PACKET_DIR}/refinement.json`,
    projectionPath: `${PACKET_DIR}/refinement.md`,
    contract: refinementContract(),
    projectionBody: refinementMarkdownBody(),
  });

  buckets.forEach((title, idx) => {
    const index = idx + 1;
    const id = `MT-${String(index).padStart(3, "0")}`;
    writeContractProjection({
      contractPath: `${PACKET_DIR}/${id}.json`,
      projectionPath: `${PACKET_DIR}/${id}.md`,
      contract: microtaskContract({ id, title, index }),
      projectionBody: microtaskMarkdownBody({ id, title, index }),
    });
  });

  console.log(`Generated ${WP_ID}: packet, refinement, and ${buckets.length} microtasks.`);
}

main();
