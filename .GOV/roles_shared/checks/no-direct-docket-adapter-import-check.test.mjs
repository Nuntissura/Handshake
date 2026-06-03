import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const CHECK_SCRIPT = fileURLToPath(new URL("./no-direct-docket-adapter-import-check.mjs", import.meta.url));

function writeRepoFile(repoRoot, relativePath, content) {
  const filePath = path.join(repoRoot, relativePath);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, `${content.trim()}\n`, "utf8");
}

function withFixture(fn) {
  const root = mkdtempSync(path.join(os.tmpdir(), "no-direct-docket-adapter-import-check-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function runCheck(repoRoot) {
  return spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--repo-root",
    repoRoot,
  ], {
    cwd: repoRoot,
    encoding: "utf8",
  });
}

function parseJsonLines(stdout) {
  return stdout.trim().split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
}

test("direct Docket/Docker KERNEL-003 identifiers in kernel/storage exit 2 with JSONL violations", () => withFixture((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/kernel/consumer.rs", `
    use crate::sandbox::docker::DocketAdapter;

    pub fn direct_consumer() {
        let docket_adapter = DocketAdapter::new();
    }
  `);
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/storage/runner.rs", `
    pub struct StorageConsumer {
        runner: DockerRunner,
    }

    fn use_runner(docker_runner: DockerRunner) {
        let _ = docker_runner;
    }
  `);

  const result = runCheck(repoRoot);
  const violations = parseJsonLines(result.stdout);

  assert.equal(result.status, 2, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(violations.length, 7);
  assert.deepEqual(
    [...new Set(violations.map((violation) => violation.pattern))].sort(),
    ["DockerRunner", "DocketAdapter", "docker_runner", "docket_adapter"],
  );
  assert.equal(violations[0].file, "src/backend/handshake_core/src/kernel/consumer.rs");
  assert.match(result.stderr, /direct KERNEL-003 Docker\/Docket references found/i);
}));

test("comments, strings, tests, and sandbox docker files are not violation targets", () => withFixture((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/kernel/clean.rs", `
    // DocketAdapter and docker_runner are historical notes, not imports.
    pub fn clean() -> &'static str {
        "DockerRunner and docket_adapter appear in a fixture string only"
    }
  `);
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/storage/clean.rs", `
    pub fn clean_storage() {}
  `);
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/kernel/tests/fixture.rs", `
    use crate::sandbox::docker::DocketAdapter;
  `);
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/sandbox/docker/kernel_003_bridge.rs", `
    // KERNEL-003 bridge comments may mention DocketAdapter.
    pub struct DockerRunner;
  `);

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /no-direct-docket-adapter-import-check ok/);
}));

test("missing kernel/storage roots fail closed", () => withFixture((repoRoot) => {
  writeRepoFile(repoRoot, "src/backend/handshake_core/src/kernel/only_kernel.rs", `
    pub fn present() {}
  `);

  const result = runCheck(repoRoot);

  assert.equal(result.status, 3, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stderr, /missing required product roots/i);
}));
