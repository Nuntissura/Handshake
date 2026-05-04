import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import {
  SESSION_BATCH_MODE_CLI_ESCALATION,
  SESSION_BATCH_MODE_PLUGIN_FIRST,
  SESSION_BATCH_MODE_VALUES,
  SESSION_BATCH_SCOPE,
  SESSION_ACTIVE_HOST_NONE,
  SESSION_ACTIVE_HOST_VALUES,
  SESSION_ACTIVE_TERMINAL_KIND_NONE,
  SESSION_ACTIVE_TERMINAL_KIND_VALUES,
  SESSION_TERMINAL_OWNERSHIP_SCOPE_NONE,
  SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION,
  SESSION_TERMINAL_OWNERSHIP_SCOPE_VALUES,
  SESSION_TERMINAL_RECLAIM_STATUS_NONE,
  SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED,
  SESSION_TERMINAL_RECLAIM_STATUS_VALUES,
  SESSION_COMMAND_STATUSES,
  SESSION_CONTROL_MODE,
  SESSION_CONTROL_BROKER_STATE_FILE,
  SESSION_CONTROL_HOST_PRIMARY,
  SESSION_CONTROL_OUTPUT_DIR,
  SESSION_CONTROL_PROTOCOL_PRIMARY,
  SESSION_CONTROL_REQUESTS_FILE,
  SESSION_CONTROL_RESULTS_FILE,
  SESSION_CONTROL_TRANSPORT_PRIMARY,
  CLI_ESCALATION_HOST_DEFAULT,
  SESSION_HOST_FALLBACK,
  SESSION_HOST_FALLBACK_LEGACY,
  SESSION_HOST_PREFERENCE,
  SESSION_PLUGIN_HOST,
  SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
  SESSION_PLUGIN_BRIDGE_COMMAND,
  SESSION_PLUGIN_BRIDGE_ID,
  SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
  SESSION_PLUGIN_REQUESTS_FILE,
  SESSION_PENDING_CONTROL_QUEUE_LIMIT,
  SESSION_REGISTRY_FILE,
  SESSION_REQUEST_STATUSES,
  SESSION_RUNTIME_STATES,
  SESSION_START_AUTHORITY,
  SESSION_WAKE_CHANNEL_FALLBACK,
  SESSION_WAKE_CHANNEL_PRIMARY,
  SESSION_WATCH_POLICY,
  normalizeActiveHostValue,
  normalizeActiveTerminalKindValue,
  normalizePath,
  sessionKey,
  terminalTitle,
} from "./session-policy.mjs";
import { SESSION_HEALTH_SOURCE, SESSION_HEALTH_STATE_VALUES } from "./session-health-projection-lib.mjs";
import { effectiveSessionGovernedAction, summarizeGovernedAction } from "./session-governed-action-lib.mjs";

export const ROLE_SESSION_REGISTRY_SCHEMA_ID = "hsk.role_session_registry@1";
export const ROLE_SESSION_REGISTRY_SCHEMA_VERSION = "role_session_registry_v1";
export const SESSION_LAUNCH_REQUEST_SCHEMA_ID = "hsk.session_launch_request@1";
export const SESSION_LAUNCH_REQUEST_SCHEMA_VERSION = "session_launch_request_v1";
const SESSION_REGISTRY_LOCK_FILE_NAME = "ROLE_SESSION_REGISTRY.lock";
const SESSION_REGISTRY_LOCK_WAIT_MS = 5000;
const SESSION_REGISTRY_LOCK_STALE_MS = 60000;
const SESSION_REGISTRY_LOCK_POLL_MS = 50;
const ATOMIC_WRITE_TEMP_FILE_STALE_MS = 5 * 60 * 1000;
const ACTIVE_BATCH_RUNTIME_STATES = new Set([
  "PLUGIN_REQUESTED",
  "TERMINAL_COMMAND_DISPATCHED",
  "PLUGIN_CONFIRMED",
  "CLI_ESCALATION_READY",
  "CLI_ESCALATION_USED",
  "STARTING",
  "READY",
  "COMMAND_RUNNING",
  "ACTIVE",
  "WAITING",
]);
const TERMINAL_BATCH_ID_PREFIX = "TBATCH";
const SESSION_ACTION_HISTORY_LIMIT = 16;

function nowIso() {
  return new Date().toISOString();
}

function sleepSync(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, Math.max(1, Math.trunc(ms)));
}

function ensureParentDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function ledgerLockPath(filePath) {
  return `${filePath}.lock`;
}

function removeFileIfPresent(filePath) {
  try {
    fs.unlinkSync(filePath);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
}

function cleanupAtomicWriteTemps(filePath, { currentTempPath = "" } = {}) {
  const targetDir = path.dirname(filePath);
  const targetBase = path.basename(filePath);
  let entries = [];
  try {
    entries = fs.readdirSync(targetDir, { withFileTypes: true });
  } catch {
    return;
  }

  const currentTempBase = currentTempPath ? path.basename(currentTempPath) : "";
  const nowMs = Date.now();
  for (const entry of entries) {
    if (!entry.isFile()) continue;
    if (!entry.name.startsWith(`${targetBase}.`) || !entry.name.endsWith(".tmp")) continue;
    if (currentTempBase && entry.name === currentTempBase) continue;
    const candidate = path.join(targetDir, entry.name);
    let stats;
    try {
      stats = fs.statSync(candidate);
    } catch {
      continue;
    }
    if ((nowMs - stats.mtimeMs) < ATOMIC_WRITE_TEMP_FILE_STALE_MS) continue;
    removeFileIfPresent(candidate);
  }
}

function renameWithRetrySync(sourcePath, destPath, {
  attempts = 40,
  delayMs = 50,
} = {}) {
  let lastError = null;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      fs.renameSync(sourcePath, destPath);
      return;
    } catch (error) {
      if (!["EPERM", "EBUSY", "EACCES"].includes(error?.code)) {
        throw error;
      }
      lastError = error;
      if (attempt === attempts - 1) break;
      sleepSync(delayMs);
    }
  }
  throw lastError;
}

function normalizeActiveHost(value) {
  const token = normalizeActiveHostValue(value);
  if (SESSION_ACTIVE_HOST_VALUES.includes(token)) return token;
  return SESSION_ACTIVE_HOST_NONE;
}

function normalizeActiveTerminalKind(value) {
  const token = normalizeActiveTerminalKindValue(value);
  if (SESSION_ACTIVE_TERMINAL_KIND_VALUES.includes(token)) return token;
  return SESSION_ACTIVE_TERMINAL_KIND_NONE;
}

function buildTerminalBatchId() {
  return `${TERMINAL_BATCH_ID_PREFIX}-${crypto.randomUUID().split("-")[0].toUpperCase()}`;
}

function deriveTerminalBatchIdFromRegistry(registry) {
  const seed = [
    registry?.launch_batch_last_reset_at || "",
    registry?.launch_batch_switched_at || "",
    registry?.updated_at || "",
    registry?.schema_version || "",
    registry?.schema_id || "",
  ].join("|");
  const digest = crypto.createHash("sha1").update(seed || "registry-bootstrap").digest("hex").slice(0, 8).toUpperCase();
  return `${TERMINAL_BATCH_ID_PREFIX}-${digest}`;
}

function normalizeTerminalBatchId(value) {
  const text = String(value || "").trim().toUpperCase();
  if (!text) return "";
  return text;
}

function normalizeTerminalBatchScope(value) {
  const text = String(value || "").trim();
  if (!text) return "";
  return text;
}

function sessionHasOwnedTerminal(session) {
  return Number.isInteger(session?.owned_terminal_process_id)
    && session.owned_terminal_process_id > 0
    && String(session?.terminal_ownership_scope || "") === SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION
    && String(session?.owned_terminal_reclaim_status || "") !== SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED;
}

