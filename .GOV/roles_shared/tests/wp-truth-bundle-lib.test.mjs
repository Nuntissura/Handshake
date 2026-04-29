import assert from "node:assert/strict";
import test from "node:test";

import {
  buildWpTruthBundle,
  formatWpTruthBundleCompact,
  WP_TRUTH_BUNDLE_MAX_COMPACT_LINES,
} from "../scripts/lib/wp-truth-bundle-lib.mjs";

function packetText(wpId) {
  return [
    `# Task Packet: ${wpId}`,
    "",
    "**Status:** Validated (PASS)",
    "",
    "## METADATA",
    `- WP_ID: ${wpId}`,
    "- PACKET_FORMAT_VERSION: 2026-04-06",
    "- LOCAL_BRANCH: feat/test",
    "- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-TRUTH/RUNTIME_STATUS.json",
    "- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-TRUTH/RECEIPTS.jsonl",
    "- WP_NOTIFICATIONS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-TRUTH/NOTIFICATIONS.jsonl",
    "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
    "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE",
    "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A",
    "- PACKET_WIDENING_DECISION: NONE",
    "- PACKET_WIDENING_EVIDENCE: N/A",
    "- MAIN_CONTAINMENT_STATUS: MERGE_PENDING",
    "- MERGED_MAIN_COMMIT: NONE",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A",
    "",
    "## VALIDATION_REPORTS",
    "Verdict: PASS",
  ].join("\n");
}

test("compact WP truth bundle reports terminal PASS with stale session residue as governance debt", () => {
  const wpId = "WP-TEST-TRUTH";
  const result = buildWpTruthBundle({
    wpId,
    packetText: packetText(wpId),
    runtimeStatus: {
      runtime_status: "completed",
      current_phase: "STATUS_SYNC",
      current_packet_status: "Validated (PASS)",
      current_task_board_status: "VALIDATED",
      next_expected_actor: "NONE",
      waiting_on: "CLOSED",
      main_containment_status: "MERGE_PENDING",
    },
    sessions: [
      { wp_id: wpId, role: "INTEGRATION_VALIDATOR", session_key: `INTEGRATION_VALIDATOR:${wpId}`, runtime_state: "READY" },
    ],
    controlRequests: [],
    controlResults: [],
    receipts: [],
    notifications: [],
    writeDetail: false,
  });

  assert.equal(result.ok, true);
  assert.equal(result.bundle.final_verdict, "PASS");
  assert.equal(result.bundle.session_summary.terminal_residue, 1);
  assert.match(result.bundle.exact_next_command, /phase-check CLOSEOUT/);
  const compact = formatWpTruthBundleCompact(result.bundle);
  assert.ok(compact.split(/\r?\n/).filter(Boolean).length <= WP_TRUTH_BUNDLE_MAX_COMPACT_LINES);
});

test("compact WP truth bundle fails clearly when no WP id is supplied", () => {
  const result = buildWpTruthBundle({ wpId: "", writeDetail: false });
  assert.equal(result.ok, false);
  assert.match(result.error, /WP_ID is required/);
});

