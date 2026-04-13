import fs from "node:fs";
import { validateSemanticProofAssets } from "../scripts/lib/semantic-proof-lib.mjs";
import { listOfficialWorkPacketPaths, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("semantic-proof-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("semantic-proof-check.mjs", message, { role: "SHARED", details });
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

const violations = [];
const files = listOfficialWorkPacketPaths();

for (const rel of files) {
  const text = fs.readFileSync(repoPathAbs(rel), "utf8");
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
