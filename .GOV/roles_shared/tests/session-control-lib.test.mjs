import assert from "node:assert/strict";
import test from "node:test";
import {
  buildRoleEnvironmentOverrides,
  buildSessionControlRequest,
  buildStartupPrompt,
  buildSteeringPrompt,
  CODEX_AUTHORITY_PATH,
  resolveRoleConfig,
} from "../scripts/session/session-control-lib.mjs";
import {
  ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX,
  ROLE_MODEL_PROFILE_OPENAI_GPT_5_4_XHIGH,
  ROLE_SESSION_PRIMARY_MODEL,
  roleModelProfile,
} from "../scripts/session/session-policy.mjs";

test("coder startup prompt carries orchestrator-managed relapse guard and lane-aware flow", () => {
  const wpId = "WP-TEST-CODER-v1";
  const roleConfig = resolveRoleConfig("CODER", wpId);
  const selectedProfile = roleModelProfile(ROLE_MODEL_PROFILE_OPENAI_GPT_5_4_XHIGH);
  const prompt = buildStartupPrompt({
    role: "CODER",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
    selectedProfileId: ROLE_MODEL_PROFILE_OPENAI_GPT_5_4_XHIGH,
    selectedProfile,
  });

  assert.match(prompt, /MODEL PROFILE: OPENAI_GPT_5_4_XHIGH/i);
  assert.match(prompt, /POST-SIGNATURE RELAPSE GUARD \(MANDATORY\):/i);
  assert.match(prompt, /POLICY_CONFLICT, AUTHORITY_OVERRIDE_REQUIRED, OPERATOR_ARTIFACT_REQUIRED, ENVIRONMENT_FAILURE/i);
  assert.match(prompt, /`MANUAL_RELAY` = .*skeleton approval when required/i);
  assert.match(prompt, /`ORCHESTRATOR_MANAGED` = .*no routine Operator approvals after signature/i);
  assert.match(prompt, /just active-lane-brief CODER WP-TEST-CODER-v1/i);
  assert.match(prompt, /just phase-check STARTUP WP-TEST-CODER-v1 CODER <your-session>/i);
  assert.match(prompt, /just check-notifications WP-TEST-CODER-v1 CODER <your-session>/i);
  assert.match(prompt, /read-only context except for the assigned packet and declared MT files/i);
  assert.match(prompt, /without committing \.GOV on the feature branch/i);
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
  assert.match(prompt, /phase-check STARTUP .* INTEGRATION_VALIDATOR/i);
  assert.match(prompt, /phase-check VERDICT .* INTEGRATION_VALIDATOR/i);
  assert.match(prompt, /Final merge-ready authority/i);
  assert.match(prompt, /HANDSHAKE_GOV_ROOT/i);
  assert.match(prompt, /Do not use handshake_main\/.GOV as the live source of truth/i);
  assert.match(prompt, /Do not manually grep, browse, or rebuild authority from handshake_main\/.GOV/i);
  assert.match(prompt, /FINAL-LANE STARTUP ORDER \(HARD\): Before any repo search, packet rediscovery, or broad \.GOV inspection/i);
  assert.match(prompt, /packet_read_path/i);
  assert.match(prompt, /ORCHESTRATOR-MANAGED RULE: do not ask the Operator for routine approval, proceed, or checkpoint actions after signature\/prepare/i);
  assert.match(prompt, /3\. just integration-validator-context-brief WP-TEST-VALIDATOR-v1/i);
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

  assert.match(roleConfig.branch, /^feat\/WP-TEST-WPVAL-v1$/);
  assert.match(roleConfig.worktreeDir, /^\.\.\/wtc-/);
  assert.match(prompt, /SESSION ISOLATION: do not spawn or use helper agents\/subagents/i);
  assert.match(prompt, /judge bootstrap\/skeleton\/micro-task direction early/i);
  assert.match(prompt, /EARLY STEERING \(MANDATORY\): You own the governed bootstrap\/skeleton checkpoint/i);
  assert.match(prompt, /WORKTREE SYNC \(MANDATORY\): You share the coder `feat\/WP-TEST-WPVAL-v1` branch and `wtc-\*` worktree surface/i);
  assert.match(prompt, /just phase-check STARTUP WP-TEST-WPVAL-v1 WP_VALIDATOR <your-session>/i);
  assert.match(prompt, /just check-notifications WP-TEST-WPVAL-v1 WP_VALIDATOR <your-session>/i);
});

