import { sessionKey } from "../session/session-policy.mjs";

const AUTO_PROGRESS_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const IN_FLIGHT_RUNTIME_STATES = new Set([
  "STARTING",
  "COMMAND_RUNNING",
  "PLUGIN_REQUESTED",
  "TERMINAL_COMMAND_DISPATCHED",
]);

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeWorkflowLane(value) {
  return String(value || "").trim().toUpperCase();
}

function pendingQueueCount(session = {}) {
  const explicitCount = Number(session?.pending_control_queue_count);
  if (Number.isInteger(explicitCount) && explicitCount >= 0) return explicitCount;
  if (Array.isArray(session?.pending_control_queue)) return session.pending_control_queue.length;
  if (session?.next_queued_control_request && typeof session.next_queued_control_request === "object") return 1;
  return 0;
}

function findTargetSession(sessions = [], wpId = "", role = "") {
  const key = sessionKey(role, wpId);
  return (sessions || []).find((session) => {
    const sessionRole = normalizeRole(session?.role);
    const sessionWp = String(session?.wp_id || "").trim();
    const candidateKey = String(session?.session_key || "").trim();
    return candidateKey === key || (sessionRole === role && sessionWp === wpId);
  }) || null;
}

export function deriveNextActionFromReceipt({
  wpId = "",
  workflowLane = "",
  receiptEntry = {},
  autoRoute = {},
  registrySessions = [],
} = {}) {
  const normalizedWpId = String(wpId || receiptEntry?.wp_id || "").trim();
  const lane = normalizeWorkflowLane(workflowLane);
  if (lane !== "ORCHESTRATOR_MANAGED") {
    return { action: "NOT_APPLICABLE", status: "NOT_APPLICABLE", reason: "NON_ORCHESTRATOR_MANAGED" };
  }
  if (!autoRoute?.applicable) {
    return { action: "NOT_APPLICABLE", status: "NOT_APPLICABLE", reason: "NO_AUTO_ROUTE" };
  }

  const nextActor = normalizeRole(autoRoute.nextExpectedActor);
  if (!AUTO_PROGRESS_ROLE_VALUES.has(nextActor)) {
    return { action: "NOT_APPLICABLE", status: "NOT_APPLICABLE", reason: "NO_GOVERNED_NEXT_ACTOR", next_actor: nextActor };
  }
  if (nextActor === normalizeRole(receiptEntry?.actor_role)) {
    return { action: "SKIP", status: "SKIPPED", reason: "NEXT_ACTOR_IS_CURRENT_ACTOR", next_actor: nextActor };
  }

  const targetSession = findTargetSession(registrySessions, normalizedWpId, nextActor);
  const queueCount = pendingQueueCount(targetSession);
  if (queueCount > 0) {
    return {
      action: "SUPPRESS_DUPLICATE",
      status: "SKIPPED",
      reason: "AUTO_RELAY_ALREADY_QUEUED",
      next_actor: nextActor,
      queue_depth: queueCount,
      session_key: targetSession?.session_key || sessionKey(nextActor, normalizedWpId),
    };
  }

  const runtimeState = normalizeRole(targetSession?.runtime_state);
  const lastCommandStatus = normalizeRole(targetSession?.last_command_status);
  if (IN_FLIGHT_RUNTIME_STATES.has(runtimeState) || lastCommandStatus === "RUNNING") {
    return {
      action: "SUPPRESS_DUPLICATE",
      status: "SKIPPED",
      reason: "AUTO_RELAY_COMMAND_IN_FLIGHT",
      next_actor: nextActor,
      runtime_state: runtimeState,
      session_key: targetSession?.session_key || sessionKey(nextActor, normalizedWpId),
    };
  }

  return {
    action: "DISPATCH",
    status: "DISPATCH",
    reason: "AUTO_RELAY_DISPATCHABLE",
    next_actor: nextActor,
    session_key: targetSession?.session_key || sessionKey(nextActor, normalizedWpId),
  };
}
