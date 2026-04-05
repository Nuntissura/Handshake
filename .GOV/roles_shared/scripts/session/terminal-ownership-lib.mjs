import { execFileSync } from "node:child_process";
import {
  ensureActiveTerminalBatch,
  getOrCreateSessionRecord,
  loadSessionRegistry,
  mutateSessionRegistrySync,
} from "./session-registry-lib.mjs";
import {
  CLI_ESCALATION_HOST_DEFAULT,
  SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION,
  SESSION_TERMINAL_RECLAIM_STATUS_ALREADY_EXITED,
  SESSION_TERMINAL_RECLAIM_STATUS_FAILED,
  SESSION_TERMINAL_RECLAIM_STATUS_NONE,
  SESSION_TERMINAL_RECLAIM_STATUS_OWNED,
  SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED,
} from "./session-policy.mjs";

function nowIso() {
  return new Date().toISOString();
}

function psQuote(value) {
  return `'${String(value || "").replace(/'/g, "''")}'`;
}

function defaultInspectProcess(processId) {
  try {
    const output = execFileSync(
      "powershell.exe",
      [
        "-NoLogo",
        "-NonInteractive",
        "-Command",
        `$proc = Get-Process -Id ${Number(processId)} -ErrorAction SilentlyContinue; if ($proc) { 'RUNNING' } else { 'MISSING' }`,
      ],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    ).trim();
    return output === "RUNNING";
  } catch {
    return false;
  }
}

function defaultStopProcess(processId) {
  execFileSync(
    "powershell.exe",
    [
      "-NoLogo",
      "-NonInteractive",
      "-Command",
      `Stop-Process -Id ${Number(processId)} -Force -ErrorAction Stop`,
    ],
    { stdio: ["ignore", "pipe", "pipe"] },
  );
}

export function launchOwnedSystemTerminal({
  worktreeAbs,
  launchScriptPath,
  terminalTitle,
  runner = execFileSync,
} = {}) {
  const output = runner(
    "powershell.exe",
    [
      "-NoLogo",
      "-NonInteractive",
      "-Command",
      [
        `$proc = Start-Process -FilePath 'powershell.exe'`,
        `  -WorkingDirectory ${psQuote(worktreeAbs)}`,
        `  -ArgumentList @('-NoLogo','-NoExit','-File',${psQuote(launchScriptPath)})`,
        "  -WindowStyle Normal",
        "  -PassThru;",
        "Write-Output $proc.Id",
      ].join(" "),
    ],
    { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  ).trim();

  const processId = Number.parseInt(String(output || "").split(/\r?\n/).at(-1) || "", 10);
  if (!Number.isInteger(processId) || processId <= 0) {
    throw new Error(`failed to resolve launched terminal pid from output: ${output || "<empty>"}`);
  }

  return {
    processId,
    hostKind: CLI_ESCALATION_HOST_DEFAULT,
    terminalTitle: String(terminalTitle || "").trim(),
  };
}

export function recordOwnedTerminalLaunch(repoRoot, sessionDescriptor, {
  processId = 0,
  hostKind = CLI_ESCALATION_HOST_DEFAULT,
  terminalTitle = "",
} = {}) {
  if (!Number.isInteger(processId) || processId <= 0) return null;
  return mutateSessionRegistrySync(repoRoot, (registry) => {
    const session = getOrCreateSessionRecord(registry, sessionDescriptor);
    const activeBatch = ensureActiveTerminalBatch(registry, {
      reason: `governed terminal launch for ${session.session_key}`,
      currentSessionKey: session.session_key,
    });
    session.terminal_ownership_scope = SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION;
    session.owned_terminal_process_id = processId;
    session.owned_terminal_host_kind = hostKind || CLI_ESCALATION_HOST_DEFAULT;
    session.owned_terminal_window_title = terminalTitle || session.terminal_title || "";
    session.owned_terminal_batch_scope = activeBatch.terminal_batch_scope || "";
    session.owned_terminal_batch_id = activeBatch.terminal_batch_id || "";
    session.owned_terminal_recorded_at = nowIso();
    registry.active_terminal_batch_claimed_at = registry.active_terminal_batch_claimed_at || session.owned_terminal_recorded_at;
    session.owned_terminal_reclaimed_at = "";
    session.owned_terminal_reclaim_status = SESSION_TERMINAL_RECLAIM_STATUS_OWNED;
    return {
      session_key: session.session_key,
      owned_terminal_process_id: session.owned_terminal_process_id,
      owned_terminal_batch_id: session.owned_terminal_batch_id,
    };
  });
}

function matchesSelector(session, selector = {}) {
  if (selector.sessionKey && String(session.session_key || "") !== String(selector.sessionKey || "")) return false;
  if (selector.role && String(session.role || "").toUpperCase() !== String(selector.role || "").toUpperCase()) return false;
  if (selector.wpId && String(session.wp_id || "") !== String(selector.wpId || "")) return false;
  if (selector.terminalBatchId && String(session.owned_terminal_batch_id || "") !== String(selector.terminalBatchId || "").toUpperCase()) return false;
  return true;
}

export function reclaimOwnedSessionTerminals(repoRoot, selector = {}, {
  inspectProcess = defaultInspectProcess,
  stopProcess = defaultStopProcess,
} = {}) {
  const { registry } = loadSessionRegistry(repoRoot);
  const candidates = (registry.sessions || [])
    .filter((session) => matchesSelector(session, selector))
    .filter((session) => Number.isInteger(session.owned_terminal_process_id) && session.owned_terminal_process_id > 0)
    .filter((session) => String(session.terminal_ownership_scope || "") === SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION)
    .filter((session) => String(session.owned_terminal_reclaim_status || "") !== SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED);

  const results = [];
  for (const session of candidates) {
    const processId = session.owned_terminal_process_id;
    let reclaimStatus = SESSION_TERMINAL_RECLAIM_STATUS_NONE;
    let error = "";
    try {
      if (!inspectProcess(processId)) {
        reclaimStatus = SESSION_TERMINAL_RECLAIM_STATUS_ALREADY_EXITED;
      } else {
        stopProcess(processId);
        reclaimStatus = SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED;
      }
    } catch (caught) {
      reclaimStatus = SESSION_TERMINAL_RECLAIM_STATUS_FAILED;
      error = String(caught?.message || caught || "");
    }

    mutateSessionRegistrySync(repoRoot, (nextRegistry) => {
      const nextSession = (nextRegistry.sessions || []).find((entry) => String(entry.session_key || "") === String(session.session_key || ""));
      if (!nextSession) return null;
      nextSession.owned_terminal_reclaim_status = reclaimStatus;
      nextSession.owned_terminal_reclaimed_at = nowIso();
      if (reclaimStatus !== SESSION_TERMINAL_RECLAIM_STATUS_FAILED) {
        nextSession.owned_terminal_process_id = 0;
      }
      if (error) {
        nextSession.last_error = error;
      }
      return null;
    });

    results.push({
      session_key: session.session_key,
      process_id: processId,
      terminal_batch_id: session.owned_terminal_batch_id || "",
      reclaim_status: reclaimStatus,
      error,
    });
  }
  return results;
}
