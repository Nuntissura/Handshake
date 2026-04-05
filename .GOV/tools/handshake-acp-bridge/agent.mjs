#!/usr/bin/env node

import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  appendJsonlLine,
  ensureSessionStateFiles,
  loadSessionControlResults,
  loadSessionRegistry,
  mutateSessionRegistrySync,
  markSessionThreadObserved,
  markSessionCommandResult,
  markSessionCommandRunning,
  writeJsonFile,
} from "../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  buildSessionControlResult,
  defaultSessionOutputFile,
  ensureBrokerAuthToken,
  runCodexThreadCommand,
  runGovernedRoleCommand,
  validateSessionControlRequestShape,
} from "../../roles_shared/scripts/session/session-control-lib.mjs";
import { settleRecoverableSessionControlResults } from "../../roles_shared/scripts/session/session-control-self-settle-lib.mjs";
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
const serverHost = "127.0.0.1";
const activeRuns = new Map();
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

function findActiveRunBySession(sessionId) {
  return [...activeRuns.values()].find((run) => run.request.session_key === sessionId) || null;
}

function ensureResultPersisted(request, session, result) {
  appendResultOnce(result);
  session.active_host = SESSION_CONTROL_HOST_PRIMARY;
  session.active_terminal_kind = SESSION_ACTIVE_TERMINAL_KIND_NONE;
  markSessionCommandResult(session, result);
}

function settleRejectedRequest(request, reason) {
  const outputFile = defaultSessionOutputFile(repoRoot, request.session_key, request.command_id);
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
    threadId: session?.session_thread_id || "",
    summary: reason,
    outputJsonlFile: outputFile,
    lastAgentMessage: "",
    error: reason,
    durationMs: 0,
    targetCommandId: request.target_command_id || "",
    cancelStatus: request.command_kind === "CANCEL_SESSION" ? "rejected" : "",
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
    });

    ensureResultPersisted(requestRecord, session, result);
    return { priorThreadId, result };
  });

  respond(socket, id, {
    command_id: requestRecord.command_id,
    session_id: requestRecord.session_key,
    status: "completed",
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
    const result = settleRejectedRequest(
      request,
      `Concurrent governed run already active for ${request.session_key} (${concurrent.request.command_id})`,
    );
    respond(socket, id, result);
    return;
  }

  const { absWorktreeDir, selectedModel, selectedProfile, outputFile, threadId } = validation;
  const requestRecord = {
    ...request,
    selected_model: selectedModel,
    output_jsonl_file: outputFile.replace(/\\/g, "/"),
    session_thread_id: request.command_kind === "SEND_PROMPT" ? threadId : "",
  };

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
    rpcId: id,
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
  persistBrokerState(server.address().port);

  notify(socket, "session/update", {
    session_id: requestRecord.session_key,
    stage: "run.started",
    command_id: requestRecord.command_id,
    timestamp: runState.startedAt,
  });

  const settle = (execution) => {
    if (runState.settled) return;
    runState.settled = true;
    if (runState.timer) clearTimeout(runState.timer);
    activeRuns.delete(requestRecord.command_id);
    persistBrokerState(server.address().port);

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
      });

      ensureResultPersisted(requestRecord, latestSession, result);
      return result;
    });

    respond(socket, id, {
      command_id: requestRecord.command_id,
      session_id: requestRecord.session_key,
      thread_id: result.thread_id || "",
      status: String(result.status || "").toLowerCase(),
      output_jsonl_file: outputFile,
      last_agent_message: result.last_agent_message || "",
      error: result.error || "",
      duration_ms: result.duration_ms,
      broker_run_id: requestRecord.command_id,
    });
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
      persistBrokerState(server.address().port);
      if (runState.terminationReason && runState.childPid) {
        killProcessTree(runState.childPid);
      }
      notify(socket, "session/update", {
        session_id: requestRecord.session_key,
        stage: "process.spawned",
        command_id: requestRecord.command_id,
        pid: runState.childPid,
        timestamp: nowIso(),
      });
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
      notify(socket, "session/update", {
        session_id: requestRecord.session_key,
        command_id: requestRecord.command_id,
        stage: event.type || "event",
        thread_id: observedThreadId,
        timestamp: nowIso(),
      });
    },
  }).then(settle).catch((error) => {
    appendOutputEvent(outputFile, {
      type: "broker.error",
      message: error.message || "Handshake ACP broker run failed",
    });
    settle({
      ok: false,
      exitCode: 1,
      threadId: session.session_thread_id || "",
      lastAgentMessage: "",
      stderr: error.message || "Handshake ACP broker run failed",
      durationMs: 0,
    });
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
  respond(socket, id, {
    session_id: session.session_key,
    session_key: session.session_key,
    role: session.role,
    wp_id: session.wp_id,
    thread_id: session.session_thread_id || "",
    requested_model: session.requested_model || "",
    control_protocol: session.control_protocol || "",
    control_transport: session.control_transport || "",
    active_host: session.active_host || "NONE",
    active_terminal_kind: session.active_terminal_kind || "NONE",
    runtime_state: session.runtime_state || "",
    last_command_kind: session.last_command_kind || "",
    last_command_status: session.last_command_status || "",
    last_command_summary: session.last_command_summary || "",
    last_command_output_file: session.last_command_output_file || "",
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
      });
      mutateSessionRegistrySync(repoRoot, (registry) => {
        const session = registry.sessions.find((entry) => entry.session_key === requestRecord.session_key);
        if (session) session.last_event_at = result.processed_at;
        appendResultOnce(result);
      });
      respond(socket, id, {
        command_id: result.command_id,
        status: "not_running",
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
});

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
