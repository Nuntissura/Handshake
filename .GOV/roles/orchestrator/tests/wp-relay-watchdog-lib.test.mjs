import assert from "node:assert/strict";
import test from "node:test";

import {
  activeRunsForTarget,
  buildRelayRepairSignal,
  buildRelayWatchdogSummary,
  deriveRelayEscalationPolicy,
  deriveRelayFailureFingerprint,
  deriveRelayLaneVerdict,
  deriveRelayWatchdogDecision,
  deriveRelayWatchdogRestartDecision,
  duplicateRelayRewakeBudget,
  formatRelayLaneVerdict,
  relayRepairSignalAlreadyPending,
  relayEscalationCycleBudget,
  workerInterruptCycleBudget,
} from "../scripts/lib/wp-relay-watchdog-lib.mjs";

function relayStatus(overrides = {}) {
  return {
    applicable: true,
    status: "WATCH",
    reason_code: "PENDING_NOTIFICATION_WAITING",
    target_role: "CODER",
    target_session: "CODER:WP-TEST-v1",
    metrics: {
      current_relay_escalation_cycle: 0,
      max_relay_escalation_cycles: 2,
    },
    ...overrides,
  };
}

test("activeRunsForTarget matches the current WP and role", () => {
  const runs = [
    { wp_id: "WP-TEST-v1", role: "CODER", session_key: "CODER:WP-TEST-v1" },
    { wp_id: "WP-OTHER-v1", role: "CODER", session_key: "CODER:WP-OTHER-v1" },
    { wp_id: "WP-TEST-v1", role: "WP_VALIDATOR", session_key: "WP_VALIDATOR:WP-TEST-v1" },
  ];

  assert.deepEqual(
    activeRunsForTarget(runs, { wpId: "WP-TEST-v1", role: "CODER", session: "CODER:WP-TEST-v1" }),
    [{ wp_id: "WP-TEST-v1", role: "CODER", session_key: "CODER:WP-TEST-v1" }],
  );
});

test("watchdog skips when relay escalation is not applicable", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: { applicable: false, status: "NOT_APPLICABLE" },
    activeRuns: [],
  });

  assert.equal(decision.action, "SKIP");
  assert.equal(decision.shouldSteer, false);
});

test("relay cycle budget normalizes missing and invalid values", () => {
  assert.deepEqual(
    relayEscalationCycleBudget({ metrics: { current_relay_escalation_cycle: "2", max_relay_escalation_cycles: "4" } }),
    {
      currentCycle: 2,
      maxCycle: 4,
      exhausted: false,
      remainingCycles: 2,
    },
  );
  assert.deepEqual(
    relayEscalationCycleBudget({ metrics: { current_relay_escalation_cycle: "-1", max_relay_escalation_cycles: "0" } }),
    {
      currentCycle: 0,
      maxCycle: 1,
      exhausted: false,
      remainingCycles: 1,
    },
  );
});

test("worker interrupt budget normalizes missing and invalid values", () => {
  assert.deepEqual(
    workerInterruptCycleBudget({ current_worker_interrupt_cycle: "1", max_worker_interrupt_cycles: "2" }),
    {
      currentCycle: 1,
      maxCycle: 2,
      exhausted: false,
      remainingCycles: 1,
    },
  );
  assert.deepEqual(
    workerInterruptCycleBudget({ current_worker_interrupt_cycle: "-1", max_worker_interrupt_cycles: "-1" }),
    {
      currentCycle: 0,
      maxCycle: 1,
      exhausted: false,
      remainingCycles: 1,
    },
  );
});

test("watchdog steers a watched route when no active run exists", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus(),
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
  });

  assert.equal(decision.action, "STEER");
  assert.equal(decision.shouldSteer, true);
  assert.equal(decision.cycleAction, "INCREMENT");
  assert.equal(decision.currentCycle, 0);
  assert.equal(decision.nextCycle, 1);
});

