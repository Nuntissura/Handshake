import fs from "node:fs";
import path from "node:path";
import {
  appendJsonlLine,
  loadSessionRegistry,
  parseJsonlFile,
} from "./session-registry-lib.mjs";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  REPO_ROOT,
  normalizePath,
} from "../lib/runtime-paths.mjs";

export const SESSION_EVENT_SCHEMA_ID = "hsk.session_event@1";
export const SESSION_EVENT_SCHEMA_VERSION = "session_event_v1";
export const PREDECESSOR_SUMMARY_SCHEMA_ID = "hsk.predecessor_summary@1";
export const PREDECESSOR_SUMMARY_VERSION = "predecessor_summary_v1";

function nowIso() {
  return new Date().toISOString();
}

function normalizeRole(value = "") {
  return String(value || "").trim().toUpperCase();
}

function normalizeWpId(value = "") {
  return String(value || "").trim();
}

function normalizeSessionId(value = "") {
  return String(value || "").trim();
}

function sessionIdentityKey(value = "") {
  return normalizeSessionId(value).toLowerCase();
}

function sanitizePathSegment(value = "") {
  return String(value || "")
    .trim()
    .replace(/[^A-Za-z0-9._-]+/g, "_")
    .replace(/^_+|_+$/g, "")
    || "session";
}

function compactText(value = "", limit = 160) {
  const text = String(value || "").replace(/\s+/g, " ").trim();
  if (text.length <= limit) return text;
  return `${text.slice(0, Math.max(0, limit - 3)).trimEnd()}...`;
}

function tokenCountApprox(text = "") {
  return String(text || "").trim().split(/\s+/).filter(Boolean).length;
}

function timestampScore(record = {}) {
  const fields = [
    "closed_at",
    "closed_at_utc",
    "last_command_completed_at",
    "last_event_at",
    "last_heartbeat_at",
    "session_thread_started_at",
    "plugin_last_request_at",
    "updated_at",
    "created_at",
  ];
  for (const field of fields) {
    const parsed = Date.parse(String(record?.[field] || ""));
    if (!Number.isNaN(parsed)) return parsed;
  }
  return 0;
}

function resolveMaybeRelativePath(filePath = "", { repoRoot = REPO_ROOT } = {}) {
  const text = String(filePath || "").trim();
  if (!text) return "";
  return path.isAbsolute(text) ? path.resolve(text) : path.resolve(repoRoot, text);
}

