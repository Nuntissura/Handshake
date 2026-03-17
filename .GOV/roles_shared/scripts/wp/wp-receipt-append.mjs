#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  deriveAuthorityKinds,
  normalize,
  parseJsonFile,
  REVIEW_OPEN_RECEIPT_KIND_VALUES,
  REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
  validateReceipt,
  validateRuntimeStatus,
} from "../lib/wp-communications-lib.mjs";

const PACKETS_DIR = path.join(".GOV", "task_packets");

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

function trackedCorrelationIds(entry) {
  const ids = new Set();
  for (const raw of [entry?.correlation_id, entry?.ack_for]) {
    const value = String(raw || "").trim();
    if (value) ids.add(value);
  }
  return ids;
}

export function updateOpenReviewItems(runtimeStatus, entry) {
  if (!runtimeStatus || typeof runtimeStatus !== "object") return;
  const currentItems = Array.isArray(runtimeStatus.open_review_items) ? runtimeStatus.open_review_items : [];
  const correlationIds = trackedCorrelationIds(entry);
  if (correlationIds.size === 0) {
    runtimeStatus.open_review_items = [...currentItems].sort((left, right) =>
      String(left.opened_at || "").localeCompare(String(right.opened_at || ""))
    );
    return;
  }

  const withoutCorrelation = currentItems.filter((item) => !correlationIds.has(String(item?.correlation_id || "").trim()));
  if (REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) {
    const correlationId = String(entry.correlation_id || "").trim();
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

export function loadPacketContext(wpId) {
  const packetPath = path.join(PACKETS_DIR, `${wpId}.md`);
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
  };
}

export function appendWpReceipt({
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
} = {}) {
  const WP_ID = String(wpId || "").trim();
  if (!WP_ID || !/^WP-/.test(WP_ID)) {
    throw new Error("WP_ID is required");
  }

  const context = loadPacketContext(WP_ID);
  const runtimeStatus = context.runtimeStatusFile && fs.existsSync(context.runtimeStatusFile)
    ? parseJsonFile(context.runtimeStatusFile)
    : null;
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
    refs: [context.packetPath, ...refs.filter(Boolean).map((value) => normalize(value))],
  };

  if (context.runtimeStatusFile && !entry.refs.includes(context.runtimeStatusFile)) entry.refs.push(context.runtimeStatusFile);
  if (context.threadFile && !entry.refs.includes(context.threadFile)) entry.refs.push(context.threadFile);
  if (!entry.refs.includes(context.receiptsFile)) entry.refs.push(context.receiptsFile);

  const errors = validateReceipt(entry);
  if (errors.length > 0) {
    throw new Error(`Receipt validation failed: ${errors.join("; ")}`);
  }

  if (runtimeStatus) {
    updateOpenReviewItems(runtimeStatus, entry);
    runtimeStatus.last_event = `receipt_${entry.receipt_kind.toLowerCase()}`;
    runtimeStatus.last_event_at = entry.timestamp_utc;
    const runtimeErrors = validateRuntimeStatus(runtimeStatus);
    if (runtimeErrors.length > 0) {
      throw new Error(`Runtime status validation failed after receipt append: ${runtimeErrors.join("; ")}`);
    }
    fs.writeFileSync(context.runtimeStatusFile, `${JSON.stringify(runtimeStatus, null, 2)}\n`, "utf8");
  }

  fs.appendFileSync(context.receiptsFile, `${JSON.stringify(entry)}\n`, "utf8");
  return { context, entry };
}

function runCli() {
  const [wpId, actorRole, actorSession, receiptKind, summary, stateBefore, stateAfter, targetRole, targetSession, correlationId, requiresAck, ackFor, specAnchor, packetRowRef] = process.argv.slice(2);
  if (!wpId || !actorRole || !actorSession || !receiptKind || !summary) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-receipt-append.mjs"
      + " WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <RECEIPT_KIND> \"<SUMMARY>\""
      + " [STATE_BEFORE] [STATE_AFTER] [TARGET_ROLE] [TARGET_SESSION] [CORRELATION_ID] [REQUIRES_ACK] [ACK_FOR] [SPEC_ANCHOR] [PACKET_ROW_REF]"
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
  });

  console.log(`[WP_RECEIPT] appended ${entry.receipt_kind} for ${entry.wp_id}`);
  console.log(`- ledger: ${context.receiptsFile}`);
  console.log(`- timestamp_utc: ${entry.timestamp_utc}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
