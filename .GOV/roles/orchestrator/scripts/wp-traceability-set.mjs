#!/usr/bin/env node
/**
 * Deterministic Base WP -> Active Packet mapping updater.
 *
 * Updates `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` without manual table editing.
 *
 * Behavior:
 * - Prefers the resolved official Work Packet path if present
 *   (current physical storage: `.GOV/task_packets/<ACTIVE>.md` or `.GOV/task_packets/<ACTIVE>/packet.md`)
 * - Otherwise falls back to the resolved stub path
 *   (current physical storage: `.GOV/task_packets/stubs/<ACTIVE>.md`)
 */

import fs from "node:fs";
import path from "node:path";
import { repoPathAbs, resolveWorkPacketPath, WORK_PACKET_STORAGE_ROOT_REPO_REL, WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const REGISTRY_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`;
const OFFICIAL_DIR = WORK_PACKET_STORAGE_ROOT_REPO_REL;
const STUB_DIR = WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL;

function fail(message, details = []) {
  console.error(`[WP_TRACEABILITY_SET] ${message}`);
  for (const line of details) console.error(`- ${line}`);
  process.exit(1);
}

function detectEol(text) {
  return text.includes("\r\n") ? "\r\n" : "\n";
}

function readText(p) {
  try {
    return fs.readFileSync(p, "utf8");
  } catch (e) {
    fail(`Failed to read: ${p}`, [String(e?.message || e)]);
  }
}

function writeText(p, text) {
  try {
    fs.writeFileSync(p, text, "utf8");
  } catch (e) {
    fail(`Failed to write: ${p}`, [String(e?.message || e)]);
  }
}

function exists(p) {
  try {
    return fs.existsSync(p);
  } catch {
    return false;
  }
}

function resolvePacketPath(activeWpId) {
  const official = resolveWorkPacketPath(activeWpId)?.packetPath || "";
  if (official && exists(repoPathAbs(official))) return official;
  const stub = path.join(STUB_DIR, `${activeWpId}.md`).replace(/\\/g, "/");
  if (exists(repoPathAbs(stub))) return stub;
  fail("Active packet file not found (official or stub)", [
    `tried=${official || path.join(OFFICIAL_DIR, `${activeWpId}.md`).replace(/\\/g, "/")}`,
    `tried=${stub}`,
  ]);
}

function parsePipeRow(line) {
  if (!line.trim().startsWith("|")) return null;
  const parts = line
    .split("|")
    .slice(1, -1)
    .map((p) => p.trim());
  return parts;
}

function main() {
  const baseWpId = (process.argv[2] || "").trim();
  const activeWpId = (process.argv[3] || "").trim();

  if (!baseWpId || !baseWpId.startsWith("WP-")) {
    fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/wp-traceability-set.mjs <BASE_WP_ID> <ACTIVE_PACKET_WP_ID>`, [
      `Example: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/wp-traceability-set.mjs WP-1-Workflow-Engine WP-1-Workflow-Engine-v4`,
    ]);
  }
  if (!activeWpId || !activeWpId.startsWith("WP-")) {
    fail("Active packet WP_ID must start with WP-", [`got=${activeWpId}`]);
  }

  const registryAbsPath = repoPathAbs(REGISTRY_PATH);
  if (!exists(registryAbsPath)) fail("Missing registry", [`Expected: ${REGISTRY_PATH}`]);

  const activePath = resolvePacketPath(activeWpId);

  const raw = readText(registryAbsPath);
  const eol = detectEol(raw);
  const lines = raw.split(/\r?\n/);

  const headerIdx = lines.findIndex((l) => l.trim() === "| Base WP ID | Active Packet | Task Board | Notes |");
  if (headerIdx === -1) {
    fail("Registry header row not found (format changed?)", [
      "Expected a table header: | Base WP ID | Active Packet | Task Board | Notes |",
    ]);
  }

  // Find the last contiguous table row after the header.
  let tableEndIdx = headerIdx + 1;
  while (tableEndIdx < lines.length) {
    const line = lines[tableEndIdx] || "";
    if (!line.trim().startsWith("|")) break;
    tableEndIdx += 1;
  }

  let updated = false;
  for (let i = headerIdx + 2; i < tableEndIdx; i += 1) {
    const parts = parsePipeRow(lines[i] || "");
    if (!parts || parts.length < 2) continue;
    if (parts[0] !== baseWpId) continue;

    // Preserve Task Board + Notes columns.
    const taskBoard = parts[2] || "TBD";
    const notes = parts.slice(3).join(" | ") || "";
    lines[i] = `| ${baseWpId} | ${activePath} | ${taskBoard} | ${notes} |`;
    updated = true;
    break;
  }

  if (!updated) {
    // Append a new row at the end of the table.
    const row = `| ${baseWpId} | ${activePath} | TBD | |`;
    lines.splice(tableEndIdx, 0, row);
  }

  const out = lines.join(eol);
  writeText(registryAbsPath, out.endsWith(eol) ? out : out + eol);

  console.log("wp-traceability-set ok");
  console.log(`- base_wp_id: ${baseWpId}`);
  console.log(`- active_packet: ${activePath}`);
}

main();
