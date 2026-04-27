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
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "repomem-durable-"));
  try {
    return fn(runtimeRoot);
  } finally {
    fs.rmSync(runtimeRoot, { recursive: true, force: true });
  }
}

function runRepomem(runtimeRoot, args = []) {
  return spawnSync(process.execPath, [repomemCli, ...args], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_GOV_RUNTIME_ROOT: runtimeRoot,
    },
  });
}

const SESSION_TOPIC = "Test session for RGF-251 durable checkpoint coverage validation across the full close path";
const CLOSE_SUMMARY = "Closed session for RGF-251 test coverage of governance-debt warning emission and bypass";
const DECISIONS = "RGF-251 test: validate the governance-debt warning emission for judgment roles closing without any durable checkpoints";
const DECISION_BODY = "Test decision capturing why a verdict shape was chosen so the durable checkpoint coverage gate is satisfied";

test("repomem close emits REPOMEM_GOVERNANCE_DEBT for INTEGRATION_VALIDATOR with no durable checkpoint", () =>
  withTempRuntime((runtimeRoot) => {
    const open = runRepomem(runtimeRoot, ["open", SESSION_TOPIC, "--role", "INTEGRATION_VALIDATOR", "--wp", "WP-RGF251-TEST"]);
    assert.equal(open.status, 0, `open failed: ${open.stderr}`);

    const close = runRepomem(runtimeRoot, ["close", CLOSE_SUMMARY, "--decisions", DECISIONS]);
    assert.equal(close.status, 0, `close failed: ${close.stderr}`);
    assert.match(close.stdout, /REPOMEM_GOVERNANCE_DEBT/);
    assert.match(close.stdout, /INTEGRATION_VALIDATOR closed without any durable checkpoint/);
  }));

test("repomem close emits REPOMEM_GOVERNANCE_DEBT for ACTIVATION_MANAGER with no durable checkpoint", () =>
  withTempRuntime((runtimeRoot) => {
    const open = runRepomem(runtimeRoot, ["open", SESSION_TOPIC, "--role", "ACTIVATION_MANAGER", "--wp", "WP-RGF251-TEST"]);
    assert.equal(open.status, 0, `open failed: ${open.stderr}`);

    const close = runRepomem(runtimeRoot, ["close", CLOSE_SUMMARY, "--decisions", DECISIONS]);
    assert.equal(close.status, 0, `close failed: ${close.stderr}`);
    assert.match(close.stdout, /REPOMEM_GOVERNANCE_DEBT/);
    assert.match(close.stdout, /ACTIVATION_MANAGER closed without any durable checkpoint/);
  }));

test("repomem close suppresses REPOMEM_GOVERNANCE_DEBT when a durable DECISION checkpoint exists", () =>
  withTempRuntime((runtimeRoot) => {
    const open = runRepomem(runtimeRoot, ["open", SESSION_TOPIC, "--role", "INTEGRATION_VALIDATOR", "--wp", "WP-RGF251-TEST"]);
    assert.equal(open.status, 0, `open failed: ${open.stderr}`);

    const decision = runRepomem(runtimeRoot, ["decision", DECISION_BODY]);
    assert.equal(decision.status, 0, `decision failed: ${decision.stderr}`);

    const close = runRepomem(runtimeRoot, ["close", CLOSE_SUMMARY, "--decisions", DECISIONS]);
    assert.equal(close.status, 0, `close failed: ${close.stderr}`);
    assert.doesNotMatch(close.stdout, /REPOMEM_GOVERNANCE_DEBT/);
  }));

test("repomem close does not emit REPOMEM_GOVERNANCE_DEBT for ORCHESTRATOR (not a judgment-bearing role)", () =>
  withTempRuntime((runtimeRoot) => {
    const open = runRepomem(runtimeRoot, ["open", SESSION_TOPIC, "--role", "ORCHESTRATOR"]);
    assert.equal(open.status, 0, `open failed: ${open.stderr}`);

    const close = runRepomem(runtimeRoot, ["close", CLOSE_SUMMARY, "--decisions", DECISIONS]);
    assert.equal(close.status, 0, `close failed: ${close.stderr}`);
    assert.doesNotMatch(close.stdout, /REPOMEM_GOVERNANCE_DEBT/);
  }));
