import assert from "node:assert/strict";
import test from "node:test";

import {
  buildWpTimelineEntries,
  buildWpTimelineSpans,
  buildWpTimelineSummary,
  parseThreadEntriesText,
} from "../scripts/session/wp-timeline-lib.mjs";

test("parseThreadEntriesText parses governance thread entries", () => {
  const entries = parseThreadEntriesText(`
- 2026-04-05T10:00:00Z | CODER | session=coder:test | target_role=WP_VALIDATOR | target_session=wpv:test | correlation_id=abc123
  first line
  second line
`);

  assert.equal(entries.length, 1);
  assert.equal(entries[0].actorRole, "CODER");
  assert.equal(entries[0].actorSession, "coder:test");
  assert.equal(entries[0].targetRole, "WP_VALIDATOR");
  assert.equal(entries[0].correlationId, "abc123");
  assert.deepEqual(entries[0].messageLines, ["first line", "second line"]);
});

test("buildWpTimelineEntries merges and sorts all event sources", () => {
  const entries = buildWpTimelineEntries({
    threadEntries: [
      {
        timestamp: "2026-04-05T10:00:00Z",
        actorRole: "CODER",
        actorSession: "coder:test",
        messageLines: ["thread body"],
      },
    ],
    receipts: [
      {
        timestamp_utc: "2026-04-05T10:01:00Z",
        actor_role: "WP_VALIDATOR",
        receipt_kind: "VALIDATOR_REVIEW",
        summary: "Looks good",
      },
    ],
    notifications: [
      {
        timestamp_utc: "2026-04-05T10:00:30Z",
        source_role: "CODER",
        source_kind: "AUTO_ROUTE",
        target_role: "WP_VALIDATOR",
        summary: "wake validator",
      },
    ],
    controlRequests: [
      {
        created_at: "2026-04-05T09:59:00Z",
        role: "CODER",
        command_kind: "START_SESSION",
        summary: "start coder",
      },
    ],
    controlResults: [
      {
        processed_at: "2026-04-05T09:59:30Z",
        role: "CODER",
        command_kind: "START_SESSION",
        status: "COMPLETED",
        summary: "started",
        duration_ms: 30000,
      },
    ],
    tokenCommands: [
      {
        command_id: "cmd-1",
        command_kind: "SEND_PROMPT",
        role: "CODER",
        turn_usage: [
          {
            timestamp: "2026-04-05T10:02:00Z",
            input_tokens: 100,
            cached_input_tokens: 20,
            output_tokens: 15,
          },
        ],
      },
    ],
  });

  assert.equal(entries.length, 6);
  assert.match(entries[0].header, /CONTROL_REQUEST/);
  assert.match(entries[entries.length - 1].header, /TURN_USAGE/);
});

test("buildWpTimelineSummary computes counts and event window", () => {
  const entries = buildWpTimelineEntries({
    controlRequests: [
      {
        created_at: "2026-04-05T09:59:00Z",
        role: "CODER",
        command_kind: "START_SESSION",
        summary: "start coder",
      },
    ],
    controlResults: [
      {
        processed_at: "2026-04-05T10:04:00Z",
        role: "CODER",
        command_kind: "SEND_PROMPT",
        status: "COMPLETED",
        summary: "prompt sent",
        duration_ms: 1000,
      },
    ],
  });
  const summary = buildWpTimelineSummary({
    wpId: "WP-TEST-TIMELINE-v1",
    packetPath: ".GOV/task_packets/WP-TEST-TIMELINE-v1/packet.md",
    workflowLane: "ORCHESTRATOR_MANAGED",
    runtimeStatus: {
      runtime_status: "working",
      current_phase: "IMPLEMENTATION",
      next_expected_actor: "CODER",
      waiting_on: "CODER_HANDOFF",
    },
    receipts: [],
    notifications: [],
    controlRequests: [{ created_at: "2026-04-05T09:59:00Z" }],
    controlResults: [{ processed_at: "2026-04-05T10:04:00Z" }],
    tokenLedger: {
      summary_source: "TRACKED_COMMAND_LEDGER",
      summary: {
        command_count: 2,
        turn_count: 3,
        usage_totals: {
          input_tokens: 500,
          cached_input_tokens: 120,
          output_tokens: 80,
        },
      },
      ledger_health: {
        status: "OK",
        severity: "PASS",
      },
    },
    entries,
    spans: [],
  });

  assert.equal(summary.wp_id, "WP-TEST-TIMELINE-v1");
  assert.equal(summary.event_count, 2);
  assert.equal(summary.control_request_count, 1);
  assert.equal(summary.control_result_count, 1);
  assert.equal(summary.token_input_total, 500);
  assert.equal(summary.event_window_duration_ms, 300000);
  assert.equal(summary.cost_estimate, null);
});

test("buildWpTimelineSpans pairs control commands and review exchanges", () => {
  const spans = buildWpTimelineSpans({
    receipts: [
      {
        timestamp_utc: "2026-04-05T10:00:00Z",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        receipt_kind: "CODER_HANDOFF",
        correlation_id: "handoff-1",
        summary: "Ready for validator review.",
      },
      {
        timestamp_utc: "2026-04-05T10:04:00Z",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        receipt_kind: "VALIDATOR_REVIEW",
        correlation_id: "handoff-1",
        summary: "Repair required.",
      },
    ],
    controlRequests: [
      {
        created_at: "2026-04-05T09:59:00Z",
        command_id: "cmd-1",
        role: "CODER",
        command_kind: "SEND_PROMPT",
        summary: "steer coder",
      },
    ],
    controlResults: [
      {
        processed_at: "2026-04-05T10:00:00Z",
        command_id: "cmd-1",
        role: "CODER",
        command_kind: "SEND_PROMPT",
        status: "COMPLETED",
        summary: "coder advanced",
        duration_ms: 60000,
      },
    ],
    tokenCommands: [
      {
        command_id: "cmd-1",
        command_kind: "SEND_PROMPT",
        role: "CODER",
        turn_count: 1,
        usage_totals: {
          input_tokens: 120,
          cached_input_tokens: 10,
          output_tokens: 40,
        },
      },
    ],
  });

  assert.equal(spans.length, 2);
  assert.equal(spans[0].span_kind, "CONTROL_COMMAND");
  assert.match(spans[0].header, /CONTROL_COMMAND/);
  assert.match(spans[1].header, /REVIEW_EXCHANGE/);
  assert.equal(spans[1].duration_ms, 240000);
});
