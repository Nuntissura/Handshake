import assert from "node:assert/strict";
import test from "node:test";
import { validateMergeProgressionTruth } from "../scripts/lib/merge-progression-truth-lib.mjs";

function buildPacket({
  packetFormatVersion = "2026-03-25",
  status = "Done",
  verdict = "PASS",
  mainContainmentStatus = "MERGE_PENDING",
  mergedMainCommit = "NONE",
  mainContainmentVerifiedAtUtc = "N/A",
  localBranch = "feat/WP-TEST-MERGE-TRUTH-v1",
  localWorktreeDir = "../wtc-merge-truth-v1",
} = {}) {
  return [
    "# Task Packet: WP-TEST-MERGE-TRUTH-v1",
    "",
    "## METADATA",
    "- WP_ID: WP-TEST-MERGE-TRUTH-v1",
    `- **Status:** ${status}`,
    `- MAIN_CONTAINMENT_STATUS: ${mainContainmentStatus}`,
    `- MERGED_MAIN_COMMIT: ${mergedMainCommit}`,
    `- MAIN_CONTAINMENT_VERIFIED_AT_UTC: ${mainContainmentVerifiedAtUtc}`,
    `- LOCAL_BRANCH: ${localBranch}`,
    `- LOCAL_WORKTREE_DIR: ${localWorktreeDir}`,
    "- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main",
    "- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-MERGE-TRUTH-v1/RUNTIME_STATUS.json",
    `- PACKET_FORMAT_VERSION: ${packetFormatVersion}`,
    "",
    "## VALIDATION_REPORTS",
    `Verdict: ${verdict}`,
    "",
  ].join("\n");
}

test("Done on new-format packets requires merge-pending truth", () => {
  const packetText = buildPacket();
  const result = validateMergeProgressionTruth(packetText, {
    runtimeStatusData: {
      current_packet_status: "Done",
      main_containment_status: "MERGE_PENDING",
      merged_main_commit: null,
      main_containment_verified_at_utc: null,
    },
    mergePendingWorktreeVerifier: () => ({ ok: true, reason: "worktree available" }),
  });
  assert.deepEqual(result.errors, []);
});

test("Done / MERGE_PENDING fails when the declared coder worktree path is unavailable", () => {
  const packetText = buildPacket();
  const result = validateMergeProgressionTruth(packetText, {
    runtimeStatusData: {
      current_packet_status: "Done",
      main_containment_status: "MERGE_PENDING",
      merged_main_commit: null,
      main_containment_verified_at_utc: null,
    },
    mergePendingWorktreeVerifier: () => ({ ok: false, reason: "declared coder worktree missing" }),
  });
  assert.match(result.errors.join("\n"), /requires an active coder worktree path/i);
});

test("Validated (PASS) on new-format packets requires containment proof and recorded main SHA", () => {
  const packetText = buildPacket({
    status: "Validated (PASS)",
    mainContainmentStatus: "CONTAINED_IN_MAIN",
    mergedMainCommit: "abc1234",
    mainContainmentVerifiedAtUtc: "2026-03-25T12:00:00Z",
  });
  const result = validateMergeProgressionTruth(packetText, {
    runtimeStatusData: {
      current_packet_status: "Validated (PASS)",
      main_containment_status: "CONTAINED_IN_MAIN",
      merged_main_commit: "abc1234",
      main_containment_verified_at_utc: "2026-03-25T12:00:00Z",
    },
    mainContainmentVerifier: () => ({ ok: true, reason: "contained in main" }),
  });
  assert.deepEqual(result.errors, []);
});

test("Validated (PASS) fails when the recorded merged commit is not proven contained in main", () => {
  const packetText = buildPacket({
    status: "Validated (PASS)",
    mainContainmentStatus: "CONTAINED_IN_MAIN",
    mergedMainCommit: "abc1234",
    mainContainmentVerifiedAtUtc: "2026-03-25T12:00:00Z",
  });
  const result = validateMergeProgressionTruth(packetText, {
    runtimeStatusData: {
      current_packet_status: "Validated (PASS)",
      main_containment_status: "CONTAINED_IN_MAIN",
      merged_main_commit: "abc1234",
      main_containment_verified_at_utc: "2026-03-25T12:00:00Z",
    },
    mainContainmentVerifier: () => ({ ok: false, reason: "commit abc1234 is not in main" }),
  });
  assert.match(result.errors.join("\n"), /requires main containment proof/i);
});

test("Legacy packet versions bypass merge progression truth enforcement", () => {
  const packetText = buildPacket({
    packetFormatVersion: "2026-03-24",
    status: "Done",
    mainContainmentStatus: "NOT_STARTED",
  });
  const result = validateMergeProgressionTruth(packetText, {
    runtimeStatusData: {
      current_packet_status: "Done",
      main_containment_status: "NOT_STARTED",
      merged_main_commit: null,
      main_containment_verified_at_utc: null,
    },
  });
  assert.deepEqual(result.errors, []);
});

test("merge progression truth fails when runtime current_packet_status lags packet status", () => {
  const packetText = buildPacket({
    status: "Validated (PASS)",
    mainContainmentStatus: "CONTAINED_IN_MAIN",
    mergedMainCommit: "abc1234",
    mainContainmentVerifiedAtUtc: "2026-03-25T12:00:00Z",
  });
  const result = validateMergeProgressionTruth(packetText, {
    runtimeStatusData: {
      current_packet_status: "Done",
      main_containment_status: "CONTAINED_IN_MAIN",
      merged_main_commit: "abc1234",
      main_containment_verified_at_utc: "2026-03-25T12:00:00Z",
    },
    mainContainmentVerifier: () => ({ ok: true, reason: "contained in main" }),
  });
  assert.match(result.errors.join("\n"), /current_packet_status .* must match packet Status/i);
});
