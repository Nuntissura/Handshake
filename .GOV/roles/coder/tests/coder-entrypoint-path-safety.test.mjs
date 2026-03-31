import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "../../../..");

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

test("coder entrypoints resolve governed packet and check paths through repo-safe helpers", () => {
  const expectations = [
    [".GOV/roles/coder/checks/pre-work.mjs", "repoPathAbs("],
    [".GOV/roles/coder/checks/post-work.mjs", "repoPathAbs("],
    [".GOV/roles/coder/checks/coder-bootstrap-claim.mjs", "repoPathAbs("],
    [".GOV/roles/coder/checks/coder-skeleton-checkpoint.mjs", "repoPathAbs("],
  ];
  for (const [relativePath, needle] of expectations) {
    assert.match(read(relativePath), new RegExp(needle.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  }
});
