#!/usr/bin/env node

import fs from "node:fs";
import {
  COMMUNICATION_HEALTH_STAGE_VALUES,
  evaluateWpCommunicationBoundary,
  evaluateWpCommunicationHealth,
} from "../scripts/lib/wp-communication-health-lib.mjs";
import { normalize, parseJsonFile, parseJsonlFile } from "../scripts/lib/wp-communications-lib.mjs";
import { GOV_ROOT_REPO_REL, repoPathAbs, resolveWorkPacketPath } from "../scripts/lib/runtime-paths.mjs";
import { ensureWpCommunications } from "../scripts/wp/ensure-wp-communications.mjs";
import { checkAllNotifications } from "../scripts/wp/wp-check-notifications.mjs";

function usage() {
  console.error(
    `Usage: node ${GOV_ROOT_REPO_REL}/roles_shared/checks/wp-communication-health-check.mjs`
    + ` WP-{ID} [${COMMUNICATION_HEALTH_STAGE_VALUES.join("|")}]`
  );
  process.exit(1);
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function resolvePacketContext(wpId) {
  const resolved = resolveWorkPacketPath(wpId);
  const packetPath = resolved?.packetPath || `${GOV_ROOT_REPO_REL}/task_packets/${wpId}.md`;
  const packetAbsPath = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbsPath)) {
    console.error(`[WP_COMMUNICATION_HEALTH] FAIL: Official packet not found`);
    console.error(`  - ${normalize(packetPath)}`);
    process.exit(1);
  }
  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  return {
    packetPath: normalize(packetPath),
    packetText,
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE"),
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    receiptsFile: parseSingleField(packetText, "WP_RECEIPTS_FILE"),
    runtimeStatusFile: parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE"),
  };
}

function printResult(evaluation) {
  const prefix = evaluation.ok ? "PASS" : "FAIL";
  console.log(`[WP_COMMUNICATION_HEALTH] ${prefix}: ${evaluation.message}`);
  for (const detail of evaluation.details || []) {
    console.log(`  - ${detail}`);
  }
  process.exit(evaluation.ok ? 0 : 1);
}

const wpId = String(process.argv[2] || "").trim();
const stage = String(process.argv[3] || "STATUS").trim().toUpperCase();
if (!wpId || !/^WP-/.test(wpId)) usage();
if (!COMMUNICATION_HEALTH_STAGE_VALUES.includes(stage)) usage();

ensureWpCommunications({ wpId });

const context = resolvePacketContext(wpId);
const receipts = context.receiptsFile && fs.existsSync(repoPathAbs(context.receiptsFile))
  ? parseJsonlFile(context.receiptsFile)
  : [];
const runtimeStatus = context.runtimeStatusFile && fs.existsSync(repoPathAbs(context.runtimeStatusFile))
  ? parseJsonFile(context.runtimeStatusFile)
  : { open_review_items: [] };
const latestReceipt = receipts.at(-1) || null;
const pendingNotifications = Object.values(checkAllNotifications({ wpId })).flatMap((entry) => entry.notifications || []);

const evaluation = evaluateWpCommunicationHealth({
  wpId,
  stage,
  packetPath: context.packetPath,
  packetContent: context.packetText,
  workflowLane: context.workflowLane,
  packetFormatVersion: context.packetFormatVersion,
  communicationContract: context.communicationContract,
  communicationHealthGate: context.communicationHealthGate,
  receipts,
  runtimeStatus,
});

const statusEvaluation = stage === "STATUS"
  ? evaluation
  : evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath: context.packetPath,
    packetContent: context.packetText,
    workflowLane: context.workflowLane,
    packetFormatVersion: context.packetFormatVersion,
    communicationContract: context.communicationContract,
    communicationHealthGate: context.communicationHealthGate,
    receipts,
    runtimeStatus,
  });
const boundary = evaluateWpCommunicationBoundary({
  stage,
  statusEvaluation,
  runtimeStatus,
  latestReceipt,
  pendingNotifications,
});

printResult({
  ...evaluation,
  ok: evaluation.ok && boundary.ok,
  message: !evaluation.ok
    ? evaluation.message
    : boundary.ok
      ? evaluation.message
      : "Direct review route projection or notification boundary is inconsistent",
  details: [
    ...(evaluation.details || []),
    ...(boundary.issues || []),
    ...((boundary.boundaryNotifications || []).map((entry) =>
      `pending_notification=${entry.source_kind}:${entry.source_role}->${entry.target_role}:${entry.correlation_id || "<none>"}:${entry.summary}`
    )),
  ],
});
