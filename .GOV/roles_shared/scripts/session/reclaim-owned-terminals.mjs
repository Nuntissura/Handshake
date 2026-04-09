#!/usr/bin/env node

import { REPO_ROOT } from "../lib/runtime-paths.mjs";
import { loadSessionRegistry } from "./session-registry-lib.mjs";
import { reclaimOwnedSessionTerminals } from "./terminal-ownership-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("reclaim-owned-terminals.mjs", { role: "SHARED" });

function fail(message) {
  failWithMemory("reclaim-owned-terminals.mjs", message, { role: "SHARED" });
}

const wpId = String(process.argv[2] || "").trim();
let role = String(process.argv[3] || "").trim().toUpperCase();
let batchSelectorArg = String(process.argv[4] || "CURRENT_BATCH").trim();
const roleLooksLikeBatchSelector = (value) => {
  const token = String(value || "").trim().toUpperCase();
  return token === "CURRENT_BATCH" || token === "ALL_BATCHES" || /^TBATCH-/.test(token);
};

if (!wpId || !/^WP-/.test(wpId)) {
  fail("Usage: node .GOV/roles_shared/scripts/session/reclaim-owned-terminals.mjs <WP_ID> [ACTIVATION_MANAGER|CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR] [CURRENT_BATCH|ALL_BATCHES|<BATCH_ID>]");
}
if (role && !["ACTIVATION_MANAGER", "CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(role)) {
  if (roleLooksLikeBatchSelector(role)) {
    batchSelectorArg = String(process.argv[4] || role).trim();
    role = "";
  } else if (!String(process.argv[4] || "").trim()) {
    batchSelectorArg = role;
    role = "";
  } else {
    fail(`Invalid role selector: ${role}`);
  }
}

const { registry } = loadSessionRegistry(REPO_ROOT);
const normalizedBatchSelector = String(batchSelectorArg || "CURRENT_BATCH").trim().toUpperCase();
const selector = {
  wpId,
  role: role || "",
};
let selectedBatchId = "";
if (normalizedBatchSelector !== "ALL_BATCHES") {
  selectedBatchId = normalizedBatchSelector === "CURRENT_BATCH"
    ? String(registry.active_terminal_batch_id || "").trim().toUpperCase()
    : normalizedBatchSelector;
  if (!selectedBatchId) {
    fail("No active terminal batch id is recorded in the session registry");
  }
  selector.terminalBatchId = selectedBatchId;
}

const results = reclaimOwnedSessionTerminals(REPO_ROOT, selector);

console.log(`[SESSION_RECLAIM_TERMINALS] wp_id=${wpId}`);
console.log(`[SESSION_RECLAIM_TERMINALS] role=${role || "<all>"}`);
console.log(`[SESSION_RECLAIM_TERMINALS] batch_selector=${normalizedBatchSelector}`);
console.log(`[SESSION_RECLAIM_TERMINALS] active_terminal_batch_id=${registry.active_terminal_batch_id || "<none>"}`);
console.log(`[SESSION_RECLAIM_TERMINALS] selected_terminal_batch_id=${selectedBatchId || "<all_batches>"}`);
console.log(`[SESSION_RECLAIM_TERMINALS] reclaimed_count=${results.length}`);
for (const entry of results) {
  console.log(`[SESSION_RECLAIM_TERMINALS] session_key=${entry.session_key}`);
  console.log(`[SESSION_RECLAIM_TERMINALS] process_id=${entry.process_id}`);
  console.log(`[SESSION_RECLAIM_TERMINALS] terminal_batch_id=${entry.terminal_batch_id || "<none>"}`);
  console.log(`[SESSION_RECLAIM_TERMINALS] reclaim_status=${entry.reclaim_status}`);
  if (entry.error) {
    console.log(`[SESSION_RECLAIM_TERMINALS] error=${entry.error}`);
  }
}
