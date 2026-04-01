#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  addMinutes,
  COMM_ROOT,
  communicationTransactionLockPathForWp,
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
import {
  deriveWpCommunicationAutoRoute,
  evaluateWpCommunicationHealth,
} from "../lib/wp-communication-health-lib.mjs";
import {
  applyWpReviewPacketProjection,
  applyWpReviewRuntimeProjection,
  deriveWpReviewPacketProjection,
} from "../lib/wp-review-projection-lib.mjs";
import { syncRuntimeProjectionFromPacket } from "../lib/packet-runtime-projection-lib.mjs";
import { GOV_ROOT_REPO_REL, repoPathAbs, workPacketPath } from "../lib/runtime-paths.mjs";
import { MAIN_CONTAINMENT_STATUS_VALUES } from "../lib/merge-progression-truth-lib.mjs";
import { withFileLockSync } from "../session/session-registry-lib.mjs";

const THREAD_TEMPLATE = path.join(GOV_ROOT_REPO_REL, "templates", "WP_COMMUNICATION_THREAD_TEMPLATE.md");
const RUNTIME_TEMPLATE = path.join(GOV_ROOT_REPO_REL, "templates", "WP_RUNTIME_STATUS_TEMPLATE.json");
const RECEIPTS_TEMPLATE = path.join(GOV_ROOT_REPO_REL, "templates", "WP_RECEIPTS_TEMPLATE.jsonl");
const TASK_BOARD_SET_SCRIPT_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
  "roles",
  "orchestrator",
  "scripts",
  "task-board-set.mjs",
);
const BUILD_ORDER_SYNC_SCRIPT_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
  "roles_shared",
  "scripts",
  "build-order-sync.mjs",
);
const RUNTIME_ROUTE_FIELD_NAMES = [
  "open_review_items",
  "next_expected_actor",
  "next_expected_session",
  "waiting_on",
  "waiting_on_session",
  "validator_trigger",
  "validator_trigger_reason",
  "ready_for_validation",
  "ready_for_validation_reason",
  "attention_required",
];
const AUTO_ROUTE_RUNTIME_FIELD_MAP = {
  next_expected_actor: "nextExpectedActor",
  next_expected_session: "nextExpectedSession",
  waiting_on: "waitingOn",
  waiting_on_session: "waitingOnSession",
  validator_trigger: "validatorTrigger",
  validator_trigger_reason: "validatorTriggerReason",
  ready_for_validation: "readyForValidation",
  ready_for_validation_reason: "readyForValidationReason",
  attention_required: "attentionRequired",
};

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

export function findUnreplacedTemplateTokens(text) {
  return [...new Set(String(text || "").match(/\{\{[A-Z0-9_]+\}\}/g) || [])].sort();
}

function assertNoUnreplacedTemplateTokens(text, label, wpId) {
  const unreplaced = findUnreplacedTemplateTokens(text);
  if (unreplaced.length > 0) {
    throw new Error(`Unreplaced template token(s) in ${label} for ${wpId}: ${unreplaced.join(", ")}`);
  }
}

function writeIfMissing(filePath, content) {
  const fileAbsPath = repoPathAbs(filePath);
  if (fs.existsSync(fileAbsPath)) return false;
  fs.writeFileSync(fileAbsPath, content, "utf8");
  return true;
}

function parseIntegerField(text, label, fallback) {
  const raw = parseSingleField(text, label);
  if (!raw) return fallback;
  const parsed = Number.parseInt(raw, 10);
  return Number.isInteger(parsed) ? parsed : fallback;
}

function normalizeNoneLike(value) {
  const raw = String(value || "").trim();
  if (!raw || /^(NONE|N\/A|NA|NULL)$/i.test(raw)) return null;
  return raw;
}

function requireTemplateFile(filePath) {
  if (!fs.existsSync(repoPathAbs(filePath))) {
    throw new Error(`Missing WP communication template: ${normalize(filePath)}`);
  }
}

function normalizeSessionValue(value) {
  const raw = normalizeNoneLike(value);
  return raw || null;
}

function parseSecondaryValidatorSessions(rawValue) {
  const raw = String(rawValue || "").trim();
  if (!raw || /^NONE$/i.test(raw)) return [];
  return raw.split(",").map((value) => value.trim()).filter(Boolean);
}

