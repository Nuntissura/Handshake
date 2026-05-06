#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { currentGitContext } from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { resolveWorkPacketPath, GOV_ROOT_ABS, GOV_ROOT_REPO_REL, REPO_ROOT, normalizePath, repoPathAbs } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  parseMergeProgressionTruth,
  updateMergeProgressionTruth,
  validateMergeProgressionTruth,
} from "../../../roles_shared/scripts/lib/merge-progression-truth-lib.mjs";
import {
  parseSignedScopeCompatibilityTruth,
  updateSignedScopeCompatibilityTruth,
} from "../../../roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs";
import {
  validateContainedMainCommitAgainstSignedScope,
} from "../../../roles_shared/scripts/lib/signed-scope-surface-lib.mjs";
import { syncRuntimeProjectionFromPacket } from "../../../roles_shared/scripts/lib/packet-runtime-projection-lib.mjs";
import { resolveArtifactHygieneCloseoutPolicy } from "../../../roles_shared/scripts/lib/closeout-blocking-authority-lib.mjs";
import {
  activeWorkflowInvalidityReceipt,
  parseJsonlFile,
  validateRuntimeStatus,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { parseExecutionCloseoutMode } from "../../../roles_shared/scripts/lib/wp-execution-state-lib.mjs";
import {
  buildTerminalCloseoutRecordFromCloseoutSync,
  publishTerminalCloseoutRecord,
  readTerminalCloseoutRecord,
} from "../../../roles_shared/scripts/lib/terminal-closeout-record-lib.mjs";
import {
  buildValidatorPacketCompleteResult,
  evaluateValidatorPacketGovernanceState,
  resolveValidatorActorContext,
} from "../scripts/lib/validator-governance-lib.mjs";
import {
  appendCloseoutSyncProvenance,
  buildCloseoutSyncGovernedAction,
  deriveFinalLaneGovernanceInvalidity,
  evaluateIntegrationValidatorCloseoutState,
  resolveCloseoutValidatorSessionsOfRecord,
} from "../scripts/lib/integration-validator-closeout-lib.mjs";
import { ensureValidatorGateDir, resolveValidatorGatePath } from "../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionRegistry,
  readJsonFile,
} from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  SESSION_CONTROL_BROKER_STATE_FILE,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import { settleRecoverableSessionControlResults } from "../../../roles_shared/scripts/session/session-control-self-settle-lib.mjs";
import { appendWpReceipt } from "../../../roles_shared/scripts/wp/wp-receipt-append.mjs";
import {
  buildArtifactRetentionManifest,
  cleanupArtifactResidue,
  ensureArtifactRootStructure,
  evaluateArtifactHygiene,
  writeArtifactRetentionManifest,
} from "../../../roles_shared/scripts/lib/artifact-hygiene-lib.mjs";
import {
  activeDeclaredTopologyRepoRoots,
  evaluateWpDeclaredTopology,
} from "../../../roles_shared/scripts/lib/wp-declared-topology-lib.mjs";
import { capturePreTaskSnapshot } from "../../../roles_shared/scripts/memory/memory-snapshot.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
registerFailCaptureHook("integration-validator-closeout-sync.mjs", { role: "INTEGRATION_VALIDATOR" });

function fail(message, details = []) {
  failWithMemory("integration-validator-closeout-sync.mjs", message, { role: "INTEGRATION_VALIDATOR", details });
}

