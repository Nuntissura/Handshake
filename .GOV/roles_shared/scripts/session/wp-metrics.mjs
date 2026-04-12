#!/usr/bin/env node

// WP Metrics: structured post-closeout metrics extraction.
// Aggregates receipts, session control, idle ledger, timeline, and validation
// evidence into a focused JSON summary for cross-WP trend comparison.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT, repoPathAbs } from "../lib/runtime-paths.mjs";
import { resolveValidatorGatePath } from "../lib/validator-gate-paths.mjs";
import {
  buildWpTimelineEntries,
  buildWpTimelineSpans,
  buildWpTimelineSummary,
  loadWpTimelineArtifacts,
} from "./wp-timeline-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("wp-metrics.mjs", { role: "SHARED" });

function fail(message) {
  failWithMemory("wp-metrics.mjs", message, { role: "SHARED" });
}

function msToMinutes(ms) {
  return ms != null ? Math.round((ms / 60_000) * 10) / 10 : null;
}

function safeRatio(numerator, denominator) {
  if (!denominator || denominator === 0) return null;
  return Math.round((numerator / denominator) * 1000) / 1000;
}

function countReceiptsByKind(receipts) {
  const counts = {};
  for (const r of receipts) {
    const kind = String(r.receipt_kind || "UNKNOWN").trim();
    counts[kind] = (counts[kind] || 0) + 1;
  }
  return counts;
}

function countFixCycles(receipts) {
  // A fix cycle = a REVIEW_RESPONSE with not-pass/steer, grouped by MT.
  // Count REVIEW_RESPONSE receipts that indicate rejection/steer.
  let total = 0;
  const byMt = {};
  for (const r of receipts) {
    if (r.receipt_kind !== "REVIEW_RESPONSE") continue;
    const summary = String(r.summary || "").toLowerCase();
    // Validator approvals contain "pass" without "not" or "steer"
    const isSteer = summary.includes("steer")
      || summary.includes("not pass")
      || summary.includes("not-pass")
      || summary.includes("rejected")
      || summary.includes("remediation");
    if (!isSteer) continue;
    total += 1;
    const mt = extractMicrotask(r);
    if (mt) {
      byMt[mt] = (byMt[mt] || 0) + 1;
    }
  }
  return { total, by_mt: byMt };
}

function extractMicrotask(receipt) {
  const summary = String(receipt.summary || "");
  const match = summary.match(/\bMT-\d+\b/i);
  return match ? match[0].toUpperCase() : null;
}

function countMicrotasks(receipts) {
  const mts = new Set();
  for (const r of receipts) {
    const mt = extractMicrotask(r);
    if (mt) mts.add(mt);
  }
  return mts.size;
}

function countSessionControlByStatus(controlResults) {
  const counts = { total: 0, completed: 0, failed: 0 };
  const byKind = {};
  for (const r of controlResults) {
    counts.total += 1;
    const status = String(r.status || "").toUpperCase();
    if (status === "COMPLETED") counts.completed += 1;
    if (status === "FAILED") counts.failed += 1;
    const kind = String(r.command_kind || "UNKNOWN").trim();
    if (!byKind[kind]) byKind[kind] = { total: 0, completed: 0, failed: 0 };
    byKind[kind].total += 1;
    if (status === "COMPLETED") byKind[kind].completed += 1;
    if (status === "FAILED") byKind[kind].failed += 1;
  }
  return { ...counts, by_kind: byKind };
}

function countSessionRestarts(controlResults) {
  // A restart = START_SESSION after CANCEL_SESSION for the same session_key.
  const cancelledKeys = new Set();
  let restarts = 0;
  for (const r of controlResults) {
    const key = String(r.session_key || "").trim();
    const kind = String(r.command_kind || "").trim();
    const status = String(r.status || "").toUpperCase();
    if (kind === "CANCEL_SESSION" && status === "COMPLETED") {
      cancelledKeys.add(key);
    }
    if (kind === "START_SESSION" && status === "COMPLETED" && cancelledKeys.has(key)) {
      restarts += 1;
    }
  }
  return restarts;
}

