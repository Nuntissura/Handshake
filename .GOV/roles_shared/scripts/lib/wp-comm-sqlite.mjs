/**
 * RGF-101: SQLite Communication Backbone for WP Communications.
 *
 * Uses Node.js built-in node:sqlite (Node 22.5+).
 * Schema uses ONLY features portable to PostgreSQL.
 *
 * Database: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/{WP_ID}/wp_comm.db
 */

import fs from "node:fs";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { SHARED_GOV_WP_COMMUNICATIONS_ROOT, repoPathAbs } from "./runtime-paths.mjs";

const CREATE_TABLES_SQL = `
CREATE TABLE IF NOT EXISTS wp_messages (
  id INTEGER PRIMARY KEY,
  wp_id TEXT NOT NULL,
  sender_role TEXT NOT NULL,
  target_role TEXT NOT NULL,
  message_type TEXT NOT NULL,
  content TEXT NOT NULL DEFAULT '{}',
  correlation_id TEXT DEFAULT '',
  acknowledged INTEGER DEFAULT 0,
  created_at TEXT NOT NULL,
  acknowledged_at TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS mt_tasks (
  id INTEGER PRIMARY KEY,
  wp_id TEXT NOT NULL,
  mt_id TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'pending',
  complexity_tier TEXT NOT NULL DEFAULT 'MEDIUM',
  claimed_by TEXT DEFAULT '',
  claimed_at TEXT DEFAULT '',
  completed_at TEXT DEFAULT '',
  fix_cycle_count INTEGER DEFAULT 0,
  evidence TEXT DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS schema_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
`;

const CREATE_INDEXES_SQL = `
CREATE INDEX IF NOT EXISTS idx_wp_messages_target ON wp_messages(wp_id, target_role, acknowledged);
CREATE INDEX IF NOT EXISTS idx_wp_messages_type ON wp_messages(wp_id, message_type);
CREATE INDEX IF NOT EXISTS idx_wp_messages_correlation ON wp_messages(correlation_id);
CREATE INDEX IF NOT EXISTS idx_mt_tasks_status ON mt_tasks(wp_id, status);
CREATE INDEX IF NOT EXISTS idx_mt_tasks_mt_id ON mt_tasks(wp_id, mt_id);
`;

export function openWpCommDb(wpId) {
  const wpCommDir = repoPathAbs(path.join(SHARED_GOV_WP_COMMUNICATIONS_ROOT, wpId));
  if (!fs.existsSync(wpCommDir)) {
    fs.mkdirSync(wpCommDir, { recursive: true });
  }
  const dbPath = path.join(wpCommDir, "wp_comm.db");
  const db = new DatabaseSync(dbPath);
  db.exec("PRAGMA journal_mode = WAL");
  db.exec(CREATE_TABLES_SQL);
  db.exec(CREATE_INDEXES_SQL);

  const versionCheck = db.prepare("SELECT value FROM schema_meta WHERE key = 'schema_version'");
  const row = versionCheck.get();
  if (!row) {
    db.prepare("INSERT INTO schema_meta (key, value) VALUES ('schema_version', '1')").run();
  }
  return { db, dbPath };
}

export function insertMessage(db, { wpId, senderRole, targetRole, messageType, content, correlationId = "" }) {
  const stmt = db.prepare(
    "INSERT INTO wp_messages (wp_id, sender_role, target_role, message_type, content, correlation_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
  );
  return stmt.run(wpId, senderRole, targetRole, messageType, JSON.stringify(content), correlationId, new Date().toISOString());
}

export function getUnacknowledgedMessages(db, wpId, targetRole) {
  return db.prepare(
    "SELECT * FROM wp_messages WHERE wp_id = ? AND target_role = ? AND acknowledged = 0 ORDER BY created_at ASC"
  ).all(wpId, targetRole);
}

export function acknowledgeMessage(db, messageId) {
  return db.prepare("UPDATE wp_messages SET acknowledged = 1, acknowledged_at = ? WHERE id = ?").run(new Date().toISOString(), messageId);
}

export function populateMtTasks(db, wpId, microtasks) {
  const stmt = db.prepare(
    "INSERT OR IGNORE INTO mt_tasks (wp_id, mt_id, description, complexity_tier, created_at) VALUES (?, ?, ?, ?, ?)"
  );
  const now = new Date().toISOString();
  for (const mt of microtasks) {
    stmt.run(wpId, mt.mtId, mt.description || "", mt.complexityTier || "MEDIUM", now);
  }
}

export function claimNextMt(db, wpId, sessionKey) {
  const row = db.prepare("SELECT id, mt_id FROM mt_tasks WHERE wp_id = ? AND status = 'pending' ORDER BY id ASC LIMIT 1").get(wpId);
  if (!row) return null;
  const result = db.prepare("UPDATE mt_tasks SET status = 'claimed', claimed_by = ?, claimed_at = ? WHERE id = ? AND status = 'pending'").run(sessionKey, new Date().toISOString(), row.id);
  if (result.changes === 0) return null;
  return { mtId: row.mt_id, taskId: row.id };
}

export function completeMt(db, wpId, mtId, evidence = {}) {
  return db.prepare("UPDATE mt_tasks SET status = 'completed', completed_at = ?, evidence = ? WHERE wp_id = ? AND mt_id = ? AND status = 'claimed'").run(new Date().toISOString(), JSON.stringify(evidence), wpId, mtId);
}

export function incrementFixCycle(db, wpId, mtId) {
  return db.prepare("UPDATE mt_tasks SET fix_cycle_count = fix_cycle_count + 1 WHERE wp_id = ? AND mt_id = ?").run(wpId, mtId);
}

export function getMtTasks(db, wpId) {
  return db.prepare("SELECT * FROM mt_tasks WHERE wp_id = ? ORDER BY id ASC").all(wpId);
}

export function formatMtBoard(db, wpId) {
  const tasks = getMtTasks(db, wpId);
  if (tasks.length === 0) return "No microtasks found.";
  const lines = ["MT Task Board for " + wpId, "\u2500".repeat(60)];
  for (const t of tasks) {
    const status = String(t.status).toUpperCase().padEnd(10);
    const claimed = t.claimed_by ? ` (${t.claimed_by})` : "";
    const fixes = t.fix_cycle_count > 0 ? ` [${t.fix_cycle_count} fix cycles]` : "";
    lines.push(`  ${t.mt_id} | ${status} | ${t.complexity_tier}${claimed}${fixes}`);
    if (t.description) lines.push(`         ${t.description}`);
  }
  return lines.join("\n");
}
