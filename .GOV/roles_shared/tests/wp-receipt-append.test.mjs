import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { deriveWpScopeContract } from "../scripts/lib/scope-surface-lib.mjs";
import {
  deriveReviewNotificationTargets,
  summarizeCommittedCoderHandoffDirtyState,
  validateWpReceiptAppendPreconditions,
} from "../scripts/wp/wp-receipt-append.mjs";
import { recordReviewExchange } from "../scripts/wp/wp-review-exchange.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

function writeReviewExchangePacket(packetDir, wpId, commDir) {
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `# Task Packet: ${wpId}`,
      "",
      "**Status:** In Progress",
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      `- WP_RECEIPTS_FILE: ${path.join(commDir, "RECEIPTS.jsonl").replace(/\\/g, "/")}`,
      `- WP_RUNTIME_STATUS_FILE: ${path.join(commDir, "RUNTIME_STATUS.json").replace(/\\/g, "/")}`,
      `- WP_THREAD_FILE: ${path.join(commDir, "THREAD.md").replace(/\\/g, "/")}`,
      "- LOCAL_BRANCH: feat/test-review-exchange",
      `- LOCAL_WORKTREE_DIR: ${path.join(commDir, "worktree").replace(/\\/g, "/")}`,
      "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
      "- PACKET_FORMAT_VERSION: 2026-03-29",
      "- COMMUNICATION_CONTRACT: v1",
      "- COMMUNICATION_HEALTH_GATE: REQUIRED",
    ].join("\n"),
    "utf8",
  );
}

function writeIntentCheckpointPacket(packetDir, wpId, commDir) {
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `# Task Packet: ${wpId}`,
      "",
      "**Status:** In Progress",
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      `- WP_RECEIPTS_FILE: ${path.join(commDir, "RECEIPTS.jsonl").replace(/\\/g, "/")}`,
      `- WP_RUNTIME_STATUS_FILE: ${path.join(commDir, "RUNTIME_STATUS.json").replace(/\\/g, "/")}`,
      `- WP_THREAD_FILE: ${path.join(commDir, "THREAD.md").replace(/\\/g, "/")}`,
      "- LOCAL_BRANCH: feat/test-intent-checkpoint",
      `- LOCAL_WORKTREE_DIR: ${path.join(commDir, "worktree").replace(/\\/g, "/")}`,
      "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
      "- PACKET_FORMAT_VERSION: 2026-03-29",
      "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1",
      "- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING",
      "- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3",
      "- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2",
      "- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1",
      "- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1",
      "- IN_SCOPE_PATHS:",
      "  - src/demo.rs",
      "",
      "## CLAUSE_CLOSURE_MATRIX",
      "- CLAUSE | CODER_STATUS=PROVED | VALIDATOR_STATUS=PENDING",
    ].join("\n"),
    "utf8",
  );
}

test("validator assessment receipts add an orchestrator governance checkpoint in orchestrator-managed lanes", () => {
  const targets = deriveReviewNotificationTargets({
    workflowLane: "ORCHESTRATOR_MANAGED",
    entry: {
      receipt_kind: "VALIDATOR_REVIEW",
      actor_role: "WP_VALIDATOR",
      actor_session: "wpv-1",
      target_role: "CODER",
      target_session: "coder-1",
      summary: "Repair required. Findings: fix mailbox projection and re-handoff.",
    },
    autoRoute: {
      nextExpectedActor: "CODER",
      notification: {
        targetRole: "CODER",
        targetSession: "coder-1",
        summary: "AUTO_ROUTE: WP validator review requires coder remediation before re-handoff",
      },
    },
  });

  assert.equal(targets.length, 2);
  assert.deepEqual(targets[0], {
    targetRole: "CODER",
    targetSession: "coder-1",
    sourceKind: "VALIDATOR_REVIEW",
    summary: "VALIDATOR_REVIEW: Repair required. Findings: fix mailbox projection and re-handoff.",
  });
  assert.equal(targets[1].targetRole, "ORCHESTRATOR");
  assert.equal(targets[1].targetSession, null);
  assert.equal(targets[1].sourceKind, "GOVERNANCE_CHECKPOINT");
  assert.match(targets[1].summary, /result=FAIL/i);
  assert.match(targets[1].summary, /why=Repair required/i);
  assert.match(targets[1].summary, /verify governance truth and ACP steering/i);
  assert.match(targets[1].summary, /projected_next_actor=CODER/i);
});

