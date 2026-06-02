import { spawn, spawnSync } from "node:child_process";
import path from "node:path";

import type { Page } from "playwright";

/**
 * MT-027 — real focus-audit driver for the a2 visual smoke.
 *
 * HBR-QUIET-001 forbids any Handshake-owned window from stealing the
 * foreground while a model is working. The a2 smoke proves this by running the
 * real `operator_foreground::focus_audit` ledger around the visual capture and
 * asserting the genuine `handshake_owned_events` vector is empty.
 *
 * There are two honest execution paths, never a hardcoded `[]`:
 *
 *   1. Live WebView2 app: when the page exposes the Tauri bridge
 *      (`__TAURI__.core.invoke`), the driver calls the real IPC commands
 *      `kernel_operator_foreground_focus_audit_start` / `_stop`, which wrap the
 *      same core `FocusAuditHandle`.
 *   2. Headless capture fixtures: when no Tauri bridge is present (the default
 *      Playwright capture harness uses `fixture:` content), the driver shells
 *      out to the `focus-audit-probe` Rust binary, which runs the **same** core
 *      `FocusAuditHandle::start`/`stop` over the run_id and prints the real
 *      `FocusAuditReport` JSON.
 *
 * On a host without the Win32 foreground hook (the Linux dev lane), the core
 * audit returns `FOCUS_AUDIT_UNSUPPORTED_PLATFORM`. The driver reports that as
 * an unsupported result rather than fabricating an empty report; the smoke
 * gates on it honestly.
 */

export type FocusAuditEvent = {
  run_id: string;
  timestamp_utc: string;
  hwnd: string;
  pid: number;
  exe_name: string | null;
  expected_foreground: boolean;
};

export type FocusAuditReport = {
  run_id: string;
  total_events: number;
  handshake_owned_events: FocusAuditEvent[];
  foreign_events: FocusAuditEvent[];
  expected_foreground_events: FocusAuditEvent[];
};

export type FocusAuditOutcome =
  | { kind: "report"; source: "tauri_ipc" | "probe_binary"; report: FocusAuditReport }
  | { kind: "unsupported_platform"; source: "tauri_ipc" | "probe_binary"; detail: string };

const UNSUPPORTED_MARKER = "FOCUS_AUDIT_UNSUPPORTED_PLATFORM";

function repoRoot(): string {
  return path.resolve(__dirname, "..", "..");
}

function artifactRoot(): string {
  return (
    process.env.HANDSHAKE_ARTIFACT_ROOT
    ?? path.resolve(repoRoot(), "..", "..", "Handshake_Artifacts")
  );
}

async function tauriInvokeAvailable(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const win = window as unknown as {
      __TAURI__?: { core?: { invoke?: unknown } };
      __TAURI_INTERNALS__?: { invoke?: unknown };
    };
    return Boolean(win.__TAURI__?.core?.invoke ?? win.__TAURI_INTERNALS__?.invoke);
  });
}

/**
 * Start the focus audit via the live Tauri IPC bridge. Returns the ledger path
 * the running hook appends to, or throws the real IPC error string.
 */
async function startViaTauri(
  page: Page,
  runId: string,
  runtimeRoot: string,
): Promise<{ run_id: string; ledger_path: string; runtime_root: string }> {
  return page.evaluate(
    async ({ runId, runtimeRoot }) => {
      const win = window as unknown as {
        __TAURI__?: { core?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> } };
        __TAURI_INTERNALS__?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> };
      };
      const invoke = win.__TAURI__?.core?.invoke ?? win.__TAURI_INTERNALS__?.invoke;
      if (!invoke) throw new Error("Tauri invoke bridge unavailable");
      return invoke("kernel_operator_foreground_focus_audit_start", {
        run_id: runId,
        runtime_root: runtimeRoot,
      });
    },
    { runId, runtimeRoot },
  ) as Promise<{ run_id: string; ledger_path: string; runtime_root: string }>;
}

async function stopViaTauri(page: Page, runId: string): Promise<FocusAuditReport> {
  return page.evaluate(
    async ({ runId }) => {
      const win = window as unknown as {
        __TAURI__?: { core?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> } };
        __TAURI_INTERNALS__?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> };
      };
      const invoke = win.__TAURI__?.core?.invoke ?? win.__TAURI_INTERNALS__?.invoke;
      if (!invoke) throw new Error("Tauri invoke bridge unavailable");
      return invoke("kernel_operator_foreground_focus_audit_stop", { run_id: runId });
    },
    { runId },
  ) as Promise<FocusAuditReport>;
}

function cargoTargetDir(): string {
  return (
    process.env.CARGO_TARGET_DIR
    ?? path.join(artifactRoot(), "handshake-cargo-target")
  );
}

