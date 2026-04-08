#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
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
    + ` WP-{ID} [${COMMUNICATION_HEALTH_STAGE_VALUES.join("|")}] [ROLE] [SESSION]`
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
    throw new Error(`Official packet not found: ${normalize(packetPath)}`);
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

export function formatWpCommunicationHealthCheckResult(evaluation) {
  const prefix = evaluation.ok ? "PASS" : "FAIL";
  return [
    `[WP_COMMUNICATION_HEALTH] ${prefix}: ${evaluation.message}`,
    ...(evaluation.details || []).map((detail) => `  - ${detail}`),
    "",
  ].join("\n");
}

export function buildWpCommunicationHealthCheckResult({
  wpId = "",
  stage = "STATUS",
  actorRole = "",
  actorSession = "",
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  const normalizedStage = String(stage || "STATUS").trim().toUpperCase();
  const normalizedRole = String(actorRole || "").trim();
  const normalizedSession = String(actorSession || "").trim();

  if (!normalizedWpId || !/^WP-/.test(normalizedWpId)) {
    throw new Error("WP_ID must start with WP-");
  }
  if (!COMMUNICATION_HEALTH_STAGE_VALUES.includes(normalizedStage)) {
    throw new Error(`stage must be one of ${COMMUNICATION_HEALTH_STAGE_VALUES.join(", ")}`);
  }

  ensureWpCommunications({ wpId: normalizedWpId });

  const context = resolvePacketContext(normalizedWpId);
  const receipts = context.receiptsFile && fs.existsSync(repoPathAbs(context.receiptsFile))
    ? parseJsonlFile(context.receiptsFile)
    : [];
  const runtimeStatus = context.runtimeStatusFile && fs.existsSync(repoPathAbs(context.runtimeStatusFile))
    ? parseJsonFile(context.runtimeStatusFile)
    : { open_review_items: [] };
  const latestReceipt = receipts.at(-1) || null;
  const pendingNotifications = Object.values(checkAllNotifications({ wpId: normalizedWpId })).flatMap((entry) => entry.notifications || []);

  const evaluation = evaluateWpCommunicationHealth({
    wpId: normalizedWpId,
    stage: normalizedStage,
    packetPath: context.packetPath,
    packetContent: context.packetText,
    workflowLane: context.workflowLane,
    packetFormatVersion: context.packetFormatVersion,
    communicationContract: context.communicationContract,
    communicationHealthGate: context.communicationHealthGate,
    receipts,
    runtimeStatus,
    actorRole: normalizedRole,
    actorSession: normalizedSession,
  });

  const statusEvaluation = normalizedStage === "STATUS" || normalizedStage === "STARTUP"
    ? evaluation
    : evaluateWpCommunicationHealth({
      wpId: normalizedWpId,
      stage: "STATUS",
      packetPath: context.packetPath,
      packetContent: context.packetText,
      workflowLane: context.workflowLane,
      packetFormatVersion: context.packetFormatVersion,
      communicationContract: context.communicationContract,
      communicationHealthGate: context.communicationHealthGate,
      receipts,
      runtimeStatus,
      actorRole: normalizedRole,
      actorSession: normalizedSession,
    });
  const boundary = evaluateWpCommunicationBoundary({
    stage: normalizedStage,
    statusEvaluation,
    runtimeStatus,
    latestReceipt,
    pendingNotifications,
  });

  return {
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
  };
}

function printResult(evaluation) {
  process.stdout.write(formatWpCommunicationHealthCheckResult(evaluation));
  process.exit(evaluation.ok ? 0 : 1);
}

export function runWpCommunicationHealthCheckCli(argv = process.argv.slice(2)) {
  const wpId = String(argv[0] || "").trim();
  const stage = String(argv[1] || "STATUS").trim().toUpperCase();
  const actorRole = String(argv[2] || "").trim();
  const actorSession = String(argv[3] || "").trim();
  if (!wpId || !/^WP-/.test(wpId)) usage();
  if (!COMMUNICATION_HEALTH_STAGE_VALUES.includes(stage)) usage();

  try {
    printResult(buildWpCommunicationHealthCheckResult({
      wpId,
      stage,
      actorRole,
      actorSession,
    }));
  } catch (error) {
    console.error(`[WP_COMMUNICATION_HEALTH] FAIL: ${error?.message || String(error || "")}`);
    process.exit(1);
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  runWpCommunicationHealthCheckCli();
}
