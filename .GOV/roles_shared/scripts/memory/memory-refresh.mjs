#!/usr/bin/env node
/**
 * memory-refresh — Startup memory maintenance.
 *
 * Runs at every role startup (orchestrator, coder, validator) and during gov-check.
 * Idempotent — safe to run multiple times.
 *
 * Phases:
 *   1. EXTRACT: run receipt extraction (--all) for any new receipts
 *   2. EXTRACT: run smoketest extraction for any new smoketest files
 *   3. COMPACT: run decay + dedup if stale (>24h since last compaction AND >5 new entries)
 *
 * Options:
 *   --force-compact    Bypass staleness check for compaction
 *   --skip-extract     Skip extraction phases (compact only)
 *   --dry-run          Show what would happen without writing
 */

import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  openGovernanceMemoryDb,
  closeDb,
  getStats,
  runDecay,
} from "./governance-memory-lib.mjs";
import { GOV_ROOT_ABS } from "../lib/runtime-paths.mjs";

const args = process.argv.slice(2);
const forceCompact = args.includes("--force-compact");
const skipExtract = args.includes("--skip-extract");
const dryRun = args.includes("--dry-run");

const SCRIPTS_DIR = path.join(GOV_ROOT_ABS, "roles_shared", "scripts", "memory");
const COMPACT_STALENESS_HOURS = 24;
const COMPACT_ACTIVITY_THRESHOLD = 5;

function runScript(scriptName, scriptArgs = []) {
  const scriptPath = path.join(SCRIPTS_DIR, scriptName);
  const result = spawnSync(process.execPath, [scriptPath, ...scriptArgs], {
    stdio: ["pipe", "pipe", "pipe"],
    timeout: 30000,
  });
  const stdout = result.stdout?.toString() || "";
  const stderr = result.stderr?.toString() || "";
  // Filter out ExperimentalWarning noise
  const filteredStderr = stderr.split("\n")
    .filter(l => !l.includes("ExperimentalWarning") && !l.includes("--trace-warnings"))
    .join("\n").trim();
  return { ok: result.status === 0, stdout: stdout.trim(), stderr: filteredStderr };
}

// ---------------------------------------------------------------------------
// Phase 1 + 2: Extraction
// ---------------------------------------------------------------------------

let extractedReceipts = 0;
let extractedSmoketests = 0;

if (!skipExtract) {
  if (!dryRun) {
    const receiptResult = runScript("memory-extract-from-receipts.mjs", ["--all"]);
    if (receiptResult.ok) {
      const match = receiptResult.stdout.match(/extracted\s+(\d+)/i);
      extractedReceipts = match ? Number(match[1]) : 0;
    }

    const smoketestResult = runScript("memory-extract-from-smoketests.mjs");
    if (smoketestResult.ok) {
      const match = smoketestResult.stdout.match(/extracted\s+(\d+)/i);
      extractedSmoketests = match ? Number(match[1]) : 0;
    }
  } else {
    console.log("[memory-refresh] DRY_RUN: would extract from receipts and smoketests");
  }
}

// ---------------------------------------------------------------------------
// Phase 3: Compaction (dual-gate: staleness + activity)
// ---------------------------------------------------------------------------

const { db } = openGovernanceMemoryDb();
let compacted = false;

try {
  const stats = getStats(db);
  const lastCompactionTime = stats.last_compaction?.run_at
    ? new Date(stats.last_compaction.run_at).getTime()
    : 0;
  const hoursSinceCompaction = (Date.now() - lastCompactionTime) / (1000 * 60 * 60);
  const activityCount = stats.active || 0;

  const shouldCompact = forceCompact
    || (hoursSinceCompaction > COMPACT_STALENESS_HOURS && activityCount > COMPACT_ACTIVITY_THRESHOLD);

  if (shouldCompact) {
    if (!dryRun) {
      const result = runDecay(db, { decayRate: 0.1, pruneThreshold: 0.05 });
      compacted = true;
      console.log(`[memory-refresh] Compaction: processed=${result.processed}, decayed=${result.decayed}, pruned=${result.pruned}`);
    } else {
      console.log(`[memory-refresh] DRY_RUN: would compact (${hoursSinceCompaction.toFixed(1)}h since last, ${activityCount} active entries)`);
    }
  }

  // Summary
  const finalStats = getStats(db);
  console.log(`MEMORY_REFRESH`);
  console.log(`  active_entries: ${finalStats.active}`);
  console.log(`  by_type: ${Object.entries(finalStats.by_type).map(([k, v]) => `${k}=${v}`).join(", ")}`);
  console.log(`  extracted: receipts=${extractedReceipts}, smoketests=${extractedSmoketests}`);
  console.log(`  compacted: ${compacted ? "yes" : "no (not stale)"}`);
  if (finalStats.last_compaction) {
    console.log(`  last_compaction: ${finalStats.last_compaction.run_at}`);
  }
} finally {
  closeDb(db);
}
