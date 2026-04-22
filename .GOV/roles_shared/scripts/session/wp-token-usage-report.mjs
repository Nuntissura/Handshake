#!/usr/bin/env node

import { evaluateWpTokenBudget } from "./wp-token-budget-lib.mjs";
import { readWpTokenUsageLedger } from "./wp-token-usage-lib.mjs";
import { REPO_ROOT } from "../lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("wp-token-usage-report.mjs", { role: "SHARED" });

const repoRoot = REPO_ROOT;
const wpId = String(process.argv[2] || "").trim();

function fail(message) {
  failWithMemory("wp-token-usage-report.mjs", message, { role: "SHARED" });
}

if (!wpId || !/^WP-/.test(wpId)) {
  fail("Usage: just wp-token-usage WP-{ID}");
}

const { filePath, ledger } = readWpTokenUsageLedger(repoRoot, wpId);
const budget = evaluateWpTokenBudget(ledger);

console.log("WP_TOKEN_USAGE");
console.log(`- wp_id: ${ledger.wp_id}`);
console.log(`- ledger_path: ${filePath.replace(/\\/g, "/")}`);
console.log(`- updated_at: ${ledger.updated_at}`);
console.log(`- summary_source: ${ledger.summary_source}`);
console.log(`- settlement_status: ${ledger.settlement?.status || "UNSETTLED"}`);
if (ledger.settlement?.settled_at) console.log(`- settled_at: ${ledger.settlement.settled_at}`);
if (ledger.settlement?.settled_reason) console.log(`- settled_reason: ${ledger.settlement.settled_reason}`);
if (ledger.settlement?.settled_by) console.log(`- settled_by: ${ledger.settlement.settled_by}`);
console.log(`- ledger_health: ${ledger.ledger_health.status}`);
console.log(`- ledger_health_severity: ${ledger.ledger_health.severity}`);
console.log(`- ledger_health_drift_class: ${ledger.ledger_health.drift_class}`);
console.log(`- ledger_health_policy_id: ${ledger.ledger_health.policy_id}`);
console.log(`- ledger_health_blocker_class: ${ledger.ledger_health.blocker_class}`);
if (ledger.ledger_health.invalidity_code) {
  console.log(`- ledger_health_invalidity_code: ${ledger.ledger_health.invalidity_code}`);
}
console.log(`- ledger_health_summary: ${ledger.ledger_health.summary}`);
console.log(`- command_count: ${ledger.summary.command_count}`);
console.log(`- turn_count: ${ledger.summary.turn_count}`);
console.log(`- input_tokens: ${ledger.summary.usage_totals.input_tokens}`);
console.log(`- cached_input_tokens: ${ledger.summary.usage_totals.cached_input_tokens}`);
console.log(`- fresh_input_tokens: ${Math.max(0, Number(ledger.summary.usage_totals.input_tokens || 0) - Number(ledger.summary.usage_totals.cached_input_tokens || 0))}`);
console.log(`- output_tokens: ${ledger.summary.usage_totals.output_tokens}`);
if (ledger.ledger_health.status !== "NO_OUTPUTS") {
  console.log(`- tracked_command_count: ${ledger.tracked_summary.command_count}`);
  console.log(`- tracked_turn_count: ${ledger.tracked_summary.turn_count}`);
  console.log(`- raw_output_command_count: ${ledger.raw_scan.summary.command_count}`);
  console.log(`- raw_output_turn_count: ${ledger.raw_scan.summary.turn_count}`);
}
if (ledger.ledger_health.status === "DRIFT") {
  console.log(`- drift_reason: ${ledger.ledger_health.reason}`);
  console.log(`- command_delta_count: ${ledger.ledger_health.metrics.command_delta_count}`);
  console.log(`- turn_delta: ${ledger.ledger_health.metrics.turn_delta}`);
  console.log(`- input_token_delta: ${ledger.ledger_health.metrics.input_token_delta}`);
  console.log(`- input_token_delta_ratio_pct: ${ledger.ledger_health.metrics.input_token_delta_ratio_pct}`);
  if (ledger.ledger_health.missing_tracked_command_count > 0) {
    console.log(`- missing_tracked_command_count: ${ledger.ledger_health.missing_tracked_command_count}`);
    console.log(`- missing_tracked_command_ids_sample: ${ledger.ledger_health.missing_tracked_command_ids_sample.join(", ")}`);
  }
  if (ledger.ledger_health.stale_tracked_command_count > 0) {
    console.log(`- stale_tracked_command_count: ${ledger.ledger_health.stale_tracked_command_count}`);
    console.log(`- stale_tracked_command_ids_sample: ${ledger.ledger_health.stale_tracked_command_ids_sample.join(", ")}`);
  }
  if (ledger.ledger_health.warnings.length > 0) {
    console.log(`- ledger_health_warnings: ${ledger.ledger_health.warnings.join(" | ")}`);
  }
  if (ledger.ledger_health.failures.length > 0) {
    console.log(`- ledger_health_failures: ${ledger.ledger_health.failures.join(" | ")}`);
  }
}

