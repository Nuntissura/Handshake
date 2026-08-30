#!/usr/bin/env node
/**
 * JSON-backed MT Task Board CLI.
 *
 * Commands:
 *   board    <WP_ID>
 *   claim    <WP_ID> <SESSION_KEY> [MT_ID]
 *   ready    <WP_ID> <MT_ID> <SESSION_KEY>
 *   complete <WP_ID> <MT_ID>
 *   populate <WP_ID>
 */

import fs from "node:fs";
import path from "node:path";

import { listDeclaredWpMicrotasks } from "../lib/wp-microtask-lib.mjs";
import { communicationPathsForWp } from "../lib/wp-communications-lib.mjs";
import { normalizePath, repoPathAbs, resolveWorkPacketPath } from "../lib/runtime-paths.mjs";

const command = String(process.argv[2] || "").trim().toLowerCase();
const wpId = String(process.argv[3] || "").trim();

function usage(exitCode = 1) {
  console.error("Usage:");
  console.error("  node mt-board.mjs board <WP_ID>");
  console.error("  node mt-board.mjs claim <WP_ID> <SESSION_KEY> [MT_ID]");
  console.error("  node mt-board.mjs ready <WP_ID> <MT_ID> <SESSION_KEY>");
  console.error("  node mt-board.mjs complete <WP_ID> <MT_ID>");
  console.error("  node mt-board.mjs populate <WP_ID>");
  process.exit(exitCode);
}

function fail(message) {
  console.error(`[MT_BOARD] ${message}`);
  process.exit(1);
}

function resolvePacketDir(wpIdValue) {
  const resolved = resolveWorkPacketPath(wpIdValue);
  if (!resolved?.isFolder || !resolved.packetDirAbs || !fs.existsSync(resolved.packetDirAbs)) {
    fail(`JSON-backed MT board requires a folder packet for ${wpIdValue}`);
  }
  return resolved.packetDirAbs;
}

function mtJsonPath(packetDirAbs, mtId) {
  return path.join(packetDirAbs, `${String(mtId || "").trim().toUpperCase()}.json`);
}

function readMtContract(packetDirAbs, mtId) {
  const absPath = mtJsonPath(packetDirAbs, mtId);
  if (!fs.existsSync(absPath)) return null;
  return JSON.parse(fs.readFileSync(absPath, "utf8"));
}

function writeMtContract(packetDirAbs, mtId, contract) {
  const absPath = mtJsonPath(packetDirAbs, mtId);
  fs.writeFileSync(absPath, `${JSON.stringify(contract, null, 2)}\n`, "utf8");
}

function normalizeStatus(value) {
  return String(value || "PENDING").trim().toUpperCase();
}

const VISIBLE_STATUS_VALUES = new Set([
  "OPEN",
  "PENDING",
  "CLAIMED",
  "READY_FOR_VALIDATION",
  "COMPLETED",
  "BLOCKED",
  "BLOCKED_ON_DEPENDENCY",
  "NEEDS_MANAGED_RESOURCE_PROOF",
  "NEEDS_EXTERNAL_RESOURCE",
  "FAIL_NEEDS_REWORK",
  "CANCELLED",
]);

const DEPENDENCY_SATISFIED_STATUS_VALUES = new Set([
  "COMPLETED",
  "READY_FOR_VALIDATION",
]);

function displayStatus(value) {
  const normalized = normalizeStatus(value);
  if (normalized === "COMPLETE") return "COMPLETED";
  if (normalized === "IN_PROGRESS" || normalized === "ACTIVE") return "CLAIMED";
  if (normalized === "READY_FOR_DEV") return "OPEN";
  if (/^CLAIMED(?:_V\d+)?$/.test(normalized)) return "CLAIMED";
  if (/^READY_FOR_VALIDATION(?:_V\d+)?$/.test(normalized)) return "READY_FOR_VALIDATION";
  if (/^(?:PASS|COMPLETED)(?:_V\d+)?$/.test(normalized)) return "COMPLETED";
  if (/^(?:NEEDS_REIMPLEMENTATION|FAILED)(?:_V\d+)?$/.test(normalized)) return "FAIL_NEEDS_REWORK";
  if (VISIBLE_STATUS_VALUES.has(normalized)) {
    return normalized;
  }
  return "PENDING";
}

