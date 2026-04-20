#!/usr/bin/env node

import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import {
  appendJsonlLine,
  enqueuePendingSessionControlRequest,
  ensureSessionStateFiles,
  loadSessionControlResults,
  loadSessionRegistry,
  mutateSessionRegistrySync,
  markSessionThreadObserved,
  markSessionCommandResult,
  markSessionCommandRunning,
  peekPendingSessionControlRequest,
  removePendingSessionControlRequest,
  registrySessionSummary,
  writeJsonFile,
} from "../../roles_shared/scripts/session/session-registry-lib.mjs";
import { reclaimOwnedSessionTerminals } from "../../roles_shared/scripts/session/terminal-ownership-lib.mjs";
import {
  buildSessionControlResult,
  classifySessionControlOutcomeState,
  defaultSessionOutputFile,
  ensureBrokerAuthToken,
  runCodexThreadCommand,
  runGovernedRoleCommand,
  validateSessionControlRequestShape,
} from "../../roles_shared/scripts/session/session-control-lib.mjs";
import {
  classifyRecoverableBrokerActiveRun,
  inferRecoverableSessionControlResult,
  settleRecoverableSessionControlResults,
} from "../../roles_shared/scripts/session/session-control-self-settle-lib.mjs";
import { syncWpTokenUsageLedger } from "../../roles_shared/scripts/session/wp-token-usage-lib.mjs";
import { appendWpNotificationCore } from "../../roles_shared/scripts/wp/wp-notification-append.mjs";
import {
  SESSION_ACTIVE_TERMINAL_KIND_NONE,
  SESSION_CONTROL_BROKER_AUTH_MODE,
  SESSION_CONTROL_BROKER_BUILD_ID,
  SESSION_CONTROL_BROKER_STATE_FILE,
  SESSION_CONTROL_HOST_PRIMARY,
  SESSION_CONTROL_PROTOCOL_PRIMARY,
  SESSION_CONTROL_REQUESTS_FILE,
  SESSION_CONTROL_RESULTS_FILE,
  SESSION_CONTROL_RUN_TIMEOUT_SECONDS,
  SESSION_CONTROL_TRANSPORT_PRIMARY,
  isAllowedPrimaryOrFallbackModel,
  isAllowedProfileModel,
  roleModelProfile,
  sessionKey,
} from "../../roles_shared/scripts/session/session-policy.mjs";

const repoRoot = process.cwd();
const brokerStatePath = path.resolve(repoRoot, SESSION_CONTROL_BROKER_STATE_FILE);
const resultsPath = path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE);
const orchestratorSteerNextScriptPath = path.resolve(repoRoot, ".GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs");
const serverHost = "127.0.0.1";
const activeRuns = new Map();
const drainingSessions = new Set();
const socketContexts = new WeakMap();
const brokerAuthToken = ensureBrokerAuthToken(repoRoot);

function nowIso() {
  return new Date().toISOString();
}

