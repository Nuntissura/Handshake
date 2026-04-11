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
  outputFreshnessStatus = "UNKNOWN",
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
    const freshnessStatus = String(outputFreshnessStatus || "").trim().toUpperCase();
    if (String(stallScanStatus || "").trim().toUpperCase() === "STALL") {
      if (["FRESH", "RECENT"].includes(freshnessStatus)) {
        return {
          action: "WAIT_ACTIVE_RUN",
          reason: "OUTPUT_PROGRESS_RECENT",
          shouldSteer: false,
          cycleAction: "KEEP",
          currentCycle: cycleBudget.currentCycle,
          nextCycle: cycleBudget.currentCycle,
          maxCycle: cycleBudget.maxCycle,
          limitReached: false,
        };
      }
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
  laneVerdict = null,
  activeRuns = [],
  stallScanStatus = "UNKNOWN",
  outputFreshnessStatus = "UNKNOWN",
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
    `output_freshness=${String(outputFreshnessStatus || "UNKNOWN").trim().toUpperCase() || "UNKNOWN"}`,
    `lane_verdict=${String(laneVerdict?.verdict || "UNKNOWN").trim().toUpperCase() || "UNKNOWN"}`,
  ];
  return parts.join(" | ");
}

function classifyWaitVerdict({
  waitingOn = "",
  reasonCode = "",
  targetRole = "",
} = {}) {
  const combined = [
    String(waitingOn || "").trim(),
    String(reasonCode || "").trim(),
    String(targetRole || "").trim(),
  ].join(" ").toUpperCase();
  if (!combined) return null;
  if (/HUMAN|OPERATOR|APPROVAL|SIGNATURE/.test(combined)) return "WAITING_ON_HUMAN_APPROVAL";
  if (/DEPENDENCY|BLOCKED/.test(combined)) return "WAITING_ON_DEPENDENCY";
  if (/VALIDATOR|REVIEW/.test(combined)) return "WAITING_ON_REVIEW";
  return null;
}

export function deriveRelayLaneVerdict({
  relayStatus = null,
  decision = null,
  activeRuns = [],
  stallScanStatus = "UNKNOWN",
  outputFreshnessStatus = "UNKNOWN",
  waitingOn = "",
} = {}) {
  const relayApplicable = relayStatus?.applicable === true;
  const activeRunCount = Array.isArray(activeRuns) ? activeRuns.length : 0;
  const decisionAction = String(decision?.action || "SKIP").trim().toUpperCase() || "SKIP";
  const relayState = String(relayStatus?.status || "NOT_APPLICABLE").trim().toUpperCase() || "NOT_APPLICABLE";
  const decisionReasonCode = String(decision?.reason || "").trim().toUpperCase();
  const relayReasonCode = String(relayStatus?.reason_code || "").trim().toUpperCase();
  const reasonCode = (decisionReasonCode && !["NORMAL", "SKIP"].includes(decisionReasonCode))
    ? decisionReasonCode
    : (relayReasonCode || decisionReasonCode || "UNKNOWN");
  const waitVerdict = classifyWaitVerdict({
    waitingOn,
    reasonCode,
    targetRole: relayStatus?.target_role,
  });

  let verdict = "ACTIVE_HEALTHY";
  let pokeTarget = "NONE";
  let workerInterruptPolicy = "FORBIDDEN";

  if (!relayApplicable) {
    verdict = "NOT_APPLICABLE";
  } else if (activeRunCount > 0) {
    if (decisionAction === "REPORT_STALLED_ACTIVE_RUN") {
      verdict = decision?.limitReached ? "ACTIVE_RUN_STALLED_ESCALATE" : "ACTIVE_RUN_STALLED_RECOVERABLE";
      pokeTarget = "ROUTE_MANAGER";
      workerInterruptPolicy = decision?.limitReached ? "ROUTE_MANAGER_FIRST" : "BOUNDED_AFTER_ROUTE_REPAIR";
    } else if (["RECENT", "FRESH"].includes(String(outputFreshnessStatus || "").trim().toUpperCase())) {
      verdict = "QUIET_BUT_PROGRESSING";
    } else if (waitVerdict) {
      verdict = waitVerdict;
    }
  } else {
    if (decisionAction === "ESCALATE_RELAY_LIMIT") {
      verdict = "RELAY_BUDGET_EXHAUSTED";
      pokeTarget = "ROUTE_MANAGER";
      workerInterruptPolicy = "ROUTE_MANAGER_FIRST";
    } else if (["WATCH", "ESCALATED"].includes(relayState) || ["STEER", "WATCH_ONLY"].includes(decisionAction)) {
      verdict = "ROUTE_STALE_NO_ACTIVE_RUN";
      pokeTarget = "ROUTE_MANAGER";
      workerInterruptPolicy = "ROUTE_MANAGER_FIRST";
    } else if (waitVerdict) {
      verdict = waitVerdict;
    }
  }

  return {
    verdict,
    reasonCode,
    pokeTarget,
    workerInterruptPolicy,
    evidence: {
      relayState,
      decisionAction,
      targetRole: normalizeRole(relayStatus?.target_role),
      targetSession: normalizeSession(relayStatus?.target_session),
      activeRunCount,
      stallScanStatus: String(stallScanStatus || "UNKNOWN").trim().toUpperCase() || "UNKNOWN",
      outputFreshnessStatus: String(outputFreshnessStatus || "UNKNOWN").trim().toUpperCase() || "UNKNOWN",
      waitingOn: String(waitingOn || "").trim() || "NONE",
    },
  };
}

