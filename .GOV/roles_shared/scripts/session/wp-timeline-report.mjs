#!/usr/bin/env node

import path from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT } from "../lib/runtime-paths.mjs";
import {
  buildWpTimelineEntries,
  buildWpTimelineSpans,
  buildWpTimelineSummary,
  loadWpTimelineArtifacts,
} from "./wp-timeline-lib.mjs";

function fail(message, details = []) {
  console.error(`[WP_TIMELINE] ${message}`);
  for (const line of details) console.error(`- ${line}`);
  process.exit(1);
}

function compactLine(value, maxLength = 240) {
  const normalized = String(value || "").replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) return normalized;
  return `${normalized.slice(0, Math.max(0, maxLength - 3))}...`;
}

function printTextReport(summary, spans, entries) {
  console.log(`[WP_TIMELINE] wp_id=${summary.wp_id}`);
  console.log(`- packet: ${summary.packet_path}`);
  console.log(`- workflow_lane: ${summary.workflow_lane}`);
  console.log(`- runtime_status: ${summary.runtime_status}`);
  console.log(`- current_phase: ${summary.current_phase}`);
  console.log(`- next_expected_actor: ${summary.next_expected_actor}`);
  console.log(`- waiting_on: ${summary.waiting_on}`);
  console.log(`- event_window_start: ${summary.event_window_start || "<none>"}`);
  console.log(`- event_window_end: ${summary.event_window_end || "<none>"}`);
  console.log(`- event_window_duration_ms: ${summary.event_window_duration_ms ?? "<none>"}`);
  console.log(`- span_window_start: ${summary.span_window_start || "<none>"}`);
  console.log(`- span_window_end: ${summary.span_window_end || "<none>"}`);
  console.log(`- span_window_duration_ms: ${summary.span_window_duration_ms ?? "<none>"}`);
  console.log(`- measured_span_duration_ms: ${summary.measured_span_duration_ms ?? "<none>"}`);
  console.log(`- event_count: ${summary.event_count}`);
  console.log(`- span_count: ${summary.span_count}`);
  console.log(`- control_span_count: ${summary.control_span_count}`);
  console.log(`- review_span_count: ${summary.review_span_count}`);
  console.log(`- token_command_span_count: ${summary.token_command_span_count}`);
  console.log(`- microtask_execution_span_count: ${summary.microtask_execution_span_count}`);
  console.log(`- stage_counts: ${JSON.stringify(summary.stage_counts || {})}`);
  console.log(`- thread_count: ${summary.thread_count}`);
  console.log(`- receipt_count: ${summary.receipt_count}`);
  console.log(`- notification_count: ${summary.notification_count}`);
  console.log(`- control_request_count: ${summary.control_request_count}`);
  console.log(`- control_result_count: ${summary.control_result_count}`);
  console.log(`- turn_usage_count: ${summary.turn_usage_count}`);
  console.log(`- token_summary_source: ${summary.token_summary_source}`);
  console.log(`- token_input_total: ${summary.token_input_total}`);
  console.log(`- token_cached_input_total: ${summary.token_cached_input_total}`);
  console.log(`- token_output_total: ${summary.token_output_total}`);
  console.log(`- token_turn_count: ${summary.token_turn_count}`);
  console.log(`- token_command_count: ${summary.token_command_count}`);
  console.log(`- ledger_health_status: ${summary.ledger_health_status}`);
  console.log(`- ledger_health_severity: ${summary.ledger_health_severity}`);
  console.log(`- budget_status: ${summary.budget_status}`);
  console.log(`- budget_summary: ${summary.budget_summary}`);
  console.log(`- relay_current_lane: ${summary.relay_policy?.current_lane || "<none>"}`);
  console.log(`- relay_default_lane: ${summary.relay_policy?.default_lane || "<none>"}`);
  console.log(`- relay_recommended_lane: ${summary.relay_policy?.recommended_lane || "<none>"}`);
  console.log(`- relay_assessment: ${summary.relay_policy?.assessment || "<none>"}`);
  console.log(`- relay_burden_level: ${summary.relay_policy?.burden_level || "<none>"}`);
  console.log(`- relay_command_count: ${summary.relay_policy?.relay_command_count ?? "<none>"}`);
  console.log(`- relay_turn_count: ${summary.relay_policy?.relay_turn_count ?? "<none>"}`);
  console.log(`- relay_duration_ms: ${summary.relay_policy?.relay_duration_ms ?? "<none>"}`);
  console.log(`- relay_token_share: ${summary.relay_policy?.relay_token_share ?? "<none>"}`);
  console.log(`- relay_recommendation: ${summary.relay_policy?.recommendation_reason || "<none>"}`);
  console.log(`- cost_estimate: ${summary.cost_estimate === null ? summary.cost_estimate_note : summary.cost_estimate}`);
  console.log("");
  console.log("SPANS");
  if (spans.length === 0) {
    console.log("No computed spans.");
  } else {
    for (const span of spans) {
      console.log(span.header);
      for (const detailLine of span.detailLines || []) {
        console.log(`  ${compactLine(detailLine)}`);
      }
    }
  }
  console.log("");
  console.log("TIMELINE");
  if (entries.length === 0) {
    console.log("No timeline entries.");
    return;
  }
  for (const entry of entries) {
    console.log(entry.header);
    for (const detailLine of entry.detailLines || []) {
      console.log(`  ${compactLine(detailLine)}`);
    }
  }
}

function main() {
  const wpId = String(process.argv[2] || "").trim();
  const jsonMode = process.argv.slice(3).some((arg) => String(arg || "").trim() === "--json");
  if (!wpId || !/^WP-/.test(wpId)) {
    fail("Usage: node .GOV/roles_shared/scripts/session/wp-timeline-report.mjs WP-{ID} [--json]");
  }

  const artifacts = loadWpTimelineArtifacts(REPO_ROOT, wpId);
  const entries = buildWpTimelineEntries({
    threadEntries: artifacts.threadEntries,
    receipts: artifacts.receipts,
    notifications: artifacts.notifications,
    controlRequests: artifacts.controlRequests,
    controlResults: artifacts.controlResults,
    tokenCommands: artifacts.tokenLedger?.commands || [],
  });
  const spans = buildWpTimelineSpans({
    receipts: artifacts.receipts,
    controlRequests: artifacts.controlRequests,
    controlResults: artifacts.controlResults,
    tokenCommands: artifacts.tokenLedger?.commands || [],
  });
  const summary = buildWpTimelineSummary({
    wpId,
    packetPath: artifacts.packetPath,
    workflowLane: artifacts.workflowLane,
    runtimeStatus: artifacts.runtimeStatus,
    receipts: artifacts.receipts,
    notifications: artifacts.notifications,
    controlRequests: artifacts.controlRequests,
    controlResults: artifacts.controlResults,
    tokenLedger: artifacts.tokenLedger,
    entries,
    spans,
  });

  if (jsonMode) {
    console.log(JSON.stringify({ summary, spans, entries }, null, 2));
    return;
  }
  printTextReport(summary, spans, entries);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
