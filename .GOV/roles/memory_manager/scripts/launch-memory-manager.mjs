#!/usr/bin/env node
/**
 * launch-memory-manager.mjs — Memory Manager session launcher.
 *
 * Runs the 4-phase memory hygiene cycle:
 *   Phase 1: Health Assessment (read-only stats)
 *   Phase 2: Active Maintenance (extraction, compaction, conversation promotion)
 *   Phase 3: Pattern Analysis (cross-WP, conversation insight patterns)
 *   Phase 4: Report (MEMORY_HYGIENE_REPORT.md)
 *
 * Staleness gate: skips unless >24h since last run AND >10 new entries.
 * Use --force to bypass the staleness gate.
 *
 * Usage:
 *   node launch-memory-manager.mjs [--force]
 */

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  openGovernanceMemoryDb,
  closeDb,
  getStats,
  runDecay,
  addMemory,
} from "../../../roles_shared/scripts/memory/governance-memory-lib.mjs";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  GOV_ROOT_ABS,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const args = process.argv.slice(2);
const force = args.includes("--force");
const startTime = Date.now();

const LAST_RUN_FILE = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "MEMORY_MANAGER_LAST_RUN.json");
const REPORT_FILE = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "MEMORY_HYGIENE_REPORT.md");
const STALENESS_HOURS = 24;
const ACTIVITY_THRESHOLD = 10;

// ---------------------------------------------------------------------------
// Staleness gate
// ---------------------------------------------------------------------------

function readLastRun() {
  try {
    if (!fs.existsSync(LAST_RUN_FILE)) return null;
    return JSON.parse(fs.readFileSync(LAST_RUN_FILE, "utf8"));
  } catch { return null; }
}

function writeLastRun(data) {
  fs.writeFileSync(LAST_RUN_FILE, JSON.stringify(data, null, 2));
}

const lastRun = readLastRun();
const hoursSinceLastRun = lastRun?.timestamp
  ? (Date.now() - new Date(lastRun.timestamp).getTime()) / (1000 * 60 * 60)
  : Infinity;

const { db } = openGovernanceMemoryDb();