function rotateTerminalBatch(registry, reason = "manual terminal batch rotation") {
  const rotatedAt = nowIso();
  registry.terminal_batch_scope = SESSION_BATCH_SCOPE;
  registry.active_terminal_batch_id = buildTerminalBatchId();
  registry.active_terminal_batch_started_at = rotatedAt;
  registry.active_terminal_batch_last_rotated_at = rotatedAt;
  registry.active_terminal_batch_claimed_at = "";
  registry.active_terminal_batch_reason = reason ? String(reason) : "";
  return registry.active_terminal_batch_id;
}

function normalizeSessionRecord(session) {
  if (!session || typeof session !== "object") return session;
  session.control_mode = session.control_mode || SESSION_CONTROL_MODE;
  session.control_transport = session.control_transport || SESSION_CONTROL_TRANSPORT_PRIMARY;
  session.control_protocol = session.control_protocol || SESSION_CONTROL_PROTOCOL_PRIMARY;
  session.session_thread_id = session.session_thread_id || "";
  session.session_thread_started_at = session.session_thread_started_at || "";
  session.startup_proof_state = session.startup_proof_state || "NONE";
  session.last_command_id = session.last_command_id || "";
  session.last_command_kind = session.last_command_kind || "NONE";
  session.last_command_status = session.last_command_status || "NONE";
  session.last_command_summary = session.last_command_summary || "";
  session.last_command_prompt_at = session.last_command_prompt_at || "";
  session.last_command_completed_at = session.last_command_completed_at || "";
  session.last_command_output_file = session.last_command_output_file || "";
  session.active_host = normalizeActiveHost(session.active_host);
  session.active_terminal_title = session.active_terminal_title || "";
  session.active_terminal_kind = normalizeActiveTerminalKind(session.active_terminal_kind);
  session.requested_profile_id = session.requested_profile_id || "";
  session.terminal_ownership_scope = SESSION_TERMINAL_OWNERSHIP_SCOPE_VALUES.includes(session.terminal_ownership_scope)
    ? session.terminal_ownership_scope
    : SESSION_TERMINAL_OWNERSHIP_SCOPE_NONE;
  session.owned_terminal_process_id = Number.isInteger(session.owned_terminal_process_id)
    && session.owned_terminal_process_id > 0
    ? session.owned_terminal_process_id
    : 0;
  session.owned_terminal_host_kind = session.owned_terminal_host_kind || "";
  session.owned_terminal_window_title = session.owned_terminal_window_title || "";
  session.owned_terminal_batch_scope = normalizeTerminalBatchScope(session.owned_terminal_batch_scope);
  session.owned_terminal_batch_id = normalizeTerminalBatchId(session.owned_terminal_batch_id);
  session.owned_terminal_recorded_at = session.owned_terminal_recorded_at || "";
  session.owned_terminal_reclaimed_at = session.owned_terminal_reclaimed_at || "";
  session.owned_terminal_reclaim_status = SESSION_TERMINAL_RECLAIM_STATUS_VALUES.includes(session.owned_terminal_reclaim_status)
    ? session.owned_terminal_reclaim_status
    : SESSION_TERMINAL_RECLAIM_STATUS_NONE;
  session.health_state = SESSION_HEALTH_STATE_VALUES.includes(String(session.health_state || "").trim().toUpperCase())
    ? String(session.health_state || "").trim().toUpperCase()
    : "UNKNOWN";
  session.health_reason_code = String(session.health_reason_code || "").trim().toUpperCase() || "UNKNOWN";
  session.health_summary = session.health_summary || "";
  session.health_source = session.health_source || SESSION_HEALTH_SOURCE;
  session.health_updated_at = session.health_updated_at || "";
  session.last_heartbeat_at = session.last_heartbeat_at || "";
  session.last_error = session.last_error || "";
  session.last_event_at = session.last_event_at || "";
  session.action_history = normalizeSessionActionHistory(session.action_history);
  session.pending_control_queue = normalizePendingControlQueue(session.pending_control_queue);
  return session;
}

function normalizeSessionActionHistoryEntry(entry = {}) {
  const summary = summarizeGovernedAction(entry);
  if (!summary) return null;
  return {
    action_id: summary.action_id,
    rule_id: summary.rule_id,
    action_kind: summary.action_kind,
    action_surface: summary.action_surface,
    command_kind: summary.command_kind,
    command_id: summary.command_id,
    action_state: summary.action_state,
    status: summary.status,
    outcome_state: summary.outcome_state,
    resume_disposition: summary.resume_disposition,
    target_command_id: summary.target_command_id,
    reason_code: summary.reason_code,
    summary: summary.summary,
    requested_at: summary.requested_at,
    processed_at: summary.processed_at,
    updated_at: summary.updated_at,
  };
}

function normalizeSessionActionHistory(entries = []) {
  if (!Array.isArray(entries)) return [];
  return entries
    .map((entry) => normalizeSessionActionHistoryEntry(entry))
    .filter(Boolean)
    .slice(-SESSION_ACTION_HISTORY_LIMIT);
}

function upsertSessionActionHistory(session, action = {}, overrides = {}) {
  const baseEntry = normalizeSessionActionHistoryEntry({
    ...action,
    ...overrides,
  });
  if (!baseEntry) return;
  const history = normalizeSessionActionHistory(session.action_history);
  const index = history.findIndex((entry) => entry.action_id === baseEntry.action_id);
  const existing = index >= 0 ? history[index] : null;
  const merged = {
    ...(existing || {}),
    ...baseEntry,
    requested_at: baseEntry.requested_at || existing?.requested_at || "",
    processed_at: baseEntry.processed_at || existing?.processed_at || "",
    updated_at: baseEntry.updated_at || existing?.updated_at || nowIso(),
  };
  if (index >= 0) {
    history[index] = merged;
  } else {
    history.push(merged);
  }
  session.action_history = history.slice(-SESSION_ACTION_HISTORY_LIMIT);
}

function normalizePendingControlQueueEntry(entry = {}) {
  const commandId = String(entry?.command_id || "").trim();
  const commandKind = String(entry?.command_kind || "").trim().toUpperCase();
  if (!commandId || !commandKind) return null;

  const governedAction = summarizeGovernedAction({
    action_id: entry?.action_id || entry?.governed_action?.action_id || commandId,
    rule_id: entry?.rule_id || entry?.governed_action?.rule_id || "",
    action_kind: entry?.action_kind || entry?.governed_action?.action_kind || "EXTERNAL_EXECUTE",
    action_surface: entry?.action_surface || entry?.governed_action?.action_surface || "SESSION_CONTROL",
    command_kind: commandKind,
    command_id: commandId,
    action_state: entry?.action_state || entry?.governed_action?.action_state || "ACCEPTED_QUEUED",
    status: entry?.status || entry?.governed_action?.status || "QUEUED",
    outcome_state: entry?.outcome_state || entry?.governed_action?.outcome_state || "ACCEPTED_QUEUED",
    resume_disposition: entry?.resume_disposition || entry?.governed_action?.resume_disposition || "PENDING",
    target_command_id: entry?.target_command_id || entry?.governed_action?.target_command_id || "",
    reason_code: entry?.reason_code || entry?.governed_action?.reason_code || "",
    summary: entry?.summary || entry?.governed_action?.summary || "",
    requested_at: entry?.requested_at || entry?.governed_action?.requested_at || entry?.queued_at || "",
    updated_at: entry?.updated_at || entry?.governed_action?.updated_at || entry?.queued_at || "",
  });
  if (!governedAction) return null;

  return {
    command_id: commandId,
    command_kind: commandKind,
    target_command_id: String(entry?.target_command_id || governedAction.target_command_id || "").trim(),
    summary: String(entry?.summary || governedAction.summary || "").trim(),
    output_jsonl_file: normalizePath(entry?.output_jsonl_file || ""),
    busy_ingress_mode: String(entry?.busy_ingress_mode || "ENQUEUE_ON_BUSY").trim().toUpperCase() || "ENQUEUE_ON_BUSY",
    queue_reason_code: String(entry?.queue_reason_code || "").trim().toUpperCase() || "BUSY_ACTIVE_RUN",
    blocking_command_id: String(entry?.blocking_command_id || "").trim(),
    queued_at: String(entry?.queued_at || governedAction.requested_at || "").trim(),
    updated_at: String(entry?.updated_at || governedAction.updated_at || entry?.queued_at || "").trim(),
    action_id: governedAction.action_id,
    rule_id: governedAction.rule_id,
    action_kind: governedAction.action_kind,
    action_surface: governedAction.action_surface,
    action_state: governedAction.action_state,
    status: governedAction.status,
    outcome_state: governedAction.outcome_state,
    resume_disposition: governedAction.resume_disposition,
    reason_code: governedAction.reason_code,
    requested_at: governedAction.requested_at,
  };
}

