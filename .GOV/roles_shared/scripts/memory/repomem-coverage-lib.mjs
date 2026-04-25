import fs from "node:fs";

import { REPO_ROOT, repoPathAbs, resolveWorkPacketPath } from "../lib/runtime-paths.mjs";
import { parseJsonlFile } from "../lib/wp-communications-lib.mjs";
import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionRegistry,
} from "../session/session-registry-lib.mjs";
import { parseThreadEntriesText } from "../session/wp-timeline-lib.mjs";
import { closeDb, openGovernanceMemoryDb } from "./governance-memory-lib.mjs";

const REPOMEM_TRACKED_ACTIVITY_ROLES = new Set([
  "ORCHESTRATOR",
  "CLASSIC_ORCHESTRATOR",
  "ACTIVATION_MANAGER",
  "CODER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "VALIDATOR",
]);

const ROLE_ORDER = [
  "ORCHESTRATOR",
  "CLASSIC_ORCHESTRATOR",
  "ACTIVATION_MANAGER",
  "CODER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "VALIDATOR",
];

export const REPOMEM_DURABLE_CHECKPOINT_TYPES = [
  "INSIGHT",
  "DECISION",
  "ERROR",
  "ABANDON",
  "CONCERN",
  "ESCALATION",
  "RESEARCH_CLOSE",
];

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function compareRoles(left = "", right = "") {
  const leftIndex = ROLE_ORDER.indexOf(left);
  const rightIndex = ROLE_ORDER.indexOf(right);
  if (leftIndex !== -1 || rightIndex !== -1) {
    if (leftIndex === -1) return 1;
    if (rightIndex === -1) return -1;
    return leftIndex - rightIndex;
  }
  return left.localeCompare(right);
}

function isTrackedActivityRole(role = "") {
  return REPOMEM_TRACKED_ACTIVITY_ROLES.has(normalizeRole(role));
}

