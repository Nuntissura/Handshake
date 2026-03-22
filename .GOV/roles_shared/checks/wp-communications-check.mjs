import fs from "node:fs";
import path from "node:path";
import {
  COMM_ROOT,
  communicationPathsForWp,
  ensureSchemaFilesExist,
  legacyCommunicationPathsForWp,
  normalize,
  parseJsonFile,
  parseJsonlFile,
  RECEIPTS_FILE_NAME,
  RUNTIME_STATUS_FILE_NAME,
  THREAD_FILE_NAME,
  validateReceipt,
  validateRuntimeStatus,
} from "../scripts/lib/wp-communications-lib.mjs";
import { GOV_ROOT_REPO_REL, resolveWorkPacketPath } from "../scripts/lib/runtime-paths.mjs";

const PACKETS_DIR = path.join(GOV_ROOT_REPO_REL, "task_packets");

function fail(message, details = []) {
  console.error(`[WP_COMMUNICATIONS_CHECK] ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(1);
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

const violations = [];
ensureSchemaFilesExist();

if (fs.existsSync(PACKETS_DIR)) {
  const entries = fs.readdirSync(PACKETS_DIR, { withFileTypes: true });
  for (const entry of entries) {
    let wpId = "";
    let packetPath = "";
    if (entry.isDirectory()) {
      if (!entry.name.startsWith("WP-")) continue;
      wpId = entry.name;
      packetPath = path.join(PACKETS_DIR, entry.name, "packet.md");
      if (!fs.existsSync(packetPath)) continue;
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      wpId = entry.name.slice(0, -3);
      packetPath = path.join(PACKETS_DIR, entry.name);
    } else {
      continue;
    }
    const text = fs.readFileSync(packetPath, "utf8");

    const communicationDir = parseSingleField(text, "WP_COMMUNICATION_DIR");
    const threadFile = parseSingleField(text, "WP_THREAD_FILE");
    const runtimeStatusFile = parseSingleField(text, "WP_RUNTIME_STATUS_FILE");
    const receiptsFile = parseSingleField(text, "WP_RECEIPTS_FILE");
    const declared = [communicationDir, threadFile, runtimeStatusFile, receiptsFile].filter(Boolean);
    if (declared.length === 0) continue;

    if (declared.length !== 4) {
      violations.push(`${packetPath}: packet declares only part of the WP communication metadata set`);
      continue;
    }

    const expected = communicationPathsForWp(wpId);
    const expectedDir = expected.dir;
    const expectedThread = expected.threadFile;
    const expectedRuntime = expected.runtimeStatusFile;
    const expectedReceipts = expected.receiptsFile;

    if (normalize(communicationDir) !== normalize(expectedDir)) {
      violations.push(`${packetPath}: WP_COMMUNICATION_DIR must be ${expectedDir} (got ${communicationDir})`);
    }
    if (normalize(threadFile) !== normalize(expectedThread)) {
      violations.push(`${packetPath}: WP_THREAD_FILE must be ${expectedThread} (got ${threadFile})`);
    }
    if (normalize(runtimeStatusFile) !== normalize(expectedRuntime)) {
      violations.push(`${packetPath}: WP_RUNTIME_STATUS_FILE must be ${expectedRuntime} (got ${runtimeStatusFile})`);
    }
    if (normalize(receiptsFile) !== normalize(expectedReceipts)) {
      violations.push(`${packetPath}: WP_RECEIPTS_FILE must be ${expectedReceipts} (got ${receiptsFile})`);
    }

    for (const requiredPath of [expectedDir, expectedThread, expectedRuntime, expectedReceipts]) {
      if (!fs.existsSync(requiredPath)) {
        violations.push(`${packetPath}: referenced communication artifact missing on disk -> ${requiredPath}`);
      }
    }

    if (fs.existsSync(expectedRuntime)) {
      try {
        const runtimeStatus = parseJsonFile(expectedRuntime);
        const runtimeErrors = validateRuntimeStatus(runtimeStatus);
        for (const error of runtimeErrors) {
          violations.push(`${packetPath}: ${RUNTIME_STATUS_FILE_NAME} invalid -> ${error}`);
        }
      } catch (error) {
        violations.push(`${packetPath}: ${RUNTIME_STATUS_FILE_NAME} parse/validation failure -> ${error.message}`);
      }
    }

    if (fs.existsSync(expectedReceipts)) {
      try {
        const receipts = parseJsonlFile(expectedReceipts);
        if (receipts.length === 0) {
          violations.push(`${packetPath}: ${RECEIPTS_FILE_NAME} must contain at least one receipt entry`);
        }
        receipts.forEach((entry, index) => {
          const receiptErrors = validateReceipt(entry);
          for (const error of receiptErrors) {
            violations.push(`${packetPath}: ${RECEIPTS_FILE_NAME} line ${index + 1} invalid -> ${error}`);
          }
        });
      } catch (error) {
        violations.push(`${packetPath}: ${RECEIPTS_FILE_NAME} parse/validation failure -> ${error.message}`);
      }
    }

    const legacyDir = legacyCommunicationPathsForWp(wpId).dir;
    if (fs.existsSync(legacyDir)) {
      violations.push(`${packetPath}: repo-local legacy communication directory still exists -> ${legacyDir}`);
    }
  }
}

if (fs.existsSync(COMM_ROOT)) {
  const entries = fs.readdirSync(COMM_ROOT, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    if (!entry.name.startsWith("WP-")) {
      violations.push(`${COMM_ROOT}/${entry.name}: unexpected directory in WP communication root`);
      continue;
    }
    const packetPath = resolveWorkPacketPath(entry.name)?.packetPath || path.join(PACKETS_DIR, `${entry.name}.md`);
    if (!fs.existsSync(packetPath)) {
      violations.push(`${COMM_ROOT}/${entry.name}: orphan communication folder with no matching official packet`);
      continue;
    }
    const packetText = fs.readFileSync(packetPath, "utf8");
    const communicationDir = parseSingleField(packetText, "WP_COMMUNICATION_DIR");
    const authoritativeDir = normalize(communicationPathsForWp(entry.name).dir);
    const currentDir = normalize(path.join(COMM_ROOT, entry.name));

    if (currentDir !== authoritativeDir) {
      continue;
    }

    if (!communicationDir) {
      violations.push(`${COMM_ROOT}/${entry.name}: communication folder exists but matching packet does not declare WP communication metadata`);
    }
    for (const requiredName of [THREAD_FILE_NAME, RUNTIME_STATUS_FILE_NAME, RECEIPTS_FILE_NAME]) {
      const requiredPath = path.join(COMM_ROOT, entry.name, requiredName);
      if (!fs.existsSync(requiredPath)) {
        violations.push(`${COMM_ROOT}/${entry.name}: missing required artifact ${requiredName}`);
      }
    }
  }
}

const legacyRoot = legacyCommunicationPathsForWp("WP-LEGACY").dir.replace(/\/WP-LEGACY$/, "");
if (fs.existsSync(legacyRoot)) {
  const entries = fs.readdirSync(legacyRoot, { withFileTypes: true });
  for (const entry of entries) {
    violations.push(`${legacyRoot}/${entry.name}: repo-local WP communication runtime residue detected; live artifacts must live under ${COMM_ROOT}`);
  }
}

if (violations.length > 0) {
  fail("WP communication artifact drift detected", violations);
}

console.log("wp-communications-check ok");
