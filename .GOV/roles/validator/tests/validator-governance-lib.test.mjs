import assert from "node:assert/strict";
import test from "node:test";
import {
  buildValidatorReadyCommands,
  evaluateValidatorPacketGovernanceState,
  evaluateValidatorPassAuthority,
  readValidatorAuthority,
  resolveValidatorActorContext,
} from "../scripts/lib/validator-governance-lib.mjs";
import { defaultWpValidatorBranch, defaultWpValidatorWorktreeDir } from "../../../roles_shared/scripts/session/session-policy.mjs";

function packetFixture({
  packetFormatVersion = "2026-03-23",
  status = "Done",
  workflowLane = "ORCHESTRATOR_MANAGED",
  technicalAuthority = "INTEGRATION_VALIDATOR",
  mergeAuthority = "INTEGRATION_VALIDATOR",
  integrationValidatorOfRecord = "<unassigned>",
} = {}) {
  return `# Task Packet: WP-TEST-VALIDATOR-v1

**Status:** ${status}

## METADATA
- WP_ID: WP-TEST-VALIDATOR-v1
- PACKET_FORMAT_VERSION: ${packetFormatVersion}
- WORKFLOW_LANE: ${workflowLane}
- TECHNICAL_AUTHORITY: ${technicalAuthority}
- MERGE_AUTHORITY: ${mergeAuthority}
- INTEGRATION_VALIDATOR_OF_RECORD: ${integrationValidatorOfRecord}
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
`.trim();
}

test("validator packet policy blocks pre-threshold folder packets as remediation-required legacy closures", () => {
  const evaluation = evaluateValidatorPacketGovernanceState({
    wpId: "WP-TEST-VALIDATOR-v1",
    packetPath: ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md",
    packetContent: packetFixture({
      packetFormatVersion: "2026-03-18",
      status: "Done",
    }),
    currentWpStatus: "Validated",
    taskBoardStatus: "VALIDATED",
    sessionStatus: "USER_ACKNOWLEDGED",
  });

  assert.equal(evaluation.allowValidationResume, false);
  assert.equal(evaluation.legacyRemediationRequired, true);
  assert.equal(evaluation.terminalReason, "LEGACY_REMEDIATION_REQUIRED");
});

test("validator packet policy leaves current completion-layer packets resumable", () => {
  const evaluation = evaluateValidatorPacketGovernanceState({
    wpId: "WP-TEST-VALIDATOR-v1",
    packetPath: ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md",
    packetContent: packetFixture({
      packetFormatVersion: "2026-03-23",
      status: "Done",
    }),
    currentWpStatus: "Ready for Validator",
    taskBoardStatus: "IN_PROGRESS",
  });

  assert.equal(evaluation.allowValidationResume, true);
  assert.equal(evaluation.legacyRemediationRequired, false);
  assert.equal(evaluation.terminalReason, "ACTIVE");
});

test("validator authority defaults keep orchestrator-managed packets on the integration-validator final lane", () => {
  const authority = readValidatorAuthority(packetFixture());

  assert.equal(authority.workflowLane, "ORCHESTRATOR_MANAGED");
  assert.equal(authority.technicalAuthority, "INTEGRATION_VALIDATOR");
  assert.equal(authority.mergeAuthority, "INTEGRATION_VALIDATOR");
});

test("validator actor context infers integration-validator from handshake_main main lane", () => {
  const actorContext = resolveValidatorActorContext({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: packetFixture(),
    gitContext: {
      branch: "main",
      topLevel: "../handshake_main",
    },
    registrySessions: [],
  });

  assert.equal(actorContext.actorRole, "INTEGRATION_VALIDATOR");
  assert.equal(actorContext.source, "WORKTREE_POLICY");
});

test("validator actor context infers wp-validator from the coder worktree lane", () => {
  const wpId = "WP-TEST-VALIDATOR-v1";
  const actorContext = resolveValidatorActorContext({
    repoRoot: ".",
    wpId,
    packetContent: packetFixture(),
    gitContext: {
      branch: defaultWpValidatorBranch(wpId),
      topLevel: defaultWpValidatorWorktreeDir(wpId),
    },
    registrySessions: [],
  });

  assert.equal(actorContext.actorRole, "WP_VALIDATOR");
  assert.equal(actorContext.source, "WORKTREE_POLICY");
});

