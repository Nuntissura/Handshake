import assert from "node:assert/strict";
import test from "node:test";

import {
  closeoutModeFromPacketStatus,
  closeoutSyncCommandForProjection,
  findActiveTokenBudgetContinuationWaiver,
  isTerminalOrchestratorBoardStatus,
  latestOrchestratorAcpHealthAlert,
  latestOrchestratorGovernanceCheckpoint,
  latestOrchestratorRelayWatchdogRepair,
  queuedGovernedWaitState,
  relayEscalationPolicyFindings,
  tokenPolicyContinuationDecision,
} from "../scripts/orchestrator-next.mjs";

test("orchestrator-next maps packet closeout states to the correct task-board closeout modes", () => {
  assert.equal(closeoutModeFromPacketStatus("Done"), "DONE_MERGE_PENDING");
  assert.equal(closeoutModeFromPacketStatus("Validated (PASS)"), "DONE_VALIDATED");
  assert.equal(closeoutModeFromPacketStatus("Validated (FAIL)"), "DONE_FAIL");
  assert.equal(closeoutModeFromPacketStatus("Validated (OUTDATED_ONLY)"), "DONE_OUTDATED_ONLY");
  assert.equal(closeoutModeFromPacketStatus("Validated (ABANDONED)"), "DONE_ABANDONED");
  assert.equal(closeoutModeFromPacketStatus("In Progress"), "");
});

test("orchestrator-next suppresses duplicate task-board closeout publication when board history is already current", () => {
  const command = closeoutSyncCommandForProjection(
    "WP-TEST",
    {
      current_packet_status: "Validated (PASS)",
      current_task_board_status: "DONE_VALIDATED",
    },
    {
      current_packet_status: "Validated (PASS)",
      current_task_board_status: "DONE_VALIDATED",
      runtime_status: "completed",
      current_phase: "STATUS_SYNC",
      main_containment_status: "CONTAINED_IN_MAIN",
    },
    null,
    "VALIDATED",
  );

  assert.equal(command, "");
});

test("orchestrator-next still requests task-board publication when closeout truth is ready but board history lags", () => {
  const command = closeoutSyncCommandForProjection(
    "WP-TEST",
    {
      current_packet_status: "Validated (PASS)",
      current_task_board_status: "DONE_VALIDATED",
    },
    {
      current_packet_status: "Validated (PASS)",
      current_task_board_status: "DONE_VALIDATED",
      runtime_status: "completed",
      current_phase: "STATUS_SYNC",
      main_containment_status: "CONTAINED_IN_MAIN",
    },
    null,
    "IN_PROGRESS",
  );

  assert.equal(command, "just task-board-set WP-TEST DONE_VALIDATED");
});

test("orchestrator-next prefers canonical closeout repair over duplicate board publication when packet truth drifts", () => {
  const command = closeoutSyncCommandForProjection(
    "WP-TEST",
    {
      current_packet_status: "Done",
      current_task_board_status: "DONE_MERGE_PENDING",
    },
    {
      current_packet_status: "Done",
      current_task_board_status: "DONE_MERGE_PENDING",
      main_containment_status: "MERGE_PENDING",
      execution_state: {
        authority: {
          packet_status: "Validated (PASS)",
          task_board_status: "DONE_VALIDATED",
          runtime_status: "completed",
          phase: "STATUS_SYNC",
          main_containment_status: "CONTAINED_IN_MAIN",
          route_anchor: {},
          review_anchor: {},
        },
      },
    },
    null,
    "VALIDATED",
  );

  assert.match(command, /^just phase-check CLOSEOUT WP-TEST --sync-mode CONTAINED_IN_MAIN /);
});

test("orchestrator-next treats abandoned task-board state as terminal orchestrator history", () => {
  assert.equal(isTerminalOrchestratorBoardStatus("ABANDONED"), true);
  assert.equal(isTerminalOrchestratorBoardStatus("VALIDATED"), true);
  assert.equal(isTerminalOrchestratorBoardStatus("SUPERSEDED"), true);
  assert.equal(isTerminalOrchestratorBoardStatus("MERGE_PENDING"), false);
  assert.equal(isTerminalOrchestratorBoardStatus("IN_PROGRESS"), false);
});

