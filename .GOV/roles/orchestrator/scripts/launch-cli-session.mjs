import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync, spawn } from "node:child_process";
import { registerFailCaptureHook } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import {
  CLI_ESCALATION_HOST_DEFAULT,
  CLI_ESCALATION_HOST_LEGACY_ALIAS,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  SESSION_HOST_PREFERENCE,
  SESSION_HOST_FALLBACK,
  SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import {
  assertOrchestratorLaunchAuthority,
  ensureSessionStateFiles,
  getOrCreateSessionRecord,
  loadSessionLaunchRequests,
  loadSessionRegistry,
  markPluginResult,
  pendingRequestStatus,
  registryBatchLaunchSummary,
  registrySessionSummary,
  settleTimedOutPluginRequests,
  markCliEscalationUsed,
  mutateSessionRegistrySync,
} from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  launchOwnedSystemTerminal,
  recordOwnedTerminalLaunch,
  reclaimOwnedSessionTerminals,
} from "../../../roles_shared/scripts/session/terminal-ownership-lib.mjs";
import {
  buildRoleEnvironmentOverrides,
  buildStartupPrompt,
  resolveCliToolForProfile,
  resolveRoleConfig,
  resolveRoleLaunchSelection,
  assertRoleLaunchProfileSupported,
} from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { evaluateSessionGovernanceState } from "../../../roles_shared/scripts/session/session-governance-state-lib.mjs";
import { GOV_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { capturePreTaskSnapshot } from "../../../roles_shared/scripts/memory/memory-snapshot.mjs";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const requestedHost = String(process.argv[4] || "").trim().toUpperCase() || "AUTO";
const requestedModel = String(process.argv[5] || "").trim().toUpperCase() || "PRIMARY";
const sessionControlCommandPath = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-command.mjs");
const debugMode = process.argv.slice(6).some((arg) => String(arg || "").trim() === "--debug");
const sessionControlEnv = {
  ...process.env,
  ...(debugMode ? { HANDSHAKE_SESSION_CONTROL_DEBUG: "1" } : {}),
};

registerFailCaptureHook("launch-cli-session.mjs", { role: "ORCHESTRATOR" });

function fail(message) {
  console.error(`[LAUNCH_CLI_SESSION] ${message}`);
  process.exit(1);
}

if (!wpId || !wpId.startsWith("WP-")) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/launch-cli-session.mjs <ACTIVATION_MANAGER|CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR|MEMORY_MANAGER> <WP_ID> [AUTO|PRINT|CURRENT|${CLI_ESCALATION_HOST_DEFAULT}|${CLI_ESCALATION_HOST_LEGACY_ALIAS}|VSCODE_PLUGIN|VSCODE] [PRIMARY|FALLBACK]`);
}
if (!["PRIMARY", "FALLBACK"].includes(requestedModel)) {
  fail(`Invalid model selector: ${requestedModel} (expected PRIMARY or FALLBACK)`);
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], windowsHide: true }).trim();
}

const roleConfig = resolveRoleConfig(role, wpId);
if (!roleConfig) fail(`Unknown role: ${role}`);
const launchEnvironmentOverrides = buildRoleEnvironmentOverrides({ role });

const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
const currentBranch = runGit(["branch", "--show-current"]);
assertOrchestratorLaunchAuthority(currentBranch);
const absWorktreeDir = path.resolve(repoRoot, roleConfig.worktreeDir);
const {
  selectedProfileId,
  selectedProfile,
} = resolveRoleLaunchSelection({
  role,
  wpId,
  modelSelector: requestedModel,
});
assertRoleLaunchProfileSupported({
  role,
  wpId,
  selectedProfileId,
  selectedProfile,
});
const selectedModel = selectedProfile.launch_model;
const sessionDescriptor = {
  wp_id: wpId,
  role,
  local_branch: roleConfig.branch,
  local_worktree_dir: roleConfig.worktreeDir,
  terminal_title: roleConfig.title,
  requested_model: selectedModel,
  requested_profile_id: selectedProfileId,
  reasoning_config_key: selectedProfile.launch_reasoning_config_key || ROLE_SESSION_REASONING_CONFIG_KEY,
  reasoning_config_value: selectedProfile.launch_reasoning_config_value || ROLE_SESSION_REASONING_CONFIG_VALUE,
};
const governance = evaluateSessionGovernanceState(repoRoot, sessionDescriptor);

// RGF-145: pre-task snapshot before WP delegation
capturePreTaskSnapshot({
  snapshotType: "PRE_WP_DELEGATION",
  wpId,
  triggerScript: "launch-cli-session.mjs",
  context: {
    role,
    selectedModel,
    selectedProfileId,
    provider: selectedProfile.provider,
    branch: roleConfig.branch,
    worktreeDir: roleConfig.worktreeDir,
    requestedHost,
    launchAllowed: governance.launchAllowed,
    launchBlockers: governance.launchBlockers || [],
  },
});

if (!governance.launchAllowed) {
  fail(`Governed session ${role}:${wpId} cannot be launched: ${governance.launchBlockers.join("; ")}`);
}

if (!fs.existsSync(absWorktreeDir)) {
  execFileSync(
    process.execPath,
    [path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "role-session-worktree-add.mjs"), role, wpId, roleConfig.branch, roleConfig.worktreeDir],
    { stdio: "inherit", windowsHide: true },
  );
}

const prompt = buildStartupPrompt({
  role,
  wpId,
  roleConfig,
  selectedModel,
  selectedProfileId,
  selectedProfile,
});

const isClaudeCode = selectedProfile.provider === "ANTHROPIC";
const isOllama = selectedProfile.provider === "OLLAMA_LOCAL";
const cliTool = resolveCliToolForProfile(selectedProfile);

function buildCodexArgs() {
  return [
    "-m",
    selectedModel,
    "-c",
    `${selectedProfile.launch_reasoning_config_key || ROLE_SESSION_REASONING_CONFIG_KEY}="${selectedProfile.launch_reasoning_config_value || ROLE_SESSION_REASONING_CONFIG_VALUE}"`,
    "-C",
    absWorktreeDir,
    prompt,
  ];
}

function buildClaudeCodeArgs() {
  return [
    "--model", selectedModel,
    "--effort", selectedProfile.launch_reasoning_config_value || ROLE_SESSION_REASONING_CONFIG_VALUE,
    "--dangerously-skip-permissions",
    prompt,
  ];
}

function buildOllamaArgs() {
  return [
    "run",
    selectedModel,
  ];
}

const cliArgs = isClaudeCode ? buildClaudeCodeArgs() : (isOllama ? buildOllamaArgs() : buildCodexArgs());

if (debugMode) {
  console.log("[LAUNCH_CLI_SESSION] debug_mode=enabled");
}

function psQuote(value) {
  return `'${String(value).replace(/'/g, "''")}'`;
}

