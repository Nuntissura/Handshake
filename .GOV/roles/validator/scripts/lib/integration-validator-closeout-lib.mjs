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
import { validateCandidateTargetAgainstSignedScope } from "../../../../roles_shared/scripts/lib/signed-scope-surface-lib.mjs";
import {
  committedEvidenceForCloseout,
  livePrepareWorktreeHealthEvidence,
} from "./committed-validation-evidence-lib.mjs";
import { GOV_ROOT_ABS, REPO_ROOT } from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";

function makeIssueSet() {
  return new Set();
}

function normalizeStatus(value) {
  return String(value || "").trim().toUpperCase();
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function normalizeSessionOfRecord(value) {
  const raw = String(value || "").trim();
  if (!raw || /^(<unassigned>|NONE|N\/A|NA|NULL)$/i.test(raw)) return null;
  return raw;
}

function normalizeOutputPath(repoRoot, filePath) {
  return normalizePath(path.resolve(repoRoot, String(filePath || "")));
}

function samePath(left, right) {
  return normalizePath(left).toLowerCase() === normalizePath(right).toLowerCase();
}

function normalizeCloseoutSyncEventMap(rawValue) {
  if (!rawValue || typeof rawValue !== "object" || Array.isArray(rawValue)) return {};
  return Object.fromEntries(
    Object.entries(rawValue)
      .filter(([key, value]) => String(key || "").trim() && Array.isArray(value))
      .map(([key, value]) => [String(key).trim(), value.filter((entry) => entry && typeof entry === "object")]),
  );
}

function defaultGovernanceViolationReporterRole({
  repoRoot = REPO_ROOT,
  actorContext = {},
  gitContext = {},
} = {}) {
  const actorRole = normalizeValidatorRole(actorContext?.actorRole);
  if (actorRole && actorRole !== "UNKNOWN") return actorRole;

  const branch = String(gitContext?.branch || actorContext?.actorBranch || "").trim();
  const topLevel = normalizePath(path.resolve(String(gitContext?.topLevel || "").trim() || repoRoot));
  const kernelRoot = normalizePath(path.resolve(repoRoot));
  if (branch === "gov_kernel" || samePath(topLevel, kernelRoot)) {
    return "ORCHESTRATOR";
  }
  return "SYSTEM";
}

function defaultGovernanceViolationReporterSession({
  actorContext = {},
  reporterRole = "SYSTEM",
} = {}) {
  const actorSessionId = String(actorContext?.actorSessionId || "").trim();
  if (actorSessionId) return actorSessionId;
  if (reporterRole === "ORCHESTRATOR") return "orchestrator-role-lock-guard";
  if (reporterRole === "INTEGRATION_VALIDATOR") return "integration-validator-final-lane-guard";
  return "workflow-boundary-guard";
}

function latestReceiptSessionForRole(receipts = [], role = "") {
  const normalizedRole = normalizeValidatorRole(role);
  return [...(Array.isArray(receipts) ? receipts : [])]
    .filter((entry) => normalizeValidatorRole(entry?.actor_role) === normalizedRole)
    .sort((left, right) => String(left?.timestamp_utc || "").localeCompare(String(right?.timestamp_utc || "")))
    .map((entry) => normalizeSessionOfRecord(entry?.actor_session))
    .filter(Boolean)
    .at(-1) || null;
}

export function resolveCloseoutValidatorSessionsOfRecord({
  packetContent = "",
  receipts = [],
  actorContext = {},
} = {}) {
  const packetWpValidator = normalizeSessionOfRecord(parseSingleField(packetContent, "WP_VALIDATOR_OF_RECORD"));
  const packetIntegrationValidator = normalizeSessionOfRecord(parseSingleField(packetContent, "INTEGRATION_VALIDATOR_OF_RECORD"));
  const actorIntegrationValidator = normalizeSessionOfRecord(actorContext?.actorSessionId)
    || normalizeSessionOfRecord(actorContext?.actorSessionKey);

  return {
    wpValidatorOfRecord: packetWpValidator || latestReceiptSessionForRole(receipts, "WP_VALIDATOR") || null,
    integrationValidatorOfRecord:
      packetIntegrationValidator
      || actorIntegrationValidator
      || latestReceiptSessionForRole(receipts, "INTEGRATION_VALIDATOR")
      || null,
  };
}

export function deriveFinalLaneGovernanceInvalidity({
  repoRoot = REPO_ROOT,
  actorContext = {},
  gitContext = {},
  governanceState = null,
  topology = null,
} = {}) {
  const actorRole = normalizeValidatorRole(actorContext?.actorRole);
  const reporterRole = defaultGovernanceViolationReporterRole({
    repoRoot,
    actorContext,
    gitContext,
  });
  const reporterSession = defaultGovernanceViolationReporterSession({
    actorContext,
    reporterRole,
  });
  const topologyIssues = Array.isArray(topology?.issues) ? topology.issues : [];
  const authorityIssues = topologyIssues.filter((issue) =>
    /requires the Integration Validator lane|final PASS authority|governed Integration Validator session identity|PASS authority belongs to|integration validator of record mismatch/i.test(
      String(issue || ""),
    )
  );
  const govRootViolation = governanceState?.terminalReason === "INTEGRATION_VALIDATOR_GOV_ROOT_MISCONFIGURED"
    || topologyIssues.some((issue) => /HANDSHAKE_GOV_ROOT|handshake_main\/\.GOV/i.test(String(issue || "")));

  if (govRootViolation) {
    return {
      workflowInvalidityCode: "FINAL_LANE_GOV_ROOT_VIOLATION",
      summary:
        "Final-lane closeout resolved live governance from handshake_main/.GOV instead of the kernel; repair HANDSHAKE_GOV_ROOT before contained-main progression.",
      actorRole: reporterRole,
      actorSession: reporterSession,
      specAnchor: "CX-212D",
      packetRowRef: "INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR",
    };
  }

  if (
    (actorRole && actorRole !== "UNKNOWN" && actorRole !== "INTEGRATION_VALIDATOR")
    || (!actorRole || actorRole === "UNKNOWN") && reporterRole === "ORCHESTRATOR"
  ) {
    return {
      workflowInvalidityCode: "ROLE_BOUNDARY_BREACH",
      summary:
        "Final-lane closeout was attempted outside the governed INTEGRATION_VALIDATOR lane; contained-main harmonization and merge authority remain final-lane validator responsibilities.",
      actorRole: reporterRole,
      actorSession: reporterSession,
      specAnchor: "CX-600",
      packetRowRef: "MERGE_AUTHORITY",
    };
  }

  if (authorityIssues.length > 0) {
    return {
      workflowInvalidityCode: "FINAL_LANE_AUTHORITY_VIOLATION",
      summary:
        "Final-lane closeout lacked governed Integration Validator authority proof; repair lane/session identity before updating packet or contained-main truth.",
      actorRole: reporterRole,
      actorSession: reporterSession,
      specAnchor: "CX-570",
      packetRowRef: "INTEGRATION_VALIDATOR_OF_RECORD",
    };
  }

  return null;
}

export function appendCloseoutSyncProvenance(gateState = {}, {
  wpId = "",
  event = null,
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId || !event || typeof event !== "object") {
    return gateState && typeof gateState === "object" ? { ...gateState } : {};
  }

  const nextState = gateState && typeof gateState === "object" ? { ...gateState } : {};
  const closeoutSyncEvents = normalizeCloseoutSyncEventMap(nextState.closeout_sync_events);
  const existingEvents = Array.isArray(closeoutSyncEvents[normalizedWpId]) ? closeoutSyncEvents[normalizedWpId] : [];
  nextState.closeout_sync_events = {
    ...closeoutSyncEvents,
    [normalizedWpId]: [...existingEvents, { ...event }],
  };
  return nextState;
}

export function latestCloseoutSyncEvent(gateState = {}, wpId = "") {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) return null;
  const closeoutSyncEvents = normalizeCloseoutSyncEventMap(gateState?.closeout_sync_events);
  return [...(closeoutSyncEvents[normalizedWpId] || [])]
    .sort((left, right) => String(right?.timestamp_utc || "").localeCompare(String(left?.timestamp_utc || "")))[0] || null;
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
  repoRoot = REPO_ROOT,
  wpId = "",
  packetContent = "",
  actorContext = {},
  committedEvidence = null,
  governanceRootAbs = "",
  gitRunner = null,
  worktreeExists = fs.existsSync,
} = {}) {
  const issues = makeIssueSet();
  const warnings = [];
  const authorityEvaluation = evaluateValidatorPassAuthority({
    packetContent,
    actorContext,
  });
  const authority = authorityEvaluation.authority || {};
  for (const issue of authorityEvaluation.issues || []) issues.add(issue);

  const actorRole = normalizeValidatorRole(actorContext?.actorRole);
  if (actorRole !== "INTEGRATION_VALIDATOR") {
    issues.add(`Closeout preflight requires the Integration Validator lane; current lane resolved to ${actorRole || "UNKNOWN"}.`);
  }

  const expectedBranch = defaultIntegrationValidatorBranch(wpId);
  const expectedWorktreeDir = normalizePath(defaultIntegrationValidatorWorktreeDir(wpId));
  const expectedWorktreeAbs = normalizePath(path.resolve(repoRoot, expectedWorktreeDir));
  const actorBranch = String(actorContext?.actorBranch || "").trim();
  const actorWorktreeDir = normalizePath(actorContext?.actorWorktreeDir || "");
  const liveGovernanceRootAbs = normalizePath(path.resolve(governanceRootAbs || GOV_ROOT_ABS || path.resolve(repoRoot, ".GOV")));
  const localMainGovernanceAbs = normalizePath(path.join(expectedWorktreeAbs, ".GOV"));

  if (actorBranch !== expectedBranch) {
    issues.add(`Integration Validator must run from branch ${expectedBranch}; current branch is ${actorBranch || "<unknown>"}.`);
  }
  if (!samePath(actorWorktreeDir, expectedWorktreeDir)) {
    issues.add(`Integration Validator must run from ${expectedWorktreeDir}; current worktree is ${actorWorktreeDir || "<unknown>"}.`);
  }
  if (authority.workflowLane === "ORCHESTRATOR_MANAGED" && samePath(liveGovernanceRootAbs, localMainGovernanceAbs)) {
    issues.add(
      `Integration Validator must resolve live governance from the kernel via HANDSHAKE_GOV_ROOT; current governance root still points at handshake_main/.GOV (${localMainGovernanceAbs}).`
    );
  }

  if (!committedEvidence || typeof committedEvidence !== "object") {
    issues.add("Committed handoff validation evidence is missing.");
    return {
      ok: false,
      issues: Array.from(issues),
      warnings,
      liveGovernanceRootAbs,
      localMainGovernanceAbs,
    };
  }
  const durableCommittedProof = committedEvidenceForCloseout(committedEvidence);
  const livePrepareHealth = livePrepareWorktreeHealthEvidence(committedEvidence);
  if (normalizeStatus(durableCommittedProof?.status) !== "PASS") {
    issues.add(`Committed handoff validation evidence must include a durable PASS proof (found ${durableCommittedProof?.status || "NONE"}).`);
  }
  if (livePrepareHealth && normalizeStatus(livePrepareHealth.status) !== "PASS") {
    warnings.push(
      `Live PREPARE worktree health is ${livePrepareHealth.status}; closeout may still proceed only because a prior committed target proof already passed.`,
    );
  }

  const targetHeadSha = String(durableCommittedProof?.target_head_sha || "").trim();
  if (!targetHeadSha) {
    issues.add("Committed handoff validation evidence is missing target_head_sha.");
    return {
      ok: false,
      issues: Array.from(issues),
      warnings,
      liveGovernanceRootAbs,
      localMainGovernanceAbs,
    };
  }

  const worktreeAbs = path.resolve(repoRoot, actorWorktreeDir || expectedWorktreeDir);
  if (!worktreeExists(worktreeAbs)) {
    issues.add(`Integration Validator worktree is unavailable: ${normalizePath(worktreeAbs)}`);
    return {
      ok: false,
      issues: Array.from(issues),
      warnings,
      liveGovernanceRootAbs,
      localMainGovernanceAbs,
    };
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
    liveGovernanceRootAbs,
    localMainGovernanceAbs,
    targetHeadSha,
    currentMainHeadSha,
    livePrepareHealth,
    durableCommittedProof,
  };
}

