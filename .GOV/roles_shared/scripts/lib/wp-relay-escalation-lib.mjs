function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  return raw || null;
}

function parseTimestamp(value) {
  const text = String(value || "").trim();
  if (!text) return null;
  const parsed = Date.parse(text);
  return Number.isNaN(parsed) ? null : parsed;
}

function isoFromTimestamp(value) {
  return Number.isFinite(value) ? new Date(value).toISOString() : null;
}

function maxTimestamp(values = []) {
  let result = null;
  for (const value of values) {
    const parsed = parseTimestamp(value);
    if (parsed === null) continue;
    if (result === null || parsed > result) result = parsed;
  }
  return result;
}

function minutesBetween(nowTs, thenTs) {
  if (!Number.isFinite(nowTs) || !Number.isFinite(thenTs)) return null;
  return Math.max(0, Math.round((nowTs - thenTs) / 60000));
}

function matchesTarget(entry, role, session) {
  const entryRole = normalizeRole(entry?.target_role);
  const entrySession = normalizeSession(entry?.target_session);
  if (entryRole !== normalizeRole(role)) return false;
  if (!session || !entrySession) return true;
  return entrySession === normalizeSession(session);
}

function matchingRoleSession(entry, role, session) {
  if (normalizeRole(entry?.role) !== normalizeRole(role)) return false;
  const expectedSession = normalizeSession(session);
  if (!expectedSession) return true;
  return normalizeSession(entry?.session_id || entry?.session_key?.split(":").slice(1).join(":")) === expectedSession;
}

function matchingRegistrySession(entry, role, wpId, session) {
  if (normalizeRole(entry?.role) !== normalizeRole(role)) return false;
  if (String(entry?.wp_id || "").trim() !== String(wpId || "").trim()) return false;
  const expectedSession = normalizeSession(session);
  if (!expectedSession) return true;
  const key = String(entry?.session_key || "").trim();
  return key === `${normalizeRole(role)}:${String(wpId || "").trim()}`
    || key.endsWith(`:${expectedSession}`)
    || normalizeSession(entry?.session_thread_id) === expectedSession
    || normalizeSession(entry?.session_id) === expectedSession;
}

function latestActorReceiptTimestamp(receipts, role, session) {
  return maxTimestamp(
    (receipts || [])
      .filter((entry) => normalizeRole(entry?.actor_role) === normalizeRole(role))
      .filter((entry) => {
        const expectedSession = normalizeSession(session);
        if (!expectedSession) return true;
        return normalizeSession(entry?.actor_session) === expectedSession;
      })
      .map((entry) => entry.timestamp_utc),
  );
}

function latestForeignReceiptTimestamp(receipts, role, session) {
  return maxTimestamp(
    (receipts || [])
      .filter((entry) => {
        const actorRole = normalizeRole(entry?.actor_role);
        const actorSession = normalizeSession(entry?.actor_session);
        if (actorRole !== normalizeRole(role)) return true;
        const expectedSession = normalizeSession(session);
        if (!expectedSession) return false;
        return actorSession !== expectedSession;
      })
      .map((entry) => entry.timestamp_utc),
  );
}

