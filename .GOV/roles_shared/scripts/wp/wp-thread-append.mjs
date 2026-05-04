#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { isInvokedAsMain } from "../lib/invocation-path-lib.mjs";
import { communicationTransactionLockPathForWp, normalize } from "../lib/wp-communications-lib.mjs";
import { repoPathAbs, workPacketPath } from "../lib/runtime-paths.mjs";
import { withFileLockSync } from "../session/session-registry-lib.mjs";
import { appendWpReceipt } from "./wp-receipt-append.mjs";
import { appendWpNotification, resolveTargetRoleFromMention } from "./wp-notification-append.mjs";

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

function nullableCliString(value) {
  const raw = String(value ?? "").trim();
  if (!raw) return "";
  if (/^(?:null|none|n\/a|false)$/i.test(raw)) return "";
  return raw;
}

const THREAD_CLI_OPTION_FIELDS = [
  "target",
  "targetRole",
  "targetSession",
  "correlationId",
  "requiresAck",
  "ackFor",
  "specAnchor",
  "packetRowRef",
];
const THREAD_CLI_OPTION_FIELD_BY_KEY = new Map([
  ["target", "target"],
  ["targetrole", "targetRole"],
  ["targetsession", "targetSession"],
  ["correlation", "correlationId"],
  ["correlationid", "correlationId"],
  ["requiresack", "requiresAck"],
  ["ackfor", "ackFor"],
  ["specanchor", "specAnchor"],
  ["packetrowref", "packetRowRef"],
]);

function normalizedCliOptionKey(value) {
  return String(value || "").trim().toLowerCase().replace(/[-_]/g, "");
}

function parseNamedCliOption(raw, fieldByKey) {
  const match = String(raw ?? "").match(/^([A-Za-z][A-Za-z0-9_-]*)=(.*)$/s);
  if (!match) return null;
  const outerField = fieldByKey.get(normalizedCliOptionKey(match[1]));
  if (!outerField) return null;
  const innerMatch = String(match[2] ?? "").match(/^([A-Za-z][A-Za-z0-9_-]*)=(.*)$/s);
  if (innerMatch) {
    const innerField = fieldByKey.get(normalizedCliOptionKey(innerMatch[1]));
    if (innerField) return { field: innerField, value: innerMatch[2] };
  }
  return { field: outerField, value: match[2] };
}

export function parseThreadAppendCliArgs(argv = []) {
  const [wpId, actorRole, actorSession, message, ...rawOptions] = argv;
  const parsed = {};
  const positional = [];
  for (const value of rawOptions) {
    const named = parseNamedCliOption(value, THREAD_CLI_OPTION_FIELD_BY_KEY);
    if (named) {
      parsed[named.field] = named.value;
      continue;
    }
    positional.push(String(value ?? ""));
  }

  let positionalIndex = 0;
  for (const field of THREAD_CLI_OPTION_FIELDS) {
    if (parsed[field] !== undefined) continue;
    if (positionalIndex >= positional.length) break;
    parsed[field] = positional[positionalIndex];
    positionalIndex += 1;
  }

  return {
    wpId,
    actorRole,
    actorSession,
    message,
    target: parsed.target,
    targetRole: parsed.targetRole,
    targetSession: parsed.targetSession,
    correlationId: parsed.correlationId,
    requiresAck: parsed.requiresAck,
    ackFor: parsed.ackFor,
    specAnchor: parsed.specAnchor,
    packetRowRef: parsed.packetRowRef,
  };
}

function loadThreadContext(wpId) {
  const packetPath = workPacketPath(wpId);
  const packetAbsPath = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbsPath)) {
    throw new Error(`Official packet not found: ${normalize(packetPath)}`);
  }
  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  const threadFile = parseSingleField(packetText, "WP_THREAD_FILE");
  if (!threadFile) {
    throw new Error(`${normalize(packetPath)} does not declare WP_THREAD_FILE`);
  }
  const threadAbsPath = repoPathAbs(threadFile);
  if (!fs.existsSync(threadAbsPath)) {
    throw new Error(`Thread file missing on disk: ${normalize(threadFile)}`);
  }
  return {
    packetPath: normalize(packetPath),
    packetAbsPath: normalize(packetAbsPath),
    threadFile: normalize(threadFile),
    threadAbsPath: normalize(threadAbsPath),
  };
}

function appendWpThreadEntryCore({
  wpId,
  actorRole,
  actorSession,
  message,
  target = "",
  recordReceipt = true,
  emitNotification = true,
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
  const TARGET = nullableCliString(target);
  const TARGET_ROLE = nullableCliString(targetRole).toUpperCase();
  const TARGET_SESSION = nullableCliString(targetSession);
  const CORRELATION_ID = nullableCliString(correlationId);
  const SPEC_ANCHOR = nullableCliString(specAnchor);
  const PACKET_ROW_REF = nullableCliString(packetRowRef);
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
  fs.appendFileSync(context.threadAbsPath, `${entryLines.join("\n")}\n`, "utf8");

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
    }, { assumeTransactionLock: true });
  }

  const resolvedTargetRole = TARGET_ROLE || resolveTargetRoleFromMention(TARGET);
  if (emitNotification && resolvedTargetRole) {
    appendWpNotification({
      wpId: WP_ID,
      sourceKind: "THREAD_MESSAGE",
      sourceRole: ACTOR_ROLE,
      sourceSession: ACTOR_SESSION,
      targetRole: resolvedTargetRole,
      targetSession: TARGET_SESSION || null,
      correlationId: CORRELATION_ID || null,
      summary: `${ACTOR_ROLE} -> ${TARGET || resolvedTargetRole}: ${bodyLines[0]}`,
      timestamp,
    }, { assumeTransactionLock: true });
  }

  return {
    threadFile: context.threadFile,
    timestamp,
    summary: `${ACTOR_ROLE} -> ${TARGET || "thread"}: ${bodyLines[0]}`,
    receiptAppended: recordReceipt,
  };
}

export function appendWpThreadEntry(args = {}, options = {}) {
  const WP_ID = String(args?.wpId || "").trim();
  const run = () => appendWpThreadEntryCore(args);
  if (options.assumeTransactionLock || !WP_ID || !/^WP-/.test(WP_ID)) {
    return run();
  }
  return withFileLockSync(communicationTransactionLockPathForWp(WP_ID), run);
}

function runCli() {
  const {
    wpId,
    actorRole,
    actorSession,
    message,
    target,
    targetRole,
    targetSession,
    correlationId,
    requiresAck,
    ackFor,
    specAnchor,
    packetRowRef,
  } = parseThreadAppendCliArgs(process.argv.slice(2));
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

if (isInvokedAsMain(import.meta.url, process.argv[1])) {
  runCli();
}
