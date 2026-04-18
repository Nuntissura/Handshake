import assert from "node:assert/strict";
import test from "node:test";
import {
  evaluatePacketRuntimeProjectionDrift,
  parseRuntimeProjectionFromPacket,
  syncRuntimeProjectionFromPacket,
} from "../scripts/lib/packet-runtime-projection-lib.mjs";

function packetFixture({
  status = "Validated (PASS)",
  containment = "CONTAINED_IN_MAIN",
  merged = "0123456789abcdef0123456789abcdef01234567",
  verifiedAt = "2026-03-26T12:00:00Z",
  wpValidatorOfRecord = "wp_validator:wp-test-validator-v1",
  integrationValidatorOfRecord = "integration_validator:wp-test-validator-v1",
} = {}) {
  return [
    "## METADATA",
    `- **Status:** ${status}`,
    `- WP_VALIDATOR_OF_RECORD: ${wpValidatorOfRecord}`,
    `- INTEGRATION_VALIDATOR_OF_RECORD: ${integrationValidatorOfRecord}`,
    "- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE",
    "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 89abcdef0123456789abcdef0123456789abcdef",
    "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-03-26T11:00:00Z",
    "- PACKET_WIDENING_DECISION: NOT_REQUIRED",
    "- PACKET_WIDENING_EVIDENCE: N/A",
    `- MAIN_CONTAINMENT_STATUS: ${containment}`,
    `- MERGED_MAIN_COMMIT: ${merged}`,
    `- MAIN_CONTAINMENT_VERIFIED_AT_UTC: ${verifiedAt}`,
  ].join("\n");
}

test("parseRuntimeProjectionFromPacket normalizes terminal packet truth", () => {
  const projection = parseRuntimeProjectionFromPacket(packetFixture());
  assert.deepEqual(projection, {
    current_packet_status: "Validated (PASS)",
    current_task_board_status: "DONE_VALIDATED",
    wp_validator_of_record: "wp_validator:wp-test-validator-v1",
    integration_validator_of_record: "integration_validator:wp-test-validator-v1",
    current_main_compatibility_status: "COMPATIBLE",
    current_main_compatibility_baseline_sha: "89abcdef0123456789abcdef0123456789abcdef",
    current_main_compatibility_verified_at_utc: "2026-03-26T11:00:00Z",
    packet_widening_decision: "NOT_REQUIRED",
    packet_widening_evidence: null,
    main_containment_status: "CONTAINED_IN_MAIN",
    merged_main_commit: "0123456789abcdef0123456789abcdef01234567",
    main_containment_verified_at_utc: "2026-03-26T12:00:00Z",
  });
});

test("syncRuntimeProjectionFromPacket updates runtime projection fields and event metadata", () => {
  const runtime = syncRuntimeProjectionFromPacket(
    {
      current_packet_status: "Done",
      wp_validator_of_record: null,
      integration_validator_of_record: null,
      current_main_compatibility_status: "NOT_RUN",
      current_main_compatibility_baseline_sha: null,
      current_main_compatibility_verified_at_utc: null,
      packet_widening_decision: null,
      packet_widening_evidence: null,
      main_containment_status: "MERGE_PENDING",
      merged_main_commit: null,
      main_containment_verified_at_utc: null,
      last_event: "old",
      last_event_at: "2026-03-25T10:00:00Z",
    },
    packetFixture(),
    {
      eventName: "task_board_sync",
      eventAt: "2026-03-26T13:00:00Z",
    },
  );

  assert.equal(runtime.current_packet_status, "Validated (PASS)");
  assert.equal(runtime.current_task_board_status, "DONE_VALIDATED");
  assert.equal(runtime.current_milestone, "CONTAINMENT");
  assert.equal(runtime.wp_validator_of_record, "wp_validator:wp-test-validator-v1");
  assert.equal(runtime.integration_validator_of_record, "integration_validator:wp-test-validator-v1");
  assert.equal(runtime.current_main_compatibility_status, "COMPATIBLE");
  assert.equal(runtime.current_main_compatibility_baseline_sha, "89abcdef0123456789abcdef0123456789abcdef");
  assert.equal(runtime.current_main_compatibility_verified_at_utc, "2026-03-26T11:00:00Z");
  assert.equal(runtime.packet_widening_decision, "NOT_REQUIRED");
  assert.equal(runtime.packet_widening_evidence, null);
  assert.equal(runtime.main_containment_status, "CONTAINED_IN_MAIN");
  assert.equal(runtime.merged_main_commit, "0123456789abcdef0123456789abcdef01234567");
  assert.equal(runtime.main_containment_verified_at_utc, "2026-03-26T12:00:00Z");
  assert.equal(runtime.last_event, "task_board_sync");
  assert.equal(runtime.last_event_at, "2026-03-26T13:00:00Z");
});

