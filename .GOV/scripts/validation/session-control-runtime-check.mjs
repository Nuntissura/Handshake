import fs from "node:fs";
import path from "node:path";
import {
  SESSION_CONTROL_BROKER_STATE_FILE,
  SESSION_CONTROL_PROTOCOL_PRIMARY,
  SESSION_CONTROL_REQUESTS_FILE,
  SESSION_CONTROL_RESULTS_FILE,
  SESSION_CONTROL_RUN_STALE_GRACE_SECONDS,
  SESSION_CONTROL_RUN_TIMEOUT_SECONDS,
  SESSION_CONTROL_TRANSPORT_PRIMARY,
  SESSION_CONTROL_OUTPUT_DIR,
  SESSION_REGISTRY_FILE,
  normalizePath,
} from "../session-policy.mjs";
import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionRegistry,
  validateRegistryShape,
} from "../session-registry-lib.mjs";
import {
  validateSessionControlRequestShape,
  validateSessionControlResultShape,
} from "../session-control-lib.mjs";

const repoRoot = process.cwd();
const registryPath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
const requestsPath = path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE);
const resultsPath = path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE);
const outputDirPath = path.resolve(repoRoot, SESSION_CONTROL_OUTPUT_DIR);
const brokerStatePath = path.resolve(repoRoot, SESSION_CONTROL_BROKER_STATE_FILE);
const maxUnsettledAgeMs = (SESSION_CONTROL_RUN_TIMEOUT_SECONDS + SESSION_CONTROL_RUN_STALE_GRACE_SECONDS) * 1000;
const GOVERNED_COMMAND_KINDS = new Set(["START_SESSION", "SEND_PROMPT", "CANCEL_SESSION"]);

