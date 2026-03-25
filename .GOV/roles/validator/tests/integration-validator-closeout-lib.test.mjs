import assert from "node:assert/strict";
import test from "node:test";
import {
  evaluateIntegrationValidatorCloseoutState,
  evaluateIntegrationValidatorTopology,
  evaluateWpSessionControlCloseoutBundle,
} from "../scripts/lib/integration-validator-closeout-lib.mjs";

function actorContextFixture() {
  return {
    actorRole: "INTEGRATION_VALIDATOR",
    actorSessionKey: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
    actorSessionId: "integration-validator-session",
    actorThreadId: "thread-123",
    actorBranch: "main",
    actorWorktreeDir: "../handshake_main",
    source: "SESSION_REGISTRY",
  };
}

function packetFixture() {
  return `# Task Packet: WP-TEST-VALIDATOR-v1

**Status:** Done

## METADATA
- WP_ID: WP-TEST-VALIDATOR-v1
- PACKET_FORMAT_VERSION: 2026-03-25
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
`.trim();
}

test("integration-validator topology passes when the governed final lane resolves the committed target", () => {
  const evaluation = evaluateIntegrationValidatorTopology({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: packetFixture(),
    actorContext: actorContextFixture(),
    committedEvidence: {
      status: "PASS",
      target_head_sha: "abc123",
    },
    worktreeExists: () => true,
    gitRunner: () => ({ code: 0, output: "" }),
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.issues.length, 0);
});

test("integration-validator topology fails when the final lane cannot resolve the committed target", () => {
  const evaluation = evaluateIntegrationValidatorTopology({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: packetFixture(),
    actorContext: actorContextFixture(),
    committedEvidence: {
      status: "PASS",
      target_head_sha: "deadbeef",
    },
    worktreeExists: () => true,
    gitRunner: () => ({ code: 1, output: "missing object" }),
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /cannot resolve committed target deadbeef/i);
});

test("WP closeout bundle passes when every request is settled and no run is active", () => {
  const evaluation = evaluateWpSessionControlCloseoutBundle({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
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
    sessions: [
      {
        wp_id: "WP-TEST-VALIDATOR-v1",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        last_command_id: "cmd-1",
        last_command_status: "COMPLETED",
      },
    ],
    brokerState: { active_runs: [] },
    fileExists: () => true,
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.issues.length, 0);
});

test("WP closeout bundle fails when an active run or unsettled request still exists", () => {
  const evaluation = evaluateWpSessionControlCloseoutBundle({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    requests: [
      {
        command_id: "cmd-2",
        wp_id: "WP-TEST-VALIDATOR-v1",
        role: "CODER",
        session_key: "CODER:WP-TEST-VALIDATOR-v1",
        command_kind: "SEND_PROMPT",
        output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/cmd-2.jsonl",
      },
    ],
    results: [],
    sessions: [
      {
        wp_id: "WP-TEST-VALIDATOR-v1",
        session_key: "CODER:WP-TEST-VALIDATOR-v1",
        last_command_id: "cmd-2",
        last_command_status: "RUNNING",
      },
    ],
    brokerState: {
      active_runs: [
        {
          command_id: "cmd-2",
          wp_id: "WP-TEST-VALIDATOR-v1",
        },
      ],
    },
    fileExists: () => false,
  });

  assert.equal(evaluation.ok, false);
  const details = evaluation.issues.join("\n");
  assert.match(details, /active broker runs still exist/i);
  assert.match(details, /has no settled result/i);
  assert.match(details, /still reports RUNNING/i);
});

test("integration-validator closeout state combines topology and WP-scoped closeout truth", () => {
  const evaluation = evaluateIntegrationValidatorCloseoutState({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: packetFixture(),
    actorContext: actorContextFixture(),
    committedEvidence: {
      status: "PASS",
      target_head_sha: "abc123",
    },
    requests: [
      {
        command_id: "cmd-3",
        wp_id: "WP-TEST-VALIDATOR-v1",
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        command_kind: "SEND_PROMPT",
        output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/cmd-3.jsonl",
      },
    ],
    results: [
      {
        command_id: "cmd-3",
        wp_id: "WP-TEST-VALIDATOR-v1",
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        command_kind: "SEND_PROMPT",
        status: "COMPLETED",
        output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/cmd-3.jsonl",
      },
    ],
    registrySessions: [
      {
        wp_id: "WP-TEST-VALIDATOR-v1",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        last_command_id: "cmd-3",
        last_command_status: "COMPLETED",
      },
    ],
    brokerState: { active_runs: [] },
    worktreeExists: () => true,
    fileExists: () => true,
    gitRunner: () => ({ code: 0, output: "" }),
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.issues.length, 0);
  assert.equal(evaluation.closeoutBundle.summary.active_run_count, 0);
});
