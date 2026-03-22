import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import { spawn } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  appendJsonlLine,
  defaultRegistry,
  getOrCreateSessionRecord,
  markPluginResult,
  parseJsonlFile,
  registryBatchLaunchSummary,
  resetBatchLaunchMode,
  saveSessionRegistry,
  sessionRegistryLockPath,
} from "../scripts/session/session-registry-lib.mjs";
import { SESSION_REGISTRY_FILE } from "../scripts/session/session-policy.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const sessionRegistryLibUrl = pathToFileURL(path.resolve(__dirname, "../scripts/session/session-registry-lib.mjs")).href;

function makeTempRepoRoot(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function removeTree(targetPath) {
  fs.rmSync(targetPath, { recursive: true, force: true });
}

function runNodeProcess(args) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, args, {
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env },
    });
    let stderr = "";
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`child exited ${code}: ${stderr.trim()}`));
    });
  });
}

test("saveSessionRegistry removes stale temp siblings and leaves no lock residue", () => {
  const repoRoot = makeTempRepoRoot("handshake-session-registry-");
  try {
    const registryPath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
    fs.mkdirSync(path.dirname(registryPath), { recursive: true });
    const staleTempPath = `${registryPath}.12345.stale.tmp`;
    fs.writeFileSync(staleTempPath, "{\"stale\":true}\n", "utf8");
    const staleDate = new Date(Date.now() - (10 * 60 * 1000));
    fs.utimesSync(staleTempPath, staleDate, staleDate);

    saveSessionRegistry(repoRoot, defaultRegistry());

    assert.equal(fs.existsSync(registryPath), true, "registry file should be written");
    assert.equal(fs.existsSync(staleTempPath), false, "stale temp sibling should be cleaned up");
    assert.equal(fs.existsSync(sessionRegistryLockPath(repoRoot)), false, "lock file should not remain after save");
  } finally {
    removeTree(repoRoot);
  }
});

test("appendJsonlLine preserves all entries across concurrent writers", async () => {
  const repoRoot = makeTempRepoRoot("handshake-session-ledger-");
  const ledgerPath = path.join(repoRoot, "gov_runtime", "roles_shared", "SESSION_CONTROL_REQUESTS.jsonl");
  const writerCount = 4;
  const linesPerWriter = 25;

  try {
    const script = [
      "const [moduleUrl, ledgerPath, writerId, count] = process.argv.slice(1);",
      "const { appendJsonlLine } = await import(moduleUrl);",
      "for (let index = 0; index < Number(count); index += 1) {",
      "  appendJsonlLine(ledgerPath, { writer: Number(writerId), seq: index });",
      "}",
    ].join("\n");

    await Promise.all(
      Array.from({ length: writerCount }, (_, writerIndex) =>
        runNodeProcess([
          "--input-type=module",
          "-e",
          script,
          sessionRegistryLibUrl,
          ledgerPath,
          String(writerIndex),
          String(linesPerWriter),
        ]),
      ),
    );

    const rows = parseJsonlFile(ledgerPath);
    assert.equal(rows.length, writerCount * linesPerWriter, "all writer entries should persist");
    assert.equal(new Set(rows.map((row) => `${row.writer}:${row.seq}`)).size, rows.length, "entries should remain unique");
    assert.equal(fs.existsSync(`${ledgerPath}.lock`), false, "ledger lock file should not remain after concurrent appends");
  } finally {
    removeTree(repoRoot);
  }
});

test("batch launch mode flips after repeated plugin failures and can be reset", () => {
  const registry = defaultRegistry();
  const session = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "CODER",
    local_branch: "feat/WP-TEST",
    local_worktree_dir: "../wtc-test",
    terminal_title: "CODER WP-TEST",
  });

  markPluginResult(registry, session, "req-1", "PLUGIN_FAILED", { error: "bridge failed once" });
  let batchSummary = registryBatchLaunchSummary(registry);
  assert.equal(batchSummary.launch_batch_mode, "PLUGIN_FIRST");
  assert.equal(batchSummary.launch_batch_plugin_failure_count, 1);

  markPluginResult(registry, session, "req-2", "PLUGIN_TIMED_OUT", { error: "bridge timed out twice" });
  batchSummary = registryBatchLaunchSummary(registry);
  assert.equal(batchSummary.launch_batch_mode, "CLI_ESCALATION_BATCH");
  assert.equal(batchSummary.launch_batch_plugin_failure_count, 2);
  assert.equal(Boolean(batchSummary.launch_batch_switched_at), true);
  assert.match(batchSummary.launch_batch_switch_reason, /plugin instability reached 2\/2/i);

  resetBatchLaunchMode(registry, "new governed batch");
  batchSummary = registryBatchLaunchSummary(registry);
  assert.equal(batchSummary.launch_batch_mode, "PLUGIN_FIRST");
  assert.equal(batchSummary.launch_batch_plugin_failure_count, 0);
  assert.equal(Boolean(batchSummary.launch_batch_last_reset_at), true);
  assert.equal(batchSummary.launch_batch_switch_reason, "new governed batch");
});
