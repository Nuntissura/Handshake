import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  appendCloseoutSyncProvenance,
  buildCloseoutSyncGovernedAction,
  deriveFinalLaneGovernanceInvalidity,
  evaluateIntegrationValidatorCloseoutState,
  evaluateIntegrationValidatorTopology,
  evaluateWpSessionControlCloseoutBundle,
  latestCloseoutSyncEvent,
  latestCloseoutSyncGovernedAction,
  resolveIntegrationValidatorCloseoutRequirements,
  resolveCloseoutValidatorSessionsOfRecord,
  summarizeCloseoutSyncGovernance,
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

function packetFixture(overrides = {}) {
  const {
    status = "Done",
    currentMainCompatibilityStatus = "COMPATIBLE",
    currentMainCompatibilityBaselineSha = "0123456789abcdef0123456789abcdef01234567",
    currentMainCompatibilityVerifiedAtUtc = "2026-03-26T10:00:00Z",
    packetWideningDecision = "NOT_REQUIRED",
    packetWideningEvidence = "N/A",
    currentWpStatus = "",
  } = overrides;
  const currentWpStatusLine = currentWpStatus
    ? `- Current WP_STATUS: ${currentWpStatus}\n`
    : "";

  return `# Task Packet: WP-TEST-VALIDATOR-v1

**Status:** ${status}

## METADATA
- WP_ID: WP-TEST-VALIDATOR-v1
- PACKET_FORMAT_VERSION: 2026-03-26
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_OF_RECORD: INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1
- CURRENT_MAIN_COMPATIBILITY_STATUS: ${currentMainCompatibilityStatus}
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: ${currentMainCompatibilityBaselineSha}
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: ${currentMainCompatibilityVerifiedAtUtc}
- PACKET_WIDENING_DECISION: ${packetWideningDecision}
- PACKET_WIDENING_EVIDENCE: ${packetWideningEvidence}
- **Target File**: \`src/demo.rs\`
- **Start**: 10
- **End**: 20
- **Line Delta**: 3
- **Artifacts**: \`artifacts/signed.patch\`
## STATUS_HANDOFF
${currentWpStatusLine}`.trim();
}

test("terminal OUTDATED_ONLY packets drop PASS-ready closeout requirements while keeping recorded compatibility truth", () => {
  const requirements = resolveIntegrationValidatorCloseoutRequirements({
    packetContent: packetFixture({
      status: "Validated (OUTDATED_ONLY)",
      currentMainCompatibilityStatus: "ADJACENT_SCOPE_REQUIRED",
      packetWideningDecision: "FOLLOW_ON_WP_REQUIRED",
      packetWideningEvidence: "Adjacent current-main drift lives in api/flight_recorder.rs outside the signed packet.",
      currentWpStatus: "DONE_OUTDATED_ONLY",
    }),
  });

  assert.equal(requirements.terminalNonPass, true);
  assert.equal(requirements.requireReadyForPass, false);
  assert.equal(requirements.requireRecordedScopeCompatibility, true);
});

test("canonical execution-state terminal non-pass status suppresses PASS-ready closeout requirements even when packet text is stale", () => {
  const requirements = resolveIntegrationValidatorCloseoutRequirements({
    packetContent: packetFixture({
      status: "Done",
      currentWpStatus: "IN_PROGRESS",
    }),
    runtimeStatus: {
      current_packet_status: "Done",
      current_task_board_status: "IN_PROGRESS",
      execution_state: {
        schema_version: "wp_execution_state@1",
        authority: {
          packet_status: "Validated (OUTDATED_ONLY)",
          task_board_status: "DONE_OUTDATED_ONLY",
          runtime_status: "completed",
        },
      },
    },
    taskBoardStatus: "IN_PROGRESS",
  });

  assert.equal(requirements.packetStatus, "Validated (OUTDATED_ONLY)");
  assert.equal(requirements.currentWpStatus, "DONE_OUTDATED_ONLY");
  assert.equal(requirements.terminalNonPass, true);
  assert.equal(requirements.requireReadyForPass, false);
  assert.equal(requirements.requireRecordedScopeCompatibility, true);
});

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

