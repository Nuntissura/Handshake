import assert from "node:assert/strict";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const REPO_ROOT = path.resolve(".");
const VALIDATOR_NEXT_PATH = path.join(".GOV", "roles", "validator", "scripts", "validator-next.mjs");

test("validator-next blocks legacy remediation packets instead of surfacing PASS merge guidance", () => {
  const result = spawnSync(process.execPath, [VALIDATOR_NEXT_PATH, "WP-1-Loom-Storage-Portability-v3"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /VERDICT:\s+BLOCKED/);
  assert.match(result.stdout, /LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED|completion-layer threshold/i);
  assert.match(result.stdout, /Request NEW remediation WP variant/i);
  assert.doesNotMatch(result.stdout, /VERDICT:\s+PASS/);
});

test("validator-handoff-check fails on legacy remediation policy before stale PREPARE worktree drift", () => {
  const result = spawnSync(
    process.execPath,
    [path.join(".GOV", "roles", "validator", "checks", "validator-handoff-check.mjs"), "WP-1-Loom-Storage-Portability-v3"],
    {
      cwd: REPO_ROOT,
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 1);
  assert.match(result.stderr, /Committed handoff validation is blocked for this packet/i);
  assert.match(result.stderr, /PRE_COMPLETION_LAYER_THRESHOLD/i);
  assert.doesNotMatch(result.stderr, /Assigned PREPARE worktree is unavailable/i);
});

test("external-validator-brief surfaces legacy remediation blocks for failed historical packets", () => {
  const result = spawnSync(
    process.execPath,
    [path.join(".GOV", "roles", "validator", "checks", "external-validator-brief.mjs"), "WP-1-Loom-Storage-Portability-v3", "--json"],
    {
      cwd: REPO_ROOT,
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.validation_mode, "LEGACY_REMEDIATION_BLOCKED");
  assert.equal(parsed.legacy_remediation_required, true);
  assert.match(parsed.blocked_reason, /completion-layer threshold/i);
  assert.deepEqual(parsed.required_commands, [
    "just validator-policy-gate WP-1-Loom-Storage-Portability-v3",
    "just validator-packet-complete WP-1-Loom-Storage-Portability-v3",
  ]);
});
