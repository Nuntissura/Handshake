#!/usr/bin/env node

import {
  importWorkPacketContracts,
  listWorkPacketIdsForContractImport,
} from "../lib/work-packet-contract-read-lib.mjs";

const target = String(process.argv[2] || "").trim();
const dryRun = process.argv.includes("--dry-run");
const repair = !process.argv.includes("--no-repair");

if (!target || (!target.startsWith("WP-") && target !== "--all")) {
  console.error("Usage: node .GOV/roles_shared/scripts/wp/wp-contract-import.mjs WP-{ID}|--all [--dry-run] [--no-repair]");
  process.exit(2);
}

const wpIds = target === "--all" ? listWorkPacketIdsForContractImport() : [target];
const results = [];
let failed = false;

for (const wpId of wpIds) {
  const result = importWorkPacketContracts(wpId, { dryRun, repair });
  results.push(result);
  if (!result.ok) failed = true;
  const actionSummary = Array.isArray(result.actions) && result.actions.length > 0
    ? result.actions.map((entry) => `${entry.kind}:${entry.path}`).join(" | ")
    : "NOOP";
  console.log(`${result.ok ? "OK" : "SKIP"} ${wpId} ${actionSummary}`);
  if (result.reason) console.log(`- reason: ${result.reason}`);
}

console.log(JSON.stringify({
  schema_id: "hsk.wp_contract_import_result@1",
  dry_run: dryRun,
  repair,
  wp_count: wpIds.length,
  failed,
  results,
}, null, 2));

process.exit(failed ? 1 : 0);
