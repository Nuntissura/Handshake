#!/usr/bin/env node
/**
 * memory-recall — Action-scoped memory retrieval.
 *
 * Surfaces relevant memory content before an orchestrator action,
 * so the model sees prior knowledge (tooling pitfalls, decisions,
 * prior session context) instead of starting blind.
 *
 * Usage:
 *   node memory-recall.mjs <ACTION> [--wp WP-{ID}] [--budget N]
 *
 * Actions (mapped to query scopes):
 *   REFINEMENT       — tooling pitfalls, prior failed refinement approaches
 *   RESUME           — last session state, unresolved blockers, decisions
 *   STEERING         — prior steering failures, coder behavior patterns
 *   RELAY            — relay communication issues, prior relay outcomes
 *   DELEGATION       — packet creation issues, dependency pitfalls
 *   PACKET_CREATE    — prior packet failures, template issues
 *
 * Output: structured block printed to stdout for model consumption.
 * Exit 0 always (best-effort; never blocks the calling command).
 */

import {
  openGovernanceMemoryDb,
  closeDb,
  searchMemories,
  getPointerIndex,
  getMemoryEntry,
} from "./governance-memory-lib.mjs";

// ---------------------------------------------------------------------------
// Action → query scope definitions
// ---------------------------------------------------------------------------