function declaredMtRows(wpIdValue) {
  const packetDirAbs = resolvePacketDir(wpIdValue);
  return listDeclaredWpMicrotasks(wpIdValue).map((mt) => {
    const contract = readMtContract(packetDirAbs, mt.mtId) || {};
    const lifecycle = contract.lifecycle && typeof contract.lifecycle === "object"
      ? contract.lifecycle
      : {};
    return {
      ...mt,
      packetDirAbs,
      contract,
      status: displayStatus(lifecycle.status),
      session: contract?.handoff?.coder_session || lifecycle.claimed_by || null,
      dependsOnList: dependencyList(contract, mt),
    };
  });
}

function dependencyList(contract, mt) {
  const lifecycleDepends = Array.isArray(contract?.lifecycle?.depends_on)
    ? contract.lifecycle.depends_on
    : [];
  const topLevelDepends = Array.isArray(contract?.depends_on_mts)
    ? contract.depends_on_mts
    : [];
  const parsedDepends = String(mt?.dependsOn || "")
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry && entry !== "NONE");
  return Array.from(new Set([...lifecycleDepends, ...topLevelDepends, ...parsedDepends]
    .map((entry) => String(entry || "").trim().toUpperCase())
    .filter(Boolean)));
}

function dependenciesCompleted(row, rows) {
  if (row.dependsOnList.length === 0) return true;
  const completed = new Set(
    rows
      .filter((entry) => DEPENDENCY_SATISFIED_STATUS_VALUES.has(entry.status))
      .map((entry) => entry.mtId.toUpperCase()),
  );
  return row.dependsOnList.every((mtId) => completed.has(mtId));
}

function complexityTier(row) {
  return row.heuristicRisk?.heuristic_risk === "YES" ? "HEURISTIC" : "MEDIUM";
}

function formatBoard(wpIdValue) {
  const rows = declaredMtRows(wpIdValue);
  const lines = [
    `MT Task Board for ${wpIdValue}`,
    "────────────────────────────────────────────────────────────",
  ];
  for (const row of rows) {
    const assignee = row.session ? ` (${row.session})` : "";
    lines.push(`  ${row.mtId} | ${row.status.padEnd(10)} | ${complexityTier(row)}${assignee}`);
    lines.push(`         ${row.clause}`);
  }
  return lines.join("\n");
}

function ensureLifecycle(contract) {
  if (!contract.lifecycle || typeof contract.lifecycle !== "object" || Array.isArray(contract.lifecycle)) {
    contract.lifecycle = {};
  }
  return contract.lifecycle;
}

function populate(wpIdValue) {
  const rows = declaredMtRows(wpIdValue);
  let changed = 0;
  for (const row of rows) {
    const contract = row.contract;
    const lifecycle = ensureLifecycle(contract);
    if (!lifecycle.status) {
      lifecycle.status = "PENDING";
      changed += 1;
      writeMtContract(row.packetDirAbs, row.mtId, contract);
    }
  }
  console.log(`[MT_BOARD] Populated ${rows.length} microtasks for ${wpIdValue}${changed ? ` (${changed} initialized)` : ""}`);
  console.log(formatBoard(wpIdValue));
}

function claim(wpIdValue, sessionKey, requestedMtId = "") {
  const rows = declaredMtRows(wpIdValue);
  const normalizedRequestedMtId = String(requestedMtId || "").trim().toUpperCase();
  const claimableStatuses = new Set(["OPEN", "PENDING", "FAIL_NEEDS_REWORK"]);
  const candidates = normalizedRequestedMtId
    ? rows.filter((row) => row.mtId.toUpperCase() === normalizedRequestedMtId)
    : rows;
  const claimable = candidates.find((row) => claimableStatuses.has(row.status) && dependenciesCompleted(row, rows));
  if (!claimable) {
    if (normalizedRequestedMtId) {
      fail(`${normalizedRequestedMtId} is missing, already owned, blocked, or has unsatisfied dependencies in ${wpIdValue}`);
    }
    console.log(`[MT_BOARD] No unclaimed microtasks available for ${wpIdValue}`);
    return;
  }
  const contract = claimable.contract;
  const lifecycle = ensureLifecycle(contract);
  lifecycle.status = "CLAIMED";
  lifecycle.active = true;
  lifecycle.claimed_by = sessionKey;
  lifecycle.claimed_at_utc = new Date().toISOString();
  contract.handoff = {
    ...(contract.handoff || {}),
    coder_session: sessionKey,
  };
  writeMtContract(claimable.packetDirAbs, claimable.mtId, contract);
  console.log(`[MT_BOARD] Claimed ${claimable.mtId} for ${sessionKey}`);
}

