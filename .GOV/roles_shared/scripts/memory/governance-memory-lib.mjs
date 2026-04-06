/**
 * Governance Memory System — Core Library
 *
 * Provider-agnostic cross-session memory for governed workflows.
 * Uses Node.js built-in node:sqlite (Node 22.5+) with FTS5 for keyword search.
 * Schema uses ONLY features portable to PostgreSQL.
 *
 * Database: gov_runtime/roles_shared/GOVERNANCE_MEMORY.db
 *
 * Memory types:
 *   episodic    — timestamped session events (who did what, when, outcome)
 *   semantic    — distilled facts (codebase patterns, decisions, preferences)
 *   procedural  — fix patterns, workflows, recipes
 *
 * Related: RGF-115 (schema), RGF-116 (FTS5), RGF-117 (pointer-index)
 */

import fs from "node:fs";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  ensureGovernanceRuntimeDir,
} from "../lib/runtime-paths.mjs";

export const GOVERNANCE_MEMORY_SCHEMA_VERSION = "1";
export const GOVERNANCE_MEMORY_DB_NAME = "GOVERNANCE_MEMORY.db";
export const VALID_MEMORY_TYPES = ["episodic", "semantic", "procedural"];

function nowIso() {
  return new Date().toISOString();
}

function governanceMemoryDbPath() {
  return path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", GOVERNANCE_MEMORY_DB_NAME);
}

const CREATE_TABLES_SQL = `
CREATE TABLE IF NOT EXISTS memory_index (
  id INTEGER PRIMARY KEY,
  memory_type TEXT NOT NULL,
  topic TEXT NOT NULL,
  summary TEXT NOT NULL,
  wp_id TEXT DEFAULT '',
  file_scope TEXT DEFAULT '',
  importance REAL DEFAULT 0.5,
  access_count INTEGER DEFAULT 0,
  consolidated INTEGER DEFAULT 0,
  created_at TEXT NOT NULL,
  last_accessed_at TEXT DEFAULT '',
  expires_at TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS memory_entries (
  id INTEGER PRIMARY KEY,
  index_id INTEGER REFERENCES memory_index(id),
  content TEXT NOT NULL,
  source_artifact TEXT DEFAULT '',
  source_wp_id TEXT DEFAULT '',
  source_role TEXT DEFAULT '',
  source_session TEXT DEFAULT '',
  metadata TEXT DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS consolidation_log (
  id INTEGER PRIMARY KEY,
  run_type TEXT NOT NULL,
  entries_processed INTEGER DEFAULT 0,
  entries_archived INTEGER DEFAULT 0,
  entries_merged INTEGER DEFAULT 0,
  run_at TEXT NOT NULL,
  summary TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS schema_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
`;

const CREATE_INDEXES_SQL = `
CREATE INDEX IF NOT EXISTS idx_memory_index_type ON memory_index(memory_type);
CREATE INDEX IF NOT EXISTS idx_memory_index_wp ON memory_index(wp_id);
CREATE INDEX IF NOT EXISTS idx_memory_index_importance ON memory_index(importance);
CREATE INDEX IF NOT EXISTS idx_memory_index_consolidated ON memory_index(consolidated);
CREATE INDEX IF NOT EXISTS idx_memory_entries_index ON memory_entries(index_id);
CREATE INDEX IF NOT EXISTS idx_memory_entries_wp ON memory_entries(source_wp_id);
`;

const CREATE_FTS_SQL = `
CREATE VIRTUAL TABLE IF NOT EXISTS memory_index_fts USING fts5(
  topic, summary, file_scope,
  content='memory_index',
  content_rowid='id'
);

CREATE VIRTUAL TABLE IF NOT EXISTS memory_entries_fts USING fts5(
  content, source_artifact, source_wp_id,
  content='memory_entries',
  content_rowid='id'
);
`;

