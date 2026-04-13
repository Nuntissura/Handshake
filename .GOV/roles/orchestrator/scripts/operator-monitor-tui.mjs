#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import readline from "node:readline";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { appendWpThreadEntry } from "../../../roles_shared/scripts/wp/wp-thread-append.mjs";
import { checkAllNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import {
  communicationMonitorState,
  evaluateWpCommunicationHealth,
} from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../../../roles_shared/scripts/lib/wp-relay-escalation-lib.mjs";
import {
  normalize,
  parseJsonFile as sharedParseJsonFile,
  parseJsonlFile as sharedParseJsonlFile,
  REVIEW_TRACKED_RECEIPT_KIND_VALUES,
  ROUTABLE_ROLE_VALUES,
  validateReceipt,
  validateRuntimeStatus,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { loadSessionRegistry, registryBatchLaunchSummary, registrySessionSummary } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { resolveValidatorGatePath } from "../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import {
  SESSION_CONTROL_BROKER_STATE_FILE,
  SESSION_CONTROL_REQUESTS_FILE,
  SESSION_CONTROL_RESULTS_FILE,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import { readWpTokenUsageLedger } from "../../../roles_shared/scripts/session/wp-token-usage-lib.mjs";
import { evaluateWpTokenBudget } from "../../../roles_shared/scripts/session/wp-token-budget-lib.mjs";
import { TOPOLOGY_REGISTRY_JSON_PATH } from "../../../roles_shared/scripts/topology/git-topology-lib.mjs";
import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  repoPathAbs,
  inferWpIdFromPacketPath,
  resolveOrchestratorGatesPath,
  resolveRefinementPath,
  resolveWorkPacketPath,
  WORK_PACKET_STORAGE_ROOT_REPO_REL,
  WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const TASK_BOARD_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`;
const TRACEABILITY_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`;
const TOPOLOGY_PATH = TOPOLOGY_REGISTRY_JSON_PATH;
const ORCHESTRATOR_GATES_PATH = resolveOrchestratorGatesPath();
const SESSION_CONTROL_REQUESTS_PATH = SESSION_CONTROL_REQUESTS_FILE;
const SESSION_CONTROL_RESULTS_PATH = SESSION_CONTROL_RESULTS_FILE;
const SESSION_CONTROL_BROKER_STATE_PATH = SESSION_CONTROL_BROKER_STATE_FILE;
const PACKETS_DIR = WORK_PACKET_STORAGE_ROOT_REPO_REL;
const PACKET_STUBS_DIR = WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL;

const FILTERS = ["ALL", "ACTIVE", "READY_FOR_DEV", "IN_PROGRESS", "BLOCKED", "STUB", "DONE", "SUPERSEDED"];
const DETAIL_VIEWS = ["OVERVIEW", "DOCS", "COMMS", "SESSIONS", "TIMELINE", "CONTROL", "EVENTS"];
const BOARD_ORDER = ["ACTIVE", "READY_FOR_DEV", "IN_PROGRESS", "BLOCKED", "STUB", "DONE", "SUPERSEDED", "OTHER"];
const REFRESH_INTERVAL_MS = 1000;
const ACTIVE_RUNTIME_STATES = new Set([
  "PLUGIN_REQUESTED",
  "TERMINAL_COMMAND_DISPATCHED",
  "PLUGIN_CONFIRMED",
  "CLI_ESCALATION_READY",
  "CLI_ESCALATION_USED",
  "STARTING",
  "COMMAND_RUNNING",
  "ACTIVE",
  "WAITING",
]);
const ACTIVE_PACKET_SESSION_STATES = new Set([
  "assigned",
  "queued",
  "starting",
  "running",
  "working",
  "waiting",
  "blocked",
]);
const ANSI_ESCAPE_RE = /\x1b\[[0-9;]*m/g;
const STATUS_COLORS = {
  ACTIVE: "\x1b[38;5;81m",
  READY_FOR_DEV: "\x1b[38;5;117m",
  IN_PROGRESS: "\x1b[38;5;190m",
  BLOCKED: "\x1b[38;5;203m",
  STUB: "\x1b[38;5;244m",
  DONE: "\x1b[38;5;114m",
  SUPERSEDED: "\x1b[38;5;245m",
  OTHER: "\x1b[38;5;252m",
};
const ROLE_COLORS = {
  ORCHESTRATOR: "\x1b[38;5;81m",
  CODER: "\x1b[38;5;114m",
  WP_VALIDATOR: "\x1b[38;5;220m",
  INTEGRATION_VALIDATOR: "\x1b[38;5;111m",
  VALIDATOR: "\x1b[38;5;222m",
};
const STATE_COLORS = {
  working: "\x1b[38;5;114m",
  waiting: "\x1b[38;5;220m",
  blocked: "\x1b[38;5;203m",
  completed: "\x1b[38;5;81m",
  idle: "\x1b[38;5;244m",
  ready: "\x1b[38;5;81m",
  starting: "\x1b[38;5;117m",
  running: "\x1b[38;5;114m",
  failed: "\x1b[38;5;203m",
  unstarted: "\x1b[38;5;244m",
  none: "\x1b[38;5;240m",
};
const FILE_CACHE = new Map();
const CURRENT_BRANCH_CACHE_TTL_MS = 5000;
let currentBranchCache = { value: "", expiresAt: 0 };
const CURRENT_WORKTREE_DIR = REPO_ROOT;

function stripAnsi(value) {
  return String(value ?? "").replace(ANSI_ESCAPE_RE, "");
}

function visibleLength(value) {
  return stripAnsi(value).length;
}

function sliceAnsi(value, width) {
  const source = String(value ?? "");
  let index = 0;
  let visible = 0;
  let result = "";
  while (index < source.length && visible < width) {
    if (source[index] === "\x1b") {
      const match = source.slice(index).match(/^\x1b\[[0-9;]*m/);
      if (match) {
        result += match[0];
        index += match[0].length;
        continue;
      }
    }
    result += source[index];
    index += 1;
    visible += 1;
  }
  if (result.includes("\x1b[")) result += "\x1b[0m";
  return result;
}

function paint(text, color, options = {}) {
  if (!process.stdout.isTTY) return String(text ?? "");
  const prefix = `${options.bold ? "\x1b[1m" : ""}${options.dim ? "\x1b[2m" : ""}${color || ""}`;
  return `${prefix}${text}\x1b[0m`;
}

function truncateVisible(value, width) {
  const text = String(value ?? "");
  if (width <= 0) return "";
  if (visibleLength(text) <= width) return `${text}${" ".repeat(Math.max(0, width - visibleLength(text)))}`;
  if (width === 1) return sliceAnsi(text, 1);
  return `${sliceAnsi(text, Math.max(0, width - 3))}...`;
}

function resolveMonitorPath(filePath) {
  if (!filePath) return "";
  return path.isAbsolute(filePath) ? path.resolve(filePath) : repoPathAbs(filePath);
}

function fileSignature(filePath) {
  const absolutePath = resolveMonitorPath(filePath);
  if (!absolutePath || !fs.existsSync(absolutePath)) return "missing";
  const stats = fs.statSync(absolutePath);
  return `${stats.size}:${stats.mtimeMs}`;
}

function readCachedFile(kind, filePath, loader) {
  const absolutePath = resolveMonitorPath(filePath);
  const key = `${kind}:${normalize(absolutePath || filePath)}`;
  const signature = fileSignature(absolutePath);
  const cached = FILE_CACHE.get(key);
  if (cached && cached.signature === signature) {
    return cached.value;
  }
  const value = loader(absolutePath);
  FILE_CACHE.set(key, { signature, value });
  return value;
}

function readText(filePath) {
  return readCachedFile("text", filePath, (absolutePath) => fs.readFileSync(absolutePath, "utf8"));
}

function parseJsonFile(filePath) {
  return readCachedFile("json", filePath, (absolutePath) => sharedParseJsonFile(absolutePath));
}

function parseJsonlFile(filePath) {
  return readCachedFile("jsonl", filePath, (absolutePath) => sharedParseJsonlFile(absolutePath));
}

function isProcessAlive(pid) {
  const numeric = Number(pid || 0);
  if (!Number.isInteger(numeric) || numeric <= 0) return false;
  try {
    process.kill(numeric, 0);
    return true;
  } catch {
    return false;
  }
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function parsePacketStatus(text) {
  return (
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    ""
  ).trim() || "Ready for Dev";
}

function parseArgs(argv) {
  const options = {
    once: false,
    admin: false,
    actorRole: "OPERATOR",
    actorSession: "operator-viewport",
    wpId: "",
    filter: "ACTIVE",
    detailView: "OVERVIEW",
    refreshMs: REFRESH_INTERVAL_MS,
  };
  const args = [...argv];
  while (args.length > 0) {
    const token = args.shift();
    if (token === "--once") {
      options.once = true;
    } else if (token === "--admin") {
      options.admin = true;
    } else if (token === "--actor-role") {
      options.actorRole = String(args.shift() || "").trim().toUpperCase() || "OPERATOR";
    } else if (token === "--actor-session") {
      options.actorSession = String(args.shift() || "").trim() || "operator-viewport";
    } else if (token === "--wp") {
      options.wpId = String(args.shift() || "").trim();
    } else if (token === "--filter") {
      const value = String(args.shift() || "").trim().toUpperCase();
      if (FILTERS.includes(value)) options.filter = value;
    } else if (token === "--view") {
      const value = String(args.shift() || "").trim().toUpperCase();
      if (DETAIL_VIEWS.includes(value)) options.detailView = value;
    } else if (token === "--refresh-ms") {
      const value = Number(args.shift() || "");
      if (Number.isInteger(value) && value >= 250) options.refreshMs = value;
    }
  }
  return options;
}

function latestTimestamp(values) {
  return values
    .filter(Boolean)
    .map((value) => String(value))
    .sort()
    .at(-1) || null;
}

function normalizeBoardText(text) {
  return String(text || "").replace(/\r\n/g, "\n").trim();
}

function parseTaskBoard(boardPath = TASK_BOARD_PATH) {
  const lines = readText(boardPath).split(/\r?\n/);
  const entries = [];
  let section = "OTHER";
  for (const line of lines) {
    if (/^##\s+Active\b/.test(line)) {
      section = "ACTIVE";
      continue;
    }
    if (/^##\s+Ready for Dev\b/.test(line)) {
      section = "READY_FOR_DEV";
      continue;
    }
    if (/^##\s+Stub Backlog\b/.test(line)) {
      section = "STUB";
      continue;
    }
    if (/^##\s+In Progress\b/.test(line)) {
      section = "IN_PROGRESS";
      continue;
    }
    if (/^##\s+Done\b/.test(line)) {
      section = "DONE";
      continue;
    }
    if (/^##\s+Blocked\b/.test(line)) {
      section = "BLOCKED";
      continue;
    }
    if (/^##\s+Superseded\b/.test(line)) {
      section = "SUPERSEDED";
      continue;
    }
    const match = line.match(/^\s*-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[([A-Z_]+)\](?:\s+-\s+(.*))?$/);
    if (!match) continue;
    entries.push({
      wpId: match[1],
      boardSection: section,
      boardToken: match[2],
      detail: (match[3] || "").trim(),
    });
  }
  return entries;
}

function parseTraceabilityRegistry() {
  const byBaseWpId = new Map();
  const byWpId = new Map();
  if (!fs.existsSync(repoPathAbs(TRACEABILITY_PATH))) return { byBaseWpId, byWpId };
  const lines = readText(TRACEABILITY_PATH).split(/\r?\n/);
  for (const line of lines) {
    if (!line.startsWith("|") || /^\|\s*-+/.test(line)) continue;
    const parts = line.split("|").slice(1, -1).map((value) => value.trim());
    if (parts.length < 4) continue;
    const [baseWpId, activePacket] = parts;
    if (!/^WP-/.test(baseWpId) || !activePacket.startsWith(`${GOV_ROOT_REPO_REL}/`)) continue;
    byBaseWpId.set(baseWpId, activePacket);
    const activeWpId = inferWpIdFromPacketPath(activePacket);
    if (activeWpId) byWpId.set(activeWpId, activePacket);
  }
  return { byBaseWpId, byWpId };
}

function parseSessionRegistry() {
  try {
    const { registry } = loadSessionRegistry(REPO_ROOT);
    const byWpId = new Map();
    for (const session of registry.sessions || []) {
      const summary = registrySessionSummary(session);
      const entries = byWpId.get(summary.wp_id) || [];
      entries.push(summary);
      byWpId.set(summary.wp_id, entries);
    }
    return {
      byWpId,
      batchSummary: registryBatchLaunchSummary(registry),
    };
  } catch {
    return {
      byWpId: new Map(),
      batchSummary: null,
    };
  }
}

function parseSessionControlResults() {
  const byWpId = new Map();
  if (!fs.existsSync(repoPathAbs(SESSION_CONTROL_RESULTS_PATH))) return byWpId;
  try {
    const results = parseJsonlFile(SESSION_CONTROL_RESULTS_PATH);
    for (const result of results) {
      const entries = byWpId.get(result.wp_id) || [];
      entries.push(result);
      byWpId.set(result.wp_id, entries);
    }
  } catch {
    return new Map();
  }
  return byWpId;
}

function parseSessionControlRequests() {
  const byWpId = new Map();
  if (!fs.existsSync(repoPathAbs(SESSION_CONTROL_REQUESTS_PATH))) return byWpId;
  try {
    const requests = parseJsonlFile(SESSION_CONTROL_REQUESTS_PATH);
    for (const request of requests) {
      const entries = byWpId.get(request.wp_id) || [];
      entries.push(request);
      byWpId.set(request.wp_id, entries);
    }
  } catch {
    return new Map();
  }
  return byWpId;
}

function parseBrokerState() {
  const activeRunsByWpId = new Map();
  if (!fs.existsSync(repoPathAbs(SESSION_CONTROL_BROKER_STATE_PATH))) {
    return {
      state: null,
      activeRunsByWpId,
      brokerAlive: false,
      activeRunCount: 0,
      buildId: "",
      summary: "broker=OFF",
    };
  }
  try {
    const state = parseJsonFile(SESSION_CONTROL_BROKER_STATE_PATH);
    for (const run of state.active_runs || []) {
      const entries = activeRunsByWpId.get(run.wp_id) || [];
      entries.push(run);
      activeRunsByWpId.set(run.wp_id, entries);
    }
    const brokerAlive = isProcessAlive(state.broker_pid);
    const activeRunCount = Array.isArray(state.active_runs) ? state.active_runs.length : 0;
    const buildId = String(state.broker_build_id || "").trim();
    return {
      state,
      activeRunsByWpId,
      brokerAlive,
      activeRunCount,
      buildId,
      summary: `broker=${brokerAlive ? "ON" : "OFF"} pid=${state.broker_pid || 0} port=${state.port || 0} runs=${activeRunCount}${buildId ? ` build=${buildId}` : ""}`,
    };
  } catch {
    return {
      state: null,
      activeRunsByWpId: new Map(),
      brokerAlive: false,
      activeRunCount: 0,
      buildId: "",
      summary: "broker=INVALID_STATE",
    };
  }
}

function currentBranch() {
  if (currentBranchCache.expiresAt > Date.now()) return currentBranchCache.value;
  try {
    currentBranchCache = {
      value: execFileSync("git", ["branch", "--show-current"], {
        cwd: CURRENT_WORKTREE_DIR,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      }).trim(),
      expiresAt: Date.now() + CURRENT_BRANCH_CACHE_TTL_MS,
    };
  } catch {
    currentBranchCache = {
      value: "",
      expiresAt: Date.now() + CURRENT_BRANCH_CACHE_TTL_MS,
    };
  }
  return currentBranchCache.value;
}

function loadBoardSourceInfo() {
  const info = {
    current_branch: currentBranch(),
    current_worktree_dir: normalize(CURRENT_WORKTREE_DIR),
    current_board_path: normalize(repoPathAbs(TASK_BOARD_PATH)),
    canonical_branch: "main",
    canonical_worktree_dir: "",
    canonical_board_path: "",
    board_drift: "UNKNOWN",
    display: "board=current",
    detail: "",
  };
  if (!fs.existsSync(repoPathAbs(TOPOLOGY_PATH))) {
    info.display = `board=current:${info.current_branch || "<unknown>"}`;
    return info;
  }
  try {
    const topology = parseJsonFile(TOPOLOGY_PATH);
    info.canonical_branch = topology.canonical_branch || "main";
    const canonical = Array.isArray(topology.protected_worktrees)
      ? topology.protected_worktrees.find((entry) => entry && entry.canonical)
      : null;
    if (canonical?.rel_path) {
      info.canonical_worktree_dir = normalize(path.resolve(CURRENT_WORKTREE_DIR, canonical.rel_path));
      info.canonical_board_path = normalize(path.resolve(CURRENT_WORKTREE_DIR, canonical.rel_path, `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`));
    }
    const isCanonical = info.canonical_worktree_dir && info.current_worktree_dir === info.canonical_worktree_dir;
    if (isCanonical) {
      info.board_drift = "CANONICAL";
      info.display = `board=canonical:${info.canonical_branch}`;
      info.detail = `current=${info.current_board_path}`;
      return info;
    }
    if (info.canonical_board_path && fs.existsSync(info.canonical_board_path) && fs.existsSync(info.current_board_path)) {
      const currentBoard = normalizeBoardText(readText(info.current_board_path));
      const canonicalBoard = normalizeBoardText(readText(info.canonical_board_path));
      info.board_drift = currentBoard === canonicalBoard ? "IN_SYNC" : "DIVERGED";
    }
    info.display = `board=mirror:${info.current_branch || "<unknown>"} | canonical=${info.canonical_branch}@${info.canonical_worktree_dir || "<unknown>"} | drift=${info.board_drift}`;
    info.detail = `current=${info.current_board_path} | canonical=${info.canonical_board_path || "<unknown>"}`;
  } catch {
    info.display = `board=current:${info.current_branch || "<unknown>"}`;
    info.detail = `current=${info.current_board_path}`;
  }
  return info;
}

function resolvePacketPath(wpId, traceability) {
  if (traceability.byWpId.has(wpId)) return traceability.byWpId.get(wpId);
  const official = resolveWorkPacketPath(wpId)?.packetPath || "";
  if (official && fs.existsSync(repoPathAbs(official))) return normalize(official);
  const stub = normalize(path.join(PACKET_STUBS_DIR, `${wpId}.md`));
  if (fs.existsSync(repoPathAbs(stub))) return stub;
  if (traceability.byBaseWpId.has(wpId)) return traceability.byBaseWpId.get(wpId);
  return null;
}

function compareBoardEntries(left, right) {
  if (!left || !right) return null;
  return left.boardSection === right.boardSection
    && left.boardToken === right.boardToken
    && String(left.detail || "").trim() === String(right.detail || "").trim();
}

function formatBoardEntry(entry) {
  if (!entry) return "<missing>";
  const detail = String(entry.detail || "").trim();
  return detail ? `${entry.boardSection}/${entry.boardToken} | ${detail}` : `${entry.boardSection}/${entry.boardToken}`;
}

function parseThreadEntries(threadPath) {
  if (!threadPath || !fs.existsSync(repoPathAbs(threadPath))) return [];
  const lines = readText(threadPath).split(/\r?\n/);
  const entries = [];
  let current = null;
  for (const line of lines) {
    if (/^\s*-\s+\d{4}-\d{2}-\d{2}T[^\s]+Z\s+\|/.test(line)) {
      const parts = line.replace(/^\s*-\s+/, "").split("|").map((value) => value.trim()).filter(Boolean);
      const [timestamp, actorRole, ...metadata] = parts;
      const entry = {
        timestamp: timestamp || "",
        actorRole: actorRole || "",
        actorSession: "",
        target: "",
        targetRole: "",
        targetSession: "",
        correlationId: "",
        requiresAck: false,
        ackFor: "",
        specAnchor: "",
        packetRowRef: "",
        messageLines: [],
      };
      for (const item of metadata) {
        if (item.startsWith("session=")) entry.actorSession = item.slice("session=".length).trim();
        else if (item.startsWith("target=")) entry.target = item.slice("target=".length).trim();
        else if (item.startsWith("target_role=")) entry.targetRole = item.slice("target_role=".length).trim();
        else if (item.startsWith("target_session=")) entry.targetSession = item.slice("target_session=".length).trim();
        else if (item.startsWith("correlation_id=")) entry.correlationId = item.slice("correlation_id=".length).trim();
        else if (item === "requires_ack=true") entry.requiresAck = true;
        else if (item.startsWith("ack_for=")) entry.ackFor = item.slice("ack_for=".length).trim();
        else if (item.startsWith("spec_anchor=")) entry.specAnchor = item.slice("spec_anchor=".length).trim();
        else if (item.startsWith("packet_row_ref=")) entry.packetRowRef = item.slice("packet_row_ref=".length).trim();
      }
      if (current) entries.push(current);
      current = entry;
      continue;
    }
    if (current && /^\s{2,}\S/.test(line)) {
      current.messageLines.push(line.trim());
    }
  }
  if (current) entries.push(current);
  return entries;
}

function formatTarget(role, session) {
  const targetRole = String(role || "").trim();
  const targetSession = String(session || "").trim();
  if (!targetRole) return "";
  return targetSession ? `${targetRole}:${targetSession}` : targetRole;
}

function boardBadge(section) {
  const normalized = String(section || "OTHER").trim().toUpperCase();
  return paint(`[${normalized}]`, STATUS_COLORS[normalized] || STATUS_COLORS.OTHER, { bold: true });
}

function shortRoleLabel(role) {
  const normalized = String(role || "").trim().toUpperCase();
  if (normalized === "ORCHESTRATOR") return "ORC";
  if (normalized === "CODER") return "COD";
  if (normalized === "WP_VALIDATOR") return "WPV";
  if (normalized === "INTEGRATION_VALIDATOR") return "INT";
  if (normalized === "VALIDATOR") return "VAL";
  return normalized.slice(0, 3) || "---";
}

function normalizeLaneState(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (!normalized) return "none";
  if (["working", "running", "command_running"].includes(normalized)) return "working";
  if (["waiting", "input_required", "plugin_requested", "plugin_confirmed", "terminal_command_dispatched"].includes(normalized)) return "waiting";
  if (["blocked", "failed", "cli_escalation_ready"].includes(normalized)) return "blocked";
  if (["completed", "closed"].includes(normalized)) return "completed";
  if (["ready"].includes(normalized)) return "ready";
  if (["starting"].includes(normalized)) return "starting";
  if (["unstarted", "none"].includes(normalized)) return "unstarted";
  if (["idle"].includes(normalized)) return "idle";
  return normalized;
}

function isTerminalPacketStatus(status = "") {
  const text = String(status || "").trim();
  return /^Validated\s*\(/i.test(text)
    || /^Done$/i.test(text);
}

function isTerminalBoardSection(section = "") {
  const text = String(section || "").trim().toUpperCase();
  return text === "DONE"
    || text === "SUPERSEDED";
}

function isTerminalRecord(record) {
  return Boolean(
    isTerminalBoardSection(record?.boardSection)
    || isTerminalPacketStatus(record?.packetRecord?.packetStatus)
    || isTerminalPacketStatus(record?.packetRecord?.runtime?.current_packet_status),
  );
}

function laneStateForRole(record, role) {
  const packetSession = (record.sessions || []).find((entry) => String(entry.role || "").trim().toUpperCase() === role);
  if (packetSession?.state) return normalizeLaneState(packetSession.state);
  const registrySession = [...(record.registrySessions || [])]
    .reverse()
    .find((entry) => String(entry.role || "").trim().toUpperCase() === role);
  if (registrySession?.runtime_state) return normalizeLaneState(registrySession.runtime_state);
  return "none";
}

function roleLaneChip(record, role) {
  const state = laneStateForRole(record, role);
  const label = `${shortRoleLabel(role)}:${state.slice(0, 4).toUpperCase()}`;
  return paint(label, ROLE_COLORS[role] || STATUS_COLORS.OTHER, { bold: state !== "none" && state !== "idle" });
}

function reviewPressureChip(record) {
  if (isTerminalRecord(record)) {
    return paint("REV:HID", STATE_COLORS.none, { dim: true });
  }
  const count = Number(record.packetRecord?.openReviewItems?.length || 0);
  const text = `REV:${count}`;
  if (count <= 0) return paint(text, STATE_COLORS.none, { dim: true });
  return paint(text, count >= 3 ? STATE_COLORS.blocked : STATE_COLORS.waiting, { bold: true });
}

function communicationHealthChip(record) {
  if (isTerminalRecord(record)) {
    return paint("COMM:HID", STATE_COLORS.none, { dim: true });
  }
  const state = String(record.communicationHealthState || "COMM_NA").trim().toUpperCase();
  const labelMap = {
    COMM_NA: "OFF",
    COMM_MISCONFIGURED: "MISCFG",
    COMM_MISSING_KICKOFF: "KICKOFF",
    COMM_WAITING_FOR_INTENT: "INTENT",
    COMM_WAITING_FOR_HANDOFF: "HANDOFF",
    COMM_WAITING_FOR_REVIEW: "REVIEW",
    COMM_BLOCKED_OPEN_ITEMS: "BLOCKED",
    COMM_OK: "OK",
    COMM_STALE: "STALE",
  };
  const colorMap = {
    COMM_NA: STATE_COLORS.none,
    COMM_MISCONFIGURED: STATE_COLORS.blocked,
    COMM_MISSING_KICKOFF: STATE_COLORS.blocked,
    COMM_WAITING_FOR_INTENT: STATE_COLORS.waiting,
    COMM_WAITING_FOR_HANDOFF: STATE_COLORS.waiting,
    COMM_WAITING_FOR_REVIEW: STATE_COLORS.waiting,
    COMM_BLOCKED_OPEN_ITEMS: STATE_COLORS.blocked,
    COMM_OK: STATE_COLORS.working,
    COMM_STALE: "\x1b[38;5;208m",
  };
  const text = `COMM:${labelMap[state] || state.replace(/^COMM_/, "")}`;
  return paint(
    text,
    colorMap[state] || STATUS_COLORS.OTHER,
    { bold: !["COMM_NA"].includes(state), dim: state === "COMM_NA" }
  );
}

function communicationHealthStateText(record) {
  const state = String(record.communicationHealthState || "COMM_NA").trim().toUpperCase();
  const labelMap = {
    COMM_NA: "NA",
    COMM_MISCONFIGURED: "MISCONFIGURED",
    COMM_MISSING_KICKOFF: "MISSING_KICKOFF",
    COMM_WAITING_FOR_INTENT: "WAITING_FOR_INTENT",
    COMM_WAITING_FOR_HANDOFF: "WAITING_FOR_HANDOFF",
    COMM_WAITING_FOR_REVIEW: "WAITING_FOR_REVIEW",
    COMM_BLOCKED_OPEN_ITEMS: "BLOCKED_OPEN_ITEMS",
    COMM_OK: "OK",
    COMM_STALE: "STALE",
  };
  return labelMap[state] || state.replace(/^COMM_/, "");
}

function communicationHealthLine(record) {
  if (isTerminalRecord(record)) {
    return "comm=HIDDEN | Communication health history is hidden for terminal WPs by default.";
  }
  return `comm=${communicationHealthStateText(record)} | ${record.packetRecord?.communicationHealthEvaluation?.message || "No communication health summary."}`;
}

function notificationChip(record) {
  if (isTerminalRecord(record)) return "";
  const pending = record.packetRecord?.pendingNotifications || { total: 0 };
  const count = pending.total || 0;
  if (count <= 0) return "";
  const text = `\u2709${count}`;
  return paint(text, count >= 3 ? STATE_COLORS.blocked : "\x1b[38;5;208m", { bold: true });
}

function formatTokenCount(tokens) {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}k`;
  return String(tokens);
}

function tokenBudgetChip(record) {
  if (isTerminalRecord(record)) return "";
  const budget = record.tokenBudget;
  if (!budget) return "";
  const inputTokens = record.tokenUsage?.summary?.usage_totals?.input_tokens || 0;
  if (inputTokens <= 0 && budget.status === "PASS") return "";
  const label = `TOK:${formatTokenCount(inputTokens)}`;
  const colors = { PASS: STATE_COLORS.ready, WARN: STATE_COLORS.waiting, FAIL: STATE_COLORS.blocked };
  return paint(label, colors[budget.status] || STATE_COLORS.none, { bold: budget.status !== "PASS" });
}

function latestReviewLabel(record) {
  if (isTerminalRecord(record)) {
    return paint("review:hidden", STATE_COLORS.none, { dim: true });
  }
  const entry = record.packetRecord?.lastReviewReceipt;
  if (!entry) return paint("review:none", STATE_COLORS.none, { dim: true });
  const target = formatTarget(entry.target_role, entry.target_session);
  const label = `${entry.receipt_kind}:${entry.actor_role}${target ? `>${target}` : ""}`;
  return paint(label, STATE_COLORS.waiting);
}

function summarizeSessions(runtime) {
  if (!runtime || !Array.isArray(runtime.active_role_sessions)) return [];
  return runtime.active_role_sessions.map((entry) => ({
    role: entry.role,
    sessionId: entry.session_id,
    authorityKind: entry.authority_kind,
    validatorRoleKind: entry.validator_role_kind,
    state: entry.state,
    worktreeDir: entry.worktree_dir,
    lastHeartbeatAt: entry.last_heartbeat_at,
  }));
}

function resolveWorktreeInfo(localWorktreeDir) {
  const value = String(localWorktreeDir || "").trim();
  if (!value || value === "<pending>") {
    return {
      absolutePath: "",
      exists: false,
      status: "PENDING",
    };
  }
  const absolutePath = path.resolve(CURRENT_WORKTREE_DIR, value);
  return {
    absolutePath: normalize(absolutePath),
    exists: fs.existsSync(absolutePath),
    status: fs.existsSync(absolutePath) ? "PRESENT" : "MISSING",
  };
}

function hasActivePacketSessions(record) {
  const runtimeStatus = String(record.packetRecord?.runtime?.runtime_status || "").trim().toLowerCase();
  if (record.stale) return false;
  if (["completed", "done", "closed", "failed", "merged"].includes(runtimeStatus)) return false;
  return (record.sessions || []).some((session) =>
    ACTIVE_PACKET_SESSION_STATES.has(String(session.state || "").trim().toLowerCase())
  );
}

function parsePrepareAssignments() {
  if (!fs.existsSync(repoPathAbs(ORCHESTRATOR_GATES_PATH))) return new Map();
  try {
    const parsed = JSON.parse(readText(ORCHESTRATOR_GATES_PATH));
    const gateLogs = Array.isArray(parsed?.gate_logs) ? parsed.gate_logs : [];
    const assignments = new Map();
    for (const entry of gateLogs) {
      if (String(entry?.type || "").toUpperCase() !== "PREPARE") continue;
      const wpId = String(entry?.wpId || "").trim();
      if (!wpId) continue;
      const previous = assignments.get(wpId);
      if (!previous || String(previous.timestamp || "") <= String(entry.timestamp || "")) {
        assignments.set(wpId, entry);
      }
    }
    return assignments;
  } catch {
    return new Map();
  }
}

function loadPendingNotifications(wpId, communicationDir) {
  const result = { total: 0, byRole: {} };
  try {
    if (!wpId || !communicationDir) return result;
    const checks = Object.values(checkAllNotifications({ wpId }));
    result.total = checks.reduce((sum, entry) => sum + Number(entry.pendingCount || 0), 0);
    for (const entry of checks) {
      result.byRole[entry.role] = Number(entry.pendingCount || 0);
    }
  } catch {
    return result;
  }
  return result;
}

function parsePacketRecord(packetPath, prepareAssignment = null) {
  if (!packetPath || !fs.existsSync(repoPathAbs(packetPath))) return null;
  const packetText = readText(packetPath);
  const wpId = inferWpIdFromPacketPath(packetPath) || path.basename(packetPath, ".md");
  const baseWpId = parseSingleField(packetText, "BASE_WP_ID") || wpId;
  const runtimePath = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsPath = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const threadPath = parseSingleField(packetText, "WP_THREAD_FILE");
  const record = {
    wpId,
    baseWpId,
    packetPath: normalize(packetPath),
    packetKind: packetPath.includes("/stubs/") ? "STUB" : "OFFICIAL",
    packetStatus: parsePacketStatus(packetText),
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE") || "<missing>",
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION") || "",
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT") || "",
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE") || "",
    executionOwner: parseSingleField(packetText, "EXECUTION_OWNER") || "<missing>",
    localBranch: parseSingleField(packetText, "LOCAL_BRANCH") || "<pending>",
    localWorktreeDir: parseSingleField(packetText, "LOCAL_WORKTREE_DIR") || "<pending>",
    workflowAuthority: parseSingleField(packetText, "WORKFLOW_AUTHORITY") || "ORCHESTRATOR",
    technicalAdvisor: parseSingleField(packetText, "TECHNICAL_ADVISOR") || "WP_VALIDATOR",
    technicalAuthority: parseSingleField(packetText, "TECHNICAL_AUTHORITY") || "INTEGRATION_VALIDATOR",
    mergeAuthority: parseSingleField(packetText, "MERGE_AUTHORITY") || "INTEGRATION_VALIDATOR",
    wpValidatorOfRecord: parseSingleField(packetText, "WP_VALIDATOR_OF_RECORD") || "<unassigned>",
    integrationValidatorOfRecord: parseSingleField(packetText, "INTEGRATION_VALIDATOR_OF_RECORD") || "<unassigned>",
    communicationDir: parseSingleField(packetText, "WP_COMMUNICATION_DIR"),
    threadPath,
    runtimePath,
    receiptsPath,
    packetText,
    artifactPreviewLines: packetText.split(/\r?\n/).slice(0, 40),
  };
  if (prepareAssignment) {
    if (!record.localBranch || record.localBranch === "<pending>") {
      record.localBranch = String(prepareAssignment.branch || "").trim() || "<pending>";
    }
    if (!record.localWorktreeDir || record.localWorktreeDir === "<pending>") {
      record.localWorktreeDir = String(prepareAssignment.worktree_dir || "").trim() || "<pending>";
    }
  }
  const worktreeInfo = resolveWorktreeInfo(record.localWorktreeDir);
  record.localWorktreeAbsPath = worktreeInfo.absolutePath;
  record.localWorktreeExists = worktreeInfo.exists;
  record.localWorktreeStatus = worktreeInfo.status;
  record.prepareAssignment = prepareAssignment ? {
    branch: String(prepareAssignment.branch || "").trim() || "",
    worktreeDir: String(prepareAssignment.worktree_dir || "").trim() || "",
    timestamp: String(prepareAssignment.timestamp || "").trim() || "",
  } : null;

  try {
    if (runtimePath && fs.existsSync(repoPathAbs(runtimePath))) {
      const runtime = parseJsonFile(runtimePath);
      record.runtimeValidationErrors = validateRuntimeStatus(runtime);
      record.runtime = runtime;
    } else {
      record.runtime = null;
      record.runtimeValidationErrors = [];
    }
  } catch (error) {
    record.runtime = null;
    record.runtimeValidationErrors = [error.message];
  }

  try {
    if (receiptsPath && fs.existsSync(repoPathAbs(receiptsPath))) {
      const receipts = parseJsonlFile(receiptsPath);
      record.receiptValidationErrors = receipts.flatMap((entry, index) =>
        validateReceipt(entry).map((message) => `line ${index + 1}: ${message}`)
      );
      record.receipts = receipts;
    } else {
      record.receipts = [];
      record.receiptValidationErrors = [];
    }
  } catch (error) {
    record.receipts = [];
    record.receiptValidationErrors = [error.message];
  }

  record.threadEntries = parseThreadEntries(threadPath);
  record.lastThreadEntry = record.threadEntries.at(-1) || null;
  record.lastReceipt = record.receipts.at(-1) || null;
  record.openReviewItems = Array.isArray(record.runtime?.open_review_items) ? record.runtime.open_review_items : [];
  record.reviewReceipts = (record.receipts || []).filter((entry) => REVIEW_TRACKED_RECEIPT_KIND_VALUES.includes(String(entry.receipt_kind || "").trim().toUpperCase()));
  record.lastReviewReceipt = record.reviewReceipts.at(-1) || null;
  record.communicationHealthEvaluation = evaluateWpCommunicationHealth({
    wpId: record.wpId,
    stage: "STATUS",
    packetPath: record.packetPath,
    packetContent: record.packetText || "",
    workflowLane: record.workflowLane,
    packetFormatVersion: record.packetFormatVersion,
    communicationContract: record.communicationContract,
    communicationHealthGate: record.communicationHealthGate,
    receipts: record.receipts || [],
    runtimeStatus: record.runtime || { open_review_items: [] },
  });
  record.lastActivityAt = [
    record.runtime?.last_event_at || null,
    record.lastThreadEntry?.timestamp || null,
    record.lastReceipt?.timestamp_utc || null,
  ]
    .filter(Boolean)
    .sort()
    .at(-1) || null;

  record.pendingNotifications = loadPendingNotifications(record.wpId, record.communicationDir);

  return record;
}

function refinementPathForWp(wpId) {
  const candidate = normalize(resolveRefinementPath(wpId) || path.join(GOV_ROOT_REPO_REL, "refinements", `${wpId}.md`));
  return fs.existsSync(repoPathAbs(candidate)) ? candidate : "";
}

function stubPathForWp(baseWpId, wpId) {
  const candidates = [
    normalize(path.join(PACKET_STUBS_DIR, `${baseWpId}.md`)),
    normalize(path.join(PACKET_STUBS_DIR, `${wpId}.md`)),
  ];
  return candidates.find((candidate) => fs.existsSync(repoPathAbs(candidate))) || "";
}

function validatorGatePathForWp(wpId) {
  const candidate = normalize(resolveValidatorGatePath(wpId));
  return fs.existsSync(repoPathAbs(candidate)) ? candidate : "";
}

function latestAuditPathForWp(baseWpId, wpId) {
  const auditDir = normalize(path.join(GOV_ROOT_REPO_REL, "Audits"));
  const auditDirAbs = repoPathAbs(auditDir);
  if (!fs.existsSync(auditDirAbs)) return "";
  const matches = fs.readdirSync(auditDirAbs, { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
    .filter((name) => name.includes(wpId) || name.includes(baseWpId))
    .sort();
  if (matches.length === 0) return "";
  return normalize(path.join(auditDir, matches.at(-1)));
}

function readArtifactLines(filePath) {
  if (!filePath || !fs.existsSync(repoPathAbs(filePath))) return ["<missing>"];
  const extension = path.extname(filePath).toLowerCase();
  try {
    if (extension === ".json") {
      return JSON.stringify(parseJsonFile(filePath), null, 2).split(/\r?\n/);
    }
    return readText(filePath).split(/\r?\n/);
  } catch (error) {
    return [`<read error: ${error.message}>`];
  }
}

function numberLines(lines) {
  const width = String(Math.max(1, lines.length)).length;
  return lines.map((line, index) => `${String(index + 1).padStart(width, " ")} | ${line}`);
}

function buildDocArtifacts(record) {
  const packet = record.packetRecord;
  if (!packet) return [];
  const artifacts = [
    {
      key: "PACKET",
      label: "Packet",
      badge: packet.packetKind === "STUB" ? "STUB_PACKET" : "ACTIVE_PACKET",
      path: packet.packetPath,
    },
    {
      key: "REFINEMENT",
      label: "Refinement",
      badge: "REFINEMENT",
      path: refinementPathForWp(record.wpId),
    },
    {
      key: "STUB",
      label: "Stub",
      badge: "ACTIVE_STUB",
      path: stubPathForWp(packet.baseWpId || record.wpId, record.wpId),
    },
    {
      key: "VALIDATOR_GATE",
      label: "Validator Gate",
      badge: "VALIDATOR_GATE",
      path: validatorGatePathForWp(record.wpId),
    },
    {
      key: "AUDIT",
      label: "Latest Audit",
      badge: "AUDIT",
      path: latestAuditPathForWp(packet.baseWpId || record.wpId, record.wpId),
    },
  ].filter((artifact) => artifact.path);
  return artifacts.map((artifact) => ({
    ...artifact,
    exists: fs.existsSync(repoPathAbs(artifact.path)),
    lines: numberLines(readArtifactLines(artifact.path)),
  }));
}

function buildCommsArtifacts(record) {
  const packet = record.packetRecord;
  if (!packet) return [];
  const artifacts = [
    {
      key: "THREAD",
      label: "Thread",
      badge: "WP_THREAD",
      path: packet.threadPath || "",
    },
    {
      key: "RECEIPTS",
      label: "Receipts",
      badge: "WP_RECEIPTS",
      path: packet.receiptsPath || "",
    },
    {
      key: "RUNTIME",
      label: "Runtime",
      badge: "WP_RUNTIME",
      path: packet.runtimePath || "",
    },
  ].filter((artifact) => artifact.path);
  return artifacts.map((artifact) => ({
    ...artifact,
    exists: fs.existsSync(repoPathAbs(artifact.path)),
    lines: numberLines(readArtifactLines(artifact.path)),
  }));
}

function loadMonitorModel() {
  const traceability = parseTraceabilityRegistry();
  const prepareAssignments = parsePrepareAssignments();
  const boardSource = loadBoardSourceInfo();
  const currentBoardEntries = parseTaskBoard();
  const currentByWpId = new Map(currentBoardEntries.map((entry) => [entry.wpId, entry]));
  const canonicalBoardEntries = boardSource.canonical_board_path && fs.existsSync(boardSource.canonical_board_path)
    ? parseTaskBoard(boardSource.canonical_board_path)
    : [];
  const selectedBoardEntries = canonicalBoardEntries.length > 0 ? canonicalBoardEntries : currentBoardEntries;
  const selectedBoardSource = canonicalBoardEntries.length > 0 ? "CANONICAL_MAIN" : "CURRENT_WORKTREE";
  const canonicalByWpId = new Map(canonicalBoardEntries.map((entry) => [entry.wpId, entry]));
  const sessionRegistry = parseSessionRegistry();
  const sessionRegistryByWpId = sessionRegistry.byWpId;
  const controlRequests = parseSessionControlRequests();
  const controlResults = parseSessionControlResults();
  const brokerState = parseBrokerState();
  const records = selectedBoardEntries.map((entry) => {
    const packetPath = resolvePacketPath(entry.wpId, traceability);
    const packetRecord = parsePacketRecord(packetPath, prepareAssignments.get(entry.wpId) || null);
    const registrySessions = [...(sessionRegistryByWpId.get(entry.wpId) || [])]
      .sort((left, right) => String(left.updated_at || "").localeCompare(String(right.updated_at || "")));
    const wpControlRequests = [...(controlRequests.get(entry.wpId) || [])]
      .sort((left, right) => String(left.created_at || "").localeCompare(String(right.created_at || "")));
    const latestRegistrySession = registrySessions.at(-1) || null;
    const controlEventStreams = registrySessions.map((session) => {
      let events = [];
      if (session.last_command_output_file && fs.existsSync(repoPathAbs(session.last_command_output_file))) {
        try {
          events = parseJsonlFile(session.last_command_output_file);
        } catch {
          events = [];
        }
      }
      return {
        session_key: session.session_key,
        role: session.role,
        output_file: session.last_command_output_file || "",
        updated_at: session.updated_at || "",
        events,
      };
    });
    const controlEventTimeline = controlEventStreams
      .flatMap((stream) => (stream.events || []).map((event) => ({
        ...event,
        session_key: stream.session_key,
        role: stream.role,
        output_file: stream.output_file,
      })))
      .sort((left, right) => String(left.timestamp || "").localeCompare(String(right.timestamp || "")));
    const wpControlResults = [...(controlResults.get(entry.wpId) || [])]
      .sort((left, right) => String(left.processed_at || "").localeCompare(String(right.processed_at || "")));
    const resultIds = new Set(wpControlResults.map((result) => result.command_id));
    const pendingControlRequests = wpControlRequests.filter((request) => !resultIds.has(request.command_id));
    const controlBrokerRuns = [...(brokerState.activeRunsByWpId.get(entry.wpId) || [])]
      .sort((left, right) => String(left.started_at || "").localeCompare(String(right.started_at || "")));
    if (packetRecord) {
      packetRecord.relayEscalation = evaluateWpRelayEscalation({
        wpId: entry.wpId,
        runtimeStatus: packetRecord.runtime || {},
        communicationEvaluation: packetRecord.communicationHealthEvaluation || null,
        receipts: packetRecord.receipts || [],
        pendingNotifications: Object.values(checkAllNotifications({ wpId: entry.wpId })).flatMap((notificationEntry) => notificationEntry.notifications || []),
        registrySessions,
      });
    }
    const canonicalBoardEntry = canonicalByWpId.get(entry.wpId) || (selectedBoardSource === "CANONICAL_MAIN" ? entry : null);
    const currentBoardEntry = currentByWpId.get(entry.wpId) || null;
    const currentBoardMatchesSelected = compareBoardEntries(entry, currentBoardEntry);
    const lastActivityAt = latestTimestamp([
      packetRecord?.runtime?.last_event_at || null,
      packetRecord?.lastThreadEntry?.timestamp || null,
      packetRecord?.lastReceipt?.timestamp_utc || null,
      ...registrySessions.map((session) => session.updated_at || null),
      ...wpControlRequests.map((request) => request.created_at || null),
      ...wpControlResults.map((result) => result.processed_at || null),
      ...controlBrokerRuns.map((run) => run.started_at || null),
      ...controlEventTimeline.map((event) => event.timestamp || null),
    ]);
    if (packetRecord) {
      packetRecord.lastActivityAt = lastActivityAt;
    }
    const stale = Boolean(packetRecord?.runtime?.stale_after && new Date(packetRecord.runtime.stale_after) < new Date());
    let tokenUsage = null;
    let tokenBudget = null;
    try {
      const { ledger } = readWpTokenUsageLedger(REPO_ROOT, entry.wpId);
      if (ledger && (ledger.summary?.command_count || 0) > 0) {
        tokenUsage = ledger;
        tokenBudget = evaluateWpTokenBudget(ledger);
      }
    } catch {}
    return {
      ...entry,
      packetPath,
      packetRecord,
      boardSource,
      brokerState,
      selectedBoardSource,
      sessions: summarizeSessions(packetRecord?.runtime),
      registrySessions,
      controlRequests: wpControlRequests,
      pendingControlRequests,
      controlResults: wpControlResults,
      controlBrokerRuns,
      latestRegistrySession,
      controlEventStreams,
      controlEventTimeline,
      canonicalBoardEntry,
      currentBoardEntry,
      currentBoardMatchesSelected,
      lastActivityAt,
      stale,
      tokenUsage,
      tokenBudget,
      communicationHealthState: communicationMonitorState(packetRecord?.communicationHealthEvaluation, { stale }),
    };
  });
  records.sort((left, right) => {
    const leftIndex = BOARD_ORDER.indexOf(left.boardSection);
    const rightIndex = BOARD_ORDER.indexOf(right.boardSection);
    if (leftIndex !== rightIndex) return leftIndex - rightIndex;
    const leftActivity = String(left.lastActivityAt || "");
    const rightActivity = String(right.lastActivityAt || "");
    if (leftActivity !== rightActivity) return rightActivity.localeCompare(leftActivity);
    return left.wpId.localeCompare(right.wpId);
  });
  return { records, boardSource, brokerState };
}

function filterRecords(records, filter) {
  if (filter === "ALL") return records;
  if (filter === "ACTIVE") {
    return records.filter((record) => {
      const activeRegistrySession = (record.registrySessions || []).some((session) => ACTIVE_RUNTIME_STATES.has(session.runtime_state));
      const activeControl =
        (record.pendingControlRequests || []).length > 0
        || (record.controlBrokerRuns || []).length > 0
        || (record.controlResults || []).some((entry) => ["QUEUED", "RUNNING"].includes(String(entry.status || "").toUpperCase()));
      if (["DONE", "SUPERSEDED"].includes(record.boardSection)) {
        return activeRegistrySession || activeControl;
      }
      return (
        record.boardSection === "ACTIVE"
        || hasActivePacketSessions(record)
        || activeRegistrySession
        || activeControl
      );
    });
  }
  return records.filter((record) => record.boardSection === filter);
}

function truncate(value, width) {
  const text = String(value ?? "");
  if (width <= 0) return "";
  if (text.length <= width) return text.padEnd(width, " ");
  if (width === 1) return text.slice(0, 1);
  return `${text.slice(0, width - 1)}…`;
}

function summarizeControlEvent(event) {
  if (!event || typeof event !== "object") return "";
  if (event.type === "thread.started") return `thread.started | thread=${event.thread_id || "<missing>"}`;
  if (event.type === "stderr") return `stderr | ${String(event.text || "").trim()}`;
  if (event.type === "spawn.error") return `spawn.error | ${event.message || ""}`;
  if (event.type === "process.closed") return `process.closed | exit=${event.exit_code ?? "<missing>"}`;
  if (event.type === "item.completed" && event.item?.type === "agent_message") {
    return `agent_message | ${String(event.item.text || "").split(/\r?\n/, 1)[0]}`;
  }
  if (event.type === "item.completed" && event.item?.type === "command_execution") {
    return `command_execution | ${event.item.command || ""}`;
  }
  return JSON.stringify(event);
}

function buildTimelineEntries(record) {
  const entries = [];
  let sequence = 0;
  for (const entry of record.packetRecord?.threadEntries || []) {
    const threadTarget = formatTarget(entry.targetRole, entry.targetSession) || entry.target;
    entries.push({
      timestamp: entry.timestamp || "",
      sequence: sequence += 1,
      header: `${entry.timestamp || "<no-ts>"} | THREAD | ${entry.actorRole} | ${entry.actorSession}${threadTarget ? ` | ${threadTarget}` : ""}`,
      detailLines: [
        ...(entry.messageLines?.length ? entry.messageLines : ["<no body>"]),
        ...(entry.correlationId ? [`corr=${entry.correlationId}`] : []),
        ...(entry.specAnchor ? [`spec=${entry.specAnchor}`] : []),
        ...(entry.packetRowRef ? [`packet=${entry.packetRowRef}`] : []),
      ],
    });
  }
  for (const entry of record.packetRecord?.receipts || []) {
    const receiptRouting = [];
    if (entry.target_role || entry.target_session) receiptRouting.push(`target=${formatTarget(entry.target_role, entry.target_session) || "<unknown>"}`);
    if (entry.correlation_id) receiptRouting.push(`corr=${entry.correlation_id}`);
    if (entry.requires_ack) receiptRouting.push(`ack=${entry.ack_for || "required"}`);
    if (entry.spec_anchor) receiptRouting.push(`spec=${entry.spec_anchor}`);
    if (entry.packet_row_ref) receiptRouting.push(`packet=${entry.packet_row_ref}`);
    entries.push({
      timestamp: entry.timestamp_utc || "",
      sequence: sequence += 1,
      header: `${entry.timestamp_utc || "<no-ts>"} | RECEIPT | ${entry.actor_role} | ${entry.receipt_kind}`,
      detailLines: [entry.summary || "<no summary>", ...receiptRouting],
    });
  }
  for (const entry of record.controlRequests || []) {
    entries.push({
      timestamp: entry.created_at || "",
      sequence: sequence += 1,
      header: `${entry.created_at || "<no-ts>"} | CONTROL_REQUEST | ${entry.role} | ${entry.command_kind}`,
      detailLines: [entry.summary || entry.prompt?.split(/\r?\n/, 1)[0] || "<no summary>"],
    });
  }
  for (const entry of record.controlResults || []) {
    entries.push({
      timestamp: entry.processed_at || "",
      sequence: sequence += 1,
      header: `${entry.processed_at || "<no-ts>"} | CONTROL_RESULT | ${entry.role} | ${entry.command_kind} | ${entry.status}`,
      detailLines: [entry.summary || entry.error || "<no summary>"],
    });
  }
  for (const entry of record.controlEventTimeline || []) {
    entries.push({
      timestamp: entry.timestamp || "",
      sequence: sequence += 1,
      header: `${entry.timestamp || "<no-ts>"} | CONTROL_EVENT | ${entry.role || "<unknown>"} | ${entry.session_key || "<none>"}`,
      detailLines: [summarizeControlEvent(entry)],
    });
  }
  return entries.sort((left, right) =>
    String(left.timestamp || "").localeCompare(String(right.timestamp || ""))
    || left.sequence - right.sequence
  );
}

function wrapText(text, width) {
  const source = String(text || "");
  if (!source) return [""];
  if (width <= 4) return [sliceAnsi(source, width)];
  const words = source.split(/\s+/).filter(Boolean);
  const lines = [];
  let current = "";
  for (const word of words) {
    if (!current) {
      current = word;
      continue;
    }
    if (visibleLength(`${current} ${word}`) <= width) {
      current += ` ${word}`;
    } else {
      lines.push(current);
      current = word;
    }
  }
  if (current) lines.push(current);
  return lines;
}

function clampIndex(index, length) {
  if (length <= 0) return 0;
  return Math.max(0, Math.min(length - 1, index));
}

function renderList(records, selectedIndex, width, height, listHasFocus) {
  const rows = [];
  const cardHeight = 2;
  const maxCards = Math.max(1, Math.floor(height / cardHeight));
  if (records.length === 0) {
    rows.push(truncateVisible("No WPs in this filter.", width));
    while (rows.length < height) rows.push(" ".repeat(width));
    return rows;
  }
  const start = Math.max(0, Math.min(selectedIndex - Math.floor(maxCards / 2), Math.max(0, records.length - maxCards)));
  const visible = records.slice(start, start + maxCards);
  for (let index = 0; index < visible.length; index += 1) {
    const record = visible[index];
    const globalIndex = start + index;
    const marker = globalIndex === selectedIndex ? ">" : " ";
    const stale = record.stale && !isTerminalRecord(record) ? "!" : " ";
    const drift = record.currentBoardMatchesSelected === false ? "~" : " ";
    const worktreeFlag = record.packetRecord?.localWorktreeStatus === "MISSING" ? "x" : " ";
    const latest = record.lastActivityAt ? record.lastActivityAt.slice(11, 16) : "-----";
    const reviewChip = reviewPressureChip(record);
    const latestReview = latestReviewLabel(record);
    const laneSummary = [
      roleLaneChip(record, "CODER"),
      roleLaneChip(record, "WP_VALIDATOR"),
      roleLaneChip(record, "INTEGRATION_VALIDATOR"),
    ].join(" ");
    const notifChip = notificationChip(record);
    const commChip = communicationHealthChip(record);
    const tokChip = tokenBudgetChip(record);
    const line1 = `${marker}${stale}${drift}${worktreeFlag} ${paint(record.wpId, STATUS_COLORS.ACTIVE, { bold: globalIndex === selectedIndex })} ${boardBadge(record.boardSection)} ${reviewChip} ${commChip}${notifChip ? ` ${notifChip}` : ""}${tokChip ? ` ${tokChip}` : ""}`;
    const line2 = `    ${laneSummary} ${paint(latest, STATE_COLORS.ready, { dim: true })} ${latestReview} ${paint(`next:${compactNextActionLabel(record)}`, STATUS_COLORS.OTHER, { dim: true })}`;
    if (globalIndex === selectedIndex) {
      const selectedLine1 = truncateVisible(line1, width);
      const selectedLine2 = truncateVisible(line2, width);
      rows.push(listHasFocus ? `\x1b[7m${selectedLine1}\x1b[0m` : `\x1b[1m${selectedLine1}\x1b[0m`);
      rows.push(listHasFocus ? `\x1b[7m${selectedLine2}\x1b[0m` : `\x1b[1m${selectedLine2}\x1b[0m`);
    } else {
      rows.push(truncateVisible(line1, width));
      rows.push(truncateVisible(line2, width));
    }
  }
  while (rows.length < height) rows.push(" ".repeat(width));
  return rows;
}

function compactNextActionLabel(record) {
  if (!record) return "none";
  if (isTerminalRecord(record)) return "history";
  const relay = record.packetRecord?.relayEscalation || null;
  if (relay?.status === "ESCALATED" && relay.target_role) {
    return `${relay.target_role}${relay.target_session ? `:${relay.target_session}` : ""}`;
  }
  const operatorPending = Number(record.packetRecord?.pendingNotifications?.byRole?.OPERATOR || 0);
  if (operatorPending > 0) return "operator";
  const runtime = record.packetRecord?.runtime || {};
  const nextTarget = formatTarget(runtime.next_expected_actor, runtime.next_expected_session);
  if (nextTarget) return nextTarget;
  if ((record.packetRecord?.pendingNotifications?.total || 0) > 0) return "notifications";
  if ((record.packetRecord?.openReviewItems?.length || 0) > 0) return "review";
  if ((record.controlBrokerRuns || []).length > 0) return "broker";
  return "idle";
}

function nextActionSummary(record) {
  if (!record) return "No WP selected.";
  if (isTerminalRecord(record)) {
    return "Terminal WP history only; stale relay and backlog residue are hidden by default.";
  }
  const relay = record.packetRecord?.relayEscalation || null;
  if (relay?.status === "ESCALATED") {
    return `${relay.summary}${relay.recommended_command ? ` Run: ${relay.recommended_command}` : ""}`;
  }
  const operatorPending = Number(record.packetRecord?.pendingNotifications?.byRole?.OPERATOR || 0);
  if (operatorPending > 0) {
    return `Operator action required: ${operatorPending} unread operator notification(s) are pending for this WP.`;
  }
  const runtime = record.packetRecord?.runtime || {};
  const nextTarget = formatTarget(runtime.next_expected_actor, runtime.next_expected_session);
  if (nextTarget) {
    return `Next governed actor: ${nextTarget}${runtime.waiting_on ? ` | waiting_on=${runtime.waiting_on}${runtime.waiting_on_session ? `:${runtime.waiting_on_session}` : ""}` : ""}`;
  }
  const pending = Number(record.packetRecord?.pendingNotifications?.total || 0);
  if (pending > 0) return `Pending notifications: ${pending}.`;
  const reviewCount = Number(record.packetRecord?.openReviewItems?.length || 0);
  if (reviewCount > 0) return `Open structured review items: ${reviewCount}.`;
  if ((record.controlBrokerRuns || []).length > 0) return `ACP broker owns ${(record.controlBrokerRuns || []).length} governed run(s).`;
  return "No immediate governed action is pending.";
}

function buildTopSummary(records, filtered, selected, model, uiState) {
  const activeRuns = records.reduce((sum, record) => sum + Number((record.controlBrokerRuns || []).length), 0);
  const activeSessions = records.reduce((sum, record) =>
    sum + Number((record.registrySessions || []).filter((session) => ACTIVE_RUNTIME_STATES.has(session.runtime_state)).length), 0);
  const openReviews = records.reduce((sum, record) => {
    if (isTerminalRecord(record)) return sum;
    return sum + Number(record.packetRecord?.openReviewItems?.length || 0);
  }, 0);
  const pendingNotifications = records.reduce((sum, record) => {
    if (isTerminalRecord(record)) return sum;
    return sum + Number(record.packetRecord?.pendingNotifications?.total || 0);
  }, 0);
  const escalatedRelayCount = records.reduce((sum, record) =>
    sum + Number(record.packetRecord?.relayEscalation?.status === "ESCALATED"), 0);
  const staleCount = records.reduce((sum, record) => sum + Number(Boolean(record.stale && !isTerminalRecord(record))), 0);
  const totalInputTokens = records.reduce((sum, record) => sum + Number(record.tokenUsage?.summary?.usage_totals?.input_tokens || 0), 0);
  const worstBudget = records.reduce((worst, record) => {
    if (!record.tokenBudget) return worst;
    const rank = { PASS: 0, WARN: 1, FAIL: 2 };
    return (rank[record.tokenBudget.status] || 0) > (rank[worst] || 0) ? record.tokenBudget.status : worst;
  }, "PASS");
  const budgetColors = { PASS: STATE_COLORS.ready, WARN: STATE_COLORS.waiting, FAIL: STATE_COLORS.blocked };
  const broker = model.brokerState?.summary || "broker=<unknown>";
  const board = model.boardSource?.display || "board=<unknown>";
  return [
    `${paint("Operator Viewport", STATUS_COLORS.ACTIVE, { bold: true })} | mode=${uiState.admin ? "ADMIN" : "VIEW"} | focus=${uiState.focusedPane} | filter=${uiState.filter} | view=${uiState.detailView}`,
    `${paint(`visible=${filtered.length}/${records.length}`, STATUS_COLORS.ACTIVE, { bold: true })}  ${paint(`sessions=${activeSessions}`, STATUS_COLORS.READY_FOR_DEV)}  ${paint(`runs=${activeRuns}`, STATUS_COLORS.IN_PROGRESS)}  ${paint(`reviews=${openReviews}`, openReviews > 0 ? STATE_COLORS.waiting : STATE_COLORS.none, { bold: openReviews > 0 })}  ${paint(`notif=${pendingNotifications}`, pendingNotifications > 0 ? "\x1b[38;5;208m" : STATE_COLORS.none, { bold: pendingNotifications > 0 })}  ${paint(`relay=${escalatedRelayCount}`, escalatedRelayCount > 0 ? STATE_COLORS.blocked : STATE_COLORS.none, { bold: escalatedRelayCount > 0 })}  ${paint(`stale=${staleCount}`, staleCount > 0 ? STATE_COLORS.blocked : STATE_COLORS.none, { bold: staleCount > 0 })}  ${paint(`tokens=${formatTokenCount(totalInputTokens)}/${worstBudget}`, budgetColors[worstBudget] || STATE_COLORS.none, { bold: worstBudget !== "PASS" })}`,
    `${board} | ${broker} | actor=${uiState.actorRole}/${uiState.actorSession}`,
    `next_action=${nextActionSummary(selected)}`,
  ];
}

function buildDetailLines(record, width, detailView, uiState) {
  if (!record) return ["No WP selected."];
  const packet = record.packetRecord;
  const runtime = packet?.runtime;
  const broker = record.brokerState || {};
  const lines = [
    `${record.wpId} | board=${record.boardSection} | token=${record.boardToken}`,
    `packet=${packet?.packetKind || "UNKNOWN"} | packet_status=${packet?.packetStatus || "<none>"}`,
    `lane=${packet?.workflowLane || "<missing>"} | owner=${packet?.executionOwner || "<missing>"}`,
    `workflow=${packet?.workflowAuthority || "ORCHESTRATOR"} | tech=${packet?.technicalAuthority || "INTEGRATION_VALIDATOR"} | merge=${packet?.mergeAuthority || "INTEGRATION_VALIDATOR"}`,
    `wpval=${packet?.wpValidatorOfRecord || "<unassigned>"} | ival=${packet?.integrationValidatorOfRecord || "<unassigned>"}`,
    `branch=${packet?.localBranch || "<pending>"}`,
    `worktree=${packet?.localWorktreeDir || "<pending>"}`,
    `worktree_state=${packet?.localWorktreeStatus || "UNKNOWN"}${packet?.localWorktreeAbsPath ? ` | abs=${packet.localWorktreeAbsPath}` : ""}`,
    `board_source=${record.boardSource?.display || "<unknown>"}`,
    `selected_board_source=${record.selectedBoardSource || "CURRENT_WORKTREE"}`,
    `acp_broker=${broker.summary || "broker=<unknown>"}`,
  ];
  if (record.boardSource?.canonical_board_path) {
    lines.push(`canonical_board=${record.boardSource.canonical_board_path} | drift=${record.boardSource.board_drift || "UNKNOWN"}`);
  }
  lines.push(`canonical_entry=${formatBoardEntry(record.canonicalBoardEntry)} | current_entry=${formatBoardEntry(record.currentBoardEntry)}`);
  lines.push(`current_entry_drift=${record.currentBoardMatchesSelected === null ? "UNKNOWN" : (record.currentBoardMatchesSelected ? "NO" : "YES")}`);
  if (runtime) {
    const terminalHistorySuppressed = isTerminalRecord(record);
    lines.push(
      `runtime=${runtime.runtime_status}/${runtime.current_phase}`
      + ` | next=${formatTarget(runtime.next_expected_actor, runtime.next_expected_session) || runtime.next_expected_actor}`
      + ` | waiting_on=${runtime.waiting_on}${runtime.waiting_on_session ? ` (${runtime.waiting_on_session})` : ""}`
    );
    if (terminalHistorySuppressed) {
      lines.push("validator_trigger=<hidden> | ready=<hidden> | stale=HIDDEN | open_reviews=HIDDEN | history_hidden=YES");
    } else {
      lines.push(`validator_trigger=${runtime.validator_trigger} | ready=${runtime.ready_for_validation ? "YES" : "NO"} | stale=${record.stale ? "YES" : "NO"} | open_reviews=${record.packetRecord?.openReviewItems?.length || 0}`);
    }
    lines.push(communicationHealthLine(record));
  } else {
    lines.push("runtime=<none>");
  }
  lines.push("");

  if (detailView === "THREAD") {
    lines.push("THREAD");
    const recent = (packet?.threadEntries || []).slice(-8);
    if (recent.length === 0) {
      lines.push("No thread entries.");
    } else {
      for (const entry of recent) {
        const threadTarget = formatTarget(entry.targetRole, entry.targetSession) || entry.target;
        lines.push(`${entry.timestamp} | ${entry.actorRole} | ${entry.actorSession}${threadTarget ? ` | ${threadTarget}` : ""}`);
        for (const bodyLine of entry.messageLines) lines.push(`  ${bodyLine}`);
      }
    }
  } else if (detailView === "RECEIPTS") {
    lines.push("RECEIPTS");
    const recent = (packet?.receipts || []).slice(-8);
    if (recent.length === 0) {
      lines.push("No receipts.");
    } else {
      for (const entry of recent) {
        lines.push(`${entry.timestamp_utc} | ${entry.actor_role} | ${entry.receipt_kind}`);
        lines.push(`  ${entry.summary}`);
      }
    }
  } else if (detailView === "CONTROL") {
    lines.push("CONTROL");
    if ((record.registrySessions || []).length === 0) {
      lines.push("No governed sessions.");
    } else {
      for (const session of record.registrySessions) {
        lines.push(`${session.role} | state=${session.runtime_state} | protocol=${session.control_protocol || "<none>"}`);
        lines.push(`  transport=${session.control_transport || "<none>"} | host=${session.active_host || session.preferred_host || "NONE"} | terminal=${session.active_terminal_kind || "NONE"}`);
        lines.push(`  thread=${session.session_thread_id || "<none>"} | cmd=${session.last_command_kind || "NONE"}/${session.last_command_status || "NONE"}`);
        if (session.last_command_summary) lines.push(`  ${session.last_command_summary}`);
      }
    }
    lines.push("");
    lines.push("RECENT REQUESTS");
    const recentRequests = (record.controlRequests || []).slice(-8);
    if (recentRequests.length === 0) {
      lines.push("No control requests.");
    } else {
      for (const entry of recentRequests) {
        const pending = (record.pendingControlRequests || []).some((request) => request.command_id === entry.command_id) ? " | pending" : "";
        lines.push(`${entry.created_at} | ${entry.role} | ${entry.command_kind}${pending}`);
        lines.push(`  ${entry.summary || entry.prompt?.split(/\r?\n/, 1)[0] || "<no summary>"}`);
      }
    }
    lines.push("");
    lines.push("RECENT RESULTS");
    const recent = (record.controlResults || []).slice(-8);
    if (recent.length === 0) {
      lines.push("No control results.");
    } else {
      for (const entry of recent) {
        lines.push(`${entry.processed_at} | ${entry.role} | ${entry.command_kind} | ${entry.status}`);
        lines.push(`  ${entry.summary || entry.error || "<no summary>"}`);
      }
    }
    lines.push("");
    lines.push("BROKER");
    const brokerRuns = record.controlBrokerRuns || [];
    if (brokerRuns.length === 0) {
      lines.push("No active broker runs.");
    } else {
      for (const run of brokerRuns) {
        lines.push(`${run.started_at || "<no-ts>"} | ${run.role} | ${run.command_kind} | pid=${run.child_pid || 0}`);
        lines.push(`  timeout=${run.timeout_at || "<none>"} | reason=${run.termination_reason || "<none>"}`);
      }
    }
  } else if (detailView === "EVENTS") {
    lines.push("EVENTS");
    lines.push("Merged governed ACP output events for this WP.");
    const recent = (record.controlEventTimeline || []).slice(-16);
    if (recent.length === 0) {
      lines.push("No control events.");
    } else {
      for (const entry of recent) {
        lines.push(`${entry.timestamp || "<no-ts>"} | ${entry.role || "<unknown>"} | ${entry.session_key || "<none>"}`);
        lines.push(`  ${summarizeControlEvent(entry)}`);
      }
    }
  } else if (detailView === "TIMELINE") {
    lines.push("TIMELINE");
    lines.push("Merged thread, receipts, control requests/results, and ACP events.");
    const recent = buildTimelineEntries(record).slice(-20);
    if (recent.length === 0) {
      lines.push("No timeline entries.");
    } else {
      for (const entry of recent) {
        lines.push(entry.header);
        for (const detailLine of entry.detailLines) lines.push(`  ${detailLine}`);
      }
    }
  } else if (detailView === "ARTIFACT") {
    lines.push("ARTIFACT");
    lines.push(`packet=${record.packetPath || "<none>"}`);
    const preview = record.packetRecord?.artifactPreviewLines || [];
    if (preview.length === 0) {
      lines.push("No packet preview.");
    } else {
      for (const line of preview) lines.push(line);
    }
  } else {
    lines.push("OVERVIEW");
    lines.push("GOVERNED SESSIONS");
    if ((record.registrySessions || []).length === 0) {
      lines.push("No governed sessions.");
    } else {
      for (const session of record.registrySessions) {
        const activeRun = (record.controlBrokerRuns || []).find((run) => run.role === session.role) || null;
        lines.push(`${session.role} | ${session.runtime_state} | host=${session.active_host || session.preferred_host}`);
        lines.push(`  req=${session.plugin_request_count} fail=${session.plugin_failure_count} last=${session.plugin_last_result}`);
        if (session.session_thread_id) {
          lines.push(`  thread=${session.session_thread_id} cmd=${session.last_command_kind}/${session.last_command_status}`);
          if (session.control_protocol) lines.push(`  protocol=${session.control_protocol} transport=${session.control_transport || "<none>"}`);
        }
        if (session.runtime_state === "CLOSED") {
          lines.push("  note=session thread registration cleared; start-session is required before steering again");
        }
        if (activeRun) {
          lines.push(`  active_run=${activeRun.command_kind} started=${activeRun.started_at || "<no-ts>"} timeout=${activeRun.timeout_at || "<none>"}`);
        }
        if (session.runtime_state === "TERMINAL_COMMAND_DISPATCHED" || session.runtime_state === "PLUGIN_CONFIRMED") {
          lines.push("  note=terminal dispatch only; wait for receipts/heartbeat/runtime movement");
        } else if (session.runtime_state === "READY") {
          lines.push("  note=steerable thread registered; use governed session prompts to continue work");
        }
      }
    }
    lines.push("");
    lines.push("LIFECYCLE ACTIONS");
    lines.push("c = close selected WP governed sessions (role prompt)");
    lines.push("b = stop ACP broker if no governed runs are active");
    lines.push("r = refresh");
    lines.push("");
    lines.push("PACKET RUNTIME SESSIONS");
    if ((record.sessions || []).length === 0) {
      lines.push("No packet runtime sessions recorded.");
    } else {
      for (const session of record.sessions) {
        lines.push(`${session.role} | ${session.sessionId} | ${session.state} | ${session.authorityKind}${session.validatorRoleKind ? `/${session.validatorRoleKind}` : ""}`);
        lines.push(`  ${session.worktreeDir}`);
      }
    }
    if (packet?.runtimeValidationErrors?.length) {
      lines.push("");
      lines.push("RUNTIME VALIDATION ERRORS");
      for (const error of packet.runtimeValidationErrors.slice(0, 5)) lines.push(`- ${error}`);
    }
    if (packet?.receiptValidationErrors?.length) {
      lines.push("");
      lines.push("RECEIPT VALIDATION ERRORS");
      for (const error of packet.receiptValidationErrors.slice(0, 5)) lines.push(`- ${error}`);
    }
  }

  return lines.flatMap((line) => wrapText(line, width));
}

function buildDetailLinesRich(record, width, detailView, uiState) {
  if (!record) return ["No WP selected."];
  const packet = record.packetRecord;
  const runtime = packet?.runtime;
  const broker = record.brokerState || {};
  const docArtifacts = buildDocArtifacts(record);
  const commsArtifacts = buildCommsArtifacts(record);
  const docIndex = clampIndex(uiState.docArtifactIndex || 0, docArtifacts.length);
  const commsIndex = clampIndex(uiState.commsArtifactIndex || 0, commsArtifacts.length);
  const lines = [
    `${record.wpId} | board=${record.boardSection} | token=${record.boardToken}`,
    `packet=${packet?.packetKind || "UNKNOWN"} | packet_status=${packet?.packetStatus || "<none>"}`,
    `lane=${packet?.workflowLane || "<missing>"} | owner=${packet?.executionOwner || "<missing>"}`,
    `workflow=${packet?.workflowAuthority || "ORCHESTRATOR"} | tech=${packet?.technicalAuthority || "INTEGRATION_VALIDATOR"} | merge=${packet?.mergeAuthority || "INTEGRATION_VALIDATOR"}`,
    `wpval=${packet?.wpValidatorOfRecord || "<unassigned>"} | ival=${packet?.integrationValidatorOfRecord || "<unassigned>"}`,
    `branch=${packet?.localBranch || "<pending>"}`,
    `worktree=${packet?.localWorktreeDir || "<pending>"}`,
    `worktree_state=${packet?.localWorktreeStatus || "UNKNOWN"}${packet?.localWorktreeAbsPath ? ` | abs=${packet.localWorktreeAbsPath}` : ""}`,
    `board_source=${record.boardSource?.display || "<unknown>"}`,
    `selected_board_source=${record.selectedBoardSource || "CURRENT_WORKTREE"}`,
    `acp_broker=${broker.summary || "broker=<unknown>"}`,
  ];
  if (record.boardSource?.canonical_board_path) {
    lines.push(`canonical_board=${record.boardSource.canonical_board_path} | drift=${record.boardSource.board_drift || "UNKNOWN"}`);
  }
  lines.push(`canonical_entry=${formatBoardEntry(record.canonicalBoardEntry)} | current_entry=${formatBoardEntry(record.currentBoardEntry)}`);
  lines.push(`current_entry_drift=${record.currentBoardMatchesSelected === null ? "UNKNOWN" : (record.currentBoardMatchesSelected ? "NO" : "YES")}`);
  if (runtime) {
    lines.push(
      `runtime=${runtime.runtime_status}/${runtime.current_phase}`
      + ` | next=${formatTarget(runtime.next_expected_actor, runtime.next_expected_session) || runtime.next_expected_actor}`
      + ` | waiting_on=${runtime.waiting_on}${runtime.waiting_on_session ? ` (${runtime.waiting_on_session})` : ""}`
    );
    lines.push(`validator_trigger=${runtime.validator_trigger} | ready=${runtime.ready_for_validation ? "YES" : "NO"} | stale=${record.stale ? "YES" : "NO"} | open_reviews=${record.packetRecord?.openReviewItems?.length || 0}`);
    lines.push(communicationHealthLine(record));
  } else {
    lines.push("runtime=<none>");
  }
  lines.push("");

  if (detailView === "DOCS") {
    lines.push("DOCS");
    if (docArtifacts.length === 0) {
      lines.push("No document artifacts found for this WP.");
    } else {
      const selectedArtifact = docArtifacts[docIndex];
      lines.push(`sources=${docArtifacts.map((artifact, index) => index === docIndex ? `[${artifact.label}]` : artifact.label).join(" | ")}`);
      lines.push(`artifact=${selectedArtifact.label} | badge=${selectedArtifact.badge} | path=${selectedArtifact.path}`);
      lines.push("");
      lines.push(...selectedArtifact.lines);
    }
  } else if (detailView === "COMMS") {
    lines.push("COMMS");
    if (commsArtifacts.length === 0) {
      lines.push("No communication artifacts found for this WP.");
    } else {
      const selectedArtifact = commsArtifacts[commsIndex];
      lines.push(`sources=${commsArtifacts.map((artifact, index) => index === commsIndex ? `[${artifact.label}]` : artifact.label).join(" | ")}`);
      lines.push(`artifact=${selectedArtifact.label} | badge=${selectedArtifact.badge} | path=${selectedArtifact.path}`);
      lines.push("");
      lines.push(...selectedArtifact.lines);
    }
  } else if (detailView === "SESSIONS") {
    lines.push("SESSIONS");
    if ((record.registrySessions || []).length === 0) {
      lines.push("No governed sessions.");
    } else {
      for (const session of record.registrySessions) {
        const activeRun = (record.controlBrokerRuns || []).find((run) => run.role === session.role) || null;
        lines.push(`${session.role} | ${session.runtime_state} | host=${session.active_host || session.preferred_host}`);
        lines.push(`  thread=${session.session_thread_id || "<none>"} | cmd=${session.last_command_kind || "NONE"}/${session.last_command_status || "NONE"}`);
        lines.push(`  protocol=${session.control_protocol || "<none>"} | transport=${session.control_transport || "<none>"}`);
        if (session.requested_model) lines.push(`  model=${session.requested_model}${session.reasoning_config_value ? ` | reasoning=${session.reasoning_config_value}` : ""}`);
        if (session.last_command_summary) lines.push(`  ${session.last_command_summary}`);
        if (activeRun) lines.push(`  active_run=${activeRun.command_kind} started=${activeRun.started_at || "<no-ts>"} timeout=${activeRun.timeout_at || "<none>"}`);
        const sessionTokens = (record.tokenUsage?.commands || []).filter((cmd) => cmd.session_key === session.session_key);
        if (sessionTokens.length > 0) {
          const sessionInput = sessionTokens.reduce((sum, cmd) => sum + (cmd.usage_totals?.input_tokens || 0), 0);
          const sessionOutput = sessionTokens.reduce((sum, cmd) => sum + (cmd.usage_totals?.output_tokens || 0), 0);
          lines.push(`  tokens: ${sessionTokens.length} cmds | in=${formatTokenCount(sessionInput)} out=${formatTokenCount(sessionOutput)}`);
        }
      }
    }
    lines.push("");
    lines.push("PACKET RUNTIME SESSIONS");
    if ((record.sessions || []).length === 0) {
      lines.push("No packet runtime sessions recorded.");
    } else {
      for (const session of record.sessions) {
        lines.push(`${session.role} | ${session.sessionId} | ${session.state} | ${session.authorityKind}${session.validatorRoleKind ? `/${session.validatorRoleKind}` : ""}`);
        lines.push(`  ${session.worktreeDir}`);
      }
    }
    lines.push("");
    lines.push(`broker=${broker.summary || "<unknown>"}`);
  } else if (detailView === "CONTROL") {
    lines.push("CONTROL");
    if ((record.registrySessions || []).length === 0) {
      lines.push("No governed sessions.");
    } else {
      for (const session of record.registrySessions) {
        lines.push(`${session.role} | state=${session.runtime_state} | protocol=${session.control_protocol || "<none>"}`);
        lines.push(`  transport=${session.control_transport || "<none>"} | host=${session.active_host || session.preferred_host || "NONE"} | terminal=${session.active_terminal_kind || "NONE"}`);
        lines.push(`  thread=${session.session_thread_id || "<none>"} | cmd=${session.last_command_kind || "NONE"}/${session.last_command_status || "NONE"}`);
        if (session.last_command_summary) lines.push(`  ${session.last_command_summary}`);
      }
    }
    lines.push("");
    lines.push("RECENT REQUESTS");
    if ((record.controlRequests || []).length === 0) {
      lines.push("No control requests.");
    } else {
      for (const entry of (record.controlRequests || []).slice(-8)) {
        const pending = (record.pendingControlRequests || []).some((request) => request.command_id === entry.command_id) ? " | pending" : "";
        lines.push(`${entry.created_at} | ${entry.role} | ${entry.command_kind}${pending}`);
        lines.push(`  ${entry.summary || entry.prompt?.split(/\r?\n/, 1)[0] || "<no summary>"}`);
      }
    }
    lines.push("");
    lines.push("RECENT RESULTS");
    if ((record.controlResults || []).length === 0) {
      lines.push("No control results.");
    } else {
      for (const entry of (record.controlResults || []).slice(-8)) {
        lines.push(`${entry.processed_at} | ${entry.role} | ${entry.command_kind} | ${entry.status}`);
        lines.push(`  ${entry.summary || entry.error || "<no summary>"}`);
      }
    }
    lines.push("");
    lines.push("BROKER");
    if ((record.controlBrokerRuns || []).length === 0) {
      lines.push("No active broker runs.");
    } else {
      for (const run of record.controlBrokerRuns || []) {
        lines.push(`${run.started_at || "<no-ts>"} | ${run.role} | ${run.command_kind} | pid=${run.child_pid || 0}`);
        lines.push(`  timeout=${run.timeout_at || "<none>"} | reason=${run.termination_reason || "<none>"}`);
      }
    }
  } else if (detailView === "EVENTS") {
    lines.push("EVENTS");
    lines.push("Merged governed ACP output events for this WP.");
    if ((record.controlEventTimeline || []).length === 0) {
      lines.push("No control events.");
    } else {
      for (const entry of (record.controlEventTimeline || []).slice(-32)) {
        lines.push(`${entry.timestamp || "<no-ts>"} | ${entry.role || "<unknown>"} | ${entry.session_key || "<none>"}`);
        lines.push(`  ${summarizeControlEvent(entry)}`);
      }
    }
  } else if (detailView === "TIMELINE") {
    lines.push("TIMELINE");
    lines.push("Merged thread, receipts, control requests/results, and ACP events. Use detail focus + j/k to scroll.");
    const timeline = buildTimelineEntries(record);
    if (timeline.length === 0) {
      lines.push("No timeline entries.");
    } else {
      for (const entry of timeline) {
        lines.push(entry.header);
        for (const detailLine of entry.detailLines) lines.push(`  ${detailLine}`);
      }
    }
  } else {
    const terminalHistorySuppressed = isTerminalRecord(record);
    lines.push("OVERVIEW");
    lines.push(`focus=${uiState.focusedPane} | mode=${uiState.admin ? "ADMIN" : "VIEW"}`);
    lines.push(`docs=${docArtifacts.map((artifact) => artifact.label).join(", ") || "<none>"} | comms=${commsArtifacts.map((artifact) => artifact.label).join(", ") || "<none>"}`);
    lines.push("");
    lines.push("NEXT ACTION");
    lines.push(nextActionSummary(record));
    lines.push("");
    lines.push("ROLE LANES");
    lines.push(`  ${roleLaneChip(record, "CODER")}  ${roleLaneChip(record, "WP_VALIDATOR")}  ${roleLaneChip(record, "INTEGRATION_VALIDATOR")}  ${reviewPressureChip(record)}  ${communicationHealthChip(record)}`);
    lines.push("");
    const pendingNotifs = record.packetRecord?.pendingNotifications || { total: 0, byRole: {} };
    lines.push("PENDING NOTIFICATIONS");
    if (terminalHistorySuppressed) {
      lines.push("Hidden by default for terminal WP; use history/runtime artifacts when explicitly needed.");
    } else if (pendingNotifs.total === 0) {
      lines.push("No pending notifications.");
    } else {
      lines.push(`Total: ${paint(String(pendingNotifs.total), pendingNotifs.total >= 3 ? STATE_COLORS.blocked : "\x1b[38;5;208m", { bold: true })}`);
      for (const [roleName, count] of Object.entries(pendingNotifs.byRole)) {
        lines.push(`  ${paint(roleName, ROLE_COLORS[roleName] || STATUS_COLORS.OTHER)}: ${count} unread`);
      }
    }
    lines.push("");
    lines.push("OPEN REVIEW ITEMS");
    if (terminalHistorySuppressed) {
      lines.push("Hidden by default for terminal WP; use packet/runtime history when explicitly needed.");
    } else if ((record.packetRecord?.openReviewItems || []).length === 0) {
      lines.push("No open coder/validator review items.");
    } else {
      for (const item of (record.packetRecord?.openReviewItems || []).slice(0, 8)) {
        const target = formatTarget(item.target_role, item.target_session);
        lines.push(`${item.receipt_kind} | ${item.opened_by_role} -> ${target || "<unknown>"} | ${item.correlation_id}`);
        lines.push(`  ${item.summary}`);
        if (item.spec_anchor || item.packet_row_ref) {
          lines.push(`  spec=${item.spec_anchor || "<none>"} | packet=${item.packet_row_ref || "<none>"}`);
        }
        if (item.microtask_contract && typeof item.microtask_contract === "object") {
          if (item.microtask_contract.scope_ref) lines.push(`  microtask.scope_ref=${item.microtask_contract.scope_ref}`);
          if (Array.isArray(item.microtask_contract.file_targets) && item.microtask_contract.file_targets.length > 0) {
            lines.push(`  microtask.files=${item.microtask_contract.file_targets.join(", ")}`);
          }
          if (Array.isArray(item.microtask_contract.proof_commands) && item.microtask_contract.proof_commands.length > 0) {
            lines.push(`  microtask.proof=${item.microtask_contract.proof_commands.join(" ; ")}`);
          }
          if (item.microtask_contract.risk_focus) lines.push(`  microtask.risk=${item.microtask_contract.risk_focus}`);
          if (item.microtask_contract.expected_receipt_kind) lines.push(`  microtask.expected_receipt=${item.microtask_contract.expected_receipt_kind}`);
        }
      }
    }
    lines.push("");
    lines.push("LATEST REVIEW TRAFFIC");
    if (terminalHistorySuppressed) {
      lines.push("Hidden by default for terminal WP; use packet/runtime history when explicitly needed.");
    } else if ((record.packetRecord?.reviewReceipts || []).length === 0) {
      lines.push("No structured coder/validator review receipts.");
    } else {
      for (const entry of (record.packetRecord?.reviewReceipts || []).slice(-6)) {
        const target = formatTarget(entry.target_role, entry.target_session);
        lines.push(`${entry.timestamp_utc} | ${entry.receipt_kind} | ${entry.actor_role}${target ? ` -> ${target}` : ""}`);
        lines.push(`  ${entry.summary}`);
      }
    }
    lines.push("");
    lines.push("GOVERNED SESSIONS");
    if ((record.registrySessions || []).length === 0) {
      lines.push("No governed sessions.");
    } else {
      for (const session of record.registrySessions) {
        const activeRun = (record.controlBrokerRuns || []).find((run) => run.role === session.role) || null;
        lines.push(`${session.role} | ${session.runtime_state} | host=${session.active_host || session.preferred_host}`);
        lines.push(`  req=${session.plugin_request_count} fail=${session.plugin_failure_count} last=${session.plugin_last_result}`);
        if (session.session_thread_id) {
          lines.push(`  thread=${session.session_thread_id} cmd=${session.last_command_kind}/${session.last_command_status}`);
          if (session.control_protocol) lines.push(`  protocol=${session.control_protocol} transport=${session.control_transport || "<none>"}`);
        }
        if (session.runtime_state === "CLOSED") lines.push("  note=session thread registration cleared; start-session is required before steering again");
        if (activeRun) lines.push(`  active_run=${activeRun.command_kind} started=${activeRun.started_at || "<no-ts>"} timeout=${activeRun.timeout_at || "<none>"}`);
        if (session.runtime_state === "TERMINAL_COMMAND_DISPATCHED" || session.runtime_state === "PLUGIN_CONFIRMED") {
          lines.push("  note=terminal dispatch only; wait for receipts/heartbeat/runtime movement");
        } else if (session.runtime_state === "READY") {
          lines.push("  note=steerable thread registered; use governed session prompts to continue work");
        }
      }
    }
    lines.push("");
    lines.push("TOKEN USAGE");
    if (!record.tokenUsage || (record.tokenUsage.summary?.command_count || 0) === 0) {
      lines.push("No token usage recorded.");
    } else {
      const summary = record.tokenUsage.summary || {};
      const totals = summary.usage_totals || {};
      const budget = record.tokenBudget || {};
      const settlementStatus = record.tokenUsage.settlement?.status || "UNSETTLED";
      lines.push(`total: ${summary.command_count || 0} cmds, ${summary.turn_count || 0} turns | input=${formatTokenCount(totals.input_tokens || 0)} cached=${formatTokenCount(totals.cached_input_tokens || 0)} output=${formatTokenCount(totals.output_tokens || 0)}`);
      if (budget.status) {
        const budgetColor = { PASS: STATE_COLORS.ready, WARN: STATE_COLORS.waiting, FAIL: STATE_COLORS.blocked }[budget.status] || "";
        lines.push(`budget: ${paint(budget.status, budgetColor, { bold: budget.status !== "PASS" })}${budget.summary ? ` | ${budget.summary}` : ""} | settlement=${settlementStatus}`);
      }
      const roleTotals = record.tokenUsage.role_totals || {};
      for (const roleName of ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]) {
        const roleData = roleTotals[roleName];
        if (!roleData || (roleData.command_count || 0) === 0) continue;
        const roleUsage = roleData.usage_totals || {};
        const roleBudget = budget.roles?.[roleName];
        const roleBudgetLabel = roleBudget ? ` [${roleBudget.status}]` : "";
        lines.push(`  ${paint(roleName, ROLE_COLORS[roleName] || STATUS_COLORS.OTHER)}: ${roleData.command_count} cmds, ${roleData.turn_count} turns | in=${formatTokenCount(roleUsage.input_tokens || 0)} out=${formatTokenCount(roleUsage.output_tokens || 0)}${roleBudgetLabel}`);
      }
    }
    lines.push("");
    lines.push("LATEST ACTIVITY");
    const latestTimeline = buildTimelineEntries(record).slice(-6);
    if (latestTimeline.length === 0) {
      lines.push("No recent activity.");
    } else {
      for (const entry of latestTimeline) {
        lines.push(entry.header);
        for (const detailLine of entry.detailLines.slice(0, 1)) lines.push(`  ${detailLine}`);
      }
    }
  }

  if (detailView === "DOCS" || detailView === "COMMS" || detailView === "TIMELINE") return lines;
  return lines.flatMap((line) => wrapText(line, width));
}

function renderScreen(model, uiState) {
  const records = model.records;
  const filtered = filterRecords(records, uiState.filter);
  if (uiState.selectedIndex >= filtered.length) uiState.selectedIndex = Math.max(0, filtered.length - 1);
  const selected = filtered[uiState.selectedIndex] || null;
  const columns = process.stdout.columns || 120;
  const rows = process.stdout.rows || 35;
  const leftWidth = Math.max(42, Math.floor(columns * 0.44));
  const rightWidth = Math.max(40, columns - leftWidth - 3);
  const counts = FILTERS.map((filter) => {
    const label = `${filter}:${filterRecords(records, filter).length}`;
    return paint(label, STATUS_COLORS[filter] || STATUS_COLORS.OTHER, { bold: filter === uiState.filter });
  }).join("  ");
  const summaryLines = buildTopSummary(records, filtered, selected, model, uiState);
  const help = uiState.admin
    ? "tab focus  j/k move-scroll  h/l source  [/ ] filter  1 overview 2 docs 3 comms 4 sessions 5 timeline 6 control 7 events  c close  b broker-stop  m message  r refresh  q quit"
    : "tab focus  j/k move-scroll  h/l source  [/ ] filter  1 overview 2 docs 3 comms 4 sessions 5 timeline 6 control 7 events  r refresh  q quit";
  const chromeLines = [...summaryLines, counts, help, "-".repeat(columns)];
  const footerLineCount = 2;
  const bodyHeight = Math.max(12, rows - chromeLines.length - footerLineCount);

  const leftLines = renderList(filtered, uiState.selectedIndex, leftWidth, bodyHeight, uiState.focusedPane === "LIST");
  const detailLines = buildDetailLinesRich(selected, rightWidth, uiState.detailView, uiState);
  const detailStart = Math.max(0, Math.min(uiState.detailScroll, Math.max(0, detailLines.length - bodyHeight)));
  const rightLines = detailLines.slice(detailStart, detailStart + bodyHeight);
  while (rightLines.length < bodyHeight) rightLines.push("");

  const frame = [...chromeLines];
  for (let index = 0; index < bodyHeight; index += 1) {
    frame.push(`${leftLines[index]} | ${truncateVisible(rightLines[index], rightWidth)}`);
  }
  frame.push("-".repeat(columns));
  const status = uiState.statusMessage ? ` | ${uiState.statusMessage}` : "";
  frame.push(selected ? `Selected: ${selected.wpId} | packet=${selected.packetPath || "<none>"} | last_activity=${selected.lastActivityAt || "n/a"} | detail_scroll=${detailStart}${status}` : `Selected: none${status}`);
  return frame.join(os.EOL);
}

function renderOnce(model, options) {
  const uiState = {
    filter: options.filter,
    selectedIndex: 0,
    detailView: options.detailView,
    actorRole: options.actorRole,
    actorSession: options.actorSession,
    admin: options.admin,
    focusedPane: "LIST",
    detailScroll: 0,
    docArtifactIndex: 0,
    commsArtifactIndex: 0,
    statusMessage: "",
  };
  const filtered = filterRecords(model.records, uiState.filter);
  if (options.wpId) {
    const index = filtered.findIndex((record) => record.wpId === options.wpId);
    if (index >= 0) uiState.selectedIndex = index;
  }
  console.log(renderScreen(model, uiState));
}

async function promptForMessage(selectedRecord, uiState) {
  if (!selectedRecord || !selectedRecord.packetRecord?.threadPath) return null;
  if (process.stdin.isTTY) process.stdin.setRawMode(false);
  process.stdout.write("\x1b[?25h");
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  const answer = await new Promise((resolve) => rl.question(`message ${selectedRecord.wpId}> `, resolve));
  rl.close();
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  process.stdout.write("\x1b[?25l");
  const trimmed = String(answer || "").trim();
  if (!trimmed) return null;
  const targetMatch = trimmed.match(/^(@\S+)\s+([\s\S]+)$/);
  return appendWpThreadEntry({
    wpId: selectedRecord.wpId,
    actorRole: uiState.actorRole,
    actorSession: uiState.actorSession,
    message: targetMatch ? targetMatch[2] : trimmed,
    target: targetMatch ? targetMatch[1] : "",
  });
}

async function promptForInput(label) {
  if (process.stdin.isTTY) process.stdin.setRawMode(false);
  process.stdout.write("\x1b[?25h");
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  const answer = await new Promise((resolve) => rl.question(label, resolve));
  rl.close();
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  process.stdout.write("\x1b[?25l");
  return String(answer || "").trim();
}

function runJustCommand(args) {
  return execFileSync("just", args, {
    cwd: CURRENT_WORKTREE_DIR,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

async function closeSelectedSessions(selectedRecord) {
  if (!selectedRecord) return "No WP selected.";
  const availableRoles = [...new Set((selectedRecord.registrySessions || []).map((session) => session.role))];
  const roleAnswer = (await promptForInput(`close role for ${selectedRecord.wpId} (${availableRoles.join("/")}/ALL)> `)).toUpperCase();
  if (!roleAnswer) return "Close canceled.";
  const targetRoles = roleAnswer === "ALL"
    ? availableRoles
    : (availableRoles.includes(roleAnswer) ? [roleAnswer] : []);
  if (targetRoles.length === 0) {
    return `No governed session matches role selection ${roleAnswer}.`;
  }
  const confirm = (await promptForInput(`confirm close ${selectedRecord.wpId} ${targetRoles.join(",")} [y/N]> `)).toLowerCase();
  if (confirm !== "y" && confirm !== "yes") return "Close canceled.";
  const outcomes = [];
  for (const role of targetRoles) {
    try {
      const output = runJustCommand(["session-close", role, selectedRecord.wpId]);
      outcomes.push(`${role}: ${output.split(/\r?\n/).slice(-2).join(" | ")}`);
    } catch (error) {
      const stderr = String(error.stderr || error.message || "close failed").trim();
      outcomes.push(`${role}: FAILED ${stderr.split(/\r?\n/).slice(-1)[0]}`);
    }
  }
  return outcomes.join(" || ");
}

async function stopBrokerFromMonitor() {
  const confirm = (await promptForInput("stop ACP broker if no governed runs are active [y/N]> ")).toLowerCase();
  if (confirm !== "y" && confirm !== "yes") return "Broker stop canceled.";
  try {
    const output = runJustCommand(["handshake-acp-broker-stop"]);
    return output.split(/\r?\n/).join(" | ");
  } catch (error) {
    const stderr = String(error.stderr || error.message || "broker stop failed").trim();
    return `BROKER STOP FAILED: ${stderr.split(/\r?\n/).slice(-1)[0]}`;
  }
}

function resetDetailViewport(uiState) {
  uiState.detailScroll = 0;
}

function setDetailView(uiState, nextView) {
  uiState.detailView = nextView;
  resetDetailViewport(uiState);
}

function cycleFocusedArtifact(record, uiState, delta) {
  if (!record) return;
  if (uiState.detailView === "DOCS") {
    const artifacts = buildDocArtifacts(record);
    if (artifacts.length === 0) return;
    uiState.docArtifactIndex = clampIndex((uiState.docArtifactIndex || 0) + delta, artifacts.length);
    resetDetailViewport(uiState);
  } else if (uiState.detailView === "COMMS") {
    const artifacts = buildCommsArtifacts(record);
    if (artifacts.length === 0) return;
    uiState.commsArtifactIndex = clampIndex((uiState.commsArtifactIndex || 0) + delta, artifacts.length);
    resetDetailViewport(uiState);
  }
}

async function runInteractive(options) {
  if (!process.stdout.isTTY || !process.stdin.isTTY) {
    renderOnce(loadMonitorModel(), options);
    return;
  }

  const uiState = {
    filter: options.filter,
    selectedIndex: 0,
    detailView: options.detailView,
    actorRole: options.actorRole,
    actorSession: options.actorSession,
    admin: options.admin,
    focusedPane: "LIST",
    detailScroll: 0,
    docArtifactIndex: 0,
    commsArtifactIndex: 0,
    statusMessage: "",
  };

  let records = loadMonitorModel();
  let lastFrame = "";
  const refresh = () => {
    records = loadMonitorModel();
    const frame = renderScreen(records, uiState);
    if (frame === lastFrame) return;
    lastFrame = frame;
    process.stdout.write("\x1b[H\x1b[J");
    process.stdout.write(frame);
  };

  process.stdout.write("\x1b[?1049h\x1b[?25l\x1b[H\x1b[J");
  process.stdin.setRawMode(true);
  process.stdin.resume();
  process.stdin.setEncoding("utf8");
  refresh();

  const timer = setInterval(refresh, options.refreshMs);
  const cleanup = () => {
    clearInterval(timer);
    if (process.stdin.isTTY) process.stdin.setRawMode(false);
    process.stdout.write("\x1b[?25h\x1b[?1049l");
  };

  process.on("SIGINT", () => {
    cleanup();
    process.exit(0);
  });

  process.stdin.on("data", async (chunk) => {
    const key = String(chunk || "");
    const filtered = filterRecords(records.records, uiState.filter);
    const selected = filtered[uiState.selectedIndex] || null;
    if (key === "q") {
      cleanup();
      process.exit(0);
    } else if (key === "\t") {
      uiState.focusedPane = uiState.focusedPane === "LIST" ? "DETAIL" : "LIST";
      refresh();
    } else if (key === "j" || key === "\u001b[B") {
      if (uiState.focusedPane === "LIST") {
        uiState.selectedIndex = Math.min(filtered.length - 1, uiState.selectedIndex + 1);
        resetDetailViewport(uiState);
      } else {
        uiState.detailScroll += 1;
      }
      refresh();
    } else if (key === "k" || key === "\u001b[A") {
      if (uiState.focusedPane === "LIST") {
        uiState.selectedIndex = Math.max(0, uiState.selectedIndex - 1);
        resetDetailViewport(uiState);
      } else {
        uiState.detailScroll = Math.max(0, uiState.detailScroll - 1);
      }
      refresh();
    } else if (key === "h" || key === "\u001b[D") {
      if (uiState.focusedPane === "DETAIL") {
        cycleFocusedArtifact(selected, uiState, -1);
        refresh();
      }
    } else if (key === "l" || key === "\u001b[C") {
      if (uiState.focusedPane === "DETAIL") {
        cycleFocusedArtifact(selected, uiState, 1);
        refresh();
      }
    } else if (key === "]") {
      if (uiState.focusedPane === "LIST") {
        const index = FILTERS.indexOf(uiState.filter);
        uiState.filter = FILTERS[(index + 1) % FILTERS.length];
        uiState.selectedIndex = 0;
        resetDetailViewport(uiState);
        refresh();
      }
    } else if (key === "[") {
      if (uiState.focusedPane === "LIST") {
        const index = FILTERS.indexOf(uiState.filter);
        uiState.filter = FILTERS[(index - 1 + FILTERS.length) % FILTERS.length];
        uiState.selectedIndex = 0;
        resetDetailViewport(uiState);
        refresh();
      }
    } else if (key === "1") {
      setDetailView(uiState, "OVERVIEW");
      refresh();
    } else if (key === "2") {
      setDetailView(uiState, "DOCS");
      refresh();
    } else if (key === "3") {
      setDetailView(uiState, "COMMS");
      refresh();
    } else if (key === "4") {
      setDetailView(uiState, "SESSIONS");
      refresh();
    } else if (key === "5") {
      setDetailView(uiState, "TIMELINE");
      refresh();
    } else if (key === "6") {
      setDetailView(uiState, "CONTROL");
      refresh();
    } else if (key === "7") {
      setDetailView(uiState, "EVENTS");
      refresh();
    } else if (key === "r") {
      uiState.statusMessage = "";
      refresh();
    } else if (uiState.admin && key === "c") {
      uiState.statusMessage = await closeSelectedSessions(selected);
      records = loadMonitorModel();
      refresh();
    } else if (uiState.admin && key === "b") {
      uiState.statusMessage = await stopBrokerFromMonitor();
      records = loadMonitorModel();
      refresh();
    } else if (uiState.admin && key === "m") {
      await promptForMessage(selected, uiState);
      records = loadMonitorModel();
      refresh();
    }
  });
}

export async function main() {
  const options = parseArgs(process.argv.slice(2));
  const records = loadMonitorModel();
  if (options.once) {
    renderOnce(records, options);
    return;
  }
  await runInteractive(options);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`[OPERATOR_MONITOR] ${error.message}`);
    process.exit(1);
  });
}
