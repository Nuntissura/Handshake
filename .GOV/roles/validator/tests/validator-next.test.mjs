import assert from "node:assert/strict";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const REPO_ROOT = path.resolve(".");
const VALIDATOR_NEXT_PATH = path.join(".GOV", "roles", "validator", "scripts", "validator-next.mjs");

test("validator-next reflects the modeled historical smoketest baseline state for v3 packets", () => {
  const result = spawnSync(process.execPath, [VALIDATOR_NEXT_PATH, "WP-1-Loom-Storage-Portability-v3"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /VERDICT:\s+PASS/);
  assert.match(result.stdout, /FAILED_HISTORICAL_SMOKETEST_BASELINE/);
  assert.match(result.stdout, /Await explicit Operator authorization for merge\/push/i);
  assert.match(result.stdout, /Validator gate status: USER_ACKNOWLEDGED/i);
});

test("phase-check HANDOFF reports PREPARE worktree context mismatch for retired v3 environments", () => {
  const result = spawnSync(
    process.execPath,
    [
      path.join(".GOV", "roles_shared", "checks", "phase-check.mjs"),
      "HANDOFF",
      "WP-1-Loom-Storage-Portability-v3",
      "WP_VALIDATOR",
      "--verbose",
    ],
    {
      cwd: REPO_ROOT,
      encoding: "utf8",
    },
  );

  const output = `${result.stdout || ""}${result.stderr || ""}`;
  assert.equal(result.status, 1);
  assert.match(output, /CONTEXT_MISMATCH/i);
  assert.match(output, /Assigned PREPARE worktree is unavailable/i);
  assert.match(output, /recorded_worktree_dir=\.\.\/wtc-storage-portability-v3/i);
});

test("external-validator-brief surfaces independent revalidation contract for historical smoketest baselines", () => {
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
  assert.equal(parsed.validation_mode, "EXTERNAL_INDEPENDENT_REVALIDATION");
  assert.equal(parsed.validation_context, "CONTEXT_MISMATCH");
  assert.equal(parsed.legacy_remediation_required, false);
  assert.equal(parsed.policy_applicability, "PRE_COMPLETION_LAYER_THRESHOLD");
  assert.match(parsed.required_commands.join("\n"), /just phase-check HANDOFF WP-1-Loom-Storage-Portability-v3 WP_VALIDATOR/);
  assert.match(parsed.context_notes.join("\n"), /PREPARE worktree is unavailable/i);
});

test("validator gate writes reject unbound governed lanes and point to the correct helper family", () => {
  const result = spawnSync(
    process.execPath,
    [path.join(".GOV", "roles", "validator", "checks", "validator_gates.mjs"), "commit", "WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1"],
    {
      cwd: REPO_ROOT,
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 1);
  assert.match(result.stderr, /Wrong lane\/tool surface for governed validator gate action commit/i);
  assert.match(result.stderr, /Use: just validator-next WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/i);
  assert.match(result.stderr, /Use: just integration-validator-context-brief WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/i);
  assert.match(result.stderr, /Use: just external-validator-brief WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/i);
});
