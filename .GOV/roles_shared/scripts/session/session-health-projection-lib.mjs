import { SESSION_CONTROL_RUN_STALE_GRACE_SECONDS } from "./session-policy.mjs";

export const SESSION_HEALTH_STATE_VALUES = ["UNKNOWN", "HEALTHY", "DEGRADED", "FAILED"];
export const SESSION_HEALTH_REASON_CODE_VALUES = [
  "UNKNOWN",
  "HEALTHY",
  "TARGET_SESSION_MISSING",
  "SESSION_RUNTIME_FAILED",
  "SESSION_NOT_STEERABLE",
  "ACTIVE_RUN_TIMEOUT",
  "COMMAND_OUTPUT_IDLE",
  "HEARTBEAT_STALE",
  "HEARTBEAT_DEGRADED",
];
export const SESSION_HEALTH_SOURCE = "ACP_WATCHDOG_V1";
export const SESSION_HEARTBEAT_DEGRADED_SECONDS = 600;
export const SESSION_HEARTBEAT_FAILED_SECONDS = 1200;
const ACTIVE_RUNTIME_STATE_VALUES = new Set(["STARTING", "READY", "COMMAND_RUNNING", "ACTIVE", "WAITING"]);
const STEERABLE_RUNTIME_STATE_VALUES = new Set(["STARTING", "READY", "COMMAND_RUNNING", "ACTIVE", "WAITING"]);

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  if (!raw || /^<unassigned>$/i.test(raw)) return null;
  return raw;
}

function normalizeHealthState(value) {
  const normalized = String(value || "").trim().toUpperCase();
  return SESSION_HEALTH_STATE_VALUES.includes(normalized) ? normalized : "UNKNOWN";
}

function normalizeReasonCode(value) {
  const normalized = String(value || "").trim().toUpperCase();
  return SESSION_HEALTH_REASON_CODE_VALUES.includes(normalized) ? normalized : "UNKNOWN";
}

function parseTimestampMs(value) {
  const ms = Date.parse(String(value || "").trim());
  return Number.isNaN(ms) ? null : ms;
}

function ageSecondsFromTimestamp(value, nowMs) {
  const parsed = parseTimestampMs(value);
  if (parsed === null) return null;
  return Math.max(0, Math.trunc((nowMs - parsed) / 1000));
}

function latestSessionActivitySeconds(session = null, nowMs = Date.now()) {
  if (!session || typeof session !== "object") return null;
  return ageSecondsFromTimestamp(
    session.last_heartbeat_at
      || session.last_event_at
      || session.last_command_completed_at
      || session.last_command_prompt_at,
    nowMs,
  );
}

function timedOutActiveRuns(activeRuns = [], nowMs = Date.now()) {
  return (Array.isArray(activeRuns) ? activeRuns : []).filter((run) => {
    const timeoutAtMs = parseTimestampMs(run?.timeout_at);
    if (timeoutAtMs === null) return false;
    return nowMs > (timeoutAtMs + (SESSION_CONTROL_RUN_STALE_GRACE_SECONDS * 1000));
  });
}

function buildProjection({
  healthState = "UNKNOWN",
  reasonCode = "UNKNOWN",
  summary = "",
  session = null,
  targetRole = "",
  targetSession = null,
  activeRuns = [],
  timedOutRuns = [],
  heartbeatAgeSeconds = null,
  outputIdleSeconds = null,
} = {}) {
  return {
    healthState: normalizeHealthState(healthState),
    reasonCode: normalizeReasonCode(reasonCode),
    summary: String(summary || "").trim() || "No ACP health summary recorded.",
    sessionKey: String(session?.session_key || "").trim() || normalizeSession(targetSession) || null,
    targetRole: normalizeRole(targetRole || session?.role),
    targetSession: normalizeSession(targetSession || session?.session_key),
    runtimeState: String(session?.runtime_state || "").trim().toUpperCase() || "UNKNOWN",
    activeRunCount: Array.isArray(activeRuns) ? activeRuns.length : 0,
    timedOutRunCount: Array.isArray(timedOutRuns) ? timedOutRuns.length : 0,
    heartbeatAgeSeconds,
    outputIdleSeconds: Number.isInteger(outputIdleSeconds) ? outputIdleSeconds : null,
    source: SESSION_HEALTH_SOURCE,
  };
}

