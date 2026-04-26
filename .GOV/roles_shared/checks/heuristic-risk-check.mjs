#!/usr/bin/env node

import { listDeclaredWpMicrotasks } from "../scripts/lib/wp-microtask-lib.mjs";
import { summarizeHeuristicRiskContract } from "../scripts/lib/heuristic-risk-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("heuristic-risk-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("heuristic-risk-check.mjs", message, { role: "SHARED", details });
}

const args = process.argv.slice(2);
const wpId = String(args.find((arg) => /^WP-/.test(arg)) || "").trim();
const json = args.includes("--json");

if (!wpId) {
  fail("Usage: node heuristic-risk-check.mjs WP-{ID} [--json]");
}

const microtasks = listDeclaredWpMicrotasks(wpId);
const rows = microtasks.map((mt) => ({
  mt_id: mt.mtId,
  clause: mt.clause,
  packet_path: mt.packetPath,
  heuristic_risk: mt.heuristicRisk?.heuristic_risk || "NO",
  heuristic_risk_class: mt.heuristicRisk?.heuristic_risk_class || "NONE",
  required_evidence: mt.heuristicRisk?.required_evidence || [],
  strategy_escalation: mt.heuristicRisk?.strategy_escalation || "NONE",
  repair_cycle_strategy_threshold: mt.heuristicRisk?.repair_cycle_strategy_threshold || 0,
  reasons: mt.heuristicRisk?.reasons || [],
}));

if (json) {
  console.log(JSON.stringify({
    schema_id: "hsk.heuristic_risk_check@1",
    wp_id: wpId,
    microtask_count: rows.length,
    heuristic_risk_count: rows.filter((row) => row.heuristic_risk === "YES").length,
    microtasks: rows,
  }, null, 2));
} else if (rows.length === 0) {
  console.log(`heuristic-risk-check ok: ${wpId} has no declared microtasks`);
} else {
  console.log(`heuristic-risk-check ok: ${wpId}`);
  for (const row of rows) {
    console.log(`- ${row.mt_id}: ${summarizeHeuristicRiskContract(row)}`);
  }
}
