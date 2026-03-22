import fs from "node:fs";
import path from "node:path";
import {
  isAllowedPrimaryOrFallbackModel,
  isDisallowedCodexModelAlias,
  packetUsesSessionPolicy,
  ROLE_SESSION_REASONING_REQUIRED,
} from "../scripts/session/session-policy.mjs";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";
import {
  hasConcreteScopeEntries,
  hasScopeOverlap,
  parsePacketScopeList,
} from "../scripts/lib/scope-surface-lib.mjs";

// Canonical governance workspace packets live under `/.GOV/task_packets/`.
// Legacy compatibility bundles must not be treated as governance SSoT.
const TASK_PACKETS_DIR = path.join(GOV_ROOT_REPO_REL, "task_packets");

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
  const isClaimedPacket = /in\s*progress/.test(statusNorm);
  const isStartablePacket = /ready\s*for\s*dev|in\s*progress/.test(statusNorm);
  if (!isStartablePacket) return;

  const packetFormatVersion = parseSingleField(text, "PACKET_FORMAT_VERSION");
  const coderModel = parseSingleField(text, "CODER_MODEL");
  const coderStrength = parseSingleField(text, "CODER_REASONING_STRENGTH");
  const enforceSessionPolicy = packetUsesSessionPolicy(packetFormatVersion);
  const enforceScopeContract = Boolean(packetFormatVersion);
  const inScopePaths = parsePacketScopeList(text, "IN_SCOPE_PATHS", { stopLabels: ["OUT_OF_SCOPE"] });
  const outOfScopePaths = parsePacketScopeList(text, "OUT_OF_SCOPE");

  const rel = filePath.split(path.sep).join("/");
  const errors = [];

  if (isClaimedPacket && isPlaceholder(coderModel)) {
    errors.push(`${rel}: CODER_MODEL is required when Status is In Progress`);
  } else if (isClaimedPacket && enforceSessionPolicy) {
    if (isDisallowedCodexModelAlias(coderModel)) {
      errors.push(`${rel}: CODER_MODEL must use the repo-approved GPT model ids, not Codex model aliases (got: ${coderModel})`);
    } else if (!isAllowedPrimaryOrFallbackModel(coderModel)) {
      errors.push(`${rel}: CODER_MODEL must be the repo-approved primary/fallback model for new packets (got: ${coderModel})`);
    }
  }

  if (isClaimedPacket && isPlaceholder(coderStrength)) {
    errors.push(`${rel}: CODER_REASONING_STRENGTH is required when Status is In Progress`);
  } else if (isClaimedPacket) {
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

  if (enforceScopeContract && !hasConcreteScopeEntries(inScopePaths)) {
    errors.push(`${rel}: IN_SCOPE_PATHS must list at least one concrete write surface when Status is Ready for Dev or In Progress`);
  }
  if (enforceScopeContract && !/OUT_OF_SCOPE/i.test(text)) {
    errors.push(`${rel}: OUT_OF_SCOPE section is required when Status is Ready for Dev or In Progress`);
  }
  const overlap = hasScopeOverlap(inScopePaths, outOfScopePaths);
  if (enforceScopeContract && overlap) {
    errors.push(
      `${rel}: IN_SCOPE_PATHS and OUT_OF_SCOPE overlap (${overlap.left} <-> ${overlap.right})`
    );
  }

  if (errors.length > 0) fail("Coder claim fields missing/invalid", errors);
}

function collectPacketFiles(rootDir) {
  const results = [];
  const stack = [{ dir: rootDir, nested: false }];
  while (stack.length > 0) {
    const current = stack.pop();
    if (!current?.dir) continue;
    for (const entry of fs.readdirSync(current.dir, { withFileTypes: true })) {
      const full = path.join(current.dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === "stubs") continue;
        stack.push({ dir: full, nested: true });
        continue;
      }
      if (!entry.isFile()) continue;
      if (entry.name === "README.md") continue;
      if ((!current.nested && entry.name.endsWith(".md")) || entry.name === "packet.md") {
        results.push(full);
      }
    }
  }
  return results;
}

function main() {
  if (!fs.existsSync(TASK_PACKETS_DIR)) return;
  const files = collectPacketFiles(TASK_PACKETS_DIR);

  for (const filePath of files) checkPacket(filePath);
  console.log("task-packet-claim-check ok");
}

main();