export function evaluateGovernedSessionHealth({
  targetRole = "",
  targetSession = null,
  session = null,
  activeRuns = [],
  outputFreshnessStatus = "UNKNOWN",
  outputIdleSeconds = null,
  relayStatus = null,
  now = new Date(),
  heartbeatDegradedSeconds = SESSION_HEARTBEAT_DEGRADED_SECONDS,
  heartbeatFailedSeconds = SESSION_HEARTBEAT_FAILED_SECONDS,
} = {}) {
  const nowMs = now instanceof Date ? now.getTime() : Date.now();
  const normalizedTargetRole = normalizeRole(targetRole || relayStatus?.target_role);
  const normalizedTargetSession = normalizeSession(targetSession || relayStatus?.target_session);
  const runs = Array.isArray(activeRuns) ? activeRuns : [];
  const runtimeState = String(session?.runtime_state || "").trim().toUpperCase();
  const heartbeatAgeSeconds = latestSessionActivitySeconds(session, nowMs);
  const expiredRuns = timedOutActiveRuns(runs, nowMs);
  const outputFreshness = String(outputFreshnessStatus || "").trim().toUpperCase();
  const targetLabel = `${normalizedTargetRole || "UNKNOWN"}${normalizedTargetSession ? `:${normalizedTargetSession}` : ""}`;
  const steerableRuntime = STEERABLE_RUNTIME_STATE_VALUES.has(runtimeState);

  if (!session) {
    return buildProjection({
      healthState: "FAILED",
      reasonCode: "TARGET_SESSION_MISSING",
      summary: `Runtime expects ${targetLabel}, but no governed session record exists in the session registry.`,
      targetRole: normalizedTargetRole,
      targetSession: normalizedTargetSession,
      activeRuns: runs,
      timedOutRuns: expiredRuns,
      heartbeatAgeSeconds,
      outputIdleSeconds,
    });
  }

  if (runtimeState === "FAILED") {
    return buildProjection({
      healthState: "FAILED",
      reasonCode: "SESSION_RUNTIME_FAILED",
      summary: `${targetLabel} is already recorded in FAILED runtime state.`,
      session,
      targetRole: normalizedTargetRole,
      targetSession: normalizedTargetSession,
      activeRuns: runs,
      timedOutRuns: expiredRuns,
      heartbeatAgeSeconds,
      outputIdleSeconds,
    });
  }

  if (normalizedTargetRole && !steerableRuntime && ACTIVE_RUNTIME_STATE_VALUES.has(runtimeState) === false && runtimeState !== "UNSTARTED") {
    return buildProjection({
      healthState: "FAILED",
      reasonCode: "SESSION_NOT_STEERABLE",
      summary: `${targetLabel} is not steerable from runtime state ${runtimeState || "UNKNOWN"}.`,
      session,
      targetRole: normalizedTargetRole,
      targetSession: normalizedTargetSession,
      activeRuns: runs,
      timedOutRuns: expiredRuns,
      heartbeatAgeSeconds,
      outputIdleSeconds,
    });
  }

  if (expiredRuns.length > 0) {
    return buildProjection({
      healthState: "FAILED",
      reasonCode: "ACTIVE_RUN_TIMEOUT",
      summary: `${targetLabel} still has ${expiredRuns.length} active run(s) recorded after broker timeout expiry.`,
      session,
      targetRole: normalizedTargetRole,
      targetSession: normalizedTargetSession,
      activeRuns: runs,
      timedOutRuns: expiredRuns,
      heartbeatAgeSeconds,
      outputIdleSeconds,
    });
  }

  if (runs.length > 0 && outputFreshness === "STALE") {
    return buildProjection({
      healthState: "FAILED",
      reasonCode: "COMMAND_OUTPUT_IDLE",
      summary: `${targetLabel} still has an active run, but the governed command output has gone idle${Number.isInteger(outputIdleSeconds) ? ` for ${outputIdleSeconds}s` : ""}.`,
      session,
      targetRole: normalizedTargetRole,
      targetSession: normalizedTargetSession,
      activeRuns: runs,
      timedOutRuns: expiredRuns,
      heartbeatAgeSeconds,
      outputIdleSeconds,
    });
  }

  if (ACTIVE_RUNTIME_STATE_VALUES.has(runtimeState) && Number.isInteger(heartbeatAgeSeconds) && heartbeatAgeSeconds >= heartbeatFailedSeconds) {
    return buildProjection({
      healthState: "FAILED",
      reasonCode: "HEARTBEAT_STALE",
      summary: `${targetLabel} has not reported heartbeat or session activity for ${heartbeatAgeSeconds}s.`,
      session,
      targetRole: normalizedTargetRole,
      targetSession: normalizedTargetSession,
      activeRuns: runs,
      timedOutRuns: expiredRuns,
      heartbeatAgeSeconds,
      outputIdleSeconds,
    });
  }

  if (ACTIVE_RUNTIME_STATE_VALUES.has(runtimeState) && Number.isInteger(heartbeatAgeSeconds) && heartbeatAgeSeconds >= heartbeatDegradedSeconds) {
    return buildProjection({
      healthState: "DEGRADED",
      reasonCode: "HEARTBEAT_DEGRADED",
      summary: `${targetLabel} heartbeat is aging (${heartbeatAgeSeconds}s since last activity) but has not crossed the hard-fail threshold yet.`,
      session,
      targetRole: normalizedTargetRole,
      targetSession: normalizedTargetSession,
      activeRuns: runs,
      timedOutRuns: expiredRuns,
      heartbeatAgeSeconds,
      outputIdleSeconds,
    });
  }

  return buildProjection({
    healthState: "HEALTHY",
    reasonCode: "HEALTHY",
    summary: `${targetLabel || (session?.session_key || "session")} is within the current ACP health thresholds.`,
    session,
    targetRole: normalizedTargetRole,
    targetSession: normalizedTargetSession,
    activeRuns: runs,
    timedOutRuns: expiredRuns,
    heartbeatAgeSeconds,
    outputIdleSeconds,
  });
}

