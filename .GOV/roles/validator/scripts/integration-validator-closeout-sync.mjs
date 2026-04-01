#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { currentGitContext } from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { resolveWorkPacketPath, GOV_ROOT_ABS, GOV_ROOT_REPO_REL, REPO_ROOT, repoPathAbs } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  parseMergeProgressionTruth,
  updateMergeProgressionTruth,
  validateMergeProgressionTruth,
} from "../../../roles_shared/scripts/lib/merge-progression-truth-lib.mjs";
import {
  updateSignedScopeCompatibilityTruth,
} from "../../../roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs";
import {
  validateContainedMainCommitAgainstSignedScope,
} from "../../../roles_shared/scripts/lib/signed-scope-surface-lib.mjs";
import { syncRuntimeProjectionFromPacket } from "../../../roles_shared/scripts/lib/packet-runtime-projection-lib.mjs";
import {
  activeWorkflowInvalidityReceipt,
  parseJsonlFile,
  validateRuntimeStatus,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import {
  evaluateValidatorPacketGovernanceState,
  resolveValidatorActorContext,
} from "../scripts/lib/validator-governance-lib.mjs";
import {
  appendCloseoutSyncProvenance,
  deriveFinalLaneGovernanceInvalidity,
  evaluateIntegrationValidatorCloseoutState,
} from "../scripts/lib/integration-validator-closeout-lib.mjs";
import { ensureValidatorGateDir, resolveValidatorGatePath } from "../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { appendWpReceipt } from "../../../roles_shared/scripts/wp/wp-receipt-append.mjs";

function fail(message, details = []) {
  console.error(`[INTEGRATION_VALIDATOR_CLOSEOUT_SYNC] ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(1);
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
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: GOV_ROOT_ABS,
      },
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
  const mode = String(rawMode || "").trim().toUpperCase();
  if (mode === "MERGE_PENDING" || mode === "DONE_MERGE_PENDING") {
    return {
      mode: "MERGE_PENDING",
      boardStatus: "DONE_MERGE_PENDING",
      packetStatus: "Done",
      mainContainmentStatus: "MERGE_PENDING",
      requireMergedMainCommit: false,
      requiredValidationVerdict: "PASS",
    };
  }
  if (mode === "CONTAINED_IN_MAIN" || mode === "DONE_VALIDATED") {
    return {
      mode: "CONTAINED_IN_MAIN",
      boardStatus: "DONE_VALIDATED",
      packetStatus: "Validated (PASS)",
      mainContainmentStatus: "CONTAINED_IN_MAIN",
      requireMergedMainCommit: true,
      requiredValidationVerdict: "PASS",
    };
  }
  if (mode === "FAIL" || mode === "DONE_FAIL") {
    return {
      mode: "FAIL",
      boardStatus: "DONE_FAIL",
      packetStatus: "Validated (FAIL)",
      mainContainmentStatus: "NOT_REQUIRED",
      requireMergedMainCommit: false,
      requiredValidationVerdict: "FAIL",
    };
  }
  if (mode === "OUTDATED_ONLY" || mode === "DONE_OUTDATED_ONLY") {
    return {
      mode: "OUTDATED_ONLY",
      boardStatus: "DONE_OUTDATED_ONLY",
      packetStatus: "Validated (OUTDATED_ONLY)",
      mainContainmentStatus: "NOT_REQUIRED",
      requireMergedMainCommit: false,
      requiredValidationVerdict: "OUTDATED_ONLY",
    };
  }
  if (mode === "ABANDONED" || mode === "DONE_ABANDONED") {
    return {
      mode: "ABANDONED",
      boardStatus: "DONE_ABANDONED",
      packetStatus: "Validated (ABANDONED)",
      mainContainmentStatus: "NOT_REQUIRED",
      requireMergedMainCommit: false,
      requiredValidationVerdict: "ABANDONED",
    };
  }
  return null;
}

const wpId = String(process.argv[2] || "").trim();
const requestedMode = parseMode(process.argv[3]);
const mergedMainCommit = String(process.argv[4] || "").trim();

if (!wpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(wpId)) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/validator/scripts/integration-validator-closeout-sync.mjs WP-{ID} <MERGE_PENDING|CONTAINED_IN_MAIN|FAIL|OUTDATED_ONLY|ABANDONED> [MERGED_MAIN_SHA]`);
}
if (!requestedMode) {
  fail("Mode must be MERGE_PENDING, CONTAINED_IN_MAIN, FAIL, OUTDATED_ONLY, or ABANDONED");
}
if (requestedMode.requireMergedMainCommit && !/^[0-9a-f]{7,40}$/i.test(mergedMainCommit)) {
  fail("CONTAINED_IN_MAIN requires MERGED_MAIN_SHA");
}

const gitContext = currentGitContext();
const repoRoot = gitContext.topLevel || REPO_ROOT;
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
const runtimeStatusPath = repoPathAbs(parseSingleField(originalPacketText, "WP_RUNTIME_STATUS_FILE") || "");
const originalRuntimeStatusText = runtimeStatusPath && fs.existsSync(runtimeStatusPath) ? readText(runtimeStatusPath) : "";
const originalRuntimeStatusData = originalRuntimeStatusText ? JSON.parse(originalRuntimeStatusText) : null;
const gateStatePath = repoPathAbs(resolveValidatorGatePath(wpId));
const originalGateStateText = fs.existsSync(gateStatePath) ? readText(gateStatePath) : null;
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

const parsedTruth = parseMergeProgressionTruth(originalPacketText);
if (parsedTruth.validationVerdict !== requestedMode.requiredValidationVerdict) {
  fail("Closeout sync requires a matching validation verdict already appended to the packet", [
    `expected_validation_verdict=${requestedMode.requiredValidationVerdict}`,
    `validation_verdict=${parsedTruth.validationVerdict || "<missing>"}`,
  ]);
}

const gateState = loadGateState(wpId);
const committedEvidence = gateState?.committed_validation_evidence?.[wpId] || null;
const actorContext = resolveValidatorActorContext({
  repoRoot,
  wpId,
  packetContent: originalPacketText,
  gitContext,
});
const evaluation = evaluateIntegrationValidatorCloseoutState({
  repoRoot,
  wpId,
  packetContent: originalPacketText,
  actorContext,
  committedEvidence,
  requireReadyForPass: false,
  requireRecordedScopeCompatibility: false,
});

if (!evaluation.ok) {
  const invalidityResult = appendCloseoutGovernanceInvalidityIfNeeded({
    wpId,
    packetText: originalPacketText,
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

const baselineSha = String(evaluation.topology.currentMainHeadSha || "").trim();
if (!/^[0-9a-f]{40}$/i.test(baselineSha)) {
  fail("Closeout sync could not resolve current local main HEAD for signed-scope compatibility truth");
}
if (requestedMode.requireMergedMainCommit) {
  const containedMainScope = validateContainedMainCommitAgainstSignedScope(originalPacketText, {
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
let nextPacketText = updateSignedScopeCompatibilityTruth(originalPacketText, {
  currentMainCompatibilityStatus: "COMPATIBLE",
  currentMainCompatibilityBaselineSha: baselineSha,
  currentMainCompatibilityVerifiedAtUtc: timestamp,
  packetWideningDecision: "NOT_REQUIRED",
  packetWideningEvidence: "N/A",
});
nextPacketText = updateMergeProgressionTruth(nextPacketText, {
  status: requestedMode.packetStatus,
  mainContainmentStatus: requestedMode.mainContainmentStatus,
  mergedMainCommit: requestedMode.requireMergedMainCommit ? mergedMainCommit : "NONE",
  mainContainmentVerifiedAtUtc: requestedMode.requireMergedMainCommit ? timestamp : "N/A",
});
const nextRuntimeStatusData = originalRuntimeStatusData
  ? syncRuntimeProjectionFromPacket(originalRuntimeStatusData, nextPacketText, {
    eventName: "integration_validator_closeout_sync",
    eventAt: timestamp,
  })
  : null;
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

writeText(packetAbsPath, nextPacketText);
if (nextRuntimeStatusData && runtimeStatusPath) {
  writeText(runtimeStatusPath, `${JSON.stringify(nextRuntimeStatusData, null, 2)}\n`);
}

try {
  execFileSync(
    process.execPath,
    [
      path.resolve(GOV_ROOT_ABS, "roles", "validator", "checks", "validator-packet-complete.mjs"),
      wpId,
    ],
    { stdio: "pipe", encoding: "utf8" },
  );
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
if (requestedMode.requireMergedMainCommit) {
  console.log(`  merged_main_commit=${mergedMainCommit}`);
}
console.log(`  next=just integration-validator-closeout-check ${wpId}`);
