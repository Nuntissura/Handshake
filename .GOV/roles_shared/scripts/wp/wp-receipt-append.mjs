#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  deriveAuthorityKinds,
  normalize,
  parseJsonFile,
  validateReceipt,
} from "../lib/wp-communications-lib.mjs";

const PACKETS_DIR = path.join(".GOV", "task_packets");

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function nullableValue(value) {
  const raw = String(value ?? "").trim();
  if (!raw || /^null$/i.test(raw) || /^none$/i.test(raw) || /^n\/a$/i.test(raw)) return null;
  return raw;
}

function loadPacketContext(wpId) {
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
    refs: [context.packetPath, ...refs.filter(Boolean).map((value) => normalize(value))],
  };

  if (context.runtimeStatusFile && !entry.refs.includes(context.runtimeStatusFile)) entry.refs.push(context.runtimeStatusFile);
  if (context.threadFile && !entry.refs.includes(context.threadFile)) entry.refs.push(context.threadFile);
  if (!entry.refs.includes(context.receiptsFile)) entry.refs.push(context.receiptsFile);

  const errors = validateReceipt(entry);
  if (errors.length > 0) {
    throw new Error(`Receipt validation failed: ${errors.join("; ")}`);
  }

  fs.appendFileSync(context.receiptsFile, `${JSON.stringify(entry)}\n`, "utf8");
  return { context, entry };
}

function runCli() {
  const [wpId, actorRole, actorSession, receiptKind, summary, stateBefore, stateAfter] = process.argv.slice(2);
  if (!wpId || !actorRole || !actorSession || !receiptKind || !summary) {
    console.error("Usage: node .GOV/roles_shared/scripts/wp/wp-receipt-append.mjs WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <RECEIPT_KIND> \"<SUMMARY>\" [STATE_BEFORE] [STATE_AFTER]");
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
  });

  console.log(`[WP_RECEIPT] appended ${entry.receipt_kind} for ${entry.wp_id}`);
  console.log(`- ledger: ${context.receiptsFile}`);
  console.log(`- timestamp_utc: ${entry.timestamp_utc}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}

