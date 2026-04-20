import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import {
  compactNextActionLabel,
  filterRecords,
  nextActionSummary,
  summarizeQueuedGovernedWork,
} from "../scripts/operator-monitor-tui.mjs";

const repoRoot = path.resolve(import.meta.dirname, "../../../..");
const scriptPath = path.join(repoRoot, ".GOV", "operator", "scripts", "operator-viewport-tui.mjs");

test("operator viewport renders the refreshed dashboard summary in once mode", () => {
  assert.equal(fs.existsSync(scriptPath), true, "operator viewport script should exist");
  const result = spawnSync(process.execPath, [scriptPath, "--once", "--filter", "ACTIVE", "--view", "OVERVIEW"], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr || "operator viewport should exit successfully");
  assert.match(result.stdout, /Operator Viewport/);
  assert.match(result.stdout, /next_action=/);
  assert.match(result.stdout, /visible=\d+\/\d+/);
});

test("operator viewport summarizes queued governed work from registry sessions", () => {
  const record = {
    boardSection: "ACTIVE",
    registrySessions: [
      {
        role: "CODER",
        updated_at: "2026-04-20T09:10:00.000Z",
        pending_control_queue_count: 2,
        next_queued_control_request: {
          command_kind: "SEND_PROMPT",
          queued_at: "2026-04-20T09:05:00.000Z",
          blocking_command_id: "run-1",
          summary: "Resume coder after queued busy ingress",
        },
      },
    ],
    packetRecord: {
      pendingNotifications: { total: 0, byRole: {} },
      openReviewItems: [],
      relayEscalation: { status: "NOT_APPLICABLE" },
      runtime: {},
    },
    controlBrokerRuns: [],
    controlResults: [],
    pendingControlRequests: [],
  };

  const queued = summarizeQueuedGovernedWork(record);
  assert.equal(queued.totalQueuedRequests, 2);
  assert.equal(queued.sessionCount, 1);
  assert.equal(queued.headSession?.role, "CODER");
  assert.equal(queued.headRequest?.command_kind, "SEND_PROMPT");
  assert.match(nextActionSummary(record), /Queued governed work/i);
  assert.match(nextActionSummary(record), /broker drain/i);
});

test("operator viewport ACTIVE filter retains terminal records with queued governed work", () => {
  const records = [
    {
      boardSection: "DONE",
      registrySessions: [
        {
          role: "CODER",
          pending_control_queue_count: 1,
          next_queued_control_request: {
            command_kind: "SEND_PROMPT",
            queued_at: "2026-04-20T09:05:00.000Z",
          },
        },
      ],
      packetRecord: null,
      pendingControlRequests: [],
      controlBrokerRuns: [],
      controlResults: [],
    },
  ];

  assert.equal(filterRecords(records, "ACTIVE").length, 1);
});

test("operator viewport escalated relay summaries include typed policy state", () => {
  const record = {
    boardSection: "ACTIVE",
    registrySessions: [],
    packetRecord: {
      pendingNotifications: { total: 0, byRole: {} },
      openReviewItems: [],
      relayEscalation: {
        status: "ESCALATED",
        summary: "Validator session is no longer steerable.",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        recommended_command: "just session-registry-status WP-TEST",
      },
      relayPolicy: {
        failure_class: "VALIDATOR_SESSION_UNAVAILABLE",
        policy_state: "AUTO_RETRY_BLOCKED",
        next_strategy: "HUMAN_STOP",
        budget_scope: "RELAY_ESCALATION_CYCLE",
        budget_used: 2,
        budget_limit: 2,
        summary: "Repeated validator wake attempts exhausted the relay budget.",
      },
      runtime: {},
    },
    controlBrokerRuns: [],
    controlResults: [],
    pendingControlRequests: [],
  };

  assert.equal(compactNextActionLabel(record), "WP_VALIDATOR:wpv-1/human-stop");
  const summary = nextActionSummary(record);
  assert.match(summary, /Validator session is no longer steerable/i);
  assert.match(summary, /failure_class=VALIDATOR_SESSION_UNAVAILABLE/);
  assert.match(summary, /policy=AUTO_RETRY_BLOCKED->HUMAN_STOP/);
  assert.match(summary, /budget=RELAY_ESCALATION_CYCLE:2\/2/);
  assert.match(summary, /Repeated validator wake attempts exhausted the relay budget/i);
});
