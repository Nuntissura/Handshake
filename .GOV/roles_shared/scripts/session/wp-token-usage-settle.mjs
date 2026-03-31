#!/usr/bin/env node

import { settleWpTokenUsageLedger } from "./wp-token-usage-lib.mjs";
import { REPO_ROOT } from "../lib/runtime-paths.mjs";

const repoRoot = REPO_ROOT;
const wpId = String(process.argv[2] || "").trim();
const reason = String(process.argv[3] || "HISTORICAL_BACKFILL").trim() || "HISTORICAL_BACKFILL";
const settledBy = String(process.argv[4] || "SYSTEM").trim() || "SYSTEM";

if (!wpId) {
  console.error("[WP_TOKEN_USAGE_SETTLE] Usage: node .GOV/roles_shared/scripts/session/wp-token-usage-settle.mjs WP-{ID} [REASON] [SETTLED_BY]");
  process.exit(2);
}

try {
  const { filePath, ledger } = settleWpTokenUsageLedger(repoRoot, wpId, {
    reason,
    settledBy,
  });
  console.log("WP_TOKEN_USAGE_SETTLEMENT");
  console.log(`- wp_id: ${ledger.wp_id}`);
  console.log(`- file: ${filePath}`);
  console.log(`- status: ${ledger.settlement.status}`);
  console.log(`- settled_at: ${ledger.settlement.settled_at}`);
  console.log(`- settled_reason: ${ledger.settlement.settled_reason}`);
  console.log(`- settled_by: ${ledger.settlement.settled_by}`);
  console.log(`- previous_health_status: ${ledger.settlement.previous_health_status || "<none>"}`);
  console.log(`- previous_health_severity: ${ledger.settlement.previous_health_severity || "<none>"}`);
  console.log(`- summary_source: ${ledger.summary_source}`);
  console.log(`- command_count: ${ledger.summary.command_count}`);
  console.log(`- turn_count: ${ledger.summary.turn_count}`);
  console.log(`- input_tokens: ${ledger.summary.usage_totals.input_tokens}`);
  console.log(`- cached_input_tokens: ${ledger.summary.usage_totals.cached_input_tokens}`);
  console.log(`- output_tokens: ${ledger.summary.usage_totals.output_tokens}`);
  console.log(`- ledger_health_status: ${ledger.ledger_health.status}`);
  console.log(`- ledger_health_severity: ${ledger.ledger_health.severity}`);
  console.log(`- ledger_health_drift_class: ${ledger.ledger_health.drift_class}`);
} catch (error) {
  console.error(`[WP_TOKEN_USAGE_SETTLE] ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
