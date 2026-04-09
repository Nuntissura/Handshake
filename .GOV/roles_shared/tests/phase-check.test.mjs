import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { buildPhaseCheckCommand, buildPhaseCheckPlan, parseCloseoutSyncOptions, runGateCheck } from "../checks/phase-check.mjs";

test("phase-check command builder keeps the role/session suffix compact", () => {
  assert.equal(
    buildPhaseCheckCommand({
      phase: "STARTUP",
      wpId: "WP-TEST-PHASE-v1",
      role: "coder",
      session: "<your-session>",
    }),
    "just phase-check STARTUP WP-TEST-PHASE-v1 CODER <your-session>",
  );
  assert.equal(
    buildPhaseCheckCommand({
      phase: "closeout",
      wpId: "WP-TEST-PHASE-v1",
      args: ["--sync-mode", "MERGE_PENDING", "--context", "record contained truth after governed review"],
    }),
    "just phase-check CLOSEOUT WP-TEST-PHASE-v1 --sync-mode MERGE_PENDING --context \"record contained truth after governed review\"",
  );
});

test("closeout sync options require explicit context and contained-main sha only when needed", () => {
  assert.deepEqual(
    parseCloseoutSyncOptions([
      "--sync-mode",
      "MERGE_PENDING",
      "--context",
      "recording merge-pending closeout truth after the governed final-lane preflight passed cleanly",
    ]),
    {
      modeSpec: {
        mode: "MERGE_PENDING",
        requireMergedMainCommit: false,
      },
      context: "recording merge-pending closeout truth after the governed final-lane preflight passed cleanly",
      mergedMainSha: "",
      debug: false,
    },
  );
  assert.deepEqual(
    parseCloseoutSyncOptions([
      "--sync-mode",
      "CONTAINED_IN_MAIN",
      "--merged-main-sha",
      "abc1234",
      "--context",
      "recording contained-main closure after the merge commit was verified against the signed scope",
      "--sync-debug",
    ]),
    {
      modeSpec: {
        mode: "CONTAINED_IN_MAIN",
        requireMergedMainCommit: true,
      },
      context: "recording contained-main closure after the merge commit was verified against the signed scope",
      mergedMainSha: "abc1234",
      debug: true,
    },
  );
  assert.throws(
    () => parseCloseoutSyncOptions(["--context", "this is long enough but missing a sync mode entirely"]),
    /require --sync-mode/,
  );
  assert.throws(
    () => parseCloseoutSyncOptions(["--sync-mode", "MERGE_PENDING", "--context", "too short"]),
    /at least 40 characters/,
  );
  assert.throws(
    () => parseCloseoutSyncOptions([
      "--sync-mode",
      "CONTAINED_IN_MAIN",
      "--context",
      "recording contained-main closure after the merge commit was verified against the signed scope",
    ]),
    /--merged-main-sha must be provided/,
  );
});

test("startup phase plan requires explicit role and preserves session routing", () => {
  const plan = buildPhaseCheckPlan({
    phase: "STARTUP",
    wpId: "WP-TEST-PHASE-v1",
    role: "CODER",
    session: "coder:test",
  });

  assert.deepEqual(plan.map((step) => step.label), [
    "ensure-wp-communications",
    "active-lane-brief",
    "wp-communication-health-check",
    "gate-check",
    "pre-work-check",
  ]);
  assert.deepEqual(plan[2]?.args, [
    "WP-TEST-PHASE-v1",
    "STARTUP",
    "CODER",
    "coder:test",
  ]);
});

test("handoff phase plan folds packet completeness into the composite boundary", () => {
  const plan = buildPhaseCheckPlan({
    phase: "HANDOFF",
    wpId: "WP-TEST-PHASE-v1",
    role: "WP_VALIDATOR",
  });

  assert.deepEqual(plan.map((step) => step.label), [
    "active-lane-brief",
    "validator-packet-complete",
    "validator-handoff-check",
    "wp-communication-health-check",
  ]);
});

test("handoff phase plan supports the coder-side post-work boundary through the same phase runner", () => {
  const plan = buildPhaseCheckPlan({
    phase: "HANDOFF",
    wpId: "WP-TEST-PHASE-v1",
    role: "CODER",
    args: ["--range", "abc123..def456"],
  });

  assert.deepEqual(plan.map((step) => step.label), [
    "gate-check",
    "post-work-check",
    "role-mailbox-export-check",
    "wp-communication-health-check",
  ]);
  assert.deepEqual(plan[1]?.args, [
    "WP-TEST-PHASE-v1",
    "--range",
    "abc123..def456",
  ]);
  assert.deepEqual(plan[3]?.args, [
    "WP-TEST-PHASE-v1",
    "KICKOFF",
  ]);
});

test("closeout phase plan includes verdict proof, context bundle, closeout preflight, and memory refresh", () => {
  const plan = buildPhaseCheckPlan({
    phase: "CLOSEOUT",
    wpId: "WP-TEST-PHASE-v1",
  });

  assert.deepEqual(plan.map((step) => step.label), [
    "active-lane-brief",
    "validator-packet-complete",
    "wp-communication-health-check",
    "integration-validator-context-brief",
    "integration-validator-closeout-check",
    "launch-memory-manager",
  ]);
});

test("gate-check resolves folder packets through packet.md", () => {
  const wpId = "WP-TEST-GATE-FOLDER-v1";
  const packetDir = path.join(".GOV", "task_packets", wpId);
  const packetPath = path.join(packetDir, "packet.md");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(packetPath, [
    `# Task Packet: ${wpId}`,
    "",
    "Status: In Progress",
    "",
    "## BOOTSTRAP",
    "Ready.",
    "",
    "## SKELETON",
    "Ready.",
  ].join("\n"), "utf8");

  try {
    const result = runGateCheck(wpId);
    assert.equal(result.ok, true, result.output);
    assert.match(result.output, /GATE PASS/i);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});
