import assert from "node:assert/strict";
import test from "node:test";
import {
  parseValidationVerdictRecord,
  readVerdictSettlementTruth,
  validateMergeProgressionTruth,
} from "../scripts/lib/merge-progression-truth-lib.mjs";

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

test("Validated (ABANDONED) requires NOT_REQUIRED containment and no merged-main fields", () => {
  const packetText = buildPacket({
    status: "Validated (ABANDONED)",
    verdict: "ABANDONED",
    mainContainmentStatus: "NOT_REQUIRED",
    mergedMainCommit: "NONE",
    mainContainmentVerifiedAtUtc: "N/A",
  });
  const result = validateMergeProgressionTruth(packetText, {
    runtimeStatusData: {
      current_packet_status: "Validated (ABANDONED)",
      main_containment_status: "NOT_REQUIRED",
      merged_main_commit: null,
      main_containment_verified_at_utc: null,
    },
  });
  assert.deepEqual(result.errors, []);
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

test("merge progression truth accepts canonical execution_state authority when flat mirrors are stale", () => {
  const packetText = buildPacket({
    status: "Validated (PASS)",
    mainContainmentStatus: "CONTAINED_IN_MAIN",
    mergedMainCommit: "abc1234",
    mainContainmentVerifiedAtUtc: "2026-03-25T12:00:00Z",
  });
  const result = validateMergeProgressionTruth(packetText, {
    runtimeStatusData: {
      current_packet_status: "Done",
      main_containment_status: "MERGE_PENDING",
      merged_main_commit: null,
      main_containment_verified_at_utc: null,
      execution_state: {
        schema_version: "wp_execution_state@1",
        authority: {
          packet_status: "Validated (PASS)",
          main_containment_status: "CONTAINED_IN_MAIN",
          merged_main_commit: "abc1234",
          main_containment_verified_at_utc: "2026-03-25T12:00:00Z",
          review_anchor: {},
          route_anchor: {},
        },
        checkpoint_lineage: {
          schema_version: "wp_execution_checkpoint_lineage@1",
          latest_checkpoint_id: null,
          latest_checkpoint_at_utc: null,
          latest_checkpoint_kind: null,
          latest_restore_point_id: null,
          latest_checkpoint_fingerprint: null,
          checkpoint_count: 0,
          checkpoints: [],
        },
      },
    },
    mainContainmentVerifier: () => ({ ok: true, reason: "contained in main" }),
  });
  assert.deepEqual(result.errors, []);
});

test("parseValidationVerdictRecord prefers the latest validation report entry and preserves verdict provenance", () => {
  const packetText = [
    "# Task Packet: WP-TEST-MERGE-TRUTH-v1",
    "",
    "## METADATA",
    "- **Status:** In Progress",
    "- MAIN_CONTAINMENT_STATUS: NOT_STARTED",
    "- MERGED_MAIN_COMMIT: NONE",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A",
    "- PACKET_FORMAT_VERSION: 2026-03-25",
    "",
    "## VALIDATION_REPORTS",
    "### 2026-03-25T10:00:00Z | INTEGRATION_VALIDATOR | session=integration-validator-old",
    "- Verdict: PASS",
    "",
    "### 2026-03-25T12:30:00Z | INTEGRATION_VALIDATOR | session=integration-validator-new",
    "- Verdict: FAIL",
    "",
  ].join("\n");

  const verdict = parseValidationVerdictRecord(packetText);

  assert.deepEqual(verdict, {
    verdict: "FAIL",
    timestampUtc: "2026-03-25T12:30:00Z",
    actorRole: "INTEGRATION_VALIDATOR",
    actorSession: "integration-validator-new",
    evidencePointer: "VALIDATION_REPORTS[2]",
    reportIndex: 2,
  });
});

test("readVerdictSettlementTruth preserves verdict-of-record before terminal closeout publication exists", () => {
  const packetText = [
    "# Task Packet: WP-TEST-MERGE-TRUTH-v1",
    "",
    "## METADATA",
    "- **Status:** In Progress",
    "- MAIN_CONTAINMENT_STATUS: NOT_STARTED",
    "- MERGED_MAIN_COMMIT: NONE",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A",
    "- PACKET_FORMAT_VERSION: 2026-03-25",
    "",
    "## VALIDATION_REPORTS",
    "### 2026-03-25T12:30:00Z | INTEGRATION_VALIDATOR | session=integration-validator-new",
    "- Verdict: FAIL",
    "",
  ].join("\n");

  const verdictSettlement = readVerdictSettlementTruth({
    packetText,
    runtimeStatus: {
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      main_containment_status: "NOT_STARTED",
    },
  });

  assert.equal(verdictSettlement.verdictOfRecord, "FAIL");
  assert.equal(verdictSettlement.closeoutMode, "");
  assert.equal(verdictSettlement.settlementState, "SETTLEMENT_DEBT");
  assert.deepEqual(verdictSettlement.settlementBlockers, ["TERMINAL_PUBLICATION_PENDING"]);
});

test("parseValidationVerdictRecord prefers the report matching settled packet publication truth", () => {
  const packetText = [
    "# Task Packet: WP-TEST-MERGE-TRUTH-v1",
    "",
    "## METADATA",
    "- **Status:** Validated (PASS)",
    "- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN",
    "- MERGED_MAIN_COMMIT: abc1234",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-03-25T12:00:00Z",
    "- PACKET_FORMAT_VERSION: 2026-03-25",
    "",
    "## VALIDATION_REPORTS",
    "### 2026-03-25T10:00:00Z | INTEGRATION_VALIDATOR | session=integration-validator-pass",
    "- Verdict: PASS",
    "",
    "### 2026-03-25T12:30:00Z | INTEGRATION_VALIDATOR | session=integration-validator-followup",
    "- Verdict: FAIL",
    "",
  ].join("\n");

  const verdict = parseValidationVerdictRecord(packetText);

  assert.equal(verdict.verdict, "PASS");
  assert.equal(verdict.actorSession, "integration-validator-pass");
  assert.equal(verdict.evidencePointer, "VALIDATION_REPORTS[1]");
});

test("parseValidationVerdictRecord recognizes legacy report headings and body metadata on settled PASS packets", () => {
  const packetText = [
    "# Task Packet: WP-TEST-MERGE-TRUTH-v1",
    "",
    "## METADATA",
    "- **Status:** Validated (PASS)",
    "- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN",
    "- MERGED_MAIN_COMMIT: abc1234",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-03-25T12:00:00Z",
    "- PACKET_FORMAT_VERSION: 2026-03-25",
    "",
    "## VALIDATION_REPORTS",
    "- (Validator appends official audits and verdicts here. Append-only.)",
    "",
    "### Integration Validator Report (Post-Fix)",
    "DATE: 2026-04-06T08:00:00Z",
    "VALIDATOR_ROLE: INTEGRATION_VALIDATOR",
    "VALIDATOR_MODEL: claude-opus-4-6",
    "Verdict: PASS",
    "",
    "### 2026-04-06T07:01:00Z | WP_VALIDATOR FAIL REPORT",
    "VALIDATOR_ROLE: WP_VALIDATOR",
    "ACTOR_SESSION: wp_validator:wp-1-merge-truth-v1",
    "Verdict: FAIL",
    "",
  ].join("\n");

  const verdict = parseValidationVerdictRecord(packetText);

  assert.deepEqual(verdict, {
    verdict: "PASS",
    timestampUtc: "2026-04-06T08:00:00Z",
    actorRole: "INTEGRATION_VALIDATOR",
    actorSession: "",
    evidencePointer: "VALIDATION_REPORTS[1]",
    reportIndex: 1,
  });
});

test("parseValidationVerdictRecord extracts role and session from legacy WP validator bullet metadata", () => {
  const packetText = [
    "# Task Packet: WP-TEST-MERGE-TRUTH-v1",
    "",
    "## METADATA",
    "- **Status:** In Progress",
    "- MAIN_CONTAINMENT_STATUS: NOT_STARTED",
    "- MERGED_MAIN_COMMIT: NONE",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A",
    "- PACKET_FORMAT_VERSION: 2026-03-25",
    "",
    "## VALIDATION_REPORTS",
    "### WP Validator Report: WP-TEST-MERGE-TRUTH-v1 (2026-04-06)",
    "- Validator: WP_VALIDATOR (claude-opus-4-6, session wp_validator:wp-test-merge-truth-v1)",
    "- Commit: abc1234 (validate/WP-TEST-MERGE-TRUTH-v1)",
    "- Verdict: FAIL",
    "",
  ].join("\n");

  const verdict = parseValidationVerdictRecord(packetText);

  assert.deepEqual(verdict, {
    verdict: "FAIL",
    timestampUtc: "",
    actorRole: "WP_VALIDATOR",
    actorSession: "wp_validator:wp-test-merge-truth-v1",
    evidencePointer: "VALIDATION_REPORTS[1]",
    reportIndex: 1,
  });
});

test("parseValidationVerdictRecord treats level-4 verdict headings as report body, not new report boundaries", () => {
  const packetText = [
    "# Task Packet: WP-TEST-MERGE-TRUTH-v1",
    "",
    "## METADATA",
    "- **Status:** Validated (PASS)",
    "- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN",
    "- MERGED_MAIN_COMMIT: abc1234",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-03-25T12:00:00Z",
    "- PACKET_FORMAT_VERSION: 2026-03-25",
    "",
    "## VALIDATION_REPORTS",
    "### INTEGRATION_VALIDATOR_REPORT [2026-04-15T15:43:00Z]",
    "- ROLE: INTEGRATION_VALIDATOR",
    "- SESSION: integration_validator:wp-test-merge-truth-v1",
    "#### Verdict: PASS",
    "#### CLAUSES_REVIEWED:",
    "- clause one",
    "",
    "### WP_VALIDATOR_REPORT [2026-04-15T14:00:00Z]",
    "- ROLE: WP_VALIDATOR",
    "- SESSION: wp_validator:wp-test-merge-truth-v1",
    "#### Verdict: FAIL",
    "",
  ].join("\n");

  const verdict = parseValidationVerdictRecord(packetText);

  assert.deepEqual(verdict, {
    verdict: "PASS",
    timestampUtc: "2026-04-15T15:43:00Z",
    actorRole: "INTEGRATION_VALIDATOR",
    actorSession: "integration_validator:wp-test-merge-truth-v1",
    evidencePointer: "VALIDATION_REPORTS[1]",
    reportIndex: 1,
  });
});

test("parseValidationVerdictRecord normalizes closeout-style report headings to the underlying validator role", () => {
  const packetText = [
    "# Task Packet: WP-TEST-MERGE-TRUTH-v1",
    "",
    "## METADATA",
    "- **Status:** Validated (PASS)",
    "- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN",
    "- MERGED_MAIN_COMMIT: abc1234",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-03-25T12:00:00Z",
    "- PACKET_FORMAT_VERSION: 2026-03-25",
    "",
    "## VALIDATION_REPORTS",
    "### INTEGRATION_VALIDATOR_REPORT_CLOSEOUT [2026-04-15T16:55:00Z]",
    "- ROLE: INTEGRATION_VALIDATOR",
    "- SESSION: integration_validator:wp-test-merge-truth-v1",
    "#### Verdict: PASS",
    "",
  ].join("\n");

  const verdict = parseValidationVerdictRecord(packetText);

  assert.deepEqual(verdict, {
    verdict: "PASS",
    timestampUtc: "2026-04-15T16:55:00Z",
    actorRole: "INTEGRATION_VALIDATOR",
    actorSession: "integration_validator:wp-test-merge-truth-v1",
    evidencePointer: "VALIDATION_REPORTS[1]",
    reportIndex: 1,
  });
});