test("orchestrator-next picks the latest governance checkpoint notification for orchestrator review", () => {
  const notification = latestOrchestratorGovernanceCheckpoint({
    ORCHESTRATOR: {
      notifications: [
        {
          source_kind: "GOVERNANCE_CHECKPOINT",
          timestamp_utc: "2026-04-01T10:00:00Z",
          summary: "older checkpoint",
        },
        {
          source_kind: "AUTO_ROUTE",
          timestamp_utc: "2026-04-01T10:01:00Z",
          summary: "not a checkpoint",
        },
        {
          source_kind: "GOVERNANCE_CHECKPOINT",
          timestamp_utc: "2026-04-01T10:02:00Z",
          summary: "latest checkpoint",
        },
      ],
    },
  });

  assert.equal(notification?.summary, "latest checkpoint");
});

test("orchestrator-next picks the latest ACP health alert notification for orchestrator repair", () => {
  const notification = latestOrchestratorAcpHealthAlert({
    ORCHESTRATOR: {
      notifications: [
        {
          source_kind: "ACP_HEALTH_ALERT",
          timestamp_utc: "2026-04-18T10:00:00Z",
          summary: "older health alert",
        },
        {
          source_kind: "GOVERNANCE_CHECKPOINT",
          timestamp_utc: "2026-04-18T10:01:00Z",
          summary: "not a health alert",
        },
        {
          source_kind: "ACP_HEALTH_ALERT",
          timestamp_utc: "2026-04-18T10:02:00Z",
          summary: "latest health alert",
        },
      ],
    },
  });

  assert.equal(notification?.summary, "latest health alert");
});

test("orchestrator-next picks the latest relay watchdog repair notification for retry suppression", () => {
  const notification = latestOrchestratorRelayWatchdogRepair({
    ORCHESTRATOR: {
      notifications: [
        {
          source_kind: "RELAY_WATCHDOG_REPAIR",
          timestamp_utc: "2026-04-18T10:00:00Z",
          summary: "older repair",
        },
        {
          source_kind: "ACP_HEALTH_ALERT",
          timestamp_utc: "2026-04-18T10:01:00Z",
          summary: "not a relay repair",
        },
        {
          source_kind: "RELAY_WATCHDOG_REPAIR",
          timestamp_utc: "2026-04-18T10:02:00Z",
          summary: "latest repair",
        },
      ],
    },
  });

  assert.equal(notification?.summary, "latest repair");
});

test("orchestrator-next detects an active governance waiver for token-budget continuation", () => {
  const waiver = findActiveTokenBudgetContinuationWaiver(`
## WAIVERS GRANTED
- WAIVER_ID: CX-TEST-TOKEN-001 | STATUS: ACTIVE | COVERS: GOVERNANCE | SCOPE: post-signature orchestrator-managed continuation after TOKEN_BUDGET_EXCEEDED on WP-TEST | JUSTIFICATION: Operator authorized bounded continuation after POLICY_CONFLICT during crash recovery. | APPROVER: Operator | EXPIRES: until closeout
`);

  assert.equal(waiver?.waiverId, "CX-TEST-TOKEN-001");
});

test("orchestrator-next ignores unrelated governance waivers for token-budget continuation", () => {
  const waiver = findActiveTokenBudgetContinuationWaiver(`
## WAIVERS GRANTED
- WAIVER_ID: CX-TEST-GOV-001 | STATUS: ACTIVE | COVERS: GOVERNANCE | SCOPE: workflow bookkeeping note | JUSTIFICATION: operator approved a governance docs cleanup | APPROVER: Operator | EXPIRES: until cleanup
`);

  assert.equal(waiver, null);
});

