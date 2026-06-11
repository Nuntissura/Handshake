#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  activeRules,
  loadRegistry,
} from "../scripts/hbr-registry-loader.mjs";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  normalizePath,
  repoPathAbs,
  REPO_ROOT,
} from "../scripts/lib/runtime-paths.mjs";

const PROVED = "PROVED";
const STEER = "STEER";
const PASS = "PASS";
const EXIT_OK = 0;
const EXIT_DOWNGRADED = 2;
const EXIT_USAGE_OR_MALFORMED = 3;
const SUPPORTED_SCHEMES = new Set(["test", "receipt", "artifact", "event"]);

function isPlainObject(value) {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function nonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function normalizeStatus(value) {
  return String(value ?? "").trim().toUpperCase();
}

function parseEvidencePointer(pointer) {
  const raw = String(pointer ?? "").trim();
  const match = raw.match(/^([a-z][a-z0-9+.-]*):\/\/(.+)$/i);
  if (!match) {
    return {
      ok: false,
      reason: `evidence_pointer must use one of ${[...SUPPORTED_SCHEMES].map((scheme) => `${scheme}://`).join(", ")}`,
    };
  }
  const scheme = match[1].toLowerCase();
  const value = decodeURIComponent(match[2]).trim();
  if (!SUPPORTED_SCHEMES.has(scheme)) {
    return {
      ok: false,
      reason: `unknown evidence_pointer scheme: ${scheme}://`,
    };
  }
  if (!value) {
    return {
      ok: false,
      reason: `evidence_pointer ${scheme}:// requires a non-empty target`,
    };
  }
  return { ok: true, scheme, value, raw };
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJsonAtomic(filePath, value) {
  const resolved = path.resolve(filePath);
  const tmpPath = path.join(
    path.dirname(resolved),
    `.${path.basename(resolved)}.${process.pid}.${Date.now()}.tmp`,
  );
  fs.writeFileSync(tmpPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  fs.renameSync(tmpPath, resolved);
}

function parseJsonl(filePath) {
  if (!filePath || !fs.existsSync(filePath)) return [];
  return fs.readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line, index) => {
      try {
        return JSON.parse(line);
      } catch (error) {
        return {
          __parse_error: `${filePath}:${index + 1}: ${error.message}`,
        };
      }
    });
}

function resolvePath(rootAbs, value) {
  const raw = String(value || "").trim();
  return path.isAbsolute(raw) ? path.resolve(raw) : path.resolve(rootAbs, raw);
}

function resolveRepoBoundPath(rootAbs, value) {
  const resolved = resolvePath(rootAbs, value);
  const relative = path.relative(rootAbs, resolved);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    return {
      ok: false,
      path: resolved,
      reason: `artifact path escapes repo root: ${normalizePath(value)}`,
    };
  }
  return { ok: true, path: resolved };
}

function collectSourceFiles(rootAbs) {
  const starts = [
    path.join(rootAbs, "src"),
    path.join(rootAbs, "app", "src"),
    path.join(rootAbs, ".GOV", "roles_shared"),
    path.join(rootAbs, ".GOV", "roles"),
  ];
  const wantedExtensions = new Set([".rs", ".mjs", ".js", ".ts", ".tsx"]);
  const files = [];
  const stack = starts.filter((entry) => fs.existsSync(entry));
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      if (["node_modules", "target", ".git"].includes(entry.name)) continue;
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(fullPath);
      } else if (entry.isFile() && wantedExtensions.has(path.extname(entry.name))) {
        files.push(fullPath);
      }
    }
  }
  return files;
}

function findTestEvidence(rootAbs, testName) {
  const escaped = testName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`\\b${escaped}\\b`);
  for (const filePath of collectSourceFiles(rootAbs)) {
    const text = fs.readFileSync(filePath, "utf8");
    if (pattern.test(text)) {
      return { filePath, text };
    }
  }
  return null;
}

function evidenceResultStatus(packet, evidencePointer) {
  const evidenceResults = Array.isArray(packet?.acceptance_matrix?.hbr_evidence_results)
    ? packet.acceptance_matrix.hbr_evidence_results
    : [];
  const match = evidenceResults.find((entry) =>
    String(entry?.evidence_pointer || "").trim() === evidencePointer
  );
  return normalizeStatus(match?.status);
}

