import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { deriveWpScopeContract } from "../scripts/lib/scope-surface-lib.mjs";
import {
  applyWorkflowInvalidityRuntimeProjection,
  buildGovernedPhaseCheckInvocation,
  deriveReviewNotificationTargets,
  summarizeCommittedCoderHandoffDirtyState,
  validateWpReceiptAppendPreconditions,
} from "../scripts/wp/wp-receipt-append.mjs";
import {
  recordReviewExchange,
  requiresSplitCommittedCoderHandoffValidation,
} from "../scripts/wp/wp-review-exchange.mjs";

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

function writeMicrotaskCheckpointPacket(packetDir, wpId, commDir, microtasks = []) {
  writeIntentCheckpointPacket(packetDir, wpId, commDir);
  for (const microtask of microtasks) {
    fs.writeFileSync(
      path.join(packetDir, `${microtask.mtId}.md`),
      [
        `# ${microtask.mtId}: ${microtask.clause}`,
        "",
        "## METADATA",
        `- WP_ID: ${wpId}`,
        `- MT_ID: ${microtask.mtId}`,
        `- CLAUSE: ${microtask.clause}`,
        `- CODE_SURFACES: ${microtask.codeSurfaces.join("; ")}`,
        `- EXPECTED_TESTS: ${microtask.expectedTests.join("; ")}`,
        `- DEPENDS_ON: ${microtask.dependsOn || "NONE"}`,
        "- RISK_IF_MISSED: demo regression slips through",
        "",
        "## CODER",
        "- STATUS: PENDING",
        "- EVIDENCE:",
        "- TESTS_RUN:",
        "- NOTES:",
        "",
        "## VALIDATOR",
        "- STATUS: PENDING",
        "- FINDINGS:",
        "- DIRECTION:",
      ].join("\n"),
      "utf8",
    );
  }
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
    autoRelay: false,
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
      autoRelay: false,
    },
  ]);
});

test("overlap auto-route adds a secondary validator wake while coder remains the routed actor", () => {
  const targets = deriveReviewNotificationTargets({
    workflowLane: "ORCHESTRATOR_MANAGED",
    entry: {
      receipt_kind: "REVIEW_REQUEST",
      actor_role: "CODER",
      actor_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-1",
      correlation_id: "review-1",
      summary: "Review MT-001 while I continue MT-002.",
    },
    autoRoute: {
      nextExpectedActor: "CODER",
      nextExpectedSession: "coder-1",
      notification: null,
      secondaryNotifications: [
        {
          targetRole: "WP_VALIDATOR",
          targetSession: "wpv-1",
          sourceKind: "AUTO_ROUTE",
          summary: "AUTO_ROUTE: WP validator overlap review required while coder continues current microtask",
          autoRelay: true,
        },
      ],
    },
  });

  assert.deepEqual(targets, [
    {
      targetRole: "WP_VALIDATOR",
      targetSession: "wpv-1",
      sourceKind: "REVIEW_REQUEST",
      summary: "REVIEW_REQUEST: Review MT-001 while I continue MT-002.",
      autoRelay: true,
    },
  ]);
});

test("committed coder handoff preflight uses the shared phase-check script instead of a justfile alias", () => {
  const invocation = buildGovernedPhaseCheckInvocation({
    phase: "STARTUP",
    wpId: "WP-TEST-HANDOFF-v1",
    role: "CODER",
    args: ["--committed-handoff-preflight"],
  });

  assert.equal(invocation.command, process.execPath);
  assert.match(
    invocation.args[0].replace(/\\/g, "/"),
    /\/\.GOV\/roles_shared\/checks\/phase-check\.mjs$/,
  );
  assert.deepEqual(invocation.args.slice(1), ["STARTUP", "WP-TEST-HANDOFF-v1", "CODER", "--committed-handoff-preflight"]);
});

