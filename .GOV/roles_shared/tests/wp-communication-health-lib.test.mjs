import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  deriveWpCommunicationAutoRoute,
  communicationMonitorState,
  evaluateWpCommunicationBoundary,
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
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
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
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
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
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        actor_session: "coder-1",
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
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "INTEGRATION_VALIDATOR",
        target_session: "intval-1",
        correlation_id: "final-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:05:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "INTEGRATION_VALIDATOR",
        actor_session: "intval-1",
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

test("boundary check fails when runtime projection drifts from the status-derived route", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "VALIDATOR_KICKOFF",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
    ],
    runtimeStatus: {
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
      waiting_on: "CODER_HANDOFF",
      waiting_on_session: "coder-1",
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      attention_required: false,
    },
  });

  const statusEvaluation = evaluateWpCommunicationHealth(input);
  const boundary = evaluateWpCommunicationBoundary({
    stage: "HANDOFF",
    statusEvaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
    pendingNotifications: [],
  });

  assert.equal(statusEvaluation.state, "COMM_WAITING_FOR_REVIEW");
  assert.equal(boundary.ok, false);
  assert.match(boundary.issues.join("\n"), /runtime\.next_expected_actor expected WP_VALIDATOR but found CODER/);
});

test("boundary check fails when the next actor still has unread direct-review notifications", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "VALIDATOR_KICKOFF",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:01:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
    ],
    runtimeStatus: {
      next_expected_actor: "WP_VALIDATOR",
      next_expected_session: "wpv-1",
      waiting_on: "WP_VALIDATOR_REVIEW",
      waiting_on_session: "wpv-1",
      validator_trigger: "HANDOFF_READY",
      validator_trigger_reason: "Coder handoff recorded; WP validator review required",
      ready_for_validation: true,
      ready_for_validation_reason: "Coder handoff recorded; WP validator review required",
      attention_required: false,
    },
  });

  const statusEvaluation = evaluateWpCommunicationHealth(input);
  const boundary = evaluateWpCommunicationBoundary({
    stage: "HANDOFF",
    statusEvaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
    pendingNotifications: [
      {
        source_kind: "CODER_HANDOFF",
        source_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        summary: "Handoff ready",
      },
    ],
  });

  assert.equal(statusEvaluation.state, "COMM_WAITING_FOR_REVIEW");
  assert.equal(boundary.ok, false);
  assert.match(boundary.issues.join("\n"), /Pending notifications for WP_VALIDATOR:wpv-1 must be acknowledged before HANDOFF can pass/);
});

test("health check rejects mixed-session review pairs even when correlations match", () => {
  const input = baseInput({
    packetFormatVersion: "2026-03-21",
    receipts: [
      {
        receipt_kind: "VALIDATOR_KICKOFF",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-old",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kick-1",
        ack_for: null,
        timestamp_utc: "2026-03-24T10:00:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-new",
        correlation_id: "kick-1",
        ack_for: "kick-1",
        timestamp_utc: "2026-03-24T10:01:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-new",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-24T10:02:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-old",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        timestamp_utc: "2026-03-24T10:03:00Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  assert.equal(evaluation.ok, false);
  assert.equal(evaluation.state, "COMM_WAITING_FOR_INTENT");
});

test("health check rejects resolution receipts with mismatched ack_for", () => {
  const input = baseInput({
    packetFormatVersion: "2026-03-21",
    receipts: [
      {
        receipt_kind: "VALIDATOR_KICKOFF",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kick-1",
        ack_for: null,
        timestamp_utc: "2026-03-24T10:00:00Z",
      },
      {
        receipt_kind: "CODER_INTENT",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kick-1",
        ack_for: "wrong-kick",
        timestamp_utc: "2026-03-24T10:01:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        timestamp_utc: "2026-03-24T10:02:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "wrong-handoff",
        timestamp_utc: "2026-03-24T10:03:00Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  assert.equal(evaluation.ok, false);
  assert.equal(evaluation.state, "COMM_WAITING_FOR_INTENT");
});

test("workflow invalidity receipts block communication health and route back to orchestrator", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "WORKFLOW_INVALIDITY",
        workflow_invalidity_code: "ORCHESTRATOR_MANAGED_CHECKPOINT_RELAPSE",
        actor_role: "ORCHESTRATOR",
        actor_session: "orch-1",
        target_role: "ORCHESTRATOR",
        target_session: null,
        correlation_id: null,
        ack_for: null,
        summary: "Manual checkpoint helper was invoked for an orchestrator-managed WP",
        timestamp_utc: "2026-03-22T10:00:00Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.ok, false);
  assert.equal(evaluation.state, "COMM_WORKFLOW_INVALID");
  assert.match(evaluation.details.join("\n"), /latest_invalidity_code=ORCHESTRATOR_MANAGED_CHECKPOINT_RELAPSE/);
  assert.equal(route.nextExpectedActor, "ORCHESTRATOR");
  assert.equal(route.waitingOn, "WORKFLOW_INVALIDITY");
  assert.equal(route.attentionRequired, true);
});
