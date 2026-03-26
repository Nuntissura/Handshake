import assert from "node:assert/strict";
import test from "node:test";
import { buildStartupPrompt, resolveRoleConfig } from "../scripts/session/session-control-lib.mjs";
import { ROLE_SESSION_PRIMARY_MODEL } from "../scripts/session/session-policy.mjs";

test("coder startup prompt carries orchestrator-managed relapse guard and lane-aware flow", () => {
  const wpId = "WP-TEST-CODER-v1";
  const roleConfig = resolveRoleConfig("CODER", wpId);
  const prompt = buildStartupPrompt({
    role: "CODER",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
  });

  assert.match(prompt, /POST-SIGNATURE RELAPSE GUARD \(MANDATORY\):/i);
  assert.match(prompt, /POLICY_CONFLICT, AUTHORITY_OVERRIDE_REQUIRED, OPERATOR_ARTIFACT_REQUIRED, ENVIRONMENT_FAILURE/i);
  assert.match(prompt, /`MANUAL_RELAY` = .*skeleton approval when required/i);
  assert.match(prompt, /`ORCHESTRATOR_MANAGED` = .*no routine Operator approvals after signature/i);
});

test("integration-validator startup prompt includes direct-review and verdict-gate instructions", () => {
  const wpId = "WP-TEST-VALIDATOR-v1";
  const roleConfig = resolveRoleConfig("INTEGRATION_VALIDATOR", wpId);
  const prompt = buildStartupPrompt({
    role: "INTEGRATION_VALIDATOR",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
  });

  assert.match(prompt, /DIRECT COMMUNICATION \(MANDATORY\): Use the structured final review lane/i);
  assert.match(prompt, /wp-review-exchange REVIEW_REQUEST/i);
  assert.match(prompt, /wp-review-response/i);
  assert.match(prompt, /integration-validator-context-brief/i);
  assert.match(prompt, /wp-communication-health-check .* VERDICT/i);
  assert.match(prompt, /Final merge-ready authority/i);
  assert.match(prompt, /ORCHESTRATOR-MANAGED RULE: do not ask the Operator for routine approval, proceed, or checkpoint actions after signature\/prepare/i);
});
