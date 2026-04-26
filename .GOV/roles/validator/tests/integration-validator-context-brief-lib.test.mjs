import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { buildIntegrationValidatorContextBrief } from "../scripts/lib/integration-validator-context-brief-lib.mjs";

function repomemCoverageFixture({
  state = "PASS",
  activeRoles = ["INTEGRATION_VALIDATOR"],
  debtRoles = [],
  debtKeys = [],
} = {}) {
  return {
    state,
    active_roles: activeRoles,
    debt_roles: debtRoles,
    debt_keys: debtKeys,
    role_details: activeRoles.map((role) => ({
      role,
      status: debtRoles.includes(role) ? "DEBT" : "PASS",
      activity_sources: ["session_registry"],
      qualifying_session_ids: debtRoles.includes(role) ? [] : [`${role}-20260401-120000`],
      debt_keys: debtKeys
        .filter((entry) => entry.startsWith(`${role}:`))
        .map((entry) => entry.split(":")[1]),
    })),
    summary: [
      `state=${state}`,
      `active_roles=${activeRoles.join(",") || "none"}`,
      `debt_roles=${debtRoles.join(",") || "none"}`,
      `debt_keys=${debtKeys.join(",") || "none"}`,
    ].join(" | "),
  };
}

function actorSession(integrationWorktreeDir) {
  return {
    wp_id: "WP-TEST-VALIDATOR-v1",
    role: "INTEGRATION_VALIDATOR",
    session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
    session_id: "integration-validator-session",
    session_thread_id: "thread-123",
    local_branch: "main",
    local_worktree_dir: integrationWorktreeDir,
    last_command_id: "cmd-1",
    last_command_status: "COMPLETED",
  };
}

function modernPacket(artifactPath, integrationWorktreeDir, {
  localBranch = "feat/WP-TEST-VALIDATOR-v1",
  localWorktreeDir = "../wtc-test-validator",
  wpValidatorLocalBranch = "feat/WP-TEST-VALIDATOR-v1",
  wpValidatorLocalWorktreeDir = "../wtc-test-validator",
} = {}) {
  return `# Task Packet: WP-TEST-VALIDATOR-v1

**Status:** Done

## METADATA
- WP_ID: WP-TEST-VALIDATOR-v1
- PACKET_FORMAT_VERSION: 2026-03-26
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 0123456789abcdef0123456789abcdef01234567
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-03-26T10:00:00Z
- PACKET_WIDENING_DECISION: NOT_REQUIRED
- PACKET_WIDENING_EVIDENCE: N/A
- MERGE_BASE_SHA: 0123456789abcdef0123456789abcdef01234567
- LOCAL_BRANCH: ${localBranch}
- LOCAL_WORKTREE_DIR: ${localWorktreeDir}
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ${integrationWorktreeDir}
- WP_VALIDATOR_LOCAL_BRANCH: ${wpValidatorLocalBranch}
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ${wpValidatorLocalWorktreeDir}
- **Artifacts**: \`${artifactPath}\`
- **Target File**: \`src/backend/handshake_core/src/example.rs\`
- **Start**: \`10\`
- **End**: \`12\`
- **Line Delta**: \`2\`

## VALIDATION_REPORTS
### 2026-04-01T12:00:00Z | INTEGRATION_VALIDATOR | session=integration-validator-session
Verdict: PASS
`.trim();
}

function legacyPacket() {
  return `# Task Packet: WP-TEST-VALIDATOR-v1

**Status:** Done

## METADATA
- WP_ID: WP-TEST-VALIDATOR-v1
- PACKET_FORMAT_VERSION: 2026-03-18
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
`.trim();
}

