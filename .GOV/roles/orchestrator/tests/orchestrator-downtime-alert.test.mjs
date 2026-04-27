import assert from "node:assert/strict";
import test from "node:test";

import {
  buildOrchestratorDowntimeAlertCandidate,
  evaluateOrchestratorDowntime,
  orchestratorDowntimeAlertAlreadyPending,
} from "../scripts/lib/orchestrator-downtime-alert-lib.mjs";

test("orchestrator downtime evaluator ignores non-orchestrator-managed lanes", () => {
  const result = evaluateOrchestratorDowntime({
    wpId: "WP-TEST",
    workflowLane: "MANUAL_RELAY",
    runtimeStatus: { last_event_at: "2026-04-26T10:00:00Z" },
    now: new Date("2026-04-26T10:30:00Z"),
  });

  assert.equal(result.applicable, false);
  assert.equal(result.shouldEmit, false);
});

test("orchestrator downtime evaluator stays clear when control-plane progress is fresh", () => {
  const result = evaluateOrchestratorDowntime({
    wpId: "WP-TEST",
    workflowLane: "ORCHESTRATOR_MANAGED",
    runtimeStatus: { runtime_status: "active", last_event_at: "2026-04-26T10:07:00Z" },
    receipts: [
      { actor_role: "CODER", receipt_kind: "HEARTBEAT", timestamp_utc: "2026-04-26T10:09:00Z" },
    ],
    now: new Date("2026-04-26T10:10:00Z"),
  });

  assert.equal(result.status, "CLEAR");
  assert.equal(result.shouldEmit, false);
  assert.equal(result.latestProgressSource, "receipt.CODER.HEARTBEAT");
});

test("orchestrator downtime evaluator emits warning band after ten minutes", () => {
  const result = evaluateOrchestratorDowntime({
    wpId: "WP-TEST",
    workflowLane: "ORCHESTRATOR_MANAGED",
    runtimeStatus: { runtime_status: "active", last_event_at: "2026-04-26T10:00:00Z" },
    now: new Date("2026-04-26T10:12:00Z"),
  });
  const candidate = buildOrchestratorDowntimeAlertCandidate({
    wpId: "WP-TEST",
    evaluation: result,
  });

  assert.equal(result.status, "RED_ALERT");
  assert.equal(result.alertBand, "WARN");
  assert.equal(result.recommendedCommand, "just orchestrator-health WP-TEST");
  assert.equal(candidate?.sourceKind, "RED_ALERT_ORCHESTRATOR_DOWNTIME");
  assert.equal(candidate?.targetRole, "ORCHESTRATOR");
  assert.match(candidate?.summary || "", /rescue threshold command: just orchestrator-rescue WP-TEST/);
});

test("orchestrator downtime evaluator recommends visible rescue after twenty minutes", () => {
  const result = evaluateOrchestratorDowntime({
    wpId: "WP-TEST",
    workflowLane: "ORCHESTRATOR_MANAGED",
    runtimeStatus: { runtime_status: "active", last_heartbeat_at: "2026-04-26T10:00:00Z" },
    now: new Date("2026-04-26T10:21:00Z"),
  });
  const candidate = buildOrchestratorDowntimeAlertCandidate({
    wpId: "WP-TEST",
    evaluation: result,
  });

  assert.equal(result.alertBand, "RESCUE");
  assert.equal(result.reason, "ORCHESTRATOR_DOWNTIME_RESCUE_READY");
  assert.equal(result.recommendedCommand, "just orchestrator-rescue WP-TEST");
  assert.match(candidate?.correlationId || "", /:RESCUE$/);
  assert.match(candidate?.summary || "", /Rescue command: just orchestrator-rescue WP-TEST/);
});

test("orchestrator downtime alert dedupe ignores old downtime alert as progress but detects pending duplicate", () => {
  const result = evaluateOrchestratorDowntime({
    wpId: "WP-TEST",
    workflowLane: "ORCHESTRATOR_MANAGED",
    runtimeStatus: { runtime_status: "active", last_event_at: "2026-04-26T10:00:00Z" },
    pendingNotifications: [
      {
        source_kind: "RED_ALERT_ORCHESTRATOR_DOWNTIME",
        target_role: "ORCHESTRATOR",
        timestamp_utc: "2026-04-26T10:15:00Z",
        correlation_id: "orchestrator-downtime:WP-TEST:WARN",
      },
    ],
    now: new Date("2026-04-26T10:16:00Z"),
  });
  const candidate = buildOrchestratorDowntimeAlertCandidate({
    wpId: "WP-TEST",
    evaluation: result,
  });

  assert.equal(result.ageSeconds, 960);
  assert.equal(orchestratorDowntimeAlertAlreadyPending([
    {
      source_kind: "RED_ALERT_ORCHESTRATOR_DOWNTIME",
      target_role: "ORCHESTRATOR",
      correlation_id: "orchestrator-downtime:WP-TEST:WARN",
    },
  ], candidate), true);
});