function writeLaunchScript() {
  const psPath = path.join(os.tmpdir(), `handshake-${role.toLowerCase()}-${wpId}-${Date.now()}.ps1`);
  const psArgsLines = cliArgs.map((arg) => `  ${psQuote(arg)}`).join(",\r\n");
  const envLines = Object.entries(launchEnvironmentOverrides)
    .map(([key, value]) => `$env:${key} = ${psQuote(value)}`)
    .join("\r\n");
  const script = [
    `$ErrorActionPreference = 'Stop'`,
    envLines,
    `Set-Location -LiteralPath ${psQuote(absWorktreeDir)}`,
    `$targetWindowTitle = ${psQuote(roleConfig.title)}`,
    `try {`,
    `  [Console]::Title = $targetWindowTitle`,
    `} catch {`,
    `  Write-Host "[LAUNCH_CLI_SESSION] Unable to set terminal title"`,
    `}`,
    `$cliArgs = @(`,
    psArgsLines,
    `)`,
    `& ${psQuote(cliTool)} @cliArgs`,
  ].join("\r\n");
  fs.writeFileSync(psPath, script, "utf8");
  return psPath;
}

const launchScriptPath = writeLaunchScript();
const launchCommand = `& ${psQuote(launchScriptPath)}`;

function launchCurrent() {
  const child = spawn(cliTool, cliArgs, {
    cwd: absWorktreeDir,
    env: {
      ...process.env,
      ...launchEnvironmentOverrides,
    },
    stdio: "inherit",
    shell: false,
  });
  child.on("exit", (code) => process.exit(code ?? 0));
}

