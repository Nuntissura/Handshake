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

// RGF-143: normalize relative date references to absolute dates at write time
function normalizeDates(text) {
  if (!text) return text;
  const now = new Date();
  const fmt = (d) => d.toISOString().slice(0, 10);
  return String(text)
    .replace(/\byesterday\b/gi, fmt(new Date(now.getTime() - 86400000)))
    .replace(/\btoday\b/gi, fmt(now))
    .replace(/\btomorrow\b/gi, fmt(new Date(now.getTime() + 86400000)))
    .replace(/\blast week\b/gi, `week of ${fmt(new Date(now.getTime() - 7 * 86400000))}`)
    .replace(/\blast month\b/gi, `${now.getFullYear()}-${String(now.getMonth()).padStart(2, "0")}`);
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

CREATE TABLE IF NOT EXISTS memory_embeddings (
  id INTEGER PRIMARY KEY,
  index_id INTEGER REFERENCES memory_index(id),
  embedding_model TEXT NOT NULL DEFAULT 'nomic-embed-text',
  embedding_dims INTEGER NOT NULL DEFAULT 768,
  embedding TEXT NOT NULL,
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

CREATE TABLE IF NOT EXISTS conversation_log (
  id INTEGER PRIMARY KEY,
  session_id TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT '',
  timestamp_utc TEXT NOT NULL,
  checkpoint_type TEXT NOT NULL,
  trigger_ref TEXT DEFAULT '',
  wp_id TEXT DEFAULT '',
  topic TEXT NOT NULL,
  content TEXT NOT NULL,
  files_referenced TEXT DEFAULT '',
  decisions TEXT DEFAULT ''
);
`;

const CREATE_INDEXES_SQL = `
CREATE INDEX IF NOT EXISTS idx_memory_index_type ON memory_index(memory_type);
CREATE INDEX IF NOT EXISTS idx_memory_index_wp ON memory_index(wp_id);
CREATE INDEX IF NOT EXISTS idx_memory_index_importance ON memory_index(importance);
CREATE INDEX IF NOT EXISTS idx_memory_index_consolidated ON memory_index(consolidated);
CREATE INDEX IF NOT EXISTS idx_memory_entries_index ON memory_entries(index_id);
CREATE INDEX IF NOT EXISTS idx_memory_entries_wp ON memory_entries(source_wp_id);
CREATE INDEX IF NOT EXISTS idx_memory_embeddings_index ON memory_embeddings(index_id);
CREATE INDEX IF NOT EXISTS idx_conversation_log_session ON conversation_log(session_id);
CREATE INDEX IF NOT EXISTS idx_conversation_log_type ON conversation_log(checkpoint_type);
CREATE INDEX IF NOT EXISTS idx_conversation_log_time ON conversation_log(timestamp_utc);
CREATE INDEX IF NOT EXISTS idx_conversation_log_wp ON conversation_log(wp_id);
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

CREATE VIRTUAL TABLE IF NOT EXISTS conversation_log_fts USING fts5(
  topic, content, decisions, files_referenced,
  content='conversation_log',
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

CREATE TRIGGER IF NOT EXISTS conversation_log_ai AFTER INSERT ON conversation_log BEGIN
  INSERT INTO conversation_log_fts(rowid, topic, content, decisions, files_referenced)
  VALUES (new.id, new.topic, new.content, new.decisions, new.files_referenced);
END;

CREATE TRIGGER IF NOT EXISTS conversation_log_ad AFTER DELETE ON conversation_log BEGIN
  INSERT INTO conversation_log_fts(conversation_log_fts, rowid, topic, content, decisions, files_referenced)
  VALUES ('delete', old.id, old.topic, old.content, old.decisions, old.files_referenced);
END;

CREATE TRIGGER IF NOT EXISTS conversation_log_au AFTER UPDATE ON conversation_log BEGIN
  INSERT INTO conversation_log_fts(conversation_log_fts, rowid, topic, content, decisions, files_referenced)
  VALUES ('delete', old.id, old.topic, old.content, old.decisions, old.files_referenced);
  INSERT INTO conversation_log_fts(rowid, topic, content, decisions, files_referenced)
  VALUES (new.id, new.topic, new.content, new.decisions, new.files_referenced);
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
  db.exec("PRAGMA busy_timeout = 5000"); // Wait up to 5s for concurrent writers instead of SQLITE_BUSY
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
  // RGF-135: write-time novelty scoring — reduce importance if near-duplicate topic exists
  let adjustedImportance = importance;
  try {
    const ftsQuery = sanitizeFtsQuery(topic);
    if (ftsQuery) {
      const similar = db.prepare(
        "SELECT id FROM memory_index_fts WHERE memory_index_fts MATCH ? LIMIT 1"
      ).get(ftsQuery);
      if (similar) adjustedImportance = importance * 0.3;
    }
  } catch { /* FTS query failure — keep original importance */ }

  // RGF-137: new procedural memories supersede matching old ones (same file_scope + type)
  if (memoryType === "procedural" && fileScope) {
    try {
      const oldMatches = db.prepare(
        "SELECT id FROM memory_index WHERE memory_type = 'procedural' AND file_scope = ? AND consolidated = 0 AND wp_id = ?"
      ).all(fileScope, wpId);
      for (const old of oldMatches) {
        db.prepare("UPDATE memory_index SET consolidated = 1 WHERE id = ?").run(old.id);
        const oldEntry = db.prepare("SELECT id, metadata FROM memory_entries WHERE index_id = ? LIMIT 1").get(old.id);
        if (oldEntry) {
          let meta = {};
          try { meta = JSON.parse(oldEntry.metadata || "{}"); } catch {}
          meta.superseded_by = "pending"; // will be updated with new id below
          db.prepare("UPDATE memory_entries SET metadata = ? WHERE id = ?").run(JSON.stringify(meta), oldEntry.id);
        }
      }
    } catch { /* best-effort supersession */ }
  }

  // RGF-141: contradiction detection for semantic memories with same file_scope
  if (memoryType === "semantic" && fileScope) {
    try {
      const conflicts = db.prepare(
        "SELECT id, topic, summary FROM memory_index WHERE memory_type = 'semantic' AND file_scope = ? AND consolidated = 0 AND topic != ?"
      ).all(fileScope, topic);
      for (const conflict of conflicts) {
        db.prepare("UPDATE memory_index SET importance = MIN(importance, 0.3) WHERE id = ?").run(conflict.id);
        const cEntry = db.prepare("SELECT id, metadata FROM memory_entries WHERE index_id = ? LIMIT 1").get(conflict.id);
        if (cEntry) {
          let meta = {};
          try { meta = JSON.parse(cEntry.metadata || "{}"); } catch {}
          meta.contradiction = true;
          meta.contradicted_by_topic = topic;
          db.prepare("UPDATE memory_entries SET metadata = ? WHERE id = ?").run(JSON.stringify(meta), cEntry.id);
        }
      }
      if (conflicts.length > 0) adjustedImportance = Math.min(adjustedImportance, 0.3);
    } catch { /* best-effort */ }
  }

  const now = nowIso();
  const indexStmt = db.prepare(
    `INSERT INTO memory_index (memory_type, topic, summary, wp_id, file_scope, importance, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)`
  );
  // RGF-143: normalize relative date references
  const normalizedSummary = normalizeDates(summary);
  const normalizedContent = normalizeDates(content);

  const result = indexStmt.run(memoryType, topic, normalizedSummary, wpId, fileScope, adjustedImportance, now);
  const indexId = Number(result.lastInsertRowid);

  if (normalizedContent) {
    db.prepare(
      `INSERT INTO memory_entries (index_id, content, source_artifact, source_wp_id, source_role, source_session, metadata, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
    ).run(indexId, normalizedContent, sourceArtifact, sourceWpId || wpId, sourceRole, sourceSession, JSON.stringify(metadata), now);
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
    // RGF-142: connectivity scoring — memories linked to many others resist decay
    const fileScope = db.prepare("SELECT file_scope FROM memory_index WHERE id = ?").get(entry.id)?.file_scope || "";
    let connectivityBoost = 1.0;
    if (fileScope) {
      const linked = db.prepare(
        "SELECT COUNT(*) as cnt FROM memory_index WHERE file_scope = ? AND id != ? AND consolidated = 0"
      ).get(fileScope, entry.id)?.cnt || 0;
      if (linked >= 3) connectivityBoost = 1.3;
      else if (linked >= 1) connectivityBoost = 1.1;
    }
    const newImportance = entry.importance * Math.exp(-decayRate * daysSinceAccess) * connectivityBoost;

    if (newImportance < pruneThreshold) {
      db.prepare("UPDATE memory_index SET consolidated = 1, importance = ? WHERE id = ?").run(newImportance, entry.id);
      pruned++;
    } else if (Math.abs(newImportance - entry.importance) > 0.001) {
      db.prepare("UPDATE memory_index SET importance = ? WHERE id = ?").run(Math.min(newImportance, 1.0), entry.id);
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

// ---------------------------------------------------------------------------
// RGF-126: Single-receipt memory extraction (event-driven)
// ---------------------------------------------------------------------------

export const HIGH_SIGNAL_RECEIPT_KINDS = new Set([
  "CODER_INTENT", "CODER_HANDOFF", "VALIDATOR_KICKOFF", "VALIDATOR_REVIEW",
  "VALIDATOR_RESPONSE", "REVIEW_REQUEST", "REVIEW_RESPONSE",
  "STEERING", "REPAIR", "WORKFLOW_INVALIDITY", "SPEC_GAP", "SPEC_CONFIRMATION",
  "MEMORY_PROPOSAL", "MEMORY_FLAG", "MEMORY_RGF_CANDIDATE",
]);
const HIGH_IMPORTANCE_RECEIPT_KINDS = new Set([
  "STEERING", "REPAIR", "WORKFLOW_INVALIDITY", "SPEC_GAP",
  "MEMORY_PROPOSAL", "MEMORY_RGF_CANDIDATE",
]);

export function extractMemoryFromReceipt(db, wpId, receipt) {
  if (!receipt || !HIGH_SIGNAL_RECEIPT_KINDS.has(receipt.receipt_kind)) return 0;

  const mc = receipt.microtask_contract;
  const fileScope = mc && Array.isArray(mc.file_targets) && mc.file_targets.length > 0
    ? mc.file_targets.join(",") : "";
  const importance = HIGH_IMPORTANCE_RECEIPT_KINDS.has(receipt.receipt_kind) ? 0.8 : 0.5;
  const mtRef = mc?.scope_ref || "";
  const topic = mtRef
    ? `${receipt.receipt_kind} on ${mtRef}`
    : `${receipt.receipt_kind} by ${receipt.actor_role}`;

  // Dedup: skip if this exact topic+wp+type already exists
  const existing = db.prepare(
    "SELECT id FROM memory_index WHERE topic = ? AND wp_id = ? AND memory_type = 'episodic'"
  ).get(topic, wpId);
  if (existing) return 0;

  let added = 0;

  // Episodic memory for the receipt
  const contentLines = [
    `Kind: ${receipt.receipt_kind}`,
    `Role: ${receipt.actor_role} (${receipt.actor_authority_kind || ""})`,
    `Time: ${receipt.timestamp_utc}`,
    receipt.summary ? `Summary: ${receipt.summary}` : "",
    receipt.state_before ? `Before: ${receipt.state_before}` : "",
    receipt.state_after ? `After: ${receipt.state_after}` : "",
    receipt.target_role ? `Target: ${receipt.target_role}` : "",
    mc?.scope_ref ? `MT: ${mc.scope_ref}` : "",
    mc?.review_outcome ? `Outcome: ${mc.review_outcome}` : "",
    mc?.file_targets?.length ? `Files: ${mc.file_targets.join(", ")}` : "",
  ].filter(Boolean).join("\n");

  addMemory(db, {
    memoryType: "episodic",
    topic,
    summary: receipt.summary || `${receipt.receipt_kind} from ${receipt.actor_role}`,
    wpId,
    fileScope,
    importance,
    content: contentLines,
    sourceArtifact: "RECEIPTS.jsonl",
    sourceWpId: wpId,
    sourceRole: receipt.actor_role || "",
    sourceSession: receipt.actor_session || "",
    metadata: {
      receipt_kind: receipt.receipt_kind,
      timestamp_utc: receipt.timestamp_utc,
      correlation_id: receipt.correlation_id || "",
      mt_scope_ref: mtRef,
    },
  });
  added++;

  // Procedural fix pattern for REPAIR receipts with state transitions
  if (receipt.receipt_kind === "REPAIR" && receipt.state_before && receipt.state_after) {
    const fixTopic = `Fix pattern: ${mtRef || receipt.actor_role}`;
    const fixExisting = db.prepare(
      "SELECT id FROM memory_index WHERE topic = ? AND wp_id = ? AND memory_type = 'procedural'"
    ).get(fixTopic, wpId);
    if (!fixExisting) {
      addMemory(db, {
        memoryType: "procedural",
        topic: fixTopic,
        summary: `${receipt.state_before} → ${receipt.state_after}: ${(receipt.summary || "").slice(0, 120)}`,
        wpId,
        fileScope,
        importance: 0.8,
        content: `Before: ${receipt.state_before}\nAfter: ${receipt.state_after}\nSummary: ${receipt.summary || ""}\nRole: ${receipt.actor_role}`,
        sourceArtifact: "RECEIPTS.jsonl",
        sourceWpId: wpId,
        sourceRole: receipt.actor_role || "",
        metadata: { receipt_kind: "REPAIR", timestamp_utc: receipt.timestamp_utc },
      });
      added++;
    }
  }

  return added;
}

// ---------------------------------------------------------------------------
// Conversation log — cross-session conversational memory
// ---------------------------------------------------------------------------

export const VALID_CHECKPOINT_TYPES = [
  "SESSION_OPEN", "PRE_TASK", "INSIGHT", "RESEARCH_CLOSE", "SESSION_CLOSE",
];

const SESSION_MARKER_FILE = "CURRENT_REPOMEM_SESSION.json";

function sessionMarkerPath() {
  return path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", SESSION_MARKER_FILE);
}

export function generateSessionId(role) {
  const ts = new Date().toISOString().replace(/[-:]/g, "").replace("T", "-").slice(0, 15);
  return `${(role || "SESSION").toUpperCase()}-${ts}`;
}

export function getCurrentSession() {
  const markerPath = sessionMarkerPath();
  if (!fs.existsSync(markerPath)) return null;
  try {
    return JSON.parse(fs.readFileSync(markerPath, "utf8"));
  } catch { return null; }
}

export function writeSessionMarker(session) {
  fs.writeFileSync(sessionMarkerPath(), JSON.stringify(session, null, 2));
}

export function clearSessionMarker() {
  const markerPath = sessionMarkerPath();
  if (fs.existsSync(markerPath)) fs.unlinkSync(markerPath);
}

export function addConversationCheckpoint(db, {
  sessionId,
  role = "",
  checkpointType,
  triggerRef = "",
  wpId = "",
  topic,
  content,
  filesReferenced = "",
  decisions = "",
}) {
  if (!VALID_CHECKPOINT_TYPES.includes(checkpointType)) {
    throw new Error(`Invalid checkpoint_type: ${checkpointType}. Must be one of: ${VALID_CHECKPOINT_TYPES.join(", ")}`);
  }
  const now = nowIso();
  const normalizedContent = normalizeDates(content);
  const normalizedDecisions = normalizeDates(decisions);

  const result = db.prepare(
    `INSERT INTO conversation_log
     (session_id, role, timestamp_utc, checkpoint_type, trigger_ref, wp_id, topic, content, files_referenced, decisions)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
  ).run(sessionId, role, now, checkpointType, triggerRef, wpId, topic, normalizedContent, filesReferenced, normalizedDecisions);
  return Number(result.lastInsertRowid);
}

export function getConversationLog(db, { sessionId = "", lastN = 10, sinceDate = "", search = "", wpId = "" } = {}) {
  let sql = "SELECT * FROM conversation_log WHERE 1=1";
  const params = [];

  if (sessionId) { sql += " AND session_id = ?"; params.push(sessionId); }
  if (wpId) { sql += " AND (wp_id = ? OR wp_id = '')"; params.push(wpId); }
  if (sinceDate) { sql += " AND timestamp_utc >= ?"; params.push(sinceDate); }

  if (search) {
    const ftsQuery = sanitizeFtsQuery(search);
    if (ftsQuery) {
      try {
        const ftsIds = db.prepare(
          "SELECT rowid FROM conversation_log_fts WHERE conversation_log_fts MATCH ? ORDER BY rank LIMIT 50"
        ).all(ftsQuery).map(r => r.rowid);
        if (ftsIds.length > 0) {
          sql += ` AND id IN (${ftsIds.map(() => "?").join(",")})`;
          params.push(...ftsIds);
        } else {
          return [];
        }
      } catch { return []; }
    }
  }

  sql += " ORDER BY timestamp_utc DESC LIMIT ?";
  params.push(lastN);
  return db.prepare(sql).all(...params).reverse(); // chronological order
}

export function getLastSession(db) {
  // Find the most recent SESSION_CLOSE, then get all entries for that session_id
  const lastClose = db.prepare(
    "SELECT session_id FROM conversation_log WHERE checkpoint_type = 'SESSION_CLOSE' ORDER BY timestamp_utc DESC LIMIT 1"
  ).get();
  if (!lastClose) {
    // No closed session — try the second-most-recent SESSION_OPEN (current session is most recent)
    const opens = db.prepare(
      "SELECT session_id FROM conversation_log WHERE checkpoint_type = 'SESSION_OPEN' ORDER BY timestamp_utc DESC LIMIT 2"
    ).all();
    if (opens.length < 2) return [];
    return db.prepare(
      "SELECT * FROM conversation_log WHERE session_id = ? ORDER BY timestamp_utc ASC"
    ).all(opens[1].session_id);
  }
  return db.prepare(
    "SELECT * FROM conversation_log WHERE session_id = ? ORDER BY timestamp_utc ASC"
  ).all(lastClose.session_id);
}

export function getRecentConversationContext(db, { maxEntries = 8, maxTokens = 600 } = {}) {
  // For injection: get last session's entries + current session's entries
  const lines = [];
  let tokenCount = 0;

  const lastSession = getLastSession(db);
  const currentSession = getCurrentSession();

  if (lastSession.length > 0) {
    const sessionDate = lastSession[0].timestamp_utc?.slice(0, 10) || "unknown";
    const openEntry = lastSession.find(e => e.checkpoint_type === "SESSION_OPEN");
    const closeEntry = lastSession.find(e => e.checkpoint_type === "SESSION_CLOSE");

    lines.push(`PRIOR SESSION (${sessionDate}, ${lastSession[0].role || "unknown role"}):`);
    tokenCount += 15;

    // Prioritize: SESSION_OPEN, INSIGHTs, RESEARCH_CLOSEs, SESSION_CLOSE
    const priority = ["SESSION_OPEN", "INSIGHT", "RESEARCH_CLOSE", "SESSION_CLOSE"];
    const sorted = [...lastSession].sort((a, b) => {
      const aP = priority.indexOf(a.checkpoint_type);
      const bP = priority.indexOf(b.checkpoint_type);
      return (aP === -1 ? 99 : aP) - (bP === -1 ? 99 : bP);
    });

    let entryCount = 0;
    for (const entry of sorted) {
      if (entryCount >= maxEntries || tokenCount >= maxTokens) break;
      const line = `- [${entry.checkpoint_type}] ${entry.topic}${entry.wp_id ? ` (${entry.wp_id})` : ""}`;
      const lineTokens = Math.ceil(line.length / 4);
      if (tokenCount + lineTokens > maxTokens) break;
      lines.push(line);
      tokenCount += lineTokens;
      entryCount++;

      // Add decisions if present
      if (entry.decisions) {
        const decLine = `  Decisions: ${entry.decisions.slice(0, 150)}`;
        const decTokens = Math.ceil(decLine.length / 4);
        if (tokenCount + decTokens <= maxTokens) {
          lines.push(decLine);
          tokenCount += decTokens;
        }
      }
    }
  }

  if (currentSession) {
    const currentEntries = db.prepare(
      "SELECT * FROM conversation_log WHERE session_id = ? ORDER BY timestamp_utc ASC"
    ).all(currentSession.session_id);
    if (currentEntries.length > 0) {
      lines.push(`CURRENT SESSION (${currentSession.role || "unknown"}, opened ${currentSession.opened_at?.slice(11, 16) || "?"}Z):`);
      tokenCount += 15;
      for (const entry of currentEntries.slice(-5)) {
        if (tokenCount >= maxTokens) break;
        const line = `- [${entry.checkpoint_type}] ${entry.topic}`;
        const lineTokens = Math.ceil(line.length / 4);
        if (tokenCount + lineTokens > maxTokens) break;
        lines.push(line);
        tokenCount += lineTokens;
      }
    }
  }

  return { lines, tokenCount };
}

export function checkSessionGate() {
  const session = getCurrentSession();
  if (!session) {
    return { open: false, message: "REPOMEM_GATE_FAIL: No active session. Run `just repomem open \"<what this session is about>\"` first." };
  }
  // Check staleness — if marker is older than 12 hours, session is likely stale
  const age = Date.now() - new Date(session.opened_at).getTime();
  if (age > 12 * 60 * 60 * 1000) {
    return { open: false, message: `REPOMEM_GATE_FAIL: Session ${session.session_id} is ${Math.round(age / 3600000)}h old. Run \`just repomem close "<summary>"\` then \`just repomem open "<new intent>"\`.` };
  }
  return { open: true, session };
}

// ---------------------------------------------------------------------------
// Embedding pipeline (RGF-118) — Ollama nomic-embed-text
// ---------------------------------------------------------------------------

const OLLAMA_EMBED_URL = "http://localhost:11434/api/embed";
const OLLAMA_EMBED_MODEL = "nomic-embed-text";

export async function generateEmbedding(text) {
  const body = JSON.stringify({ model: OLLAMA_EMBED_MODEL, input: String(text || "").slice(0, 8000) });
  const response = await fetch(OLLAMA_EMBED_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
    signal: AbortSignal.timeout(30000),
  });
  if (!response.ok) throw new Error(`Ollama embed failed: ${response.status}`);
  const data = await response.json();
  if (!data.embeddings?.[0]) throw new Error("No embedding returned");
  return data.embeddings[0];
}

export async function embedMemoryEntry(db, indexId) {
  const idx = db.prepare("SELECT topic, summary, file_scope FROM memory_index WHERE id = ?").get(indexId);
  if (!idx) return false;
  const existing = db.prepare("SELECT id FROM memory_embeddings WHERE index_id = ?").get(indexId);
  if (existing) return false;

  const text = `${idx.topic} ${idx.summary} ${idx.file_scope || ""}`.trim();
  const embedding = await generateEmbedding(text);
  db.prepare(
    "INSERT INTO memory_embeddings (index_id, embedding_model, embedding_dims, embedding, created_at) VALUES (?, ?, ?, ?, ?)"
  ).run(indexId, OLLAMA_EMBED_MODEL, embedding.length, JSON.stringify(embedding), nowIso());
  return true;
}

export async function embedAllUnembedded(db, { batchSize = 20 } = {}) {
  const unembedded = db.prepare(
    `SELECT mi.id FROM memory_index mi
     LEFT JOIN memory_embeddings me ON me.index_id = mi.id
     WHERE me.id IS NULL AND mi.consolidated = 0
     ORDER BY mi.importance DESC LIMIT ?`
  ).all(batchSize);

  let embedded = 0;
  let errors = 0;
  const MAX_ERRORS = 3;
  for (const row of unembedded) {
    try {
      const added = await embedMemoryEntry(db, row.id);
      if (added) embedded++;
    } catch (e) {
      errors++;
      console.error(`[embedding] Failed for #${row.id}: ${e.message}`);
      if (errors >= MAX_ERRORS) {
        console.error(`[embedding] Stopping after ${MAX_ERRORS} consecutive errors`);
        break;
      }
    }
  }
  return embedded;
}

// ---------------------------------------------------------------------------
// Vector search (cosine similarity in JS — no sqlite-vec needed)
// ---------------------------------------------------------------------------

function cosineSimilarity(a, b) {
  if (a.length !== b.length) return 0;
  let dot = 0, normA = 0, normB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  return denom === 0 ? 0 : dot / denom;
}

export async function vectorSearch(db, query, { limit = 20 } = {}) {
  const queryEmbedding = await generateEmbedding(query);
  const allEmbeddings = db.prepare(
    `SELECT me.index_id, me.embedding FROM memory_embeddings me
     JOIN memory_index mi ON mi.id = me.index_id
     WHERE mi.consolidated = 0`
  ).all();

  const scored = [];
  for (const row of allEmbeddings) {
    try {
      const embedding = JSON.parse(row.embedding);
      const similarity = cosineSimilarity(queryEmbedding, embedding);
      scored.push({ indexId: row.index_id, similarity });
    } catch {}
  }
  scored.sort((a, b) => b.similarity - a.similarity);
  return scored.slice(0, limit);
}

// ---------------------------------------------------------------------------
// Hybrid search: FTS5 + vector + RRF (RGF-119)
// ---------------------------------------------------------------------------

const RRF_K = 60;

export async function hybridSearch(db, query, { memoryType = "", wpId = "", limit = 20, vectorWeight = 0.6, ftsWeight = 0.4 } = {}) {
  const ftsResults = searchMemories(db, query, { memoryType, wpId, limit: limit * 2 });

  let vectorResults = [];
  try {
    vectorResults = await vectorSearch(db, query, { limit: limit * 2 });
  } catch {}

  const rrfScores = new Map();

  for (let rank = 0; rank < ftsResults.length; rank++) {
    const id = ftsResults[rank].id;
    rrfScores.set(id, (rrfScores.get(id) || 0) + ftsWeight / (RRF_K + rank + 1));
  }

  for (let rank = 0; rank < vectorResults.length; rank++) {
    const id = vectorResults[rank].indexId;
    rrfScores.set(id, (rrfScores.get(id) || 0) + vectorWeight / (RRF_K + rank + 1));
  }

  const sorted = [...rrfScores.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, limit);

  const results = [];
  for (const [id, score] of sorted) {
    const idx = db.prepare("SELECT * FROM memory_index WHERE id = ?").get(id);
    if (!idx) continue;
    if (memoryType && idx.memory_type !== memoryType) continue;
    if (wpId && idx.wp_id && idx.wp_id !== wpId) continue;
    const entry = db.prepare("SELECT content, source_artifact, source_role FROM memory_entries WHERE index_id = ? LIMIT 1").get(id);
    results.push({
      ...idx,
      content: entry?.content || "",
      source_artifact: entry?.source_artifact || "",
      source_role: entry?.source_role || "",
      _rrf_score: score,
    });
  }
  return results;
}
