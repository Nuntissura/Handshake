import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync, spawn } from "node:child_process";
import {
  SESSION_BATCH_MODE_CLI_ESCALATION,
  CLI_ESCALATION_HOST_DEFAULT,
  CLI_ESCALATION_HOST_LEGACY_ALIAS,
  CLI_SESSION_TOOL,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  ROLE_SESSION_FALLBACK_MODEL,
  SESSION_HOST_PREFERENCE,
  SESSION_HOST_FALLBACK,
  SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
  SESSION_PLUGIN_BRIDGE_COMMAND,
  SESSION_PLUGIN_BRIDGE_ID,
  SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import {
  assertOrchestratorLaunchAuthority,
  buildLaunchRequest,
  ensureSessionStateFiles,
  getOrCreateSessionRecord,
  loadSessionLaunchRequests,
  loadSessionRegistry,
  markPluginResult,
  pendingRequestStatus,
  queuePluginLaunch,
  registryBatchLaunchSummary,
  registrySessionSummary,
  settleTimedOutPluginRequests,
  markCliEscalationUsed,
  mutateSessionRegistrySync,
} from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  launchOwnedSystemTerminal,
  recordOwnedTerminalLaunch,
} from "../../../roles_shared/scripts/session/terminal-ownership-lib.mjs";
import {
  buildRoleEnvironmentOverrides,
  buildStartupPrompt,
  resolveRoleConfig,
  selectModel,
} from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { evaluateSessionGovernanceState } from "../../../roles_shared/scripts/session/session-governance-state-lib.mjs";
import { GOV_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const requestedHost = String(process.argv[4] || "").trim().toUpperCase() || "AUTO";
const requestedModel = String(process.argv[5] || "").trim().toUpperCase() || "PRIMARY";
const sessionControlCommandPath = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-command.mjs");

function fail(message) {
  console.error(`[LAUNCH_CLI_SESSION] ${message}`);
  process.exit(1);
}

if (!wpId || !wpId.startsWith("WP-")) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/launch-cli-session.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID> [AUTO|PRINT|CURRENT|${CLI_ESCALATION_HOST_DEFAULT}|${CLI_ESCALATION_HOST_LEGACY_ALIAS}|VSCODE_PLUGIN|VSCODE] [PRIMARY|FALLBACK]`);
}
if (!["PRIMARY", "FALLBACK"].includes(requestedModel)) {
  fail(`Invalid model selector: ${requestedModel} (expected PRIMARY or FALLBACK)`);
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

const roleConfig = resolveRoleConfig(role, wpId);
if (!roleConfig) fail(`Unknown role: ${role}`);
const selectedModel = selectModel(requestedModel);
const launchEnvironmentOverrides = buildRoleEnvironmentOverrides({ role });

const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
const currentBranch = runGit(["branch", "--show-current"]);
assertOrchestratorLaunchAuthority(currentBranch);
const absWorktreeDir = path.resolve(repoRoot, roleConfig.worktreeDir);
const sessionDescriptor = {
  wp_id: wpId,
  role,
  local_branch: roleConfig.branch,
  local_worktree_dir: roleConfig.worktreeDir,
  terminal_title: roleConfig.title,
  requested_model: selectedModel,
  reasoning_config_key: ROLE_SESSION_REASONING_CONFIG_KEY,
  reasoning_config_value: ROLE_SESSION_REASONING_CONFIG_VALUE,
};
const governance = evaluateSessionGovernanceState(repoRoot, sessionDescriptor);

if (!governance.launchAllowed) {
  fail(`Governed session ${role}:${wpId} cannot be launched: ${governance.launchBlockers.join("; ")}`);
}

if (!fs.existsSync(absWorktreeDir)) {
  execFileSync(
    process.execPath,
    [path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "role-session-worktree-add.mjs"), role, wpId, roleConfig.branch, roleConfig.worktreeDir],
    { stdio: "inherit" },
  );
}

const prompt = buildStartupPrompt({ role, wpId, roleConfig, selectedModel });

const codexArgs = [
  "-m",
  selectedModel,
  "-c",
  `${ROLE_SESSION_REASONING_CONFIG_KEY}="${ROLE_SESSION_REASONING_CONFIG_VALUE}"`,
  "-C",
  absWorktreeDir,
  prompt,
];

function psQuote(value) {
  return `'${String(value).replace(/'/g, "''")}'`;
}

