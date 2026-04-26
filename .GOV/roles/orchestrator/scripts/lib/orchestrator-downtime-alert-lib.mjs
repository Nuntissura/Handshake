export const ORCHESTRATOR_DOWNTIME_SOURCE_KIND = "RED_ALERT_ORCHESTRATOR_DOWNTIME";
export const ORCHESTRATOR_DOWNTIME_WARN_SECONDS = 600;
export const ORCHESTRATOR_DOWNTIME_RESCUE_SECONDS = 1200;

const TERMINAL_RUNTIME_STATUS_VALUES = new Set([
  "VALIDATED_CLOSED",
  "CLOSED",
  "DONE",
  "SUPERSEDED",
  "TERMINAL",
  "COMPLETED",
  "CANCELLED",
  "CANCELED",
  "ABORTED",
]);

function normalizeUpper(value = "", fallback = "") {
  const text = String(value || "").trim().toUpperCase();
  return text || fallback;
}

function normalizeText(value = "") {
  return String(value || "").trim();
}

function parseTimestampMs(value = "") {
  const parsed = Date.parse(String(value || "").trim());
  return Number.isNaN(parsed) ? null : parsed;
}

function pushTimestamp(sources, value, source) {
  const timestampMs = parseTimestampMs(value);
  if (timestampMs === null) return;
  sources.push({
    timestampMs,
    timestampIso: new Date(timestampMs).toISOString(),
    source,
  });
}

function latestTimestampSource(sources = []) {
  return [...sources].sort((left, right) => right.timestampMs - left.timestampMs)[0] || null;
}

function latestGovernedActionTimestamp(session = {}) {
  const action = session?.effective_governed_action || session?.last_governed_action || {};
  return [
    action.updated_at,
    action.completed_at,
    action.requested_at,
    session.next_queued_control_request?.updated_at,
    session.next_queued_control_request?.queued_at,
  ];
}

function progressTimestampSources({
  runtimeStatus = {},
  receipts = [],
  pendingNotifications = [],
  registrySessions = [],
  brokerActiveRuns = [],
} = {}) {
  const sources = [];
  pushTimestamp(sources, runtimeStatus?.last_event_at, "runtime.last_event_at");
  pushTimestamp(sources, runtimeStatus?.last_heartbeat_at, "runtime.last_heartbeat_at");
  pushTimestamp(sources, runtimeStatus?.last_milestone_sync_at, "runtime.last_milestone_sync_at");
  pushTimestamp(sources, runtimeStatus?.main_containment_verified_at_utc, "runtime.main_containment_verified_at_utc");
  pushTimestamp(sources, runtimeStatus?.current_main_compatibility_verified_at_utc, "runtime.current_main_compatibility_verified_at_utc");

  for (const entry of Array.isArray(receipts) ? receipts : []) {
    pushTimestamp(
      sources,
      entry?.timestamp_utc || entry?.timestamp,
      `receipt.${normalizeUpper(entry?.actor_role, "UNKNOWN")}.${normalizeUpper(entry?.receipt_kind, "UNKNOWN")}`,
    );
  }

  for (const entry of Array.isArray(pendingNotifications) ? pendingNotifications : []) {
    if (normalizeUpper(entry?.source_kind) === ORCHESTRATOR_DOWNTIME_SOURCE_KIND) continue;
    pushTimestamp(
      sources,
      entry?.timestamp_utc || entry?.created_at,
      `notification.${normalizeUpper(entry?.source_kind, "UNKNOWN")}.${normalizeUpper(entry?.target_role, "UNKNOWN")}`,
    );
  }

  for (const entry of Array.isArray(registrySessions) ? registrySessions : []) {
    pushTimestamp(sources, entry?.updated_at, `session.${normalizeUpper(entry?.role, "UNKNOWN")}.updated_at`);
    pushTimestamp(sources, entry?.health_updated_at, `session.${normalizeUpper(entry?.role, "UNKNOWN")}.health_updated_at`);
    pushTimestamp(sources, entry?.last_event_at, `session.${normalizeUpper(entry?.role, "UNKNOWN")}.last_event_at`);
    pushTimestamp(sources, entry?.last_heartbeat_at, `session.${normalizeUpper(entry?.role, "UNKNOWN")}.last_heartbeat_at`);
    for (const timestamp of latestGovernedActionTimestamp(entry)) {
      pushTimestamp(sources, timestamp, `session.${normalizeUpper(entry?.role, "UNKNOWN")}.governed_action`);
    }
  }

  for (const entry of Array.isArray(brokerActiveRuns) ? brokerActiveRuns : []) {
    pushTimestamp(sources, entry?.started_at, `broker.${normalizeUpper(entry?.role, "UNKNOWN")}.started_at`);
    pushTimestamp(sources, entry?.updated_at, `broker.${normalizeUpper(entry?.role, "UNKNOWN")}.updated_at`);
  }

  return sources;
}

export function formatDowntimeAge(seconds = null) {
  if (!Number.isInteger(seconds)) return "<unknown>";
  if (seconds < 90) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 90) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const rem = minutes % 60;
  return rem > 0 ? `${hours}h${rem}m` : `${hours}h`;
}