test("activation-manager startup and steering prompts enforce the workflow split", () => {
  const wpId = "WP-TEST-ACTMAN-v1";
  const roleConfig = resolveRoleConfig("ACTIVATION_MANAGER", wpId);
  const prompt = buildStartupPrompt({
    role: "ACTIVATION_MANAGER",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
  });
  const steerPrompt = buildSteeringPrompt({
    role: "ACTIVATION_MANAGER",
    wpId,
    roleConfig,
  });

  assert.equal(roleConfig.branch, "gov_kernel");
  assert.equal(roleConfig.worktreeDir, ".");
  assert.match(prompt, /WORKFLOW SPLIT \(MANDATORY\): For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`/i);
  assert.match(prompt, /mandatory governed pre-launch authoring lane and temporary worker/i);
  assert.match(prompt, /must own the heavy pre-launch reasoning/i);
  assert.match(prompt, /REFINEMENT STANDARD \(HARD\):/i);
  assert.match(prompt, /STUB DISCOVERY RULE \(HARD\):/i);
  assert.match(prompt, /HANDOFF CHUNKING RULE \(HARD\):/i);
  assert.match(prompt, /SIGNATURE ROUND-TRIP \(MANDATORY\):/i);
  assert.match(prompt, /For `MANUAL_RELAY`, pre-launch remains Orchestrator-owned/i);
  assert.match(prompt, /just activation-manager readiness WP-TEST-ACTMAN-v1 --write/i);
  assert.match(prompt, /just activation-manager startup/i);
  assert.match(prompt, /just activation-manager next WP-TEST-ACTMAN-v1/i);

  assert.match(steerPrompt, /RESUME GOVERNED ACTIVATION_MANAGER lane/i);
  assert.match(steerPrompt, /mandatory temporary pre-launch worker/i);
  assert.match(steerPrompt, /HANDOFF CHUNKING RULE \(HARD\):/i);
  assert.match(steerPrompt, /REPAIR LOOP \(MANDATORY\):/i);
  assert.match(steerPrompt, /just activation-manager next WP-TEST-ACTMAN-v1/i);
  assert.match(steerPrompt, /just activation-manager readiness WP-TEST-ACTMAN-v1 --write/i);
  assert.doesNotMatch(steerPrompt, /check-notifications/i);
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
  assert.match(prompt, /just integration-validator-context-brief WP-TEST-STEER-v1/i);
  assert.match(prompt, /just validator-next WP-TEST-STEER-v1/i);
  assert.match(prompt, /just check-notifications WP-TEST-STEER-v1 INTEGRATION_VALIDATOR <your-session>/i);
  assert.match(prompt, /Do not manually inspect handshake_main\/.GOV as authoritative context/i);
  assert.match(prompt, /FIRST READ RULE: before any repo-wide search or packet rediscovery/i);
  assert.match(prompt, /packet_read_path/i);
  assert.match(prompt, /Do not request routine Operator approval/i);
});

test("integration-validator control requests carry kernel governance env override", () => {
  const env = buildRoleEnvironmentOverrides({
    role: "INTEGRATION_VALIDATOR",
    governanceRootAbs: "D:/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV",
  });
  const request = buildSessionControlRequest({
    commandKind: "START_SESSION",
    wpId: "WP-TEST-STEER-v1",
    role: "INTEGRATION_VALIDATOR",
    sessionKey: "INTEGRATION_VALIDATOR:WP-TEST-STEER-v1",
    localBranch: "main",
    localWorktreeDir: "../handshake_main",
    absWorktreeDir: "D:/Handshake/Handshake Worktrees/handshake_main",
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
    selectedProfileId: ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX,
    prompt: "just validator-startup",
    outputJsonlFile: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/test.jsonl",
    environmentOverrides: env,
  });

  assert.deepEqual(request.environment_overrides, {
    HANDSHAKE_GOV_ROOT: "D:/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV",
  });
  assert.equal(request.selected_profile_id, ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX);
});
