#!/usr/bin/env node
/**
 * Deterministic Base WP -> Active Packet mapping updater.
 *
 * JSON-primary authority: writes
 * `.GOV/roles_shared/records/wp_traceability_registry.json` then regenerates
 * the operator-readable projection at
 * `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`. The .md is a
 * generated surface — do not hand-edit; this script is the single writer.
 *
 * Active packet resolution:
 * - Prefers the resolved official Work Packet path if present
 *   (current physical storage: `.GOV/task_packets/<ACTIVE>.md` or
 *   `.GOV/task_packets/<ACTIVE>/packet.md`).
 * - Otherwise falls back to the resolved stub path
 *   (current physical storage: `.GOV/task_packets/stubs/<ACTIVE>.md` or
 *   `.GOV/task_packets/stubs/<ACTIVE>.contract.json` for the new .json-primary
 *   stub policy).
 */

import fs from "node:fs";
import path from "node:path";
import {
  repoPathAbs,
  resolveWorkPacketPath,
  GOV_ROOT_REPO_REL,
  WORK_PACKET_STORAGE_ROOT_REPO_REL,
  WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  registerFailCaptureHook,
  failWithMemory,
} from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import {
  migrateRegistryFromMarkdownIfNeeded,
  upsertEntry,
  persistRegistry,
} from "../../../roles_shared/scripts/lib/traceability-registry-lib.mjs";
registerFailCaptureHook("wp-traceability-set.mjs", { role: "ORCHESTRATOR" });

const OFFICIAL_DIR = WORK_PACKET_STORAGE_ROOT_REPO_REL;
const STUB_DIR = WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL;

function fail(message, details = []) {
  failWithMemory("wp-traceability-set.mjs", message, {
    role: "ORCHESTRATOR",
    details,
  });
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
  const stubMd = path.join(STUB_DIR, `${activeWpId}.md`).replace(/\\/g, "/");
  if (exists(repoPathAbs(stubMd))) return stubMd;
  const stubJson = path
    .join(STUB_DIR, `${activeWpId}.contract.json`)
    .replace(/\\/g, "/");
  if (exists(repoPathAbs(stubJson))) return stubJson;
  fail("Active packet file not found (official, stub .md, or stub .contract.json)", [
    `tried=${official || path.join(OFFICIAL_DIR, `${activeWpId}.md`).replace(/\\/g, "/")}`,
    `tried=${stubMd}`,
    `tried=${stubJson}`,
  ]);
}

function main() {
  const baseWpId = (process.argv[2] || "").trim();
  const activeWpId = (process.argv[3] || "").trim();

  if (!baseWpId || !baseWpId.startsWith("WP-")) {
    fail(
      `Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/wp-traceability-set.mjs <BASE_WP_ID> <ACTIVE_PACKET_WP_ID>`,
      [
        `Example: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/wp-traceability-set.mjs WP-1-Workflow-Engine WP-1-Workflow-Engine-v4`,
      ],
    );
  }
  if (!activeWpId || !activeWpId.startsWith("WP-")) {
    fail("Active packet WP_ID must start with WP-", [`got=${activeWpId}`]);
  }

  const activePath = resolvePacketPath(activeWpId);

  // Migrate-if-needed is idempotent: returns the loaded registry if .json
  // already exists. On first run after MD-ELIM-PHASE-2 lands, this parses the
  // legacy .md once and writes the .json + projection.
  let registry;
  try {
    registry = migrateRegistryFromMarkdownIfNeeded();
  } catch (e) {
    fail("Failed to load or migrate traceability registry", [
      String(e?.message || e),
    ]);
  }

  upsertEntry(registry, {
    base_wp_id: baseWpId,
    active_packet_path: activePath,
  });

  try {
    persistRegistry(registry);
  } catch (e) {
    fail("Failed to persist traceability registry", [String(e?.message || e)]);
  }

  console.log("wp-traceability-set ok");
  console.log(`- base_wp_id: ${baseWpId}`);
  console.log(`- active_packet: ${activePath}`);
}

main();
