#!/usr/bin/env node
/**
 * RGF-129: Cross-WP pattern synthesis → governance improvement candidates.
 *
 * Scans the governance memory DB for systemic patterns:
 * 1. TOPIC CLUSTERS — group procedural memories by topic similarity (exact prefix match)
 * 2. RECURRING SMOKE-FIND — same failure category across 3+ WPs
 * 3. REPEATED REPAIRS — same state transitions appearing across multiple WPs
 * 4. HIGH-ACCESS — memories accessed 5+ times are systemic patterns worth codifying
 * 5. CROSS-WP EPISODIC — receipt kinds that recur across many WPs (governance friction signals)
 *
 * Output: markdown report to stdout (pipe to file or review in-session).
 * All rule-based, no LLM.
 *
 * Usage:
 *   node memory-patterns.mjs [--min-wps 3] [--min-access 5]
 */

import {
  openGovernanceMemoryDb,
  closeDb,
} from "./governance-memory-lib.mjs";

function parseFlags(args) {
  const flags = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i].startsWith("--") && i + 1 < args.length) {
      flags[args[i].slice(2).replace(/-/g, "_")] = args[i + 1];
      i++;
    }
  }
  return flags;
}

const flags = parseFlags(process.argv.slice(2));
const MIN_WPS = Number(flags.min_wps) || 3;
const MIN_ACCESS = Number(flags.min_access) || 5;

const { db } = openGovernanceMemoryDb();

