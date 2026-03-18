import fs from "node:fs";
import path from "node:path";
import { validateSemanticProofAssets } from "../scripts/lib/semantic-proof-lib.mjs";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";

const PACKETS_DIR = path.join(GOV_ROOT_REPO_REL, "task_packets");

function fail(message, details = []) {
  console.error(`[SEMANTIC_PROOF_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function parseStatus(text) {
  return (
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1] ||
    ""
  ).trim();
}

function isClosedStatus(status) {
  return /\b(done|validated)\b/i.test(String(status || ""));
}

if (!fs.existsSync(PACKETS_DIR)) {
  fail("Task packet directory missing", [PACKETS_DIR.replace(/\\/g, "/")]);
}

const violations = [];
const files = fs.readdirSync(PACKETS_DIR).filter((name) => name.endsWith(".md") && name !== "README.md");

for (const name of files) {
  const rel = path.join(PACKETS_DIR, name).replace(/\\/g, "/");
  const text = fs.readFileSync(rel, "utf8");
  const packetFormatVersion = parseSingleField(text, "PACKET_FORMAT_VERSION");
  const semanticProofProfile = parseSingleField(text, "SEMANTIC_PROOF_PROFILE");
  const usesSemanticProofProfile = /^DIFF_SCOPED_SEMANTIC_V1$/i.test(semanticProofProfile);
  const closed = isClosedStatus(parseStatus(text));

  if (packetFormatVersion >= "2026-03-16" && !closed && !usesSemanticProofProfile) {
    violations.push(`${rel}: active 2026-03-16+ packet missing SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`);
    continue;
  }

  if (!usesSemanticProofProfile) continue;

  const semanticProofValidation = validateSemanticProofAssets(text);
  for (const error of semanticProofValidation.errors) {
    violations.push(`${rel}: ${error}`);
  }
}

if (violations.length > 0) {
  fail("Semantic proof violations found", violations);
}

console.log("semantic-proof-check ok");


