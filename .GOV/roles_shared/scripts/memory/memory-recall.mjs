#!/usr/bin/env node
/**
 * memory-recall - Action-scoped memory retrieval.
 *
 * Surfaces relevant memory content before a role action, so the model sees
 * prior knowledge (tooling pitfalls, decisions, role habits, trigger-specific
 * failures) instead of starting blind.
 *
 * Usage:
 *   node memory-recall.mjs <ACTION> [--wp WP-{ID}] [--budget N] [--role ROLE] [--trigger CMD] [--script SCRIPT]
 *
 * Actions (mapped to query scopes):
 *   REFINEMENT       - tooling pitfalls, prior failed refinement approaches
 *   RESUME           - last session state, unresolved blockers, key decisions
 *   STEERING         - prior steering failures, coder behavior patterns
 *   RELAY            - relay communication issues, prior relay outcomes
 *   DELEGATION       - packet creation issues, dependency pitfalls
 *   PACKET_CREATE    - prior packet failures, template issues
 *   COMMAND          - shell command family habits, trigger-specific failures
 *
 * Output: structured block printed to stdout for model consumption.
 * Exit 0 always (best-effort; never blocks the calling command).
 */

import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  openGovernanceMemoryDb,
  closeDb,
  searchMemories,
  getPointerIndex,
  getMemoryEntry,
} from "./governance-memory-lib.mjs";

// ---------------------------------------------------------------------------
// Action -> query scope definitions
// ---------------------------------------------------------------------------

