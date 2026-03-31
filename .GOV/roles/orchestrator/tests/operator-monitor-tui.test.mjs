import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "../../../..");
const scriptPath = path.join(repoRoot, ".GOV", "roles", "orchestrator", "scripts", "operator-monitor-tui.mjs");

test("operator monitor renders the refreshed dashboard summary in once mode", () => {
  assert.equal(fs.existsSync(scriptPath), true, "operator monitor script should exist");
  const result = spawnSync(process.execPath, [scriptPath, "--once", "--filter", "ACTIVE", "--view", "OVERVIEW"], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr || "operator monitor should exit successfully");
  assert.match(result.stdout, /Operator Monitor/);
  assert.match(result.stdout, /next_action=/);
  assert.match(result.stdout, /visible=\d+\/\d+/);
});
