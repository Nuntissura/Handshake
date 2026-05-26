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

const CHECK_SCRIPT = fileURLToPath(new URL("./mt-packet-scope-alignment-check.mjs", import.meta.url));
const GOV_CHECK_SCRIPT = fileURLToPath(new URL("./gov-check.mjs", import.meta.url));

function withFixture(fn) {
  const root = mkdtempSync(path.join(os.tmpdir(), "mt-packet-scope-alignment-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function writePacket(taskPacketsRoot, wpId, packet) {
  const packetDir = path.join(taskPacketsRoot, wpId);
  mkdirSync(packetDir, { recursive: true });
  writeFileSync(path.join(packetDir, "packet.json"), JSON.stringify(packet, null, 2), "utf8");
}

function writeMt(taskPacketsRoot, wpId, mtId, contract) {
  const packetDir = path.join(taskPacketsRoot, wpId);
  mkdirSync(packetDir, { recursive: true });
  writeFileSync(path.join(packetDir, `${mtId}.json`), JSON.stringify(contract, null, 2), "utf8");
}

function runCheck(taskPacketsRoot, extraArgs = []) {
  return spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--task-packets-root",
    taskPacketsRoot,
    "--json",
    ...extraArgs,
  ], { encoding: "utf8" });
}

function parseJsonStdout(stdout) {
  return JSON.parse(stdout.trim());
}

test("happy path: every owned_file matches at least one allowed_paths glob", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  mkdirSync(taskPacketsRoot, { recursive: true });
  writePacket(taskPacketsRoot, "WP-TEST-HAPPY", {
    scope: {
      allowed_paths: [
        "src/foo/**",
        "tests/**",
        "Cargo.toml",
      ],
    },
  });
  writeMt(taskPacketsRoot, "WP-TEST-HAPPY", "MT-001", {
    mt_id: "MT-001",
    owned_files: [
      "src/foo/bar.rs",
      "src/foo/nested/deep/baz.rs",
      "tests/integration_test.rs",
      "Cargo.toml",
    ],
  });

  const result = runCheck(taskPacketsRoot);
  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, true);
  assert.equal(json.packets_scanned, 1);
  assert.equal(json.concerns.length, 0);
  assert.equal(json.info.length, 0);
}));

test("drift: owned_file outside all allowed_paths globs yields a structured CONCERN and exit 1", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  mkdirSync(taskPacketsRoot, { recursive: true });
  writePacket(taskPacketsRoot, "WP-TEST-DRIFT", {
    scope: {
      allowed_paths: [
        "src/sandbox/**",
      ],
    },
  });
  writeMt(taskPacketsRoot, "WP-TEST-DRIFT", "MT-046", {
    mt_id: "MT-046",
    owned_files: [
      "src/sandbox/inner.rs",
      "src/bin/mt046_token_probe.rs",
    ],
  });

  const result = runCheck(taskPacketsRoot);
  assert.equal(result.status, 1, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, false);
  assert.equal(json.packets_scanned, 1);
  assert.equal(json.concerns.length, 1);
  const concern = json.concerns[0];
  assert.equal(concern.severity, "MT_OWNED_FILE_OUTSIDE_PACKET_ALLOWED_PATHS");
  assert.equal(concern.wp_id, "WP-TEST-DRIFT");
  assert.equal(concern.mt_id, "MT-046");
  assert.equal(concern.file, "src/bin/mt046_token_probe.rs");
  assert.match(concern.packet_path, /WP-TEST-DRIFT\/packet\.json$/);
  assert.match(concern.mt_path, /WP-TEST-DRIFT\/MT-046\.json$/);
}));

test("dead glob: allowed_paths entry that no MT consumes becomes INFO, not CONCERN", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  mkdirSync(taskPacketsRoot, { recursive: true });
  writePacket(taskPacketsRoot, "WP-TEST-DEAD-GLOB", {
    scope: {
      allowed_paths: [
        "src/used/**",
        "src/unused/**",
      ],
    },
  });
  writeMt(taskPacketsRoot, "WP-TEST-DEAD-GLOB", "MT-001", {
    mt_id: "MT-001",
    owned_files: ["src/used/file.rs"],
  });

  const result = runCheck(taskPacketsRoot);
  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, true);
  assert.equal(json.concerns.length, 0);
  assert.equal(json.info.length, 1);
  const info = json.info[0];
  assert.equal(info.severity, "PACKET_ALLOWED_PATHS_GLOB_NOT_CONSUMED");
  assert.equal(info.wp_id, "WP-TEST-DEAD-GLOB");
  assert.equal(info.glob, "src/unused/**");
}));

test("empty: no packets present yields exit 0 with no concerns", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  mkdirSync(taskPacketsRoot, { recursive: true });

  const result = runCheck(taskPacketsRoot);
  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, true);
  assert.equal(json.packets_scanned, 0);
  assert.equal(json.concerns.length, 0);
  assert.equal(json.info.length, 0);
}));

test("--wp scopes the check to a single packet", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  mkdirSync(taskPacketsRoot, { recursive: true });
  // Two packets — one with drift, one clean. --wp must isolate the clean one.
  writePacket(taskPacketsRoot, "WP-DIRTY", {
    scope: { allowed_paths: ["src/dirty/**"] },
  });
  writeMt(taskPacketsRoot, "WP-DIRTY", "MT-001", {
    mt_id: "MT-001",
    owned_files: ["src/outside/file.rs"],
  });
  writePacket(taskPacketsRoot, "WP-CLEAN", {
    scope: { allowed_paths: ["src/clean/**"] },
  });
  writeMt(taskPacketsRoot, "WP-CLEAN", "MT-001", {
    mt_id: "MT-001",
    owned_files: ["src/clean/file.rs"],
  });

  const result = runCheck(taskPacketsRoot, ["--wp", "WP-CLEAN"]);
  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.packets_scanned, 1);
  assert.equal(json.concerns.length, 0);
}));

test("read error: malformed JSON in a packet yields exit 2", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  mkdirSync(path.join(taskPacketsRoot, "WP-MALFORMED"), { recursive: true });
  writeFileSync(path.join(taskPacketsRoot, "WP-MALFORMED", "packet.json"), "{not valid json", "utf8");

  const result = runCheck(taskPacketsRoot);
  assert.equal(result.status, 2, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.read_errors.length, 1);
}));

test("text mode: human-readable output prints CONCERNS block on drift", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  mkdirSync(taskPacketsRoot, { recursive: true });
  writePacket(taskPacketsRoot, "WP-TEXT-DRIFT", {
    scope: { allowed_paths: ["src/foo/**"] },
  });
  writeMt(taskPacketsRoot, "WP-TEXT-DRIFT", "MT-007", {
    mt_id: "MT-007",
    owned_files: ["src/bar/baz.rs"],
  });

  const result = spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--task-packets-root",
    taskPacketsRoot,
  ], { encoding: "utf8" });

  assert.equal(result.status, 1, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stdout, /mt-packet-scope-alignment-check: scanned 1 packet/);
  assert.match(result.stdout, /CONCERNS: 1/);
  assert.match(result.stdout, /WP-TEXT-DRIFT \/ MT-007: src\/bar\/baz\.rs/);
}));

test("check is wired into the gov-check bundle", () => {
  const govCheck = readFileSync(GOV_CHECK_SCRIPT, "utf8");
  assert.match(
    govCheck,
    /\["mt-packet-scope-alignment-check", "\.\/mt-packet-scope-alignment-check\.mjs", "WORK_PACKET"\]/,
  );
});
