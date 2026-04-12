/**
 * closeout-repair.mjs [RGF-193]
 *
 * Mechanical closeout pre-repair script.
 * Runs all closeout precondition checks, collects ALL failures (not just the first),
 * applies known mechanical fixes, and re-verifies.
 *
 * This script runs BEFORE the Integration Validator launches.
 * It eliminates the multi-retry closeout loop that previously consumed 85% of token budget.
 *
 * Usage: node .GOV/roles/orchestrator/scripts/closeout-repair.mjs <WP_ID> [--dry-run] [--debug]
 */

import fs from "node:fs";
import path from "node:path";
import { execFileSync, execSync } from "node:child_process";
import { registerFailCaptureHook, failWithMemory } from "../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import {
  REPO_ROOT,
  GOV_ROOT,
  GOV_ROOT_REPO_REL,
  repoPathAbs,
  govRootAbsPath,
  resolvePacketPath,
} from "../../roles_shared/scripts/lib/runtime-paths.mjs";

registerFailCaptureHook("closeout-repair");

const args = process.argv.slice(2);
const wpId = args.find((a) => a.startsWith("WP-"));
const dryRun = args.includes("--dry-run");
const debug = args.includes("--debug");

if (!wpId) {
  failWithMemory(
    `Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/closeout-repair.mjs <WP_ID> [--dry-run] [--debug]`,
    { role: "ORCHESTRATOR" }
  );
}

function log(msg) {
  console.log(`[closeout-repair] ${msg}`);
}

function logDebug(msg) {
  if (debug) console.log(`[closeout-repair][debug] ${msg}`);
}

function runCommand(cmd, opts = {}) {
  try {
    return execSync(cmd, {
      encoding: "utf8",
      cwd: REPO_ROOT,
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 60000,
      ...opts,
    }).trim();
  } catch (e) {
    return { error: true, message: e.message, stderr: e.stderr || "", stdout: e.stdout || "" };
  }
}

function runPhaseCheck(wpId, extraArgs = "") {
  const cmd = `node "${govRootAbsPath("roles_shared", "checks", "phase-check.mjs")}" CLOSEOUT ${wpId} ${extraArgs}`;
  logDebug(`Running: ${cmd}`);
  try {
    const result = execSync(cmd, {
      encoding: "utf8",
      cwd: REPO_ROOT,
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 120000,
    });
    return { passed: true, stdout: result, stderr: "" };
  } catch (e) {
    return {
      passed: false,
      stdout: e.stdout || "",
      stderr: e.stderr || "",
      message: e.message || "",
    };
  }
}

// ── Step 1: Run phase-check CLOSEOUT and collect all failures ──

log(`Step 1: Running phase-check CLOSEOUT for ${wpId} to identify failures...`);
const initialCheck = runPhaseCheck(wpId);

if (initialCheck.passed) {
  log("RESULT: phase-check CLOSEOUT already passes. No repair needed.");
  process.exit(0);
}

log("phase-check CLOSEOUT failed. Analyzing failures...");
logDebug(`stdout: ${initialCheck.stdout}`);
logDebug(`stderr: ${initialCheck.stderr}`);

const combinedOutput = `${initialCheck.stdout}\n${initialCheck.stderr}\n${initialCheck.message}`;
const failures = [];
const repairs = [];

// ── Step 2: Identify specific failure types ──

// 2a: CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA mismatch
if (combinedOutput.includes("CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA") ||
    combinedOutput.includes("baseline") && combinedOutput.includes("main HEAD")) {
  failures.push("BASELINE_SHA_MISMATCH");
  log("  Detected: CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA mismatch");
}

// 2b: Missing signed-scope.patch artifact
if (combinedOutput.includes("signed-scope") || combinedOutput.includes("signed_scope") ||
    combinedOutput.includes("patch artifact")) {
  failures.push("MISSING_SIGNED_SCOPE_PATCH");
  log("  Detected: signed-scope patch artifact issue");
}

// 2c: Clause coverage mismatch
if (combinedOutput.includes("CLAUSE") && combinedOutput.includes("mismatch") ||
    combinedOutput.includes("CLAUSES_REVIEWED") ||
    combinedOutput.includes("clause") && combinedOutput.includes("coverage")) {
  failures.push("CLAUSE_COVERAGE_MISMATCH");
  log("  Detected: clause coverage mismatch between matrix and validation reports");
}

