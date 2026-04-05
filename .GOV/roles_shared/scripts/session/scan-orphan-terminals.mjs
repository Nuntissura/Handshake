#!/usr/bin/env node

import { REPO_ROOT } from "../lib/runtime-paths.mjs";
import { loadSessionRegistry } from "./session-registry-lib.mjs";
import { reclaimOwnedSessionTerminals } from "./terminal-ownership-lib.mjs";
import {
  SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION,
  SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED,
} from "./session-policy.mjs";

const dryRun = process.argv.includes("--dry-run");

const { registry } = loadSessionRegistry(REPO_ROOT);
const activeBatchId = String(registry.active_terminal_batch_id || "").trim().toUpperCase();

const orphanCandidates = (registry.sessions || []).filter((session) => {
  if (String(session.terminal_ownership_scope || "") !== SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION) return false;
  if (!Number.isInteger(session.owned_terminal_process_id) || session.owned_terminal_process_id <= 0) return false;
  if (String(session.owned_terminal_reclaim_status || "") === SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED) return false;
  return true;
});

if (orphanCandidates.length === 0) {
  console.log("ORPHAN_TERMINAL_SCAN ok: no unreturned owned terminals");
  process.exit(0);
}

const currentBatch = orphanCandidates.filter(
  (s) => activeBatchId && String(s.owned_terminal_batch_id || "").toUpperCase() === activeBatchId,
);
const staleBatch = orphanCandidates.filter(
  (s) => !activeBatchId || String(s.owned_terminal_batch_id || "").toUpperCase() !== activeBatchId,
);

console.log(`ORPHAN_TERMINAL_SCAN active_batch=${activeBatchId || "<none>"}`);
console.log(`ORPHAN_TERMINAL_SCAN owned_unreturned=${orphanCandidates.length}`);
console.log(`ORPHAN_TERMINAL_SCAN current_batch_owned=${currentBatch.length}`);
console.log(`ORPHAN_TERMINAL_SCAN stale_batch_owned=${staleBatch.length}`);

for (const s of orphanCandidates) {
  const isStale = staleBatch.includes(s);
  console.log(
    `ORPHAN_TERMINAL_SCAN ${isStale ? "STALE" : "CURRENT"} session_key=${s.session_key} pid=${s.owned_terminal_process_id} batch=${s.owned_terminal_batch_id || "<none>"} title=${s.owned_terminal_window_title || "<none>"}`,
  );
}

if (staleBatch.length === 0) {
  console.log("ORPHAN_TERMINAL_SCAN no stale-batch terminals to reclaim");
  process.exit(0);
}

if (dryRun) {
  console.log(`ORPHAN_TERMINAL_SCAN --dry-run: would reclaim ${staleBatch.length} stale-batch terminal(s)`);
  process.exit(0);
}

const staleResults = [];
for (const s of staleBatch) {
  const results = reclaimOwnedSessionTerminals(REPO_ROOT, { sessionKey: s.session_key });
  staleResults.push(...results);
}

console.log(`ORPHAN_TERMINAL_SCAN reclaimed=${staleResults.length}`);
for (const r of staleResults) {
  console.log(
    `ORPHAN_TERMINAL_SCAN RECLAIMED session_key=${r.session_key} pid=${r.process_id} batch=${r.terminal_batch_id} status=${r.reclaim_status}${r.error ? ` error=${r.error}` : ""}`,
  );
}
