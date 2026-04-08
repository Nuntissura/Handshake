import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";
import {
  buildValidatorReadyCommands,
  deriveValidatorResumeState,
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

test("integration-validator lane blocks resume when governance root falls back to handshake_main .GOV", () => {
  const evaluation = evaluateValidatorPacketGovernanceState({
    wpId: "WP-TEST-VALIDATOR-v1",
    packetPath: ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md",
    packetContent: packetFixture({
      packetFormatVersion: "2026-03-29",
      status: "In Progress",
    }),
    currentWpStatus: "Ready for Validator",
    taskBoardStatus: "IN_PROGRESS",
    actorContext: {
      actorRole: "INTEGRATION_VALIDATOR",
      actorBranch: "main",
      actorWorktreeDir: "../handshake_main",
    },
    governanceRootAbs: path.resolve("../handshake_main/.GOV"),
  });

  assert.equal(evaluation.allowValidationResume, false);
  assert.equal(evaluation.terminalReason, "INTEGRATION_VALIDATOR_GOV_ROOT_MISCONFIGURED");
  assert.match(evaluation.message, /HANDSHAKE_GOV_ROOT/i);
});

test("integration-validator lane remains resumable when governance root points at the kernel", () => {
  const evaluation = evaluateValidatorPacketGovernanceState({
    wpId: "WP-TEST-VALIDATOR-v1",
    packetPath: ".GOV/task_packets/WP-TEST-VALIDATOR-v1/packet.md",
    packetContent: packetFixture({
      packetFormatVersion: "2026-03-29",
      status: "In Progress",
    }),
    currentWpStatus: "Ready for Validator",
    taskBoardStatus: "IN_PROGRESS",
    actorContext: {
      actorRole: "INTEGRATION_VALIDATOR",
      actorBranch: "main",
      actorWorktreeDir: "../handshake_main",
    },
    governanceRootAbs: path.resolve(".GOV"),
  });

  assert.equal(evaluation.allowValidationResume, true);
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
    postWorkCommand: "just phase-check HANDOFF WP-TEST-VALIDATOR-v1 CODER",
  });

  assert.deepEqual(commands, [
    "just integration-validator-context-brief WP-TEST-VALIDATOR-v1",
    "just check-notifications WP-TEST-VALIDATOR-v1 INTEGRATION_VALIDATOR",
    "just ack-notifications WP-TEST-VALIDATOR-v1 INTEGRATION_VALIDATOR intval:test",
    "just phase-check CLOSEOUT WP-TEST-VALIDATOR-v1",
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
    "just phase-check HANDOFF WP-TEST-VALIDATOR-v1 WP_VALIDATOR",
  ]);
});

test("wp-validator ready commands switch to intent-checkpoint guidance when checkpoint review is pending", () => {
  const commands = buildValidatorReadyCommands({
    wpId: "WP-TEST-VALIDATOR-v1",
    actorRole: "WP_VALIDATOR",
    actorSessionId: "wpval:test",
    waitingOn: "WP_VALIDATOR_INTENT_CHECKPOINT",
  });

  assert.deepEqual(commands, [
    "just check-notifications WP-TEST-VALIDATOR-v1 WP_VALIDATOR",
    "just ack-notifications WP-TEST-VALIDATOR-v1 WP_VALIDATOR wpval:test",
    "just active-lane-brief WP_VALIDATOR WP-TEST-VALIDATOR-v1",
    "just wp-validator-response WP-TEST-VALIDATOR-v1 WP_VALIDATOR wpval:test <coder-session> \"<summary>\" <correlation_id>",
    "just wp-spec-gap WP-TEST-VALIDATOR-v1 WP_VALIDATOR wpval:test CODER <coder-session> \"<summary>\" [correlation_id] [spec_anchor] [packet_row_ref]",
  ]);
});

test("wp-validator ready commands surface overlap microtask review guidance when parallel review is allowed", () => {
  const commands = buildValidatorReadyCommands({
    wpId: "WP-TEST-VALIDATOR-v1",
    actorRole: "WP_VALIDATOR",
    actorSessionId: "wpval:test",
    parallelReview: true,
  });

  assert.deepEqual(commands, [
    "just check-notifications WP-TEST-VALIDATOR-v1 WP_VALIDATOR",
    "just ack-notifications WP-TEST-VALIDATOR-v1 WP_VALIDATOR wpval:test",
    "just active-lane-brief WP_VALIDATOR WP-TEST-VALIDATOR-v1",
    "just wp-validator-response WP-TEST-VALIDATOR-v1 WP_VALIDATOR wpval:test <coder-session> \"<summary>\" <correlation_id>",
    "just wp-review-exchange VALIDATOR_QUERY WP-TEST-VALIDATOR-v1 WP_VALIDATOR wpval:test CODER <coder-session> \"<summary>\" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for] [microtask_json]",
  ]);
});

