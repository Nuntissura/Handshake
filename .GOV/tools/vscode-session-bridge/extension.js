const vscode = require("vscode");
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const GOVERNANCE_RUNTIME_ROOT_ENV_VAR = "HANDSHAKE_GOV_RUNTIME_ROOT";
const PRODUCT_RUNTIME_ROOT_ENV_VAR = "HANDSHAKE_RUNTIME_ROOT";
const REGISTRY_LOCK_FILE_NAME = "ROLE_SESSION_REGISTRY.lock";
const REGISTRY_LOCK_WAIT_MS = 5000;
const REGISTRY_LOCK_STALE_MS = 60000;
const REGISTRY_LOCK_POLL_MS = 50;

function nowIso() {
  return new Date().toISOString();
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function looksLikeRepoRoot(candidate) {
  return !!candidate
    && fs.existsSync(path.join(candidate, ".GOV"))
    && fs.existsSync(path.join(candidate, "justfile"));
}

function findRepoRoot(startPath) {
  let current = startPath;
  if (!current) return "";
  if (fs.existsSync(current) && fs.statSync(current).isFile()) {
    current = path.dirname(current);
  }
  while (current) {
    if (looksLikeRepoRoot(current)) return current;
    const parent = path.dirname(current);
    if (!parent || parent === current) break;
    current = parent;
  }
  return "";
}

function getRepoRoot() {
  const candidates = [];
  const activeFile = vscode.window.activeTextEditor?.document?.uri?.fsPath || "";
  if (activeFile) candidates.push(activeFile);
  for (const folder of vscode.workspace.workspaceFolders || []) {
    candidates.push(folder.uri.fsPath);
  }
  for (const candidate of candidates) {
    const repoRoot = findRepoRoot(candidate);
    if (repoRoot) return repoRoot;
  }
  return "";
}

function readPersistedUserEnv(name) {
  if (process.platform !== "win32") return "";
  try {
    return execFileSync(
      "powershell.exe",
      ["-NoLogo", "-NonInteractive", "-Command", `[Environment]::GetEnvironmentVariable('${name}','User')`],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }
    ).trim();
  } catch {
    return "";
  }
}

function normalizeRelativePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function resolveGovernanceRuntimeRoot(repoRoot) {
  const directValue = String(
    process.env[GOVERNANCE_RUNTIME_ROOT_ENV_VAR]
    || readPersistedUserEnv(GOVERNANCE_RUNTIME_ROOT_ENV_VAR)
    || ""
  ).trim();
  if (directValue) return path.resolve(directValue);

  const productRuntimeRoot = String(
    process.env[PRODUCT_RUNTIME_ROOT_ENV_VAR]
    || readPersistedUserEnv(PRODUCT_RUNTIME_ROOT_ENV_VAR)
    || ""
  ).trim();
  if (productRuntimeRoot) return path.resolve(productRuntimeRoot, "repo-governance");

  return path.resolve(repoRoot, "..", "..", "Handshake Runtime", "repo-governance");
}

function governanceRuntimeFile(repoRoot, ...segments) {
  return path.resolve(resolveGovernanceRuntimeRoot(repoRoot), "roles_shared", ...segments);
}

function governanceRuntimeRepoRelativePath(repoRoot, ...segments) {
  return normalizeRelativePath(path.relative(repoRoot, governanceRuntimeFile(repoRoot, ...segments)));
}

function queuePath(repoRoot) {
  return governanceRuntimeFile(repoRoot, "SESSION_LAUNCH_REQUESTS.jsonl");
}

function registryPath(repoRoot) {
  return governanceRuntimeFile(repoRoot, "ROLE_SESSION_REGISTRY.json");
}

function controlResultsPath(repoRoot) {
  return governanceRuntimeFile(repoRoot, "SESSION_CONTROL_RESULTS.jsonl");
}

function wpCommunicationsRoot(repoRoot) {
  return governanceRuntimeFile(repoRoot, "WP_COMMUNICATIONS");
}

