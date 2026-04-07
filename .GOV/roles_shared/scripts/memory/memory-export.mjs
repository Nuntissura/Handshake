#!/usr/bin/env node
/**
 * Export governance memory to JSONL for git-trackable archival.
 *
 * Outputs all active (non-consolidated) memories as one JSON object per line.
 * Pipe to file for archival: just memory-export > memory-archive-2026-04-07.jsonl
 *
 * Can also restore from export: just memory-import <file.jsonl>
 *
 * Usage:
 *   node memory-export.mjs [--all]          (default: active only)
 *   node memory-export.mjs --all            (include consolidated)
 *   node memory-export.mjs --import <file>  (restore from export)
 */

import fs from "node:fs";
import path from "node:path";
import {
  openGovernanceMemoryDb,
  closeDb,
  addMemory,
} from "./governance-memory-lib.mjs";

const args = process.argv.slice(2);
const includeAll = args.includes("--all");
const importIdx = args.indexOf("--import");

if (importIdx >= 0) {
  // Import mode: restore from JSONL
  const importFile = args[importIdx + 1];
  if (!importFile || !fs.existsSync(importFile)) {
    console.error("Usage: memory-export.mjs --import <file.jsonl>");
    process.exit(1);
  }
  const lines = fs.readFileSync(importFile, "utf8").split("\n").filter(Boolean);
  const { db } = openGovernanceMemoryDb();
  let imported = 0;
  let skipped = 0;
  try {
    for (const line of lines) {
      try {
        const entry = JSON.parse(line);
        // Dedup: skip if topic+wp+type already exists
        const existing = db.prepare(
          "SELECT id FROM memory_index WHERE topic = ? AND wp_id = ? AND memory_type = ?"
        ).get(entry.topic || "", entry.wp_id || "", entry.memory_type || "");
        if (existing) { skipped++; continue; }
        addMemory(db, {
          memoryType: entry.memory_type || "semantic",
          topic: entry.topic || "",
          summary: entry.summary || "",
          wpId: entry.wp_id || "",
          fileScope: entry.file_scope || "",
          importance: entry.importance || 0.5,
          content: entry.content || "",
          sourceArtifact: entry.source_artifact || "memory-import",
          sourceWpId: entry.source_wp_id || entry.wp_id || "",
          sourceRole: entry.source_role || "",
          sourceSession: entry.source_session || "",
          metadata: entry.metadata || {},
        });
        imported++;
      } catch { skipped++; }
    }
  } finally { closeDb(db); }
  console.error(`[memory-import] Imported ${imported}, skipped ${skipped} (dupes/errors) from ${path.basename(importFile)}`);
  process.exit(0);
}

// Export mode: dump to JSONL
const { db } = openGovernanceMemoryDb();
try {
  const filter = includeAll ? "" : " WHERE consolidated = 0";
  const rows = db.prepare(
    `SELECT mi.id, mi.memory_type, mi.topic, mi.summary, mi.wp_id, mi.file_scope,
            mi.importance, mi.access_count, mi.consolidated, mi.created_at, mi.last_accessed_at,
            me.content, me.source_artifact, me.source_wp_id, me.source_role, me.source_session, me.metadata
     FROM memory_index mi
     LEFT JOIN memory_entries me ON me.index_id = mi.id
     ${filter}
     ORDER BY mi.id`
  ).all();

  for (const row of rows) {
    let metadata = {};
    try { metadata = JSON.parse(row.metadata || "{}"); } catch {}
    const entry = {
      id: row.id,
      memory_type: row.memory_type,
      topic: row.topic,
      summary: row.summary,
      wp_id: row.wp_id,
      file_scope: row.file_scope,
      importance: row.importance,
      access_count: row.access_count,
      consolidated: row.consolidated,
      created_at: row.created_at,
      last_accessed_at: row.last_accessed_at,
      content: row.content || "",
      source_artifact: row.source_artifact || "",
      source_wp_id: row.source_wp_id || "",
      source_role: row.source_role || "",
      source_session: row.source_session || "",
      metadata,
    };
    process.stdout.write(JSON.stringify(entry) + "\n");
  }
  console.error(`[memory-export] Exported ${rows.length} entries (${includeAll ? "all" : "active only"})`);
} finally {
  closeDb(db);
}