export function evaluateWpSessionControlCloseoutBundle({
  repoRoot = REPO_ROOT,
  wpId = "",
  requests = [],
  results = [],
  sessions = [],
  brokerState = null,
  actorContext = {},
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
  const actorRole = normalizeValidatorRole(actorContext?.actorRole);
  const actorSessionKey = String(actorContext?.actorSessionKey || "").trim();
  const selfActiveRuns = activeRuns.filter((run) =>
    actorRole === "INTEGRATION_VALIDATOR"
      && normalizeValidatorRole(run?.role) === actorRole
      && String(run?.session_key || "").trim() === actorSessionKey,
  );
  const blockingActiveRuns = activeRuns.filter((run) => !selfActiveRuns.includes(run));
  const selfActiveRunIds = new Set(
    selfActiveRuns
      .map((run) => String(run?.command_id || "").trim())
      .filter(Boolean),
  );

  const requestById = new Map();
  for (const request of wpRequests) requestById.set(String(request?.command_id || "").trim(), request);
  const resultById = new Map();
  for (const result of wpResults) resultById.set(String(result?.command_id || "").trim(), result);

  if (selfActiveRuns.length > 1) {
    issues.add(
      `Multiple self-owned Integration Validator broker runs still exist for ${wpId}: ${selfActiveRuns.map((run) => String(run?.command_id || "<missing>")).join(", ")}`
    );
  } else if (selfActiveRuns.length === 1) {
    warnings.push(
      `Closeout is executing inside the current Integration Validator broker run (${String(selfActiveRuns[0]?.command_id || "<missing>")}); treating that self-owned run as non-blocking.`
    );
  }

  if (blockingActiveRuns.length > 0) {
    issues.add(`Active broker runs still exist for ${wpId}: ${blockingActiveRuns.map((run) => String(run?.command_id || "<missing>")).join(", ")}`);
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
      if (selfActiveRunIds.has(commandId)) {
        warnings.push(`Self-owned closeout run ${commandId} has no settled result yet because the current final-lane command is still active.`);
        continue;
      }
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
      if (selfActiveRunIds.has(lastCommandId) && sessionKey === actorSessionKey) {
        warnings.push(`Session ${sessionKey} still reports RUNNING for self-owned closeout command ${lastCommandId}; tolerated while the current final-lane command is in flight.`);
        continue;
      }
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
      self_active_run_count: selfActiveRuns.length,
      blocking_active_run_count: blockingActiveRuns.length,
    },
  };
}