export function sessionEventsFile({
  wpId,
  sessionId,
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const WP_ID = normalizeWpId(wpId);
  const SESSION_ID = normalizeSessionId(sessionId);
  if (!WP_ID || !SESSION_ID) return "";
  return path.resolve(
    runtimeRootAbs,
    "roles_shared",
    "WP_SESSIONS",
    sanitizePathSegment(WP_ID),
    sanitizePathSegment(SESSION_ID),
    "events.jsonl",
  );
}

function legacySessionEventsFile({
  wpId,
  sessionId,
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const WP_ID = normalizeWpId(wpId);
  const SESSION_ID = normalizeSessionId(sessionId);
  if (!WP_ID || !SESSION_ID) return "";
  return path.resolve(
    runtimeRootAbs,
    sanitizePathSegment(WP_ID),
    "sessions",
    sanitizePathSegment(SESSION_ID),
    "events.jsonl",
  );
}

function sessionEventCandidateFiles(session = {}, {
  wpId,
  role,
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
  repoRoot = REPO_ROOT,
} = {}) {
  const candidates = [];
  for (const field of ["session_events_file", "events_jsonl_file", "predecessor_events_file"]) {
    const resolved = resolveMaybeRelativePath(session?.[field], { repoRoot });
    if (resolved) candidates.push(resolved);
  }
  const ids = [
    session?.session_id,
    session?.session_key,
    session?.session_thread_id,
    `${normalizeRole(role)}:${normalizeWpId(wpId)}`,
  ]
    .map((value) => normalizeSessionId(value))
    .filter(Boolean);
  for (const id of ids) {
    candidates.push(sessionEventsFile({ wpId, sessionId: id, runtimeRootAbs }));
    candidates.push(legacySessionEventsFile({ wpId, sessionId: id, runtimeRootAbs }));
  }
  return [...new Set(candidates.filter(Boolean).map((candidate) => path.resolve(candidate)))];
}

function findSessionEventsFile(session, options) {
  return sessionEventCandidateFiles(session, options).find((candidate) => fs.existsSync(candidate)) || "";
}

function readEventLog(filePath = "") {
  if (!filePath || !fs.existsSync(filePath)) return [];
  try {
    return parseJsonlFile(filePath).filter((entry) => entry && typeof entry === "object" && !Array.isArray(entry));
  } catch {
    const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
    const entries = [];
    for (const line of lines) {
      try {
        const entry = JSON.parse(line);
        if (entry && typeof entry === "object" && !Array.isArray(entry)) entries.push(entry);
      } catch {
        // Ignore malformed event rows. A bad telemetry row should not block startup.
      }
    }
    return entries;
  }
}

export function appendSessionEvent({
  wpId,
  role,
  sessionId,
  eventType,
  event = {},
  timestamp = "",
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const WP_ID = normalizeWpId(wpId);
  const ROLE = normalizeRole(role);
  const SESSION_ID = normalizeSessionId(sessionId);
  const EVENT_TYPE = String(eventType || event?.event_type || event?.type || "").trim();
  if (!WP_ID || !WP_ID.startsWith("WP-")) throw new Error("appendSessionEvent requires WP_ID");
  if (!ROLE) throw new Error("appendSessionEvent requires role");
  if (!SESSION_ID) throw new Error("appendSessionEvent requires session_id");
  if (!EVENT_TYPE) throw new Error("appendSessionEvent requires event_type");

  const filePath = sessionEventsFile({ wpId: WP_ID, sessionId: SESSION_ID, runtimeRootAbs });
  const row = {
    schema_id: SESSION_EVENT_SCHEMA_ID,
    schema_version: SESSION_EVENT_SCHEMA_VERSION,
    timestamp: String(timestamp || event?.timestamp || event?.timestamp_utc || nowIso()),
    wp_id: WP_ID,
    role: ROLE,
    session_id: SESSION_ID,
    event_type: EVENT_TYPE,
    ...(event && typeof event === "object" && !Array.isArray(event) ? event : {}),
  };
  appendJsonlLine(filePath, row);
  return { filePath, row };
}

export function tryAppendSessionEvent(args = {}) {
  try {
    return appendSessionEvent(args);
  } catch {
    return null;
  }
}

function formatToolEvent(event = {}) {
  const name = compactText(event.tool_name || event.name || event.command_kind || event.type || event.event_type || "tool", 48);
  const resultClass = compactText(event.result_class || event.status || event.outcome_state || event.result || "UNKNOWN", 48);
  const duration = Number.isFinite(Number(event.duration_ms)) ? ` ${Number(event.duration_ms)}ms` : "";
  const args = compactText(event.args_summary || event.summary || event.command_summary || "", 90);
  return `${name} -> ${resultClass}${duration}${args ? ` (${args})` : ""}`;
}

function formatReceiptEvent(event = {}) {
  const kind = compactText(event.receipt_kind || event.kind || "RECEIPT", 48);
  const verb = compactText(event.verb || "", 36);
  const mtId = compactText(event.mt_id || event.scope_ref || event.microtask_id || "", 24);
  const corr = compactText(event.correlation_id || "", 36);
  const parts = [kind, verb ? `verb=${verb}` : "", mtId ? `mt=${mtId}` : "", corr ? `corr=${corr}` : ""].filter(Boolean);
  return parts.join(" ");
}

function formatFileEvent(event = {}) {
  const action = compactText(event.action || event.file_action || "touch", 16);
  const filePath = compactText(event.path || event.file || event.file_path || "<unknown>", 110);
  return `${action} ${filePath}`;
}

function formatMtEvent(event = {}) {
  const mtId = compactText(event.mt_id || event.microtask_id || event.scope_ref || "MT", 24);
  const transition = compactText(event.transition || event.state_transition || "", 80);
  const fromTo = event.from || event.to ? `${event.from || "?"}->${event.to || "?"}` : "";
  return `${mtId} ${transition || fromTo || compactText(event.summary || "progress", 80)}`;
}

function formatVerdictEvent(event = {}) {
  const kind = compactText(event.kind || event.verdict_kind || event.receipt_kind || "VERDICT", 36);
  const mtId = compactText(event.mt_id || event.scope_ref || event.microtask_id || "", 24);
  const fromTo = event.from || event.to ? `${event.from || "?"}->${event.to || "?"}` : compactText(event.transition || event.verdict || event.status || "", 60);
  return `${kind}${mtId ? ` ${mtId}` : ""} ${fromTo}`.trim();
}

function formatSteerEvent(event = {}) {
  const source = compactText(event.source_role || event.from_role || event.created_by_role || "ORCHESTRATOR", 36);
  const summary = compactText(event.summary || event.prompt_summary || event.args_summary || event.command_summary || "", 160);
  return `${source}: ${summary || compactText(event.command_kind || event.type || event.event_type || "steer", 80)}`;
}

function eventType(event = {}) {
  return String(event?.event_type || event?.type || "").trim().toLowerCase();
}

function lastMatching(events = [], predicate, limit) {
  return events.filter(predicate).slice(-limit);
}

function isToolEvent(event = {}) {
  const type = eventType(event);
  return type === "tool_call"
    || type === "model_tool_call"
    || type === "session_command"
    || type === "command_result"
    || Boolean(event.tool_name);
}

function isReceiptEvent(event = {}) {
  const type = eventType(event);
  return type === "receipt_emitted" || Boolean(event.receipt_kind);
}

function isFileEvent(event = {}) {
  const type = eventType(event);
  return type === "file_touched" || type === "file_edit" || type === "file_change" || Boolean(event.file_path);
}

function isMtEvent(event = {}) {
  const type = eventType(event);
  return type === "mt_progression" || type === "microtask_progression";
}

function isVerdictEvent(event = {}) {
  const type = eventType(event);
  return type === "verdict_transition" || type === "mt_verdict_transition";
}

function isSteerEvent(event = {}) {
  const type = eventType(event);
  return type === "steer_received"
    || type === "direct_steer"
    || (type === "session_command" && String(event.command_kind || "").trim().toUpperCase() === "SEND_PROMPT");
}

function appendSection(lines, label, entries, formatter, emptyValue = "<none>") {
  if (!entries.length) {
    lines.push(`- ${label}: ${emptyValue}`);
    return;
  }
  lines.push(`- ${label}:`);
  for (const entry of entries) lines.push(`  - ${formatter(entry)}`);
}

function applyTokenBudget(text = "", tokenBudget = 500) {
  const budget = Number.isFinite(Number(tokenBudget)) && Number(tokenBudget) > 0 ? Math.floor(Number(tokenBudget)) : 500;
  const words = String(text || "").split(/\s+/).filter(Boolean);
  if (words.length <= budget) return text;
  const kept = words.slice(0, Math.max(0, budget - 8)).join(" ");
  return `${kept}\n- TRUNCATED: token budget ${budget} reached.\n</predecessor-summary>`;
}

export function buildPredecessorSummary({
  wpId,
  role,
  predecessorSession,
  eventsFile,
  events,
  tokenBudget = 500,
} = {}) {
  const toolEvents = lastMatching(events, isToolEvent, 10);
  const receiptEvents = lastMatching(events, isReceiptEvent, 5);
  const fileEvents = lastMatching(events, isFileEvent, 3);
  const mtEvents = lastMatching(events, isMtEvent, 3);
  const verdictEvents = lastMatching(events, isVerdictEvent, 2);
  const lastSteer = lastMatching(events, isSteerEvent, 1);
  const lines = [
    `<predecessor-summary schema="${PREDECESSOR_SUMMARY_SCHEMA_ID}" version="${PREDECESSOR_SUMMARY_VERSION}">`,
    `PREDECESSOR_SESSION [RGF-249]`,
    `- ROLE: ${normalizeRole(role)}`,
    `- WP_ID: ${normalizeWpId(wpId)}`,
    `- PREDECESSOR_SESSION_ID: ${compactText(predecessorSession?.session_id || predecessorSession?.session_key || "<unknown>", 120)}`,
    `- EVENTS_FILE: ${normalizePath(eventsFile || "<none>")}`,
    `- EVENT_COUNT: ${events.length}`,
  ];
  appendSection(lines, "TOOL_CALLS", toolEvents, formatToolEvent);
  appendSection(lines, "RECEIPTS", receiptEvents, formatReceiptEvent);
  appendSection(lines, "FILES", fileEvents, formatFileEvent);
  appendSection(lines, "MT_PROGRESS", mtEvents, formatMtEvent);
  appendSection(lines, "VERDICTS", verdictEvents, formatVerdictEvent);
  appendSection(lines, "LAST_STEER", lastSteer, formatSteerEvent);
  lines.push("</predecessor-summary>");
  return applyTokenBudget(lines.join("\n"), tokenBudget);
}

function candidateMatches(session = {}, { wpId, role, currentSessionId = "", includeCurrent = false } = {}) {
  if (normalizeWpId(session?.wp_id) !== normalizeWpId(wpId)) return false;
  if (normalizeRole(session?.role) !== normalizeRole(role)) return false;
  if (includeCurrent) return true;
  const currentKey = sessionIdentityKey(currentSessionId);
  if (!currentKey) return true;
  const identities = [
    session?.session_id,
    session?.session_key,
    session?.session_thread_id,
  ].map((value) => sessionIdentityKey(value)).filter(Boolean);
  return !identities.includes(currentKey);
}

export async function getPredecessorSummary({
  wpId,
  role,
  currentSessionId,
  tokenBudget = 500,
  includeCurrent = false,
  registry = null,
  repoRoot = REPO_ROOT,
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const WP_ID = normalizeWpId(wpId);
  const ROLE = normalizeRole(role);
  if (!WP_ID || !WP_ID.startsWith("WP-") || !ROLE) return null;
  const sourceRegistry = registry || loadSessionRegistry(repoRoot).registry;
  const candidates = (Array.isArray(sourceRegistry?.sessions) ? sourceRegistry.sessions : [])
    .filter((session) => candidateMatches(session, {
      wpId: WP_ID,
      role: ROLE,
      currentSessionId,
      includeCurrent,
    }))
    .map((session, index) => ({
      session,
      index,
      score: timestampScore(session),
      eventsFile: findSessionEventsFile(session, { wpId: WP_ID, role: ROLE, runtimeRootAbs, repoRoot }),
    }))
    .filter((candidate) => candidate.eventsFile)
    .sort((left, right) => {
      if (right.score !== left.score) return right.score - left.score;
      return right.index - left.index;
    });

  for (const candidate of candidates) {
    const events = readEventLog(candidate.eventsFile);
    if (events.length === 0) continue;
    const summary = buildPredecessorSummary({
      wpId: WP_ID,
      role: ROLE,
      predecessorSession: candidate.session,
      eventsFile: candidate.eventsFile,
      events,
      tokenBudget,
    });
    return tokenCountApprox(summary) <= Number(tokenBudget || 500) + 8
      ? summary
      : applyTokenBudget(summary, tokenBudget);
  }

  return null;
}