test("WP closeout bundle tolerates the current integration-validator broker run while final-lane closeout is executing", () => {
  const evaluation = evaluateWpSessionControlCloseoutBundle({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    actorContext: actorContextFixture(),
    requests: [
      {
        command_id: "cmd-closeout-self",
        wp_id: "WP-TEST-VALIDATOR-v1",
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        command_kind: "SEND_PROMPT",
        output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/cmd-closeout-self.jsonl",
      },
    ],
    results: [],
    sessions: [
      {
        wp_id: "WP-TEST-VALIDATOR-v1",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        last_command_id: "cmd-closeout-self",
        last_command_status: "RUNNING",
      },
    ],
    brokerState: {
      active_runs: [
        {
          command_id: "cmd-closeout-self",
          wp_id: "WP-TEST-VALIDATOR-v1",
          role: "INTEGRATION_VALIDATOR",
          session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        },
      ],
    },
    fileExists: () => false,
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.issues.length, 0);
  assert.equal(evaluation.summary.active_run_count, 1);
  assert.equal(evaluation.summary.self_active_run_count, 1);
  assert.equal(evaluation.summary.blocking_active_run_count, 0);
  assert.match(evaluation.warnings.join("\n"), /treating that self-owned run as non-blocking/i);
});

test("WP closeout bundle infers the self-owned validator session key when actor context omits it", () => {
  const evaluation = evaluateWpSessionControlCloseoutBundle({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    actorContext: {
      actorRole: "INTEGRATION_VALIDATOR",
      actorSessionKey: "",
      actorSessionId: "integration-validator-session",
      actorThreadId: "thread-123",
      actorBranch: "main",
      actorWorktreeDir: "../handshake_main",
      source: "SESSION_REGISTRY",
    },
    requests: [
      {
        command_id: "cmd-closeout-self",
        wp_id: "WP-TEST-VALIDATOR-v1",
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        command_kind: "SEND_PROMPT",
        output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/cmd-closeout-self.jsonl",
      },
    ],
    results: [],
    sessions: [
      {
        wp_id: "WP-TEST-VALIDATOR-v1",
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        session_thread_id: "thread-123",
        local_branch: "main",
        local_worktree_dir: "../handshake_main",
        last_command_id: "cmd-closeout-self",
        last_command_status: "RUNNING",
      },
    ],
    brokerState: {
      active_runs: [
        {
          command_id: "cmd-closeout-self",
          wp_id: "WP-TEST-VALIDATOR-v1",
          role: "INTEGRATION_VALIDATOR",
          session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        },
      ],
    },
    fileExists: () => false,
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.issues.length, 0);
  assert.equal(evaluation.summary.self_active_run_count, 1);
  assert.equal(evaluation.summary.blocking_active_run_count, 0);
  assert.match(evaluation.warnings.join("\n"), /self-owned run/i);
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
      if (args[0] === "merge-base" && args[1] === "--is-ancestor") return { code: 1, output: "" };
      if (args[0] === "merge-base") return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
      if (args[0] === "diff") return { code: 0, output: matchingDiff };
      return { code: 0, output: "" };
    },
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.issues.length, 0);
  assert.equal(evaluation.closeoutBundle.summary.active_run_count, 0);
  assert.equal(evaluation.dependencyView.ok, true);
  assert.equal(evaluation.dependencyView.publication.closeout_mode, "MERGE_PENDING");
  assert.equal(evaluation.dependencyView.dependencies.scope_compatibility.status, "PASS");
});

test("integration-validator closeout state passes with a self-owned active final-lane broker run", () => {
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
        command_id: "cmd-closeout-self",
        wp_id: "WP-TEST-VALIDATOR-v1",
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        command_kind: "SEND_PROMPT",
        output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/cmd-closeout-self.jsonl",
      },
    ],
    results: [],
    registrySessions: [
      {
        wp_id: "WP-TEST-VALIDATOR-v1",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        last_command_id: "cmd-closeout-self",
        last_command_status: "RUNNING",
      },
    ],
    brokerState: {
      active_runs: [
        {
          command_id: "cmd-closeout-self",
          wp_id: "WP-TEST-VALIDATOR-v1",
          role: "INTEGRATION_VALIDATOR",
          session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        },
      ],
    },
    worktreeExists: () => true,
    fileExists: () => true,
    gitRunner: (args) => {
      if (args[0] === "rev-parse") return { code: 0, output: "0123456789abcdef0123456789abcdef01234567" };
      if (args[0] === "merge-base" && args[1] === "--is-ancestor") return { code: 1, output: "" };
      if (args[0] === "merge-base") return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
      if (args[0] === "diff") return { code: 0, output: matchingDiff };
      return { code: 0, output: "" };
    },
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.issues.length, 0);
  assert.equal(evaluation.closeoutBundle.summary.self_active_run_count, 1);
  assert.equal(evaluation.closeoutBundle.summary.blocking_active_run_count, 0);
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

test("integration-validator closeout state can refresh stale recorded compatibility truth during sync", () => {
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
    requireRecordedScopeCompatibility: false,
    worktreeExists: () => true,
    fileExists: () => true,
    gitRunner: (args) => {
      if (args[0] === "rev-parse") return { code: 0, output: "89abcdef0123456789abcdef0123456789abcdef" };
      if (args[0] === "merge-base" && args[1] === "--is-ancestor") return { code: 1, output: "" };
      if (args[0] === "merge-base") return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
      if (args[0] === "diff") return { code: 0, output: matchingDiff };
      return { code: 0, output: "" };
    },
  });

  assert.equal(evaluation.ok, true);
  assert.deepEqual(evaluation.issues, []);
  assert.match(
    evaluation.scopeCompatibility.errors.join("\n"),
    /does not match current local main HEAD/i,
  );
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
      if (args[0] === "merge-base" && args[1] === "--is-ancestor") return { code: 1, output: "" };
      if (args[0] === "merge-base") return { code: 0, output: "fedcba9876543210fedcba9876543210fedcba98" };
      if (args[0] === "diff") return { code: 0, output: driftedDiff };
      return { code: 0, output: "" };
    },
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /candidate target diff does not match the signed patch artifact/i);
});

