import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

export const ORCHESTRATOR_GATES_PATH = path.join(
  ".GOV",
  "roles",
  "orchestrator",
  "ORCHESTRATOR_GATES.json",
);
export const TASK_BOARD_PATH = path.join(".GOV", "roles_shared", "TASK_BOARD.md");
export const TERMINAL_TASK_BOARD_STATUSES = ["VALIDATED", "FAIL", "OUTDATED_ONLY", "SUPERSEDED"];

function safeExec(command) {
  try {
    return execSync(command, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
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
  return path.join(".GOV", "task_packets", `${wpId}.md`);
}

export function packetExists(wpId) {
  return exists(packetPath(wpId));
}

export function loadPacket(wpId) {
  const filePath = packetPath(wpId);
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

export function taskBoardEntries() {
  if (!exists(TASK_BOARD_PATH)) return [];
  const entries = [];
  const content = readUtf8(TASK_BOARD_PATH);
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
      return !isTerminalTaskBoardStatus(status);
    })
    .sort((left, right) => String(right.timestamp || "").localeCompare(String(left.timestamp || "")));
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
