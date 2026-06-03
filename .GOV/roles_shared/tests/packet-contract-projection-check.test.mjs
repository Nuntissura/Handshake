import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

test("packet projection check accepts JSON-first contracts without generated markdown", () => {
  const result = spawnSync(
    process.execPath,
    [".GOV/roles_shared/checks/packet-contract-projection-check.mjs"],
    { encoding: "utf8" },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /json-first projection opt-out/);
  assert.doesNotMatch(result.stderr, /SQLite|node:sqlite/i);
});
