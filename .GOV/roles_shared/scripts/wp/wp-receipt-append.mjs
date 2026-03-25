#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  communicationTransactionLockPathForWp,
  deriveAuthorityKinds,
  normalize,
  normalizeWorkflowInvalidityCode,
  parseJsonFile,
  parseJsonlFile,
  REVIEW_OPEN_RECEIPT_KIND_VALUES,
  REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
  REVIEW_TRACKED_RECEIPT_KIND_VALUES,
  validateReceipt,
  validateRuntimeStatus,
  WORKFLOW_INVALIDITY_RECEIPT_KIND,
} from "../lib/wp-communications-lib.mjs";
import {
  deriveWpCommunicationAutoRoute,
  evaluateWpCommunicationHealth,
} from "../lib/wp-communication-health-lib.mjs";
import { workPacketPath } from "../lib/runtime-paths.mjs";
import { appendJsonlLine, withFileLockSync, writeJsonFile } from "../session/session-registry-lib.mjs";
import { appendWpNotification } from "./wp-notification-append.mjs";

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function nullableValue(value) {
  const raw = String(value ?? "").trim();
  if (!raw || /^null$/i.test(raw) || /^none$/i.test(raw) || /^n\/a$/i.test(raw) || /^false$/i.test(raw)) return null;
  return raw;
}

function parseBooleanLike(value) {
  const raw = String(value ?? "").trim();
  if (!raw) return false;
  return ["1", "true", "yes", "y"].includes(raw.toLowerCase());
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  return raw || null;
}

function sameRouteTarget(leftRole, leftSession, rightRole, rightSession) {
  return normalizeRole(leftRole) === normalizeRole(rightRole)
    && normalizeSession(leftSession) === normalizeSession(rightSession);
}

function updateOpenReviewItems(runtimeStatus, entry) {
  if (!runtimeStatus || typeof runtimeStatus !== "object") return;
  const currentItems = Array.isArray(runtimeStatus.open_review_items) ? runtimeStatus.open_review_items : [];
  const correlationId = String(entry.correlation_id || "").trim();
  if (!correlationId) {
    runtimeStatus.open_review_items = currentItems;
    return;
  }

  const withoutCorrelation = currentItems.filter((item) => String(item?.correlation_id || "").trim() !== correlationId);
  if (REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) {
    withoutCorrelation.push({
      correlation_id: correlationId,
      receipt_kind: entry.receipt_kind,
      summary: entry.summary,
      opened_by_role: entry.actor_role,
      opened_by_session: entry.actor_session,
      target_role: entry.target_role,
      target_session: entry.target_session ?? null,
      spec_anchor: entry.spec_anchor ?? null,
      packet_row_ref: entry.packet_row_ref ?? null,
      requires_ack: entry.requires_ack,
      opened_at: entry.timestamp_utc,
      updated_at: entry.timestamp_utc,
    });
  } else if (REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) {
    // Resolution receipts close the matching open review item.
  } else {
    runtimeStatus.open_review_items = currentItems;
    return;
  }

  runtimeStatus.open_review_items = withoutCorrelation.sort((left, right) =>
    String(left.opened_at || "").localeCompare(String(right.opened_at || ""))
  );
}

function loadPacketContext(wpId) {
  const packetPath = workPacketPath(wpId);
  if (!fs.existsSync(packetPath)) {
    throw new Error(`Official packet not found: ${normalize(packetPath)}`);
  }
  const packetText = fs.readFileSync(packetPath, "utf8");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const threadFile = parseSingleField(packetText, "WP_THREAD_FILE");
  const branch = parseSingleField(packetText, "LOCAL_BRANCH") || null;
  const worktreeDir = parseSingleField(packetText, "LOCAL_WORKTREE_DIR") || null;

  if (!receiptsFile) {
    throw new Error(`${normalize(packetPath)} does not declare WP_RECEIPTS_FILE`);
  }
  if (!fs.existsSync(receiptsFile)) {
    throw new Error(`Receipts ledger missing on disk: ${normalize(receiptsFile)}`);
  }

  return {
    packetPath: normalize(packetPath),
    receiptsFile: normalize(receiptsFile),
    runtimeStatusFile: normalize(runtimeStatusFile),
    threadFile: normalize(threadFile),
    branch: branch ? normalize(branch) : null,
    worktreeDir: worktreeDir ? normalize(worktreeDir) : null,
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE") || "",
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION") || "",
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT") || "",
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE") || "",
  };
}

