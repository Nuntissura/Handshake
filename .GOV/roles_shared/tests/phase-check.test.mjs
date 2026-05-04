import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { REPO_ROOT } from "../scripts/lib/runtime-paths.mjs";
import {
  buildCloseoutNextCommands,
  buildPhaseCheckCommand,
  buildPhaseCheckPlan,
  parseCloseoutSyncOptions,
  parseCommittedTargetArgs,
  isCloseoutSyncKernelRoot,
  resolveCloseoutSyncCwd,
  resolveTerminalReadySessionsForWp,
  resolvePhaseCheckCwd,
  runGateCheck,
} from "../checks/phase-check.mjs";

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

test("closeout sync options rebuild multi-token context values after Windows wrapper splitting", () => {
  assert.deepEqual(
    parseCloseoutSyncOptions([
      "--sync-mode",
      "FAIL",
      "--context",
      "Final",
      "lane",
      "FAIL",
      "current",
      "main",
      "candidate",
      "does",
      "not",
      "compile",
      "and",
      "handoff",
      "proof",
      "is",
      "not",
      "reproducible",
    ]),
    {
      modeSpec: {
        mode: "FAIL",
        requireMergedMainCommit: false,
      },
      context: "Final lane FAIL current main candidate does not compile and handoff proof is not reproducible",
      mergedMainSha: "",
      debug: false,
    },
  );
});

test("phase-check closeout uses injected active repo root when present", () => {
  const original = process.env.HANDSHAKE_ACTIVE_REPO_ROOT;
  try {
    process.env.HANDSHAKE_ACTIVE_REPO_ROOT = "D:/tmp/handshake_main";
    assert.equal(resolvePhaseCheckCwd(), path.resolve("D:/tmp/handshake_main"));
    delete process.env.HANDSHAKE_ACTIVE_REPO_ROOT;
    assert.equal(resolvePhaseCheckCwd(), path.resolve(REPO_ROOT));
  } finally {
    if (original === undefined) {
      delete process.env.HANDSHAKE_ACTIVE_REPO_ROOT;
    } else {
      process.env.HANDSHAKE_ACTIVE_REPO_ROOT = original;
    }
  }
});

test("closeout sync runs from the integration-validator product worktree when phase-check starts in kernel", () => {
  const resolved = resolveCloseoutSyncCwd({
    wpId: "WP-TEST-PHASE-v1",
    phaseCheckCwd: REPO_ROOT,
    registrySessions: [
      {
        wp_id: "WP-TEST-PHASE-v1",
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-PHASE-v1",
        local_branch: "main",
        local_worktree_dir: "../handshake_main",
      },
    ],
  });

  assert.equal(resolved, path.resolve(REPO_ROOT, "../handshake_main"));
});

test("closeout sync falls back to the default integration-validator worktree without a registry lane", () => {
  const resolved = resolveCloseoutSyncCwd({
    wpId: "WP-TEST-PHASE-v1",
    phaseCheckCwd: REPO_ROOT,
    registrySessions: [],
  });

  assert.equal(resolved, path.resolve(REPO_ROOT, "../handshake_main"));
});

test("closeout sync kernel guard distinguishes product main from live kernel root", () => {
  assert.equal(isCloseoutSyncKernelRoot(REPO_ROOT), true);
  assert.equal(isCloseoutSyncKernelRoot(path.resolve(REPO_ROOT, "../handshake_main")), false);
});

