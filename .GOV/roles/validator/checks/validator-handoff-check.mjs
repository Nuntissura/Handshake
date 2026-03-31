#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  currentGitContext,
  loadOrchestratorGateLogs,
  lastGateLog,
  loadPacket,
  packetPath,
  packetExists,
  parseClaimField,
  parseMergeBaseSha,
  preparedWorktreeSyncState,
  resolvePrepareWorktreeAbs,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import {
  ensureValidatorGateDir,
  resolveValidatorGatePath,
} from "../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import { GOV_ROOT_REPO_REL, REPO_ROOT, resolveWorkPacketPath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { formatBoundedItemList } from "../../../roles_shared/scripts/lib/scope-surface-lib.mjs";
import { evaluateValidatorPacketGovernanceState } from "../scripts/lib/validator-governance-lib.mjs";
import {
  committedEvidenceForCloseout,
  livePrepareWorktreeHealthEvidence,
  recordCommittedValidationRun,
} from "../scripts/lib/committed-validation-evidence-lib.mjs";

function usage() {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles/validator/checks/validator-handoff-check.mjs WP-{ID} [--rev <git-rev> | --range <base>..<head>]`);
  process.exit(1);
}

function fail(kind, message, details = [], exitCode = 1) {
  console.error(`[VALIDATOR_HANDOFF_CHECK] ${kind}: ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(exitCode);
}

function contextMismatch(message, details = []) {
  fail("CONTEXT_MISMATCH", message, details, 2);
}

function hardFail(message, details = []) {
  fail("FAIL", message, details, 1);
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase().replace(/[\s-]+/g, "_");
}

function authorityRoleToBranch(authorityRole) {
  const normalized = normalizeRole(authorityRole);
  if (normalized === "ORCHESTRATOR") return "gov_kernel";
  if (normalized === "VALIDATOR" || normalized === "WP_VALIDATOR" || normalized === "INTEGRATION_VALIDATOR") {
    return "gov_kernel";
  }
  if (normalized === "OPERATOR") return "user_ilja";
  return "gov_kernel";
}

function ensureStateDir() {
  ensureValidatorGateDir();
}

function stateFilePath(wpId) {
  return resolveValidatorGatePath(wpId);
}

function normalizeState(raw) {
  const validationSessions =
    raw?.validation_sessions && typeof raw.validation_sessions === "object"
      ? raw.validation_sessions
      : {};
  const committedValidationEvidence =
    raw?.committed_validation_evidence && typeof raw.committed_validation_evidence === "object"
      ? raw.committed_validation_evidence
      : {};

  return {
    validation_sessions: validationSessions,
    archived_sessions: Array.isArray(raw?.archived_sessions) ? raw.archived_sessions : [],
    committed_validation_evidence: committedValidationEvidence,
  };
}

function loadWpState(wpId) {
  ensureStateDir();
  const filePath = resolveValidatorGatePath(wpId);
  if (!fs.existsSync(filePath)) return normalizeState({});
  return normalizeState(JSON.parse(fs.readFileSync(filePath, "utf8")));
}

function saveWpState(wpId, state) {
  ensureStateDir();
  fs.writeFileSync(stateFilePath(wpId), `${JSON.stringify(normalizeState(state), null, 2)}\n`, "utf8");
}

function parseArgs(argv) {
  const wpId = String(argv[0] || "").trim();
  if (!wpId || !wpId.startsWith("WP-")) usage();
  const parsed = { wpId, rev: "", range: "" };
  for (let index = 1; index < argv.length; index += 1) {
    const token = String(argv[index] || "").trim();
    if (token === "--rev") {
      parsed.rev = String(argv[index + 1] || "").trim();
      if (!parsed.rev) usage();
      index += 1;
      continue;
    }
    if (token === "--range") {
      parsed.range = String(argv[index + 1] || "").trim();
      if (!parsed.range || !parsed.range.includes("..")) usage();
      index += 1;
      continue;
    }
    usage();
  }
  if (parsed.rev && parsed.range) usage();
  return parsed;
}

