import assert from "node:assert/strict";
import test from "node:test";

import {
  closeoutModeFromPacketStatus,
  isTerminalOrchestratorBoardStatus,
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
