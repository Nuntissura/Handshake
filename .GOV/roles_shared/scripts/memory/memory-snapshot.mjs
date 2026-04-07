/**
 * RGF-144: Pre-task snapshot capture for governance memory.
 *
 * Captures context and intent BEFORE complex operations so post-hoc analysis
 * can compare what was planned vs what happened.
 *
 * Follows memory-capture-from-check.mjs pattern:
 *   - Best-effort: failures are silent, never block the triggering operation
 *   - Dedup: skips if same snapshot_type + wp_id written in last 60 seconds
 *   - Importance: 0.85 (high-signal by definition)
 *
 * Snapshot types:
 *   PRE_WP_DELEGATION      — before launch-cli-session hands off to a role
 *   PRE_STEERING            — before orchestrator-steer-next evaluates routing
 *   PRE_RELAY_DISPATCH      — before manual-relay-dispatch sends to next actor
 *   PRE_CLOSEOUT            — before integration-validator-closeout-sync evaluates
 *   PRE_PACKET_CREATE       — before create-task-packet generates a work packet
 *   PRE_BOARD_STATUS_CHANGE — before task-board-set moves a WP between sections
 *
 * Usage:
 *   import { capturePreTaskSnapshot } from "../memory/memory-snapshot.mjs";
 *   capturePreTaskSnapshot({
 *     snapshotType: "PRE_WP_DELEGATION",
 *     wpId: "WP-1-Foo-v1",
 *     triggerScript: "launch-cli-session.mjs",
 *     context: { role: "CODER", model: "...", branch: "..." },
 *   });
 */

import {
  openGovernanceMemoryDb,
  closeDb,
  addMemory,
} from "./governance-memory-lib.mjs";

export const VALID_SNAPSHOT_TYPES = [
  "PRE_WP_DELEGATION",
  "PRE_STEERING",
  "PRE_RELAY_DISPATCH",
  "PRE_CLOSEOUT",
  "PRE_PACKET_CREATE",
  "PRE_BOARD_STATUS_CHANGE",
  "INTENT",
];

const DEDUP_WINDOW_MS = 60_000; // 60 seconds

/**
 * Capture a pre-task snapshot into governance memory.
 *
 * @param {object} opts
 * @param {string} opts.snapshotType - One of VALID_SNAPSHOT_TYPES
 * @param {string} opts.wpId - Work packet ID (may be empty for cross-WP ops)
 * @param {string} opts.triggerScript - Script name that triggered the snapshot
 * @param {object} opts.context - Structured context payload (serialized to JSON)
 * @param {string} [opts.summary] - Optional human-readable summary override
 */
export function capturePreTaskSnapshot({
  snapshotType = "",
  wpId = "",
  triggerScript = "",
  context = {},
  summary = "",
} = {}) {
  try {
    if (!snapshotType || !VALID_SNAPSHOT_TYPES.includes(snapshotType)) return;

    const { db } = openGovernanceMemoryDb();
    try {
      // Ensure snapshot_type column exists (schema migration)
      ensureSnapshotTypeColumn(db);

      // Dedup: skip if same snapshot_type + wp_id written in last 60s
      const cutoff = new Date(Date.now() - DEDUP_WINDOW_MS).toISOString();
      const existing = db.prepare(
        "SELECT id FROM memory_index WHERE snapshot_type = ? AND wp_id = ? AND created_at > ?"
      ).get(snapshotType, wpId, cutoff);
      if (existing) return;

      const topic = `${snapshotType}: ${wpId || "cross-WP"}`;
      const contextStr = JSON.stringify(context, null, 0);
      const defaultSummary = `${snapshotType} snapshot before ${triggerScript || "unknown"} for ${wpId || "cross-WP"}`;

      const indexId = addMemory(db, {
        memoryType: "episodic",
        topic,
        summary: summary || defaultSummary,
        wpId,
        importance: 0.85,
        content: contextStr,
        sourceArtifact: triggerScript || "memory-snapshot",
        sourceRole: "ORCHESTRATOR",
        metadata: {
          snapshot_type: snapshotType,
          trigger_script: triggerScript,
          pre_task_snapshot: true,
        },
      });

      // Set snapshot_type on the index row (addMemory doesn't know about this column)
      if (indexId) {
        db.prepare("UPDATE memory_index SET snapshot_type = ? WHERE id = ?").run(snapshotType, indexId);
      }
    } finally {
      closeDb(db);
    }
  } catch {
    // Best-effort: snapshot capture failure must not block the triggering operation
  }
}