function runInWorktree(worktreeAbs, command, args) {
  const result = spawnSync(command, args, {
    cwd: worktreeAbs,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    code: typeof result.status === "number" ? result.status : 1,
    output: `${result.stdout || ""}${result.stderr || ""}`.trim(),
  };
}

function gitInWorktree(worktreeAbs, args) {
  const result = runInWorktree(worktreeAbs, "git", args);
  if (result.code !== 0) {
    throw new Error(result.output || `git ${args.join(" ")} failed`);
  }
  return result.output.trim();
}

function selectCommittedTarget(worktreeAbs, packetContent, parsedArgs) {
  if (parsedArgs.range) {
    return {
      mode: "COMMITTED_RANGE",
      args: ["--range", parsedArgs.range],
      summary: parsedArgs.range,
      targetHeadSha: parsedArgs.range.split("..")[1].trim(),
    };
  }
  if (parsedArgs.rev) {
    return {
      mode: "COMMITTED_REV",
      args: ["--rev", parsedArgs.rev],
      summary: parsedArgs.rev,
      targetHeadSha: parsedArgs.rev,
    };
  }

  const mergeBaseSha = parseMergeBaseSha(packetContent);
  if (mergeBaseSha) {
    return {
      mode: "COMMITTED_RANGE",
      args: ["--range", `${mergeBaseSha}..HEAD`],
      summary: `${mergeBaseSha}..HEAD`,
      targetHeadSha: "HEAD",
    };
  }

  return {
    mode: "COMMITTED_REV",
    args: ["--rev", "HEAD"],
    summary: "HEAD",
    targetHeadSha: "HEAD",
  };
}

function persistEvidence(wpId, evidence) {
  const state = loadWpState(wpId);
  state.committed_validation_evidence[wpId] = recordCommittedValidationRun(
    state.committed_validation_evidence[wpId],
    evidence,
  );
  saveWpState(wpId, state);
  return state.committed_validation_evidence[wpId];
}

const parsed = parseArgs(process.argv.slice(2));
const gitContext = currentGitContext();
const repoRoot = gitContext.topLevel || REPO_ROOT;
const packetContentForContext = loadPacket(parsed.wpId);
const workflowLane = parseClaimField(packetContentForContext, "WORKFLOW_LANE");
const workflowAuthority = parseClaimField(packetContentForContext, "WORKFLOW_AUTHORITY")
  || (String(workflowLane || "").trim().toUpperCase() === "ORCHESTRATOR_MANAGED" ? "ORCHESTRATOR" : "ORCHESTRATOR");
const expectedGovernanceBranch = authorityRoleToBranch(workflowAuthority);

if (!packetExists(parsed.wpId)) {
  hardFail("Task packet not found", [packetPath(parsed.wpId)]);
}

const validatorGovernanceStateForContext = evaluateValidatorPacketGovernanceState({
  wpId: parsed.wpId,
  packetPath: packetPath(parsed.wpId),
  packetContent: packetContentForContext,
});
if (!validatorGovernanceStateForContext.allowValidationResume) {
  hardFail("Committed handoff validation is blocked for this packet", [
    validatorGovernanceStateForContext.message,
    `computed_policy_outcome=${validatorGovernanceStateForContext.computedPolicy.outcome}`,
    `computed_policy_applicability=${validatorGovernanceStateForContext.computedPolicy.applicability_reason || "APPLICABLE"}`,
  ]);
}

const logs = loadOrchestratorGateLogs();
const prepareEntry = lastGateLog(logs, parsed.wpId, "PREPARE");
if (!prepareEntry) {
  if (gitContext.branch !== expectedGovernanceBranch) {
    contextMismatch("PREPARE gate entry is unavailable in this checkout", [
      `current_branch=${gitContext.branch || "<detached>"}`,
      `expected_governance_branch=${expectedGovernanceBranch}`,
      `rerun=just validator-handoff-check ${parsed.wpId}`,
    ]);
  }
  hardFail("PREPARE gate entry is missing", [`Run: just orchestrator-next ${parsed.wpId}`]);
}

const syncState = preparedWorktreeSyncState(parsed.wpId, prepareEntry, repoRoot);
const worktreeAbs = resolvePrepareWorktreeAbs(prepareEntry, repoRoot);
if (!worktreeAbs || !fs.existsSync(worktreeAbs)) {
  contextMismatch("Assigned PREPARE worktree is unavailable in this environment", [
    `recorded_worktree_dir=${String(prepareEntry.worktree_dir || "<missing>")}`,
    `current_branch=${gitContext.branch || "<detached>"}`,
    "This blocks committed handoff validation in this checkout but does not by itself prove a WP failure.",
  ]);
}
if (!String(syncState.actualBranch || "").trim()) {
  contextMismatch("Assigned PREPARE worktree branch could not be resolved", [
    worktreeAbs,
    "The committed handoff source cannot be inspected from this environment.",
  ]);
}
if (
  String(syncState.expectedBranch || "").trim()
  && String(syncState.actualBranch || "").trim() !== String(syncState.expectedBranch || "").trim()
) {
  hardFail("Assigned PREPARE worktree branch does not match PREPARE", [
    `expected=${syncState.expectedBranch}`,
    `actual=${syncState.actualBranch}`,
  ]);
}

const nonBlockingSyncWarnings = (syncState.issues || []).filter((issue) =>
  !/does not exist|branch mismatch|PREPARE is missing worktree_dir|could not be resolved/i.test(issue),
);

const resolvedPacket = resolveWorkPacketPath(parsed.wpId);
const packetPathRel = resolvedPacket?.packetPath || path.join(GOV_ROOT_REPO_REL, "task_packets", `${parsed.wpId}.md`);
const worktreePacketPath = path.join(worktreeAbs, packetPathRel);
const packetContent = fs.existsSync(worktreePacketPath)
  ? fs.readFileSync(worktreePacketPath, "utf8")
  : fs.readFileSync(packetPathRel, "utf8");
const communicationHealthCheckPath = path.join(
  GOV_ROOT_REPO_REL,
  "roles_shared",
  "checks",
  "wp-communication-health-check.mjs",
);
const communicationHealth = runInWorktree(repoRoot, process.execPath, [
  communicationHealthCheckPath,
  parsed.wpId,
  "HANDOFF",
]);
if (communicationHealth.code !== 0) {
  hardFail("Direct review communication contract is not ready for validator handoff", [
    ...communicationHealth.output.split(/\r?\n/).filter(Boolean),
  ]);
}
const packetComplete = runInWorktree(repoRoot, process.execPath, [
  path.join(GOV_ROOT_REPO_REL, "roles", "validator", "checks", "validator-packet-complete.mjs"),
  parsed.wpId,
]);
if (packetComplete.code !== 0) {
  hardFail("Packet closure hygiene is incomplete for validator handoff", [
    ...packetComplete.output.split(/\r?\n/).filter(Boolean),
  ]);
}
const committedTarget = selectCommittedTarget(syncState.worktreeAbs, packetContent, parsed);
let targetHeadSha = committedTarget.targetHeadSha;
try {
  targetHeadSha = gitInWorktree(worktreeAbs, ["rev-parse", committedTarget.targetHeadSha]);
} catch {
  // Keep user-specified ref literal in the evidence summary if rev-parse fails.
}

const preWork = runInWorktree(worktreeAbs, "just", ["pre-work", parsed.wpId]);
const cargoClean = runInWorktree(worktreeAbs, "just", ["cargo-clean"]);
const postWork = runInWorktree(worktreeAbs, "just", ["post-work", parsed.wpId, ...committedTarget.args]);
const cargoCleanStatus = cargoClean.code === 0 ? "PASS" : "FAIL";

const evidence = {
  wp_id: parsed.wpId,
  status: preWork.code === 0 && cargoClean.code === 0 && postWork.code === 0 ? "PASS" : "FAIL",
  validated_at: new Date().toISOString(),
  source_truth: "PREPARE_WORKTREE",
  prepare_branch: String(prepareEntry.branch || "").trim(),
  prepare_worktree_dir: String(prepareEntry.worktree_dir || "").trim(),
  prepare_worktree_sync_warnings: nonBlockingSyncWarnings,
  committed_validation_mode: committedTarget.mode,
  committed_validation_target: committedTarget.summary,
  target_head_sha: targetHeadSha,
  pre_work_status: preWork.code === 0 ? "PASS" : "FAIL",
  cargo_clean_required: true,
  cargo_clean_status: cargoCleanStatus,
  post_work_status: postWork.code === 0 ? "PASS" : "FAIL",
  pre_work_command: `just pre-work ${parsed.wpId}`,
  cargo_clean_command: "just cargo-clean",
  post_work_command: `just post-work ${parsed.wpId} ${committedTarget.args.join(" ")}`.trim(),
  pre_work_output: preWork.output,
  cargo_clean_output: cargoClean.output,
  post_work_output: postWork.output,
};

const persistedEvidence = persistEvidence(parsed.wpId, evidence);
const durableCommittedProof = committedEvidenceForCloseout(persistedEvidence);
const livePrepareHealth = livePrepareWorktreeHealthEvidence(persistedEvidence);

if (evidence.status !== "PASS") {
  const extraDetails = [];
  if (durableCommittedProof?.status === "PASS") {
    extraDetails.push(`durable_committed_target_head_sha=${durableCommittedProof.target_head_sha}`);
    extraDetails.push(`durable_committed_proof_validated_at=${durableCommittedProof.validated_at}`);
  }
  hardFail("Committed handoff validation failed", [
    `prepare_worktree_dir=${evidence.prepare_worktree_dir}`,
    `committed_validation_target=${evidence.committed_validation_target}`,
    `pre_work_status=${evidence.pre_work_status}`,
    `cargo_clean_status=${evidence.cargo_clean_status}`,
    `post_work_status=${evidence.post_work_status}`,
    ...(livePrepareHealth
      ? [
        `live_prepare_worktree_status=${livePrepareHealth.status}`,
      ]
      : []),
    ...extraDetails,
    `evidence_file=${stateFilePath(parsed.wpId).replace(/\\/g, "/")}`,
  ]);
}

console.log(`[VALIDATOR_HANDOFF_CHECK] PASS`);
console.log(`  wp_id=${parsed.wpId}`);
console.log(`  prepare_worktree_dir=${evidence.prepare_worktree_dir}`);
console.log(`  committed_validation_mode=${evidence.committed_validation_mode}`);
console.log(`  committed_validation_target=${evidence.committed_validation_target}`);
console.log(`  target_head_sha=${evidence.target_head_sha}`);
console.log(`  live_prepare_worktree_status=${livePrepareHealth?.status || evidence.status}`);
console.log(`  durable_committed_proof_status=${durableCommittedProof?.status || evidence.status}`);
console.log(`  evidence_file=${stateFilePath(parsed.wpId).replace(/\\/g, "/")}`);
if (nonBlockingSyncWarnings.length > 0) {
  console.log(`  sync_warnings=${formatBoundedItemList(nonBlockingSyncWarnings, { noun: "warning" })}`);
}