export function evaluateIntegrationValidatorCloseoutState({
  repoRoot = REPO_ROOT,
  wpId = "",
  packetContent = "",
  actorContext = {},
  committedEvidence = null,
  requireReadyForPass = true,
  requireRecordedScopeCompatibility = true,
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
    governanceRootAbs: GOV_ROOT_ABS,
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
    actorContext,
    fileExists,
  });
  const scopeCompatibility = validateSignedScopeCompatibilityTruth(packetContent, {
    packetPath: `<${wpId}>`,
    currentMainHeadSha: topology.currentMainHeadSha || "",
    requireReadyForPass,
  });
  const candidateSignedScope = topology.targetHeadSha
    ? validateCandidateTargetAgainstSignedScope(packetContent, {
      repoRoot,
      targetHeadSha: topology.targetHeadSha,
      currentMainHeadSha: topology.currentMainHeadSha || "",
      gitRunner,
    })
    : {
      ok: false,
      errors: ["candidate target validation requires committed target_head_sha"],
    };

  return {
    ok: topology.ok
      && closeoutBundle.ok
      && (!requireRecordedScopeCompatibility || scopeCompatibility.errors.length === 0)
      && candidateSignedScope.errors.length === 0,
    topology,
    closeoutBundle,
    scopeCompatibility,
    candidateSignedScope,
    issues: [
      ...topology.issues,
      ...closeoutBundle.issues,
      ...(requireRecordedScopeCompatibility ? scopeCompatibility.errors : []),
      ...candidateSignedScope.errors,
    ],
    warnings: [...topology.warnings, ...closeoutBundle.warnings],
  };
}
