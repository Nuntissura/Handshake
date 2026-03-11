import fs from "node:fs";
import path from "node:path";

const PACKETS_DIR = path.join(".GOV", "task_packets");
const COMM_ROOT = ".GOV/roles_shared/WP_COMMUNICATIONS";

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

function normalize(value) {
  return String(value || "").replace(/\\/g, "/").trim();
}

const violations = [];

if (fs.existsSync(PACKETS_DIR)) {
  const files = fs.readdirSync(PACKETS_DIR).filter((name) => name.endsWith(".md"));
  for (const name of files) {
    const wpId = name.slice(0, -3);
    const packetPath = path.join(PACKETS_DIR, name);
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

    const expectedDir = `${COMM_ROOT}/${wpId}`;
    const expectedThread = `${expectedDir}/THREAD.md`;
    const expectedRuntime = `${expectedDir}/RUNTIME_STATUS.json`;
    const expectedReceipts = `${expectedDir}/RECEIPTS.md`;

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
  }
}

if (violations.length > 0) {
  fail("WP communication artifact drift detected", violations);
}

console.log("wp-communications-check ok");
