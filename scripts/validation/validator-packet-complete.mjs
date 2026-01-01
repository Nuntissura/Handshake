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

const path = `docs/task_packets/${wpId}.md`;

function fail(msg) {
  console.error(`validator-packet-complete: FAIL — ${msg}`);
  process.exit(1);
}

let text;
try {
  text = readFileSync(path, "utf8");
} catch (err) {
  fail(`cannot read ${path}: ${err.message}`);
}

function hasLine(re) {
  return re.test(text);
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

console.log(`validator-packet-complete: PASS — ${wpId} has required fields.`);
