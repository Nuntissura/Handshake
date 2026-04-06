#!/usr/bin/env node
/**
 * RGF-123: Cron-based memory compaction and hygiene.
 *
 * Usage:
 *   node memory-compact.mjs [--older-than 30d] [--dry-run]
 *
 * Operations (in order):
 * 1. DEDUP: merge near-duplicate memory_index entries (same topic + type)
 * 2. CONSOLIDATE: group old episodic entries by WP, create semantic summaries
 * 3. DECAY: apply importance decay to all active entries, prune below threshold
 * 4. INTEGRITY: clean orphaned entries
 *
 * All operations are rule-based (no LLM needed).
 */

import fs from "node:fs";
import path from "node:path";
import {
  openGovernanceMemoryDb,
  closeDb,
  addMemory,
  runDecay,
} from "./governance-memory-lib.mjs";

function nowIso() {
  return new Date().toISOString();
}

function parseOlderThan(value) {
  const match = String(value || "30d").match(/^(\d+)([dhm])$/);
  if (!match) return 30 * 24 * 60 * 60 * 1000;
  const num = Number(match[1]);
  const unit = match[2];
  if (unit === "d") return num * 24 * 60 * 60 * 1000;
  if (unit === "h") return num * 60 * 60 * 1000;
  if (unit === "m") return num * 60 * 1000;
  return 30 * 24 * 60 * 60 * 1000;
}

function parseFlags(args) {
  const flags = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--dry-run") { flags.dryRun = true; continue; }
    if (args[i].startsWith("--") && i + 1 < args.length) {
      flags[args[i].slice(2).replace(/-/g, "_")] = args[i + 1];
      i++;
    }
  }
  return flags;
}

// ---------------------------------------------------------------------------
// 1. Deduplication
// ---------------------------------------------------------------------------

function dedup(db, dryRun) {
  const dupes = db.prepare(`
    SELECT topic, memory_type, wp_id, COUNT(*) as cnt, GROUP_CONCAT(id) as ids
    FROM memory_index
    WHERE consolidated = 0
    GROUP BY topic, memory_type, wp_id
    HAVING cnt > 1
  `).all();

  let merged = 0;
  for (const group of dupes) {
    const ids = group.ids.split(",").map(Number).sort((a, b) => a - b);
    const keepId = ids[0];
    const removeIds = ids.slice(1);

    if (dryRun) {
      console.log(`  [dedup] would merge ${removeIds.length} dupes for "${group.topic}" [${group.memory_type}] into #${keepId}`);
      merged += removeIds.length;
      continue;
    }

    const keepRow = db.prepare("SELECT importance, access_count FROM memory_index WHERE id = ?").get(keepId);
    let maxImportance = keepRow?.importance || 0;
    let totalAccess = keepRow?.access_count || 0;

    for (const rid of removeIds) {
      const row = db.prepare("SELECT importance, access_count FROM memory_index WHERE id = ?").get(rid);
      if (row) {
        maxImportance = Math.max(maxImportance, row.importance || 0);
        totalAccess += row.access_count || 0;
      }
      db.prepare("UPDATE memory_entries SET index_id = ? WHERE index_id = ?").run(keepId, rid);
      db.prepare("DELETE FROM memory_index WHERE id = ?").run(rid);
    }

    db.prepare("UPDATE memory_index SET importance = ?, access_count = ? WHERE id = ?").run(maxImportance, totalAccess, keepId);
    merged += removeIds.length;
  }
  return merged;
}

// ---------------------------------------------------------------------------
// 2. Consolidation (episodic → semantic summaries)
// ---------------------------------------------------------------------------

