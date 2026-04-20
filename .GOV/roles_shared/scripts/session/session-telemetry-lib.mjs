import path from "node:path";

import { REPO_ROOT } from "../lib/runtime-paths.mjs";
import {
  ageSecondsFromEvent,
  inspectSessionOutputActivity,
  itemTypeOfEvent,
  summarizeActivityEvent,
} from "./session-output-activity-lib.mjs";

const RUN_STARTING_STATES = new Set([
  "PLUGIN_REQUESTED",
  "TERMINAL_COMMAND_DISPATCHED",
  "PLUGIN_CONFIRMED",
  "CLI_ESCALATION_READY",
  "CLI_ESCALATION_USED",
  "STARTING",
]);
const RUN_RUNNING_STATES = new Set(["COMMAND_RUNNING", "ACTIVE"]);
const RUN_WAITING_STATES = new Set(["WAITING"]);
const RUN_READY_STATES = new Set(["READY"]);
const RUN_TERMINAL_STATES = new Set(["CLOSED", "COMPLETED"]);
const RUN_FAILED_STATES = new Set(["FAILED"]);
const PUSH_ALERT_SOURCE_KINDS = new Set(["ACP_HEALTH_ALERT", "RELAY_WATCHDOG_REPAIR"]);

function normalizeUpper(value, fallback = "UNKNOWN") {
  const text = String(value || "").trim().toUpperCase();
  return text || fallback;
}

function normalizeText(value, fallback = "") {
  const text = String(value || "").trim();
  return text || fallback;
}

function parseTimestampMs(value) {
  const parsed = Date.parse(String(value || "").trim());
  return Number.isNaN(parsed) ? null : parsed;
}

function latestTimestamp(...values) {
  const ordered = values
    .flat()
    .map((value) => String(value || "").trim())
    .filter(Boolean)
    .sort((left, right) => left.localeCompare(right));
  return ordered.at(-1) || "";
}

function queueCountOf(session = {}) {
  const explicit = Number(session?.pending_control_queue_count);
  if (Number.isInteger(explicit) && explicit >= 0) return explicit;
  if (Array.isArray(session?.pending_control_queue)) return session.pending_control_queue.length;
  if (session?.next_queued_control_request && typeof session.next_queued_control_request === "object") return 1;
  return 0;
}

function waitReasonForSession(session = {}, activeRunCount = 0) {
  const queueCount = queueCountOf(session);
  if (queueCount > 0) {
    return normalizeUpper(
      session?.next_queued_control_request?.queue_reason_code
      || session?.next_queued_control_request?.blocking_command_id
      || "BUSY_QUEUE",
      "BUSY_QUEUE",
    );
  }
  if (activeRunCount > 0) return "ACTIVE_RUN";
  const healthReason = normalizeUpper(session?.health_reason_code, "");
  if (healthReason && healthReason !== "HEALTHY" && healthReason !== "UNKNOWN") return healthReason;
  const disposition = normalizeUpper(session?.effective_governed_action?.resume_disposition, "");
  if (disposition && disposition !== "NONE") return disposition;
  const runtimeState = normalizeUpper(session?.runtime_state);
  if (runtimeState === "READY") return "STEERABLE";
  return runtimeState;
}

function stateForRunTelemetry(session = {}, activeRunCount = 0) {
  const runtimeState = normalizeUpper(session?.runtime_state);
  const queueCount = queueCountOf(session);
  if (RUN_FAILED_STATES.has(runtimeState)) return "FAILED";
  if (activeRunCount > 0 || RUN_RUNNING_STATES.has(runtimeState)) return "RUNNING";
  if (queueCount > 0) return "QUEUED";
  if (RUN_STARTING_STATES.has(runtimeState)) return "STARTING";
  if (RUN_WAITING_STATES.has(runtimeState)) return "WAITING";
  if (RUN_READY_STATES.has(runtimeState)) return "READY";
  if (RUN_TERMINAL_STATES.has(runtimeState)) return "TERMINAL";
  if (runtimeState === "UNSTARTED") return "UNSTARTED";
  return "UNKNOWN";
}

