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

test("shared checks remain valid when launched from a temp directory", () => {
  const probeDir = fs.mkdtempSync(path.join(os.tmpdir(), "handshake-shared-cwd-probe-"));
  try {
    const governanceReference = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "governance-reference.mjs"),
      "--json",
    ], probeDir);
    assert.equal(governanceReference.status, 0, governanceReference.stderr || governanceReference.stdout);
    const governanceReferenceJson = JSON.parse(governanceReference.stdout.trim());
    assert.match(governanceReferenceJson.specCurrentPathAbs, /wt-gov-kernel\\\.GOV\\spec\\SPEC_CURRENT\.md$/);

    const sessionPolicy = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "session-policy-check.mjs"),
    ], probeDir);
    assert.equal(sessionPolicy.status, 0, sessionPolicy.stderr || sessionPolicy.stdout);

    const worktreeConcurrency = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "worktree-concurrency-check.mjs"),
    ], probeDir);
    assert.equal(worktreeConcurrency.status, 0, worktreeConcurrency.stderr || worktreeConcurrency.stdout);

    const ossRegister = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "oss-register-check.mjs"),
    ], probeDir);
    assert.equal(ossRegister.status, 0, ossRegister.stderr || ossRegister.stdout);

    const lifecycleUx = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "lifecycle-ux-check.mjs"),
    ], probeDir);
    assert.equal(lifecycleUx.status, 0, lifecycleUx.stderr || lifecycleUx.stdout);

    const deprecationSunset = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "deprecation-sunset-check.mjs"),
    ], probeDir);
    assert.equal(deprecationSunset.status, 0, deprecationSunset.stderr || deprecationSunset.stdout);

    const packetClosureMonitor = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "packet-closure-monitor-check.mjs"),
    ], probeDir);
    assert.equal(packetClosureMonitor.status, 0, packetClosureMonitor.stderr || packetClosureMonitor.stdout);

    const semanticProof = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "semantic-proof-check.mjs"),
    ], probeDir);
    assert.equal(semanticProof.status, 0, semanticProof.stderr || semanticProof.stdout);

    const specEofAppendices = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "spec-eof-appendices-check.mjs"),
    ], probeDir);
    assert.equal(specEofAppendices.status, 0, specEofAppendices.stderr || specEofAppendices.stdout);

    const specGovernanceReference = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "spec-governance-reference-check.mjs"),
    ], probeDir);
    assert.equal(specGovernanceReference.status, 0, specGovernanceReference.stderr || specGovernanceReference.stdout);

    const phase1AddCoverage = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "phase1-add-coverage-check.mjs"),
    ], probeDir);
    assert.equal(phase1AddCoverage.status, 0, phase1AddCoverage.stderr || phase1AddCoverage.stdout);

    const taskBoardCheck = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "task-board-check.mjs"),
    ], probeDir);
    assert.equal(taskBoardCheck.status, 0, taskBoardCheck.stderr || taskBoardCheck.stdout);

    const packetTruthCheck = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "packet-truth-check.mjs"),
    ], probeDir);
    assert.equal(packetTruthCheck.status, 0, packetTruthCheck.stderr || packetTruthCheck.stdout);

    const wpActivationTraceability = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "wp-activation-traceability-check.mjs"),
    ], probeDir);
    assert.equal(wpActivationTraceability.status, 0, wpActivationTraceability.stderr || wpActivationTraceability.stdout);

    const historicalSmoketestLineage = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "historical-smoketest-lineage-check.mjs"),
    ], probeDir);
    assert.equal(historicalSmoketestLineage.status, 0, historicalSmoketestLineage.stderr || historicalSmoketestLineage.stdout);

    const buildOrderSync = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "scripts", "build-order-sync.mjs"),
      "--check",
    ], probeDir);
    assert.equal(buildOrderSync.status, 0, buildOrderSync.stderr || buildOrderSync.stdout);

    const atelierRoleRegistry = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "checks", "atelier_role_registry_check.mjs"),
    ], probeDir);
    assert.equal(atelierRoleRegistry.status, 0, atelierRoleRegistry.stderr || atelierRoleRegistry.stdout);

    const activeLaneBrief = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "scripts", "session", "active-lane-brief-lib.mjs"),
      "CODER",
      "WP-1-Workflow-Projection-Correlation-v1",
      "--json",
    ], probeDir);
    assert.equal(activeLaneBrief.status, 0, activeLaneBrief.stderr || activeLaneBrief.stdout);
    const activeLaneBriefJson = JSON.parse(activeLaneBrief.stdout.trim());
    assert.equal(activeLaneBriefJson.schema_id, "hsk.active_lane_brief@1");

    const tokenUsageReport = runNode([
      path.join(repoRoot, ".GOV", "roles_shared", "scripts", "session", "wp-token-usage-report.mjs"),
      "WP-1-Workflow-Projection-Correlation-v1",
    ], probeDir);
    assert.equal(tokenUsageReport.status, 0, tokenUsageReport.stderr || tokenUsageReport.stdout);
    assert.match(tokenUsageReport.stdout, /WP_TOKEN_USAGE/);
  } finally {
    fs.rmSync(probeDir, { recursive: true, force: true });
  }
});

test("packet gate checks remain valid when launched from a temp directory", () => {
  const probeDir = fs.mkdtempSync(path.join(os.tmpdir(), "handshake-packet-cwd-probe-"));
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
