import fs from "node:fs";
import { validateClauseReportConsistency, validatePacketClosureMonitoring } from "../scripts/lib/packet-closure-monitor-lib.mjs";
import { listOfficialWorkPacketPaths } from "../scripts/lib/runtime-paths.mjs";

function fail(message, details = []) {
  console.error(`[PACKET_CLOSURE_MONITOR_CHECK] ${message}`);
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

const violations = [];
const files = listOfficialWorkPacketPaths();

for (const rel of files) {
  const text = fs.readFileSync(rel, "utf8");
  const packetFormatVersion = parseSingleField(text, "PACKET_FORMAT_VERSION");
  const closureMonitorProfile = parseSingleField(text, "CLAUSE_CLOSURE_MONITOR_PROFILE");
  const usesClauseMonitor = /^CLAUSE_MONITOR_V1$/i.test(closureMonitorProfile);
  const status = parseStatus(text);
  const closed = isClosedStatus(status);

  if (packetFormatVersion >= "2026-03-15" && !closed && !usesClauseMonitor) {
    violations.push(`${rel}: active 2026-03-15+ packet missing CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`);
    continue;
  }

  if (!usesClauseMonitor) continue;

  const monitorValidation = validatePacketClosureMonitoring(text, {
    requireRows: true,
    requireClosedConsistency: closed,
  });
  for (const error of monitorValidation.errors) {
    violations.push(`${rel}: ${error}`);
  }

  if (!closed) continue;

  const reportConsistency = validateClauseReportConsistency(text);
  for (const error of reportConsistency.errors) {
    violations.push(`${rel}: ${error}`);
  }

  const reportsText = String(reportConsistency.parsed?.reports?.raw || "");
  const specAlignmentVerdict = ((reportsText.match(/^\s*SPEC_ALIGNMENT_VERDICT\s*:\s*(.+)\s*$/im) || [])[1] || "").trim().toUpperCase();
  if (specAlignmentVerdict === "PASS") {
    const passConsistency = validatePacketClosureMonitoring(text, {
      requireRows: true,
      requireClosedConsistency: true,
      requirePassConsistency: true,
    });
    for (const error of passConsistency.errors) {
      violations.push(`${rel}: ${error}`);
    }
  }
}

if (violations.length > 0) {
  fail("Packet closure monitoring violations found", violations);
}

console.log("packet-closure-monitor-check ok");
