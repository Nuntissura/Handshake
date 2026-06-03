import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

test("packet truth check discovers JSON-first official packets", () => {
  const result = spawnSync(
    process.execPath,
    [".GOV/roles_shared/checks/packet-truth-check.mjs"],
    { encoding: "utf8" },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /packet-truth-check ok/);
  assert.doesNotMatch(result.stderr, /SQLite|node:sqlite/i);
});
