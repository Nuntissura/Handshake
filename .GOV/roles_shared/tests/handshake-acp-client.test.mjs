import assert from "node:assert/strict";
import test from "node:test";
import {
  killProcessTree,
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
