import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import {
  CLI_ESCALATION_HOST_DEFAULT,
  SESSION_HOST_FALLBACK,
  SESSION_HOST_PREFERENCE,
  SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
  SESSION_PLUGIN_BRIDGE_COMMAND,
  SESSION_PLUGIN_BRIDGE_ID,
  SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
  SESSION_PLUGIN_REQUESTS_FILE,
  SESSION_REGISTRY_FILE,
  SESSION_REQUEST_STATUSES,
  SESSION_RUNTIME_STATES,
  SESSION_START_AUTHORITY,
  SESSION_WAKE_CHANNEL_FALLBACK,
  SESSION_WAKE_CHANNEL_PRIMARY,
  SESSION_WATCH_POLICY,
  normalizePath,
  sessionKey,
  terminalTitle,
} from "./session-policy.mjs";

export const ROLE_SESSION_REGISTRY_SCHEMA_ID = "hsk.role_session_registry@1";
export const ROLE_SESSION_REGISTRY_SCHEMA_VERSION = "role_session_registry_v1";
export const SESSION_LAUNCH_REQUEST_SCHEMA_ID = "hsk.session_launch_request@1";
export const SESSION_LAUNCH_REQUEST_SCHEMA_VERSION = "session_launch_request_v1";

function nowIso() {
  return new Date().toISOString();
}

function ensureParentDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

export function defaultRegistry() {
  return {
    schema_id: ROLE_SESSION_REGISTRY_SCHEMA_ID,
    schema_version: ROLE_SESSION_REGISTRY_SCHEMA_VERSION,
    updated_at: nowIso(),
    session_start_authority: SESSION_START_AUTHORITY,
    session_host_preference: SESSION_HOST_PREFERENCE,
    session_host_fallback: SESSION_HOST_FALLBACK,
    session_watch_policy: SESSION_WATCH_POLICY,
    session_plugin_bridge_id: SESSION_PLUGIN_BRIDGE_ID,
    session_plugin_bridge_command: SESSION_PLUGIN_BRIDGE_COMMAND,
    session_plugin_requests_file: SESSION_PLUGIN_REQUESTS_FILE,
    session_wake_channel_primary: SESSION_WAKE_CHANNEL_PRIMARY,
    session_wake_channel_fallback: SESSION_WAKE_CHANNEL_FALLBACK,
    session_plugin_max_retries_before_escalation: SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
    session_plugin_attempt_timeout_seconds: SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
    sessions: [],
    processed_requests: [],
  };
}

export function readJsonFile(filePath, fallbackValue) {
  if (!fs.existsSync(filePath)) return JSON.parse(JSON.stringify(fallbackValue));
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

export function writeJsonFile(filePath, value) {
  ensureParentDir(filePath);
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
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

export function appendJsonlLine(filePath, value) {
  ensureParentDir(filePath);
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, "utf8");
}

export function loadSessionRegistry(repoRoot) {
  const filePath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
  const registry = readJsonFile(filePath, defaultRegistry());
  registry.sessions = Array.isArray(registry.sessions) ? registry.sessions : [];
  registry.processed_requests = Array.isArray(registry.processed_requests) ? registry.processed_requests : [];
  return { filePath, registry };
}

export function saveSessionRegistry(repoRoot, registry) {
  const filePath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
  const next = {
    ...defaultRegistry(),
    ...registry,
    updated_at: nowIso(),
    sessions: Array.isArray(registry.sessions) ? registry.sessions : [],
    processed_requests: Array.isArray(registry.processed_requests) ? registry.processed_requests : [],
  };
  writeJsonFile(filePath, next);
  return filePath;
}

export function loadSessionLaunchRequests(repoRoot) {
  const filePath = path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE);
  return { filePath, requests: parseJsonlFile(filePath) };
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
      reasoning_config_key: sessionDescriptor.reasoning_config_key || "",
      reasoning_config_value: sessionDescriptor.reasoning_config_value || "",
      plugin_request_count: 0,
      plugin_failure_count: 0,
      plugin_last_request_id: "",
      plugin_last_request_at: "",
      plugin_last_result: "NONE",
      plugin_last_error: "",
      cli_escalation_allowed: false,
      cli_escalation_used: false,
      runtime_state: "UNSTARTED",
      active_host: "NONE",
      active_terminal_title: "",
      active_terminal_kind: "",
      last_heartbeat_at: "",
      last_error: "",
      last_event_at: "",
    };
    registry.sessions.push(session);
  } else {
    session.local_branch = normalizePath(sessionDescriptor.local_branch || session.local_branch || "");
    session.local_worktree_dir = normalizePath(sessionDescriptor.local_worktree_dir || session.local_worktree_dir || "");
    session.terminal_title = sessionDescriptor.terminal_title || session.terminal_title || terminalTitle(sessionDescriptor.role, sessionDescriptor.wp_id);
    session.requested_model = sessionDescriptor.requested_model || session.requested_model || "";
    session.reasoning_config_key = sessionDescriptor.reasoning_config_key || session.reasoning_config_key || "";
    session.reasoning_config_value = sessionDescriptor.reasoning_config_value || session.reasoning_config_value || "";
  }
  return session;
}