function loadValidationEvidence(wpId) {
  const gatePath = repoPathAbs(resolveValidatorGatePath(wpId));
  if (!fs.existsSync(gatePath)) return null;
  try {
    const raw = JSON.parse(fs.readFileSync(gatePath, "utf8"));
    const evidence = raw?.committed_validation_evidence?.[wpId];
    if (!evidence) return null;
    const history = Array.isArray(evidence.proof_history) ? evidence.proof_history : [];
    const zeroExecutionCount = history.filter((p) => p.zero_execution_detected).length;
    const proofRuns = history.length;
    const passCount = history.filter((p) => p.status === "PASS").length;
    const failCount = history.filter((p) => p.status === "FAIL").length;
    return {
      proof_runs: proofRuns,
      proof_pass: passCount,
      proof_fail: failCount,
      zero_execution_incidents: zeroExecutionCount,
      first_pass_success: proofRuns > 0 && history[0]?.status === "PASS",
    };
  } catch {
    return null;
  }
}

function countStaleRouteIncidents(receipts, controlResults) {
  // Count receipts or control results that indicate stale route state.
  let count = 0;
  for (const r of receipts) {
    const summary = String(r.summary || "").toLowerCase();
    if (summary.includes("route_stale") || summary.includes("stale route")) {
      count += 1;
    }
  }
  for (const r of controlResults) {
    const summary = String(r.summary || r.error || "").toLowerCase();
    if (summary.includes("route_stale") || summary.includes("stale")) {
      count += 1;
    }
  }
  return count;
}

function countDuplicateReceipts(receipts) {
  const seen = new Set();
  let duplicates = 0;
  for (const r of receipts) {
    const key = `${r.receipt_kind}:${r.correlation_id || ""}:${r.actor_role}:${r.target_role}`;
    if (r.correlation_id && seen.has(key)) {
      duplicates += 1;
    }
    seen.add(key);
  }
  return duplicates;
}

function buildWpMetrics(wpId) {
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

  const dt = summary.downtime_attribution || {};
  const activeMs = dt.active_build_ms || 0;
  const repairMs = dt.repair_overhead_ms || 0;
  const validatorWaitMs = dt.validator_wait_ms || 0;
  const routeWaitMs = dt.route_wait_ms || 0;
  const dependencyWaitMs = dt.dependency_wait_ms || 0;
  const humanWaitMs = dt.human_wait_ms || 0;
  const coderWaitMs = dt.coder_wait_ms || 0;

  const fixCycles = countFixCycles(artifacts.receipts);
  const sessionControl = countSessionControlByStatus(artifacts.controlResults);
  const validationEvidence = loadValidationEvidence(wpId);

  return {
    wp_id: wpId,
    extracted_at: new Date().toISOString(),

    // Velocity
    wall_clock_minutes: msToMinutes(summary.event_window_duration_ms),
    product_active_minutes: msToMinutes(activeMs),
    repair_minutes: msToMinutes(repairMs),
    validator_wait_minutes: msToMinutes(validatorWaitMs),
    route_wait_minutes: msToMinutes(routeWaitMs),
    coder_wait_minutes: msToMinutes(coderWaitMs),
    governance_overhead_ratio: safeRatio(repairMs + routeWaitMs, activeMs + validatorWaitMs + coderWaitMs),

    // Communication
    receipt_count: artifacts.receipts.length,
    receipt_kinds: countReceiptsByKind(artifacts.receipts),
    duplicate_receipts: countDuplicateReceipts(artifacts.receipts),
    stale_route_incidents: countStaleRouteIncidents(artifacts.receipts, artifacts.controlResults),
    review_rtt_max_ms: summary.downtime_attribution?.review_rtt_max_ms ?? null,

    // Session control
    acp_commands: sessionControl.total,
    acp_failures: sessionControl.failed,
    acp_by_kind: sessionControl.by_kind,
    session_restarts: countSessionRestarts(artifacts.controlResults),

    // Microtask & fix cycles
    mt_count: countMicrotasks(artifacts.receipts),
    fix_cycles: fixCycles.total,
    fix_cycles_by_mt: fixCycles.by_mt,

    // Validation evidence
    proof_runs: validationEvidence?.proof_runs ?? 0,
    proof_pass: validationEvidence?.proof_pass ?? 0,
    proof_fail: validationEvidence?.proof_fail ?? 0,
    zero_execution_incidents: validationEvidence?.zero_execution_incidents ?? 0,
    first_pass_compile_success: validationEvidence?.first_pass_success ?? null,

    // Token usage
    token_input_total: summary.token_input_total,
    token_output_total: summary.token_output_total,
    token_turn_count: summary.token_turn_count,
    ledger_health: summary.ledger_health_status,
    budget_status: summary.budget_status,
    cost_estimate: summary.cost_estimate,

    // Queue pressure (snapshot)
    queue_pressure_max_score: summary.queue_pressure?.score ?? null,

    // Status
    runtime_status: summary.runtime_status,
    current_phase: summary.current_phase,
    workflow_lane: summary.workflow_lane,
  };
}

