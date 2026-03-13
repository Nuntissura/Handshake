#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  loadOrchestratorGateLogs,
  lastGateLog,
  packetExists,
  parseMergeBaseSha,
  preparedWorktreeSyncState,
  resolvePrepareWorktreeAbs,
} from "../role-resume-utils.mjs";

const STATE_DIR = path.join(".GOV", "validator_gates");

function usage() {
  console.error("Usage: node .GOV/scripts/validation/validator-handoff-check.mjs WP-{ID} [--rev <git-rev> | --range <base>..<head>]");
  process.exit(1);
}

function fail(message, details = []) {
  console.error(`[VALIDATOR_HANDOFF_CHECK] ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(1);
}

function ensureStateDir() {
  if (!fs.existsSync(STATE_DIR)) fs.mkdirSync(STATE_DIR, { recursive: true });
}

function stateFilePath(wpId) {
  return path.join(STATE_DIR, `${wpId}.json`);
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
  const filePath = stateFilePath(wpId);
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
  state.committed_validation_evidence[wpId] = evidence;
  saveWpState(wpId, state);
}

const parsed = parseArgs(process.argv.slice(2));
const repoRoot = process.cwd();

if (!packetExists(parsed.wpId)) {
  fail("Task packet not found", [`.GOV/task_packets/${parsed.wpId}.md`]);
}

const logs = loadOrchestratorGateLogs();
const prepareEntry = lastGateLog(logs, parsed.wpId, "PREPARE");
if (!prepareEntry) {
  fail("PREPARE gate entry is missing", [`Run: just orchestrator-next ${parsed.wpId}`]);
}

const syncState = preparedWorktreeSyncState(parsed.wpId, prepareEntry, repoRoot);
const worktreeAbs = resolvePrepareWorktreeAbs(prepareEntry, repoRoot);
if (!worktreeAbs || !fs.existsSync(worktreeAbs)) {
  fail("Assigned PREPARE worktree is missing", [String(prepareEntry.worktree_dir || "<missing>")]);
}
if (!String(syncState.actualBranch || "").trim()) {
  fail("Assigned PREPARE worktree branch could not be resolved", [worktreeAbs]);
}
if (
  String(syncState.expectedBranch || "").trim()
  && String(syncState.actualBranch || "").trim() !== String(syncState.expectedBranch || "").trim()
) {
  fail("Assigned PREPARE worktree branch does not match PREPARE", [
    `expected=${syncState.expectedBranch}`,
    `actual=${syncState.actualBranch}`,
  ]);
}

const nonBlockingSyncWarnings = (syncState.issues || []).filter((issue) =>
  !/does not exist|branch mismatch|PREPARE is missing worktree_dir|could not be resolved/i.test(issue),
);

const worktreePacketPath = path.join(worktreeAbs, ".GOV", "task_packets", `${parsed.wpId}.md`);
const packetContent = fs.existsSync(worktreePacketPath)
  ? fs.readFileSync(worktreePacketPath, "utf8")
  : fs.readFileSync(path.join(".GOV", "task_packets", `${parsed.wpId}.md`), "utf8");
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
  cargo_clean_status: cargoClean.code === 0 ? "PASS" : "FAIL",
  post_work_status: postWork.code === 0 ? "PASS" : "FAIL",
  pre_work_command: `just pre-work ${parsed.wpId}`,
  cargo_clean_command: "just cargo-clean",
  post_work_command: `just post-work ${parsed.wpId} ${committedTarget.args.join(" ")}`.trim(),
  pre_work_output: preWork.output,
  cargo_clean_output: cargoClean.output,
  post_work_output: postWork.output,
};

persistEvidence(parsed.wpId, evidence);

if (evidence.status !== "PASS") {
  fail("Committed handoff validation failed", [
    `prepare_worktree_dir=${evidence.prepare_worktree_dir}`,
    `committed_validation_target=${evidence.committed_validation_target}`,
    `pre_work_status=${evidence.pre_work_status}`,
    `cargo_clean_status=${evidence.cargo_clean_status}`,
    `post_work_status=${evidence.post_work_status}`,
    `evidence_file=${stateFilePath(parsed.wpId).replace(/\\/g, "/")}`,
  ]);
}

console.log(`[VALIDATOR_HANDOFF_CHECK] PASS`);
console.log(`  wp_id=${parsed.wpId}`);
console.log(`  prepare_worktree_dir=${evidence.prepare_worktree_dir}`);
console.log(`  committed_validation_mode=${evidence.committed_validation_mode}`);
console.log(`  committed_validation_target=${evidence.committed_validation_target}`);
console.log(`  target_head_sha=${evidence.target_head_sha}`);
console.log(`  evidence_file=${stateFilePath(parsed.wpId).replace(/\\/g, "/")}`);
if (nonBlockingSyncWarnings.length > 0) {
  console.log("  sync_warnings=");
  for (const warning of nonBlockingSyncWarnings) {
    console.log(`    - ${warning}`);
  }
}
