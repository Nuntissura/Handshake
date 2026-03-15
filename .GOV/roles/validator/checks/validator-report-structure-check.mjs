import fs from "node:fs";
import path from "node:path";
import { packetUsesStructuredValidationReport } from "../../../roles_shared/scripts/session/session-policy.mjs";

const PACKETS_DIR = path.join(".GOV", "task_packets");

function fail(message, details = []) {
  console.error(`[VALIDATOR_REPORT_STRUCTURE_CHECK] ${message}`);
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

function extractSectionAfterHeading(text, heading) {
  const lines = String(text || "").split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${heading}\\b`, "i");
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) return "";

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }
  return lines.slice(startIndex + 1, endIndex).join("\n");
}

function parseSectionField(sectionText, label) {
  const re = new RegExp(`^\\s*${label}\\s*:\\s*(.+)\\s*$`, "im");
  const match = String(sectionText || "").match(re);
  return match ? match[1].trim() : "";
}

function extractListItemsAfterLabel(sectionText, label) {
  const lines = String(sectionText || "").split(/\r?\n/);
  const labelRe = new RegExp(`^\\s*${label}\\s*:\\s*$`, "i");
  const headingRe = /^#{1,6}\s+\S/;
  const nextLabelRe = /^\s*[A-Z][A-Z0-9_ ()/-]*\s*:\s*$/;

  const labelIdx = lines.findIndex((line) => labelRe.test(line));
  if (labelIdx === -1) return [];

  const items = [];
  for (let index = labelIdx + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (headingRe.test(line)) break;
    if (nextLabelRe.test(line)) break;
    const match = line.match(/^\s*-\s+(.+)\s*$/);
    if (match) items.push((match[1] || "").trim());
  }
  return items;
}

function isClosedStatus(status) {
  return /\b(done|validated)\b/i.test(String(status || ""));
}

if (!fs.existsSync(PACKETS_DIR)) {
  fail("Task packet directory missing", [PACKETS_DIR.replace(/\\/g, "/")]);
}

const files = fs.readdirSync(PACKETS_DIR).filter((name) => name.endsWith(".md") && name !== "README.md");
const violations = [];

for (const name of files) {
  const rel = path.join(PACKETS_DIR, name).replace(/\\/g, "/");
  const text = fs.readFileSync(rel, "utf8");
  const packetFormatVersion = parseSingleField(text, "PACKET_FORMAT_VERSION");
  if (!packetUsesStructuredValidationReport(packetFormatVersion)) continue;

  const status = parseStatus(text);
  if (!isClosedStatus(status)) continue;

  const reports = extractSectionAfterHeading(text, "VALIDATION_REPORTS");
  if (!reports.trim()) {
    violations.push(`${rel}: VALIDATION_REPORTS missing/empty for closed packet`);
    continue;
  }

  const requiredFields = [
    "VALIDATION_CONTEXT",
    "GOVERNANCE_VERDICT",
    "TEST_VERDICT",
    "CODE_REVIEW_VERDICT",
    "SPEC_ALIGNMENT_VERDICT",
    "ENVIRONMENT_VERDICT",
    "DISPOSITION",
    "LEGAL_VERDICT",
    "SPEC_CONFIDENCE",
  ];

  for (const label of requiredFields) {
    const value = parseSectionField(reports, label);
    if (!value) {
      violations.push(`${rel}: ${label} missing in VALIDATION_REPORTS`);
    }
  }

  if (!/^\s*Verdict\s*:\s*(PASS|FAIL|OUTDATED_ONLY)\b/im.test(reports)) {
    violations.push(`${rel}: VALIDATION_REPORTS missing top-level Verdict: PASS|FAIL|OUTDATED_ONLY`);
  }

  const clausesReviewed = extractListItemsAfterLabel(reports, "CLAUSES_REVIEWED");
  if (clausesReviewed.length === 0) {
    violations.push(`${rel}: CLAUSES_REVIEWED missing bullet items in VALIDATION_REPORTS`);
  }

  const notProven = extractListItemsAfterLabel(reports, "NOT_PROVEN");
  if (notProven.length === 0) {
    violations.push(`${rel}: NOT_PROVEN missing bullet items in VALIDATION_REPORTS`);
  }

  const specAlignmentVerdict = parseSectionField(reports, "SPEC_ALIGNMENT_VERDICT").toUpperCase();
  if (specAlignmentVerdict === "PASS") {
    const hasOnlyNone = notProven.length === 1 && notProven[0].toUpperCase() === "NONE";
    if (!hasOnlyNone) {
      violations.push(`${rel}: SPEC_ALIGNMENT_VERDICT=PASS requires NOT_PROVEN to be exactly "- NONE"`);
    }
  }
}

if (violations.length > 0) {
  fail("Structured validator report violations found", violations);
}

console.log("validator-report-structure-check ok");