test("deriveFinalLaneGovernanceInvalidity classifies kernel-side final-lane misuse as a role-boundary breach", () => {
  const invalidity = deriveFinalLaneGovernanceInvalidity({
    repoRoot: ".",
    actorContext: {
      actorRole: "UNKNOWN",
      actorSessionId: "",
    },
    gitContext: {
      branch: "gov_kernel",
      topLevel: path.resolve("."),
    },
  });

  assert.equal(invalidity?.workflowInvalidityCode, "ROLE_BOUNDARY_BREACH");
  assert.equal(invalidity?.actorRole, "ORCHESTRATOR");
});

test("deriveFinalLaneGovernanceInvalidity classifies handshake_main governance-root drift as a final-lane gov-root violation", () => {
  const invalidity = deriveFinalLaneGovernanceInvalidity({
    repoRoot: ".",
    actorContext: actorContextFixture(),
    gitContext: {
      branch: "main",
      topLevel: path.resolve(".", "..", "handshake_main"),
    },
    governanceState: {
      terminalReason: "INTEGRATION_VALIDATOR_GOV_ROOT_MISCONFIGURED",
    },
  });

  assert.equal(invalidity?.workflowInvalidityCode, "FINAL_LANE_GOV_ROOT_VIOLATION");
  assert.equal(invalidity?.actorRole, "INTEGRATION_VALIDATOR");
});

test("appendCloseoutSyncProvenance records and returns the latest closeout event per WP", () => {
  const nextState = appendCloseoutSyncProvenance({}, {
    wpId: "WP-TEST-VALIDATOR-v1",
    event: {
      timestamp_utc: "2026-04-01T12:00:00Z",
      mode: "MERGE_PENDING",
      actor_role: "INTEGRATION_VALIDATOR",
      actor_session_id: "integration-validator-session",
      governed_action: buildCloseoutSyncGovernedAction({
        wpId: "WP-TEST-VALIDATOR-v1",
        mode: "MERGE_PENDING",
        packetStatus: "Validated (PASS)",
        mainContainmentStatus: "MERGE_PENDING",
        actorSessionId: "integration-validator-session",
        processedAt: "2026-04-01T12:00:00Z",
      }),
    },
  });

  const latest = latestCloseoutSyncEvent(nextState, "WP-TEST-VALIDATOR-v1");
  assert.equal(latest?.mode, "MERGE_PENDING");
  assert.equal(latest?.actor_role, "INTEGRATION_VALIDATOR");
  assert.equal(latest?.actor_session_id, "integration-validator-session");
  assert.equal(latest?.governed_action_summary?.rule_id, "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE");
  assert.equal(
    latestCloseoutSyncGovernedAction(nextState, "WP-TEST-VALIDATOR-v1")?.resume_disposition,
    "CONSUME_RESULT",
  );
});

test("summarizeCloseoutSyncGovernance returns the typed final-lane sync summary", () => {
  const gateState = appendCloseoutSyncProvenance({}, {
    wpId: "WP-TEST-VALIDATOR-v1",
    event: {
      timestamp_utc: "2026-04-01T12:00:00Z",
      mode: "CONTAINED_IN_MAIN",
      actor_role: "INTEGRATION_VALIDATOR",
      actor_session_id: "integration-validator-session",
      governed_action: buildCloseoutSyncGovernedAction({
        wpId: "WP-TEST-VALIDATOR-v1",
        mode: "CONTAINED_IN_MAIN",
        packetStatus: "Validated (PASS)",
        mainContainmentStatus: "CONTAINED_IN_MAIN",
        actorSessionId: "integration-validator-session",
        processedAt: "2026-04-01T12:00:00Z",
      }),
    },
  });

  const summary = summarizeCloseoutSyncGovernance(gateState, "WP-TEST-VALIDATOR-v1");
  assert.equal(summary.status, "RECORDED");
  assert.equal(summary.latestEvent?.mode, "CONTAINED_IN_MAIN");
  assert.equal(summary.latestGovernedAction?.rule_id, "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE");
});

test("resolveCloseoutValidatorSessionsOfRecord derives terminal validator-of-record values from receipts and actor context", () => {
  const sessions = resolveCloseoutValidatorSessionsOfRecord({
    packetContent: [
      "## METADATA",
      "- WP_VALIDATOR_OF_RECORD: <unassigned>",
      "- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>",
    ].join("\n"),
    receipts: [
      {
        timestamp_utc: "2026-04-03T05:41:50.089Z",
        actor_role: "WP_VALIDATOR",
        actor_session: "wp_validator:wp-test-validator-v1",
      },
    ],
    actorContext: actorContextFixture(),
  });

  assert.equal(sessions.wpValidatorOfRecord, "wp_validator:wp-test-validator-v1");
  assert.equal(sessions.integrationValidatorOfRecord, "integration-validator-session");
});
