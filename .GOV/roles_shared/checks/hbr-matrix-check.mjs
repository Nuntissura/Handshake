import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const BLOCKING_STATUSES = new Set(["PENDING", "STEER", "BLOCKED"]);

class CliError extends Error {
  constructor(message) {
    super(message);
    this.name = "CliError";
  }
}

class MalformedInputError extends Error {
  constructor(message, packet = null) {
    super(message);
    this.name = "MalformedInputError";
    this.packet = packet;
  }
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function normalizeStatus(status) {
  return String(status ?? "").trim().toUpperCase();
}

function parseArgs(args) {
  let packet = null;
  let allPackets = false;

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--packet") {
      if (packet !== null) throw new CliError("--packet may only be provided once");
      const value = args[index + 1];
      if (!isNonEmptyString(value)) throw new CliError("--packet requires a path");
      packet = path.resolve(value);
      index += 1;
      continue;
    }

    if (arg === "--all-packets") {
      if (allPackets) throw new CliError("--all-packets may only be provided once");
      allPackets = true;
      continue;
    }

    throw new CliError(`unknown argument: ${arg}`);
  }

  if ((packet === null && !allPackets) || (packet !== null && allPackets)) {
    throw new CliError("provide exactly one of --packet <path> or --all-packets");
  }

  return { packet, allPackets };
}

function listAllPacketPaths(root = process.cwd()) {
  const taskPacketsRoot = path.join(root, ".GOV", "task_packets");
  if (!fs.existsSync(taskPacketsRoot)) return [];

  return fs.readdirSync(taskPacketsRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(taskPacketsRoot, entry.name, "packet.json"))
    .filter((packetPath) => fs.existsSync(packetPath) && fs.statSync(packetPath).isFile())
    .sort((left, right) => left.localeCompare(right));
}

function readPacket(packetPath) {
  let text = "";
  try {
    text = fs.readFileSync(packetPath, "utf8");
  } catch (error) {
    throw new MalformedInputError(`unable to read packet: ${error.message}`, packetPath);
  }

  try {
    return JSON.parse(text);
  } catch (error) {
    throw new MalformedInputError(`invalid packet JSON: ${error.message}`, packetPath);
  }
}

function matchingNotApplicableLedgerEntry(ledger, hbrId) {
  return ledger.find((entry) => (
    entry &&
    typeof entry === "object" &&
    entry.hbr_id === hbrId &&
    isNonEmptyString(entry.reason)
  ));
}

function validateOpenBlockers(packet, packetPath) {
  if (packet.open_blockers === undefined) return [];
  if (!Array.isArray(packet.open_blockers)) {
    throw new MalformedInputError("open_blockers must be an array when present", packetPath);
  }

  const failures = [];
  for (let index = 0; index < packet.open_blockers.length; index += 1) {
    const blocker = packet.open_blockers[index];
    if (!blocker || typeof blocker !== "object" || Array.isArray(blocker)) {
      throw new MalformedInputError(`open_blockers[${index}] must be an object`, packetPath);
    }

    const hbrId = isNonEmptyString(blocker.hbr_id) ? blocker.hbr_id.trim() : "OPEN_BLOCKER";
    const surfaceName = isNonEmptyString(blocker.surface_name)
      ? blocker.surface_name.trim()
      : `open_blockers[${index}]`;
    const gapClass = isNonEmptyString(blocker.gap_class) ? blocker.gap_class.trim() : "unknown_gap_class";
    const blockerId = isNonEmptyString(blocker.blocker_id) ? blocker.blocker_id.trim() : `open_blockers[${index}]`;

    failures.push({
      hbr_id: hbrId,
      packet: packetPath,
      reason: `OPEN_BLOCKERS entry ${blockerId} blocks PASS closure until ${surfaceName} (${gapClass}) has an automation hook or a follow-up WP`,
      severity: "OPEN_BLOCKER",
    });
  }
  return failures;
}