function launchSystemTerminal() {
  const launch = launchOwnedSystemTerminal({
    worktreeAbs: absWorktreeDir,
    launchScriptPath,
    terminalTitle: roleConfig.title,
  });
  recordOwnedTerminalLaunch(repoRoot, sessionDescriptor, {
    processId: launch.processId,
    hostKind: launch.hostKind,
    terminalTitle: roleConfig.title,
  });
  console.log(`[LAUNCH_CLI_SESSION] launched hidden via ${CLI_ESCALATION_HOST_DEFAULT} (${roleConfig.title})`);
  console.log(`[LAUNCH_CLI_SESSION] worktree=${absWorktreeDir}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_model=${selectedModel}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_profile_id=${selectedProfileId}`);
  console.log(`[LAUNCH_CLI_SESSION] startup=${roleConfig.startupCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] next=${roleConfig.nextCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] terminal_pid=${launch.processId}`);
}

function printOnly(reason, resolvedHost) {
  console.log(`[LAUNCH_CLI_SESSION] host_preference=${SESSION_HOST_PREFERENCE}`);
  console.log(`[LAUNCH_CLI_SESSION] host_fallback=${SESSION_HOST_FALLBACK}`);
  console.log(`[LAUNCH_CLI_SESSION] host_resolved=${resolvedHost}`);
  console.log(`[LAUNCH_CLI_SESSION] reason=${reason}`);
  console.log(`[LAUNCH_CLI_SESSION] worktree=${absWorktreeDir}`);
  console.log(`[LAUNCH_CLI_SESSION] branch=${roleConfig.branch}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_model=${selectedModel}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_profile_id=${selectedProfileId}`);
  console.log(`[LAUNCH_CLI_SESSION] launch_script=${launchScriptPath}`);
  if (Object.keys(launchEnvironmentOverrides).length > 0) {
    console.log(`[LAUNCH_CLI_SESSION] env_overrides=${JSON.stringify(launchEnvironmentOverrides)}`);
  }
  console.log(`[LAUNCH_CLI_SESSION] startup=${roleConfig.startupCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] next=${roleConfig.nextCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] command=${launchCommand}`);
}

function printLaunchResolution(reason, resolvedHost) {
  console.log(`[LAUNCH_CLI_SESSION] host_preference=${SESSION_HOST_PREFERENCE}`);
  console.log(`[LAUNCH_CLI_SESSION] host_fallback=${SESSION_HOST_FALLBACK}`);
  console.log(`[LAUNCH_CLI_SESSION] host_resolved=${resolvedHost}`);
  console.log(`[LAUNCH_CLI_SESSION] reason=${reason}`);
}

ensureSessionStateFiles(repoRoot);
let { sessionSummary, batchSummary } = mutateSessionRegistrySync(repoRoot, (registry) => {
  const { requests } = loadSessionLaunchRequests(repoRoot);
  settleTimedOutPluginRequests(registry, requests);
  const session = getOrCreateSessionRecord(registry, sessionDescriptor);
  return {
    sessionSummary: registrySessionSummary(session),
    batchSummary: registryBatchLaunchSummary(registry),
  };
});

function printSessionSummary() {
  console.log(`[LAUNCH_CLI_SESSION] session_key=${sessionSummary.session_key}`);
  console.log(`[LAUNCH_CLI_SESSION] runtime_state=${sessionSummary.runtime_state}`);
  console.log(`[LAUNCH_CLI_SESSION] startup_proof_state=${sessionSummary.startup_proof_state}`);
  console.log(`[LAUNCH_CLI_SESSION] thread_id=${sessionSummary.session_thread_id || "<none>"}`);
  console.log(`[LAUNCH_CLI_SESSION] plugin_request_count=${sessionSummary.plugin_request_count}`);
  console.log(`[LAUNCH_CLI_SESSION] plugin_failure_count=${sessionSummary.plugin_failure_count}`);
  console.log(`[LAUNCH_CLI_SESSION] plugin_last_result=${sessionSummary.plugin_last_result}`);
  console.log(`[LAUNCH_CLI_SESSION] cli_escalation_allowed=${sessionSummary.cli_escalation_allowed ? "YES" : "NO"}`);
  console.log(`[LAUNCH_CLI_SESSION] launch_batch_mode=${batchSummary.launch_batch_mode}`);
  console.log(`[LAUNCH_CLI_SESSION] launch_batch_plugin_failure_count=${batchSummary.launch_batch_plugin_failure_count}`);
  console.log(`[LAUNCH_CLI_SESSION] active_terminal_batch_id=${batchSummary.active_terminal_batch_id || "<none>"}`);
  if (batchSummary.launch_batch_switched_at) {
    console.log(`[LAUNCH_CLI_SESSION] launch_batch_switched_at=${batchSummary.launch_batch_switched_at}`);
  }
  if (batchSummary.launch_batch_switch_reason) {
    console.log(`[LAUNCH_CLI_SESSION] launch_batch_switch_reason=${batchSummary.launch_batch_switch_reason}`);
  }
}