const FTS_TRIGGERS_SQL = `
CREATE TRIGGER IF NOT EXISTS memory_index_ai AFTER INSERT ON memory_index BEGIN
  INSERT INTO memory_index_fts(rowid, topic, summary, file_scope)
  VALUES (new.id, new.topic, new.summary, new.file_scope);
END;

CREATE TRIGGER IF NOT EXISTS memory_index_ad AFTER DELETE ON memory_index BEGIN
  INSERT INTO memory_index_fts(memory_index_fts, rowid, topic, summary, file_scope)
  VALUES ('delete', old.id, old.topic, old.summary, old.file_scope);
END;

CREATE TRIGGER IF NOT EXISTS memory_index_au AFTER UPDATE ON memory_index BEGIN
  INSERT INTO memory_index_fts(memory_index_fts, rowid, topic, summary, file_scope)
  VALUES ('delete', old.id, old.topic, old.summary, old.file_scope);
  INSERT INTO memory_index_fts(rowid, topic, summary, file_scope)
  VALUES (new.id, new.topic, new.summary, new.file_scope);
END;

CREATE TRIGGER IF NOT EXISTS memory_entries_ai AFTER INSERT ON memory_entries BEGIN
  INSERT INTO memory_entries_fts(rowid, content, source_artifact, source_wp_id)
  VALUES (new.id, new.content, new.source_artifact, new.source_wp_id);
END;

CREATE TRIGGER IF NOT EXISTS memory_entries_ad AFTER DELETE ON memory_entries BEGIN
  INSERT INTO memory_entries_fts(memory_entries_fts, rowid, content, source_artifact, source_wp_id)
  VALUES ('delete', old.id, old.content, old.source_artifact, old.source_wp_id);
END;

CREATE TRIGGER IF NOT EXISTS memory_entries_au AFTER UPDATE ON memory_entries BEGIN
  INSERT INTO memory_entries_fts(memory_entries_fts, rowid, content, source_artifact, source_wp_id)
  VALUES ('delete', old.id, old.content, old.source_artifact, old.source_wp_id);
  INSERT INTO memory_entries_fts(rowid, content, source_artifact, source_wp_id)
  VALUES (new.id, new.content, new.source_artifact, new.source_wp_id);
END;
`;

// ---------------------------------------------------------------------------
// Database lifecycle
// ---------------------------------------------------------------------------

export function openGovernanceMemoryDb() {
  ensureGovernanceRuntimeDir("roles_shared");
  const dbPath = governanceMemoryDbPath();
  const db = new DatabaseSync(dbPath);
  db.exec("PRAGMA journal_mode = WAL");
  db.exec(CREATE_TABLES_SQL);
  db.exec(CREATE_INDEXES_SQL);
  db.exec(CREATE_FTS_SQL);
  db.exec(FTS_TRIGGERS_SQL);

  const row = db.prepare("SELECT value FROM schema_meta WHERE key = 'schema_version'").get();
  if (!row) {
    db.prepare("INSERT INTO schema_meta (key, value) VALUES ('schema_version', ?)").run(GOVERNANCE_MEMORY_SCHEMA_VERSION);
  }
  return { db, dbPath };
}

