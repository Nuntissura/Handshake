import assert from "node:assert/strict";
import test from "node:test";

import {
  buildWorkflowDossierIdleMetrics,
  buildWpTimelineEntries,
  buildWpTimelineSpans,
  buildWpTimelineSummary,
  evaluateWpRelayCostPolicy,
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
  assert.equal(summary.microtask_execution_span_count, 0);
  assert.deepEqual(summary.stage_counts, {});
  assert.equal(summary.relay_policy.current_lane, "ORCHESTRATOR_MANAGED");
  assert.equal(summary.relay_policy.default_lane, "MANUAL_RELAY");
  assert.equal(summary.relay_policy.recommended_lane, "MANUAL_RELAY");
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

  assert.equal(spans.length, 3);
  assert.equal(spans[0].span_kind, "CONTROL_COMMAND");
  assert.equal(spans[0].span_stage, "RELAY");
  assert.match(spans[0].header, /CONTROL_COMMAND/);
  assert.equal(spans[1].span_kind, "TOKEN_COMMAND");
  assert.match(spans[1].header, /TOKEN_COMMAND/);
  assert.equal(spans[1].token_input_total, 120);
  assert.equal(spans[2].span_kind, "REVIEW_EXCHANGE");
  assert.match(spans[2].header, /REVIEW_EXCHANGE/);
  assert.equal(spans[2].duration_ms, 240000);
});

test("buildWpTimelineSpans derives microtask execution spans from coder intent to overlap review", () => {
  const spans = buildWpTimelineSpans({
    receipts: [
      {
        timestamp_utc: "2026-04-05T10:00:00Z",
        actor_role: "CODER",
        actor_session: "coder-1",
        receipt_kind: "CODER_INTENT",
        correlation_id: "intent-1",
        summary: "Starting MT-001.",
        microtask_contract: {
          scope_ref: "MT-001",
          phase_gate: "MICROTASK",
        },
      },
      {
        timestamp_utc: "2026-04-05T10:02:00Z",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        receipt_kind: "REVIEW_REQUEST",
        correlation_id: "review-1",
        summary: "Review MT-001 while I continue MT-002.",
        microtask_contract: {
          scope_ref: "MT-001",
          review_mode: "OVERLAP",
          phase_gate: "MICROTASK",
        },
      },
      {
        timestamp_utc: "2026-04-05T10:03:00Z",
        actor_role: "CODER",
        actor_session: "coder-1",
        receipt_kind: "CODER_INTENT",
        correlation_id: "intent-2",
        summary: "Starting MT-002.",
        microtask_contract: {
          scope_ref: "MT-002",
          phase_gate: "MICROTASK",
        },
      },
    ],
  });

  const microtaskSpan = spans.find((entry) => entry.span_kind === "MICROTASK_EXECUTION" && entry.microtask_scope_ref === "MT-001");
  assert.ok(microtaskSpan);
  assert.equal(microtaskSpan.span_stage, "MICROTASK_EXECUTION");
  assert.equal(microtaskSpan.started_at, "2026-04-05T10:00:00Z");
  assert.equal(microtaskSpan.ended_at, "2026-04-05T10:02:00Z");
  assert.equal(microtaskSpan.duration_ms, 120000);
  assert.equal(microtaskSpan.terminal_receipt_kind, "REVIEW_REQUEST");
});

test("evaluateWpRelayCostPolicy recommends MANUAL_RELAY when relay prompt tax is visible", () => {
  const policy = evaluateWpRelayCostPolicy({
    workflowLane: "ORCHESTRATOR_MANAGED",
    spans: [
      {
        span_kind: "CONTROL_COMMAND",
        span_stage: "RELAY",
        command_id: "cmd-1",
        duration_ms: 600000,
      },
      {
        span_kind: "TOKEN_COMMAND",
        span_stage: "RELAY",
        command_id: "cmd-1",
        turn_count: 5,
        token_input_total: 900,
        token_cached_input_total: 100,
        token_output_total: 200,
      },
    ],
    tokenLedger: {
      summary: {
        usage_totals: {
          input_tokens: 1600,
          output_tokens: 300,
        },
      },
    },
  });

  assert.equal(policy.default_lane, "MANUAL_RELAY");
  assert.equal(policy.recommended_lane, "MANUAL_RELAY");
  assert.equal(policy.burden_level, "HIGH");
  assert.equal(policy.relay_command_count, 1);
  assert.equal(policy.relay_turn_count, 5);
  assert.match(policy.recommendation_reason, /Observed orchestrator-managed relay burden is high/i);
});

