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

function maxParsedTimestamp(values = []) {
  let result = null;
  for (const value of values) {
    if (!Number.isFinite(value)) continue;
    if (result === null || value > result) result = value;
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

function routeReasonFromCommunicationState(communicationState = "", {
  nextActor = "",
  waitingOn = "",
} = {}) {
  const state = String(communicationState || "").trim().toUpperCase();
  const actor = normalizeRole(nextActor);
  const waiting = String(waitingOn || "").trim().toUpperCase();

  if (/HUMAN|OPERATOR|APPROVAL|SIGNATURE|MAIN_MERGE_PUSH/.test(waiting)) return "WAITING_ON_HUMAN_APPROVAL";
  if (/DEPENDENCY|BLOCKED/.test(waiting)) return "WAITING_ON_DEPENDENCY";
  if (/CHECKPOINT/.test(waiting)) return "WAITING_ON_ORCHESTRATOR_CHECKPOINT";

  switch (state) {
    case "COMM_WAITING_FOR_REVIEW":
      return actor === "WP_VALIDATOR" || actor === "INTEGRATION_VALIDATOR"
        ? "WAITING_ON_VALIDATOR_REVIEW"
        : "WAITING_ON_REVIEW";
    case "COMM_WAITING_FOR_FINAL_REVIEW":
      return "WAITING_ON_FINAL_REVIEW";
    case "COMM_REPAIR_REQUIRED":
      return "WAITING_ON_CODER_REPAIR";
    case "COMM_DEFERRED_REPAIR_QUEUE":
      return "WAITING_ON_CODER_DEFERRED_REPAIR";
    case "COMM_WAITING_FOR_HANDOFF":
      return "WAITING_ON_CODER_HANDOFF";
    case "COMM_WAITING_FOR_INTENT":
      return "WAITING_ON_CODER_INTENT";
    case "COMM_WAITING_FOR_INTENT_CHECKPOINT":
      return "WAITING_ON_VALIDATOR_CHECKPOINT";
    case "COMM_BLOCKED_OPEN_ITEMS":
      return "WAITING_ON_DEPENDENCY_OPEN_REVIEW_ITEMS";
    default:
      return "";
  }
}

function staleRouteReasonCode(baseReasonCode = "", fallback = "") {
  const reason = String(baseReasonCode || "").trim().toUpperCase();
  if (reason.startsWith("WAITING_ON_")) return `ROUTE_STALE_${reason}`;
  return String(fallback || "ROUTE_STALE").trim().toUpperCase() || "ROUTE_STALE";
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
  const routeAnchorTs = maxParsedTimestamp([
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
  const latestSessionActivityTs = maxParsedTimestamp([runtimeSessionActivityTs, registrySessionActivityTs]);
  const pendingNotificationCount = targetNotifications.length;
  const recommendedCommand = `just orchestrator-steer-next ${wpId} "<why this stalled relay should be re-woken, >=40 chars>"`;
  const blockingReasonCode = routeReasonFromCommunicationState(communicationEvaluation?.state, {
    nextActor,
    waitingOn: runtimeStatus?.waiting_on,
  });

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
    current_worker_interrupt_cycle: Number(runtimeStatus?.current_worker_interrupt_cycle || 0),
    max_worker_interrupt_cycles: Number(runtimeStatus?.max_worker_interrupt_cycles ?? 1),
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

  // RGF-185: Auto-settle route-stale WAITING_ON_CODER_HANDOFF when the coder session
  // is no longer active (CLOSED/COMPLETED/FAILED) or the next actor has already shifted
  // to a validator-owned action. This prevents stale handoff residue from requiring
  // manual steering when the coder is done.
  if (blockingReasonCode === "WAITING_ON_CODER_HANDOFF" && thresholdPassed) {
    const coderSessions = (registrySessions || []).filter(
      (entry) => normalizeRole(entry?.role) === "CODER" && String(entry?.wp_id || "").trim() === wpId,
    );
    const allCoderSessionsClosed = coderSessions.length > 0
      && coderSessions.every((entry) => {
        const state = String(entry?.runtime_state || "").trim().toUpperCase();
        return ["CLOSED", "COMPLETED", "FAILED", "STALE"].includes(state);
      });
    if (allCoderSessionsClosed) {
      return {
        applicable: true,
        status: "SELF_SETTLED",
        severity: "INFO",
        summary: `Route-stale WAITING_ON_CODER_HANDOFF auto-settled for ${targetLabel}: all coder sessions are closed (${coderSessions.map((e) => e.runtime_state).join(", ")}). Route residue should be compacted.`,
        reason_code: "ROUTE_STALE_HANDOFF_SELF_SETTLED",
        target_role: nextActor,
        target_session: nextSession,
        recommended_command: null,
        metrics,
        warnings: [`Route-stale handoff residue auto-settled: coder sessions are ${coderSessions.map((e) => e.runtime_state).join(", ")}.`],
        failures: [],
      };
    }
  }

  if (thresholdPassed && pendingNotificationCount > 0 && !receiptMovedAfterRoute) {
    status = "ESCALATED";
    severity = "FAIL";
    reasonCode = sessionMovedAfterRoute
      ? "SESSION_ACTIVE_NO_RECEIPT_PROGRESS"
      : staleRouteReasonCode(blockingReasonCode, "PENDING_NOTIFICATION_STALE");
    summary = sessionMovedAfterRoute
      ? `Relay is stalled for ${targetLabel}: the target session moved after the wake surface opened, but no receipt progress followed. Use ${recommendedCommand}.`
      : `Relay is stalled for ${targetLabel}: pending notifications crossed stale_after without receipt progress${blockingReasonCode ? ` while ${blockingReasonCode}.` : "."} Use ${recommendedCommand}.`;
    failures.push(summary);
  } else if (thresholdPassed && !receiptMovedAfterRoute) {
    status = "ESCALATED";
    severity = "FAIL";
    reasonCode = staleRouteReasonCode(blockingReasonCode, "RECEIPT_PROGRESS_STALE");
    summary = `Relay is stalled for ${targetLabel}: waiting crossed stale_after without new ${nextActor} receipt progress${blockingReasonCode ? ` while ${blockingReasonCode}.` : "."} Use ${recommendedCommand}.`;
    failures.push(summary);
  } else if (watchThresholdPassed && pendingNotificationCount > 0 && !receiptMovedAfterRoute) {
    status = "WATCH";
    severity = "WARN";
    reasonCode = blockingReasonCode || "PENDING_NOTIFICATION_WAITING";
    summary = `Relay is waiting on ${targetLabel}: pending notifications exist and the lane has crossed heartbeat_due_at without receipt progress yet${blockingReasonCode ? ` (${blockingReasonCode})` : ""}.`;
    warnings.push(summary);
  } else if (watchThresholdPassed && !receiptMovedAfterRoute) {
    status = "WATCH";
    severity = "WARN";
    reasonCode = blockingReasonCode || "RECEIPT_PROGRESS_WAITING";
    summary = `Relay is waiting on ${targetLabel}: no new ${nextActor} receipt progress since the current route opened, but stale_after has not been crossed yet${blockingReasonCode ? ` (${blockingReasonCode})` : ""}.`;
    warnings.push(summary);
  } else if (blockingReasonCode) {
    reasonCode = blockingReasonCode;
    summary = `Relay is waiting on ${targetLabel}: ${blockingReasonCode}.`;
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
