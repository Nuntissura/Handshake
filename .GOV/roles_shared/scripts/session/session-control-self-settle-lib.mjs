import fs from "node:fs";
import path from "node:path";
import {
  appendJsonlLine,
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionRegistry,
  markSessionCommandResult,
  mutateSessionRegistrySync,
} from "./session-registry-lib.mjs";
import {
  buildSessionControlResult,
  defaultSessionOutputFile,
} from "./session-control-lib.mjs";
import {
  SESSION_CONTROL_RESULTS_FILE,
  normalizePath,
} from "./session-policy.mjs";

function parseJsonlFileSafe(filePath) {
  if (!fs.existsSync(filePath)) return [];
  const lines = fs.readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const events = [];
  for (const line of lines) {
    try {
      events.push(JSON.parse(line));
    } catch {
      // Ignore malformed lines; the broker/runtime checks still validate shape elsewhere.
    }
  }
  return events;
}

function normalizeStatus(value) {
  return String(value || "").trim().toUpperCase();
}

function appendResultOnce(repoRoot, result, resultById) {
  if (resultById.has(result.command_id)) return;
  appendJsonlLine(path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE), result);
  resultById.set(result.command_id, result);
}

function appendSelfSettleEvent(repoRoot, outputJsonlFile, details = {}) {
  const outputPath = path.resolve(repoRoot, String(outputJsonlFile || ""));
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.appendFileSync(
    outputPath,
    `${JSON.stringify({ timestamp: new Date().toISOString(), type: "broker.self_settle", ...details })}\n`,
    "utf8",
  );
}

export function inferRecoverableSessionControlResult({
  repoRoot,
  request,
  session = null,
  resultById = new Map(),
} = {}) {
  const outputJsonlFile = String(
    request?.output_jsonl_file
    || defaultSessionOutputFile(repoRoot, request?.session_key || "", request?.command_id || ""),
  ).trim();
  const outputEvents = parseJsonlFileSafe(path.resolve(repoRoot, outputJsonlFile));
  const rejectedEvent = outputEvents.find((event) => event?.type === "broker.rejected");
  if (rejectedEvent) {
    const reason = String(rejectedEvent.reason || "Governed request rejected by broker.").trim();
    return {
      status: "FAILED",
      summary: reason,
      error: reason,
      threadId: String(session?.session_thread_id || request?.session_thread_id || "").trim(),
      outputJsonlFile,
      targetCommandId: String(request?.target_command_id || "").trim(),
      cancelStatus: String(request?.command_kind || "").trim().toUpperCase() === "CANCEL_SESSION" ? "rejected" : "",
      repairReason: "rejected_without_result_row",
    };
  }

  if (String(request?.command_kind || "").trim().toUpperCase() === "CANCEL_SESSION") {
    const targetCommandId = String(request?.target_command_id || "").trim();
    if (targetCommandId && resultById.has(targetCommandId)) {
      return {
        status: "COMPLETED",
        summary: `Recovered cancel result after target ${targetCommandId} was already settled.`,
        error: "",
        threadId: String(session?.session_thread_id || request?.session_thread_id || "").trim(),
        outputJsonlFile,
        targetCommandId,
        cancelStatus: "target_already_settled",
        repairReason: "cancel_target_already_settled",
      };
    }
  }

  if (session && String(session?.last_command_id || "").trim() === String(request?.command_id || "").trim()) {
    const lastCommandStatus = normalizeStatus(session?.last_command_status);
    if (lastCommandStatus === "COMPLETED" || lastCommandStatus === "FAILED") {
      const outputFileFromSession = String(session?.last_command_output_file || outputJsonlFile).trim();
      return {
        status: lastCommandStatus,
        summary: `Recovered missing terminal result from session registry state for ${request.command_id}.`,
        error: lastCommandStatus === "FAILED"
          ? String(session?.last_error || "Recovered missing failed result from session registry state.").trim()
          : "",
        threadId: String(session?.session_thread_id || request?.session_thread_id || "").trim(),
        outputJsonlFile: outputFileFromSession,
        targetCommandId: String(request?.target_command_id || "").trim(),
        cancelStatus: "",
        repairReason: "session_registry_terminal_state",
      };
    }
    if (lastCommandStatus === "RUNNING") {
      return {
        status: "FAILED",
        summary: `Recovered orphaned governed request ${request.command_id} after session stayed RUNNING without an active broker run.`,
        error: "Governed request remained RUNNING in session registry but no active broker run or settled result survived.",
        threadId: String(session?.session_thread_id || request?.session_thread_id || "").trim(),
        outputJsonlFile,
        targetCommandId: String(request?.target_command_id || "").trim(),
        cancelStatus: "",
        repairReason: "running_without_active_broker_run",
      };
    }
  }

  return {
    status: "FAILED",
    summary: `Self-settled missing terminal result for governed request ${request?.command_id || "<missing>"}.`,
    error: "No active broker run or settled result remained for this request.",
    threadId: String(session?.session_thread_id || request?.session_thread_id || "").trim(),
    outputJsonlFile,
    targetCommandId: String(request?.target_command_id || "").trim(),
    cancelStatus: "",
    repairReason: "missing_terminal_result_without_active_run",
  };
}