try {
  const now = new Date().toISOString();
  const sections = [];

  // -------------------------------------------------------------------------
  // 1. TOPIC CLUSTERS — procedural memories with the same topic prefix
  // -------------------------------------------------------------------------
  const procedurals = db.prepare(
    `SELECT id, topic, summary, wp_id, importance, access_count
     FROM memory_index WHERE memory_type = 'procedural' AND consolidated = 0
     ORDER BY topic`
  ).all();

  const topicGroups = new Map();
  for (const p of procedurals) {
    // Normalize: strip "Fix pattern: " prefix, take first meaningful segment
    const normalized = p.topic.replace(/^Fix pattern:\s*/i, "").split(/\s+on\s+|\s+by\s+/)[0].trim();
    if (!topicGroups.has(normalized)) topicGroups.set(normalized, []);
    topicGroups.get(normalized).push(p);
  }

  const clusters = [...topicGroups.entries()]
    .filter(([, items]) => items.length >= 2)
    .sort((a, b) => b[1].length - a[1].length);

  if (clusters.length > 0) {
    const lines = [`## 1. Procedural Memory Clusters (${clusters.length} clusters)\n`];
    lines.push("Recurring fix patterns that may warrant codification as governance rules or tooling.\n");
    for (const [topic, items] of clusters.slice(0, 15)) {
      const wps = [...new Set(items.map(i => i.wp_id).filter(Boolean))];
      lines.push(`### "${topic}" (${items.length} entries, ${wps.length} WPs)`);
      for (const item of items) {
        lines.push(`- #${item.id} [${item.wp_id || "global"}] ${item.summary.slice(0, 120)} (importance=${item.importance.toFixed(2)}, access=${item.access_count})`);
      }
      if (wps.length >= MIN_WPS) {
        lines.push(`\n**CANDIDATE:** This pattern spans ${wps.length} WPs — consider promoting to an RGF item or governance rule.\n`);
      }
      lines.push("");
    }
    sections.push(lines.join("\n"));
  }

  // -------------------------------------------------------------------------
  // 2. RECURRING SMOKE-FIND categories across WPs
  // -------------------------------------------------------------------------
  const smokeFindings = db.prepare(
    `SELECT mi.id, mi.topic, mi.summary, mi.wp_id, mi.importance, me.metadata
     FROM memory_index mi
     LEFT JOIN memory_entries me ON me.index_id = mi.id
     WHERE mi.consolidated = 0
       AND mi.topic LIKE 'SMOKE-FIND-%'
     ORDER BY mi.topic`
  ).all();

  const categoryGroups = new Map();
  for (const f of smokeFindings) {
    let meta = {};
    try { meta = JSON.parse(f.metadata || "{}"); } catch {}
    const category = meta.category || f.topic.split(":")[1]?.trim() || f.topic;
    if (!categoryGroups.has(category)) categoryGroups.set(category, []);
    categoryGroups.get(category).push(f);
  }

  const recurringCategories = [...categoryGroups.entries()]
    .filter(([, items]) => {
      const wps = new Set(items.map(i => i.wp_id).filter(Boolean));
      return wps.size >= MIN_WPS;
    })
    .sort((a, b) => b[1].length - a[1].length);

  if (recurringCategories.length > 0) {
    const lines = [`## 2. Recurring Smoketest Failure Categories (${recurringCategories.length} patterns)\n`];
    lines.push(`Failure categories appearing across ${MIN_WPS}+ WPs — systemic issues.\n`);
    for (const [category, items] of recurringCategories) {
      const wps = [...new Set(items.map(i => i.wp_id).filter(Boolean))];
      const maxSeverity = items.reduce((max, i) => {
        let meta = {};
        try { meta = JSON.parse(i.metadata || "{}"); } catch {}
        const sev = meta.severity || "";
        if (sev === "HIGH") return "HIGH";
        if (sev === "MEDIUM" && max !== "HIGH") return "MEDIUM";
        return max;
      }, "LOW");
      lines.push(`### ${category} (${items.length} findings, ${wps.length} WPs, max severity: ${maxSeverity})`);
      lines.push(`WPs: ${wps.join(", ")}`);
      for (const item of items.slice(0, 5)) {
        lines.push(`- #${item.id} ${item.summary.slice(0, 120)}`);
      }
      if (items.length > 5) lines.push(`- ... and ${items.length - 5} more`);
      lines.push(`\n**CANDIDATE:** Systemic failure pattern across ${wps.length} WPs. Consider an RGF item to address root cause.\n`);
      lines.push("");
    }
    sections.push(lines.join("\n"));
  }

  // -------------------------------------------------------------------------
  // 3. REPEATED REPAIR state transitions
  // -------------------------------------------------------------------------
  const repairs = db.prepare(
    `SELECT mi.id, mi.topic, mi.summary, mi.wp_id, me.content
     FROM memory_index mi
     LEFT JOIN memory_entries me ON me.index_id = mi.id
     WHERE mi.memory_type = 'procedural' AND mi.consolidated = 0
       AND mi.topic LIKE 'Fix pattern:%'
     ORDER BY mi.topic`
  ).all();

  const transitionGroups = new Map();
  for (const r of repairs) {
    // Extract state transition from summary: "STATE_A → STATE_B: description"
    const match = (r.summary || "").match(/^(\S+)\s*→\s*(\S+)/);
    if (match) {
      const transition = `${match[1]} → ${match[2]}`;
      if (!transitionGroups.has(transition)) transitionGroups.set(transition, []);
      transitionGroups.get(transition).push(r);
    }
  }

  const repeatedTransitions = [...transitionGroups.entries()]
    .filter(([, items]) => items.length >= 2)
    .sort((a, b) => b[1].length - a[1].length);

  if (repeatedTransitions.length > 0) {
    const lines = [`## 3. Repeated REPAIR State Transitions (${repeatedTransitions.length} patterns)\n`];
    lines.push("Same state transitions happening across WPs — friction points in the workflow.\n");
    for (const [transition, items] of repeatedTransitions) {
      const wps = [...new Set(items.map(i => i.wp_id).filter(Boolean))];
      lines.push(`### ${transition} (${items.length} occurrences, ${wps.length} WPs)`);
      for (const item of items.slice(0, 5)) {
        lines.push(`- #${item.id} [${item.wp_id || "global"}] ${item.summary.slice(0, 120)}`);
      }
      if (wps.length >= MIN_WPS) {
        lines.push(`\n**CANDIDATE:** This transition repeats across ${wps.length} WPs — workflow friction that may need tooling.\n`);
      }
      lines.push("");
    }
    sections.push(lines.join("\n"));
  }

  // -------------------------------------------------------------------------
  // 4. HIGH-ACCESS memories (systemic patterns worth codifying)
  // -------------------------------------------------------------------------
  const highAccess = db.prepare(
    `SELECT id, memory_type, topic, summary, wp_id, importance, access_count
     FROM memory_index
     WHERE consolidated = 0 AND access_count >= ?
     ORDER BY access_count DESC LIMIT 20`
  ).all(MIN_ACCESS);

  if (highAccess.length > 0) {
    const lines = [`## 4. High-Access Memories (${highAccess.length} entries, access >= ${MIN_ACCESS})\n`];
    lines.push("Memories loaded into sessions repeatedly — systemic knowledge worth formalizing.\n");
    for (const m of highAccess) {
      lines.push(`- #${m.id} [${m.memory_type}] ${m.topic}: ${m.summary.slice(0, 120)} (access=${m.access_count}, importance=${m.importance.toFixed(2)}, wp=${m.wp_id || "global"})`);
    }
    lines.push(`\n**ACTION:** Review these for promotion to governance rules, protocol additions, or tooling improvements.\n`);
    sections.push(lines.join("\n"));
  }

  // -------------------------------------------------------------------------
  // 5. CROSS-WP episodic density — receipt kinds that appear across many WPs
  // -------------------------------------------------------------------------
  const wpReceiptDensity = db.prepare(
    `SELECT topic, COUNT(*) as cnt, COUNT(DISTINCT wp_id) as wp_cnt
     FROM memory_index
     WHERE memory_type = 'episodic' AND consolidated = 0 AND wp_id != ''
     GROUP BY SUBSTR(topic, 1, INSTR(topic || ' ', ' ') - 1)
     HAVING wp_cnt >= ?
     ORDER BY wp_cnt DESC, cnt DESC`
  ).all(MIN_WPS);

  if (wpReceiptDensity.length > 0) {
    const lines = [`## 5. Cross-WP Episodic Density (${wpReceiptDensity.length} receipt kinds)\n`];
    lines.push(`Receipt kinds appearing across ${MIN_WPS}+ WPs — governance interaction patterns.\n`);
    for (const r of wpReceiptDensity) {
      const kind = r.topic.split(/\s/)[0];
      lines.push(`- **${kind}**: ${r.cnt} events across ${r.wp_cnt} WPs`);
    }
    lines.push("");
    sections.push(lines.join("\n"));
  }

  // -------------------------------------------------------------------------
  // Report assembly
  // -------------------------------------------------------------------------
  const stats = db.prepare("SELECT COUNT(*) as total FROM memory_index WHERE consolidated = 0").get();

  console.log(`# Governance Improvement Candidates`);
  console.log(`\n- Generated: ${now}`);
  console.log(`- Active memories scanned: ${stats.total}`);
  console.log(`- Thresholds: min_wps=${MIN_WPS}, min_access=${MIN_ACCESS}`);
  console.log("");

  if (sections.length === 0) {
    console.log("No systemic patterns detected at current thresholds. Lower `--min-wps` or `--min-access` to broaden the scan.");
  } else {
    console.log(sections.join("\n---\n\n"));
  }

  console.log("---\n");
  console.log("*Generated by `just memory-patterns` (RGF-129). Review candidates and promote to RGF items where warranted.*");

} finally {
  closeDb(db);
}