test("duplicate rewake budget tracks identical failure fingerprints and suppresses repeated identical steers", () => {
  const staleRelay = relayStatus({
    status: "ESCALATED",
    reason_code: "PENDING_NOTIFICATION_STALE",
    metrics: {
      current_relay_escalation_cycle: 1,
      max_relay_escalation_cycles: 3,
      route_anchor_at: "2026-04-18T10:00:00Z",
      latest_notification_at: "2026-04-18T10:00:00Z",
      latest_target_receipt_at: "2026-04-18T09:55:00Z",
      latest_session_activity_at: "2026-04-18T09:54:00Z",
    },
  });
  const fingerprint = deriveRelayFailureFingerprint({
    relayStatus: staleRelay,
    decision: { action: "STEER", reason: staleRelay.reason_code },
    laneVerdict: { verdict: "ROUTE_STALE_NO_ACTIVE_RUN" },
  });
  const budget = duplicateRelayRewakeBudget({
    last_relay_failure_fingerprint: fingerprint,
    current_same_failure_rewake_count: 2,
    max_same_failure_rewake_attempts: 2,
  }, fingerprint);
  const decision = deriveRelayWatchdogDecision({
    relayStatus: staleRelay,
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
    duplicateRewakeBudget: budget,
  });

  assert.equal(budget.exhausted, true);
  assert.equal(decision.action, "SUPPRESS_DUPLICATE_REWAKE");
  assert.equal(decision.shouldSteer, false);
});

test("watchdog waits when the target role already has an active run", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus(),
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "CLEAR",
  });

  assert.equal(decision.action, "WAIT_ACTIVE_RUN");
  assert.equal(decision.shouldSteer, false);
});

test("watchdog reports stalled active runs without auto-killing them", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({ status: "ESCALATED", reason_code: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS" }),
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "STALL",
  });

  assert.equal(decision.action, "REPORT_STALLED_ACTIVE_RUN");
  assert.equal(decision.shouldSteer, false);
});

test("watchdog does not report a stalled active run when output progress is recent", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({ status: "ESCALATED", reason_code: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS" }),
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "STALL",
    outputFreshnessStatus: "RECENT",
  });

  assert.equal(decision.action, "WAIT_ACTIVE_RUN");
  assert.equal(decision.reason, "OUTPUT_PROGRESS_RECENT");
  assert.equal(decision.shouldSteer, false);
});

test("watchdog resets relay cycle state once the route is healthy again", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({
      applicable: true,
      status: "NORMAL",
      reason_code: "ROUTE_HEALTHY",
      metrics: {
        current_relay_escalation_cycle: 2,
        max_relay_escalation_cycles: 3,
      },
    }),
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
  });

  assert.equal(decision.action, "SKIP");
  assert.equal(decision.cycleAction, "RESET");
  assert.equal(decision.nextCycle, 0);
});

test("watchdog stops auto-steering once the relay cycle budget is exhausted", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({
      status: "ESCALATED",
      reason_code: "PENDING_NOTIFICATION_STALE",
      metrics: {
        current_relay_escalation_cycle: 2,
        max_relay_escalation_cycles: 2,
      },
    }),
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
  });

  assert.equal(decision.action, "ESCALATE_RELAY_LIMIT");
  assert.equal(decision.shouldSteer, false);
  assert.equal(decision.limitReached, true);
  assert.equal(decision.currentCycle, 2);
  assert.equal(decision.nextCycle, 2);
});

test("watchdog summary is compact and includes the relay decision", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus(),
    activeRuns: [],
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: relayStatus(),
    decision,
    activeRuns: [],
  });
  const summary = buildRelayWatchdogSummary({
    wpId: "WP-TEST-v1",
    relayStatus: relayStatus(),
    decision,
    laneVerdict,
    duplicateRewakeBudget: {
      currentAttempts: 1,
      maxAttempts: 2,
    },
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
    outputFreshnessStatus: "UNKNOWN",
  });

  assert.match(summary, /RELAY_WATCHDOG/);
  assert.match(summary, /decision=STEER/);
  assert.match(summary, /target=CODER:WP-TEST-v1/);
  assert.match(summary, /cycle=0\/2/);
  assert.match(summary, /next_cycle=1\/2/);
  assert.match(summary, /output_freshness=UNKNOWN/);
  assert.match(summary, /lane_verdict=ROUTE_STALE_NO_ACTIVE_RUN/);
  assert.match(summary, /worker_interrupt=0\/1/);
  assert.match(summary, /same_failure_rewake=1\/2/);
});

