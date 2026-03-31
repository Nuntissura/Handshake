import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "../../../..");

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

test("validator entrypoints resolve packet/gate surfaces through repo-safe helpers", () => {
  const expectations = [
    [".GOV/roles/validator/scripts/validator-next.mjs", "repoPathAbs("],
    [".GOV/roles/validator/checks/integration-validator-context-brief.mjs", "repoPathAbs("],
    [".GOV/roles/validator/checks/integration-validator-closeout-check.mjs", "repoPathAbs("],
  ];
  for (const [relativePath, needle] of expectations) {
    assert.match(read(relativePath), new RegExp(needle.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  }
});
