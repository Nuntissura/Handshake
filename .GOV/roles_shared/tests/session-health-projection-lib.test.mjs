import assert from "node:assert/strict";
import test from "node:test";

import {
  applySessionHealthProjection,
  buildAcpHealthAlertCandidate,
  evaluateGovernedSessionHealth,
} from "../scripts/session/session-health-projection-lib.mjs";

function makeSession(overrides = {}) {
  return {
    session_key: "CODER:WP-TEST",
    role: "CODER",
    runtime_state: "READY",
    last_heartbeat_at: "2026-04-18T09:58:00Z",
    last_event_at: "2026-04-18T09:58:00Z",
    last_command_completed_at: "",
    last_command_prompt_at: "",
    health_state: "UNKNOWN",
    health_reason_code: "UNKNOWN",
    health_summary: "",
    health_source: "",
    health_updated_at: "",
    ...overrides,
  };
}

test("session health projection fails when runtime expects a target session that is missing", () => {
  const projection = evaluateGovernedSessionHealth({
    targetRole: "CODER",
    targetSession: "CODER:WP-TEST",
    session: null,
    now: new Date("2026-04-18T10:00:00Z"),
  });

  assert.equal(projection.healthState, "FAILED");
  assert.equal(projection.reasonCode, "TARGET_SESSION_MISSING");
  const alert = buildAcpHealthAlertCandidate({
    wpId: "WP-TEST",
    projection,
  });
  assert.equal(alert?.sourceKind, "ACP_HEALTH_ALERT");
  assert.match(alert?.correlationId || "", /TARGET_SESSION_MISSING$/);
});

test("session health projection fails when broker active runs have expired", () => {
  const projection = evaluateGovernedSessionHealth({
    targetRole: "CODER",
    targetSession: "CODER:WP-TEST",
    session: makeSession({
      runtime_state: "COMMAND_RUNNING",
      last_heartbeat_at: "2026-04-18T09:59:30Z",
      last_event_at: "2026-04-18T09:59:30Z",
    }),
    activeRuns: [
      {
        role: "CODER",
        session_key: "CODER:WP-TEST",
        timeout_at: "2026-04-18T09:30:00Z",
      },
    ],
    outputFreshnessStatus: "RECENT",
    now: new Date("2026-04-18T10:00:00Z"),
  });

  assert.equal(projection.healthState, "FAILED");
  assert.equal(projection.reasonCode, "ACTIVE_RUN_TIMEOUT");
});

test("session health projection degrades aging heartbeat before the hard fail threshold", () => {
  const projection = evaluateGovernedSessionHealth({
    targetRole: "WP_VALIDATOR",
    targetSession: "WP_VALIDATOR:WP-TEST",
    session: makeSession({
      session_key: "WP_VALIDATOR:WP-TEST",
      role: "WP_VALIDATOR",
      last_heartbeat_at: "2026-04-18T09:49:00Z",
      last_event_at: "2026-04-18T09:49:00Z",
    }),
    now: new Date("2026-04-18T10:00:00Z"),
  });

  assert.equal(projection.healthState, "DEGRADED");
  assert.equal(projection.reasonCode, "HEARTBEAT_DEGRADED");
});

test("session health projection records timestamp only when persisted truth changes", () => {
  const session = makeSession();
  const projection = evaluateGovernedSessionHealth({
    targetRole: "CODER",
    targetSession: "CODER:WP-TEST",
    session,
    now: new Date("2026-04-18T10:00:00Z"),
  });

  const changed = applySessionHealthProjection(session, projection, {
    updatedAt: "2026-04-18T10:00:00Z",
  });
  assert.equal(changed, true);
  assert.equal(session.health_state, "HEALTHY");
  assert.equal(session.health_updated_at, "2026-04-18T10:00:00Z");

  const unchanged = applySessionHealthProjection(session, projection, {
    updatedAt: "2026-04-18T10:05:00Z",
  });
  assert.equal(unchanged, false);
  assert.equal(session.health_updated_at, "2026-04-18T10:00:00Z");
  assert.equal(buildAcpHealthAlertCandidate({ wpId: "WP-TEST", projection }), null);
});
