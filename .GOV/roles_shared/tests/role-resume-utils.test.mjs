import test from "node:test";
import assert from "node:assert/strict";

import { comparePrepareAgainstPacketTruth } from "../scripts/lib/role-resume-utils.mjs";

const REPO_ROOT = "D:/Projects/LLM projects/Handshake/Handshake Worktrees/handshake_main";

test("comparePrepareAgainstPacketTruth accepts matching packet and PREPARE authority", () => {
  const packet = [
    "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
    "- EXECUTION_OWNER: CODER_A",
    "- LOCAL_BRANCH: feat/WP-1-Example-v1",
    "- LOCAL_WORKTREE_DIR: ../wtc-example-v1",
  ].join("\n");
  const prepare = {
    workflow_lane: "ORCHESTRATOR_MANAGED",
    execution_lane: "CODER_A",
    branch: "feat/WP-1-Example-v1",
    worktree_dir: "../wtc-example-v1",
  };

  const result = comparePrepareAgainstPacketTruth(packet, prepare, REPO_ROOT);

  assert.equal(result.ok, true);
  assert.deepEqual(result.issues, []);
});

test("comparePrepareAgainstPacketTruth flags packet/PREPARE authority drift", () => {
  const packet = [
    "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
    "- EXECUTION_OWNER: CODER_A",
    "- LOCAL_BRANCH: feat/WP-1-Example-v1",
    "- LOCAL_WORKTREE_DIR: ../wtc-example-v1",
  ].join("\n");
  const prepare = {
    workflow_lane: "MANUAL_RELAY",
    execution_lane: "CODER_B",
    branch: "feat/WP-1-Other-v1",
    worktree_dir: "../wtc-other-v1",
  };

  const result = comparePrepareAgainstPacketTruth(packet, prepare, REPO_ROOT);

  assert.equal(result.ok, false);
  assert.deepEqual(result.issues, [
    "Official packet WORKFLOW_LANE conflicts with PREPARE: expected ORCHESTRATOR_MANAGED, got MANUAL_RELAY",
    "Official packet EXECUTION_OWNER conflicts with PREPARE: expected CODER_A, got CODER_B",
    "Official packet LOCAL_BRANCH conflicts with PREPARE: expected feat/WP-1-Example-v1, got feat/WP-1-Other-v1",
    "Official packet LOCAL_WORKTREE_DIR conflicts with PREPARE: expected ../wtc-example-v1, got ../wtc-other-v1",
  ]);
});