function syncRuntimeDeclaredFieldsFromPacket(runtimeStatus = {}, packetText = "", {
  packetPath = "",
} = {}) {
  const syncedRuntime = syncRuntimeProjectionFromPacket(runtimeStatus, packetText, {
    eventName: runtimeStatus?.last_event || "ensure_wp_communications",
    eventAt: runtimeStatus?.last_event_at || new Date().toISOString(),
  });
  syncedRuntime.task_packet = normalize(packetPath || parseSingleField(packetText, "TASK_ID"));
  syncedRuntime.communication_dir = normalize(parseSingleField(packetText, "WP_COMMUNICATION_DIR"));
  syncedRuntime.thread_file = normalize(parseSingleField(packetText, "WP_THREAD_FILE"));
  syncedRuntime.runtime_status_file = normalize(parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE"));
  syncedRuntime.receipts_file = normalize(parseSingleField(packetText, "WP_RECEIPTS_FILE"));
  syncedRuntime.workflow_lane = parseSingleField(packetText, "WORKFLOW_LANE");
  syncedRuntime.execution_owner = parseSingleField(packetText, "EXECUTION_OWNER");
  syncedRuntime.workflow_authority = parseSingleField(packetText, "WORKFLOW_AUTHORITY");
  syncedRuntime.technical_advisor = parseSingleField(packetText, "TECHNICAL_ADVISOR");
  syncedRuntime.technical_authority = parseSingleField(packetText, "TECHNICAL_AUTHORITY");
  syncedRuntime.merge_authority = parseSingleField(packetText, "MERGE_AUTHORITY");
  syncedRuntime.wp_validator_of_record = normalizeSessionValue(parseSingleField(packetText, "WP_VALIDATOR_OF_RECORD"));
  syncedRuntime.integration_validator_of_record = normalizeSessionValue(parseSingleField(packetText, "INTEGRATION_VALIDATOR_OF_RECORD"));
  syncedRuntime.secondary_validator_sessions = parseSecondaryValidatorSessions(
    parseSingleField(packetText, "SECONDARY_VALIDATOR_SESSIONS"),
  );
  syncedRuntime.agentic_mode = parseSingleField(packetText, "AGENTIC_MODE") || syncedRuntime.agentic_mode;
  syncedRuntime.current_branch = parseSingleField(packetText, "LOCAL_BRANCH") || syncedRuntime.current_branch;
  syncedRuntime.current_worktree_dir = parseSingleField(packetText, "LOCAL_WORKTREE_DIR") || syncedRuntime.current_worktree_dir;
  return syncedRuntime;
}

function isTerminalPacketStatus(status) {
  return status === "Done" || /^Validated \(/i.test(String(status || "").trim());
}

export function reconcileWpCommunicationTruth({
  wpId,
  packetPath,
  packetText,
  runtimeStatus = {},
  receipts = [],
} = {}) {
  const evaluation = evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath,
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE"),
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    receipts,
    runtimeStatus,
  });
  const latestReceipt = receipts.at(-1) || null;
  const autoRoute = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus,
    latestReceipt,
  });
  const packetProjection = deriveWpReviewPacketProjection({
    evaluation,
    autoRoute,
    packetText,
  });
  const nextPacketText = packetProjection?.packetStatus
    ? applyWpReviewPacketProjection(packetText, packetProjection)
    : packetText;
  const terminalPacketStatus = isTerminalPacketStatus(parsePacketStatus(nextPacketText));
  let nextRuntimeStatus = syncRuntimeDeclaredFieldsFromPacket(runtimeStatus, nextPacketText, {
    packetPath,
  });

  if (autoRoute.applicable && !terminalPacketStatus) {
    for (const fieldName of RUNTIME_ROUTE_FIELD_NAMES) {
      if (fieldName === "open_review_items") continue;
      const autoRouteFieldName = AUTO_ROUTE_RUNTIME_FIELD_MAP[fieldName];
      if (Object.prototype.hasOwnProperty.call(autoRoute, autoRouteFieldName)) {
        nextRuntimeStatus[fieldName] = autoRoute[autoRouteFieldName];
      }
    }
    if (latestReceipt) {
      nextRuntimeStatus.last_event = `receipt_${String(latestReceipt.receipt_kind || "").trim().toLowerCase()}`;
      nextRuntimeStatus.last_event_at = latestReceipt.timestamp_utc || nextRuntimeStatus.last_event_at;
    }
    nextRuntimeStatus = applyWpReviewRuntimeProjection(nextRuntimeStatus, { evaluation });
  }

  return {
    evaluation,
    autoRoute,
    packetProjection,
    latestReceipt,
    nextPacketText,
    nextRuntimeStatus,
  };
}

