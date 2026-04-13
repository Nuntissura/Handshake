#!/usr/bin/env node

import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  deriveAuthorityKinds,
  ensurePacketlessWpCommunicationScaffold,
  normalize,
  validateReceipt,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { appendWpNotificationCore } from "../../../roles_shared/scripts/wp/wp-notification-append.mjs";
import { appendJsonlLine } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  closeDb as closeMemoryDb,
  extractMemoryFromReceipt,
  openGovernanceMemoryDb,
} from "../../../roles_shared/scripts/memory/governance-memory-lib.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..");
const REPORT_REF = normalize("../gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md");
const MEMORY_SIGNAL_RECEIPT_KIND_VALUES = new Set(["MEMORY_PROPOSAL", "MEMORY_FLAG", "MEMORY_RGF_CANDIDATE"]);
const DEFAULT_TARGET_ROLE = "ORCHESTRATOR";
const STATE_AFTER_BY_KIND = Object.freeze({
  MEMORY_PROPOSAL: "PROPOSAL_WRITTEN",
  MEMORY_FLAG: "FLAG_RECORDED",
  MEMORY_RGF_CANDIDATE: "RGF_CANDIDATE_DRAFTED",
});

function normalizeOptional(value) {
  const raw = String(value ?? "").trim();
  return raw ? raw : null;
}

function normalizeRef(value) {
  const raw = String(value ?? "").trim();
  if (!raw) return null;
  if (path.isAbsolute(raw)) {
    return normalize(path.relative(REPO_ROOT, raw));
  }
  return normalize(raw);
}

function buildReceiptEntry({
  wpId,
  actorSession,
  receiptKind,
  summary,
  backupRef = null,
  correlationId = null,
  targetRole = DEFAULT_TARGET_ROLE,
} = {}) {
  const normalizedKind = String(receiptKind || "").trim().toUpperCase();
  if (!MEMORY_SIGNAL_RECEIPT_KIND_VALUES.has(normalizedKind)) {
    throw new Error(`RECEIPT_KIND must be one of ${Array.from(MEMORY_SIGNAL_RECEIPT_KIND_VALUES).join(", ")}`);
  }

  const { authorityKind, validatorRoleKind } = deriveAuthorityKinds({
    actorRole: "MEMORY_MANAGER",
    actorSession,
    runtimeStatus: null,
  });
  const normalizedBackupRef = normalizeRef(backupRef);
  const refs = [REPORT_REF, normalizedBackupRef].filter(Boolean);

  return {
    schema_version: "wp_receipt@1",
    timestamp_utc: new Date().toISOString(),
    wp_id: String(wpId || "").trim(),
    actor_role: "MEMORY_MANAGER",
    actor_session: String(actorSession || "").trim(),
    actor_authority_kind: authorityKind,
    validator_role_kind: validatorRoleKind,
    receipt_kind: normalizedKind,
    summary: String(summary || "").trim(),
    branch: "gov_kernel",
    worktree_dir: ".",
    state_before: null,
    state_after: STATE_AFTER_BY_KIND[normalizedKind] || null,
    target_role: normalizeOptional(targetRole)?.toUpperCase() || DEFAULT_TARGET_ROLE,
    target_session: null,
    correlation_id: normalizeOptional(correlationId),
    requires_ack: false,
    ack_for: null,
    spec_anchor: null,
    packet_row_ref: null,
    refs,
  };
}

export function appendMemoryManagerReceipt({
  wpId,
  actorSession,
  receiptKind,
  summary,
  backupRef = null,
  correlationId = null,
  targetRole = DEFAULT_TARGET_ROLE,
  skipMemoryExtraction = false,
} = {}) {
  const entry = buildReceiptEntry({
    wpId,
    actorSession,
    receiptKind,
    summary,
    backupRef,
    correlationId,
    targetRole,
  });
  const errors = validateReceipt(entry);
  if (errors.length > 0) {
    throw new Error(`Receipt validation failed: ${errors.join("; ")}`);
  }

  const scaffold = ensurePacketlessWpCommunicationScaffold(entry.wp_id, {
    threadHeading: `# WP Thread: ${entry.wp_id}`,
    noteLines: [
      "Synthetic packetless governed communication lane for Memory Manager proposals and flags.",
    ],
  });

  appendJsonlLine(repoPathAbs(scaffold.receiptsFile), entry);
  const notification = appendWpNotificationCore({
    wpId: entry.wp_id,
    sourceKind: entry.receipt_kind,
    sourceRole: entry.actor_role,
    sourceSession: entry.actor_session,
    targetRole: entry.target_role,
    targetSession: entry.target_session,
    correlationId: entry.correlation_id,
    summary: `${entry.receipt_kind}: ${entry.summary}`,
    timestamp: entry.timestamp_utc,
  });

  if (!skipMemoryExtraction) {
    try {
      const { db } = openGovernanceMemoryDb();
      try {
        extractMemoryFromReceipt(db, entry.wp_id, entry);
      } finally {
        closeMemoryDb(db);
      }
    } catch {
      // Best-effort: receipt emission must remain the primary success path.
    }
  }

  return {
    entry,
    notification,
    scaffold,
  };
}

function runCli() {
  const [wpId, actorSession, receiptKind, summary, backupRef = "", correlationId = "", targetRole = DEFAULT_TARGET_ROLE] = process.argv.slice(2);
  if (!wpId || !actorSession || !receiptKind || !summary) {
    console.error(
      "Usage: node .GOV/roles/memory_manager/scripts/memory-manager-receipt.mjs"
      + " WP-{ID} <ACTOR_SESSION> <MEMORY_PROPOSAL|MEMORY_FLAG|MEMORY_RGF_CANDIDATE> \"<SUMMARY>\""
      + " [BACKUP_REF] [CORRELATION_ID] [TARGET_ROLE]"
    );
    process.exit(1);
  }

  const result = appendMemoryManagerReceipt({
    wpId,
    actorSession,
    receiptKind,
    summary,
    backupRef,
    correlationId,
    targetRole,
  });

  console.log(`[MEMORY_MANAGER_RECEIPT] appended ${result.entry.receipt_kind} for ${result.entry.wp_id}`);
  console.log(`- actor: ${result.entry.actor_role}:${result.entry.actor_session}`);
  console.log(`- target: ${result.entry.target_role}`);
  console.log(`- receipts_file: ${result.scaffold.receiptsFile}`);
  console.log(`- notifications_file: ${result.scaffold.notificationsFile}`);
  if (result.entry.refs.length > 0) {
    console.log(`- refs: ${result.entry.refs.join(" | ")}`);
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) runCli();