test("orchestrator checkpoint does not duplicate an auto-route that already targets orchestrator", () => {
  const targets = deriveReviewNotificationTargets({
    workflowLane: "ORCHESTRATOR_MANAGED",
    entry: {
      receipt_kind: "REVIEW_RESPONSE",
      actor_role: "INTEGRATION_VALIDATOR",
      actor_session: "intval-1",
      target_role: "CODER",
      target_session: "coder-1",
      summary: "Suitable for integration review closure.",
    },
    autoRoute: {
      nextExpectedActor: "ORCHESTRATOR",
      notification: {
        targetRole: "ORCHESTRATOR",
        targetSession: null,
        summary: "AUTO_ROUTE: direct review lane complete; orchestrator verdict progression ready",
      },
    },
  });

  assert.equal(targets.filter((entry) => entry.targetRole === "ORCHESTRATOR").length, 1);
  assert.equal(targets[1].sourceKind, "AUTO_ROUTE");
});

test("non-assessment receipts do not add orchestrator checkpoint notifications", () => {
  const targets = deriveReviewNotificationTargets({
    workflowLane: "ORCHESTRATOR_MANAGED",
    entry: {
      receipt_kind: "CODER_HANDOFF",
      actor_role: "CODER",
      actor_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-1",
      summary: "Implemented the requested scope and attached proof.",
    },
    autoRoute: {
      nextExpectedActor: "WP_VALIDATOR",
      notification: {
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        summary: "AUTO_ROUTE: WP validator review required after coder handoff",
      },
    },
  });

  assert.deepEqual(targets, [
    {
      targetRole: "WP_VALIDATOR",
      targetSession: "wpv-1",
      sourceKind: "CODER_HANDOFF",
      summary: "CODER_HANDOFF: Implemented the requested scope and attached proof.",
    },
  ]);
});

test("committed coder handoff dirty-state summary ignores governance-only drift and transient proof logs but blocks product dirt", () => {
  const scopeContract = deriveWpScopeContract({
    wpId: "WP-TEST-HANDOFF-v1",
    packetContent: `# Task Packet: WP-TEST-HANDOFF-v1

**Status:** In Progress

## METADATA
- WP_ID: WP-TEST-HANDOFF-v1
- PACKET_FORMAT_VERSION: 2026-03-29
- IN_SCOPE_PATHS:
  - src/demo.rs
`.trim(),
  });
  const summary = summarizeCommittedCoderHandoffDirtyState([
    " M .GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md",
    " M .GOV/roles_shared/records/TASK_BOARD.md",
    "?? tmp-test-proof.log",
    " M src/demo.rs",
  ].join("\n"), scopeContract);

  assert.equal(summary.ok, false);
  assert.deepEqual(summary.governanceNoisePaths, [
    ".GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md",
    ".GOV/roles_shared/records/TASK_BOARD.md",
  ]);
  assert.deepEqual(summary.transientArtifactPaths, ["tmp-test-proof.log"]);
  assert.deepEqual(summary.blockingPaths, ["src/demo.rs (IN_SCOPE)"]);
});

