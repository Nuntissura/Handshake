#!/usr/bin/env node
/**
 * Phase gate check: ensure no Ready-for-Dev items remain before phase progression and validator scans are clean.
 */
import { readFileSync } from "node:fs";

const phase = process.argv[2] || "Phase-1";
const taskBoardPath = ".GOV/roles_shared/TASK_BOARD.md";

function fail(msg) {
  console.error(`validator-phase-gate: FAIL - ${msg}`);
  process.exit(1);
}

function extractSectionLines(board, headingText) {
  const lines = board.split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${headingText}\\s*$`, "i");

  const startIndex = lines.findIndex((line) => headingRe.test(line.trimEnd()));
  if (startIndex === -1) return null;

  const section = [];
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.startsWith("## ")) break;
    section.push(line);
  }

  return section;
}

function countWpEntries(sectionLines) {
  const wpEntryRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*/;
  const ids = new Set();
  for (const line of sectionLines) {
    const match = line.match(wpEntryRe);
    if (match) ids.add(match[1]);
  }
  return ids.size;
}

function main() {
  let board;
  try {
    board = readFileSync(taskBoardPath, "utf8");
  } catch (err) {
    fail(`cannot read ${taskBoardPath}: ${err.message}`);
  }

  const readyForDevLines = extractSectionLines(board, "Ready for Dev");
  if (!readyForDevLines) {
    fail(`missing "## Ready for Dev" section in ${taskBoardPath}`);
  }

  const readyCount = countWpEntries(readyForDevLines);
  if (readyCount > 0) {
    fail(
      `Task Board still has ${readyCount} Ready for Dev item(s); phase progression for ${phase} is blocked.`
    );
  }

  console.log(`validator-phase-gate: PASS - no Ready for Dev items detected for ${phase}.`);
}

main();