export function formatRelayLaneVerdict(laneVerdict = null) {
  if (!laneVerdict) return "NONE";
  const verdict = String(laneVerdict.verdict || "UNKNOWN").trim().toUpperCase() || "UNKNOWN";
  const reasonCode = String(laneVerdict.reasonCode || "UNKNOWN").trim().toUpperCase() || "UNKNOWN";
  return `${verdict}/${reasonCode}`;
}

export function deriveRelayWatchdogRestartDecision({
  decision = null,
  allowRestart = false,
  freshness = null,
} = {}) {
  const action = String(decision?.action || "").trim().toUpperCase();
  const currentCycle = parseNonNegativeInteger(decision?.currentCycle, 0);
  const maxCycle = Math.max(1, parseNonNegativeInteger(decision?.maxCycle, 1));

  if (!allowRestart) {
    return {
      action: "RESTART_DISABLED",
      shouldRestart: false,
      reason: "RESTART_DISABLED",
      currentCycle,
      nextCycle: currentCycle,
      maxCycle,
    };
  }

  if (action !== "REPORT_STALLED_ACTIVE_RUN") {
    return {
      action: "RESTART_NOT_APPLICABLE",
      shouldRestart: false,
      reason: action || "NOT_APPLICABLE",
      currentCycle,
      nextCycle: currentCycle,
      maxCycle,
    };
  }

  if (currentCycle >= maxCycle) {
    return {
      action: "RESTART_BUDGET_EXHAUSTED",
      shouldRestart: false,
      reason: "MAX_RELAY_ESCALATION_CYCLES_REACHED",
      currentCycle,
      nextCycle: currentCycle,
      maxCycle,
    };
  }

  if (!freshness?.eligible) {
    return {
      action: "RESTART_BLOCKED",
      shouldRestart: false,
      reason: String(freshness?.reason || "FRESHNESS_GUARD_BLOCKED").trim() || "FRESHNESS_GUARD_BLOCKED",
      currentCycle,
      nextCycle: currentCycle,
      maxCycle,
    };
  }

  return {
    action: "CANCEL_AND_RESTEER",
    shouldRestart: true,
    reason: String(freshness.reason || "STALE_ACTIVE_RUN_CONFIRMED").trim() || "STALE_ACTIVE_RUN_CONFIRMED",
    currentCycle,
    nextCycle: currentCycle + 1,
    maxCycle,
  };
}

export function buildRelayRepairSignal({
  wpId = "",
  relayStatus = null,
  decision = null,
  stallScanStatus = "UNKNOWN",
} = {}) {
  const action = String(decision?.action || "").trim().toUpperCase();
  if (!["REPORT_STALLED_ACTIVE_RUN", "ESCALATE_RELAY_LIMIT"].includes(action)) {
    return null;
  }

  const targetRole = normalizeRole(relayStatus?.target_role) || "UNKNOWN";
  const targetSession = normalizeSession(relayStatus?.target_session);
  const targetLabel = targetSession
    ? (targetSession.startsWith(`${targetRole}:`) ? targetSession : `${targetRole}:${targetSession}`)
    : targetRole;
  const reason = String(decision?.reason || relayStatus?.reason_code || "UNKNOWN").trim() || "UNKNOWN";
  const cycle = `${Number.isInteger(decision?.currentCycle) ? decision.currentCycle : 0}/${Number.isInteger(decision?.maxCycle) ? decision.maxCycle : 1}`;
  const summary = action === "REPORT_STALLED_ACTIVE_RUN"
    ? `RELAY_WATCHDOG_REPAIR: active run for ${targetLabel} appears stalled (${reason}); stall_scan=${String(stallScanStatus || "UNKNOWN").trim().toUpperCase() || "UNKNOWN"}; bounded repair escalation is required.`
    : `RELAY_WATCHDOG_REPAIR: relay budget exhausted for ${targetLabel} after cycle=${cycle} (${reason}); automatic re-wake is halted until orchestrator repair intervenes.`;

  return {
    sourceKind: "RELAY_WATCHDOG_REPAIR",
    targetRole: "ORCHESTRATOR",
    targetSession: null,
    correlationId: [
      "relay-watchdog-repair",
      String(wpId || "").trim() || "WP-UNKNOWN",
      targetRole || "UNKNOWN",
      targetSession || "NONE",
      action,
      reason || "UNKNOWN",
    ].join(":"),
    summary,
  };
}

export function relayRepairSignalAlreadyPending(pendingNotifications = [], repairSignal = null) {
  const correlationId = normalizeSession(repairSignal?.correlationId);
  if (!correlationId) return false;
  return (Array.isArray(pendingNotifications) ? pendingNotifications : []).some((entry) =>
    normalizeRole(entry?.target_role) === "ORCHESTRATOR"
    && normalizeSession(entry?.correlation_id) === correlationId
  );
}