function ensureDirForWatcher(targetPath, isDirectory = false) {
  const dirPath = isDirectory ? targetPath : path.dirname(targetPath);
  fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(filePath, fallbackValue) {
  if (!fs.existsSync(filePath)) return JSON.parse(JSON.stringify(fallbackValue));
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const tempPath = `${filePath}.${process.pid}.${Date.now()}.tmp`;
  fs.writeFileSync(tempPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  fs.renameSync(tempPath, filePath);
}

function parseJsonl(filePath) {
  if (!fs.existsSync(filePath)) return [];
  return fs
    .readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function defaultRegistry(repoRoot) {
  return {
    schema_id: "hsk.role_session_registry@1",
    schema_version: "role_session_registry_v1",
    updated_at: nowIso(),
    session_start_authority: "ORCHESTRATOR_ONLY",
    session_host_preference: "VSCODE_EXTENSION_TERMINAL",
    session_host_fallback: "CLI_ESCALATION_WINDOW",
    session_watch_policy: "EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK",
    session_plugin_bridge_id: "handshake.handshake-session-bridge",
    session_plugin_bridge_command: "handshakeSessionBridge.processLaunchQueue",
    session_plugin_requests_file: governanceRuntimeRepoRelativePath(repoRoot, "SESSION_LAUNCH_REQUESTS.jsonl"),
    session_control_mode: "STEERABLE",
    session_control_transport_primary: "CODEX_EXEC_RESUME_JSON",
    session_control_protocol_primary: "HANDSHAKE_ACP_STDIO_V1",
    session_control_requests_file: governanceRuntimeRepoRelativePath(repoRoot, "SESSION_CONTROL_REQUESTS.jsonl"),
    session_control_results_file: governanceRuntimeRepoRelativePath(repoRoot, "SESSION_CONTROL_RESULTS.jsonl"),
    session_control_output_dir: governanceRuntimeRepoRelativePath(repoRoot, "SESSION_CONTROL_OUTPUTS"),
    session_wake_channel_primary: "VS_CODE_FILE_WATCH",
    session_wake_channel_fallback: "WP_HEARTBEAT",
    session_plugin_max_retries_before_escalation: 2,
    session_plugin_attempt_timeout_seconds: 20,
    sessions: [],
    processed_requests: []
  };
}

function loadRegistry(repoRoot) {
  return readJson(registryPath(repoRoot), defaultRegistry(repoRoot));
}

function saveRegistry(repoRoot, registry) {
  registry.updated_at = nowIso();
  writeJson(registryPath(repoRoot), registry);
}

function registryLockPath(repoRoot) {
  return path.join(path.dirname(registryPath(repoRoot)), REGISTRY_LOCK_FILE_NAME);
}

function isLockFileStale(lockPath) {
  try {
    const stats = fs.statSync(lockPath);
    return (Date.now() - stats.mtimeMs) > REGISTRY_LOCK_STALE_MS;
  } catch {
    return false;
  }
}

async function withFileLock(lockPath, fn) {
  fs.mkdirSync(path.dirname(lockPath), { recursive: true });
  const deadline = Date.now() + REGISTRY_LOCK_WAIT_MS;
  while (true) {
    try {
      const fd = fs.openSync(lockPath, "wx");
      try {
        fs.writeFileSync(fd, JSON.stringify({ pid: process.pid, created_at: nowIso() }));
        return await fn();
      } finally {
        try {
          fs.closeSync(fd);
        } catch {
          // ignore
        }
        try {
          fs.unlinkSync(lockPath);
        } catch {
          // ignore
        }
      }
    } catch (error) {
      if (error?.code !== "EEXIST") throw error;
      if (isLockFileStale(lockPath)) {
        try {
          fs.unlinkSync(lockPath);
          continue;
        } catch {
          // another writer won the cleanup race
        }
      }
      if (Date.now() >= deadline) {
        throw new Error(`Timed out waiting for session registry lock: ${lockPath}`);
      }
      await sleep(REGISTRY_LOCK_POLL_MS);
    }
  }
}

async function mutateRegistry(repoRoot, mutator) {
  return withFileLock(registryLockPath(repoRoot), async () => {
    const registry = loadRegistry(repoRoot);
    const result = await mutator(registry);
    saveRegistry(repoRoot, registry);
    return result;
  });
}

function processedRequest(registry, requestId) {
  return (registry.processed_requests || []).find((entry) => entry.request_id === requestId) || null;
}

function upsertProcessedRequest(registry, requestId, status, extra = {}) {
  const record = {
    request_id: requestId,
    status,
    processed_at: nowIso(),
    ...extra
  };
  const existing = processedRequest(registry, requestId);
  if (existing) {
    Object.assign(existing, record);
    return existing;
  }
  registry.processed_requests.push(record);
  return record;
}

function sessionKey(request) {
  return request.session_key || `${String(request.role || "").trim().toUpperCase()}:${request.wp_id}`;
}

function upsertSession(registry, request, patch) {
  const key = sessionKey(request);
  let session = (registry.sessions || []).find((entry) => entry.session_key === key);
  if (!session) {
    session = {
      session_key: key,
      session_id: key.toLowerCase().replace(/[^a-z0-9:_-]/g, "-"),
      wp_id: request.wp_id,
      role: request.role,
      launch_authority: request.launch_authority || "ORCHESTRATOR_ONLY",
      preferred_host: request.preferred_host || "VSCODE_EXTENSION_TERMINAL",
      fallback_host: request.fallback_host || "CLI_ESCALATION_WINDOW",
      cli_escalation_host: "SYSTEM_TERMINAL",
      local_branch: request.local_branch || "",
      local_worktree_dir: request.local_worktree_dir || "",
      terminal_title: request.terminal_title || "",
      requested_model: request.selected_model || "",
      reasoning_config_key: request.reasoning_config_key || "",
      reasoning_config_value: request.reasoning_config_value || "",
      plugin_request_count: 0,
      plugin_failure_count: 0,
      plugin_last_request_id: "",
      plugin_last_request_at: "",
      plugin_last_result: "NONE",
      plugin_last_error: "",
      cli_escalation_allowed: false,
      cli_escalation_used: false,
      runtime_state: "UNSTARTED",
      active_host: "NONE",
      active_terminal_title: "",
      active_terminal_kind: "",
      last_heartbeat_at: "",
      last_error: "",
      last_event_at: ""
    };
    registry.sessions.push(session);
  }
  Object.assign(session, patch);
  return session;
}

function validateRequest(request) {
  const errors = [];
  if (request.schema_id !== "hsk.session_launch_request@1") errors.push("schema_id mismatch");
  if (request.schema_version !== "session_launch_request_v1") errors.push("schema_version mismatch");
  if (request.launch_authority !== "ORCHESTRATOR_ONLY") errors.push("launch_authority must be ORCHESTRATOR_ONLY");
  if (request.preferred_host !== "VSCODE_EXTENSION_TERMINAL") errors.push("preferred_host must be VSCODE_EXTENSION_TERMINAL");
  if (!request.command) errors.push("command missing");
  if (!request.local_worktree_dir && !request.abs_worktree_dir) errors.push("local_worktree_dir missing");
  return errors;
}

function resolveLaunchCwd(repoRoot, request) {
  const relativeWorktree = String(request.local_worktree_dir || "").trim();
  if (relativeWorktree) return path.resolve(repoRoot, relativeWorktree);
  const absoluteWorktree = String(request.abs_worktree_dir || "").trim();
  if (absoluteWorktree) return absoluteWorktree;
  return repoRoot;
}

function getOrCreateTerminal(title, cwd) {
  const existing = vscode.window.terminals.find((terminal) => terminal.name === title);
  if (existing) return existing;
  return vscode.window.createTerminal({
    name: title,
    cwd
  });
}

async function processLaunchQueue() {
  const repoRoot = getRepoRoot();
  if (!repoRoot) return;

  const queueFilePath = queuePath(repoRoot);
  ensureDirForWatcher(queueFilePath);
  const requests = parseJsonl(queueFilePath);
  const launchFailures = [];

  await mutateRegistry(repoRoot, async (registry) => {
    for (const request of requests) {
      if (!request || processedRequest(registry, request.request_id)) continue;

      const sessionPatch = {
        wp_id: request.wp_id,
        role: request.role,
        local_branch: request.local_branch || "",
        local_worktree_dir: request.local_worktree_dir || "",
        terminal_title: request.terminal_title || "",
        requested_model: request.selected_model || "",
        reasoning_config_key: request.reasoning_config_key || "",
        reasoning_config_value: request.reasoning_config_value || "",
        plugin_request_count: Number(request.plugin_attempt_number || 1),
        plugin_last_request_id: request.request_id,
        plugin_last_request_at: request.created_at || nowIso(),
        last_event_at: nowIso()
      };
      const session = upsertSession(registry, request, sessionPatch);
      const requestErrors = validateRequest(request);

      if (requestErrors.length > 0) {
        session.plugin_failure_count += 1;
        session.plugin_last_result = "PLUGIN_FAILED";
        session.plugin_last_error = requestErrors.join("; ");
        session.last_error = session.plugin_last_error;
        session.runtime_state = session.plugin_failure_count >= 2 ? "CLI_ESCALATION_READY" : "UNSTARTED";
        session.cli_escalation_allowed = session.plugin_failure_count >= 2;
        upsertProcessedRequest(registry, request.request_id, "PLUGIN_FAILED", {
          error: session.plugin_last_error
        });
        continue;
      }

      try {
        const terminal = getOrCreateTerminal(request.terminal_title, resolveLaunchCwd(repoRoot, request));
        terminal.show(true);
        terminal.sendText(request.command, true);
        session.plugin_last_result = "PLUGIN_DISPATCHED";
        session.plugin_last_error = "";
        session.runtime_state = "TERMINAL_COMMAND_DISPATCHED";
        session.active_host = "VSCODE_EXTENSION_TERMINAL";
        session.active_terminal_title = request.terminal_title;
        session.active_terminal_kind = "VSCODE_EXTENSION_TERMINAL";
        session.last_error = "";
        session.last_event_at = nowIso();
        upsertProcessedRequest(registry, request.request_id, "PLUGIN_DISPATCHED", {
          terminal_title: request.terminal_title
        });
      } catch (error) {
        session.plugin_failure_count += 1;
        session.plugin_last_result = "PLUGIN_FAILED";
        session.plugin_last_error = error && error.message ? error.message : String(error);
        session.last_error = session.plugin_last_error;
        session.runtime_state = session.plugin_failure_count >= 2 ? "CLI_ESCALATION_READY" : "UNSTARTED";
        session.cli_escalation_allowed = session.plugin_failure_count >= 2;
        session.last_event_at = nowIso();
        upsertProcessedRequest(registry, request.request_id, "PLUGIN_FAILED", {
          error: session.plugin_last_error
        });
        launchFailures.push(`Handshake session launch failed for ${request.role} ${request.wp_id}: ${session.plugin_last_error}`);
      }
    }
  });

  for (const message of launchFailures) {
    void vscode.window.showWarningMessage(message);
  }
}

function installRuntimeStatusWatcher(context) {
  const repoRoot = getRepoRoot();
  if (!repoRoot) return;
  const lastSeen = new Map();
  const basePath = wpCommunicationsRoot(repoRoot);
  ensureDirForWatcher(basePath, true);
  const watcher = vscode.workspace.createFileSystemWatcher(
    new vscode.RelativePattern(vscode.Uri.file(basePath), "**/RUNTIME_STATUS.json")
  );
  const handle = async (uri) => {
    try {
      const text = fs.readFileSync(uri.fsPath, "utf8");
      const runtime = JSON.parse(text);
      const signature = JSON.stringify({
        runtime_status: runtime.runtime_status,
        current_phase: runtime.current_phase,
        next_expected_actor: runtime.next_expected_actor,
        validator_trigger: runtime.validator_trigger,
        waiting_on: runtime.waiting_on
      });
      if (lastSeen.get(uri.fsPath) === signature) return;
      lastSeen.set(uri.fsPath, signature);
      if (runtime.validator_trigger && runtime.validator_trigger !== "NONE") {
        void vscode.window.showInformationMessage(
          `Handshake validator wake-up: ${path.basename(path.dirname(uri.fsPath))} -> ${runtime.validator_trigger}`
        );
      }
    } catch {
      // Ignore malformed runtime status notifications; governance checks remain authoritative.
    }
  };
  watcher.onDidChange(handle, null, context.subscriptions);
  watcher.onDidCreate(handle, null, context.subscriptions);
  context.subscriptions.push(watcher);
}

function installLaunchQueueWatcher(context) {
  const repoRoot = getRepoRoot();
  if (!repoRoot) return;
  const queueFilePath = queuePath(repoRoot);
  ensureDirForWatcher(queueFilePath);
  const watcher = vscode.workspace.createFileSystemWatcher(
    new vscode.RelativePattern(vscode.Uri.file(path.dirname(queueFilePath)), path.basename(queueFilePath))
  );
  const handle = async () => {
    await processLaunchQueue();
  };
  watcher.onDidChange(handle, null, context.subscriptions);
  watcher.onDidCreate(handle, null, context.subscriptions);
  context.subscriptions.push(watcher);
}

function installSessionControlResultsWatcher(context) {
  const repoRoot = getRepoRoot();
  if (!repoRoot) return;
  let lastSeenCommandId = "";
  const resultsFilePath = controlResultsPath(repoRoot);
  ensureDirForWatcher(resultsFilePath);
  const watcher = vscode.workspace.createFileSystemWatcher(
    new vscode.RelativePattern(vscode.Uri.file(path.dirname(resultsFilePath)), path.basename(resultsFilePath))
  );
  const handle = async () => {
    try {
      const results = parseJsonl(resultsFilePath);
      const latest = results.at(-1);
      if (!latest || latest.command_id === lastSeenCommandId) return;
      lastSeenCommandId = latest.command_id;
      void vscode.window.showInformationMessage(
        `Handshake session command ${latest.status}: ${latest.session_key} (${latest.command_kind})`
      );
    } catch {
      // Ignore malformed control-result notifications; registry and logs remain authoritative.
    }
  };
  watcher.onDidChange(handle, null, context.subscriptions);
  watcher.onDidCreate(handle, null, context.subscriptions);
  context.subscriptions.push(watcher);
}

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("handshakeSessionBridge.processLaunchQueue", async () => {
      await processLaunchQueue();
      void vscode.window.showInformationMessage("Handshake launch queue processed.");
    }),
    vscode.commands.registerCommand("handshakeSessionBridge.openSessionRegistry", async () => {
      const repoRoot = getRepoRoot();
      if (!repoRoot) return;
      const uri = vscode.Uri.file(registryPath(repoRoot));
      const document = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(document, { preview: false });
    })
  );

  installRuntimeStatusWatcher(context);
  installLaunchQueueWatcher(context);
  installSessionControlResultsWatcher(context);
  void processLaunchQueue();
}

function deactivate() {}

module.exports = {
  activate,
  deactivate
};
