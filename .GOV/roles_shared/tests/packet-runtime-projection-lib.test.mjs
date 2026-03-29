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
} = {}) {
  return [
    "## METADATA",
    `- **Status:** ${status}`,
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
      open_review_items: [{ correlation_id: "review-1" }],
    },
    packetFixture(),
  );

  assert.equal(runtime.current_phase, "STATUS_SYNC");
  assert.equal(runtime.runtime_status, "completed");
  assert.equal(runtime.next_expected_actor, "NONE");
  assert.equal(runtime.waiting_on, "CLOSED");
  assert.equal(runtime.ready_for_validation, false);
  assert.equal(runtime.attention_required, false);
  assert.deepEqual(runtime.open_review_items, []);
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
  assert.match(drift.issues.join("\n"), /runtime\.current_phase is still BOOTSTRAP/i);
  assert.match(drift.issues.join("\n"), /CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN/i);
});