export function closeDb(db) {
  try { db.close(); } catch {}
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

export function addMemory(db, {
  memoryType,
  topic,
  summary,
  wpId = "",
  fileScope = "",
  importance = 0.5,
  content = "",
  sourceArtifact = "",
  sourceWpId = "",
  sourceRole = "",
  sourceSession = "",
  metadata = {},
}) {
  if (!VALID_MEMORY_TYPES.includes(memoryType)) {
    throw new Error(`Invalid memory_type: ${memoryType}. Must be one of: ${VALID_MEMORY_TYPES.join(", ")}`);
  }
  const now = nowIso();
  const indexStmt = db.prepare(
    `INSERT INTO memory_index (memory_type, topic, summary, wp_id, file_scope, importance, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)`
  );
  const result = indexStmt.run(memoryType, topic, summary, wpId, fileScope, importance, now);
  const indexId = Number(result.lastInsertRowid);

  if (content) {
    db.prepare(
      `INSERT INTO memory_entries (index_id, content, source_artifact, source_wp_id, source_role, source_session, metadata, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
    ).run(indexId, content, sourceArtifact, sourceWpId || wpId, sourceRole, sourceSession, JSON.stringify(metadata), now);
  }

  return indexId;
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

export function getPointerIndex(db, { memoryType = "", wpId = "", limit = 50 } = {}) {
  let sql = "SELECT id, memory_type, topic, summary, wp_id, file_scope, importance, access_count, created_at FROM memory_index WHERE consolidated = 0";
  const params = [];
  if (memoryType) { sql += " AND memory_type = ?"; params.push(memoryType); }
  if (wpId) { sql += " AND (wp_id = ? OR wp_id = '')"; params.push(wpId); }
  sql += " ORDER BY importance DESC, created_at DESC LIMIT ?";
  params.push(limit);
  return db.prepare(sql).all(...params);
}

export function getMemoryEntry(db, indexId) {
  db.prepare("UPDATE memory_index SET access_count = access_count + 1, last_accessed_at = ? WHERE id = ?").run(nowIso(), indexId);
  return db.prepare("SELECT * FROM memory_entries WHERE index_id = ?").get(indexId);
}

// ---------------------------------------------------------------------------
// Search (FTS5 keyword)
// ---------------------------------------------------------------------------

export function searchMemories(db, query, { memoryType = "", wpId = "", limit = 20 } = {}) {
  const ftsQuery = sanitizeFtsQuery(query);
  if (!ftsQuery) return [];

  const indexMatches = db.prepare(
    "SELECT rowid, rank FROM memory_index_fts WHERE memory_index_fts MATCH ? ORDER BY rank LIMIT ?"
  ).all(ftsQuery, limit * 2);

  const entryMatches = db.prepare(
    "SELECT rowid, rank FROM memory_entries_fts WHERE memory_entries_fts MATCH ? ORDER BY rank LIMIT ?"
  ).all(ftsQuery, limit * 2);

  const indexIdScores = new Map();
  for (const m of indexMatches) {
    indexIdScores.set(m.rowid, (indexIdScores.get(m.rowid) || 0) + Math.abs(m.rank));
  }
  for (const m of entryMatches) {
    const entry = db.prepare("SELECT index_id FROM memory_entries WHERE id = ?").get(m.rowid);
    if (entry) {
      indexIdScores.set(entry.index_id, (indexIdScores.get(entry.index_id) || 0) + Math.abs(m.rank));
    }
  }

  if (indexIdScores.size === 0) return [];

  const sortedIds = [...indexIdScores.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, limit)
    .map(([id]) => id);

  const results = [];
  for (const id of sortedIds) {
    const idx = db.prepare("SELECT * FROM memory_index WHERE id = ?").get(id);
    if (!idx) continue;
    if (memoryType && idx.memory_type !== memoryType) continue;
    if (wpId && idx.wp_id && idx.wp_id !== wpId) continue;
    const entry = db.prepare("SELECT content, source_artifact, source_role FROM memory_entries WHERE index_id = ? LIMIT 1").get(id);
    results.push({ ...idx, content: entry?.content || "", source_artifact: entry?.source_artifact || "", source_role: entry?.source_role || "" });
  }
  return results;
}

function sanitizeFtsQuery(query) {
  return String(query || "")
    .replace(/[^\w\s\-_.]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

// ---------------------------------------------------------------------------
// Scoped priming (MT-level context)
// ---------------------------------------------------------------------------

export function primeForMt(db, { wpId, fileTargets = [], description = "", tokenBudget = 2000 } = {}) {
  const results = [];
  const seen = new Set();

  if (fileTargets.length > 0) {
    for (const file of fileTargets) {
      const matches = db.prepare(
        "SELECT * FROM memory_index WHERE file_scope LIKE ? AND consolidated = 0 ORDER BY importance DESC LIMIT 10"
      ).all(`%${path.basename(file)}%`);
      for (const m of matches) {
        if (!seen.has(m.id)) { seen.add(m.id); results.push(m); }
      }
    }
  }

  if (description) {
    const ftsQuery = sanitizeFtsQuery(description);
    if (ftsQuery) {
      try {
        const ftsMatches = db.prepare(
          "SELECT rowid FROM memory_index_fts WHERE memory_index_fts MATCH ? ORDER BY rank LIMIT 20"
        ).all(ftsQuery);
        for (const m of ftsMatches) {
          if (!seen.has(m.rowid)) {
            const idx = db.prepare("SELECT * FROM memory_index WHERE id = ?").get(m.rowid);
            if (idx && idx.consolidated === 0) { seen.add(m.rowid); results.push(idx); }
          }
        }
      } catch {}
    }
  }

  if (wpId) {
    const wpMatches = db.prepare(
      "SELECT * FROM memory_index WHERE wp_id = ? AND consolidated = 0 ORDER BY importance DESC LIMIT 10"
    ).all(wpId);
    for (const m of wpMatches) {
      if (!seen.has(m.id)) { seen.add(m.id); results.push(m); }
    }
  }

  results.sort((a, b) => (b.importance || 0) - (a.importance || 0));

  let tokenEstimate = 0;
  const budgeted = [];
  for (const r of results) {
    const entryTokens = Math.ceil((r.topic.length + r.summary.length + (r.file_scope || "").length) / 4);
    if (tokenEstimate + entryTokens > tokenBudget) break;
    tokenEstimate += entryTokens;
    budgeted.push(r);
  }

  return budgeted;
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

export function getStats(db) {
  const totalIndex = db.prepare("SELECT COUNT(*) as count FROM memory_index").get().count;
  const totalEntries = db.prepare("SELECT COUNT(*) as count FROM memory_entries").get().count;
  const consolidated = db.prepare("SELECT COUNT(*) as count FROM memory_index WHERE consolidated = 1").get().count;
  const byType = db.prepare("SELECT memory_type, COUNT(*) as count FROM memory_index WHERE consolidated = 0 GROUP BY memory_type").all();
  const lastCompaction = db.prepare("SELECT run_at, summary FROM consolidation_log ORDER BY run_at DESC LIMIT 1").get();
  const oldestActive = db.prepare("SELECT created_at FROM memory_index WHERE consolidated = 0 ORDER BY created_at ASC LIMIT 1").get();
  const schemaVersion = db.prepare("SELECT value FROM schema_meta WHERE key = 'schema_version'").get();
  return {
    schema_version: schemaVersion?.value || "unknown",
    total_index: totalIndex,
    total_entries: totalEntries,
    consolidated,
    active: totalIndex - consolidated,
    by_type: Object.fromEntries(byType.map(r => [r.memory_type, r.count])),
    oldest_active: oldestActive?.created_at || null,
    last_compaction: lastCompaction || null,
  };
}

// ---------------------------------------------------------------------------
// Compaction
// ---------------------------------------------------------------------------

export function runDecay(db, { decayRate = 0.1, pruneThreshold = 0.05 } = {}) {
  const now = Date.now();
  const entries = db.prepare("SELECT id, importance, last_accessed_at, created_at FROM memory_index WHERE consolidated = 0").all();
  let decayed = 0;
  let pruned = 0;

  for (const entry of entries) {
    const lastAccess = entry.last_accessed_at || entry.created_at;
    const daysSinceAccess = (now - new Date(lastAccess).getTime()) / (1000 * 60 * 60 * 24);
    const newImportance = entry.importance * Math.exp(-decayRate * daysSinceAccess);

    if (newImportance < pruneThreshold) {
      db.prepare("UPDATE memory_index SET consolidated = 1, importance = ? WHERE id = ?").run(newImportance, entry.id);
      pruned++;
    } else if (Math.abs(newImportance - entry.importance) > 0.001) {
      db.prepare("UPDATE memory_index SET importance = ? WHERE id = ?").run(newImportance, entry.id);
      decayed++;
    }
  }

  db.prepare(
    "INSERT INTO consolidation_log (run_type, entries_processed, entries_archived, run_at, summary) VALUES (?, ?, ?, ?, ?)"
  ).run("decay", entries.length, pruned, nowIso(), `Decayed ${decayed}, pruned ${pruned} of ${entries.length} entries`);

  return { processed: entries.length, decayed, pruned };
}

// ---------------------------------------------------------------------------
// Migration: import failure-memory.json entries
// ---------------------------------------------------------------------------

export function migrateFailureMemory(db, failureMemoryPath) {
  if (!fs.existsSync(failureMemoryPath)) return 0;
  let entries;
  try {
    entries = JSON.parse(fs.readFileSync(failureMemoryPath, "utf8"));
    if (!Array.isArray(entries)) return 0;
  } catch { return 0; }

  let migrated = 0;
  for (const entry of entries) {
    const existing = db.prepare(
      "SELECT id FROM memory_index WHERE memory_type = 'procedural' AND topic = ? AND summary = ?"
    ).get(entry.file_surface || "", entry.error_pattern || "");
    if (existing) continue;

    addMemory(db, {
      memoryType: "procedural",
      topic: entry.file_surface || "unknown",
      summary: `${entry.error_category || "error"}: ${entry.error_pattern || ""}`,
      wpId: entry.wp_id || "",
      fileScope: entry.file_surface || "",
      importance: Math.min(0.5 + (entry.occurrences || 1) * 0.1, 1.0),
      content: `Error: ${entry.error_pattern || ""}\nFix: ${entry.fix_pattern || ""}\nOccurrences: ${entry.occurrences || 1}`,
      sourceArtifact: "FAILURE_MEMORY.json",
      sourceWpId: entry.wp_id || "",
      metadata: { migrated_from: "failure-memory.json", original_category: entry.error_category },
    });
    migrated++;
  }
  return migrated;
}
