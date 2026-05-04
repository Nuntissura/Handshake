import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  formatVerboseCheckDetails,
  recordCheckResult,
  runSubprocessCheckStep,
} from "../scripts/lib/check-result-lib.mjs";

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

// Governance-only checks (no product source scanning).
// Bundled checks [RGF-194] run sub-checks as child processes and collect ALL failures.
// 6 bundles replace 24 individual imports; 8 standalone checks remain (unique purpose, no natural grouping).
const checkSteps = [
  ["spec-current-check", "../scripts/spec-current-check.mjs"],
  ["spec-bundle-check", "./spec-bundle-check.mjs"],
  ["atelier_role_registry_check", "./atelier_role_registry_check.mjs"],
  ["validator-report-structure-check", "../../roles/validator/checks/validator-report-structure-check.mjs"],
  ["packet-truth-bundle-check", "./packet-truth-bundle-check.mjs"],
  ["semantic-proof-check", "./semantic-proof-check.mjs"],
  ["computed-policy-gate-check", "./computed-policy-gate-check.mjs"],
  ["historical-smoketest-lineage-check", "./historical-smoketest-lineage-check.mjs"],
  ["build-order-check", "./build-order-check.mjs"],
  ["repo-governance-board-check", "./repo-governance-board-check.mjs"],
  ["wp-comm-bundle-check", "./wp-comm-bundle-check.mjs"],
  ["session-bundle-check", "./session-bundle-check.mjs"],
  ["cache-stability-check", "./cache-stability-check.mjs"],
  ["verb-coverage-check", "./verb-coverage-check.mjs"],
  ["governance-structure-bundle-check", "./governance-structure-bundle-check.mjs"],
  ["topology-bundle-check", "./topology-bundle-check.mjs"],
  ["phase1-add-coverage-check", "./phase1-add-coverage-check.mjs"],
  ["memory-health-check", "./memory-health-check.mjs"],
].map(([check, relativePath]) => ({
  check,
  scriptPath: checkScript(relativePath),
}));

const env = {
  ...process.env,
  HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
  HANDSHAKE_GOV_ROOT: govRoot,
};

const failures = [];
for (const step of checkSteps) {
  const result = runSubprocessCheckStep({
    check: step.check,
    scriptPath: step.scriptPath,
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

// Lightweight memory maintenance: runs dedup if >6h stale, full compact if >24h stale.
// Safe on every gov-check: staleness gates prevent redundant work.
async function runMemoryMaintenance() {
  const outputLines = [];
  try {
    const memFs = await import("node:fs");
    const memPath = await import("node:path");
    const { GOVERNANCE_RUNTIME_ROOT_ABS } = await import("../scripts/lib/runtime-paths.mjs");
    const dbPath = memPath.default.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "GOVERNANCE_MEMORY.db");
    if (!memFs.default.existsSync(dbPath)) {
      outputLines.push("memory database not present; maintenance skipped");
    } else {
      const { DatabaseSync } = await import("node:sqlite");
      const db = new DatabaseSync(dbPath, { readOnly: true });
      let sinceMs = Infinity;
      try {
        const last = db.prepare("SELECT run_at FROM consolidation_log ORDER BY run_at DESC LIMIT 1").get();
        sinceMs = last ? Date.now() - new Date(last.run_at).getTime() : Infinity;
      } finally {
        try { db.close(); } catch {}
      }
      if (sinceMs > 6 * 60 * 60 * 1000) {
        const scriptPath = path.resolve(govRoot, "roles_shared", "scripts", "memory", "memory-compact.mjs");
        try {
          execFileSync(process.execPath, [scriptPath], { cwd: checkCwd, stdio: "ignore", env });
          outputLines.push("compaction ran");
        } catch {
          outputLines.push("compaction attempted; non-fatal failure");
        }
      } else {
        outputLines.push("compaction not due");
      }
    }
  } catch (error) {
    outputLines.push(`maintenance skipped: ${error?.message || String(error || "")}`);
  }

  return recordCheckResult({
    check: "memory-maintenance",
    verdict: "OK",
    summary: "memory-maintenance ok",
    details: { output_lines: outputLines },
  });
}

printRecorded(await runMemoryMaintenance());

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

process.exit(failures.length === 0 ? 0 : 1);
