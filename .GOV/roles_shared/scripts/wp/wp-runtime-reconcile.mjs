#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { addMinutes, parseJsonFile, validateReceipt, validateRuntimeStatus } from "../lib/wp-communications-lib.mjs";
import { appendWpReceipt, loadPacketContext, updateOpenReviewItems } from "./wp-receipt-append.mjs";

function loadJsonl(filePath) {
  if (!fs.existsSync(filePath)) return [];
  return fs.readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function parsePacketStatus(packetText) {
  return (
    (packetText.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (packetText.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    ""
  ).trim() || "Ready for Dev";
}

function parseIntegerField(packetText, label, fallback) {
  const match = packetText.match(new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi"));
  const raw = match ? String(match[1] || "").trim() : "";
  const parsed = Number.parseInt(raw, 10);
  return Number.isInteger(parsed) ? parsed : fallback;
}

export function reconcileWpRuntime(wpId, options = {}) {
  const context = loadPacketContext(wpId);
  if (!context.runtimeStatusFile || !fs.existsSync(context.runtimeStatusFile)) {
    throw new Error(`Runtime status file missing on disk: ${context.runtimeStatusFile || "<missing>"}`);
  }

  const packetText = fs.readFileSync(context.packetPath, "utf8");
  const runtimeStatus = parseJsonFile(context.runtimeStatusFile);
  const stateBefore = `${runtimeStatus.runtime_status}/${runtimeStatus.current_phase}`;
  const receipts = loadJsonl(context.receiptsFile);
  runtimeStatus.open_review_items = [];
  for (const entry of receipts) {
    const errors = validateReceipt(entry);
    if (errors.length > 0) {
      throw new Error(`Receipt validation failed during reconcile for ${entry.wp_id || "<unknown>"}: ${errors.join("; ")}`);
    }
    updateOpenReviewItems(runtimeStatus, entry);
  }

  if (options.finalize === true) {
    const now = new Date().toISOString();
    const heartbeatIntervalMinutes = parseIntegerField(packetText, "HEARTBEAT_INTERVAL_MINUTES", runtimeStatus.heartbeat_interval_minutes || 15);
    const staleAfterMinutes = parseIntegerField(packetText, "STALE_AFTER_MINUTES", 45);
    runtimeStatus.current_packet_status = parsePacketStatus(packetText);
    runtimeStatus.runtime_status = "completed";
    runtimeStatus.current_phase = "CLOSEOUT_COMPLETE";
    runtimeStatus.next_expected_actor = "NONE";
    runtimeStatus.next_expected_session = null;
    runtimeStatus.waiting_on = String(options.waitingOn || "final packet PASS recorded on canonical handshake_main");
    runtimeStatus.waiting_on_session = null;
    runtimeStatus.validator_trigger = "NONE";
    runtimeStatus.validator_trigger_reason = null;
    runtimeStatus.attention_required = false;
    runtimeStatus.ready_for_validation = false;
    runtimeStatus.ready_for_validation_reason = null;
    runtimeStatus.active_role_sessions = [];
    runtimeStatus.last_event = "runtime_reconcile_closeout";
    runtimeStatus.last_event_at = now;
    runtimeStatus.last_heartbeat_at = now;
    runtimeStatus.heartbeat_due_at = addMinutes(now, heartbeatIntervalMinutes);
    runtimeStatus.stale_after = addMinutes(now, staleAfterMinutes);
  }

  const runtimeErrors = validateRuntimeStatus(runtimeStatus);
  if (runtimeErrors.length > 0) {
    throw new Error(`Runtime status validation failed after reconcile: ${runtimeErrors.join("; ")}`);
  }

  fs.writeFileSync(context.runtimeStatusFile, `${JSON.stringify(runtimeStatus, null, 2)}\n`, "utf8");

  if (options.finalize === true) {
    appendWpReceipt({
      wpId,
      actorRole: "ORCHESTRATOR",
      actorSession: String(options.actorSession || "wt-orchestrator"),
      receiptKind: "THREAD_MESSAGE",
      summary: String(options.receiptSummary || `ORCHESTRATOR -> ALL: runtime reconciled to completed after canonical PASS closeout. ${runtimeStatus.waiting_on}`),
      stateBefore,
      stateAfter: `${runtimeStatus.runtime_status}/${runtimeStatus.current_phase}`,
      targetRole: null,
      targetSession: null,
      branch: runtimeStatus.current_branch || null,
      worktreeDir: runtimeStatus.current_worktree_dir || null,
    });
  }

  return {
    wpId,
    runtimeStatusFile: context.runtimeStatusFile,
    openReviewItems: runtimeStatus.open_review_items,
    openReviewItemCount: runtimeStatus.open_review_items.length,
    runtimeStatus,
  };
}

function runCli() {
  const args = process.argv.slice(2);
  const wpId = String(args[0] || "").trim();
  if (!wpId || !/^WP-/.test(wpId)) {
    console.error("Usage: node .GOV/roles_shared/scripts/wp/wp-runtime-reconcile.mjs WP-{ID} [--finalize \"<waiting_on_summary>\"]");
    process.exit(1);
  }

  const finalizeIndex = args.indexOf("--finalize");
  const finalize = finalizeIndex >= 0;
  const waitingOn = finalize ? String(args[finalizeIndex + 1] || "").trim() : "";
  const result = reconcileWpRuntime(wpId, {
    finalize,
    waitingOn: waitingOn || null,
  });
  console.log(`[WP_RUNTIME_RECONCILE] ${result.wpId}`);
  console.log(`- runtime_status_file: ${result.runtimeStatusFile}`);
  console.log(`- runtime_status: ${result.runtimeStatus.runtime_status}/${result.runtimeStatus.current_phase}`);
  console.log(`- open_review_items: ${result.openReviewItemCount}`);
  for (const item of result.openReviewItems) {
    const target = item.target_session ? `${item.target_role}:${item.target_session}` : item.target_role;
    console.log(`  - ${item.receipt_kind} ${item.correlation_id} -> ${target}`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
