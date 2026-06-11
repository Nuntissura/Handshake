#!/usr/bin/env node

/**
 * kb-ready-checklist-coverage-check.mjs [F-002]
 *
 * Consumer-side gate for the KB_READY_CHECKLIST_RECEIPT producer
 * (`.GOV/roles/kernel_builder/scripts/kb-ready-checklist.mjs`).
 *
 * Without an automated consumer, the Spec-Realism Gate self-check is purely
 * "social discipline": Kernel Builder can transition an MT to
 * READY_FOR_VALIDATION without emitting (or after emitting a BLOCKED) receipt
 * and nothing in the governance bundle flags it. This check closes that loop:
 *
 *   For every MT contract under `.GOV/task_packets/<WP_ID>/MT-NNN.json`
 *   whose `lifecycle.status` is READY_FOR_VALIDATION, there MUST exist a
 *   matching KB_READY_CHECKLIST receipt with `overall_verdict == "PASS"` in
 *   the JSONL log
 *
 *     <governance_runtime_root>/roles_shared/WP_COMMUNICATIONS/<WP_ID>/
 *       KB_READY_CHECKLIST_RECEIPTS.jsonl
 *
 *   The JSONL is append-only; if multiple receipts exist for the same
 *   (wp_id, mt_id), only the LATEST line matters (later lines supersede
 *   earlier ones).
 *
 * Concern conditions:
 *   - MT is READY_FOR_VALIDATION but no receipt line matches its mt_id.
 *   - MT is READY_FOR_VALIDATION and the latest matching receipt has
 *     overall_verdict == "BLOCKED" with a non-empty blockers[] list.
 *
 * Skip conditions (NOT a concern):
 *   - MT is in any non-READY_FOR_VALIDATION lifecycle state (CLAIMED,
 *     COMPLETED, BLOCKED, etc.). The receipt is only required as a precondition
 *     for the CLAIMED -> READY_FOR_VALIDATION transition.
 *
 * Exit codes:
 *   0  - no concerns
 *   1  - at least one CONCERN
 *   2  - read/parse error (missing/malformed JSON, IO problems)
 *
 * Flags:
 *   --wp <WP_ID>                 Limit to a single WP packet directory.
 *   --json                       Emit a single JSON document; otherwise human text.
 *   --task-packets-root <dir>    Test-only override of the task_packets root.
 *   --comms-root <dir>           Test-only override of the WP_COMMUNICATIONS root.
 *   --help, -h                   Print usage.
 *
 * Wiring:
 *   - Registered as a gov-check sub-step in
 *     .GOV/roles_shared/checks/gov-check.mjs (phase WORK_PACKET).
 *   - Wraps registerFailCaptureHook per [CX-205N].
 *   - Uses runtime-paths.mjs + wp-communications-lib.mjs helpers per the
 *     project's path-resolution contract.
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
import { COMM_ROOT } from "../scripts/lib/wp-communications-lib.mjs";
import { registerFailCaptureHook, captureFailure } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("kb-ready-checklist-coverage-check.mjs", { role: "SHARED" });

const EXIT_OK = 0;
const EXIT_CONCERN = 1;
const EXIT_READ_ERROR = 2;

const TASK_PACKETS_DIR_REPO_REL = `${GOV_ROOT_REPO_REL}/task_packets`;
const RECEIPTS_FILE_NAME = "KB_READY_CHECKLIST_RECEIPTS.jsonl";

const MT_FILENAME_RE = /^MT-\d{3,4}\.json$/i;
const READY_FOR_VALIDATION_STATUS = "READY_FOR_VALIDATION";

function parseArgs(argv) {
  const args = {
    wp: "",
    json: false,
    help: false,
    taskPacketsRoot: "",
    commsRoot: "",
  };
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
      args.taskPacketsRoot = String(argv[index + 1] || "").trim();
      index += 1;
      continue;
    }
    if (arg === "--comms-root") {
      args.commsRoot = String(argv[index + 1] || "").trim();
      index += 1;
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

function usage() {
  return [
    "Usage: node .GOV/roles_shared/checks/kb-ready-checklist-coverage-check.mjs [--wp <WP_ID>] [--json]",
    "",
    "Verifies that every MT contract in READY_FOR_VALIDATION lifecycle status",
    "has a matching KB_READY_CHECKLIST_RECEIPT with overall_verdict=PASS in the",
    "WP communications KB_READY_CHECKLIST_RECEIPTS.jsonl log.",
    "",
    "Flags:",
    "  --wp <WP_ID>             Limit to a single WP packet directory.",
    "  --json                   Emit machine-readable JSON instead of text.",
    "  --task-packets-root DIR  Override the task_packets root (test fixtures).",
    "  --comms-root DIR         Override the WP_COMMUNICATIONS root (test fixtures).",
    "",
    "Exit codes:",
    "  0 - no concerns",
    "  1 - one or more coverage concerns",
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

/**
 * Read every line of a KB_READY_CHECKLIST_RECEIPTS.jsonl file and return a map
 * keyed by mt_id -> LATEST matching receipt (later lines win). Receipts that
 * fail to parse are returned as readErrors so the caller can surface them.
 */
