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

function runRepomem(runtimeRoot, args, env = {}) {
  return spawnSync(process.execPath, [repomemCli, ...args], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_GOV_RUNTIME_ROOT: runtimeRoot,
      ...env,
    },
  });
}

test("repomem opens role/WP-scoped sessions without auto-closing concurrent roles", () => {
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "repomem-parallel-"));
  const wpId = "WP-TEST-REPOMEM-PARALLEL-v1";
  try {
    const orchestratorOpen = runRepomem(runtimeRoot, [
      "open",
      "Start governed ORCHESTRATOR recovery session for a parallel ACP workflow and preserve supervisory memory state.",
      "--role",
      "ORCHESTRATOR",
      "--wp",
      wpId,
    ]);
    assert.equal(orchestratorOpen.status, 0, orchestratorOpen.stderr || orchestratorOpen.stdout);
    assert.match(orchestratorOpen.stdout, /session_id: ORCHESTRATOR-/);

    const coderOpen = runRepomem(runtimeRoot, [
      "open",
      "Start governed CODER implementation session while the Orchestrator remains active in the same ACP workflow.",
      "--role",
      "CODER",
      "--wp",
      wpId,
    ], {
      HANDSHAKE_ROLE: "CODER",
      WP_ID: wpId,
    });
    assert.equal(coderOpen.status, 0, coderOpen.stderr || coderOpen.stdout);
    assert.match(coderOpen.stdout, /Preserved concurrent session ORCHESTRATOR-/);
    assert.doesNotMatch(coderOpen.stdout, /Auto-closed stale session ORCHESTRATOR-/);

    const orchestratorClose = runRepomem(runtimeRoot, [
      "close",
      "Closed the orchestrator recovery lane after confirming role-scoped memory markers preserve parallel ACP sessions.",
      "--decisions",
      "Keep role-scoped repomem markers for concurrent governed roles.",
      "--role",
      "ORCHESTRATOR",
      "--wp",
      wpId,
    ]);
    assert.equal(orchestratorClose.status, 0, orchestratorClose.stderr || orchestratorClose.stdout);
    assert.match(orchestratorClose.stdout, /session_id: ORCHESTRATOR-/);

    const coderClose = runRepomem(runtimeRoot, [
      "close",
      "Closed the coder lane using only environment role and WP hints, proving scoped marker lookup works mechanically.",
      "--decisions",
      "Use HANDSHAKE_ROLE and WP_ID to find the role-scoped repomem session.",
    ], {
      HANDSHAKE_ROLE: "CODER",
      WP_ID: wpId,
    });
    assert.equal(coderClose.status, 0, coderClose.stderr || coderClose.stdout);
    assert.match(coderClose.stdout, /session_id: CODER-/);
  } finally {
    fs.rmSync(runtimeRoot, { recursive: true, force: true });
  }
});

test("repomem rejects an unscoped legacy marker when a WP-scoped session is required", () => {
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "repomem-scope-gate-"));
  const wpId = "WP-TEST-REPOMEM-SCOPE-v1";
  try {
    const unscopedOpen = runRepomem(runtimeRoot, [
      "open",
      "Start an unscoped ORCHESTRATOR memory session that must not satisfy a later WP-scoped mutation gate.",
      "--role",
      "ORCHESTRATOR",
    ]);
    assert.equal(unscopedOpen.status, 0, unscopedOpen.stderr || unscopedOpen.stdout);

    const scopedGate = runRepomem(runtimeRoot, [
      "gate",
      "--role",
      "ORCHESTRATOR",
      "--wp",
      wpId,
    ]);
    assert.notEqual(scopedGate.status, 0);
    assert.match(scopedGate.stderr, /REPOMEM_GATE_FAIL/);

    const scopedContext = runRepomem(runtimeRoot, [
      "context",
      "Attempt a WP-scoped governed mutation while only an unscoped Orchestrator marker exists.",
      "--trigger",
      "just orchestrator-steer-next",
      "--role",
      "ORCHESTRATOR",
      "--wp",
      wpId,
    ]);
    assert.notEqual(scopedContext.status, 0);
    assert.match(scopedContext.stderr, /No active session/);
  } finally {
    fs.rmSync(runtimeRoot, { recursive: true, force: true });
  }
});
