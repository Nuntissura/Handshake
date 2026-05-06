import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import {
  activationReadinessRequiresActivationManager,
  activationReadinessArtifactPath,
  buildActivationManagerLaunchCommands,
  buildDownstreamGovernedLaunchCommands,
  buildManualRelayCommands,
  readActivationReadinessState,
} from "../scripts/lib/workflow-lane-guidance-lib.mjs";

test("readActivationReadinessState parses the activation readiness artifact", () => {
  const wpId = "WP-TEST-ACTIVATION-READINESS-v1";
  const artifactPath = activationReadinessArtifactPath(wpId);
  fs.mkdirSync(path.dirname(artifactPath), { recursive: true });
  fs.writeFileSync(artifactPath, [
    "ACTIVATION_READINESS",
    `- WP_ID: ${wpId}`,
    "- GENERATED_AT_UTC: 2026-05-05T22:40:00.000Z",
    "- STATE_SOURCE: RECOMPUTED",
    "- VERDICT: READY_FOR_ORCHESTRATOR_REVIEW",
    "- READY_FOR_DOWNSTREAM_LAUNCH: YES",
    "- PACKET_STATUS: Ready for Dev",
    "- OUTSTANDING_ISSUES: NONE",
    "- NEXT_ORCHESTRATOR_ACTION: Launch downstream governed lanes.",
    "",
  ].join("\n"), "utf8");

  try {
    const readiness = readActivationReadinessState(wpId);
    assert.equal(readiness.exists, true);
    assert.equal(readiness.verdict, "READY_FOR_ORCHESTRATOR_REVIEW");
    assert.equal(readiness.generatedAtUtc, "2026-05-05T22:40:00.000Z");
    assert.equal(readiness.stateSource, "RECOMPUTED");
    assert.equal(readiness.readyField, "YES");
    assert.equal(readiness.packetStatus, "Ready for Dev");
    assert.equal(readiness.outstandingIssues, "NONE");
    assert.equal(readiness.readyForDownstreamLaunch, true);
    assert.match(readiness.path, /roles\/activation_manager\/runtime\/activation_readiness\//);
    assert.match(readiness.nextOrchestratorAction, /Launch downstream governed lanes/i);
  } finally {
    fs.rmSync(artifactPath, { force: true });
  }
});

test("workflow lane guidance keeps orchestrator-managed launch on Activation Manager first", () => {
  const wpId = "WP-TEST-GUIDANCE-v1";
  const activationCommands = buildActivationManagerLaunchCommands(wpId, {
    exists: true,
    verdict: "REPAIR_REQUIRED",
    nextOrchestratorAction: "Repair activation bundle.",
    generatedAtUtc: "2026-05-05T22:40:00.000Z",
    path: "tmp/readiness.md",
  });
  const manualCommands = buildManualRelayCommands(wpId);
  const downstreamCommands = buildDownstreamGovernedLaunchCommands(wpId);

  assert.deepEqual(activationCommands.slice(0, 2), [
    `just activation-manager readiness ${wpId} --write`,
    `just activation-manager next ${wpId}`,
  ]);
  assert.match(activationCommands.join("\n"), /Current ACTIVATION_READINESS: REPAIR_REQUIRED/i);
  assert.match(activationCommands.join("\n"), /Readiness generated_at: 2026-05-05T22:40:00.000Z/i);
  assert.match(activationCommands.join("\n"), /still not READY_FOR_ORCHESTRATOR_REVIEW: just launch-activation-manager-session/i);
  assert.match(activationCommands.join("\n"), /cheap recovery gate/i);
  assert.deepEqual(manualCommands.slice(0, 2), [
    `just manual-relay-next ${wpId}`,
    `just session-registry-status ${wpId}`,
  ]);
  assert.match(manualCommands.join("\n"), /CLASSIC_ORCHESTRATOR/i);
  assert.match(manualCommands.join("\n"), /keeping the Operator in the relay loop/i);
  assert.deepEqual(downstreamCommands.slice(0, 3), [
    `just launch-coder-session ${wpId}`,
    `just launch-wp-validator-session ${wpId}`,
    `just session-registry-status ${wpId}`,
  ]);
});

test("missing activation readiness keeps downstream launch blocked on Activation Manager", () => {
  const wpId = "WP-TEST-MISSING-ACTIVATION-READINESS-v1";
  const gate = activationReadinessRequiresActivationManager(wpId);
  assert.equal(gate.readiness.exists, false);
  assert.equal(gate.readiness.verdict, "<missing>");
  assert.equal(gate.requiresActivationManager, true);
});
