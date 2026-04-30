import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const repomemCli = path.resolve(__dirname, "../scripts/memory/repomem.mjs");

test("repomem open quality gate prints a role-aware corrected command", () => {
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "repomem-quality-"));
  try {
    const result = spawnSync(process.execPath, [
      repomemCli,
      "open",
      "push gov kernel",
      "--role",
      "ORCHESTRATOR",
    ], {
      cwd: repoRoot,
      encoding: "utf8",
      env: {
        ...process.env,
        HANDSHAKE_GOV_RUNTIME_ROOT: runtimeRoot,
      },
    });

    assert.equal(result.status, 1);
    assert.match(result.stderr, /REPOMEM_QUALITY_GATE_FAIL/);
    assert.match(result.stderr, /Suggested command:/);
    assert.match(result.stderr, /just repomem open "Start governed ORCHESTRATOR session to push gov kernel;/);
    assert.match(result.stderr, /--role ORCHESTRATOR/);
  } finally {
    fs.rmSync(runtimeRoot, { recursive: true, force: true });
  }
});

test("repomem open suggestion includes WP placeholder for WP-bound roles", () => {
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "repomem-quality-"));
  try {
    const result = spawnSync(process.execPath, [
      repomemCli,
      "open",
      "start work",
      "--role",
      "CODER",
    ], {
      cwd: repoRoot,
      encoding: "utf8",
      env: {
        ...process.env,
        HANDSHAKE_GOV_RUNTIME_ROOT: runtimeRoot,
      },
    });

    assert.equal(result.status, 1);
    assert.match(result.stderr, /--role CODER --wp WP-ID/);
  } finally {
    fs.rmSync(runtimeRoot, { recursive: true, force: true });
  }
});
