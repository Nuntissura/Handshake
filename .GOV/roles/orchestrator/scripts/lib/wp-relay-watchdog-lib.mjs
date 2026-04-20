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

function normalizeNullableToken(value = "") {
  const text = String(value || "").trim();
  return text || "NONE";
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

export function workerInterruptCycleBudget(runtimeStatus = null) {
  const currentCycle = parseNonNegativeInteger(runtimeStatus?.current_worker_interrupt_cycle, 0);
  const maxCycle = parseNonNegativeInteger(runtimeStatus?.max_worker_interrupt_cycles, 1);
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

export function deriveRelayFailureFingerprint({
  relayStatus = null,
  decision = null,
  laneVerdict = null,
} = {}) {
  if (relayStatus?.applicable !== true) return null;
  const action = String(decision?.action || "").trim().toUpperCase();
  if (!["STEER", "ESCALATE_RELAY_LIMIT", "REPORT_STALLED_ACTIVE_RUN", "SUPPRESS_DUPLICATE_REWAKE"].includes(action)) {
    return null;
  }
  const metrics = relayStatus?.metrics || {};
  return [
    action,
    normalizeNullableToken(relayStatus?.status),
    normalizeNullableToken(decision?.reason || relayStatus?.reason_code),
    normalizeNullableToken(laneVerdict?.verdict),
    normalizeNullableToken(relayStatus?.target_role),
    normalizeNullableToken(relayStatus?.target_session),
    normalizeNullableToken(metrics.route_anchor_at),
    normalizeNullableToken(metrics.latest_notification_at),
    normalizeNullableToken(metrics.latest_target_receipt_at),
    normalizeNullableToken(metrics.latest_session_activity_at),
  ].join("|");
}

export function duplicateRelayRewakeBudget(runtimeStatus = null, failureFingerprint = null) {
  const normalizedFingerprint = String(failureFingerprint || "").trim();
  const maxAttempts = Math.max(
    1,
    parseNonNegativeInteger(runtimeStatus?.max_same_failure_rewake_attempts, 2),
  );
  const currentAttempts = normalizedFingerprint
    && String(runtimeStatus?.last_relay_failure_fingerprint || "").trim() === normalizedFingerprint
    ? parseNonNegativeInteger(runtimeStatus?.current_same_failure_rewake_count, 0)
    : 0;
  return {
    failureFingerprint: normalizedFingerprint || null,
    currentAttempts,
    maxAttempts,
    exhausted: Boolean(normalizedFingerprint) && currentAttempts >= maxAttempts,
    remainingAttempts: Math.max(0, maxAttempts - currentAttempts),
  };
}

export function deriveRelayWatchdogDecision({
  relayStatus = null,
  activeRuns = [],
  stallScanStatus = "UNKNOWN",
  outputFreshnessStatus = "UNKNOWN",
  allowWatchSteer = true,
  duplicateRewakeBudget = null,
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

  const rewakeBudget = duplicateRewakeBudget || duplicateRelayRewakeBudget();
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

  if (rewakeBudget.exhausted) {
    return {
      action: "SUPPRESS_DUPLICATE_REWAKE",
      reason: "SAME_FAILURE_REWAKE_BUDGET_EXHAUSTED",
      shouldSteer: false,
      cycleAction: "KEEP",
      currentCycle: cycleBudget.currentCycle,
      nextCycle: cycleBudget.currentCycle,
      maxCycle: cycleBudget.maxCycle,
      limitReached: false,
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
  workerInterruptBudget = null,
  duplicateRewakeBudget = null,
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
    `worker_interrupt=${Number.isInteger(workerInterruptBudget?.currentCycle) ? workerInterruptBudget.currentCycle : 0}/${Number.isInteger(workerInterruptBudget?.maxCycle) ? workerInterruptBudget.maxCycle : 1}`,
    `same_failure_rewake=${Number.isInteger(duplicateRewakeBudget?.currentAttempts) ? duplicateRewakeBudget.currentAttempts : 0}/${Number.isInteger(duplicateRewakeBudget?.maxAttempts) ? duplicateRewakeBudget.maxAttempts : 2}`,
  ];
  return parts.join(" | ");
}

function classifyWaitVerdict({
  waitingOn = "",
  reasonCode = "",
} = {}) {
  const combined = [
    String(waitingOn || "").trim(),
    String(reasonCode || "").trim(),
  ].join(" ").toUpperCase();
  if (!combined) return null;
  if (/HUMAN|OPERATOR|APPROVAL|SIGNATURE/.test(combined)) return "WAITING_ON_HUMAN_APPROVAL";
  if (/DEPENDENCY|BLOCKED/.test(combined)) return "WAITING_ON_DEPENDENCY";
  if (/ORCHESTRATOR|CHECKPOINT/.test(combined)) return "WAITING_ON_ORCHESTRATOR_CHECKPOINT";
  if (/VALIDATOR|REVIEW|FINAL_REVIEW|INTEGRATION_VALIDATOR/.test(combined)) return "WAITING_ON_VALIDATOR";
  if (/CODER|HANDOFF|REPAIR|INTENT/.test(combined)) return "WAITING_ON_CODER";
  return null;
}

function extractStallVerdict(stallScanStatus = "", stallScanSummary = "") {
  if (String(stallScanStatus || "").trim().toUpperCase() !== "STALL") return null;
  const summary = String(stallScanSummary || "").trim().toUpperCase();
  if (summary.includes("STALL_RETRY_LOOP")) return "STALL_RETRY_LOOP";
  if (summary.includes("STALL_COMMAND_LOOP")) return "STALL_COMMAND_LOOP";
  if (summary.includes("STALL_REPEATED_ERROR")) return "STALL_REPEATED_ERROR";
  if (summary.includes("STALL_NO_PROGRESS")) return "STALL_NO_PROGRESS";
  return null;
}

export function deriveRelayLaneVerdict({
  relayStatus = null,
  decision = null,
  activeRuns = [],
  stallScanStatus = "UNKNOWN",
  stallScanSummary = "",
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
  });

  let verdict = "ACTIVE_HEALTHY";
  let pokeTarget = "NONE";
  let workerInterruptPolicy = "FORBIDDEN";

  if (!relayApplicable) {
    verdict = "NOT_APPLICABLE";
  } else if (activeRunCount > 0) {
    if (decisionAction === "REPORT_STALLED_ACTIVE_RUN") {
      verdict = extractStallVerdict(stallScanStatus, stallScanSummary)
        || (decision?.limitReached ? "ACTIVE_RUN_STALLED_ESCALATE" : "ACTIVE_RUN_STALLED_RECOVERABLE");
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
    } else if (waitVerdict && !reasonCode.startsWith("ROUTE_STALE")) {
      verdict = waitVerdict;
      if (["STEER", "WATCH_ONLY"].includes(decisionAction) || ["WATCH", "ESCALATED"].includes(relayState)) {
        pokeTarget = "ROUTE_MANAGER";
        workerInterruptPolicy = "ROUTE_MANAGER_FIRST";
      }
    } else if (["WATCH", "ESCALATED"].includes(relayState) || ["STEER", "WATCH_ONLY"].includes(decisionAction)) {
      verdict = "ROUTE_STALE_NO_ACTIVE_RUN";
      pokeTarget = "ROUTE_MANAGER";
      workerInterruptPolicy = "ROUTE_MANAGER_FIRST";
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

function stringifyRelayBudget(scope = "NONE", used = 0, limit = 0) {
  if (String(scope || "").trim().toUpperCase() === "NONE") return "budget=NONE";
  return `budget=${scope} ${used}/${limit}`;
}

export function deriveRelayEscalationPolicy({
  relayStatus = null,
  decision = null,
  laneVerdict = null,
  duplicateRewakeBudget = null,
  workerInterruptBudget = null,
  restartDecision = null,
  updatedAt = null,
} = {}) {
  if (relayStatus?.applicable !== true) return null;

  const timestamp = String(updatedAt || new Date().toISOString()).trim() || new Date().toISOString();
  const decisionAction = String(decision?.action || "").trim().toUpperCase();
  const laneClass = String(laneVerdict?.verdict || "").trim().toUpperCase();
  const reasonCode = String(
    restartDecision?.reason
    || decision?.reason
    || laneVerdict?.reasonCode
    || relayStatus?.reason_code
    || "UNKNOWN",
  ).trim().toUpperCase() || "UNKNOWN";

  const buildPolicy = ({
    failureClass = "UNKNOWN",
    policyState = "AUTO_RETRY_BLOCKED",
    nextStrategy = "HUMAN_STOP",
    budgetScope = "NONE",
    budgetUsed = 0,
    budgetLimit = 0,
    summary = "",
  } = {}) => ({
    source_surface: "RELAY_WATCHDOG",
    failure_class: failureClass,
    policy_state: policyState,
    next_strategy: nextStrategy,
    reason_code: reasonCode,
    budget_scope: budgetScope,
    budget_used: budgetUsed,
    budget_limit: budgetLimit,
    summary,
    updated_at: timestamp,
  });

  if (decisionAction === "WAIT_ACTIVE_RUN") {
    const failureClass = laneClass && laneClass !== "ACTIVE_HEALTHY"
      ? laneClass
      : "ACTIVE_RUN_PRESENT";
    return buildPolicy({
      failureClass,
      policyState: "DEFERRED",
      nextStrategy: "QUEUED_DEFER",
      summary: `${failureClass}: automation deferred while the governed lane is still active; next_strategy=QUEUED_DEFER; ${stringifyRelayBudget("NONE")}.`,
    });
  }

  if (decisionAction === "WATCH_ONLY") {
    const failureClass = laneClass && laneClass !== "ACTIVE_HEALTHY"
      ? laneClass
      : "WATCH_THRESHOLD_ONLY";
    return buildPolicy({
      failureClass,
      policyState: "DEFERRED",
      nextStrategy: "QUEUED_DEFER",
      summary: `${failureClass}: relay remains in watch-only mode; next_strategy=QUEUED_DEFER; ${stringifyRelayBudget("NONE")}.`,
    });
  }

  if (decisionAction === "STEER") {
    const budgetUsed = parseNonNegativeInteger(decision?.nextCycle, parseNonNegativeInteger(decision?.currentCycle, 0));
    const budgetLimit = Math.max(1, parseNonNegativeInteger(decision?.maxCycle, 1));
    const failureClass = laneClass && laneClass !== "ACTIVE_HEALTHY"
      ? laneClass
      : "ROUTE_STALE_NO_ACTIVE_RUN";
    return buildPolicy({
      failureClass,
      policyState: "AUTO_RETRY_ALLOWED",
      nextStrategy: "ALTERNATE_METHOD",
      budgetScope: "RELAY_ESCALATION_CYCLE",
      budgetUsed,
      budgetLimit,
      summary: `${failureClass}: automatic re-wake is still allowed, but unchanged failure now spends relay budget and must shift to ALTERNATE_METHOD when exhausted; ${stringifyRelayBudget("RELAY_ESCALATION_CYCLE", budgetUsed, budgetLimit)}.`,
    });
  }

  if (decisionAction === "SUPPRESS_DUPLICATE_REWAKE") {
    const budgetUsed = parseNonNegativeInteger(
      duplicateRewakeBudget?.currentAttempts,
      parseNonNegativeInteger(duplicateRewakeBudget?.maxAttempts, 0),
    );
    const budgetLimit = Math.max(1, parseNonNegativeInteger(duplicateRewakeBudget?.maxAttempts, 1));
    return buildPolicy({
      failureClass: "DUPLICATE_REWAKE_LOOP",
      policyState: "AUTO_RETRY_BLOCKED",
      nextStrategy: "ALTERNATE_METHOD",
      budgetScope: "SAME_FAILURE_REWAKE",
      budgetUsed,
      budgetLimit,
      summary: `DUPLICATE_REWAKE_LOOP: unchanged route failure exhausted the same-failure re-wake budget; automation is blocked until route repair changes method; ${stringifyRelayBudget("SAME_FAILURE_REWAKE", budgetUsed, budgetLimit)}.`,
    });
  }

  if (decisionAction === "ESCALATE_RELAY_LIMIT") {
    const budgetUsed = parseNonNegativeInteger(decision?.currentCycle, 0);
    const budgetLimit = Math.max(1, parseNonNegativeInteger(decision?.maxCycle, 1));
    return buildPolicy({
      failureClass: "RELAY_ESCALATION_LIMIT",
      policyState: "AUTO_RETRY_BLOCKED",
      nextStrategy: "HUMAN_STOP",
      budgetScope: "RELAY_ESCALATION_CYCLE",
      budgetUsed,
      budgetLimit,
      summary: `RELAY_ESCALATION_LIMIT: automatic re-wake exhausted the relay-cycle budget; stop same-method retries and escalate to human repair; ${stringifyRelayBudget("RELAY_ESCALATION_CYCLE", budgetUsed, budgetLimit)}.`,
    });
  }

  if (decisionAction === "REPORT_STALLED_ACTIVE_RUN") {
    const restartAction = String(restartDecision?.action || "").trim().toUpperCase();
    const baseFailureClass = laneClass.startsWith("STALL_")
      ? laneClass
      : (laneClass && laneClass !== "ACTIVE_HEALTHY" ? laneClass : "ACTIVE_RUN_STALL");
    const budgetUsed = restartDecision?.shouldRestart
      ? parseNonNegativeInteger(restartDecision?.nextCycle, parseNonNegativeInteger(workerInterruptBudget?.currentCycle, 0))
      : parseNonNegativeInteger(workerInterruptBudget?.currentCycle, 0);
    const budgetLimit = Math.max(1, parseNonNegativeInteger(workerInterruptBudget?.maxCycle, 1));

    if (restartDecision?.shouldRestart) {
      return buildPolicy({
        failureClass: baseFailureClass,
        policyState: "AUTO_RETRY_ALLOWED",
        nextStrategy: "ALTERNATE_METHOD",
        budgetScope: "WORKER_INTERRUPT_CYCLE",
        budgetUsed,
        budgetLimit,
        summary: `${baseFailureClass}: bounded cancel-and-resteer is still allowed for this stalled active run; if the interrupt budget is exhausted, the next shift must leave the current execution method; ${stringifyRelayBudget("WORKER_INTERRUPT_CYCLE", budgetUsed, budgetLimit)}.`,
      });
    }

    if (restartAction === "RESTART_BUDGET_EXHAUSTED") {
      return buildPolicy({
        failureClass: "WORKER_INTERRUPT_LIMIT",
        policyState: "AUTO_RETRY_BLOCKED",
        nextStrategy: "ALTERNATE_MODEL",
        budgetScope: "WORKER_INTERRUPT_CYCLE",
        budgetUsed,
        budgetLimit,
        summary: `WORKER_INTERRUPT_LIMIT: bounded cancel-and-resteer attempts are spent for this stalled run; the next recovery must shift to ALTERNATE_MODEL instead of another same-model restart; ${stringifyRelayBudget("WORKER_INTERRUPT_CYCLE", budgetUsed, budgetLimit)}.`,
      });
    }

    return buildPolicy({
      failureClass: baseFailureClass,
      policyState: "AUTO_RETRY_BLOCKED",
      nextStrategy: "ALTERNATE_METHOD",
      budgetScope: "WORKER_INTERRUPT_CYCLE",
      budgetUsed,
      budgetLimit,
      summary: `${baseFailureClass}: active-run automation is blocked until route repair changes method for the stalled lane; ${stringifyRelayBudget("WORKER_INTERRUPT_CYCLE", budgetUsed, budgetLimit)}.`,
    });
  }

  return null;
}

export function deriveRelayWatchdogRestartDecision({
  decision = null,
  laneVerdict = null,
  workerInterruptBudget = null,
  allowRestart = false,
  freshness = null,
} = {}) {
  const action = String(decision?.action || "").trim().toUpperCase();
  const budget = workerInterruptBudget || workerInterruptCycleBudget();
  const currentCycle = parseNonNegativeInteger(budget.currentCycle, 0);
  const maxCycle = parseNonNegativeInteger(budget.maxCycle, 1);
  const interruptPolicy = String(laneVerdict?.workerInterruptPolicy || "").trim().toUpperCase();
  const verdict = String(laneVerdict?.verdict || "").trim().toUpperCase();

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

  if (interruptPolicy !== "BOUNDED_AFTER_ROUTE_REPAIR") {
    return {
      action: "RESTART_POLICY_FORBIDS",
      shouldRestart: false,
      reason: interruptPolicy || "WORKER_INTERRUPT_FORBIDDEN",
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

  if (!(verdict.startsWith("STALL_") || verdict.startsWith("ACTIVE_RUN_STALLED"))) {
    return {
      action: "RESTART_NOT_APPLICABLE",
      shouldRestart: false,
      reason: verdict || "RESTART_VERDICT_NOT_STALLED",
      currentCycle,
      nextCycle: currentCycle,
      maxCycle,
    };
  }

  if (budget.exhausted) {
    return {
      action: "RESTART_BUDGET_EXHAUSTED",
      shouldRestart: false,
      reason: "MAX_WORKER_INTERRUPT_CYCLES_REACHED",
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
  relayEscalationPolicy = null,
} = {}) {
  const action = String(decision?.action || "").trim().toUpperCase();
  if (!["REPORT_STALLED_ACTIVE_RUN", "ESCALATE_RELAY_LIMIT", "SUPPRESS_DUPLICATE_REWAKE"].includes(action)) {
    return null;
  }

  const targetRole = normalizeRole(relayStatus?.target_role) || "UNKNOWN";
  const targetSession = normalizeSession(relayStatus?.target_session);
  const targetLabel = targetSession
    ? (targetSession.startsWith(`${targetRole}:`) ? targetSession : `${targetRole}:${targetSession}`)
    : targetRole;
  const reason = String(decision?.reason || relayStatus?.reason_code || "UNKNOWN").trim() || "UNKNOWN";
  const cycle = `${Number.isInteger(decision?.currentCycle) ? decision.currentCycle : 0}/${Number.isInteger(decision?.maxCycle) ? decision.maxCycle : 1}`;
  const policySuffix = relayEscalationPolicy?.next_strategy
    ? ` next_strategy=${relayEscalationPolicy.next_strategy}; policy_state=${relayEscalationPolicy.policy_state || "UNKNOWN"}.`
    : "";
  const summary = action === "REPORT_STALLED_ACTIVE_RUN"
    ? `RELAY_WATCHDOG_REPAIR: active run for ${targetLabel} appears stalled (${reason}); stall_scan=${String(stallScanStatus || "UNKNOWN").trim().toUpperCase() || "UNKNOWN"}; bounded repair escalation is required.${policySuffix}`
    : action === "ESCALATE_RELAY_LIMIT"
      ? `RELAY_WATCHDOG_REPAIR: relay budget exhausted for ${targetLabel} after cycle=${cycle} (${reason}); automatic re-wake is halted until orchestrator repair intervenes.${policySuffix}`
      : `RELAY_WATCHDOG_REPAIR: repeated identical route failure persisted for ${targetLabel} (${reason}); duplicate auto re-wake is suppressed until orchestrator repair changes the route state.${policySuffix}`;

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