// 2d: Missing validation verdict
if (combinedOutput.includes("validation_verdict") ||
    combinedOutput.includes("Verdict") && combinedOutput.includes("missing") ||
    combinedOutput.includes("validator-packet-complete")) {
  failures.push("MISSING_VALIDATION_VERDICT");
  log("  Detected: missing or incomplete validation verdict in packet");
}

// 2e: Communication health check
if (combinedOutput.includes("wp-communication-health-check") ||
    combinedOutput.includes("pending") && combinedOutput.includes("route")) {
  failures.push("COMMUNICATION_HEALTH");
  log("  Detected: communication health check issue");
}

// 2f: Integration validator closeout check
if (combinedOutput.includes("integration-validator-closeout-check")) {
  failures.push("INTEGRATION_VALIDATOR_CLOSEOUT");
  log("  Detected: integration-validator-closeout-check failure");
}

if (failures.length === 0) {
  log("  Could not classify specific failures. Raw output:");
  console.log(combinedOutput.slice(0, 2000));
  failWithMemory("closeout-repair could not classify failures from phase-check output", {
    wpId,
    role: "ORCHESTRATOR",
    details: [combinedOutput.slice(0, 300)],
  });
}

log(`  Total failures identified: ${failures.length}`);

// ── Step 3: Apply mechanical fixes ──

if (dryRun) {
  log("DRY RUN: Would attempt the following repairs:");
  for (const f of failures) log(`  - ${f}`);
  log("Exiting without changes.");
  process.exit(0);
}

log("Step 3: Applying mechanical fixes...");

// 3a: Fix BASELINE_SHA_MISMATCH
if (failures.includes("BASELINE_SHA_MISMATCH")) {
  log("  Repairing: CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA...");
  try {
    // Get current main HEAD from handshake_main
    const mainWorktree = repoPathAbs("../handshake_main");
    const currentMainHead = execFileSync("git", ["-C", mainWorktree, "rev-parse", "HEAD"], {
      encoding: "utf8",
    }).trim();

    if (/^[0-9a-f]{40}$/i.test(currentMainHead)) {
      const packetPath = resolvePacketPath(wpId);
      if (packetPath && fs.existsSync(repoPathAbs(packetPath))) {
        let packetText = fs.readFileSync(repoPathAbs(packetPath), "utf8");
        const oldMatch = packetText.match(
          /(\*\*CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA\*\*:\s*)`?([0-9a-f]{40}|NOT_RUN|NONE)`?/i
        );
        if (oldMatch) {
          packetText = packetText.replace(
            oldMatch[0],
            `${oldMatch[1]}${currentMainHead}`
          );
          fs.writeFileSync(repoPathAbs(packetPath), packetText, "utf8");
          repairs.push("BASELINE_SHA_MISMATCH");
          log(`    Fixed: updated to ${currentMainHead}`);
        } else {
          log("    Could not find CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA field in packet");
        }
      } else {
        log(`    Could not find packet at resolved path for ${wpId}`);
      }
    } else {
      log(`    Invalid main HEAD: ${currentMainHead}`);
    }
  } catch (e) {
    log(`    Failed to repair: ${e.message}`);
  }
}

