import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  buildRoleInbox,
  deriveWpCommunicationAutoRoute,
  deriveActiveWpNotificationProjection,
  deriveLatestValidatorAssessment,
  deriveValidatorReviewOutcome,
  communicationMonitorState,
  evaluateWpCommunicationBoundary,
  evaluateWpCommunicationHealth,
} from "../scripts/lib/wp-communication-health-lib.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = path.resolve(__dirname, "../fixtures/wp-communication-health");
const repoRoot = path.resolve(__dirname, "..", "..", "..");

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
  wpId = "WP-TEST-AUTO-ROUTE",
  packetFormatVersion = "2026-03-22",
  communicationHealthGate = "HANDOFF_VERDICT_BLOCKING",
  packetContent = "",
  receipts = [],
  runtimeStatus = {},
} = {}) {
  return {
    wpId,
    stage: "STATUS",
    packetPath: `.GOV/task_packets/${wpId}/packet.md`,
    packetContent,
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

function writeMicrotask(packetDir, wpId, mtId, clause, codeSurfaces) {
  fs.writeFileSync(
    path.join(packetDir, `${mtId}.md`),
    [
      `# ${mtId}: ${clause}`,
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      `- MT_ID: ${mtId}`,
      `- CLAUSE: ${clause}`,
      `- CODE_SURFACES: ${codeSurfaces.join("; ")}`,
      "- EXPECTED_TESTS: cargo test demo",
      "- DEPENDS_ON: NONE",
    ].join("\n"),
    "utf8",
  );
}

function contractHeavyPacketFixture() {
  return `# Task Packet: WP-TEST-AUTO-ROUTE

## METADATA
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- PACKET_FORMAT_VERSION: 2026-03-29
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
- IN_SCOPE_PATHS:
  - src/demo.rs
  - src/demo_support.rs

## CLAUSE_CLOSURE_MATRIX
- CLAUSE | CODER_STATUS=PROVED | VALIDATOR_STATUS=PENDING
`.trim();
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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

test("wp validator final REVIEW_RESPONSE closes coder handoff review and returns control to orchestrator", () => {
  const input = baseInput({
    packetFormatVersion: "2026-04-06",
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        receipt_kind: "REVIEW_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        summary: "FINAL WP PASS.",
        timestamp_utc: "2026-03-22T10:04:00Z",
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

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.state, "COMM_OK");
  assert.equal(route.nextExpectedActor, "ORCHESTRATOR");
  assert.equal(route.waitingOn, "VERDICT_PROGRESSION");
});

test("terminal closed runtime keeps auto route at NONE/CLOSED instead of projecting verdict progression", () => {
  const input = baseInput({
    packetFormatVersion: "2026-04-06",
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        receipt_kind: "REVIEW_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        summary: "FINAL WP PASS.",
        timestamp_utc: "2026-03-22T10:03:30Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "INTEGRATION_VALIDATOR",
        target_session: "intval-1",
        correlation_id: "final-review-1",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:03:45Z",
      },
      {
        receipt_kind: "REVIEW_RESPONSE",
        actor_role: "INTEGRATION_VALIDATOR",
        actor_session: "intval-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "final-review-1",
        ack_for: "final-review-1",
        summary: "OUTDATED_ONLY. Follow-on WP required for adjacent current-main scope.",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
    ],
    runtimeStatus: {
      current_packet_status: "Validated (OUTDATED_ONLY)",
      runtime_status: "completed",
      current_phase: "STATUS_SYNC",
      next_expected_actor: "NONE",
      next_expected_session: null,
      waiting_on: "CLOSED",
      waiting_on_session: null,
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      attention_required: false,
      active_role_sessions: [],
      open_review_items: [],
    },
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });
  const boundary = evaluateWpCommunicationBoundary({
    stage: "VERDICT",
    statusEvaluation: evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
    pendingNotifications: [],
  });

  assert.equal(evaluation.state, "COMM_OK");
  assert.equal(route.nextExpectedActor, "NONE");
  assert.equal(route.waitingOn, "CLOSED");
  assert.equal(boundary.ok, true, JSON.stringify(boundary, null, 2));
});

test("communication health ignores historical invalidity once a later repair receipt exists", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "WORKFLOW_INVALIDITY",
        actor_role: "ORCHESTRATOR",
        actor_session: "orch-1",
        target_role: "ORCHESTRATOR",
        target_session: null,
        correlation_id: null,
        ack_for: null,
        workflow_invalidity_code: "SCOPE_CONFLICT",
        timestamp_utc: "2026-03-22T10:00:00Z",
      },
      {
        receipt_kind: "REPAIR",
        actor_role: "ORCHESTRATOR",
        actor_session: "orch-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: null,
        ack_for: null,
        timestamp_utc: "2026-03-22T10:00:30Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);

  assert.equal(evaluation.state, "COMM_MISSING_KICKOFF");
  assert.equal(evaluation.activeWorkflowInvalidityCode, null);
});

test("startup communication health stays green once the lane has already advanced past bootstrap", () => {
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
      },
    ],
    runtimeStatus: {
      active_role_sessions: [],
      next_expected_actor: "CODER",
      waiting_on: "CODER_HANDOFF",
    },
  });

  const evaluation = evaluateWpCommunicationHealth({
    ...input,
    stage: "STARTUP",
    actorRole: "CODER",
    actorSession: "coder-1",
  });

  assert.equal(evaluation.ok, true, JSON.stringify(evaluation, null, 2));
  assert.equal(evaluation.state, "COMM_OK");
  assert.match(evaluation.message, /already satisfied earlier in this lane/i);
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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

