#!/usr/bin/env node

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

console.log("WP_TOKEN_USAGE");
console.log(`- wp_id: ${ledger.wp_id}`);
console.log(`- ledger_path: ${filePath.replace(/\\/g, "/")}`);
console.log(`- updated_at: ${ledger.updated_at}`);
console.log(`- command_count: ${ledger.summary.command_count}`);
console.log(`- turn_count: ${ledger.summary.turn_count}`);
console.log(`- input_tokens: ${ledger.summary.usage_totals.input_tokens}`);
console.log(`- cached_input_tokens: ${ledger.summary.usage_totals.cached_input_tokens}`);
console.log(`- output_tokens: ${ledger.summary.usage_totals.output_tokens}`);

const roleNames = Object.keys(ledger.role_totals || {}).sort((left, right) => left.localeCompare(right));
if (roleNames.length === 0) {
  console.log("- role_totals: <none>");
  process.exit(0);
}

for (const roleName of roleNames) {
  const totals = ledger.role_totals[roleName];
  console.log(`- role: ${roleName}`);
  console.log(`  command_count: ${totals.command_count}`);
  console.log(`  turn_count: ${totals.turn_count}`);
  console.log(`  input_tokens: ${totals.usage_totals.input_tokens}`);
  console.log(`  cached_input_tokens: ${totals.usage_totals.cached_input_tokens}`);
  console.log(`  output_tokens: ${totals.usage_totals.output_tokens}`);
}
