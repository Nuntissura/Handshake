import fs from "node:fs";
import path from "node:path";
import {
  isAllowedPrimaryOrFallbackModel,
  isDisallowedCodexModelAlias,
  packetUsesSessionPolicy,
  ROLE_SESSION_REASONING_REQUIRED,
} from "../scripts/session/session-policy.mjs";

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
  // Treat "<pending> (notes)" or "<unclaimed> (options)" as placeholders too.
  if (/^<fill/i.test(v)) return true;
  if (/^<pending>/i.test(v)) return true;
  if (/^<unclaimed>/i.test(v)) return true;
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

function extractStrengthToken(value) {
  const v = String(value || "").trim();
  // Allow trailing notes like: "HIGH (EXTRA_HIGH also acceptable)" while still enforcing the leading token.
  return (v.split(/[\s(]/)[0] || "").trim();
}

function checkPacket(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const status = parseStatus(text);
  const statusNorm = status.toLowerCase();
  if (!/in\s*progress/.test(statusNorm)) return;

  const packetFormatVersion = parseSingleField(text, "PACKET_FORMAT_VERSION");
  const coderModel = parseSingleField(text, "CODER_MODEL");
  const coderStrength = parseSingleField(text, "CODER_REASONING_STRENGTH");
  const enforceSessionPolicy = packetUsesSessionPolicy(packetFormatVersion);

  const rel = filePath.split(path.sep).join("/");
  const errors = [];

  if (isPlaceholder(coderModel)) {
    errors.push(`${rel}: CODER_MODEL is required when Status is In Progress`);
  } else if (enforceSessionPolicy) {
    if (isDisallowedCodexModelAlias(coderModel)) {
      errors.push(`${rel}: CODER_MODEL must use the repo-approved GPT model ids, not Codex model aliases (got: ${coderModel})`);
    } else if (!isAllowedPrimaryOrFallbackModel(coderModel)) {
      errors.push(`${rel}: CODER_MODEL must be the repo-approved primary/fallback model for new packets (got: ${coderModel})`);
    }
  }

  if (isPlaceholder(coderStrength)) {
    errors.push(`${rel}: CODER_REASONING_STRENGTH is required when Status is In Progress`);
  } else {
    const token = extractStrengthToken(coderStrength);
    const norm = normalizeStrength(token);
    const allowed = new Set(["low", "medium", "high", "extrahigh"]);
    if (!allowed.has(norm)) {
      errors.push(
        `${rel}: CODER_REASONING_STRENGTH must be LOW|MEDIUM|HIGH|EXTRA_HIGH (got: ${coderStrength})`
      );
    } else if (enforceSessionPolicy && norm !== "extrahigh") {
      errors.push(
        `${rel}: CODER_REASONING_STRENGTH must be ${ROLE_SESSION_REASONING_REQUIRED} for PACKET_FORMAT_VERSION >= 2026-03-12 (got: ${coderStrength})`
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


