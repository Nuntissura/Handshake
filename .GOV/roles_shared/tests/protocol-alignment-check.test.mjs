import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import test from "node:test";

const repoRoot = path.resolve(".");
const checkScript = path.join(repoRoot, ".GOV", "roles_shared", "checks", "protocol-alignment-check.mjs");

test("protocol-alignment-check accepts quoted justfile session-control recipes", () => {
  const result = spawnSync(process.execPath, [checkScript], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
    },
  });

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /protocol-alignment-check ok/);
});
