import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const RUNTIME_ROOT = path.resolve(".GOV", "roles_shared", "runtime");

test("repo-local roles_shared runtime bucket contains only the sanctioned local exceptions", () => {
  const entries = fs.readdirSync(RUNTIME_ROOT).sort();
  assert.deepEqual(entries, [
    "PRODUCT_GOVERNANCE_SNAPSHOT.json",
    "validator_gates",
  ]);
});

test("runtime-placement-check passes when repo-local runtime residue is cleaned", () => {
  const result = spawnSync(
    process.execPath,
    [path.join(".GOV", "roles_shared", "checks", "runtime-placement-check.mjs")],
    {
      cwd: process.cwd(),
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /runtime-placement-check ok/i);
});
