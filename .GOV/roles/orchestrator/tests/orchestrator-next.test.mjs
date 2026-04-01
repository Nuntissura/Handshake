import assert from "node:assert/strict";
import test from "node:test";

import {
  closeoutModeFromPacketStatus,
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
