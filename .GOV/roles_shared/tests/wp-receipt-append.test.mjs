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