function syncProjectedTaskBoardTruth(wpId, projection = {}) {
  if (!projection?.taskBoardStatus) return;
  execFileSync(
    process.execPath,
    [
      TASK_BOARD_SET_SCRIPT_PATH,
      wpId,
      projection.taskBoardStatus,
      projection.taskBoardReason || "",
    ],
    { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );
  execFileSync(
    process.execPath,
    [BUILD_ORDER_SYNC_SCRIPT_PATH],
    { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );
}

export function buildWpCommunicationTemplateReplacements({
  wpId,
  baseWpId,
  dateIso,
  workflowLane,
  executionOwner,
  workflowAuthority,
  technicalAdvisor,
  technicalAuthority,
  mergeAuthority,
  wpValidatorOfRecord,
  integrationValidatorOfRecord,
  secondaryValidatorSessionsJson,
  localBranch,
  localWorktreeDir,
  agenticMode,
  packetStatus,
  mainContainmentStatusJson,
  mergedMainCommitJson,
  mainContainmentVerifiedAtUtcJson,
  currentMainCompatibilityStatusJson,
  currentMainCompatibilityBaselineShaJson,
  currentMainCompatibilityVerifiedAtUtcJson,
  packetWideningDecisionJson,
  packetWideningEvidenceJson,
  taskPacketPath,
  communicationDir,
  threadFile,
  runtimeStatusFile,
  receiptsFile,
  heartbeatIntervalMinutes,
  heartbeatDueAt,
  staleAfter,
  maxCoderRevisionCycles,
  maxValidatorReviewCycles,
  maxRelayEscalationCycles,
} = {}) {
  return {
    "{{WP_ID}}": wpId,
    "{{BASE_WP_ID}}": baseWpId,
    "{{DATE_ISO}}": dateIso,
    "{{WORKFLOW_LANE}}": workflowLane,
    "{{EXECUTION_OWNER}}": executionOwner,
    "{{WORKFLOW_AUTHORITY}}": workflowAuthority,
    "{{TECHNICAL_ADVISOR}}": technicalAdvisor,
    "{{TECHNICAL_AUTHORITY}}": technicalAuthority,
    "{{MERGE_AUTHORITY}}": mergeAuthority,
    "{{WP_VALIDATOR_OF_RECORD}}": wpValidatorOfRecord,
    "{{INTEGRATION_VALIDATOR_OF_RECORD}}": integrationValidatorOfRecord,
    "{{SECONDARY_VALIDATOR_SESSIONS}}": secondaryValidatorSessionsJson,
    "{{LOCAL_BRANCH}}": localBranch,
    "{{LOCAL_WORKTREE_DIR}}": localWorktreeDir,
    "{{AGENTIC_MODE}}": agenticMode,
    "{{PACKET_STATUS}}": packetStatus,
    "{{MAIN_CONTAINMENT_STATUS}}": mainContainmentStatusJson,
    "{{MERGED_MAIN_COMMIT}}": mergedMainCommitJson,
    "{{MAIN_CONTAINMENT_VERIFIED_AT_UTC}}": mainContainmentVerifiedAtUtcJson,
    "{{CURRENT_MAIN_COMPATIBILITY_STATUS}}": currentMainCompatibilityStatusJson,
    "{{CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA}}": currentMainCompatibilityBaselineShaJson,
    "{{CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC}}": currentMainCompatibilityVerifiedAtUtcJson,
    "{{PACKET_WIDENING_DECISION}}": packetWideningDecisionJson,
    "{{PACKET_WIDENING_EVIDENCE}}": packetWideningEvidenceJson,
    "{{TASK_PACKET_PATH}}": taskPacketPath,
    "{{WP_COMMUNICATION_DIR}}": communicationDir,
    "{{WP_THREAD_FILE}}": threadFile,
    "{{WP_RUNTIME_STATUS_FILE}}": runtimeStatusFile,
    "{{WP_RECEIPTS_FILE}}": receiptsFile,
    "{{HEARTBEAT_INTERVAL_MINUTES}}": heartbeatIntervalMinutes,
    "{{HEARTBEAT_DUE_AT}}": heartbeatDueAt,
    "{{STALE_AFTER}}": staleAfter,
    "{{MAX_CODER_REVISION_CYCLES}}": maxCoderRevisionCycles,
    "{{MAX_VALIDATOR_REVIEW_CYCLES}}": maxValidatorReviewCycles,
    "{{MAX_RELAY_ESCALATION_CYCLES}}": maxRelayEscalationCycles,
  };
}

function ensureWpCommunicationsCore({
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
  const packetAbsPath = repoPathAbs(packetPath);
  let packetText = "";
  if (fs.existsSync(packetAbsPath)) {
    packetText = fs.readFileSync(packetAbsPath, "utf8");
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
  const MAIN_CONTAINMENT_STATUS = String(
    normalizeNoneLike(parseSingleField(packetText, "MAIN_CONTAINMENT_STATUS")) || "NOT_STARTED",
  ).trim().toUpperCase();
  const MERGED_MAIN_COMMIT = normalizeNoneLike(parseSingleField(packetText, "MERGED_MAIN_COMMIT"));
  const MAIN_CONTAINMENT_VERIFIED_AT_UTC = normalizeNoneLike(
    parseSingleField(packetText, "MAIN_CONTAINMENT_VERIFIED_AT_UTC"),
  );
  const CURRENT_MAIN_COMPATIBILITY_STATUS = String(
    normalizeNoneLike(parseSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_STATUS")) || "NOT_RUN",
  ).trim().toUpperCase();
  const CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA = normalizeNoneLike(
    parseSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA"),
  );
  const CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC = normalizeNoneLike(
    parseSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC"),
  );
  const PACKET_WIDENING_DECISION = String(
    normalizeNoneLike(parseSingleField(packetText, "PACKET_WIDENING_DECISION")) || "NONE",
  ).trim().toUpperCase();
  const PACKET_WIDENING_EVIDENCE = normalizeNoneLike(
    parseSingleField(packetText, "PACKET_WIDENING_EVIDENCE"),
  );
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

  fs.mkdirSync(repoPathAbs(COMM_ROOT), { recursive: true });
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
  fs.mkdirSync(repoPathAbs(wpCommDir), { recursive: true });

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
  if (!MAIN_CONTAINMENT_STATUS_VALUES.includes(MAIN_CONTAINMENT_STATUS)) {
    throw new Error(`Invalid MAIN_CONTAINMENT_STATUS for ${WP_ID}: ${MAIN_CONTAINMENT_STATUS}`);
  }
  if (!CURRENT_MAIN_COMPATIBILITY_STATUS) {
    throw new Error(`Invalid CURRENT_MAIN_COMPATIBILITY_STATUS for ${WP_ID}: ${CURRENT_MAIN_COMPATIBILITY_STATUS}`);
  }
  if (!PACKET_WIDENING_DECISION) {
    throw new Error(`Invalid PACKET_WIDENING_DECISION for ${WP_ID}: ${PACKET_WIDENING_DECISION}`);
  }

  const replacements = buildWpCommunicationTemplateReplacements({
    wpId: WP_ID,
    baseWpId: BASE_WP_ID,
    dateIso: DATE_ISO,
    workflowLane: WORKFLOW_LANE,
    executionOwner: EXECUTION_OWNER,
    workflowAuthority: WORKFLOW_AUTHORITY,
    technicalAdvisor: TECHNICAL_ADVISOR,
    technicalAuthority: TECHNICAL_AUTHORITY,
    mergeAuthority: MERGE_AUTHORITY,
    wpValidatorOfRecord: WP_VALIDATOR_OF_RECORD ? JSON.stringify(WP_VALIDATOR_OF_RECORD) : "null",
    integrationValidatorOfRecord: INTEGRATION_VALIDATOR_OF_RECORD ? JSON.stringify(INTEGRATION_VALIDATOR_OF_RECORD) : "null",
    secondaryValidatorSessionsJson: SECONDARY_VALIDATOR_SESSIONS_RAW.toUpperCase() === "NONE"
      ? "[]"
      : JSON.stringify(SECONDARY_VALIDATOR_SESSIONS_RAW.split(",").map((value) => value.trim()).filter(Boolean)),
    localBranch: LOCAL_BRANCH,
    localWorktreeDir: LOCAL_WORKTREE_DIR,
    agenticMode: AGENTIC_MODE,
    packetStatus: PACKET_STATUS,
    mainContainmentStatusJson: JSON.stringify(MAIN_CONTAINMENT_STATUS),
    mergedMainCommitJson: MERGED_MAIN_COMMIT ? JSON.stringify(MERGED_MAIN_COMMIT) : "null",
    mainContainmentVerifiedAtUtcJson: MAIN_CONTAINMENT_VERIFIED_AT_UTC ? JSON.stringify(MAIN_CONTAINMENT_VERIFIED_AT_UTC) : "null",
    currentMainCompatibilityStatusJson: JSON.stringify(CURRENT_MAIN_COMPATIBILITY_STATUS),
    currentMainCompatibilityBaselineShaJson: CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA ? JSON.stringify(CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA) : "null",
    currentMainCompatibilityVerifiedAtUtcJson: CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC ? JSON.stringify(CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC) : "null",
    packetWideningDecisionJson: JSON.stringify(PACKET_WIDENING_DECISION),
    packetWideningEvidenceJson: PACKET_WIDENING_EVIDENCE ? JSON.stringify(PACKET_WIDENING_EVIDENCE) : "null",
    taskPacketPath: normalize(packetPath),
    communicationDir: normalize(wpCommDir),
    threadFile: normalize(threadPath),
    runtimeStatusFile: normalize(runtimeStatusPath),
    receiptsFile: normalize(receiptsPath),
    heartbeatIntervalMinutes: String(HEARTBEAT_INTERVAL_MINUTES),
    heartbeatDueAt: addMinutes(DATE_ISO, HEARTBEAT_INTERVAL_MINUTES),
    staleAfter: addMinutes(DATE_ISO, STALE_AFTER_MINUTES),
    maxCoderRevisionCycles: String(MAX_CODER_REVISION_CYCLES),
    maxValidatorReviewCycles: String(MAX_VALIDATOR_REVIEW_CYCLES),
    maxRelayEscalationCycles: String(MAX_RELAY_ESCALATION_CYCLES),
  });

  const threadTemplate = fs.readFileSync(repoPathAbs(THREAD_TEMPLATE), "utf8");
  const runtimeTemplate = fs.readFileSync(repoPathAbs(RUNTIME_TEMPLATE), "utf8");
  const receiptsTemplate = fs.readFileSync(repoPathAbs(RECEIPTS_TEMPLATE), "utf8");

  const renderedThread = fillAll(threadTemplate, replacements);
  const renderedRuntime = fillAll(runtimeTemplate, replacements);
  const renderedReceipts = fillAll(receiptsTemplate, replacements);

  assertNoUnreplacedTemplateTokens(renderedThread, THREAD_FILE_NAME, WP_ID);
  assertNoUnreplacedTemplateTokens(renderedRuntime, RUNTIME_STATUS_FILE_NAME, WP_ID);
  assertNoUnreplacedTemplateTokens(renderedReceipts, RECEIPTS_FILE_NAME, WP_ID);

  writeIfMissing(threadPath, renderedThread);
  writeIfMissing(runtimeStatusPath, renderedRuntime);
  writeIfMissing(receiptsPath, renderedReceipts);

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

  const reconciliation = reconcileWpCommunicationTruth({
    wpId: WP_ID,
    packetPath: normalize(packetPath),
    packetText,
    runtimeStatus,
    receipts,
  });
  const reconciledRuntimeErrors = validateRuntimeStatus(reconciliation.nextRuntimeStatus);
  if (reconciledRuntimeErrors.length > 0) {
    throw new Error(`Reconciled runtime status failed validation for ${WP_ID}: ${reconciledRuntimeErrors.join("; ")}`);
  }
  if (reconciliation.nextPacketText !== packetText) {
    fs.writeFileSync(packetAbsPath, reconciliation.nextPacketText, "utf8");
  }
  if (JSON.stringify(reconciliation.nextRuntimeStatus) !== JSON.stringify(runtimeStatus)) {
    fs.writeFileSync(repoPathAbs(runtimeStatusPath), `${JSON.stringify(reconciliation.nextRuntimeStatus, null, 2)}\n`, "utf8");
  }
  syncProjectedTaskBoardTruth(WP_ID, reconciliation.packetProjection);

  return {
    dir: normalize(wpCommDir),
    threadFile: normalize(threadPath),
    runtimeStatusFile: normalize(runtimeStatusPath),
    receiptsFile: normalize(receiptsPath),
  };
}

export function ensureWpCommunications(args = {}, options = {}) {
  const WP_ID = String(args?.wpId || "").trim();
  const run = () => ensureWpCommunicationsCore(args);
  if (options.assumeTransactionLock || !WP_ID || !/^WP-/.test(WP_ID)) {
    return run();
  }
  return withFileLockSync(communicationTransactionLockPathForWp(WP_ID), run);
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
