import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import {
  SESSION_ACTIVE_HOST_NONE,
  SESSION_ACTIVE_HOST_VALUES,
  SESSION_ACTIVE_TERMINAL_KIND_NONE,
  SESSION_ACTIVE_TERMINAL_KIND_VALUES,
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
  normalizeActiveHostValue,
  normalizeActiveTerminalKindValue,
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
  session.last_heartbeat_at = session.last_heartbeat_at || "";
  session.last_error = session.last_error || "";
  session.last_event_at = session.last_event_at || "";
  return session;
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
  registry.sessions = Array.isArray(registry.sessions) ? registry.sessions.map((session) => normalizeSessionRecord(session)) : [];
  registry.processed_requests = Array.isArray(registry.processed_requests) ? registry.processed_requests : [];
  return { filePath, registry };
}

export function saveSessionRegistry(repoRoot, registry) {
  const filePath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
  const next = {
    ...defaultRegistry(),
    ...registry,
    updated_at: nowIso(),
    sessions: Array.isArray(registry.sessions) ? registry.sessions.map((session) => normalizeSessionRecord(session)) : [],
    processed_requests: Array.isArray(registry.processed_requests) ? registry.processed_requests : [],
  };
  writeJsonFile(filePath, next);
  return filePath;
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

  if (status === "PLUGIN_DISPATCHED" || status === "PLUGIN_CONFIRMED") {
    session.runtime_state = status === "PLUGIN_CONFIRMED" ? "PLUGIN_CONFIRMED" : "TERMINAL_COMMAND_DISPATCHED";
    session.active_host = SESSION_HOST_PREFERENCE;
    session.active_terminal_title = details.terminal_title || session.terminal_title;
    session.active_terminal_kind = SESSION_HOST_PREFERENCE;
    session.plugin_last_error = "";
  } else if (status === "CLI_ESCALATION_USED") {
    session.runtime_state = "CLI_ESCALATION_USED";
    session.active_host = SESSION_HOST_FALLBACK;
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

export function markSessionCommandQueued(session, command) {
  session.last_command_id = command.command_id;
  session.last_command_kind = command.command_kind;
  session.last_command_status = "QUEUED";
  session.last_command_summary = command.summary || "";
  session.last_command_prompt_at = command.created_at || nowIso();
  session.last_command_completed_at = "";
  session.last_command_output_file = command.output_jsonl_file || "";
  session.last_event_at = command.created_at || nowIso();
  if (command.command_kind === "START_SESSION") {
    session.runtime_state = "STARTING";
    session.startup_proof_state = "START_REQUESTED";
  }
  normalizeSessionRecord(session);
}

export function markSessionCommandRunning(session, command) {
  session.last_command_id = command.command_id;
  session.last_command_kind = command.command_kind;
  session.last_command_status = "RUNNING";
  session.last_command_summary = command.summary || "";
  session.last_command_prompt_at = command.created_at || nowIso();
  session.last_command_output_file = command.output_jsonl_file || "";
  session.last_event_at = nowIso();
  session.runtime_state = command.command_kind === "START_SESSION" ? "STARTING" : "COMMAND_RUNNING";
  normalizeSessionRecord(session);
}

export function markSessionCommandResult(session, result) {
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
    control_mode: session.control_mode || SESSION_CONTROL_MODE,
    control_transport: session.control_transport || SESSION_CONTROL_TRANSPORT_PRIMARY,
    control_protocol: session.control_protocol || SESSION_CONTROL_PROTOCOL_PRIMARY,
    session_thread_id: session.session_thread_id || "",
    session_thread_started_at: session.session_thread_started_at || "",
    startup_proof_state: session.startup_proof_state || "NONE",
    preferred_host: session.preferred_host,
    active_host: session.active_host || "NONE",
    active_terminal_kind: session.active_terminal_kind || "NONE",
    plugin_request_count: session.plugin_request_count,
    plugin_failure_count: session.plugin_failure_count,
    cli_escalation_allowed: session.cli_escalation_allowed,
    cli_escalation_used: session.cli_escalation_used,
    plugin_last_result: session.plugin_last_result,
    plugin_last_error: session.plugin_last_error,
    active_terminal_title: session.active_terminal_title,
    last_command_id: session.last_command_id || "",
    last_command_kind: session.last_command_kind || "NONE",
    last_command_status: session.last_command_status || "NONE",
    last_command_summary: session.last_command_summary || "",
    last_command_output_file: session.last_command_output_file || "",
    updated_at: session.last_event_at || "",
  };
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
  if (registry.schema_id !== ROLE_SESSION_REGISTRY_SCHEMA_ID) errors.push(`schema_id must be ${ROLE_SESSION_REGISTRY_SCHEMA_ID}`);
  if (registry.schema_version !== ROLE_SESSION_REGISTRY_SCHEMA_VERSION) errors.push(`schema_version must be ${ROLE_SESSION_REGISTRY_SCHEMA_VERSION}`);
  if (registry.session_control_mode !== SESSION_CONTROL_MODE) errors.push(`session_control_mode must be ${SESSION_CONTROL_MODE}`);
  if (registry.session_control_transport_primary !== SESSION_CONTROL_TRANSPORT_PRIMARY) {
    errors.push(`session_control_transport_primary must be ${SESSION_CONTROL_TRANSPORT_PRIMARY}`);
  }
  if (registry.session_control_protocol_primary !== SESSION_CONTROL_PROTOCOL_PRIMARY) {
    errors.push(`session_control_protocol_primary must be ${SESSION_CONTROL_PROTOCOL_PRIMARY}`);
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
    if (session.last_command_status && session.last_command_status !== "NONE" && !SESSION_COMMAND_STATUSES.includes(session.last_command_status)) {
      errors.push(`session ${session.session_key || "<missing>"} has invalid last_command_status ${session.last_command_status}`);
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
  if (!request.local_worktree_dir) errors.push("local_worktree_dir is required");
  if (!request.command) errors.push("command is required");
  return errors;
}