test("review exchange preflight blocks invalid direct-review receipts before thread append", () => {
  const wpId = "WP-TEST-REVIEW-EXCHANGE-PREFLIGHT";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-review-exchange-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");

  fs.writeFileSync(receiptsPath, "", "utf8");
  writeReviewExchangePacket(packetDir, wpId, commDir);

  try {
    assert.throws(
      () => recordReviewExchange({
        receiptKind: "REVIEW_REQUEST",
        wpId,
        actorRole: "CODER",
        actorSession: "coder-test",
        targetRole: "INTEGRATION_VALIDATOR",
        targetSession: null,
        summary: "Requesting final integration review.",
        correlationId: "review-request-test",
      }),
      /target_session is required for REVIEW_REQUEST/,
    );

    assert.equal(fs.existsSync(threadPath), false);
    assert.equal(fs.readFileSync(receiptsPath, "utf8"), "");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("review exchange preflight rejects placeholder unassigned target sessions", () => {
  const wpId = "WP-TEST-REVIEW-EXCHANGE-UNASSIGNED";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-review-exchange-unassigned-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");

  fs.writeFileSync(receiptsPath, "", "utf8");
  writeReviewExchangePacket(packetDir, wpId, commDir);

  try {
    assert.throws(
      () => recordReviewExchange({
        receiptKind: "REVIEW_REQUEST",
        wpId,
        actorRole: "CODER",
        actorSession: "coder-test",
        targetRole: "INTEGRATION_VALIDATOR",
        targetSession: "<unassigned>",
        summary: "Requesting final integration review.",
        correlationId: "review-request-test",
      }),
      /target_session is required for REVIEW_REQUEST/,
    );

    assert.equal(fs.existsSync(threadPath), false);
    assert.equal(fs.readFileSync(receiptsPath, "utf8"), "");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("validator review preflight suppresses duplicate decisive approvals for the same handoff round", () => {
  const wpId = "WP-TEST-DUPLICATE-VALIDATOR-APPROVAL";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-duplicate-validator-approval-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");

  writeReviewExchangePacket(packetDir, wpId, commDir);
  fs.writeFileSync(
    receiptsPath,
    [
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-01T10:00:00Z",
        wp_id: wpId,
        actor_role: "CODER",
        actor_session: "coder-1",
        actor_authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        receipt_kind: "CODER_HANDOFF",
        summary: "Ready for WP validator review.",
        branch: "feat/test-review-exchange",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "handoff-1",
        requires_ack: true,
        ack_for: null,
        spec_anchor: null,
        packet_row_ref: null,
        refs: [],
      }),
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-01T10:01:00Z",
        wp_id: wpId,
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        actor_authority_kind: "WP_VALIDATOR",
        validator_role_kind: "WP_VALIDATOR",
        receipt_kind: "VALIDATOR_REVIEW",
        summary: "Approved for final review. Suitable for integration review closure.",
        branch: "feat/test-review-exchange",
        worktree_dir: "../wtv-test",
        state_before: null,
        state_after: null,
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "handoff-1",
        requires_ack: false,
        ack_for: "handoff-1",
        spec_anchor: null,
        packet_row_ref: null,
        refs: [],
      }),
    ].join("\n"),
    "utf8",
  );

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "WP_VALIDATOR",
        actorSession: "wpv-1",
        receiptKind: "VALIDATOR_REVIEW",
        summary: "Approved for final review. Suitable for integration review closure.",
        targetRole: "CODER",
        targetSession: "coder-1",
        correlationId: "handoff-1",
        ackFor: "handoff-1",
      }),
      /Duplicate decisive validator outcome suppressed/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("coder handoff preflight rejects missing validator intent checkpoint on contract-heavy packets", () => {
  const wpId = "WP-TEST-INTENT-CHECKPOINT";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-intent-checkpoint-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  writeIntentCheckpointPacket(packetDir, wpId, commDir);
  fs.writeFileSync(
    receiptsPath,
    [
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-01T10:00:00Z",
        wp_id: wpId,
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        actor_authority_kind: "WP_VALIDATOR",
        validator_role_kind: "WP_VALIDATOR",
        receipt_kind: "VALIDATOR_KICKOFF",
        summary: "Kickoff",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "kickoff-1",
        requires_ack: true,
        ack_for: null,
        spec_anchor: null,
        packet_row_ref: null,
        refs: [],
      }),
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-01T10:01:00Z",
        wp_id: wpId,
        actor_role: "CODER",
        actor_session: "coder-1",
        actor_authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        receipt_kind: "CODER_INTENT",
        summary: "Implementation order drafted.",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "kickoff-1",
        requires_ack: false,
        ack_for: "kickoff-1",
        spec_anchor: null,
        packet_row_ref: null,
        refs: [],
      }),
    ].join("\n"),
    "utf8",
  );
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      workflow_lane: "ORCHESTRATOR_MANAGED",
      wp_validator_of_record: "wpv-1",
      integration_validator_of_record: "intval-1",
      active_role_sessions: [
        {
          role: "CODER",
          session_id: "coder-1",
          worktree_dir: "../wtc-test",
          state: "working",
          last_heartbeat_at: "2026-04-01T10:01:00Z",
        },
        {
          role: "WP_VALIDATOR",
          session_id: "wpv-1",
          worktree_dir: "../wtv-test",
          state: "waiting",
          last_heartbeat_at: "2026-04-01T10:01:00Z",
        },
      ],
      open_review_items: [],
      next_expected_actor: "WP_VALIDATOR",
      next_expected_session: "wpv-1",
      waiting_on: "WP_VALIDATOR_INTENT_CHECKPOINT",
      waiting_on_session: "wpv-1",
      validator_trigger: "BLOCKED_NEEDS_VALIDATOR",
      validator_trigger_reason: "Coder intent recorded; WP validator checkpoint review is required before full handoff",
      ready_for_validation: true,
      ready_for_validation_reason: "Coder intent recorded; WP validator checkpoint review is required before full handoff",
      attention_required: false,
    }, null, 2),
    "utf8",
  );

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "CODER",
        actorSession: "coder-1",
        receiptKind: "CODER_HANDOFF",
        summary: "Ready for review.",
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        correlationId: "handoff-1",
        requiresAck: true,
      }, {
        skipCommittedCoderHandoffGate: true,
      }),
      /checkpoint review of CODER_INTENT is still required/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("coder handoff preflight rejects unresolved overlap microtask reviews before full handoff", () => {
  const wpId = "WP-TEST-OVERLAP-HANDOFF";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-overlap-handoff-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  writeIntentCheckpointPacket(packetDir, wpId, commDir);
  fs.writeFileSync(receiptsPath, "", "utf8");
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      workflow_lane: "ORCHESTRATOR_MANAGED",
      wp_validator_of_record: "wpv-1",
      integration_validator_of_record: "intval-1",
      active_role_sessions: [
        {
          role: "CODER",
          session_id: "coder-1",
          worktree_dir: "../wtc-test",
          state: "working",
          last_heartbeat_at: "2026-04-01T10:01:00Z",
        },
        {
          role: "WP_VALIDATOR",
          session_id: "wpv-1",
          worktree_dir: "../wtv-test",
          state: "waiting",
          last_heartbeat_at: "2026-04-01T10:01:00Z",
        },
      ],
      open_review_items: [
        {
          correlation_id: "micro-1",
          receipt_kind: "REVIEW_REQUEST",
          summary: "Review completed microtask 1 while coder continues microtask 2.",
          opened_by_role: "CODER",
          opened_by_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          spec_anchor: "CX-MICRO-001",
          packet_row_ref: "CLAUSE_CLOSURE_MATRIX",
          microtask_contract: {
            scope_ref: "CLAUSE_CLOSURE_MATRIX/CX-MICRO-001",
            file_targets: ["src/demo.rs"],
            proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
            review_mode: "OVERLAP",
            phase_gate: "MICROTASK",
            expected_receipt_kind: "VALIDATOR_RESPONSE",
          },
          requires_ack: true,
          opened_at: "2026-04-01T10:02:00Z",
          updated_at: "2026-04-01T10:02:00Z",
        },
      ],
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
      waiting_on: "CODER_HANDOFF",
      waiting_on_session: "coder-1",
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      attention_required: false,
    }, null, 2),
    "utf8",
  );

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "CODER",
        actorSession: "coder-1",
        receiptKind: "CODER_HANDOFF",
        summary: "Ready for full review.",
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        correlationId: "handoff-1",
        requiresAck: true,
      }, {
        skipCommittedCoderHandoffGate: true,
      }),
      /pending overlap microtask reviews must be resolved/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("overlap review request preflight rejects queue growth beyond the bounded backlog", () => {
  const wpId = "WP-TEST-OVERLAP-BACKPRESSURE";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-overlap-backpressure-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  writeIntentCheckpointPacket(packetDir, wpId, commDir);
  fs.writeFileSync(receiptsPath, "", "utf8");
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      workflow_lane: "ORCHESTRATOR_MANAGED",
      wp_validator_of_record: "wpv-1",
      integration_validator_of_record: "intval-1",
      active_role_sessions: [
        {
          role: "CODER",
          session_id: "coder-1",
          worktree_dir: "../wtc-test",
          state: "working",
          last_heartbeat_at: "2026-04-01T10:01:00Z",
        },
        {
          role: "WP_VALIDATOR",
          session_id: "wpv-1",
          worktree_dir: "../wtv-test",
          state: "waiting",
          last_heartbeat_at: "2026-04-01T10:01:00Z",
        },
      ],
      open_review_items: ["micro-1", "micro-2"].map((id, index) => ({
        correlation_id: id,
        receipt_kind: "REVIEW_REQUEST",
        summary: `Review ${id}`,
        opened_by_role: "CODER",
        opened_by_session: "coder-1",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        spec_anchor: `CX-MICRO-00${index + 1}`,
        packet_row_ref: "CLAUSE_CLOSURE_MATRIX",
        microtask_contract: {
          scope_ref: `CLAUSE_CLOSURE_MATRIX/CX-MICRO-00${index + 1}`,
          file_targets: ["src/demo.rs"],
          proof_commands: [`cargo test demo::tests::micro_${index + 1} -- --exact`],
          review_mode: "OVERLAP",
          phase_gate: "MICROTASK",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        requires_ack: true,
        opened_at: `2026-04-01T10:0${index + 1}:00Z`,
        updated_at: `2026-04-01T10:0${index + 1}:00Z`,
      })),
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
      waiting_on: "CODER_HANDOFF",
      waiting_on_session: "coder-1",
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      attention_required: false,
    }, null, 2),
    "utf8",
  );

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "CODER",
        actorSession: "coder-1",
        receiptKind: "REVIEW_REQUEST",
        summary: "Review completed microtask 3 while I continue microtask 4.",
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        correlationId: "micro-3",
        requiresAck: true,
        microtaskContract: {
          scope_ref: "CLAUSE_CLOSURE_MATRIX/CX-MICRO-003",
          file_targets: ["src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::micro_3 -- --exact"],
          review_mode: "OVERLAP",
          phase_gate: "MICROTASK",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
      }),
      /overlap microtask review backlog already reached 2/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});