function normalizePendingControlQueue(entries = []) {
  if (!Array.isArray(entries)) return [];
  return entries
    .map((entry) => normalizePendingControlQueueEntry(entry))
    .filter(Boolean)
    .slice(-SESSION_PENDING_CONTROL_QUEUE_LIMIT);
}

export function enqueuePendingSessionControlRequest(session, request, {
  queueReasonCode = "BUSY_ACTIVE_RUN",
  blockingCommandId = "",
  queuedAt = "",
} = {}) {
  const normalizedEntry = normalizePendingControlQueueEntry({
    command_id: request?.command_id,
    command_kind: request?.command_kind,
    target_command_id: request?.target_command_id || "",
    summary: request?.summary || request?.governed_action?.summary || "",
    output_jsonl_file: request?.output_jsonl_file || "",
    busy_ingress_mode: request?.busy_ingress_mode || "ENQUEUE_ON_BUSY",
    queue_reason_code: queueReasonCode,
    blocking_command_id: blockingCommandId,
    queued_at: queuedAt || request?.created_at || nowIso(),
    updated_at: queuedAt || request?.created_at || nowIso(),
    action_state: "ACCEPTED_QUEUED",
    status: "QUEUED",
    outcome_state: "ACCEPTED_QUEUED",
    governed_action: request?.governed_action || {},
  });
  if (!normalizedEntry) return null;

  const queue = normalizePendingControlQueue(session.pending_control_queue);
  const existingIndex = queue.findIndex((entry) => entry.command_id === normalizedEntry.command_id);
  if (existingIndex >= 0) {
    queue[existingIndex] = {
      ...queue[existingIndex],
      ...normalizedEntry,
      queued_at: queue[existingIndex].queued_at || normalizedEntry.queued_at,
      updated_at: normalizedEntry.updated_at || nowIso(),
    };
  } else {
    queue.push(normalizedEntry);
  }
  session.pending_control_queue = queue.slice(-SESSION_PENDING_CONTROL_QUEUE_LIMIT);
  session.last_event_at = normalizedEntry.updated_at || nowIso();
  return normalizedEntry;
}

export function removePendingSessionControlRequest(session, commandId = "") {
  const normalizedCommandId = String(commandId || "").trim();
  if (!normalizedCommandId) return null;
  const queue = normalizePendingControlQueue(session.pending_control_queue);
  const index = queue.findIndex((entry) => entry.command_id === normalizedCommandId);
  if (index < 0) {
    session.pending_control_queue = queue;
    return null;
  }
  const [removed] = queue.splice(index, 1);
  session.pending_control_queue = queue;
  session.last_event_at = nowIso();
  return removed;
}

export function peekPendingSessionControlRequest(session) {
  const queue = normalizePendingControlQueue(session?.pending_control_queue);
  return queue.length > 0 ? queue[0] : null;
}

export function isPendingSessionControlRequest(session, commandId = "") {
  const normalizedCommandId = String(commandId || "").trim();
  if (!normalizedCommandId) return false;
  return normalizePendingControlQueue(session?.pending_control_queue)
    .some((entry) => entry.command_id === normalizedCommandId);
}

function normalizeRegistryBatchState(registry) {
  if (!registry || typeof registry !== "object") return registry;
  const currentMode = String(registry.launch_batch_mode || "").trim().toUpperCase();
  registry.launch_batch_mode = SESSION_BATCH_MODE_VALUES.includes(currentMode)
    ? currentMode
    : SESSION_BATCH_MODE_PLUGIN_FIRST;
  registry.launch_batch_scope = registry.launch_batch_scope || SESSION_BATCH_SCOPE;
  registry.launch_batch_plugin_failure_count = Number.isInteger(registry.launch_batch_plugin_failure_count)
    && registry.launch_batch_plugin_failure_count >= 0
    ? registry.launch_batch_plugin_failure_count
    : 0;
  registry.launch_batch_switched_at = registry.launch_batch_switched_at || "";
  registry.launch_batch_switch_reason = registry.launch_batch_switch_reason || "";
  registry.launch_batch_last_reset_at = registry.launch_batch_last_reset_at || "";
  registry.terminal_batch_scope = registry.terminal_batch_scope || SESSION_BATCH_SCOPE;
  registry.active_terminal_batch_id = normalizeTerminalBatchId(registry.active_terminal_batch_id) || deriveTerminalBatchIdFromRegistry(registry);
  registry.active_terminal_batch_started_at = registry.active_terminal_batch_started_at || nowIso();
  registry.active_terminal_batch_last_rotated_at = registry.active_terminal_batch_last_rotated_at || registry.active_terminal_batch_started_at;
  registry.active_terminal_batch_claimed_at = registry.active_terminal_batch_claimed_at || "";
  registry.active_terminal_batch_reason = registry.active_terminal_batch_reason || "registry bootstrap";
  return registry;
}

function normalizeOwnedTerminalBatchAssignments(registry) {
  if (!registry || typeof registry !== "object") return registry;
  normalizeRegistryBatchState(registry);
  registry.sessions = Array.isArray(registry.sessions) ? registry.sessions : [];
  for (const session of registry.sessions) {
    normalizeSessionRecord(session);
    if (!sessionHasOwnedTerminal(session)) continue;
    session.owned_terminal_batch_scope = session.owned_terminal_batch_scope || registry.terminal_batch_scope;
    session.owned_terminal_batch_id = session.owned_terminal_batch_id || registry.active_terminal_batch_id;
  }
  return registry;
}

export function ensureActiveTerminalBatch(registry, {
  reason = "governed terminal batch activation",
  currentSessionKey = "",
} = {}) {
  normalizeOwnedTerminalBatchAssignments(registry);
  const otherActiveSessions = (registry.sessions || []).some((session) => {
    const sessionKeyValue = String(session?.session_key || "");
    if (currentSessionKey && sessionKeyValue === String(currentSessionKey)) return false;
    return ACTIVE_BATCH_RUNTIME_STATES.has(String(session?.runtime_state || "").trim().toUpperCase());
  });
  const currentSession = (registry.sessions || []).find((session) => String(session?.session_key || "") === String(currentSessionKey || ""));
  const currentSessionOwnsTerminal = sessionHasOwnedTerminal(currentSession);
  if (registry.active_terminal_batch_claimed_at && !otherActiveSessions && !currentSessionOwnsTerminal) {
    rotateTerminalBatch(registry, reason);
  }
  return {
    terminal_batch_scope: registry.terminal_batch_scope,
    terminal_batch_id: registry.active_terminal_batch_id,
  };
}

function recordBatchPluginFailure(registry, {
  requestId = "",
  sessionKey: failedSessionKey = "",
  error = "",
  status = "",
} = {}) {
  normalizeRegistryBatchState(registry);
  registry.launch_batch_plugin_failure_count += 1;
  const threshold = SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION;
  if (
    registry.launch_batch_plugin_failure_count < threshold
    || registry.launch_batch_mode === SESSION_BATCH_MODE_CLI_ESCALATION
  ) {
    return;
  }

  registry.launch_batch_mode = SESSION_BATCH_MODE_CLI_ESCALATION;
  registry.launch_batch_switched_at = nowIso();
  const reasonParts = [
    `plugin instability reached ${registry.launch_batch_plugin_failure_count}/${threshold}`,
    failedSessionKey ? `session ${failedSessionKey}` : "",
    requestId ? `request ${requestId}` : "",
    status ? `status ${status}` : "",
    error ? `error ${error}` : "",
  ].filter(Boolean);
  registry.launch_batch_switch_reason = reasonParts.join(" | ");
}

