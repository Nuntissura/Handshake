import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";
import crypto from "node:crypto";
import { GOV_ROOT_REPO_REL, GOVERNANCE_RUNTIME_ROOT_REPO_REL, resolveOrchestratorGatesPath, resolveWorkPacketPath } from "./runtime-paths.mjs";
import { executionOwnerToPacketValue } from "../session/session-policy.mjs";

export const ORCHESTRATOR_GATES_PATH = resolveOrchestratorGatesPath();
export const TASK_BOARD_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "records", "TASK_BOARD.md");
export const TERMINAL_TASK_BOARD_STATUSES = ["VALIDATED", "FAIL", "OUTDATED_ONLY", "SUPERSEDED"];
export const IMPLICIT_ORCHESTRATOR_RESUME_LOOKBACK_HOURS = 168;
export const ACTIVE_ORCHESTRATOR_TASK_BOARD_STATUSES = ["READY_FOR_DEV", "IN_PROGRESS", "BLOCKED", "MERGE_PENDING"];

function safeExec(command) {
  try {
    return execSync(command, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
  } catch {
    return "";
  }
}

function safeExecInDir(cwd, command) {
  try {
    return execSync(command, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "";
  }
}

export function exists(filePath) {
  try {
    return fs.existsSync(filePath);
  } catch {
    return false;
  }
}

export function readUtf8(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

export function loadJson(filePath, fallback = {}) {
  if (!exists(filePath)) return fallback;
  try {
    return JSON.parse(readUtf8(filePath));
  } catch {
    return fallback;
  }
}

export function currentGitContext() {
  return {
    branch: safeExec("git rev-parse --abbrev-ref HEAD"),
    topLevel: safeExec("git rev-parse --show-toplevel"),
    statusShort: safeExec("git status -sb"),
    statusPorcelain: safeExec("git status --porcelain=v1"),
  };
}

export function inferWpIdFromBranch(branch) {
  const value = String(branch || "").trim();
  if (!value) return null;

  const patterns = [
    /^feat\/(WP-[A-Za-z0-9][A-Za-z0-9._-]*)$/,
    /^(WP-[A-Za-z0-9][A-Za-z0-9._-]*)$/,
  ];

  for (const pattern of patterns) {
    const match = value.match(pattern);
    if (match) return match[1];
  }

  return null;
}

export function packetPath(wpId) {
  return packetPathAtRepo(wpId);
}

export function packetPathAtRepo(wpId, referenceRepoRoot = "") {
  const packetPathRel = resolveWorkPacketPath(wpId)?.packetPath || path.join(GOV_ROOT_REPO_REL, "task_packets", `${wpId}.md`);
  return referenceRepoRoot ? path.join(referenceRepoRoot, packetPathRel) : packetPathRel;
}

export function packetExists(wpId) {
  return packetExistsAtRepo(wpId);
}

export function packetExistsAtRepo(wpId, referenceRepoRoot = "") {
  return exists(packetPathAtRepo(wpId, referenceRepoRoot));
}

export function loadPacket(wpId) {
  return loadPacketAtRepo(wpId);
}

export function loadPacketAtRepo(wpId, referenceRepoRoot = "") {
  const filePath = packetPathAtRepo(wpId, referenceRepoRoot);
  return exists(filePath) ? readUtf8(filePath) : "";
}

export function parseStatus(packetContent) {
  const match =
    packetContent.match(/^\s*-\s*\*\*Status:\*\*[ \t]*([^\r\n]+)[ \t]*$/mi) ||
    packetContent.match(/^\s*\*\*Status:\*\*[ \t]*([^\r\n]+)[ \t]*$/mi) ||
    packetContent.match(/^\s*Status:[ \t]*([^\r\n]+)[ \t]*$/mi);
  return match ? match[1].trim() : "";
}

export function parseCurrentWpStatus(packetContent) {
  const match = packetContent.match(/^\s*-\s*Current WP_STATUS:[ \t]*([^\r\n]*)[ \t]*$/mi);
  return match ? match[1].trim() : "";
}

export function parseMergeBaseSha(packetContent) {
  const match = packetContent.match(/^\s*-\s*MERGE_BASE_SHA\s*:\s*([a-f0-9]{40})\s*$/mi);
  return match ? match[1].trim() : "";
}

export function parseClaimField(packetContent, label) {
  const re = new RegExp(`^\\s*-\\s*${label}\\s*:[ \\t]*([^\\r\\n]+)[ \\t]*$`, "mi");
  const match = packetContent.match(re);
  return match ? match[1].trim() : "";
}

export function sectionBody(packetContent, heading) {
  const lines = packetContent.split(/\r?\n/);
  const headingIndex = lines.findIndex((line) =>
    new RegExp(`^#{2,6}\\s+${heading.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`, "i").test(line),
  );
  if (headingIndex === -1) return "";

  let endIndex = lines.length;
  for (let index = headingIndex + 1; index < lines.length; index += 1) {
    if (/^#{1,6}\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }

  return lines.slice(headingIndex + 1, endIndex).join("\n").trim();
}

export function sectionHasMaterialContent(packetContent, heading) {
  const body = sectionBody(packetContent, heading);
  if (!body) return false;

  const meaningfulLines = body
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .filter((line) => !/^[-*]\s*\(/.test(line))
    .filter((line) => !/^<!--/.test(line))
    .filter((line) => !/^(Coder|Validator)\s+fills?/i.test(line))
    .filter((line) => !/^N\/A$/i.test(line));

  return meaningfulLines.length > 0;
}

export function buildPostWorkCommand(wpId, packetContent) {
  const mergeBaseSha = parseMergeBaseSha(packetContent);
  if (mergeBaseSha) return `just post-work ${wpId} --range ${mergeBaseSha}..HEAD`;
  return `just post-work ${wpId}`;
}

export function hasCommitSubject(pattern) {
  const result = safeExec(`git log -n 1 --format=%H --grep="${pattern}"`);
  return Boolean(result);
}

export function escapeRegex(value) {
  return String(value || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function loadOrchestratorGateLogs() {
  const state = loadJson(ORCHESTRATOR_GATES_PATH, { gate_logs: [] });
  return Array.isArray(state.gate_logs) ? state.gate_logs : [];
}

export function lastGateLog(logs, wpId, type) {
  return [...logs].reverse().find((entry) => entry?.wpId === wpId && entry?.type === type) || null;
}

export function inferWpIdFromPrepare(logs, gitContext) {
  const currentBranchWp = inferWpIdFromBranch(gitContext.branch);
  if (currentBranchWp) return { wpId: currentBranchWp, source: "branch", candidates: [currentBranchWp] };

  const matches = new Set();

  for (const entry of logs) {
    if (!entry?.wpId || !String(entry.wpId).startsWith("WP-")) continue;
    if (isTerminalTaskBoardStatus(taskBoardStatus(entry.wpId))) continue;

    const entryBranch = String(entry.branch || "").trim();
    if (entryBranch && gitContext.branch && entryBranch === gitContext.branch) {
      matches.add(entry.wpId);
    }

    const entryWorktree = String(entry.worktree_dir || "").trim();
    if (entryWorktree && gitContext.topLevel) {
      const currentAbs = path.resolve(gitContext.topLevel);
      const expectedAbs = path.isAbsolute(entryWorktree)
        ? path.resolve(entryWorktree)
        : path.resolve(gitContext.topLevel, entryWorktree);
      if (currentAbs.toLowerCase() === expectedAbs.toLowerCase()) {
        matches.add(entry.wpId);
      }
    }
  }

  const candidates = [...matches];
  if (candidates.length === 1) {
    return { wpId: candidates[0], source: "prepare", candidates };
  }

  return { wpId: null, source: "prepare", candidates };
}

export function taskBoardStatus(wpId) {
  if (!exists(TASK_BOARD_PATH)) return "";
  const content = readUtf8(TASK_BOARD_PATH);
  const match = content.match(
    new RegExp(`- \\*\\*\\[${escapeRegex(wpId)}\\]\\*\\* - \\[([^\\]]+)\\]`, "i"),
  );
  return match ? match[1].trim().toUpperCase() : "";
}

export function isTerminalTaskBoardStatus(status) {
  return TERMINAL_TASK_BOARD_STATUSES.includes(String(status || "").trim().toUpperCase());
}

function isRecentImplicitResumeTimestamp(timestamp) {
  const parsed = Date.parse(String(timestamp || ""));
  if (Number.isNaN(parsed)) return false;
  const ageHours = Math.max(0, (Date.now() - parsed) / (1000 * 60 * 60));
  return ageHours <= IMPLICIT_ORCHESTRATOR_RESUME_LOOKBACK_HOURS;
}

export function taskBoardEntries() {
  if (!exists(TASK_BOARD_PATH)) return [];
  return taskBoardEntriesAtRepo();
}

export function taskBoardEntriesAtRepo(repoRoot = "") {
  const taskBoardPath = repoRoot
    ? path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "records", "TASK_BOARD.md")
    : TASK_BOARD_PATH;
  if (!exists(taskBoardPath)) return [];
  const entries = [];
  const content = readUtf8(taskBoardPath);
  const pattern = /^- \*\*\[(WP-[^\]]+)\]\*\* - \[([^\]]+)\]/gm;
  let match = pattern.exec(content);
  while (match) {
    entries.push({
      wpId: match[1].trim(),
      status: match[2].trim().toUpperCase(),
    });
    match = pattern.exec(content);
  }
  return entries;
}

function taskBoardStatusAtRepo(repoRoot, wpId) {
  const taskBoardPath = path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "records", "TASK_BOARD.md");
  if (!exists(taskBoardPath)) return "";
  const content = readUtf8(taskBoardPath);
  const match = content.match(
    new RegExp(`- \\*\\*\\[${escapeRegex(wpId)}\\]\\*\\* - \\[([^\\]]+)\\]`, "i"),
  );
  return match ? match[1].trim().toUpperCase() : "";
}

function traceabilityPacketPathAtRepo(repoRoot, baseWpId) {
  const traceabilityPath = path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "records", "WP_TRACEABILITY_REGISTRY.md");
  if (!exists(traceabilityPath)) return "";
  const content = readUtf8(traceabilityPath);
  const lines = content.split(/\r?\n/);
  for (const line of lines) {
    if (!line.trimStart().startsWith("|")) continue;
    if (line.includes("Base WP ID") || line.includes("---")) continue;
    const cols = line.split("|").slice(1, -1).map((cell) => cell.trim());
    if (cols.length < 2) continue;
    if (cols[0] === baseWpId) return cols[1];
  }
  return "";
}

function resolveSpecSnapshotAtRepo(repoRoot) {
  const specCurrentPath = path.join(repoRoot, GOV_ROOT_REPO_REL, "spec", "SPEC_CURRENT.md");
  if (!exists(specCurrentPath)) {
    return { ok: false, error: `Missing ${specCurrentPath}` };
  }
  const specCurrent = readUtf8(specCurrentPath);
  const match = specCurrent.match(/Handshake_Master_Spec_v[0-9._]+\.md/);
  if (!match) {
    return { ok: false, error: `Could not resolve spec filename from ${specCurrentPath}` };
  }
  const specFileName = match[0];
  const specFilePath = path.join(path.dirname(specCurrentPath), specFileName);
  if (!exists(specFilePath)) {
    return { ok: false, error: `Resolved spec file does not exist: ${specFilePath}` };
  }
  const sha1 = crypto.createHash("sha1").update(fs.readFileSync(specFilePath)).digest("hex");
  return { ok: true, specFileName, sha1 };
}

function lastPrepareEntryAtRepo(repoRoot, wpId) {
  const gatesPath = path.join(repoRoot, GOVERNANCE_RUNTIME_ROOT_REPO_REL, "roles_shared", "ORCHESTRATOR_GATES.json");
  if (!exists(gatesPath)) return null;
  let state = {};
  try {
    state = JSON.parse(readUtf8(gatesPath));
  } catch {
    return null;
  }
  const logs = Array.isArray(state.gate_logs) ? state.gate_logs : [];
  return [...logs].reverse().find((entry) => entry?.wpId === wpId && entry?.type === "PREPARE") || null;
}

export function resolvePrepareWorktreeAbs(prepareEntry, referenceRepoRoot) {
  const worktreeDir = String(prepareEntry?.worktree_dir || "").trim();
  if (!worktreeDir) return "";
  return path.isAbsolute(worktreeDir)
    ? path.resolve(worktreeDir)
    : path.resolve(referenceRepoRoot || process.cwd(), worktreeDir);
}

function isPendingAuthorityValue(value) {
  const normalized = String(value || "").trim();
  return normalized === "" || normalized === "<pending>" || normalized === "<missing>";
}

export function normalizeComparableRepoPath(value, referenceRepoRoot) {
  const normalized = String(value || "").trim();
  if (!normalized || isPendingAuthorityValue(normalized)) return "";
  const absolute = path.isAbsolute(normalized)
    ? path.resolve(normalized)
    : path.resolve(referenceRepoRoot || currentGitContext().topLevel || process.cwd(), normalized);
  return process.platform === "win32" ? absolute.toLowerCase() : absolute;
}

export function comparePrepareAgainstPacketTruth(packetContent, prepareEntry, referenceRepoRoot) {
  const workflowLane = parseClaimField(packetContent, "WORKFLOW_LANE");
  const executionOwner = parseClaimField(packetContent, "EXECUTION_OWNER");
  const localBranch = parseClaimField(packetContent, "LOCAL_BRANCH");
  const localWorktreeDir = parseClaimField(packetContent, "LOCAL_WORKTREE_DIR");
  const prepareWorkflowLane = String(prepareEntry?.workflow_lane || "").trim();
  const prepareExecutionOwner = String(prepareEntry?.execution_lane || prepareEntry?.coder_id || "").trim();
  const normalizedPrepareExecutionOwner = executionOwnerToPacketValue(prepareExecutionOwner) || prepareExecutionOwner;
  const prepareBranch = String(prepareEntry?.branch || "").trim();
  const prepareWorktreeDir = String(prepareEntry?.worktree_dir || "").trim();
  const issues = [];

  if (!isPendingAuthorityValue(workflowLane) && prepareWorkflowLane && workflowLane !== prepareWorkflowLane) {
    issues.push(`Official packet WORKFLOW_LANE conflicts with PREPARE: expected ${workflowLane}, got ${prepareWorkflowLane}`);
  }
  if (!isPendingAuthorityValue(executionOwner) && prepareExecutionOwner && executionOwner !== normalizedPrepareExecutionOwner) {
    issues.push(`Official packet EXECUTION_OWNER conflicts with PREPARE: expected ${executionOwner}, got ${prepareExecutionOwner}`);
  }
  if (!isPendingAuthorityValue(localBranch) && prepareBranch && localBranch !== prepareBranch) {
    issues.push(`Official packet LOCAL_BRANCH conflicts with PREPARE: expected ${localBranch}, got ${prepareBranch}`);
  }

  const packetWorktreeAbs = normalizeComparableRepoPath(localWorktreeDir, referenceRepoRoot);
  const prepareWorktreeAbs = normalizeComparableRepoPath(prepareWorktreeDir, referenceRepoRoot);
  if (packetWorktreeAbs && prepareWorktreeAbs && packetWorktreeAbs !== prepareWorktreeAbs) {
    issues.push(`Official packet LOCAL_WORKTREE_DIR conflicts with PREPARE: expected ${localWorktreeDir}, got ${prepareWorktreeDir}`);
  }

  return {
    ok: issues.length === 0,
    workflowLane,
    executionOwner,
    localBranch,
    localWorktreeDir,
    prepareWorkflowLane,
    prepareExecutionOwner,
    prepareBranch,
    prepareWorktreeDir,
    issues,
  };
}

export function preparePacketTruthState(wpId, prepareEntry, referenceRepoRoot) {
  const filePath = packetPathAtRepo(wpId, referenceRepoRoot);
  const packetContent = loadPacketAtRepo(wpId, referenceRepoRoot);
  if (!packetContent) {
    return {
      ok: true,
      wpId,
      packetPresent: false,
      packetPath: filePath,
      issues: [],
    };
  }

  const comparison = comparePrepareAgainstPacketTruth(packetContent, prepareEntry, referenceRepoRoot);
  return {
    ...comparison,
    wpId,
    packetPresent: true,
    packetPath: filePath,
  };
}

export function preparedWorktreeSyncState(wpId, prepareEntry, referenceRepoRoot) {
  const repoRoot = referenceRepoRoot || currentGitContext().topLevel || process.cwd();
  const worktreeAbs = resolvePrepareWorktreeAbs(prepareEntry, repoRoot);
  const expectedBranch = String(prepareEntry?.branch || "").trim();
  const issues = [];

  if (!worktreeAbs) {
    issues.push("PREPARE is missing worktree_dir");
    return { ok: false, repoRoot, worktreeAbs: "", expectedBranch, issues };
  }
  if (!exists(worktreeAbs)) {
    issues.push(`Assigned worktree does not exist: ${worktreeAbs}`);
    return { ok: false, repoRoot, worktreeAbs, expectedBranch, issues };
  }

  const actualBranch = safeExecInDir(worktreeAbs, "git rev-parse --abbrev-ref HEAD");
  if (expectedBranch && actualBranch && expectedBranch !== actualBranch) {
    issues.push(`Assigned worktree branch mismatch: expected ${expectedBranch}, got ${actualBranch}`);
  }

  const resolvedPacket = resolveWorkPacketPath(wpId);
  const packetPathRel = resolvedPacket?.packetPath || path.join(GOV_ROOT_REPO_REL, "task_packets", `${wpId}.md`);
  const packetPath = path.join(worktreeAbs, packetPathRel);
  const referencePacketPath = path.join(repoRoot, packetPathRel);
  if (!exists(packetPath)) {
    issues.push(`Assigned worktree is missing the official packet: ${packetPath}`);
  } else if (exists(referencePacketPath)) {
    const referencePacket = fs.readFileSync(referencePacketPath, "utf8");
    const worktreePacket = fs.readFileSync(packetPath, "utf8");
    if (referencePacket !== worktreePacket) {
      issues.push("Assigned worktree official packet content is stale relative to the current orchestrator state");
    }
  }

  const currentPrepare = lastPrepareEntryAtRepo(repoRoot, wpId);
  const worktreePrepare = lastPrepareEntryAtRepo(worktreeAbs, wpId);
  // ORCHESTRATOR_GATES.json is external governance runtime state excluded from
  // gov-to-main sync, so WP worktrees legitimately lack it — only flag a mismatch
  // when the worktree *does* have a PREPARE record that disagrees with the reference.
  if (!worktreePrepare && !currentPrepare) {
    issues.push("Assigned worktree does not contain the current PREPARE record");
  } else if (
    worktreePrepare
    && currentPrepare
    && (
      String(worktreePrepare.branch || "").trim() !== String(currentPrepare.branch || "").trim()
      || String(worktreePrepare.worktree_dir || "").trim() !== String(currentPrepare.worktree_dir || "").trim()
      || String(worktreePrepare.coder_id || "").trim() !== String(currentPrepare.coder_id || "").trim()
    )
  ) {
    issues.push("Assigned worktree PREPARE record does not match current orchestrator gate state");
  }

  const referenceSpec = resolveSpecSnapshotAtRepo(repoRoot);
  const worktreeSpec = resolveSpecSnapshotAtRepo(worktreeAbs);
  if (!referenceSpec.ok) {
    issues.push(referenceSpec.error);
  } else if (!worktreeSpec.ok) {
    issues.push(worktreeSpec.error);
  } else if (
    referenceSpec.specFileName !== worktreeSpec.specFileName
    || referenceSpec.sha1 !== worktreeSpec.sha1
  ) {
    issues.push(
      `Assigned worktree SPEC_CURRENT snapshot is stale: expected ${referenceSpec.specFileName} @ ${referenceSpec.sha1}, got ${worktreeSpec.specFileName} @ ${worktreeSpec.sha1}`,
    );
  }

  const referenceBoardStatus = taskBoardStatusAtRepo(repoRoot, wpId);
  const worktreeBoardStatus = taskBoardStatusAtRepo(worktreeAbs, wpId);
  if (referenceBoardStatus && referenceBoardStatus !== worktreeBoardStatus) {
    issues.push(`Assigned worktree TASK_BOARD status is stale: expected ${referenceBoardStatus}, got ${worktreeBoardStatus || "<missing>"}`);
  }

  const baseWpId = String(wpId || "").replace(/-v\d+$/i, "");
  const referenceTraceabilityPath = traceabilityPacketPathAtRepo(repoRoot, baseWpId);
  const worktreeTraceabilityPath = traceabilityPacketPathAtRepo(worktreeAbs, baseWpId);
  if (referenceTraceabilityPath && referenceTraceabilityPath !== worktreeTraceabilityPath) {
    issues.push(`Assigned worktree traceability mapping is stale: expected ${referenceTraceabilityPath}, got ${worktreeTraceabilityPath || "<missing>"}`);
  }

  return {
    ok: issues.length === 0,
    repoRoot,
    worktreeAbs,
    expectedBranch,
    actualBranch,
    referenceBoardStatus,
    worktreeBoardStatus,
    referenceTraceabilityPath,
    worktreeTraceabilityPath,
    referenceSpec,
    worktreeSpec,
    issues,
  };
}

export function activeOrchestratorCandidates(logs) {
  const latestByWp = new Map();

  for (const entry of logs) {
    const wpId = String(entry?.wpId || "").trim();
    if (!wpId.startsWith("WP-")) continue;
    latestByWp.set(wpId, entry);
  }

  return [...latestByWp.values()]
    .filter((entry) => {
      const status = taskBoardStatus(entry.wpId);
      if (isTerminalTaskBoardStatus(status)) return false;
      return isRecentImplicitResumeTimestamp(entry?.timestamp);
    })
    .sort((left, right) => String(right.timestamp || "").localeCompare(String(left.timestamp || "")));
}

function uniqueSorted(values) {
  return [...new Set(values.filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

export function workflowStartReadinessState({ repoRoot, gateLogs } = {}) {
  const resolvedRepoRoot = repoRoot || currentGitContext().topLevel || process.cwd();
  const logs = Array.isArray(gateLogs) ? gateLogs : loadOrchestratorGateLogs();
  const activeBoardEntries = taskBoardEntriesAtRepo(resolvedRepoRoot)
    .filter((entry) => ACTIVE_ORCHESTRATOR_TASK_BOARD_STATUSES.includes(String(entry.status || "").trim().toUpperCase()));
  const activeBoardWpIds = uniqueSorted(activeBoardEntries.map((entry) => entry.wpId));
  const activeCandidateWpIds = uniqueSorted(activeOrchestratorCandidates(logs).map((entry) => entry.wpId));
  const candidateWpIdSet = new Set(activeCandidateWpIds);
  const wpIds = uniqueSorted([...activeBoardWpIds, ...activeCandidateWpIds]);
  const violations = [];

  for (const wpId of wpIds) {
    const boardStatus = taskBoardStatusAtRepo(resolvedRepoRoot, wpId) || "<none>";
    const prepareEntry = lastGateLog(logs, wpId, "PREPARE");
    const hasPacket = packetExistsAtRepo(wpId, resolvedRepoRoot);
    const boardSaysActive = ACTIVE_ORCHESTRATOR_TASK_BOARD_STATUSES.includes(boardStatus);
    const candidateSaysActive = candidateWpIdSet.has(wpId);
    const requiresPreparedAuthority = boardStatus === "IN_PROGRESS" || candidateSaysActive;

    if (boardSaysActive && !hasPacket) {
      violations.push(`${wpId}: TASK_BOARD is ${boardStatus}, but the official packet is missing`);
    }
    if (requiresPreparedAuthority && !prepareEntry) {
      violations.push(`${wpId}: TASK_BOARD is ${boardStatus}, but no PREPARE authority exists in ORCHESTRATOR_GATES`);
    }
    if (prepareEntry && !boardSaysActive && candidateSaysActive) {
      violations.push(`${wpId}: PREPARE authority exists, but TASK_BOARD is ${boardStatus} instead of READY_FOR_DEV|IN_PROGRESS|BLOCKED`);
    }
    if (!prepareEntry || !requiresPreparedAuthority) continue;

    const packetTruth = preparePacketTruthState(wpId, prepareEntry, resolvedRepoRoot);
    if (!packetTruth.ok) {
      for (const issue of packetTruth.issues) {
        violations.push(`${wpId}: ${issue}`);
      }
    }

    if (!packetTruth.packetPresent) {
      const worktreeAbs = resolvePrepareWorktreeAbs(prepareEntry, resolvedRepoRoot);
      if (!worktreeAbs) {
        violations.push(`${wpId}: PREPARE is missing worktree_dir`);
      } else if (!exists(worktreeAbs)) {
        violations.push(`${wpId}: PREPARE points to a missing worktree: ${worktreeAbs}`);
      }
      continue;
    }

    const syncState = preparedWorktreeSyncState(wpId, prepareEntry, resolvedRepoRoot);
    if (!syncState.ok) {
      for (const issue of syncState.issues) {
        violations.push(`${wpId}: ${issue}`);
      }
    }
  }

  return {
    ok: violations.length === 0,
    repoRoot: resolvedRepoRoot,
    activeBoardWpIds,
    activeCandidateWpIds,
    checkedWps: wpIds.length,
    wpIds,
    violations,
  };
}

export function inferOrchestratorWpId(logs, gitContext) {
  const fromPrepare = inferWpIdFromPrepare(logs, gitContext);
  if (fromPrepare.wpId) return { wpId: fromPrepare.wpId, source: fromPrepare.source, candidates: fromPrepare.candidates };

  const candidates = activeOrchestratorCandidates(logs);
  if (candidates.length === 1) {
    return { wpId: candidates[0].wpId, source: "latest-active", candidates: [candidates[0].wpId] };
  }

  return { wpId: null, source: "latest-active", candidates: candidates.map((entry) => entry.wpId) };
}

export function printLifecycle({ wpId, stage, next }) {
  console.log("LIFECYCLE [CX-LIFE-001]");
  console.log(`- WP_ID: ${wpId}`);
  console.log(`- STAGE: ${stage}`);
  console.log(`- NEXT: ${next}`);
  console.log("");
}

export function printOperatorAction(action) {
  console.log(`OPERATOR_ACTION: ${action || "NONE"}`);
  console.log("");
}

export function printConfidence(confidence, detail = "") {
  if (!confidence) return;
  console.log(`CONFIDENCE: ${confidence}${detail ? ` (${detail})` : ""}`);
  console.log("");
}

export function printState(state) {
  console.log(`STATE: ${state}`);
  console.log("");
}

export function printFindings(lines = []) {
  if (!lines.length) return;
  console.log("FINDINGS:");
  for (const line of lines) console.log(`- ${line}`);
  console.log("");
}

export function printVerdict(verdict) {
  if (!verdict) return;
  console.log(`VERDICT: ${verdict}`);
  console.log("");
}

export function printNextCommands(commands = []) {
  console.log("NEXT_COMMANDS [CX-GATE-UX-001]");
  for (const command of commands) console.log(`- ${command}`);
}

export function failWithContext({
  wpId = "N/A",
  stage = "BOOTSTRAP",
  next = "STOP",
  operatorAction = "NONE",
  confidence = "LOW",
  confidenceDetail = "",
  state,
  findings = [],
  nextCommands = [],
}) {
  printLifecycle({ wpId, stage, next });
  printOperatorAction(operatorAction);
  printConfidence(confidence, confidenceDetail);
  printState(state);
  printFindings(findings);
  printNextCommands(nextCommands);
  process.exit(1);
}

export function normalizeVerdict(value) {
  const verdict = String(value || "").trim().toUpperCase();
  if (verdict === "PASS" || verdict === "FAIL" || verdict === "PENDING") return verdict;
  return "PENDING";
}