export function settleRecoverableSessionControlResults(repoRoot, {
  commandIds = [],
  brokerState = null,
} = {}) {
  const { requests } = loadSessionControlRequests(repoRoot);
  const { results } = loadSessionControlResults(repoRoot);
  const { registry } = loadSessionRegistry(repoRoot);
  const resultById = new Map(results.map((result) => [String(result?.command_id || "").trim(), result]));
  const sessionByKey = new Map((registry.sessions || []).map((session) => [String(session?.session_key || "").trim(), session]));
  const activeRunIds = new Set(
    Array.isArray(brokerState?.active_runs)
      ? brokerState.active_runs.map((run) => String(run?.command_id || "").trim()).filter(Boolean)
      : [],
  );
  const onlyCommandIds = new Set((commandIds || []).map((value) => String(value || "").trim()).filter(Boolean));
  const settled = [];

  for (const request of requests) {
    const commandId = String(request?.command_id || "").trim();
    if (!commandId) continue;
    if (onlyCommandIds.size > 0 && !onlyCommandIds.has(commandId)) continue;
    if (resultById.has(commandId)) continue;
    if (activeRunIds.has(commandId)) continue;

    const session = sessionByKey.get(String(request?.session_key || "").trim()) || null;
    const inferred = inferRecoverableSessionControlResult({
      repoRoot,
      request,
      session,
      resultById,
    });
    const normalizedOutputPath = normalizePath(inferred.outputJsonlFile);
    appendSelfSettleEvent(repoRoot, normalizedOutputPath, {
      command_id: commandId,
      session_key: request.session_key,
      reason: inferred.repairReason,
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
      outputJsonlFile: normalizedOutputPath,
      lastAgentMessage: "",
      error: inferred.error,
      durationMs: 0,
      targetCommandId: inferred.targetCommandId,
      cancelStatus: inferred.cancelStatus,
    });

    mutateSessionRegistrySync(repoRoot, (liveRegistry) => {
      const liveSession = (liveRegistry.sessions || []).find((entry) => entry.session_key === request.session_key) || null;
      appendResultOnce(repoRoot, result, resultById);
      if (!liveSession) return;
      if (String(request.command_kind || "").trim().toUpperCase() === "CANCEL_SESSION") {
        liveSession.last_event_at = result.processed_at;
        return;
      }
      if (String(liveSession.last_command_id || "").trim() === commandId) {
        markSessionCommandResult(liveSession, result);
      } else {
        liveSession.last_event_at = result.processed_at;
      }
    });

    settled.push({
      command_id: commandId,
      status: result.status,
      repair_reason: inferred.repairReason,
      output_jsonl_file: normalizedOutputPath,
    });
  }

  return { settled };
}
