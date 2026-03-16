#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { normalize } from "../lib/wp-communications-lib.mjs";
import { appendWpReceipt } from "./wp-receipt-append.mjs";

const PACKETS_DIR = path.join(".GOV", "task_packets");

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function normalizeMultilineMessage(message) {
  return String(message || "")
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) => line.trimEnd())
    .filter((line, index, all) => !(index === 0 && line.trim() === "") && !(index === all.length - 1 && line.trim() === ""));
}

function parseBooleanLike(value) {
  const raw = String(value ?? "").trim();
  if (!raw) return false;
  return ["1", "true", "yes", "y"].includes(raw.toLowerCase());
}

function loadThreadContext(wpId) {
  const packetPath = path.join(PACKETS_DIR, `${wpId}.md`);
  if (!fs.existsSync(packetPath)) {
    throw new Error(`Official packet not found: ${normalize(packetPath)}`);
  }
  const packetText = fs.readFileSync(packetPath, "utf8");
  const threadFile = parseSingleField(packetText, "WP_THREAD_FILE");
  if (!threadFile) {
    throw new Error(`${normalize(packetPath)} does not declare WP_THREAD_FILE`);
  }
  if (!fs.existsSync(threadFile)) {
    throw new Error(`Thread file missing on disk: ${normalize(threadFile)}`);
  }
  return { packetPath: normalize(packetPath), threadFile: normalize(threadFile) };
}

export function appendWpThreadEntry({
  wpId,
  actorRole,
  actorSession,
  message,
  target = "",
  recordReceipt = true,
  targetRole = null,
  targetSession = null,
  correlationId = null,
  requiresAck = false,
  ackFor = null,
  specAnchor = null,
  packetRowRef = null,
} = {}) {
  const WP_ID = String(wpId || "").trim();
  const ACTOR_ROLE = String(actorRole || "").trim().toUpperCase();
  const ACTOR_SESSION = String(actorSession || "").trim();
  const TARGET = String(target || "").trim();
  const TARGET_ROLE = String(targetRole || "").trim().toUpperCase();
  const TARGET_SESSION = String(targetSession || "").trim();
  const CORRELATION_ID = String(correlationId || "").trim();
  const SPEC_ANCHOR = String(specAnchor || "").trim();
  const PACKET_ROW_REF = String(packetRowRef || "").trim();
  const bodyLines = normalizeMultilineMessage(message);

  if (!WP_ID || !/^WP-/.test(WP_ID)) throw new Error("WP_ID is required");
  if (!ACTOR_ROLE) throw new Error("ACTOR_ROLE is required");
  if (!ACTOR_SESSION) throw new Error("ACTOR_SESSION is required");
  if (bodyLines.length === 0 || !bodyLines.some((line) => line.trim().length > 0)) {
    throw new Error("message is required");
  }

  const context = loadThreadContext(WP_ID);
  const timestamp = new Date().toISOString();
  const header = [`- ${timestamp}`, ACTOR_ROLE, `session=${ACTOR_SESSION}`];
  if (TARGET) header.push(`target=${TARGET}`);
  if (TARGET_ROLE) header.push(`target_role=${TARGET_ROLE}`);
  if (TARGET_SESSION) header.push(`target_session=${TARGET_SESSION}`);
  if (CORRELATION_ID) header.push(`correlation_id=${CORRELATION_ID}`);
  if (requiresAck) header.push("requires_ack=true");
  if (ackFor) header.push(`ack_for=${ackFor}`);
  if (SPEC_ANCHOR) header.push(`spec_anchor=${SPEC_ANCHOR}`);
  if (PACKET_ROW_REF) header.push(`packet_row_ref=${PACKET_ROW_REF}`);
  const entryLines = [header.join(" | "), ...bodyLines.map((line) => `  ${line}`), ""];
  fs.appendFileSync(context.threadFile, `${entryLines.join("\n")}\n`, "utf8");

  if (recordReceipt) {
    appendWpReceipt({
      wpId: WP_ID,
      actorRole: ACTOR_ROLE,
      actorSession: ACTOR_SESSION,
      receiptKind: "THREAD_MESSAGE",
      summary: `${ACTOR_ROLE} -> ${TARGET || "thread"}: ${bodyLines[0]}`,
      refs: [context.threadFile],
      timestamp,
      targetRole: TARGET_ROLE || null,
      targetSession: TARGET_SESSION || null,
      correlationId: CORRELATION_ID || null,
      requiresAck,
      ackFor,
      specAnchor: SPEC_ANCHOR || null,
      packetRowRef: PACKET_ROW_REF || null,
    });
  }

  return {
    threadFile: context.threadFile,
    timestamp,
    summary: `${ACTOR_ROLE} -> ${TARGET || "thread"}: ${bodyLines[0]}`,
    receiptAppended: recordReceipt,
  };
}

function runCli() {
  const [wpId, actorRole, actorSession, message, target, targetRole, targetSession, correlationId, requiresAck, ackFor, specAnchor, packetRowRef] = process.argv.slice(2);
  if (!wpId || !actorRole || !actorSession || !message) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-thread-append.mjs"
      + " WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> \"<message>\" [TARGET]"
      + " [TARGET_ROLE] [TARGET_SESSION] [CORRELATION_ID] [REQUIRES_ACK] [ACK_FOR] [SPEC_ANCHOR] [PACKET_ROW_REF]"
    );
    process.exit(1);
  }

  const result = appendWpThreadEntry({
    wpId,
    actorRole,
    actorSession,
    message,
    target,
    targetRole,
    targetSession,
    correlationId,
    requiresAck: parseBooleanLike(requiresAck),
    ackFor,
    specAnchor,
    packetRowRef,
  });
  console.log(`[WP_THREAD] appended message for ${wpId}`);
  console.log(`- thread: ${result.threadFile}`);
  console.log(`- timestamp_utc: ${result.timestamp}`);
  console.log(`- summary: ${result.summary}`);
  console.log(`- receipt_appended: ${result.receiptAppended ? 'YES' : 'NO'}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