function reclaimLaunchedTerminal(context = "") {
  const targetSessionKey = String(sessionSummary?.session_key || "").trim();
  if (!targetSessionKey) return;
  if (debugMode) {
    const tag = context ? `[${context}] ` : "";
    console.log(`[LAUNCH_CLI_SESSION] ${tag}debug_mode=enabled skip_terminal_reclaim`);
    return;
  }
  try {
    const reclaimResults = reclaimOwnedSessionTerminals(repoRoot, { sessionKey: targetSessionKey });
    if (reclaimResults.length === 0) return;
    const tag = context ? `[${context}] ` : "";
    for (const reclaim of reclaimResults) {
      console.log(`[LAUNCH_CLI_SESSION] ${tag}terminal_reclaim_session=${reclaim.session_key}`);
      console.log(`[LAUNCH_CLI_SESSION] ${tag}terminal_reclaim_process_id=${reclaim.process_id}`);
      console.log(`[LAUNCH_CLI_SESSION] ${tag}terminal_reclaim_status=${reclaim.reclaim_status}`);
      if (reclaim.error) console.log(`[LAUNCH_CLI_SESSION] ${tag}terminal_reclaim_error=${reclaim.error}`);
    }
  } catch (error) {
    console.log(`[LAUNCH_CLI_SESSION] terminal_reclaim_error=${String(error?.message || error || "").slice(0, 200)}`);
  }
}

function refreshSessionSummary() {
  const registry = loadSessionRegistry(repoRoot).registry;
  const refreshedSession = (registry.sessions || []).find((entry) => entry.session_key === sessionSummary.session_key);
  if (refreshedSession) {
    sessionSummary = registrySessionSummary(refreshedSession);
  }
  batchSummary = registryBatchLaunchSummary(registry);
}

function needsGovernedAutoStart(summary = sessionSummary) {
  const runtimeState = String(summary?.runtime_state || "").trim().toUpperCase();
  const startupProofState = String(summary?.startup_proof_state || "").trim().toUpperCase();
  const threadId = String(summary?.session_thread_id || "").trim();
  const lastCommandKind = String(summary?.last_command_kind || "").trim().toUpperCase();
  const lastCommandStatus = String(summary?.last_command_status || "").trim().toUpperCase();
  if (threadId || startupProofState === "READY") return false;
  if (["READY", "COMMAND_RUNNING", "ACTIVE", "WAITING"].includes(runtimeState)) return false;
  if (lastCommandKind === "START_SESSION" && ["QUEUED", "RUNNING"].includes(lastCommandStatus)) return false;
  return true;
}

function previewOutput(value, maxChars = 1400) {
  const output = String(value || "").replace(/\r/g, "");
  return output.length <= maxChars ? output : `${output.slice(0, maxChars)}...`;
}

function isHarmlessAutoStartFailure(output) {
  const normalized = String(output || "").toLowerCase();
  if (!normalized) return false;
  return [
    "outcome_state=already_ready",
    "outcome_state=busy_active_run",
    "already has thread",
    "cannot be started",
    "governed session",
    "usage limit",
    "quota exceeded",
    "credits",
  ].some((token) => normalized.includes(token));
}

