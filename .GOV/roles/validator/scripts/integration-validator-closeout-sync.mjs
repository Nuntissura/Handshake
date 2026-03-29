#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { currentGitContext } from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { resolveWorkPacketPath, GOV_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  parseMergeProgressionTruth,
  updateMergeProgressionTruth,
  validateMergeProgressionTruth,
} from "../../../roles_shared/scripts/lib/merge-progression-truth-lib.mjs";
import {
  updateSignedScopeCompatibilityTruth,
} from "../../../roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs";
import {
  evaluateValidatorPacketGovernanceState,
  resolveValidatorActorContext,
} from "../scripts/lib/validator-governance-lib.mjs";
import { evaluateIntegrationValidatorCloseoutState } from "../scripts/lib/integration-validator-closeout-lib.mjs";
import { ensureValidatorGateDir, resolveValidatorGatePath } from "../../../roles_shared/scripts/lib/validator-gate-paths.mjs";

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

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function loadGateState(wpId) {
  ensureValidatorGateDir();
  const filePath = resolveValidatorGatePath(wpId);
  if (!fs.existsSync(filePath)) return {};
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
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
  return null;
}

const wpId = String(process.argv[2] || "").trim();
const requestedMode = parseMode(process.argv[3]);
const mergedMainCommit = String(process.argv[4] || "").trim();

if (!wpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(wpId)) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/validator/scripts/integration-validator-closeout-sync.mjs WP-{ID} <MERGE_PENDING|CONTAINED_IN_MAIN|FAIL|OUTDATED_ONLY> [MERGED_MAIN_SHA]`);
}
if (!requestedMode) {
  fail("Mode must be MERGE_PENDING, CONTAINED_IN_MAIN, FAIL, or OUTDATED_ONLY");
}
if (requestedMode.requireMergedMainCommit && !/^[0-9a-f]{7,40}$/i.test(mergedMainCommit)) {
  fail("CONTAINED_IN_MAIN requires MERGED_MAIN_SHA");
}

const repoRoot = process.cwd();
const taskBoardPath = path.resolve(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "records", "TASK_BOARD.md");
const resolvedPacket = resolveWorkPacketPath(wpId);
if (!resolvedPacket?.packetPath || !fs.existsSync(resolvedPacket.packetPath)) {
  fail(`Official packet not found for ${wpId}`);
}
const packetPath = resolvedPacket.packetPath;
const originalPacketText = readText(packetPath);
const originalTaskBoardText = fs.existsSync(taskBoardPath) ? readText(taskBoardPath) : "";
const runtimeStatusPath = path.resolve(repoRoot, parseSingleField(originalPacketText, "WP_RUNTIME_STATUS_FILE") || "");
const originalRuntimeStatusText = runtimeStatusPath && fs.existsSync(runtimeStatusPath) ? readText(runtimeStatusPath) : "";
const governanceState = evaluateValidatorPacketGovernanceState({
  wpId,
  packetPath,
  packetContent: originalPacketText,
});
if (!governanceState.allowValidationResume) {
  fail("Closeout sync is blocked for this packet", [
    governanceState.message,
    `computed_policy_outcome=${governanceState.computedPolicy.outcome}`,
  ]);
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
  gitContext: currentGitContext(),
});
const evaluation = evaluateIntegrationValidatorCloseoutState({
  repoRoot,
  wpId,
  packetContent: originalPacketText,
  actorContext,
  committedEvidence,
  requireReadyForPass: false,
});

if (!evaluation.topology?.ok || !evaluation.closeoutBundle?.ok) {
  fail("Closeout sync preflight failed", [
    ...evaluation.issues,
  ]);
}

const baselineSha = String(evaluation.topology.currentMainHeadSha || "").trim();
if (!/^[0-9a-f]{40}$/i.test(baselineSha)) {
  fail("Closeout sync could not resolve current local main HEAD for signed-scope compatibility truth");
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

const mergeValidation = validateMergeProgressionTruth(nextPacketText, { repoRoot });
if (mergeValidation.errors.length > 0) {
  fail("Closeout sync would produce invalid merge-progression truth", mergeValidation.errors);
}

writeText(packetPath, nextPacketText);

try {
  execFileSync(
    process.execPath,
    [
      path.join(GOV_ROOT_REPO_REL, "roles", "validator", "checks", "validator-packet-complete.mjs"),
      wpId,
    ],
    { stdio: "pipe", encoding: "utf8" },
  );
  execFileSync(
    process.execPath,
    [
      path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "task-board-set.mjs"),
      wpId,
      requestedMode.boardStatus,
    ],
    { stdio: "pipe", encoding: "utf8" },
  );
} catch (error) {
  writeText(packetPath, originalPacketText);
  if (originalTaskBoardText) writeText(taskBoardPath, originalTaskBoardText);
  if (originalRuntimeStatusText) writeText(runtimeStatusPath, originalRuntimeStatusText);
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
