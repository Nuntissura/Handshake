#!/usr/bin/env node
/**
 * DEPRECATED: Legacy failure-memory CLI.
 *
 * This script now redirects to the governance memory system (GOVERNANCE_MEMORY.db).
 * The legacy FAILURE_MEMORY.json has been migrated and archived.
 *
 * `record` → writes a procedural memory via governance-memory-lib addMemory()
 * `query`  → searches governance memory via FTS5
 *
 * Prefer the direct commands instead:
 *   just memory-capture procedural "<fix pattern>" --scope "<file>" --wp WP-{ID}
 *   just memory-search "<query>"
 */

import {
  openGovernanceMemoryDb,
  closeDb,
  addMemory,
  searchMemories,
} from "../memory/governance-memory-lib.mjs";

const [mode, ...rest] = process.argv.slice(2);

if (mode === "record") {
  const [errorCategory, fileSurface, errorPattern, fixPattern, wpId] = rest;
  if (!errorCategory || !fileSurface || !errorPattern || !fixPattern) {
    console.error("Usage: failure-memory.mjs record <error_category> <file_surface> <error_pattern> <fix_pattern> [wp_id]");
    console.error("DEPRECATED: prefer `just memory-capture procedural \"<fix>\" --scope \"<file>\" --wp WP-{ID}`");
    process.exit(1);
  }
  console.error("[failure-memory] DEPRECATED: redirecting to governance memory DB. Prefer `just memory-capture procedural`.");
  const { db } = openGovernanceMemoryDb();
  try {
    addMemory(db, {
      memoryType: "procedural",
      topic: fileSurface,
      summary: `${errorCategory}: ${errorPattern}`,
      wpId: wpId || "",
      fileScope: fileSurface,
      importance: 0.7,
      content: `Error: ${errorPattern}\nFix: ${fixPattern}\nCategory: ${errorCategory}`,
      sourceArtifact: "failure-memory-record",
      metadata: { legacy_redirect: true, error_category: errorCategory },
    });
    console.log(`[failure-memory] Recorded procedural memory for ${fileSurface} (→ GOVERNANCE_MEMORY.db)`);
  } finally { closeDb(db); }
} else if (mode === "query") {
  const [queryText] = rest;
  if (!queryText) {
    console.error("Usage: failure-memory.mjs query <query_text>");
    console.error("DEPRECATED: prefer `just memory-search \"<query>\"`");
    process.exit(1);
  }
  console.error("[failure-memory] DEPRECATED: redirecting to governance memory DB. Prefer `just memory-search`.");
  const { db } = openGovernanceMemoryDb();
  try {
    const results = searchMemories(db, queryText, { memoryType: "procedural", limit: 10 });
    if (results.length === 0) {
      console.log(`[failure-memory] No matches for "${queryText}"`);
    } else {
      console.log(`[failure-memory] ${results.length} match(es) for "${queryText}":\n`);
      for (const r of results) {
        console.log(`  Topic:       ${r.topic}`);
        console.log(`  Summary:     ${r.summary}`);
        console.log(`  WP:          ${r.wp_id || "(none)"}`);
        console.log(`  Scope:       ${r.file_scope || "(none)"}`);
        console.log(`  Importance:  ${r.importance?.toFixed(2)}`);
        if (r.content) console.log(`  Content:     ${r.content.slice(0, 200)}`);
        console.log("");
      }
    }
  } finally { closeDb(db); }
} else {
  console.error("Usage: failure-memory.mjs <record|query> ...");
  console.error("DEPRECATED: prefer `just memory-capture` and `just memory-search`");
  process.exit(1);
}
