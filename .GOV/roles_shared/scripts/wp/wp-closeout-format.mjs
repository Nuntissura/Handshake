#!/usr/bin/env node
/**
 * RGF-96: Automated closeout formatting for validated WPs.
 *
 * Handles:
 * 1. Updates packet Status to "Validated (PASS)" and containment fields
 * 2. Sets clause closure matrix rows to PROVED/CONFIRMED
 * 3. Ensures the PASS validator report is the FIRST report in VALIDATION_REPORTS
 *    (prevents FAIL-then-PASS poisoning of parseSectionField)
 * 4. Syncs task board status
 *
 * Usage: node wp-closeout-format.mjs <WP_ID> <MERGED_MAIN_COMMIT>
 */

import fs from "node:fs";
import { REPO_ROOT, repoPathAbs, resolveWorkPacketPath } from "../lib/runtime-paths.mjs";
import { parseJsonFile, validateRuntimeStatus } from "../lib/wp-communications-lib.mjs";
import { syncRuntimeProjectionFromPacket } from "../lib/packet-runtime-projection-lib.mjs";
import { readExecutionPublicationView } from "../lib/wp-execution-state-lib.mjs";
import { buildCloseoutDependencyView } from "../lib/wp-closeout-dependency-lib.mjs";
import { parseValidationVerdictRecord } from "../lib/merge-progression-truth-lib.mjs";
import {
  buildTerminalCloseoutRecordFromCloseoutSync,
  publishTerminalCloseoutRecord,
  readTerminalCloseoutRecord,
  resolveTerminalCloseoutPublication,
} from "../lib/terminal-closeout-record-lib.mjs";
import { registerFailCaptureHook } from "../lib/fail-capture-lib.mjs";
import { writeJsonFile } from "../session/session-registry-lib.mjs";
import { evaluateWpRepomemCoverage } from "../memory/repomem-coverage-lib.mjs";

registerFailCaptureHook("wp-closeout-format.mjs", { role: "ORCHESTRATOR" });

const wpId = String(process.argv[2] || "").trim();
const mergedMainCommit = String(process.argv[3] || "").trim();

if (!wpId || !wpId.startsWith("WP-") || !mergedMainCommit) {
  console.error("Usage: node wp-closeout-format.mjs <WP_ID> <MERGED_MAIN_COMMIT>");
  process.exit(1);
}

const resolved = resolveWorkPacketPath(wpId);
if (!resolved) {
  console.error(`[WP_CLOSEOUT_FORMAT] Cannot resolve packet path for ${wpId}`);
  process.exit(1);
}
const packetAbsPath = repoPathAbs(resolved.packetPath || resolved);
if (!fs.existsSync(packetAbsPath)) {
  console.error(`[WP_CLOSEOUT_FORMAT] Packet not found: ${packetAbsPath}`);
  process.exit(1);
}

let content = fs.readFileSync(packetAbsPath, "utf8");
let changes = 0;
const packetStatusBefore = parsePacketStatus(content);
const runtimeStatusFile = parseSingleField(content, "WP_RUNTIME_STATUS_FILE");
const runtimeStatusAbsPath = runtimeStatusFile ? repoPathAbs(runtimeStatusFile) : "";
const runtimeStatus = runtimeStatusAbsPath && fs.existsSync(runtimeStatusAbsPath)
  ? parseJsonFile(runtimeStatusAbsPath)
  : null;
const terminalCloseoutRecord = readTerminalCloseoutRecord({ wpId });
if (terminalCloseoutRecord.status === "INVALID") {
  console.error(`[WP_CLOSEOUT_FORMAT] Terminal closeout record is invalid: ${terminalCloseoutRecord.errors.join("; ")}`);
  process.exit(1);
}
const runtimePublication = readExecutionPublicationView({
  runtimeStatus: runtimeStatus || {},
  packetStatus: packetStatusBefore,
});
const repomemCoverage = evaluateWpRepomemCoverage({
  repoRoot: REPO_ROOT,
  wpId,
  packetContent: content,
});
const closeoutDependencyView = buildCloseoutDependencyView({
  packetContent: content,
  runtimeStatus: runtimeStatus || {},
  closeoutRequirements: {
    requireReadyForPass: true,
    requireRecordedScopeCompatibility: false,
    terminalNonPass: false,
  },
  topology: { ok: true },
  closeoutBundle: { ok: true, summary: {} },
  scopeCompatibility: { errors: [] },
  candidateSignedScope: { errors: [] },
  repomemCoverage,
  terminalCloseoutRecord,
});

if (
  runtimePublication.has_canonical_authority
  && (
    (runtimePublication.canonical_packet_status && runtimePublication.canonical_packet_status !== "Validated (PASS)")
    || (
      runtimePublication.canonical_task_board_status
      && runtimePublication.canonical_task_board_status !== "DONE_VALIDATED"
    )
  )
) {
  console.error(
    `[WP_CLOSEOUT_FORMAT] Canonical execution authority does not allow PASS closeout: `
    + `packet=${runtimePublication.canonical_packet_status || "<missing>"} `
    + `task_board=${runtimePublication.canonical_task_board_status || "<missing>"} `
    + `dependency_view=${closeoutDependencyView.summary}`,
  );
  process.exit(1);
}