test("workflow invalidity runtime projection keeps runtime route generic while receipts carry the code", () => {
  const runtimeStatus = {
    runtime_status: "working",
    next_expected_actor: "CODER",
    next_expected_session: "coder-1",
    waiting_on: "CODER_HANDOFF",
    waiting_on_session: "coder-1",
    validator_trigger: "HANDOFF_READY",
    validator_trigger_reason: "Waiting on coder handoff",
    attention_required: false,
    ready_for_validation: true,
    ready_for_validation_reason: "handoff complete",
  };

  const projected = applyWorkflowInvalidityRuntimeProjection(runtimeStatus, {
    receipt_kind: "WORKFLOW_INVALIDITY",
    workflow_invalidity_code: "PHASE_CHECK_RECIPE_MISSING",
  });

  assert.equal(projected.next_expected_actor, "ORCHESTRATOR");
  assert.equal(projected.next_expected_session, null);
  assert.equal(projected.waiting_on, "WORKFLOW_INVALIDITY");
  assert.equal(projected.waiting_on_session, null);
  assert.equal(projected.validator_trigger, "NONE");
  assert.equal(projected.validator_trigger_reason, "Workflow invalidity flagged");
  assert.equal(projected.attention_required, true);
  assert.equal(projected.ready_for_validation, false);
  assert.equal(projected.ready_for_validation_reason, null);
  assert.equal(projected.runtime_status, "input_required");
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

test("committed coder handoff dirty-state summary tolerates pre-existing out-of-scope dirt in committed-range mode", () => {
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
    " M src/ambient.rs",
    "?? tmp-test-proof.log",
  ].join("\n"), scopeContract, { allowAmbientOutOfScope: true });

  assert.equal(summary.ok, true);
  assert.deepEqual(summary.governanceNoisePaths, [
    ".GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md",
  ]);
  assert.deepEqual(summary.transientArtifactPaths, ["tmp-test-proof.log"]);
  assert.deepEqual(summary.ambientOutOfScopePaths, ["src/ambient.rs (PRODUCT_OUT_OF_SCOPE)"]);
  assert.deepEqual(summary.blockingPaths, []);
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

test("review exchange splits the committed coder handoff preflight out of the transaction lock only for coder handoffs", () => {
  assert.equal(
    requiresSplitCommittedCoderHandoffValidation({
      receiptKind: "CODER_HANDOFF",
      actorRole: "CODER",
    }),
    true,
  );
  assert.equal(
    requiresSplitCommittedCoderHandoffValidation({
      receiptKind: "CODER_HANDOFF",
      actorRole: "WP_VALIDATOR",
    }),
    false,
  );
  assert.equal(
    requiresSplitCommittedCoderHandoffValidation({
      receiptKind: "REVIEW_REQUEST",
      actorRole: "CODER",
    }),
    false,
  );
});

test("review exchange preflight rejects resolution receipts that do not answer an existing open correlation", () => {
  const wpId = "WP-TEST-REVIEW-EXCHANGE-CORRELATION-MISMATCH";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-review-exchange-correlation-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  writeReviewExchangePacket(packetDir, wpId, commDir);
  fs.writeFileSync(
    receiptsPath,
    `${JSON.stringify({
      schema_version: "wp_receipt@1",
      timestamp_utc: "2026-04-08T23:02:03.797Z",
      wp_id: wpId,
      actor_role: "WP_VALIDATOR",
      actor_session: "wpv-1",
      actor_authority_kind: "WP_VALIDATOR",
      validator_role_kind: "WP_VALIDATOR",
      receipt_kind: "VALIDATOR_KICKOFF",
      summary: "Kickoff recorded.",
      branch: "feat/test-review-exchange",
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
      workflow_invalidity_code: null,
      refs: [],
    })}\n`,
    "utf8",
  );
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      workflow_lane: "ORCHESTRATOR_MANAGED",
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
      waiting_on: "CODER_INTENT",
      waiting_on_session: null,
      runtime_status: "submitted",
      current_phase: "BOOTSTRAP",
      open_review_items: [],
    }, null, 2),
    "utf8",
  );

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "CODER",
        actorSession: "coder-1",
        receiptKind: "CODER_INTENT",
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        summary: "Intent recorded with the wrong reply correlation.",
        correlationId: "intent-1",
        ackFor: "intent-1",
        microtaskContract: {
          scope_ref: "MT-001",
          file_targets: ["src/demo.rs"],
          proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
          phase_gate: "BOOTSTRAP",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
      }),
      /must reference an existing open review receipt/i,
    );

    assert.equal(fs.readFileSync(receiptsPath, "utf8").trim().split(/\r?\n/).length, 1);
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