test("syncRuntimeProjectionFromPacket drives validated packets into STATUS_SYNC completed runtime state", () => {
  const runtime = syncRuntimeProjectionFromPacket(
    {
      current_phase: "BOOTSTRAP",
      runtime_status: "submitted",
      next_expected_actor: "ORCHESTRATOR",
      waiting_on: "CODER",
      validator_trigger: "HANDOFF_READY",
      ready_for_validation: true,
      attention_required: true,
      current_files_touched: ["src/demo.rs"],
      active_role_sessions: [
        {
          role: "CODER",
          session_id: "coder:wp-test-validator-v1",
          authority_kind: "PRIMARY_CODER",
          validator_role_kind: null,
          worktree_dir: "../wtc-test-validator-v1",
          state: "working",
          last_heartbeat_at: "2026-03-26T12:55:00Z",
        },
      ],
      open_review_items: [{ correlation_id: "review-1" }],
    },
    packetFixture(),
  );

  assert.equal(runtime.current_phase, "STATUS_SYNC");
  assert.equal(runtime.current_milestone, "CONTAINMENT");
  assert.equal(runtime.runtime_status, "completed");
  assert.equal(runtime.next_expected_actor, "NONE");
  assert.equal(runtime.waiting_on, "CLOSED");
  assert.equal(runtime.ready_for_validation, false);
  assert.equal(runtime.attention_required, false);
  assert.equal(runtime.wp_validator_of_record, "wp_validator:wp-test-validator-v1");
  assert.equal(runtime.integration_validator_of_record, "integration_validator:wp-test-validator-v1");
  assert.deepEqual(runtime.current_files_touched, []);
  assert.deepEqual(runtime.active_role_sessions, []);
  assert.deepEqual(runtime.open_review_items, []);
});

test("syncRuntimeProjectionFromPacket treats Validated (ABANDONED) as a closed terminal runtime state", () => {
  const runtime = syncRuntimeProjectionFromPacket(
    {
      current_phase: "VALIDATION",
      runtime_status: "working",
      next_expected_actor: "INTEGRATION_VALIDATOR",
      waiting_on: "MERGE",
      validator_trigger: "HANDOFF_READY",
      ready_for_validation: true,
      attention_required: true,
      open_review_items: [{ correlation_id: "review-2" }],
    },
    packetFixture({
      status: "Validated (ABANDONED)",
      containment: "NOT_REQUIRED",
      merged: "NONE",
      verifiedAt: "N/A",
    }),
  );

  assert.equal(runtime.current_phase, "STATUS_SYNC");
  assert.equal(runtime.current_milestone, "CONTAINMENT");
  assert.equal(runtime.runtime_status, "completed");
  assert.equal(runtime.next_expected_actor, "NONE");
  assert.equal(runtime.waiting_on, "CLOSED");
  assert.equal(runtime.ready_for_validation, false);
  assert.equal(runtime.attention_required, false);
  assert.deepEqual(runtime.open_review_items, []);
});

