function normalizeRole(value = "") {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value = "") {
  const text = String(value || "").trim();
  return text || null;
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
  if (!relayStatus?.applicable) {
    return {
      action: "SKIP",
      reason: "NOT_APPLICABLE",
      shouldSteer: false,
    };
  }

  const relayState = String(relayStatus.status || "").trim().toUpperCase();
  if (!["WATCH", "ESCALATED"].includes(relayState)) {
    return {
      action: "SKIP",
      reason: relayState || "NORMAL",
      shouldSteer: false,
    };
  }

  const activeRunCount = Array.isArray(activeRuns) ? activeRuns.length : 0;
  if (activeRunCount > 0) {
    if (String(stallScanStatus || "").trim().toUpperCase() === "STALL") {
      return {
        action: "REPORT_STALLED_ACTIVE_RUN",
        reason: relayStatus.reason_code || "ACTIVE_RUN_STALLED",
        shouldSteer: false,
      };
    }
    return {
      action: "WAIT_ACTIVE_RUN",
      reason: relayStatus.reason_code || "ACTIVE_RUN_PRESENT",
      shouldSteer: false,
    };
  }

  if (relayState === "WATCH" && !allowWatchSteer) {
    return {
      action: "WATCH_ONLY",
      reason: relayStatus.reason_code || "WATCH_THRESHOLD_ONLY",
      shouldSteer: false,
    };
  }

  return {
    action: "STEER",
    reason: relayStatus.reason_code || (relayState === "ESCALATED" ? "STALE_ROUTE" : "WATCH_ROUTE"),
    shouldSteer: true,
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
    `active_runs=${Array.isArray(activeRuns) ? activeRuns.length : 0}`,
    `stall_scan=${String(stallScanStatus || "UNKNOWN").trim().toUpperCase() || "UNKNOWN"}`,
  ];
  return parts.join(" | ");
}