export function resetBatchLaunchMode(registry, reason = "manual reset") {
  normalizeRegistryBatchState(registry);
  registry.launch_batch_mode = SESSION_BATCH_MODE_PLUGIN_FIRST;
  registry.launch_batch_plugin_failure_count = 0;
  registry.launch_batch_last_reset_at = nowIso();
  registry.launch_batch_switch_reason = reason ? String(reason) : "";
  registry.launch_batch_switched_at = "";
  rotateTerminalBatch(registry, reason ? `operator-approved terminal batch reset: ${reason}` : "operator-approved terminal batch reset");
}

export function defaultRegistry() {
  const createdAt = nowIso();
  return {
    schema_id: ROLE_SESSION_REGISTRY_SCHEMA_ID,
    schema_version: ROLE_SESSION_REGISTRY_SCHEMA_VERSION,
    updated_at: createdAt,
    session_start_authority: SESSION_START_AUTHORITY,
    session_host_preference: SESSION_HOST_PREFERENCE,
    session_host_fallback: SESSION_HOST_FALLBACK,
    session_watch_policy: SESSION_WATCH_POLICY,
    session_plugin_bridge_id: SESSION_PLUGIN_BRIDGE_ID,
    session_plugin_bridge_command: SESSION_PLUGIN_BRIDGE_COMMAND,
    session_plugin_requests_file: SESSION_PLUGIN_REQUESTS_FILE,
    session_control_mode: SESSION_CONTROL_MODE,
    session_control_transport_primary: SESSION_CONTROL_TRANSPORT_PRIMARY,
    session_control_protocol_primary: SESSION_CONTROL_PROTOCOL_PRIMARY,
    session_control_requests_file: SESSION_CONTROL_REQUESTS_FILE,
    session_control_results_file: SESSION_CONTROL_RESULTS_FILE,
    session_control_output_dir: SESSION_CONTROL_OUTPUT_DIR,
    session_wake_channel_primary: SESSION_WAKE_CHANNEL_PRIMARY,
    session_wake_channel_fallback: SESSION_WAKE_CHANNEL_FALLBACK,
    session_plugin_max_retries_before_escalation: SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
    session_plugin_attempt_timeout_seconds: SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
    launch_batch_mode: SESSION_BATCH_MODE_PLUGIN_FIRST,
    launch_batch_scope: SESSION_BATCH_SCOPE,
    launch_batch_plugin_failure_count: 0,
    launch_batch_switched_at: "",
    launch_batch_switch_reason: "",
    launch_batch_last_reset_at: "",
    terminal_batch_scope: SESSION_BATCH_SCOPE,
    active_terminal_batch_id: buildTerminalBatchId(),
    active_terminal_batch_started_at: createdAt,
    active_terminal_batch_last_rotated_at: createdAt,
    active_terminal_batch_claimed_at: "",
    active_terminal_batch_reason: "registry bootstrap",
    sessions: [],
    processed_requests: [],
  };
}

export function readJsonFile(filePath, fallbackValue) {
  if (!fs.existsSync(filePath)) return JSON.parse(JSON.stringify(fallbackValue));
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJsonFileUnlocked(filePath, value) {
  ensureParentDir(filePath);
  const tmpPath = `${filePath}.${process.pid}.${crypto.randomUUID()}.tmp`;
  try {
    cleanupAtomicWriteTemps(filePath, { currentTempPath: tmpPath });
    fs.writeFileSync(tmpPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
    renameWithRetrySync(tmpPath, filePath);
    cleanupAtomicWriteTemps(filePath);
  } catch (error) {
    removeFileIfPresent(tmpPath);
    throw error;
  }
}

export function writeJsonFile(filePath, value, { lockPath = "" } = {}) {
  const targetLockPath = lockPath || ledgerLockPath(filePath);
  return withFileLockSync(targetLockPath, () => writeJsonFileUnlocked(filePath, value));
}

export function parseJsonlFile(filePath) {
  if (!fs.existsSync(filePath)) return [];
  const lines = fs
    .readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  return lines.map((line) => JSON.parse(line));
}

export function appendJsonlLine(filePath, value, { lockPath = "" } = {}) {
  const targetLockPath = lockPath || ledgerLockPath(filePath);
  return withFileLockSync(targetLockPath, () => {
    ensureParentDir(filePath);
    fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, "utf8");
  });
}

export function loadSessionRegistry(repoRoot) {
  const filePath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
  const registry = readJsonFile(filePath, defaultRegistry());
  normalizeRegistryBatchState(registry);
  registry.sessions = Array.isArray(registry.sessions) ? registry.sessions.map((session) => normalizeSessionRecord(session)) : [];
  normalizeOwnedTerminalBatchAssignments(registry);
  registry.processed_requests = Array.isArray(registry.processed_requests) ? registry.processed_requests : [];
  return { filePath, registry };
}

export function saveSessionRegistry(repoRoot, registry) {
  const lockPath = sessionRegistryLockPath(repoRoot);
  return withFileLockSync(lockPath, () => saveSessionRegistryUnlocked(repoRoot, registry));
}

function saveSessionRegistryUnlocked(repoRoot, registry) {
  const filePath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
  normalizeOwnedTerminalBatchAssignments(registry);
  const next = {
    ...defaultRegistry(),
    ...registry,
    updated_at: nowIso(),
    sessions: Array.isArray(registry.sessions) ? registry.sessions.map((session) => normalizeSessionRecord(session)) : [],
    processed_requests: Array.isArray(registry.processed_requests) ? registry.processed_requests : [],
  };
  writeJsonFileUnlocked(filePath, next);
  return filePath;
}

export function sessionRegistryLockPath(repoRoot) {
  return path.resolve(repoRoot, path.dirname(SESSION_REGISTRY_FILE), SESSION_REGISTRY_LOCK_FILE_NAME);
}

function readLockState(lockPath) {
  try {
    return JSON.parse(fs.readFileSync(lockPath, "utf8"));
  } catch {
    return null;
  }
}

export function withFileLockSync(lockPath, fn, {
  waitMs = SESSION_REGISTRY_LOCK_WAIT_MS,
  staleMs = SESSION_REGISTRY_LOCK_STALE_MS,
  pollMs = SESSION_REGISTRY_LOCK_POLL_MS,
} = {}) {
  ensureParentDir(lockPath);
  const startedAt = Date.now();
  const token = {
    pid: process.pid,
    created_at: nowIso(),
  };
  while (true) {
    try {
      const fd = fs.openSync(lockPath, "wx");
      fs.writeFileSync(fd, `${JSON.stringify(token)}\n`, "utf8");
      fs.closeSync(fd);
      break;
    } catch (error) {
      if (!["EEXIST", "EPERM", "EACCES"].includes(error?.code)) throw error;
      const state = readLockState(lockPath);
      const createdAtMs = Date.parse(state?.created_at || "");
      const isStale = Number.isNaN(createdAtMs) || ((Date.now() - createdAtMs) > staleMs);
      if (isStale) {
        removeFileIfPresent(lockPath);
        continue;
      }
      if ((Date.now() - startedAt) > waitMs) {
        throw new Error(`Timed out waiting for file lock ${normalizePath(lockPath)}`);
      }
      sleepSync(pollMs);
    }
  }

  try {
    return fn();
  } finally {
    removeFileIfPresent(lockPath);
  }
}

export function mutateSessionRegistrySync(repoRoot, mutator) {
  const lockPath = sessionRegistryLockPath(repoRoot);
  return withFileLockSync(lockPath, () => {
    const { registry } = loadSessionRegistry(repoRoot);
    const result = mutator(registry);
    saveSessionRegistryUnlocked(repoRoot, registry);
    return result;
  });
}

export function loadSessionLaunchRequests(repoRoot) {
  const filePath = path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE);
  return { filePath, requests: parseJsonlFile(filePath) };
}

export function loadSessionControlRequests(repoRoot) {
  const filePath = path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE);
  return { filePath, requests: parseJsonlFile(filePath) };
}

export function loadSessionControlResults(repoRoot) {
  const filePath = path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE);
  return { filePath, results: parseJsonlFile(filePath) };
}

