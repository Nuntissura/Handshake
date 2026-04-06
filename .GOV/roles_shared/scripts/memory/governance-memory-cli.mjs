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

  } else {
    console.error(`Unknown command: ${command}`);
    console.error("Usage: governance-memory-cli.mjs <add|search|hybrid-search|embed|prime|stats|decay|migrate-failure-memory>");
    process.exit(1);
  }
} finally {
  closeDb(db);
}
