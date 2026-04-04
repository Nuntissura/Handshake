import assert from "node:assert/strict";
import test from "node:test";

import {
  closeoutModeFromPacketStatus,
  findActiveTokenBudgetContinuationWaiver,
  isTerminalOrchestratorBoardStatus,
  latestOrchestratorGovernanceCheckpoint,
} from "../scripts/orchestrator-next.mjs";

test("orchestrator-next maps packet closeout states to the correct task-board closeout modes", () => {
  assert.equal(closeoutModeFromPacketStatus("Done"), "DONE_MERGE_PENDING");
  assert.equal(closeoutModeFromPacketStatus("Validated (PASS)"), "DONE_VALIDATED");
  assert.equal(closeoutModeFromPacketStatus("Validated (FAIL)"), "DONE_FAIL");
  assert.equal(closeoutModeFromPacketStatus("Validated (OUTDATED_ONLY)"), "DONE_OUTDATED_ONLY");
  assert.equal(closeoutModeFromPacketStatus("Validated (ABANDONED)"), "DONE_ABANDONED");
  assert.equal(closeoutModeFromPacketStatus("In Progress"), "");
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