test("negative validator review routes the lane back to coder remediation instead of final review", () => {
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        summary: "Repair required. Findings: task-board parity is incomplete and the signed scope was exceeded.",
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

  assert.equal(evaluation.state, "COMM_REPAIR_REQUIRED");
  assert.equal(evaluation.latestWpValidatorReviewOutcome, "REPAIR_REQUIRED");
  assert.equal(route.nextExpectedActor, "CODER");
  assert.equal(route.nextExpectedSession, "coder-1");
  assert.equal(route.waitingOn, "CODER_REPAIR_HANDOFF");
  assert.equal(route.notification, null, "validator review already targets coder directly");
});

test("contract-heavy packets wait for WP validator checkpoint review after coder intent", () => {
  const input = baseInput({
    packetContent: contractHeavyPacketFixture(),
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
        summary: "Implementation order drafted.",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.state, "COMM_WAITING_FOR_INTENT_CHECKPOINT");
  assert.match(evaluation.details.join("\n"), /intent_checkpoint_required=YES/);
  assert.equal(route.nextExpectedActor, "WP_VALIDATOR");
  assert.equal(route.nextExpectedSession, "wpv-1");
  assert.equal(route.waitingOn, "WP_VALIDATOR_INTENT_CHECKPOINT");
  assert.equal(route.validatorTrigger, "BLOCKED_NEEDS_VALIDATOR");
  assert.equal(route.readyForValidation, true);
});

test("all orchestrator-managed lanes require explicit WP validator bootstrap and skeleton clearance after coder intent", () => {
  const input = baseInput({
    packetContent: `# Task Packet: WP-TEST-AUTO-ROUTE

## METADATA
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- PACKET_FORMAT_VERSION: 2026-03-29
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
- IN_SCOPE_PATHS:
  - src/demo.rs
`.trim(),
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
        summary: "Bootstrap, skeleton, and first implementation slice plan drafted.",
        microtask_contract: {
          scope_ref: "BOOTSTRAP/CX-LANE-001",
          file_targets: ["src/demo.rs"],
          proof_commands: ["cargo test demo::tests::smoke -- --exact"],
          phase_gate: "SKELETON",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
    ],
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  assert.equal(evaluation.state, "COMM_WAITING_FOR_INTENT_CHECKPOINT");
  assert.match(evaluation.details.join("\n"), /bootstrap_skeleton_validator_gate/);
});

test("validator checkpoint clearance unlocks handoff after contract-heavy intent review", () => {
  const input = baseInput({
    packetContent: contractHeavyPacketFixture(),
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
        summary: "Implementation order drafted.",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Intent checkpoint cleared; proceed to implementation and full handoff.",
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

  assert.equal(evaluation.state, "COMM_WAITING_FOR_HANDOFF");
  assert.equal(route.nextExpectedActor, "CODER");
  assert.equal(route.waitingOn, "CODER_HANDOFF");
});

test("later WP validator review also clears the bootstrap checkpoint once handoff progression has begun", () => {
  const input = baseInput({
    packetContent: contractHeavyPacketFixture(),
    packetFormatVersion: "2026-03-29",
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
        summary: "Bootstrap, skeleton, and implementation order drafted.",
        microtask_contract: {
          scope_ref: "MT-001",
          file_targets: ["src/demo.rs", "src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::smoke -- --exact"],
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        timestamp_utc: "2026-03-22T10:05:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        summary: "PASS. Ready for final review.",
        timestamp_utc: "2026-03-22T10:06:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "INTEGRATION_VALIDATOR",
        target_session: "intval-1",
        correlation_id: "final-review-1",
        timestamp_utc: "2026-03-22T10:07:00Z",
      },
      {
        receipt_kind: "REVIEW_RESPONSE",
        actor_role: "INTEGRATION_VALIDATOR",
        actor_session: "intval-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "final-review-1",
        ack_for: "final-review-1",
        summary: "PASS. No further coder repair is requested.",
        timestamp_utc: "2026-03-22T10:08:00Z",
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
});

test("overlap microtask review requests do not block coder progression while backlog stays bounded", () => {
  const input = baseInput({
    packetContent: contractHeavyPacketFixture(),
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
        summary: "Intent drafted.",
        microtask_contract: {
          scope_ref: "BOOTSTRAP/CX-LANE-001",
          file_targets: ["src/demo.rs", "src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::smoke -- --exact"],
          phase_gate: "SKELETON",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        microtask_contract: {
          phase_gate: "SKELETON",
          review_outcome: "UNKNOWN",
        },
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "micro-1",
        ack_for: null,
        summary: "Review completed microtask 1 while I continue microtask 2.",
        microtask_contract: {
          scope_ref: "CLAUSE_CLOSURE_MATRIX/CX-MICRO-001",
          file_targets: ["src/demo.rs"],
          proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
          risk_focus: "surface drift",
          review_mode: "OVERLAP",
          phase_gate: "MICROTASK",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
    ],
    runtimeStatus: {
      open_review_items: [
        {
          correlation_id: "micro-1",
          receipt_kind: "REVIEW_REQUEST",
          summary: "Review completed microtask 1 while I continue microtask 2.",
          opened_by_role: "CODER",
          opened_by_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          spec_anchor: "CX-MICRO-001",
          packet_row_ref: "CLAUSE_CLOSURE_MATRIX",
          microtask_contract: {
            scope_ref: "CLAUSE_CLOSURE_MATRIX/CX-MICRO-001",
            file_targets: ["src/demo.rs"],
            proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
            review_mode: "OVERLAP",
            phase_gate: "MICROTASK",
            expected_receipt_kind: "VALIDATOR_RESPONSE",
          },
          requires_ack: true,
          opened_at: "2026-03-22T10:04:00Z",
          updated_at: "2026-03-22T10:04:00Z",
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
  assert.equal(evaluation.state, "COMM_WAITING_FOR_HANDOFF");
  assert.equal(evaluation.counts.overlapOpenReviewItems, 1);
  assert.equal(evaluation.counts.blockingOpenReviewItems, 0);
  assert.equal(route.nextExpectedActor, "CODER");
  assert.equal(route.waitingOn, "CODER_HANDOFF");
  assert.equal(route.validatorTrigger, "MICROTASK_REVIEW_READY");
  assert.equal(route.secondaryNotifications.length, 1);
  assert.equal(route.secondaryNotifications[0].targetRole, "WP_VALIDATOR");
  assert.equal(route.secondaryNotifications[0].targetSession, "wpv-1");
  assert.equal(route.secondaryNotifications[0].autoRelay, true);
});

test("overlap repair debt is deferred until the coder closes the current active microtask", () => {
  const wpId = "WP-TEST-AUTO-ROUTE-DEFERRED-REPAIR-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(packetDir, wpId, "MT-001", "Review-first scope [CX-MT-201]", ["src/review_first.rs"]);
  writeMicrotask(packetDir, wpId, "MT-002", "Current active scope [CX-MT-202]", ["src/current_active.rs"]);

  try {
    const input = baseInput({
      wpId,
      packetContent: contractHeavyPacketFixture(),
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
          summary: "Intent drafted.",
          microtask_contract: {
            scope_ref: "MT-001",
            file_targets: ["src/review_first.rs"],
            proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
            phase_gate: "MICROTASK",
            expected_receipt_kind: "VALIDATOR_RESPONSE",
          },
          timestamp_utc: "2026-03-22T10:02:00Z",
        },
        {
          receipt_kind: "VALIDATOR_RESPONSE",
          actor_role: "WP_VALIDATOR",
          actor_session: "wpv-1",
          target_role: "CODER",
          target_session: "coder-1",
          correlation_id: "kickoff-1",
          ack_for: "kickoff-1",
          summary: "Bootstrap cleared.",
          timestamp_utc: "2026-03-22T10:03:00Z",
        },
        {
          receipt_kind: "REVIEW_REQUEST",
          actor_role: "CODER",
          actor_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          correlation_id: "micro-1",
          summary: "MT-001 review_mode=OVERLAP: review the completed slice while I continue.",
          microtask_contract: {
            scope_ref: "MT-001",
            file_targets: ["src/review_first.rs"],
            proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
            review_mode: "OVERLAP",
            phase_gate: "MICROTASK",
            expected_receipt_kind: "REVIEW_RESPONSE",
          },
          timestamp_utc: "2026-03-22T10:04:00Z",
        },
        {
          receipt_kind: "CODER_INTENT",
          actor_role: "CODER",
          actor_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          correlation_id: "intent-2",
          ack_for: "micro-1",
          summary: "Advance into the next declared microtask.",
          microtask_contract: {
            scope_ref: "MT-002",
            file_targets: ["src/current_active.rs"],
            proof_commands: ["cargo test demo::tests::micro_2 -- --exact"],
            phase_gate: "MICROTASK",
            expected_receipt_kind: "REVIEW_RESPONSE",
          },
          timestamp_utc: "2026-03-22T10:05:00Z",
        },
        {
          receipt_kind: "REVIEW_RESPONSE",
          actor_role: "WP_VALIDATOR",
          actor_session: "wpv-1",
          target_role: "CODER",
          target_session: "coder-1",
          correlation_id: "micro-1",
          ack_for: "micro-1",
          summary: "Repair required on MT-001 before wider closure.",
          microtask_contract: {
            scope_ref: "MT-001",
            review_mode: "OVERLAP",
            review_outcome: "REPAIR_REQUIRED",
          },
          timestamp_utc: "2026-03-22T10:06:00Z",
        },
      ],
      runtimeStatus: {
        open_review_items: [],
      },
    });

    const evaluation = evaluateWpCommunicationHealth(input);
    const route = deriveWpCommunicationAutoRoute({
      evaluation,
      runtimeStatus: input.runtimeStatus,
      latestReceipt: input.receipts.at(-1),
    });

    assert.equal(evaluation.state, "COMM_DEFERRED_REPAIR_QUEUE");
    assert.equal(evaluation.counts.deferredRepairQueued, 1);
    assert.equal(evaluation.correlations.microtaskRepair, "micro-1");
    assert.match(evaluation.details.join("\n"), /deferred_repair_queue=MT-001->after:MT-002/);
    assert.equal(route.nextExpectedActor, "CODER");
    assert.equal(route.waitingOn, "CURRENT_MICROTASK_COMPLETION_BEFORE_REPAIR");
    assert.equal(route.notification, null);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("overlap routing recovers from MT review requests that omitted microtask_contract but kept MT packet_row_ref", () => {
  const input = baseInput({
    packetContent: contractHeavyPacketFixture(),
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
        summary: "Intent drafted.",
        microtask_contract: {
          scope_ref: "BOOTSTRAP/CX-LANE-001",
          file_targets: ["src/demo.rs", "src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::smoke -- --exact"],
          phase_gate: "SKELETON",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap cleared.",
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "micro-1",
        summary: "MT-001 review_mode=OVERLAP: review the completed slice while I continue.",
        packet_row_ref: "MT-001",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
    ],
    runtimeStatus: {
      open_review_items: [
        {
          correlation_id: "micro-1",
          receipt_kind: "REVIEW_REQUEST",
          summary: "MT-001 review_mode=OVERLAP: review the completed slice while I continue.",
          opened_by_role: "CODER",
          opened_by_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          packet_row_ref: "MT-001",
          microtask_contract: null,
          requires_ack: true,
          opened_at: "2026-03-22T10:04:00Z",
          updated_at: "2026-03-22T10:04:00Z",
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

  assert.equal(evaluation.state, "COMM_WAITING_FOR_HANDOFF");
  assert.equal(evaluation.counts.overlapOpenReviewItems, 1);
  assert.equal(route.nextExpectedActor, "CODER");
  assert.equal(route.validatorTrigger, "MICROTASK_REVIEW_READY");
  assert.equal(route.secondaryNotifications.length, 1);
  assert.equal(route.secondaryNotifications[0].targetRole, "WP_VALIDATOR");
});

test("overlap microtask review backlog becomes blocking once the bounded queue is exceeded", () => {
  const overlapItems = ["micro-1", "micro-2"].map((id, index) => ({
    correlation_id: id,
    receipt_kind: "REVIEW_REQUEST",
    summary: `Review ${id}`,
    opened_by_role: "CODER",
    opened_by_session: "coder-1",
    target_role: "WP_VALIDATOR",
    target_session: "wpv-1",
    spec_anchor: `CX-MICRO-00${index + 1}`,
    packet_row_ref: "CLAUSE_CLOSURE_MATRIX",
    microtask_contract: {
      scope_ref: `CLAUSE_CLOSURE_MATRIX/CX-MICRO-00${index + 1}`,
      file_targets: ["src/demo.rs"],
      proof_commands: [`cargo test demo::tests::micro_${index + 1} -- --exact`],
      review_mode: "OVERLAP",
      phase_gate: "MICROTASK",
      expected_receipt_kind: "VALIDATOR_RESPONSE",
    },
    requires_ack: true,
    opened_at: `2026-03-22T10:0${index + 4}:00Z`,
    updated_at: `2026-03-22T10:0${index + 4}:00Z`,
  }));
  const input = baseInput({
    packetContent: contractHeavyPacketFixture(),
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
        summary: "Intent drafted.",
        microtask_contract: {
          scope_ref: "BOOTSTRAP/CX-LANE-001",
          file_targets: ["src/demo.rs", "src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::smoke -- --exact"],
          phase_gate: "SKELETON",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
    ],
    runtimeStatus: {
      open_review_items: overlapItems,
    },
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: {
      receipt_kind: "REVIEW_REQUEST",
      actor_role: "CODER",
      actor_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-1",
    },
  });

  assert.equal(evaluation.state, "COMM_BLOCKED_OPEN_ITEMS");
  assert.match(evaluation.message, /bounded validator queue/i);
  assert.match(evaluation.details.join("\n"), /overlap_backpressure_limit=1/);
  assert.equal(route.nextExpectedActor, "WP_VALIDATOR");
});

test("validator review outcome honors explicit microtask review_outcome overrides", () => {
  assert.equal(
    deriveValidatorReviewOutcome({
      summary: "Advisory review complete: suitable for integration review.",
      microtask_contract: {
        review_outcome: "REPAIR_REQUIRED",
      },
    }),
    "REPAIR_REQUIRED",
  );
});

test("validator review outcome treats unhyphenated rehandoff guidance as repair required", () => {
  assert.equal(
    deriveValidatorReviewOutcome({
      summary: "Repair workflows/api semantics and rehandoff.",
    }),
    "REPAIR_REQUIRED",
  );
});

test("latest validator assessment reports FAIL for repair-required validator review", () => {
  const assessment = deriveLatestValidatorAssessment([
    {
      receipt_kind: "CODER_HANDOFF",
      actor_role: "CODER",
      actor_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-1",
      correlation_id: "handoff-1",
      ack_for: null,
      summary: "Ready for review.",
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
      summary: "Repair required. Findings: fix mailbox projection and re-handoff.",
      timestamp_utc: "2026-03-22T10:04:00Z",
    },
  ]);

  assert.equal(assessment?.actorRole, "WP_VALIDATOR");
  assert.equal(assessment?.receiptKind, "VALIDATOR_REVIEW");
  assert.equal(assessment?.verdict, "FAIL");
  assert.equal(assessment?.reviewOutcome, "REPAIR_REQUIRED");
  assert.match(assessment?.reason || "", /Repair required/i);
});

test("latest validator assessment reports PASS for approved final review response", () => {
  const assessment = deriveLatestValidatorAssessment([
    {
      receipt_kind: "REVIEW_REQUEST",
      actor_role: "CODER",
      actor_session: "coder-1",
      target_role: "INTEGRATION_VALIDATOR",
      target_session: "intval-1",
      correlation_id: "final-1",
      ack_for: null,
      summary: "Please review final lane.",
      timestamp_utc: "2026-03-22T10:05:00Z",
    },
    {
      receipt_kind: "REVIEW_RESPONSE",
      actor_role: "INTEGRATION_VALIDATOR",
      actor_session: "intval-1",
      target_role: "CODER",
      target_session: "coder-1",
      correlation_id: "final-1",
      ack_for: "final-1",
      summary: "Approved for final review. Suitable for integration review closure.",
      timestamp_utc: "2026-03-22T10:06:00Z",
    },
  ]);

  assert.equal(assessment?.actorRole, "INTEGRATION_VALIDATOR");
  assert.equal(assessment?.receiptKind, "REVIEW_RESPONSE");
  assert.equal(assessment?.verdict, "PASS");
  assert.equal(assessment?.reviewOutcome, "APPROVED_FOR_FINAL_REVIEW");
});

test("latest validator assessment reports PASS for explicit PASS summaries", () => {
  const assessment = deriveLatestValidatorAssessment([
    {
      receipt_kind: "VALIDATOR_REVIEW",
      actor_role: "WP_VALIDATOR",
      actor_session: "wpv-1",
      target_role: "CODER",
      target_session: "coder-1",
      correlation_id: "handoff-1",
      ack_for: "handoff-1",
      summary: "PASS. No blocking findings remain in the committed reviewable state.",
      timestamp_utc: "2026-03-22T10:06:00Z",
    },
  ]);

  assert.equal(assessment?.actorRole, "WP_VALIDATOR");
  assert.equal(assessment?.receiptKind, "VALIDATOR_REVIEW");
  assert.equal(assessment?.verdict, "PASS");
});

test("latest validator assessment collapses duplicate decisive approvals for the same review round", () => {
  const assessment = deriveLatestValidatorAssessment([
    {
      receipt_kind: "CODER_HANDOFF",
      actor_role: "CODER",
      actor_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-1",
      correlation_id: "handoff-1",
      ack_for: null,
      summary: "Ready for review.",
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
      summary: "Approved for final review. Suitable for integration review closure.",
      timestamp_utc: "2026-03-22T10:04:00Z",
    },
    {
      receipt_kind: "VALIDATOR_REVIEW",
      actor_role: "WP_VALIDATOR",
      actor_session: "wpv-1",
      target_role: "CODER",
      target_session: "coder-1",
      correlation_id: "handoff-1",
      ack_for: "handoff-1",
      summary: "Approved for final review. Suitable for integration review closure.",
      timestamp_utc: "2026-03-22T10:05:00Z",
    },
  ]);

  assert.equal(assessment?.verdict, "PASS");
  assert.equal(assessment?.timestampUtc, "2026-03-22T10:04:00Z");
  assert.equal(assessment?.suppressedDuplicateCount, 1);
});

test("latest validator assessment treats a re-opened handoff as a new review round even on the same correlation", () => {
  const assessment = deriveLatestValidatorAssessment([
    {
      receipt_kind: "CODER_HANDOFF",
      actor_role: "CODER",
      actor_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-1",
      correlation_id: "handoff-1",
      ack_for: null,
      summary: "Ready for review.",
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
      summary: "Repair required. Findings: retry after remediation.",
      timestamp_utc: "2026-03-22T10:04:00Z",
    },
    {
      receipt_kind: "CODER_HANDOFF",
      actor_role: "CODER",
      actor_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-1",
      correlation_id: "handoff-1",
      ack_for: null,
      summary: "Re-handoff after remediation.",
      timestamp_utc: "2026-03-22T10:05:00Z",
    },
    {
      receipt_kind: "VALIDATOR_REVIEW",
      actor_role: "WP_VALIDATOR",
      actor_session: "wpv-1",
      target_role: "CODER",
      target_session: "coder-1",
      correlation_id: "handoff-1",
      ack_for: "handoff-1",
      summary: "Approved for final review. Suitable for integration review closure.",
      timestamp_utc: "2026-03-22T10:06:00Z",
    },
  ]);

  assert.equal(assessment?.verdict, "PASS");
  assert.equal(assessment?.timestampUtc, "2026-03-22T10:06:00Z");
  assert.equal(assessment?.suppressedDuplicateCount, 0);
});

test("a newer coder re-handoff takes precedence over an older repaired review pair", () => {
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        summary: "Repair required. Findings: repair task-board parity and re-handoff.",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-2",
        ack_for: null,
        timestamp_utc: "2026-03-22T10:05:00Z",
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
  assert.equal(evaluation.correlations.handoff, "handoff-2");
  assert.equal(route.nextExpectedActor, "WP_VALIDATOR");
  assert.equal(route.waitingOn, "WP_VALIDATOR_REVIEW");
});

test("re-opened handoff on the same correlation binds to the latest validator review reply", () => {
  const packetContent = contractHeavyPacketFixture();
  const input = baseInput({
    packetFormatVersion: "2026-03-29",
    packetContent,
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        summary: "Ready for review.",
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
        summary: "Repair required. Findings: retry after remediation and rehandoff.",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
      {
        receipt_kind: "CODER_HANDOFF",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        ack_for: null,
        summary: "Reopened handoff after remediation.",
        timestamp_utc: "2026-03-22T10:05:00Z",
      },
      {
        receipt_kind: "VALIDATOR_REVIEW",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        ack_for: "handoff-1",
        summary: "PASS. No blocking findings remain in the repaired committed state.",
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

  assert.equal(evaluation.state, "COMM_WAITING_FOR_FINAL_REVIEW");
  assert.equal(route.nextExpectedActor, "CODER");
  assert.equal(route.waitingOn, "FINAL_REVIEW_EXCHANGE");
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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

test("contained-main terminal closeout collapses verdict progression to NONE/CLOSED", () => {
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        summary: "PASS. No blocking findings remain in the committed reviewable state.",
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
    runtimeStatus: {
      runtime_status: "completed",
      current_packet_status: "Validated (PASS)",
      main_containment_status: "CONTAINED_IN_MAIN",
      current_main_compatibility_status: "COMPATIBLE",
      current_task_board_status: "DONE_VALIDATED",
      next_expected_actor: "NONE",
      next_expected_session: null,
      waiting_on: "CLOSED",
      waiting_on_session: null,
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      attention_required: false,
    },
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });
  const boundary = evaluateWpCommunicationBoundary({
    stage: "VERDICT",
    statusEvaluation: evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
    pendingNotifications: [],
  });

  assert.equal(evaluation.state, "COMM_OK");
  assert.equal(route.nextExpectedActor, "NONE");
  assert.equal(route.waitingOn, "CLOSED");
  assert.equal(route.notification, null);
  assert.equal(boundary.ok, true);
});

test("active notification projection keeps only the live route notification for the current correlation", () => {
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        summary: "Repair required. Findings remain on the active handoff.",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
    ],
  });
  const statusEvaluation = evaluateWpCommunicationHealth(input);
  const projection = deriveActiveWpNotificationProjection({
    statusEvaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
    pendingNotifications: [
      {
        source_kind: "REVIEW_REQUEST",
        source_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "older-handoff",
        timestamp_utc: "2026-03-22T10:03:00Z",
        summary: "Older review request no longer blocks the live route",
      },
      {
        source_kind: "VALIDATOR_REVIEW",
        source_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        timestamp_utc: "2026-03-22T10:04:00Z",
        summary: "Earlier duplicate validator review",
      },
      {
        source_kind: "VALIDATOR_REVIEW",
        source_role: "WP_VALIDATOR",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        timestamp_utc: "2026-03-22T10:04:30Z",
        summary: "Latest validator review for the active remediation cycle",
      },
      {
        source_kind: "GOVERNANCE_CHECKPOINT",
        source_role: "WP_VALIDATOR",
        target_role: "ORCHESTRATOR",
        correlation_id: "older-handoff",
        timestamp_utc: "2026-03-22T10:04:31Z",
        summary: "Historical checkpoint that should stay in history",
      },
    ],
  });

  assert.equal(statusEvaluation.state, "COMM_REPAIR_REQUIRED");
  assert.equal(projection.pendingCount, 1);
  assert.deepEqual(projection.byKind, { VALIDATOR_REVIEW: 1 });
  assert.equal(projection.notifications[0].summary, "Latest validator review for the active remediation cycle");
  assert.equal(projection.hiddenPendingCount, 3);
  assert.equal(projection.historyHidden, true);
});

test("active notification projection hides unread review residue once the WP is closed", () => {
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        summary: "PASS. No blocking findings remain in the committed reviewable state.",
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
    runtimeStatus: {
      runtime_status: "completed",
      current_packet_status: "Validated (PASS)",
      main_containment_status: "CONTAINED_IN_MAIN",
      current_main_compatibility_status: "COMPATIBLE",
      current_task_board_status: "DONE_VALIDATED",
      next_expected_actor: "NONE",
      next_expected_session: null,
      waiting_on: "CLOSED",
      waiting_on_session: null,
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      attention_required: false,
    },
  });
  const statusEvaluation = evaluateWpCommunicationHealth(input);
  const projection = deriveActiveWpNotificationProjection({
    statusEvaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
    pendingNotifications: [
      {
        source_kind: "AUTO_ROUTE",
        source_role: "INTEGRATION_VALIDATOR",
        target_role: "ORCHESTRATOR",
        correlation_id: "final-1",
        timestamp_utc: "2026-03-22T10:06:30Z",
        summary: "AUTO_ROUTE: direct review lane complete; orchestrator verdict progression ready",
      },
      {
        source_kind: "SESSION_COMPLETION",
        source_role: "CODER",
        target_role: "ORCHESTRATOR",
        correlation_id: "close-1",
        timestamp_utc: "2026-03-22T10:07:00Z",
        summary: "Governed session closed cleanly",
      },
    ],
  });

  assert.equal(statusEvaluation.state, "COMM_OK");
  assert.equal(projection.pendingCount, 0);
  assert.equal(projection.hiddenPendingCount, 2);
  assert.equal(projection.historyHidden, true);
});

test("active notification projection keeps overlap sidecar validator review visible while coder remains primary next actor", () => {
  const input = baseInput({
    packetContent: contractHeavyPacketFixture(),
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
        summary: "Intent drafted.",
        microtask_contract: {
          scope_ref: "BOOTSTRAP/CX-LANE-001",
          file_targets: ["src/demo.rs", "src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::smoke -- --exact"],
          phase_gate: "SKELETON",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap cleared.",
        timestamp_utc: "2026-03-22T10:03:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "micro-1",
        summary: "MT-001 review_mode=OVERLAP: review the completed slice while I continue.",
        packet_row_ref: "MT-001",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
    ],
    runtimeStatus: {
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
      waiting_on: "CODER_HANDOFF",
      waiting_on_session: "coder-1",
      validator_trigger: "MICROTASK_REVIEW_READY",
      validator_trigger_reason: "Previous completed microtask is awaiting overlap review while coder continues the current microtask",
      ready_for_validation: true,
      ready_for_validation_reason: "Overlap microtask review is open for the WP validator while coder remains in bounded forward execution",
      open_review_items: [
        {
          correlation_id: "micro-1",
          receipt_kind: "REVIEW_REQUEST",
          summary: "MT-001 review_mode=OVERLAP: review the completed slice while I continue.",
          opened_by_role: "CODER",
          opened_by_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          packet_row_ref: "MT-001",
          microtask_contract: null,
          requires_ack: true,
          opened_at: "2026-03-22T10:04:00Z",
          updated_at: "2026-03-22T10:04:00Z",
        },
      ],
    },
  });

  const statusEvaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation: statusEvaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });
  const projection = deriveActiveWpNotificationProjection({
    statusEvaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
    pendingNotifications: [
      {
        source_kind: "REVIEW_REQUEST",
        source_role: "CODER",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "micro-1",
        timestamp_utc: "2026-03-22T10:04:00Z",
        summary: "REVIEW_REQUEST: MT-001 review_mode=OVERLAP: review the completed slice while I continue.",
      },
    ],
    autoRoute: route,
  });

  assert.equal(statusEvaluation.state, "COMM_WAITING_FOR_HANDOFF");
  assert.equal(route.nextExpectedActor, "CODER");
  assert.equal(route.secondaryNotifications.length, 1);
  assert.equal(projection.pendingCount, 1);
  assert.equal(projection.notifications[0].target_role, "WP_VALIDATOR");
  assert.equal(projection.hiddenPendingCount, 0);
});

test("final review closes when the open request targeted an unassigned integration-validator placeholder", () => {
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        summary: "PASS. No blocking findings remain in the committed reviewable state.",
        timestamp_utc: "2026-03-22T10:04:00Z",
      },
      {
        receipt_kind: "REVIEW_REQUEST",
        actor_role: "CODER",
        actor_session: "coder-1",
        target_role: "INTEGRATION_VALIDATOR",
        target_session: "<unassigned>",
        correlation_id: "final-1",
        ack_for: null,
        summary: "Final authority review requested for the committed reviewable state.",
        timestamp_utc: "2026-03-22T10:05:00Z",
      },
      {
        receipt_kind: "REVIEW_RESPONSE",
        actor_role: "INTEGRATION_VALIDATOR",
        actor_session: "intval-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "final-1",
        ack_for: "final-1",
        summary: "No new blocking product-code findings were found in final authority review.",
        timestamp_utc: "2026-03-22T10:06:00Z",
      },
    ],
    runtimeStatus: {
      integration_validator_of_record: "intval-1",
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
      waiting_on: "FINAL_REVIEW_EXCHANGE",
      waiting_on_session: null,
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      attention_required: false,
    },
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.state, "COMM_OK");
  assert.equal(evaluation.correlations.finalReview, "final-1");
  assert.equal(route.nextExpectedActor, "ORCHESTRATOR");
  assert.equal(route.waitingOn, "VERDICT_PROGRESSION");
});

test("merge-pending closeout keeps the route on integration validator for main containment", () => {
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        summary: "PASS. Repair verified.",
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
        receipt_kind: "REVIEW_RESPONSE",
        actor_role: "INTEGRATION_VALIDATOR",
        actor_session: "intval-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "final-1",
        ack_for: "final-1",
        summary: "No new blocking product-code findings were found in final authority review.",
        timestamp_utc: "2026-03-22T10:06:00Z",
      },
    ],
    runtimeStatus: {
      integration_validator_of_record: "intval-1",
      runtime_status: "completed",
      current_phase: "STATUS_SYNC",
      current_packet_status: "Done",
      current_task_board_status: "DONE_MERGE_PENDING",
      main_containment_status: "MERGE_PENDING",
      next_expected_actor: "INTEGRATION_VALIDATOR",
      next_expected_session: null,
      waiting_on: "MAIN_CONTAINMENT",
      waiting_on_session: null,
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      attention_required: false,
    },
  });

  const evaluation = evaluateWpCommunicationHealth(input);
  const route = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
  });

  assert.equal(evaluation.state, "COMM_OK");
  assert.equal(route.nextExpectedActor, "INTEGRATION_VALIDATOR");
  assert.equal(route.nextExpectedSession, "intval-1");
  assert.equal(route.waitingOn, "MAIN_CONTAINMENT");
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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
        receipt_kind: "VALIDATOR_RESPONSE",
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        ack_for: "kickoff-1",
        summary: "Bootstrap and skeleton cleared; proceed.",
        timestamp_utc: "2026-03-22T10:02:30Z",
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

test("boundary check accepts runtime-projected validator session after watchdog steering receipts", () => {
  const input = baseInput({
    packetContent: contractHeavyPacketFixture(),
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
        summary: "Implementation order drafted.",
        timestamp_utc: "2026-03-22T10:02:00Z",
      },
      {
        receipt_kind: "STEERING",
        actor_role: "ORCHESTRATOR",
        actor_session: "ORCHESTRATOR_WATCHDOG",
        target_role: null,
        target_session: null,
        correlation_id: null,
        ack_for: null,
        summary: "RELAY_WATCHDOG | target=WP_VALIDATOR:wpv-1 | decision=STEER",
        timestamp_utc: "2026-03-22T10:02:30Z",
      },
    ],
    runtimeStatus: {
      wp_validator_of_record: "<unassigned>",
      active_role_sessions: [],
      next_expected_actor: "WP_VALIDATOR",
      next_expected_session: "wpv-1",
      waiting_on: "WP_VALIDATOR_INTENT_CHECKPOINT",
      waiting_on_session: "wpv-1",
      validator_trigger: "BLOCKED_NEEDS_VALIDATOR",
      validator_trigger_reason: "Coder intent recorded; WP validator must clear bootstrap/skeleton intent review before implementation or full handoff",
      ready_for_validation: true,
      ready_for_validation_reason: "Coder intent recorded; WP validator must clear bootstrap/skeleton intent review before implementation or full handoff",
      attention_required: false,
    },
  });

  const statusEvaluation = evaluateWpCommunicationHealth(input);
  const boundary = evaluateWpCommunicationBoundary({
    stage: "STATUS",
    statusEvaluation,
    runtimeStatus: input.runtimeStatus,
    latestReceipt: input.receipts.at(-1),
    pendingNotifications: [],
  });

  assert.equal(statusEvaluation.state, "COMM_WAITING_FOR_INTENT_CHECKPOINT");
  assert.equal(boundary.ok, true, JSON.stringify(boundary, null, 2));
  assert.equal(boundary.autoRoute?.nextExpectedSession, "wpv-1");
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

test("operator rule restatement invalidity requires a lane reset route", () => {
  const input = baseInput({
    receipts: [
      {
        receipt_kind: "WORKFLOW_INVALIDITY",
        workflow_invalidity_code: "OPERATOR_RULE_RESTATEMENT",
        actor_role: "ORCHESTRATOR",
        actor_session: "orch-1",
        target_role: "ORCHESTRATOR",
        target_session: null,
        correlation_id: null,
        ack_for: null,
        summary: "Operator had to restate the orchestrator-managed no-checkpoint rule mid-run",
        timestamp_utc: "2026-03-26T10:00:00Z",
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
  assert.match(evaluation.details.join("\n"), /lane_reset_required=YES/);
  assert.equal(route.nextExpectedActor, "ORCHESTRATOR");
  assert.equal(route.waitingOn, "LANE_RESET_REQUIRED");
  assert.equal(route.attentionRequired, true);
  if (route.notification) {
    assert.equal(route.notification.summary, "AUTO_ROUTE: operator rule restatement recorded; orchestrator lane reset required");
  }
});

// --- buildRoleInbox tests ---

test("buildRoleInbox returns PROCEED when no items target the role", () => {
  const inbox = buildRoleInbox("CODER", {
    open_review_items: [
      { receipt_kind: "REVIEW_REQUEST", target_role: "WP_VALIDATOR", opened_by_role: "CODER", summary: "MT-001 ready" },
    ],
  });
  assert.equal(inbox.next_action, "PROCEED");
  assert.equal(inbox.items.length, 0);
  assert.equal(inbox.blocked_reason, null);
});

test("buildRoleInbox detects steer items and returns REMEDIATE action", () => {
  const inbox = buildRoleInbox("CODER", {
    open_review_items: [
      { receipt_kind: "REVIEW_RESPONSE", target_role: "CODER", opened_by_role: "WP_VALIDATOR", summary: "MT-001 STEER: fix compile error" },
      { receipt_kind: "REVIEW_REQUEST", target_role: "WP_VALIDATOR", opened_by_role: "CODER", summary: "MT-002 ready" },
    ],
  });
  assert.equal(inbox.items.length, 1);
  assert.equal(inbox.next_action, "REMEDIATE_MT-001");
  assert.ok(inbox.blocked_reason.includes("remediation"));
  assert.equal(inbox.items[0].is_steer, true);
  assert.equal(inbox.items[0].mt, "MT-001");
});

test("buildRoleInbox detects review requests for WP_VALIDATOR", () => {
  const inbox = buildRoleInbox("WP_VALIDATOR", {
    open_review_items: [
      { receipt_kind: "REVIEW_REQUEST", target_role: "WP_VALIDATOR", opened_by_role: "CODER", summary: "MT-003 ready for review" },
    ],
  });
  assert.equal(inbox.items.length, 1);
  assert.equal(inbox.next_action, "REVIEW_MT-003");
  assert.equal(inbox.items[0].is_review_request, true);
});

test("buildRoleInbox prioritizes steers over review requests", () => {
  const inbox = buildRoleInbox("CODER", {
    open_review_items: [
      { receipt_kind: "REVIEW_REQUEST", target_role: "CODER", opened_by_role: "WP_VALIDATOR", summary: "MT-002 review notes" },
      { receipt_kind: "REVIEW_RESPONSE", target_role: "CODER", opened_by_role: "WP_VALIDATOR", summary: "MT-001 STEER: missing proof" },
    ],
  });
  assert.equal(inbox.items.length, 2);
  assert.equal(inbox.items[0].is_steer, true);
  assert.equal(inbox.items[0].mt, "MT-001");
  assert.equal(inbox.next_action, "REMEDIATE_MT-001");
});

test("buildRoleInbox returns empty for missing or null runtime status", () => {
  assert.equal(buildRoleInbox("CODER", null).next_action, "PROCEED");
  assert.equal(buildRoleInbox("CODER", {}).next_action, "PROCEED");
  assert.equal(buildRoleInbox("CODER", { open_review_items: [] }).next_action, "PROCEED");
});