const roleNames = Object.keys(ledger.role_totals || {}).sort((left, right) => left.localeCompare(right));
if (roleNames.length === 0) {
  console.log("- role_totals: <none>");
} else {
  for (const roleName of roleNames) {
    const totals = ledger.role_totals[roleName];
    console.log(`- role: ${roleName}`);
    console.log(`  command_count: ${totals.command_count}`);
    console.log(`  turn_count: ${totals.turn_count}`);
    console.log(`  input_tokens: ${totals.usage_totals.input_tokens}`);
    console.log(`  cached_input_tokens: ${totals.usage_totals.cached_input_tokens}`);
    console.log(`  fresh_input_tokens: ${Math.max(0, Number(totals.usage_totals.input_tokens || 0) - Number(totals.usage_totals.cached_input_tokens || 0))}`);
    console.log(`  output_tokens: ${totals.usage_totals.output_tokens}`);
  }
}

console.log("");
console.log("WP_TOKEN_BUDGET");
console.log(`- policy_id: ${budget.policy_id}`);
console.log(`- enforcement_mode: ${budget.enforcement_mode}`);
console.log(`- status: ${budget.status}`);
console.log(`- blocker_class: ${budget.blocker_class}`);
if (budget.invalidity_code) {
  console.log(`- invalidity_code: ${budget.invalidity_code}`);
}
console.log(`- summary: ${budget.summary}`);
console.log(`- total_warn_turn_count: ${budget.total.budgets.warn_turn_count}`);
console.log(`- total_fail_turn_count: ${budget.total.budgets.fail_turn_count}`);
console.log(`- total_warn_fresh_input_tokens: ${budget.total.budgets.warn_input_tokens}`);
console.log(`- total_fail_fresh_input_tokens: ${budget.total.budgets.fail_input_tokens}`);
for (const roleName of Object.keys(budget.roles || {}).sort((left, right) => left.localeCompare(right))) {
  const evaluation = budget.roles[roleName];
  console.log(`- role: ${roleName}`);
  console.log(`  status: ${evaluation.status}`);
  console.log(`  warn_turn_count: ${evaluation.budgets.warn_turn_count}`);
  console.log(`  fail_turn_count: ${evaluation.budgets.fail_turn_count}`);
  console.log(`  warn_fresh_input_tokens: ${evaluation.budgets.warn_input_tokens}`);
  console.log(`  fail_fresh_input_tokens: ${evaluation.budgets.fail_input_tokens}`);
  console.log(`  gross_input_tokens: ${evaluation.input_tokens}`);
  console.log(`  cached_input_tokens: ${evaluation.cached_input_tokens}`);
  console.log(`  fresh_input_tokens: ${evaluation.fresh_input_tokens}`);
}
if (budget.warnings.length > 0) {
  console.log(`- warnings: ${budget.warnings.join(" | ")}`);
}
if (budget.failures.length > 0) {
  console.log(`- failures: ${budget.failures.join(" | ")}`);
}
