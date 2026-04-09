function normalizeRole(value = "") {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value = "") {
  const text = String(value || "").trim();
  return text || null;
}

function parseNonNegativeInteger(value, fallback = 0) {
  const parsed = Number.parseInt(String(value ?? "").trim(), 10);
  if (!Number.isInteger(parsed) || parsed < 0) return fallback;
  return parsed;
}

export function relayEscalationCycleBudget(relayStatus = null) {
  const currentCycle = parseNonNegativeInteger(relayStatus?.metrics?.current_relay_escalation_cycle, 0);
  const maxCycleRaw = parseNonNegativeInteger(relayStatus?.metrics?.max_relay_escalation_cycles, 1);
  const maxCycle = Math.max(1, maxCycleRaw);
  return {
    currentCycle,
    maxCycle,
    exhausted: currentCycle >= maxCycle,
    remainingCycles: Math.max(0, maxCycle - currentCycle),
  };
}

export function activeRunsForTarget(activeRuns = [], {
  wpId = "",
  role = "",
  session = null,
} = {}) {
  const normalizedRole = normalizeRole(role);
  const normalizedSession = normalizeSession(session);
  return (Array.isArray(activeRuns) ? activeRuns : []).filter((run) => {
    if (String(run?.wp_id || "").trim() !== String(wpId || "").trim()) return false;
    if (normalizeRole(run?.role) !== normalizedRole) return false;
    if (!normalizedSession) return true;
    const sessionKey = normalizeSession(run?.session_key);
    return sessionKey === normalizedSession
      || sessionKey === `${normalizedRole}:${String(wpId || "").trim()}`
      || normalizeSession(run?.session_id) === normalizedSession;
  });
}

export function deriveRelayWatchdogDecision({
  relayStatus = null,
  activeRuns = [],
  stallScanStatus = "UNKNOWN",
  allowWatchSteer = true,
} = {}) {
  const cycleBudget = relayEscalationCycleBudget(relayStatus);
  if (!relayStatus?.applicable) {
    return {
      action: "SKIP",
      reason: "NOT_APPLICABLE",
      shouldSteer: false,
      cycleAction: cycleBudget.currentCycle > 0 ? "RESET" : "KEEP",
      currentCycle: cycleBudget.currentCycle,
      nextCycle: cycleBudget.currentCycle > 0 ? 0 : cycleBudget.currentCycle,
      maxCycle: cycleBudget.maxCycle,
      limitReached: false,
    };
  }

  const relayState = String(relayStatus.status || "").trim().toUpperCase();
  if (!["WATCH", "ESCALATED"].includes(relayState)) {
    return {
      action: "SKIP",
      reason: relayState || "NORMAL",
      shouldSteer: false,
      cycleAction: cycleBudget.currentCycle > 0 ? "RESET" : "KEEP",
      currentCycle: cycleBudget.currentCycle,
      nextCycle: cycleBudget.currentCycle > 0 ? 0 : cycleBudget.currentCycle,
      maxCycle: cycleBudget.maxCycle,
      limitReached: false,
    };
  }

  const activeRunCount = Array.isArray(activeRuns) ? activeRuns.length : 0;
  if (activeRunCount > 0) {
    if (String(stallScanStatus || "").trim().toUpperCase() === "STALL") {
      return {
        action: "REPORT_STALLED_ACTIVE_RUN",
        reason: relayStatus.reason_code || "ACTIVE_RUN_STALLED",
        shouldSteer: false,
        cycleAction: "KEEP",
        currentCycle: cycleBudget.currentCycle,
        nextCycle: cycleBudget.currentCycle,
        maxCycle: cycleBudget.maxCycle,
        limitReached: false,
      };
    }
    return {
      action: "WAIT_ACTIVE_RUN",
      reason: relayStatus.reason_code || "ACTIVE_RUN_PRESENT",
      shouldSteer: false,
      cycleAction: "KEEP",
      currentCycle: cycleBudget.currentCycle,
      nextCycle: cycleBudget.currentCycle,
      maxCycle: cycleBudget.maxCycle,
      limitReached: false,
    };
  }

  if (relayState === "WATCH" && !allowWatchSteer) {
    return {
      action: "WATCH_ONLY",
      reason: relayStatus.reason_code || "WATCH_THRESHOLD_ONLY",
      shouldSteer: false,
      cycleAction: "KEEP",
      currentCycle: cycleBudget.currentCycle,
      nextCycle: cycleBudget.currentCycle,
      maxCycle: cycleBudget.maxCycle,
      limitReached: false,
    };
  }

  if (cycleBudget.exhausted) {
    return {
      action: "ESCALATE_RELAY_LIMIT",
      reason: "MAX_RELAY_ESCALATION_CYCLES_REACHED",
      shouldSteer: false,
      cycleAction: "KEEP",
      currentCycle: cycleBudget.currentCycle,
      nextCycle: cycleBudget.currentCycle,
      maxCycle: cycleBudget.maxCycle,
      limitReached: true,
    };
  }

  return {
    action: "STEER",
    reason: relayStatus.reason_code || (relayState === "ESCALATED" ? "STALE_ROUTE" : "WATCH_ROUTE"),
    shouldSteer: true,
    cycleAction: "INCREMENT",
    currentCycle: cycleBudget.currentCycle,
    nextCycle: cycleBudget.currentCycle + 1,
    maxCycle: cycleBudget.maxCycle,
    limitReached: false,
  };
}

export function buildRelayWatchdogSummary({
  wpId = "",
  relayStatus = null,
  decision = null,
  activeRuns = [],
  stallScanStatus = "UNKNOWN",
} = {}) {
  const targetRole = normalizeRole(relayStatus?.target_role) || "NONE";
  const targetSession = normalizeSession(relayStatus?.target_session);
  const target = targetSession
    ? (targetSession.startsWith(`${targetRole}:`) ? targetSession : `${targetRole}:${targetSession}`)
    : targetRole;
  const relayState = String(relayStatus?.status || "NOT_APPLICABLE").trim().toUpperCase();
  const decisionAction = String(decision?.action || "SKIP").trim().toUpperCase();
  const reason = String(decision?.reason || relayStatus?.reason_code || "UNKNOWN").trim();
  const parts = [
    "RELAY_WATCHDOG",
    `wp=${String(wpId || "").trim() || "<missing>"}`,
    `relay=${relayState}`,
    `target=${target}`,
    `decision=${decisionAction}`,
    `reason=${reason || "UNKNOWN"}`,
    `cycle=${Number.isInteger(decision?.currentCycle) ? decision.currentCycle : 0}/${Number.isInteger(decision?.maxCycle) ? decision.maxCycle : 1}`,
    ...(Number.isInteger(decision?.nextCycle) && decision?.nextCycle !== decision?.currentCycle
      ? [`next_cycle=${decision.nextCycle}/${Number.isInteger(decision?.maxCycle) ? decision.maxCycle : 1}`]
      : []),
    `limit_reached=${decision?.limitReached ? "YES" : "NO"}`,
    `active_runs=${Array.isArray(activeRuns) ? activeRuns.length : 0}`,
    `stall_scan=${String(stallScanStatus || "UNKNOWN").trim().toUpperCase() || "UNKNOWN"}`,
  ];
  return parts.join(" | ");
}
