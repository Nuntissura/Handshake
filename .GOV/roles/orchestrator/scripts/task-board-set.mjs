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
import {
  expectedPacketStatusForTaskBoardStatus,
  parsePacketStatus,
} from "../../../roles_shared/scripts/lib/wp-authority-projection-lib.mjs";
import {
  materializeRuntimeAuthorityView,
  readExecutionPublicationView,
} from "../../../roles_shared/scripts/lib/wp-execution-state-lib.mjs";
import { writeJsonFile } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { reconcileWpCommunicationTruth } from "../../../roles_shared/scripts/wp/ensure-wp-communications.mjs";
import { capturePreTaskSnapshot } from "../../../roles_shared/scripts/memory/memory-snapshot.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("task-board-set.mjs", { role: "ORCHESTRATOR" });

const TASK_BOARD_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`;

function fail(message, details = []) {
  failWithMemory("task-board-set.mjs", message, { role: "ORCHESTRATOR", details });
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

function writeText(p, text) {
  try {
    fs.writeFileSync(p, text, "utf8");
  } catch (e) {
    fail(`Failed to write: ${p}`, [String(e?.message || e)]);
  }
}

function writeTextIfChanged(p, text) {
  const currentText = fs.existsSync(p) ? readText(p) : null;
  if (currentText === text) return false;
  writeText(p, text);
  return true;
}

function writeJsonFileIfChanged(p, value) {
  const nextText = `${JSON.stringify(value, null, 2)}\n`;
  const currentText = fs.existsSync(p) ? readText(p) : null;
  if (currentText === nextText) return false;
  writeJsonFile(p, value);
  return true;
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

function loadDeclaredRuntimeContext(packetText) {
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
  return {
    runtimeStatusFile,
    runtimeStatusAbsPath,
    runtimeStatus,
    receipts,
  };
}

function syncRuntimeProjectionIfDeclared(wpId, packetText, runtimeContext = null, {
  syncFromPacket = true,
} = {}) {
  const declaredRuntime = runtimeContext || loadDeclaredRuntimeContext(packetText);
  if (!declaredRuntime) return null;
  const {
    runtimeStatusFile,
    runtimeStatusAbsPath,
    runtimeStatus,
    receipts,
  } = declaredRuntime;

  if (!syncFromPacket) {
    const materializedRuntime = materializeRuntimeAuthorityView(runtimeStatus);
    const runtimeErrors = validateRuntimeStatus(materializedRuntime);
    if (runtimeErrors.length > 0) {
      fail(`Runtime authority materialization failed for ${wpId}`, runtimeErrors);
    }
    return {
      filePath: runtimeStatusFile,
      wrote: writeJsonFileIfChanged(runtimeStatusAbsPath, materializedRuntime),
    };
  }

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
  return {
    filePath: runtimeStatusAbsPath,
    wrote: writeJsonFileIfChanged(runtimeStatusAbsPath, syncedRuntimeStatus),
  };
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

  // RGF-146: pre-task snapshot before board status change
  capturePreTaskSnapshot({
    snapshotType: "PRE_BOARD_STATUS_CHANGE",
    wpId,
    triggerScript: "task-board-set.mjs",
    context: {
      newStatus: status,
      reason: reason || "",
    },
  });

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
  const runtimeContext = loadDeclaredRuntimeContext(packetText);
  const expectedPacketStatus = expectedPacketStatusForTaskBoardStatus(status);
  const actualPacketStatus = parsePacketStatus(packetText);
  const runtimePublication = readExecutionPublicationView({
    runtimeStatus: runtimeContext?.runtimeStatus || {},
    packetStatus: actualPacketStatus,
  });
  if (runtimePublication.has_canonical_authority && runtimePublication.task_board_status) {
    if (status !== runtimePublication.task_board_status) {
      fail("TASK_BOARD status transition conflicts with canonical execution authority", [
        `wp_id=${wpId}`,
        `task_board_status=${status}`,
        `canonical_task_board_status=${runtimePublication.task_board_status}`,
        `canonical_packet_status=${runtimePublication.canonical_packet_status || "<missing>"}`,
        `packet_artifact_status=${actualPacketStatus || "<missing>"}`,
      ]);
    }
  } else if (expectedPacketStatus && actualPacketStatus !== expectedPacketStatus) {
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

  // Keep the entry anchored to the actual section body instead of drifting downward
  // through trailing blank lines on repeated idempotent writes.
  while (insertIdx > (section.startIdx + 1) && String(lines[insertIdx - 1] || "").trim() === "") {
    insertIdx -= 1;
  }

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
  const nextTaskBoardText = out.endsWith(eol) ? out : out + eol;
  const boardChanged = writeTextIfChanged(taskBoardAbsPath, nextTaskBoardText);
  const runtimeSync = syncRuntimeProjectionIfDeclared(wpId, packetText, runtimeContext, {
    syncFromPacket: !runtimePublication.has_canonical_authority,
  });

  console.log("task-board-set ok");
  console.log(`- wp_id: ${wpId}`);
  console.log(`- status: ${status}`);
  console.log(`- task_board_change: ${boardChanged ? "updated" : "no-op"}`);
  if (runtimePublication.has_canonical_authority) console.log("- runtime_authority: canonical");
  if (runtimeSync?.filePath) console.log(`- runtime_synced: ${runtimeSync.filePath}`);
  if (runtimeSync) console.log(`- runtime_change: ${runtimeSync.wrote ? "updated" : "no-op"}`);
}

main();
