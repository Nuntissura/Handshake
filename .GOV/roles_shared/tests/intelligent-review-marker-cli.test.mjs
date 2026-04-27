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
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "intelligent-review-"));
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
    env: { ...process.env, HANDSHAKE_GOV_RUNTIME_ROOT: runtimeRoot },
  });
}

const SESSION_TOPIC = "Intelligent review test session for RGF-254 marker writing on memory_manager close path";
const CLOSE_SUMMARY = "Closing intelligent review test session for RGF-254 marker write coverage and verification";
const DECISIONS = "RGF-254 test: confirm the marker file is written when MEMORY_MANAGER role calls repomem close";

test("repomem close writes INTELLIGENT_REVIEW_LAST_RUN.json for MEMORY_MANAGER role", () =>
  withTempRuntime((runtimeRoot) => {
    const open = runRepomem(runtimeRoot, ["open", SESSION_TOPIC, "--role", "MEMORY_MANAGER"]);
    assert.equal(open.status, 0, `open failed: ${open.stderr}`);

    const close = runRepomem(runtimeRoot, ["close", CLOSE_SUMMARY, "--decisions", DECISIONS]);
    assert.equal(close.status, 0, `close failed: ${close.stderr}`);

    const markerPath = path.join(runtimeRoot, "roles_shared", "INTELLIGENT_REVIEW_LAST_RUN.json");
    assert.ok(fs.existsSync(markerPath), "marker file not written");

    const payload = JSON.parse(fs.readFileSync(markerPath, "utf8"));
    assert.equal(payload.schema_version, "intelligent_review_last_run@1");
    assert.match(payload.session_id, /^MEMORY_MANAGER-/);
    assert.match(payload.timestamp_utc, /^\d{4}-\d{2}-\d{2}T/);
    assert.match(payload.summary, /Closing intelligent review test session/);
    assert.match(payload.decisions, /confirm the marker file is written/);
  }));

test("repomem close does NOT write the marker for non-MEMORY_MANAGER roles", () =>
  withTempRuntime((runtimeRoot) => {
    const open = runRepomem(runtimeRoot, ["open", SESSION_TOPIC, "--role", "ORCHESTRATOR"]);
    assert.equal(open.status, 0, `open failed: ${open.stderr}`);

    const close = runRepomem(runtimeRoot, ["close", CLOSE_SUMMARY, "--decisions", DECISIONS]);
    assert.equal(close.status, 0, `close failed: ${close.stderr}`);

    const markerPath = path.join(runtimeRoot, "roles_shared", "INTELLIGENT_REVIEW_LAST_RUN.json");
    assert.equal(fs.existsSync(markerPath), false, "marker should not be written for ORCHESTRATOR role");
  }));