function buildComparisonTable(metricsA, metricsB) {
  const fields = [
    ["wall_clock_minutes", "Wall clock (min)"],
    ["product_active_minutes", "Product active (min)"],
    ["repair_minutes", "Repair overhead (min)"],
    ["validator_wait_minutes", "Validator wait (min)"],
    ["governance_overhead_ratio", "Gov overhead ratio"],
    ["receipt_count", "Receipts"],
    ["duplicate_receipts", "Duplicate receipts"],
    ["stale_route_incidents", "Stale route incidents"],
    ["acp_commands", "ACP commands"],
    ["acp_failures", "ACP failures"],
    ["session_restarts", "Session restarts"],
    ["mt_count", "Microtasks"],
    ["fix_cycles", "Fix cycles"],
    ["zero_execution_incidents", "Zero-execution incidents"],
    ["token_input_total", "Tokens in"],
    ["token_output_total", "Tokens out"],
    ["token_turn_count", "Turns"],
    ["cost_estimate", "Cost estimate"],
  ];

  const rows = [];
  for (const [key, label] of fields) {
    const a = metricsA[key];
    const b = metricsB[key];
    const delta = (a != null && b != null) ? Math.round((b - a) * 10) / 10 : null;
    const trend = delta == null ? "" : delta > 0 ? "UP" : delta < 0 ? "DOWN" : "SAME";
    rows.push({ metric: label, wp_a: a, wp_b: b, delta, trend });
  }
  return rows;
}

function printTextMetrics(metrics) {
  console.log(`[WP_METRICS] ${metrics.wp_id}`);
  console.log(`  extracted_at: ${metrics.extracted_at}`);
  console.log(`  status: ${metrics.runtime_status} / ${metrics.current_phase}`);
  console.log("");
  console.log("VELOCITY");
  console.log(`  wall_clock_minutes: ${metrics.wall_clock_minutes ?? "N/A"}`);
  console.log(`  product_active_minutes: ${metrics.product_active_minutes ?? "N/A"}`);
  console.log(`  repair_minutes: ${metrics.repair_minutes ?? "N/A"}`);
  console.log(`  validator_wait_minutes: ${metrics.validator_wait_minutes ?? "N/A"}`);
  console.log(`  route_wait_minutes: ${metrics.route_wait_minutes ?? "N/A"}`);
  console.log(`  coder_wait_minutes: ${metrics.coder_wait_minutes ?? "N/A"}`);
  console.log(`  governance_overhead_ratio: ${metrics.governance_overhead_ratio ?? "N/A"}`);
  console.log("");
  console.log("COMMUNICATION");
  console.log(`  receipt_count: ${metrics.receipt_count}`);
  console.log(`  receipt_kinds: ${JSON.stringify(metrics.receipt_kinds)}`);
  console.log(`  duplicate_receipts: ${metrics.duplicate_receipts}`);
  console.log(`  stale_route_incidents: ${metrics.stale_route_incidents}`);
  console.log("");
  console.log("SESSION CONTROL");
  console.log(`  acp_commands: ${metrics.acp_commands}`);
  console.log(`  acp_failures: ${metrics.acp_failures}`);
  console.log(`  session_restarts: ${metrics.session_restarts}`);
  console.log("");
  console.log("MICROTASKS & FIX CYCLES");
  console.log(`  mt_count: ${metrics.mt_count}`);
  console.log(`  fix_cycles: ${metrics.fix_cycles}`);
  if (Object.keys(metrics.fix_cycles_by_mt).length > 0) {
    console.log(`  fix_cycles_by_mt: ${JSON.stringify(metrics.fix_cycles_by_mt)}`);
  }
  console.log("");
  console.log("VALIDATION EVIDENCE");
  console.log(`  proof_runs: ${metrics.proof_runs}`);
  console.log(`  proof_pass: ${metrics.proof_pass}`);
  console.log(`  proof_fail: ${metrics.proof_fail}`);
  console.log(`  zero_execution_incidents: ${metrics.zero_execution_incidents}`);
  console.log(`  first_pass_compile_success: ${metrics.first_pass_compile_success ?? "N/A"}`);
  console.log("");
  console.log("TOKENS & COST");
  console.log(`  token_input_total: ${metrics.token_input_total}`);
  console.log(`  token_output_total: ${metrics.token_output_total}`);
  console.log(`  token_turn_count: ${metrics.token_turn_count}`);
  console.log(`  ledger_health: ${metrics.ledger_health}`);
  console.log(`  budget_status: ${metrics.budget_status}`);
  console.log(`  cost_estimate: ${metrics.cost_estimate ?? "N/A"}`);
}