function verifyTestPointer({ packet, pointer, row, rule, repoRootAbs }) {
  const found = findTestEvidence(repoRootAbs, pointer.value);
  if (!found) {
    return {
      ok: false,
      reason: `test evidence not found for ${pointer.raw}`,
    };
  }

  const status = evidenceResultStatus(packet, pointer.raw);
  if (status !== PASS) {
    return {
      ok: false,
      reason: `test evidence ${pointer.raw} has no recorded PASS result`,
    };
  }

  if (
    rule?.evidence_kind === "concurrent_session_test_run"
    && !/concurrent/i.test(pointer.value)
    && !found.text.includes("#[concurrent]")
  ) {
    return {
      ok: false,
      reason: `concurrent_session_test_run evidence must name a concurrent test or carry #[concurrent]: ${pointer.raw}`,
    };
  }

  return { ok: true };
}

function resolveReceiptsPath(packet, repoRootAbs) {
  const declared = String(packet?.workflow?.receipts_file || "").trim();
  if (declared) return resolvePath(repoRootAbs, declared);
  return path.join(
    GOVERNANCE_RUNTIME_ROOT_ABS,
    "roles_shared",
    "WP_COMMUNICATIONS",
    String(packet?.wp_id || "").trim(),
    "RECEIPTS.jsonl",
  );
}

function receiptMatches(receipt, id) {
  if (!isPlainObject(receipt)) return false;
  if ([receipt.receipt_id, receipt.id, receipt.correlation_id].some((value) => String(value || "") === id)) {
    return true;
  }
  if (Array.isArray(receipt.refs) && receipt.refs.some((ref) => String(ref || "") === `receipt://${id}` || String(ref || "") === id)) {
    return true;
  }
  return false;
}

function verifyReceiptPointer({ packet, pointer, repoRootAbs }) {
  const receiptsPath = resolveReceiptsPath(packet, repoRootAbs);
  const receipts = parseJsonl(receiptsPath);
  if (receipts.some((receipt) => receiptMatches(receipt, pointer.value))) {
    return { ok: true };
  }
  return {
    ok: false,
    reason: `receipt evidence not found for ${pointer.raw} in ${normalizePath(receiptsPath)}`,
  };
}

function verifyArtifactPointer({ pointer, repoRootAbs }) {
  const resolved = resolveRepoBoundPath(repoRootAbs, pointer.value);
  if (!resolved.ok) {
    return { ok: false, reason: resolved.reason };
  }
  if (!fs.existsSync(resolved.path) || !fs.statSync(resolved.path).isFile()) {
    return {
      ok: false,
      reason: `artifact evidence not found for ${pointer.raw}`,
    };
  }
  return { ok: true };
}

function eventRowsFromPacket(packet) {
  return Array.isArray(packet?.acceptance_matrix?.hbr_event_ledger)
    ? packet.acceptance_matrix.hbr_event_ledger
    : [];
}

function eventMatches(event, id) {
  return isPlainObject(event) && [event.event_id, event.id, event.kernel_event_id].some((value) => String(value || "") === id);
}

function verifyEventPointer({ packet, pointer, eventLedgerPath }) {
  const rows = [
    ...eventRowsFromPacket(packet),
    ...parseJsonl(eventLedgerPath),
  ];
  if (rows.some((event) => eventMatches(event, pointer.value))) {
    return { ok: true };
  }
  return {
    ok: false,
    reason: `EventLedger evidence not found for ${pointer.raw}`,
  };
}

function ruleMap(registryPath) {
  const registry = loadRegistry(registryPath);
  return new Map(activeRules(registry).map((rule) => [String(rule?.id || "").trim(), rule]));
}

function verifyPointer(context) {
  const pointer = parseEvidencePointer(context.row.evidence_pointer);
  if (!pointer.ok) return pointer;
  switch (pointer.scheme) {
    case "test":
      return verifyTestPointer({ ...context, pointer });
    case "receipt":
      return verifyReceiptPointer({ ...context, pointer });
    case "artifact":
      return verifyArtifactPointer({ ...context, pointer });
    case "event":
      return verifyEventPointer({ ...context, pointer });
    default:
      return {
        ok: false,
        reason: `unknown evidence_pointer scheme: ${pointer.scheme}://`,
      };
  }
}

function downgradeRow(row, reason, checkedAtUtc) {
  row.status = STEER;
  row.validator_verdict = STEER;
  row.steer_reason = reason;
  row.evidence_verification = {
    status: STEER,
    checked_at_utc: checkedAtUtc,
    reason,
  };
}