export function buildLaunchRequest({
  wpId,
  role,
  localBranch,
  localWorktreeDir,
  absWorktreeDir,
  selectedModel,
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
    preferred_host: SESSION_HOST_PREFERENCE,
    fallback_host: SESSION_HOST_FALLBACK,
    plugin_bridge_id: SESSION_PLUGIN_BRIDGE_ID,
    plugin_bridge_command: SESSION_PLUGIN_BRIDGE_COMMAND,
    wp_id: wpId,
    role,
    session_key: sessionKey(role, wpId),
    terminal_title: terminalTitleValue || terminalTitle(role, wpId),
    local_branch: normalizePath(localBranch),
    local_worktree_dir: normalizePath(localWorktreeDir),
    abs_worktree_dir: normalizePath(absWorktreeDir),
    selected_model: selectedModel,
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
  saveSessionRegistry(repoRoot, registry);
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

  if (status === "PLUGIN_CONFIRMED") {
    session.runtime_state = "PLUGIN_CONFIRMED";
    session.active_host = SESSION_HOST_PREFERENCE;
    session.active_terminal_title = details.terminal_title || session.terminal_title;
    session.active_terminal_kind = "VSCODE_EXTENSION_TERMINAL";
    session.plugin_last_error = "";
  } else if (status === "CLI_ESCALATION_USED") {
    session.runtime_state = "CLI_ESCALATION_USED";
    session.active_host = details.host_kind || CLI_ESCALATION_HOST_DEFAULT;
    session.active_terminal_title = details.terminal_title || session.terminal_title;
    session.active_terminal_kind = details.host_kind || CLI_ESCALATION_HOST_DEFAULT;
    session.cli_escalation_used = true;
    session.last_error = "";
  } else {
    session.plugin_failure_count += 1;
    session.runtime_state = session.plugin_failure_count >= SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION
      ? "CLI_ESCALATION_READY"
      : "UNSTARTED";
    session.plugin_last_error = details.error || status;
    session.last_error = details.error || status;
  }

  session.cli_escalation_allowed = session.plugin_failure_count >= SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION;
  return processed;
}

export function settleTimedOutPluginRequests(registry, requests, now = new Date()) {
  const thresholdMs = SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS * 1000;
  for (const request of requests) {
    if (!request || request.preferred_host !== SESSION_HOST_PREFERENCE) continue;
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

export function assertOrchestratorLaunchAuthority(currentBranch) {
  const branch = String(currentBranch || "").trim();
  if (branch !== "role_orchestrator") {
    throw new Error(
      `Repo-governed session launch is ORCHESTRATOR_ONLY. Current branch must be role_orchestrator (got: ${branch || "<unknown>"})`,
    );
  }
}

export function registrySessionSummary(session) {
  return {
    session_key: session.session_key,
    role: session.role,
    wp_id: session.wp_id,
    runtime_state: session.runtime_state,
    preferred_host: session.preferred_host,
    active_host: session.active_host,
    plugin_request_count: session.plugin_request_count,
    plugin_failure_count: session.plugin_failure_count,
    cli_escalation_allowed: session.cli_escalation_allowed,
    cli_escalation_used: session.cli_escalation_used,
    plugin_last_result: session.plugin_last_result,
    plugin_last_error: session.plugin_last_error,
    active_terminal_title: session.active_terminal_title,
    updated_at: session.last_event_at || "",
  };
}

export function ensureSessionStateFiles(repoRoot) {
  const registryPath = saveSessionRegistry(repoRoot, loadSessionRegistry(repoRoot).registry);
  ensureParentDir(path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE));
  const requestsFile = path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE);
  if (!fs.existsSync(requestsFile)) fs.writeFileSync(requestsFile, "", "utf8");
  return {
    registryPath,
    requestsFile,
  };
}

export function validateRegistryShape(registry) {
  const errors = [];
  if (!registry || typeof registry !== "object") errors.push("registry must be an object");
  if (registry.schema_id !== ROLE_SESSION_REGISTRY_SCHEMA_ID) errors.push(`schema_id must be ${ROLE_SESSION_REGISTRY_SCHEMA_ID}`);
  if (registry.schema_version !== ROLE_SESSION_REGISTRY_SCHEMA_VERSION) errors.push(`schema_version must be ${ROLE_SESSION_REGISTRY_SCHEMA_VERSION}`);
  if (!Array.isArray(registry.sessions)) errors.push("sessions must be an array");
  if (!Array.isArray(registry.processed_requests)) errors.push("processed_requests must be an array");
  for (const session of registry.sessions || []) {
    if (!session.session_key) errors.push("session.session_key is required");
    if (!SESSION_RUNTIME_STATES.includes(session.runtime_state)) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid runtime_state ${session.runtime_state}`);
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
  if (request.preferred_host !== SESSION_HOST_PREFERENCE) errors.push(`preferred_host must be ${SESSION_HOST_PREFERENCE}`);
  if (request.fallback_host !== SESSION_HOST_FALLBACK) errors.push(`fallback_host must be ${SESSION_HOST_FALLBACK}`);
  if (!request.request_id) errors.push("request_id is required");
  if (!request.role) errors.push("role is required");
  if (!request.wp_id) errors.push("wp_id is required");
  if (!request.command) errors.push("command is required");
  return errors;
}
