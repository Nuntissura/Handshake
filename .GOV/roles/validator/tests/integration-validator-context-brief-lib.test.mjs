import assert from "node:assert/strict";
import test from "node:test";
import { buildIntegrationValidatorContextBrief } from "../scripts/lib/integration-validator-context-brief-lib.mjs";

function actorSession() {
  return {
    wp_id: "WP-TEST-VALIDATOR-v1",
    role: "INTEGRATION_VALIDATOR",
    session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
    session_id: "integration-validator-session",
    session_thread_id: "thread-123",
    local_branch: "main",
    local_worktree_dir: "../handshake_main",
    last_command_id: "cmd-1",
    last_command_status: "COMPLETED",
  };
}

function modernPacket() {
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
- WP_VALIDATOR_LOCAL_BRANCH: review/WP-TEST-VALIDATOR-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtv-test-validator
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
  const brief = buildIntegrationValidatorContextBrief({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: modernPacket(),
    gitContext: {
      branch: "main",
      topLevel: "../handshake_main",
    },
    committedEvidence: {
      status: "PASS",
      committed_validation_mode: "COMMITTED_REV",
      committed_validation_target: "HEAD",
      target_head_sha: "abc123",
      prepare_worktree_dir: "../wtc-test-validator",
    },
    registrySessions: [actorSession()],
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
    gitRunner: (args) => (
      args[0] === "rev-parse"
        ? { code: 0, output: "0123456789abcdef0123456789abcdef01234567" }
        : { code: 0, output: "" }
    ),
    declaredTopologyEvaluation: {
      ok: true,
      issues: [],
    },
    gateStatePath: "../gov_runtime/roles_shared/validator_gates/WP-TEST-VALIDATOR-v1.json",
  });

  assert.equal(brief.context_status, "OK");
  assert.equal(brief.closeout_readiness, "READY");
  assert.equal(brief.actor_context.role, "INTEGRATION_VALIDATOR");
  assert.equal(brief.current_main_compatibility.status, "COMPATIBLE");
  assert.equal(brief.required_commands[0], "just integration-validator-context-brief WP-TEST-VALIDATOR-v1");
  assert.match(brief.anti_rediscovery_rule, /Do not rebuild final-lane/i);
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
    "just validator-packet-complete WP-TEST-VALIDATOR-v1",
  ]);
});
