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
        // All referenced files are gone — check if this has general applicability
        const hasAccess = db.prepare("SELECT access_count FROM memory_index WHERE id = ?").get(entry.id);
        if (hasAccess && hasAccess.access_count < 2) {
          db.prepare("UPDATE memory_index SET importance = 0.1 WHERE id = ?").run(entry.id);
          staleFlagged++;
          actions.push(`Flagged stale #${entry.id}: ${entry.topic} (all file refs gone, low access)`);
        }
      }
    }
  } catch {}

  // 2d. Conversation insight promotion
  // Find insights that appear across 3+ sessions → promote to semantic memory
  let promotedInsights = 0;
  try {
    const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='conversation_log'").get();
    if (tables) {
      // Group insights by topic similarity (exact match for now, FTS for future)
      const insights = db.prepare(
        "SELECT topic, content, COUNT(DISTINCT session_id) as session_count, GROUP_CONCAT(DISTINCT wp_id) as wp_ids FROM conversation_log WHERE checkpoint_type = 'INSIGHT' GROUP BY topic HAVING session_count >= 3 ORDER BY session_count DESC"
      ).all();

      for (const insight of insights) {
        // Check if we already promoted this
        const existing = db.prepare(
          "SELECT id FROM memory_index WHERE topic = ? AND memory_type = 'semantic'"
        ).get(`[promoted-insight] ${insight.topic.slice(0, 100)}`);
        if (existing) continue;

        addMemory(db, {
          memoryType: "semantic",
          topic: `[promoted-insight] ${insight.topic.slice(0, 100)}`,
          summary: `Cross-session insight (${insight.session_count} sessions): ${insight.content.slice(0, 200)}`,
          wpId: insight.wp_ids?.split(",")[0] || "",
          importance: 0.8,
          content: insight.content,
          sourceArtifact: "conversation-promotion",
          metadata: { promoted_from: "conversation_log", session_count: insight.session_count },
        });
        promotedInsights++;
        actions.push(`Promoted insight to semantic memory: "${insight.topic.slice(0, 80)}" (${insight.session_count} sessions)`);
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

  // 2e. Conversation log pruning — old SESSION_OPEN/CLOSE without insights
  let conversationPruned = 0;
  try {
    const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='conversation_log'").get();
    if (tables) {
      const thirtyDaysAgo = new Date(Date.now() - 30 * 86400000).toISOString();
      // Find sessions older than 30 days that have no INSIGHTs or RESEARCH_CLOSEs
      const oldSessions = db.prepare(
        `SELECT session_id, COUNT(*) as total,
                SUM(CASE WHEN checkpoint_type IN ('INSIGHT', 'RESEARCH_CLOSE') THEN 1 ELSE 0 END) as valuable
         FROM conversation_log
         WHERE timestamp_utc < ?
         GROUP BY session_id
         HAVING valuable = 0`
      ).all(thirtyDaysAgo);

      for (const session of oldSessions) {
        // Keep auto-closed entries as they may carry context
        const autoCloseCheck = db.prepare(
          "SELECT COUNT(*) as c FROM conversation_log WHERE session_id = ? AND topic LIKE '%(auto-closed%'"
        ).get(session.session_id);
        // Only prune sessions with just OPEN + CLOSE and no insights
        if (session.total <= 2 && (!autoCloseCheck || autoCloseCheck.c === 0)) {
          db.prepare("DELETE FROM conversation_log WHERE session_id = ?").run(session.session_id);
          conversationPruned += session.total;
        }
      }
    }
  } catch {}

  actions.push(`Extracted: receipts=${extractedReceipts}, smoketests=${extractedSmoketests}`);
  actions.push(`Compacted: processed=${compactResult.processed}, decayed=${compactResult.decayed}, pruned=${compactResult.pruned}`);
  actions.push(`Stale flagged: ${staleFlagged}`);
  actions.push(`Conversation insights promoted: ${promotedInsights}`);
  actions.push(`Conversation entries pruned: ${conversationPruned}`);

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