test("integration-validator context brief surfaces canonical final-lane authority bundle", () => {
  const repoRoot = path.resolve(".");
  const integrationWorktreeDir = "../handshake_main";
  const artifactDir = fs.mkdtempSync(path.join(repoRoot, ".tmp-integration-validator-brief-"));
  const artifactAbsPath = path.join(artifactDir, "validator-signed-scope.patch");
  const artifactPath = path.relative(repoRoot, artifactAbsPath).replace(/\\/g, "/");
  const normalizedDiff = [
    "diff --git a/src/backend/handshake_core/src/example.rs b/src/backend/handshake_core/src/example.rs",
    "index 1111111..2222222 100644",
    "--- a/src/backend/handshake_core/src/example.rs",
    "+++ b/src/backend/handshake_core/src/example.rs",
    "@@ -10,0 +11,2 @@",
    "+alpha",
    "+beta",
    "",
  ].join("\n");
  fs.writeFileSync(artifactAbsPath, normalizedDiff, "utf8");

  try {
    const brief = buildIntegrationValidatorContextBrief({
      repoRoot,
      wpId: "WP-TEST-VALIDATOR-v1",
      packetContent: modernPacket(artifactPath, integrationWorktreeDir),
      packetPathValueOverride: ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md",
      gitContext: {
        branch: "main",
        topLevel: integrationWorktreeDir,
      },
      committedEvidence: {
        status: "PASS",
        committed_validation_mode: "COMMITTED_REV",
        committed_validation_target: "HEAD",
        target_head_sha: "abcdefabcdefabcdefabcdefabcdefabcdefabcd",
        prepare_worktree_dir: "../wtc-test-validator",
      },
      registrySessions: [actorSession(integrationWorktreeDir)],
      requests: [
        {
          command_id: "cmd-1",
          wp_id: "WP-TEST-VALIDATOR-v1",
          role: "INTEGRATION_VALIDATOR",
          session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
          command_kind: "SEND_PROMPT",
          output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/cmd-1.jsonl",
        },
      ],
      results: [
        {
          command_id: "cmd-1",
          wp_id: "WP-TEST-VALIDATOR-v1",
          role: "INTEGRATION_VALIDATOR",
          session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
          command_kind: "SEND_PROMPT",
          status: "COMPLETED",
          output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/cmd-1.jsonl",
        },
      ],
      brokerState: { active_runs: [] },
      gateState: {
        closeout_sync_events: {
          "WP-TEST-VALIDATOR-v1": [
            {
              timestamp_utc: "2026-04-01T12:00:00Z",
              mode: "MERGE_PENDING",
              actor_role: "INTEGRATION_VALIDATOR",
              actor_session_id: "integration-validator-session",
              governed_action: {
                schema_id: "hsk.governed_action_result@1",
                schema_version: "governed_action_result_v1",
                action_id: "closeout-sync-action-1",
                processed_at: "2026-04-01T12:00:00Z",
                rule_id: "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE",
                action_kind: "EXTERNAL_EXECUTE",
                action_surface: "INTEGRATION_VALIDATOR_CLOSEOUT",
                command_kind: "CLOSEOUT_SYNC",
                command_id: "closeout-sync-action-1",
                session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
                wp_id: "WP-TEST-VALIDATOR-v1",
                role: "INTEGRATION_VALIDATOR",
                status: "COMPLETED",
                outcome_state: "SETTLED",
                result_state: "SETTLED",
                resume_disposition: "CONSUME_RESULT",
                target_command_id: "",
                summary: "Integration Validator recorded closeout sync MERGE_PENDING for WP-TEST-VALIDATOR-v1.",
                error: "",
                metadata: {},
              },
            },
          ],
        },
      },
      worktreeExists: () => true,
      fileExists: () => true,
      gitRunner: (args) => {
        if (args[0] === "cat-file") {
          return { code: 0, output: "" };
        }
        if (args[0] === "rev-parse") {
          return { code: 0, output: "0123456789abcdef0123456789abcdef01234567" };
        }
        if (args[0] === "merge-base" && args[1] === "--is-ancestor") {
          return args[2] === "0123456789abcdef0123456789abcdef01234567"
            ? { code: 0, output: "" }
            : { code: 1, output: "" };
        }
        if (args[0] === "merge-base") {
          return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
        }
        if (args[0] === "diff") {
          return { code: 0, output: normalizedDiff };
        }
        return { code: 0, output: "" };
      },
      declaredTopologyEvaluation: {
        ok: true,
        issues: [],
      },
      gateStatePath: "../gov_runtime/roles_shared/validator_gates/WP-TEST-VALIDATOR-v1.json",
      repomemCoverage: repomemCoverageFixture(),
    });

    assert.equal(brief.context_status, "OK");
    assert.equal(brief.closeout_readiness, "READY");
    assert.equal(brief.actor_context.role, "INTEGRATION_VALIDATOR");
    assert.equal(brief.governance_root.mode, "KERNEL");
    assert.equal(
      brief.packet_read_path,
      path.resolve(repoRoot, ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md").replace(/\\/g, "/"),
    );
    assert.match(brief.minimal_live_read_set[0], /startup output/i);
    assert.match(brief.minimal_live_read_set[1], /packet_read_path/i);
    assert.equal(brief.current_main_compatibility.status, "COMPATIBLE");
    assert.deepEqual(brief.required_commands, [
      "just check-notifications WP-TEST-VALIDATOR-v1 INTEGRATION_VALIDATOR",
      "just ack-notifications WP-TEST-VALIDATOR-v1 INTEGRATION_VALIDATOR integration-validator-session",
      "just phase-check CLOSEOUT WP-TEST-VALIDATOR-v1",
    ]);
    assert.equal(brief.closeout_provenance.governed_action_rule, "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE");
    assert.equal(brief.closeout_provenance.governed_action_resume_disposition, "CONSUME_RESULT");
    assert.match(brief.closeout_dependency_summary, /blockers=none/);
    assert.equal(brief.closeout_publication.closeout_mode, "MERGE_PENDING");
    assert.equal(brief.closeout_settlement.state, "SETTLED");
    assert.deepEqual(brief.closeout_settlement.blockers, []);
    assert.equal(brief.candidate_under_review.branch, "feat/WP-TEST-VALIDATOR-v1");
    assert.equal(brief.candidate_under_review.validator_policy_branch, "feat/WP-TEST-VALIDATOR-v1");
    assert.equal(brief.closeout_dependencies.sync_provenance.status, "RECORDED");
    assert.equal(brief.closeout_dependencies.repomem_coverage.status, "PASS");
    assert.match(brief.anti_rediscovery_rule, /Do not rebuild final-lane/i);
  } finally {
    fs.rmSync(artifactDir, { recursive: true, force: true });
  }
});

test("integration-validator context brief falls back to remediation commands for blocked historical packets", () => {
  const brief = buildIntegrationValidatorContextBrief({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: legacyPacket(),
    packetPathValueOverride: ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md",
    gitContext: {
      branch: "main",
      topLevel: "../handshake_main",
    },
    registrySessions: [],
    requests: [],
    results: [],
    brokerState: { active_runs: [] },
    worktreeExists: () => true,
    fileExists: () => true,
    declaredTopologyEvaluation: {
      ok: true,
      issues: [],
    },
    repomemCoverage: repomemCoverageFixture({
      state: "NO_ACTIVE_ROLES",
      activeRoles: [],
    }),
  });

  assert.equal(brief.context_status, "GOVERNANCE_BLOCKED");
  assert.equal(
    brief.packet_read_path,
    path.resolve(".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md").replace(/\\/g, "/"),
  );
  assert.deepEqual(brief.required_commands, [
    "just validator-policy-gate WP-TEST-VALIDATOR-v1",
    "just phase-check CLOSEOUT WP-TEST-VALIDATOR-v1",
  ]);
});

test("integration-validator context brief prefers canonical execution-state closeout status over stale packet and board artifacts", () => {
  const brief = buildIntegrationValidatorContextBrief({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: modernPacket("artifacts/validator-signed-scope.patch", "../handshake_main"),
    packetPathValueOverride: ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md",
    gitContext: {
      branch: "main",
      topLevel: "../handshake_main",
    },
    runtimeStatus: {
      current_packet_status: "Done",
      current_task_board_status: "IN_PROGRESS",
      execution_state: {
        schema_version: "wp_execution_state@1",
        authority: {
          packet_status: "Validated (FAIL)",
          task_board_status: "DONE_FAIL",
          runtime_status: "completed",
        },
      },
    },
    taskBoardStatusOverride: "IN_PROGRESS",
    registrySessions: [],
    requests: [],
    results: [],
    brokerState: { active_runs: [] },
    worktreeExists: () => true,
    fileExists: () => true,
    declaredTopologyEvaluation: {
      ok: true,
      issues: [],
    },
    repomemCoverage: repomemCoverageFixture({
      state: "DEBT",
      activeRoles: ["ORCHESTRATOR"],
      debtRoles: ["ORCHESTRATOR"],
      debtKeys: ["ORCHESTRATOR:NO_WP_DURABLE_CHECKPOINT"],
    }),
  });

  assert.equal(brief.packet_status, "Validated (FAIL)");
  assert.equal(brief.current_wp_status, "DONE_FAIL");
    assert.equal(brief.task_board_status, "DONE_FAIL");
    assert.equal(brief.closeout_requirements.terminal_non_pass_packet, true);
    assert.equal(brief.closeout_requirements.require_ready_for_pass, false);
    assert.equal(brief.closeout_dependencies.repomem_coverage.status, "DEBT");
  });

test("integration-validator context brief separates candidate-under-review truth from validator branch policy", () => {
  const repoRoot = path.resolve(".");
  const integrationWorktreeDir = "../handshake_main";
  const artifactDir = fs.mkdtempSync(path.join(repoRoot, ".tmp-integration-validator-brief-"));
  const artifactAbsPath = path.join(artifactDir, "validator-signed-scope.patch");
  const artifactPath = path.relative(repoRoot, artifactAbsPath).replace(/\\/g, "/");
  const normalizedDiff = [
    "diff --git a/src/backend/handshake_core/src/example.rs b/src/backend/handshake_core/src/example.rs",
    "index 1111111..2222222 100644",
    "--- a/src/backend/handshake_core/src/example.rs",
    "+++ b/src/backend/handshake_core/src/example.rs",
    "@@ -10,0 +11,2 @@",
    "+alpha",
    "+beta",
    "",
  ].join("\n");
  fs.writeFileSync(artifactAbsPath, normalizedDiff, "utf8");

  try {
    const brief = buildIntegrationValidatorContextBrief({
      repoRoot,
      wpId: "WP-TEST-VALIDATOR-v1",
      packetContent: modernPacket(artifactPath, integrationWorktreeDir, {
        localBranch: "feat/WP-TEST-VALIDATOR-v1-mainproof",
        localWorktreeDir: "../wtc-test-validator-mainproof",
        wpValidatorLocalBranch: "feat/WP-TEST-VALIDATOR-v1",
        wpValidatorLocalWorktreeDir: "../wtc-test-validator",
      }),
      packetPathValueOverride: ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md",
      gitContext: {
        branch: "main",
        topLevel: integrationWorktreeDir,
      },
      committedEvidence: {
        status: "PASS",
        committed_validation_mode: "COMMITTED_REV",
        committed_validation_target: "HEAD",
        target_head_sha: "abcdefabcdefabcdefabcdefabcdefabcdefabcd",
        prepare_worktree_dir: "../wtc-test-validator-mainproof",
      },
      registrySessions: [actorSession(integrationWorktreeDir)],
      requests: [],
      results: [],
      brokerState: { active_runs: [] },
      worktreeExists: () => true,
      fileExists: () => true,
      gitRunner: (args) => {
        if (args[0] === "cat-file") return { code: 0, output: "" };
        if (args[0] === "rev-parse") return { code: 0, output: "0123456789abcdef0123456789abcdef01234567" };
        if (args[0] === "merge-base" && args[1] === "--is-ancestor") {
          return args[2] === "0123456789abcdef0123456789abcdef01234567"
            ? { code: 0, output: "" }
            : { code: 1, output: "" };
        }
        if (args[0] === "merge-base") return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
        if (args[0] === "diff") return { code: 0, output: normalizedDiff };
        return { code: 0, output: "" };
      },
      declaredTopologyEvaluation: {
        ok: true,
        issues: [],
      },
      repomemCoverage: repomemCoverageFixture(),
    });

    assert.equal(brief.candidate_under_review.branch, "feat/WP-TEST-VALIDATOR-v1-mainproof");
    assert.equal(brief.candidate_under_review.worktree_dir, "../wtc-test-validator-mainproof");
    assert.equal(brief.candidate_under_review.validator_policy_branch, "feat/WP-TEST-VALIDATOR-v1");
    assert.equal(brief.candidate_under_review.validator_policy_worktree_dir, "../wtc-test-validator");
    assert.match(brief.candidate_under_review.handoff_range, /\.\./);
  } finally {
    fs.rmSync(artifactDir, { recursive: true, force: true });
  }
});
