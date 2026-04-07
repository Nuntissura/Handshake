#!/usr/bin/env node
/**
 * Governance Memory CLI — entry point for `just memory-*` commands.
 *
 * Usage:
 *   node governance-memory-cli.mjs add <type> <topic> "<summary>" [--wp WP-{ID}] [--scope "file1,file2"] [--content "<full>"] [--source "<artifact>"] [--role "<role>"]
 *   node governance-memory-cli.mjs search "<query>" [--type <type>] [--wp WP-{ID}] [--limit N]
 *   node governance-memory-cli.mjs prime <WP-{ID}> [--files "file1,file2"] [--desc "<description>"] [--budget N]
 *   node governance-memory-cli.mjs stats
 *   node governance-memory-cli.mjs decay [--rate 0.1] [--threshold 0.05]
 *   node governance-memory-cli.mjs migrate-failure-memory
 */

import path from "node:path";
import {
  openGovernanceMemoryDb,
  closeDb,
  addMemory,
  getPointerIndex,
  searchMemories,
  hybridSearch,
  embedAllUnembedded,
  primeForMt,
  getStats,
  runDecay,
  migrateFailureMemory,
  VALID_MEMORY_TYPES,
} from "./governance-memory-lib.mjs";
import { querySnapshots, captureIntentSnapshot, VALID_SNAPSHOT_TYPES } from "./memory-snapshot.mjs";
import { GOVERNANCE_RUNTIME_ROOT_ABS } from "../lib/runtime-paths.mjs";

function parseFlags(args) {
  const flags = {};
  const positional = [];
  for (let i = 0; i < args.length; i++) {
    if (args[i].startsWith("--") && i + 1 < args.length) {
      flags[args[i].slice(2)] = args[i + 1];
      i++;
    } else {
      positional.push(args[i]);
    }
  }
  return { flags, positional };
}

const [command, ...rawArgs] = process.argv.slice(2);
const { flags, positional } = parseFlags(rawArgs);

if (!command) {
  console.error("Usage: governance-memory-cli.mjs <add|search|prime|stats|decay|migrate-failure-memory>");
  process.exit(1);
}

const { db, dbPath } = openGovernanceMemoryDb();