function complete(wpIdValue, mtId) {
  const packetDirAbs = resolvePacketDir(wpIdValue);
  const contract = readMtContract(packetDirAbs, mtId);
  if (!contract) fail(`Missing microtask contract for ${mtId}`);
  const lifecycle = ensureLifecycle(contract);
  const status = displayStatus(lifecycle.status);
  if (status !== "CLAIMED") {
    console.log(`[MT_BOARD] ${String(mtId).toUpperCase()} is not in 'claimed' state — cannot complete`);
    return;
  }
  lifecycle.status = "COMPLETED";
  lifecycle.active = false;
  lifecycle.completed_at_utc = new Date().toISOString();
  writeMtContract(packetDirAbs, mtId, contract);
  console.log(`[MT_BOARD] ${String(mtId).toUpperCase()} marked completed`);
}

function latestReadyReceipt(wpIdValue, mtId) {
  const commPaths = communicationPathsForWp(wpIdValue);
  const receiptsPath = path.join(repoPathAbs(commPaths.dir), "KB_READY_CHECKLIST_RECEIPTS.jsonl");
  if (!fs.existsSync(receiptsPath)) return null;
  let latest = null;
  for (const line of fs.readFileSync(receiptsPath, "utf8").split(/\r?\n/)) {
    if (!line.trim()) continue;
    let receipt;
    try {
      receipt = JSON.parse(line);
    } catch {
      fail(`Invalid JSON in readiness receipt log: ${normalizePath(receiptsPath)}`);
    }
    if (String(receipt?.mt_id || "").trim().toUpperCase() === mtId) latest = receipt;
  }
  return latest;
}

function ready(wpIdValue, mtId, sessionKey) {
  const packetDirAbs = resolvePacketDir(wpIdValue);
  const contract = readMtContract(packetDirAbs, mtId);
  if (!contract) fail(`Missing microtask contract for ${mtId}`);
  const lifecycle = ensureLifecycle(contract);
  if (displayStatus(lifecycle.status) !== "CLAIMED") {
    fail(`${mtId} must be CLAIMED before READY_FOR_VALIDATION`);
  }
  if (String(lifecycle.claimed_by || "").trim() !== sessionKey) {
    fail(`${mtId} is claimed by ${lifecycle.claimed_by || "<none>"}, not ${sessionKey}`);
  }
  const receipt = latestReadyReceipt(wpIdValue, mtId);
  if (receipt?.schema_id !== "hsk.kb_ready_checklist_receipt@1"
      || receipt?.overall_verdict !== "PASS"
      || String(receipt?.actor_session || "").trim() !== sessionKey) {
    fail(`${mtId} requires a latest PASS KB readiness receipt from ${sessionKey}`);
  }
  lifecycle.status = "READY_FOR_VALIDATION";
  lifecycle.active = false;
  lifecycle.ready_for_validation_at_utc = new Date().toISOString();
  lifecycle.ready_for_validation_by = sessionKey;
  writeMtContract(packetDirAbs, mtId, contract);
  console.log(`[MT_BOARD] ${mtId} marked ready for validation by ${sessionKey}`);
}

if (!command || !wpId) usage();

if (command === "board") {
  console.log(formatBoard(wpId));
} else if (command === "populate") {
  populate(wpId);
} else if (command === "claim") {
  const sessionKey = String(process.argv[4] || "").trim();
  if (!sessionKey) usage();
  claim(wpId, sessionKey, process.argv[5]);
} else if (command === "ready") {
  const mtId = String(process.argv[4] || "").trim().toUpperCase();
  const sessionKey = String(process.argv[5] || "").trim();
  if (!mtId || !sessionKey) usage();
  ready(wpId, mtId, sessionKey);
} else if (command === "complete") {
  const mtId = String(process.argv[4] || "").trim().toUpperCase();
  if (!mtId) usage();
  complete(wpId, mtId);
} else {
  fail(`Unknown command: ${command}`);
}