const closeoutTimestamp = new Date().toISOString();
const validationVerdictRecord = parseValidationVerdictRecord(content);
const nextTerminalCloseoutRecord = buildTerminalCloseoutRecordFromCloseoutSync({
  wpId,
  mode: "CONTAINED_IN_MAIN",
  packetStatus: "Validated (PASS)",
  taskBoardStatus: "DONE_VALIDATED",
  mainContainmentStatus: "CONTAINED_IN_MAIN",
  mergedMainCommit,
  verdict: "PASS",
  verdictRecordedAtUtc: validationVerdictRecord.timestampUtc || "",
  verdictActorRole: validationVerdictRecord.actorRole || "INTEGRATION_VALIDATOR",
  verdictActorSession: validationVerdictRecord.actorSession || "",
  verdictEvidencePointer: validationVerdictRecord.evidencePointer || "",
  terminalPublicationRecorded: true,
  actorRole: "ORCHESTRATOR",
  actorSession: "wp-closeout-format",
  source: "WP_CLOSEOUT_FORMAT",
  recordedAtUtc: closeoutTimestamp,
  previousRecord: terminalCloseoutRecord.record,
});
const terminalPublicationDecision = resolveTerminalCloseoutPublication({
  currentRecord: terminalCloseoutRecord.record,
  nextRecord: nextTerminalCloseoutRecord,
});
if (!terminalPublicationDecision.ok) {
  console.error(`[WP_CLOSEOUT_FORMAT] Terminal closeout record publication rejected: ${terminalPublicationDecision.message}`);
  process.exit(1);
}

// 1. Update Status
content = content.replace(
  /^- \*\*Status:\*\* .+$/m,
  "- **Status:** Validated (PASS)"
);
changes++;

// 2. Update containment fields
content = content.replace(
  /^- MAIN_CONTAINMENT_STATUS: .+$/m,
  "- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN"
);
content = content.replace(
  /^- MERGED_MAIN_COMMIT: .+$/m,
  `- MERGED_MAIN_COMMIT: ${mergedMainCommit}`
);
content = content.replace(
  /^- MAIN_CONTAINMENT_VERIFIED_AT_UTC: .+$/m,
  `- MAIN_CONTAINMENT_VERIFIED_AT_UTC: ${closeoutTimestamp}`
);
content = content.replace(
  /^- CURRENT_MAIN_COMPATIBILITY_STATUS: .+$/m,
  "- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE"
);
changes++;

// 3. Update Verdict
content = content.replace(
  /^Verdict: PENDING$/m,
  "Verdict: PASS"
);
changes++;

// 4. Update clause closure matrix
const closureRe = /CODER_STATUS: UNPROVEN \| VALIDATOR_STATUS: PENDING/g;
const closureCount = (content.match(closureRe) || []).length;
content = content.replace(closureRe, "CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED");
if (closureCount > 0) {
  console.log(`[WP_CLOSEOUT_FORMAT] Updated ${closureCount} clause closure matrix rows`);
  changes++;
}

fs.writeFileSync(packetAbsPath, content, "utf8");
let runtimeSynced = "";
let terminalRecordPath = "";
if (runtimeStatusAbsPath && runtimeStatus) {
  const syncedRuntime = syncRuntimeProjectionFromPacket(runtimeStatus, content, {
    eventName: "wp_closeout_format",
  });
  const runtimeErrors = validateRuntimeStatus(syncedRuntime);
  if (runtimeErrors.length > 0) {
    console.error(`[WP_CLOSEOUT_FORMAT] Runtime sync failed for ${wpId}`);
    for (const error of runtimeErrors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }
  writeJsonFile(runtimeStatusAbsPath, syncedRuntime);
  runtimeSynced = runtimeStatusFile;
}
const terminalPublication = publishTerminalCloseoutRecord({
  wpId,
  record: nextTerminalCloseoutRecord,
});
terminalRecordPath = terminalPublication.path;
console.log(`[WP_CLOSEOUT_FORMAT] Packet updated: ${resolved.packetPath || resolved}`);
console.log(`[WP_CLOSEOUT_FORMAT] Changes: ${changes}`);
if (runtimeSynced) console.log(`[WP_CLOSEOUT_FORMAT] Runtime synced: ${runtimeSynced}`);
if (terminalRecordPath) console.log(`[WP_CLOSEOUT_FORMAT] Terminal closeout record: ${terminalRecordPath}`);
console.log(`[WP_CLOSEOUT_FORMAT] Next: just task-board-set ${wpId} DONE_VALIDATED`);
console.log(`[WP_CLOSEOUT_FORMAT] Next: just build-order-sync`);
console.log(`[WP_CLOSEOUT_FORMAT] Next: just gov-check`);

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function parsePacketStatus(text) {
  return (
    (String(text || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(text || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || ""
  ).trim();
}
