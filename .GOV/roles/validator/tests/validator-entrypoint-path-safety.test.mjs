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
    [".GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs", "repoPathAbs("],
    [".GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs", "repoPathAbs("],
    [".GOV/roles/validator/scripts/lib/validator-governance-lib.mjs", "repoPathAbs("],
    [".GOV/roles/validator/checks/validator_gates.mjs", "workPacketAbsPath("],
  ];
  for (const [relativePath, needle] of expectations) {
    assert.match(read(relativePath), new RegExp(needle.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  }
  assert.match(
    read(".GOV/roles/validator/checks/validator_gates.mjs"),
    /wp-communication-health-check\.mjs[\s\S]*actorAuthority\.actorContext\.actorRole[\s\S]*actorAuthority\.actorContext\.actorSessionKey \|\| actorAuthority\.actorContext\.actorSessionId/,
  );
  assert.match(
    read(".GOV/roles/validator/checks/validator_gates.mjs"),
    /path\.resolve\(REPO_ROOT,\s*commandArgs\[0\]\)/,
  );
});
