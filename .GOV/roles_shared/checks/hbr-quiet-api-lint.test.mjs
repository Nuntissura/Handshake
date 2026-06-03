import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const CHECK_SCRIPT = fileURLToPath(new URL("./hbr-quiet-api-lint.mjs", import.meta.url));
const GOV_CHECK_SCRIPT = fileURLToPath(new URL("./gov-check.mjs", import.meta.url));
const GOV_ROOT = path.resolve(path.dirname(CHECK_SCRIPT), "../..");
const REPO_ROOT = path.resolve(GOV_ROOT, "..");
const ACTIVE_REPO_ROOT = path.resolve(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || process.cwd());
const JUSTFILE = path.join(REPO_ROOT, "justfile");
const CLIPPY_TOML = path.join(ACTIVE_REPO_ROOT, "clippy.toml");

function writeRepoFile(repoRoot, relativePath, content) {
  const filePath = path.join(repoRoot, relativePath);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, `${content.trim()}\n`, "utf8");
}

function withFixture(fn) {
  const root = mkdtempSync(path.join(os.tmpdir(), "hbr-quiet-api-lint-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function runCheck(repoRoot, extraArgs = []) {
  return spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--repo-root",
    repoRoot,
    ...extraArgs,
  ], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
      HANDSHAKE_GOV_ROOT: path.join(repoRoot, ".GOV"),
    },
  });
}

function parseJsonLines(stdout) {
  return stdout.trim().split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
}

test("forbidden foreground API outside allowlist exits 2 with JSONL violation", () => withFixture((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/bad_foreground.rs", `
    use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

    pub unsafe fn bad(hwnd: HWND) {
        SetForegroundWindow(hwnd);
    }
  `);

  const result = runCheck(repoRoot);
  const violations = parseJsonLines(result.stdout);

  assert.equal(result.status, 2, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(result.stderr, "");
  assert.equal(violations.length, 2);
  assert.equal(violations[0].api_name, "SetForegroundWindow");
  assert.equal(violations[0].reason, "HBR-QUIET-002");
  assert.equal(violations[0].file, "src/backend/handshake_core/src/bad_foreground.rs");
  assert.equal(violations[0].line, 1);
}));

test("operator_foreground and quiet_window escape hatches are allowed", () => withFixture((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/operator_foreground/foreground_exception.rs", `
    use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, SetForegroundWindow};

    pub unsafe fn exception(hwnd: HWND) {
        AllowSetForegroundWindow(0)?;
        SetForegroundWindow(hwnd);
    }
  `);
  writeRepoFile(repoRoot, "app/src-tauri/src/quiet_window.rs", `
    pub fn show_quiet_escape(window: &tauri::Window) {
        window.show()?;
        window.set_focus()?;
        window.unminimize()?;
    }
  `);

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /hbr-quiet-api-lint ok/);
}));

test("strings and comments mentioning forbidden APIs are not treated as references", () => withFixture((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/tests/quiet_policy_notes.rs", `
    pub fn notes() {
        let _string = "SetForegroundWindow(hwnd); window.set_focus(); window.show();";
        let _raw = r#"AttachThreadInput(1, 2, true); LockSetForegroundWindow(0);"#;
        // AllowSetForegroundWindow must remain banned outside the escape hatch.
    }
  `);

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(result.stderr, "");
}));

test("quiet API lint is wired into justfile and gov-check bundle", () => {
  const justfile = readFileSync(JUSTFILE, "utf8");
  const govCheck = readFileSync(GOV_CHECK_SCRIPT, "utf8");

  assert.match(justfile, /^hbr-quiet-api-lint \*FLAGS="":/m);
  assert.match(justfile, /hbr-quiet-api-lint\.mjs/);
  assert.match(govCheck, /\["hbr-quiet-api-lint", "\.\/hbr-quiet-api-lint\.mjs", "PRODUCT_QUIET"\]/);
});

test("clippy disallowed-methods fixture rejects HBR quiet foreground API", () => withFixture((repoRoot) => {
  assert.ok(
    readFileSync(CLIPPY_TOML, "utf8").includes("windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow"),
    `missing HBR-QUIET clippy entry in ${CLIPPY_TOML}`,
  );

  copyFileSync(CLIPPY_TOML, path.join(repoRoot, "clippy.toml"));
  writeRepoFile(repoRoot, "Cargo.toml", `
    [package]
    name = "hbr_quiet_api_lint_fixture"
    version = "0.1.0"
    edition = "2021"

    [dependencies]
    windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging"] }
  `);
  writeRepoFile(repoRoot, "src/lib.rs", `
    pub unsafe fn forbidden_foreground(hwnd: windows_sys::Win32::Foundation::HWND) {
        windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
    }
  `);

  const artifactRoot = path.resolve(ACTIVE_REPO_ROOT, "../Handshake_Artifacts/handshake-cargo-target/hbr-quiet-api-lint-fixture");
  const result = spawnSync("cargo", [
    "clippy",
    "--quiet",
    "--manifest-path",
    path.join(repoRoot, "Cargo.toml"),
    "--",
    "-D",
    "warnings",
  ], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      CARGO_TARGET_DIR: artifactRoot,
    },
  });

  assert.notEqual(result.status, 0, `cargo clippy unexpectedly passed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stderr, /HBR-QUIET-002/);
  assert.match(result.stderr, /SetForegroundWindow/);
}));
