/**
 * memory-health-check: validates governance memory database integrity.
 *
 * Checks:
 * 1. Database exists and opens successfully
 * 2. Schema version matches expected
 * 3. FTS5 tables are operational
 * 4. No orphaned entries (entries without matching index)
 *
 * Registered in gov-check.mjs. Passes silently; exits 1 on failure.
 * The database is optional — if it doesn't exist, the check passes
 * (memory system hasn't been initialized yet, which is valid).
 */

import fs from "node:fs";
import path from "node:path";
import { GOVERNANCE_RUNTIME_ROOT_ABS } from "../scripts/lib/runtime-paths.mjs";
import {
  GOVERNANCE_MEMORY_DB_NAME,
  GOVERNANCE_MEMORY_SCHEMA_VERSION,
} from "../scripts/memory/governance-memory-lib.mjs";

const dbPath = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", GOVERNANCE_MEMORY_DB_NAME);

function fail(message) {
  console.error(`memory-health-check FAIL: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(dbPath)) {
  console.log("memory-health-check ok (database not yet initialized)");
} else {
  try {
    const { DatabaseSync } = await import("node:sqlite");
    const db = new DatabaseSync(dbPath, { readOnly: true });

    const version = db.prepare("SELECT value FROM schema_meta WHERE key = 'schema_version'").get();
    if (!version) fail("missing schema_version in schema_meta");
    if (version.value !== GOVERNANCE_MEMORY_SCHEMA_VERSION) {
      fail(`schema_version mismatch: expected ${GOVERNANCE_MEMORY_SCHEMA_VERSION}, found ${version.value}`);
    }

    const tables = db.prepare("SELECT name FROM sqlite_master WHERE type = 'table'").all().map(r => r.name);
    for (const required of ["memory_index", "memory_entries", "consolidation_log", "schema_meta"]) {
      if (!tables.includes(required)) fail(`missing table: ${required}`);
    }

    const vtables = db.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND sql LIKE '%fts5%'").all().map(r => r.name);
    for (const required of ["memory_index_fts", "memory_entries_fts"]) {
      if (!vtables.includes(required)) fail(`missing FTS5 table: ${required}`);
    }

    const orphanedEntries = db.prepare(
      "SELECT COUNT(*) as count FROM memory_entries WHERE index_id NOT IN (SELECT id FROM memory_index)"
    ).get();
    if (orphanedEntries.count > 0) fail(`${orphanedEntries.count} orphaned memory_entries (no matching memory_index row)`);

    const orphanedEmbeddings = db.prepare(
      "SELECT COUNT(*) as count FROM memory_embeddings WHERE index_id NOT IN (SELECT id FROM memory_index)"
    ).get();
    if (orphanedEmbeddings.count > 0) fail(`${orphanedEmbeddings.count} orphaned memory_embeddings (no matching memory_index row)`);

    db.close();
    console.log("memory-health-check ok");
  } catch (error) {
    fail(`database error: ${error.message}`);
  }
}