test("orchestrator-next continuation waiver suppresses token policy hard stops but keeps findings", () => {
  const decision = tokenPolicyContinuationDecision({
    workflowLane: "ORCHESTRATOR_MANAGED",
    boardStatus: "IN_PROGRESS",
    ledgerHealthSeverity: "FAIL",
    tokenBudgetStatus: "FAIL",
    waiver: { waiverId: "CX-TEST-TOKEN-001" },
  });

  assert.equal(decision.continuationActive, true);
  assert.equal(decision.blockLedgerHealth, false);
  assert.equal(decision.blockBudget, false);
  assert.match(decision.findings.join(" | "), /CX-TEST-TOKEN-001/);
  assert.match(decision.findings.join(" | "), /token-ledger policy remains FAIL/i);
  assert.match(decision.findings.join(" | "), /token-budget policy remains FAIL/i);
});

test("orchestrator-next blocks token policy conflicts without a continuation waiver", () => {
  const decision = tokenPolicyContinuationDecision({
    workflowLane: "ORCHESTRATOR_MANAGED",
    boardStatus: "IN_PROGRESS",
    ledgerHealthSeverity: "FAIL",
    tokenBudgetStatus: "FAIL",
    waiver: null,
  });

  assert.equal(decision.continuationActive, false);
  assert.equal(decision.blockLedgerHealth, true);
  assert.equal(decision.blockBudget, true);
  assert.deepEqual(decision.findings, []);
});

test("orchestrator-next classifies queue-backed governed wait state for the projected actor", () => {
  const queued = queuedGovernedWaitState({
    workflowLane: "ORCHESTRATOR_MANAGED",
    runtimeStatus: {
      next_expected_actor: "CODER",
      next_expected_session: "CODER-WP-1-Test-v1",
    },
    registrySessions: [
      {
        role: "CODER",
        session_thread_id: "thread-123",
        updated_at: "2026-04-20T12:00:00Z",
        pending_control_queue_count: 2,
        next_queued_control_request: {
          command_kind: "SEND_PROMPT",
          queued_at: "2026-04-20T11:59:00Z",
          blocking_command_id: "command-123",
          summary: "Resume queued coder follow-up",
        },
      },
    ],
  });

  assert.equal(queued?.role, "CODER");
  assert.equal(queued?.target, "CODER:CODER-WP-1-Test-v1");
  assert.equal(queued?.queueCount, 2);
  assert.equal(queued?.queuedRequest?.command_kind, "SEND_PROMPT");
});

test("orchestrator-next ignores queued work when the projected next actor is not a governed relay target", () => {
  const queued = queuedGovernedWaitState({
    workflowLane: "ORCHESTRATOR_MANAGED",
    runtimeStatus: {
      next_expected_actor: "ORCHESTRATOR",
    },
    registrySessions: [
      {
        role: "CODER",
        pending_control_queue_count: 1,
        next_queued_control_request: {
          command_kind: "SEND_PROMPT",
        },
      },
    ],
  });

  assert.equal(queued, null);
});

test("orchestrator-next formats relay escalation policy findings from runtime truth", () => {
  const findings = relayEscalationPolicyFindings({
    failure_class: "DUPLICATE_REWAKE_LOOP",
    policy_state: "AUTO_RETRY_BLOCKED",
    next_strategy: "ALTERNATE_METHOD",
    budget_scope: "SAME_FAILURE_REWAKE",
    budget_used: 2,
    budget_limit: 2,
    reason_code: "SAME_FAILURE_REWAKE_BUDGET_EXHAUSTED",
    summary: "duplicate re-wake loop exhausted the same-failure budget",
  });

  assert.match(findings.join(" | "), /DUPLICATE_REWAKE_LOOP/);
  assert.match(findings.join(" | "), /AUTO_RETRY_BLOCKED -> ALTERNATE_METHOD/);
  assert.match(findings.join(" | "), /SAME_FAILURE_REWAKE:2\/2/);
  assert.match(findings.join(" | "), /SAME_FAILURE_REWAKE_BUDGET_EXHAUSTED/);
});
