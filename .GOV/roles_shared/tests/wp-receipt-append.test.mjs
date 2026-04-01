import assert from "node:assert/strict";
import test from "node:test";
import { deriveWpScopeContract } from "../scripts/lib/scope-surface-lib.mjs";
import {
  deriveReviewNotificationTargets,
  summarizeCommittedCoderHandoffDirtyState,
} from "../scripts/wp/wp-receipt-append.mjs";

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

test("committed coder handoff dirty-state summary ignores shared governance junction drift but blocks product dirt", () => {
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
    " M src/demo.rs",
  ].join("\n"), scopeContract);

  assert.equal(summary.ok, false);
  assert.deepEqual(summary.governanceJunctionPaths, [".GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md"]);
  assert.deepEqual(summary.blockingPaths, ["src/demo.rs (IN_SCOPE)"]);
});
