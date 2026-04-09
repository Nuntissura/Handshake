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
    packetText: packetFixture("In Progress"),
  });

  const nextPacketText = applyWpReviewPacketProjection(packetFixture("In Progress"), projection);

  assert.equal(projection.packetStatus, "In Progress");
  assert.equal(projection.taskBoardStatus, "IN_PROGRESS");
  assert.match(nextPacketText, /\*\*Status:\*\*\s*In Progress/);
  assert.match(nextPacketText, /Blockers:\s*WP validator review requires coder remediation/i);
  assert.match(nextPacketText, /Next:\s*CODER repairs against the authoritative latest VALIDATOR_REVIEW/i);
});

test("intent checkpoint review preserves ready packet status during validator-side bootstrap review", () => {
  const projection = deriveWpReviewPacketProjection({
    evaluation: {
      applicable: true,
      state: "COMM_WAITING_FOR_INTENT_CHECKPOINT",
    },
    autoRoute: {
      nextExpectedActor: "WP_VALIDATOR",
      waitingOn: "WP_VALIDATOR_INTENT_CHECKPOINT",
    },
    packetText: packetFixture("Ready for Dev"),
  });

  const nextPacketText = applyWpReviewPacketProjection(packetFixture("Ready for Dev"), projection);
  const runtime = applyWpReviewRuntimeProjection(
    {
      current_packet_status: "Ready for Dev",
      runtime_status: "submitted",
      current_phase: "BOOTSTRAP",
    },
    {
      evaluation: {
        applicable: true,
        state: "COMM_WAITING_FOR_INTENT_CHECKPOINT",
      },
    },
  );

  assert.equal(projection.packetStatus, null);
  assert.equal(projection.taskBoardStatus, null);
  assert.match(nextPacketText, /\*\*Status:\*\*\s*Ready for Dev/);
  assert.match(nextPacketText, /Bootstrap and skeleton clearance now belongs to the WP validator/i);
  assert.equal(runtime.runtime_status, "working");
  assert.equal(runtime.current_phase, "BOOTSTRAP");
  assert.equal(runtime.current_milestone, "SKELETON");
});

test("missing kickoff preserves ready packet status before coder claim", () => {
  const projection = deriveWpReviewPacketProjection({
    evaluation: {
      applicable: true,
      state: "COMM_MISSING_KICKOFF",
    },
    autoRoute: {
      nextExpectedActor: "WP_VALIDATOR",
      waitingOn: "VALIDATOR_KICKOFF",
    },
    packetText: packetFixture("Ready for Dev"),
  });

  const nextPacketText = applyWpReviewPacketProjection(packetFixture("Ready for Dev"), projection);

  assert.equal(projection.packetStatus, null);
  assert.equal(projection.taskBoardStatus, null);
  assert.match(nextPacketText, /\*\*Status:\*\*\s*Ready for Dev/);
  assert.match(nextPacketText, /Awaiting WP validator kickoff/i);
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

test("overlap microtask review projects validator-owned next action while implementation stays in progress", () => {
  const projection = deriveWpReviewPacketProjection({
    evaluation: {
      applicable: true,
      state: "COMM_WAITING_FOR_HANDOFF",
      counts: {
        overlapOpenReviewItems: 1,
      },
    },
    autoRoute: {
      nextExpectedActor: "WP_VALIDATOR",
      waitingOn: "WP_VALIDATOR_MICROTASK_REVIEW",
    },
    packetText: packetFixture("In Progress"),
  });

  const nextPacketText = applyWpReviewPacketProjection(packetFixture("In Progress"), projection);

  assert.equal(projection.packetStatus, "In Progress");
  assert.equal(projection.taskBoardStatus, "IN_PROGRESS");
  assert.match(nextPacketText, /previous microtask is awaiting WP validator overlap review/i);
  assert.match(nextPacketText, /WP_VALIDATOR reviews the open overlap microtask item/i);
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
  assert.equal(runtime.current_milestone, "MICROTASK");
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
