import assert from "node:assert/strict";
import test from "node:test";
import { validateGovernedActionResultShape } from "../scripts/session/session-governed-action-lib.mjs";
import {
  appendValidatorGateGovernedAction,
  buildValidatorGateGovernedAction,
  deriveValidatorGateSessionStatus,
} from "../scripts/lib/validator-gate-governed-action-lib.mjs";

test("validator gate append action builds a typed governed-action result", () => {
  const action = buildValidatorGateGovernedAction({
    wpId: "WP-TEST-v1",
    gateAction: "APPEND",
    sessionKey: "WP_VALIDATOR:WP-TEST-v1",
    role: "WP_VALIDATOR",
    summary: "Append gate recorded.",
    gateStatus: "WP_APPENDED",
    gateVerdict: "PASS",
  });

  assert.equal(action.rule_id, "VALIDATOR_GATE_APPEND_APPROVE");
  assert.equal(action.action_kind, "APPROVE");
  assert.equal(action.command_kind, "APPEND");
  assert.equal(action.resume_disposition, "RESUME_ALLOWED");
  assert.equal(action.metadata.gate_status, "WP_APPENDED");
  assert.equal(action.metadata.gate_verdict, "PASS");
  assert.deepEqual(validateGovernedActionResultShape(action), []);
});

test("validator gate session status prefers the last governed action over the legacy status mirror", () => {
  const appended = buildValidatorGateGovernedAction({
    wpId: "WP-TEST-v1",
    gateAction: "APPEND",
    sessionKey: "WP_VALIDATOR:WP-TEST-v1",
    role: "WP_VALIDATOR",
    summary: "Append gate recorded.",
    gateStatus: "WP_APPENDED",
    gateVerdict: "PASS",
    processedAt: "2026-04-20T10:00:00.000Z",
  });
  const committed = buildValidatorGateGovernedAction({
    wpId: "WP-TEST-v1",
    gateAction: "COMMIT",
    sessionKey: "WP_VALIDATOR:WP-TEST-v1",
    role: "WP_VALIDATOR",
    summary: "Commit gate recorded.",
    gateStatus: "COMMITTED",
    gateVerdict: "PASS",
    previousStatus: "WP_APPENDED",
    processedAt: "2026-04-20T10:05:00.000Z",
  });
  const session = appendValidatorGateGovernedAction({
    wpId: "WP-TEST-v1",
    verdict: "PASS",
    status: "WP_APPENDED",
    governed_action_history: [],
  }, appended);
  const finalSession = appendValidatorGateGovernedAction(session, committed);
  finalSession.status = "WP_APPENDED";

  const view = deriveValidatorGateSessionStatus(finalSession);
  assert.equal(view.status, "COMMITTED");
  assert.equal(view.lastGovernedAction.rule_id, "VALIDATOR_GATE_COMMIT_APPROVE");
  assert.equal(view.lastGovernedAction.gate_status, "COMMITTED");
  assert.equal(view.governedActionHistory.length, 2);
});
