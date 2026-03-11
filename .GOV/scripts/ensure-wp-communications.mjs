#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const COMM_ROOT = path.join(".GOV", "roles_shared", "WP_COMMUNICATIONS");
const PACKETS_DIR = path.join(".GOV", "task_packets");
const THREAD_TEMPLATE = path.join(".GOV", "templates", "WP_COMMUNICATION_THREAD_TEMPLATE.md");
const RUNTIME_TEMPLATE = path.join(".GOV", "templates", "WP_RUNTIME_STATUS_TEMPLATE.json");
const RECEIPTS_TEMPLATE = path.join(".GOV", "templates", "WP_RECEIPTS_TEMPLATE.md");

function normalize(value) {
  return String(value || "").replace(/\\/g, "/").trim();
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function parsePacketStatus(text) {
  return (
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    ""
  ).trim() || "Ready for Dev";
}

function fillAll(text, replacements) {
  let output = text;
  for (const [token, value] of Object.entries(replacements)) {
    output = output.split(token).join(value);
  }
  return output;
}

function writeIfMissing(filePath, content) {
  if (fs.existsSync(filePath)) return false;
  fs.writeFileSync(filePath, content, "utf8");
  return true;
}

export function ensureWpCommunications({
  wpId,
  baseWpId,
  localBranch,
  localWorktreeDir,
  agenticMode,
  packetStatus,
  initializedAt,
} = {}) {
  const WP_ID = String(wpId || "").trim();
  if (!WP_ID || !WP_ID.startsWith("WP-")) {
    throw new Error("WP_ID is required");
  }

  const packetPath = path.join(PACKETS_DIR, `${WP_ID}.md`);
  let packetText = "";
  if (fs.existsSync(packetPath)) {
    packetText = fs.readFileSync(packetPath, "utf8");
  }

  const BASE_WP_ID = String(
    baseWpId ||
      parseSingleField(packetText, "BASE_WP_ID").replace(/\s*\(.*/, "") ||
      WP_ID.replace(/-v\d+$/, "")
  ).trim();
  const LOCAL_BRANCH = String(localBranch || parseSingleField(packetText, "LOCAL_BRANCH") || "<pending>").trim();
  const LOCAL_WORKTREE_DIR = String(localWorktreeDir || parseSingleField(packetText, "LOCAL_WORKTREE_DIR") || "<pending>").trim();
  const AGENTIC_MODE = String(agenticMode || parseSingleField(packetText, "AGENTIC_MODE") || "<pending>").trim();
  const PACKET_STATUS = String(packetStatus || parsePacketStatus(packetText) || "Ready for Dev").trim();
  const DATE_ISO = String(initializedAt || new Date().toISOString()).trim();

  fs.mkdirSync(COMM_ROOT, { recursive: true });
  const wpCommDir = path.join(COMM_ROOT, WP_ID);
  fs.mkdirSync(wpCommDir, { recursive: true });

  const threadPath = path.join(wpCommDir, "THREAD.md");
  const runtimeStatusPath = path.join(wpCommDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(wpCommDir, "RECEIPTS.md");

  const replacements = {
    "{{WP_ID}}": WP_ID,
    "{{BASE_WP_ID}}": BASE_WP_ID,
    "{{DATE_ISO}}": DATE_ISO,
    "{{LOCAL_BRANCH}}": LOCAL_BRANCH,
    "{{LOCAL_WORKTREE_DIR}}": LOCAL_WORKTREE_DIR,
    "{{AGENTIC_MODE}}": AGENTIC_MODE,
    "{{PACKET_STATUS}}": PACKET_STATUS,
  };

  const threadTemplate = fs.readFileSync(THREAD_TEMPLATE, "utf8");
  const runtimeTemplate = fs.readFileSync(RUNTIME_TEMPLATE, "utf8");
  const receiptsTemplate = fs.readFileSync(RECEIPTS_TEMPLATE, "utf8");

  writeIfMissing(threadPath, fillAll(threadTemplate, replacements));
  writeIfMissing(runtimeStatusPath, fillAll(runtimeTemplate, replacements));
  writeIfMissing(receiptsPath, fillAll(receiptsTemplate, replacements));

  return {
    dir: normalize(wpCommDir),
    threadFile: normalize(threadPath),
    runtimeStatusFile: normalize(runtimeStatusPath),
    receiptsFile: normalize(receiptsPath),
  };
}

function runCli() {
  const wpId = (process.argv[2] || "").trim();
  if (!wpId) {
    console.error("Usage: node .GOV/scripts/ensure-wp-communications.mjs WP-{ID}");
    process.exit(1);
  }

  const result = ensureWpCommunications({ wpId });
  console.log(`[WP_COMMUNICATIONS] ready ${result.dir}`);
  console.log(`- THREAD.md: ${result.threadFile}`);
  console.log(`- RUNTIME_STATUS.json: ${result.runtimeStatusFile}`);
  console.log(`- RECEIPTS.md: ${result.receiptsFile}`);
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) runCli();
