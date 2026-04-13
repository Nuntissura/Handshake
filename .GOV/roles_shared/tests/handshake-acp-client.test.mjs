import assert from "node:assert/strict";
import test from "node:test";
import {
  killProcessTree,
  reconcileRecoverableBrokerState,
  waitForProcessExit,
} from "../scripts/session/handshake-acp-client.mjs";

test("killProcessTree bounds Windows taskkill calls so stale broker stop cannot hang indefinitely", () => {
  const calls = [];
  const result = killProcessTree(1234, {
    platform: "win32",
    taskkill: (...args) => {
      calls.push(args);
      return { status: 0 };
    },
    killer: () => {
      throw new Error("fallback killer should not be used when taskkill succeeds");
    },
  });

  assert.deepEqual(result, { attempted: true, usedFallback: false });
  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], "taskkill");
  assert.deepEqual(calls[0][1], ["/PID", "1234", "/T", "/F"]);
  assert.equal(calls[0][2].timeout, 5000);
  assert.equal(calls[0][2].windowsHide, true);
});

test("killProcessTree falls back to a direct kill when Windows taskkill cannot clear the broker", () => {
  const fallbackCalls = [];
  const result = killProcessTree(4321, {
    platform: "win32",
    taskkill: () => ({ status: 1 }),
    killer: (pid, signal) => {
      fallbackCalls.push({ pid, signal });
    },
  });

  assert.deepEqual(result, { attempted: true, usedFallback: true });
  assert.deepEqual(fallbackCalls, [{ pid: 4321, signal: "SIGTERM" }]);
});

test("waitForProcessExit returns false after the timeout when the process never dies", async () => {
  let probeCount = 0;
  const exited = await waitForProcessExit(9999, 20, {
    isAlive: () => {
      probeCount += 1;
      return true;
    },
  });

  assert.equal(exited, false);
  assert.ok(probeCount >= 1);
});

test("reconcileRecoverableBrokerState reports a change when recoverable active runs are pruned", () => {
  const settleCalls = [];
  const result = reconcileRecoverableBrokerState("D:/repo", {
    active_runs: [{ command_id: "cmd-1" }, { command_id: "cmd-2" }],
  }, {
    settle: (repoRoot, options) => {
      settleCalls.push({ repoRoot, options });
      return {
        settled: [{ command_id: "cmd-1", repair_reason: "stale_active_run_with_settled_result" }],
      };
    },
    readState: () => ({ active_runs: [{ command_id: "cmd-2" }] }),
  });

  assert.equal(settleCalls.length, 1);
  assert.equal(settleCalls[0].repoRoot, "D:/repo");
  assert.equal(settleCalls[0].options.brokerState.active_runs.length, 2);
  assert.equal(result.changed, true);
  assert.equal(result.priorActiveRunCount, 2);
  assert.equal(result.nextActiveRunCount, 1);
});

test("reconcileRecoverableBrokerState stays unchanged when self-settle leaves broker state untouched", () => {
  const result = reconcileRecoverableBrokerState("D:/repo", {
    active_runs: [{ command_id: "cmd-1" }],
  }, {
    settle: () => ({ settled: [] }),
    readState: () => ({ active_runs: [{ command_id: "cmd-1" }] }),
  });

  assert.equal(result.changed, false);
  assert.equal(result.priorActiveRunCount, 1);
  assert.equal(result.nextActiveRunCount, 1);
});
