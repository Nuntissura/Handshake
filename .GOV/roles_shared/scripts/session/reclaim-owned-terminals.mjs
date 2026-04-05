#!/usr/bin/env node

import { REPO_ROOT } from "../lib/runtime-paths.mjs";
import { reclaimOwnedSessionTerminals } from "./terminal-ownership-lib.mjs";

function fail(message) {
  console.error(`[SESSION_RECLAIM_TERMINALS] ${message}`);
  process.exit(1);
}

const wpId = String(process.argv[2] || "").trim();
const role = String(process.argv[3] || "").trim().toUpperCase();

if (!wpId || !/^WP-/.test(wpId)) {
  fail("Usage: node .GOV/roles_shared/scripts/session/reclaim-owned-terminals.mjs <WP_ID> [CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR]");
}
if (role && !["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(role)) {
  fail(`Invalid role selector: ${role}`);
}

const results = reclaimOwnedSessionTerminals(REPO_ROOT, {
  wpId,
  role: role || "",
});

console.log(`[SESSION_RECLAIM_TERMINALS] wp_id=${wpId}`);
console.log(`[SESSION_RECLAIM_TERMINALS] role=${role || "<all>"}`);
console.log(`[SESSION_RECLAIM_TERMINALS] reclaimed_count=${results.length}`);
for (const entry of results) {
  console.log(`[SESSION_RECLAIM_TERMINALS] session_key=${entry.session_key}`);
  console.log(`[SESSION_RECLAIM_TERMINALS] process_id=${entry.process_id}`);
  console.log(`[SESSION_RECLAIM_TERMINALS] reclaim_status=${entry.reclaim_status}`);
  if (entry.error) {
    console.log(`[SESSION_RECLAIM_TERMINALS] error=${entry.error}`);
  }
}
