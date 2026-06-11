import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
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

const CHECK_SCRIPT = fileURLToPath(new URL("./docker-not-default-adapter-check.mjs", import.meta.url));
const GOV_CHECK_SCRIPT = fileURLToPath(new URL("./gov-check.mjs", import.meta.url));

function writeFile(root, relativePath, content) {
  const filePath = path.join(root, relativePath);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, `${content.trim()}\n`, "utf8");
}

function withFixture(fn) {
  const root = mkdtempSync(path.join(os.tmpdir(), "docker-not-default-adapter-check-"));
  try {
    const repoRoot = path.join(root, "repo");
    const govRoot = path.join(repoRoot, ".GOV");
    mkdirSync(repoRoot, { recursive: true });
    return fn({ repoRoot, govRoot });
  } finally {
    rmSync(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function runCheck(repoRoot, govRoot = path.join(repoRoot, ".GOV")) {
  return spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--repo-root",
    repoRoot,
    "--gov-root",
    govRoot,
  ], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
      HANDSHAKE_GOV_ROOT: govRoot,
    },
  });
}

function parseJsonLines(stderr) {
  return stderr
    .trim()
    .split(/\r?\n/)
    .filter((line) => line.trim().startsWith("{"))
    .map((line) => JSON.parse(line));
}

function writeCleanBootstrap(repoRoot) {
  writeFile(repoRoot, "src/backend/handshake_core/src/sandbox/bootstrap.rs", `
    use crate::sandbox::{AdapterId, SandboxAdapterRegistry, WSL2_PODMAN_ADAPTER_ID};

    pub fn bootstrap() {
        let default_adapter_id = AdapterId::new(WSL2_PODMAN_ADAPTER_ID);
        let _registry = SandboxAdapterRegistry::new(default_adapter_id);
    }
  `);
}

test("synthetic docker registry default exits 2 with stderr JSONL violation", () => withFixture(({ repoRoot, govRoot }) => {
  writeFile(repoRoot, "src/backend/handshake_core/src/sandbox/bootstrap.rs", `
    use crate::sandbox::{AdapterId, SandboxAdapterRegistry, DOCKER_ADAPTER_ID};

    pub fn bootstrap() {
        let _registry = SandboxAdapterRegistry::new(AdapterId::new(DOCKER_ADAPTER_ID));
    }
  `);

  const result = runCheck(repoRoot, govRoot);
  const violations = parseJsonLines(result.stderr);

  assert.equal(result.status, 2, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].severity, "DOCKER_DEFAULT_ADAPTER");
  assert.equal(violations[0].file, "src/backend/handshake_core/src/sandbox/bootstrap.rs");
  assert.match(result.stderr, /Docker must remain compat-only/i);
}));

test("settings default_adapter docker exits 2 with stderr JSONL violation", () => withFixture(({ repoRoot, govRoot }) => {
  writeCleanBootstrap(repoRoot);
  writeFile(govRoot, "settings/operator-defaults.json", `
    {
      "sandbox": {
        "default_adapter": "docker",
        "docker_explicit_opt_in": true
      }
    }
  `);

  const result = runCheck(repoRoot, govRoot);
  const violations = parseJsonLines(result.stderr);

  assert.equal(result.status, 2, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].severity, "DOCKER_DEFAULT_ADAPTER");
  assert.equal(violations[0].file, ".GOV/settings/operator-defaults.json");
}));

test("comments and explicit opt-in fields do not fail when default remains non-docker", () => withFixture(({ repoRoot, govRoot }) => {
  writeFile(repoRoot, "src/backend/handshake_core/src/sandbox/bootstrap.rs", `
    // Docker remains a compat adapter, not the default.
    use crate::sandbox::{AdapterId, SandboxAdapterRegistry, WSL2_PODMAN_ADAPTER_ID};

    pub fn bootstrap() {
        let docker_explicit_opt_in = true;
        let _ = docker_explicit_opt_in;
        let _registry = SandboxAdapterRegistry::new(AdapterId::new(WSL2_PODMAN_ADAPTER_ID));
    }
  `);
  writeFile(govRoot, "settings/operator-defaults.json", `
    {
      "sandbox": {
        "default_adapter": "wsl2_podman",
        "docker_explicit_opt_in": true
      }
    }
  `);

  const result = runCheck(repoRoot, govRoot);

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stdout, /docker-not-default-adapter-check ok/);
}));

test("current repository state passes docker default lint", () => {
  const result = spawnSync(process.execPath, [CHECK_SCRIPT], {
    cwd: path.resolve(path.dirname(CHECK_SCRIPT), "../../.."),
    encoding: "utf8",
    env: process.env,
  });

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stdout, /docker-not-default-adapter-check ok/);
});

test("docker default lint is wired into gov-check bundle", () => {
  const govCheck = readFileSync(GOV_CHECK_SCRIPT, "utf8");

  assert.match(
    govCheck,
    /\["docker-not-default-adapter-check", "\.\/docker-not-default-adapter-check\.mjs", "PRODUCT_SANDBOX"\]/,
  );
});
