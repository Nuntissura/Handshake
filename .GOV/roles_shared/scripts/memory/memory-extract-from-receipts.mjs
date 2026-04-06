#!/usr/bin/env node
/**
 * RGF-121: Extract episodic and procedural memories from WP RECEIPTS.jsonl.
 *
 * Usage:
 *   node memory-extract-from-receipts.mjs <WP_ID>
 *   node memory-extract-from-receipts.mjs --all
 *
 * Extracts high-signal receipts into the governance memory database:
 *   - CODER_HANDOFF, VALIDATOR_REVIEW, REVIEW_RESPONSE → episodic
 *   - STEERING, REPAIR, WORKFLOW_INVALIDITY → episodic (high importance)
 *   - REPAIR with state_before/state_after → procedural (fix pattern)
 */

import fs from "node:fs";
import path from "node:path";
import {
  communicationPathsForWp,
  parseJsonlFile,
  COMM_ROOT,
} from "../lib/wp-communications-lib.mjs";
import { repoPathAbs } from "../lib/runtime-paths.mjs";
import {
  openGovernanceMemoryDb,
  closeDb,
  addMemory,
} from "./governance-memory-lib.mjs";

const HIGH_SIGNAL_RECEIPT_KINDS = new Set([
  "CODER_INTENT",
  "CODER_HANDOFF",
  "VALIDATOR_KICKOFF",
  "VALIDATOR_REVIEW",
  "VALIDATOR_RESPONSE",
  "REVIEW_REQUEST",
  "REVIEW_RESPONSE",
  "STEERING",
  "REPAIR",
  "WORKFLOW_INVALIDITY",
  "SPEC_GAP",
  "SPEC_CONFIRMATION",
]);

const HIGH_IMPORTANCE_KINDS = new Set([
  "STEERING",
  "REPAIR",
  "WORKFLOW_INVALIDITY",
  "SPEC_GAP",
]);

function extractFileScope(receipt) {
  const mc = receipt.microtask_contract;
  if (mc && Array.isArray(mc.file_targets) && mc.file_targets.length > 0) {
    return mc.file_targets.join(",");
  }
  return "";
}

function extractMemories(wpId, receipts) {
  const memories = [];
  const seen = new Set();

  for (const receipt of receipts) {
    if (!HIGH_SIGNAL_RECEIPT_KINDS.has(receipt.receipt_kind)) continue;

    const dedupeKey = `${receipt.receipt_kind}:${receipt.actor_role}:${receipt.timestamp_utc}`;
    if (seen.has(dedupeKey)) continue;
    seen.add(dedupeKey);

    const fileScope = extractFileScope(receipt);
    const importance = HIGH_IMPORTANCE_KINDS.has(receipt.receipt_kind) ? 0.8 : 0.5;
    const mtRef = receipt.microtask_contract?.scope_ref || "";
    const topic = mtRef
      ? `${receipt.receipt_kind} on ${mtRef}`
      : `${receipt.receipt_kind} by ${receipt.actor_role}`;

    memories.push({
      memoryType: "episodic",
      topic,
      summary: receipt.summary || `${receipt.receipt_kind} from ${receipt.actor_role}`,
      wpId,
      fileScope,
      importance,
      content: buildContentBlock(receipt),
      sourceArtifact: "RECEIPTS.jsonl",
      sourceWpId: wpId,
      sourceRole: receipt.actor_role || "",
      sourceSession: receipt.actor_session || "",
      metadata: {
        receipt_kind: receipt.receipt_kind,
        timestamp_utc: receipt.timestamp_utc,
        correlation_id: receipt.correlation_id || "",
        mt_scope_ref: mtRef,
      },
    });

    if (receipt.receipt_kind === "REPAIR" && receipt.state_before && receipt.state_after) {
      memories.push({
        memoryType: "procedural",
        topic: `Fix pattern: ${mtRef || receipt.actor_role}`,
        summary: `${receipt.state_before} → ${receipt.state_after}: ${(receipt.summary || "").slice(0, 120)}`,
        wpId,
        fileScope,
        importance: 0.8,
        content: `Before: ${receipt.state_before}\nAfter: ${receipt.state_after}\nSummary: ${receipt.summary || ""}\nRole: ${receipt.actor_role}`,
        sourceArtifact: "RECEIPTS.jsonl",
        sourceWpId: wpId,
        sourceRole: receipt.actor_role || "",
        metadata: { receipt_kind: "REPAIR", timestamp_utc: receipt.timestamp_utc },
      });
    }
  }

  return memories;
}