function readJson(filePath, fallbackValue) {
  if (!fs.existsSync(filePath)) return fallbackValue;
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  writeJsonFile(filePath, value);
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

function killProcessTree(pid) {
  const numeric = Number(pid || 0);
  if (!Number.isInteger(numeric) || numeric <= 0) return;
  try {
    if (process.platform === "win32") {
      spawnSync("taskkill", ["/PID", String(numeric), "/T", "/F"], { stdio: "ignore" });
      return;
    }
    process.kill(numeric, "SIGTERM");
  } catch {
    // Ignore already-dead child processes during broker cleanup.
  }
}

function currentStateSkeleton(port = 0) {
  return {
    schema_id: "hsk.session_control_broker_state@1",
    schema_version: "session_control_broker_state_v1",
    protocol: SESSION_CONTROL_PROTOCOL_PRIMARY,
    control_transport: SESSION_CONTROL_TRANSPORT_PRIMARY,
    broker_build_id: SESSION_CONTROL_BROKER_BUILD_ID,
    broker_auth_mode: SESSION_CONTROL_BROKER_AUTH_MODE,
    host: serverHost,
    port,
    broker_pid: process.pid,
    started_at: nowIso(),
    updated_at: nowIso(),
    active_runs: [],
  };
}

function projectActiveRuns() {
  return [...activeRuns.values()].map((run) => ({
    command_id: run.request.command_id,
    session_key: run.request.session_key,
    wp_id: run.request.wp_id,
    role: run.request.role,
    command_kind: run.request.command_kind,
    child_pid: run.childPid || 0,
    started_at: run.startedAt,
    timeout_at: run.timeoutAt,
    output_jsonl_file: run.outputFile,
    termination_reason: run.terminationReason || "",
  }));
}

function persistBrokerState(port) {
  const prior = readJson(brokerStatePath, currentStateSkeleton(port));
  const next = {
    ...prior,
    schema_id: "hsk.session_control_broker_state@1",
    schema_version: "session_control_broker_state_v1",
    protocol: SESSION_CONTROL_PROTOCOL_PRIMARY,
    control_transport: SESSION_CONTROL_TRANSPORT_PRIMARY,
    broker_build_id: SESSION_CONTROL_BROKER_BUILD_ID,
    broker_auth_mode: SESSION_CONTROL_BROKER_AUTH_MODE,
    host: serverHost,
    port,
    broker_pid: process.pid,
    updated_at: nowIso(),
    active_runs: projectActiveRuns(),
  };
  writeJson(brokerStatePath, next);
}

function loadBrokerState() {
  return readJson(brokerStatePath, currentStateSkeleton(0));
}

function loadRequestMap() {
  const requestsPath = path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE);
  if (!fs.existsSync(requestsPath)) return new Map();
  const lines = fs.readFileSync(requestsPath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  return new Map(lines.map((request) => [request.command_id, request]));
}

function loadResultMap() {
  const { results } = loadSessionControlResults(repoRoot);
  return new Map(results.map((result) => [result.command_id, result]));
}

function appendResultOnce(result) {
  const results = loadResultMap();
  if (!results.has(result.command_id)) {
    appendJsonlLine(resultsPath, result);
  }
}

function appendOutputEvent(outputFile, event) {
  const outputPath = path.resolve(outputFile);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.appendFileSync(outputPath, `${JSON.stringify({ timestamp: nowIso(), ...event })}\n`, "utf8");
}

function safeSend(socket, value) {
  if (!socket || socket.destroyed || !socket.writable) return;
  try {
    socket.write(`${JSON.stringify(value)}\n`);
  } catch {
    // Ignore client disconnects after the broker has accepted the run.
  }
}

function respond(socket, id, result) {
  safeSend(socket, { jsonrpc: "2.0", id, result });
}

function respondError(socket, id, code, message, data = {}) {
  safeSend(socket, {
    jsonrpc: "2.0",
    id,
    error: {
      code,
      message,
      data,
    },
  });
}

function notify(socket, method, params) {
  safeSend(socket, {
    jsonrpc: "2.0",
    method,
    params,
  });
}

function brokerPort() {
  const address = server?.address?.();
  return typeof address === "object" && address ? address.port || 0 : 0;
}

function findActiveRunBySession(sessionId) {
  return [...activeRuns.values()].find((run) => run.request.session_key === sessionId) || null;
}

function buildRequestRecord(request, validation) {
  return {
    ...request,
    selected_model: validation.selectedModel || request.selected_model || "",
    output_jsonl_file: validation.outputFile.replace(/\\/g, "/"),
    session_thread_id: request.command_kind === "SEND_PROMPT" ? validation.threadId : "",
  };
}

function requestWirePayload(requestRecord, {
  status = "",
  outcomeState = "",
  threadId = "",
  outputJsonlFile = "",
  lastAgentMessage = "",
  error = "",
  durationMs = 0,
  brokerRunId = "",
  blockingRunId = "",
  queueDepth = 0,
  queuedAt = "",
} = {}) {
  return {
    command_id: requestRecord.command_id,
    session_id: requestRecord.session_key,
    thread_id: threadId,
    status: String(status || "").toLowerCase(),
    outcome_state: outcomeState,
    output_jsonl_file: outputJsonlFile,
    last_agent_message: lastAgentMessage,
    error,
    duration_ms: durationMs,
    broker_run_id: brokerRunId,
    blocking_run_id: blockingRunId,
    queue_depth: queueDepth,
    queued_at: queuedAt,
  };
}

function recoverActiveRun(run, repairReason) {
  if (!run) return null;
  const request = run.request;
  const commandId = String(request?.command_id || "").trim();
  if (!commandId) return null;

  if (run.childPid) {
    killProcessTree(run.childPid);
  }
  if (run.timer) clearTimeout(run.timer);
  run.settled = true;
  activeRuns.delete(commandId);
  persistBrokerState(brokerPort());

  const resultMap = loadResultMap();
  const existingResult = resultMap.get(commandId);
  if (existingResult) {
    respond(run.socket, run.rpcId, {
      command_id: existingResult.command_id,
      session_id: existingResult.session_key,
      thread_id: existingResult.thread_id || "",
      status: String(existingResult.status || "").toLowerCase(),
      outcome_state: existingResult.outcome_state || classifySessionControlOutcomeState({
        status: existingResult.status,
        commandKind: request.command_kind,
        error: existingResult.error,
        summary: existingResult.summary,
        cancelStatus: existingResult.cancel_status,
      }),
      output_jsonl_file: existingResult.output_jsonl_file || defaultSessionOutputFile(repoRoot, request.session_key, commandId),
      last_agent_message: existingResult.last_agent_message || "",
      error: existingResult.error || "",
      duration_ms: existingResult.duration_ms || 0,
      broker_run_id: commandId,
    });
    void drainQueuedRequestsForSession(request.session_key);
    return existingResult;
  }

  const { registry } = loadSessionRegistry(repoRoot);
  const session = registry.sessions.find((entry) => entry.session_key === request.session_key) || null;
  const inferred = inferRecoverableSessionControlResult({
    repoRoot,
    request,
    session,
    resultById: resultMap,
  });
  const outputFile = inferred.outputJsonlFile || defaultSessionOutputFile(repoRoot, request.session_key, commandId);
  appendOutputEvent(outputFile, {
    type: "broker.repair",
    reason: repairReason || inferred.repairReason || "stale_active_run_recovered",
  });
  const result = buildSessionControlResult({
    commandId,
    commandKind: request.command_kind,
    sessionKey: request.session_key,
    wpId: request.wp_id,
    role: request.role,
    status: inferred.status,
    threadId: inferred.threadId,
    summary: inferred.summary,
    outputJsonlFile: outputFile,
    lastAgentMessage: "",
    error: inferred.error,
    durationMs: 0,
    targetCommandId: inferred.targetCommandId,
    cancelStatus: inferred.cancelStatus,
    governedAction: request.governed_action,
  });

  const persisted = mutateSessionRegistrySync(repoRoot, (liveRegistry) => {
    const liveSession = (liveRegistry.sessions || []).find((entry) => entry.session_key === request.session_key) || null;
    if (!liveSession) return false;
    ensureResultPersisted(request, liveSession, result);
    return true;
  });
  if (!persisted) {
    appendResultOnce(result);
  }
  syncWpTokenUsageLedger(repoRoot, result, { session: session || undefined });

  respond(run.socket, run.rpcId, {
    command_id: request.command_id,
    session_id: request.session_key,
    thread_id: result.thread_id || "",
    status: String(result.status || "").toLowerCase(),
    outcome_state: result.outcome_state || classifySessionControlOutcomeState({
      status: result.status,
      commandKind: request.command_kind,
      error: result.error,
      summary: result.summary,
      cancelStatus: result.cancel_status,
    }),
    output_jsonl_file: outputFile,
    last_agent_message: result.last_agent_message || "",
    error: result.error || "",
    duration_ms: result.duration_ms,
    broker_run_id: request.command_id,
  });
  void drainQueuedRequestsForSession(request.session_key);
  return result;
}

function maybeAutoContinueGovernedRoute(request) {
  const role = String(request?.role || "").trim().toUpperCase();
  if (!["ACTIVATION_MANAGER", "CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(role)) return;
  if (String(request?.command_kind || "").trim().toUpperCase() !== "SEND_PROMPT") return;
  const wpId = String(request?.wp_id || "").trim();
  if (!wpId) return;

  try {
    const packetPath = path.resolve(repoRoot, ".GOV", "task_packets", wpId, "packet.md");
    if (!fs.existsSync(packetPath)) return;
    const packetText = fs.readFileSync(packetPath, "utf8");
    if (!/^\s*-\s*(?:\*\*)?WORKFLOW_LANE(?:\*\*)?\s*:\s*ORCHESTRATOR_MANAGED\s*$/mi.test(packetText)) return;

    const runtimeStatusPathMatch = packetText.match(/^\s*-\s*(?:\*\*)?WP_RUNTIME_STATUS_FILE(?:\*\*)?\s*:\s*(.+)\s*$/mi);
    if (!runtimeStatusPathMatch) return;
    const runtimeStatusPath = path.resolve(repoRoot, String(runtimeStatusPathMatch[1] || "").trim());
    if (!fs.existsSync(runtimeStatusPath)) return;

    const runtimeStatus = JSON.parse(fs.readFileSync(runtimeStatusPath, "utf8"));
    const nextActor = String(runtimeStatus?.next_expected_actor || "").trim().toUpperCase();
    const validatorTrigger = String(runtimeStatus?.validator_trigger || "").trim().toUpperCase();
    if (!["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR", "ACTIVATION_MANAGER"].includes(nextActor)) return;

    const overlapForwardRoute = validatorTrigger === "MICROTASK_REVIEW_READY";
    if (!overlapForwardRoute) return;

    execFileSync(process.execPath, [orchestratorSteerNextScriptPath, wpId, "PRIMARY"], {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
  } catch {
    // Best-effort: completion auto-continue is opportunistic and must not break request settlement.
  }
}

function ensureResultPersisted(request, session, result) {
  appendResultOnce(result);
  session.active_host = SESSION_CONTROL_HOST_PRIMARY;
  session.active_terminal_kind = SESSION_ACTIVE_TERMINAL_KIND_NONE;
  markSessionCommandResult(session, result);

  // RGF-93: Mechanical completion notification.
  // Fire-and-forget: inject a notification into WP communications so the orchestrator
  // does not need to poll for session results. The notification targets the ORCHESTRATOR
  // so it can resume lifecycle progression without sleeping and reading output files.
  const status = String(result.status || "").toUpperCase();
  if ((status === "COMPLETED" || status === "FAILED") && request.wp_id && request.command_kind !== "CANCEL_SESSION") {
    try {
      appendWpNotificationCore({
        wpId: request.wp_id,
        sourceKind: "SESSION_COMPLETION",
        sourceRole: String(request.role || "BROKER").toUpperCase(),
        sourceSession: request.session_key || "",
        targetRole: "ORCHESTRATOR",
        correlationId: request.command_id || "",
        summary: `${request.role || "ROLE"} session ${status.toLowerCase()} for ${request.command_kind || "command"}: ${String(result.summary || result.last_agent_message || "").slice(0, 200)}`,
      });
    } catch {
      // Non-fatal: notification is a convenience, not a hard requirement.
      // The orchestrator can still poll or check session-registry-status as fallback.
    }

    // RGF-95: Auto-reclaim the terminal window for this session.
    // Only reclaim terminals owned by this specific session — never touch other apps or processes.
    try {
      const results = reclaimOwnedSessionTerminals(repoRoot, { sessionKey: request.session_key });
      for (const r of results) {
        if (r.reclaim_status === "RECLAIMED") {
          console.log(`[BROKER] Auto-reclaimed terminal for ${r.session_key} (pid ${r.process_id})`);
        }
      }
    } catch {
      // Non-fatal: terminal cleanup is best-effort.
    }

    if (status === "COMPLETED") {
      maybeAutoContinueGovernedRoute(request);
    }
  }
}

function settleRejectedRequest(request, reason) {
  const outputFile = defaultSessionOutputFile(repoRoot, request.session_key, request.command_id);
  const { registry } = loadSessionRegistry(repoRoot);
  const currentSession = (registry.sessions || []).find((entry) => entry.session_key === request.session_key) || null;
  const requestsPath = path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE);
  const requestMap = loadRequestMap();
  if (!requestMap.has(request.command_id)) {
    appendJsonlLine(requestsPath, {
      ...request,
      output_jsonl_file: outputFile.replace(/\\/g, "/"),
    });
  }
  appendOutputEvent(outputFile, { type: "broker.rejected", reason });
  const result = buildSessionControlResult({
    commandId: request.command_id,
    commandKind: request.command_kind,
    sessionKey: request.session_key,
    wpId: request.wp_id,
    role: request.role,
    status: "FAILED",
    threadId: currentSession?.session_thread_id || "",
    summary: reason,
    outputJsonlFile: outputFile,
    lastAgentMessage: "",
    error: reason,
    durationMs: 0,
    targetCommandId: request.target_command_id || "",
    cancelStatus: request.command_kind === "CANCEL_SESSION" ? "rejected" : "",
    governedAction: request.governed_action,
  });
  const persisted = mutateSessionRegistrySync(repoRoot, (registry) => {
    const session = registry.sessions.find((entry) => entry.session_key === request.session_key);
    if (!session) return false;
    if (request.command_kind === "CANCEL_SESSION") {
      session.last_event_at = result.processed_at;
      appendResultOnce(result);
    } else {
      ensureResultPersisted(request, session, result);
    }
    return true;
  });
  if (!persisted) {
    appendResultOnce(result);
  }
  return result;
}

function reconcileOrphanedRuns() {
  const prior = loadBrokerState();
  if (!prior?.active_runs?.length) return;
  if (prior.broker_pid && isProcessAlive(prior.broker_pid)) return;

  const requests = loadRequestMap();
  const existingResults = loadResultMap();

  for (const run of prior.active_runs) {
    if (run.child_pid) killProcessTree(run.child_pid);
    if (existingResults.has(run.command_id)) continue;
    const request = requests.get(run.command_id);
    if (!request) continue;
    const { registry } = loadSessionRegistry(repoRoot);
    const session = registry.sessions.find((entry) => entry.session_key === request.session_key) || null;
    const outputFile = defaultSessionOutputFile(repoRoot, request.session_key, request.command_id);
    appendOutputEvent(outputFile, {
      type: "broker.repair",
      reason: "Recovered abandoned governed run after prior broker exit.",
    });
    const result = buildSessionControlResult({
      commandId: request.command_id,
      commandKind: request.command_kind,
      sessionKey: request.session_key,
      wpId: request.wp_id,
      role: request.role,
      status: "FAILED",
      threadId: session?.session_thread_id || "",
      summary: "Recovered abandoned governed run after prior broker exit.",
      outputJsonlFile: outputFile,
      lastAgentMessage: "",
      error: "Handshake ACP broker restarted while the governed run was active.",
      durationMs: 0,
      governedAction: request.governed_action,
    });
    const persisted = mutateSessionRegistrySync(repoRoot, (registry) => {
      const session = registry.sessions.find((entry) => entry.session_key === request.session_key);
      if (!session) return false;
      ensureResultPersisted(request, session, result);
      return true;
    });
    if (!persisted) {
      appendJsonlLine(resultsPath, result);
    }
  }
}

function reconcileRecoverableRequests() {
  settleRecoverableSessionControlResults(repoRoot, {
    brokerState: loadBrokerState(),
  });
}

function validateGovernedRequest(request, expectedCommandKind) {
  const errors = validateSessionControlRequestShape(request);
  if (errors.length > 0) return { ok: false, reason: errors.join("; ") };
  if (request.command_kind !== expectedCommandKind) {
    return { ok: false, reason: `command_kind must be ${expectedCommandKind}` };
  }

  const existingResults = loadResultMap();
  if (existingResults.has(request.command_id)) {
    return { ok: true, existingResult: existingResults.get(request.command_id) };
  }

  const { registry } = loadSessionRegistry(repoRoot);
  const session = registry.sessions.find((entry) => entry.session_key === request.session_key);
  if (!session) {
    return { ok: false, reason: `Governed session ${request.session_key} is not registered` };
  }
  if (session.role !== request.role || session.wp_id !== request.wp_id) {
    return { ok: false, reason: `Governed session identity mismatch for ${request.session_key}` };
  }
  if ((session.local_branch || "") !== (request.local_branch || "")) {
    return { ok: false, reason: `local_branch drift for ${request.session_key}` };
  }
  if ((session.local_worktree_dir || "") !== (request.local_worktree_dir || "")) {
    return { ok: false, reason: `local_worktree_dir drift for ${request.session_key}` };
  }
  const absWorktreeDir = path.resolve(repoRoot, session.local_worktree_dir);
  if (expectedCommandKind !== "CLOSE_SESSION" && !fs.existsSync(absWorktreeDir)) {
    return { ok: false, reason: `Assigned worktree missing for ${request.session_key}` };
  }
  const expectedOutputFile = defaultSessionOutputFile(repoRoot, session.session_key, request.command_id);
  const existingRequest = loadRequestMap().get(request.command_id);
  if (existingRequest && existingRequest.session_key !== request.session_key) {
    return { ok: false, reason: `command_id ${request.command_id} is already bound to another session` };
  }
  if (expectedCommandKind !== "CANCEL_SESSION") {
    const selectedModel = session.requested_model || request.selected_model;
    const selectedProfileId = session.requested_profile_id || request.selected_profile_id || "";
    const selectedProfile = selectedProfileId ? roleModelProfile(selectedProfileId) : null;
    if (selectedProfile && selectedProfile.runtime_support === "GOVERNED_LAUNCH_SUPPORTED") {
      if (String(selectedModel || "").toLowerCase() !== String(selectedProfile.launch_model || "").toLowerCase()) {
        return { ok: false, reason: `Selected model ${selectedModel} does not match profile ${selectedProfileId} launch_model ${selectedProfile.launch_model}` };
      }
    } else if (!isAllowedPrimaryOrFallbackModel(selectedModel) && !isAllowedProfileModel(selectedModel)) {
      return { ok: false, reason: `Selected model is not repo-governed for ${request.session_key}` };
    }
    if (request.command_kind === "SEND_PROMPT" && !session.session_thread_id) {
      return { ok: false, reason: `No steerable thread id is registered for ${request.session_key}` };
    }
    return {
      ok: true,
      registry,
      session,
      absWorktreeDir,
      selectedModel,
      selectedProfile,
      outputFile: expectedOutputFile,
      threadId: session.session_thread_id || "",
    };
  }
  return {
    ok: true,
    registry,
    session,
    absWorktreeDir,
    outputFile: expectedOutputFile,
    threadId: session.session_thread_id || "",
  };
}

async function launchGovernedRequestRun({
  socket = null,
  rpcId = null,
  requestRecord,
  validation,
  queuedLaunch = false,
} = {}) {
  const { absWorktreeDir, selectedModel, selectedProfile, outputFile, threadId } = validation;
  const requestMap = loadRequestMap();
  if (!requestMap.has(requestRecord.command_id)) {
    appendOutputEvent(outputFile, {
      type: "control.requested",
      session_id: requestRecord.session_key,
      command_id: requestRecord.command_id,
      command_kind: requestRecord.command_kind,
    });
    appendJsonlLine(path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE), requestRecord);
  }
  if (queuedLaunch) {
    appendOutputEvent(outputFile, {
      type: "control.queue.dequeued",
      session_id: requestRecord.session_key,
      command_id: requestRecord.command_id,
      command_kind: requestRecord.command_kind,
    });
  }

  mutateSessionRegistrySync(repoRoot, (registry) => {
    const session = registry.sessions.find((entry) => entry.session_key === requestRecord.session_key);
    if (!session) {
      throw new Error(`Governed session ${requestRecord.session_key} is not registered`);
    }
    markSessionCommandRunning(session, requestRecord);
    session.active_host = SESSION_CONTROL_HOST_PRIMARY;
    session.active_terminal_kind = SESSION_ACTIVE_TERMINAL_KIND_NONE;
  });

  const runState = {
    socket,
    rpcId,
    request: requestRecord,
    outputFile,
    startedAt: nowIso(),
    timeoutAt: new Date(Date.now() + (SESSION_CONTROL_RUN_TIMEOUT_SECONDS * 1000)).toISOString(),
    child: null,
    childPid: 0,
    terminationReason: "",
    cancellationRequested: false,
    settled: false,
  };
  activeRuns.set(requestRecord.command_id, runState);
  persistBrokerState(brokerPort());

  if (socket && rpcId !== null) {
    notify(socket, "session/update", {
      session_id: requestRecord.session_key,
      stage: "run.started",
      command_id: requestRecord.command_id,
      timestamp: runState.startedAt,
    });
  }

  const settle = (execution) => {
    if (runState.settled) return;
    runState.settled = true;
    if (runState.timer) clearTimeout(runState.timer);
    activeRuns.delete(requestRecord.command_id);
    persistBrokerState(brokerPort());

    const errorMessage = runState.terminationReason || execution.stderr || "";
    const summary = execution.lastAgentMessage || (errorMessage ? errorMessage : requestRecord.summary);
    const result = mutateSessionRegistrySync(repoRoot, (registry) => {
      const latestSession = registry.sessions.find((entry) => entry.session_key === requestRecord.session_key);
      if (!latestSession) {
        throw new Error(`Governed session ${requestRecord.session_key} is not registered`);
      }
      const result = buildSessionControlResult({
        commandId: requestRecord.command_id,
        commandKind: requestRecord.command_kind,
        sessionKey: requestRecord.session_key,
        wpId: requestRecord.wp_id,
        role: requestRecord.role,
        status: execution.ok && !runState.terminationReason ? "COMPLETED" : "FAILED",
        threadId: execution.threadId || latestSession.session_thread_id || "",
        summary,
        outputJsonlFile: outputFile,
        lastAgentMessage: execution.lastAgentMessage || "",
        error: execution.ok && !runState.terminationReason ? "" : (errorMessage || `governed role command exited with ${execution.exitCode}`),
        durationMs: execution.durationMs,
        targetCommandId: requestRecord.target_command_id || "",
        cancelStatus: "",
        governedAction: requestRecord.governed_action,
      });

      ensureResultPersisted(requestRecord, latestSession, result);
      return result;
    });

    if (socket && rpcId !== null) {
      respond(socket, rpcId, requestWirePayload(requestRecord, {
        status: result.status,
        outcomeState: result.outcome_state || classifySessionControlOutcomeState({
          status: result.status,
          commandKind: requestRecord.command_kind,
          error: result.error,
          summary: result.summary,
          cancelStatus: result.cancel_status,
        }),
        threadId: result.thread_id || "",
        outputJsonlFile: outputFile,
        lastAgentMessage: result.last_agent_message || "",
        error: result.error || "",
        durationMs: result.duration_ms,
        brokerRunId: requestRecord.command_id,
      }));
    }

    try {
      const reclaimResults = reclaimOwnedSessionTerminals(repoRoot, { sessionKey: requestRecord.session_key });
      if (socket && rpcId !== null) {
        for (const reclaim of reclaimResults) {
          notify(socket, "session/update", {
            session_id: requestRecord.session_key,
            stage: "terminal.reclaimed",
            command_id: requestRecord.command_id,
            reclaim_status: reclaim.reclaim_status,
            process_id: reclaim.process_id,
            terminal_batch_id: reclaim.terminal_batch_id,
            timestamp: nowIso(),
          });
        }
      }
    } catch {
      // Reclaim is best-effort and must not block request settlement.
    }

    void drainQueuedRequestsForSession(requestRecord.session_key);
  };

  runState.timer = setTimeout(() => {
    runState.terminationReason = `Handshake ACP broker timed out after ${SESSION_CONTROL_RUN_TIMEOUT_SECONDS}s`;
    if (runState.childPid) killProcessTree(runState.childPid);
  }, SESSION_CONTROL_RUN_TIMEOUT_SECONDS * 1000);

  const runFn = selectedProfile ? runGovernedRoleCommand : runCodexThreadCommand;
  void runFn({
    profile: selectedProfile || null,
    absWorktreeDir,
    selectedModel,
    prompt: requestRecord.prompt,
    outputFile,
    threadId: requestRecord.command_kind === "SEND_PROMPT" ? threadId : "",
    environmentOverrides: requestRecord.environment_overrides || {},
    onSpawn: (child) => {
      runState.child = child;
      runState.childPid = child.pid || 0;
      persistBrokerState(brokerPort());
      if (runState.terminationReason && runState.childPid) {
        killProcessTree(runState.childPid);
      }
      if (socket && rpcId !== null) {
        notify(socket, "session/update", {
          session_id: requestRecord.session_key,
          stage: "process.spawned",
          command_id: requestRecord.command_id,
          pid: runState.childPid,
          timestamp: nowIso(),
        });
      }
    },
    onEvent: (event) => {
      const observedThreadId = event.thread_id || event.session_id || "";
      if ((event.type === "thread.started" || event.type === "result") && observedThreadId) {
        mutateSessionRegistrySync(repoRoot, (registry) => {
          const liveSession = registry.sessions.find((entry) => entry.session_key === requestRecord.session_key);
          if (!liveSession) return;
          markSessionThreadObserved(
            liveSession,
            observedThreadId,
            event.timestamp || nowIso(),
          );
        });
      }
      if (socket && rpcId !== null) {
        notify(socket, "session/update", {
          session_id: requestRecord.session_key,
          command_id: requestRecord.command_id,
          stage: event.type || "event",
          thread_id: observedThreadId,
          timestamp: nowIso(),
        });
      }
    },
  }).then(settle).catch((error) => {
    appendOutputEvent(outputFile, {
      type: "broker.error",
      message: error.message || "Handshake ACP broker run failed",
    });
    settle({
      ok: false,
      exitCode: 1,
      threadId: threadId || "",
      lastAgentMessage: "",
      stderr: error.message || "Handshake ACP broker run failed",
      durationMs: 0,
    });
  });
}

function queueGovernedRequestOnBusy(socket, rpcId, requestRecord, blockingRun) {
  const outputFile = requestRecord.output_jsonl_file;
  const queuedAt = nowIso();
  const requestMap = loadRequestMap();
  if (!requestMap.has(requestRecord.command_id)) {
    appendOutputEvent(outputFile, {
      type: "control.requested",
      session_id: requestRecord.session_key,
      command_id: requestRecord.command_id,
      command_kind: requestRecord.command_kind,
    });
    appendJsonlLine(path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE), requestRecord);
  }
  appendOutputEvent(outputFile, {
    type: "control.busy_queued",
    session_id: requestRecord.session_key,
    command_id: requestRecord.command_id,
    command_kind: requestRecord.command_kind,
    blocking_command_id: blockingRun?.request?.command_id || "",
  });

  let queueDepth = 0;
  mutateSessionRegistrySync(repoRoot, (registry) => {
    const session = registry.sessions.find((entry) => entry.session_key === requestRecord.session_key);
    if (!session) {
      throw new Error(`Governed session ${requestRecord.session_key} is not registered`);
    }
    enqueuePendingSessionControlRequest(session, requestRecord, {
      queueReasonCode: "BUSY_ACTIVE_RUN",
      blockingCommandId: blockingRun?.request?.command_id || "",
      queuedAt,
    });
    queueDepth = Array.isArray(session.pending_control_queue) ? session.pending_control_queue.length : 0;
  });
  persistBrokerState(brokerPort());

  notify(socket, "session/update", {
    session_id: requestRecord.session_key,
    stage: "run.queued",
    command_id: requestRecord.command_id,
    blocking_command_id: blockingRun?.request?.command_id || "",
    queue_depth: queueDepth,
    timestamp: queuedAt,
  });
  respond(socket, rpcId, requestWirePayload(requestRecord, {
    status: "QUEUED",
    outcomeState: "ACCEPTED_PENDING",
    threadId: requestRecord.session_thread_id || "",
    outputJsonlFile: outputFile,
    lastAgentMessage: "",
    error: "",
    durationMs: 0,
    brokerRunId: blockingRun?.request?.command_id || "",
    blockingRunId: blockingRun?.request?.command_id || "",
    queueDepth,
    queuedAt,
  }));
}

async function drainQueuedRequestsForSession(sessionKeyValue = "") {
  const normalizedSessionKey = String(sessionKeyValue || "").trim();
  if (!normalizedSessionKey || drainingSessions.has(normalizedSessionKey)) return;
  drainingSessions.add(normalizedSessionKey);
  try {
    while (!findActiveRunBySession(normalizedSessionKey)) {
      const { registry } = loadSessionRegistry(repoRoot);
      const session = (registry.sessions || []).find((entry) => entry.session_key === normalizedSessionKey) || null;
      const queued = session ? peekPendingSessionControlRequest(session) : null;
      if (!session || !queued) break;

      const request = loadRequestMap().get(queued.command_id);
      if (!request) {
        mutateSessionRegistrySync(repoRoot, (liveRegistry) => {
          const liveSession = (liveRegistry.sessions || []).find((entry) => entry.session_key === normalizedSessionKey);
          if (!liveSession) return;
          removePendingSessionControlRequest(liveSession, queued.command_id);
        });
        continue;
      }
      if (loadResultMap().has(queued.command_id)) {
        mutateSessionRegistrySync(repoRoot, (liveRegistry) => {
          const liveSession = (liveRegistry.sessions || []).find((entry) => entry.session_key === normalizedSessionKey);
          if (!liveSession) return;
          removePendingSessionControlRequest(liveSession, queued.command_id);
        });
        continue;
      }

      const validation = validateGovernedRequest(request, request.command_kind);
      if (validation.existingResult) {
        mutateSessionRegistrySync(repoRoot, (liveRegistry) => {
          const liveSession = (liveRegistry.sessions || []).find((entry) => entry.session_key === normalizedSessionKey);
          if (!liveSession) return;
          removePendingSessionControlRequest(liveSession, queued.command_id);
        });
        continue;
      }
      if (!validation.ok) {
        mutateSessionRegistrySync(repoRoot, (liveRegistry) => {
          const liveSession = (liveRegistry.sessions || []).find((entry) => entry.session_key === normalizedSessionKey);
          if (!liveSession) return;
          removePendingSessionControlRequest(liveSession, queued.command_id);
        });
        settleRejectedRequest(request, `Queued request ${queued.command_id} could not be resumed: ${validation.reason}`);
        continue;
      }

      await launchGovernedRequestRun({
        requestRecord: buildRequestRecord(request, validation),
        validation,
        queuedLaunch: true,
      });
      break;
    }
  } finally {
    drainingSessions.delete(normalizedSessionKey);
  }
}

function handleSessionClose(socket, id, params = {}) {
  const request = params.request || params;
  const validation = validateGovernedRequest(request, "CLOSE_SESSION");
  if (validation.existingResult) {
    respond(socket, id, validation.existingResult);
    return;
  }
  if (!validation.ok) {
    const result = settleRejectedRequest(request, validation.reason);
    respond(socket, id, result);
    return;
  }

  const concurrent = findActiveRunBySession(request.session_key);
  if (concurrent) {
    const result = settleRejectedRequest(
      request,
      `Cannot close ${request.session_key} while governed run ${concurrent.request.command_id} is active.`,
    );
    respond(socket, id, result);
    return;
  }
  if (Array.isArray(validation.session?.pending_control_queue) && validation.session.pending_control_queue.length > 0) {
    const queuedHead = peekPendingSessionControlRequest(validation.session);
    const result = settleRejectedRequest(
      request,
      `Cannot close ${request.session_key} while queued governed prompt ${queuedHead?.command_id || "<unknown>"} is pending.`,
    );
    respond(socket, id, result);
    return;
  }

  const { outputFile } = validation;
  const requestRecord = {
    ...request,
    output_jsonl_file: outputFile.replace(/\\/g, "/"),
  };
  const requestMap = loadRequestMap();
  if (!requestMap.has(requestRecord.command_id)) {
    appendOutputEvent(outputFile, {
      type: "control.close.requested",
      session_id: requestRecord.session_key,
      command_id: requestRecord.command_id,
    });
    appendJsonlLine(path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE), requestRecord);
  }

  const { priorThreadId } = mutateSessionRegistrySync(repoRoot, (registry) => {
    const session = registry.sessions.find((entry) => entry.session_key === requestRecord.session_key);
    if (!session) {
      throw new Error(`Governed session ${requestRecord.session_key} is not registered`);
    }
    const priorThreadId = session.session_thread_id || "";
    markSessionCommandRunning(session, requestRecord);
    session.active_host = SESSION_CONTROL_HOST_PRIMARY;
    session.active_terminal_kind = SESSION_ACTIVE_TERMINAL_KIND_NONE;

    const result = buildSessionControlResult({
      commandId: requestRecord.command_id,
      commandKind: requestRecord.command_kind,
      sessionKey: requestRecord.session_key,
      wpId: requestRecord.wp_id,
      role: requestRecord.role,
      status: "COMPLETED",
      threadId: "",
      summary: priorThreadId
        ? `Governed session closed and steerable thread ${priorThreadId} was cleared.`
        : "Governed session closed; no steerable thread was registered.",
      outputJsonlFile: outputFile,
      lastAgentMessage: "",
      error: "",
      durationMs: 0,
      governedAction: requestRecord.governed_action,
    });

    ensureResultPersisted(requestRecord, session, result);
    return { priorThreadId, result };
  });

  respond(socket, id, {
    command_id: requestRecord.command_id,
    session_id: requestRecord.session_key,
    status: "completed",
    outcome_state: result.outcome_state || classifySessionControlOutcomeState({
      status: result.status,
      commandKind: requestRecord.command_kind,
      error: result.error,
      summary: result.summary,
      cancelStatus: result.cancel_status,
    }),
    output_jsonl_file: outputFile,
    closed_thread_id: priorThreadId,
  });
}

async function runGovernedRequest(socket, id, request, expectedCommandKind) {
  const validation = validateGovernedRequest(request, expectedCommandKind);
  if (validation.existingResult) {
    respond(socket, id, validation.existingResult);
    return;
  }
  if (!validation.ok) {
    const result = settleRejectedRequest(request, validation.reason);
    respond(socket, id, result);
    return;
  }

  const concurrent = findActiveRunBySession(request.session_key);
  if (concurrent) {
    const recovery = classifyRecoverableBrokerActiveRun({
      run: concurrent,
      resultById: loadResultMap(),
      isChildProcessAlive: Number(concurrent.childPid || 0) > 0 ? isProcessAlive(concurrent.childPid) : true,
    });
    if (recovery.recoverable) {
      recoverActiveRun(concurrent, recovery.reason);
    }
  }

  const remainingConcurrent = findActiveRunBySession(request.session_key);
  if (remainingConcurrent) {
    const requestRecord = buildRequestRecord(request, validation);
    if (
      requestRecord.command_kind === "SEND_PROMPT"
      && String(requestRecord.busy_ingress_mode || "").trim().toUpperCase() === "ENQUEUE_ON_BUSY"
    ) {
      queueGovernedRequestOnBusy(socket, id, requestRecord, remainingConcurrent);
      return;
    }
    const result = settleRejectedRequest(
      requestRecord,
      `Concurrent governed run already active for ${request.session_key} (${remainingConcurrent.request.command_id})`,
    );
    respond(socket, id, result);
    return;
  }
  await launchGovernedRequestRun({
    socket,
    rpcId: id,
    requestRecord: buildRequestRecord(request, validation),
    validation,
  });
}

function handleSessionLoad(socket, id, params = {}) {
  const { registry } = loadSessionRegistry(repoRoot);
  const sessionId = params.session_id || params.session_key || sessionKey(params.role, params.wp_id);
  const session = registry.sessions.find((entry) =>
    entry.session_key === sessionId || entry.session_id === sessionId,
  );
  if (!session) {
    respondError(socket, id, -32004, "Governed session not found", { session_id: sessionId });
    return;
  }
  const summary = registrySessionSummary(session);
  respond(socket, id, {
    session_id: summary.session_key,
    session_key: summary.session_key,
    role: summary.role,
    wp_id: summary.wp_id,
    thread_id: summary.session_thread_id || "",
    requested_model: session.requested_model || "",
    control_protocol: summary.control_protocol || "",
    control_transport: summary.control_transport || "",
    active_host: summary.active_host || "NONE",
    active_terminal_kind: summary.active_terminal_kind || "NONE",
    runtime_state: summary.runtime_state || "",
    last_command_kind: summary.last_command_kind || "",
    last_command_status: summary.last_command_status || "",
    last_command_summary: summary.last_command_summary || "",
    last_command_output_file: summary.last_command_output_file || "",
    pending_control_queue_count: summary.pending_control_queue_count || 0,
    next_queued_control_request: summary.next_queued_control_request || null,
  });
}

function handleSessionCancel(socket, id, params = {}) {
  const request = params.request || null;
  let requestRecord = null;
  let requestSession = null;
  let requestRegistry = null;
  let cancelOutputFile = "";

  if (request) {
    const validation = validateGovernedRequest(request, "CANCEL_SESSION");
    if (validation.existingResult) {
      respond(socket, id, validation.existingResult);
      return;
    }
    if (!validation.ok) {
      const result = settleRejectedRequest(request, validation.reason);
      respond(socket, id, result);
      return;
    }

    requestRegistry = validation.registry;
    requestSession = validation.session;
    cancelOutputFile = validation.outputFile;
    requestRecord = {
      ...request,
      output_jsonl_file: cancelOutputFile.replace(/\\/g, "/"),
    };

    const requestMap = loadRequestMap();
    if (!requestMap.has(requestRecord.command_id)) {
      appendOutputEvent(cancelOutputFile, {
        type: "control.cancel.command_requested",
        session_id: requestRecord.session_key,
        command_id: requestRecord.command_id,
        target_command_id: requestRecord.target_command_id || "",
      });
      appendJsonlLine(path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE), requestRecord);
    }
  }

  const runId = String(params.run_id || requestRecord?.target_command_id || "").trim();
  const sessionId = String(params.session_id || params.session_key || requestRecord?.session_key || "").trim();
  const run = runId
    ? activeRuns.get(runId)
    : (sessionId ? findActiveRunBySession(sessionId) : null);
  if (!run) {
    if (requestRecord && requestSession && requestRegistry) {
      const result = buildSessionControlResult({
        commandId: requestRecord.command_id,
        commandKind: requestRecord.command_kind,
        sessionKey: requestRecord.session_key,
        wpId: requestRecord.wp_id,
        role: requestRecord.role,
        status: "COMPLETED",
        threadId: requestSession.session_thread_id || "",
        summary: "No active governed run matched the cancel request.",
        outputJsonlFile: cancelOutputFile,
        lastAgentMessage: "",
        error: "",
        durationMs: 0,
        targetCommandId: requestRecord.target_command_id || runId || "",
        cancelStatus: "not_running",
        governedAction: requestRecord.governed_action,
      });
      mutateSessionRegistrySync(repoRoot, (registry) => {
        const session = registry.sessions.find((entry) => entry.session_key === requestRecord.session_key);
        if (session) session.last_event_at = result.processed_at;
        appendResultOnce(result);
      });
      respond(socket, id, {
        command_id: result.command_id,
        status: "not_running",
        outcome_state: result.outcome_state || "SETTLED",
        run_id: runId || "",
        session_id: sessionId || "",
        output_jsonl_file: cancelOutputFile,
      });
      return;
    }
    respond(socket, id, {
      status: "not_running",
      run_id: runId || "",
      session_id: sessionId || "",
    });
    return;
  }

  run.cancellationRequested = true;
  run.terminationReason = "Canceled by Handshake ACP request.";
  appendOutputEvent(run.outputFile, {
    type: "control.cancel_requested",
    run_id: run.request.command_id,
    session_id: run.request.session_key,
  });
  notify(run.socket, "session/update", {
    session_id: run.request.session_key,
    stage: "control.cancel_requested",
    command_id: run.request.command_id,
    timestamp: nowIso(),
  });
  if (run.childPid) {
    killProcessTree(run.childPid);
  }

  if (requestRecord && requestSession && requestRegistry) {
    appendOutputEvent(cancelOutputFile, {
      type: "control.cancel_requested",
      run_id: run.request.command_id,
      session_id: run.request.session_key,
      target_command_id: requestRecord.target_command_id || run.request.command_id,
    });
    const result = buildSessionControlResult({
      commandId: requestRecord.command_id,
      commandKind: requestRecord.command_kind,
      sessionKey: requestRecord.session_key,
      wpId: requestRecord.wp_id,
      role: requestRecord.role,
      status: "COMPLETED",
      threadId: requestSession.session_thread_id || "",
      summary: `Cancel requested for governed run ${run.request.command_id}.`,
      outputJsonlFile: cancelOutputFile,
      lastAgentMessage: "",
      error: "",
      durationMs: 0,
      targetCommandId: run.request.command_id,
      cancelStatus: "cancellation_requested",
      governedAction: requestRecord.governed_action,
    });
    mutateSessionRegistrySync(repoRoot, (registry) => {
      const session = registry.sessions.find((entry) => entry.session_key === requestRecord.session_key);
      if (session) session.last_event_at = result.processed_at;
      appendResultOnce(result);
    });
  }

  respond(socket, id, {
    command_id: requestRecord?.command_id || "",
    status: "cancellation_requested",
    outcome_state: "SETTLED",
    run_id: run.request.command_id,
    session_id: run.request.session_key,
    output_jsonl_file: cancelOutputFile,
  });
}