function appendReviewNotifications({ wpId, entry, autoRoute }) {
  const targets = [];
  const explicitTargetRole = normalizeRole(entry?.target_role);
  const explicitTargetSession = normalizeSession(entry?.target_session);

  if (explicitTargetRole && explicitTargetRole !== normalizeRole(entry?.actor_role)) {
    targets.push({
      targetRole: explicitTargetRole,
      targetSession: explicitTargetSession,
      sourceKind: entry.receipt_kind,
      summary: `${entry.receipt_kind}: ${entry.summary}`,
    });
  }

  if (autoRoute?.notification?.targetRole) {
    const routeTargetRole = normalizeRole(autoRoute.notification.targetRole);
    const routeTargetSession = normalizeSession(autoRoute.notification.targetSession);
    const duplicatesExplicit = sameRouteTarget(routeTargetRole, routeTargetSession, explicitTargetRole, explicitTargetSession);
    if (!duplicatesExplicit && routeTargetRole !== normalizeRole(entry?.actor_role)) {
      targets.push({
        targetRole: routeTargetRole,
        targetSession: routeTargetSession,
        sourceKind: "AUTO_ROUTE",
        summary: autoRoute.notification.summary,
      });
    }
  }

  for (const target of targets) {
    appendWpNotification({
      wpId,
      sourceKind: target.sourceKind,
      sourceRole: entry.actor_role,
      sourceSession: entry.actor_session,
      targetRole: target.targetRole,
      targetSession: target.targetSession,
      correlationId: entry.correlation_id ?? null,
      summary: target.summary,
      timestamp: entry.timestamp_utc,
    }, { assumeTransactionLock: true });
  }
}

function appendWpReceiptCore({
  wpId,
  actorRole,
  actorSession,
  receiptKind,
  summary,
  stateBefore = null,
  stateAfter = null,
  refs = [],
  branch = null,
  worktreeDir = null,
  timestamp = null,
  targetRole = null,
  targetSession = null,
  correlationId = null,
  requiresAck = false,
  ackFor = null,
  specAnchor = null,
  packetRowRef = null,
  workflowInvalidityCode = null,
} = {}) {
  const WP_ID = String(wpId || "").trim();
  if (!WP_ID || !/^WP-/.test(WP_ID)) {
    throw new Error("WP_ID is required");
  }

  const context = loadPacketContext(WP_ID);
  const runtimeStatus = context.runtimeStatusFile && fs.existsSync(context.runtimeStatusFile)
    ? parseJsonFile(context.runtimeStatusFile)
    : null;
  const reviewTrackedReceipt = REVIEW_TRACKED_RECEIPT_KIND_VALUES.includes(String(receiptKind || "").trim().toUpperCase());
  const { authorityKind, validatorRoleKind } = deriveAuthorityKinds({
    actorRole,
    actorSession,
    runtimeStatus,
  });
  const entry = {
    schema_version: "wp_receipt@1",
    timestamp_utc: String(timestamp || new Date().toISOString()),
    wp_id: WP_ID,
    actor_role: String(actorRole || "").trim().toUpperCase(),
    actor_session: String(actorSession || "").trim(),
    actor_authority_kind: authorityKind,
    validator_role_kind: validatorRoleKind,
    receipt_kind: String(receiptKind || "").trim().toUpperCase(),
    summary: String(summary || "").trim(),
    branch: branch === undefined ? context.branch : nullableValue(branch),
    worktree_dir: worktreeDir === undefined ? context.worktreeDir : nullableValue(worktreeDir),
    state_before: nullableValue(stateBefore),
    state_after: nullableValue(stateAfter),
    target_role: nullableValue(targetRole),
    target_session: nullableValue(targetSession),
    correlation_id: nullableValue(correlationId),
    requires_ack: Boolean(requiresAck),
    ack_for: nullableValue(ackFor),
    spec_anchor: nullableValue(specAnchor),
    packet_row_ref: nullableValue(packetRowRef),
    workflow_invalidity_code: nullableValue(normalizeWorkflowInvalidityCode(workflowInvalidityCode)),
    refs: [context.packetPath, ...refs.filter(Boolean).map((value) => normalize(value))],
  };

  if (context.runtimeStatusFile && !entry.refs.includes(context.runtimeStatusFile)) entry.refs.push(context.runtimeStatusFile);
  if (context.threadFile && !entry.refs.includes(context.threadFile)) entry.refs.push(context.threadFile);
  if (!entry.refs.includes(context.receiptsFile)) entry.refs.push(context.receiptsFile);

  const errors = validateReceipt(entry);
  if (errors.length > 0) {
    throw new Error(`Receipt validation failed: ${errors.join("; ")}`);
  }

  let autoRoute = null;
  if (runtimeStatus) {
    updateOpenReviewItems(runtimeStatus, entry);
    runtimeStatus.last_event = `receipt_${entry.receipt_kind.toLowerCase()}`;
    runtimeStatus.last_event_at = entry.timestamp_utc;
    if (entry.receipt_kind === WORKFLOW_INVALIDITY_RECEIPT_KIND) {
      runtimeStatus.next_expected_actor = "ORCHESTRATOR";
      runtimeStatus.next_expected_session = null;
      runtimeStatus.waiting_on = entry.workflow_invalidity_code
        ? `WORKFLOW_INVALIDITY_${entry.workflow_invalidity_code}`
        : "WORKFLOW_INVALIDITY";
      runtimeStatus.waiting_on_session = null;
      runtimeStatus.validator_trigger = "NONE";
      runtimeStatus.validator_trigger_reason = entry.workflow_invalidity_code
        ? `Workflow invalidity flagged: ${entry.workflow_invalidity_code}`
        : "Workflow invalidity flagged";
      runtimeStatus.attention_required = true;
      runtimeStatus.ready_for_validation = false;
      runtimeStatus.ready_for_validation_reason = null;
      if (!["completed", "failed", "canceled"].includes(String(runtimeStatus.runtime_status || "").trim().toLowerCase())) {
        runtimeStatus.runtime_status = "input_required";
      }
    } else if (reviewTrackedReceipt) {
      const receipts = parseJsonlFile(context.receiptsFile);
      const evaluation = evaluateWpCommunicationHealth({
        wpId: WP_ID,
        stage: "STATUS",
        packetPath: context.packetPath,
        workflowLane: context.workflowLane || runtimeStatus.workflow_lane || "",
        packetFormatVersion: context.packetFormatVersion || "",
        communicationContract: context.communicationContract || "",
        communicationHealthGate: context.communicationHealthGate || "",
        receipts: [...receipts, entry],
        runtimeStatus,
      });
      autoRoute = deriveWpCommunicationAutoRoute({
        evaluation,
        runtimeStatus,
        latestReceipt: entry,
      });
      if (autoRoute.applicable) {
        runtimeStatus.next_expected_actor = autoRoute.nextExpectedActor;
        runtimeStatus.next_expected_session = autoRoute.nextExpectedSession;
        runtimeStatus.waiting_on = autoRoute.waitingOn;
        runtimeStatus.waiting_on_session = autoRoute.waitingOnSession;
        runtimeStatus.validator_trigger = autoRoute.validatorTrigger;
        runtimeStatus.validator_trigger_reason = autoRoute.validatorTriggerReason;
        runtimeStatus.ready_for_validation = autoRoute.readyForValidation;
        runtimeStatus.ready_for_validation_reason = autoRoute.readyForValidationReason;
        runtimeStatus.attention_required = autoRoute.attentionRequired;
      }
    }
    const runtimeErrors = validateRuntimeStatus(runtimeStatus);
    if (runtimeErrors.length > 0) {
      throw new Error(`Runtime status validation failed after receipt append: ${runtimeErrors.join("; ")}`);
    }
    writeJsonFile(context.runtimeStatusFile, runtimeStatus);
  }

  appendJsonlLine(context.receiptsFile, entry);
  if (reviewTrackedReceipt) {
    appendReviewNotifications({ wpId: WP_ID, entry, autoRoute });
  }
  return { context, entry };
}