const ACTION_SCOPES = {
  REFINEMENT: {
    label: "Refinement",
    description: "Prior refinement knowledge — tooling pitfalls, failed approaches, workarounds",
    queries: [
      { type: "procedural", keywords: "patch refinement edit scaffold tool payload path" },
      { type: "procedural", keywords: "Windows path limit file size" },
      { type: "episodic",   keywords: "refinement failed retry workaround" },
      { type: "semantic",   keywords: "refinement scope discovery primitives" },
    ],
    // Also fetch top procedural memories regardless of keyword match
    topN: { memoryType: "procedural", limit: 10 },
  },
  RESUME: {
    label: "Resume",
    description: "Prior session state — last actions, unresolved blockers, key decisions",
    queries: [
      { type: "episodic",   keywords: "session close handoff blocker" },
      { type: "semantic",   keywords: "decision architecture approach" },
      { type: "procedural", keywords: "fix repair workaround" },
    ],
    topN: { memoryType: "episodic", limit: 10 },
    includeConversationLog: true,
  },
  STEERING: {
    label: "Steering",
    description: "Prior steering knowledge — what worked, what failed, coder patterns",
    queries: [
      { type: "procedural", keywords: "steering steer coder session" },
      { type: "episodic",   keywords: "steering handoff coder validator" },
      { type: "semantic",   keywords: "coder behavior pattern" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
  RELAY: {
    label: "Relay",
    description: "Prior relay knowledge — communication patterns, dispatch issues",
    queries: [
      { type: "procedural", keywords: "relay dispatch communication" },
      { type: "episodic",   keywords: "relay manual dispatch escalation" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
  DELEGATION: {
    label: "Delegation",
    description: "Prior delegation knowledge — packet issues, worktree setup, dependency pitfalls",
    queries: [
      { type: "procedural", keywords: "delegation packet worktree dependency" },
      { type: "episodic",   keywords: "delegation launch coder worktree" },
      { type: "semantic",   keywords: "dependency blocker build order" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
  CODER_RESUME: {
    label: "Coder resume",
    description: "Prior coder session knowledge — implementation pitfalls, tool failures, fix patterns",
    queries: [
      { type: "procedural", keywords: "compile build import path error tool" },
      { type: "procedural", keywords: "edit patch file size payload limit" },
      { type: "procedural", keywords: "test runner cargo npm fixture" },
      { type: "episodic",   keywords: "coder handoff implementation blocker" },
    ],
    topN: { memoryType: "procedural", limit: 10 },
    includeConversationLog: true,
  },
  VALIDATOR_RESUME: {
    label: "Validator resume",
    description: "Prior validator session knowledge — validation pitfalls, check failures, spec drift",
    queries: [
      { type: "procedural", keywords: "validation check false positive spec anchor" },
      { type: "procedural", keywords: "smoketest parser review regression" },
      { type: "semantic",   keywords: "spec drift governance alignment" },
      { type: "episodic",   keywords: "validator review verdict blocker" },
    ],
    topN: { memoryType: "procedural", limit: 10 },
    includeConversationLog: true,
  },
  PACKET_CREATE: {
    label: "Packet creation",
    description: "Prior packet creation knowledge — template issues, field errors",
    queries: [
      { type: "procedural", keywords: "packet template create task" },
      { type: "episodic",   keywords: "packet creation failed" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
};

const VALID_ACTIONS = Object.keys(ACTION_SCOPES);

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

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

const [action, ...rawArgs] = process.argv.slice(2);
const { flags } = parseFlags(rawArgs);
const wpId = flags.wp || "";
const tokenBudget = Number(flags.budget) || 1500;

if (!action || !VALID_ACTIONS.includes(action.toUpperCase())) {
  console.error(`Usage: memory-recall.mjs <${VALID_ACTIONS.join("|")}> [--wp WP-{ID}] [--budget N]`);
  process.exit(0); // best-effort — never block
}

const scope = ACTION_SCOPES[action.toUpperCase()];

// ---------------------------------------------------------------------------
// Main recall logic
// ---------------------------------------------------------------------------

try {
  const { db } = openGovernanceMemoryDb();

  try {
    const seen = new Set();
    const collected = [];

    // 1. Run scoped keyword searches
    for (const q of scope.queries) {
      const results = searchMemories(db, q.keywords, {
        memoryType: q.type,
        wpId,
        limit: 10,
      });
      for (const r of results) {
        if (!seen.has(r.id) && r.consolidated === 0) {
          seen.add(r.id);
          collected.push(r);
        }
      }
    }

    // 2. Fetch top-N by importance for the primary memory type
    if (scope.topN) {
      const topResults = getPointerIndex(db, {
        memoryType: scope.topN.memoryType,
        wpId,
        limit: scope.topN.limit,
      });
      for (const r of topResults) {
        if (!seen.has(r.id)) {
          seen.add(r.id);
          // Fetch content for top-N entries
          const entry = getMemoryEntry(db, r.id);
          collected.push({
            ...r,
            content: entry?.content || "",
            source_artifact: entry?.source_artifact || "",
          });
        }
      }
    }

    // 3. WP-scoped entries (all types) if wpId provided
    if (wpId) {
      const wpResults = getPointerIndex(db, { wpId, limit: 10 });
      for (const r of wpResults) {
        if (!seen.has(r.id)) {
          seen.add(r.id);
          const entry = getMemoryEntry(db, r.id);
          collected.push({
            ...r,
            content: entry?.content || "",
            source_artifact: entry?.source_artifact || "",
          });
        }
      }
    }

    // 4. Optional: last conversation log entries for RESUME action
    let conversationEntries = [];
    if (scope.includeConversationLog) {
      try {
        let sql = `SELECT checkpoint_type, topic, content, decisions, wp_id, timestamp_utc
                    FROM conversation_log ORDER BY timestamp_utc DESC LIMIT 8`;
        if (wpId) {
          sql = `SELECT checkpoint_type, topic, content, decisions, wp_id, timestamp_utc
                 FROM conversation_log WHERE wp_id = ? OR wp_id = ''
                 ORDER BY timestamp_utc DESC LIMIT 8`;
        }
        conversationEntries = wpId
          ? db.prepare(sql).all(wpId)
          : db.prepare(sql).all();
      } catch { /* conversation_log may not exist in older schemas */ }
    }

    // 5. Sort by importance (operator-reported entries get a boost) and apply token budget
    const OPERATOR_SOURCES = new Set(["operator-reported", "memory-capture"]);
    collected.sort((a, b) => {
      const aBoost = OPERATOR_SOURCES.has(a.source_artifact) ? 0.5 : 0;
      const bBoost = OPERATOR_SOURCES.has(b.source_artifact) ? 0.5 : 0;
      return ((b.importance || 0) + bBoost) - ((a.importance || 0) + aBoost);
    });

    let tokenEstimate = 0;
    const budgeted = [];
    for (const r of collected) {
      const text = r.content || r.summary || "";
      const entryTokens = Math.ceil((r.topic.length + text.length) / 4);
      if (tokenEstimate + entryTokens > tokenBudget) continue;
      tokenEstimate += entryTokens;
      budgeted.push(r);
    }

    // ---------------------------------------------------------------------------
    // Output
    // ---------------------------------------------------------------------------

    if (budgeted.length === 0 && conversationEntries.length === 0) {
      console.log(`MEMORY_RECALL [${scope.label}]: no relevant memories found.`);
    } else {
      console.log(`MEMORY_RECALL [${scope.label}]`);
      console.log(`  scope: ${scope.description}`);
      console.log(`  entries: ${budgeted.length} (budget: ~${tokenBudget} tokens)`);
      if (wpId) console.log(`  wp: ${wpId}`);
      console.log("");

      if (budgeted.length > 0) {
        // Separate operator-reported warnings from system-extracted memories
        const operatorEntries = budgeted.filter(r => OPERATOR_SOURCES.has(r.source_artifact));
        const systemEntries = budgeted.filter(r => !OPERATOR_SOURCES.has(r.source_artifact));

        if (operatorEntries.length > 0) {
          console.log("  *** OPERATOR WARNINGS (do not repeat these mistakes) ***");
          for (const r of operatorEntries) {
            const content = r.content || r.summary || "";
            const wpTag = r.wp_id ? ` [${r.wp_id}]` : "";
            console.log(`  [!]${wpTag} ${r.topic}`);
            console.log(`    ${content}`);
            console.log("");
          }
        }

        if (systemEntries.length > 0) {
          console.log("  SYSTEM FINDINGS:");
          for (const r of systemEntries) {
            const content = r.content || r.summary || "";
            const typeTag = r.memory_type.toUpperCase().slice(0, 4);
            const wpTag = r.wp_id ? ` [${r.wp_id}]` : "";
            console.log(`  [${typeTag}]${wpTag} ${r.topic}`);
            console.log(`    ${content}`);
            if (r.file_scope) console.log(`    files: ${r.file_scope}`);
            console.log("");
          }
        }
      }

      if (conversationEntries.length > 0) {
        console.log("  PRIOR_SESSION_LOG:");
        for (const c of conversationEntries) {
          const wpTag = c.wp_id ? ` [${c.wp_id}]` : "";
          console.log(`    [${c.checkpoint_type}]${wpTag} ${c.topic}`);
          if (c.decisions) console.log(`      decisions: ${c.decisions}`);
        }
        console.log("");
      }
    }

    closeDb(db);
  } catch (err) {
    try { closeDb(db); } catch {}
    // Best-effort: print error but don't block
    console.log(`MEMORY_RECALL [${scope.label}]: recall error — ${err.message}`);
  }
} catch (err) {
  // DB open failed — skip silently
  console.log(`MEMORY_RECALL [${scope.label}]: database unavailable — ${err.message}`);
}