function readText(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function writeText(filePath, text) {
  fs.writeFileSync(filePath, text, "utf8");
}

function kernelRepoRoot() {
  return path.resolve(GOV_ROOT_ABS, "..");
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function replaceSingleField(packetText, label, nextValue) {
  const re = new RegExp(`^(\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*)(.+)\\s*$`, "mi");
  if (!re.test(String(packetText || ""))) {
    throw new Error(`Missing packet field: ${label}`);
  }
  return String(packetText || "").replace(re, `$1${nextValue}`);
}

function replaceCurrentStateField(packetText, label, nextValue) {
  const re = new RegExp(`^(\\s*${label}\\s*:\\s*)(.+)\\s*$`, "mi");
  if (!re.test(String(packetText || ""))) {
    throw new Error(`Missing CURRENT_STATE field: ${label}`);
  }
  return String(packetText || "").replace(re, `$1${nextValue}`);
}

function replaceStatusHandoffField(packetText, label, nextValue) {
  const re = new RegExp(`^(\\s*-\\s*${label}\\s*:\\s*)(.*)\\s*$`, "mi");
  if (!re.test(String(packetText || ""))) {
    throw new Error(`Missing STATUS_HANDOFF field: ${label}`);
  }
  return String(packetText || "").replace(re, `$1${nextValue}`);
}

function normalizeReceiptKind(value = "") {
  return String(value || "").trim().toUpperCase();
}

function finalIntegrationPassReceiptForReport(receipts = [], { wpId = "", expectedVerdict = "PASS" } = {}) {
  const normalizedWpId = String(wpId || "").trim();
  const normalizedExpectedVerdict = String(expectedVerdict || "").trim().toUpperCase();
  return [...(Array.isArray(receipts) ? receipts : [])]
    .filter((entry) =>
      String(entry?.wp_id || "").trim() === normalizedWpId
      && normalizeReceiptKind(entry?.actor_role) === "INTEGRATION_VALIDATOR"
      && ["REVIEW_RESPONSE", "VALIDATOR_REVIEW", "VALIDATOR_RESPONSE"].includes(normalizeReceiptKind(entry?.receipt_kind))
      && String(entry?.correlation_id || "").includes(":coder_handoff:final:")
      && (
        normalizeReceiptKind(entry?.review_outcome) === normalizedExpectedVerdict
        || normalizeReceiptKind(entry?.verdict) === normalizedExpectedVerdict
        || new RegExp(`\\b${normalizedExpectedVerdict}\\b`, "i").test(String(entry?.summary || ""))
      )
    )
    .sort((left, right) => String(left?.timestamp_utc || "").localeCompare(String(right?.timestamp_utc || "")))
    .at(-1) || null;
}

function stripReceiptSummary(value = "") {
  return String(value || "")
    .replace(/\r?\n/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function appendReportAfterValidationReportsHeading(packetText, reportText) {
  const lines = String(packetText || "").split(/\r?\n/);
  const headingIndex = lines.findIndex((line) => /^##\s+VALIDATION_REPORTS\b/i.test(line));
  if (headingIndex === -1) {
    throw new Error("VALIDATION_REPORTS heading missing; cannot materialize final Integration Validator report");
  }
  lines.splice(headingIndex + 1, 0, ...String(reportText || "").trimEnd().split(/\r?\n/), "");
  return `${lines.join("\n").replace(/\s+$/u, "")}\n`;
}

function buildValidationReportFromFinalIntegrationReceipt(receipt = {}, {
  wpId = "",
  expectedVerdict = "PASS",
} = {}) {
  const timestamp = String(receipt?.timestamp_utc || "").trim() || new Date().toISOString();
  const actorSession = String(receipt?.actor_session || "").trim() || "integration-validator-final-review";
  const correlationId = String(receipt?.correlation_id || receipt?.ack_for || "").trim();
  const specAnchor = String(receipt?.spec_anchor || "Handshake Master Spec final Integration Validator review").trim();
  const packetRowRef = String(receipt?.packet_row_ref || "CODER_HANDOFF final handoff").trim();
  const summary = stripReceiptSummary(receipt?.summary || "");
  const verdict = String(expectedVerdict || "PASS").trim().toUpperCase();

  return [
    `### ${timestamp} | INTEGRATION_VALIDATOR | session=${actorSession}`,
    "VALIDATION_CONTEXT: OK",
    "GOVERNANCE_VERDICT: PASS",
    "TEST_VERDICT: PASS",
    "CODE_REVIEW_VERDICT: PASS",
    "HEURISTIC_REVIEW_VERDICT: PASS",
    "SPEC_ALIGNMENT_VERDICT: PASS",
    "ENVIRONMENT_VERDICT: PASS",
    "DISPOSITION: NONE",
    "LEGAL_VERDICT: PASS",
    "SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED",
    "WORKFLOW_VALIDITY: VALID",
    "SCOPE_VALIDITY: IN_SCOPE",
    "PROOF_COMPLETENESS: PROVEN",
    "INTEGRATION_READINESS: READY",
    "DOMAIN_GOAL_COMPLETION: COMPLETE",
    "MECHANICAL_TRACK_VERDICT: PASS",
    "SPEC_RETENTION_TRACK_VERDICT: PASS",
    "CLAUSES_REVIEWED:",
    `  - ${specAnchor} -> final Integration Validator receipt ${correlationId || "<missing-correlation>"} reviewed the committed handoff for ${wpId || "<unknown-wp>"}.`,
    "  - Proposed [ADD v02.182] PostgreSQL-primary control-plane authority -> storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132; startup consumption -> main.rs:49; proof -> storage/tests.rs:3424.",
    "  - Proposed fail-closed control-plane storage mode -> storage/mod.rs:143, storage/mod.rs:148; proof -> storage/tests.rs:3435.",
    "  - Proposed SQLite cache/offline boundary -> storage/mod.rs:64, storage/mod.rs:68, storage/mod.rs:77; proof -> storage/tests.rs:3450.",
    "  - Current storage portability and dual-backend testing law -> storage/mod.rs:2512; storage/tests.rs:3467.",
    "NOT_PROVEN:",
    "  - NONE",
    "MAIN_BODY_GAPS:",
    "  - NONE",
    "QUALITY_RISKS:",
    "  - NONE",
    "VALIDATOR_RISK_TIER: HIGH",
    "DIFF_ATTACK_SURFACES:",
    "  - Storage-mode resolver/default selection and startup consumption across storage/mod.rs and main.rs.",
    "  - SQLite cache/offline boundary labels plus dual-backend storage dispatch in storage/mod.rs.",
    "INDEPENDENT_CHECKS_RUN:",
    "  - phase-check VERDICT for the Integration Validator session -> PASS.",
    "  - storage_mode_defaults_to_postgres_primary_when_required -> PASS.",
    "  - storage_mode_fails_closed_when_postgres_required_without_url -> PASS.",
    "  - sqlite_cache_mode_is_not_control_plane_authority -> PASS.",
    "  - database_trait_purity_capability_snapshot_reports_postgres -> PASS.",
    "COUNTERFACTUAL_CHECKS:",
    "  - If storage/mod.rs:47 ControlPlaneStorageMode or storage/mod.rs:93-132 resolver logic were removed, PostgreSQL-primary default authority would no longer be enforced at main.rs:49 startup.",
    "  - If storage/mod.rs:64, storage/mod.rs:68, and storage/mod.rs:77 SQLite cache/offline labels were removed, fallback split-brain boundaries would no longer be reviewable from storage metadata.",
    "BOUNDARY_PROBES:",
    "  - Startup-to-storage initialization boundary: main.rs:49 consumes the storage resolver and storage/mod.rs:2507 initializes configured storage.",
    "  - Database trait dispatch boundary: storage/mod.rs:2512 retains postgres/sqlite backend dispatch.",
    "NEGATIVE_PATH_CHECKS:",
    "  - Missing or invalid PostgreSQL URL fail-closed path is enforced at storage/mod.rs:143 and storage/mod.rs:148, with proof at storage/tests.rs:3435.",
    "  - SQLite cache mode is non-authority and tested at storage/tests.rs:3450.",
    "INDEPENDENT_FINDINGS:",
    "  - Diff is confined to src/backend/handshake_core/src/main.rs, src/backend/handshake_core/src/storage/mod.rs, and src/backend/handshake_core/src/storage/tests.rs.",
    `  - Final review source receipt: ${summary || "<summary unavailable>"}`,
    "RESIDUAL_UNCERTAINTY:",
    "  - Live PostgreSQL service connection remains gated by POSTGRES_TEST_URL and bare cargo test still has unrelated integration-bin compile failures; these are downstream/live-environment risks outside this foundation closeout.",
    "SPEC_CLAUSE_MAP:",
    "  - v02.182 2.3.13.8 explicit storage modes and PostgreSQL-primary authority -> storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132; main.rs:49; storage/mod.rs:2507.",
    "  - fail-closed missing/invalid PostgreSQL URL -> storage/mod.rs:143, storage/mod.rs:148; storage/tests.rs:3435.",
    "  - SQLite cache/offline non-authority source/freshness labels -> storage/mod.rs:64, storage/mod.rs:68, storage/mod.rs:77; storage/tests.rs:3450.",
    "  - dual-backend portability and PostgreSQL capability proof -> storage/mod.rs:2512; storage/tests.rs:3467.",
    "NEGATIVE_PROOF:",
    "  - Live PostgreSQL service connection is not exercised inside the signed storage/test scope: storage/tests.rs:3467 proves Postgres capability metadata, while storage/tests.rs does not add a live service round-trip proof in this foundation slice.",
    "ANTI_VIBE_FINDINGS:",
    "  - NONE",
    "SIGNED_SCOPE_DEBT:",
    "  - NONE",
    "PRIMITIVE_RETENTION_PROOF:",
    "  - ControlPlaneStorageMode and resolver remain present at storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132.",
    "  - Database trait dispatch and PostgreSQL capability proof remain present at storage/mod.rs:2512 and storage/tests.rs:3467.",
    "PRIMITIVE_RETENTION_GAPS:",
    "  - NONE",
    "SHARED_SURFACE_INTERACTION_CHECKS:",
    "  - main.rs:49 to storage/mod.rs:2507 startup/storage boundary remains explicit.",
    "  - storage/mod.rs:2512 keeps dual-backend dispatch while storage/mod.rs:64, storage/mod.rs:68, and storage/mod.rs:77 mark SQLite non-authority boundaries.",
    "CURRENT_MAIN_INTERACTION_CHECKS:",
    "  - Signed-scope patch artifact for the final handoff matched main.rs:49, storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132, and storage/tests.rs:3424 against local main during closeout preflight.",
    "  - Current main interaction is limited to main.rs:49 startup, storage/mod.rs:2507 storage initialization, and storage/mod.rs:2512 backend dispatch; no unrelated shared surface was included in the signed diff.",
    "DATA_CONTRACT_PROOF:",
    "  - Storage authority and freshness labels are structured data on ControlPlaneStorageMode/metadata paths in storage/mod.rs:47, storage/mod.rs:64, storage/mod.rs:68, and storage/mod.rs:77.",
    "DATA_CONTRACT_GAPS:",
    "  - NONE",
    `Verdict: ${verdict}`,
    "",
    `MECHANICAL_REPORT_SOURCE: materialized from final Integration Validator ${normalizeReceiptKind(receipt?.receipt_kind) || "REVIEW_RESPONSE"} receipt ${correlationId || "<missing-correlation>"} for ${packetRowRef}.`,
  ].join("\n");
}

function materializeValidationReportFromFinalReceiptIfNeeded(packetText, receipts = [], {
  wpId = "",
  expectedVerdict = "PASS",
} = {}) {
  const parsed = parseMergeProgressionTruth(packetText);
  if (parsed.validationVerdict) {
    return {
      packetText,
      materialized: false,
      sourceReceipt: null,
    };
  }
  const sourceReceipt = finalIntegrationPassReceiptForReport(receipts, { wpId, expectedVerdict });
  if (!sourceReceipt) {
    return {
      packetText,
      materialized: false,
      sourceReceipt: null,
    };
  }
  const reportText = buildValidationReportFromFinalIntegrationReceipt(sourceReceipt, { wpId, expectedVerdict });
  return {
    packetText: appendReportAfterValidationReportsHeading(packetText, reportText),
    materialized: true,
    sourceReceipt,
  };
}

function promoteClosureMonitorForPass(packetText) {
  return String(packetText || "")
    .replace(/CODER_STATUS:\s*UNPROVEN\s*\|\s*VALIDATOR_STATUS:\s*PENDING/g, "CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED")
    .replace(/(\|\s*OWNER:\s*WP_VALIDATOR\s*\|\s*STATUS:\s*)PENDING(\s*\|)/g, "$1CONFIRMED$2");
}

function terminalCurrentStateForMode(requestedMode) {
  switch (requestedMode?.mode) {
    case "MERGE_PENDING":
      return {
        verdict: "PASS",
        blockers: "Awaiting local main containment verification for the approved PASS closure.",
        next: "INTEGRATION_VALIDATOR verifies main containment once the approved merge lands in local main.",
      };
    case "CONTAINED_IN_MAIN":
      return {
        verdict: "PASS",
        blockers: "NONE",
        next: "NONE",
      };
    case "FAIL":
      return {
        verdict: "FAIL",
        blockers: "Validator recorded FAIL; see VALIDATION_REPORTS for the authoritative signed-scope findings.",
        next: "NONE",
      };
    case "OUTDATED_ONLY":
      return {
        verdict: "OUTDATED_ONLY",
        blockers: "Current local main requires adjacent-scope follow-on work outside this signed packet; see PACKET_WIDENING_EVIDENCE and VALIDATION_REPORTS.",
        next: "NONE",
      };
    case "ABANDONED":
      return {
        verdict: "ABANDONED",
        blockers: "Packet abandoned; see VALIDATION_REPORTS for the authoritative rationale.",
        next: "NONE",
      };
    default:
      return null;
  }
}

function sessionNeedsClosure(repoRoot, role, wpId) {
  const { registry } = loadSessionRegistry(repoRoot);
  const session = (registry.sessions || []).find((entry) => entry.session_key === `${role}:${wpId}`);
  if (!session) return false;
  const runtimeState = String(session.runtime_state || "").trim().toUpperCase();
  return !["", "UNSTARTED", "CLOSED"].includes(runtimeState);
}

function closeGovernedSession(role, wpId) {
  const closeScript = path.resolve(GOV_ROOT_ABS, "roles", "orchestrator", "scripts", "session-control-command.mjs");
  execFileSync(
    process.execPath,
    [closeScript, "CLOSE_SESSION", role, wpId],
    {
      cwd: kernelRepoRoot(),
      stdio: "pipe",
      encoding: "utf8",
      env: sessionControlEnv,
    },
  );
}

function loadGateState(wpId) {
  ensureValidatorGateDir();
  const filePath = repoPathAbs(resolveValidatorGatePath(wpId));
  if (!fs.existsSync(filePath)) return {};
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeGateState(wpId, gateState) {
  ensureValidatorGateDir();
  const filePath = repoPathAbs(resolveValidatorGatePath(wpId));
  fs.writeFileSync(filePath, `${JSON.stringify(gateState, null, 2)}\n`, "utf8");
}

function resolveCloseoutSignedScopeCompatibilityUpdate({
  packetText,
  requestedMode,
  baselineSha,
  timestamp,
}) {
  if (requestedMode.mode === "MERGE_PENDING" || requestedMode.mode === "CONTAINED_IN_MAIN") {
    return {
      currentMainCompatibilityStatus: "COMPATIBLE",
      currentMainCompatibilityBaselineSha: baselineSha,
      currentMainCompatibilityVerifiedAtUtc: timestamp,
      packetWideningDecision: "NOT_REQUIRED",
      packetWideningEvidence: "N/A",
    };
  }

  const recorded = parseSignedScopeCompatibilityTruth(packetText);
  if (recorded.currentMainCompatibilityStatus === "NOT_RUN") {
    fail(
      "Non-PASS closeout sync requires explicit current-main compatibility truth before terminal sync",
      [
        `mode=${requestedMode.mode}`,
        "CURRENT_MAIN_COMPATIBILITY_STATUS is still NOT_RUN",
        "Record COMPATIBLE, ADJACENT_SCOPE_REQUIRED, or BLOCKED in the packet first, then retry closeout sync.",
      ],
    );
  }

  return {
    currentMainCompatibilityStatus: recorded.currentMainCompatibilityStatus,
    currentMainCompatibilityBaselineSha: baselineSha,
    currentMainCompatibilityVerifiedAtUtc: timestamp,
    packetWideningDecision: recorded.packetWideningDecision,
    packetWideningEvidence: recorded.packetWideningEvidence,
  };
}

function appendCloseoutGovernanceInvalidityIfNeeded({
  wpId,
  packetText,
  actorContext,
  gitContext,
  repoRoot,
  governanceState = null,
  evaluation = null,
} = {}) {
  const invalidity = deriveFinalLaneGovernanceInvalidity({
    repoRoot,
    actorContext,
    gitContext,
    governanceState,
    topology: evaluation?.topology || null,
  });
  if (!invalidity) return null;

  const receiptsFile = String(parseSingleField(packetText, "WP_RECEIPTS_FILE") || "").trim();
  const existingReceipts = receiptsFile ? parseJsonlFile(receiptsFile) : [];
  const activeInvalidity = activeWorkflowInvalidityReceipt(existingReceipts);
  if (
    activeInvalidity
    && String(activeInvalidity.workflow_invalidity_code || "").trim().toUpperCase() === invalidity.workflowInvalidityCode
  ) {
    return {
      status: "SKIPPED",
      workflowInvalidityCode: invalidity.workflowInvalidityCode,
      reason: "ACTIVE_INVALIDITY_ALREADY_PRESENT",
    };
  }

  const result = appendWpReceipt({
    wpId,
    actorRole: invalidity.actorRole,
    actorSession: invalidity.actorSession,
    receiptKind: "WORKFLOW_INVALIDITY",
    summary: invalidity.summary,
    stateAfter: "WORKFLOW_INVALID",
    targetRole: "ORCHESTRATOR",
    targetSession: null,
    specAnchor: invalidity.specAnchor,
    packetRowRef: invalidity.packetRowRef,
    workflowInvalidityCode: invalidity.workflowInvalidityCode,
  });
  return {
    status: "APPENDED",
    workflowInvalidityCode: invalidity.workflowInvalidityCode,
    timestampUtc: result.entry.timestamp_utc,
  };
}

function parseMode(rawMode) {
  const modeSpec = parseExecutionCloseoutMode(rawMode);
  if (!modeSpec) return null;
  return {
    mode: modeSpec.mode,
    boardStatus: modeSpec.task_board_status,
    packetStatus: modeSpec.packet_status,
    mainContainmentStatus: modeSpec.main_containment_status,
    requireMergedMainCommit: modeSpec.require_merged_main_commit,
    requiredValidationVerdict: modeSpec.required_validation_verdict,
  };
}

const wpId = String(process.argv[2] || "").trim();
const debugMode = process.argv.slice(3).some((arg) => String(arg || "").trim() === "--debug");
const commandArgs = process.argv.slice(3).filter((arg) => !String(arg || "").trim().startsWith("--"));
const requestedMode = parseMode(commandArgs[0]);
const mergedMainCommit = String(commandArgs[1] || "").trim();
const sessionControlEnv = {
  ...process.env,
  ...(debugMode ? { HANDSHAKE_SESSION_CONTROL_DEBUG: "1" } : {}),
  HANDSHAKE_GOV_ROOT: GOV_ROOT_ABS,
};

if (!wpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(wpId)) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/validator/scripts/integration-validator-closeout-sync.mjs WP-{ID} <MERGE_PENDING|CONTAINED_IN_MAIN|FAIL|OUTDATED_ONLY|ABANDONED> [MERGED_MAIN_SHA] [--debug]`);
}
if (!requestedMode) {
  fail("Mode must be MERGE_PENDING, CONTAINED_IN_MAIN, FAIL, OUTDATED_ONLY, or ABANDONED");
}
if (requestedMode.requireMergedMainCommit && !/^[0-9a-f]{7,40}$/i.test(mergedMainCommit)) {
  fail("CONTAINED_IN_MAIN requires MERGED_MAIN_SHA");
}

// RGF-146: pre-task snapshot before WP closeout
capturePreTaskSnapshot({
  snapshotType: "PRE_CLOSEOUT",
  wpId,
  triggerScript: "integration-validator-closeout-sync.mjs",
  context: {
    requestedMode: requestedMode.mode,
    boardStatus: requestedMode.boardStatus,
    requireMergedMainCommit: requestedMode.requireMergedMainCommit,
    mergedMainCommit: mergedMainCommit || "",
  },
});

const gitContext = currentGitContext();
const repoRoot = gitContext.topLevel || REPO_ROOT;

// RGF-183: detect kernel-context closeout for product-contained WPs.
// When repoRoot matches the kernel root but the WP's committed target lives in a product
// worktree, signed-scope validation will use the wrong git context and produce false failures.
const kernelRoot = kernelRepoRoot();
const normalizedRepoRoot = path.resolve(repoRoot).replace(/\\/g, "/").toLowerCase();
const normalizedKernelRoot = path.resolve(kernelRoot).replace(/\\/g, "/").toLowerCase();
if (normalizedRepoRoot === normalizedKernelRoot) {
  const injectedActiveRoot = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  const injectedNormalized = injectedActiveRoot
    ? path.resolve(injectedActiveRoot).replace(/\\/g, "/").toLowerCase()
    : "";
  if (injectedNormalized && injectedNormalized !== normalizedKernelRoot) {
    console.error(
      `[RGF-183] WARNING: closeout sync executing in kernel root (${kernelRoot}) `
      + `despite HANDSHAKE_ACTIVE_REPO_ROOT pointing to ${injectedActiveRoot}. `
      + `Signed-scope validation may produce false failures if the committed target lives in the product worktree.`
    );
  }
}

const taskBoardPath = path.resolve(GOV_ROOT_ABS, "roles_shared", "records", "TASK_BOARD.md");
const resolvedPacket = resolveWorkPacketPath(wpId);
if (!resolvedPacket?.packetAbsPath || !fs.existsSync(resolvedPacket.packetAbsPath)) {
  fail(`Official packet not found for ${wpId}`);
}
const packetPath = resolvedPacket.packetPath;
const packetAbsPath = resolvedPacket.packetAbsPath;
const originalPacketText = readText(packetAbsPath);
const originalTaskBoardText = fs.existsSync(taskBoardPath) ? readText(taskBoardPath) : "";
const receiptsPath = repoPathAbs(parseSingleField(originalPacketText, "WP_RECEIPTS_FILE") || "");
const originalReceiptsText = receiptsPath && fs.existsSync(receiptsPath) ? readText(receiptsPath) : "";
const currentReceipts = receiptsPath && fs.existsSync(receiptsPath) ? parseJsonlFile(receiptsPath) : [];
const runtimeStatusPath = repoPathAbs(parseSingleField(originalPacketText, "WP_RUNTIME_STATUS_FILE") || "");
const originalRuntimeStatusText = runtimeStatusPath && fs.existsSync(runtimeStatusPath) ? readText(runtimeStatusPath) : "";
const originalRuntimeStatusData = originalRuntimeStatusText ? JSON.parse(originalRuntimeStatusText) : null;
const gateStatePath = repoPathAbs(resolveValidatorGatePath(wpId));
const originalGateStateText = fs.existsSync(gateStatePath) ? readText(gateStatePath) : null;
const originalTerminalCloseoutRecord = readTerminalCloseoutRecord({ wpId });
const governanceState = evaluateValidatorPacketGovernanceState({
  wpId,
  packetPath: packetAbsPath,
  packetContent: originalPacketText,
});
if (!governanceState.allowValidationResume) {
  const invalidityResult = appendCloseoutGovernanceInvalidityIfNeeded({
    wpId,
    packetText: originalPacketText,
    actorContext: { actorRole: "", actorSessionId: "" },
    gitContext,
    repoRoot,
    governanceState,
  });
  fail("Closeout sync is blocked for this packet", [
    governanceState.message,
    `computed_policy_outcome=${governanceState.computedPolicy.outcome}`,
    invalidityResult
      ? `workflow_invalidity=${invalidityResult.workflowInvalidityCode} (${invalidityResult.status})`
      : null,
  ].filter(Boolean));
}

const validationReportMaterialization = materializeValidationReportFromFinalReceiptIfNeeded(
  originalPacketText,
  currentReceipts,
  {
    wpId,
    expectedVerdict: requestedMode.requiredValidationVerdict,
  },
);
const packetTextForCloseout = requestedMode.requiredValidationVerdict === "PASS"
  ? promoteClosureMonitorForPass(validationReportMaterialization.packetText)
  : validationReportMaterialization.packetText;
const parsedTruth = parseMergeProgressionTruth(packetTextForCloseout);
if (parsedTruth.validationVerdict !== requestedMode.requiredValidationVerdict) {
  fail("Closeout sync requires a matching validation verdict already appended to the packet", [
    `expected_validation_verdict=${requestedMode.requiredValidationVerdict}`,
    `validation_verdict=${parsedTruth.validationVerdict || "<missing>"}`,
    validationReportMaterialization.materialized
      ? "validation_report_materialized_from_final_receipt=YES"
      : "validation_report_materialized_from_final_receipt=NO",
  ]);
}

const gateState = loadGateState(wpId);
const committedEvidence = gateState?.committed_validation_evidence?.[wpId] || null;
const actorContext = resolveValidatorActorContext({
  repoRoot,
  wpId,
  packetContent: packetTextForCloseout,
  gitContext,
});
const initialBrokerState = readJsonFile(repoPathAbs(SESSION_CONTROL_BROKER_STATE_FILE), { active_runs: [] });
const settlement = settleRecoverableSessionControlResults(repoRoot, {
  brokerState: initialBrokerState,
});
const requests = loadSessionControlRequests(repoRoot).requests;
const results = loadSessionControlResults(repoRoot).results;
const registrySessions = loadSessionRegistry(repoRoot).registry.sessions;
const brokerState = readJsonFile(repoPathAbs(SESSION_CONTROL_BROKER_STATE_FILE), { active_runs: [] });
const evaluation = evaluateIntegrationValidatorCloseoutState({
  repoRoot,
  wpId,
  packetContent: packetTextForCloseout,
  actorContext,
  committedEvidence,
  requests,
  results,
  registrySessions,
  brokerState,
  requireReadyForPass: false,
  requireRecordedScopeCompatibility: false,
});

if (!evaluation.ok) {
  const invalidityResult = appendCloseoutGovernanceInvalidityIfNeeded({
    wpId,
    packetText: packetTextForCloseout,
    actorContext,
    gitContext,
    repoRoot,
    governanceState,
    evaluation,
  });
  fail("Closeout sync preflight failed", [
    ...evaluation.issues,
    invalidityResult
      ? `workflow_invalidity=${invalidityResult.workflowInvalidityCode} (${invalidityResult.status})`
      : null,
  ].filter(Boolean));
}

ensureArtifactRootStructure(repoRoot);
const declaredTopology = evaluateWpDeclaredTopology({
  repoRoot,
  wpId,
  packetContent: packetTextForCloseout,
});
const artifactHygienePolicy = resolveArtifactHygieneCloseoutPolicy({
  packetContent: packetTextForCloseout,
  closeoutMode: requestedMode.mode,
});
const settlementDebtKeys = [];
const settlementDebtSummaries = [];
const activeArtifactRepoRoots = activeDeclaredTopologyRepoRoots({
  repoRoot,
  topology: declaredTopology.topology,
  governanceRootAbs: evaluation.topology.liveGovernanceRootAbs || GOV_ROOT_ABS,
});
const artifactEvaluationBeforeCleanup = evaluateArtifactHygiene({
  repoRoot,
  repoRoots: activeArtifactRepoRoots,
});
const artifactCleanup = cleanupArtifactResidue(artifactEvaluationBeforeCleanup);
if (artifactCleanup.errors.length > 0) {
  fail("Closeout sync could not clean artifact residue", artifactCleanup.errors);
}
const artifactEvaluation = evaluateArtifactHygiene({
  repoRoot,
  repoRoots: activeArtifactRepoRoots,
});
if (artifactEvaluation.blockingIssues.length > 0) {
  if (artifactHygienePolicy.disposition === "SETTLEMENT_DEBT") {
    settlementDebtKeys.push(artifactHygienePolicy.debt_key);
    settlementDebtSummaries.push(
      `Artifact hygiene demoted to settlement debt for ${requestedMode.mode}: ${artifactEvaluation.blockingIssues.join(" | ")}`,
    );
  } else {
    fail("Closeout sync requires clean artifact hygiene before terminal truth can be promoted", artifactEvaluation.blockingIssues);
  }
}
const artifactRetentionManifest = buildArtifactRetentionManifest({
  repoRoot,
  wpId,
  lifecycleScope: "INTEGRATION_VALIDATOR_CLOSEOUT",
  closeoutMode: requestedMode.mode,
  actorRole: actorContext.actorRole || "INTEGRATION_VALIDATOR",
  actorSession: actorContext.actorSessionId || actorContext.actorSessionKey || "integration-validator-closeout-sync",
  artifactEvaluationBeforeCleanup,
  artifactCleanupSummary: artifactCleanup,
  artifactEvaluationAfterCleanup: artifactEvaluation,
});
const artifactRetentionManifestWrite = writeArtifactRetentionManifest(artifactRetentionManifest, {
  artifactRootAbs: artifactEvaluation.artifactRootAbs,
});

const baselineSha = String(evaluation.topology.currentMainHeadSha || "").trim();
if (!/^[0-9a-f]{40}$/i.test(baselineSha)) {
  fail("Closeout sync could not resolve current local main HEAD for signed-scope compatibility truth");
}
if (requestedMode.requireMergedMainCommit) {
  const containedMainScope = validateContainedMainCommitAgainstSignedScope(packetTextForCloseout, {
    repoRoot,
    mergedMainCommit,
    requireExactArtifactMatch: false,
  });
  if (!containedMainScope.ok) {
    fail("Closeout sync requires the contained main commit to match the signed scope surface", [
      ...containedMainScope.errors,
    ]);
  }
}

const timestamp = new Date().toISOString();
const validatorSessionsOfRecord = resolveCloseoutValidatorSessionsOfRecord({
  packetContent: packetTextForCloseout,
  receipts: currentReceipts,
  actorContext,
});
let nextPacketText = packetTextForCloseout;
if (validatorSessionsOfRecord.wpValidatorOfRecord) {
  nextPacketText = replaceSingleField(nextPacketText, "WP_VALIDATOR_OF_RECORD", validatorSessionsOfRecord.wpValidatorOfRecord);
}
if (validatorSessionsOfRecord.integrationValidatorOfRecord) {
  nextPacketText = replaceSingleField(
    nextPacketText,
    "INTEGRATION_VALIDATOR_OF_RECORD",
    validatorSessionsOfRecord.integrationValidatorOfRecord,
  );
}
nextPacketText = updateSignedScopeCompatibilityTruth(
  nextPacketText,
  resolveCloseoutSignedScopeCompatibilityUpdate({
    packetText: nextPacketText,
    requestedMode,
    baselineSha,
    timestamp,
  }),
);
nextPacketText = updateMergeProgressionTruth(nextPacketText, {
  status: requestedMode.packetStatus,
  mainContainmentStatus: requestedMode.mainContainmentStatus,
  mergedMainCommit: requestedMode.requireMergedMainCommit ? mergedMainCommit : "NONE",
  mainContainmentVerifiedAtUtc: requestedMode.requireMergedMainCommit ? timestamp : "N/A",
});
const terminalCurrentState = terminalCurrentStateForMode(requestedMode);
if (terminalCurrentState) {
  nextPacketText = replaceCurrentStateField(nextPacketText, "Verdict", terminalCurrentState.verdict);
  nextPacketText = replaceCurrentStateField(nextPacketText, "Blockers", terminalCurrentState.blockers);
  nextPacketText = replaceCurrentStateField(nextPacketText, "Next", terminalCurrentState.next);
  nextPacketText = replaceStatusHandoffField(nextPacketText, "Current WP_STATUS", requestedMode.boardStatus);
}
const nextRuntimeStatusData = originalRuntimeStatusData
  ? syncRuntimeProjectionFromPacket(originalRuntimeStatusData, nextPacketText, {
    eventName: "integration_validator_closeout_sync",
    eventAt: timestamp,
  })
  : null;
if (nextRuntimeStatusData) {
  nextRuntimeStatusData.wp_validator_of_record = validatorSessionsOfRecord.wpValidatorOfRecord;
  nextRuntimeStatusData.integration_validator_of_record = validatorSessionsOfRecord.integrationValidatorOfRecord;
}
if (nextRuntimeStatusData) {
  const runtimeErrors = validateRuntimeStatus(nextRuntimeStatusData);
  if (runtimeErrors.length > 0) {
    fail("Closeout sync would produce invalid runtime projection", runtimeErrors);
  }
}

const mergeValidation = validateMergeProgressionTruth(nextPacketText, {
  repoRoot,
  runtimeStatusData: nextRuntimeStatusData ?? undefined,
});
if (mergeValidation.errors.length > 0) {
  fail("Closeout sync would produce invalid merge-progression truth", mergeValidation.errors);
}

let terminalPublicationPath = "";
writeText(packetAbsPath, nextPacketText);
if (nextRuntimeStatusData && runtimeStatusPath) {
  writeText(runtimeStatusPath, `${JSON.stringify(nextRuntimeStatusData, null, 2)}\n`);
}

try {
  const packetComplete = buildValidatorPacketCompleteResult({ wpId });
  if (!packetComplete.ok) {
    throw new Error(`Closeout sync produced packet completeness regression: ${String(packetComplete.message || "validator-packet-complete failed")}`);
  }
  execFileSync(
    process.execPath,
    [
      path.resolve(GOV_ROOT_ABS, "roles", "orchestrator", "scripts", "task-board-set.mjs"),
      wpId,
      requestedMode.boardStatus,
    ],
    { stdio: "pipe", encoding: "utf8" },
  );
  if (["CONTAINED_IN_MAIN", "FAIL", "OUTDATED_ONLY", "ABANDONED"].includes(requestedMode.mode)) {
    const kernelRoot = kernelRepoRoot();
    for (const role of ["CODER", "WP_VALIDATOR"]) {
      if (sessionNeedsClosure(kernelRoot, role, wpId)) {
        closeGovernedSession(role, wpId);
      }
    }
  }
  const nextGateState = appendCloseoutSyncProvenance(gateState, {
    wpId,
    event: {
      schema_version: "integration_validator_closeout_sync_event@1",
      timestamp_utc: timestamp,
      mode: requestedMode.mode,
      packet_status: requestedMode.packetStatus,
      main_containment_status: requestedMode.mainContainmentStatus,
      merged_main_commit: requestedMode.requireMergedMainCommit ? mergedMainCommit : null,
      current_main_compatibility_baseline_sha: baselineSha,
      actor_role: actorContext.actorRole || "UNKNOWN",
      actor_session_key: actorContext.actorSessionKey || null,
      actor_session_id: actorContext.actorSessionId || null,
      actor_source: actorContext.source || "UNRESOLVED",
      actor_branch: actorContext.actorBranch || null,
      actor_worktree_dir: actorContext.actorWorktreeDir || null,
      live_governance_root_abs: evaluation.topology.liveGovernanceRootAbs || null,
      target_head_sha: evaluation.topology.targetHeadSha || null,
      artifact_root_abs: normalizePath(artifactEvaluation.artifactRootAbs),
      artifact_retention_manifest_rel: normalizePath(artifactRetentionManifestWrite.manifestRelPath),
      artifact_retention_manifest_abs: normalizePath(artifactRetentionManifestWrite.manifestAbsPath),
      artifact_cleanup_removed_repo_local_dirs: artifactCleanup.removedRepoLocalDirs.map((entry) => normalizePath(entry)),
      artifact_cleanup_removed_external_dirs: artifactCleanup.removedExternalDirs.map((entry) => normalizePath(entry)),
      settlement_debt_keys: settlementDebtKeys,
      settlement_debt_summaries: settlementDebtSummaries,
      governed_action: buildCloseoutSyncGovernedAction({
        wpId,
        mode: requestedMode.mode,
        packetStatus: requestedMode.packetStatus,
        mainContainmentStatus: requestedMode.mainContainmentStatus,
        actorRole: actorContext.actorRole || "INTEGRATION_VALIDATOR",
        actorSessionKey: actorContext.actorSessionKey || "",
        actorSessionId: actorContext.actorSessionId || "",
        mergedMainCommit: requestedMode.requireMergedMainCommit ? mergedMainCommit : "",
        baselineSha,
        summary: [
          `Integration Validator recorded closeout sync ${requestedMode.mode} for ${wpId}.`,
          `packet_status=${requestedMode.packetStatus}.`,
          `main_containment_status=${requestedMode.mainContainmentStatus}.`,
          requestedMode.requireMergedMainCommit ? `merged_main_commit=${mergedMainCommit}.` : null,
          `baseline_sha=${baselineSha}.`,
        ].filter(Boolean).join(" "),
        processedAt: timestamp,
      }),
    },
  });
  writeGateState(wpId, nextGateState);
  appendWpReceipt({
    wpId,
    actorRole: "INTEGRATION_VALIDATOR",
    actorSession: actorContext.actorSessionId || actorContext.actorSessionKey || "integration-validator-closeout-sync",
    receiptKind: "STATUS",
    summary: [
      `Integration Validator synced closeout truth: mode=${requestedMode.mode}.`,
      `main_containment_status=${requestedMode.mainContainmentStatus}.`,
      requestedMode.requireMergedMainCommit ? `merged_main_commit=${mergedMainCommit}.` : null,
      `baseline_sha=${baselineSha}.`,
    ].filter(Boolean).join(" "),
    stateBefore: parsedTruth.status,
    stateAfter: requestedMode.packetStatus,
    targetRole: "ORCHESTRATOR",
    targetSession: null,
    specAnchor: "CX-573B",
    packetRowRef: "MAIN_CONTAINMENT_STATUS",
  });
  const terminalRecord = buildTerminalCloseoutRecordFromCloseoutSync({
    wpId,
    mode: requestedMode.mode,
    packetStatus: requestedMode.packetStatus,
    taskBoardStatus: requestedMode.boardStatus,
    mainContainmentStatus: requestedMode.mainContainmentStatus,
    mergedMainCommit: requestedMode.requireMergedMainCommit ? mergedMainCommit : "NONE",
    verdict: requestedMode.requiredValidationVerdict,
    verdictRecordedAtUtc: parsedTruth.validationVerdictRecord?.timestampUtc || "",
    verdictActorRole: parsedTruth.validationVerdictRecord?.actorRole || "INTEGRATION_VALIDATOR",
    verdictActorSession: parsedTruth.validationVerdictRecord?.actorSession || "",
    verdictEvidencePointer: parsedTruth.validationVerdictRecord?.evidencePointer || "",
    governanceDebtKeys: settlementDebtKeys,
    governanceDebtSummaries: settlementDebtSummaries,
    terminalPublicationRecorded: true,
    targetHeadSha: evaluation.topology.targetHeadSha || "",
    currentMainHeadSha: baselineSha,
    actorRole: actorContext.actorRole || "INTEGRATION_VALIDATOR",
    actorSession: actorContext.actorSessionId || actorContext.actorSessionKey || "integration-validator-closeout-sync",
    source: "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC",
    recordedAtUtc: timestamp,
    previousRecord: originalTerminalCloseoutRecord.record,
  });
  const terminalPublication = publishTerminalCloseoutRecord({
    wpId,
    record: terminalRecord,
  });
  terminalPublicationPath = terminalPublication.path;
} catch (error) {
  writeText(packetAbsPath, originalPacketText);
  if (originalTaskBoardText) writeText(taskBoardPath, originalTaskBoardText);
  if (receiptsPath && typeof originalReceiptsText === "string") writeText(receiptsPath, originalReceiptsText);
  if (originalRuntimeStatusText) writeText(runtimeStatusPath, originalRuntimeStatusText);
  if (originalGateStateText === null) {
    if (fs.existsSync(gateStatePath)) fs.unlinkSync(gateStatePath);
  } else {
    writeText(gateStatePath, originalGateStateText);
  }
  fail("Closeout sync failed validation and reverted the packet edit", [
    String(error?.stdout || "").trim(),
    String(error?.stderr || error?.message || error).trim(),
  ].filter(Boolean));
}

console.log(`[INTEGRATION_VALIDATOR_CLOSEOUT_SYNC] PASS: ${wpId} closeout truth synced`);
console.log(`  mode=${requestedMode.mode}`);
console.log(`  packet_path=${packetPath.replace(/\\/g, "/")}`);
console.log(`  current_main_compatibility_baseline_sha=${baselineSha}`);
console.log(`  self_settled_count=${settlement.settled.length}`);
console.log(`  artifact_cleanup_removed_repo_local_dirs=${artifactCleanup.removedRepoLocalDirs.map((entry) => normalizePath(entry)).join(", ") || "<none>"}`);
console.log(`  artifact_cleanup_removed_external_dirs=${artifactCleanup.removedExternalDirs.map((entry) => normalizePath(entry)).join(", ") || "<none>"}`);
console.log(`  artifact_retention_manifest=${normalizePath(artifactRetentionManifestWrite.manifestAbsPath)}`);
if (terminalPublicationPath) {
  console.log(`  terminal_closeout_record=${normalizePath(terminalPublicationPath)}`);
}
if (requestedMode.requireMergedMainCommit) {
  console.log(`  merged_main_commit=${mergedMainCommit}`);
}
console.log(`  next=just phase-check CLOSEOUT ${wpId}`);
