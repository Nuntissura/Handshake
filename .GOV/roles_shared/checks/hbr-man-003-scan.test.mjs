import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const CHECK_SCRIPT = fileURLToPath(new URL("./hbr-man-003-scan.mjs", import.meta.url));
const GOV_ROOT = path.resolve(path.dirname(CHECK_SCRIPT), "../..");
const REPO_ROOT = path.resolve(GOV_ROOT, "..");
const ARTIFACT_ROOT = path.resolve(
  process.env.HANDSHAKE_ARTIFACT_ROOT || path.join(REPO_ROOT, "..", "..", "Handshake_Artifacts"),
);
const TEST_ARTIFACT_ROOT = path.join(ARTIFACT_ROOT, "hbr-man-003-scan-tests");

function writeRepoFile(repoRoot, relativePath, content) {
  const filePath = path.join(repoRoot, relativePath);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, content, "utf8");
}

function withFixture(fn) {
  mkdirSync(TEST_ARTIFACT_ROOT, { recursive: true });
  const root = mkdtempSync(path.join(TEST_ARTIFACT_ROOT, "repo-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function commandReference({
  id = "real_command",
  status = "Wired",
  ipcChannel = 'Some("kernel.real.command")',
  tauriCommand = 'Some("real_command")',
  schemaFields = '"real_field"',
  cliFlag = "None",
} = {}) {
  return [
    "    CommandReference {",
    `        id: "${id}",`,
    `        name: "${id}",`,
    `        status: CommandStatus::${status},`,
    `        ipc_channel: ${ipcChannel},`,
    `        tauri_command: ${tauriCommand},`,
    `        schema_fields: &[${schemaFields}],`,
    `        cli_flag: ${cliFlag},`,
    '        description: "Fixture command.",',
    '        expected_input: "Fixture input.",',
    '        expected_output: "Fixture output.",',
    '        common_errors: &["Fixture error."],',
    '        recovery_steps: &["Fixture recovery."],',
    "    },",
  ].join("\n");
}

function writeManual(repoRoot, refs) {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/model_manual/content.rs", [
    "use super::types::{CommandReference, CommandStatus};",
    "",
    "pub static COMMAND_REFERENCE: &[CommandReference] = &[",
    refs.join("\n"),
    "];",
    "",
  ].join("\n"));
}

function writeRealSource(repoRoot) {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/commands.rs", [
    "#[tauri::command]",
    "pub fn real_command() -> String {",
    '    "ok".to_string()',
    "}",
    "",
    "pub const REAL_IPC: &str = \"kernel.real.command\";",
    "",
    "pub struct RealPayload {",
    "    pub real_field: String,",
    "}",
    "",
  ].join("\n"));
}

function runCheck(repoRoot, extraArgs = []) {
  return spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--repo-root",
    repoRoot,
    "--gov-root",
    path.join(repoRoot, ".GOV"),
    ...extraArgs,
  ], {
    cwd: repoRoot,
    encoding: "utf8",
  });
}

function parseJsonLines(stderr) {
  return stderr.trim().split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
}

function liveProductRoot() {
  const candidates = [
    process.env.HANDSHAKE_ACTIVE_REPO_ROOT,
    path.resolve(REPO_ROOT, "..", "wtc-kernel-004-fold-v1"),
    path.resolve(REPO_ROOT, "..", "handshake_main"),
    REPO_ROOT,
  ].filter(Boolean);
  return candidates.find((candidate) =>
    existsSync(path.join(candidate, "src/backend/handshake_core/src/model_manual/content.rs"))
  );
}

test("wired manual entry with unresolved IPC channel exits 2 with JSONL failure", () => withFixture((repoRoot) => {
  writeManual(repoRoot, [
    commandReference({ ipcChannel: 'Some("kernel.real.missing")' }),
  ]);
  writeRealSource(repoRoot);

  const result = runCheck(repoRoot);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].manual_id, "real_command");
  assert.equal(failures[0].kind, "ipc_channel");
  assert.equal(failures[0].name, "kernel.real.missing");
  assert.equal(failures[0].reason, "no_source_match");
}));

test("planned manual entry with unresolved names is skipped", () => withFixture((repoRoot) => {
  writeManual(repoRoot, [
    commandReference({
      status: "Planned",
      ipcChannel: 'Some("kernel.future.missing")',
      tauriCommand: 'Some("future_missing")',
      schemaFields: '"future_field"',
      cliFlag: 'Some("--future-flag")',
    }),
  ]);

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
}));

test("wired manual entry resolves command, IPC channel, schema field, and CLI flag", () => withFixture((repoRoot) => {
  writeManual(repoRoot, [
    commandReference({ cliFlag: 'Some("--real-flag")' }),
  ]);
  writeRealSource(repoRoot);
  writeRepoFile(repoRoot, ".GOV/roles_shared/checks/real-command.mjs", [
    "const args = process.argv.slice(2);",
    'if (args.includes("--real-flag")) console.log("ok");',
    "",
  ].join("\n"));

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /hbr-man-003-scan ok/);
}));

test("live ModelManual wired entries resolve against product and governance source", () => {
  const productRoot = liveProductRoot();
  assert.ok(productRoot, "live product root with ModelManual content.rs must exist");

  const result = spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--repo-root",
    productRoot,
    "--gov-root",
    GOV_ROOT,
  ], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(result.stderr, "");
});
