import assert from "node:assert/strict";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const REPO_ROOT = path.resolve(".");

test("validator-scan reports context mismatch instead of crashing in governance kernel checkout", () => {
  const result = spawnSync(process.execPath, [path.join(".GOV", "roles", "validator", "checks", "validator-scan.mjs")], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });

  assert.equal(result.status, 2, result.stderr);
  assert.match(result.stderr, /validator-scan: CONTEXT_MISMATCH/i);
  assert.match(result.stderr, /product target paths are unavailable/i);
  assert.doesNotMatch(result.stderr, /IO error for operation on/i);
});
