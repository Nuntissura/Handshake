import fs from "node:fs";
import path from "node:path";

// Canonical governance workspace packets live under `/.GOV/task_packets/`.
// Legacy compatibility bundles must not be treated as governance SSoT.
const TASK_PACKETS_DIR = path.join(".GOV", "task_packets");

function fail(message, details = []) {
  console.error(`[TASK_PACKET_CLAIM_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function isPlaceholder(value) {
  const v = (value || "").trim();
  if (!v) return true;
  if (/^\{.+\}$/.test(v)) return true;
  if (/^<fill/i.test(v)) return true;
  if (/^<pending>$/i.test(v)) return true;
  if (/^<unclaimed>$/i.test(v)) return true;
  return false;
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const m = text.match(re);
  return m ? m[1].trim() : "";
}

function parseStatus(text) {
  const statusLine =
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1] ||
    "";
  return statusLine.trim();
}

function normalizeStrength(value) {
  return value.toLowerCase().replace(/[\s_-]+/g, "");
}

function checkPacket(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const status = parseStatus(text);
  const statusNorm = status.toLowerCase();
  if (!/in\s*progress/.test(statusNorm)) return;

  const coderModel = parseSingleField(text, "CODER_MODEL");
  const coderStrength = parseSingleField(text, "CODER_REASONING_STRENGTH");

  const rel = filePath.split(path.sep).join("/");
  const errors = [];

  if (isPlaceholder(coderModel)) {
    errors.push(`${rel}: CODER_MODEL is required when Status is In Progress`);
  }

  if (isPlaceholder(coderStrength)) {
    errors.push(`${rel}: CODER_REASONING_STRENGTH is required when Status is In Progress`);
  } else {
    const norm = normalizeStrength(coderStrength);
    const allowed = new Set(["low", "medium", "high", "extrahigh"]);
    if (!allowed.has(norm)) {
      errors.push(
        `${rel}: CODER_REASONING_STRENGTH must be LOW|MEDIUM|HIGH|EXTRA_HIGH (got: ${coderStrength})`
      );
    }
  }

  if (errors.length > 0) fail("Coder claim fields missing/invalid", errors);
}

function main() {
  if (!fs.existsSync(TASK_PACKETS_DIR)) return;
  const files = fs
    .readdirSync(TASK_PACKETS_DIR)
    .filter((name) => name.endsWith(".md"))
    .map((name) => path.join(TASK_PACKETS_DIR, name));

  for (const filePath of files) checkPacket(filePath);
  console.log("task-packet-claim-check ok");
}

main();
