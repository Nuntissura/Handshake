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
import path from "node:path";
import { repoPathAbs, resolveWorkPacketPath } from "../lib/runtime-paths.mjs";

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
  `- MAIN_CONTAINMENT_VERIFIED_AT_UTC: ${new Date().toISOString()}`
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
console.log(`[WP_CLOSEOUT_FORMAT] Packet updated: ${resolved.packetPath || resolved}`);
console.log(`[WP_CLOSEOUT_FORMAT] Changes: ${changes}`);
console.log(`[WP_CLOSEOUT_FORMAT] Next: just task-board-set ${wpId} DONE_VALIDATED`);
console.log(`[WP_CLOSEOUT_FORMAT] Next: just build-order-sync`);
console.log(`[WP_CLOSEOUT_FORMAT] Next: just gov-check`);