function probeBinaryPath(): string | null {
  const explicit = process.env.HANDSHAKE_FOCUS_AUDIT_PROBE?.trim();
  if (explicit) return explicit;
  const exe = process.platform === "win32" ? "focus-audit-probe.exe" : "focus-audit-probe";
  // Prefer release, fall back to debug.
  const target = cargoTargetDir();
  const fs = require("node:fs") as typeof import("node:fs");
  for (const profile of ["release", "debug"]) {
    const candidate = path.join(target, profile, exe);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

/**
 * MT-027 remediation (defect 1): guarantee the real probe binary exists before
 * the headless audit runs. Builds it on demand (debug profile) into the
 * configured cargo target dir so a clean/fresh checkout — or a no-context runner
 * who never separately built the probe — gets the REAL audit instead of a hard
 * "binary not found" failure. This is also wired as the Playwright globalSetup
 * (see app/tests/visual/global_setup.ts); calling it here is the belt-and-
 * suspenders fallback so the path can never silently skip or hard-fail.
 */
export function ensureProbeBinary(): string {
  const existing = probeBinaryPath();
  if (existing) return existing;

  const manifest = path.join(
    repoRoot(),
    "src",
    "backend",
    "handshake_core",
    "Cargo.toml",
  );
  const env = { ...process.env };
  // Build into the same target dir the probe is resolved from so the freshly
  // built binary is discoverable by probeBinaryPath() immediately after.
  env.CARGO_TARGET_DIR = cargoTargetDir();
  const build = spawnSync(
    "cargo",
    [
      "build",
      "--manifest-path",
      manifest,
      "--bin",
      "focus-audit-probe",
    ],
    { cwd: repoRoot(), env, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );
  if (build.error) {
    throw new Error(
      `focus-audit-probe build-on-demand failed to spawn cargo: ${String(build.error)}`,
    );
  }
  if (build.status !== 0) {
    throw new Error(
      "focus-audit-probe build-on-demand failed "
      + `(cargo exit ${build.status}):\n${build.stdout ?? ""}\n${build.stderr ?? ""}`,
    );
  }

  const built = probeBinaryPath();
  if (!built) {
    throw new Error(
      "focus-audit-probe build-on-demand reported success but the binary was "
      + `not found under ${cargoTargetDir()} (debug/release). Build output:\n`
      + `${build.stdout ?? ""}\n${build.stderr ?? ""}`,
    );
  }
  return built;
}

/**
 * Run the real `focus-audit-probe` binary with its foreground hook OPEN for the
 * ENTIRE `scenario`, then stop it via an explicit, causally-synchronized signal.
 *
 * MT-027 remediation:
 *  - Defect 1: the binary is guaranteed to exist (`ensureProbeBinary` builds it
 *    on demand) so the headless path can never hard-fail with "binary not
 *    found" on a clean target.
 *  - Defect 2: the probe keeps the hook open until WE signal completion (a line
 *    written to its stdin) AFTER `await scenario()` resolves — instead of a
 *    fixed `--hold-ms` floor that could expire mid-scenario and miss a late
 *    foreground steal (a false-negative for HBR-QUIET-001). `--hold-ms 0`
 *    disables the timer cap so the probe waits for the real stop signal; a
 *    sentinel file is wired as a redundant fallback signal.
 */
async function runViaProbe(
  runId: string,
  runtimeRoot: string,
  scenario: () => Promise<void>,
): Promise<FocusAuditOutcome> {
  const binary = ensureProbeBinary();
  const nodeProbeScript = process.env.HANDSHAKE_FOCUS_AUDIT_PROBE_NODE_SCRIPT?.trim();
  const command = nodeProbeScript ? process.execPath : binary;
  const commandPrefixArgs = nodeProbeScript ? [nodeProbeScript] : [];

  // The hook now closes on the explicit stop signal (stdin line / sentinel
  // file), not a timer. We keep an optional MAXIMUM safety cap so a hung
  // scenario can never wedge the probe forever; it is set well above any
  // realistic scenario duration and is opt-out via HANDSHAKE_FOCUS_AUDIT_HOLD_MS=0.
  const holdCapMs = Number(process.env.HANDSHAKE_FOCUS_AUDIT_HOLD_MS ?? 120000);
  const fs = require("node:fs") as typeof import("node:fs");
  const os = require("node:os") as typeof import("node:os");
  const sentinelDir = fs.mkdtempSync(path.join(os.tmpdir(), "focus-audit-stop-"));
  const sentinelPath = path.join(sentinelDir, `${sanitizeRunId(runId)}.stop`);

  const child = spawn(
    command,
    [
      ...commandPrefixArgs,
      "--run-id", runId,
      "--runtime-root", runtimeRoot,
      "--hold-ms", String(holdCapMs),
      "--stop-signal-file", sentinelPath,
    ],
    // stdin is piped so we can send the primary stop signal after the scenario.
    { stdio: ["pipe", "pipe", "pipe"] },
  );

  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (chunk: Buffer) => { stdout += chunk.toString("utf8"); });
  child.stderr.on("data", (chunk: Buffer) => { stderr += chunk.toString("utf8"); });

  const exited = new Promise<number>((resolve, reject) => {
    child.on("error", reject);
    child.on("close", (code) => resolve(code ?? -1));
  });

  // Give the hook a brief head start so it is installed before the scenario can
  // surface any window, run the FULL scenario with the hook open, THEN signal
  // completion. The probe stays hooked across the entire scenario regardless of
  // how long it takes (bounded only by the safety cap).
  await delay(150);
  let scenarioError: unknown = null;
  let exitError: unknown = null;
  let code: number | null = null;
  try {
    await scenario();
  } catch (error) {
    scenarioError = error;
  } finally {
    // Causally-synchronized stop: write the sentinel and send a stdin line +
    // close stdin. The probe stops on whichever it observes first. Both are
    // emitted AFTER the scenario resolves/rejects, so the hook is guaranteed to
    // have been open for the entire scenario and the driver proves shutdown
    // before returning the scenario failure to the caller.
    try { fs.writeFileSync(sentinelPath, "stop\n"); } catch { /* probe may exit first */ }
    try {
      child.stdin?.write("stop\n");
      child.stdin?.end();
    } catch { /* probe stdin may already be closed */ }
  }
  try {
    code = await exited;
  } catch (error) {
    exitError = error;
  } finally {
    try { fs.rmSync(sentinelDir, { recursive: true, force: true }); } catch { /* best effort */ }
  }

  if (scenarioError) {
    if (exitError || code !== 0) {
      const shutdownFailure = exitError
        ? formatUnknownError(exitError)
        : `focus-audit-probe exited ${code}:\n${stdout}\n${stderr}`;
      throw new Error(
        "focus audit scenario failed and probe shutdown also failed:\n"
        + `scenario: ${formatUnknownError(scenarioError)}\n`
        + `probe: ${shutdownFailure}`,
      );
    }
    throw scenarioError;
  }
  if (exitError) {
    throw exitError;
  }

  if (stderr.includes(UNSUPPORTED_MARKER)) {
    return { kind: "unsupported_platform", source: "probe_binary", detail: stderr.trim() };
  }
  if (code !== 0) {
    throw new Error(`focus-audit-probe exited ${code}:\n${stdout}\n${stderr}`);
  }
  const line = stdout.trim().split(/\r?\n/).filter(Boolean).pop();
  if (!line) {
    throw new Error(`focus-audit-probe produced no report output:\n${stderr}`);
  }
  return { kind: "report", source: "probe_binary", report: JSON.parse(line) as FocusAuditReport };
}

function formatUnknownError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

/** Reduce a run_id to a filesystem-safe stem for the stop-sentinel filename. */
function sanitizeRunId(runId: string): string {
  const safe = runId.replace(/[^A-Za-z0-9._-]/g, "_");
  return safe.length > 0 ? safe : "run";
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Drive a real focus audit around `scenario`. Starts the audit (Tauri IPC when
 * available, else the probe binary), runs the supplied scenario callback, then
 * stops the audit and returns the genuine outcome.
 */
export async function withFocusAudit(
  page: Page,
  runId: string,
  runtimeRoot: string,
  scenario: () => Promise<void>,
): Promise<FocusAuditOutcome> {
  if (await tauriInvokeAvailable(page)) {
    try {
      await startViaTauri(page, runId, runtimeRoot);
    } catch (error) {
      const message = String(error);
      if (message.includes(UNSUPPORTED_MARKER)) {
        await scenario();
        return { kind: "unsupported_platform", source: "tauri_ipc", detail: message };
      }
      throw error;
    }
    let report: FocusAuditReport | null = null;
    let stopError: unknown = null;
    try {
      await scenario();
    } finally {
      try {
        report = await stopViaTauri(page, runId);
      } catch (error) {
        stopError = error;
      }
    }
    if (stopError) {
      const message = String(stopError);
      if (message.includes(UNSUPPORTED_MARKER)) {
        return { kind: "unsupported_platform", source: "tauri_ipc", detail: message };
      }
      throw stopError;
    }
    if (!report) {
      throw new Error("Tauri focus audit stopped without returning a report");
    }
    return { kind: "report", source: "tauri_ipc", report };
  }

  // Headless fixture path: the probe holds the real hook open while the scenario
  // runs, then returns the genuine report.
  return runViaProbe(runId, runtimeRoot, scenario);
}
