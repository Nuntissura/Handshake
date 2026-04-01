import assert from "node:assert/strict";
import test from "node:test";
import {
  applyWpReviewPacketProjection,
  applyWpReviewRuntimeProjection,
  deriveWpReviewPacketProjection,
} from "../scripts/lib/wp-review-projection-lib.mjs";

function packetFixture(status = "Ready for Dev") {
  return [
    "# Task Packet: WP-TEST-REVIEW-PROJECTION-v1",
    "",
    "## METADATA",
    `- **Status:** ${status}`,
    "",
    "## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)",
    "Verdict: PENDING",
    "Blockers: NONE",
    "Next: N/A",
    "",
  ].join("\n");
}

test("negative validator review projects packet truth back to active coder remediation", () => {
  const projection = deriveWpReviewPacketProjection({
    evaluation: {
      applicable: true,
      state: "COMM_REPAIR_REQUIRED",
    },
    autoRoute: {
      nextExpectedActor: "CODER",
      waitingOn: "CODER_REPAIR_HANDOFF",
    },
    packetText: packetFixture("Ready for Dev"),
  });

  const nextPacketText = applyWpReviewPacketProjection(packetFixture("Ready for Dev"), projection);

  assert.equal(projection.packetStatus, "In Progress");
  assert.equal(projection.taskBoardStatus, "IN_PROGRESS");
  assert.match(nextPacketText, /\*\*Status:\*\*\s*In Progress/);
  assert.match(nextPacketText, /Blockers:\s*WP validator review requires coder remediation/i);
  assert.match(nextPacketText, /Next:\s*CODER repairs against the latest VALIDATOR_REVIEW/i);
});

test("workflow invalidity projects packet truth into blocked state", () => {
  const projection = deriveWpReviewPacketProjection({
    evaluation: {
      applicable: true,
      state: "COMM_WORKFLOW_INVALID",
    },
    autoRoute: {
      nextExpectedActor: "ORCHESTRATOR",
      waitingOn: "WORKFLOW_INVALIDITY",
    },
    packetText: packetFixture("In Progress"),
  });

  const nextPacketText = applyWpReviewPacketProjection(packetFixture("In Progress"), projection);

  assert.equal(projection.packetStatus, "Blocked");
  assert.equal(projection.taskBoardStatus, "BLOCKED");
  assert.match(nextPacketText, /\*\*Status:\*\*\s*Blocked/);
  assert.match(nextPacketText, /workflow invalidity is active/i);
});

test("active review projection moves runtime out of stale bootstrap when remediation is required", () => {
  const runtime = applyWpReviewRuntimeProjection(
    {
      current_packet_status: "In Progress",
      runtime_status: "submitted",
      current_phase: "BOOTSTRAP",
    },
    {
      evaluation: {
        applicable: true,
        state: "COMM_REPAIR_REQUIRED",
      },
    },
  );

  assert.equal(runtime.runtime_status, "working");
  assert.equal(runtime.current_phase, "IMPLEMENTATION");
});

test("active review projection keeps terminal packet runtime untouched", () => {
  const runtime = applyWpReviewRuntimeProjection(
    {
      current_packet_status: "Validated (PASS)",
      runtime_status: "completed",
      current_phase: "STATUS_SYNC",
    },
    {
      evaluation: {
        applicable: true,
        state: "COMM_REPAIR_REQUIRED",
      },
    },
  );

  assert.equal(runtime.runtime_status, "completed");
  assert.equal(runtime.current_phase, "STATUS_SYNC");
});
