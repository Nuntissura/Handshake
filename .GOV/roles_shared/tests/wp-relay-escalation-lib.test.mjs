import assert from "node:assert/strict";
import test from "node:test";
import { evaluateWpRelayEscalation } from "../scripts/lib/wp-relay-escalation-lib.mjs";

function baseRuntime(overrides = {}) {
  return {
    next_expected_actor: "WP_VALIDATOR",
    next_expected_session: "wpv-1",
    heartbeat_due_at: "2026-03-30T10:10:00Z",
    stale_after: "2026-03-30T10:20:00Z",
    current_relay_escalation_cycle: 0,
    max_relay_escalation_cycles: 2,
    active_role_sessions: [],
    ...overrides,
  };
}

test("relay escalation is not applicable when no governed actor is projected", () => {
  const result = evaluateWpRelayEscalation({
    wpId: "WP-TEST-RELAY-v1",
    runtimeStatus: baseRuntime({ next_expected_actor: "ORCHESTRATOR", next_expected_session: null }),
    communicationEvaluation: { applicable: true },
    receipts: [],
    pendingNotifications: [],
    nowIso: "2026-03-30T10:30:00Z",
  });

  assert.equal(result.applicable, false);
  assert.equal(result.status, "NOT_APPLICABLE");
});

test("relay escalation warns after heartbeat_due_at when pending notifications are still waiting", () => {
  const result = evaluateWpRelayEscalation({
    wpId: "WP-TEST-RELAY-v1",
    runtimeStatus: baseRuntime(),
    communicationEvaluation: { applicable: true, state: "COMM_WAITING_FOR_REVIEW" },
    receipts: [
      { actor_role: "CODER", actor_session: "coder-1", timestamp_utc: "2026-03-30T10:00:00Z" },
    ],
    pendingNotifications: [
      { target_role: "WP_VALIDATOR", target_session: "wpv-1", timestamp_utc: "2026-03-30T10:00:01Z" },
    ],
    nowIso: "2026-03-30T10:15:00Z",
  });

  assert.equal(result.applicable, true);
  assert.equal(result.status, "WATCH");
  assert.equal(result.reason_code, "WAITING_ON_VALIDATOR_REVIEW");
});

test("relay escalation fails when stale notifications cross stale_after without receipt progress", () => {
  const result = evaluateWpRelayEscalation({
    wpId: "WP-TEST-RELAY-v1",
    runtimeStatus: baseRuntime({
      active_role_sessions: [
        { role: "WP_VALIDATOR", session_id: "wpv-1", last_heartbeat_at: "2026-03-30T10:05:00Z" },
      ],
    }),
    communicationEvaluation: { applicable: true },
    receipts: [
      { actor_role: "CODER", actor_session: "coder-1", timestamp_utc: "2026-03-30T10:00:00Z" },
    ],
    pendingNotifications: [
      { target_role: "WP_VALIDATOR", target_session: "wpv-1", timestamp_utc: "2026-03-30T10:00:01Z" },
    ],
    nowIso: "2026-03-30T10:25:00Z",
  });

  assert.equal(result.status, "ESCALATED");
  assert.match(result.reason_code, /PENDING_NOTIFICATION_STALE|SESSION_ACTIVE_NO_RECEIPT_PROGRESS/);
  assert.match(
    result.summary,
    /Use just orchestrator-steer-next WP-TEST-RELAY-v1 "<why this stalled relay should be re-woken, >=40 chars>"/i,
  );
});

test("relay escalation fails when receipt progress is stale even without pending notifications", () => {
  const result = evaluateWpRelayEscalation({
    wpId: "WP-TEST-RELAY-v1",
    runtimeStatus: baseRuntime({
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
    }),
    communicationEvaluation: { applicable: true, state: "COMM_WAITING_FOR_HANDOFF" },
    receipts: [
      { actor_role: "WP_VALIDATOR", actor_session: "wpv-1", timestamp_utc: "2026-03-30T10:00:00Z" },
    ],
    pendingNotifications: [],
    nowIso: "2026-03-30T10:25:00Z",
  });

  assert.equal(result.status, "ESCALATED");
  assert.equal(result.reason_code, "ROUTE_STALE_WAITING_ON_CODER_HANDOFF");
});

test("relay escalation records route and session activity timestamps when registry activity occurs after the route opened", () => {
  const result = evaluateWpRelayEscalation({
    wpId: "WP-TEST-RELAY-v1",
    runtimeStatus: baseRuntime({
      next_expected_actor: "CODER",
      next_expected_session: "coder-2",
    }),
    communicationEvaluation: { applicable: true },
    receipts: [],
    pendingNotifications: [
      { target_role: "CODER", target_session: "coder-2", timestamp_utc: "2026-03-30T10:00:00Z" },
    ],
    registrySessions: [
      {
        role: "CODER",
        wp_id: "WP-TEST-RELAY-v1",
        session_id: "coder-2",
        session_key: "CODER:WP-TEST-RELAY-v1",
        updated_at: "2026-03-30T10:25:00Z",
      },
    ],
    nowIso: "2026-03-30T10:30:00Z",
  });

  assert.equal(result.status, "ESCALATED");
  assert.equal(result.reason_code, "SESSION_ACTIVE_NO_RECEIPT_PROGRESS");
  assert.equal(result.metrics.route_anchor_at, "2026-03-30T10:00:00.000Z");
  assert.equal(result.metrics.latest_session_activity_at, "2026-03-30T10:25:00.000Z");
});

test("relay escalation surfaces human approval waits even before watchdog thresholds are crossed", () => {
  const result = evaluateWpRelayEscalation({
    wpId: "WP-TEST-RELAY-v1",
    runtimeStatus: baseRuntime({
      waiting_on: "OPERATOR_APPROVAL",
    }),
    communicationEvaluation: { applicable: true, state: "COMM_OK" },
    receipts: [],
    pendingNotifications: [],
    nowIso: "2026-03-30T10:05:00Z",
  });

  assert.equal(result.status, "NORMAL");
  assert.equal(result.reason_code, "WAITING_ON_HUMAN_APPROVAL");
});

test("relay escalation surfaces dependency waits from blocked open review items", () => {
  const result = evaluateWpRelayEscalation({
    wpId: "WP-TEST-RELAY-v1",
    runtimeStatus: baseRuntime({
      waiting_on: "BLOCKED_OPEN_REVIEW_ITEM",
    }),
    communicationEvaluation: { applicable: true, state: "COMM_BLOCKED_OPEN_ITEMS" },
    receipts: [],
    pendingNotifications: [],
    nowIso: "2026-03-30T10:12:00Z",
  });

  assert.equal(result.reason_code, "WAITING_ON_DEPENDENCY");
});

test("relay escalation surfaces deferred overlap repair as a coder-owned wait state", () => {
  const result = evaluateWpRelayEscalation({
    wpId: "WP-TEST-RELAY-v1",
    runtimeStatus: baseRuntime({
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
      waiting_on: "CURRENT_MICROTASK_COMPLETION_BEFORE_REPAIR",
    }),
    communicationEvaluation: { applicable: true, state: "COMM_DEFERRED_REPAIR_QUEUE" },
    receipts: [],
    pendingNotifications: [],
    nowIso: "2026-03-30T10:12:00Z",
  });

  assert.equal(result.reason_code, "WAITING_ON_CODER_DEFERRED_REPAIR");
});
