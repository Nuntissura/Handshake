import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
  CLI_ESCALATION_HOST_DEFAULT,
  CLI_SESSION_TOOL,
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  ROLE_SESSION_FALLBACK_MODEL,
  SESSION_HOST_PREFERENCE,
  roleNextCommand,
  roleStartupCommand,
  SESSION_HOST_FALLBACK,
  SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
  SESSION_PLUGIN_BRIDGE_COMMAND,
  SESSION_PLUGIN_BRIDGE_ID,
  SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
} from "./session-policy.mjs";
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
  registrySessionSummary,
  saveSessionRegistry,
  settleTimedOutPluginRequests,
} from "./session-registry-lib.mjs";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const requestedHost = String(process.argv[4] || "").trim().toUpperCase() || "AUTO";
const requestedModel = String(process.argv[5] || "").trim().toUpperCase() || "PRIMARY";

function fail(message) {
  console.error(`[LAUNCH_CLI_SESSION] ${message}`);
  process.exit(1);
}

if (!wpId || !wpId.startsWith("WP-")) {
  fail("Usage: node .GOV/scripts/launch-cli-session.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID> [AUTO|PRINT|CURRENT|WINDOWS_TERMINAL|VSCODE_PLUGIN|VSCODE] [PRIMARY|FALLBACK]");
}
if (!["PRIMARY", "FALLBACK"].includes(requestedModel)) {
  fail(`Invalid model selector: ${requestedModel} (expected PRIMARY or FALLBACK)`);
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function commandExists(command) {
  const lookup = process.platform === "win32" ? "where.exe" : "which";
  const result = spawnSync(lookup, [command], { stdio: "ignore" });
  return result.status === 0;
}

function resolveRoleConfig(roleName, workPacketId) {
  if (roleName === "CODER") {
    return {
      branch: defaultCoderBranch(workPacketId),
      worktreeDir: defaultCoderWorktreeDir(workPacketId),
      title: `CODER ${workPacketId}`,
      startupCommand: roleStartupCommand("CODER"),
      nextCommand: roleNextCommand("CODER", workPacketId),
      focus: "implementation, governance paperwork, and coder-side delegation only when the packet allows it",
    };
  }
  if (roleName === "WP_VALIDATOR") {
    return {
      branch: defaultWpValidatorBranch(workPacketId),
      worktreeDir: defaultWpValidatorWorktreeDir(workPacketId),
      title: `WPVAL ${workPacketId}`,
      startupCommand: roleStartupCommand("WP_VALIDATOR"),
      nextCommand: roleNextCommand("WP_VALIDATOR", workPacketId),
      focus: "advisory technical review, steering, and packet-scoped validation receipts",
    };
  }
  if (roleName === "INTEGRATION_VALIDATOR") {
    return {
      branch: defaultIntegrationValidatorBranch(workPacketId),
      worktreeDir: defaultIntegrationValidatorWorktreeDir(workPacketId),
      title: `INTVAL ${workPacketId}`,
      startupCommand: roleStartupCommand("INTEGRATION_VALIDATOR"),
      nextCommand: roleNextCommand("INTEGRATION_VALIDATOR", workPacketId),
      focus: "final technical verdict, merge authority, and main/origin integration decisions only after validation handoff",
    };
  }
  return null;
}

const roleConfig = resolveRoleConfig(role, wpId);
if (!roleConfig) fail(`Unknown role: ${role}`);
const selectedModel = requestedModel === "FALLBACK" ? ROLE_SESSION_FALLBACK_MODEL : ROLE_SESSION_PRIMARY_MODEL;

const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
const currentBranch = runGit(["branch", "--show-current"]);
assertOrchestratorLaunchAuthority(currentBranch);
const absWorktreeDir = path.resolve(repoRoot, roleConfig.worktreeDir);

if (!fs.existsSync(absWorktreeDir)) {
  execFileSync(
    process.execPath,
    [path.join(".GOV", "scripts", "role-session-worktree-add.mjs"), role, wpId, roleConfig.branch, roleConfig.worktreeDir],
    { stdio: "inherit" },
  );
}

const prompt = [
  `ROLE LOCK: You are the ${role}.`,
  `WP_ID: ${wpId}`,
  `WORKTREE: ${roleConfig.worktreeDir}`,
  `BRANCH: ${roleConfig.branch}`,
  `FIRST COMMAND: ${roleConfig.startupCommand}`,
  `AFTER STARTUP: ${roleConfig.nextCommand}`,
  `AUTHORITY: AGENTS.md + startup output + the role protocol + .GOV/task_packets/${wpId}.md`,
  `FOCUS: ${roleConfig.focus}.`,
  `MODEL POLICY: selected ${selectedModel}; primary ${ROLE_SESSION_PRIMARY_MODEL} with ${ROLE_SESSION_REASONING_CONFIG_KEY}=${ROLE_SESSION_REASONING_CONFIG_VALUE}; fallback ${ROLE_SESSION_FALLBACK_MODEL} with the same reasoning value if primary is unavailable.`,
  `REPO POLICY: do not switch to Codex model aliases for repo-governed sessions.`,
  `REMINDER: the Orchestrator remains workflow authority; only the Integration Validator can own merge-to-main authority.`,
].join("\n");

const codexArgs = [
  "-m",
  selectedModel,
  "-c",
  `${ROLE_SESSION_REASONING_CONFIG_KEY}="${ROLE_SESSION_REASONING_CONFIG_VALUE}"`,
  "-C",
  absWorktreeDir,
  prompt,
];

const codexCommand = `${CLI_SESSION_TOOL} ${codexArgs.map((part) => JSON.stringify(part)).join(" ")}`;

function writeLaunchScript() {
  const psPath = path.join(os.tmpdir(), `handshake-${role.toLowerCase()}-${wpId}-${Date.now()}.ps1`);
  const script = [
    `$ErrorActionPreference = 'Stop'`,
    `Set-Location -LiteralPath '${absWorktreeDir.replace(/'/g, "''")}'`,
    `${CLI_SESSION_TOOL} -m ${selectedModel} -c '${ROLE_SESSION_REASONING_CONFIG_KEY}="${ROLE_SESSION_REASONING_CONFIG_VALUE}"' -C '${absWorktreeDir.replace(/'/g, "''")}' @'`,
    prompt,
    `'@`,
  ].join("\r\n");
  fs.writeFileSync(psPath, script, "utf8");
  return psPath;
}

function launchCurrent() {
  const child = spawn(CLI_SESSION_TOOL, codexArgs, {
    cwd: absWorktreeDir,
    stdio: "inherit",
    shell: false,
  });
  child.on("exit", (code) => process.exit(code ?? 0));
}

function launchWindowsTerminal() {
  const launchScript = writeLaunchScript();
  const child = spawn(
    "wt.exe",
    [
      "new-tab",
      "--title",
      roleConfig.title,
      "powershell.exe",
      "-NoLogo",
      "-NoExit",
      "-File",
      launchScript,
    ],
    {
      cwd: repoRoot,
      detached: true,
      stdio: "ignore",
    },
  );
  child.unref();
  console.log(`[LAUNCH_CLI_SESSION] launched via WINDOWS_TERMINAL (${roleConfig.title})`);
  console.log(`[LAUNCH_CLI_SESSION] worktree=${absWorktreeDir}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_model=${selectedModel}`);
  console.log(`[LAUNCH_CLI_SESSION] startup=${roleConfig.startupCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] next=${roleConfig.nextCommand}`);
}

function printOnly(reason, resolvedHost) {
  console.log(`[LAUNCH_CLI_SESSION] host_preference=${SESSION_HOST_PREFERENCE}`);
  console.log(`[LAUNCH_CLI_SESSION] host_fallback=${SESSION_HOST_FALLBACK}`);
  console.log(`[LAUNCH_CLI_SESSION] host_resolved=${resolvedHost}`);
  console.log(`[LAUNCH_CLI_SESSION] reason=${reason}`);
  console.log(`[LAUNCH_CLI_SESSION] worktree=${absWorktreeDir}`);
  console.log(`[LAUNCH_CLI_SESSION] branch=${roleConfig.branch}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_model=${selectedModel}`);
  console.log(`[LAUNCH_CLI_SESSION] startup=${roleConfig.startupCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] next=${roleConfig.nextCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] command=${codexCommand}`);
}

ensureSessionStateFiles(repoRoot);
const { registry } = loadSessionRegistry(repoRoot);
const { requests } = loadSessionLaunchRequests(repoRoot);
settleTimedOutPluginRequests(registry, requests);
const session = getOrCreateSessionRecord(registry, {
  wp_id: wpId,
  role,
  local_branch: roleConfig.branch,
  local_worktree_dir: roleConfig.worktreeDir,
  terminal_title: roleConfig.title,
  requested_model: selectedModel,
  reasoning_config_key: ROLE_SESSION_REASONING_CONFIG_KEY,
  reasoning_config_value: ROLE_SESSION_REASONING_CONFIG_VALUE,
});
saveSessionRegistry(repoRoot, registry);

function printSessionSummary() {
  const summary = registrySessionSummary(session);
  console.log(`[LAUNCH_CLI_SESSION] session_key=${summary.session_key}`);
  console.log(`[LAUNCH_CLI_SESSION] runtime_state=${summary.runtime_state}`);
  console.log(`[LAUNCH_CLI_SESSION] plugin_request_count=${summary.plugin_request_count}`);
  console.log(`[LAUNCH_CLI_SESSION] plugin_failure_count=${summary.plugin_failure_count}`);
  console.log(`[LAUNCH_CLI_SESSION] plugin_last_result=${summary.plugin_last_result}`);
  console.log(`[LAUNCH_CLI_SESSION] cli_escalation_allowed=${summary.cli_escalation_allowed ? "YES" : "NO"}`);
}

function queueVsCodePluginLaunch() {
  const pending = session.plugin_last_request_id ? pendingRequestStatus(registry, session.plugin_last_request_id) : null;
  const lastRequestAtMs = Date.parse(session.plugin_last_request_at || "");
  const hasFreshPendingRequest =
    session.plugin_last_result === "QUEUED" &&
    !pending &&
    !Number.isNaN(lastRequestAtMs) &&
    (Date.now() - lastRequestAtMs) < (SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS * 1000);
  if (hasFreshPendingRequest) {
    console.log(`[LAUNCH_CLI_SESSION] plugin launch request still pending for ${SESSION_PLUGIN_BRIDGE_ID}`);
    console.log(`[LAUNCH_CLI_SESSION] request_id=${session.plugin_last_request_id}`);
    console.log(`[LAUNCH_CLI_SESSION] wait_timeout_seconds=${SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS}`);
    printSessionSummary();
    return;
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
  console.log(`[LAUNCH_CLI_SESSION] queued plugin launch request for ${SESSION_PLUGIN_BRIDGE_ID}`);
  console.log(`[LAUNCH_CLI_SESSION] request_id=${request.request_id}`);
  console.log(`[LAUNCH_CLI_SESSION] preferred_host=${SESSION_HOST_PREFERENCE}`);
  console.log(`[LAUNCH_CLI_SESSION] plugin_command=${SESSION_PLUGIN_BRIDGE_COMMAND}`);
  console.log(`[LAUNCH_CLI_SESSION] wait_timeout_seconds=${SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS}`);
  printSessionSummary();
}

function cliEscalationGatePassed() {
  return session.cli_escalation_allowed || session.plugin_failure_count >= SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION;
}

function maybeRecordCliEscalation(hostKind) {
  const pending = session.plugin_last_request_id ? pendingRequestStatus(registry, session.plugin_last_request_id) : null;
  if (!pending && session.plugin_last_request_id) {
    markPluginResult(registry, session, session.plugin_last_request_id, "CLI_ESCALATION_USED", {
      host_kind: hostKind,
      terminal_title: roleConfig.title,
    });
  } else {
    session.runtime_state = "CLI_ESCALATION_USED";
    session.active_host = hostKind;
    session.active_terminal_title = roleConfig.title;
    session.active_terminal_kind = hostKind;
    session.cli_escalation_used = true;
    session.last_event_at = new Date().toISOString();
  }
  saveSessionRegistry(repoRoot, registry);
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
      `Plugin launch has already failed ${session.plugin_failure_count} time(s); CLI escalation is now allowed and should be used from AUTO/CURRENT/WINDOWS_TERMINAL`,
      "PRINT",
    );
    printSessionSummary();
    process.exit(0);
  }
}

if ((requestedHost === "CURRENT" || requestedHost === "WINDOWS_TERMINAL") && !cliEscalationGatePassed()) {
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

if (requestedHost === "WINDOWS_TERMINAL") {
  if (!commandExists("wt")) {
    printOnly("wt.exe is unavailable on this host", "PRINT");
    printSessionSummary();
    process.exit(0);
  }
  maybeRecordCliEscalation("WINDOWS_TERMINAL");
  launchWindowsTerminal();
  printSessionSummary();
  process.exit(0);
}

if (requestedHost === "AUTO") {
  if (commandExists("wt")) {
    maybeRecordCliEscalation("WINDOWS_TERMINAL");
    launchWindowsTerminal();
    printSessionSummary();
    process.exit(0);
  }
  maybeRecordCliEscalation("PRINT");
  printOnly("Plugin retry budget exhausted and no CLI window host is available; run the printed command manually in an escalation window", "PRINT");
  printSessionSummary();
  process.exit(0);
}

printOnly("Unsupported host mode; use AUTO, VSCODE_PLUGIN, CURRENT, WINDOWS_TERMINAL, or PRINT", "PRINT");
printSessionSummary();
