import type { Page } from "playwright";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import { expect, test } from "./console_error_scan";
import { withFocusAudit, type FocusAuditReport } from "./focus_audit_driver";

type EvaluateCallback = (args?: unknown) => unknown | Promise<unknown>;

function tauriPageWithInvoke(
  invoke: (command: string, args?: unknown) => Promise<unknown>,
): Page {
  return {
    evaluate: async (callback: EvaluateCallback, args?: unknown) => {
      const globalWithWindow = globalThis as typeof globalThis & { window?: unknown };
      const previousWindow = globalWithWindow.window;
      globalWithWindow.window = {
        __TAURI__: {
          core: { invoke },
        },
      };
      try {
        return await callback(args);
      } finally {
        globalWithWindow.window = previousWindow;
      }
    },
  } as unknown as Page;
}

test("withFocusAudit stops a Tauri IPC audit when the scenario throws", async () => {
  const calls: string[] = [];
  const report: FocusAuditReport = {
    run_id: "focus-cleanup-test",
    total_events: 0,
    handshake_owned_events: [],
    foreign_events: [],
    expected_foreground_events: [],
  };
  const page = tauriPageWithInvoke(async (command) => {
    calls.push(command);
    if (command === "kernel_operator_foreground_focus_audit_start") {
      return {
        run_id: report.run_id,
        ledger_path: "focus-cleanup-test.jsonl",
        runtime_root: "focus-runtime",
      };
    }
    if (command === "kernel_operator_foreground_focus_audit_stop") {
      return report;
    }
    throw new Error(`unexpected command ${command}`);
  });

  await expect(
    withFocusAudit(page, report.run_id, "focus-runtime", async () => {
      throw new Error("scenario failed");
    }),
  ).rejects.toThrow("scenario failed");

  expect(calls).toEqual([
    "kernel_operator_foreground_focus_audit_start",
    "kernel_operator_foreground_focus_audit_stop",
  ]);
});

test("withFocusAudit runs the scenario when Tauri IPC reports unsupported platform", async () => {
  const calls: string[] = [];
  let scenarioRan = false;
  const page = tauriPageWithInvoke(async (command) => {
    calls.push(command);
    if (command === "kernel_operator_foreground_focus_audit_start") {
      throw new Error("FOCUS_AUDIT_UNSUPPORTED_PLATFORM");
    }
    throw new Error(`unexpected command ${command}`);
  });

  const outcome = await withFocusAudit(page, "unsupported-test", "focus-runtime", async () => {
    scenarioRan = true;
  });

  expect(outcome.kind).toBe("unsupported_platform");
  expect(outcome.source).toBe("tauri_ipc");
  expect(scenarioRan).toBe(true);
  expect(calls).toEqual(["kernel_operator_foreground_focus_audit_start"]);
});

test("withFocusAudit propagates a Tauri IPC stop failure after a completed scenario", async () => {
  const calls: string[] = [];
  let scenarioRan = false;
  const page = tauriPageWithInvoke(async (command) => {
    calls.push(command);
    if (command === "kernel_operator_foreground_focus_audit_start") {
      return {
        run_id: "stop-failure-test",
        ledger_path: "stop-failure-test.jsonl",
        runtime_root: "focus-runtime",
      };
    }
    if (command === "kernel_operator_foreground_focus_audit_stop") {
      throw new Error("stop failed");
    }
    throw new Error(`unexpected command ${command}`);
  });

  await expect(
    withFocusAudit(page, "stop-failure-test", "focus-runtime", async () => {
      scenarioRan = true;
    }),
  ).rejects.toThrow("stop failed");

  expect(scenarioRan).toBe(true);
  expect(calls).toEqual([
    "kernel_operator_foreground_focus_audit_start",
    "kernel_operator_foreground_focus_audit_stop",
  ]);
});

test("withFocusAudit waits for probe shutdown before rethrowing a scenario failure", async ({
  page,
}) => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "focus-audit-fake-probe-"));
  const closedMarker = path.join(temp, "probe-closed.txt");
  const fakeProbeJs = path.join(temp, "fake-probe.js");

  fs.writeFileSync(
    fakeProbeJs,
    `
const fs = require("node:fs");
const closedMarker = ${JSON.stringify(closedMarker)};
let stopped = false;
function finish() {
  if (stopped) return;
  stopped = true;
  setTimeout(() => {
    fs.writeFileSync(closedMarker, "closed\\n");
    console.log(JSON.stringify({
      run_id: "probe-scenario-failure-test",
      total_events: 0,
      handshake_owned_events: [],
      foreign_events: [],
      expected_foreground_events: []
    }));
    process.exit(0);
  }, 350);
}
process.stdin.on("data", finish);
process.stdin.on("end", finish);
setTimeout(() => process.exit(9), 5000);
`,
  );

  const previousProbe = process.env.HANDSHAKE_FOCUS_AUDIT_PROBE;
  const previousNodeScript = process.env.HANDSHAKE_FOCUS_AUDIT_PROBE_NODE_SCRIPT;
  const previousHoldMs = process.env.HANDSHAKE_FOCUS_AUDIT_HOLD_MS;
  process.env.HANDSHAKE_FOCUS_AUDIT_PROBE = process.execPath;
  process.env.HANDSHAKE_FOCUS_AUDIT_PROBE_NODE_SCRIPT = fakeProbeJs;
  process.env.HANDSHAKE_FOCUS_AUDIT_HOLD_MS = "5000";
  try {
    await expect(
      withFocusAudit(page, "probe-scenario-failure-test", temp, async () => {
        throw new Error("scenario failed");
      }),
    ).rejects.toThrow("scenario failed");
    expect(fs.existsSync(closedMarker)).toBe(true);
  } finally {
    if (previousProbe === undefined) {
      delete process.env.HANDSHAKE_FOCUS_AUDIT_PROBE;
    } else {
      process.env.HANDSHAKE_FOCUS_AUDIT_PROBE = previousProbe;
    }
    if (previousNodeScript === undefined) {
      delete process.env.HANDSHAKE_FOCUS_AUDIT_PROBE_NODE_SCRIPT;
    } else {
      process.env.HANDSHAKE_FOCUS_AUDIT_PROBE_NODE_SCRIPT = previousNodeScript;
    }
    if (previousHoldMs === undefined) {
      delete process.env.HANDSHAKE_FOCUS_AUDIT_HOLD_MS;
    } else {
      process.env.HANDSHAKE_FOCUS_AUDIT_HOLD_MS = previousHoldMs;
    }
    fs.rmSync(temp, { recursive: true, force: true });
  }
});
