import assert from "node:assert/strict";
import test from "node:test";
import {
  boundPromptLines,
  buildStartupInjectionLines,
  buildRoleEnvironmentOverrides,
  buildSessionControlRequest,
  buildSessionControlResult,
  buildStartupPrompt,
  buildSteeringPrompt,
  classifySessionControlOutcomeState,
  CODEX_AUTHORITY_PATH,
  resolveRoleConfig,
} from "../scripts/session/session-control-lib.mjs";
import {
  ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX,
  ROLE_MODEL_PROFILE_OPENAI_GPT_5_4_XHIGH,
  ROLE_SESSION_PRIMARY_MODEL,
  resolveRoleModelProfileSelection,
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
    startupMemoryLines: [],
    conversationContextLines: [],
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
    startupMemoryLines: [],
    conversationContextLines: [],
  });

  assert.match(prompt, /VERDICT COMMUNICATION \(MANDATORY\): The Integration Validator does NOT communicate directly with the Coder/i);
  assert.match(prompt, /wp-receipt-append/i);
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
  assert.match(prompt, /\.GOV\/roles\/integration_validator\/INTEGRATION_VALIDATOR_PROTOCOL\.md/i);
});

test("wp-validator startup prompt uses the dedicated validator lane and early steering instructions", () => {
  const wpId = "WP-TEST-WPVAL-v1";
  const roleConfig = resolveRoleConfig("WP_VALIDATOR", wpId);
  const prompt = buildStartupPrompt({
    role: "WP_VALIDATOR",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
    startupMemoryLines: [],
    conversationContextLines: [],
  });

  assert.match(roleConfig.branch, /^feat\/WP-TEST-WPVAL-v1$/);
  assert.match(roleConfig.worktreeDir, /^\.\.\/wtc-/);
  assert.match(prompt, /SESSION ISOLATION: do not spawn or use helper agents\/subagents/i);
  assert.match(prompt, /judge bootstrap\/skeleton\/micro-task direction early/i);
  assert.match(prompt, /EARLY STEERING \(MANDATORY\): You own the governed bootstrap\/skeleton checkpoint/i);
  assert.match(prompt, /WORKTREE SYNC \(MANDATORY\): You share the coder `feat\/WP-TEST-WPVAL-v1` branch and `wtc-\*` worktree surface/i);
  assert.match(prompt, /just phase-check STARTUP WP-TEST-WPVAL-v1 WP_VALIDATOR <your-session>/i);
  assert.match(prompt, /just check-notifications WP-TEST-WPVAL-v1 WP_VALIDATOR <your-session>/i);
  assert.match(prompt, /\.GOV\/roles\/wp_validator\/WP_VALIDATOR_PROTOCOL\.md/i);
  assert.doesNotMatch(prompt, /dedicated validator worktree/i);
});

