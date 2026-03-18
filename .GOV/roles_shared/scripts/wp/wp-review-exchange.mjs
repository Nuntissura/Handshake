#!/usr/bin/env node

import crypto from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  REVIEW_OPEN_RECEIPT_KIND_VALUES,
  REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
} from "../lib/wp-communications-lib.mjs";
import { appendWpReceipt } from "./wp-receipt-append.mjs";
import { appendWpThreadEntry } from "./wp-thread-append.mjs";
import { appendWpNotification } from "./wp-notification-append.mjs";

const SUPPORTED_RECEIPT_KINDS = [
  ...REVIEW_OPEN_RECEIPT_KIND_VALUES,
  ...REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
];
const EXPLICIT_REVIEW_ROLE_VALUES = ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"];

function fail(message) {
  throw new Error(message);
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function nullableValue(value) {
  const raw = String(value ?? "").trim();
  if (!raw || /^null$/i.test(raw) || /^none$/i.test(raw) || /^n\/a$/i.test(raw)) return null;
  return raw;
}

function inferTargetRole(receiptKind, actorRole) {
  const role = normalizeRole(actorRole);
  if (!EXPLICIT_REVIEW_ROLE_VALUES.includes(role)) return null;
  if (role === "CODER") return "WP_VALIDATOR";
  if (role === "WP_VALIDATOR" || role === "INTEGRATION_VALIDATOR") return "CODER";
  return null;
}

function requiresAck(receiptKind) {
  return REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(receiptKind);
}

function buildCorrelationId(wpId, receiptKind) {
  return `review:${wpId}:${receiptKind.toLowerCase()}:${Date.now().toString(36)}:${crypto.randomBytes(3).toString("hex")}`;
}

function buildTargetLabel(targetRole, targetSession) {
  if (!targetRole) return "";
  return targetSession ? `${targetRole}:${targetSession}` : targetRole;
}

function buildThreadMessage({ receiptKind, summary, specAnchor, packetRowRef, correlationId }) {
  const lines = [`${receiptKind}: ${summary}`];
  if (specAnchor) lines.push(`spec_anchor=${specAnchor}`);
  if (packetRowRef) lines.push(`packet_row_ref=${packetRowRef}`);
  lines.push(`correlation_id=${correlationId}`);
  return lines.join("\n");
}

export function recordReviewExchange({
  receiptKind,
  wpId,
  actorRole,
  actorSession,
  targetRole = null,
  targetSession = null,
  summary,
  correlationId = null,
  specAnchor = null,
  packetRowRef = null,
  ackFor = null,
} = {}) {
  const RECEIPT_KIND = String(receiptKind || "").trim().toUpperCase();
  const WP_ID = String(wpId || "").trim();
  const ACTOR_ROLE = normalizeRole(actorRole);
  const ACTOR_SESSION = String(actorSession || "").trim();
  const SUMMARY = String(summary || "").trim();
  const TARGET_ROLE = normalizeRole(targetRole) || inferTargetRole(RECEIPT_KIND, ACTOR_ROLE);
  const TARGET_SESSION = nullableValue(targetSession);
  const SPEC_ANCHOR = nullableValue(specAnchor);
  const PACKET_ROW_REF = nullableValue(packetRowRef);
  let ACK_FOR = nullableValue(ackFor);

  if (!SUPPORTED_RECEIPT_KINDS.includes(RECEIPT_KIND)) {
    fail(`Unsupported review receipt kind: ${RECEIPT_KIND}`);
  }
  if (!WP_ID || !/^WP-/.test(WP_ID)) fail("WP_ID is required");
  if (!EXPLICIT_REVIEW_ROLE_VALUES.includes(ACTOR_ROLE)) {
    fail(`ACTOR_ROLE must be one of ${EXPLICIT_REVIEW_ROLE_VALUES.join(", ")}`);
  }
  if (!ACTOR_SESSION) fail("ACTOR_SESSION is required");
  if (!SUMMARY) fail("SUMMARY is required");
  if (!TARGET_ROLE || !EXPLICIT_REVIEW_ROLE_VALUES.includes(TARGET_ROLE)) {
    fail(`TARGET_ROLE must resolve to one of ${EXPLICIT_REVIEW_ROLE_VALUES.join(", ")}`);
  }

  const CORRELATION_ID = nullableValue(correlationId)
    || (REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(RECEIPT_KIND) ? buildCorrelationId(WP_ID, RECEIPT_KIND) : null);
  if (!CORRELATION_ID) {
    fail(`CORRELATION_ID is required for ${RECEIPT_KIND}`);
  }
  if (!ACK_FOR && REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(RECEIPT_KIND)) {
    ACK_FOR = CORRELATION_ID;
  }

  const threadResult = appendWpThreadEntry({
    wpId: WP_ID,
    actorRole: ACTOR_ROLE,
    actorSession: ACTOR_SESSION,
    message: buildThreadMessage({
      receiptKind: RECEIPT_KIND,
      summary: SUMMARY,
      specAnchor: SPEC_ANCHOR,
      packetRowRef: PACKET_ROW_REF,
      correlationId: CORRELATION_ID,
    }),
    target: buildTargetLabel(TARGET_ROLE, TARGET_SESSION),
    recordReceipt: false,
    targetRole: TARGET_ROLE,
    targetSession: TARGET_SESSION,
    correlationId: CORRELATION_ID,
    requiresAck: requiresAck(RECEIPT_KIND),
    ackFor: ACK_FOR,
    specAnchor: SPEC_ANCHOR,
    packetRowRef: PACKET_ROW_REF,
  });

  const receiptResult = appendWpReceipt({
    wpId: WP_ID,
    actorRole: ACTOR_ROLE,
    actorSession: ACTOR_SESSION,
    receiptKind: RECEIPT_KIND,
    summary: SUMMARY,
    targetRole: TARGET_ROLE,
    targetSession: TARGET_SESSION,
    correlationId: CORRELATION_ID,
    requiresAck: requiresAck(RECEIPT_KIND),
    ackFor: ACK_FOR,
    specAnchor: SPEC_ANCHOR,
    packetRowRef: PACKET_ROW_REF,
    refs: [threadResult.threadFile],
  });

  appendWpNotification({
    wpId: WP_ID,
    sourceKind: RECEIPT_KIND,
    sourceRole: ACTOR_ROLE,
    sourceSession: ACTOR_SESSION,
    targetRole: TARGET_ROLE,
    targetSession: TARGET_SESSION,
    correlationId: CORRELATION_ID,
    summary: `${RECEIPT_KIND}: ${SUMMARY}`,
    timestamp: receiptResult.entry.timestamp_utc,
  });

  return {
    correlationId: CORRELATION_ID,
    threadFile: threadResult.threadFile,
    receiptsFile: receiptResult.context.receiptsFile,
    runtimeStatusFile: receiptResult.context.runtimeStatusFile,
    receipt: receiptResult.entry,
  };
}

function runCli() {
  const [receiptKind, wpId, actorRole, actorSession, targetRole, targetSession, summary, correlationId, specAnchor, packetRowRef, ackFor] =
    process.argv.slice(2);
  if (!receiptKind || !wpId || !actorRole || !actorSession || !summary) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-review-exchange.mjs"
      + " <RECEIPT_KIND> WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <TARGET_ROLE> <TARGET_SESSION>"
      + " \"<SUMMARY>\" [CORRELATION_ID] [SPEC_ANCHOR] [PACKET_ROW_REF] [ACK_FOR]"
    );
    process.exit(1);
  }

  const result = recordReviewExchange({
    receiptKind,
    wpId,
    actorRole,
    actorSession,
    targetRole,
    targetSession,
    summary,
    correlationId,
    specAnchor,
    packetRowRef,
    ackFor,
  });

  console.log(`[WP_REVIEW_EXCHANGE] appended ${String(receiptKind).trim().toUpperCase()} for ${wpId}`);
  console.log(`- correlation_id: ${result.correlationId}`);
  console.log(`- thread: ${result.threadFile}`);
  console.log(`- receipts: ${result.receiptsFile}`);
  console.log(`- runtime: ${result.runtimeStatusFile}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