export function evaluateWpRelayEscalation({
  wpId = "",
  runtimeStatus = {},
  communicationEvaluation = null,
  receipts = [],
  pendingNotifications = [],
  registrySessions = [],
  nowIso = new Date().toISOString(),
} = {}) {
  const nowTs = parseTimestamp(nowIso) ?? Date.now();
  const nextActor = normalizeRole(runtimeStatus?.next_expected_actor);
  const nextSession = normalizeSession(runtimeStatus?.next_expected_session);
  const activeRelayRole = ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(nextActor);

  if (!communicationEvaluation?.applicable || !activeRelayRole) {
    return {
      applicable: false,
      status: "NOT_APPLICABLE",
      severity: "NONE",
      summary: "Relay escalation is not applicable for the current route.",
      reason_code: "NOT_APPLICABLE",
      target_role: nextActor || null,
      target_session: nextSession,
      recommended_command: null,
      metrics: {},
      warnings: [],
      failures: [],
    };
  }

  const heartbeatDueTs = parseTimestamp(runtimeStatus?.heartbeat_due_at);
  const staleAfterTs = parseTimestamp(runtimeStatus?.stale_after);
  const targetNotifications = (pendingNotifications || []).filter((entry) => matchesTarget(entry, nextActor, nextSession));
  const latestNotificationTs = maxTimestamp(targetNotifications.map((entry) => entry.timestamp_utc));
  const latestTargetReceiptTs = latestActorReceiptTimestamp(receipts, nextActor, nextSession);
  const latestForeignReceiptTs = latestForeignReceiptTimestamp(receipts, nextActor, nextSession);
  const routeAnchorTs = maxTimestamp([
    latestNotificationTs,
    latestForeignReceiptTs,
    (latestTargetReceiptTs && latestForeignReceiptTs === null) ? latestTargetReceiptTs : null,
  ]);
  const runtimeSessionActivityTs = maxTimestamp(
    (Array.isArray(runtimeStatus?.active_role_sessions) ? runtimeStatus.active_role_sessions : [])
      .filter((entry) => matchingRoleSession(entry, nextActor, nextSession))
      .map((entry) => entry.last_heartbeat_at),
  );
  const registrySessionActivityTs = maxTimestamp(
    (registrySessions || [])
      .filter((entry) => matchingRegistrySession(entry, nextActor, wpId, nextSession))
      .map((entry) => entry.updated_at || entry.last_event_at),
  );
  const latestSessionActivityTs = maxTimestamp([runtimeSessionActivityTs, registrySessionActivityTs]);
  const pendingNotificationCount = targetNotifications.length;
  const recommendedCommand = `just orchestrator-steer-next ${wpId}`;

  const metrics = {
    now_at: isoFromTimestamp(nowTs),
    heartbeat_due_at: isoFromTimestamp(heartbeatDueTs),
    stale_after: isoFromTimestamp(staleAfterTs),
    route_anchor_at: isoFromTimestamp(routeAnchorTs),
    latest_notification_at: isoFromTimestamp(latestNotificationTs),
    latest_target_receipt_at: isoFromTimestamp(latestTargetReceiptTs),
    latest_session_activity_at: isoFromTimestamp(latestSessionActivityTs),
    pending_notification_count: pendingNotificationCount,
    minutes_since_route_anchor: minutesBetween(nowTs, routeAnchorTs),
    minutes_since_latest_notification: minutesBetween(nowTs, latestNotificationTs),
    minutes_since_latest_target_receipt: minutesBetween(nowTs, latestTargetReceiptTs),
    minutes_since_latest_session_activity: minutesBetween(nowTs, latestSessionActivityTs),
    current_relay_escalation_cycle: Number(runtimeStatus?.current_relay_escalation_cycle || 0),
    max_relay_escalation_cycles: Number(runtimeStatus?.max_relay_escalation_cycles || 0),
  };

  const warnings = [];
  const failures = [];
  const targetLabel = `${nextActor}${nextSession ? `:${nextSession}` : ""}`;
  const thresholdPassed = Number.isFinite(staleAfterTs) && nowTs > staleAfterTs;
  const watchThresholdPassed = Number.isFinite(heartbeatDueTs) && nowTs > heartbeatDueTs;
  const sessionMovedAfterRoute = Number.isFinite(latestSessionActivityTs) && Number.isFinite(routeAnchorTs)
    && latestSessionActivityTs > routeAnchorTs;
  const receiptMovedAfterRoute = Number.isFinite(latestTargetReceiptTs) && Number.isFinite(routeAnchorTs)
    && latestTargetReceiptTs > routeAnchorTs;

  let status = "NORMAL";
  let severity = "NONE";
  let reasonCode = "ROUTE_HEALTHY";
  let summary = `Relay is healthy for ${targetLabel}.`;

  if (thresholdPassed && pendingNotificationCount > 0 && !receiptMovedAfterRoute) {
    status = "ESCALATED";
    severity = "FAIL";
    reasonCode = sessionMovedAfterRoute ? "SESSION_ACTIVE_NO_RECEIPT_PROGRESS" : "PENDING_NOTIFICATION_STALE";
    summary = sessionMovedAfterRoute
      ? `Relay is stalled for ${targetLabel}: the target session moved after the wake surface opened, but no receipt progress followed. Use ${recommendedCommand}.`
      : `Relay is stalled for ${targetLabel}: pending notifications crossed stale_after without receipt progress. Use ${recommendedCommand}.`;
    failures.push(summary);
  } else if (thresholdPassed && !receiptMovedAfterRoute) {
    status = "ESCALATED";
    severity = "FAIL";
    reasonCode = "RECEIPT_PROGRESS_STALE";
    summary = `Relay is stalled for ${targetLabel}: waiting crossed stale_after without new ${nextActor} receipt progress. Use ${recommendedCommand}.`;
    failures.push(summary);
  } else if (watchThresholdPassed && pendingNotificationCount > 0 && !receiptMovedAfterRoute) {
    status = "WATCH";
    severity = "WARN";
    reasonCode = "PENDING_NOTIFICATION_WAITING";
    summary = `Relay is waiting on ${targetLabel}: pending notifications exist and the lane has crossed heartbeat_due_at without receipt progress yet.`;
    warnings.push(summary);
  } else if (watchThresholdPassed && !receiptMovedAfterRoute) {
    status = "WATCH";
    severity = "WARN";
    reasonCode = "RECEIPT_PROGRESS_WAITING";
    summary = `Relay is waiting on ${targetLabel}: no new ${nextActor} receipt progress since the current route opened, but stale_after has not been crossed yet.`;
    warnings.push(summary);
  }

  return {
    applicable: true,
    status,
    severity,
    summary,
    reason_code: reasonCode,
    target_role: nextActor,
    target_session: nextSession,
    recommended_command: recommendedCommand,
    metrics,
    warnings,
    failures,
  };
}
