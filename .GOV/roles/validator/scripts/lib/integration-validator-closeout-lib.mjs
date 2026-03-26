import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionRegistry,
  readJsonFile,
} from "../../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  SESSION_CONTROL_BROKER_STATE_FILE,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  normalizePath,
} from "../../../../roles_shared/scripts/session/session-policy.mjs";
import {
  evaluateValidatorPassAuthority,
  normalizeValidatorRole,
} from "./validator-governance-lib.mjs";
import { validateSignedScopeCompatibilityTruth } from "../../../../roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs";

function makeIssueSet() {
  return new Set();
}

function normalizeStatus(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeOutputPath(repoRoot, filePath) {
  return normalizePath(path.resolve(repoRoot, String(filePath || "")));
}

function samePath(left, right) {
  return normalizePath(left).toLowerCase() === normalizePath(right).toLowerCase();
}

function defaultGitRunner(worktreeAbs, args) {
  const result = spawnSync("git", args, {
    cwd: worktreeAbs,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    code: typeof result.status === "number" ? result.status : 1,
    output: `${result.stdout || ""}${result.stderr || ""}`.trim(),
  };
}

export function evaluateIntegrationValidatorTopology({
  repoRoot = process.cwd(),
  wpId = "",
  packetContent = "",
  actorContext = {},
  committedEvidence = null,
  gitRunner = null,
  worktreeExists = fs.existsSync,
} = {}) {
  const issues = makeIssueSet();
  const warnings = [];
  const authority = evaluateValidatorPassAuthority({
    packetContent,
    actorContext,
  });
  for (const issue of authority.issues || []) issues.add(issue);

  const actorRole = normalizeValidatorRole(actorContext?.actorRole);
  if (actorRole !== "INTEGRATION_VALIDATOR") {
    issues.add(`Closeout preflight requires the Integration Validator lane; current lane resolved to ${actorRole || "UNKNOWN"}.`);
  }

  const expectedBranch = defaultIntegrationValidatorBranch(wpId);
  const expectedWorktreeDir = normalizePath(defaultIntegrationValidatorWorktreeDir(wpId));
  const actorBranch = String(actorContext?.actorBranch || "").trim();
  const actorWorktreeDir = normalizePath(actorContext?.actorWorktreeDir || "");

  if (actorBranch !== expectedBranch) {
    issues.add(`Integration Validator must run from branch ${expectedBranch}; current branch is ${actorBranch || "<unknown>"}.`);
  }
  if (!samePath(actorWorktreeDir, expectedWorktreeDir)) {
    issues.add(`Integration Validator must run from ${expectedWorktreeDir}; current worktree is ${actorWorktreeDir || "<unknown>"}.`);
  }

  if (!committedEvidence || typeof committedEvidence !== "object") {
    issues.add("Committed handoff validation evidence is missing.");
    return { ok: false, issues: Array.from(issues), warnings };
  }
  if (normalizeStatus(committedEvidence.status) !== "PASS") {
    issues.add(`Committed handoff validation evidence must be PASS (found ${committedEvidence.status || "NONE"}).`);
  }

  const targetHeadSha = String(committedEvidence?.target_head_sha || "").trim();
  if (!targetHeadSha) {
    issues.add("Committed handoff validation evidence is missing target_head_sha.");
    return { ok: false, issues: Array.from(issues), warnings };
  }

  const worktreeAbs = path.resolve(repoRoot, actorWorktreeDir || expectedWorktreeDir);
  if (!worktreeExists(worktreeAbs)) {
    issues.add(`Integration Validator worktree is unavailable: ${normalizePath(worktreeAbs)}`);
    return { ok: false, issues: Array.from(issues), warnings };
  }

  const runGit = typeof gitRunner === "function"
    ? gitRunner
    : (args) => defaultGitRunner(worktreeAbs, args);
  const targetCheck = runGit(["cat-file", "-e", `${targetHeadSha}^{commit}`]);
  if (targetCheck.code !== 0) {
    issues.add(`Integration Validator worktree cannot resolve committed target ${targetHeadSha}.`);
  }

  const currentMainHead = runGit(["rev-parse", "HEAD"]);
  const currentMainHeadSha = currentMainHead.code === 0 ? String(currentMainHead.output || "").trim() : "";
  if (!currentMainHeadSha) {
    issues.add("Integration Validator could not resolve current local main HEAD.");
  }

  return {
    ok: issues.size === 0,
    issues: Array.from(issues),
    warnings,
    expectedBranch,
    expectedWorktreeDir,
    resolvedWorktreeAbs: normalizePath(worktreeAbs),
    targetHeadSha,
    currentMainHeadSha,
  };
}

export function evaluateWpSessionControlCloseoutBundle({
  repoRoot = process.cwd(),
  wpId = "",
  requests = [],
  results = [],
  sessions = [],
  brokerState = null,
  fileExists = fs.existsSync,
} = {}) {
  const issues = makeIssueSet();
  const warnings = [];
  const wpRequests = (requests || []).filter((request) => String(request?.wp_id || "").trim() === wpId);
  const wpResults = (results || []).filter((result) => String(result?.wp_id || "").trim() === wpId);
  const wpSessions = (sessions || []).filter((session) => String(session?.wp_id || "").trim() === wpId);
  const activeRuns = Array.isArray(brokerState?.active_runs)
    ? brokerState.active_runs.filter((run) => String(run?.wp_id || "").trim() === wpId)
    : [];

  const requestById = new Map();
  for (const request of wpRequests) requestById.set(String(request?.command_id || "").trim(), request);
  const resultById = new Map();
  for (const result of wpResults) resultById.set(String(result?.command_id || "").trim(), result);

  if (activeRuns.length > 0) {
    issues.add(`Active broker runs still exist for ${wpId}: ${activeRuns.map((run) => String(run?.command_id || "<missing>")).join(", ")}`);
  }

  for (const result of wpResults) {
    const commandId = String(result?.command_id || "").trim();
    if (!commandId) continue;
    if (!requestById.has(commandId)) {
      issues.add(`Result ${commandId} has no matching request for ${wpId}.`);
    }
  }

  for (const request of wpRequests) {
    const commandId = String(request?.command_id || "").trim();
    if (!commandId) continue;
    const result = resultById.get(commandId);
    if (!result) {
      issues.add(`Request ${commandId} has no settled result for ${wpId}.`);
      continue;
    }
    if (String(request?.session_key || "").trim() !== String(result?.session_key || "").trim()) {
      issues.add(`Result ${commandId} session_key does not match request.`);
    }
    if (String(request?.role || "").trim() !== String(result?.role || "").trim()) {
      issues.add(`Result ${commandId} role does not match request.`);
    }
    if (String(request?.command_kind || "").trim() !== String(result?.command_kind || "").trim()) {
      issues.add(`Result ${commandId} command_kind does not match request.`);
    }
    if (normalizeOutputPath(repoRoot, request?.output_jsonl_file) !== normalizeOutputPath(repoRoot, result?.output_jsonl_file)) {
      issues.add(`Result ${commandId} output_jsonl_file does not match request.`);
    }
    const outputPath = path.resolve(repoRoot, String(result?.output_jsonl_file || request?.output_jsonl_file || ""));
    if (!fileExists(outputPath)) {
      issues.add(`Settled output log is missing for ${commandId}: ${normalizePath(outputPath)}`);
    }
  }

  for (const session of wpSessions) {
    const sessionKey = String(session?.session_key || "<missing>").trim();
    const lastCommandId = String(session?.last_command_id || "").trim();
    const lastCommandStatus = normalizeStatus(session?.last_command_status);
    if (!lastCommandId || lastCommandStatus === "NONE") continue;

    const result = resultById.get(lastCommandId);
    if (lastCommandStatus === "RUNNING") {
      issues.add(`Session ${sessionKey} still reports RUNNING for ${lastCommandId}.`);
      continue;
    }
    if ((lastCommandStatus === "COMPLETED" || lastCommandStatus === "FAILED") && !result) {
      issues.add(`Session ${sessionKey} reports ${lastCommandStatus} for ${lastCommandId} but no settled result exists.`);
      continue;
    }
    if (result && normalizeStatus(result?.status) !== lastCommandStatus) {
      issues.add(`Session ${sessionKey} last_command_status ${lastCommandStatus} disagrees with settled result ${normalizeStatus(result?.status)} for ${lastCommandId}.`);
    }
  }

  return {
    ok: issues.size === 0,
    issues: Array.from(issues),
    warnings,
    summary: {
      request_count: wpRequests.length,
      result_count: wpResults.length,
      session_count: wpSessions.length,
      active_run_count: activeRuns.length,
    },
  };
}

export function evaluateIntegrationValidatorCloseoutState({
  repoRoot = process.cwd(),
  wpId = "",
  packetContent = "",
  actorContext = {},
  committedEvidence = null,
  gitRunner = null,
  worktreeExists = fs.existsSync,
  fileExists = fs.existsSync,
  registrySessions = null,
  requests = null,
  results = null,
  brokerState = null,
} = {}) {
  const resolvedRequests = Array.isArray(requests)
    ? requests
    : loadSessionControlRequests(repoRoot).requests;
  const resolvedResults = Array.isArray(results)
    ? results
    : loadSessionControlResults(repoRoot).results;
  const resolvedSessions = Array.isArray(registrySessions)
    ? registrySessions
    : loadSessionRegistry(repoRoot).registry.sessions;
  const resolvedBrokerState = brokerState ?? readJsonFile(
    path.resolve(repoRoot, SESSION_CONTROL_BROKER_STATE_FILE),
    { active_runs: [] },
  );

  const topology = evaluateIntegrationValidatorTopology({
    repoRoot,
    wpId,
    packetContent,
    actorContext,
    committedEvidence,
    gitRunner,
    worktreeExists,
  });
  const closeoutBundle = evaluateWpSessionControlCloseoutBundle({
    repoRoot,
    wpId,
    requests: resolvedRequests,
    results: resolvedResults,
    sessions: resolvedSessions,
    brokerState: resolvedBrokerState,
    fileExists,
  });
  const scopeCompatibility = validateSignedScopeCompatibilityTruth(packetContent, {
    packetPath: `<${wpId}>`,
    currentMainHeadSha: topology.currentMainHeadSha || "",
    requireReadyForPass: true,
  });

  return {
    ok: topology.ok && closeoutBundle.ok && scopeCompatibility.errors.length === 0,
    topology,
    closeoutBundle,
    scopeCompatibility,
    issues: [...topology.issues, ...closeoutBundle.issues, ...scopeCompatibility.errors],
    warnings: [...topology.warnings, ...closeoutBundle.warnings],
  };
}
