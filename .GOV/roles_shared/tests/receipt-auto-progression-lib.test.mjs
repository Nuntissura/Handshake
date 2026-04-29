import assert from "node:assert/strict";
import test from "node:test";

import { deriveNextActionFromReceipt } from "../scripts/lib/receipt-auto-progression-lib.mjs";

test("deriveNextActionFromReceipt dispatches governed next actor when no duplicate exists", () => {
  const decision = deriveNextActionFromReceipt({
    wpId: "WP-TEST-AUTO",
    workflowLane: "ORCHESTRATOR_MANAGED",
    receiptEntry: { actor_role: "CODER" },
    autoRoute: { applicable: true, nextExpectedActor: "WP_VALIDATOR" },
    registrySessions: [],
  });

  assert.equal(decision.action, "DISPATCH");
  assert.equal(decision.next_actor, "WP_VALIDATOR");
  assert.equal(decision.reason, "AUTO_RELAY_DISPATCHABLE");
});

test("deriveNextActionFromReceipt suppresses duplicate queued wake for target session", () => {
  const decision = deriveNextActionFromReceipt({
    wpId: "WP-TEST-AUTO",
    workflowLane: "ORCHESTRATOR_MANAGED",
    receiptEntry: { actor_role: "CODER" },
    autoRoute: { applicable: true, nextExpectedActor: "WP_VALIDATOR" },
    registrySessions: [{
      session_key: "WP_VALIDATOR:WP-TEST-AUTO",
      role: "WP_VALIDATOR",
      wp_id: "WP-TEST-AUTO",
      runtime_state: "READY",
      pending_control_queue_count: 1,
      next_queued_control_request: { command_kind: "SEND_PROMPT" },
    }],
  });

  assert.equal(decision.action, "SUPPRESS_DUPLICATE");
  assert.equal(decision.reason, "AUTO_RELAY_ALREADY_QUEUED");
  assert.equal(decision.queue_depth, 1);
});

test("deriveNextActionFromReceipt suppresses in-flight command for target session", () => {
  const decision = deriveNextActionFromReceipt({
    wpId: "WP-TEST-AUTO",
    workflowLane: "ORCHESTRATOR_MANAGED",
    receiptEntry: { actor_role: "CODER" },
    autoRoute: { applicable: true, nextExpectedActor: "WP_VALIDATOR" },
    registrySessions: [{
      session_key: "WP_VALIDATOR:WP-TEST-AUTO",
      role: "WP_VALIDATOR",
      wp_id: "WP-TEST-AUTO",
      runtime_state: "COMMAND_RUNNING",
      pending_control_queue_count: 0,
    }],
  });

  assert.equal(decision.action, "SUPPRESS_DUPLICATE");
  assert.equal(decision.reason, "AUTO_RELAY_COMMAND_IN_FLIGHT");
});

test("deriveNextActionFromReceipt skips self-routing receipts", () => {
  const decision = deriveNextActionFromReceipt({
    wpId: "WP-TEST-AUTO",
    workflowLane: "ORCHESTRATOR_MANAGED",
    receiptEntry: { actor_role: "WP_VALIDATOR" },
    autoRoute: { applicable: true, nextExpectedActor: "WP_VALIDATOR" },
  });

  assert.equal(decision.status, "SKIPPED");
  assert.equal(decision.reason, "NEXT_ACTOR_IS_CURRENT_ACTOR");
});