export function applySessionHealthProjection(session = null, projection = null, { updatedAt = new Date().toISOString() } = {}) {
  if (!session || typeof session !== "object" || !projection || typeof projection !== "object") return false;
  const nextState = normalizeHealthState(projection.healthState);
  const nextReason = normalizeReasonCode(projection.reasonCode);
  const nextSummary = String(projection.summary || "").trim();
  const nextSource = String(projection.source || SESSION_HEALTH_SOURCE).trim() || SESSION_HEALTH_SOURCE;
  const changed = session.health_state !== nextState
    || session.health_reason_code !== nextReason
    || session.health_summary !== nextSummary
    || session.health_source !== nextSource
    || !String(session.health_updated_at || "").trim();
  if (!changed) return false;
  session.health_state = nextState;
  session.health_reason_code = nextReason;
  session.health_summary = nextSummary;
  session.health_source = nextSource;
  session.health_updated_at = String(updatedAt || new Date().toISOString());
  return changed;
}

export function buildAcpHealthAlertCandidate({
  wpId = "",
  projection = null,
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId || !projection || typeof projection !== "object") return null;
  if (normalizeHealthState(projection.healthState) === "HEALTHY") return null;
  if (normalizeHealthState(projection.healthState) === "UNKNOWN") return null;
  const sessionKey = String(projection.sessionKey || projection.targetSession || "unknown-session").trim();
  const targetLabel = `${normalizeRole(projection.targetRole)}${projection.targetSession ? `:${projection.targetSession}` : ""}`.replace(/^:/, "");
  return {
    wpId: normalizedWpId,
    sourceKind: "ACP_HEALTH_ALERT",
    targetRole: "ORCHESTRATOR",
    correlationId: `acp-health:${normalizedWpId}:${sessionKey}:${normalizeReasonCode(projection.reasonCode)}`,
    summary: [
      `ACP_HEALTH_ALERT: ${targetLabel || sessionKey}`,
      normalizeHealthState(projection.healthState),
      normalizeReasonCode(projection.reasonCode),
      String(projection.summary || "").trim(),
    ].filter(Boolean).join(" | "),
  };
}

export function acpHealthAlertAlreadyPending(pendingNotifications = [], candidate = null) {
  if (!candidate?.correlationId) return false;
  return (Array.isArray(pendingNotifications) ? pendingNotifications : []).some((entry) =>
    normalizeRole(entry?.target_role) === "ORCHESTRATOR"
    && normalizeRole(entry?.source_kind) === "ACP_HEALTH_ALERT"
    && String(entry?.correlation_id || "").trim() === String(candidate.correlationId || "").trim()
    && String(entry?.summary || "").trim() === String(candidate.summary || "").trim()
  );
}