test("PASS authority blocks wp-validator from owning final authority on orchestrator-managed packets", () => {
  const wpId = "WP-TEST-VALIDATOR-v1";
  const actorContext = resolveValidatorActorContext({
    repoRoot: ".",
    wpId,
    packetContent: packetFixture(),
    gitContext: {
      branch: defaultWpValidatorBranch(wpId),
      topLevel: defaultWpValidatorWorktreeDir(wpId),
    },
    registrySessions: [],
  });
  const evaluation = evaluateValidatorPassAuthority({
    packetContent: packetFixture(),
    actorContext,
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /PASS authority belongs to INTEGRATION_VALIDATOR/i);
});

test("PASS authority blocks worktree-policy fallback when no governed integration-validator session is bound", () => {
  const actorContext = resolveValidatorActorContext({
    repoRoot: ".",
    wpId: "WP-TEST-VALIDATOR-v1",
    packetContent: packetFixture(),
    gitContext: {
      branch: "main",
      topLevel: "../handshake_main",
    },
    registrySessions: [],
  });
  const evaluation = evaluateValidatorPassAuthority({
    packetContent: packetFixture(),
    actorContext,
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /requires a governed INTEGRATION_VALIDATOR session/i);
});

test("PASS authority accepts the integration-validator lane when a governed session is bound", () => {
  const wpId = "WP-TEST-VALIDATOR-v1";
  const actorContext = resolveValidatorActorContext({
    repoRoot: ".",
    wpId,
    packetContent: packetFixture(),
    gitContext: {
      branch: "main",
      topLevel: "../handshake_main",
    },
    registrySessions: [
      {
        wp_id: wpId,
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        session_id: "integration_validator:wp-test-validator-v1",
        session_thread_id: "thread-123",
        local_branch: "main",
        local_worktree_dir: "../handshake_main",
      },
    ],
  });
  const evaluation = evaluateValidatorPassAuthority({
    packetContent: packetFixture(),
    actorContext,
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.authority.technicalAuthority, "INTEGRATION_VALIDATOR");
  assert.equal(actorContext.source, "SESSION_REGISTRY");
});

test("validator actor context matches governed integration-validator sessions from the handshake_main root", () => {
  const wpId = "WP-TEST-VALIDATOR-v1";
  const actorContext = resolveValidatorActorContext({
    repoRoot: "../handshake_main",
    wpId,
    packetContent: packetFixture(),
    gitContext: {
      branch: "main",
      topLevel: "../handshake_main",
    },
    registrySessions: [
      {
        wp_id: wpId,
        role: "INTEGRATION_VALIDATOR",
        session_key: "INTEGRATION_VALIDATOR:WP-TEST-VALIDATOR-v1",
        session_id: "integration_validator:wp-test-validator-v1",
        session_thread_id: "thread-intval",
        local_branch: "main",
        local_worktree_dir: "../handshake_main",
      },
    ],
  });

  assert.equal(actorContext.actorRole, "INTEGRATION_VALIDATOR");
  assert.equal(actorContext.actorSessionId, "integration_validator:wp-test-validator-v1");
  assert.equal(actorContext.actorWorktreeDir, "../handshake_main");
  assert.equal(actorContext.source, "SESSION_REGISTRY");
});

test("validator actor context matches governed wp-validator sessions from the coder worktree root", () => {
  const wpId = "WP-TEST-VALIDATOR-v1";
  const worktreeDir = defaultWpValidatorWorktreeDir(wpId);
  const branch = defaultWpValidatorBranch(wpId);
  const actorContext = resolveValidatorActorContext({
    repoRoot: worktreeDir,
    wpId,
    packetContent: packetFixture(),
    gitContext: {
      branch,
      topLevel: worktreeDir,
    },
    registrySessions: [
      {
        wp_id: wpId,
        role: "WP_VALIDATOR",
        session_key: "WP_VALIDATOR:WP-TEST-VALIDATOR-v1",
        session_id: "wp_validator:wp-test-validator-v1",
        session_thread_id: "thread-wpval",
        local_branch: branch,
        local_worktree_dir: worktreeDir,
      },
    ],
  });

  assert.equal(actorContext.actorRole, "WP_VALIDATOR");
  assert.equal(actorContext.actorSessionId, "wp_validator:wp-test-validator-v1");
  assert.equal(actorContext.actorWorktreeDir, worktreeDir);
  assert.equal(actorContext.source, "SESSION_REGISTRY");
});

test("integration-validator ready commands emphasize final review and verdict health", () => {
  const commands = buildValidatorReadyCommands({
    wpId: "WP-TEST-VALIDATOR-v1",
    actorRole: "INTEGRATION_VALIDATOR",
    actorSessionId: "intval:test",
    postWorkCommand: "just post-work WP-TEST-VALIDATOR-v1",
  });

  assert.deepEqual(commands, [
    "just integration-validator-context-brief WP-TEST-VALIDATOR-v1",
    "just check-notifications WP-TEST-VALIDATOR-v1 INTEGRATION_VALIDATOR",
    "just ack-notifications WP-TEST-VALIDATOR-v1 INTEGRATION_VALIDATOR intval:test",
    "just validator-packet-complete WP-TEST-VALIDATOR-v1",
    "just wp-communication-health-check WP-TEST-VALIDATOR-v1 VERDICT",
    "just validator-handoff-check WP-TEST-VALIDATOR-v1",
    "just integration-validator-closeout-check WP-TEST-VALIDATOR-v1",
  ]);
});

test("wp-validator ready commands surface packet completeness before handoff validation", () => {
  const commands = buildValidatorReadyCommands({
    wpId: "WP-TEST-VALIDATOR-v1",
    actorRole: "WP_VALIDATOR",
    actorSessionId: "wpval:test",
  });

  assert.deepEqual(commands, [
    "just check-notifications WP-TEST-VALIDATOR-v1 WP_VALIDATOR",
    "just ack-notifications WP-TEST-VALIDATOR-v1 WP_VALIDATOR wpval:test",
    "just validator-packet-complete WP-TEST-VALIDATOR-v1",
    "just wp-communication-health-check WP-TEST-VALIDATOR-v1 HANDOFF",
    "just validator-handoff-check WP-TEST-VALIDATOR-v1",
  ]);
});
