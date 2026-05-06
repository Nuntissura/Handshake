function normalizeToken(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizedPendingControlQueue(session = {}) {
  if (Array.isArray(session?.pending_control_queue)) {
    return session.pending_control_queue.filter((entry) => entry && typeof entry === "object");
  }
  if (session?.next_queued_control_request && typeof session.next_queued_control_request === "object") {
    return [session.next_queued_control_request];
  }
  return [];
}

export function pendingControlQueueCount(session) {
  const explicitCount = Number(session?.pending_control_queue_count);
  if (Number.isInteger(explicitCount) && explicitCount >= 0) return explicitCount;
  return normalizedPendingControlQueue(session).length;
}

export function nextQueuedControlRequest(session) {
  if (session?.next_queued_control_request && typeof session.next_queued_control_request === "object") {
    return session.next_queued_control_request;
  }
  const queue = normalizedPendingControlQueue(session);
  return queue.length > 0 ? queue[0] : null;
}

export function steerActionForSession(session) {
  if (!session) return "START_SESSION";
  const threadId = String(session.session_thread_id || "").trim();
  if (!threadId) return "START_SESSION";
  const runtimeState = normalizeToken(session.runtime_state);
  if (runtimeState === "CLOSED") return "START_SESSION";
  return "SEND_PROMPT";
}

export function shouldDirectSteerReadySession({
  action = "",
  sendNow = false,
  nextActor = "",
  governedRuntimeState = "",
  queuedControlCount = 0,
  relayEscalation = null,
} = {}) {
  if (normalizeToken(action) !== "SEND_PROMPT") return false;
  if (sendNow) return false;
  if (normalizeToken(nextActor) === "ACTIVATION_MANAGER") return false;
  if (normalizeToken(governedRuntimeState) !== "READY") return false;
  if (Number(queuedControlCount || 0) !== 0) return false;

  const relayStatus = normalizeToken(relayEscalation?.status);
  if (relayStatus === "ESCALATED") return true;

  const reasonCode = normalizeToken(relayEscalation?.reason_code);
  const pendingNotifications = Number(
    relayEscalation?.pending_notification_count
    || relayEscalation?.metrics?.pending_notification_count
    || 0,
  );
  return relayStatus === "WATCH"
    && pendingNotifications > 0
    && ["SESSION_STARTED_AWAITING_RECEIPT", "ROUTE_OPENED_AWAITING_RECEIPT"].includes(reasonCode);
}