function autoStartGovernedSession(reason) {
  refreshSessionSummary();
  if (!needsGovernedAutoStart()) {
    console.log(`[LAUNCH_CLI_SESSION] auto_start=SKIPPED`);
    console.log(`[LAUNCH_CLI_SESSION] auto_start_reason=${reason}`);
    return;
  }
  console.log(`[LAUNCH_CLI_SESSION] auto_start=START_SESSION`);
  console.log(`[LAUNCH_CLI_SESSION] auto_start_reason=${reason}`);
  try {
    const output = execFileSync(
      process.execPath,
      [sessionControlCommandPath, "START_SESSION", role, wpId, "", requestedModel],
      {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        env: sessionControlEnv,
        windowsHide: true,
      },
    );
    if (output?.trim()) {
      console.log(`[LAUNCH_CLI_SESSION] auto_start_output=${previewOutput(output)}`);
    }
  } catch (error) {
    const output = `${error.stdout || ""}${error.stderr || ""}${error.message || ""}`;
    const normalizedOutput = previewOutput(output);
    console.log(`[LAUNCH_CLI_SESSION] auto_start_error=${normalizedOutput}`);
    if (isHarmlessAutoStartFailure(output)) {
      console.log("[LAUNCH_CLI_SESSION] auto_start_failure_expected=DEFERRED");
      refreshSessionSummary();
      return;
    }
    throw error;
  }
  refreshSessionSummary();
}

function cliEscalationGatePassed() {
  return batchSummary.batch_cli_escalation_active
    || sessionSummary.cli_escalation_allowed
    || sessionSummary.plugin_failure_count >= SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION;
}

function isSystemTerminalMode(host) {
  return host === CLI_ESCALATION_HOST_DEFAULT || host === CLI_ESCALATION_HOST_LEGACY_ALIAS;
}

function maybeRecordCliEscalation(hostKind) {
  ({ sessionSummary, batchSummary } = mutateSessionRegistrySync(repoRoot, (registry) => {
    const session = getOrCreateSessionRecord(registry, sessionDescriptor);
    const pending = session.plugin_last_request_id ? pendingRequestStatus(registry, session.plugin_last_request_id) : null;
    if (!pending && session.plugin_last_request_id) {
      markPluginResult(registry, session, session.plugin_last_request_id, "CLI_ESCALATION_USED", {
        host_kind: hostKind,
        terminal_title: roleConfig.title,
      });
    } else {
      markCliEscalationUsed(session, {
        hostKind,
        terminalTitle: roleConfig.title,
      });
    }
    return {
      sessionSummary: registrySessionSummary(session),
      batchSummary: registryBatchLaunchSummary(registry),
    };
  }));
}

if (requestedHost === "PRINT") {
  printOnly("print-only requested", "PRINT");
  printSessionSummary();
  process.exit(0);
}

if (requestedHost === "AUTO") {
  printLaunchResolution("auto-resolved governed ACP headless launch", SESSION_HOST_PREFERENCE);
  autoStartGovernedSession("auto headless ACP direct launch");
  printSessionSummary();
  process.exit(0);
}

if (requestedHost === "VSCODE_PLUGIN" || requestedHost === "VSCODE") {
  printOnly(
    "VSCODE_PLUGIN launch is disabled by the headless-only role-session policy; use AUTO for ACP direct launch or PRINT for debug output",
    "PRINT",
  );
  printSessionSummary();
  process.exit(1);
}

if ((requestedHost === "CURRENT" || isSystemTerminalMode(requestedHost)) && !cliEscalationGatePassed()) {
  printOnly(
    `CLI escalation is blocked until ${SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION} plugin failures/timeouts have been recorded for this ${role}/${wpId} session`,
    "PRINT",
  );
  printSessionSummary();
  process.exit(1);
}

if (requestedHost === "CURRENT") {
  maybeRecordCliEscalation("CURRENT");
  launchCurrent();
  process.exit(0);
}

if (isSystemTerminalMode(requestedHost)) {
  maybeRecordCliEscalation(CLI_ESCALATION_HOST_DEFAULT);
  try {
    launchSystemTerminal();
    autoStartGovernedSession("system terminal launch");
  } catch (error) {
    reclaimLaunchedTerminal("system terminal launch failure");
    throw error;
  }
  printSessionSummary();
  process.exit(0);
}

printOnly(
  `Unsupported host mode; use AUTO, CURRENT, ${CLI_ESCALATION_HOST_DEFAULT}, or PRINT (${CLI_ESCALATION_HOST_LEGACY_ALIAS} remains a legacy alias; VSCODE_PLUGIN is disabled by headless-only policy)`,
  "PRINT",
);
printSessionSummary();