export function getOrCreateSessionRecord(registry, sessionDescriptor) {
  const key = sessionKey(sessionDescriptor.role, sessionDescriptor.wp_id);
  let session = registry.sessions.find((entry) => entry.session_key === key);
  if (!session) {
    session = {
      session_key: key,
      session_id: key.toLowerCase().replace(/[^a-z0-9:_-]/g, "-"),
      wp_id: sessionDescriptor.wp_id,
      role: sessionDescriptor.role,
      launch_authority: SESSION_START_AUTHORITY,
      preferred_host: SESSION_HOST_PREFERENCE,
      fallback_host: SESSION_HOST_FALLBACK,
      cli_escalation_host: CLI_ESCALATION_HOST_DEFAULT,
      local_branch: normalizePath(sessionDescriptor.local_branch || ""),
      local_worktree_dir: normalizePath(sessionDescriptor.local_worktree_dir || ""),
      terminal_title: sessionDescriptor.terminal_title || terminalTitle(sessionDescriptor.role, sessionDescriptor.wp_id),
      requested_model: sessionDescriptor.requested_model || "",
      requested_profile_id: sessionDescriptor.requested_profile_id || "",
      reasoning_config_key: sessionDescriptor.reasoning_config_key || "",
      reasoning_config_value: sessionDescriptor.reasoning_config_value || "",
      control_mode: SESSION_CONTROL_MODE,
      control_transport: SESSION_CONTROL_TRANSPORT_PRIMARY,
      control_protocol: SESSION_CONTROL_PROTOCOL_PRIMARY,
      session_thread_id: "",
      session_thread_started_at: "",
      startup_proof_state: "NONE",
      last_command_id: "",
      last_command_kind: "NONE",
      last_command_status: "NONE",
      last_command_summary: "",
      last_command_prompt_at: "",
      last_command_completed_at: "",
      last_command_output_file: "",
      action_history: [],
      pending_control_queue: [],
      plugin_request_count: 0,
      plugin_failure_count: 0,
      plugin_last_request_id: "",
      plugin_last_request_at: "",
      plugin_last_result: "NONE",
      plugin_last_error: "",
      cli_escalation_allowed: false,
      cli_escalation_used: false,
      runtime_state: "UNSTARTED",
      active_host: SESSION_ACTIVE_HOST_NONE,
      active_terminal_title: "",
      active_terminal_kind: SESSION_ACTIVE_TERMINAL_KIND_NONE,
      terminal_ownership_scope: SESSION_TERMINAL_OWNERSHIP_SCOPE_NONE,
      owned_terminal_process_id: 0,
      owned_terminal_host_kind: "",
      owned_terminal_window_title: "",
      owned_terminal_batch_scope: "",
      owned_terminal_batch_id: "",
      owned_terminal_recorded_at: "",
      owned_terminal_reclaimed_at: "",
      owned_terminal_reclaim_status: SESSION_TERMINAL_RECLAIM_STATUS_NONE,
      health_state: "UNKNOWN",
      health_reason_code: "UNKNOWN",
      health_summary: "",
      health_source: SESSION_HEALTH_SOURCE,
      health_updated_at: "",
      last_heartbeat_at: "",
      last_error: "",
      last_event_at: "",
    };
    registry.sessions.push(session);
  } else {
    session.local_branch = normalizePath(session.local_branch || sessionDescriptor.local_branch || "");
    session.local_worktree_dir = normalizePath(session.local_worktree_dir || sessionDescriptor.local_worktree_dir || "");
    session.terminal_title = sessionDescriptor.terminal_title || session.terminal_title || terminalTitle(sessionDescriptor.role, sessionDescriptor.wp_id);
    const runtimeState = String(session.runtime_state || "").trim().toUpperCase();
    const startupProofState = String(session.startup_proof_state || "").trim().toUpperCase();
    const hasLaunchSelectionOverride = Boolean(
      sessionDescriptor.requested_model ||
      sessionDescriptor.requested_profile_id ||
      sessionDescriptor.reasoning_config_key ||
      sessionDescriptor.reasoning_config_value
    );
    const restartableLaunchSelectionStates = ["FAILED", "CLOSED", "CLI_ESCALATION_READY", "CLI_ESCALATION_USED"];
    const allowLaunchSelectionRefresh = hasLaunchSelectionOverride
      && restartableLaunchSelectionStates.includes(runtimeState);

    if (allowLaunchSelectionRefresh) {
      session.requested_model = sessionDescriptor.requested_model || session.requested_model || "";
      session.requested_profile_id = sessionDescriptor.requested_profile_id || session.requested_profile_id || "";
      session.reasoning_config_key = sessionDescriptor.reasoning_config_key || session.reasoning_config_key || "";
      session.reasoning_config_value = sessionDescriptor.reasoning_config_value || session.reasoning_config_value || "";
      if (startupProofState !== "READY" || restartableLaunchSelectionStates.includes(runtimeState)) {
        session.session_thread_id = "";
        session.session_thread_started_at = "";
      }
    } else {
      session.requested_model = session.requested_model || sessionDescriptor.requested_model || "";
      session.requested_profile_id = session.requested_profile_id || sessionDescriptor.requested_profile_id || "";
      session.reasoning_config_key = session.reasoning_config_key || sessionDescriptor.reasoning_config_key || "";
      session.reasoning_config_value = session.reasoning_config_value || sessionDescriptor.reasoning_config_value || "";
    }
    normalizeSessionRecord(session);
  }
  return session;
}

export function buildLaunchRequest({
  wpId,
  role,
  localBranch,
  localWorktreeDir,
  selectedModel,
  selectedProfileId = "",
  reasoningConfigKey,
  reasoningConfigValue,
  startupCommand,
  nextCommand,
  terminalTitleValue,
  command,
  pluginAttemptNumber,
}) {
  return {
    schema_id: SESSION_LAUNCH_REQUEST_SCHEMA_ID,
    schema_version: SESSION_LAUNCH_REQUEST_SCHEMA_VERSION,
    request_id: crypto.randomUUID(),
    created_at: nowIso(),
    request_kind: "LAUNCH_SESSION",
    created_by_role: "ORCHESTRATOR",
    launch_authority: SESSION_START_AUTHORITY,
    preferred_host: SESSION_PLUGIN_HOST,
    fallback_host: SESSION_HOST_FALLBACK,
    plugin_bridge_id: SESSION_PLUGIN_BRIDGE_ID,
    plugin_bridge_command: SESSION_PLUGIN_BRIDGE_COMMAND,
    wp_id: wpId,
    role,
    session_key: sessionKey(role, wpId),
    terminal_title: terminalTitleValue || terminalTitle(role, wpId),
    local_branch: normalizePath(localBranch),
    local_worktree_dir: normalizePath(localWorktreeDir),
    selected_model: selectedModel,
    selected_profile_id: selectedProfileId,
    reasoning_config_key: reasoningConfigKey,
    reasoning_config_value: reasoningConfigValue,
    startup_command: startupCommand,
    next_command: nextCommand,
    command,
    plugin_attempt_number: pluginAttemptNumber,
  };
}

export function queuePluginLaunch(repoRoot, registry, request) {
  const session = getOrCreateSessionRecord(registry, {
    wp_id: request.wp_id,
    role: request.role,
    local_branch: request.local_branch,
    local_worktree_dir: request.local_worktree_dir,
    terminal_title: request.terminal_title,
    requested_model: request.selected_model,
    requested_profile_id: request.selected_profile_id || "",
    reasoning_config_key: request.reasoning_config_key,
    reasoning_config_value: request.reasoning_config_value,
  });
  session.plugin_request_count += 1;
  session.plugin_last_request_id = request.request_id;
  session.plugin_last_request_at = request.created_at;
  session.plugin_last_result = "QUEUED";
  session.plugin_last_error = "";
  session.runtime_state = "PLUGIN_REQUESTED";
  session.last_event_at = request.created_at;
  appendJsonlLine(path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE), request);
  saveSessionRegistryUnlocked(repoRoot, registry);
  return session;
}

export function pendingRequestStatus(registry, requestId) {
  return registry.processed_requests.find((entry) => entry.request_id === requestId) || null;
}