function printComparisonTable(comparison, wpIdA, wpIdB) {
  console.log(`[WP_METRICS_COMPARE] ${wpIdA} -> ${wpIdB}`);
  console.log("");
  const labelWidth = 24;
  const colWidth = 14;
  console.log(
    "Metric".padEnd(labelWidth)
    + wpIdA.slice(0, colWidth).padStart(colWidth)
    + wpIdB.slice(0, colWidth).padStart(colWidth)
    + "Delta".padStart(colWidth)
    + "Trend".padStart(8),
  );
  console.log("-".repeat(labelWidth + colWidth * 3 + 8));
  for (const row of comparison) {
    const a = row.wp_a != null ? String(row.wp_a) : "N/A";
    const b = row.wp_b != null ? String(row.wp_b) : "N/A";
    const d = row.delta != null ? (row.delta > 0 ? `+${row.delta}` : String(row.delta)) : "";
    console.log(
      row.metric.padEnd(labelWidth)
      + a.padStart(colWidth)
      + b.padStart(colWidth)
      + d.padStart(colWidth)
      + row.trend.padStart(8),
    );
  }
}

function main() {
  const args = process.argv.slice(2).map((a) => String(a || "").trim()).filter(Boolean);
  const jsonMode = args.includes("--json");
  const compareMode = args.includes("--compare");
  const wpArgs = args.filter((a) => /^WP-/i.test(a));

  if (compareMode) {
    if (wpArgs.length < 2) {
      fail("Usage: node wp-metrics.mjs --compare WP-A WP-B [--json]");
    }
    const metricsA = buildWpMetrics(wpArgs[0]);
    const metricsB = buildWpMetrics(wpArgs[1]);
    if (jsonMode) {
      const comparison = buildComparisonTable(metricsA, metricsB);
      console.log(JSON.stringify({ wp_a: metricsA, wp_b: metricsB, comparison }, null, 2));
    } else {
      const comparison = buildComparisonTable(metricsA, metricsB);
      printComparisonTable(comparison, wpArgs[0], wpArgs[1]);
    }
    return;
  }

  if (wpArgs.length < 1) {
    fail("Usage: node wp-metrics.mjs WP-{ID} [--json] [--compare WP-A WP-B]");
  }
  const metrics = buildWpMetrics(wpArgs[0]);
  if (jsonMode) {
    console.log(JSON.stringify(metrics, null, 2));
  } else {
    printTextMetrics(metrics);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}

export { buildWpMetrics, buildComparisonTable };