export function appendWpReceipt(args = {}, options = {}) {
  const WP_ID = String(args?.wpId || "").trim();
  const run = () => appendWpReceiptCore(args);
  if (options.assumeTransactionLock || !WP_ID || !/^WP-/.test(WP_ID)) {
    return run();
  }
  return withFileLockSync(communicationTransactionLockPathForWp(WP_ID), run);
}

function runCli() {
  const [wpId, actorRole, actorSession, receiptKind, summary, stateBefore, stateAfter, targetRole, targetSession, correlationId, requiresAck, ackFor, specAnchor, packetRowRef, workflowInvalidityCode] = process.argv.slice(2);
  if (!wpId || !actorRole || !actorSession || !receiptKind || !summary) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-receipt-append.mjs"
      + " WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <RECEIPT_KIND> \"<SUMMARY>\""
      + " [STATE_BEFORE] [STATE_AFTER] [TARGET_ROLE] [TARGET_SESSION] [CORRELATION_ID] [REQUIRES_ACK] [ACK_FOR] [SPEC_ANCHOR] [PACKET_ROW_REF] [WORKFLOW_INVALIDITY_CODE]"
    );
    process.exit(1);
  }

  const { context, entry } = appendWpReceipt({
    wpId,
    actorRole,
    actorSession,
    receiptKind,
    summary,
    stateBefore,
    stateAfter,
    targetRole,
    targetSession,
    correlationId,
    requiresAck: parseBooleanLike(requiresAck),
    ackFor,
    specAnchor,
    packetRowRef,
    workflowInvalidityCode,
  });

  console.log(`[WP_RECEIPT] appended ${entry.receipt_kind} for ${entry.wp_id}`);
  console.log(`- ledger: ${context.receiptsFile}`);
  console.log(`- timestamp_utc: ${entry.timestamp_utc}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
