import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  formatVerboseCheckDetails,
  recordCheckResult,
  runSubprocessCheckStep,
} from "../scripts/lib/check-result-lib.mjs";
import {
  buildGovernanceTopology,
  writeGovernanceTopology,
} from "../scripts/lib/governance-topology-lib.mjs";
import { writePublicSurfaceConsolidation } from "../scripts/topology/public-surface-consolidation.mjs";

function resolveRepoRoot() {
  const injectedRepoRoot = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  if (injectedRepoRoot) {
    return injectedRepoRoot;
  }

  const fileRelativeRepoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  try {
    const out = execFileSync("git", ["-C", fileRelativeRepoRoot, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore; fall back to relative-to-file resolution.
  }

  // This file lives at: /.GOV/roles_shared/checks/gov-check.mjs
  // Up 3 => repo root.
  return fileRelativeRepoRoot;
}

const currentFileDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(resolveRepoRoot());
const verboseMode = process.argv.includes("--verbose");
const listMode = process.argv.includes("--list");
const dryRunMode = process.argv.includes("--dry-run");
const jsonMode = process.argv.includes("--json");
const syncTopologyMode = process.argv.includes("--sync-topology");

if (!String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim()) {
  process.env.HANDSHAKE_ACTIVE_REPO_ROOT = repoRoot;
}

function resolveGovRootForChecks(activeRepoRoot) {
  const injectedGovRoot = String(process.env.HANDSHAKE_GOV_ROOT || "").trim();
  if (injectedGovRoot) {
    return path.resolve(injectedGovRoot);
  }
  return path.resolve(activeRepoRoot, ".GOV");
}

const govRoot = resolveGovRootForChecks(repoRoot);
const checkCwd = path.resolve(govRoot, "..");
process.chdir(checkCwd);

function checkScript(relativePath) {
  return path.resolve(currentFileDir, relativePath);
}

function printRecorded(recorded) {
  console.log(recorded.summaryLine);
  if (verboseMode) {
    console.log(formatVerboseCheckDetails(recorded.writeResult.entry));
  }
}

// Governance checks plus lightweight HBR product-surface drift gates.
// Bundled checks [RGF-194] run sub-checks as child processes and collect ALL failures.
// HBR matrix/manual validation is wired here so `just gov-check` blocks incomplete HBR evidence.
const allCheckSteps = [
  ["spec-current-check", "../scripts/spec-current-check.mjs", "SPEC"],
  ["spec-bundle-check", "./spec-bundle-check.mjs", "SPEC"],
  ["atelier_role_registry_check", "./atelier_role_registry_check.mjs", "GOVERNANCE_STRUCTURE"],
  ["validator-report-structure-check", "../../roles/validator/checks/validator-report-structure-check.mjs", "VALIDATION"],
  ["packet-truth-bundle-check", "./packet-truth-bundle-check.mjs", "WORK_PACKET"],
  ["mt-packet-scope-alignment-check", "./mt-packet-scope-alignment-check.mjs", "WORK_PACKET"],
  ["kb-ready-checklist-coverage-check", "./kb-ready-checklist-coverage-check.mjs", "WORK_PACKET"],
  ["hbr-matrix-check", "./hbr-matrix-check.mjs", "WORK_PACKET", ["--all-packets"]],
  // [IV-20260603] HBR product-surface smoke checks removed from gov-check. They are
  // model-operated product tools (frontend/backend/visual/GUI/swarm inspection), not
  // governance machine gates, and they resolved repoRoot to the product-free
  // wt-gov-kernel worktree (pnpm "No package found", cargo "Cargo.toml does not exist").
  // They belong inside the Handshake product, driven and judged by a role/model.
  // Removed per operator decision (kernel reset; governance teardown after WP-KERNEL-004).
  // Was: hbr-visual-smoke, hbr-inspector-smoke, hbr-swarm-n8, hbr-swarm-invariants.
  // See REPO_GOVERNANCE_CHANGELOG.
  ["role-protocol-hbr-linkage", "./role-protocol-hbr-linkage.mjs", "GOVERNANCE_STRUCTURE"],
  ["template-hbr-fields", "./template-hbr-fields.mjs", "GOVERNANCE_STRUCTURE"],
  ["discovery-hbr-pointers", "./discovery-hbr-pointers.mjs", "GOVERNANCE_STRUCTURE"],
  ["hbr-man-001-paired-diff", "./hbr-man-001-paired-diff.mjs", "PRODUCT_MANUAL"],
  // [IV-20260603] hbr-man-003-scan removed: product ModelManual self-consistency scan
  // resolved to wt-gov-kernel (no product code) -> "ModelManual content missing".
  // Product-owned, not a governance gate. See REPO_GOVERNANCE_CHANGELOG.
  ["hbr-quiet-api-lint", "./hbr-quiet-api-lint.mjs", "PRODUCT_QUIET"],
  ["docker-not-default-adapter-check", "./docker-not-default-adapter-check.mjs", "PRODUCT_SANDBOX"],
  ["semantic-proof-check", "./semantic-proof-check.mjs", "GOVERNANCE_STRUCTURE"],
  ["computed-policy-gate-check", "./computed-policy-gate-check.mjs", "GOVERNANCE_STRUCTURE"],
  ["historical-smoketest-lineage-check", "./historical-smoketest-lineage-check.mjs", "AUDIT"],
  ["build-order-check", "./build-order-check.mjs", "WORK_PACKET"],
  ["repo-governance-board-check", "./repo-governance-board-check.mjs", "GOVERNANCE_RECORDS"],
  ["wp-comm-bundle-check", "./wp-comm-bundle-check.mjs", "WORK_PACKET_COMMUNICATION"],
  ["session-bundle-check", "./session-bundle-check.mjs", "SESSION_CONTROL"],
  ["cache-stability-check", "./cache-stability-check.mjs", "CACHE"],
  ["verb-coverage-check", "./verb-coverage-check.mjs", "CONTRACT_COVERAGE"],
  ["governance-structure-bundle-check", "./governance-structure-bundle-check.mjs", "GOVERNANCE_STRUCTURE"],
  ["topology-bundle-check", "./topology-bundle-check.mjs", "TOPOLOGY"],
  ["phase1-add-coverage-check", "./phase1-add-coverage-check.mjs", "COVERAGE"],
].map(([check, relativePath, phase, args = []]) => ({
  check,
  scriptPath: checkScript(relativePath),
  args,
  phase,
  ownerRole: "SHARED",
  sideEffectClass: "READ_ONLY_OR_DIAGNOSTIC",
}));

const onlyChecks = String(process.env.HANDSHAKE_GOV_CHECK_ONLY || "")
  .split(",")
  .map((entry) => entry.trim())
  .filter(Boolean);
if (onlyChecks.length > 0 && process.env.HANDSHAKE_GOV_CHECK_TEST_MODE !== "1") {
  console.error("HANDSHAKE_GOV_CHECK_ONLY is restricted to HANDSHAKE_GOV_CHECK_TEST_MODE=1");
  process.exit(3);
}
const checkSteps = onlyChecks.length > 0
  ? allCheckSteps.filter((step) => onlyChecks.includes(step.check))
  : allCheckSteps;
if (onlyChecks.length > 0 && checkSteps.length !== onlyChecks.length) {
  const found = new Set(checkSteps.map((step) => step.check));
  const missing = onlyChecks.filter((check) => !found.has(check));
  console.error(`unknown HANDSHAKE_GOV_CHECK_ONLY check(s): ${missing.join(", ")}`);
  process.exit(3);
}

const env = {
  ...process.env,
  HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
  HANDSHAKE_GOV_ROOT: govRoot,
};

const failures = [];

if (syncTopologyMode) {
  const publicSurfaceRecord = writePublicSurfaceConsolidation();
  console.log(`public-surface-consolidation-sync ok: ${publicSurfaceRecord.totals.public_surfaces} public surface(s)`);
  const topologyPath = writeGovernanceTopology(buildGovernanceTopology());
  console.log(`governance-topology-sync ok: ${topologyPath}`);
}

if (listMode || dryRunMode) {
  const plan = {
    schema_id: "handshake.gov.gov_check_plan",
    schema_version: "gov_check_plan_v1",
    runner: "gov-check",
    dry_run: dryRunMode,
    checks: checkSteps.map((step) => ({
      check: step.check,
      phase: step.phase,
      script_path: step.scriptPath,
      args: step.args,
      owner_role: step.ownerRole,
      side_effect_class: step.sideEffectClass,
    })),
  };
  console.log(JSON.stringify(plan, null, 2));
  if (dryRunMode) process.exit(0);
}

for (const step of checkSteps) {
  const result = runSubprocessCheckStep({
    check: step.check,
    scriptPath: step.scriptPath,
    args: step.args,
    phase: step.phase,
    bundle: "gov-check",
    ownerRole: step.ownerRole,
    sideEffectClass: step.sideEffectClass,
    invariant: `${step.check} must pass inside gov-check phase bundle`,
    remediationHint: "Run just gov-check --verbose, inspect check_details.jsonl, and inspect the structured failure dossier row.",
    relatedTopologyRows: [`file:${path.relative(repoRoot, step.scriptPath).replace(/\\/g, "/")}`],
    cwd: checkCwd,
    env,
  });
  printRecorded(result);
  if (!result.ok) {
    failures.push({
      check: step.check,
      status: result.status,
      summary: result.result.summary,
      log_path: result.writeResult.logPath,
      entry_id: result.writeResult.entry.entry_id,
    });
  }
}

const finalResult = recordCheckResult({
  check: "gov-check",
  verdict: failures.length === 0 ? "OK" : "FAIL",
  summary: failures.length === 0 ? "gov-check ok" : `gov-check failed (${failures.length} check(s))`,
  details: {
    total_checks: checkSteps.length,
    failed_checks: failures,
  },
});
printRecorded(finalResult);
if (jsonMode) {
  console.log(JSON.stringify({
    schema_id: "handshake.gov.gov_check_result",
    schema_version: "gov_check_result_v1",
    verdict: failures.length === 0 ? "OK" : "FAIL",
    total_checks: checkSteps.length,
    failed_checks: failures,
  }, null, 2));
}

process.exit(failures.length === 0 ? 0 : 1);
