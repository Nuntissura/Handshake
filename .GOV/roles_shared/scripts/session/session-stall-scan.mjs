#!/usr/bin/env node
/**
 * Session Stall Scan — RGF-104
 *
 * Scans the latest session output JSONL for stuck patterns and reports
 * whether the session appears stalled.
 *
 * Usage: node session-stall-scan.mjs <ROLE> <WP_ID>
 *
 * Exit code 1 if stall detected, 0 otherwise.
 */

import fs from "node:fs";
import path from "node:path";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
} from "../lib/runtime-paths.mjs";
import { sanitizeSessionKey } from "./session-control-lib.mjs";
import {
  findLatestProgressEvent,
  inspectSessionOutputActivity,
  isProgressEvent,
  summarizeActivityEvent,
} from "./session-output-activity-lib.mjs";

const PREFIX = "[STALL_SCAN]";
const TAIL_LINES = 80;
const OUTPUT_PROGRESS_GRACE_SECONDS = 180;

// --- CLI args ---
const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();

if (!role || !wpId) {
  console.error(`Usage: node session-stall-scan.mjs <ROLE> <WP_ID>`);
  process.exit(1);
}

const sessionKey = `${role}:${wpId}`;
const safeKey = sanitizeSessionKey(sessionKey);

// --- Locate output directory ---
const outputDir = path.resolve(
  GOVERNANCE_RUNTIME_ROOT_ABS,
  "roles_shared",
  "SESSION_CONTROL_OUTPUTS",
  safeKey,
);

if (!fs.existsSync(outputDir)) {
  console.error(`${PREFIX} ERROR: output directory not found: ${outputDir}`);
  process.exit(1);
}

// --- Find latest JSONL file ---
const jsonlFiles = fs
  .readdirSync(outputDir)
  .filter((f) => f.endsWith(".jsonl"))
  .map((f) => ({
    name: f,
    abs: path.join(outputDir, f),
    mtime: fs.statSync(path.join(outputDir, f)).mtimeMs,
  }))
  .sort((a, b) => b.mtime - a.mtime);

if (jsonlFiles.length === 0) {
  console.error(`${PREFIX} ERROR: no JSONL files found in ${outputDir}`);
  process.exit(1);
}

const latestFile = jsonlFiles[0].abs;
const latestFileMtimeMs = jsonlFiles[0].mtime;

const activity = inspectSessionOutputActivity(latestFile, { tailLines: TAIL_LINES });
const entries = activity.entries;

if (entries.length === 0) {
  console.log(`${PREFIX} OK: no parseable entries in ${path.basename(latestFile)} for ${role}:${wpId}`);
  process.exit(0);
}

// --- Stall detection ---
let stallType = "";

// (a) Same error message text (from stderr lines) repeated 3+ times
if (!stallType) {
  const stderrTexts = entries
    .filter((e) => e.type === "stderr")
    .map((e) => String(e.text || "").trim())
    .filter((t) => t.length > 0);
  const errorCounts = new Map();
  for (const text of stderrTexts) {
    errorCounts.set(text, (errorCounts.get(text) || 0) + 1);
  }
  for (const [, count] of errorCounts) {
    if (count >= 3) {
      stallType = "STALL_REPEATED_ERROR";
      break;
    }
  }
}

// (b) Agent messages containing "try again" / "retry" / "let me try" 3+ times
if (!stallType) {
  const retryPattern = /try again|retry|let me try/i;
  const agentMessages = entries
    .filter(
      (e) =>
        e.type === "item.completed" &&
        e.item?.type === "agent_message",
    )
    .map((e) => String(e.item?.text || ""));
  const retryCount = agentMessages.filter((m) => retryPattern.test(m)).length;
  if (retryCount >= 3) {
    stallType = "STALL_RETRY_LOOP";
  }
}

// (c) No ACP progress events in the last 20 entries
if (!stallType) {
  const last20 = entries.slice(-20);
  const hasProgressEvent = last20.some((entry) => isProgressEvent(entry));
  if (!hasProgressEvent && last20.length >= 20) {
    stallType = "STALL_NO_PROGRESS";
  }
}

// (d) Same command string executed 3+ times in the last 10 command entries
if (!stallType) {
  const commandEntries = entries
    .filter(
      (e) =>
        e.type === "item.completed" &&
        e.item?.type === "command_execution",
    )
    .slice(-10);
  const cmdCounts = new Map();
  for (const entry of commandEntries) {
    const cmd = String(entry.item?.command || entry.item?.input || "").trim();
    if (!cmd) continue;
    cmdCounts.set(cmd, (cmdCounts.get(cmd) || 0) + 1);
  }
  for (const [, count] of cmdCounts) {
    if (count >= 3) {
      stallType = "STALL_COMMAND_LOOP";
      break;
    }
  }
}

// --- Output ---
const latestFileIdleSeconds = Math.max(0, Math.trunc((Date.now() - latestFileMtimeMs) / 1000));
const latestProgressEvent = activity.latestProgressEvent || findLatestProgressEvent(entries);
const latestProgressSummary = summarizeActivityEvent(latestProgressEvent);
if (stallType === "STALL_NO_PROGRESS" && latestFileIdleSeconds <= OUTPUT_PROGRESS_GRACE_SECONDS) {
  console.log(
    `${PREFIX} OK: recent output progress (${latestFileIdleSeconds}s <= ${OUTPUT_PROGRESS_GRACE_SECONDS}s) suppresses STALL_NO_PROGRESS for ${role}:${wpId} | latest_progress=${latestProgressSummary}`,
  );
  process.exit(0);
}

if (stallType) {
  console.log(`${PREFIX} STALL DETECTED: ${stallType} for ${role}:${wpId} | latest_progress=${latestProgressSummary}`);
  process.exit(1);
} else {
  console.log(`${PREFIX} OK: no stall patterns detected for ${role}:${wpId} | latest_progress=${latestProgressSummary}`);
  process.exit(0);
}