export function evaluateOrchestratorDowntime({
  wpId = "",
  workflowLane = "",
  runtimeStatus = {},
  receipts = [],
  pendingNotifications = [],
  registrySessions = [],
  brokerActiveRuns = [],
  now = new Date(),
  warnSeconds = ORCHESTRATOR_DOWNTIME_WARN_SECONDS,
  rescueSeconds = ORCHESTRATOR_DOWNTIME_RESCUE_SECONDS,
} = {}) {
  const normalizedWpId = normalizeText(wpId);
  const lane = normalizeUpper(workflowLane || runtimeStatus?.workflow_lane);
  if (!normalizedWpId || lane !== "ORCHESTRATOR_MANAGED") {
    return {
      applicable: false,
      status: "NOT_APPLICABLE",
      reason: "NON_ORCHESTRATOR_MANAGED",
      shouldEmit: false,
      severity: "NONE",
    };
  }

  const runtimeStatusValue = normalizeUpper(runtimeStatus?.runtime_status || runtimeStatus?.current_packet_status);
  if (TERMINAL_RUNTIME_STATUS_VALUES.has(runtimeStatusValue)) {
    return {
      applicable: false,
      status: "NOT_APPLICABLE",
      reason: `TERMINAL_RUNTIME_${runtimeStatusValue}`,
      shouldEmit: false,
      severity: "NONE",
    };
  }

  const nowMs = now instanceof Date ? now.getTime() : Date.now();
  const latestProgress = latestTimestampSource(progressTimestampSources({
    runtimeStatus,
    receipts,
    pendingNotifications,
    registrySessions,
    brokerActiveRuns,
  }));

  if (!latestProgress) {
    return {
      applicable: true,
      status: "UNKNOWN",
      reason: "NO_PROGRESS_TIMESTAMP",
      shouldEmit: false,
      severity: "WARN",
      wpId: normalizedWpId,
      ageSeconds: null,
      latestProgressAt: null,
      latestProgressSource: null,
      alertBand: "UNKNOWN",
      recommendedCommand: `just orchestrator-health ${normalizedWpId}`,
      summary: "No timestamped Orchestrator/control-plane progress evidence is available.",
    };
  }

  const ageSeconds = Math.max(0, Math.trunc((nowMs - latestProgress.timestampMs) / 1000));
  const rescueReached = ageSeconds >= rescueSeconds;
  const warnReached = ageSeconds >= warnSeconds;
  const recommendedCommand = rescueReached
    ? `just orchestrator-rescue ${normalizedWpId}`
    : `just orchestrator-health ${normalizedWpId}`;

  if (!warnReached) {
    return {
      applicable: true,
      status: "CLEAR",
      reason: "CONTROL_PLANE_PROGRESS_FRESH",
      shouldEmit: false,
      severity: "NONE",
      wpId: normalizedWpId,
      ageSeconds,
      latestProgressAt: latestProgress.timestampIso,
      latestProgressSource: latestProgress.source,
      alertBand: "CLEAR",
      recommendedCommand,
      summary: `Latest Orchestrator/control-plane progress is ${formatDowntimeAge(ageSeconds)} old.`,
    };
  }

  return {
    applicable: true,
    status: "RED_ALERT",
    reason: rescueReached ? "ORCHESTRATOR_DOWNTIME_RESCUE_READY" : "ORCHESTRATOR_DOWNTIME_WARN",
    shouldEmit: true,
    severity: rescueReached ? "FAIL" : "WARN",
    wpId: normalizedWpId,
    ageSeconds,
    latestProgressAt: latestProgress.timestampIso,
    latestProgressSource: latestProgress.source,
    alertBand: rescueReached ? "RESCUE" : "WARN",
    recommendedCommand,
    summary: rescueReached
      ? `No fresh Orchestrator/control-plane progress for ${formatDowntimeAge(ageSeconds)}; launch visible takeover with ${recommendedCommand}.`
      : `No fresh Orchestrator/control-plane progress for ${formatDowntimeAge(ageSeconds)}; inspect with ${recommendedCommand}.`,
  };
}

export function buildOrchestratorDowntimeAlertCandidate({
  wpId = "",
  evaluation = null,
} = {}) {
  const normalizedWpId = normalizeText(wpId || evaluation?.wpId);
  if (!normalizedWpId || !evaluation?.shouldEmit) return null;
  const band = normalizeUpper(evaluation.alertBand || "WARN");
  const ageLabel = formatDowntimeAge(evaluation.ageSeconds);
  const rescueSuffix = band === "RESCUE"
    ? ` Rescue command: just orchestrator-rescue ${normalizedWpId}.`
    : ` Health command: just orchestrator-health ${normalizedWpId}; rescue threshold command: just orchestrator-rescue ${normalizedWpId}.`;
  return {
    wpId: normalizedWpId,
    sourceKind: ORCHESTRATOR_DOWNTIME_SOURCE_KIND,
    targetRole: "ORCHESTRATOR",
    targetSession: null,
    correlationId: [
      "orchestrator-downtime",
      normalizedWpId,
      band,
    ].join(":"),
    summary: [
      `${ORCHESTRATOR_DOWNTIME_SOURCE_KIND}: ${normalizedWpId}`,
      `band=${band}`,
      `age=${ageLabel}`,
      `last_progress_at=${evaluation.latestProgressAt || "<unknown>"}`,
      `last_progress_source=${evaluation.latestProgressSource || "<unknown>"}`,
      evaluation.summary,
      rescueSuffix,
    ].filter(Boolean).join(" | "),
  };
}

export function orchestratorDowntimeAlertAlreadyPending(pendingNotifications = [], candidate = null) {
  if (!candidate?.correlationId) return false;
  return (Array.isArray(pendingNotifications) ? pendingNotifications : []).some((entry) =>
    normalizeUpper(entry?.target_role) === normalizeUpper(candidate.targetRole)
    && normalizeUpper(entry?.source_kind) === ORCHESTRATOR_DOWNTIME_SOURCE_KIND
    && normalizeText(entry?.correlation_id) === normalizeText(candidate.correlationId)
  );
}