test("relay escalation policy records relay-cycle retries as alternate-method budgeted recovery", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus(),
    activeRuns: [],
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: relayStatus(),
    decision,
    activeRuns: [],
  });
  const policy = deriveRelayEscalationPolicy({
    relayStatus: relayStatus(),
    decision,
    laneVerdict,
  });

  assert.equal(policy.failure_class, "ROUTE_STALE_NO_ACTIVE_RUN");
  assert.equal(policy.policy_state, "AUTO_RETRY_ALLOWED");
  assert.equal(policy.next_strategy, "ALTERNATE_METHOD");
  assert.equal(policy.budget_scope, "RELAY_ESCALATION_CYCLE");
  assert.equal(policy.budget_used, 1);
  assert.equal(policy.budget_limit, 2);
});

test("relay escalation policy records queued defer when the governed lane is still active", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({ reason_code: "WAITING_ON_VALIDATOR_REVIEW" }),
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "CLEAR",
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: relayStatus({ reason_code: "WAITING_ON_VALIDATOR_REVIEW" }),
    decision,
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "CLEAR",
    outputFreshnessStatus: "RECENT",
    waitingOn: "WP_VALIDATOR review response",
  });
  const policy = deriveRelayEscalationPolicy({
    relayStatus: relayStatus({ reason_code: "WAITING_ON_VALIDATOR_REVIEW" }),
    decision,
    laneVerdict,
  });

  assert.equal(policy.policy_state, "DEFERRED");
  assert.equal(policy.next_strategy, "QUEUED_DEFER");
  assert.equal(policy.budget_scope, "NONE");
  assert.equal(policy.budget_used, 0);
  assert.equal(policy.budget_limit, 0);
});

test("relay escalation policy blocks duplicate re-wake loops until route method changes", () => {
  const policy = deriveRelayEscalationPolicy({
    relayStatus: relayStatus({ status: "ESCALATED", reason_code: "PENDING_NOTIFICATION_STALE" }),
    decision: {
      action: "SUPPRESS_DUPLICATE_REWAKE",
      reason: "SAME_FAILURE_REWAKE_BUDGET_EXHAUSTED",
      currentCycle: 1,
      maxCycle: 3,
    },
    laneVerdict: {
      verdict: "ROUTE_STALE_NO_ACTIVE_RUN",
      reasonCode: "PENDING_NOTIFICATION_STALE",
    },
    duplicateRewakeBudget: {
      currentAttempts: 2,
      maxAttempts: 2,
    },
  });

  assert.equal(policy.failure_class, "DUPLICATE_REWAKE_LOOP");
  assert.equal(policy.policy_state, "AUTO_RETRY_BLOCKED");
  assert.equal(policy.next_strategy, "ALTERNATE_METHOD");
  assert.equal(policy.budget_scope, "SAME_FAILURE_REWAKE");
  assert.equal(policy.budget_used, 2);
  assert.equal(policy.budget_limit, 2);
});

