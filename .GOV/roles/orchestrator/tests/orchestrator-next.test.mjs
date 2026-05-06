import assert from "node:assert/strict";
import test from "node:test";

import {
  closeoutModeFromPacketStatus,
  closeoutSyncCommandForProjection,
  buildDownstreamGovernedContinuationCommands,
  downstreamGovernedSessionState,
  findActiveTokenBudgetContinuationWaiver,
  isTerminalOrchestratorBoardStatus,
  latestOrchestratorAcpHealthAlert,
  latestOrchestratorDowntimeRedAlert,
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

test("orchestrator-next routes direct-review completion to final coder handoff before closeout sync", () => {
  const command = closeoutSyncCommandForProjection(
    "WP-TEST",
    {
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      current_main_compatibility_status: "NOT_RUN",
    },
    {
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      runtime_status: "working",
      current_phase: "VALIDATION",
      next_expected_actor: "ORCHESTRATOR",
      waiting_on: "VERDICT_PROGRESSION",
      committed_handoff_head_sha: "d7f3f760945c21076d75188fb2c90f1eafb155c3",
    },
    {
      ok: true,
      state: "COMM_OK",
      counts: {
        coderHandoffs: 0,
      },
    },
    "IN_PROGRESS",
  );

  assert.match(command, /^just session-send CODER WP-TEST /);
  assert.match(command, /final CODER_HANDOFF/);
});

test("orchestrator-next routes resolved final Integration Validator review to closeout sync", () => {
  const command = closeoutSyncCommandForProjection(
    "WP-TEST",
    {
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      current_main_compatibility_status: "NOT_RUN",
    },
    {
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      runtime_status: "working",
      current_phase: "VALIDATION",
      next_expected_actor: "ORCHESTRATOR",
      waiting_on: "VERDICT_PROGRESSION",
      committed_handoff_head_sha: "d7f3f760945c21076d75188fb2c90f1eafb155c3",
    },
    {
      ok: true,
      state: "COMM_OK",
      counts: {
        coderHandoffs: 0,
        integrationFinalOpenReceipts: 1,
        integrationFinalResolutionReceipts: 1,
      },
    },
    "IN_PROGRESS",
  );

  assert.equal(
    command,
    'just phase-check CLOSEOUT WP-TEST --sync-mode MERGE_PENDING --context "<why this closeout truth is being recorded, >=40 chars>"',
  );
});

test("orchestrator-next inserts committed handoff validation before Integration Validator relay", () => {
  const commands = buildDownstreamGovernedContinuationCommands({
    wpId: "WP-TEST-MISSING-COMMITTED-EVIDENCE-v1",
    runtimeStatus: {
      next_expected_actor: "INTEGRATION_VALIDATOR",
      next_expected_session: "integration-validator:test",
      waiting_on: "OPEN_REVIEW_ITEM_CODER_HANDOFF",
      committed_handoff_base_sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      committed_handoff_head_sha: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    },
    registrySessions: [
      {
        role: "CODER",
        wp_id: "WP-TEST-MISSING-COMMITTED-EVIDENCE-v1",
        runtime_state: "READY",
      },
      {
        role: "WP_VALIDATOR",
        wp_id: "WP-TEST-MISSING-COMMITTED-EVIDENCE-v1",
        runtime_state: "READY",
      },
    ],
  });

  assert.equal(
    commands[0],
    "just phase-check HANDOFF WP-TEST-MISSING-COMMITTED-EVIDENCE-v1 WP_VALIDATOR --range aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa..bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  );
  assert.match(commands[1], /^just active-lane-brief INTEGRATION_VALIDATOR /);
});

test("orchestrator-next treats abandoned task-board state as terminal orchestrator history", () => {
  assert.equal(isTerminalOrchestratorBoardStatus("ABANDONED"), true);
  assert.equal(isTerminalOrchestratorBoardStatus("VALIDATED"), true);
  assert.equal(isTerminalOrchestratorBoardStatus("DONE_MERGE_PENDING"), true);
  assert.equal(isTerminalOrchestratorBoardStatus("SUPERSEDED"), true);
  assert.equal(isTerminalOrchestratorBoardStatus("MERGE_PENDING"), true);
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

test("orchestrator-next picks the latest orchestrator downtime red alert", () => {
  const notification = latestOrchestratorDowntimeRedAlert({
    ORCHESTRATOR: {
      notifications: [
        {
          source_kind: "RED_ALERT_ORCHESTRATOR_DOWNTIME",
          timestamp_utc: "2026-04-26T10:00:00Z",
          summary: "older downtime alert",
        },
        {
          source_kind: "ACP_HEALTH_ALERT",
          timestamp_utc: "2026-04-26T10:01:00Z",
          summary: "not downtime",
        },
        {
          source_kind: "RED_ALERT_ORCHESTRATOR_DOWNTIME",
          timestamp_utc: "2026-04-26T10:02:00Z",
          summary: "latest downtime alert",
        },
      ],
    },
  });

  assert.equal(notification?.summary, "latest downtime alert");
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

test("orchestrator-next keeps token policy findings diagnostic even when a legacy waiver is present", () => {
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
  assert.match(decision.findings.join(" | "), /must not stop orchestrator-managed continuation/i);
  assert.match(decision.findings.join(" | "), /diagnostic only/i);
  assert.match(decision.findings.join(" | "), /no longer requires a waiver/i);
});

test("orchestrator-next does not block token policy conflicts without a continuation waiver", () => {
  const decision = tokenPolicyContinuationDecision({
    workflowLane: "ORCHESTRATOR_MANAGED",
    boardStatus: "IN_PROGRESS",
    ledgerHealthSeverity: "FAIL",
    tokenBudgetStatus: "FAIL",
    waiver: null,
  });

  assert.equal(decision.continuationActive, false);
  assert.equal(decision.blockLedgerHealth, false);
  assert.equal(decision.blockBudget, false);
  assert.match(decision.findings.join(" | "), /governance telemetry only/i);
  assert.match(decision.findings.join(" | "), /recorded mechanically/i);
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

test("orchestrator-next routes to existing downstream governed sessions instead of relaunching them", () => {
  const wpId = "WP-TEST-ORCH-MANAGED-v1";
  const registrySessions = [
    { role: "CODER", wp_id: wpId, runtime_state: "READY" },
    { role: "WP_VALIDATOR", wp_id: wpId, runtime_state: "READY" },
  ];

  const state = downstreamGovernedSessionState({ registrySessions, wpId });
  assert.equal(state.hasBothInitialRoles, true);

  const commands = buildDownstreamGovernedContinuationCommands({
    wpId,
    runtimeStatus: {
      next_expected_actor: "WP_VALIDATOR",
    },
    registrySessions,
  });

  assert(commands.some((line) => /^just session-send WP_VALIDATOR /.test(line)));
  assert(commands.every((line) => !/launch-coder-session|launch-wp-validator-session/.test(line)));
});

test("orchestrator-next launches only missing downstream governed sessions", () => {
  const wpId = "WP-TEST-ORCH-MANAGED-v1";
  const commands = buildDownstreamGovernedContinuationCommands({
    wpId,
    runtimeStatus: {
      next_expected_actor: "WP_VALIDATOR",
    },
    registrySessions: [
      { role: "CODER", wp_id: wpId, runtime_state: "READY" },
    ],
  });

  assert.deepEqual(commands, [
    `just launch-wp-validator-session ${wpId}`,
    `just session-registry-status ${wpId}`,
  ]);
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