function fail(message, details = []) {
  console.error(`[SESSION_CONTROL_RUNTIME_CHECK] ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(1);
}

function readJson(filePath, fallbackValue = null) {
  if (!fs.existsSync(filePath)) return fallbackValue;
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function toAbsNormalized(filePath) {
  return normalizePath(path.resolve(repoRoot, String(filePath || "")));
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

function pushShapeErrors(lines, validator, itemName) {
  const errors = [];
  for (let index = 0; index < lines.length; index += 1) {
    const itemErrors = validator(lines[index]);
    for (const error of itemErrors) errors.push(`line ${index + 1}: ${itemName} ${error}`);
  }
  return errors;
}

function collectDuplicateIds(items, label) {
  const seen = new Set();
  const duplicates = [];
  for (const item of items) {
    const id = String(item?.command_id || "").trim();
    if (!id) continue;
    if (seen.has(id)) duplicates.push(`${label} ${id} appears more than once`);
    seen.add(id);
  }
  return duplicates;
}

function normalizedCommandKind(value) {
  return String(value || "").trim().toUpperCase();
}

function resolveTargetCommandId(item) {
  const targetId = String(item?.cancel_target_command_id || item?.target_command_id || "").trim();
  return targetId || "";
}

function validateRequestShapeForRuntime(request) {
  const kind = normalizedCommandKind(request?.command_kind);
  const legacyErrors = validateSessionControlRequestShape(request);
  const errors = [];

  if (kind === "CANCEL_SESSION") {
    for (const error of legacyErrors) {
      if (error === "command_kind is invalid" || error === "prompt is required") continue;
      errors.push(error);
    }
  } else {
    errors.push(...legacyErrors);
  }

  if (!GOVERNED_COMMAND_KINDS.has(kind)) errors.push("command_kind is invalid");
  if (!request?.created_at) errors.push("created_at is required");
  if (request?.created_by_role !== "ORCHESTRATOR") errors.push("created_by_role must be ORCHESTRATOR");
  if (!request?.session_key) errors.push("session_key is required");
  if (!request?.wp_id) errors.push("wp_id is required");
  if (!request?.role) errors.push("role is required");
  if (!request?.output_jsonl_file) errors.push("output_jsonl_file is required");
  if (kind === "START_SESSION" || kind === "SEND_PROMPT") {
    if (!request?.selected_model) errors.push("selected_model is required");
    if (!request?.prompt) errors.push("prompt is required");
  }
  if (kind === "CANCEL_SESSION" && !resolveTargetCommandId(request)) {
    errors.push("CANCEL_SESSION requires target_command_id or cancel_target_command_id");
  }
  return [...new Set(errors)];
}

function validateResultShapeForRuntime(result) {
  const kind = normalizedCommandKind(result?.command_kind);
  const legacyErrors = validateSessionControlResultShape(result);
  const errors = [];

  if (kind === "CANCEL_SESSION") {
    for (const error of legacyErrors) {
      if (error === "command_kind is invalid") continue;
      errors.push(error);
    }
  } else {
    errors.push(...legacyErrors);
  }

  if (!GOVERNED_COMMAND_KINDS.has(kind)) errors.push("command_kind is invalid");
  if (!result?.processed_at) errors.push("processed_at is required");
  if (!result?.command_id) errors.push("command_id is required");
  if (!result?.session_key) errors.push("session_key is required");
  if (!result?.wp_id) errors.push("wp_id is required");
  if (!result?.role) errors.push("role is required");
  if (!result?.output_jsonl_file) errors.push("output_jsonl_file is required");
  if (kind === "CANCEL_SESSION" && !resolveTargetCommandId(result)) {
    errors.push("CANCEL_SESSION requires target_command_id or cancel_target_command_id");
  }
  return [...new Set(errors)];
}

function validateBrokerStateShape(state) {
  const errors = [];
  if (!state || typeof state !== "object") return ["broker state must be an object"];
  if (state.schema_id !== "hsk.session_control_broker_state@1") {
    errors.push("broker state schema_id must be hsk.session_control_broker_state@1");
  }
  if (state.schema_version !== "session_control_broker_state_v1") {
    errors.push("broker state schema_version must be session_control_broker_state_v1");
  }
  if (state.protocol !== SESSION_CONTROL_PROTOCOL_PRIMARY) {
    errors.push(`broker state protocol must be ${SESSION_CONTROL_PROTOCOL_PRIMARY}`);
  }
  if (state.control_transport !== SESSION_CONTROL_TRANSPORT_PRIMARY) {
    errors.push(`broker state control_transport must be ${SESSION_CONTROL_TRANSPORT_PRIMARY}`);
  }
  if ("broker_build_id" in state && !String(state.broker_build_id || "").trim()) {
    errors.push("broker state broker_build_id must be a non-empty string when present");
  }
  if ("broker_version" in state && !String(state.broker_version || "").trim()) {
    errors.push("broker state broker_version must be a non-empty string when present");
  }
  if ("broker_build_version" in state && !String(state.broker_build_version || "").trim()) {
    errors.push("broker state broker_build_version must be a non-empty string when present");
  }
  if (!Array.isArray(state.active_runs)) {
    errors.push("broker state active_runs must be an array");
    return errors;
  }
  for (const run of state.active_runs) {
    if (!run.command_id) errors.push("broker active run command_id is required");
    if (!run.session_key) errors.push(`broker active run ${run.command_id || "<missing>"} session_key is required`);
    if (!run.wp_id) errors.push(`broker active run ${run.command_id || "<missing>"} wp_id is required`);
    if (!run.role) errors.push(`broker active run ${run.command_id || "<missing>"} role is required`);
    if (!run.command_kind) errors.push(`broker active run ${run.command_id || "<missing>"} command_kind is required`);
    if (!run.started_at) errors.push(`broker active run ${run.command_id || "<missing>"} started_at is required`);
    if (!run.timeout_at) errors.push(`broker active run ${run.command_id || "<missing>"} timeout_at is required`);
    if (!run.output_jsonl_file) errors.push(`broker active run ${run.command_id || "<missing>"} output_jsonl_file is required`);
  }
  return errors;
}

if (!fs.existsSync(registryPath)) {
  fail("Missing role session registry", [SESSION_REGISTRY_FILE]);
}

if (!fs.existsSync(requestsPath)) {
  fail("Missing session control requests file", [SESSION_CONTROL_REQUESTS_FILE]);
}

if (!fs.existsSync(resultsPath)) {
  fail("Missing session control results file", [SESSION_CONTROL_RESULTS_FILE]);
}

if (!fs.existsSync(outputDirPath)) {
  fail("Missing session control output directory", [SESSION_CONTROL_OUTPUT_DIR]);
}

const { registry } = loadSessionRegistry(repoRoot);
const registryErrors = validateRegistryShape(registry);
if (registryErrors.length > 0) {
  fail("Role session registry schema violations found", registryErrors);
}

const { requests } = loadSessionControlRequests(repoRoot);
const { results } = loadSessionControlResults(repoRoot);

const requestErrors = pushShapeErrors(requests, validateRequestShapeForRuntime, "request");
if (requestErrors.length > 0) {
  fail("Session control request schema violations found", requestErrors);
}

const resultErrors = pushShapeErrors(results, validateResultShapeForRuntime, "result");
if (resultErrors.length > 0) {
  fail("Session control result schema violations found", resultErrors);
}

const invariantErrors = [];
invariantErrors.push(...collectDuplicateIds(requests, "request command_id"));
invariantErrors.push(...collectDuplicateIds(results, "result command_id"));

const sessionByKey = new Map((registry.sessions || []).map((session) => [session.session_key, session]));
const requestById = new Map();
for (const request of requests) requestById.set(request.command_id, request);

const resultById = new Map();
for (const result of results) resultById.set(result.command_id, result);

for (const request of requests) {
  const session = sessionByKey.get(request.session_key);
  if (!session) {
    invariantErrors.push(`request ${request.command_id} references missing session ${request.session_key}`);
    continue;
  }
  if (session.role !== request.role || session.wp_id !== request.wp_id) {
    invariantErrors.push(`request ${request.command_id} identity disagrees with session registry for ${request.session_key}`);
  }
  if (normalizePath(session.local_branch || "") !== normalizePath(request.local_branch || "")) {
    invariantErrors.push(`request ${request.command_id} local_branch drifts from session registry`);
  }
  if (normalizePath(session.local_worktree_dir || "") !== normalizePath(request.local_worktree_dir || "")) {
    invariantErrors.push(`request ${request.command_id} local_worktree_dir drifts from session registry`);
  }
  if (normalizedCommandKind(request.command_kind) === "CANCEL_SESSION") {
    const targetCommandId = resolveTargetCommandId(request);
    const targetRequest = requestById.get(targetCommandId);
    if (!targetCommandId) {
      invariantErrors.push(`request ${request.command_id} CANCEL_SESSION is missing a target command reference`);
    } else if (!targetRequest) {
      invariantErrors.push(`request ${request.command_id} CANCEL_SESSION target ${targetCommandId} has no matching request`);
    } else {
      if (targetCommandId === request.command_id) {
        invariantErrors.push(`request ${request.command_id} CANCEL_SESSION cannot target itself`);
      }
      if (targetRequest.session_key !== request.session_key) {
        invariantErrors.push(`request ${request.command_id} CANCEL_SESSION target session_key does not match target request`);
      }
      if (targetRequest.wp_id !== request.wp_id) {
        invariantErrors.push(`request ${request.command_id} CANCEL_SESSION target wp_id does not match target request`);
      }
      if (targetRequest.role !== request.role) {
        invariantErrors.push(`request ${request.command_id} CANCEL_SESSION target role does not match target request`);
      }
    }
  }
}

for (const result of results) {
  const request = requestById.get(result.command_id);
  if (!request) {
    invariantErrors.push(`result ${result.command_id} has no matching request`);
    continue;
  }
  if (request.command_kind !== result.command_kind) {
    invariantErrors.push(`result ${result.command_id} command_kind does not match request`);
  }
  if (request.session_key !== result.session_key) {
    invariantErrors.push(`result ${result.command_id} session_key does not match request`);
  }
  if (request.wp_id !== result.wp_id) {
    invariantErrors.push(`result ${result.command_id} wp_id does not match request`);
  }
  if (request.role !== result.role) {
    invariantErrors.push(`result ${result.command_id} role does not match request`);
  }
  if (toAbsNormalized(request.output_jsonl_file) !== toAbsNormalized(result.output_jsonl_file)) {
    invariantErrors.push(`result ${result.command_id} output_jsonl_file does not match request`);
  }
  if (normalizedCommandKind(result.command_kind) === "CANCEL_SESSION") {
    const requestTargetCommandId = resolveTargetCommandId(request);
    const resultTargetCommandId = resolveTargetCommandId(result);
    if (requestTargetCommandId !== resultTargetCommandId) {
      invariantErrors.push(`result ${result.command_id} CANCEL_SESSION target does not match request`);
    }
  }
  const outputPath = toAbsNormalized(result.output_jsonl_file);
  if (!fs.existsSync(outputPath)) {
    invariantErrors.push(`result ${result.command_id} is missing output log ${outputPath}`);
  }
}

let brokerState = null;
if (fs.existsSync(brokerStatePath)) {
  brokerState = readJson(brokerStatePath, null);
  const brokerErrors = validateBrokerStateShape(brokerState);
  for (const error of brokerErrors) invariantErrors.push(error);
}

const brokerRunById = new Map();
if (brokerState?.active_runs?.length) {
  for (const duplicate of collectDuplicateIds(brokerState.active_runs, "broker active run")) {
    invariantErrors.push(duplicate);
  }
  if (!isProcessAlive(brokerState.broker_pid)) {
    invariantErrors.push("broker state lists active_runs but broker_pid is not alive");
  }
  for (const run of brokerState.active_runs) {
    brokerRunById.set(run.command_id, run);
    const request = requestById.get(run.command_id);
    if (!request) {
      invariantErrors.push(`broker active run ${run.command_id} has no matching request`);
      continue;
    }
    if (resultById.has(run.command_id)) {
      invariantErrors.push(`broker active run ${run.command_id} already has a settled result`);
    }
    if (request.session_key !== run.session_key) {
      invariantErrors.push(`broker active run ${run.command_id} session_key does not match request`);
    }
    if (request.wp_id !== run.wp_id) {
      invariantErrors.push(`broker active run ${run.command_id} wp_id does not match request`);
    }
    if (request.role !== run.role) {
      invariantErrors.push(`broker active run ${run.command_id} role does not match request`);
    }
    if (request.command_kind !== run.command_kind) {
      invariantErrors.push(`broker active run ${run.command_id} command_kind does not match request`);
    }
    if (toAbsNormalized(request.output_jsonl_file) !== toAbsNormalized(run.output_jsonl_file)) {
      invariantErrors.push(`broker active run ${run.command_id} output_jsonl_file does not match request`);
    }
    const timeoutAtMs = Date.parse(run.timeout_at || "");
    if (Number.isNaN(timeoutAtMs)) {
      invariantErrors.push(`broker active run ${run.command_id} has invalid timeout_at`);
    } else if (Date.now() > (timeoutAtMs + (SESSION_CONTROL_RUN_STALE_GRACE_SECONDS * 1000))) {
      invariantErrors.push(`broker active run ${run.command_id} is stale past timeout_at`);
    }
  }
}

for (const request of requests) {
  const result = resultById.get(request.command_id);
  const session = sessionByKey.get(request.session_key);
  const brokerRun = brokerRunById.get(request.command_id);
  const outputPath = toAbsNormalized(request.output_jsonl_file);
  const outputExists = fs.existsSync(outputPath);

  if (result) {
    if (!outputExists) {
      invariantErrors.push(`request ${request.command_id} is missing settled output log ${outputPath}`);
    }
    continue;
  }

  const createdAtMs = Date.parse(request.created_at || "");
  const requestAgeMs = Number.isNaN(createdAtMs) ? Number.POSITIVE_INFINITY : (Date.now() - createdAtMs);
  const registryShowsRunning = Boolean(
    session
    && session.last_command_id === request.command_id
    && session.last_command_status === "RUNNING",
  );

  if (!outputExists && !brokerRun && !registryShowsRunning) {
    invariantErrors.push(`request ${request.command_id} is missing output log ${outputPath}`);
  }

  if (!brokerRun && !registryShowsRunning) {
    invariantErrors.push(`request ${request.command_id} has no settled result and no active broker/session state`);
    continue;
  }

  if (requestAgeMs > maxUnsettledAgeMs) {
    invariantErrors.push(`request ${request.command_id} is older than ${SESSION_CONTROL_RUN_TIMEOUT_SECONDS + SESSION_CONTROL_RUN_STALE_GRACE_SECONDS}s without a settled result`);
  }
}

for (const session of registry.sessions || []) {
  if (!session.last_command_id) continue;
  const request = requestById.get(session.last_command_id);
  const result = resultById.get(session.last_command_id);
  const brokerRun = brokerRunById.get(session.last_command_id);

  if (!request) {
    invariantErrors.push(`session ${session.session_key} references missing last_command_id ${session.last_command_id}`);
    continue;
  }

  const lastCommandOutputPath = toAbsNormalized(session.last_command_output_file);
  const lastCommandOutputExists = normalizePath(session.last_command_output_file || "")
    ? fs.existsSync(lastCommandOutputPath)
    : false;

  if (session.last_command_status === "RUNNING") {
    if (normalizePath(session.last_command_output_file || "") && !lastCommandOutputExists) {
      const createdAtMs = Date.parse(request.created_at || "");
      const requestAgeMs = Number.isNaN(createdAtMs) ? Number.POSITIVE_INFINITY : (Date.now() - createdAtMs);
      if (requestAgeMs > SESSION_CONTROL_RUN_STALE_GRACE_SECONDS * 1000) {
        invariantErrors.push(`session ${session.session_key} RUNNING last_command_output_file is missing on disk`);
      }
    }
    if (result) {
      invariantErrors.push(`session ${session.session_key} is RUNNING but result ${result.command_id} is already settled`);
    }
    if (!brokerRun) {
      const createdAtMs = Date.parse(request.created_at || "");
      const requestAgeMs = Number.isNaN(createdAtMs) ? Number.POSITIVE_INFINITY : (Date.now() - createdAtMs);
      if (requestAgeMs > SESSION_CONTROL_RUN_STALE_GRACE_SECONDS * 1000) {
        invariantErrors.push(`session ${session.session_key} is RUNNING without a matching active broker run`);
      }
    }
  }

  if (session.last_command_status === "COMPLETED" || session.last_command_status === "FAILED") {
    if (normalizePath(session.last_command_output_file || "") && !lastCommandOutputExists) {
      invariantErrors.push(`session ${session.session_key} last_command_output_file is missing on disk`);
    }
    if (!result) {
      invariantErrors.push(`session ${session.session_key} last command ${session.last_command_id} has no settled result`);
    } else if (result.status !== session.last_command_status) {
      invariantErrors.push(`session ${session.session_key} last command status disagrees with settled result`);
    }
  }

  if (session.startup_proof_state === "READY" && !session.session_thread_id) {
    invariantErrors.push(`session ${session.session_key} is READY without a session_thread_id`);
  }
}

if (invariantErrors.length > 0) {
  fail("Session control runtime invariants failed", invariantErrors);
}

console.log("session-control-runtime-check ok");