test("buildWorkflowDossierIdleMetrics reports review latency, pass-to-coder delay, and drift markers", () => {
  const entries = buildWpTimelineEntries({
    receipts: [
      {
        timestamp_utc: "2026-04-05T10:00:00Z",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        receipt_kind: "REVIEW_REQUEST",
        correlation_id: "review-1",
        summary: "Review MT-001.",
      },
      {
        timestamp_utc: "2026-04-05T10:03:00Z",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        receipt_kind: "REVIEW_RESPONSE",
        correlation_id: "review-1",
        summary: "Approved to continue.",
        microtask_contract: {
          review_outcome: "APPROVED_FOR_FINAL_REVIEW",
        },
      },
      {
        timestamp_utc: "2026-04-05T10:07:00Z",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        receipt_kind: "CODER_INTENT",
        correlation_id: "intent-2",
        summary: "Starting MT-002.",
      },
      {
        timestamp_utc: "2026-04-05T10:03:00Z",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        receipt_kind: "REVIEW_RESPONSE",
        correlation_id: "review-1",
        summary: "Approved to continue.",
        microtask_contract: {
          review_outcome: "APPROVED_FOR_FINAL_REVIEW",
        },
      },
    ],
    controlRequests: [
      {
        created_at: "2026-04-05T10:08:00Z",
        role: "CODER",
        command_kind: "SEND_PROMPT",
        command_id: "cmd-2",
        summary: "resume coder",
      },
      {
        created_at: "2026-04-05T10:09:00Z",
        role: "WP_VALIDATOR",
        command_kind: "SEND_PROMPT",
        command_id: "cmd-open",
        summary: "resume validator",
      },
    ],
    controlResults: [
      {
        processed_at: "2026-04-05T10:08:30Z",
        role: "CODER",
        command_kind: "SEND_PROMPT",
        command_id: "cmd-2",
        status: "COMPLETED",
        summary: "coder resumed",
      },
    ],
  });
  const spans = buildWpTimelineSpans({
    receipts: [
      {
        timestamp_utc: "2026-04-05T10:00:00Z",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        receipt_kind: "REVIEW_REQUEST",
        correlation_id: "review-1",
        summary: "Review MT-001.",
      },
      {
        timestamp_utc: "2026-04-05T10:03:00Z",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        receipt_kind: "REVIEW_RESPONSE",
        correlation_id: "review-1",
        summary: "Approved to continue.",
      },
    ],
    controlRequests: [],
    controlResults: [],
  });
  const metrics = buildWorkflowDossierIdleMetrics({
    entries,
    spans,
    receipts: [
      {
        timestamp_utc: "2026-04-05T10:00:00Z",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        receipt_kind: "REVIEW_REQUEST",
        correlation_id: "review-1",
        summary: "Review MT-001.",
      },
      {
        timestamp_utc: "2026-04-05T10:03:00Z",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        receipt_kind: "REVIEW_RESPONSE",
        correlation_id: "review-1",
        summary: "Approved to continue.",
        microtask_contract: {
          review_outcome: "APPROVED_FOR_FINAL_REVIEW",
        },
      },
      {
        timestamp_utc: "2026-04-05T10:03:00Z",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        receipt_kind: "REVIEW_RESPONSE",
        correlation_id: "review-1",
        summary: "Approved to continue.",
        microtask_contract: {
          review_outcome: "APPROVED_FOR_FINAL_REVIEW",
        },
      },
      {
        timestamp_utc: "2026-04-05T10:07:00Z",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        receipt_kind: "CODER_INTENT",
        correlation_id: "intent-2",
        summary: "Starting MT-002.",
      },
    ],
    controlRequests: [
      { created_at: "2026-04-05T10:08:00Z" },
      { created_at: "2026-04-05T10:09:00Z" },
    ],
    controlResults: [
      { processed_at: "2026-04-05T10:08:30Z" },
    ],
    now: Date.parse("2026-04-05T10:40:00Z"),
    idleThresholdMs: 5 * 60 * 1000,
  });

  assert.equal(metrics.review_response.latest_ms, 180000);
  assert.equal(metrics.validator_pass_to_next_coder_action.latest_ms, 240000);
  assert.equal(metrics.idle_gap_count, 1);
  assert.equal(metrics.max_idle_gap_ms, 1860000);
  assert.equal(metrics.current_idle_ms, 1860000);
  assert.equal(metrics.drift_markers.duplicate_receipt_count, 1);
  assert.equal(metrics.drift_markers.open_review_count, 0);
  assert.equal(metrics.drift_markers.unresolved_control_count, 1);
});
