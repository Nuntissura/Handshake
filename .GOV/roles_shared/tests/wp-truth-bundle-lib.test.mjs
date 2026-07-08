import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  buildWpTruthBundle,
  formatWpTruthBundleCompact,
  WP_TRUTH_BUNDLE_MAX_COMPACT_LINES,
} from "../scripts/lib/wp-truth-bundle-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

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

test("blocked folded Kernel Builder packets keep explicit active MT as next command target", () => {
  const wpId = "WP-TEST-FOLDED-BLOCKED-CURSOR-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.json"),
    JSON.stringify({
      schema_id: "hsk.work_packet_contract@1",
      schema_version: "work_packet_contract_v1",
      contract_authority: "PRIMARY_MACHINE_READABLE",
      wp_id: wpId,
      workflow: {
        lane: "KERNEL_BUILDER_FOLDED_NO_ACP",
        runtime_status_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-FOLDED-BLOCKED-CURSOR-v1/RUNTIME_STATUS.json",
        receipts_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-FOLDED-BLOCKED-CURSOR-v1/RECEIPTS.jsonl",
        notifications_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-FOLDED-BLOCKED-CURSOR-v1/NOTIFICATIONS.jsonl",
      },
      lifecycle: {
        status: "Blocked",
        packet_format_version: "2026-06-28",
        main_containment_status: "NOT_STARTED",
        merged_main_commit: "NONE",
        main_containment_verified_at_utc: "N/A",
        current_main_compatibility_status: "NOT_RUN",
        current_main_compatibility_baseline_sha: "NONE",
        current_main_compatibility_verified_at_utc: "N/A",
        packet_widening_decision: "NONE",
        packet_widening_evidence: "N/A",
      },
      source_control: {
        work_branch: "feat/test",
      },
      microtasks: {
        declared_ids: ["MT-013", "MT-014"],
        active_id: "MT-013",
        next_id: "MT-013",
      },
      authority_files: {
        packet_contract: `.GOV/task_packets/${wpId}/packet.json`,
      },
    }),
    "utf8",
  );

  try {
    const result = buildWpTruthBundle({
      wpId,
      runtimeStatus: {
        runtime_status: "submitted",
        current_packet_status: "Blocked",
        current_task_board_status: "BLOCKED",
        main_containment_status: "NOT_STARTED",
        current_main_compatibility_status: "NOT_RUN",
      },
      sessions: [],
      controlRequests: [],
      controlResults: [],
      receipts: [],
      notifications: [],
      writeDetail: false,
    });

    assert.equal(result.ok, true);
    assert.equal(result.bundle.packet_status, "Blocked");
    assert.equal(result.bundle.active_mt, "MT-013");
    assert.equal(result.bundle.next_mt, "MT-013");
    assert.equal(result.bundle.exact_next_command, `just mt-board ${wpId}`);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("compact WP truth bundle fails clearly when no WP id is supplied", () => {
  const result = buildWpTruthBundle({ wpId: "", writeDetail: false });
  assert.equal(result.ok, false);
  assert.match(result.error, /WP_ID is required/);
});

test("folded Kernel Builder packets route exact next command to validator-next", () => {
  const wpId = "WP-TEST-FOLDED";
  const result = buildWpTruthBundle({
    wpId,
    packetText: [
      `# Task Packet: ${wpId}`,
      "",
      "**Status:** In Progress",
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- WORKFLOW_LANE: KERNEL_BUILDER_FOLDED_NO_ACP",
      "- VALIDATION_TOPOLOGY: INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1",
      "- LOCAL_BRANCH: feat/test",
    ].join("\n"),
    runtimeStatus: {
      runtime_status: "UNKNOWN",
      current_task_board_status: "IN_PROGRESS",
      next_expected_actor: "INTEGRATION_VALIDATOR",
      waiting_on: "WP_VALIDATION",
    },
    sessions: [],
    controlRequests: [],
    controlResults: [],
    receipts: [],
    notifications: [],
    writeDetail: false,
  });

  assert.equal(result.ok, true);
  assert.equal(result.bundle.workflow_lane, "KERNEL_BUILDER_FOLDED_NO_ACP");
  assert.match(result.bundle.exact_next_command, new RegExp(`^just validator-next INTEGRATION_VALIDATOR ${wpId}$`));
  assert.doesNotMatch(result.bundle.exact_next_command, /orchestrator-steer-next/);
});
