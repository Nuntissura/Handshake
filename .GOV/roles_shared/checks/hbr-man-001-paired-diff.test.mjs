import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const CHECK_SCRIPT = fileURLToPath(new URL("./hbr-man-001-paired-diff.mjs", import.meta.url));
const REPO_ROOT = path.resolve(path.dirname(CHECK_SCRIPT), "../../..");
const ARTIFACT_ROOT = path.resolve(
  process.env.HANDSHAKE_ARTIFACT_ROOT || path.join(REPO_ROOT, "..", "Handshake_Artifacts"),
);
const TEST_ARTIFACT_ROOT = path.join(ARTIFACT_ROOT, "hbr-man-001-paired-diff-tests");

function withTempGitRepo(fn) {
  mkdirSync(TEST_ARTIFACT_ROOT, { recursive: true });
  const repoRoot = mkdtempSync(path.join(TEST_ARTIFACT_ROOT, "repo-"));
  try {
    initRepo(repoRoot);
    return fn(repoRoot);
  } finally {
    try {
      rmSync(repoRoot, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
    } catch {
      // Windows can briefly retain handles to freshly written Git object files.
      // The fixture lives under HANDSHAKE_ARTIFACT_ROOT, so cleanup is best-effort.
    }
  }
}

function git(repoRoot, args) {
  const result = spawnSync("git", ["-C", repoRoot, ...args], {
    encoding: "utf8",
  });
  assert.equal(
    result.status,
    0,
    `git ${args.join(" ")} failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
  );
  return result.stdout.trim();
}

function writeRepoFile(repoRoot, relativePath, content) {
  const filePath = path.join(repoRoot, relativePath);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, content, "utf8");
}

function appendRepoFile(repoRoot, relativePath, content) {
  const filePath = path.join(repoRoot, relativePath);
  writeFileSync(filePath, content, { encoding: "utf8", flag: "a" });
}

function commitAll(repoRoot, message) {
  git(repoRoot, ["add", "."]);
  git(repoRoot, ["commit", "-m", message]);
}

function initRepo(repoRoot) {
  git(repoRoot, ["init", "-q"]);
  git(repoRoot, ["config", "user.email", "hbr-man-001@example.invalid"]);
  git(repoRoot, ["config", "user.name", "HBR MAN 001 Test"]);
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/model_manual/mod.rs", [
    'pub const MANUAL_VERSION: &str = "1.0.0";',
    "pub mod content;",
    "",
  ].join("\n"));
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/model_manual/content.rs", [
    "pub fn model_manual_content() -> &'static str {",
    '    "Initial manual"',
    "}",
    "",
  ].join("\n"));
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/commands.rs", [
    "pub fn internal_helper() -> i32 {",
    "    1",
    "}",
    "",
  ].join("\n"));
  commitAll(repoRoot, "initial model manual fixture");
}

function addTauriCommand(repoRoot, commandName = "kernel_model_status") {
  appendRepoFile(repoRoot, "src/backend/handshake_core/src/commands.rs", [
    "",
    "#[tauri::command]",
    `pub async fn ${commandName}() -> Result<(), String> {`,
    "    Ok(())",
    "}",
    "",
  ].join("\n"));
}

function updateManualContent(repoRoot, commandName = "kernel_model_status") {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/model_manual/content.rs", [
    "pub fn model_manual_content() -> &'static str {",
    `    "CommandReference: ${commandName}"`,
    "}",
    "",
  ].join("\n"));
}

function bumpManualVersion(repoRoot, version) {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/model_manual/mod.rs", [
    `pub const MANUAL_VERSION: &str = "${version}";`,
    "pub mod content;",
    "",
  ].join("\n"));
}

function runCheck(repoRoot) {
  return spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--repo-root",
    repoRoot,
    "--base-ref",
    "HEAD~1",
    "--target-ref",
    "HEAD",
  ], {
    cwd: repoRoot,
    encoding: "utf8",
  });
}

function parseJsonLines(stderr) {
  return stderr.trim().split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
}

test("adding a Rust tauri command without ModelManual content exits 2 with JSONL failure", () => withTempGitRepo((repoRoot) => {
  addTauriCommand(repoRoot);
  commitAll(repoRoot, "add tauri command without manual");

  const result = runCheck(repoRoot);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.match(failures[0].file, /src\/backend\/handshake_core\/src\/commands\.rs$/);
  assert.equal(failures[0].surface_kind, "tauri_command");
  assert.equal(failures[0].name, "kernel_model_status");
  assert.match(failures[0].reason, /content\.rs/);
}));

test("adding the same command with content change and MANUAL_VERSION semver bump exits 0", () => withTempGitRepo((repoRoot) => {
  addTauriCommand(repoRoot);
  updateManualContent(repoRoot);
  bumpManualVersion(repoRoot, "1.0.1");
  commitAll(repoRoot, "add tauri command with manual bump");

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /hbr-man-001-paired-diff ok/);
}));

test("changing only a non-wired internal Rust function exits 0", () => withTempGitRepo((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/commands.rs", [
    "fn renamed_internal_helper() -> i32 {",
    "    2",
    "}",
    "",
  ].join("\n"));
  commitAll(repoRoot, "rename internal helper");

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
}));

test("wired surface with content change but no MANUAL_VERSION bump exits 2", () => withTempGitRepo((repoRoot) => {
  addTauriCommand(repoRoot);
  updateManualContent(repoRoot);
  commitAll(repoRoot, "add tauri command with manual content only");

  const result = runCheck(repoRoot);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].surface_kind, "tauri_command");
  assert.equal(failures[0].name, "kernel_model_status");
  assert.match(failures[0].reason, /MANUAL_VERSION/);
}));

test("later manual commit cannot satisfy an earlier wired-surface commit", () => withTempGitRepo((repoRoot) => {
  addTauriCommand(repoRoot);
  commitAll(repoRoot, "add tauri command without same-commit manual update");
  updateManualContent(repoRoot);
  bumpManualVersion(repoRoot, "1.0.1");
  commitAll(repoRoot, "add delayed manual update");

  const result = spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--repo-root",
    repoRoot,
    "--base-ref",
    "HEAD~2",
    "--target-ref",
    "HEAD",
  ], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].surface_kind, "tauri_command");
  assert.equal(failures[0].name, "kernel_model_status");
  assert.match(failures[0].reason, /same commit/);
}));

test("unrelated ModelManual content diff does not satisfy a wired-surface change", () => withTempGitRepo((repoRoot) => {
  addTauriCommand(repoRoot);
  updateManualContent(repoRoot, "unrelated_operator_note");
  bumpManualVersion(repoRoot, "1.0.1");
  commitAll(repoRoot, "add tauri command with unrelated manual bump");

  const result = runCheck(repoRoot);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].surface_kind, "tauri_command");
  assert.equal(failures[0].name, "kernel_model_status");
  assert.match(failures[0].reason, /corresponding ModelManual/);
}));

test("test-only IPC string changes are not treated as runtime wired surfaces", () => withTempGitRepo((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/tests/ipc_assertions.rs", [
    "#[test]",
    "fn documents_test_literal() {",
    '    assert_eq!("kernel.foo.bar", "kernel.foo.bar");',
    "}",
    "",
  ].join("\n"));
  commitAll(repoRoot, "add ipc test literal");

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
}));

test("schema fields are detected even when serde derive is outside diff context", () => withTempGitRepo((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/types.rs", [
    "use serde::{Deserialize, Serialize};",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct KernelStatus {",
    ...Array.from({ length: 120 }, (_, index) => `    pub field_${index}: String,`),
    "}",
    "",
  ].join("\n"));
  commitAll(repoRoot, "add baseline schema");
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/types.rs", [
    "use serde::{Deserialize, Serialize};",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct KernelStatus {",
    ...Array.from({ length: 120 }, (_, index) => `    pub field_${index}: String,`),
    "    pub operator_visible_status: String,",
    "}",
    "",
  ].join("\n"));
  commitAll(repoRoot, "add schema field without manual");

  const result = runCheck(repoRoot);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].surface_kind, "schema_field");
  assert.equal(failures[0].name, "operator_visible_status");
}));

test("MT-011: a field added to a *Config struct body is flagged as config_key", () => withTempGitRepo((repoRoot) => {
  // File name is deliberately NOT config/settings/options-like, so detection is
  // driven purely by the struct name + struct-body scoping, not the filename.
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/kernel_runtime.rs", [
    "pub struct RuntimeConfig {",
    "    pub max_sessions: u32,",
    "}",
    "",
  ].join("\n"));
  commitAll(repoRoot, "baseline runtime config");
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/kernel_runtime.rs", [
    "pub struct RuntimeConfig {",
    "    pub max_sessions: u32,",
    "    pub idle_timeout_ms: u64,",
    "}",
    "",
  ].join("\n"));
  commitAll(repoRoot, "add config field without manual");

  const result = runCheck(repoRoot);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].surface_kind, "config_key");
  assert.equal(failures[0].name, "idle_timeout_ms");
}));

test("MT-011: ordinary struct fields and enum variant fields are NOT flagged as config_key even when the file contains a *Config struct", () => withTempGitRepo((repoRoot) => {
  // Regression guard for the 455-false-positive flood: the prior detector flagged
  // EVERY changed `name:` line in any patch that merely *contained* a *Config
  // struct anywhere. The Config struct here is unchanged; the changes land in an
  // ordinary (non-config) struct body and an enum variant body, which must NOT
  // surface as config keys. No serde derive, so schema_field must not fire either.
  const baseline = [
    "pub struct ScopeConfig {",
    "    pub depth: u32,",
    "}",
    "",
    "pub struct PlainState {",
    "    pub started: bool,",
    "}",
    "",
    "pub enum Mode {",
    "    Active { worker_id: u32 },",
    "}",
    "",
  ].join("\n");
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/kernel_state.rs", baseline);
  commitAll(repoRoot, "baseline scope/state/mode");
  const changed = [
    "pub struct ScopeConfig {",
    "    pub depth: u32,",
    "}",
    "",
    "pub struct PlainState {",
    "    pub started: bool,",
    "    pub retries: u32,",
    "}",
    "",
    "pub enum Mode {",
    "    Active { worker_id: u32 },",
    "    Draining { remaining: u32 },",
    "}",
    "",
  ].join("\n");
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/kernel_state.rs", changed);
  commitAll(repoRoot, "extend non-config struct and enum");

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
}));
