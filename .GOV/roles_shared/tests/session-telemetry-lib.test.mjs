import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  activeRunsForSession,
  buildSessionRunTelemetry,
  buildSessionStepTelemetry,
  buildSessionTelemetry,
  formatPushAlertInline,
  formatSessionRunTelemetryInline,
  formatSessionStepTelemetryInline,
  selectLatestPushAlert,
} from "../scripts/session/session-telemetry-lib.mjs";

function writeJsonl(filePath, entries = []) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(
    filePath,
    entries.map((entry) => JSON.stringify(entry)).join("\n") + "\n",
    "utf8",
  );
}

test("session telemetry splits queue-backed run state from step activity", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-session-telemetry-"));
  const outputFile = path.join(tempRoot, "gov_runtime", "roles_shared", "SESSION_CONTROL", "out.jsonl");
  writeJsonl(outputFile, [
    {
      type: "item.completed",
      timestamp: "2026-04-20T10:00:00.000Z",
      item: { type: "file_change", timestamp: "2026-04-20T10:00:00.000Z" },
    },
  ]);

  const session = {
    runtime_state: "READY",
    pending_control_queue_count: 2,
    next_queued_control_request: { queue_reason_code: "SESSION_BUSY" },
    last_event_at: "2026-04-20T10:00:05.000Z",
    last_command_output_file: path.relative(tempRoot, outputFile).replace(/\\/g, "/"),
  };

  const telemetry = buildSessionTelemetry({
    session,
    repoRoot: tempRoot,
    now: new Date("2026-04-20T10:01:00.000Z"),
  });

  assert.equal(telemetry.run.state, "QUEUED");
  assert.equal(telemetry.run.wait_reason_code, "SESSION_BUSY");
  assert.equal(telemetry.run.queued_request_count, 2);
  assert.equal(telemetry.step.state, "ACTIVE");
  assert.equal(telemetry.step.latest_step_kind, "file_change");
  assert.match(formatSessionRunTelemetryInline(telemetry.run), /run=QUEUED/);
  assert.match(formatSessionStepTelemetryInline(telemetry.step), /step=ACTIVE/);
});

test("session step telemetry marks stale output when progress goes quiet", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-session-telemetry-"));
  const outputFile = path.join(tempRoot, "run.jsonl");
  writeJsonl(outputFile, [
    {
      type: "item.completed",
      timestamp: "2026-04-20T08:00:00.000Z",
      item: { type: "command_execution", timestamp: "2026-04-20T08:00:00.000Z" },
    },
  ]);
  fs.utimesSync(outputFile, new Date("2026-04-20T08:00:00.000Z"), new Date("2026-04-20T08:00:00.000Z"));

  const telemetry = buildSessionStepTelemetry({
    session: {
      last_command_output_file: outputFile,
    },
    repoRoot: tempRoot,
    now: new Date("2026-04-20T10:30:00.000Z"),
  });

  assert.equal(telemetry.state, "STALE");
  assert.equal(telemetry.latest_step_kind, "command_execution");
  assert.ok(telemetry.output_idle_seconds >= 0);
});

test("session run telemetry uses active runs before raw runtime state", () => {
  const runTelemetry = buildSessionRunTelemetry({
    session: {
      runtime_state: "READY",
      pending_control_queue_count: 0,
    },
    activeRuns: [
      {
        timeout_at: "2026-04-20T10:15:00.000Z",
      },
    ],
    now: new Date("2026-04-20T10:10:00.000Z"),
  });

  assert.equal(runTelemetry.state, "RUNNING");
  assert.equal(runTelemetry.active_run_count, 1);
  assert.equal(runTelemetry.wait_reason_code, "ACTIVE_RUN");
});

test("activeRunsForSession prefers session_key matches and falls back to wp/role", () => {
  const session = {
    session_key: "CODER:WP-123",
    wp_id: "WP-123",
    role: "CODER",
  };
  const activeRuns = [
    { session_key: "CODER:WP-123", wp_id: "WP-123", role: "CODER", command_kind: "SEND_PROMPT" },
    { wp_id: "WP-123", role: "CODER", command_kind: "RETRY" },
    { wp_id: "WP-123", role: "WP_VALIDATOR", command_kind: "SEND_PROMPT" },
  ];

  assert.deepEqual(
    activeRunsForSession(session, activeRuns),
    [activeRuns[0], activeRuns[1]],
  );
});

test("session telemetry selects the latest pending push alert for the requested role", () => {
  const alert = selectLatestPushAlert(
    [
      {
        source_kind: "ACP_HEALTH_ALERT",
        target_role: "ORCHESTRATOR",
        timestamp_utc: "2026-04-20T10:00:00.000Z",
        summary: "older alert",
        acknowledged: false,
      },
      {
        source_kind: "RELAY_WATCHDOG_REPAIR",
        target_role: "CODER",
        target_session: "coder-1",
        timestamp_utc: "2026-04-20T10:05:00.000Z",
        summary: "latest lane alert",
        acknowledged: false,
      },
      {
        source_kind: "RED_ALERT_ORCHESTRATOR_DOWNTIME",
        target_role: "ORCHESTRATOR",
        timestamp_utc: "2026-04-20T10:06:00.000Z",
        summary: "operator-visible downtime alert",
        acknowledged: false,
      },
      {
        source_kind: "ACP_HEALTH_ALERT",
        target_role: "CODER",
        target_session: "coder-1",
        timestamp_utc: "2026-04-20T10:03:00.000Z",
        summary: "older coder alert",
        acknowledged: true,
      },
    ],
    {
      targetRole: "CODER",
      targetSession: "coder-1",
    },
  );

  assert.equal(alert?.source_kind, "RELAY_WATCHDOG_REPAIR");
  assert.equal(alert?.state, "PENDING");
  assert.match(formatPushAlertInline(alert), /RELAY_WATCHDOG_REPAIR/);
});

test("session telemetry treats orchestrator downtime red alert as a push alert", () => {
  const alert = selectLatestPushAlert(
    [
      {
        source_kind: "RED_ALERT_ORCHESTRATOR_DOWNTIME",
        target_role: "ORCHESTRATOR",
        timestamp_utc: "2026-04-20T10:06:00.000Z",
        summary: "visible rescue may be needed",
        acknowledged: false,
      },
    ],
    { targetRole: "ORCHESTRATOR" },
  );

  assert.equal(alert?.source_kind, "RED_ALERT_ORCHESTRATOR_DOWNTIME");
  assert.match(formatPushAlertInline(alert), /RED_ALERT_ORCHESTRATOR_DOWNTIME/);
});