function buildContentBlock(receipt) {
  const lines = [];
  lines.push(`Kind: ${receipt.receipt_kind}`);
  lines.push(`Role: ${receipt.actor_role} (${receipt.actor_authority_kind || ""})`);
  lines.push(`Time: ${receipt.timestamp_utc}`);
  if (receipt.summary) lines.push(`Summary: ${receipt.summary}`);
  if (receipt.state_before) lines.push(`Before: ${receipt.state_before}`);
  if (receipt.state_after) lines.push(`After: ${receipt.state_after}`);
  if (receipt.target_role) lines.push(`Target: ${receipt.target_role}`);
  const mc = receipt.microtask_contract;
  if (mc) {
    if (mc.scope_ref) lines.push(`MT: ${mc.scope_ref}`);
    if (mc.review_outcome) lines.push(`Outcome: ${mc.review_outcome}`);
    if (Array.isArray(mc.file_targets) && mc.file_targets.length > 0) lines.push(`Files: ${mc.file_targets.join(", ")}`);
  }
  return lines.join("\n");
}

function discoverWpIds() {
  const commDir = repoPathAbs(COMM_ROOT);
  if (!fs.existsSync(commDir)) return [];
  return fs.readdirSync(commDir)
    .filter(name => name.startsWith("WP-") && fs.statSync(path.join(commDir, name)).isDirectory());
}

function processWp(db, wpId) {
  const paths = communicationPathsForWp(wpId);
  const receiptsFile = repoPathAbs(paths.receiptsFile);
  if (!fs.existsSync(receiptsFile)) return 0;

  const receipts = parseJsonlFile(receiptsFile);
  if (receipts.length === 0) return 0;

  const existing = db.prepare(
    "SELECT COUNT(*) as count FROM memory_entries WHERE source_wp_id = ? AND source_artifact = 'RECEIPTS.jsonl'"
  ).get(wpId);

  if (existing.count >= receipts.length) return 0;

  const memories = extractMemories(wpId, receipts);
  let added = 0;
  for (const mem of memories) {
    const dupe = db.prepare(
      "SELECT id FROM memory_index WHERE topic = ? AND wp_id = ? AND memory_type = ?"
    ).get(mem.topic, mem.wpId, mem.memoryType);
    if (dupe) continue;
    addMemory(db, mem);
    added++;
  }
  return added;
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const arg = process.argv[2] || "";
if (!arg) {
  console.error("Usage: memory-extract-from-receipts.mjs <WP_ID> | --all");
  process.exit(1);
}

const { db } = openGovernanceMemoryDb();
try {
  if (arg === "--all") {
    const wpIds = discoverWpIds();
    let total = 0;
    for (const wpId of wpIds) {
      const added = processWp(db, wpId);
      if (added > 0) console.log(`[memory-extract] ${wpId}: extracted ${added} memories`);
      total += added;
    }
    console.log(`[memory-extract] Total: ${total} memories from ${wpIds.length} WPs`);
  } else {
    if (!arg.startsWith("WP-")) {
      console.error("WP_ID must start with WP-");
      process.exit(1);
    }
    const added = processWp(db, arg);
    console.log(`[memory-extract] ${arg}: extracted ${added} memories`);
  }
} finally {
  closeDb(db);
}