function loadLatestReceiptsByMt(receiptsAbsPath) {
  const result = { byMt: new Map(), readErrors: [], existed: false };
  if (!fs.existsSync(receiptsAbsPath)) return result;
  result.existed = true;

  let raw;
  try {
    raw = fs.readFileSync(receiptsAbsPath, "utf8");
  } catch (error) {
    result.readErrors.push({
      path: normalizePath(path.relative(REPO_ROOT, receiptsAbsPath)),
      error: error?.message || String(error),
    });
    return result;
  }

  const lines = raw.split(/\r?\n/);
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    if (!line || !line.trim()) continue;
    let parsed;
    try {
      parsed = JSON.parse(line);
    } catch (error) {
      result.readErrors.push({
        path: `${normalizePath(path.relative(REPO_ROOT, receiptsAbsPath))}:${lineIndex + 1}`,
        error: error?.message || String(error),
      });
      continue;
    }
    const mtId = String(parsed?.mt_id || "").trim().toUpperCase();
    if (!mtId) continue;
    // Later lines win — overwriting is intentional.
    result.byMt.set(mtId, parsed);
  }
  return result;
}

export function scanCoverage({ taskPacketsRootAbs, commsRootAbs, wpFilter = "" }) {
  const concerns = [];
  const info = [];
  const readErrors = [];
  const packets = listPacketDirs(taskPacketsRootAbs, wpFilter);

  for (const packet of packets) {
    const packetRel = relFromAbs(packet.packetAbs);
    const wpCommDirAbs = path.join(commsRootAbs, packet.wpId);
    const receiptsAbsPath = path.join(wpCommDirAbs, RECEIPTS_FILE_NAME);
    const expectedReceiptRel = normalizePath(path.relative(REPO_ROOT, receiptsAbsPath));

    const receiptLoad = loadLatestReceiptsByMt(receiptsAbsPath);
    for (const err of receiptLoad.readErrors) readErrors.push(err);

    const mtFiles = listMtFiles(packet.dirAbs);
    for (const mtFile of mtFiles) {
      const mtRel = relFromAbs(mtFile.abs);
      const mtRead = readJsonOrNull(mtFile.abs);
      if (mtRead.error) {
        readErrors.push({ path: mtRel, error: mtRead.error });
        continue;
      }
      const contract = mtRead.value || {};
      const mtId = String(contract.mt_id || mtFile.name.replace(/\.json$/i, ""))
        .trim()
        .toUpperCase();
      const lifecycleStatus = String(contract?.lifecycle?.status || "").trim();

      if (lifecycleStatus !== READY_FOR_VALIDATION_STATUS) {
        // Per F-002 contract: only READY_FOR_VALIDATION MTs require a receipt.
        // Other lifecycle states are skipped silently.
        continue;
      }

      const receipt = receiptLoad.byMt.get(mtId);
      if (!receipt) {
        concerns.push({
          severity: "MT_READY_FOR_VALIDATION_WITHOUT_KB_READY_RECEIPT",
          wp_id: packet.wpId,
          mt_id: mtId,
          mt_path: mtRel,
          lifecycle_status: lifecycleStatus,
          expected_receipt_path: expectedReceiptRel,
          reason:
            "MT is in READY_FOR_VALIDATION lifecycle status but no matching"
            + " KB_READY_CHECKLIST_RECEIPT exists in the WP communications log."
            + " The Spec-Realism Gate self-check is mandatory for this transition.",
        });
        continue;
      }

      const verdict = String(receipt.overall_verdict || "").trim();
      const blockers = Array.isArray(receipt.blockers) ? receipt.blockers : [];
      if (verdict === "BLOCKED" && blockers.length > 0) {
        concerns.push({
          severity: "MT_READY_FOR_VALIDATION_WITH_BLOCKED_KB_READY_RECEIPT",
          wp_id: packet.wpId,
          mt_id: mtId,
          mt_path: mtRel,
          lifecycle_status: lifecycleStatus,
          expected_receipt_path: expectedReceiptRel,
          receipt_verdict: verdict,
          blockers,
          reason:
            "MT is in READY_FOR_VALIDATION lifecycle status but the latest"
            + " matching KB_READY_CHECKLIST_RECEIPT has overall_verdict=BLOCKED"
            + " with one or more unresolved blockers.",
        });
        continue;
      }

      if (verdict !== "PASS") {
        concerns.push({
          severity: "MT_READY_FOR_VALIDATION_WITH_UNEXPECTED_RECEIPT_VERDICT",
          wp_id: packet.wpId,
          mt_id: mtId,
          mt_path: mtRel,
          lifecycle_status: lifecycleStatus,
          expected_receipt_path: expectedReceiptRel,
          receipt_verdict: verdict || "<missing>",
          reason:
            "MT is in READY_FOR_VALIDATION lifecycle status and a receipt"
            + " exists, but its overall_verdict is neither PASS nor BLOCKED."
            + " Receipt verdict must be one of {PASS, BLOCKED}.",
        });
      }
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
  lines.push(`kb-ready-checklist-coverage-check: scanned ${result.packets_scanned} packet(s)`);
  if (result.read_errors.length > 0) {
    lines.push(`  read/parse errors: ${result.read_errors.length}`);
    for (const err of result.read_errors) {
      lines.push(`    ${err.path}: ${err.error}`);
    }
  }
  if (result.concerns.length > 0) {
    lines.push(`  CONCERNS: ${result.concerns.length}`);
    // Group concerns by WP for human readability.
    const byWp = new Map();
    for (const concern of result.concerns) {
      if (!byWp.has(concern.wp_id)) byWp.set(concern.wp_id, []);
      byWp.get(concern.wp_id).push(concern);
    }
    for (const [wpId, wpConcerns] of Array.from(byWp.entries()).sort((a, b) => a[0].localeCompare(b[0]))) {
      lines.push(`    ${wpId}: ${wpConcerns.length} concern(s)`);
      for (const c of wpConcerns) {
        lines.push(`      ${c.mt_id} (${c.severity})`);
        lines.push(`        mt=${c.mt_path}`);
        lines.push(`        expected_receipt=${c.expected_receipt_path}`);
        if (c.receipt_verdict) lines.push(`        receipt_verdict=${c.receipt_verdict}`);
        if (Array.isArray(c.blockers) && c.blockers.length > 0) {
          for (const blocker of c.blockers) {
            lines.push(`        blocker: ${blocker}`);
          }
        }
      }
    }
  }
  if (result.info.length > 0) {
    lines.push(`  INFO: ${result.info.length}`);
    for (const i of result.info) {
      lines.push(`    ${i.wp_id} / ${i.mt_id || ""}: ${i.severity || ""}`);
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
    console.error(`[KB_READY_CHECKLIST_COVERAGE] ${error.message}`);
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
  const commsRootAbs = args.commsRoot
    ? path.resolve(args.commsRoot)
    : repoPathAbs(COMM_ROOT);

  const result = scanCoverage({ taskPacketsRootAbs, commsRootAbs, wpFilter: args.wp });

  if (args.json) {
    console.log(JSON.stringify({
      schema_id: "handshake.gov.kb_ready_checklist_coverage@1",
      schema_version: "kb_ready_checklist_coverage_v1",
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
      "kb-ready-checklist-coverage-check.mjs",
      `read/parse errors in ${result.read_errors.length} contract or receipt file(s)`,
      {
        role: "SHARED",
        details: result.read_errors.map((entry) => `${entry.path}: ${entry.error}`),
      },
    );
    if (!args.json) {
      console.error(
        `[kb-ready-checklist-coverage-check.mjs] read/parse errors in ${result.read_errors.length} file(s)`,
      );
      for (const entry of result.read_errors) {
        console.error(`  - ${entry.path}: ${entry.error}`);
      }
    }
    return EXIT_READ_ERROR;
  }
  if (result.concerns.length > 0) {
    captureFailure(
      "kb-ready-checklist-coverage-check.mjs",
      `KB_READY_CHECKLIST receipt coverage drift: ${result.concerns.length} concern(s)`,
      {
        role: "SHARED",
        details: result.concerns.map((c) =>
          `${c.wp_id}/${c.mt_id}: ${c.severity} (expected=${c.expected_receipt_path})`),
      },
    );
    if (!args.json) {
      console.error(
        `[kb-ready-checklist-coverage-check.mjs] ${result.concerns.length} concern(s) — MT(s) in READY_FOR_VALIDATION without a valid KB_READY_CHECKLIST receipt`,
      );
      for (const c of result.concerns) {
        console.error(`  - ${c.wp_id}/${c.mt_id}: ${c.severity}`);
      }
    }
    return EXIT_CONCERN;
  }
  if (!args.json) {
    console.log("kb-ready-checklist-coverage-check ok");
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