test("syncRuntimeProjectionFromPacket keeps the integration-validator session bound during merge-pending closeout", () => {
  const runtime = syncRuntimeProjectionFromPacket(
    {
      current_phase: "VERDICT",
      runtime_status: "working",
      next_expected_actor: "ORCHESTRATOR",
      next_expected_session: null,
      waiting_on: "VERDICT_PROGRESSION",
      validator_trigger: "HANDOFF_READY",
      ready_for_validation: true,
      attention_required: true,
    },
    packetFixture({
      status: "Done",
      containment: "MERGE_PENDING",
      merged: "NONE",
      verifiedAt: "N/A",
    }),
  );

  assert.equal(runtime.current_phase, "STATUS_SYNC");
  assert.equal(runtime.runtime_status, "completed");
  assert.equal(runtime.next_expected_actor, "INTEGRATION_VALIDATOR");
  assert.equal(runtime.next_expected_session, "integration_validator:wp-test-validator-v1");
  assert.equal(runtime.waiting_on, "MAIN_CONTAINMENT");
  assert.equal(runtime.attention_required, false);
});

test("evaluatePacketRuntimeProjectionDrift flags stale bootstrap runtime after direct review is complete", () => {
  const drift = evaluatePacketRuntimeProjectionDrift(
    [
      "## METADATA",
      "- **Status:** In Progress",
      "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
      "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A",
      "- PACKET_WIDENING_DECISION: NONE",
      "- PACKET_WIDENING_EVIDENCE: N/A",
      "- MAIN_CONTAINMENT_STATUS: NOT_STARTED",
      "- MERGED_MAIN_COMMIT: NONE",
      "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A",
    ].join("\n"),
    {
      current_packet_status: "In Progress",
      current_phase: "BOOTSTRAP",
      runtime_status: "submitted",
      main_containment_status: "NOT_STARTED",
    },
    {
      communicationEvaluation: {
        applicable: true,
        ok: true,
        state: "COMM_OK",
      },
    },
  );

  assert.equal(drift.ok, false);
  assert.deepEqual(drift.owner_classes, ["PACKET_CLOSEOUT_TRUTH", "RUNTIME_PROJECTION"]);
  assert.deepEqual(drift.repair_order, ["PACKET_CLOSEOUT_TRUTH", "RUNTIME_PROJECTION"]);
  assert.match(drift.owner_summary, /packet closeout truth and runtime projection/i);
  assert.match(drift.issues.join("\n"), /runtime\.current_phase is still BOOTSTRAP/i);
  assert.match(drift.issues.join("\n"), /runtime\.current_milestone .* should be VERDICT/i);
  assert.match(drift.issues.join("\n"), /CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN/i);
  assert.equal(
    drift.issue_details.find((detail) => detail.message.includes("CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN"))?.owner,
    "PACKET_CLOSEOUT_TRUTH",
  );
});

test("evaluatePacketRuntimeProjectionDrift isolates runtime-only ownership when packet closeout truth is already aligned", () => {
  const drift = evaluatePacketRuntimeProjectionDrift(
    packetFixture({
      status: "Validated (PASS)",
      containment: "CONTAINED_IN_MAIN",
    }),
    {
      current_packet_status: "Validated (PASS)",
      current_task_board_status: "DONE_VALIDATED",
      current_phase: "BOOTSTRAP",
      runtime_status: "submitted",
      main_containment_status: "CONTAINED_IN_MAIN",
    },
  );

  assert.equal(drift.ok, false);
  assert.deepEqual(drift.owner_classes, ["RUNTIME_PROJECTION"]);
  assert.deepEqual(drift.repair_order, ["RUNTIME_PROJECTION"]);
  assert.match(drift.owner_summary, /runtime projection/i);
  assert.ok(drift.issue_details.length >= 2);
  assert.ok(drift.issue_details.every((detail) => detail.owner === "RUNTIME_PROJECTION"));
  assert.match(drift.issues.join("\n"), /runtime\.current_phase .* should be STATUS_SYNC/i);
  assert.match(drift.issues.join("\n"), /runtime\.runtime_status .* should be completed/i);
});