try {
  if (command === "add") {
    const [memoryType, topic, summary] = positional;
    if (!memoryType || !topic || !summary) {
      console.error('Usage: add <episodic|semantic|procedural> <topic> "<summary>" [--wp WP-{ID}] [--scope "files"] [--content "<full>"] [--source "<artifact>"] [--role "<role>"]');
      process.exit(1);
    }
    const indexId = addMemory(db, {
      memoryType,
      topic,
      summary,
      wpId: flags.wp || "",
      fileScope: flags.scope || "",
      content: flags.content || "",
      sourceArtifact: flags.source || "",
      sourceRole: flags.role || "",
    });
    console.log(`[governance-memory] Added ${memoryType} memory #${indexId}: ${topic}`);

  } else if (command === "search") {
    const [query] = positional;
    if (!query) {
      console.error('Usage: search "<query>" [--type <type>] [--wp WP-{ID}] [--limit N]');
      process.exit(1);
    }
    const results = searchMemories(db, query, {
      memoryType: flags.type || "",
      wpId: flags.wp || "",
      limit: Number(flags.limit) || 20,
    });
    if (results.length === 0) {
      console.log(`[governance-memory] No matches for "${query}"`);
    } else {
      console.log(`[governance-memory] ${results.length} match(es) for "${query}":\n`);
      for (const r of results) {
        console.log(`  #${r.id} [${r.memory_type}] ${r.topic}`);
        console.log(`    ${r.summary}`);
        if (r.wp_id) console.log(`    wp=${r.wp_id}`);
        if (r.file_scope) console.log(`    scope=${r.file_scope}`);
        if (r.content) console.log(`    content=${r.content.slice(0, 120)}${r.content.length > 120 ? "..." : ""}`);
        console.log(`    importance=${r.importance.toFixed(2)} access=${r.access_count} created=${r.created_at}`);
        console.log("");
      }
    }

  } else if (command === "prime") {
    const [wpId] = positional;
    if (!wpId) {
      console.error('Usage: prime <WP-{ID}> [--files "file1,file2"] [--desc "<description>"] [--budget N]');
      process.exit(1);
    }
    const fileTargets = flags.files ? flags.files.split(",").map(f => f.trim()) : [];
    const results = primeForMt(db, {
      wpId,
      fileTargets,
      description: flags.desc || "",
      tokenBudget: Number(flags.budget) || 2000,
    });
    if (results.length === 0) {
      console.log(`[governance-memory] No relevant memories for ${wpId}`);
    } else {
      console.log(`## SESSION MEMORY (${results.length} entries)\n`);
      for (const r of results) {
        console.log(`- [${r.memory_type}] **${r.topic}**: ${r.summary}${r.file_scope ? ` (${r.file_scope})` : ""}`);
      }
      console.log(`\nUse \`just memory-search "<query>"\` to retrieve full content.`);
    }

  } else if (command === "stats") {
    const stats = getStats(db);
    console.log(`GOVERNANCE_MEMORY_STATS`);
    console.log(`${"─".repeat(50)}`);
    console.log(`  schema_version: ${stats.schema_version}`);
    console.log(`  database: ${dbPath}`);
    console.log(`  total_index: ${stats.total_index} (active=${stats.active}, consolidated=${stats.consolidated})`);
    console.log(`  total_entries: ${stats.total_entries}`);
    for (const [type, count] of Object.entries(stats.by_type)) {
      console.log(`  ${type}: ${count}`);
    }
    if (stats.oldest_active) console.log(`  oldest_active: ${stats.oldest_active}`);
    if (stats.last_compaction) console.log(`  last_compaction: ${stats.last_compaction.run_at} — ${stats.last_compaction.summary}`);

  } else if (command === "decay") {
    const result = runDecay(db, {
      decayRate: Number(flags.rate) || 0.1,
      pruneThreshold: Number(flags.threshold) || 0.05,
    });
    console.log(`[governance-memory] Decay: processed=${result.processed}, decayed=${result.decayed}, pruned=${result.pruned}`);

  } else if (command === "migrate-failure-memory") {
    const failureMemoryPath = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "FAILURE_MEMORY.json");
    const migrated = migrateFailureMemory(db, failureMemoryPath);
    console.log(`[governance-memory] Migrated ${migrated} failure memory entries`);

  } else if (command === "embed") {
    const batchSize = Number(flags.batch) || 20;
    const embedded = await embedAllUnembedded(db, { batchSize });
    console.log(`[governance-memory] Embedded ${embedded} entries via nomic-embed-text`);

  } else if (command === "hybrid-search") {
    const [query] = positional;
    if (!query) {
      console.error('Usage: hybrid-search "<query>" [--type <type>] [--wp WP-{ID}] [--limit N]');
      process.exit(1);
    }
    const results = await hybridSearch(db, query, {
      memoryType: flags.type || "",
      wpId: flags.wp || "",
      limit: Number(flags.limit) || 20,
    });
    if (results.length === 0) {
      console.log(`[governance-memory] No hybrid matches for "${query}"`);
    } else {
      console.log(`[governance-memory] ${results.length} hybrid match(es) for "${query}":\n`);
      for (const r of results) {
        console.log(`  #${r.id} [${r.memory_type}] ${r.topic} (rrf=${r._rrf_score?.toFixed(4)})`);
        console.log(`    ${r.summary}`);
        if (r.wp_id) console.log(`    wp=${r.wp_id}`);
        if (r.content) console.log(`    content=${r.content.slice(0, 120)}${r.content.length > 120 ? "..." : ""}`);
        console.log("");
      }
    }

  } else if (command === "capture") {
    const [memoryType, insight] = positional;
    if (!memoryType || !insight) {
      console.error('Usage: capture <procedural|semantic|episodic> "<insight>" [--wp WP-{ID}] [--scope "files"] [--role "<role>"]');
      process.exit(1);
    }
    if (!VALID_MEMORY_TYPES.includes(memoryType)) {
      console.error(`Invalid type: ${memoryType}. Must be one of: ${VALID_MEMORY_TYPES.join(", ")}`);
      process.exit(1);
    }
    const indexId = addMemory(db, {
      memoryType,
      topic: insight.slice(0, 80),
      summary: insight,
      wpId: flags.wp || "",
      fileScope: flags.scope || "",
      importance: 0.7,
      content: insight,
      sourceArtifact: "memory-capture",
      sourceRole: flags.role || "",
      metadata: { captured_mid_session: true },
    });
    console.log(`[memory-capture] Stored ${memoryType} #${indexId}: ${insight.slice(0, 80)}`);

  } else if (command === "flag") {
    const [idStr, reason] = positional;
    if (!idStr || !reason) {
      console.error('Usage: flag <memory-id> "<reason>"');
      process.exit(1);
    }
    const id = Number(idStr);
    const existing = db.prepare("SELECT id, topic, importance FROM memory_index WHERE id = ?").get(id);
    if (!existing) {
      console.error(`[memory-flag] Memory #${id} not found`);
      process.exit(1);
    }
    // Suppress: set importance to 0.1 and record flag in metadata
    db.prepare("UPDATE memory_index SET importance = 0.1 WHERE id = ?").run(id);
    const entry = db.prepare("SELECT id, metadata FROM memory_entries WHERE index_id = ? LIMIT 1").get(id);
    if (entry) {
      let meta = {};
      try { meta = JSON.parse(entry.metadata || "{}"); } catch {}
      meta.flagged = true;
      meta.flag_reason = reason;
      meta.flagged_at = new Date().toISOString();
      meta.importance_before_flag = existing.importance;
      db.prepare("UPDATE memory_entries SET metadata = ? WHERE id = ?").run(JSON.stringify(meta), entry.id);
    }
    console.log(`[memory-flag] Flagged #${id} "${existing.topic}" — importance ${existing.importance.toFixed(2)} → 0.10, reason: ${reason}`);

  } else if (command === "intent-snapshot") {
    const [intent] = positional;
    if (!intent) {
      console.error('Usage: intent-snapshot "<what you are about to do>" [--wp WP-{ID}] [--role ROLE] [--reason "why"] [--expected "outcome"] [--scope "files"]');
      process.exit(1);
    }
    // Close the shared db — captureIntentSnapshot opens its own
    closeDb(db);
    const indexId = captureIntentSnapshot({
      wpId: flags.wp || "",
      role: flags.role || "",
      intent,
      reason: flags.reason || "",
      expectedOutcome: flags.expected || "",
      scope: flags.scope || "",
    });
    if (indexId) {
      console.log(`[intent-snapshot] Stored #${indexId}: ${intent.slice(0, 80)}`);
    } else {
      console.log(`[intent-snapshot] Skipped (dedup window or empty intent)`);
    }
    process.exit(0);

  } else if (command === "debug-snapshot") {
    const [wpIdOrType] = positional;
    const wpFilter = flags.wp || (wpIdOrType && wpIdOrType.startsWith("WP-") ? wpIdOrType : "");
    const typeFilter = flags.type || (wpIdOrType && VALID_SNAPSHOT_TYPES.includes(wpIdOrType) ? wpIdOrType : "");
    const limit = Number(flags.limit) || 20;
    const snapshots = querySnapshots(db, { wpId: wpFilter, snapshotType: typeFilter, limit });
    if (snapshots.length === 0) {
      console.log(`[governance-memory] No pre-task snapshots found${wpFilter ? ` for ${wpFilter}` : ""}${typeFilter ? ` type=${typeFilter}` : ""}`);
    } else {
      console.log(`PRE_TASK_SNAPSHOTS (${snapshots.length} entries):\n`);
      for (const s of snapshots) {
        console.log(`  #${s.id} [${s.snapshot_type}] ${s.wp_id || "cross-WP"} @ ${s.created_at}`);
        console.log(`    ${s.summary}`);
        if (s.content) {
          try {
            const ctx = JSON.parse(s.content);
            if (s.snapshot_type === "INTENT") {
              // Intent snapshots have readable sentence fields — show them fully
              if (ctx.intent) console.log(`    intent: ${ctx.intent}`);
              if (ctx.reason) console.log(`    reason: ${ctx.reason}`);
              if (ctx.expectedOutcome) console.log(`    expected: ${ctx.expectedOutcome}`);
              if (ctx.scope) console.log(`    scope: ${ctx.scope}`);
            } else {
              const keys = Object.keys(ctx).slice(0, 6);
              console.log(`    context: {${keys.map(k => `${k}: ${JSON.stringify(ctx[k]).slice(0, 60)}`).join(", ")}}`);
            }
          } catch {
            console.log(`    content=${s.content.slice(0, 200)}`);
          }
        }
        console.log("");
      }
    }

  } else {
    console.error(`Unknown command: ${command}`);
    console.error("Usage: governance-memory-cli.mjs <add|search|hybrid-search|embed|capture|flag|debug-snapshot|prime|stats|decay|migrate-failure-memory>");
    process.exit(1);
  }
} finally {
  closeDb(db);
}