/**
 * Capture a context-and-intent snapshot — judgment-based, not mechanical.
 *
 * Called by roles before complex reasoning tasks where the protocol requires
 * recording what is about to happen and why. No automatic trigger exists;
 * the model must decide to call this based on protocol guidance.
 *
 * @param {object} opts
 * @param {string} opts.wpId - Work packet ID (may be empty for gov-only work)
 * @param {string} opts.role - Role capturing the intent (ORCHESTRATOR, CODER, etc.)
 * @param {string} opts.intent - What the role is about to do (1-2 sentences)
 * @param {string} [opts.reason] - Why this action is being taken
 * @param {string} [opts.expectedOutcome] - What the role expects to produce
 * @param {string} [opts.scope] - Files or surfaces involved
 */
export function captureIntentSnapshot({
  wpId = "",
  role = "",
  intent = "",
  reason = "",
  expectedOutcome = "",
  scope = "",
} = {}) {
  try {
    if (!intent) return;

    const { db } = openGovernanceMemoryDb();
    try {
      ensureSnapshotTypeColumn(db);

      // Dedup: skip if same role + wp wrote an intent in last 120s
      // (wider window than mechanical snapshots — intent captures are less frequent)
      const cutoff = new Date(Date.now() - 120_000).toISOString();
      const existing = db.prepare(
        "SELECT id FROM memory_index WHERE snapshot_type = 'INTENT' AND wp_id = ? AND created_at > ?"
      ).get(wpId, cutoff);
      if (existing) return;

      const topic = `INTENT: ${intent.slice(0, 70)}`;
      const summary = [
        intent,
        reason ? `Reason: ${reason}` : "",
        expectedOutcome ? `Expected: ${expectedOutcome}` : "",
      ].filter(Boolean).join(" | ");

      const indexId = addMemory(db, {
        memoryType: "episodic",
        topic,
        summary,
        wpId,
        fileScope: scope,
        importance: 0.9, // higher than mechanical snapshots — this is deliberate
        content: JSON.stringify({ intent, reason, expectedOutcome, scope }),
        sourceArtifact: "memory-intent-snapshot",
        sourceRole: role || "UNKNOWN",
        metadata: {
          snapshot_type: "INTENT",
          pre_task_snapshot: true,
          intent_based: true,
        },
      });

      if (indexId) {
        db.prepare("UPDATE memory_index SET snapshot_type = ? WHERE id = ?").run("INTENT", indexId);
      }

      return indexId;
    } finally {
      closeDb(db);
    }
  } catch {
    // Best-effort
  }
}

/**
 * Ensure the snapshot_type column exists on memory_index.
 * Idempotent — safe to call on every snapshot write.
 */
function ensureSnapshotTypeColumn(db) {
  try {
    // Check if column already exists by querying table info
    const columns = db.prepare("PRAGMA table_info(memory_index)").all();
    const hasColumn = columns.some(c => c.name === "snapshot_type");
    if (!hasColumn) {
      db.exec("ALTER TABLE memory_index ADD COLUMN snapshot_type TEXT DEFAULT ''");
      db.exec("CREATE INDEX IF NOT EXISTS idx_memory_snapshot_type ON memory_index(snapshot_type)");
    }
  } catch {
    // Column may already exist from a concurrent write — ignore
  }
}

/**
 * Query recent pre-task snapshots for debugging / inspection.
 *
 * @param {object} db - Open DatabaseSync handle
 * @param {object} [opts]
 * @param {string} [opts.wpId] - Filter by WP
 * @param {string} [opts.snapshotType] - Filter by snapshot type
 * @param {number} [opts.limit] - Max results (default 20)
 * @returns {Array} Snapshot rows with content
 */
export function querySnapshots(db, { wpId = "", snapshotType = "", limit = 20 } = {}) {
  ensureSnapshotTypeColumn(db);
  let sql = `SELECT mi.id, mi.memory_type, mi.topic, mi.summary, mi.wp_id, mi.snapshot_type,
                    mi.importance, mi.access_count, mi.created_at,
                    me.content, me.metadata
             FROM memory_index mi
             LEFT JOIN memory_entries me ON me.index_id = mi.id
             WHERE mi.snapshot_type != '' AND mi.consolidated = 0`;
  const params = [];
  if (wpId) { sql += " AND mi.wp_id = ?"; params.push(wpId); }
  if (snapshotType) { sql += " AND mi.snapshot_type = ?"; params.push(snapshotType); }
  sql += " ORDER BY mi.created_at DESC LIMIT ?";
  params.push(limit);
  return db.prepare(sql).all(...params);
}
