import assert from "node:assert/strict";
import test from "node:test";

import {
  activeRunsForTarget,
  buildRelayWatchdogSummary,
  deriveRelayWatchdogDecision,
} from "../scripts/lib/wp-relay-watchdog-lib.mjs";

function relayStatus(overrides = {}) {
  return {
    applicable: true,
    status: "WATCH",
    reason_code: "PENDING_NOTIFICATION_WAITING",
    target_role: "CODER",
    target_session: "CODER:WP-TEST-v1",
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

test("watchdog steers a watched route when no active run exists", () => {
  const decision = deriveRelayWatchdogDecision({
    relayStatus: relayStatus(),
    activeRuns: [],
    stallScanStatus: "UNKNOWN",
  });

  assert.equal(decision.action, "STEER");
  assert.equal(decision.shouldSteer, true);
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
});