export function scanPacket(packet, options = {}) {
  if (!isPlainObject(packet)) {
    throw new Error("packet JSON must be an object");
  }
  if (packet.acceptance_matrix !== undefined && !isPlainObject(packet.acceptance_matrix)) {
    throw new Error("packet.acceptance_matrix must be an object when present");
  }
  if (
    packet.acceptance_matrix?.hbr !== undefined
    && !Array.isArray(packet.acceptance_matrix.hbr)
  ) {
    throw new Error("packet.acceptance_matrix.hbr must be an array when present");
  }

  const repoRootAbs = path.resolve(String(options.repoRoot || REPO_ROOT));
  const rules = options.rules || ruleMap(options.registryPath);
  const eventLedgerPath = options.eventLedgerPath
    || process.env.HANDSHAKE_HBR_EVENT_LEDGER_EXPORT
    || "";
  const checkedAtUtc = options.checkedAtUtc || new Date().toISOString();
  const findings = [];
  const hbrRows = Array.isArray(packet.acceptance_matrix?.hbr)
    ? packet.acceptance_matrix.hbr
    : [];

  for (const row of hbrRows) {
    if (!isPlainObject(row) || normalizeStatus(row.status) !== PROVED) continue;
    const hbrId = String(row.hbr_id || "").trim();
    const rule = rules.get(hbrId);
    if (!rule) {
      const reason = `HBR rule ${hbrId || "<missing>"} is not present in HANDSHAKE_BUILD_RULES active registry`;
      findings.push({ hbr_id: hbrId, reason });
      downgradeRow(row, reason, checkedAtUtc);
      continue;
    }
    if (!nonEmptyString(row.evidence_pointer)) {
      const reason = `PROVED HBR row ${hbrId} requires evidence_pointer`;
      findings.push({ hbr_id: hbrId, reason });
      downgradeRow(row, reason, checkedAtUtc);
      continue;
    }

    const verification = verifyPointer({
      packet,
      row,
      rule,
      repoRootAbs,
      eventLedgerPath,
    });
    if (!verification.ok) {
      findings.push({
        hbr_id: hbrId,
        evidence_kind: rule.evidence_kind,
        evidence_pointer: row.evidence_pointer,
        reason: verification.reason,
      });
      downgradeRow(row, verification.reason, checkedAtUtc);
    }
  }

  return {
    ok: findings.length === 0,
    findings,
    packet,
  };
}

export function scanPacketFile(packetPath, options = {}) {
  const resolvedPacketPath = path.resolve(String(packetPath || ""));
  if (!resolvedPacketPath || !fs.existsSync(resolvedPacketPath)) {
    throw new Error(`packet not found: ${packetPath}`);
  }
  const packet = readJson(resolvedPacketPath);
  const result = scanPacket(packet, options);
  if (!result.ok && !options.dryRun) {
    writeJsonAtomic(resolvedPacketPath, result.packet);
  }
  return result;
}

function parseArgs(argv) {
  const args = {
    packet: "",
    repoRoot: REPO_ROOT,
    eventLedgerPath: "",
    registryPath: "",
    dryRun: false,
    help: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--packet") {
      args.packet = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--repo-root") {
      args.repoRoot = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--event-ledger") {
      args.eventLedgerPath = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--registry") {
      args.registryPath = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--dry-run") {
      args.dryRun = true;
    } else if (arg === "--help" || arg === "-h") {
      args.help = true;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return args;
}

function usage() {
  console.error("Usage: node hbr-validator-scan.mjs --packet <path> [--repo-root <path>] [--event-ledger <jsonl>] [--registry <path>] [--dry-run]");
}

function emitFindings(findings) {
  for (const finding of findings) {
    console.error(JSON.stringify({
      severity: "HBR_EVIDENCE_STEER",
      ...finding,
    }));
  }
}

export function runCli(argv = process.argv.slice(2)) {
  let args;
  try {
    args = parseArgs(argv);
  } catch (error) {
    console.error(`[HBR_VALIDATOR_SCAN] ${error.message}`);
    return EXIT_USAGE_OR_MALFORMED;
  }
  if (args.help) {
    usage();
    return EXIT_OK;
  }
  if (!args.packet) {
    usage();
    return EXIT_USAGE_OR_MALFORMED;
  }

  try {
    const result = scanPacketFile(args.packet, {
      repoRoot: args.repoRoot,
      eventLedgerPath: args.eventLedgerPath,
      registryPath: args.registryPath,
      dryRun: args.dryRun,
    });
    if (!result.ok) {
      emitFindings(result.findings);
      return EXIT_DOWNGRADED;
    }
    console.log("hbr-validator-scan ok");
    return EXIT_OK;
  } catch (error) {
    console.error(`[HBR_VALIDATOR_SCAN] ${error instanceof Error ? error.message : String(error)}`);
    return EXIT_USAGE_OR_MALFORMED;
  }
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  const invoked = fs.realpathSync.native(path.resolve(process.argv[1]));
  const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
  return invoked === current;
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