test("validator resume state follows projected WP validator review truth", () => {
  const state = deriveValidatorResumeState({
    actorRole: "WP_VALIDATOR",
    communicationState: {
      runtimeStatus: {
        next_expected_actor: "WP_VALIDATOR",
        waiting_on: "WP_VALIDATOR_REVIEW",
      },
      communicationEvaluation: {
        applicable: true,
        state: "COMM_WAITING_FOR_REVIEW",
      },
      latestValidatorAssessment: null,
    },
  });

  assert.equal(state.ready, true);
  assert.equal(state.blockedByRoute, false);
  assert.equal(state.nextExpectedActor, "WP_VALIDATOR");
  assert.equal(state.waitingOn, "WP_VALIDATOR_REVIEW");
  assert.match(state.message, /WP validator review is required now/i);
});

test("validator resume state follows projected WP validator intent checkpoint truth", () => {
  const state = deriveValidatorResumeState({
    actorRole: "WP_VALIDATOR",
    communicationState: {
      runtimeStatus: {
        next_expected_actor: "WP_VALIDATOR",
        waiting_on: "WP_VALIDATOR_INTENT_CHECKPOINT",
      },
      communicationEvaluation: {
        applicable: true,
        state: "COMM_WAITING_FOR_INTENT_CHECKPOINT",
      },
      latestValidatorAssessment: null,
    },
  });

  assert.equal(state.ready, true);
  assert.equal(state.blockedByRoute, false);
  assert.equal(state.nextExpectedActor, "WP_VALIDATOR");
  assert.equal(state.waitingOn, "WP_VALIDATOR_INTENT_CHECKPOINT");
  assert.match(state.message, /checkpoint review is required/i);
});

test("validator resume state reports coder remediation after failed assessment", () => {
  const state = deriveValidatorResumeState({
    actorRole: "WP_VALIDATOR",
    communicationState: {
      runtimeStatus: {
        next_expected_actor: "CODER",
        waiting_on: "CODER_REPAIR_HANDOFF",
      },
      communicationEvaluation: {
        applicable: true,
        state: "COMM_REPAIR_REQUIRED",
      },
      latestValidatorAssessment: {
        verdict: "FAIL",
        receiptKind: "VALIDATOR_REVIEW",
        reason: "Repair required. Findings: fix mailbox projection and re-handoff.",
      },
    },
  });

  assert.equal(state.ready, false);
  assert.equal(state.blockedByRoute, true);
  assert.equal(state.nextExpectedActor, "CODER");
  assert.match(state.message, /Latest validator assessment already recorded FAIL/i);
  assert.match(state.message, /coder remediation is next/i);
});

test("validator resume state exposes parallel overlap review work even when coder remains the routed next actor", () => {
  const state = deriveValidatorResumeState({
    actorRole: "WP_VALIDATOR",
    communicationState: {
      runtimeStatus: {
        next_expected_actor: "CODER",
        waiting_on: "CODER_HANDOFF",
        wp_validator_of_record: "wpv-1",
        open_review_items: [
          {
            correlation_id: "micro-1",
            receipt_kind: "REVIEW_REQUEST",
            summary: "Review the completed microtask while coder continues.",
            opened_by_role: "CODER",
            opened_by_session: "coder-1",
            target_role: "WP_VALIDATOR",
            target_session: "wpv-1",
            microtask_contract: {
              review_mode: "OVERLAP",
              phase_gate: "MICROTASK",
            },
          },
        ],
      },
      communicationEvaluation: {
        applicable: true,
        state: "COMM_WAITING_FOR_HANDOFF",
      },
      latestValidatorAssessment: null,
    },
  });

  assert.equal(state.ready, true);
  assert.equal(state.parallelReviewReady, true);
  assert.equal(state.blockedByRoute, false);
  assert.match(state.message, /Parallel microtask review queue is available/i);
});

test("validator resume state follows projected integration-validator review truth", () => {
  const state = deriveValidatorResumeState({
    actorRole: "INTEGRATION_VALIDATOR",
    communicationState: {
      runtimeStatus: {
        next_expected_actor: "INTEGRATION_VALIDATOR",
        waiting_on: "OPEN_REVIEW_ITEM_REVIEW_REQUEST",
      },
      communicationEvaluation: {
        applicable: true,
        state: "COMM_BLOCKED_OPEN_ITEMS",
      },
      latestValidatorAssessment: null,
    },
  });

  assert.equal(state.ready, true);
  assert.equal(state.blockedByRoute, false);
  assert.equal(state.nextExpectedActor, "INTEGRATION_VALIDATOR");
  assert.match(state.message, /final direct review exchange/i);
});