function validatePacket(packet, packetPath) {
  if (!packet || typeof packet !== "object" || Array.isArray(packet)) {
    throw new MalformedInputError("packet JSON must be an object", packetPath);
  }

  const acceptanceMatrix = packet.acceptance_matrix;
  if (!acceptanceMatrix || typeof acceptanceMatrix !== "object" || Array.isArray(acceptanceMatrix)) {
    throw new MalformedInputError("acceptance_matrix must be an object", packetPath);
  }

  const hbr = acceptanceMatrix.hbr;
  if (!Array.isArray(hbr)) {
    throw new MalformedInputError("acceptance_matrix.hbr must be an array", packetPath);
  }

  const hbrNotApplicable = acceptanceMatrix.hbr_not_applicable ?? [];
  if (!Array.isArray(hbrNotApplicable)) {
    throw new MalformedInputError("acceptance_matrix.hbr_not_applicable must be an array when present", packetPath);
  }

  const failures = [];
  failures.push(...validateOpenBlockers(packet, packetPath));
  for (let index = 0; index < hbr.length; index += 1) {
    const row = hbr[index];
    if (!row || typeof row !== "object" || Array.isArray(row)) {
      throw new MalformedInputError(`acceptance_matrix.hbr[${index}] must be an object`, packetPath);
    }

    const hbrId = isNonEmptyString(row.hbr_id) ? row.hbr_id.trim() : `acceptance_matrix.hbr[${index}]`;
    const status = normalizeStatus(row.status);

    if (BLOCKING_STATUSES.has(status)) {
      failures.push({
        hbr_id: hbrId,
        packet: packetPath,
        reason: `HBR status ${status} is not accepted for closure`,
        severity: "MATRIX_VIOLATION",
      });
    }

    if (status === "PROVED") {
      if (!isNonEmptyString(row.evidence_pointer)) {
        failures.push({
          hbr_id: hbrId,
          packet: packetPath,
          reason: "PROVED HBR row requires a non-empty evidence_pointer",
          severity: "MATRIX_VIOLATION",
        });
      }

      if (row.validator_verdict !== "PROVED") {
        failures.push({
          hbr_id: hbrId,
          packet: packetPath,
          reason: 'PROVED HBR row requires validator_verdict === "PROVED"',
          severity: "MATRIX_VIOLATION",
        });
      }
    }

    if (status === "NOT_APPLICABLE" && !matchingNotApplicableLedgerEntry(hbrNotApplicable, hbrId)) {
      failures.push({
        hbr_id: hbrId,
        packet: packetPath,
        reason: "NOT_APPLICABLE HBR row requires matching acceptance_matrix.hbr_not_applicable entry with non-empty reason",
        severity: "MATRIX_VIOLATION",
      });
    }

    if (hbrId === "HBR-QUIET-004" && packet.requires_foreground !== true) {
      failures.push({
        hbr_id: hbrId,
        packet: packetPath,
        reason: "HBR-QUIET-004 requires packet.requires_foreground === true before any foreground exception run",
        severity: "MATRIX_VIOLATION",
      });
    }
  }

  return failures;
}

function hasHbrAcceptanceMatrix(packet) {
  return Boolean(packet && typeof packet === "object" && !Array.isArray(packet) && packet.acceptance_matrix);
}

function hbrEnforcementActive(root = process.cwd()) {
  const registryPath = path.join(root, ".GOV", "roles_shared", "records", "HANDSHAKE_BUILD_RULES.json");
  try {
    const registry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
    return normalizeStatus(registry?.status) === "ACTIVE" ||
      normalizeStatus(registry?.enforcement?.implementation_status) === "ACTIVE";
  } catch {
    return false;
  }
}

function emitJsonLines(records) {
  for (const record of records) {
    console.error(JSON.stringify(record));
  }
}

function cliErrorRecord(error) {
  return {
    hbr_id: null,
    packet: null,
    reason: error.message,
    severity: "CLI_ERROR",
  };
}

function malformedRecord(error) {
  return {
    hbr_id: null,
    packet: error.packet,
    reason: error.message,
    severity: "MALFORMED",
  };
}

function matrixCoverageGapRecord() {
  return {
    hbr_id: null,
    packet: null,
    reason: "--all-packets found 0 packets with acceptance_matrix.hbr while HBR enforcement is ACTIVE",
    severity: "MATRIX_COVERAGE_GAP",
  };
}

export function runCli(args = process.argv.slice(2), root = process.cwd()) {
  let options;
  try {
    options = parseArgs(args);
  } catch (error) {
    emitJsonLines([cliErrorRecord(error)]);
    return 3;
  }

  const packetPaths = options.allPackets ? listAllPacketPaths(root) : [options.packet];
  let checkedPacketCount = 0;
  const malformed = [];
  const violations = [];

  for (const packetPath of packetPaths) {
    try {
      const packet = readPacket(packetPath);
      if (options.allPackets && !hasHbrAcceptanceMatrix(packet)) {
        continue;
      }
      checkedPacketCount += 1;
      violations.push(...validatePacket(packet, packetPath));
    } catch (error) {
      if (error instanceof MalformedInputError) {
        malformed.push(malformedRecord(error));
        continue;
      }
      malformed.push({
        hbr_id: null,
        packet: packetPath,
        reason: error?.message || String(error),
        severity: "MALFORMED",
      });
    }
  }

  if (malformed.length > 0) {
    emitJsonLines(malformed);
    return 3;
  }

  if (options.allPackets && checkedPacketCount === 0 && hbrEnforcementActive(root)) {
    emitJsonLines([matrixCoverageGapRecord()]);
    return 2;
  }

  if (violations.length > 0) {
    emitJsonLines(violations);
    return 2;
  }

  const noun = checkedPacketCount === 1 ? "packet" : "packets";
  console.log(`hbr-matrix-check ok (${checkedPacketCount} ${noun})`);
  return 0;
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  const invoked = fs.realpathSync.native(path.resolve(process.argv[1]));
  const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
  return invoked === current;
}

const isMain = isInvokedAsMain();
if (isMain) {
  process.exitCode = runCli();
}
