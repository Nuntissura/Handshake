import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const repomemCli = path.resolve(__dirname, "../scripts/memory/repomem.mjs");

function withTempRuntime(fn) {
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "repomem-gate-"));
  try {
    return fn(runtimeRoot);
  } finally {
    fs.rmSync(runtimeRoot, { recursive: true, force: true });
  }
}

function runGate(runtimeRoot, args = []) {
  return spawnSync(process.execPath, [repomemCli, "gate", ...args], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_GOV_RUNTIME_ROOT: runtimeRoot,
    },
  });
}

test("repomem gate blocks when no session is open", () => withTempRuntime((runtimeRoot) => {
  const result = runGate(runtimeRoot);

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /REPOMEM_GATE_FAIL: No active session/);
}));

test("repomem gate --soft warns but exits successfully when no session is open", () => withTempRuntime((runtimeRoot) => {
  const result = runGate(runtimeRoot, ["--soft"]);

  assert.equal(result.status, 0);
  assert.match(result.stderr, /REPOMEM_GATE_WARN: No active session/);
  assert.match(result.stderr, /Read-only command may continue/);
}));

test("repomem gate succeeds when a current session marker is present", () => withTempRuntime((runtimeRoot) => {
  const markerDir = path.join(runtimeRoot, "roles_shared");
  fs.mkdirSync(markerDir, { recursive: true });
  fs.writeFileSync(path.join(markerDir, "CURRENT_REPOMEM_SESSION.json"), JSON.stringify({
    session_id: "ORCHESTRATOR-TEST",
    role: "ORCHESTRATOR",
    opened_at: new Date().toISOString(),
  }), "utf8");

  const result = runGate(runtimeRoot);

  assert.equal(result.status, 0);
  assert.match(result.stdout, /REPOMEM_GATE_OK: session=ORCHESTRATOR-TEST/);
}));