test("relay escalation policy escalates exhausted worker interrupts to alternate-model recovery", () => {
  const policy = deriveRelayEscalationPolicy({
    relayStatus: relayStatus({ status: "ESCALATED", reason_code: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS" }),
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      reason: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS",
      currentCycle: 1,
      maxCycle: 3,
    },
    laneVerdict: {
      verdict: "STALL_RETRY_LOOP",
      reasonCode: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS",
    },
    workerInterruptBudget: {
      currentCycle: 1,
      maxCycle: 1,
      exhausted: true,
    },
    restartDecision: {
      action: "RESTART_BUDGET_EXHAUSTED",
      reason: "MAX_WORKER_INTERRUPT_CYCLES_REACHED",
      shouldRestart: false,
    },
  });

  assert.equal(policy.failure_class, "WORKER_INTERRUPT_LIMIT");
  assert.equal(policy.policy_state, "AUTO_RETRY_BLOCKED");
  assert.equal(policy.next_strategy, "ALTERNATE_MODEL");
  assert.equal(policy.budget_scope, "WORKER_INTERRUPT_CYCLE");
  assert.equal(policy.budget_used, 1);
  assert.equal(policy.budget_limit, 1);
});

test("lane verdict classifies active runs with fresh output as quiet but progressing", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({ status: "WATCH", reason_code: "WAITING_ON_VALIDATOR_REVIEW" }),
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "CLEAR",
    outputFreshnessStatus: "RECENT",
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: relayStatus({ status: "WATCH", reason_code: "WAITING_ON_VALIDATOR_REVIEW" }),
    decision,
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "CLEAR",
    outputFreshnessStatus: "RECENT",
    waitingOn: "WP_VALIDATOR review response",
  });

  assert.equal(laneVerdict.verdict, "QUIET_BUT_PROGRESSING");
  assert.equal(laneVerdict.pokeTarget, "NONE");
  assert.equal(laneVerdict.workerInterruptPolicy, "FORBIDDEN");
});

test("lane verdict classifies stalled active runs as bounded route-manager repair", () => {
  const stalledRelay = relayStatus({ status: "ESCALATED", reason_code: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS" });
  const decision = deriveRelayWatchdogDecision({
    relayStatus: stalledRelay,
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "STALL",
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: stalledRelay,
    decision,
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "STALL",
    outputFreshnessStatus: "STALE",
    waitingOn: "CODER progress",
  });

  assert.equal(laneVerdict.verdict, "ACTIVE_RUN_STALLED_RECOVERABLE");
  assert.equal(laneVerdict.pokeTarget, "ROUTE_MANAGER");
  assert.equal(laneVerdict.workerInterruptPolicy, "BOUNDED_AFTER_ROUTE_REPAIR");
});

test("lane verdict classifies stalled active runs by specific stall type when available", () => {
  const stalledRelay = relayStatus({ status: "ESCALATED", reason_code: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS" });
  const decision = deriveRelayWatchdogDecision({
    relayStatus: stalledRelay,
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "STALL",
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: stalledRelay,
    decision,
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "STALL",
    stallScanSummary: "[STALL_SCAN] STALL DETECTED: STALL_RETRY_LOOP for CODER:WP-TEST-v1",
    outputFreshnessStatus: "STALE",
    waitingOn: "CODER progress",
  });

  assert.equal(laneVerdict.verdict, "STALL_RETRY_LOOP");
  assert.equal(laneVerdict.pokeTarget, "ROUTE_MANAGER");
});

test("lane verdict classifies human approval waits separately from stale routes", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({
      status: "NORMAL",
      reason_code: "WAITING_ON_HUMAN_APPROVAL",
      metrics: {
        current_relay_escalation_cycle: 0,
        max_relay_escalation_cycles: 2,
      },
    }),
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: relayStatus({
      status: "NORMAL",
      reason_code: "WAITING_ON_HUMAN_APPROVAL",
      metrics: {
        current_relay_escalation_cycle: 0,
        max_relay_escalation_cycles: 2,
      },
    }),
    decision,
    activeRuns: [],
    waitingOn: "operator approval",
  });

  assert.equal(laneVerdict.verdict, "WAITING_ON_HUMAN_APPROVAL");
  assert.equal(formatRelayLaneVerdict(laneVerdict), "WAITING_ON_HUMAN_APPROVAL/WAITING_ON_HUMAN_APPROVAL");
});

test("lane verdict classifies validator waits from relay reason codes", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({
      status: "WATCH",
      reason_code: "WAITING_ON_VALIDATOR_REVIEW",
      metrics: {
        current_relay_escalation_cycle: 0,
        max_relay_escalation_cycles: 2,
      },
    }),
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
    allowWatchSteer: false,
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: relayStatus({
      status: "WATCH",
      reason_code: "WAITING_ON_VALIDATOR_REVIEW",
      metrics: {
        current_relay_escalation_cycle: 0,
        max_relay_escalation_cycles: 2,
      },
    }),
    decision,
    activeRuns: [],
    waitingOn: "WP_VALIDATOR review response",
  });

  assert.equal(laneVerdict.verdict, "WAITING_ON_VALIDATOR");
  assert.equal(laneVerdict.workerInterruptPolicy, "ROUTE_MANAGER_FIRST");
});

