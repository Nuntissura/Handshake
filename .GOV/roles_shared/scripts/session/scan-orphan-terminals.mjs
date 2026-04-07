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
const STALE_ACTIVE_TERMINAL_AGE_MS = 2 * 60 * 60 * 1000; // 2 hours
const TERMINAL_ACTIVE_RUNTIME_STATES = new Set([
  "UNSTARTED",
  "PLUGIN_REQUESTED",
  "TERMINAL_COMMAND_DISPATCHED",
  "PLUGIN_CONFIRMED",
  "CLI_ESCALATION_READY",
  "CLI_ESCALATION_USED",
  "STARTING",
  "READY",
  "COMMAND_RUNNING",
  "ACTIVE",
  "WAITING",
]);

// Terminal states where the session is done and the terminal should be reclaimed.
const TERMINAL_SESSION_STATUSES = new Set([
  "COMPLETED",
  "FAILED",
  "CANCELLED",
  "CANCELED",
  "TIMED_OUT",
  "TIMEDOUT",
  "ERROR",
  "SETTLED",
]);

const TERMINAL_RUNTIME_STATES = new Set([
  "FAILED",
  "CLOSED",
]);

function isOrphanTerminalCandidate(session) {
  const sessionStatus = String(session?.session_status || session?.status || session?.runtime_state || "").trim().toUpperCase();
  if (TERMINAL_SESSION_STATUSES.has(sessionStatus)) return true;

  const commandStatus = String(session?.last_command_status || "").trim().toUpperCase();
  if (TERMINAL_SESSION_STATUSES.has(commandStatus)) return true;

  const runtimeState = String(session?.runtime_state || "").trim().toUpperCase();
  if (TERMINAL_RUNTIME_STATES.has(runtimeState)) return true;

  return false;
}

const orphanCandidates = (registry.sessions || []).filter((session) => {
  if (String(session.terminal_ownership_scope || "") !== SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION) return false;
  if (!Number.isInteger(session.owned_terminal_process_id) || session.owned_terminal_process_id <= 0) return false;
  if (String(session.owned_terminal_reclaim_status || "") === SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED) return false;
  return true;
});

function parseTimestampMs(value) {
  const numeric = Date.parse(String(value || "").trim());
  return Number.isNaN(numeric) ? NaN : numeric;
}

function latestTimestampMs(session) {
  const candidates = [
    parseTimestampMs(session.last_heartbeat_at),
    parseTimestampMs(session.last_event_at),
    parseTimestampMs(session.owned_terminal_recorded_at),
  ];
  const normalized = candidates.filter((entry) => Number.isFinite(entry));
  return normalized.length === 0 ? NaN : Math.max(...normalized);
}

function isStaleActiveTerminalSession(session) {
  const runtimeState = String(session?.runtime_state || "").trim().toUpperCase();
  if (!TERMINAL_ACTIVE_RUNTIME_STATES.has(runtimeState)) return false;
  const latestActivity = latestTimestampMs(session);
  if (Number.isNaN(latestActivity)) return false;
  return (Date.now() - latestActivity) > STALE_ACTIVE_TERMINAL_AGE_MS;
}

// Auto-reclaim terminals for sessions in terminal states (completed/failed/cancelled)
// regardless of batch — these are done and should never keep a window open
const completedWithOpenTerminals = orphanCandidates.filter((session) => {
  return isOrphanTerminalCandidate(session);
});

if (completedWithOpenTerminals.length > 0 && !dryRun) {
  for (const s of completedWithOpenTerminals) {
    const results = reclaimOwnedSessionTerminals(REPO_ROOT, { sessionKey: s.session_key });
    for (const r of results) {
      console.log(
        `ORPHAN_TERMINAL_SCAN AUTO_RECLAIM_COMPLETED session_key=${r.session_key} pid=${r.process_id} status=${r.reclaim_status}${r.error ? ` error=${r.error}` : ""}`,
      );
    }
  }
  console.log(`ORPHAN_TERMINAL_SCAN auto_reclaimed_completed=${completedWithOpenTerminals.length}`);
} else if (completedWithOpenTerminals.length > 0) {
  console.log(`ORPHAN_TERMINAL_SCAN --dry-run: would auto-reclaim ${completedWithOpenTerminals.length} completed/failed session terminal(s)`);
}

const currentBatch = orphanCandidates.filter(
  (s) => activeBatchId && String(s.owned_terminal_batch_id || "").toUpperCase() === activeBatchId,
);
const staleBatch = orphanCandidates.filter(
  (s) => !activeBatchId || String(s.owned_terminal_batch_id || "").toUpperCase() !== activeBatchId,
);

const staleActiveWithOpenTerminals = orphanCandidates.filter((session) => {
  if (isOrphanTerminalCandidate(session)) return false;
  return isStaleActiveTerminalSession(session);
});

const staleCandidates = (() => {
  const targetByKey = new Map();
  for (const s of [...staleBatch, ...staleActiveWithOpenTerminals]) {
    if (!s?.session_key) continue;
    targetByKey.set(s.session_key, s);
  }
  return Array.from(targetByKey.values());
})();

if (orphanCandidates.length === 0 && staleCandidates.length === 0) {
  console.log("ORPHAN_TERMINAL_SCAN ok: no unreturned owned terminals");
  process.exit(0);
}

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
  if (staleActiveWithOpenTerminals.length === 0) {
    console.log("ORPHAN_TERMINAL_SCAN no stale-batch terminals to reclaim");
    process.exit(0);
  }
  console.log(`ORPHAN_TERMINAL_SCAN stale-active terminals to reclaim: ${staleActiveWithOpenTerminals.length}`);
}

if (staleCandidates.length === 0) {
  process.exit(0);
}

for (const s of staleCandidates) {
  const reason = staleBatch.some((batchSession) => String(batchSession.session_key || "") === String(s.session_key || ""))
    ? "STALE_BATCH"
    : isStaleActiveTerminalSession(s)
      ? "STALE_ACTIVE"
      : "STALE";
  const ageMs = latestTimestampMs(s);
  const staleAgeMs = Number.isFinite(ageMs) ? Math.max(0, Date.now() - ageMs) : null;
  console.log(
    `ORPHAN_TERMINAL_SCAN ${reason} session_key=${s.session_key} pid=${s.owned_terminal_process_id} batch=${s.owned_terminal_batch_id || "<none>"} age_ms=${staleAgeMs ?? "unknown"} title=${s.owned_terminal_window_title || "<none>"}`,
  );
}

if (dryRun) {
  console.log(`ORPHAN_TERMINAL_SCAN --dry-run: would reclaim ${staleCandidates.length} terminal(s)`);
  process.exit(0);
}

const staleResults = [];
for (const s of staleCandidates) {
  const results = reclaimOwnedSessionTerminals(REPO_ROOT, { sessionKey: s.session_key });
  staleResults.push(...results);
}

console.log(`ORPHAN_TERMINAL_SCAN reclaimed=${staleResults.length}`);
for (const r of staleResults) {
  console.log(
    `ORPHAN_TERMINAL_SCAN RECLAIMED session_key=${r.session_key} pid=${r.process_id} batch=${r.terminal_batch_id} status=${r.reclaim_status}${r.error ? ` error=${r.error}` : ""}`,
  );
}