try {
  const stats = getStats(db);
  const newEntriesSinceLastRun = lastRun?.active_count
    ? Math.max(0, stats.active - lastRun.active_count)
    : stats.active;

  if (!force && hoursSinceLastRun < STALENESS_HOURS) {
    console.log(`[memory-manager] Skipped: last run ${hoursSinceLastRun.toFixed(1)}h ago (gate: ${STALENESS_HOURS}h)`);
    process.exit(0);
  }
  if (!force && newEntriesSinceLastRun < ACTIVITY_THRESHOLD) {
    console.log(`[memory-manager] Skipped: only ${newEntriesSinceLastRun} new entries since last run (gate: ${ACTIVITY_THRESHOLD})`);
    process.exit(0);
  }

  console.log(`[memory-manager] Starting (${force ? "forced" : "staleness gate passed"})...`);

  const report = [];
  const actions = [];
  const calibration = [];
  const contradictions = [];
  const candidates = [];
  const recommendations = [];

  // =========================================================================
  // Phase 1: Health Assessment
  // =========================================================================

  report.push("## Health");
  report.push(`- Active: ${stats.active} | Consolidated: ${stats.consolidated} | By type: ${Object.entries(stats.by_type).map(([k, v]) => `${k}=${v}`).join(" ")}`);

  // Snapshot compliance (7 days)
  const sevenDaysAgo = new Date(Date.now() - 7 * 86400000).toISOString();
  let mechanicalSnapshots = 0;
  let intentSnapshots = 0;
  try {
    const cols = db.prepare("PRAGMA table_info(memory_index)").all();
    if (cols.some(c => c.name === "snapshot_type")) {
      mechanicalSnapshots = db.prepare(
        "SELECT COUNT(*) as c FROM memory_index WHERE snapshot_type != '' AND snapshot_type != 'INTENT' AND created_at > ?"
      ).get(sevenDaysAgo)?.c || 0;
      intentSnapshots = db.prepare(
        "SELECT COUNT(*) as c FROM memory_index WHERE snapshot_type = 'INTENT' AND created_at > ?"
      ).get(sevenDaysAgo)?.c || 0;
    }
  } catch {}
  report.push(`- Snapshots (7d): mechanical=${mechanicalSnapshots} intent=${intentSnapshots}`);

  // Last compaction
  report.push(`- Last compaction: ${stats.last_compaction?.run_at || "never"}`);

  // DB size
  const sizeStatus = stats.active > 450 ? "OVER_CAP" : stats.active > 400 ? "APPROACHING_CAP" : "OK";
  report.push(`- DB size status: ${sizeStatus} (${stats.active}/500)`);

  // Embedding coverage
  let embeddingCount = 0;
  try {
    embeddingCount = db.prepare(
      "SELECT COUNT(DISTINCT me.index_id) as c FROM memory_embeddings me JOIN memory_index mi ON mi.id = me.index_id WHERE mi.consolidated = 0"
    ).get()?.c || 0;
  } catch {}
  const embeddingPct = stats.active > 0 ? Math.round(100 * embeddingCount / stats.active) : 0;
  report.push(`- Embedding coverage: ${embeddingCount}/${stats.active} (${embeddingPct}%)`);

  // Trust distribution
  const trustDist = {};
  try {
    const rows = db.prepare(
      "SELECT source_artifact, COUNT(*) as c FROM memory_entries me JOIN memory_index mi ON mi.id = me.index_id WHERE mi.consolidated = 0 GROUP BY source_artifact ORDER BY c DESC"
    ).all();
    for (const r of rows) trustDist[r.source_artifact || "unknown"] = r.c;
  } catch {}
  report.push(`- Trust distribution: ${Object.entries(trustDist).map(([k, v]) => `${k}=${v}`).join(" ")}`);

  // Conversation log stats
  let conversationTotal = 0;
  let conversationSessions = 0;
  let conversationInsights = 0;
  try {
    const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='conversation_log'").get();
    if (tables) {
      conversationTotal = db.prepare("SELECT COUNT(*) as c FROM conversation_log").get()?.c || 0;
      conversationSessions = db.prepare("SELECT COUNT(DISTINCT session_id) as c FROM conversation_log").get()?.c || 0;
      conversationInsights = db.prepare(
        "SELECT COUNT(*) as c FROM conversation_log WHERE checkpoint_type = 'INSIGHT'"
      ).get()?.c || 0;
    }
  } catch {}
  report.push(`- Conversation log: ${conversationTotal} entries, ${conversationSessions} sessions, ${conversationInsights} insights`);

  // =========================================================================
  // Phase 2: Active Maintenance
  // =========================================================================

  // 2a. Run extraction (receipts + smoketests)
  const scriptsDir = path.join(GOV_ROOT_ABS, "roles_shared", "scripts", "memory");
  let extractedReceipts = 0;
  let extractedSmoketests = 0;

  try {
    const receiptResult = spawnSync(process.execPath, [path.join(scriptsDir, "memory-extract-from-receipts.mjs"), "--all"], {
      stdio: ["pipe", "pipe", "pipe"], timeout: 30000,
    });
    if (receiptResult.status === 0) {
      const match = receiptResult.stdout?.toString().match(/extracted\s+(\d+)/i);
      extractedReceipts = match ? Number(match[1]) : 0;
    }
  } catch {}

  try {
    const smoketestResult = spawnSync(process.execPath, [path.join(scriptsDir, "memory-extract-from-smoketests.mjs")], {
      stdio: ["pipe", "pipe", "pipe"], timeout: 30000,
    });
    if (smoketestResult.status === 0) {
      const match = smoketestResult.stdout?.toString().match(/extracted\s+(\d+)/i);
      extractedSmoketests = match ? Number(match[1]) : 0;
    }
  } catch {}

  // 2b. Compaction (decay + prune)
  let compactResult = { processed: 0, decayed: 0, pruned: 0 };
  try {
    compactResult = runDecay(db, { decayRate: 0.1, pruneThreshold: 0.05 });
  } catch (e) {
    actions.push(`Compaction error: ${e.message}`);
  }

  // 2c. Stale file_scope audit
  let staleFlagged = 0;
  try {
    const proceduralWithScope = db.prepare(
      "SELECT id, topic, file_scope FROM memory_index WHERE memory_type IN ('procedural', 'semantic') AND file_scope != '' AND consolidated = 0 AND created_at < ?"
    ).all(sevenDaysAgo);
    for (const entry of proceduralWithScope) {
      const files = entry.file_scope.split(",").map(f => f.trim()).filter(Boolean);
      if (files.length === 0) continue;
      const existingFiles = files.filter(f => {
        try { return fs.existsSync(path.resolve(process.cwd(), f)); } catch { return false; }
      });
      if (existingFiles.length === 0) {
        const hasAccess = db.prepare("SELECT access_count FROM memory_index WHERE id = ?").get(entry.id);
        if (hasAccess && hasAccess.access_count < 2) {
          db.prepare("UPDATE memory_index SET importance = 0.1 WHERE id = ?").run(entry.id);
          staleFlagged++;
          actions.push(`Flagged stale #${entry.id}: ${entry.topic} (all file refs gone, low access)`);
        }
      }
    }
  } catch {}

  // 2d. Contradiction resolution — find entries flagged with contradiction metadata
  let contradictionsResolved = 0;
  try {
    const contradicted = db.prepare(
      "SELECT mi.id, mi.topic, mi.importance, mi.created_at, mi.file_scope, mi.memory_type, me.metadata FROM memory_index mi JOIN memory_entries me ON me.index_id = mi.id WHERE mi.consolidated = 0 AND me.metadata LIKE '%contradiction%'"
    ).all();
    // Group by file_scope to find pairs
    const byScope = new Map();
    for (const entry of contradicted) {
      const scope = entry.file_scope || "__global__";
      if (!byScope.has(scope)) byScope.set(scope, []);
      byScope.get(scope).push(entry);
    }
    for (const [, entries] of byScope) {
      if (entries.length < 2) continue;
      // Sort by created_at desc — newer wins unless older has more access
      entries.sort((a, b) => b.created_at.localeCompare(a.created_at));
      const newer = entries[0];
      for (let i = 1; i < entries.length; i++) {
        const older = entries[i];
        // Newer wins: consolidate older, restore newer importance
        db.prepare("UPDATE memory_index SET consolidated = 1 WHERE id = ?").run(older.id);
        if (newer.importance < 0.5) {
          db.prepare("UPDATE memory_index SET importance = 0.6 WHERE id = ?").run(newer.id);
        }
        contradictionsResolved++;
        contradictions.push(`- #${older.id} vs #${newer.id}: newer wins — consolidated #${older.id} ("${older.topic.slice(0, 60)}")`);
      }
    }
  } catch {}

  // 2e. Supersession chain audit — check superseded_by chains for broken links
  let supersessionRepairs = 0;
  try {
    const superseded = db.prepare(
      "SELECT mi.id, mi.topic, me.metadata FROM memory_index mi JOIN memory_entries me ON me.index_id = mi.id WHERE mi.consolidated = 1 AND me.metadata LIKE '%superseded_by%'"
    ).all();
    for (const entry of superseded) {
      try {
        const meta = JSON.parse(entry.metadata || "{}");
        if (meta.superseded_by) {
          const successor = db.prepare("SELECT id, consolidated FROM memory_index WHERE id = ?").get(meta.superseded_by);
          if (!successor || successor.consolidated === 1) {
            // Successor is gone or also consolidated — un-consolidate the original
            db.prepare("UPDATE memory_index SET consolidated = 0, importance = 0.4 WHERE id = ?").run(entry.id);
            supersessionRepairs++;
            actions.push(`Restored #${entry.id}: "${entry.topic.slice(0, 60)}" (successor #${meta.superseded_by} was pruned/consolidated)`);
          }
        }
      } catch { /* malformed metadata — skip */ }
    }
  } catch {}

  // 2f. Conversation insight promotion (FTS5 similarity instead of exact match)
  let promotedInsights = 0;
  try {
    const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='conversation_log'").get();
    if (tables) {
      // Get all unique insight topics, then use FTS to find similar ones across sessions
      const insightSessions = db.prepare(
        "SELECT id, session_id, topic, content, wp_id FROM conversation_log WHERE checkpoint_type = 'INSIGHT' ORDER BY timestamp_utc DESC"
      ).all();

      // Group by FTS similarity: for each insight, find others with overlapping keywords
      const promoted = new Set();
      for (const insight of insightSessions) {
        if (promoted.has(insight.id)) continue;
        // Find similar insights across different sessions using keyword overlap
        const keywords = insight.topic.replace(/[^\w\s]/g, " ").split(/\s+/).filter(w => w.length > 3).slice(0, 5).join(" ");
        if (!keywords) continue;
        let similar;
        try {
          similar = db.prepare(
            "SELECT cl.id, cl.session_id, cl.topic, cl.content, cl.wp_id FROM conversation_log cl JOIN conversation_log_fts fts ON fts.rowid = cl.id WHERE fts.topic MATCH ? AND cl.checkpoint_type = 'INSIGHT' AND cl.id != ?"
          ).all(keywords.replace(/[^\w\s]/g, " ").trim(), insight.id);
        } catch { continue; }
        // Count distinct sessions
        const sessions = new Set([insight.session_id, ...similar.map(s => s.session_id)]);
        if (sessions.size < 3) continue;
        // Mark all as processed
        promoted.add(insight.id);
        for (const s of similar) promoted.add(s.id);
        // Check if already promoted
        const existing = db.prepare(
          "SELECT id FROM memory_entries WHERE source_artifact = 'conversation-promotion' AND content LIKE ?"
        ).get(`%${insight.topic.slice(0, 50)}%`);
        if (existing) continue;

        const wpIds = [...new Set([insight.wp_id, ...similar.map(s => s.wp_id)].filter(Boolean))];
        addMemory(db, {
          memoryType: "semantic",
          topic: `[promoted-insight] ${insight.topic.slice(0, 100)}`,
          summary: `Cross-session insight (${sessions.size} sessions): ${insight.content.slice(0, 200)}`,
          wpId: wpIds[0] || "",
          importance: 0.8,
          content: insight.content,
          sourceArtifact: "conversation-promotion",
          metadata: { promoted_from: "conversation_log", session_count: sessions.size },
        });
        promotedInsights++;
        actions.push(`Promoted insight to semantic memory: "${insight.topic.slice(0, 80)}" (${sessions.size} sessions)`);
      }

      // Also promote decisions that repeat across sessions
      const decisions = db.prepare(
        "SELECT decisions, COUNT(DISTINCT session_id) as session_count FROM conversation_log WHERE decisions != '' AND decisions IS NOT NULL GROUP BY decisions HAVING session_count >= 2 ORDER BY session_count DESC LIMIT 10"
      ).all();

      for (const decision of decisions) {
        const existing = db.prepare(
          "SELECT id FROM memory_index WHERE topic = ? AND memory_type = 'semantic'"
        ).get(`[promoted-decision] ${decision.decisions.slice(0, 100)}`);
        if (existing) continue;

        addMemory(db, {
          memoryType: "semantic",
          topic: `[promoted-decision] ${decision.decisions.slice(0, 100)}`,
          summary: `Repeated decision (${decision.session_count} sessions): ${decision.decisions.slice(0, 200)}`,
          importance: 0.75,
          content: decision.decisions,
          sourceArtifact: "conversation-promotion",
          metadata: { promoted_from: "conversation_log_decisions", session_count: decision.session_count },
        });
        promotedInsights++;
        actions.push(`Promoted decision to semantic memory: "${decision.decisions.slice(0, 80)}" (${decision.session_count} sessions)`);
      }
    }
  } catch {}

  // 2g. Conversation log pruning — old SESSION_OPEN/CLOSE without insights
  let conversationPruned = 0;
  try {
    const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='conversation_log'").get();
    if (tables) {
      const thirtyDaysAgo = new Date(Date.now() - 30 * 86400000).toISOString();
      const oldSessions = db.prepare(
        `SELECT session_id, COUNT(*) as total,
                SUM(CASE WHEN checkpoint_type IN ('INSIGHT', 'RESEARCH_CLOSE') THEN 1 ELSE 0 END) as valuable
         FROM conversation_log
         WHERE timestamp_utc < ?
         GROUP BY session_id
         HAVING valuable = 0`
      ).all(thirtyDaysAgo);

      for (const session of oldSessions) {
        const autoCloseCheck = db.prepare(
          "SELECT COUNT(*) as c FROM conversation_log WHERE session_id = ? AND topic LIKE '%(auto-closed%'"
        ).get(session.session_id);
        if (session.total <= 2 && (!autoCloseCheck || autoCloseCheck.c === 0)) {
          db.prepare("DELETE FROM conversation_log WHERE session_id = ?").run(session.session_id);
          conversationPruned += session.total;
        }
      }
    }
  } catch {}

  // 2h. Age-based consolidation — entries >30d old with low access get consolidated
  let ageConsolidated = 0;
  try {
    const thirtyDaysAgo = new Date(Date.now() - 30 * 86400000).toISOString();
    const oldLowAccess = db.prepare(
      "SELECT id, topic FROM memory_index WHERE consolidated = 0 AND created_at < ? AND access_count < 2 AND importance < 0.4"
    ).all(thirtyDaysAgo);
    for (const entry of oldLowAccess) {
      db.prepare("UPDATE memory_index SET consolidated = 1 WHERE id = ?").run(entry.id);
      ageConsolidated++;
    }
    if (ageConsolidated > 0) {
      actions.push(`Age-consolidated: ${ageConsolidated} entries (>30d old, <2 accesses, importance <0.4)`);
    }
  } catch {}

  // 2i. Embedding refresh — if coverage <50% and Ollama is reachable, embed a batch
  let embeddingsAdded = 0;
  if (embeddingPct < 50) {
    try {
      const embedResult = spawnSync(process.execPath, [
        path.join(scriptsDir, "governance-memory-cli.mjs"), "embed", "--batch", "20"
      ], { stdio: ["pipe", "pipe", "pipe"], timeout: 60000 });
      if (embedResult.status === 0) {
        const match = embedResult.stdout?.toString().match(/Embedded\s+(\d+)/i);
        embeddingsAdded = match ? Number(match[1]) : 0;
      }
    } catch {}
    if (embeddingsAdded > 0) actions.push(`Embeddings added: ${embeddingsAdded}`);
  }

  // 2j. Recall effectiveness audit — check operator-reported entries haven't decayed
  let recallAuditNotes = [];
  try {
    const operatorEntries = db.prepare(
      "SELECT mi.id, mi.topic, mi.importance, me.source_artifact FROM memory_index mi JOIN memory_entries me ON me.index_id = mi.id WHERE me.source_artifact IN ('operator-reported', 'memory-capture') AND mi.consolidated = 0"
    ).all();
    const decayedOperator = operatorEntries.filter(e => e.importance < 0.5);
    if (decayedOperator.length > 0) {
      // Restore operator-reported entries that decayed below 0.5 — these are high-value
      for (const entry of decayedOperator) {
        db.prepare("UPDATE memory_index SET importance = 0.8 WHERE id = ?").run(entry.id);
      }
      recallAuditNotes.push(`Restored ${decayedOperator.length} operator-reported entries from decay (importance < 0.5 → 0.8)`);
      actions.push(`Recall audit: restored ${decayedOperator.length} operator-reported entries from decay`);
    }
    recallAuditNotes.push(`Operator-reported entries: ${operatorEntries.length} active (${decayedOperator.length} restored from decay)`);
  } catch {}

  actions.push(`Extracted: receipts=${extractedReceipts}, smoketests=${extractedSmoketests}`);
  actions.push(`Compacted: processed=${compactResult.processed}, decayed=${compactResult.decayed}, pruned=${compactResult.pruned}`);
  actions.push(`Stale flagged: ${staleFlagged}`);
  actions.push(`Contradictions resolved: ${contradictionsResolved}`);
  actions.push(`Supersession chains repaired: ${supersessionRepairs}`);
  actions.push(`Conversation insights promoted: ${promotedInsights}`);
  actions.push(`Conversation entries pruned: ${conversationPruned}`);
  actions.push(`Age-consolidated: ${ageConsolidated}`);

  // =========================================================================
  // Phase 3: Pattern Analysis
  // =========================================================================

  // 3a. Novelty calibration
  let noveltyPenaltyRate = 0;
  try {
    const recentEntries = db.prepare(
      "SELECT COUNT(*) as c FROM memory_index WHERE created_at > ? AND consolidated = 0"
    ).get(sevenDaysAgo)?.c || 0;
    const noveltyPenalized = db.prepare(
      "SELECT COUNT(*) as c FROM memory_index WHERE created_at > ? AND importance < 0.2 AND consolidated = 0"
    ).get(sevenDaysAgo)?.c || 0;
    noveltyPenaltyRate = recentEntries > 0 ? Math.round(100 * noveltyPenalized / recentEntries) : 0;
  } catch {}
  calibration.push(`- Novelty penalty rate: ${noveltyPenaltyRate}% of recent entries hit low importance (threshold: 30%)`);

  // 3b. Session diversity
  try {
    const dominantSessions = db.prepare(
      `SELECT me.source_session, COUNT(*) as c
       FROM memory_entries me JOIN memory_index mi ON mi.id = me.index_id
       WHERE mi.consolidated = 0 AND me.source_session != ''
       GROUP BY me.source_session HAVING c > 5
       ORDER BY c DESC LIMIT 5`
    ).all();
    if (dominantSessions.length > 0) {
      calibration.push(`- Session diversity: ${dominantSessions.length} session(s) with >5 active memories: ${dominantSessions.map(s => `${s.source_session}=${s.c}`).join(", ")}`);
    } else {
      calibration.push(`- Session diversity: OK (no session dominates)`);
    }
  } catch {}

  // 3c. Intent snapshot compliance
  calibration.push(`- Intent snapshot compliance: ${intentSnapshots} in last 7d ${intentSnapshots === 0 ? "(CONCERN: roles not writing intents)" : intentSnapshots > 20 ? "(CONCERN: possible noise)" : "(OK)"}`);

  // 3d. Conversation checkpoint compliance
  try {
    const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='conversation_log'").get();
    if (tables) {
      const recentCheckpoints = db.prepare(
        "SELECT checkpoint_type, COUNT(*) as c FROM conversation_log WHERE timestamp_utc > ? GROUP BY checkpoint_type"
      ).all(sevenDaysAgo);
      const checkpointMap = Object.fromEntries(recentCheckpoints.map(r => [r.checkpoint_type, r.c]));
      calibration.push(`- Conversation checkpoints (7d): OPEN=${checkpointMap.SESSION_OPEN || 0} CLOSE=${checkpointMap.SESSION_CLOSE || 0} INSIGHT=${checkpointMap.INSIGHT || 0} PRE_TASK=${checkpointMap.PRE_TASK || 0} RESEARCH_CLOSE=${checkpointMap.RESEARCH_CLOSE || 0}`);

      const openCount = checkpointMap.SESSION_OPEN || 0;
      const closeCount = checkpointMap.SESSION_CLOSE || 0;
      if (openCount > 0 && closeCount === 0) {
        recommendations.push("Sessions are opened but never closed — models may not be running `just repomem close`");
      }
      if (openCount > 0 && (checkpointMap.INSIGHT || 0) === 0) {
        recommendations.push("Sessions have no INSIGHTs — models may not be capturing operator decisions and discoveries");
      }
    }
  } catch {}

  // 3e. RGF candidates from cross-WP patterns
  try {
    const crossWpPatterns = db.prepare(
      `SELECT topic, COUNT(DISTINCT wp_id) as wp_count, SUM(access_count) as total_access
       FROM memory_index
       WHERE consolidated = 0 AND wp_id != '' AND memory_type = 'procedural'
       GROUP BY topic HAVING wp_count >= 3
       ORDER BY wp_count DESC LIMIT 5`
    ).all();
    for (const p of crossWpPatterns) {
      candidates.push(`- CANDIDATE: Codify "${p.topic}" — appears across ${p.wp_count} WPs, ${p.total_access} total accesses`);
    }

    // High-access memories
    const highAccess = db.prepare(
      "SELECT topic, access_count, memory_type FROM memory_index WHERE access_count >= 10 AND consolidated = 0 ORDER BY access_count DESC LIMIT 5"
    ).all();
    for (const h of highAccess) {
      candidates.push(`- CANDIDATE: Promote "${h.topic}" (${h.memory_type}, ${h.access_count} accesses) to governance rule`);
    }
  } catch {}

  // =========================================================================
  // Phase 4: Report
  // =========================================================================

  const duration = Math.round((Date.now() - startTime) / 1000);
  const finalStats = getStats(db);

  const reportLines = [
    "# Memory Hygiene Report",
    `- Run: ${new Date().toISOString()}`,
    `- Mode: mechanical (no model session)`,
    `- Duration: ${duration}s`,
    `- Trigger: ${force ? "forced" : "staleness gate passed"}`,
    "",
    ...report,
    "",
    "## Actions Taken",
    ...actions.map(a => `- ${a}`),
    "",
  ];

  if (contradictions.length > 0) {
    reportLines.push("## Contradiction Resolutions", ...contradictions, "");
  }

  if (recallAuditNotes.length > 0) {
    reportLines.push("## Recall Effectiveness", ...recallAuditNotes.map(n => `- ${n}`), "");
  }

  reportLines.push("## Calibration Notes", ...calibration, "");

  if (candidates.length > 0) {
    reportLines.push("## RGF Candidates (for orchestrator review)", ...candidates, "");
  }

  if (recommendations.length > 0) {
    reportLines.push("## Recommendations", ...recommendations.map(r => `- ${r}`), "");
  }

  reportLines.push(
    "## Post-Run Stats",
    `- Active: ${finalStats.active} | Consolidated: ${finalStats.consolidated}`,
    `- By type: ${Object.entries(finalStats.by_type).map(([k, v]) => `${k}=${v}`).join(" ")}`,
  );

  fs.writeFileSync(REPORT_FILE, reportLines.join("\n") + "\n");

  // Update last-run marker
  writeLastRun({
    timestamp: new Date().toISOString(),
    duration_seconds: duration,
    active_count: finalStats.active,
    actions_taken: actions.length,
    forced: force,
  });

  console.log(`[memory-manager] Complete in ${duration}s. Report: ${REPORT_FILE}`);
  console.log(`  active=${finalStats.active} extracted=${extractedReceipts + extractedSmoketests} compacted=${compactResult.pruned} promoted=${promotedInsights}`);

} finally {
  closeDb(db);
}