test("activation-manager startup and steering prompts enforce the workflow split", () => {
  const wpId = "WP-TEST-ACTMAN-v1";
  const roleConfig = resolveRoleConfig("ACTIVATION_MANAGER", wpId);
  const prompt = buildStartupPrompt({
    role: "ACTIVATION_MANAGER",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
    startupMemoryLines: [],
    conversationContextLines: [],
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
  assert.match(prompt, /FILE-FIRST HANDOFF RULE \(HARD\):/i);
  assert.match(prompt, /REFINEMENT_HANDOFF_SUMMARY \(HARD\):/i);
  assert.match(prompt, /REFINEMENT_CHECK RULE \(HARD\):/i);
  assert.match(prompt, /UPGRADE DISCIPLINE \(HARD\):/i);
  assert.match(prompt, /EXCERPT FALLBACK RULE \(HARD\):/i);
  assert.match(prompt, /SIGNATURE ROUND-TRIP \(MANDATORY\):/i);
  assert.match(prompt, /For `MANUAL_RELAY`, pre-launch belongs to `CLASSIC_ORCHESTRATOR`/i);
  assert.match(prompt, /just activation-manager readiness WP-TEST-ACTMAN-v1 --write/i);
  assert.match(prompt, /just activation-manager startup/i);
  assert.match(prompt, /just activation-manager next WP-TEST-ACTMAN-v1/i);

  assert.match(steerPrompt, /RESUME GOVERNED ACTIVATION_MANAGER lane/i);
  assert.match(steerPrompt, /mandatory temporary pre-launch worker/i);
  assert.match(steerPrompt, /FILE-FIRST HANDOFF RULE \(HARD\):/i);
  assert.match(steerPrompt, /REFINEMENT_HANDOFF_SUMMARY \(HARD\):/i);
  assert.match(steerPrompt, /REFINEMENT_CHECK RULE \(HARD\):/i);
  assert.match(steerPrompt, /UPGRADE DISCIPLINE \(HARD\):/i);
  assert.match(steerPrompt, /EXCERPT FALLBACK RULE \(HARD\):/i);
  assert.match(steerPrompt, /REPAIR LOOP \(MANDATORY\):/i);
  assert.match(steerPrompt, /manual workflow, pre-launch belongs to CLASSIC_ORCHESTRATOR/i);
  assert.match(steerPrompt, /just activation-manager next WP-TEST-ACTMAN-v1/i);
  assert.match(steerPrompt, /just activation-manager readiness WP-TEST-ACTMAN-v1 --write/i);
  assert.doesNotMatch(steerPrompt, /check-notifications/i);
});

test("activation-manager profile selection honors an explicit declared Claude profile", () => {
  const packetLikeText = [
    "- ACTIVATION_MANAGER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX",
    "- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH",
  ].join("\n");

  const selection = resolveRoleModelProfileSelection("ACTIVATION_MANAGER", packetLikeText, "PRIMARY");
  const fallbackSelection = resolveRoleModelProfileSelection("ACTIVATION_MANAGER", packetLikeText, "FALLBACK");

  assert.equal(selection.primary_profile_id, ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX);
  assert.equal(selection.selected_profile_id, ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX);
  assert.equal(selection.profile?.launch_model, "claude-opus-4-6");
  assert.equal(selection.profile?.launch_reasoning_config_value, "max");
  assert.equal(fallbackSelection.selected_profile_id, ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX);
});

test("memory-manager prompts advertise synthetic receipt emission instead of packet assumptions", () => {
  const wpId = "WP-MEMORY-HYGIENE_2026-04-09T2115Z";
  const roleConfig = resolveRoleConfig("MEMORY_MANAGER", wpId);
  const startupPrompt = buildStartupPrompt({
    role: "MEMORY_MANAGER",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
    startupMemoryLines: [],
    conversationContextLines: [],
  });
  const steerPrompt = buildSteeringPrompt({
    role: "MEMORY_MANAGER",
    wpId,
    roleConfig,
  });

  assert.match(startupPrompt, /SYNTHETIC-WP RULE:/i);
  assert.match(startupPrompt, /just memory-manager-proposal WP-MEMORY-HYGIENE_2026-04-09T2115Z/i);
  assert.match(startupPrompt, /just memory-manager-flag-receipt WP-MEMORY-HYGIENE_2026-04-09T2115Z/i);
  assert.match(startupPrompt, /just memory-manager-rgf-candidate WP-MEMORY-HYGIENE_2026-04-09T2115Z/i);
  assert.match(startupPrompt, /just repomem close "<session summary>" --decisions/i);
  assert.match(startupPrompt, /SESSION_COMPLETION/i);
  assert.match(startupPrompt, /do not expect an official packet/i);

  assert.match(steerPrompt, /There is no official packet for this lane/i);
  assert.match(steerPrompt, /MEMORY_PROPOSAL \/ MEMORY_FLAG \/ MEMORY_RGF_CANDIDATE/i);
  assert.match(steerPrompt, /SESSION_COMPLETION/i);
  assert.match(steerPrompt, /just repomem close "<session summary>" --decisions/i);
  assert.doesNotMatch(steerPrompt, /active-lane-brief/i);
});

test("startup prompt includes bounded memory injection when lines are supplied", () => {
  const wpId = "WP-TEST-INJECT-v1";
  const roleConfig = resolveRoleConfig("CODER", wpId);
  const prompt = buildStartupPrompt({
    role: "CODER",
    wpId,
    roleConfig,
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
    startupMemoryLines: [
      "SESSION MEMORY (1 pattern, 24 est. tokens):",
      "FAIL LOG:",
      "- Script failure: coder-next.mjs - wrong packet path",
    ],
    conversationContextLines: [
      "CONVERSATION CONTEXT (prior session 2026-04-09, CODER):",
      "- [INSIGHT] packet path should come from validator-next output",
    ],
  });

  assert.match(prompt, /MEMORY INJECTION \(BOUNDED\):/);
  assert.match(prompt, /SESSION MEMORY \(1 pattern, 24 est\. tokens\):/);
  assert.match(prompt, /FAIL LOG:/);
  assert.match(prompt, /CONVERSATION CONTEXT \(prior session 2026-04-09, CODER\):/);
});

test("boundPromptLines enforces line and token caps", () => {
  const lines = boundPromptLines(
    [
      "12345678901234567890",
      "abcdefghijabcdefghij",
      "XYZXYZXYZXYZXYZXYZXYZXYZ",
    ],
    { tokenBudget: 12, maxLines: 2 },
  );

  assert.deepEqual(lines, [
    "12345678901234567890",
    "abcdefghijabcdefghij",
  ]);
});

test("buildStartupInjectionLines returns no section when both sources are empty", () => {
  const lines = buildStartupInjectionLines({
    role: "CODER",
    wpId: "WP-TEST-v1",
    startupMemoryLines: [],
    conversationContextLines: [],
  });

  assert.deepEqual(lines, []);
});

test("session-control outcome classifier distinguishes ready, busy, recovery, and missing-start cases", () => {
  assert.equal(classifySessionControlOutcomeState({
    status: "COMPLETED",
    commandKind: "START_SESSION",
    summary: "Session CODER:WP-TEST already has steerable thread thread_123.",
  }), "ALREADY_READY");
  assert.equal(classifySessionControlOutcomeState({
    status: "FAILED",
    commandKind: "START_SESSION",
    error: "Concurrent governed run already active for CODER:WP-TEST (1234)",
  }), "BUSY_ACTIVE_RUN");
  assert.equal(classifySessionControlOutcomeState({
    status: "FAILED",
    commandKind: "SEND_PROMPT",
    error: "No steerable thread id is registered yet for CODER:WP-TEST. Start the session first.",
  }), "REQUIRES_START");
  assert.equal(classifySessionControlOutcomeState({
    status: "FAILED",
    commandKind: "SEND_PROMPT",
    error: "Recovered orphaned governed request 123 after session stayed RUNNING without an active broker run.",
  }), "REQUIRES_RECOVERY");
});

test("session-control results persist outcome_state", () => {
  const result = buildSessionControlResult({
    commandId: "123e4567-e89b-12d3-a456-426614174000",
    commandKind: "START_SESSION",
    sessionKey: "CODER:WP-TEST-v1",
    wpId: "WP-TEST-v1",
    role: "CODER",
    status: "FAILED",
    summary: "Concurrent governed run already active for CODER:WP-TEST-v1 (abc)",
    error: "Concurrent governed run already active for CODER:WP-TEST-v1 (abc)",
    outputJsonlFile: "../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/test.jsonl",
  });

  assert.equal(result.outcome_state, "BUSY_ACTIVE_RUN");
  assert.equal(result.governed_action.action_id, "123e4567-e89b-12d3-a456-426614174000");
  assert.equal(result.governed_action.rule_id, "SESSION_CONTROL_START_SESSION_EXTERNAL_EXECUTE");
  assert.equal(result.governed_action.resume_disposition, "REPAIR_REQUIRED");
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
  assert.equal(request.governed_action.rule_id, "SESSION_CONTROL_START_SESSION_EXTERNAL_EXECUTE");
  assert.equal(request.governed_action.command_id, request.command_id);
  assert.equal(request.busy_ingress_mode, "REJECT");
});

test("send prompt control requests default to queued busy ingress mode", () => {
  const request = buildSessionControlRequest({
    commandKind: "SEND_PROMPT",
    wpId: "WP-TEST-STEER-v1",
    role: "CODER",
    sessionKey: "CODER:WP-TEST-STEER-v1",
    localBranch: "feat/WP-TEST-STEER-v1",
    localWorktreeDir: "../wtc-test",
    absWorktreeDir: "D:/Handshake/Handshake Worktrees/wtc-test",
    selectedModel: ROLE_SESSION_PRIMARY_MODEL,
    prompt: "Continue the governed coder lane.",
    threadId: "thread_test",
    outputJsonlFile: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/test.jsonl",
  });

  assert.equal(request.busy_ingress_mode, "ENQUEUE_ON_BUSY");
});
