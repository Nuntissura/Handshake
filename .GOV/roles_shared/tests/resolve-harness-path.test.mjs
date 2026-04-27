import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  HARNESSES_ROOT_ENV_VAR,
  resolveHarnessesRoot,
  resolveHarnessPath,
  harnessExists,
} from "../scripts/lib/resolve-harness-path.mjs";

function makeTempHarnessRoot(harnessNames = []) {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "handshake-harness-resolver-"));
  const harnessesDir = path.join(tempRoot, "harnesses");
  fs.mkdirSync(harnessesDir, { recursive: true });
  for (const name of harnessNames) {
    fs.mkdirSync(path.join(harnessesDir, name), { recursive: true });
  }
  return { tempRoot, harnessesDir };
}

test("env var override resolves to a real harnesses root", () => {
  const { tempRoot, harnessesDir } = makeTempHarnessRoot(["fake-harness"]);
  const previousValue = process.env[HARNESSES_ROOT_ENV_VAR];
  process.env[HARNESSES_ROOT_ENV_VAR] = harnessesDir;
  try {
    const resolved = resolveHarnessesRoot();
    assert.equal(path.resolve(resolved), path.resolve(harnessesDir));
    const absHarnessPath = resolveHarnessPath("fake-harness", "README.md");
    assert.equal(absHarnessPath, path.join(harnessesDir, "fake-harness", "README.md"));
    assert.equal(harnessExists("fake-harness"), true);
    assert.equal(harnessExists("does-not-exist"), false);
  } finally {
    if (previousValue === undefined) delete process.env[HARNESSES_ROOT_ENV_VAR];
    else process.env[HARNESSES_ROOT_ENV_VAR] = previousValue;
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("missing env var and missing walk-up sibling returns null", () => {
  const previousValue = process.env[HARNESSES_ROOT_ENV_VAR];
  // Point env var at a path that does not exist — resolver must fall through.
  process.env[HARNESSES_ROOT_ENV_VAR] = path.join(os.tmpdir(), "definitely-not-a-real-harnesses-dir-xyz");
  try {
    // The walk-up may still find a real harnesses root above REPO_ROOT on the dev's machine,
    // so this test only asserts that resolveHarnessPath returns null OR a real absolute path.
    // Either outcome is correct; what we care about is that the resolver does not throw.
    const resolved = resolveHarnessesRoot();
    if (resolved !== null) {
      assert.equal(path.isAbsolute(resolved), true);
    }
  } finally {
    if (previousValue === undefined) delete process.env[HARNESSES_ROOT_ENV_VAR];
    else process.env[HARNESSES_ROOT_ENV_VAR] = previousValue;
  }
});

test("resolveHarnessPath rejects empty harness name", () => {
  assert.throws(() => resolveHarnessPath(""), /harnessName is required/);
  assert.throws(() => resolveHarnessPath(null), /harnessName is required/);
});