test("validator response preflight accepts intent checkpoint clearance after coder intent", () => {
  const wpId = "WP-TEST-INTENT-CHECKPOINT-CLEAR";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-intent-checkpoint-clear-"));
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
    assert.doesNotThrow(() => validateWpReceiptAppendPreconditions({
      wpId,
      actorRole: "WP_VALIDATOR",
      actorSession: "wpv-1",
      receiptKind: "VALIDATOR_RESPONSE",
      summary: "Bootstrap checkpoint cleared.",
      targetRole: "CODER",
      targetSession: "coder-1",
      correlationId: "kickoff-1",
      ackFor: "kickoff-1",
      specAnchor: "MT-001",
      packetRowRef: "MT-001",
    }));
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("validator response preflight accepts intent checkpoint clearance after spec confirmation", () => {
  const wpId = "WP-TEST-INTENT-CHECKPOINT-REPAIR";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-intent-checkpoint-repair-"));
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
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-01T10:02:00Z",
        wp_id: wpId,
        actor_role: "WP_VALIDATOR",
        actor_session: "wpv-1",
        actor_authority_kind: "WP_VALIDATOR",
        validator_role_kind: "WP_VALIDATOR",
        receipt_kind: "SPEC_GAP",
        summary: "Need lifecycle family first.",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "CODER",
        target_session: "coder-1",
        correlation_id: "spec-gap-1",
        requires_ack: true,
        ack_for: null,
        spec_anchor: "MT-001",
        packet_row_ref: "MT-001",
        refs: [],
      }),
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-01T10:03:00Z",
        wp_id: wpId,
        actor_role: "CODER",
        actor_session: "coder-1",
        actor_authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        receipt_kind: "SPEC_CONFIRMATION",
        summary: "Revised to lifecycle family first.",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "spec-gap-1",
        requires_ack: false,
        ack_for: "spec-gap-1",
        spec_anchor: "MT-001",
        packet_row_ref: "MT-001",
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
          last_heartbeat_at: "2026-04-01T10:03:00Z",
        },
        {
          role: "WP_VALIDATOR",
          session_id: "wpv-1",
          worktree_dir: "../wtv-test",
          state: "waiting",
          last_heartbeat_at: "2026-04-01T10:03:00Z",
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
    assert.doesNotThrow(() => validateWpReceiptAppendPreconditions({
      wpId,
      actorRole: "WP_VALIDATOR",
      actorSession: "wpv-1",
      receiptKind: "VALIDATOR_RESPONSE",
      summary: "Bootstrap checkpoint cleared after repair loop.",
      targetRole: "CODER",
      targetSession: "coder-1",
      correlationId: "spec-gap-1",
      ackFor: "spec-gap-1",
      specAnchor: "MT-001",
      packetRowRef: "MT-001",
    }));
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
      open_review_items: ["micro-1"].map((id, index) => ({
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
      /overlap microtask review backlog already reached 1/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("coder intent preflight requires a declared microtask contract when MT packets exist", () => {
  const wpId = "WP-TEST-MICROTASK-CONTRACT-REQUIRED";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-microtask-contract-required-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");

  writeMicrotaskCheckpointPacket(packetDir, wpId, commDir, [{
    mtId: "MT-001",
    clause: "Demo microtask [CX-MICRO-001]",
    codeSurfaces: ["src/demo.rs", "src/demo_support.rs"],
    expectedTests: ["cargo test demo::tests::micro_1 -- --exact"],
  }]);
  fs.writeFileSync(receiptsPath, "", "utf8");

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "CODER",
        actorSession: "coder-1",
        receiptKind: "CODER_INTENT",
        summary: "Starting MT-001.",
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        correlationId: "intent-1",
        ackFor: "intent-1",
      }),
      /declared microtask contract is required/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("coder intent preflight rejects file targets outside the declared microtask budget", () => {
  const wpId = "WP-TEST-MICROTASK-FILE-BUDGET";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-microtask-file-budget-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");

  writeMicrotaskCheckpointPacket(packetDir, wpId, commDir, [{
    mtId: "MT-001",
    clause: "Demo microtask [CX-MICRO-001]",
    codeSurfaces: ["src/demo.rs", "src/demo_support.rs"],
    expectedTests: ["cargo test demo::tests::micro_1 -- --exact"],
  }]);
  fs.writeFileSync(receiptsPath, "", "utf8");

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "CODER",
        actorSession: "coder-1",
        receiptKind: "CODER_INTENT",
        summary: "Starting MT-001.",
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        correlationId: "intent-1",
        ackFor: "intent-1",
        microtaskContract: {
          scope_ref: "MT-001",
          file_targets: ["src/out_of_budget.rs"],
          proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
          phase_gate: "MICROTASK",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
      }),
      /file_targets escape MT-001 CODE_SURFACES/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("coder review request preflight accepts clause-token scope refs inside the declared microtask budget", () => {
  const wpId = "WP-TEST-MICROTASK-SCOPE-ALIAS";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-microtask-scope-alias-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");

  writeMicrotaskCheckpointPacket(packetDir, wpId, commDir, [{
    mtId: "MT-001",
    clause: "Demo microtask [CX-MICRO-001]",
    codeSurfaces: ["src/demo.rs", "src/demo_support.rs"],
    expectedTests: ["cargo test demo::tests::micro_1 -- --exact"],
  }]);
  fs.writeFileSync(receiptsPath, "", "utf8");

  try {
    assert.doesNotThrow(() => validateWpReceiptAppendPreconditions({
      wpId,
      actorRole: "CODER",
      actorSession: "coder-1",
      receiptKind: "REVIEW_REQUEST",
      summary: "Review MT-001 while I continue the next slice.",
      targetRole: "WP_VALIDATOR",
      targetSession: "wpv-1",
      correlationId: "review-1",
      requiresAck: true,
      microtaskContract: {
        scope_ref: "CLAUSE_CLOSURE_MATRIX/CX-MICRO-001",
        file_targets: ["src/demo.rs"],
        proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
        review_mode: "OVERLAP",
        phase_gate: "MICROTASK",
        expected_receipt_kind: "REVIEW_RESPONSE",
      },
    }));
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("coder intent preflight rejects out-of-sequence microtask jumps", () => {
  const wpId = "WP-TEST-MICROTASK-OUT-OF-SEQUENCE";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-microtask-out-of-sequence-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");

  writeMicrotaskCheckpointPacket(packetDir, wpId, commDir, [
    {
      mtId: "MT-001",
      clause: "Demo microtask [CX-MICRO-001]",
      codeSurfaces: ["src/demo.rs"],
      expectedTests: ["cargo test demo::tests::micro_1 -- --exact"],
    },
    {
      mtId: "MT-002",
      clause: "Demo microtask [CX-MICRO-002]",
      codeSurfaces: ["src/demo_support.rs"],
      expectedTests: ["cargo test demo::tests::micro_2 -- --exact"],
    },
    {
      mtId: "MT-003",
      clause: "Demo microtask [CX-MICRO-003]",
      codeSurfaces: ["src/demo_tail.rs"],
      expectedTests: ["cargo test demo::tests::micro_3 -- --exact"],
    },
  ]);
  fs.writeFileSync(receiptsPath, "", "utf8");

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "CODER",
        actorSession: "coder-1",
        receiptKind: "CODER_INTENT",
        summary: "Attempting to skip to MT-003.",
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        correlationId: "intent-3",
        ackFor: "kickoff-1",
        microtaskContract: {
          scope_ref: "MT-003",
          file_targets: ["src/demo_tail.rs"],
          proof_commands: ["cargo test demo::tests::micro_3 -- --exact"],
          phase_gate: "MICROTASK",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
      }),
      /active execution budget is MT-001, not MT-003/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("overlap review request preflight rejects targeting a non-active microtask", () => {
  const wpId = "WP-TEST-MICROTASK-OVERLAP-SEQUENCE";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-microtask-overlap-sequence-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");

  writeMicrotaskCheckpointPacket(packetDir, wpId, commDir, [
    {
      mtId: "MT-001",
      clause: "Demo microtask [CX-MICRO-001]",
      codeSurfaces: ["src/demo.rs"],
      expectedTests: ["cargo test demo::tests::micro_1 -- --exact"],
    },
    {
      mtId: "MT-002",
      clause: "Demo microtask [CX-MICRO-002]",
      codeSurfaces: ["src/demo_support.rs"],
      expectedTests: ["cargo test demo::tests::micro_2 -- --exact"],
    },
  ]);
  fs.writeFileSync(
    receiptsPath,
    `${JSON.stringify({
      schema_version: "wp_receipt@1",
      timestamp_utc: "2026-04-05T10:00:00Z",
      wp_id: wpId,
      actor_role: "CODER",
      actor_session: "coder-1",
      actor_authority_kind: "PRIMARY_CODER",
      validator_role_kind: null,
      receipt_kind: "CODER_INTENT",
      summary: "Starting MT-001.",
      branch: "feat/test-intent-checkpoint",
      worktree_dir: "../wtc-test",
      state_before: null,
      state_after: null,
      target_role: "WP_VALIDATOR",
      target_session: "wpv-1",
      correlation_id: "intent-1",
      requires_ack: false,
      ack_for: "kickoff-1",
      spec_anchor: null,
      packet_row_ref: null,
      microtask_contract: {
        scope_ref: "MT-001",
        file_targets: ["src/demo.rs"],
        proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
        phase_gate: "MICROTASK",
        expected_receipt_kind: "VALIDATOR_RESPONSE",
      },
      refs: [],
    })}\n`,
    "utf8",
  );

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "CODER",
        actorSession: "coder-1",
        receiptKind: "REVIEW_REQUEST",
        summary: "Attempting overlap review on MT-002 before MT-001 completes.",
        targetRole: "WP_VALIDATOR",
        targetSession: "wpv-1",
        correlationId: "review-2",
        requiresAck: true,
        microtaskContract: {
          scope_ref: "MT-002",
          file_targets: ["src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::micro_2 -- --exact"],
          review_mode: "OVERLAP",
          phase_gate: "MICROTASK",
          expected_receipt_kind: "REVIEW_RESPONSE",
        },
      }),
      /overlap review must bind to the current active microtask MT-001, not MT-002/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("coder intent preflight allows advancing to the next microtask after overlap review opens", () => {
  const wpId = "WP-TEST-MICROTASK-OVERLAP-ADVANCE";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-microtask-overlap-advance-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  writeMicrotaskCheckpointPacket(packetDir, wpId, commDir, [
    {
      mtId: "MT-001",
      clause: "Demo microtask [CX-MICRO-001]",
      codeSurfaces: ["src/demo.rs"],
      expectedTests: ["cargo test demo::tests::micro_1 -- --exact"],
    },
    {
      mtId: "MT-002",
      clause: "Demo microtask [CX-MICRO-002]",
      codeSurfaces: ["src/demo_support.rs"],
      expectedTests: ["cargo test demo::tests::micro_2 -- --exact"],
    },
  ]);
  fs.writeFileSync(
    receiptsPath,
    [
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-05T10:00:00Z",
        wp_id: wpId,
        actor_role: "CODER",
        actor_session: "coder-1",
        actor_authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        receipt_kind: "CODER_INTENT",
        summary: "Starting MT-001.",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "intent-1",
        requires_ack: false,
        ack_for: "kickoff-1",
        spec_anchor: null,
        packet_row_ref: null,
        microtask_contract: {
          scope_ref: "MT-001",
          file_targets: ["src/demo.rs"],
          proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
          phase_gate: "MICROTASK",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        refs: [],
      }),
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-05T10:05:00Z",
        wp_id: wpId,
        actor_role: "CODER",
        actor_session: "coder-1",
        actor_authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        receipt_kind: "REVIEW_REQUEST",
        summary: "Review MT-001 while I continue MT-002.",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "review-1",
        requires_ack: true,
        ack_for: null,
        spec_anchor: null,
        packet_row_ref: null,
        microtask_contract: {
          scope_ref: "MT-001",
          file_targets: ["src/demo.rs"],
          proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
          review_mode: "OVERLAP",
          phase_gate: "MICROTASK",
          expected_receipt_kind: "REVIEW_RESPONSE",
        },
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
      active_role_sessions: [],
      open_review_items: [
        {
          correlation_id: "review-1",
          receipt_kind: "REVIEW_REQUEST",
          summary: "Review MT-001 while coder continues MT-002.",
          opened_by_role: "CODER",
          opened_by_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          microtask_contract: {
            scope_ref: "MT-001",
            file_targets: ["src/demo.rs"],
            proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
            review_mode: "OVERLAP",
            phase_gate: "MICROTASK",
            expected_receipt_kind: "REVIEW_RESPONSE",
          },
          requires_ack: true,
          opened_at: "2026-04-05T10:05:00Z",
          updated_at: "2026-04-05T10:05:00Z",
        },
      ],
    }, null, 2),
    "utf8",
  );

  try {
    assert.doesNotThrow(() => validateWpReceiptAppendPreconditions({
      wpId,
      actorRole: "CODER",
      actorSession: "coder-1",
      receiptKind: "CODER_INTENT",
      summary: "Starting MT-002 while MT-001 is under overlap review.",
      targetRole: "WP_VALIDATOR",
      targetSession: "wpv-1",
      correlationId: "intent-2",
      ackFor: "intent-2",
      microtaskContract: {
        scope_ref: "MT-002",
        file_targets: ["src/demo_support.rs"],
        proof_commands: ["cargo test demo::tests::micro_2 -- --exact"],
        phase_gate: "MICROTASK",
        expected_receipt_kind: "VALIDATOR_RESPONSE",
      },
    }));
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("validator overlap resolution preflight rejects resolving the wrong previous microtask", () => {
  const wpId = "WP-TEST-MICROTASK-OVERLAP-RESOLUTION";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-microtask-overlap-resolution-"));
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  writeMicrotaskCheckpointPacket(packetDir, wpId, commDir, [
    {
      mtId: "MT-001",
      clause: "Demo microtask [CX-MICRO-001]",
      codeSurfaces: ["src/demo.rs"],
      expectedTests: ["cargo test demo::tests::micro_1 -- --exact"],
    },
    {
      mtId: "MT-002",
      clause: "Demo microtask [CX-MICRO-002]",
      codeSurfaces: ["src/demo_support.rs"],
      expectedTests: ["cargo test demo::tests::micro_2 -- --exact"],
    },
  ]);
  fs.writeFileSync(
    receiptsPath,
    [
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-05T10:00:00Z",
        wp_id: wpId,
        actor_role: "CODER",
        actor_session: "coder-1",
        actor_authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        receipt_kind: "CODER_INTENT",
        summary: "Starting MT-001.",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "intent-1",
        requires_ack: false,
        ack_for: "kickoff-1",
        spec_anchor: null,
        packet_row_ref: null,
        microtask_contract: {
          scope_ref: "MT-001",
          file_targets: ["src/demo.rs"],
          proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
          phase_gate: "MICROTASK",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
        refs: [],
      }),
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-05T10:05:00Z",
        wp_id: wpId,
        actor_role: "CODER",
        actor_session: "coder-1",
        actor_authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        receipt_kind: "REVIEW_REQUEST",
        summary: "Review MT-001 while I continue MT-002.",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "review-1",
        requires_ack: true,
        ack_for: null,
        spec_anchor: null,
        packet_row_ref: null,
        microtask_contract: {
          scope_ref: "MT-001",
          file_targets: ["src/demo.rs"],
          proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
          review_mode: "OVERLAP",
          phase_gate: "MICROTASK",
          expected_receipt_kind: "REVIEW_RESPONSE",
        },
        refs: [],
      }),
      JSON.stringify({
        schema_version: "wp_receipt@1",
        timestamp_utc: "2026-04-05T10:06:00Z",
        wp_id: wpId,
        actor_role: "CODER",
        actor_session: "coder-1",
        actor_authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        receipt_kind: "CODER_INTENT",
        summary: "Starting MT-002.",
        branch: "feat/test-intent-checkpoint",
        worktree_dir: "../wtc-test",
        state_before: null,
        state_after: null,
        target_role: "WP_VALIDATOR",
        target_session: "wpv-1",
        correlation_id: "intent-2",
        requires_ack: false,
        ack_for: "review-1",
        spec_anchor: null,
        packet_row_ref: null,
        microtask_contract: {
          scope_ref: "MT-002",
          file_targets: ["src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::micro_2 -- --exact"],
          phase_gate: "MICROTASK",
          expected_receipt_kind: "VALIDATOR_RESPONSE",
        },
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
      active_role_sessions: [],
      open_review_items: [
        {
          correlation_id: "review-1",
          receipt_kind: "REVIEW_REQUEST",
          summary: "Review MT-001 while coder continues MT-002.",
          opened_by_role: "CODER",
          opened_by_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          microtask_contract: {
            scope_ref: "MT-001",
            file_targets: ["src/demo.rs"],
            proof_commands: ["cargo test demo::tests::micro_1 -- --exact"],
            review_mode: "OVERLAP",
            phase_gate: "MICROTASK",
            expected_receipt_kind: "REVIEW_RESPONSE",
          },
          requires_ack: true,
          opened_at: "2026-04-05T10:05:00Z",
          updated_at: "2026-04-05T10:05:00Z",
        },
      ],
    }, null, 2),
    "utf8",
  );

  try {
    assert.throws(
      () => validateWpReceiptAppendPreconditions({
        wpId,
        actorRole: "WP_VALIDATOR",
        actorSession: "wpv-1",
        receiptKind: "VALIDATOR_RESPONSE",
        summary: "Cleared MT-002.",
        targetRole: "CODER",
        targetSession: "coder-1",
        correlationId: "review-1",
        ackFor: "review-1",
        microtaskContract: {
          scope_ref: "MT-002",
          file_targets: ["src/demo_support.rs"],
          proof_commands: ["cargo test demo::tests::micro_2 -- --exact"],
          review_mode: "OVERLAP",
          review_outcome: "APPROVED_FOR_FINAL_REVIEW",
          phase_gate: "MICROTASK",
          expected_receipt_kind: "CODER_INTENT",
        },
      }),
      /overlap review resolution must bind to previous microtask MT-001, not MT-002/i,
    );
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});