function handleBrokerShutdown(socket, id, params = {}) {
  const force = Boolean(params.force);
  if (!force && activeRuns.size > 0) {
    respondError(socket, id, -32010, "Cannot shut down broker while governed runs are active", {
      active_run_count: activeRuns.size,
    });
    return;
  }
  respond(socket, id, {
    status: "shutdown_requested",
    broker_build_id: SESSION_CONTROL_BROKER_BUILD_ID,
  });
  setTimeout(() => {
    try {
      server.close(() => process.exit(0));
    } catch {
      process.exit(0);
    }
  }, 100);
}

function handleRequest(socket, message) {
  const { id, method, params } = message;
  if (typeof id === "undefined") return;

  if (method === "initialize") {
    const authToken = String(params?.auth_token || "").trim();
    const authorityRole = String(params?.authority_role || "").trim().toUpperCase();
    const authorityBranch = String(params?.authority_branch || "").trim();
    const expectedBuildId = String(params?.expected_broker_build_id || "").trim();
    const expectedAuthMode = String(params?.expected_auth_mode || "").trim();
    if (!authToken || authToken !== brokerAuthToken) {
      respondError(socket, id, -32001, "Handshake ACP broker authentication failed");
      return;
    }
    if (authorityRole !== "ORCHESTRATOR" || authorityBranch !== "gov_kernel") {
      respondError(socket, id, -32002, "Handshake ACP broker requires ORCHESTRATOR authority on gov_kernel");
      return;
    }
    if (expectedBuildId && expectedBuildId !== SESSION_CONTROL_BROKER_BUILD_ID) {
      respondError(socket, id, -32003, "Handshake ACP broker build mismatch", {
        expected_broker_build_id: expectedBuildId,
        broker_build_id: SESSION_CONTROL_BROKER_BUILD_ID,
      });
      return;
    }
    if (expectedAuthMode && expectedAuthMode !== SESSION_CONTROL_BROKER_AUTH_MODE) {
      respondError(socket, id, -32011, "Handshake ACP broker auth mode mismatch", {
        expected_auth_mode: expectedAuthMode,
        broker_auth_mode: SESSION_CONTROL_BROKER_AUTH_MODE,
      });
      return;
    }
    socketContexts.set(socket, {
      authenticated: true,
      client_name: String(params?.client?.name || ""),
      client_version: String(params?.client?.version || ""),
    });
    respond(socket, id, {
      protocol: "ACP",
      protocol_version: "1.0",
      agent: {
        name: "handshake-acp-bridge",
        version: "0.3.0",
      },
      broker_build_id: SESSION_CONTROL_BROKER_BUILD_ID,
      broker_auth_mode: SESSION_CONTROL_BROKER_AUTH_MODE,
      capabilities: {
        methods: [
          "session/new",
          "session/load",
          "session/prompt",
          "session/cancel",
          "session/close",
        ],
        notifications: ["session/update"],
      },
    });
    return;
  }

  const context = socketContexts.get(socket);
  if (!context?.authenticated) {
    respondError(socket, id, -32000, "Handshake ACP broker requires authenticated initialize before use");
    return;
  }

  if (method === "session/load") {
    handleSessionLoad(socket, id, params);
    return;
  }
  if (method === "broker/shutdown") {
    handleBrokerShutdown(socket, id, params);
    return;
  }
  if (method === "session/cancel") {
    handleSessionCancel(socket, id, params);
    return;
  }
  if (method === "session/close") {
    handleSessionClose(socket, id, params);
    return;
  }
  if (method === "session/new") {
    void runGovernedRequest(socket, id, params.request || params, "START_SESSION");
    return;
  }
  if (method === "session/prompt") {
    void runGovernedRequest(socket, id, params.request || params, "SEND_PROMPT");
    return;
  }

  respondError(socket, id, -32601, `Method not found: ${method}`);
}