export const ACTION_SCOPES = {
  REFINEMENT: {
    label: "Refinement",
    description: "Prior refinement knowledge - tooling pitfalls, failed approaches, workarounds",
    queries: [
      { type: "procedural", keywords: "patch refinement edit scaffold tool payload path" },
      { type: "procedural", keywords: "Windows path limit file size" },
      { type: "episodic", keywords: "refinement failed retry workaround" },
      { type: "semantic", keywords: "refinement scope discovery primitives" },
    ],
    topN: { memoryType: "procedural", limit: 10 },
  },
  RESUME: {
    label: "Resume",
    description: "Prior session state - last actions, unresolved blockers, key decisions",
    queries: [
      { type: "episodic", keywords: "session close handoff blocker" },
      { type: "semantic", keywords: "decision architecture approach" },
      { type: "procedural", keywords: "fix repair workaround" },
    ],
    topN: { memoryType: "episodic", limit: 10 },
    includeConversationLog: true,
  },
  STEERING: {
    label: "Steering",
    description: "Prior steering knowledge - what worked, what failed, coder patterns",
    queries: [
      { type: "procedural", keywords: "steering steer coder session" },
      { type: "episodic", keywords: "steering handoff coder validator" },
      { type: "semantic", keywords: "coder behavior pattern" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
  RELAY: {
    label: "Relay",
    description: "Prior relay knowledge - communication patterns, dispatch issues",
    queries: [
      { type: "procedural", keywords: "relay dispatch communication" },
      { type: "episodic", keywords: "relay manual dispatch escalation" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
  DELEGATION: {
    label: "Delegation",
    description: "Prior delegation knowledge - packet issues, worktree setup, dependency pitfalls",
    queries: [
      { type: "procedural", keywords: "delegation packet worktree dependency" },
      { type: "episodic", keywords: "delegation launch coder worktree" },
      { type: "semantic", keywords: "dependency blocker build order" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
  CODER_RESUME: {
    label: "Coder resume",
    description: "Prior coder session knowledge - implementation pitfalls, tool failures, fix patterns",
    queries: [
      { type: "procedural", keywords: "compile build import path error tool" },
      { type: "procedural", keywords: "edit patch file size payload limit" },
      { type: "procedural", keywords: "test runner cargo npm fixture" },
      { type: "episodic", keywords: "coder handoff implementation blocker" },
    ],
    topN: { memoryType: "procedural", limit: 10 },
    includeConversationLog: true,
  },
  VALIDATOR_RESUME: {
    label: "Validator resume",
    description: "Prior validator session knowledge - validation pitfalls, check failures, spec drift",
    queries: [
      { type: "procedural", keywords: "validation check false positive spec anchor" },
      { type: "procedural", keywords: "smoketest parser review regression" },
      { type: "semantic", keywords: "spec drift governance alignment" },
      { type: "episodic", keywords: "validator review verdict blocker" },
    ],
    topN: { memoryType: "procedural", limit: 10 },
    includeConversationLog: true,
  },
  PACKET_CREATE: {
    label: "Packet creation",
    description: "Prior packet creation knowledge - template issues, field errors",
    queries: [
      { type: "procedural", keywords: "packet template create task" },
      { type: "episodic", keywords: "packet creation failed" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
  COMMAND: {
    label: "Command",
    description: "Shell command family habits - trigger-specific failures, execution workarounds, safe usage patterns",
    queries: [
      { type: "procedural", keywords: "shell command terminal powershell bash workaround exit code" },
      { type: "procedural", keywords: "script failure retry invocation quoting path" },
      { type: "episodic", keywords: "command failed workaround" },
    ],
    topN: { memoryType: "procedural", limit: 8 },
  },
};

export const VALID_ACTIONS = Object.keys(ACTION_SCOPES);

const ACTION_HINTS = {
  REFINEMENT: {
    roleCandidates: ["ORCHESTRATOR"],
    triggerRefs: ["begin-refinement"],
    scriptCandidates: [],
  },
  STEERING: {
    roleCandidates: ["ORCHESTRATOR"],
    triggerRefs: ["orchestrator-steer-next"],
    scriptCandidates: ["orchestrator-steer-next.mjs"],
  },
  RELAY: {
    roleCandidates: ["ORCHESTRATOR"],
    triggerRefs: ["manual-relay-next", "manual-relay-dispatch"],
    scriptCandidates: ["manual-relay-next.mjs", "manual-relay-dispatch.mjs"],
  },
  DELEGATION: {
    roleCandidates: ["ORCHESTRATOR", "ACTIVATION_MANAGER"],
    triggerRefs: ["orchestrator-prepare-and-packet", "activation-prepare-and-packet"],
    scriptCandidates: ["orchestrator-prepare-and-packet.mjs"],
  },
  CODER_RESUME: {
    roleCandidates: ["CODER"],
    triggerRefs: ["coder-next"],
    scriptCandidates: ["coder-next.mjs"],
  },
  VALIDATOR_RESUME: {
    roleCandidates: ["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR"],
    triggerRefs: ["validator-next"],
    scriptCandidates: ["validator-next.mjs", "integration-validator-closeout-sync.mjs"],
  },
  PACKET_CREATE: {
    roleCandidates: ["ORCHESTRATOR", "ACTIVATION_MANAGER"],
    triggerRefs: ["create-task-packet", "activation-create-task-packet"],
    scriptCandidates: ["create-task-packet.mjs"],
  },
  COMMAND: {
    roleCandidates: [],
    triggerRefs: [],
    scriptCandidates: [],
  },
};

const ROLE_HABIT_SOURCES = new Set([
  "memory-capture",
  "memory-intent-snapshot",
  "fail-capture",
  "shell-command",
  "RECEIPTS.jsonl",
]);
const OPERATOR_SOURCES = new Set(["operator-reported", "memory-capture"]);
const TRIGGER_SENSITIVE_SOURCES = new Set(["fail-capture", "memory-capture", "memory-intent-snapshot", "shell-command"]);

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

export function parseFlags(args) {
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

function splitCsvFlag(value) {
  return String(value || "")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function uniqueStrings(values) {
  return [...new Set(values.map((value) => String(value || "").trim()).filter(Boolean))];
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeScriptName(value) {
  const trimmed = String(value || "").trim();
  if (!trimmed) return "";
  return trimmed.endsWith(".mjs") ? trimmed : `${trimmed}.mjs`;
}

function buildActionHints(action, wpId = "") {
  if (action === "RESUME" && wpId) {
    return {
      roleCandidates: ["ORCHESTRATOR"],
      triggerRefs: ["orchestrator-next"],
      scriptCandidates: ["orchestrator-next.mjs"],
    };
  }
  return ACTION_HINTS[action] || { roleCandidates: [], triggerRefs: [], scriptCandidates: [] };
}

export function resolveRecallContext(action, { wpId = "", role = "", trigger = "", script = "" } = {}) {
  const normalizedAction = String(action || "").trim().toUpperCase();
  const hints = buildActionHints(normalizedAction, wpId);
  const roleCandidates = uniqueStrings([
    ...splitCsvFlag(role).map(normalizeRole),
    ...hints.roleCandidates.map(normalizeRole),
  ]);
  const triggerRefs = uniqueStrings([
    ...splitCsvFlag(trigger),
    ...hints.triggerRefs,
  ]);
  const scriptCandidates = uniqueStrings([
    ...splitCsvFlag(script).map(normalizeScriptName),
    ...hints.scriptCandidates.map(normalizeScriptName),
    ...triggerRefs.map(normalizeScriptName),
  ]);

  return {
    action: normalizedAction,
    wpId,
    roleCandidates,
    triggerRefs,
    scriptCandidates,
    primaryRole: roleCandidates[0] || "",
    primaryTrigger: triggerRefs[0] || "",
  };
}

function safeJsonParse(value) {
  try {
    return JSON.parse(String(value || "{}"));
  } catch {
    return {};
  }
}

function loadActiveMemoryRows(db, { wpId = "", limit = 250 } = {}) {
  let sql = `
    SELECT
      mi.id,
      mi.memory_type,
      mi.topic,
      mi.summary,
      mi.wp_id,
      mi.file_scope,
      mi.importance,
      mi.access_count,
      mi.created_at,
      me.content,
      me.source_artifact,
      me.source_role,
      me.source_session,
      me.metadata
    FROM memory_index mi
    LEFT JOIN memory_entries me ON me.index_id = mi.id
    WHERE mi.consolidated = 0
  `;
  const params = [];
  if (wpId) {
    sql += " AND (mi.wp_id = ? OR mi.wp_id = '')";
    params.push(wpId);
  }
  sql += " ORDER BY mi.importance DESC, mi.created_at DESC LIMIT ?";
  params.push(limit);

  return db.prepare(sql).all(...params).map((row) => ({
    ...row,
    _metadata: safeJsonParse(row.metadata),
  }));
}

function loadTriggerConversationRows(db, context, { wpId = "", limit = 6 } = {}) {
  if (context.triggerRefs.length === 0) return [];
  const lowerTriggers = context.triggerRefs.map((entry) => entry.toLowerCase());
  const placeholders = lowerTriggers.map(() => "?").join(", ");
  let sql = `
    SELECT checkpoint_type, topic, content, decisions, wp_id, timestamp_utc, trigger_ref, role
    FROM conversation_log
    WHERE LOWER(trigger_ref) IN (${placeholders})
  `;
  const params = [...lowerTriggers];
  if (wpId) {
    sql += " AND (wp_id = ? OR wp_id = '')";
    params.push(wpId);
  }
  sql += " ORDER BY timestamp_utc DESC LIMIT ?";
  params.push(limit);
  return db.prepare(sql).all(...params);
}

export function entryMatchesTriggerContext(entry, context) {
  if (!entry || (context.triggerRefs.length === 0 && context.scriptCandidates.length === 0)) return false;
  const metadata = entry._metadata || safeJsonParse(entry.metadata);
  const searchable = [
    entry.topic,
    entry.summary,
    entry.content,
    entry.source_artifact,
    entry.file_scope,
    metadata.script,
    metadata.trigger_script,
    metadata.trigger,
    metadata.command_family,
    metadata.raw_command,
  ]
    .filter(Boolean)
    .join("\n")
    .toLowerCase();

  const sourceArtifact = String(entry.source_artifact || "").toLowerCase();
  const metadataScript = String(metadata.script || "").toLowerCase();
  const metadataTrigger = String(metadata.trigger_script || "").toLowerCase();
  const metadataCommandFamily = String(metadata.command_family || "").toLowerCase();
  const metadataTriggerRef = String(metadata.trigger || "").toLowerCase();

  for (const scriptName of context.scriptCandidates.map((entry) => entry.toLowerCase())) {
    if (!scriptName) continue;
    if (sourceArtifact === scriptName || metadataScript === scriptName || metadataTrigger === scriptName) {
      return true;
    }
    if (searchable.includes(scriptName)) return true;
  }

  for (const triggerRef of context.triggerRefs.map((entry) => entry.toLowerCase())) {
    if (!triggerRef) continue;
    if (metadataCommandFamily === triggerRef || metadataTriggerRef === triggerRef) {
      return true;
    }
  }

  for (const triggerRef of context.triggerRefs.map((entry) => entry.toLowerCase())) {
    if (!triggerRef) continue;
    if (sourceArtifact === triggerRef || metadataTrigger === triggerRef) return true;
    if (searchable.includes(triggerRef)) return true;
  }

  return false;
}

export function entryMatchesRoleContext(entry, context) {
  const sourceRole = normalizeRole(entry?.source_role || "");
  if (!sourceRole || !context.roleCandidates.includes(sourceRole)) return false;
  const metadata = entry._metadata || safeJsonParse(entry?.metadata);
  if (ROLE_HABIT_SOURCES.has(String(entry?.source_artifact || ""))) {
    return String(entry?.source_artifact || "") !== "RECEIPTS.jsonl" || String(entry?.memory_type || "") === "procedural";
  }
  if (metadata.captured_mid_session || metadata.intent_based) return true;
  return false;
}

export function scoreMemoryForRecall(entry, context) {
  const sourceArtifact = String(entry?.source_artifact || "");
  const sourceRole = normalizeRole(entry?.source_role || "");
  let score = Number(entry?.importance || 0);

  if (OPERATOR_SOURCES.has(sourceArtifact)) score += 0.5;
  if (sourceArtifact === "fail-capture") score += 0.4;
  if (sourceArtifact === "memory-intent-snapshot") score += 0.25;
  if (context.roleCandidates.includes(sourceRole)) score += 0.35;
  if (entryMatchesTriggerContext(entry, context)) score += 0.75;
  if (context.wpId && entry?.wp_id === context.wpId) score += 0.2;
  if (entry?.access_count) score += Math.min(0.25, Number(entry.access_count) * 0.02);

  return score;
}

function sortEntriesForRecall(entries, context) {
  return entries
    .map((entry) => ({
      ...entry,
      _trigger_match: entryMatchesTriggerContext(entry, context),
      _role_match: context.roleCandidates.includes(normalizeRole(entry.source_role || "")),
      _recall_score: scoreMemoryForRecall(entry, context),
    }))
    .sort((left, right) => {
      if (right._recall_score !== left._recall_score) return right._recall_score - left._recall_score;
      return String(right.created_at || "").localeCompare(String(left.created_at || ""));
    });
}

function estimateMemoryTokens(entry) {
  return Math.ceil(
    (
      String(entry?.topic || "").length
      + String(entry?.summary || "").length
      + String(entry?.content || "").length
      + String(entry?.file_scope || "").length
    ) / 4,
  );
}

function budgetEntries(entries, tokenState, maxCount, selectedIds) {
  const accepted = [];
  for (const entry of entries) {
    if (accepted.length >= maxCount) break;
    if (selectedIds.has(entry.id)) continue;
    const entryTokens = estimateMemoryTokens(entry);
    if (tokenState.used + entryTokens > tokenState.limit) continue;
    tokenState.used += entryTokens;
    selectedIds.add(entry.id);
    accepted.push(entry);
  }
  return accepted;
}

function formatAuditTopic(entry, maxLength = 60) {
  const topic = String(entry?.topic || "").replace(/\s+/g, " ").trim();
  if (!topic) return "(untitled)";
  return topic.length > maxLength ? `${topic.slice(0, maxLength - 3)}...` : topic;
}

export function buildRecallAuditLine({
  triggerEntries = [],
  roleEntries = [],
  generalEntries = [],
  triggerConversationEntries = [],
  conversationEntries = [],
} = {}) {
  const memoryEntries = [...triggerEntries, ...roleEntries, ...generalEntries];
  const topEntries = memoryEntries
    .slice(0, 3)
    .map((entry) => `#${entry.id} ${formatAuditTopic(entry)}`);

  return `MEMORY_INJECTION_APPLIED: memory_entries=${memoryEntries.length} trigger_context=${triggerConversationEntries.length} prior_session=${conversationEntries.length} top=${topEntries.length > 0 ? topEntries.join(" | ") : "none"}`;
}

function printMemoryEntry(entry) {
  const content = entry.content || entry.summary || "";
  const typeTag = String(entry.memory_type || "").toUpperCase().slice(0, 4) || "MEM";
  const wpTag = entry.wp_id ? ` [${entry.wp_id}]` : "";
  const roleTag = entry.source_role ? ` {${entry.source_role}}` : "";
  console.log(`  [${typeTag}]${wpTag}${roleTag} ${entry.topic}`);
  console.log(`    ${content}`);
  if (entry.file_scope) console.log(`    files: ${entry.file_scope}`);
  if (entry.source_artifact) console.log(`    source: ${entry.source_artifact}`);
  console.log("");
}

function printConversationEntry(entry) {
  const wpTag = entry.wp_id ? ` [${entry.wp_id}]` : "";
  const triggerTag = entry.trigger_ref ? ` trigger=${entry.trigger_ref}` : "";
  console.log(`    [${entry.checkpoint_type}]${wpTag} ${entry.role || "?"} ${entry.topic}${triggerTag}`);
  if (entry.decisions) console.log(`      decisions: ${entry.decisions}`);
}

export function runRecall(action, flags = {}) {
  const normalizedAction = String(action || "").trim().toUpperCase();
  if (!normalizedAction || !VALID_ACTIONS.includes(normalizedAction)) {
    console.error(`Usage: memory-recall.mjs <${VALID_ACTIONS.join("|")}> [--wp WP-{ID}] [--budget N] [--role ROLE] [--trigger CMD] [--script SCRIPT]`);
    return 0;
  }

  const wpId = flags.wp || "";
  const tokenBudget = Number(flags.budget) || 1500;
  const scope = ACTION_SCOPES[normalizedAction];
  const context = resolveRecallContext(normalizedAction, {
    wpId,
    role: flags.role || "",
    trigger: flags.trigger || "",
    script: flags.script || "",
  });

  try {
    const { db } = openGovernanceMemoryDb();

    try {
      const seen = new Set();
      const collected = [];

      for (const query of scope.queries) {
        const results = searchMemories(db, query.keywords, {
          memoryType: query.type,
          wpId,
          limit: 10,
        });
        for (const result of results) {
          if (!seen.has(result.id) && result.consolidated === 0) {
            seen.add(result.id);
            collected.push(result);
          }
        }
      }

      if (scope.topN) {
        const topResults = getPointerIndex(db, {
          memoryType: scope.topN.memoryType,
          wpId,
          limit: scope.topN.limit,
        });
        for (const result of topResults) {
          if (!seen.has(result.id)) {
            seen.add(result.id);
            const entry = getMemoryEntry(db, result.id);
            collected.push({
              ...result,
              content: entry?.content || "",
              source_artifact: entry?.source_artifact || "",
              source_role: entry?.source_role || "",
              metadata: entry?.metadata || "{}",
              _metadata: safeJsonParse(entry?.metadata || "{}"),
            });
          }
        }
      }

      if (wpId) {
        const wpResults = getPointerIndex(db, { wpId, limit: 10 });
        for (const result of wpResults) {
          if (!seen.has(result.id)) {
            seen.add(result.id);
            const entry = getMemoryEntry(db, result.id);
            collected.push({
              ...result,
              content: entry?.content || "",
              source_artifact: entry?.source_artifact || "",
              source_role: entry?.source_role || "",
              metadata: entry?.metadata || "{}",
              _metadata: safeJsonParse(entry?.metadata || "{}"),
            });
          }
        }
      }

      let conversationEntries = [];
      if (scope.includeConversationLog) {
        try {
          let sql = `
            SELECT checkpoint_type, topic, content, decisions, wp_id, timestamp_utc, role, trigger_ref
            FROM conversation_log
            ORDER BY timestamp_utc DESC
            LIMIT 8
          `;
          if (wpId) {
            sql = `
              SELECT checkpoint_type, topic, content, decisions, wp_id, timestamp_utc, role, trigger_ref
              FROM conversation_log
              WHERE wp_id = ? OR wp_id = ''
              ORDER BY timestamp_utc DESC
              LIMIT 8
            `;
          }
          conversationEntries = wpId
            ? db.prepare(sql).all(wpId)
            : db.prepare(sql).all();
        } catch {
          conversationEntries = [];
        }
      }

      const triggerConversationEntries = loadTriggerConversationRows(db, context, { wpId, limit: 6 });
      const activeEntries = loadActiveMemoryRows(db, { wpId, limit: 250 });

      const triggerEntries = sortEntriesForRecall(
        activeEntries.filter((entry) => TRIGGER_SENSITIVE_SOURCES.has(String(entry.source_artifact || "")) && entryMatchesTriggerContext(entry, context)),
        context,
      );
      const roleEntries = sortEntriesForRecall(
        activeEntries.filter((entry) => entryMatchesRoleContext(entry, context)),
        context,
      );
      const generalEntries = sortEntriesForRecall(collected, context);

      const memoryTokenState = { used: 0, limit: Math.max(400, tokenBudget - 250) };
      const selectedIds = new Set();
      const budgetedTriggerEntries = budgetEntries(triggerEntries, memoryTokenState, 4, selectedIds);
      const budgetedRoleEntries = budgetEntries(roleEntries, memoryTokenState, 4, selectedIds);
      const budgetedGeneralEntries = budgetEntries(generalEntries, memoryTokenState, 12, selectedIds);
      const auditLine = buildRecallAuditLine({
        triggerEntries: budgetedTriggerEntries,
        roleEntries: budgetedRoleEntries,
        generalEntries: budgetedGeneralEntries,
        triggerConversationEntries,
        conversationEntries,
      });

      if (
        budgetedTriggerEntries.length === 0
        && budgetedRoleEntries.length === 0
        && budgetedGeneralEntries.length === 0
        && triggerConversationEntries.length === 0
        && conversationEntries.length === 0
      ) {
        console.log(`MEMORY_RECALL [${scope.label}]`);
        console.log(`  scope: ${scope.description}`);
        if (wpId) console.log(`  wp: ${wpId}`);
        if (context.primaryRole) console.log(`  role_hint: ${context.primaryRole}`);
        if (context.primaryTrigger) console.log(`  trigger_hint: ${context.primaryTrigger}`);
        console.log(`  ${auditLine}`);
        console.log("");
        console.log("  no relevant memories found.");
        return 0;
      }

      console.log(`MEMORY_RECALL [${scope.label}]`);
      console.log(`  scope: ${scope.description}`);
      console.log(`  entries: ${budgetedTriggerEntries.length + budgetedRoleEntries.length + budgetedGeneralEntries.length} (budget: ~${tokenBudget} tokens)`);
      if (wpId) console.log(`  wp: ${wpId}`);
      if (context.primaryRole) console.log(`  role_hint: ${context.primaryRole}`);
      if (context.primaryTrigger) console.log(`  trigger_hint: ${context.primaryTrigger}`);
      console.log(`  ${auditLine}`);
      console.log("");

      if (budgetedTriggerEntries.length > 0) {
        console.log("  TRIGGER PITFALLS:");
        for (const entry of budgetedTriggerEntries) printMemoryEntry(entry);
      }

      if (budgetedRoleEntries.length > 0) {
        console.log("  ROLE HABITS:");
        for (const entry of budgetedRoleEntries) printMemoryEntry(entry);
      }

      if (budgetedGeneralEntries.length > 0) {
        const operatorEntries = budgetedGeneralEntries.filter((entry) => OPERATOR_SOURCES.has(entry.source_artifact));
        const systemEntries = budgetedGeneralEntries.filter((entry) => !OPERATOR_SOURCES.has(entry.source_artifact));

        if (operatorEntries.length > 0) {
          console.log("  OPERATOR WARNINGS:");
          for (const entry of operatorEntries) printMemoryEntry(entry);
        }

        if (systemEntries.length > 0) {
          console.log("  GENERAL FINDINGS:");
          for (const entry of systemEntries) printMemoryEntry(entry);
        }
      }

      if (triggerConversationEntries.length > 0) {
        console.log("  TRIGGER CONTEXT:");
        for (const entry of triggerConversationEntries) printConversationEntry(entry);
        console.log("");
      }

      if (conversationEntries.length > 0) {
        console.log("  PRIOR_SESSION_LOG:");
        for (const entry of conversationEntries) printConversationEntry(entry);
        console.log("");
      }

      return 0;
    } catch (error) {
      console.log(`MEMORY_RECALL [${scope.label}]: recall error - ${error.message}`);
      return 0;
    } finally {
      try {
        closeDb(db);
      } catch {
        // best effort
      }
    }
  } catch (error) {
    console.log(`MEMORY_RECALL [${scope.label}]: database unavailable - ${error.message}`);
    return 0;
  }
}

function main(argv = process.argv.slice(2)) {
  const [action, ...rawArgs] = argv;
  const { flags } = parseFlags(rawArgs);
  return runRecall(action, flags);
}

const invokedAsScript = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedAsScript) {
  process.exit(main());
}
