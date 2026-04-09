import assert from "node:assert/strict";
import test from "node:test";

import {
  activeRunsForTarget,
  buildRelayRepairSignal,
  buildRelayWatchdogSummary,
  deriveRelayWatchdogDecision,
  deriveRelayWatchdogRestartDecision,
  relayRepairSignalAlreadyPending,
  relayEscalationCycleBudget,
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
  const summary = buildRelayWatchdogSummary({
    wpId: "WP-TEST-v1",
    relayStatus: relayStatus(),
    decision,
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
  });

  assert.match(summary, /RELAY_WATCHDOG/);
  assert.match(summary, /decision=STEER/);
  assert.match(summary, /target=CODER:WP-TEST-v1/);
  assert.match(summary, /cycle=0\/2/);
  assert.match(summary, /next_cycle=1\/2/);
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
  });

  assert.equal(repair.targetRole, "ORCHESTRATOR");
  assert.match(repair.summary, /active run/i);
  assert.match(repair.summary, /stall_scan=STALL/);
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

test("restart decision stays disabled unless explicitly allowed", () => {
  const restart = deriveRelayWatchdogRestartDecision({
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      currentCycle: 0,
      maxCycle: 2,
    },
    allowRestart: false,
    freshness: { eligible: true, reason: "STALE_ACTIVE_RUN_CONFIRMED" },
  });

  assert.equal(restart.shouldRestart, false);
  assert.equal(restart.action, "RESTART_DISABLED");
});

test("restart decision blocks when freshness guard is not satisfied", () => {
  const restart = deriveRelayWatchdogRestartDecision({
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      currentCycle: 0,
      maxCycle: 2,
    },
    allowRestart: true,
    freshness: { eligible: false, reason: "OUTPUT_RECENTLY_UPDATED" },
  });

  assert.equal(restart.shouldRestart, false);
  assert.equal(restart.action, "RESTART_BLOCKED");
  assert.equal(restart.reason, "OUTPUT_RECENTLY_UPDATED");
});

test("restart decision consumes bounded relay budget when a stalled run is confirmed", () => {
  const restart = deriveRelayWatchdogRestartDecision({
    decision: {
      action: "REPORT_STALLED_ACTIVE_RUN",
      currentCycle: 1,
      maxCycle: 3,
    },
    allowRestart: true,
    freshness: { eligible: true, reason: "STALE_ACTIVE_RUN_CONFIRMED" },
  });

  assert.equal(restart.shouldRestart, true);
  assert.equal(restart.action, "CANCEL_AND_RESTEER");
  assert.equal(restart.currentCycle, 1);
  assert.equal(restart.nextCycle, 2);
});
