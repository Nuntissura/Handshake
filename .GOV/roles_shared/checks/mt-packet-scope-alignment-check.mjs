#!/usr/bin/env node

/**
 * mt-packet-scope-alignment-check.mjs
 *
 * Hardening gate that prevents drift between two surfaces inside a Work Packet
 * folder:
 *
 *   .GOV/task_packets/<WP_ID>/packet.json     -> scope.allowed_paths (glob list)
 *   .GOV/task_packets/<WP_ID>/MT-<NNN>.json   -> owned_files (literal path list)
 *
 * Root cause we are catching here (real session, KERNEL-004 / MT-046):
 *   - MT-046.json declared owned_files including
 *     `src/backend/handshake_core/src/bin/mt046_token_probe.rs`.
 *   - packet.json scope.allowed_paths did NOT include
 *     `src/backend/handshake_core/src/bin/**`.
 *   - KERNEL_BUILDER committed the file. wp-validator-mechanical-track.mjs then
 *     flagged CHANGED_FILES_OUTSIDE_PACKET_IN_SCOPE_PATHS *after the commit*.
 *
 * This check moves that detection BEFORE the commit. For every MT contract in
 * every packet, every owned_files entry must be matched by at least one glob in
 * the parent packet's scope.allowed_paths. Unmatched literal paths are emitted
 * as structured CONCERNS (fail).
 *
 * As a low-effort complement, packet allowed_paths globs that no MT owned_files
 * entry consumes are emitted as INFO (warn-only) — they often indicate stale
 * scope after MTs were folded or retired.
 *
 * Exit codes:
 *   0  - no concerns (info-only is still exit 0)
 *   1  - at least one CONCERN (owned_file not matched by any allowed_paths glob)
 *   2  - read/parse error (missing/malformed JSON, IO problems)
 *
 * Flags:
 *   --wp <WP_ID>   Limit to a single WP packet folder.
 *   --json         Emit a single JSON document; otherwise human-readable text.
 *   --help, -h     Print usage.
 *
 * Wiring:
 *   - Registered as a gov-check sub-step in
 *     .GOV/roles_shared/checks/gov-check.mjs (phase WORK_PACKET).
 *   - Registered in .GOV/roles_shared/records/GOVERNANCE_TOPOLOGY.json by the
 *     deterministic topology builder (auto-detected because it lives under
 *     .GOV/roles_shared/checks/).
 *   - Wraps registerFailCaptureHook per [CX-205N].
 *   - Uses runtime-paths.mjs helpers per the project's path-resolution contract.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  normalizePath,
  repoPathAbs,
} from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, captureFailure } from "../scripts/lib/fail-capture-lib.mjs";
import { matchesScopeEntry } from "../scripts/lib/scope-surface-lib.mjs";

registerFailCaptureHook("mt-packet-scope-alignment-check.mjs", { role: "SHARED" });

const EXIT_OK = 0;
const EXIT_CONCERN = 1;
const EXIT_READ_ERROR = 2;

const TASK_PACKETS_DIR_REPO_REL = `${GOV_ROOT_REPO_REL}/task_packets`;

const MT_FILENAME_RE = /^MT-\d{3,4}\.json$/i;

function parseArgs(argv) {
  const args = { wp: "", json: false, help: false, taskPacketsRoot: "" };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") { args.help = true; continue; }
    if (arg === "--json") { args.json = true; continue; }
    if (arg === "--wp") {
      args.wp = String(argv[index + 1] || "").trim();
      index += 1;
      continue;
    }
    if (arg === "--task-packets-root") {
      // Test-only override so fixture-based tests can point at a temp dir.
      args.taskPacketsRoot = String(argv[index + 1] || "").trim();
      index += 1;
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

function usage() {
  return [
    "Usage: node .GOV/roles_shared/checks/mt-packet-scope-alignment-check.mjs [--wp <WP_ID>] [--json] [--task-packets-root <dir>]",
    "",
    "Verifies that every MT-NNN.json owned_files entry is matched by at least one",
    "glob in the parent packet.json scope.allowed_paths array.",
    "",
    "Flags:",
    "  --wp <WP_ID>             Limit to a single WP packet directory.",
    "  --json                   Emit machine-readable JSON instead of text.",
    "  --task-packets-root DIR  Override the task_packets root (test fixtures).",
    "",
    "Exit codes:",
    "  0 - no concerns",
    "  1 - one or more drift concerns",
    "  2 - read/parse error",
  ].join("\n");
}

function relFromAbs(absPath) {
  return normalizePath(path.relative(REPO_ROOT, absPath));
}

function readJsonOrNull(absPath) {
  try {
    return { value: JSON.parse(fs.readFileSync(absPath, "utf8")), error: null };
  } catch (error) {
    return { value: null, error: error?.message || String(error) };
  }
}

function listPacketDirs(taskPacketsAbs, wpFilter) {
  if (!fs.existsSync(taskPacketsAbs)) return [];
  const out = [];
  for (const entry of fs.readdirSync(taskPacketsAbs, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    if (!/^WP-/.test(entry.name)) continue;
    if (wpFilter && entry.name !== wpFilter) continue;
    const packetAbs = path.join(taskPacketsAbs, entry.name, "packet.json");
    if (!fs.existsSync(packetAbs)) continue;
    out.push({
      wpId: entry.name,
      dirAbs: path.join(taskPacketsAbs, entry.name),
      packetAbs,
    });
  }
  return out.sort((a, b) => a.wpId.localeCompare(b.wpId));
}

function listMtFiles(packetDirAbs) {
  return fs.readdirSync(packetDirAbs, { withFileTypes: true })
    .filter((entry) => entry.isFile() && MT_FILENAME_RE.test(entry.name))
    .map((entry) => ({
      name: entry.name,
      abs: path.join(packetDirAbs, entry.name),
    }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

function normalizeGlobOrPath(value) {
  return String(value || "")
    .trim()
    .replace(/\\/g, "/")
    .replace(/^\.\/+/, "")
    .replace(/^\/+/, "");
}

// Glob matching delegates to scope-surface-lib's matchesScopeEntry, the
// same matcher used by wp-validator-mechanical-track.mjs. This guarantees
// pre-commit (this check) and post-commit (mechanical-track) use identical
// semantics — closes adversarial-review finding F-004 (semantic drift
// across three matchers in the bespoke implementation).
function fileMatchesGlob(filePath, pattern) {
  const candidate = normalizeGlobOrPath(filePath);
  const glob = normalizeGlobOrPath(pattern);
  if (!candidate || !glob) return false;
  return matchesScopeEntry(candidate, glob);
}

function arrayOfStrings(value) {
  return Array.isArray(value)
    ? value.map((entry) => String(entry || "").trim()).filter(Boolean)
    : [];
}

export function scanPackets({ taskPacketsRootAbs, wpFilter = "" }) {
  const concerns = [];
  const info = [];
  const readErrors = [];
  const packets = listPacketDirs(taskPacketsRootAbs, wpFilter);

  for (const packet of packets) {
    const packetRel = relFromAbs(packet.packetAbs);
    const packetRead = readJsonOrNull(packet.packetAbs);
    if (packetRead.error) {
      readErrors.push({ path: packetRel, error: packetRead.error });
      continue;
    }
    const allowedPaths = arrayOfStrings(packetRead.value?.scope?.allowed_paths);

    const mtFiles = listMtFiles(packet.dirAbs);
    const consumedGlobs = new Set();

    for (const mtFile of mtFiles) {
      const mtRel = relFromAbs(mtFile.abs);
      const mtRead = readJsonOrNull(mtFile.abs);
      if (mtRead.error) {
        readErrors.push({ path: mtRel, error: mtRead.error });
        continue;
      }
      const ownedFiles = arrayOfStrings(mtRead.value?.owned_files);
      const mtId = String(mtRead.value?.mt_id || mtFile.name.replace(/\.json$/i, ""));

      for (const owned of ownedFiles) {
        const ownedNormalized = normalizeGlobOrPath(owned);
        const matchedGlob = allowedPaths.find((glob) => fileMatchesGlob(ownedNormalized, glob));
        if (matchedGlob) {
          consumedGlobs.add(matchedGlob);
          continue;
        }
        concerns.push({
          severity: "MT_OWNED_FILE_OUTSIDE_PACKET_ALLOWED_PATHS",
          wp_id: packet.wpId,
          mt_id: String(mtId).trim().toUpperCase(),
          file: ownedNormalized,
          packet_path: packetRel,
          mt_path: mtRel,
          reason:
            "MT owned_files entry is not matched by any glob in packet.scope.allowed_paths."
            + " The MT contract claims authority over a path the packet does not allow.",
        });
      }
    }

    for (const glob of allowedPaths) {
      if (consumedGlobs.has(glob)) continue;
      info.push({
        severity: "PACKET_ALLOWED_PATHS_GLOB_NOT_CONSUMED",
        wp_id: packet.wpId,
        glob,
        packet_path: packetRel,
        reason:
          "scope.allowed_paths entry is not consumed by any MT owned_files. May indicate stale"
          + " scope after MTs were folded, renamed, or retired. Warn-only.",
      });
    }
  }

  return {
    ok: concerns.length === 0 && readErrors.length === 0,
    packets_scanned: packets.length,
    concerns,
    info,
    read_errors: readErrors,
  };
}

function emitText(result) {
  const lines = [];
  lines.push(`mt-packet-scope-alignment-check: scanned ${result.packets_scanned} packet(s)`);
  if (result.read_errors.length > 0) {
    lines.push(`  read/parse errors: ${result.read_errors.length}`);
    for (const err of result.read_errors) {
      lines.push(`    ${err.path}: ${err.error}`);
    }
  }
  if (result.concerns.length > 0) {
    lines.push(`  CONCERNS: ${result.concerns.length}`);
    for (const c of result.concerns) {
      lines.push(`    ${c.wp_id} / ${c.mt_id}: ${c.file}`);
      lines.push(`      packet=${c.packet_path}`);
      lines.push(`      mt=${c.mt_path}`);
    }
  }
  if (result.info.length > 0) {
    lines.push(`  INFO (dead globs, warn-only): ${result.info.length}`);
    for (const i of result.info) {
      lines.push(`    ${i.wp_id}: ${i.glob}  (${i.packet_path})`);
    }
  }
  if (result.concerns.length === 0 && result.read_errors.length === 0) {
    lines.push("  ok");
  }
  return lines.join("\n");
}

export function runCli(argv = process.argv.slice(2)) {
  let args;
  try {
    args = parseArgs(argv);
  } catch (error) {
    console.error(`[MT_PACKET_SCOPE_ALIGNMENT] ${error.message}`);
    console.error(usage());
    return EXIT_READ_ERROR;
  }
  if (args.help) {
    console.log(usage());
    return EXIT_OK;
  }

  const taskPacketsRootAbs = args.taskPacketsRoot
    ? path.resolve(args.taskPacketsRoot)
    : repoPathAbs(TASK_PACKETS_DIR_REPO_REL);
  const result = scanPackets({ taskPacketsRootAbs, wpFilter: args.wp });

  if (args.json) {
    console.log(JSON.stringify({
      schema_id: "handshake.gov.mt_packet_scope_alignment@1",
      schema_version: "mt_packet_scope_alignment_v1",
      ok: result.ok,
      packets_scanned: result.packets_scanned,
      concerns: result.concerns,
      info: result.info,
      read_errors: result.read_errors,
    }, null, 2));
  } else {
    console.log(emitText(result));
  }

  if (result.read_errors.length > 0) {
    captureFailure(
      "mt-packet-scope-alignment-check.mjs",
      `read/parse errors in ${result.read_errors.length} contract file(s)`,
      {
        role: "SHARED",
        details: result.read_errors.map((entry) => `${entry.path}: ${entry.error}`),
      },
    );
    if (!args.json) {
      console.error(
        `[mt-packet-scope-alignment-check.mjs] read/parse errors in ${result.read_errors.length} contract file(s)`,
      );
      for (const entry of result.read_errors) {
        console.error(`  - ${entry.path}: ${entry.error}`);
      }
    }
    return EXIT_READ_ERROR;
  }
  if (result.concerns.length > 0) {
    captureFailure(
      "mt-packet-scope-alignment-check.mjs",
      `MT owned_files drift outside packet scope.allowed_paths: ${result.concerns.length} concern(s)`,
      {
        role: "SHARED",
        details: result.concerns.map((c) => `${c.wp_id}/${c.mt_id}: ${c.file} (packet=${c.packet_path})`),
      },
    );
    if (!args.json) {
      console.error(
        `[mt-packet-scope-alignment-check.mjs] ${result.concerns.length} concern(s) — MT owned_files outside packet.scope.allowed_paths`,
      );
      for (const c of result.concerns) {
        console.error(`  - ${c.wp_id}/${c.mt_id}: ${c.file} (packet=${c.packet_path})`);
      }
    }
    return EXIT_CONCERN;
  }
  if (!args.json) {
    console.log("mt-packet-scope-alignment-check ok");
  }
  return EXIT_OK;
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  try {
    const invoked = fs.realpathSync.native(path.resolve(process.argv[1]));
    const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
    return invoked === current;
  } catch {
    return false;
  }
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