test("committed target args preserve validator handoff range selection", () => {
  assert.deepEqual(
    parseCommittedTargetArgs(["--range", "abc123..def456"]),
    { rev: "", range: "abc123..def456" },
  );
  assert.deepEqual(
    parseCommittedTargetArgs(["--rev", "deadbee"]),
    { rev: "deadbee", range: "" },
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

test("startup phase plan skips pre-work-check for committed handoff preflight mode", () => {
  const plan = buildPhaseCheckPlan({
    phase: "STARTUP",
    wpId: "WP-TEST-PHASE-v1",
    role: "CODER",
    args: ["--committed-handoff-preflight"],
  });

  assert.deepEqual(plan.map((step) => step.label), [
    "ensure-wp-communications",
    "active-lane-brief",
    "wp-communication-health-check",
    "gate-check",
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

test("handoff phase plan forwards committed target args to the validator boundary", () => {
  const plan = buildPhaseCheckPlan({
    phase: "HANDOFF",
    wpId: "WP-TEST-PHASE-v1",
    role: "WP_VALIDATOR",
    args: ["--range", "abc123..def456"],
  });

  assert.deepEqual(plan[2]?.args, [
    "WP-TEST-PHASE-v1",
    "--range",
    "abc123..def456",
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
    "artifact-root-preflight-check",
    "active-lane-brief",
    "validator-packet-complete",
    "wp-communication-health-check",
    "integration-validator-context-brief",
    "integration-validator-closeout-check",
    "launch-memory-manager",
    "intelligent-review-cadence-check",
  ]);
});

test("closeout phase plan forwards governed sync args into the closeout preflight", () => {
  const plan = buildPhaseCheckPlan({
    phase: "CLOSEOUT",
    wpId: "WP-TEST-PHASE-v1",
    args: [
      "--sync-mode",
      "MERGE_PENDING",
      "--context",
      "recording merge-pending truth after governed final-lane review completed cleanly",
    ],
  });

  assert.deepEqual(plan[5]?.args, [
    "WP-TEST-PHASE-v1",
    "--sync-mode",
    "MERGE_PENDING",
    "--context",
    "recording merge-pending truth after governed final-lane review completed cleanly",
  ]);
});

test("closeout next commands prefer canonical runtime publication when packet merge truth is stale", () => {
  const wpId = "WP-TEST-PHASE-CLOSEOUT-v1";
  const packetDir = path.join(".GOV", "task_packets", wpId);
  const packetPath = path.join(packetDir, "packet.md");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(packetPath, [
    `# Task Packet: ${wpId}`,
    "",
    "Status: Done",
    "",
    "## METADATA",
    "- MAIN_CONTAINMENT_STATUS: NOT_STARTED",
    "- MERGED_MAIN_COMMIT: NONE",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: NONE",
    "",
    "## VALIDATION_REPORTS",
    "#### Verdict: PENDING",
  ].join("\n"), "utf8");

  try {
    const nextCommands = buildCloseoutNextCommands({
      wpId,
      ok: true,
      runtimeStatusOverride: {
        current_packet_status: "Done",
        current_task_board_status: "IN_PROGRESS",
        main_containment_status: "NOT_STARTED",
        execution_state: {
          schema_version: "wp_execution_state@1",
          authority: {
            packet_status: "Validated (PASS)",
            task_board_status: "DONE_VALIDATED",
            main_containment_status: "MERGE_PENDING",
          },
        },
      },
    });

    assert(nextCommands.some((line) =>
      /just phase-check CLOSEOUT WP-TEST-PHASE-CLOSEOUT-v1 --sync-mode CONTAINED_IN_MAIN --merged-main-sha <MERGED_MAIN_SHA>/.test(line)
    ));
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("closeout next commands surface the typed governed closeout sync when merge-pending truth is already recorded", () => {
  const wpId = "WP-TEST-PHASE-CLOSEOUT-GOV-v1";
  const packetDir = path.join(".GOV", "task_packets", wpId);
  const packetPath = path.join(packetDir, "packet.md");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(packetPath, [
    `# Task Packet: ${wpId}`,
    "",
    "Status: Done",
    "",
    "## METADATA",
    "- MAIN_CONTAINMENT_STATUS: MERGE_PENDING",
    "- MERGED_MAIN_COMMIT: NONE",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: NONE",
    "",
    "## VALIDATION_REPORTS",
    "#### Verdict: PASS",
  ].join("\n"), "utf8");

  try {
    const nextCommands = buildCloseoutNextCommands({
      wpId,
      ok: true,
      integrationValidatorCloseoutResult: {
        closeoutSyncGovernance: {
          latestEvent: {
            mode: "MERGE_PENDING",
            timestamp_utc: "2026-04-20T10:15:00Z",
          },
          latestGovernedAction: {
            rule_id: "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE",
            resume_disposition: "CONSUME_RESULT",
            updated_at: "2026-04-20T10:15:00Z",
          },
        },
      },
      runtimeStatusOverride: {
        current_packet_status: "Validated (PASS)",
        current_task_board_status: "DONE_VALIDATED",
        main_containment_status: "MERGE_PENDING",
        execution_state: {
          schema_version: "wp_execution_state@1",
          authority: {
            packet_status: "Validated (PASS)",
            task_board_status: "DONE_VALIDATED",
            main_containment_status: "MERGE_PENDING",
          },
        },
      },
    });

    assert(nextCommands.some((line) =>
      /Merge-pending closeout sync is already recorded via governed action INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE @ 2026-04-20T10:15:00Z/.test(line)
    ));
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("closeout next commands classify workflow dossier failure as diagnostic debt", () => {
  const nextCommands = buildCloseoutNextCommands({
    wpId: "WP-TEST-DOSSIER-DEBT-v1",
    ok: true,
    workflowDossierCloseoutOk: false,
  });

  assert(nextCommands.some((line) => /diagnostic debt/i.test(line)));
  assert(nextCommands.some((line) => /do not block product closeout/i.test(line)));
});

test("terminal READY session cleanup targets only governed READY sessions for the active WP", () => {
  const sessions = resolveTerminalReadySessionsForWp({
    wpId: "WP-TEST-PHASE-v1",
    registrySessions: [
      { wp_id: "WP-TEST-PHASE-v1", role: "CODER", runtime_state: "READY" },
      { wp_id: "WP-TEST-PHASE-v1", role: "WP_VALIDATOR", runtime_state: "CLOSED" },
      { wp_id: "WP-TEST-PHASE-v1", role: "ACTIVATION_MANAGER", runtime_state: "READY" },
      { wp_id: "WP-OTHER-v1", role: "INTEGRATION_VALIDATOR", runtime_state: "READY" },
    ],
  });

  assert.deepEqual(
    sessions.map((session) => `${session.role}:${session.runtime_state}`),
    ["CODER:READY", "ACTIVATION_MANAGER:READY"],
  );
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