test("lane verdict classifies coder waits from relay reason codes", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({
      status: "NORMAL",
      reason_code: "WAITING_ON_CODER_HANDOFF",
      metrics: {
        current_relay_escalation_cycle: 0,
        max_relay_escalation_cycles: 2,
      },
    }),
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: relayStatus({
      status: "NORMAL",
      reason_code: "WAITING_ON_CODER_HANDOFF",
      metrics: {
        current_relay_escalation_cycle: 0,
        max_relay_escalation_cycles: 2,
      },
    }),
    decision,
    activeRuns: [],
    waitingOn: "CODER_HANDOFF",
  });

  assert.equal(laneVerdict.verdict, "WAITING_ON_CODER");
});

test("watchdog builds a stalled-run repair signal for orchestrator visibility", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({ status: "ESCALATED", reason_code: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS" }),
    activeRuns: [{ role: "CODER", session_key: "CODER:WP-TEST-v1" }],
    stallScanStatus: "STALL",
  });
  const repair = buildRelayRepairSignal({
    wpId: "WP-TEST-v1",
    relayStatus: relayStatus({ status: "ESCALATED", reason_code: "SESSION_ACTIVE_NO_RECEIPT_PROGRESS" }),
    decision,
    stallScanStatus: "STALL",
    relayEscalationPolicy: {
      next_strategy: "ALTERNATE_METHOD",
      policy_state: "AUTO_RETRY_BLOCKED",
    },
  });

  assert.equal(repair.targetRole, "ORCHESTRATOR");
  assert.match(repair.summary, /active run/i);
  assert.match(repair.summary, /stall_scan=STALL/);
  assert.match(repair.summary, /next_strategy=ALTERNATE_METHOD/);
  assert.match(repair.correlationId, /REPORT_STALLED_ACTIVE_RUN/);
});

test("watchdog builds a relay-limit repair signal and suppresses duplicates", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus({
      status: "ESCALATED",
      reason_code: "PENDING_NOTIFICATION_STALE",
      metrics: {
        current_relay_escalation_cycle: 2,
        max_relay_escalation_cycles: 2,
      },
    }),
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
  });
  const repair = buildRelayRepairSignal({
    wpId: "WP-TEST-v1",
    relayStatus: relayStatus({
      status: "ESCALATED",
      reason_code: "PENDING_NOTIFICATION_STALE",
      metrics: {
        current_relay_escalation_cycle: 2,
        max_relay_escalation_cycles: 2,
      },
    }),
    decision,
  });

  assert.match(repair.summary, /budget exhausted/i);
  assert.equal(
    relayRepairSignalAlreadyPending([
      { target_role: "ORCHESTRATOR", correlation_id: repair.correlationId },
    ], repair),
    true,
  );
});

test("watchdog builds a repair signal when duplicate re-wake suppression activates", () => {
  const repair = buildRelayRepairSignal({
    wpId: "WP-TEST-v1",
    relayStatus: relayStatus({
      status: "ESCALATED",
      reason_code: "PENDING_NOTIFICATION_STALE",
    }),
    decision: {
      action: "SUPPRESS_DUPLICATE_REWAKE",
      reason: "SAME_FAILURE_REWAKE_BUDGET_EXHAUSTED",
      currentCycle: 1,
      maxCycle: 3,
    },
  });

  assert.match(repair.summary, /duplicate auto re-wake is suppressed/i);
  assert.match(repair.correlationId, /SUPPRESS_DUPLICATE_REWAKE/);
});