export function markRequestProcessed(registry, requestId, status, extra = {}) {
  if (!SESSION_REQUEST_STATUSES.includes(status)) {
    throw new Error(`Unknown session request status: ${status}`);
  }
  const existing = registry.processed_requests.find((entry) => entry.request_id === requestId);
  const record = {
    request_id: requestId,
    status,
    processed_at: nowIso(),
    ...extra,
  };
  if (existing) {
    Object.assign(existing, record);
    return existing;
  }
  registry.processed_requests.push(record);
  return record;
}

export function markPluginResult(registry, session, requestId, status, details = {}) {
  if (!SESSION_REQUEST_STATUSES.includes(status)) {
    throw new Error(`Unknown plugin result status: ${status}`);
  }
  const processed = markRequestProcessed(registry, requestId, status, details);
  session.plugin_last_result = status;
  session.plugin_last_request_id = requestId;
  session.last_event_at = processed.processed_at;

  if (status === "PLUGIN_DISPATCHED" || status === "PLUGIN_CONFIRMED") {
    session.runtime_state = status === "PLUGIN_CONFIRMED" ? "PLUGIN_CONFIRMED" : "TERMINAL_COMMAND_DISPATCHED";
    session.active_host = SESSION_PLUGIN_HOST;
    session.active_terminal_title = details.terminal_title || session.terminal_title;
    session.active_terminal_kind = SESSION_PLUGIN_HOST;
    session.plugin_last_error = "";
  } else if (status === "CLI_ESCALATION_USED") {
    markCliEscalationUsed(session, {
      hostKind: details.host_kind || CLI_ESCALATION_HOST_DEFAULT,
      terminalTitle: details.terminal_title || session.terminal_title,
    });
  } else {
    session.plugin_failure_count += 1;
    session.runtime_state = session.plugin_failure_count >= SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION
      ? "CLI_ESCALATION_READY"
      : "UNSTARTED";
    session.plugin_last_error = details.error || status;
    session.last_error = details.error || status;
    recordBatchPluginFailure(registry, {
      requestId,
      sessionKey: session.session_key,
      error: details.error || status,
      status,
    });
  }

  session.cli_escalation_allowed = session.plugin_failure_count >= SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION;
  return processed;
}

export function markCliEscalationUsed(session, {
  hostKind = CLI_ESCALATION_HOST_DEFAULT,
  terminalTitle = "",
} = {}) {
  session.runtime_state = "CLI_ESCALATION_USED";
  session.active_host = SESSION_HOST_FALLBACK;
  session.active_terminal_title = terminalTitle || session.terminal_title;
  session.active_terminal_kind = hostKind;
  session.cli_escalation_used = true;
  session.startup_proof_state = "START_REQUESTED";
  session.last_error = "";
  session.last_event_at = nowIso();
  normalizeSessionRecord(session);
}

export function settleTimedOutPluginRequests(registry, requests, now = new Date()) {
  const thresholdMs = SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS * 1000;
  for (const request of requests) {
    if (!request || request.preferred_host !== SESSION_PLUGIN_HOST) continue;
    if (pendingRequestStatus(registry, request.request_id)) continue;
    const createdAtMs = Date.parse(request.created_at || "");
    if (Number.isNaN(createdAtMs)) continue;
    if ((now.getTime() - createdAtMs) < thresholdMs) continue;
    const session = registry.sessions.find((entry) => entry.session_key === request.session_key);
    if (!session) continue;
    markPluginResult(registry, session, request.request_id, "PLUGIN_TIMED_OUT", {
      error: `No ${SESSION_PLUGIN_BRIDGE_ID} acknowledgment within ${SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS}s`,
    });
  }
}

export function markSessionCommandQueued(session, command) {
  session.last_command_id = command.command_id;
  session.last_command_kind = command.command_kind;
  session.last_command_status = "QUEUED";
  session.last_command_summary = command.summary || "";
  session.last_command_prompt_at = command.created_at || nowIso();
  session.last_command_completed_at = "";
  session.last_command_output_file = command.output_jsonl_file || "";
  session.last_event_at = command.created_at || nowIso();
  upsertSessionActionHistory(session, command.governed_action, {
    command_id: command.command_id,
    command_kind: command.command_kind,
    action_state: "ACCEPTED_QUEUED",
    status: "QUEUED",
    outcome_state: "ACCEPTED_QUEUED",
    summary: command.summary || command.governed_action?.summary || "",
    requested_at: command.created_at || nowIso(),
    updated_at: command.created_at || nowIso(),
  });
  if (command.command_kind === "START_SESSION") {
    session.runtime_state = "STARTING";
    session.startup_proof_state = "START_REQUESTED";
  }
  normalizeSessionRecord(session);
}

export function markSessionCommandRunning(session, command) {
  removePendingSessionControlRequest(session, command.command_id);
  session.last_command_id = command.command_id;
  session.last_command_kind = command.command_kind;
  session.last_command_status = "RUNNING";
  session.last_command_summary = command.summary || "";
  session.last_command_prompt_at = command.created_at || nowIso();
  session.last_command_output_file = command.output_jsonl_file || "";
  session.last_event_at = nowIso();
  upsertSessionActionHistory(session, command.governed_action, {
    command_id: command.command_id,
    command_kind: command.command_kind,
    action_state: "ACCEPTED_RUNNING",
    status: "RUNNING",
    outcome_state: "ACCEPTED_RUNNING",
    summary: command.summary || command.governed_action?.summary || "",
    requested_at: command.created_at || nowIso(),
    updated_at: session.last_event_at,
  });
  session.runtime_state = command.command_kind === "START_SESSION" ? "STARTING" : "COMMAND_RUNNING";
  if (command.command_kind === "START_SESSION") {
    session.startup_proof_state = "START_REQUESTED";
  }
  normalizeSessionRecord(session);
}

export function markSessionThreadObserved(session, threadId, observedAt = nowIso()) {
  const normalizedThreadId = String(threadId || "").trim();
  if (!normalizedThreadId) return;
  session.session_thread_id = normalizedThreadId;
  session.session_thread_started_at = session.session_thread_started_at || observedAt;
  if (session.last_command_kind === "START_SESSION" && session.last_command_status === "RUNNING") {
    session.startup_proof_state = "START_REQUESTED";
    session.runtime_state = "STARTING";
  }
  session.last_event_at = observedAt;
  normalizeSessionRecord(session);
}

export function markSessionCommandResult(session, result) {
  removePendingSessionControlRequest(session, result.command_id);
  const status = String(result.status || "").trim().toUpperCase();
  if (!SESSION_COMMAND_STATUSES.includes(status)) {
    throw new Error(`Unknown session command status: ${status}`);
  }

  session.last_command_id = result.command_id || session.last_command_id;
  session.last_command_kind = result.command_kind || session.last_command_kind || "NONE";
  session.last_command_status = status;
  session.last_command_summary = result.summary || session.last_command_summary || "";
  session.last_command_completed_at = result.processed_at || nowIso();
  session.last_command_output_file = result.output_jsonl_file || session.last_command_output_file || "";
  session.last_event_at = result.processed_at || nowIso();
  upsertSessionActionHistory(session, result.governed_action, {
    command_id: result.command_id,
    command_kind: result.command_kind,
    action_state: result.governed_action?.result_state || status,
    status,
    outcome_state: result.outcome_state || "",
    resume_disposition: result.governed_action?.resume_disposition || "",
    summary: result.summary || result.governed_action?.summary || "",
    processed_at: result.processed_at || nowIso(),
    updated_at: result.processed_at || nowIso(),
  });

  if (result.thread_id) {
    session.session_thread_id = result.thread_id;
    session.session_thread_started_at = session.session_thread_started_at || (result.processed_at || nowIso());
  }

  if (status === "COMPLETED") {
    if (session.last_command_kind === "START_SESSION") {
      session.startup_proof_state = result.thread_id ? "READY" : "NO_THREAD_ID";
      session.runtime_state = result.thread_id ? "READY" : "FAILED";
    } else if (session.last_command_kind === "CLOSE_SESSION") {
      session.session_thread_id = "";
      session.session_thread_started_at = "";
      session.startup_proof_state = "CLOSED";
      session.runtime_state = "CLOSED";
      session.active_host = SESSION_ACTIVE_HOST_NONE;
      session.active_terminal_title = "";
      session.active_terminal_kind = SESSION_ACTIVE_TERMINAL_KIND_NONE;
    } else {
      session.runtime_state = "READY";
    }
    session.last_error = "";
  } else if (status === "FAILED") {
    if (session.last_command_kind === "START_SESSION") {
      session.startup_proof_state = "FAILED";
    }
    session.runtime_state = "FAILED";
    session.last_error = result.error || "session command failed";
  } else if (status === "RUNNING") {
    session.runtime_state = session.last_command_kind === "START_SESSION" ? "STARTING" : "COMMAND_RUNNING";
  }
  normalizeSessionRecord(session);
}