function consolidate(db, olderThanMs, dryRun) {
  const cutoff = new Date(Date.now() - olderThanMs).toISOString();
  const oldEpisodic = db.prepare(`
    SELECT wp_id, COUNT(*) as cnt
    FROM memory_index
    WHERE memory_type = 'episodic' AND consolidated = 0 AND created_at < ?
    GROUP BY wp_id
    HAVING cnt >= 3
  `).all(cutoff);

  let consolidated = 0;
  for (const group of oldEpisodic) {
    const entries = db.prepare(`
      SELECT id, topic, summary FROM memory_index
      WHERE memory_type = 'episodic' AND consolidated = 0 AND wp_id = ? AND created_at < ?
      ORDER BY created_at ASC
    `).all(group.wp_id, cutoff);

    const receiptKinds = {};
    for (const e of entries) {
      const kind = e.topic.split(" ")[0] || "OTHER";
      receiptKinds[kind] = (receiptKinds[kind] || 0) + 1;
    }
    const kindSummary = Object.entries(receiptKinds).map(([k, v]) => `${v}x ${k}`).join(", ");
    const compactSummary = `${group.wp_id}: ${entries.length} events (${kindSummary})`;

    if (dryRun) {
      console.log(`  [consolidate] would summarize ${entries.length} episodic entries for ${group.wp_id}: ${kindSummary}`);
      consolidated += entries.length;
      continue;
    }

    addMemory(db, {
      memoryType: "semantic",
      topic: `Session history: ${group.wp_id}`,
      summary: compactSummary,
      wpId: group.wp_id,
      importance: 0.4,
      content: entries.map(e => `- ${e.topic}: ${e.summary.slice(0, 100)}`).join("\n"),
      sourceArtifact: "memory-compact",
      metadata: { consolidated_from: entries.length, receipt_kinds: receiptKinds },
    });

    for (const e of entries) {
      db.prepare("UPDATE memory_index SET consolidated = 1 WHERE id = ?").run(e.id);
    }
    consolidated += entries.length;
  }
  return consolidated;
}

// ---------------------------------------------------------------------------
// 3. Integrity cleanup
// ---------------------------------------------------------------------------

function cleanOrphans(db, dryRun) {
  const orphaned = db.prepare(
    "SELECT id FROM memory_entries WHERE index_id NOT IN (SELECT id FROM memory_index)"
  ).all();

  if (dryRun) {
    if (orphaned.length > 0) console.log(`  [integrity] would remove ${orphaned.length} orphaned entries`);
    return orphaned.length;
  }

  for (const o of orphaned) {
    db.prepare("DELETE FROM memory_entries WHERE id = ?").run(o.id);
  }
  return orphaned.length;
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const flags = parseFlags(process.argv.slice(2));
const dryRun = Boolean(flags.dryRun);
const olderThanMs = parseOlderThan(flags.older_than);

if (dryRun) console.log("[memory-compact] DRY RUN — no changes will be made\n");

const { db } = openGovernanceMemoryDb();
try {
  console.log("[memory-compact] Step 1: Deduplication");
  const merged = dedup(db, dryRun);
  console.log(`  merged: ${merged}`);

  console.log("[memory-compact] Step 2: Consolidation (episodic → semantic)");
  const consolidated = consolidate(db, olderThanMs, dryRun);
  console.log(`  consolidated: ${consolidated}`);

  console.log("[memory-compact] Step 3: Importance decay");
  if (dryRun) {
    console.log("  (skipped in dry run)");
  } else {
    const decay = runDecay(db, {
      decayRate: Number(flags.decay_rate) || 0.1,
      pruneThreshold: Number(flags.prune_threshold) || 0.05,
    });
    console.log(`  processed: ${decay.processed}, decayed: ${decay.decayed}, pruned: ${decay.pruned}`);
  }

  console.log("[memory-compact] Step 4: Integrity cleanup");
  const orphans = cleanOrphans(db, dryRun);
  console.log(`  orphans removed: ${orphans}`);

  if (!dryRun) {
    db.prepare(
      "INSERT INTO consolidation_log (run_type, entries_processed, entries_archived, entries_merged, run_at, summary) VALUES (?, ?, ?, ?, ?, ?)"
    ).run("compact", consolidated + merged + orphans, consolidated, merged, nowIso(),
      `dedup=${merged} consolidate=${consolidated} orphans=${orphans}`);
  }

  console.log("\n[memory-compact] Done.");
} finally {
  closeDb(db);
}
