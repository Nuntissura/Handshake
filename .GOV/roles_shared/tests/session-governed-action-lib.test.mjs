import assert from "node:assert/strict";
import test from "node:test";
import {
  buildGovernedActionRequest,
  buildGovernedActionResult,
  classifyGovernedActionResumeDisposition,
  classifyGovernedActionResultState,
  defaultGovernedActionRuleIdForSessionCommand,
  effectiveSessionGovernedAction,
  governedSessionMirrorDrift,
  validateGovernedActionRequestShape,
  validateGovernedActionResultShape,
} from "../scripts/session/session-governed-action-lib.mjs";

test("session-control external-execute requests default to the command-scoped rule registry", () => {
  const action = buildGovernedActionRequest({
    commandKind: "START_SESSION",
    commandId: "11111111-1111-1111-1111-111111111111",
    sessionKey: "CODER:WP-TEST-v1",
    wpId: "WP-TEST-v1",
    role: "CODER",
    createdByRole: "ORCHESTRATOR",
    summary: "Start the coder lane.",
  });

  assert.equal(
    action.rule_id,
    defaultGovernedActionRuleIdForSessionCommand("START_SESSION", "EXTERNAL_EXECUTE"),
  );
  assert.equal(action.action_kind, "EXTERNAL_EXECUTE");
  assert.equal(action.command_kind, "START_SESSION");
  assert.equal(action.resume_policy, "WAIT_FOR_TRANSPORT_RESULT");
  assert.deepEqual(validateGovernedActionRequestShape(action), []);
});

test("validator-gate deny result projects stop semantics without transcript inference", () => {
  const action = buildGovernedActionResult({
    ruleId: "VALIDATOR_GATE_DENY_RESUME",
    actionKind: "DENY",
    commandId: "22222222-2222-2222-2222-222222222222",
    sessionKey: "INTEGRATION_VALIDATOR:WP-TEST-v1",
    wpId: "WP-TEST-v1",
    role: "INTEGRATION_VALIDATOR",
    status: "COMPLETED",
    outcomeState: "SETTLED",
    summary: "Do not resume validation until packet debt is repaired.",
  });

  assert.equal(action.result_state, "REJECTED");
  assert.equal(action.resume_disposition, "STOP");
  assert.deepEqual(validateGovernedActionResultShape(action), []);
});

test("integration-validator closeout sync uses a registry-backed external-execute rule", () => {
  const action = buildGovernedActionResult({
    ruleId: "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE",
    commandKind: "CLOSEOUT_SYNC",
    commandId: "33333333-3333-3333-3333-333333333333",
    sessionKey: "INTEGRATION_VALIDATOR:WP-TEST-v1",
    wpId: "WP-TEST-v1",
    role: "INTEGRATION_VALIDATOR",
    status: "COMPLETED",
    outcomeState: "SETTLED",
    summary: "Record closeout sync MERGE_PENDING for the final lane.",
  });

  assert.equal(action.action_kind, "EXTERNAL_EXECUTE");
  assert.equal(action.command_kind, "CLOSEOUT_SYNC");
  assert.equal(action.result_state, "SETTLED");
  assert.equal(action.resume_disposition, "CONSUME_RESULT");
  assert.deepEqual(validateGovernedActionResultShape(action), []);
});

test("governed action classifiers expose accepted queued/running, retry, and settled external execution semantics", () => {
  assert.equal(classifyGovernedActionResultState({
    actionKind: "EXTERNAL_EXECUTE",
    status: "RUNNING",
  }), "ACCEPTED_RUNNING");
  assert.equal(classifyGovernedActionResultState({
    actionKind: "EXTERNAL_EXECUTE",
    status: "QUEUED",
  }), "ACCEPTED_QUEUED");
  assert.equal(classifyGovernedActionResumeDisposition({
    actionKind: "RETRY",
    resultState: "SETTLED",
  }), "RETRY_ALLOWED");
  assert.equal(classifyGovernedActionResumeDisposition({
    actionKind: "EXTERNAL_EXECUTE",
    resultState: "ACCEPTED_RUNNING",
  }), "PENDING");
  assert.equal(classifyGovernedActionResumeDisposition({
    actionKind: "EXTERNAL_EXECUTE",
    resultState: "SETTLED",
  }), "CONSUME_RESULT");
});

test("effectiveSessionGovernedAction prefers typed action history over legacy command mirrors", () => {
  const effective = effectiveSessionGovernedAction({
    last_command_id: "legacy-command",
    last_command_kind: "SEND_PROMPT",
    last_command_status: "RUNNING",
    last_command_summary: "Legacy mirror summary.",
    action_history: [
      {
        action_id: "44444444-4444-4444-4444-444444444444",
        rule_id: "SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE",
        action_kind: "EXTERNAL_EXECUTE",
        action_surface: "SESSION_CONTROL",
        command_kind: "SEND_PROMPT",
        command_id: "44444444-4444-4444-4444-444444444444",
        action_state: "SETTLED",
        status: "COMPLETED",
        resume_disposition: "CONSUME_RESULT",
        summary: "Typed action history wins.",
        processed_at: "2026-04-20T10:01:00Z",
      },
    ],
  });

  assert.equal(effective.command_id, "44444444-4444-4444-4444-444444444444");
  assert.equal(effective.status, "COMPLETED");
  assert.equal(effective.action_state, "SETTLED");
  assert.equal(effective.summary, "Typed action history wins.");
  assert.equal(effective.source, "governed_action");
});

test("governedSessionMirrorDrift reports when legacy mirrors diverge from typed governed action truth", () => {
  const drift = governedSessionMirrorDrift({
    last_command_id: "legacy-command",
    last_command_kind: "START_SESSION",
    last_command_status: "RUNNING",
    action_history: [
      {
        action_id: "55555555-5555-5555-5555-555555555555",
        rule_id: "SESSION_CONTROL_START_SESSION_EXTERNAL_EXECUTE",
        action_kind: "EXTERNAL_EXECUTE",
        action_surface: "SESSION_CONTROL",
        command_kind: "START_SESSION",
        command_id: "55555555-5555-5555-5555-555555555555",
        action_state: "SETTLED",
        status: "COMPLETED",
        resume_disposition: "CONSUME_RESULT",
        summary: "Typed action finished.",
        processed_at: "2026-04-20T10:02:00Z",
      },
    ],
  });

  assert.equal(drift.length, 2);
  assert.match(drift[0], /command_id disagrees/i);
  assert.match(drift[1], /status disagrees/i);
});