export function assertOrchestratorLaunchAuthority(currentBranch) {
  const branch = String(currentBranch || "").trim();
  if (branch !== "gov_kernel") {
    throw new Error(
      `Repo-governed session launch is ORCHESTRATOR_ONLY. Current branch must be gov_kernel (got: ${branch || "<unknown>"})`,
    );
  }
}

export function registrySessionSummary(session) {
  const actionHistory = normalizeSessionActionHistory(session.action_history);
  const pendingControlQueue = normalizePendingControlQueue(session.pending_control_queue);
  const nextQueuedControlRequest = pendingControlQueue.length > 0 ? pendingControlQueue[0] : null;
  const lastGovernedAction = actionHistory.length > 0 ? actionHistory[actionHistory.length - 1] : null;
  const effectiveGovernedAction = effectiveSessionGovernedAction({
    ...session,
    action_history: actionHistory,
    last_governed_action: lastGovernedAction,
  });
  return {
    session_key: session.session_key,
    role: session.role,
    wp_id: session.wp_id,
    local_branch: session.local_branch || "",
    local_worktree_dir: session.local_worktree_dir || "",
    requested_profile_id: session.requested_profile_id || "",
    runtime_state: session.runtime_state,
    control_mode: session.control_mode || SESSION_CONTROL_MODE,
    control_transport: session.control_transport || SESSION_CONTROL_TRANSPORT_PRIMARY,
    control_protocol: session.control_protocol || SESSION_CONTROL_PROTOCOL_PRIMARY,
    session_thread_id: session.session_thread_id || "",
    session_thread_started_at: session.session_thread_started_at || "",
    startup_proof_state: session.startup_proof_state || "NONE",
    preferred_host: session.preferred_host,
    active_host: session.active_host || "NONE",
    active_terminal_kind: session.active_terminal_kind || "NONE",
    terminal_ownership_scope: session.terminal_ownership_scope || SESSION_TERMINAL_OWNERSHIP_SCOPE_NONE,
    owned_terminal_process_id: session.owned_terminal_process_id || 0,
    owned_terminal_host_kind: session.owned_terminal_host_kind || "",
    owned_terminal_window_title: session.owned_terminal_window_title || "",
    owned_terminal_batch_scope: session.owned_terminal_batch_scope || "",
    owned_terminal_batch_id: session.owned_terminal_batch_id || "",
    owned_terminal_recorded_at: session.owned_terminal_recorded_at || "",
    owned_terminal_reclaimed_at: session.owned_terminal_reclaimed_at || "",
    owned_terminal_reclaim_status: session.owned_terminal_reclaim_status || SESSION_TERMINAL_RECLAIM_STATUS_NONE,
    plugin_request_count: session.plugin_request_count,
    plugin_failure_count: session.plugin_failure_count,
    cli_escalation_allowed: session.cli_escalation_allowed,
    cli_escalation_used: session.cli_escalation_used,
    plugin_last_result: session.plugin_last_result,
    plugin_last_error: session.plugin_last_error,
    active_terminal_title: session.active_terminal_title,
    health_state: session.health_state || "UNKNOWN",
    health_reason_code: session.health_reason_code || "UNKNOWN",
    health_summary: session.health_summary || "",
    health_source: session.health_source || SESSION_HEALTH_SOURCE,
    health_updated_at: session.health_updated_at || "",
    last_command_id: session.last_command_id || "",
    last_command_kind: session.last_command_kind || "NONE",
    last_command_status: session.last_command_status || "NONE",
    last_command_summary: session.last_command_summary || "",
    last_command_output_file: session.last_command_output_file || "",
    last_governed_action: lastGovernedAction,
    effective_governed_action: effectiveGovernedAction,
    action_history: actionHistory,
    pending_control_queue_count: pendingControlQueue.length,
    next_queued_control_request: nextQueuedControlRequest,
    pending_control_queue: pendingControlQueue,
    updated_at: session.last_event_at || "",
  };
}

export function registryBatchLaunchSummary(registry) {
  normalizeOwnedTerminalBatchAssignments(registry);
  return {
    launch_batch_mode: registry.launch_batch_mode,
    launch_batch_scope: registry.launch_batch_scope,
    launch_batch_plugin_failure_count: registry.launch_batch_plugin_failure_count,
    launch_batch_switched_at: registry.launch_batch_switched_at || "",
    launch_batch_switch_reason: registry.launch_batch_switch_reason || "",
    launch_batch_last_reset_at: registry.launch_batch_last_reset_at || "",
    terminal_batch_scope: registry.terminal_batch_scope || SESSION_BATCH_SCOPE,
    active_terminal_batch_id: registry.active_terminal_batch_id || "",
    active_terminal_batch_started_at: registry.active_terminal_batch_started_at || "",
    active_terminal_batch_last_rotated_at: registry.active_terminal_batch_last_rotated_at || "",
    active_terminal_batch_claimed_at: registry.active_terminal_batch_claimed_at || "",
    active_terminal_batch_reason: registry.active_terminal_batch_reason || "",
    batch_cli_escalation_active: registry.launch_batch_mode === SESSION_BATCH_MODE_CLI_ESCALATION,
  };
}

export function registryHasActiveBatchSessions(registry) {
  return (registry.sessions || []).some((session) => ACTIVE_BATCH_RUNTIME_STATES.has(String(session?.runtime_state || "").trim().toUpperCase()));
}

export function ensureSessionStateFiles(repoRoot) {
  const registryPath = saveSessionRegistry(repoRoot, loadSessionRegistry(repoRoot).registry);
  ensureParentDir(path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE));
  const requestsFile = path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE);
  if (!fs.existsSync(requestsFile)) fs.writeFileSync(requestsFile, "", "utf8");
  ensureParentDir(path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE));
  ensureParentDir(path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE));
  ensureParentDir(path.resolve(repoRoot, SESSION_CONTROL_OUTPUT_DIR, ".keep"));
  ensureParentDir(path.resolve(repoRoot, SESSION_CONTROL_BROKER_STATE_FILE));
  const controlRequestsFile = path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE);
  const controlResultsFile = path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE);
  if (!fs.existsSync(controlRequestsFile)) fs.writeFileSync(controlRequestsFile, "", "utf8");
  if (!fs.existsSync(controlResultsFile)) fs.writeFileSync(controlResultsFile, "", "utf8");
  return {
    registryPath,
    requestsFile,
    controlRequestsFile,
    controlResultsFile,
  };
}