function resolveOutputPath(repoRoot, filePath) {
  const raw = String(filePath || "").trim();
  if (!raw) return "";
  return path.isAbsolute(raw) ? path.resolve(raw) : path.resolve(repoRoot || REPO_ROOT, raw);
}

function summarizeEventKind(event = null) {
  if (!event || typeof event !== "object") return "none";
  const itemType = normalizeText(itemTypeOfEvent(event), "");
  return itemType || normalizeText(event?.type, "none");
}

function summarizeAlertLine(alert = null) {
  if (!alert) return "none";
  return `${alert.state} ${alert.source_kind} @ ${alert.timestamp_utc || "<no-ts>"}`;
}

export function activeRunsForSession(session = {}, activeRuns = []) {
  const sessionKey = normalizeText(session?.session_key, "");
  const wpId = normalizeText(session?.wp_id, "");
  const role = normalizeUpper(session?.role, "");
  return (Array.isArray(activeRuns) ? activeRuns : []).filter((run) => {
    const runSessionKey = normalizeText(run?.session_key, "");
    if (sessionKey && runSessionKey) return runSessionKey === sessionKey;
    return normalizeText(run?.wp_id, "") === wpId
      && normalizeUpper(run?.role, "") === role;
  });
}

export function buildSessionRunTelemetry({
  session = {},
  activeRuns = [],
  now = new Date(),
} = {}) {
  const nowMs = now instanceof Date ? now.getTime() : Date.now();
  const runs = Array.isArray(activeRuns) ? activeRuns : [];
  const timedOutRunCount = runs.filter((run) => {
    const timeoutMs = parseTimestampMs(run?.timeout_at);
    return Number.isFinite(timeoutMs) && timeoutMs <= nowMs;
  }).length;
  const queuedRequestCount = queueCountOf(session);
  const activeRunCount = runs.length;
  const state = stateForRunTelemetry(session, activeRunCount);
  const waitReason = waitReasonForSession(session, activeRunCount);
  const updatedAt = latestTimestamp(
    session?.last_event_at,
    session?.health_updated_at,
    session?.last_command_completed_at,
    session?.last_command_prompt_at,
  );
  return {
    state,
    runtime_state: normalizeUpper(session?.runtime_state),
    wait_reason_code: waitReason,
    active_run_count: activeRunCount,
    timed_out_run_count: timedOutRunCount,
    queued_request_count: queuedRequestCount,
    health_state: normalizeUpper(session?.health_state),
    updated_at: updatedAt || null,
    summary: [
      `state=${state}`,
      `active=${activeRunCount}`,
      `queued=${queuedRequestCount}`,
      `wait=${waitReason}`,
      timedOutRunCount > 0 ? `timed_out=${timedOutRunCount}` : "",
    ].filter(Boolean).join(" | "),
  };
}

export function buildSessionStepTelemetry({
  session = {},
  repoRoot = REPO_ROOT,
  now = new Date(),
} = {}) {
  const nowMs = now instanceof Date ? now.getTime() : Date.now();
  const outputPath = resolveOutputPath(repoRoot, session?.last_command_output_file);
  const activity = inspectSessionOutputActivity(outputPath, { nowMs });
  const latestProgressEvent = activity.latestProgressEvent;
  const latestAgentMessageEvent = activity.latestAgentMessageEvent;
  const latestStepEvent = latestProgressEvent || latestAgentMessageEvent || activity.latestEvent || null;
  const outputIdleSeconds = Number.isInteger(activity.outputFileIdleSeconds) ? activity.outputFileIdleSeconds : null;
  const progressIdleSeconds = ageSecondsFromEvent(latestProgressEvent, nowMs);
  const latestStepAt = latestStepEvent?.timestamp || latestStepEvent?.item?.timestamp || null;

  let state = "NONE";
  if (activity.exists) {
    if (Number.isInteger(progressIdleSeconds) && progressIdleSeconds <= 120) {
      state = "ACTIVE";
    } else if (latestStepEvent && Number.isInteger(outputIdleSeconds) && outputIdleSeconds <= 600) {
      state = "IDLE";
    } else if (latestStepEvent || Number.isInteger(outputIdleSeconds)) {
      state = "STALE";
    } else {
      state = "OUTPUT_ONLY";
    }
  }

  const latestStepSummary = summarizeActivityEvent(latestStepEvent, { nowMs });
  return {
    state,
    output_exists: activity.exists,
    latest_step_kind: summarizeEventKind(latestStepEvent),
    latest_step_at: latestStepAt,
    latest_step_summary: latestStepSummary,
    latest_progress_summary: summarizeActivityEvent(latestProgressEvent, { nowMs }),
    latest_agent_message_summary: summarizeActivityEvent(latestAgentMessageEvent, { nowMs }),
    progress_idle_seconds: Number.isInteger(progressIdleSeconds) ? progressIdleSeconds : null,
    output_idle_seconds: outputIdleSeconds,
    summary: [
      `state=${state}`,
      `step=${latestStepSummary}`,
      Number.isInteger(outputIdleSeconds) ? `output_idle=${outputIdleSeconds}s` : "",
    ].filter(Boolean).join(" | "),
  };
}

