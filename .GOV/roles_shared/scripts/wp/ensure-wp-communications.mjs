#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  addMinutes,
  COMM_ROOT,
  communicationPathsForWp,
  ensureSchemaFilesExist,
  normalize,
  parseJsonFile,
  parseJsonlFile,
  NOTIFICATIONS_FILE_NAME,
  NOTIFICATION_CURSOR_FILE_NAME,
  RECEIPTS_FILE_NAME,
  RUNTIME_STATUS_FILE_NAME,
  THREAD_FILE_NAME,
  validateReceipt,
  validateRuntimeStatus,
  WORKFLOW_LANE_VALUES,
  EXECUTION_OWNER_VALUES,
  AGENTIC_MODE_VALUES,
} from "../lib/wp-communications-lib.mjs";
import { GOV_ROOT_REPO_REL, workPacketPath } from "../lib/runtime-paths.mjs";

const THREAD_TEMPLATE = path.join(GOV_ROOT_REPO_REL, "templates", "WP_COMMUNICATION_THREAD_TEMPLATE.md");
const RUNTIME_TEMPLATE = path.join(GOV_ROOT_REPO_REL, "templates", "WP_RUNTIME_STATUS_TEMPLATE.json");
const RECEIPTS_TEMPLATE = path.join(GOV_ROOT_REPO_REL, "templates", "WP_RECEIPTS_TEMPLATE.jsonl");

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

function parseIntegerField(text, label, fallback) {
  const raw = parseSingleField(text, label);
  if (!raw) return fallback;
  const parsed = Number.parseInt(raw, 10);
  return Number.isInteger(parsed) ? parsed : fallback;
}

function requireTemplateFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Missing WP communication template: ${normalize(filePath)}`);
  }
}

export function ensureWpCommunications({
  wpId,
  baseWpId,
  workflowLane,
  executionOwner,
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

  const packetPath = workPacketPath(WP_ID);
  let packetText = "";
  if (fs.existsSync(packetPath)) {
    packetText = fs.readFileSync(packetPath, "utf8");
  }

  const BASE_WP_ID = String(
    baseWpId ||
      parseSingleField(packetText, "BASE_WP_ID").replace(/\s*\(.*/, "") ||
      WP_ID.replace(/-v\d+$/, "")
  ).trim();
  const WORKFLOW_LANE = String(workflowLane || parseSingleField(packetText, "WORKFLOW_LANE") || "").trim();
  const EXECUTION_OWNER = String(executionOwner || parseSingleField(packetText, "EXECUTION_OWNER") || "").trim();
  const LOCAL_BRANCH = String(localBranch || parseSingleField(packetText, "LOCAL_BRANCH") || "<pending>").trim();
  const LOCAL_WORKTREE_DIR = String(localWorktreeDir || parseSingleField(packetText, "LOCAL_WORKTREE_DIR") || "<pending>").trim();
  const AGENTIC_MODE = String(agenticMode || parseSingleField(packetText, "AGENTIC_MODE") || "NO").trim();
  const PACKET_STATUS = String(packetStatus || parsePacketStatus(packetText) || "Ready for Dev").trim();
  const WORKFLOW_AUTHORITY = String(parseSingleField(packetText, "WORKFLOW_AUTHORITY") || "ORCHESTRATOR").trim();
  const TECHNICAL_ADVISOR = String(parseSingleField(packetText, "TECHNICAL_ADVISOR") || "WP_VALIDATOR").trim();
  const TECHNICAL_AUTHORITY = String(parseSingleField(packetText, "TECHNICAL_AUTHORITY") || "INTEGRATION_VALIDATOR").trim();
  const MERGE_AUTHORITY = String(parseSingleField(packetText, "MERGE_AUTHORITY") || "INTEGRATION_VALIDATOR").trim();
  const WP_VALIDATOR_OF_RECORD = String(parseSingleField(packetText, "WP_VALIDATOR_OF_RECORD") || "").trim();
  const INTEGRATION_VALIDATOR_OF_RECORD = String(parseSingleField(packetText, "INTEGRATION_VALIDATOR_OF_RECORD") || "").trim();
  const SECONDARY_VALIDATOR_SESSIONS_RAW = String(parseSingleField(packetText, "SECONDARY_VALIDATOR_SESSIONS") || "NONE").trim();
  const DATE_ISO = String(initializedAt || new Date().toISOString()).trim();
  const HEARTBEAT_INTERVAL_MINUTES = parseIntegerField(packetText, "HEARTBEAT_INTERVAL_MINUTES", 15);
  const STALE_AFTER_MINUTES = parseIntegerField(packetText, "STALE_AFTER_MINUTES", 45);
  const MAX_CODER_REVISION_CYCLES = parseIntegerField(packetText, "MAX_CODER_REVISION_CYCLES", 3);
  const MAX_VALIDATOR_REVIEW_CYCLES = parseIntegerField(packetText, "MAX_VALIDATOR_REVIEW_CYCLES", 3);
  const MAX_RELAY_ESCALATION_CYCLES = parseIntegerField(packetText, "MAX_RELAY_ESCALATION_CYCLES", 2);
  const declaredCommunicationDir = parseSingleField(packetText, "WP_COMMUNICATION_DIR");
  const declaredThreadFile = parseSingleField(packetText, "WP_THREAD_FILE");
  const declaredRuntimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const declaredReceiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");

  if (packetText) {
    const warnings = [];
    if (!parseSingleField(packetText, "BASE_WP_ID") && !baseWpId) warnings.push("BASE_WP_ID missing; defaulted from WP_ID");
    if (!parseSingleField(packetText, "WORKFLOW_LANE") && !workflowLane) warnings.push("WORKFLOW_LANE missing; explicit workflow tuple is required");
    if (!parseSingleField(packetText, "EXECUTION_OWNER") && !executionOwner) warnings.push("EXECUTION_OWNER missing; explicit coder ownership is required");
    if (!parseSingleField(packetText, "LOCAL_BRANCH") && !localBranch) warnings.push("LOCAL_BRANCH missing; defaulted to <pending>");
    if (!parseSingleField(packetText, "LOCAL_WORKTREE_DIR") && !localWorktreeDir) warnings.push("LOCAL_WORKTREE_DIR missing; defaulted to <pending>");
    if (!parseSingleField(packetText, "AGENTIC_MODE") && !agenticMode) warnings.push("AGENTIC_MODE missing; defaulted to NO");
    if (!parseSingleField(packetText, "WORKFLOW_AUTHORITY")) warnings.push("WORKFLOW_AUTHORITY missing; defaulted to ORCHESTRATOR");
    if (!parseSingleField(packetText, "TECHNICAL_ADVISOR")) warnings.push("TECHNICAL_ADVISOR missing; defaulted to WP_VALIDATOR");
    if (!parseSingleField(packetText, "TECHNICAL_AUTHORITY")) warnings.push("TECHNICAL_AUTHORITY missing; defaulted to INTEGRATION_VALIDATOR");
    if (!parseSingleField(packetText, "MERGE_AUTHORITY")) warnings.push("MERGE_AUTHORITY missing; defaulted to INTEGRATION_VALIDATOR");
    for (const warning of warnings) {
      console.warn(`[WP_COMMUNICATIONS] ${WP_ID}: ${warning}`);
    }
  }

  ensureSchemaFilesExist();
  requireTemplateFile(THREAD_TEMPLATE);
  requireTemplateFile(RUNTIME_TEMPLATE);
  requireTemplateFile(RECEIPTS_TEMPLATE);

  fs.mkdirSync(COMM_ROOT, { recursive: true });
  const expectedPaths = communicationPathsForWp(WP_ID);
  if (declaredCommunicationDir || declaredThreadFile || declaredRuntimeStatusFile || declaredReceiptsFile) {
    const declaredPaths = {
      dir: normalize(declaredCommunicationDir),
      threadFile: normalize(declaredThreadFile),
      runtimeStatusFile: normalize(declaredRuntimeStatusFile),
      receiptsFile: normalize(declaredReceiptsFile),
    };
    if (
      declaredPaths.dir !== expectedPaths.dir
      || declaredPaths.threadFile !== expectedPaths.threadFile
      || declaredPaths.runtimeStatusFile !== expectedPaths.runtimeStatusFile
      || declaredPaths.receiptsFile !== expectedPaths.receiptsFile
    ) {
      throw new Error(
        `Packet ${WP_ID} declares legacy or non-authoritative WP communication paths. Expected ${expectedPaths.dir} and matching THREAD/RUNTIME_STATUS/RECEIPTS files.`,
      );
    }
  }
  const paths = expectedPaths;
  const wpCommDir = paths.dir;
  fs.mkdirSync(wpCommDir, { recursive: true });

  const threadPath = paths.threadFile;
  const runtimeStatusPath = paths.runtimeStatusFile;
  const receiptsPath = paths.receiptsFile;

  if (!WORKFLOW_LANE_VALUES.includes(WORKFLOW_LANE)) {
    throw new Error(`Invalid WORKFLOW_LANE for ${WP_ID}: ${WORKFLOW_LANE}`);
  }
  if (!EXECUTION_OWNER_VALUES.includes(EXECUTION_OWNER)) {
    throw new Error(`Invalid EXECUTION_OWNER for ${WP_ID}: ${EXECUTION_OWNER}`);
  }
  if (!AGENTIC_MODE_VALUES.includes(AGENTIC_MODE)) {
    throw new Error(`Invalid AGENTIC_MODE for ${WP_ID}: ${AGENTIC_MODE}`);
  }

  const replacements = {
    "{{WP_ID}}": WP_ID,
    "{{BASE_WP_ID}}": BASE_WP_ID,
    "{{DATE_ISO}}": DATE_ISO,
    "{{WORKFLOW_LANE}}": WORKFLOW_LANE,
    "{{EXECUTION_OWNER}}": EXECUTION_OWNER,
    "{{WORKFLOW_AUTHORITY}}": WORKFLOW_AUTHORITY,
    "{{TECHNICAL_ADVISOR}}": TECHNICAL_ADVISOR,
    "{{TECHNICAL_AUTHORITY}}": TECHNICAL_AUTHORITY,
    "{{MERGE_AUTHORITY}}": MERGE_AUTHORITY,
    "{{WP_VALIDATOR_OF_RECORD}}": WP_VALIDATOR_OF_RECORD ? JSON.stringify(WP_VALIDATOR_OF_RECORD) : "null",
    "{{INTEGRATION_VALIDATOR_OF_RECORD}}": INTEGRATION_VALIDATOR_OF_RECORD ? JSON.stringify(INTEGRATION_VALIDATOR_OF_RECORD) : "null",
    "{{SECONDARY_VALIDATOR_SESSIONS}}": SECONDARY_VALIDATOR_SESSIONS_RAW.toUpperCase() === "NONE"
      ? "[]"
      : JSON.stringify(SECONDARY_VALIDATOR_SESSIONS_RAW.split(",").map((value) => value.trim()).filter(Boolean)),
    "{{LOCAL_BRANCH}}": LOCAL_BRANCH,
    "{{LOCAL_WORKTREE_DIR}}": LOCAL_WORKTREE_DIR,
    "{{AGENTIC_MODE}}": AGENTIC_MODE,
    "{{PACKET_STATUS}}": PACKET_STATUS,
    "{{TASK_PACKET_PATH}}": normalize(packetPath),
    "{{WP_COMMUNICATION_DIR}}": normalize(wpCommDir),
    "{{WP_THREAD_FILE}}": normalize(threadPath),
    "{{WP_RUNTIME_STATUS_FILE}}": normalize(runtimeStatusPath),
    "{{WP_RECEIPTS_FILE}}": normalize(receiptsPath),
    "{{HEARTBEAT_INTERVAL_MINUTES}}": String(HEARTBEAT_INTERVAL_MINUTES),
    "{{HEARTBEAT_DUE_AT}}": addMinutes(DATE_ISO, HEARTBEAT_INTERVAL_MINUTES),
    "{{STALE_AFTER}}": addMinutes(DATE_ISO, STALE_AFTER_MINUTES),
    "{{MAX_CODER_REVISION_CYCLES}}": String(MAX_CODER_REVISION_CYCLES),
    "{{MAX_VALIDATOR_REVIEW_CYCLES}}": String(MAX_VALIDATOR_REVIEW_CYCLES),
    "{{MAX_RELAY_ESCALATION_CYCLES}}": String(MAX_RELAY_ESCALATION_CYCLES),
  };

  const threadTemplate = fs.readFileSync(THREAD_TEMPLATE, "utf8");
  const runtimeTemplate = fs.readFileSync(RUNTIME_TEMPLATE, "utf8");
  const receiptsTemplate = fs.readFileSync(RECEIPTS_TEMPLATE, "utf8");

  writeIfMissing(threadPath, fillAll(threadTemplate, replacements));
  writeIfMissing(runtimeStatusPath, fillAll(runtimeTemplate, replacements));
  writeIfMissing(receiptsPath, fillAll(receiptsTemplate, replacements));

  const notificationsPath = normalize(path.join(wpCommDir, NOTIFICATIONS_FILE_NAME));
  const cursorPath = normalize(path.join(wpCommDir, NOTIFICATION_CURSOR_FILE_NAME));
  writeIfMissing(notificationsPath, "");
  writeIfMissing(cursorPath, `${JSON.stringify({ schema_version: "wp_notification_cursor@1", cursors: {} }, null, 2)}\n`);

  const runtimeStatus = parseJsonFile(runtimeStatusPath);
  const runtimeErrors = validateRuntimeStatus(runtimeStatus);
  if (runtimeErrors.length > 0) {
    throw new Error(`Generated runtime status failed validation for ${WP_ID}: ${runtimeErrors.join("; ")}`);
  }

  const receipts = parseJsonlFile(receiptsPath);
  if (receipts.length === 0) {
    throw new Error(`Generated receipts ledger is empty for ${WP_ID}`);
  }
  const receiptErrors = [];
  receipts.forEach((entry, index) => {
    for (const error of validateReceipt(entry)) {
      receiptErrors.push(`line ${index + 1}: ${error}`);
    }
  });
  if (receiptErrors.length > 0) {
    throw new Error(`Generated receipts ledger failed validation for ${WP_ID}: ${receiptErrors.join("; ")}`);
  }

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
    console.error("Usage: node .GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs WP-{ID}");
    process.exit(1);
  }

  const result = ensureWpCommunications({ wpId });
  console.log(`[WP_COMMUNICATIONS] ready ${result.dir}`);
  console.log(`- THREAD.md: ${result.threadFile}`);
  console.log(`- ${RUNTIME_STATUS_FILE_NAME}: ${result.runtimeStatusFile}`);
  console.log(`- ${RECEIPTS_FILE_NAME}: ${result.receiptsFile}`);
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) runCli();