function writeLaunchScript() {
  const psPath = path.join(os.tmpdir(), `handshake-${role.toLowerCase()}-${wpId}-${Date.now()}.ps1`);
  const psArgsLines = codexArgs.map((arg) => `  ${psQuote(arg)}`).join(",\r\n");
  const envLines = Object.entries(launchEnvironmentOverrides)
    .map(([key, value]) => `$env:${key} = ${psQuote(value)}`)
    .join("\r\n");
  const script = [
    `$ErrorActionPreference = 'Stop'`,
    envLines,
    `Set-Location -LiteralPath ${psQuote(absWorktreeDir)}`,
    `$Host.UI.RawUI.WindowTitle = ${psQuote(roleConfig.title)}`,
    `$codexArgs = @(`,
    psArgsLines,
    `)`,
    `& ${psQuote(CLI_SESSION_TOOL)} @codexArgs`,
  ].join("\r\n");
  fs.writeFileSync(psPath, script, "utf8");
  return psPath;
}

const launchScriptPath = writeLaunchScript();
const codexCommand = `& ${psQuote(launchScriptPath)}`;

function launchCurrent() {
  const child = spawn(CLI_SESSION_TOOL, codexArgs, {
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
  console.log(`[LAUNCH_CLI_SESSION] launched via ${CLI_ESCALATION_HOST_DEFAULT} (${roleConfig.title})`);
  console.log(`[LAUNCH_CLI_SESSION] worktree=${absWorktreeDir}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_model=${selectedModel}`);
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
  console.log(`[LAUNCH_CLI_SESSION] launch_script=${launchScriptPath}`);
  if (Object.keys(launchEnvironmentOverrides).length > 0) {
    console.log(`[LAUNCH_CLI_SESSION] env_overrides=${JSON.stringify(launchEnvironmentOverrides)}`);
  }
  console.log(`[LAUNCH_CLI_SESSION] startup=${roleConfig.startupCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] next=${roleConfig.nextCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] command=${codexCommand}`);
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

function autoStartGovernedSession(reason) {
  refreshSessionSummary();
  if (!needsGovernedAutoStart()) {
    console.log(`[LAUNCH_CLI_SESSION] auto_start=SKIPPED`);
    console.log(`[LAUNCH_CLI_SESSION] auto_start_reason=${reason}`);
    return;
  }
  console.log(`[LAUNCH_CLI_SESSION] auto_start=START_SESSION`);
  console.log(`[LAUNCH_CLI_SESSION] auto_start_reason=${reason}`);
  execFileSync(
    process.execPath,
    [sessionControlCommandPath, "START_SESSION", role, wpId, "", requestedModel],
    { stdio: "inherit" },
  );
  refreshSessionSummary();
}

function queueVsCodePluginLaunch() {
  const result = mutateSessionRegistrySync(repoRoot, (registry) => {
    const { requests } = loadSessionLaunchRequests(repoRoot);
    settleTimedOutPluginRequests(registry, requests);
    const session = getOrCreateSessionRecord(registry, sessionDescriptor);
    const currentBatchSummary = registryBatchLaunchSummary(registry);
    if (currentBatchSummary.batch_cli_escalation_active) {
      return {
        status: "batch_cli",
        summary: registrySessionSummary(session),
        batchSummary: currentBatchSummary,
      };
    }
    const pending = session.plugin_last_request_id ? pendingRequestStatus(registry, session.plugin_last_request_id) : null;
    const lastRequestAtMs = Date.parse(session.plugin_last_request_at || "");
    const hasFreshPendingRequest =
      session.plugin_last_result === "QUEUED" &&
      !pending &&
      !Number.isNaN(lastRequestAtMs) &&
      (Date.now() - lastRequestAtMs) < (SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS * 1000);
    if (hasFreshPendingRequest) {
      return {
        status: "pending",
        summary: registrySessionSummary(session),
        batchSummary: registryBatchLaunchSummary(registry),
        requestId: session.plugin_last_request_id,
      };
    }
    const request = buildLaunchRequest({
      wpId,
      role,
      localBranch: roleConfig.branch,
      localWorktreeDir: roleConfig.worktreeDir,
      absWorktreeDir,
      selectedModel,
      reasoningConfigKey: ROLE_SESSION_REASONING_CONFIG_KEY,
      reasoningConfigValue: ROLE_SESSION_REASONING_CONFIG_VALUE,
      startupCommand: roleConfig.startupCommand,
      nextCommand: roleConfig.nextCommand,
      terminalTitleValue: roleConfig.title,
      command: codexCommand,
      pluginAttemptNumber: session.plugin_request_count + 1,
    });
    queuePluginLaunch(repoRoot, registry, request);
    return {
      status: "queued",
      summary: registrySessionSummary(session),
      batchSummary: registryBatchLaunchSummary(registry),
      requestId: request.request_id,
    };
  });
  sessionSummary = result.summary;
  batchSummary = result.batchSummary;
  if (result.status === "batch_cli") {
    console.log("[LAUNCH_CLI_SESSION] plugin launch is disabled for the current governed batch");
    printSessionSummary();
    return;
  }
  if (result.status === "pending") {
    console.log(`[LAUNCH_CLI_SESSION] plugin launch request still pending for ${SESSION_PLUGIN_BRIDGE_ID}`);
    console.log(`[LAUNCH_CLI_SESSION] request_id=${result.requestId}`);
    console.log(`[LAUNCH_CLI_SESSION] wait_timeout_seconds=${SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS}`);
    autoStartGovernedSession("plugin launch request already pending");
    printSessionSummary();
    return;
  }
  console.log(`[LAUNCH_CLI_SESSION] queued plugin launch request for ${SESSION_PLUGIN_BRIDGE_ID}`);
  console.log(`[LAUNCH_CLI_SESSION] request_id=${result.requestId}`);
  console.log(`[LAUNCH_CLI_SESSION] preferred_host=${SESSION_HOST_PREFERENCE}`);
  console.log(`[LAUNCH_CLI_SESSION] plugin_command=${SESSION_PLUGIN_BRIDGE_COMMAND}`);
  console.log(`[LAUNCH_CLI_SESSION] wait_timeout_seconds=${SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS}`);
  autoStartGovernedSession("plugin launch request queued");
  printSessionSummary();
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

if (requestedHost === "AUTO" || requestedHost === "VSCODE_PLUGIN" || requestedHost === "VSCODE") {
  if (!cliEscalationGatePassed()) {
    queueVsCodePluginLaunch();
    process.exit(0);
  }
  if (requestedHost !== "AUTO") {
    printOnly(
      batchSummary.launch_batch_mode === SESSION_BATCH_MODE_CLI_ESCALATION
        ? `Batch launch mode is ${SESSION_BATCH_MODE_CLI_ESCALATION}; use AUTO/CURRENT/${CLI_ESCALATION_HOST_DEFAULT} until the governed batch is reset`
        : `Plugin launch has already failed ${sessionSummary.plugin_failure_count} time(s); CLI escalation is now allowed and should be used from AUTO/CURRENT/${CLI_ESCALATION_HOST_DEFAULT}`,
      "PRINT",
    );
    printSessionSummary();
    process.exit(0);
  }
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
  launchSystemTerminal();
  autoStartGovernedSession("system terminal launch");
  printSessionSummary();
  process.exit(0);
}

if (requestedHost === "AUTO") {
  maybeRecordCliEscalation(CLI_ESCALATION_HOST_DEFAULT);
  launchSystemTerminal();
  autoStartGovernedSession("auto-resolved system terminal launch");
  printSessionSummary();
  process.exit(0);
}

printOnly(
  `Unsupported host mode; use AUTO, VSCODE_PLUGIN, CURRENT, ${CLI_ESCALATION_HOST_DEFAULT}, or PRINT (${CLI_ESCALATION_HOST_LEGACY_ALIAS} remains a legacy alias)`,
  "PRINT",
);
printSessionSummary();
