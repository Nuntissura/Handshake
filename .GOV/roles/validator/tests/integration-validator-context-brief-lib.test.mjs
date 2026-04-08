import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { buildIntegrationValidatorContextBrief } from "../scripts/lib/integration-validator-context-brief-lib.mjs";

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

function modernPacket(artifactPath, integrationWorktreeDir) {
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
- LOCAL_BRANCH: feat/WP-TEST-VALIDATOR-v1
- LOCAL_WORKTREE_DIR: ../wtc-test-validator
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ${integrationWorktreeDir}
- WP_VALIDATOR_LOCAL_BRANCH: review/WP-TEST-VALIDATOR-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtv-test-validator
- **Artifacts**: \`${artifactPath}\`
- **Target File**: \`src/backend/handshake_core/src/example.rs\`
- **Start**: \`10\`
- **End**: \`12\`
- **Line Delta**: \`2\`
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
          return { code: 1, output: "" };
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
    });

    assert.equal(brief.context_status, "OK");
    assert.equal(brief.closeout_readiness, "READY");
    assert.equal(brief.actor_context.role, "INTEGRATION_VALIDATOR");
    assert.equal(brief.governance_root.mode, "KERNEL");
    assert.equal(brief.current_main_compatibility.status, "COMPATIBLE");
    assert.deepEqual(brief.required_commands, [
      "just check-notifications WP-TEST-VALIDATOR-v1 INTEGRATION_VALIDATOR",
      "just ack-notifications WP-TEST-VALIDATOR-v1 INTEGRATION_VALIDATOR integration-validator-session",
      "just phase-check CLOSEOUT WP-TEST-VALIDATOR-v1",
    ]);
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
  });

  assert.equal(brief.context_status, "GOVERNANCE_BLOCKED");
  assert.deepEqual(brief.required_commands, [
    "just validator-policy-gate WP-TEST-VALIDATOR-v1",
    "just phase-check CLOSEOUT WP-TEST-VALIDATOR-v1",
  ]);
});