test("restart decision stays disabled unless explicitly allowed", () => {
  const restart = deriveRelayWatchdogRestartDecision({
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      currentCycle: 0,
      maxCycle: 2,
    },
    laneVerdict: {
      verdict: "ACTIVE_RUN_STALLED_RECOVERABLE",
      workerInterruptPolicy: "BOUNDED_AFTER_ROUTE_REPAIR",
    },
    workerInterruptBudget: {
      currentCycle: 0,
      maxCycle: 1,
      exhausted: false,
    },
    allowRestart: false,
    freshness: { eligible: true, reason: "STALE_ACTIVE_RUN_CONFIRMED" },
  });

  assert.equal(restart.shouldRestart, false);
  assert.equal(restart.action, "RESTART_DISABLED");
});

test("restart decision is blocked when lane policy does not permit worker interrupts", () => {
  const restart = deriveRelayWatchdogRestartDecision({
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      currentCycle: 0,
      maxCycle: 2,
    },
    laneVerdict: {
      verdict: "WAITING_ON_VALIDATOR",
      workerInterruptPolicy: "ROUTE_MANAGER_FIRST",
    },
    workerInterruptBudget: {
      currentCycle: 0,
      maxCycle: 1,
      exhausted: false,
    },
    allowRestart: true,
    freshness: { eligible: true, reason: "STALE_ACTIVE_RUN_CONFIRMED" },
  });

  assert.equal(restart.shouldRestart, false);
  assert.equal(restart.action, "RESTART_POLICY_FORBIDS");
  assert.equal(restart.reason, "ROUTE_MANAGER_FIRST");
});

test("restart decision blocks when freshness guard is not satisfied", () => {
  const restart = deriveRelayWatchdogRestartDecision({
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      currentCycle: 0,
      maxCycle: 2,
    },
    laneVerdict: {
      verdict: "STALL_RETRY_LOOP",
      workerInterruptPolicy: "BOUNDED_AFTER_ROUTE_REPAIR",
    },
    workerInterruptBudget: {
      currentCycle: 0,
      maxCycle: 1,
      exhausted: false,
    },
    allowRestart: true,
    freshness: { eligible: false, reason: "OUTPUT_RECENTLY_UPDATED" },
  });

  assert.equal(restart.shouldRestart, false);
  assert.equal(restart.action, "RESTART_BLOCKED");
  assert.equal(restart.reason, "OUTPUT_RECENTLY_UPDATED");
});

test("restart decision consumes bounded worker interrupt budget when a stalled run is confirmed", () => {
  const restart = deriveRelayWatchdogRestartDecision({
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      currentCycle: 1,
      maxCycle: 3,
    },
    laneVerdict: {
      verdict: "STALL_RETRY_LOOP",
      workerInterruptPolicy: "BOUNDED_AFTER_ROUTE_REPAIR",
    },
    workerInterruptBudget: {
      currentCycle: 0,
      maxCycle: 1,
      exhausted: false,
    },
    allowRestart: true,
    freshness: { eligible: true, reason: "STALE_ACTIVE_RUN_CONFIRMED" },
  });

  assert.equal(restart.shouldRestart, true);
  assert.equal(restart.action, "CANCEL_AND_RESTEER");
  assert.equal(restart.currentCycle, 0);
  assert.equal(restart.nextCycle, 1);
});

test("restart decision stops when the worker interrupt budget is exhausted", () => {
  const restart = deriveRelayWatchdogRestartDecision({
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      currentCycle: 1,
      maxCycle: 3,
    },
    laneVerdict: {
      verdict: "STALL_RETRY_LOOP",
      workerInterruptPolicy: "BOUNDED_AFTER_ROUTE_REPAIR",
    },
    workerInterruptBudget: {
      currentCycle: 1,
      maxCycle: 1,
      exhausted: true,
    },
    allowRestart: true,
    freshness: { eligible: true, reason: "STALE_ACTIVE_RUN_CONFIRMED" },
  });

  assert.equal(restart.shouldRestart, false);
  assert.equal(restart.action, "RESTART_BUDGET_EXHAUSTED");
  assert.equal(restart.reason, "MAX_WORKER_INTERRUPT_CYCLES_REACHED");
});
