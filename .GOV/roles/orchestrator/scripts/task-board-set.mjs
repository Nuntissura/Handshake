#!/usr/bin/env node
/**
 * Deterministic TASK_BOARD updater.
 *
 * Goal: remove manual markdown editing friction + format mistakes.
 * Scope: only moves a single WP_ID entry between sections.
 */

import fs from "node:fs";

const TASK_BOARD_PATH = ".GOV/roles_shared/TASK_BOARD.md";

function fail(message, details = []) {
  console.error(`[TASK_BOARD_SET] ${message}`);
  for (const line of details) console.error(`- ${line}`);
  process.exit(1);
}

function readText(p) {
  try {
    return fs.readFileSync(p, "utf8");
  } catch (e) {
    fail(`Failed to read: ${p}`, [String(e?.message || e)]);
  }
}

function writeText(p, text) {
  try {
    fs.writeFileSync(p, text, "utf8");
  } catch (e) {
    fail(`Failed to write: ${p}`, [String(e?.message || e)]);
  }
}

function detectEol(text) {
  return text.includes("\r\n") ? "\r\n" : "\n";
}

function escapeRegExp(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function findSection(lines, headingRe) {
  const startIdx = lines.findIndex((l) => headingRe.test(l));
  if (startIdx === -1) return null;
  const endIdxRel = lines.slice(startIdx + 1).findIndex((l) => /^##\s+/.test(l));
  const endIdx = endIdxRel === -1 ? lines.length : startIdx + 1 + endIdxRel;
  return { startIdx, endIdx };
}

function buildLine(wpId, status, reason) {
  const base = `- **[${wpId}]**`;
  switch (status) {
    case "READY_FOR_DEV":
      return `${base} - [READY_FOR_DEV]`;
    case "STUB":
      return `${base} - [STUB]`;
    case "IN_PROGRESS":
      return `${base} - [IN_PROGRESS]`;
    case "DONE_VALIDATED":
      return `${base} - [VALIDATED]`;
    case "DONE_FAIL":
      return `${base} - [FAIL]`;
    case "DONE_OUTDATED_ONLY":
      return `${base} - [OUTDATED_ONLY]`;
    case "BLOCKED":
      return reason ? `${base} - [BLOCKED] - ${reason}` : `${base} - [BLOCKED]`;
    case "SUPERSEDED":
      return `${base} - [SUPERSEDED]`;
    default:
      fail("Unknown status", [
        `got=${status}`,
        "allowed: READY_FOR_DEV|STUB|IN_PROGRESS|DONE_VALIDATED|DONE_FAIL|DONE_OUTDATED_ONLY|BLOCKED|SUPERSEDED",
      ]);
  }
}

function sectionForStatus(status) {
  switch (status) {
    case "READY_FOR_DEV":
      return /^##\s+Ready for Dev\s*$/;
    case "STUB":
      return /^##\s+Stub Backlog\b/;
    case "IN_PROGRESS":
      return /^##\s+In Progress\s*$/;
    case "DONE_VALIDATED":
    case "DONE_FAIL":
    case "DONE_OUTDATED_ONLY":
      return /^##\s+Done\s*$/;
    case "BLOCKED":
      return /^##\s+Blocked\s*$/;
    case "SUPERSEDED":
      return /^##\s+Superseded\b/;
    default:
      return null;
  }
}

function main() {
  const wpId = (process.argv[2] || "").trim();
  const status = (process.argv[3] || "").trim().toUpperCase();
  const reason = (process.argv[4] || "").trim();

  if (!wpId || !wpId.startsWith("WP-")) {
    fail("Usage: node .GOV/roles/orchestrator/scripts/task-board-set.mjs <WP_ID> <STATUS> [reason]", [
      "Example: node .GOV/roles/orchestrator/scripts/task-board-set.mjs WP-1-ModelSession-Core-Scheduler-v1 DONE_VALIDATED",
    ]);
  }

  if (!fs.existsSync(TASK_BOARD_PATH)) {
    fail("Missing task board", [`Expected: ${TASK_BOARD_PATH}`]);
  }

  const raw = readText(TASK_BOARD_PATH);
  const eol = detectEol(raw);
  let lines = raw.split(/\r?\n/);

  // Match TASK_BOARD entries like: `- **[WP-...-vN]** - [STATUS]`
  // Note: `\b` doesn't work here because the pattern ends in `**` (non-word chars).
  const wpLineRe = new RegExp(`^\\s*-\\s+\\*\\*\\[${escapeRegExp(wpId)}\\]\\*\\*(?=\\s|$)`);
  lines = lines.filter((l) => !wpLineRe.test(l));

  const headingRe = sectionForStatus(status);
  if (!headingRe) fail("Internal: missing section mapping for status", [status]);

  const section = findSection(lines, headingRe);
  if (!section) {
    fail("Target section not found in TASK_BOARD.md", [
      `status=${status}`,
      `expected_heading=${String(headingRe)}`,
    ]);
  }

  const targetLine = buildLine(wpId, status, reason);

  // Insert near end of section (keeps existing ordering stable).
  let insertIdx = section.endIdx;

  // Special-case: if the section contains a standalone horizontal rule at the top, keep it at the bottom
  // (treat it as a separator to the next section), and insert before it.
  {
    const body = lines.slice(section.startIdx + 1, section.endIdx);
    const firstNonEmptyRelIdx = body.findIndex((l) => l.trim() !== "");
    if (firstNonEmptyRelIdx !== -1) {
      const firstNonEmpty = body[firstNonEmptyRelIdx].trim();
      if (firstNonEmpty === "---") {
        insertIdx = section.startIdx + 1 + firstNonEmptyRelIdx;
      }
    }
  }

  lines.splice(insertIdx, 0, targetLine);

  // Readability: if we inserted right before a heading, keep a blank line between the entry and the heading.
  if (insertIdx + 1 < lines.length && /^##\s+/.test(lines[insertIdx + 1] || "")) {
    lines.splice(insertIdx + 1, 0, "");
  }

  // Ensure file ends with a newline.
  const out = lines.join(eol);
  writeText(TASK_BOARD_PATH, out.endsWith(eol) ? out : out + eol);

  console.log("task-board-set ok");
  console.log(`- wp_id: ${wpId}`);
  console.log(`- status: ${status}`);
}

main();
