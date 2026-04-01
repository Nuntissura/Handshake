#!/usr/bin/env node
/**
 * Deterministic TASK_BOARD updater.
 *
 * Goal: remove manual markdown editing friction + format mistakes.
 * Scope: only moves a single WP_ID entry between sections.
 */

import fs from "node:fs";
import { GOV_ROOT_REPO_REL, repoPathAbs, resolveWorkPacketPath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { parseJsonFile, validateRuntimeStatus } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { syncRuntimeProjectionFromPacket } from "../../../roles_shared/scripts/lib/packet-runtime-projection-lib.mjs";
import { writeJsonFile } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { reconcileWpCommunicationTruth } from "../../../roles_shared/scripts/wp/ensure-wp-communications.mjs";

const TASK_BOARD_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`;

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

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function parsePacketStatus(text) {
  return (
    (String(text || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(text || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || ""
  ).trim() || "Ready for Dev";
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
    case "DONE_MERGE_PENDING":
      return `${base} - [MERGE_PENDING]`;
    case "DONE_FAIL":
      return `${base} - [FAIL]`;
    case "DONE_OUTDATED_ONLY":
      return `${base} - [OUTDATED_ONLY]`;
    case "DONE_ABANDONED":
      return `${base} - [ABANDONED]`;
    case "BLOCKED":
      return reason ? `${base} - [BLOCKED] - ${reason}` : `${base} - [BLOCKED]`;
    case "SUPERSEDED":
      return `${base} - [SUPERSEDED]`;
    default:
      fail("Unknown status", [
        `got=${status}`,
        "allowed: READY_FOR_DEV|STUB|IN_PROGRESS|DONE_MERGE_PENDING|DONE_VALIDATED|DONE_FAIL|DONE_OUTDATED_ONLY|DONE_ABANDONED|BLOCKED|SUPERSEDED",
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
    case "DONE_MERGE_PENDING":
    case "DONE_FAIL":
    case "DONE_OUTDATED_ONLY":
    case "DONE_ABANDONED":
      return /^##\s+Done\s*$/;
    case "BLOCKED":
      return /^##\s+Blocked\s*$/;
    case "SUPERSEDED":
      return /^##\s+Superseded\b/;
    default:
      return null;
  }
}

function expectedPacketStatusForBoardStatus(status) {
  switch (status) {
    case "READY_FOR_DEV":
      return "Ready for Dev";
    case "IN_PROGRESS":
      return "In Progress";
    case "BLOCKED":
      return "Blocked";
    case "DONE_MERGE_PENDING":
      return "Done";
    case "DONE_VALIDATED":
      return "Validated (PASS)";
    case "DONE_FAIL":
      return "Validated (FAIL)";
    case "DONE_OUTDATED_ONLY":
      return "Validated (OUTDATED_ONLY)";
    case "DONE_ABANDONED":
      return "Validated (ABANDONED)";
    default:
      return null;
  }
}

function syncRuntimeProjectionIfDeclared(wpId, packetText) {
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  if (!runtimeStatusFile) return null;
  const runtimeStatusAbsPath = repoPathAbs(runtimeStatusFile);
  if (!fs.existsSync(runtimeStatusAbsPath)) return null;
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");

  const runtimeStatus = parseJsonFile(runtimeStatusAbsPath);
  const receipts = receiptsFile && fs.existsSync(repoPathAbs(receiptsFile))
    ? String(fs.readFileSync(repoPathAbs(receiptsFile), "utf8"))
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => JSON.parse(line))
    : [];
  const reconciled = reconcileWpCommunicationTruth({
    wpId,
    packetPath: resolveWorkPacketPath(wpId)?.packetPath || "",
    packetText,
    runtimeStatus,
    receipts,
  });
  const syncedRuntimeStatus = syncRuntimeProjectionFromPacket(reconciled.nextRuntimeStatus, packetText, {
    eventName: "task_board_sync",
  });
  const runtimeErrors = validateRuntimeStatus(syncedRuntimeStatus);
  if (runtimeErrors.length > 0) {
    fail(`Runtime projection sync failed for ${wpId}`, runtimeErrors);
  }
  writeJsonFile(runtimeStatusAbsPath, syncedRuntimeStatus);
  return runtimeStatusAbsPath;
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

  const taskBoardAbsPath = repoPathAbs(TASK_BOARD_PATH);
  if (!fs.existsSync(taskBoardAbsPath)) {
    fail("Missing task board", [`Expected: ${TASK_BOARD_PATH}`]);
  }

  const resolvedPacket = resolveWorkPacketPath(wpId);
  const packetPath = resolvedPacket?.packetPath || "";
  const packetAbsPath = repoPathAbs(packetPath);
  if (!packetPath || !fs.existsSync(packetAbsPath)) {
    fail("Official packet not found", [packetPath || `<missing packet path for ${wpId}>`]);
  }
  const packetText = readText(packetAbsPath);
  const expectedPacketStatus = expectedPacketStatusForBoardStatus(status);
  const actualPacketStatus = parsePacketStatus(packetText);
  if (expectedPacketStatus && actualPacketStatus !== expectedPacketStatus) {
    fail("TASK_BOARD status transition conflicts with packet truth", [
      `wp_id=${wpId}`,
      `task_board_status=${status}`,
      `expected_packet_status=${expectedPacketStatus}`,
      `actual_packet_status=${actualPacketStatus || "<missing>"}`,
    ]);
  }

  const raw = readText(taskBoardAbsPath);
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
  writeText(taskBoardAbsPath, out.endsWith(eol) ? out : out + eol);
  const runtimeStatusFile = syncRuntimeProjectionIfDeclared(wpId, packetText);

  console.log("task-board-set ok");
  console.log(`- wp_id: ${wpId}`);
  console.log(`- status: ${status}`);
  if (runtimeStatusFile) console.log(`- runtime_synced: ${runtimeStatusFile}`);
}

main();
