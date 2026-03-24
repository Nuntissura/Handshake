import assert from "node:assert/strict";
import test from "node:test";
import { buildStartupPrompt, resolveRoleConfig } from "../scripts/session/session-control-lib.mjs";
import { ROLE_SESSION_PRIMARY_MODEL } from "../scripts/session/session-policy.mjs";

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
  assert.match(prompt, /wp-communication-health-check .* VERDICT/i);
  assert.match(prompt, /Final merge-ready authority/i);
});
