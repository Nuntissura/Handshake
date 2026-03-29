#!/usr/bin/env node

import { evaluateWpTokenBudget } from "./wp-token-budget-lib.mjs";
import { readWpTokenUsageLedger } from "./wp-token-usage-lib.mjs";

const repoRoot = process.cwd();
const wpId = String(process.argv[2] || "").trim();

function fail(message) {
  console.error(`[WP_TOKEN_USAGE] ${message}`);
  process.exit(1);
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
console.log(`- ledger_health: ${ledger.ledger_health.status}`);
console.log(`- command_count: ${ledger.summary.command_count}`);
console.log(`- turn_count: ${ledger.summary.turn_count}`);
console.log(`- input_tokens: ${ledger.summary.usage_totals.input_tokens}`);
console.log(`- cached_input_tokens: ${ledger.summary.usage_totals.cached_input_tokens}`);
console.log(`- output_tokens: ${ledger.summary.usage_totals.output_tokens}`);
if (ledger.ledger_health.status !== "NO_OUTPUTS") {
  console.log(`- tracked_command_count: ${ledger.tracked_summary.command_count}`);
  console.log(`- tracked_turn_count: ${ledger.tracked_summary.turn_count}`);
  console.log(`- raw_output_command_count: ${ledger.raw_scan.summary.command_count}`);
  console.log(`- raw_output_turn_count: ${ledger.raw_scan.summary.turn_count}`);
}
if (ledger.ledger_health.status === "DRIFT") {
  console.log(`- drift_reason: ${ledger.ledger_health.reason}`);
  if (ledger.ledger_health.missing_tracked_command_count > 0) {
    console.log(`- missing_tracked_command_count: ${ledger.ledger_health.missing_tracked_command_count}`);
    console.log(`- missing_tracked_command_ids_sample: ${ledger.ledger_health.missing_tracked_command_ids_sample.join(", ")}`);
  }
  if (ledger.ledger_health.stale_tracked_command_count > 0) {
    console.log(`- stale_tracked_command_count: ${ledger.ledger_health.stale_tracked_command_count}`);
    console.log(`- stale_tracked_command_ids_sample: ${ledger.ledger_health.stale_tracked_command_ids_sample.join(", ")}`);
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
    console.log(`  output_tokens: ${totals.usage_totals.output_tokens}`);
  }
}

console.log("");
console.log("WP_TOKEN_BUDGET");
console.log(`- policy_id: ${budget.policy_id}`);
console.log(`- status: ${budget.status}`);
console.log(`- blocker_class: ${budget.blocker_class}`);
if (budget.invalidity_code) {
  console.log(`- invalidity_code: ${budget.invalidity_code}`);
}
console.log(`- summary: ${budget.summary}`);
console.log(`- total_warn_turn_count: ${budget.total.budgets.warn_turn_count}`);
console.log(`- total_fail_turn_count: ${budget.total.budgets.fail_turn_count}`);
console.log(`- total_warn_input_tokens: ${budget.total.budgets.warn_input_tokens}`);
console.log(`- total_fail_input_tokens: ${budget.total.budgets.fail_input_tokens}`);
for (const roleName of Object.keys(budget.roles || {}).sort((left, right) => left.localeCompare(right))) {
  const evaluation = budget.roles[roleName];
  console.log(`- role: ${roleName}`);
  console.log(`  status: ${evaluation.status}`);
  console.log(`  warn_turn_count: ${evaluation.budgets.warn_turn_count}`);
  console.log(`  fail_turn_count: ${evaluation.budgets.fail_turn_count}`);
  console.log(`  warn_input_tokens: ${evaluation.budgets.warn_input_tokens}`);
  console.log(`  fail_input_tokens: ${evaluation.budgets.fail_input_tokens}`);
}
if (budget.warnings.length > 0) {
  console.log(`- warnings: ${budget.warnings.join(" | ")}`);
}
if (budget.failures.length > 0) {
  console.log(`- failures: ${budget.failures.join(" | ")}`);
}
