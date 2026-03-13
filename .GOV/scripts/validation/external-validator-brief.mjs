#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  currentGitContext,
  loadJson,
  loadOrchestratorGateLogs,
  loadPacket,
  packetExists,
  lastGateLog,
  parseClaimField,
  parseMergeBaseSha,
  resolvePrepareWorktreeAbs,
} from "../role-resume-utils.mjs";

function usage() {
  console.error("Usage: node .GOV/scripts/validation/external-validator-brief.mjs WP-{ID} [--json]");
  process.exit(1);
}

function parseArgs(argv) {
  const wpId = String(argv[0] || "").trim();
  if (!wpId || !wpId.startsWith("WP-")) usage();

  let json = false;
  for (let index = 1; index < argv.length; index += 1) {
    const token = String(argv[index] || "").trim();
    if (token === "--json") {
      json = true;
      continue;
    }
    usage();
  }

  return { wpId, json };
}

function safeGit(cwd, args) {
  try {
    return execFileSync("git", args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "";
  }
}

function findLocalBranchWorktree(repoRoot, branchName) {
  const output = safeGit(repoRoot, ["worktree", "list", "--porcelain"]);
  if (!output) return "";

  const lines = output.split(/\r?\n/);
  let currentWorktree = "";
  let currentBranch = "";

  function flushMatch() {
    if (!currentWorktree || !currentBranch) return "";
    const normalizedBranch = currentBranch.replace(/^refs\/heads\//, "");
    return normalizedBranch === branchName ? currentWorktree : "";
  }

  for (const line of lines) {
    if (!line.trim()) {
      const match = flushMatch();
      if (match) return match;
      currentWorktree = "";
      currentBranch = "";
      continue;
    }
    if (line.startsWith("worktree ")) {
      currentWorktree = line.slice("worktree ".length).trim();
      continue;
    }
    if (line.startsWith("branch ")) {
      currentBranch = line.slice("branch ".length).trim();
    }
  }

  return flushMatch();
}

function loadCommittedValidationEvidence(wpId) {
  const statePath = path.join(".GOV", "validator_gates", `${wpId}.json`);
  const state = loadJson(statePath, {});
  const evidence =
    state?.committed_validation_evidence && typeof state.committed_validation_evidence === "object"
      ? state.committed_validation_evidence[wpId]
      : null;
  return evidence && typeof evidence === "object" ? evidence : null;
}

function pushUnique(target, value) {
  const normalized = String(value || "").trim();
  if (!normalized || target.includes(normalized)) return;
  target.push(normalized);
}

function formatText(brief) {
  const lines = [
    "EXTERNAL_VALIDATOR_BRIEF [CX-VAL-EXT-001]",
    `- WP_ID: ${brief.wp_id}`,
    `- WORKFLOW_LANE: ${brief.workflow_lane}`,
    `- VALIDATION_CONTEXT: ${brief.validation_context}`,
    `- CODE_TARGET_BRANCH: ${brief.code_target.branch}`,
    `- CODE_TARGET_COMMIT: ${brief.code_target.commit}`,
    `- CODE_TARGET_HINT: ${brief.code_target.hint}`,
    `- GOVERNANCE_CHECKOUT_BRANCH: ${brief.governance_target.branch}`,
    `- GOVERNANCE_CHECKOUT_PATH: ${brief.governance_target.checkout_path}`,
    `- PREPARE_WORKTREE_DIR: ${brief.handoff_target.prepare_worktree_dir}`,
    `- PREPARE_WORKTREE_HEAD: ${brief.handoff_target.prepare_worktree_head}`,
    `- HANDOFF_COMMAND: ${brief.handoff_target.command}`,
    `- GOVERNANCE_COMMAND: ${brief.governance_target.command}`,
    `- LEGAL_VERDICTS: PASS | FAIL | PENDING`,
    `- SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | LEGAL_VERDICT`,
    `- RUNTIME_LEDGER_RULE: session ledgers/output logs are operator/runtime evidence only; they are not packet-scope implementation authority`,
  ];

  if (brief.context_notes.length > 0) {
    lines.push("- CONTEXT_NOTES:");
    for (const note of brief.context_notes) lines.push(`  - ${note}`);
  }

  if (brief.required_commands.length > 0) {
    lines.push("- REQUIRED_COMMANDS:");
    for (const command of brief.required_commands) lines.push(`  - ${command}`);
  }

  lines.push("- REPORT_TEMPLATE:");
  lines.push("  - VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH");
  lines.push("  - CODE_VERDICT: PASS | FAIL | NOT_RUN");
  lines.push("  - GOVERNANCE_VERDICT: PASS | FAIL | NOT_RUN");
  lines.push("  - ENVIRONMENT_VERDICT: PASS | FAIL | NOT_RUN");
  lines.push("  - LEGAL_VERDICT: PASS | FAIL | PENDING");
  lines.push("  - RULE: strings like accept/approved/technical pass are not legal verdicts");

  return `${lines.join("\n")}\n`;
}

const parsed = parseArgs(process.argv.slice(2));
const gitContext = currentGitContext();
const repoRoot = gitContext.topLevel || process.cwd();
const orchestratorWorktree = findLocalBranchWorktree(repoRoot, "role_orchestrator");

if (!packetExists(parsed.wpId)) {
  console.error(`[EXTERNAL_VALIDATOR_BRIEF] Task packet not found: .GOV/task_packets/${parsed.wpId}.md`);
  process.exit(1);
}

const packetContent = loadPacket(parsed.wpId);
const workflowLane = parseClaimField(packetContent, "WORKFLOW_LANE") || "<missing>";
const packetBranch = parseClaimField(packetContent, "LOCAL_BRANCH") || "<missing>";
const packetWorktreeDir = parseClaimField(packetContent, "LOCAL_WORKTREE_DIR") || "<missing>";
const mergeBaseField = parseClaimField(packetContent, "MERGE_BASE_SHA");
const mergeBaseSha = (String(mergeBaseField || "").match(/[a-f0-9]{40}/i)?.[0] || parseMergeBaseSha(packetContent) || "").trim();
const orchestratorLogs = loadOrchestratorGateLogs();
const prepareEntry = lastGateLog(orchestratorLogs, parsed.wpId, "PREPARE");
const prepareWorktreeAbs = prepareEntry ? resolvePrepareWorktreeAbs(prepareEntry, repoRoot) : "";
const prepareWorktreeHead =
  prepareWorktreeAbs && fs.existsSync(prepareWorktreeAbs)
    ? safeGit(prepareWorktreeAbs, ["rev-parse", "HEAD"])
    : "";
const committedEvidence = loadCommittedValidationEvidence(parsed.wpId);

const contextNotes = [];
let validationContext = "OK";

if (gitContext.branch !== "role_orchestrator") {
  validationContext = "CONTEXT_MISMATCH";
  pushUnique(
    contextNotes,
    `Current checkout branch is ${gitContext.branch || "<detached>"}; governance validation target is role_orchestrator.`,
  );
}

if (!prepareEntry) {
  validationContext = "CONTEXT_MISMATCH";
  pushUnique(
    contextNotes,
    "PREPARE gate entry is not available in this checkout; committed handoff validation must run from the governance lane.",
  );
}

if (prepareEntry && (!prepareWorktreeAbs || !fs.existsSync(prepareWorktreeAbs))) {
  validationContext = "CONTEXT_MISMATCH";
  pushUnique(
    contextNotes,
    `Recorded PREPARE worktree is unavailable in this environment: ${String(prepareEntry.worktree_dir || "<missing>")}.`,
  );
}

const codeTargetCommit =
  String(committedEvidence?.target_head_sha || "").trim()
  || prepareWorktreeHead
  || "HEAD of PREPARE worktree";

const codeTargetHint = mergeBaseSha
  ? `Use a clean checkout of ${packetBranch} and validate the committed target against ${mergeBaseSha}..${codeTargetCommit}.`
  : `Use a clean checkout of ${packetBranch} and validate commit ${codeTargetCommit}.`;

const requiredCommands = [];
pushUnique(requiredCommands, `just external-validator-brief ${parsed.wpId}`);
pushUnique(requiredCommands, `just validator-handoff-check ${parsed.wpId}`);
pushUnique(requiredCommands, "just gov-check");
pushUnique(requiredCommands, "just cargo-clean");
pushUnique(requiredCommands, `just post-work ${parsed.wpId}${mergeBaseSha ? ` --range ${mergeBaseSha}..HEAD` : ""}`);

const brief = {
  schema_id: "hsk.external_validator_brief@1",
  schema_version: "external_validator_brief_v1",
  wp_id: parsed.wpId,
  workflow_lane: workflowLane,
  validation_context: validationContext,
  code_target: {
    branch: packetBranch,
    packet_worktree_dir: packetWorktreeDir,
    commit: codeTargetCommit,
    hint: codeTargetHint,
  },
  governance_target: {
    branch: "role_orchestrator",
    checkout_path: (orchestratorWorktree || repoRoot).replace(/\\/g, "/"),
    command: "just gov-check",
  },
  handoff_target: {
    prepare_worktree_dir: String(prepareEntry?.worktree_dir || packetWorktreeDir || "<missing>").trim(),
    prepare_worktree_head: prepareWorktreeHead || codeTargetCommit,
    command: `just validator-handoff-check ${parsed.wpId}`,
  },
  legal_verdicts: ["PASS", "FAIL", "PENDING"],
  split_fields: [
    "VALIDATION_CONTEXT",
    "CODE_VERDICT",
    "GOVERNANCE_VERDICT",
    "ENVIRONMENT_VERDICT",
    "LEGAL_VERDICT",
  ],
  required_commands: requiredCommands,
  context_notes: contextNotes,
};

if (parsed.json) {
  process.stdout.write(`${JSON.stringify(brief, null, 2)}\n`);
} else {
  process.stdout.write(formatText(brief));
}
