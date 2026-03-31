import assert from "node:assert/strict";
import test from "node:test";
import { buildStartupPrompt, buildSteeringPrompt, CODEX_AUTHORITY_PATH, resolveRoleConfig } from "../scripts/session/session-control-lib.mjs";
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
  assert.match(prompt, /just active-lane-brief CODER WP-TEST-CODER-v1/i);
  assert.match(prompt, /just check-notifications WP-TEST-CODER-v1 CODER <your-session>/i);
  assert.match(prompt, new RegExp(CODEX_AUTHORITY_PATH.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
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

test("wp-validator startup prompt uses the dedicated validator lane and early steering instructions", () => {
  const wpId = "WP-TEST-WPVAL-v1";
  const roleConfig = resolveRoleConfig("WP_VALIDATOR", wpId);
  const prompt = buildStartupPrompt({
    role: "WP_VALIDATOR",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
  });

  assert.match(roleConfig.branch, /^validate\/WP-TEST-WPVAL-v1$/);
  assert.match(roleConfig.worktreeDir, /^\.\.\/wtv-/);
  assert.match(prompt, /SESSION ISOLATION: do not spawn or use helper agents\/subagents/i);
  assert.match(prompt, /judge bootstrap\/skeleton\/micro-task direction early/i);
  assert.match(prompt, /EARLY STEERING \(MANDATORY\): You are the first technical judge for coder BOOTSTRAP, SKELETON, and completed micro tasks/i);
  assert.match(prompt, /WORKTREE SYNC \(MANDATORY\): Keep your dedicated validator branch\/worktree reviewable against the coder branch/i);
  assert.match(prompt, /just check-notifications WP-TEST-WPVAL-v1 WP_VALIDATOR <your-session>/i);
});

test("steering prompt stays compact and codex-explicit", () => {
  const wpId = "WP-TEST-STEER-v1";
  const prompt = buildSteeringPrompt({
    role: "INTEGRATION_VALIDATOR",
    wpId,
  });

  assert.match(prompt, /RESUME GOVERNED INTEGRATION_VALIDATOR lane/i);
  assert.match(prompt, new RegExp(CODEX_AUTHORITY_PATH.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  assert.match(prompt, /just active-lane-brief INTEGRATION_VALIDATOR WP-TEST-STEER-v1/i);
  assert.match(prompt, /Run in order:/i);
  assert.match(prompt, /just validator-next WP-TEST-STEER-v1/i);
  assert.match(prompt, /just check-notifications WP-TEST-STEER-v1 INTEGRATION_VALIDATOR <your-session>/i);
  assert.match(prompt, /Do not request routine Operator approval/i);
});
