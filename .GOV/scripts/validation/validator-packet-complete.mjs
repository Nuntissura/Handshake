#!/usr/bin/env node
/**
 * Packet completeness checker for validators.
 * Ensures required fields are present and sane.
 */
import { readFileSync } from "node:fs";

const wpId = process.argv[2];
if (!wpId) {
  console.error("Usage: just validator-packet-complete WP-1-Example");
  process.exit(1);
}

const packetPath = `.GOV/task_packets/${wpId}.md`;

function fail(msg) {
  console.error(`validator-packet-complete: FAIL - ${msg}`);
  process.exit(1);
}

let text;
try {
  text = readFileSync(packetPath, "utf8");
} catch (err) {
  fail(`cannot read ${packetPath}: ${err.message}`);
}

const lines = text.split(/\r?\n/);

function hasLine(re) {
  return re.test(text);
}

function isPlaceholder(value) {
  const v = (value || "").trim();
  if (!v) return true;
  if (/^\{.+\}$/.test(v)) return true;
  if (/^<fill/i.test(v)) return true;
  if (/^<pending>$/i.test(v)) return true;
  if (/^<unclaimed>$/i.test(v)) return true;
  if (/^tbd$/i.test(v)) return true;
  return false;
}

function parseSingleField(label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.*)\\s*$`, "i");
  for (const line of lines) {
    const m = line.match(re);
    if (m) return (m[1] ?? "").trim();
  }
  return "";
}

function hasNonPlaceholderListItemAfterLabel(label) {
  const labelRe = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*$`, "i");
  const topLevelBulletRe = /^\s*-\s*[A-Z0-9_]+\s*:/i;
  const sectionHeaderRe = /^\s*##\s+/;

  const labelIdx = lines.findIndex((line) => labelRe.test(line));
  if (labelIdx === -1) return false;

  for (let i = labelIdx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (sectionHeaderRe.test(line)) break;
    if (topLevelBulletRe.test(line)) break;

    const m = line.match(/^\s*-\s+(.+)\s*$/);
    if (!m) continue;
    const v = (m[1] ?? "").trim().replace(/^`|`$/g, "");
    if (!isPlaceholder(v)) return true;
  }

  return false;
}

if (!hasLine(/(?:\*\*Status:\*\*|STATUS:)\s*(Ready for Dev|In Progress|Done(?:\s*\(Historical\))?)\b/i)) {
  fail("STATUS missing or invalid (must be Ready for Dev / In Progress / Done / Done (Historical))");
}

const hasLegacySpec = hasLine(/SPEC_CURRENT/i);
const hasSpecBaseline = hasLine(/SPEC_BASELINE/i);
const hasSpecTarget = hasLine(/SPEC_TARGET/i);
if (!hasLegacySpec && !(hasSpecBaseline && hasSpecTarget)) {
  fail("SPEC reference missing (need SPEC_CURRENT or SPEC_BASELINE+SPEC_TARGET)");
}
if (!hasLine(/RISK_TIER/i)) {
  fail("RISK_TIER missing");
}
if (!hasLine(/DONE_MEANS/i) || hasLine(/DONE_MEANS\s*:\s*$/i) || hasLine(/DONE_MEANS\s*:\s*tbd/i)) {
  fail("DONE_MEANS missing or placeholder");
}
if (!hasLine(/TEST_PLAN/i) || hasLine(/TEST_PLAN\s*:\s*$/i) || hasLine(/TEST_PLAN\s*:\s*tbd/i)) {
  fail("TEST_PLAN missing or placeholder");
}
if (!hasLine(/BOOTSTRAP/i)) {
  fail("BOOTSTRAP missing");
}
if (!hasLine(/USER_SIGNATURE/i) && !hasLine(/User Signature Locked/i)) {
  fail("USER_SIGNATURE missing");
}

// Newer template-only requirements (avoid breaking legacy packets).
const packetFormatVersion = parseSingleField("PACKET_FORMAT_VERSION");
if (packetFormatVersion) {
  if (isPlaceholder(packetFormatVersion)) {
    fail("PACKET_FORMAT_VERSION present but placeholder");
  }

  if (!hasLine(/^##\s*END_TO_END_CLOSURE_PLAN\b/im)) {
    fail("END_TO_END_CLOSURE_PLAN section missing (required for PACKET_FORMAT_VERSION packets)");
  }

  const applicable = parseSingleField("END_TO_END_CLOSURE_PLAN_APPLICABLE");
  if (!/^(YES|NO)$/i.test(applicable)) {
    fail("END_TO_END_CLOSURE_PLAN_APPLICABLE missing/invalid (must be YES or NO)");
  }

  if (/^YES$/i.test(applicable)) {
    const trustBoundary = parseSingleField("TRUST_BOUNDARY");
    if (isPlaceholder(trustBoundary)) {
      fail("TRUST_BOUNDARY missing/placeholder (required when END_TO_END_CLOSURE_PLAN_APPLICABLE is YES)");
    }

    const requiredLists = [
      "SERVER_SOURCES_OF_TRUTH",
      "REQUIRED_PROVENANCE_FIELDS",
      "VERIFICATION_PLAN",
      "ERROR_TAXONOMY_PLAN",
      "UI_GUARDRAILS",
      "VALIDATOR_ASSERTIONS",
    ];

    for (const label of requiredLists) {
      if (!hasNonPlaceholderListItemAfterLabel(label)) {
        fail(`${label} missing/placeholder list items (required when END_TO_END_CLOSURE_PLAN_APPLICABLE is YES)`);
      }
    }
  }
}

console.log(`validator-packet-complete: PASS - ${wpId} has required fields.`);