function handleSocket(socket) {
  socket.setEncoding("utf8");
  socketContexts.set(socket, { authenticated: false });
  let buffer = "";
  socket.on("data", (chunk) => {
    buffer += chunk.toString("utf8");
    const lines = buffer.split(/\r?\n/);
    buffer = lines.pop() || "";
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      try {
        handleRequest(socket, JSON.parse(trimmed));
      } catch (error) {
        respondError(socket, null, -32700, `Invalid JSON: ${error.message}`);
      }
    }
  });
  socket.on("close", () => {
    socketContexts.delete(socket);
  });
}

function shutdown() {
  for (const run of activeRuns.values()) {
    run.terminationReason = "Handshake ACP broker shutdown while governed run was active.";
    if (run.childPid) killProcessTree(run.childPid);
  }
  setTimeout(() => process.exit(0), 250);
}

ensureSessionStateFiles(repoRoot);
reconcileOrphanedRuns();
reconcileRecoverableRequests();

const prior = loadBrokerState();
if (prior.broker_pid && prior.broker_pid !== process.pid && isProcessAlive(prior.broker_pid)) {
  console.error("[HANDSHAKE_ACP_BROKER] Another broker is already running.");
  process.exit(1);
}

const server = net.createServer(handleSocket);
server.listen(0, serverHost, () => {
  persistBrokerState(server.address().port);
  const { registry } = loadSessionRegistry(repoRoot);
  for (const session of registry.sessions || []) {
    if (Array.isArray(session.pending_control_queue) && session.pending_control_queue.length > 0) {
      void drainQueuedRequestsForSession(session.session_key);
    }
  }
});

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