// 3b: Fix MISSING_SIGNED_SCOPE_PATCH
if (failures.includes("MISSING_SIGNED_SCOPE_PATCH")) {
  log("  Repairing: signed-scope.patch...");
  try {
    const packetDir = path.dirname(repoPathAbs(resolvePacketPath(wpId)));
    const patchPath = path.join(packetDir, "signed-scope.patch");

    // Read packet to find MERGE_BASE_SHA and target
    const packetPath = resolvePacketPath(wpId);
    const packetText = fs.readFileSync(repoPathAbs(packetPath), "utf8");
    const baseMatch = packetText.match(/\*\*MERGE_BASE_SHA\*\*:\s*`?([0-9a-f]{40})`?/i);
    const targetMatch = packetText.match(/\*\*COMMITTED_TARGET_HEAD_SHA\*\*:\s*`?([0-9a-f]{40})`?/i);

    if (baseMatch && targetMatch) {
      const base = baseMatch[1];
      const target = targetMatch[1];
      // Find the worktree that has this commit
      const mainWorktree = repoPathAbs("../handshake_main");
      const diff = execFileSync("git", ["-C", mainWorktree, "diff", `${base}..${target}`], {
        encoding: "utf8",
        maxBuffer: 10 * 1024 * 1024,
      });
      fs.writeFileSync(patchPath, diff, "utf8");
      repairs.push("MISSING_SIGNED_SCOPE_PATCH");
      log(`    Fixed: generated patch from ${base.slice(0, 8)}..${target.slice(0, 8)} (${diff.length} bytes)`);
    } else {
      log("    Could not find MERGE_BASE_SHA and/or COMMITTED_TARGET_HEAD_SHA in packet");
    }
  } catch (e) {
    log(`    Failed to repair: ${e.message}`);
  }
}

// 3c: CLAUSE_COVERAGE_MISMATCH — report but don't auto-fix (requires judgment)
if (failures.includes("CLAUSE_COVERAGE_MISMATCH")) {
  log("  CLAUSE_COVERAGE_MISMATCH: cannot auto-fix (requires clause-level judgment)");
  log("    Manual action: ensure CLAUSE_CLOSURE_MATRIX rows match VALIDATION_REPORTS CLAUSES_REVIEWED");
  log("    This may require Integration Validator judgment or Orchestrator manual sync");
}

// 3d: MISSING_VALIDATION_VERDICT — check if validator gate has verdict
if (failures.includes("MISSING_VALIDATION_VERDICT")) {
  log("  Checking validator gate state for existing verdict...");
  try {
    const gateResult = runCommand(
      `node "${govRootAbsPath("roles", "validator", "checks", "validator_gates.mjs")}" status ${wpId}`
    );
    if (typeof gateResult === "string" && gateResult.includes("PASS") && gateResult.includes("COMMITTED")) {
      log("    Validator gate shows PASS/COMMITTED. Verdict should already be in packet.");
      log("    If packet lacks the verdict block, the closeout-truth-sync should handle this.");
    } else {
      log("    Validator gate does not show a committed PASS verdict.");
      log("    Manual action: run validator-gate-append and validator-gate-commit if appropriate");
    }
  } catch (e) {
    log(`    Could not check validator gate state: ${e.message}`);
  }
}

// 3e: COMMUNICATION_HEALTH — report only
if (failures.includes("COMMUNICATION_HEALTH")) {
  log("  COMMUNICATION_HEALTH: checking for stale route residue...");
  log("    This is typically caused by pending receipts or unacknowledged notifications.");
  log("    Manual action: verify all review receipts are settled and notifications acknowledged.");
}

// 3f: INTEGRATION_VALIDATOR_CLOSEOUT — report
if (failures.includes("INTEGRATION_VALIDATOR_CLOSEOUT")) {
  log("  INTEGRATION_VALIDATOR_CLOSEOUT: topology or session control issue.");
  log("    This is typically caused by session registry/control misalignment.");
  log("    Manual action: verify session states and broker consistency.");
}

// ── Step 4: Re-verify ──

if (repairs.length > 0) {
  log(`Step 4: Re-verifying after ${repairs.length} repair(s)...`);
  const recheck = runPhaseCheck(wpId);

  if (recheck.passed) {
    log("RESULT: phase-check CLOSEOUT now passes after repair.");
    log(`Repairs applied: ${repairs.join(", ")}`);
    process.exit(0);
  } else {
    log("RESULT: phase-check CLOSEOUT still fails after repair.");
    log(`Repairs applied: ${repairs.join(", ")}`);
    log(`Remaining failures: ${failures.filter((f) => !repairs.includes(f)).join(", ")}`);
    logDebug(`Recheck output: ${recheck.stdout}`);
    process.exit(1);
  }
} else {
  log("RESULT: No mechanical repairs could be applied automatically.");
  log(`Identified failures: ${failures.join(", ")}`);
  log("Manual intervention required before Integration Validator launch.");
  process.exit(1);
}
