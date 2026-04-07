#!/usr/bin/env node
/**
 * Memory refresh: extract new memories + tiered maintenance.
 *
 * Called at every role startup (orchestrator, coder, validator) to keep the
 * memory DB current. Staleness gates prevent redundant work — whichever role
 * starts first in a period handles the maintenance.
 *
 * Steps:
 * 1. Extract new memories from receipts (all WPs, idempotent)
 * 2. Extract new memories from smoketest reviews (idempotent)
 * 3. Tiered maintenance (staleness-gated):
 *    - Dedup:      runs if >6h since last compaction (daily target)
 *    - Compaction:  runs if >24h since last compaction (weekly-ish target)
 *    - Full cycle:  runs if >7d since last compaction (includes decay + orphan cleanup)
 *
 * Usage:
 *   node memory-refresh.mjs [--force-compact]
 */

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  openGovernanceMemoryDb,
  closeDb,
} from "./governance-memory-lib.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const forceCompact = process.argv.includes("--force-compact");

const DEDUP_STALENESS_MS = 6 * 60 * 60 * 1000;      // 6 hours
const COMPACT_STALENESS_MS = 24 * 60 * 60 * 1000;    // 24 hours

function runScript(scriptPath, args = []) {
  try {
    execFileSync(process.execPath, [scriptPath, ...args], {
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 30000,
    });
    return true;
  } catch (e) {
    const stderr = e.stderr ? e.stderr.toString().trim() : e.message;
    console.error(`  [memory-refresh] warn: ${path.basename(scriptPath)} — ${stderr}`);
    return false;
  }
}

// ---------------------------------------------------------------------------
// 1. Extract from receipts (always, idempotent)
// ---------------------------------------------------------------------------
console.log("[memory-refresh] Extracting from receipts...");
const receiptOk = runScript(path.join(__dirname, "memory-extract-from-receipts.mjs"), ["--all"]);

// ---------------------------------------------------------------------------
// 2. Extract from smoketests (always, idempotent)
// ---------------------------------------------------------------------------
console.log("[memory-refresh] Extracting from smoketests...");
const smoketestOk = runScript(path.join(__dirname, "memory-extract-from-smoketests.mjs"));

// ---------------------------------------------------------------------------
// 3. Tiered maintenance (staleness-gated)
// ---------------------------------------------------------------------------
let maintenanceResult = "skipped";
try {
  const { db } = openGovernanceMemoryDb();
  try {
    const last = db.prepare(
      "SELECT run_at FROM consolidation_log ORDER BY run_at DESC LIMIT 1"
    ).get();

    const lastRunMs = last ? new Date(last.run_at).getTime() : 0;
    const sinceLastMs = Date.now() - lastRunMs;
    const hoursAgo = (sinceLastMs / (1000 * 60 * 60)).toFixed(1);

    // RGF-134: dual-gate — compaction needs BOTH time AND activity thresholds
    const ACTIVITY_THRESHOLD = 5;
    const newEntriesSinceCompact = last
      ? db.prepare("SELECT COUNT(*) as cnt FROM memory_index WHERE created_at > ?").get(last.run_at)?.cnt || 0
      : Infinity;
    const activityGateMet = newEntriesSinceCompact >= ACTIVITY_THRESHOLD;

    if (forceCompact || (sinceLastMs > COMPACT_STALENESS_MS && activityGateMet)) {
      // Full compaction: dedup + consolidation + decay + orphan cleanup
      console.log(`[memory-refresh] Full compaction ${forceCompact ? "(forced)" : `(stale ${hoursAgo}h, ${newEntriesSinceCompact} new entries)`} — running...`);
      closeDb(db);
      if (runScript(path.join(__dirname, "memory-compact.mjs"))) {
        maintenanceResult = "full-compact";
      }
    } else if (sinceLastMs > DEDUP_STALENESS_MS && newEntriesSinceCompact > 0) {
      // Light maintenance: dedup only (quick pass), only if new entries exist
      console.log(`[memory-refresh] Light dedup (${hoursAgo}h, ${newEntriesSinceCompact} new entries)...`);
      // Run compact with very short consolidation window so only dedup + orphans run
      closeDb(db);
      if (runScript(path.join(__dirname, "memory-compact.mjs"), ["--older-than", "999d"])) {
        maintenanceResult = "light-dedup";
      }
    } else {
      console.log(`[memory-refresh] Maintenance skipped (last run ${hoursAgo}h ago)`);
      closeDb(db);
    }
  } catch {
    closeDb(db);
  }
} catch {
  console.log("[memory-refresh] Maintenance skipped (DB not yet initialized)");
}

console.log(`[memory-refresh] Done. receipts=${receiptOk ? "ok" : "warn"} smoketests=${smoketestOk ? "ok" : "warn"} maintenance=${maintenanceResult}`);