export function buildSessionTelemetry({
  session = {},
  activeRuns = [],
  repoRoot = REPO_ROOT,
  now = new Date(),
} = {}) {
  const run = buildSessionRunTelemetry({
    session,
    activeRuns,
    now,
  });
  const step = buildSessionStepTelemetry({
    session,
    repoRoot,
    now,
  });
  return { run, step };
}

export function selectLatestPushAlert(
  notifications = [],
  {
    targetRole = "",
    targetSession = "",
  } = {},
) {
  const normalizedRole = normalizeUpper(targetRole, "");
  const normalizedSession = normalizeText(targetSession, "");
  const candidates = (Array.isArray(notifications) ? notifications : [])
    .filter((entry) => PUSH_ALERT_SOURCE_KINDS.has(normalizeUpper(entry?.source_kind)))
    .map((entry) => {
      const entryRole = normalizeUpper(entry?.target_role, "");
      const entrySession = normalizeText(entry?.target_session, "");
      const timestamp = String(entry?.timestamp_utc || entry?.created_at || "").trim();
      return {
        source_kind: normalizeUpper(entry?.source_kind),
        target_role: entryRole || null,
        target_session: entrySession || null,
        timestamp_utc: timestamp || null,
        summary: normalizeText(entry?.summary, ""),
        state: entry?.acknowledged ? "ACKED" : "PENDING",
        score: [
          entry?.acknowledged ? 0 : 1,
          normalizedRole && entryRole === normalizedRole ? 2 : (entryRole === "ORCHESTRATOR" ? 1 : 0),
          normalizedSession && entrySession === normalizedSession ? 1 : 0,
          parseTimestampMs(timestamp) || 0,
        ],
      };
    })
    .sort((left, right) => {
      for (let index = 0; index < left.score.length; index += 1) {
        const delta = Number(right.score[index] || 0) - Number(left.score[index] || 0);
        if (delta !== 0) return delta;
      }
      return String(right.timestamp_utc || "").localeCompare(String(left.timestamp_utc || ""));
    });
  return candidates[0] || null;
}

export function formatSessionRunTelemetryInline(runTelemetry = null) {
  if (!runTelemetry || typeof runTelemetry !== "object") return "run=none";
  return `run=${runTelemetry.state}(active=${runTelemetry.active_run_count || 0}|queued=${runTelemetry.queued_request_count || 0}|wait=${runTelemetry.wait_reason_code || "UNKNOWN"})`;
}

export function formatSessionStepTelemetryInline(stepTelemetry = null) {
  if (!stepTelemetry || typeof stepTelemetry !== "object") return "step=none";
  return `step=${stepTelemetry.state}(${stepTelemetry.latest_step_summary || "none"}${Number.isInteger(stepTelemetry.output_idle_seconds) ? `|idle=${stepTelemetry.output_idle_seconds}s` : ""})`;
}

export function formatPushAlertInline(alert = null) {
  return `alert=${summarizeAlertLine(alert)}`;
}