export function validateRegistryShape(registry) {
  const errors = [];
  if (!registry || typeof registry !== "object") errors.push("registry must be an object");
  normalizeOwnedTerminalBatchAssignments(registry);
  if (registry.schema_id !== ROLE_SESSION_REGISTRY_SCHEMA_ID) errors.push(`schema_id must be ${ROLE_SESSION_REGISTRY_SCHEMA_ID}`);
  if (registry.schema_version !== ROLE_SESSION_REGISTRY_SCHEMA_VERSION) errors.push(`schema_version must be ${ROLE_SESSION_REGISTRY_SCHEMA_VERSION}`);
  if (registry.session_control_mode !== SESSION_CONTROL_MODE) errors.push(`session_control_mode must be ${SESSION_CONTROL_MODE}`);
  if (normalizePath(registry.session_plugin_requests_file || "") !== normalizePath(SESSION_PLUGIN_REQUESTS_FILE)) {
    errors.push(`session_plugin_requests_file must be ${SESSION_PLUGIN_REQUESTS_FILE}`);
  }
  if (registry.session_control_transport_primary !== SESSION_CONTROL_TRANSPORT_PRIMARY) {
    errors.push(`session_control_transport_primary must be ${SESSION_CONTROL_TRANSPORT_PRIMARY}`);
  }
  if (registry.session_control_protocol_primary !== SESSION_CONTROL_PROTOCOL_PRIMARY) {
    errors.push(`session_control_protocol_primary must be ${SESSION_CONTROL_PROTOCOL_PRIMARY}`);
  }
  if (normalizePath(registry.session_control_requests_file || "") !== normalizePath(SESSION_CONTROL_REQUESTS_FILE)) {
    errors.push(`session_control_requests_file must be ${SESSION_CONTROL_REQUESTS_FILE}`);
  }
  if (normalizePath(registry.session_control_results_file || "") !== normalizePath(SESSION_CONTROL_RESULTS_FILE)) {
    errors.push(`session_control_results_file must be ${SESSION_CONTROL_RESULTS_FILE}`);
  }
  if (normalizePath(registry.session_control_output_dir || "") !== normalizePath(SESSION_CONTROL_OUTPUT_DIR)) {
    errors.push(`session_control_output_dir must be ${SESSION_CONTROL_OUTPUT_DIR}`);
  }
  if (!SESSION_BATCH_MODE_VALUES.includes(registry.launch_batch_mode)) {
    errors.push(`launch_batch_mode must be one of ${SESSION_BATCH_MODE_VALUES.join(" | ")}`);
  }
  if (registry.launch_batch_scope !== SESSION_BATCH_SCOPE) {
    errors.push(`launch_batch_scope must be ${SESSION_BATCH_SCOPE}`);
  }
  if (!Number.isInteger(registry.launch_batch_plugin_failure_count) || registry.launch_batch_plugin_failure_count < 0) {
    errors.push("launch_batch_plugin_failure_count must be a non-negative integer");
  }
  if (registry.terminal_batch_scope !== SESSION_BATCH_SCOPE) {
    errors.push(`terminal_batch_scope must be ${SESSION_BATCH_SCOPE}`);
  }
  if (!String(registry.active_terminal_batch_id || "").trim()) {
    errors.push("active_terminal_batch_id is required");
  }
  if (!Array.isArray(registry.sessions)) errors.push("sessions must be an array");
  if (!Array.isArray(registry.processed_requests)) errors.push("processed_requests must be an array");
  for (const session of registry.sessions || []) {
    normalizeSessionRecord(session);
    if (!session.session_key) errors.push("session.session_key is required");
    if (!SESSION_RUNTIME_STATES.includes(session.runtime_state)) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid runtime_state ${session.runtime_state}`);
    }
    if (session.control_protocol !== SESSION_CONTROL_PROTOCOL_PRIMARY) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid control_protocol ${session.control_protocol}`);
    }
    if (!SESSION_ACTIVE_HOST_VALUES.includes(session.active_host || "NONE")) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid active_host ${session.active_host}`);
    }
    if (!SESSION_ACTIVE_TERMINAL_KIND_VALUES.includes(session.active_terminal_kind || "NONE")) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid active_terminal_kind ${session.active_terminal_kind}`);
    }
    if (!SESSION_TERMINAL_OWNERSHIP_SCOPE_VALUES.includes(session.terminal_ownership_scope || SESSION_TERMINAL_OWNERSHIP_SCOPE_NONE)) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid terminal_ownership_scope ${session.terminal_ownership_scope}`);
    }
    if (!Number.isInteger(session.owned_terminal_process_id || 0) || (session.owned_terminal_process_id || 0) < 0) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid owned_terminal_process_id ${session.owned_terminal_process_id}`);
    }
    if (sessionHasOwnedTerminal(session)) {
      if (!String(session.owned_terminal_batch_id || "").trim()) {
        errors.push(`session ${session.session_key || "<missing>"} is missing owned_terminal_batch_id`);
      }
      if (String(session.owned_terminal_batch_scope || "") !== SESSION_BATCH_SCOPE) {
        errors.push(`session ${session.session_key || "<missing>"} has invalid owned_terminal_batch_scope ${session.owned_terminal_batch_scope}`);
      }
    }
    if (!SESSION_TERMINAL_RECLAIM_STATUS_VALUES.includes(session.owned_terminal_reclaim_status || SESSION_TERMINAL_RECLAIM_STATUS_NONE)) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid owned_terminal_reclaim_status ${session.owned_terminal_reclaim_status}`);
    }
    if (session.last_command_status && session.last_command_status !== "NONE" && !SESSION_COMMAND_STATUSES.includes(session.last_command_status)) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid last_command_status ${session.last_command_status}`);
    }
    if (!Array.isArray(session.action_history || [])) {
      errors.push(`session ${session.session_key || "<missing>"} action_history must be an array`);
    } else {
      for (const entry of session.action_history || []) {
        const normalizedEntry = normalizeSessionActionHistoryEntry(entry);
        if (!normalizedEntry) {
          errors.push(`session ${session.session_key || "<missing>"} has invalid action_history entry`);
          continue;
        }
        if (!normalizedEntry.command_id) {
          errors.push(`session ${session.session_key || "<missing>"} action_history entry ${normalizedEntry.action_id} is missing command_id`);
        }
      }
    }
    if (!Array.isArray(session.pending_control_queue || [])) {
      errors.push(`session ${session.session_key || "<missing>"} pending_control_queue must be an array`);
    } else {
      for (const entry of session.pending_control_queue || []) {
        const normalizedEntry = normalizePendingControlQueueEntry(entry);
        if (!normalizedEntry) {
          errors.push(`session ${session.session_key || "<missing>"} has invalid pending_control_queue entry`);
          continue;
        }
        if (!normalizedEntry.output_jsonl_file) {
          errors.push(`session ${session.session_key || "<missing>"} pending_control_queue entry ${normalizedEntry.command_id} is missing output_jsonl_file`);
        }
      }
    }
    if (!SESSION_HEALTH_STATE_VALUES.includes(session.health_state || "UNKNOWN")) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid health_state ${session.health_state}`);
    }
  }
  for (const entry of registry.processed_requests || []) {
    if (!SESSION_REQUEST_STATUSES.includes(entry.status)) {
      errors.push(`processed request ${entry.request_id || "<missing>"} has invalid status ${entry.status}`);
    }
  }
  return errors;
}

export function validateLaunchRequestShape(request) {
  const errors = [];
  if (!request || typeof request !== "object") return ["request must be an object"];
  if (request.schema_id !== SESSION_LAUNCH_REQUEST_SCHEMA_ID) errors.push(`schema_id must be ${SESSION_LAUNCH_REQUEST_SCHEMA_ID}`);
  if (request.schema_version !== SESSION_LAUNCH_REQUEST_SCHEMA_VERSION) errors.push(`schema_version must be ${SESSION_LAUNCH_REQUEST_SCHEMA_VERSION}`);
  if (request.launch_authority !== SESSION_START_AUTHORITY) errors.push(`launch_authority must be ${SESSION_START_AUTHORITY}`);
  if (request.preferred_host !== SESSION_PLUGIN_HOST) {
    errors.push(`preferred_host must be ${SESSION_PLUGIN_HOST} for compatibility queue launches`);
  }
  if (![SESSION_HOST_FALLBACK, SESSION_HOST_FALLBACK_LEGACY].includes(request.fallback_host)) {
    errors.push(`fallback_host must be ${SESSION_HOST_FALLBACK} for compatibility queue launches`);
  }
  if (!request.request_id) errors.push("request_id is required");
  if (!request.role) errors.push("role is required");
  if (!request.wp_id) errors.push("wp_id is required");
  if (!request.local_worktree_dir) errors.push("local_worktree_dir is required");
  if (!request.command) errors.push("command is required");
  return errors;
}
