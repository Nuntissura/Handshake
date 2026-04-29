import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const TEST_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(TEST_DIR, "../../..");
const LIB_PATH = path.resolve(REPO_ROOT, ".GOV/roles_shared/scripts/lib/fail-capture-lib.mjs");

function walkMjs(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const entryPath = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walkMjs(entryPath));
    else if (entry.isFile() && entry.name.endsWith(".mjs")) out.push(entryPath);
  }
  return out;
}

test("registerFailCaptureHook installs only one process listener pair", () => {
  const script = `
    const before = {
      uncaughtException: process.listenerCount("uncaughtException"),
      unhandledRejection: process.listenerCount("unhandledRejection"),
      exit: process.listenerCount("exit"),
    };
    const lib = await import(${JSON.stringify(`file://${LIB_PATH.replace(/\\/g, "/")}`)});
    lib.registerFailCaptureHook("script-a.mjs", { role: "SHARED" });
    lib.registerFailCaptureHook("script-b.mjs", { role: "SHARED" });
    lib.registerFailCaptureHook("script-a.mjs", { role: "SHARED" });
    const after = {
      uncaughtException: process.listenerCount("uncaughtException"),
      unhandledRejection: process.listenerCount("unhandledRejection"),
      exit: process.listenerCount("exit"),
    };
    console.log(JSON.stringify({ before, after }));
  `;
  const result = spawnSync(process.execPath, ["--input-type=module", "-e", script], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, result.stderr);
  const payload = JSON.parse(String(result.stdout || "").trim());
  assert.equal(payload.after.uncaughtException - payload.before.uncaughtException, 1);
  assert.equal(payload.after.unhandledRejection - payload.before.unhandledRejection, 1);
  assert.equal(payload.after.exit - payload.before.exit, 1);
});

test("registerFailCaptureHook captures exit(1) without capturing expected exit(2)", () => {
  const oneScript = `
    const lib = await import(${JSON.stringify(`file://${LIB_PATH.replace(/\\/g, "/")}`)});
    lib.registerFailCaptureHook("exit-one.mjs", { role: "SHARED" });
    console.error("synthetic exit one");
    process.exit(1);
  `;
  const twoScript = `
    const lib = await import(${JSON.stringify(`file://${LIB_PATH.replace(/\\/g, "/")}`)});
    lib.registerFailCaptureHook("exit-two.mjs", { role: "SHARED" });
    console.error("synthetic exit two");
    process.exit(2);
  `;
  const env = {
    ...process.env,
    HANDSHAKE_GOV_RUNTIME_ROOT: path.join(os.tmpdir(), "handshake-fail-capture-test-runtime"),
  };

  const exitOne = spawnSync(process.execPath, ["--input-type=module", "-e", oneScript], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    env,
  });
  const exitTwo = spawnSync(process.execPath, ["--input-type=module", "-e", twoScript], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    env,
  });

  assert.equal(exitOne.status, 1);
  assert.equal(exitTwo.status, 2);
});

test("fail-capture call sites use role-bound hooks and the full failWithMemory signature", () => {
  const roots = [
    path.join(REPO_ROOT, ".GOV", "roles"),
    path.join(REPO_ROOT, ".GOV", "roles_shared"),
  ];
  const findings = [];
  for (const file of roots.flatMap(walkMjs)) {
    const rel = path.relative(REPO_ROOT, file).replace(/\\/g, "/");
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(/registerFailCaptureHook\(\s*(['"])([^'"]+)\1\s*\)/g)) {
      findings.push(`${rel}: registerFailCaptureHook missing options for ${match[2]}`);
    }
    for (const _match of text.matchAll(/failWithMemory\(\s*(?:`[^`]+`|['"][^'"]+['"])\s*,\s*\{/g)) {
      findings.push(`${rel}: failWithMemory missing message argument`);
    }
  }
  assert.deepEqual(findings, []);
});
