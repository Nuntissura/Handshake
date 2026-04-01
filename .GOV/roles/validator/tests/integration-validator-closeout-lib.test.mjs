import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  evaluateIntegrationValidatorCloseoutState,
  evaluateIntegrationValidatorTopology,
  evaluateWpSessionControlCloseoutBundle,
} from "../scripts/lib/integration-validator-closeout-lib.mjs";

function writeFile(targetPath, content) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, content, "utf8");
}

function repoRootWithArtifact(diffText) {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "integration-closeout-signed-scope-"));
  writeFile(path.join(tempRoot, "artifacts", "signed.patch"), diffText);
  return tempRoot;
}

const matchingDiff = [
  "diff --git a/src/demo.rs b/src/demo.rs",
  "--- a/src/demo.rs",
  "+++ b/src/demo.rs",
  "@@ -10 +10,2 @@",
  "-old",
  "+new",
  "+extra",
  "",
].join("\n");

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
- PACKET_FORMAT_VERSION: 2026-03-26
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 0123456789abcdef0123456789abcdef01234567
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-03-26T10:00:00Z
- PACKET_WIDENING_DECISION: NOT_REQUIRED
- PACKET_WIDENING_EVIDENCE: N/A
- **Target File**: \`src/demo.rs\`
- **Start**: 10
- **End**: 20
- **Line Delta**: 3
- **Artifacts**: \`artifacts/signed.patch\`
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
    gitRunner: (args) => (
      args[0] === "rev-parse"
        ? { code: 0, output: "0123456789abcdef0123456789abcdef01234567" }
        : { code: 0, output: "" }
    ),
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

test("integration-validator topology fails when live governance still resolves to handshake_main backup state", () => {
  const evaluation = evaluateIntegrationValidatorTopology({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: packetFixture(),
    actorContext: actorContextFixture(),
    committedEvidence: {
      status: "PASS",
      target_head_sha: "abc123",
    },
    governanceRootAbs: path.resolve(".", "..", "handshake_main", ".GOV"),
    worktreeExists: () => true,
    gitRunner: (args) => (
      args[0] === "rev-parse"
        ? { code: 0, output: "0123456789abcdef0123456789abcdef01234567" }
        : { code: 0, output: "" }
    ),
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /must resolve live governance from the kernel via HANDSHAKE_GOV_ROOT/i);
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
  const repoRoot = repoRootWithArtifact(matchingDiff);
  const evaluation = evaluateIntegrationValidatorCloseoutState({
    repoRoot,
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
    gitRunner: (args) => {
      if (args[0] === "rev-parse") return { code: 0, output: "0123456789abcdef0123456789abcdef01234567" };
      if (args[0] === "merge-base") return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
      if (args[0] === "diff") return { code: 0, output: matchingDiff };
      return { code: 0, output: "" };
    },
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.issues.length, 0);
  assert.equal(evaluation.closeoutBundle.summary.active_run_count, 0);
});

test("integration-validator closeout state fails when signed scope compatibility is stale against current main", () => {
  const repoRoot = repoRootWithArtifact(matchingDiff);
  const evaluation = evaluateIntegrationValidatorCloseoutState({
    repoRoot,
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: packetFixture(),
    actorContext: actorContextFixture(),
    committedEvidence: {
      status: "PASS",
      target_head_sha: "abc123",
    },
    requests: [],
    results: [],
    registrySessions: [],
    brokerState: { active_runs: [] },
    worktreeExists: () => true,
    fileExists: () => true,
    gitRunner: (args) => {
      if (args[0] === "rev-parse") return { code: 0, output: "89abcdef0123456789abcdef0123456789abcdef" };
      if (args[0] === "merge-base") return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
      if (args[0] === "diff") return { code: 0, output: matchingDiff };
      return { code: 0, output: "" };
    },
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /does not match current local main HEAD/i);
});

test("integration-validator closeout state fails when the committed target diff drifts from the signed artifact", () => {
  const repoRoot = repoRootWithArtifact(matchingDiff);
  const driftedDiff = [
    "diff --git a/src/demo.rs b/src/demo.rs",
    "--- a/src/demo.rs",
    "+++ b/src/demo.rs",
    "@@ -10 +10,3 @@",
    "-old",
    "+new",
    "+extra",
    "+drift",
    "",
  ].join("\n");
  const evaluation = evaluateIntegrationValidatorCloseoutState({
    repoRoot,
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: packetFixture(),
    actorContext: actorContextFixture(),
    committedEvidence: {
      status: "PASS",
      target_head_sha: "abc123",
    },
    requests: [],
    results: [],
    registrySessions: [],
    brokerState: { active_runs: [] },
    worktreeExists: () => true,
    fileExists: () => true,
    gitRunner: (args) => {
      if (args[0] === "rev-parse") return { code: 0, output: "0123456789abcdef0123456789abcdef01234567" };
      if (args[0] === "merge-base") return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
      if (args[0] === "diff") return { code: 0, output: driftedDiff };
      return { code: 0, output: "" };
    },
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /candidate target diff does not match the signed patch artifact/i);
});
