/**
 * fail-capture-lib.mjs - Shared fail capture for governance scripts.
 *
 * Two modes:
 *   A) Explicit: call failWithMemory() as a drop-in replacement for fail()
 *   B) Hook: import registerFailCaptureHook() at script top to auto-capture on exit(1)
 *
 * Both write procedural memories to the governance DB (best-effort, never blocks).
 * These memories are surfaced automatically via memory-recall before future actions.
 */

import { openGovernanceMemoryDb, addMemory, closeDb } from "../memory/governance-memory-lib.mjs";

/**
 * Best-effort write a procedural memory for a script failure.
 * Never throws, never blocks - if memory write fails, the original error still propagates.
 */
function captureFailure(scriptName, message, { wpId = "", role = "", details = [] } = {}) {
  try {
    const { db } = openGovernanceMemoryDb();
    try {
      const detailStr = details.length > 0 ? ` Details: ${details.join("; ")}` : "";
      const content = `Script ${scriptName} failed: ${message}${detailStr}`.slice(0, 500);
      addMemory(db, {
        memoryType: "procedural",
        topic: `Script failure: ${scriptName} - ${message.slice(0, 60)}`,
        summary: content,
        wpId,
        importance: 0.7,
        content,
        sourceArtifact: "fail-capture",
        sourceRole: role,
        metadata: { script: scriptName, captured_at: new Date().toISOString() },
      });
    } finally {
      closeDb(db);
    }
  } catch {
    // best-effort - never block the script on memory failure
  }
}

/**
 * Drop-in replacement for the per-script fail() pattern.
 * Captures to memory, then exits.
 *
 * Usage:
 *   import { failWithMemory } from "../lib/fail-capture-lib.mjs";
 *   failWithMemory("orchestrator-next.mjs", "packet not found", { wpId, role: "ORCHESTRATOR" });
 */
export function failWithMemory(scriptName, message, { wpId = "", role = "", details = [], exitCode = 1 } = {}) {
  _terminalFailureCaptured = true;
  captureFailure(scriptName, message, { wpId, role, details });
  console.error(`[${scriptName}] ${message}`);
  for (const d of details) console.error(`  - ${d}`);
  process.exit(exitCode);
}

/**
 * Register a process exit hook that captures failures on non-zero exit.
 * Call once at the top of a script. Captures the last stderr line as context.
 *
 * Usage:
 *   import { registerFailCaptureHook } from "../lib/fail-capture-lib.mjs";
 *   registerFailCaptureHook("orchestrator-next.mjs", { role: "ORCHESTRATOR" });
 */
const _registered = new Set();
let _lastError = "";
let _lastOutput = "";
let _activeHookContext = {
  scriptName: "unknown-script",
  wpId: "",
  role: "",
  captureExitCodes: new Set([1]),
};
let _processHooksRegistered = false;
let _stdioHooksRegistered = false;
let _terminalFailureCaptured = false;

function currentHookContext() {
  return _activeHookContext;
}

function noteOutput(chunk) {
  const text = Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk || "");
  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  if (lines.length > 0) _lastOutput = lines.at(-1);
}

function ensureStdioHooksRegistered() {
  if (_stdioHooksRegistered) return;
  _stdioHooksRegistered = true;

  const originalStdoutWrite = process.stdout.write.bind(process.stdout);
  process.stdout.write = function writeStdoutWithCapture(chunk, ...args) {
    noteOutput(chunk);
    return originalStdoutWrite(chunk, ...args);
  };

  const originalStderrWrite = process.stderr.write.bind(process.stderr);
  process.stderr.write = function writeStderrWithCapture(chunk, ...args) {
    noteOutput(chunk);
    _lastError = _lastOutput;
    return originalStderrWrite(chunk, ...args);
  };
}

function ensureProcessHooksRegistered() {
  if (_processHooksRegistered) return;
  _processHooksRegistered = true;
  ensureStdioHooksRegistered();

  process.on("uncaughtException", (err) => {
    const { scriptName, wpId, role } = currentHookContext();
    _lastError = err?.message || String(err);
    _terminalFailureCaptured = true;
    captureFailure(scriptName, _lastError, { wpId, role, details: [err?.stack?.split("\n")[1]?.trim() || ""] });
    console.error(`[${scriptName}] Uncaught: ${_lastError}`);
    process.exit(1);
  });

  process.on("unhandledRejection", (reason) => {
    const { scriptName, wpId, role } = currentHookContext();
    _lastError = reason?.message || String(reason);
    _terminalFailureCaptured = true;
    captureFailure(scriptName, `Unhandled rejection: ${_lastError}`, { wpId, role });
    console.error(`[${scriptName}] Unhandled rejection: ${_lastError}`);
    process.exit(1);
  });

  process.on("exit", (code) => {
    const { scriptName, wpId, role, captureExitCodes } = currentHookContext();
    const exitCode = Number(code);
    const captures = captureExitCodes instanceof Set ? captureExitCodes : new Set([1]);
    if (_terminalFailureCaptured || !captures.has(exitCode)) return;
    const message = `Process exited with code ${exitCode}${_lastError || _lastOutput ? `: ${_lastError || _lastOutput}` : ""}`;
    _terminalFailureCaptured = true;
    captureFailure(scriptName, message, { wpId, role });
  });
}

export function registerFailCaptureHook(scriptName, { wpId = "", role = "", captureExitCodes = [1] } = {}) {
  _activeHookContext = {
    scriptName,
    wpId,
    role,
    captureExitCodes: new Set(captureExitCodes),
  };
  if (_registered.has(scriptName)) return; // idempotent per script
  _registered.add(scriptName);
  ensureProcessHooksRegistered();
}

export { captureFailure };
