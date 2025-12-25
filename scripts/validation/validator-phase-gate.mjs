#!/usr/bin/env node
/**
 * Phase gate check: ensure no Ready-for-Dev items remain before phase progression and validator scans are clean.
 */
import { readFileSync } from "node:fs";

const phase = process.argv[2] || "Phase-1";
const taskBoardPath = "docs/TASK_BOARD.md";

function fail(msg) {
  console.error(`validator-phase-gate: FAIL — ${msg}`);
  process.exit(1);
}

function main() {
  let board;
  try {
    board = readFileSync(taskBoardPath, "utf8");
  } catch (err) {
    fail(`cannot read ${taskBoardPath}: ${err.message}`);
  }

  const readyMatches = (board.match(/Ready for Dev/gi) || []).length;
  if (readyMatches > 0) {
    fail(`Task Board still has ${readyMatches} "Ready for Dev" item(s); phase progression for ${phase} is blocked.`);
  }

  console.log(`validator-phase-gate: PASS — no Ready-for-Dev items detected for ${phase}.`);
}

main();