function formatList(values = [], fallback = "none") {
  const normalized = (Array.isArray(values) ? values : [])
    .map((value) => String(value || "").trim())
    .filter(Boolean);
  return normalized.length > 0 ? normalized.join(",") : fallback;
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function packetAbsolutePathForWp(wpId = "") {
  const resolvedPacket = resolveWorkPacketPath(wpId);
  if (!resolvedPacket) return "";
  if (resolvedPacket.packetAbsPath && fs.existsSync(resolvedPacket.packetAbsPath)) {
    return resolvedPacket.packetAbsPath;
  }
  const packetPath = resolvedPacket.packetPath || resolvedPacket;
  const packetAbsPath = repoPathAbs(packetPath);
  return fs.existsSync(packetAbsPath) ? packetAbsPath : "";
}

function loadPacketContentForWp(wpId = "") {
  if (!String(wpId || "").trim()) return "";
  const packetAbsPath = packetAbsolutePathForWp(wpId);
  if (!packetAbsPath) return "";
  return fs.readFileSync(packetAbsPath, "utf8");
}

function normalizeWpEntries(entries = [], wpId = "", fieldName = "wp_id") {
  const normalizedWpId = String(wpId || "").trim();
  return (Array.isArray(entries) ? entries : [])
    .filter((entry) => String(entry?.[fieldName] || "").trim() === normalizedWpId);
}

function readPacketArtifacts(packetContent = "") {
  return {
    receiptsPath: parseSingleField(packetContent, "WP_RECEIPTS_FILE"),
    threadPath: parseSingleField(packetContent, "WP_THREAD_FILE"),
  };
}

function loadThreadEntries(threadPath = "") {
  if (!threadPath) return [];
  const threadAbsPath = repoPathAbs(threadPath);
  if (!fs.existsSync(threadAbsPath)) return [];
  return parseThreadEntriesText(fs.readFileSync(threadAbsPath, "utf8"));
}

function loadReceipts(receiptsPath = "") {
  if (!receiptsPath) return [];
  const receiptsAbsPath = repoPathAbs(receiptsPath);
  if (!fs.existsSync(receiptsAbsPath)) return [];
  return parseJsonlFile(receiptsAbsPath);
}

function addRoleActivity(roleActivity, role, source) {
  const normalizedRole = normalizeRole(role);
  if (!isTrackedActivityRole(normalizedRole)) return;
  if (!roleActivity.has(normalizedRole)) {
    roleActivity.set(normalizedRole, {
      role: normalizedRole,
      activity_sources: [],
      activity_source_count: 0,
    });
  }
  const detail = roleActivity.get(normalizedRole);
  if (!detail.activity_sources.includes(source)) {
    detail.activity_sources.push(source);
    detail.activity_source_count = detail.activity_sources.length;
  }
}

function isAutoClosedSessionClose(entry = {}) {
  if (normalizeRole(entry.checkpoint_type) !== "SESSION_CLOSE") return false;
  const combined = [
    entry.topic,
    entry.content,
    entry.decisions,
  ].join(" ").toLowerCase();
  return combined.includes("auto-closed");
}

function loadConversationEntriesForWp({
  db = null,
  wpId = "",
  activeRoles = [],
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  const normalizedActiveRoles = (Array.isArray(activeRoles) ? activeRoles : [])
    .map((role) => normalizeRole(role))
    .filter(Boolean);
  if (!normalizedWpId || normalizedActiveRoles.length === 0) return [];

  let openedDb = null;
  const database = db || (() => {
    openedDb = openGovernanceMemoryDb();
    return openedDb.db;
  })();
  try {
    const rolePlaceholders = normalizedActiveRoles.map(() => "?").join(",");
    const sql = `SELECT *
      FROM conversation_log
      WHERE (wp_id = ? OR wp_id = '')
        AND role IN (${rolePlaceholders})
      ORDER BY timestamp_utc ASC`;
    return database.prepare(sql).all(normalizedWpId, ...normalizedActiveRoles);
  } finally {
    if (openedDb?.db) {
      closeDb(openedDb.db);
    }
  }
}

function buildRoleSessionCoverage({
  wpId = "",
  activity = {},
  sessionEntries = new Map(),
} = {}) {
  const allSessions = [...sessionEntries.values()]
    .sort((left, right) => String(left.session_id || "").localeCompare(String(right.session_id || "")));
  const openSessionIds = allSessions.filter((entry) => entry.has_open).map((entry) => entry.session_id);
  const explicitCloseSessionIds = allSessions.filter((entry) => entry.has_explicit_close).map((entry) => entry.session_id);
  const wpDurableSessionIds = allSessions.filter((entry) => entry.has_wp_durable).map((entry) => entry.session_id);
  const qualifyingSessionIds = allSessions
    .filter((entry) => entry.has_open && entry.has_explicit_close && entry.has_wp_durable)
    .map((entry) => entry.session_id);

  const debtKeys = [];
  if (qualifyingSessionIds.length === 0) {
    if (openSessionIds.length === 0) debtKeys.push("NO_SESSION_OPEN");
    if (explicitCloseSessionIds.length === 0) debtKeys.push("NO_SESSION_CLOSE");
    if (wpDurableSessionIds.length === 0) debtKeys.push("NO_WP_DURABLE_CHECKPOINT");
    if (debtKeys.length === 0) debtKeys.push("FRAGMENTED_SESSION_PROOF");
  }

  const status = qualifyingSessionIds.length > 0 ? "PASS" : "DEBT";
  const summary = status === "PASS"
    ? [
        `qualifying_session=${qualifyingSessionIds[0]}`,
        `sources=${formatList(activity.activity_sources)}`,
        `wp=${wpId}`,
      ].join(" | ")
    : [
        `sources=${formatList(activity.activity_sources)}`,
        `debt=${formatList(debtKeys)}`,
        `sessions=${allSessions.length}`,
      ].join(" | ");

  return {
    role: activity.role,
    status,
    activity_sources: activity.activity_sources,
    activity_source_count: activity.activity_source_count,
    observed_session_count: allSessions.length,
    qualifying_session_ids: qualifyingSessionIds,
    open_session_ids: openSessionIds,
    explicit_close_session_ids: explicitCloseSessionIds,
    wp_durable_session_ids: wpDurableSessionIds,
    debt_keys: debtKeys,
    summary,
  };
}

export function collectWpRepomemActivity({
  receipts = [],
  threadEntries = [],
  sessions = [],
  controlRequests = [],
  controlResults = [],
} = {}) {
  const roleActivity = new Map();

  for (const entry of Array.isArray(receipts) ? receipts : []) {
    addRoleActivity(roleActivity, entry?.actor_role, "receipt");
  }
  for (const entry of Array.isArray(threadEntries) ? threadEntries : []) {
    addRoleActivity(roleActivity, entry?.actorRole, "thread");
  }
  for (const entry of Array.isArray(sessions) ? sessions : []) {
    addRoleActivity(roleActivity, entry?.role, "session_registry");
  }
  for (const entry of Array.isArray(controlRequests) ? controlRequests : []) {
    addRoleActivity(roleActivity, entry?.role, "control_request");
  }
  for (const entry of Array.isArray(controlResults) ? controlResults : []) {
    addRoleActivity(roleActivity, entry?.role, "control_result");
  }

  return [...roleActivity.values()].sort((left, right) => compareRoles(left.role, right.role));
}

export function loadWpRepomemCoverageInputs({
  repoRoot = REPO_ROOT,
  wpId = "",
  packetContent = "",
  receipts = null,
  threadEntries = null,
  sessions = null,
  controlRequests = null,
  controlResults = null,
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  const resolvedPacketContent = String(packetContent || "").trim() || loadPacketContentForWp(normalizedWpId);
  const packetArtifacts = readPacketArtifacts(resolvedPacketContent);

  const resolvedReceipts = Array.isArray(receipts)
    ? normalizeWpEntries(receipts, normalizedWpId)
    : normalizeWpEntries(loadReceipts(packetArtifacts.receiptsPath), normalizedWpId);
  const resolvedThreadEntries = Array.isArray(threadEntries)
    ? threadEntries
    : loadThreadEntries(packetArtifacts.threadPath);
  const resolvedSessions = Array.isArray(sessions)
    ? normalizeWpEntries(sessions, normalizedWpId)
    : normalizeWpEntries(loadSessionRegistry(repoRoot).registry?.sessions, normalizedWpId);
  const resolvedControlRequests = Array.isArray(controlRequests)
    ? normalizeWpEntries(controlRequests, normalizedWpId)
    : normalizeWpEntries(loadSessionControlRequests(repoRoot).requests, normalizedWpId);
  const resolvedControlResults = Array.isArray(controlResults)
    ? normalizeWpEntries(controlResults, normalizedWpId)
    : normalizeWpEntries(loadSessionControlResults(repoRoot).results, normalizedWpId);

  return {
    wp_id: normalizedWpId,
    packet_content: resolvedPacketContent,
    receipts: resolvedReceipts,
    thread_entries: resolvedThreadEntries,
    sessions: resolvedSessions,
    control_requests: resolvedControlRequests,
    control_results: resolvedControlResults,
  };
}

export function evaluateWpRepomemCoverage({
  repoRoot = REPO_ROOT,
  wpId = "",
  packetContent = "",
  receipts = null,
  threadEntries = null,
  sessions = null,
  controlRequests = null,
  controlResults = null,
  conversationEntries = null,
  db = null,
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  const inputs = loadWpRepomemCoverageInputs({
    repoRoot,
    wpId: normalizedWpId,
    packetContent,
    receipts,
    threadEntries,
    sessions,
    controlRequests,
    controlResults,
  });
  const activity = collectWpRepomemActivity({
    receipts: inputs.receipts,
    threadEntries: inputs.thread_entries,
    sessions: inputs.sessions,
    controlRequests: inputs.control_requests,
    controlResults: inputs.control_results,
  });
  const activeRoles = activity.map((entry) => entry.role);
  const resolvedConversationEntries = Array.isArray(conversationEntries)
    ? conversationEntries
    : loadConversationEntriesForWp({
        db,
        wpId: normalizedWpId,
        activeRoles,
      });

  const sessionCoverageByRole = new Map();
  for (const entry of Array.isArray(resolvedConversationEntries) ? resolvedConversationEntries : []) {
    const role = normalizeRole(entry?.role);
    const sessionId = String(entry?.session_id || "").trim();
    if (!activeRoles.includes(role) || !sessionId) continue;
    if (!sessionCoverageByRole.has(role)) {
      sessionCoverageByRole.set(role, new Map());
    }
    const roleSessions = sessionCoverageByRole.get(role);
    if (!roleSessions.has(sessionId)) {
      roleSessions.set(sessionId, {
        session_id: sessionId,
        has_open: false,
        has_explicit_close: false,
        has_wp_durable: false,
      });
    }
    const roleSession = roleSessions.get(sessionId);
    const checkpointType = normalizeRole(entry?.checkpoint_type);
    if (checkpointType === "SESSION_OPEN") {
      roleSession.has_open = true;
    }
    if (checkpointType === "SESSION_CLOSE" && !isAutoClosedSessionClose(entry)) {
      roleSession.has_explicit_close = true;
    }
    if (
      REPOMEM_DURABLE_CHECKPOINT_TYPES.includes(checkpointType)
      && String(entry?.wp_id || "").trim() === normalizedWpId
    ) {
      roleSession.has_wp_durable = true;
    }
  }

  const roleDetails = activity.map((entry) => buildRoleSessionCoverage({
    wpId: normalizedWpId,
    activity: entry,
    sessionEntries: sessionCoverageByRole.get(entry.role) || new Map(),
  }));
  const debtRoles = roleDetails.filter((entry) => entry.status === "DEBT").map((entry) => entry.role);
  const debtKeys = roleDetails.flatMap((entry) =>
    entry.debt_keys.map((debtKey) => `${entry.role}:${debtKey}`)
  );
  const state = roleDetails.length === 0
    ? "NO_ACTIVE_ROLES"
    : debtRoles.length === 0
      ? "PASS"
      : "DEBT";

  return {
    schema_id: "hsk.repomem_coverage@1",
    schema_version: "repomem_coverage_v1",
    wp_id: normalizedWpId,
    state,
    active_roles: roleDetails.map((entry) => entry.role),
    debt_roles: debtRoles,
    debt_keys: debtKeys,
    role_details: roleDetails,
    summary: [
      `state=${state}`,
      `active_roles=${formatList(roleDetails.map((entry) => entry.role))}`,
      `debt_roles=${formatList(debtRoles)}`,
      `debt_keys=${formatList(debtKeys)}`,
    ].join(" | "),
  };
}

function parseTimestampMs(value) {
  const date = new Date(String(value || "").trim());
  return Number.isNaN(date.getTime()) ? null : date.getTime();
}

function hasRecentTimestamp(values = [], sinceMs = null) {
  if (sinceMs === null) return true;
  return values.some((value) => {
    const timestampMs = parseTimestampMs(value);
    return timestampMs !== null && timestampMs >= sinceMs;
  });
}

export function collectRecentRepomemCoverageWpIds({
  sessions = [],
  controlRequests = [],
  controlResults = [],
  sinceDate = "",
} = {}) {
  const sinceMs = String(sinceDate || "").trim()
    ? parseTimestampMs(sinceDate)
    : null;
  const wpIds = new Set();
  const addWp = (wpId, timestamps = []) => {
    const normalizedWpId = String(wpId || "").trim();
    if (!normalizedWpId.startsWith("WP-")) return;
    if (!hasRecentTimestamp(timestamps, sinceMs)) return;
    wpIds.add(normalizedWpId);
  };

  for (const session of Array.isArray(sessions) ? sessions : []) {
    addWp(session?.wp_id, [session?.updated_at, session?.created_at, session?.started_at]);
  }
  for (const request of Array.isArray(controlRequests) ? controlRequests : []) {
    addWp(request?.wp_id, [request?.created_at, request?.timestamp, request?.requested_at]);
  }
  for (const result of Array.isArray(controlResults) ? controlResults : []) {
    addWp(result?.wp_id, [result?.completed_at, result?.created_at, result?.timestamp]);
  }

  return [...wpIds].sort((left, right) => left.localeCompare(right));
}
