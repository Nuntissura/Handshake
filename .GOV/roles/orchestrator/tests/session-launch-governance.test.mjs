import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repoRoot = path.resolve(".");
const launchScript = path.join(repoRoot, ".GOV", "roles", "orchestrator", "scripts", "launch-cli-session.mjs");
const controlScript = path.join(repoRoot, ".GOV", "roles", "orchestrator", "scripts", "session-control-command.mjs");
const blockedWpId = "WP-1-Loom-Storage-Portability-v3";

function collectFiles(rootDir) {
  if (!fs.existsSync(rootDir)) return [];
  const queue = [rootDir];
  const files = [];
  while (queue.length) {
    const current = queue.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const target = path.join(current, entry.name);
      if (entry.isDirectory()) {
        queue.push(target);
      } else {
        files.push(target);
      }
    }
  }
  return files;
}

function withTempRuntime(fn) {
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-launch-guard-"));
  try {
    fn(runtimeRoot);
  } finally {
    fs.rmSync(runtimeRoot, { recursive: true, force: true });
  }
}

test("launch-cli-session blocks blocked legacy packets before runtime/session artifacts are created", () => {
  withTempRuntime((runtimeRoot) => {
    const result = spawnSync(
      process.execPath,
      [launchScript, "INTEGRATION_VALIDATOR", blockedWpId, "PRINT", "PRIMARY"],
      {
        cwd: repoRoot,
        encoding: "utf8",
        env: { ...process.env, HANDSHAKE_GOV_RUNTIME_ROOT: runtimeRoot },
      },
    );
    const output = `${result.stdout || ""}${result.stderr || ""}`;

    assert.notEqual(result.status, 0);
    assert.match(output, /cannot be launched/i);
    assert.equal(collectFiles(runtimeRoot).length, 0);
  });
});

test("session-control START_SESSION blocks blocked legacy packets before runtime/session artifacts are created", () => {
  withTempRuntime((runtimeRoot) => {
    const result = spawnSync(
      process.execPath,
      [controlScript, "START_SESSION", "INTEGRATION_VALIDATOR", blockedWpId, "", "PRIMARY"],
      {
        cwd: repoRoot,
        encoding: "utf8",
        env: { ...process.env, HANDSHAKE_GOV_RUNTIME_ROOT: runtimeRoot },
      },
    );
    const output = `${result.stdout || ""}${result.stderr || ""}`;

    assert.notEqual(result.status, 0);
    assert.match(output, /cannot be started/i);
    assert.equal(collectFiles(runtimeRoot).length, 0);
  });
});
