import fs from "node:fs";
import { validateClauseReportConsistency, validatePacketClosureMonitoring } from "../scripts/lib/packet-closure-monitor-lib.mjs";
import { listOfficialWorkPacketPaths, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("packet-closure-monitor-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("packet-closure-monitor-check.mjs", message, { role: "SHARED", details });
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
  const specAlignmentVerdict = (() => {
    const re = /^(?:\s*-\s*|\s*#{1,6}\s+|\s*)SPEC_ALIGNMENT_VERDICT\s*:\s*(.+)\s*$/gim;
    const matches = [...reportsText.matchAll(re)];
    return matches.length > 0 ? (matches[matches.length - 1][1] || "").trim().toUpperCase() : "";
  })();
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
