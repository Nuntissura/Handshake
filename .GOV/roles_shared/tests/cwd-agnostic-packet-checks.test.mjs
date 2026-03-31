import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");

function runNode(args, cwd) {
  const result = spawnSync(process.execPath, args, {
    cwd,
    encoding: "utf8",
  });
  return {
    status: result.status,
    stdout: String(result.stdout || ""),
    stderr: String(result.stderr || ""),
  };
}

test("packet gate checks remain valid when launched from a repo subdirectory", () => {
  const probeDir = fs.mkdtempSync(path.join(os.tmpdir(), "handshake-cwd-probe-"));
  try {
    const gateCheck = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "gate-check.mjs"),
      "WP-1-Workflow-Projection-Correlation-v1",
    ], probeDir);
    assert.equal(gateCheck.status, 0, gateCheck.stderr || gateCheck.stdout);

    const computedPolicy = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "computed-policy-gate-check.mjs"),
      "WP-1-Workflow-Projection-Correlation-v1",
      "--json",
    ], probeDir);
    assert.equal(computedPolicy.status, 0, computedPolicy.stderr || computedPolicy.stdout);
  } finally {
    fs.rmSync(probeDir, { recursive: true, force: true });
  }
});
