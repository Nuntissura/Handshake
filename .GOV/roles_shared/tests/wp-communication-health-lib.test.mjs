import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  deriveWpCommunicationAutoRoute,
  communicationMonitorState,
  evaluateWpCommunicationHealth,
} from "../scripts/lib/wp-communication-health-lib.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = path.resolve(__dirname, "../fixtures/wp-communication-health");

for (const fixtureName of fs.readdirSync(FIXTURES_DIR).filter((name) => name.endsWith(".json")).sort()) {
  const fixturePath = path.join(FIXTURES_DIR, fixtureName);
  const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));

  test(`wp communication health fixture: ${fixtureName} :: ${fixture.description}`, () => {
    const evaluation = evaluateWpCommunicationHealth(fixture.input);

    assert.equal(evaluation.applicable, fixture.expected.applicable, "applicable mismatch");
    assert.equal(evaluation.ok, fixture.expected.ok, "ok mismatch");
    assert.equal(evaluation.state, fixture.expected.state, "state mismatch");

    for (const monitorExpectation of fixture.monitor || []) {
      assert.equal(
        communicationMonitorState(evaluation, { stale: Boolean(monitorExpectation.stale) }),
        monitorExpectation.state,
        `monitor state mismatch for stale=${monitorExpectation.stale}`
      );
    }
  });
}

function baseInput({
  packetFormatVersion = "2026-03-22",
  communicationHealthGate = "HANDOFF_VERDICT_BLOCKING",
  receipts = [],
  runtimeStatus = {},
} = {}) {
  return {
    wpId: "WP-TEST-AUTO-ROUTE",
    stage: "STATUS",
    packetPath: ".GOV/task_packets/WP-TEST-AUTO-ROUTE/packet.md",
    workflowLane: "ORCHESTRATOR_MANAGED",
    packetFormatVersion,
    communicationContract: "DIRECT_REVIEW_V1",
    communicationHealthGate,
    receipts,
    runtimeStatus: {
      workflow_lane: "ORCHESTRATOR_MANAGED",
      wp_validator_of_record: "wpv-1",
      integration_validator_of_record: "intval-1",
      active_role_sessions: [
        {
          role: "CODER",
          session_id: "coder-1",
          authority_kind: "PRIMARY_CODER",
          validator_role_kind: null,
          worktree_dir: "D:/tmp/coder",
          state: "working",
          last_heartbeat_at: "2026-03-22T10:00:00Z",
        },
        {
          role: "WP_VALIDATOR",
          session_id: "wpv-1",
          authority_kind: "WP_VALIDATOR",
          validator_role_kind: "WP_VALIDATOR",
          worktree_dir: "D:/tmp/wpv",
          state: "waiting",
          last_heartbeat_at: "2026-03-22T10:00:01Z",
        },
        {
          role: "INTEGRATION_VALIDATOR",
          session_id: "intval-1",
          authority_kind: "INTEGRATION_VALIDATOR",
          validator_role_kind: "INTEGRATION_VALIDATOR",
          worktree_dir: "D:/tmp/intval",
          state: "waiting",
          last_heartbeat_at: "2026-03-22T10:00:02Z",
        },
      ],
      open_review_items: [],
      ...runtimeStatus,
    },
  };
}

test("auto route projects coder handoff into validator review wake state", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "VALIDATOR_KICKOFF",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.state, "COMM_WAITING_FOR_REVIEW");
  assert.equal(route.nextExpectedActor, "WP_VALIDATOR");
  assert.equal(route.nextExpectedSession, "wpv-1");
  assert.equal(route.validatorTrigger, "HANDOFF_READY");
  assert.equal(route.readyForValidation, true);
  assert.equal(route.notification, null, "explicit handoff target should not get a duplicate auto-route notification");
});

test("auto route sends coder back to initiate missing final review exchange", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "VALIDATOR_KICKOFF",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.state, "COMM_WAITING_FOR_FINAL_REVIEW");
  assert.equal(route.nextExpectedActor, "CODER");
  assert.equal(route.nextExpectedSession, "coder-1");
  assert.equal(route.waitingOn, "FINAL_REVIEW_EXCHANGE");
  assert.equal(route.validatorTrigger, "NONE");
  assert.equal(route.notification, null, "validator review already targets coder directly");
});

test("auto route wakes integration validator when final review request is open", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "VALIDATOR_KICKOFF",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        target_role: "INTEGRATION_VALIDATOR",
        target_session: "intval-1",
        correlation_id: "final-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:05:00Z",
      },
    ],
    runtimeStatus: {
      open_review_items: [
        {
          correlation_id: "final-1",
          receipt_kind: "REVIEW_REQUEST",
          summary: "Final review requested",
          opened_by_role: "CODER",
          opened_by_session: "coder-1",
          target_role: "INTEGRATION_VALIDATOR",
          target_session: "intval-1",
          spec_anchor: null,
          packet_row_ref: null,
          requires_ack: true,
          opened_at: "2026-03-22T10:05:00Z",
          updated_at: "2026-03-22T10:05:00Z",
        },
      ],
    },
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.state, "COMM_BLOCKED_OPEN_ITEMS");
  assert.equal(route.nextExpectedActor, "INTEGRATION_VALIDATOR");
  assert.equal(route.nextExpectedSession, "intval-1");
  assert.equal(route.validatorTrigger, "BLOCKED_NEEDS_VALIDATOR");
  assert.equal(route.notification, null, "explicit review request already targets integration validator");
});

test("auto route notifies orchestrator when the direct review lane is complete", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "VALIDATOR_KICKOFF",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        target_role: "INTEGRATION_VALIDATOR",
        target_session: "intval-1",
        correlation_id: "final-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:05:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "INTEGRATION_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "final-1",
        ack_for: "final-1",
        timestamp_utc: "2026-03-22T10:06:00Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.state, "COMM_OK");
  assert.equal(route.nextExpectedActor, "ORCHESTRATOR");
  assert.equal(route.waitingOn, "VERDICT_PROGRESSION");
  assert.deepEqual(route.notification, {
    targetRole: "ORCHESTRATOR",
    targetSession: null,
    summary: "AUTO_ROUTE: direct review lane complete; orchestrator verdict progression ready",
  });
});
