/**
 * closeout-repair.mjs [RGF-193]
 *
 * Mechanical closeout pre-repair script.
 * Runs closeout precondition checks, classifies failures from direct packet/runtime
 * truth, applies known mechanical fixes, and re-verifies.
 *
 * This script runs BEFORE the Integration Validator launches.
 * It eliminates the multi-retry closeout loop that previously consumed 85% of token budget.
 *
 * Usage: node .GOV/roles/orchestrator/scripts/closeout-repair.mjs <WP_ID> [--dry-run] [--debug]
 */

import fs from "node:fs";
import path from "node:path";
import { execFileSync, execSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import { runAbsorber } from "../../../roles_shared/scripts/lib/artifact-normalizers/index.mjs";
import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  govRootAbsPath,
  normalizePath,
  resolveWorkPacketPathAtRepo,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { buildWpCommunicationHealthCheckResult } from "../../../roles_shared/checks/wp-communication-health-check.mjs";
import { resolveCloseoutSyncCwd } from "../../../roles_shared/checks/phase-check.mjs";
import {
  buildIntegrationValidatorCloseoutCheckResult,
  loadDeclaredRuntimeStatus,
} from "../../../roles/validator/scripts/lib/integration-validator-closeout-lib.mjs";
import { buildValidatorPacketCompleteResult } from "../../../roles/validator/scripts/lib/validator-governance-lib.mjs";
import { validateSignedScopeCompatibilityTruth } from "../../../roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs";
import {
  parseSignedScopePatchArtifacts,
  validateSignedScopeSurface,
} from "../../../roles_shared/scripts/lib/signed-scope-surface-lib.mjs";
import { validateClauseReportConsistency } from "../../../roles_shared/scripts/lib/packet-closure-monitor-lib.mjs";

registerFailCaptureHook("closeout-repair");

const CLOSEOUT_FAILURE_MESSAGES = {
  BASELINE_SHA_MISMATCH: "CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA mismatch",
  SIGNED_SCOPE_COMPATIBILITY_INVALID: "signed-scope compatibility truth is invalid",
  MISSING_SIGNED_SCOPE_PATCH: "signed-scope patch artifact is missing",
  SIGNED_SCOPE_SURFACE_INVALID: "signed-scope surface declarations are invalid",
  CLAUSE_COVERAGE_MISMATCH: "clause coverage mismatch between matrix and validation reports",
  MISSING_VALIDATION_VERDICT: "missing or incomplete validation verdict in packet",
  PACKET_COMPLETENESS_OTHER: "packet completeness issue",
  COMMUNICATION_HEALTH: "communication health check issue",
  INTEGRATION_VALIDATOR_CLOSEOUT: "integration-validator closeout check failure",
};

function currentGitContextAt(cwd = REPO_ROOT) {
  const resolvedCwd = path.resolve(cwd || REPO_ROOT);
  const runGit = (args = []) => {
    const result = spawnSync("git", args, {
      cwd: resolvedCwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    return result.status === 0 ? String(result.stdout || "").trim() : "";
  };
  return {
    branch: runGit(["rev-parse", "--abbrev-ref", "HEAD"]),
    topLevel: runGit(["rev-parse", "--show-toplevel"]),
    statusShort: runGit(["status", "-sb"]),
    statusPorcelain: runGit(["status", "--porcelain=v1"]),
  };
}

function normalizeDisplayDetail(detail, repoRoot = REPO_ROOT) {
  let text = normalizePath(String(detail || "").trim());
  if (!text) return text;
  const replacements = [
    [normalizePath(path.resolve(repoRoot, "../handshake_main")), "../handshake_main"],
    [normalizePath(path.resolve(repoRoot)), "."],
  ].sort((left, right) => right[0].length - left[0].length);
  for (const [absoluteValue, displayValue] of replacements) {
    if (!absoluteValue) continue;
    text = text.split(absoluteValue).join(displayValue);
  }
  return text;
}

function log(msg) {
  console.log(`[closeout-repair] ${msg}`);
}

function logDebug(enabled, msg) {
  if (enabled) console.log(`[closeout-repair][debug] ${msg}`);
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

export function runPhaseCheck(wpId, extraArgs = "", {
  repoRoot = REPO_ROOT,
} = {}) {
  const cmd = `node "${govRootAbsPath("roles_shared", "checks", "phase-check.mjs")}" CLOSEOUT ${wpId} ${extraArgs}`;
  try {
    const result = execSync(cmd, {
      encoding: "utf8",
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 120000,
    });
    return { passed: true, stdout: result, stderr: "", message: "" };
  } catch (e) {
    return {
      passed: false,
      stdout: e.stdout || "",
      stderr: e.stderr || "",
      message: e.message || "",
    };
  }
}

function extractValidationReportsSection(packetText = "") {
  const lines = String(packetText || "").split(/\r?\n/);
  const startIndex = lines.findIndex((line) => /^##\s+VALIDATION_REPORTS\b/i.test(line));
  if (startIndex === -1) return "";
  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }
  return lines.slice(startIndex, endIndex).join("\n");
}

export function packetHasValidationVerdict(packetText = "") {
  const reportsSection = extractValidationReportsSection(packetText);
  return /^(?:\s*-\s*|\s*#{1,6}\s+|\s*)Verdict\s*:\s*(PASS|FAIL|ABANDONED|OUTDATED_ONLY|PENDING)\s*$/gim.test(reportsSection);
}

function pushFailure(failures, code, details = []) {
  const normalizedDetails = (details || [])
    .map((detail) => String(detail || "").trim())
    .filter(Boolean);
  if (failures.some((entry) => entry.code === code)) return;
  failures.push({
    code,
    summary: CLOSEOUT_FAILURE_MESSAGES[code] || code,
    details: normalizedDetails,
  });
}

export function readCurrentMainHeadSha({
  repoRoot = REPO_ROOT,
  gitExec = execFileSync,
} = {}) {
  const mainWorktree = path.resolve(repoRoot, "../handshake_main");
  return String(
    gitExec("git", ["-C", mainWorktree, "rev-parse", "HEAD"], {
      encoding: "utf8",
    }) || "",
  ).trim();
}

function resolvePacketContext(wpId, repoRoot = REPO_ROOT) {
  const resolved = resolveWorkPacketPathAtRepo(repoRoot, wpId, ".GOV");
  if (!resolved?.packetPath || !resolved?.packetAbsPath || !fs.existsSync(resolved.packetAbsPath)) {
    throw new Error(`Official packet not found for ${wpId}`);
  }
  return {
    packetPath: normalizePath(path.relative(repoRoot, resolved.packetAbsPath)) || normalizePath(resolved.packetPath),
    packetAbsPath: resolved.packetAbsPath,
    packetText: fs.readFileSync(resolved.packetAbsPath, "utf8"),
  };
}

export function runCloseoutAbsorberPrepass({
  wpId = "",
  repoRoot = REPO_ROOT,
  dryRun = false,
} = {}) {
  const packetContext = resolvePacketContext(wpId, repoRoot);
  const absorbed = runAbsorber(packetContext.packetText, {
    artifactKind: "packet",
    wpId,
  });
  if (absorbed.applied.length > 0 && !dryRun) {
    fs.writeFileSync(packetContext.packetAbsPath, absorbed.output, "utf8");
  }
  return {
    ...packetContext,
    output: absorbed.output,
    applied: absorbed.applied,
    wrote: absorbed.applied.length > 0 && !dryRun,
  };
}

export function resolveDeclaredPatchArtifactPath({
  packetText = "",
  packetPath = "",
  repoRoot = REPO_ROOT,
} = {}) {
  const patchArtifacts = parseSignedScopePatchArtifacts(packetText);
  if (patchArtifacts.length === 1) {
    return path.resolve(repoRoot, patchArtifacts[0]);
  }
  if (!packetPath) return "";
  const packetAbsPath = path.resolve(repoRoot, packetPath);
  return path.join(path.dirname(packetAbsPath), "signed-scope.patch");
}

function parseCommitField(packetText, label) {
  const match = String(packetText || "").match(
    new RegExp(`\\*\\*${label}\\*\\*:\\s*\`?([0-9a-f]{40})\`?`, "i"),
  );
  return match ? match[1] : "";
}

export function collectCloseoutRepairFailures({
  packetText = "",
  packetPath = "",
  currentMainHeadSha = "",
  validatorPacketCompleteResult = null,
  communicationHealthResult = null,
  closeoutResult = null,
  signedScopeSurfaceValidation = null,
  clauseConsistencyValidation = null,
} = {}) {
  const failures = [];
  const compatibilityValidation = validateSignedScopeCompatibilityTruth(packetText, {
    packetPath,
    currentMainHeadSha,
    requireReadyForPass: false,
  });

  const baselineMismatchErrors = (compatibilityValidation.errors || []).filter((error) =>
    /CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA .* does not match current local main HEAD/i.test(String(error || ""))
  );
  if (baselineMismatchErrors.length > 0) {
    pushFailure(failures, "BASELINE_SHA_MISMATCH", baselineMismatchErrors);
  }

  const remainingCompatibilityErrors = (compatibilityValidation.errors || []).filter((error) =>
    !baselineMismatchErrors.includes(error)
  );
  if (remainingCompatibilityErrors.length > 0) {
    pushFailure(failures, "SIGNED_SCOPE_COMPATIBILITY_INVALID", remainingCompatibilityErrors);
  }

  const patchArtifactMissingErrors = (signedScopeSurfaceValidation?.errors || []).filter((error) =>
    /signed scope patch artifact is missing/i.test(String(error || ""))
  );
  if (patchArtifactMissingErrors.length > 0) {
    pushFailure(failures, "MISSING_SIGNED_SCOPE_PATCH", patchArtifactMissingErrors);
  }

  const remainingSignedScopeErrors = (signedScopeSurfaceValidation?.errors || []).filter((error) =>
    !patchArtifactMissingErrors.includes(error)
  );
  if (remainingSignedScopeErrors.length > 0) {
    pushFailure(failures, "SIGNED_SCOPE_SURFACE_INVALID", remainingSignedScopeErrors);
  }

  if ((clauseConsistencyValidation?.errors || []).length > 0) {
    pushFailure(failures, "CLAUSE_COVERAGE_MISMATCH", clauseConsistencyValidation.errors);
  }

  const validatorPacketDetails = [
    validatorPacketCompleteResult?.message,
    ...(validatorPacketCompleteResult?.details || []),
  ].filter(Boolean);
  const missingVerdictSignals = !packetHasValidationVerdict(packetText)
    || validatorPacketDetails.some((detail) =>
      /validation_verdict|verdict/i.test(String(detail || ""))
    );
  if (missingVerdictSignals) {
    pushFailure(
      failures,
      "MISSING_VALIDATION_VERDICT",
      validatorPacketDetails.length > 0
        ? validatorPacketDetails
        : ["VALIDATION_REPORTS does not contain a concrete Verdict line."],
    );
  }

  if (validatorPacketCompleteResult && !validatorPacketCompleteResult.ok) {
    const clauseMessage = validatorPacketDetails.some((detail) =>
      /CLAUSE_CLOSURE_MATRIX \/ VALIDATION_REPORTS mismatch|CLAUSES_REVIEWED|clause/i.test(String(detail || ""))
    );
    const verdictMessage = validatorPacketDetails.some((detail) =>
      /validation_verdict|verdict/i.test(String(detail || ""))
    );
    if (!clauseMessage && !verdictMessage) {
      pushFailure(failures, "PACKET_COMPLETENESS_OTHER", validatorPacketDetails);
    }
  }

  if (communicationHealthResult && !communicationHealthResult.ok) {
    pushFailure(
      failures,
      "COMMUNICATION_HEALTH",
      [communicationHealthResult.message, ...(communicationHealthResult.details || [])].filter(Boolean),
    );
  }

  if (closeoutResult && !closeoutResult.ok) {
    pushFailure(
      failures,
      "INTEGRATION_VALIDATOR_CLOSEOUT",
      [closeoutResult.message, ...(closeoutResult.details || [])].filter(Boolean),
    );
  }

  return {
    failures,
    compatibilityValidation,
  };
}

export function applyBaselineShaRepair({
  packetText = "",
  packetPath = "",
  currentMainHeadSha = "",
  repoRoot = REPO_ROOT,
} = {}) {
  if (!/^[0-9a-f]{40}$/i.test(String(currentMainHeadSha || "").trim())) {
    return { applied: false, reason: `Invalid main HEAD: ${currentMainHeadSha || "<missing>"}` };
  }

  const packetAbsPath = path.resolve(repoRoot, packetPath);
  const oldMatch = String(packetText || "").match(
    /(\*\*CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA\*\*:\s*)`?([0-9a-f]{40}|NOT_RUN|NONE)`?/i,
  );
  if (!oldMatch) {
    return { applied: false, reason: "Could not find CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA field in packet" };
  }

  const nextPacketText = String(packetText || "").replace(oldMatch[0], `${oldMatch[1]}${currentMainHeadSha}`);
  fs.writeFileSync(packetAbsPath, nextPacketText, "utf8");
  return {
    applied: true,
    packetText: nextPacketText,
    path: normalizePath(path.relative(repoRoot, packetAbsPath)),
    currentMainHeadSha,
  };
}

export function applySignedScopePatchRepair({
  packetText = "",
  packetPath = "",
  repoRoot = REPO_ROOT,
  gitExec = execFileSync,
} = {}) {
  const mergeBaseSha = parseCommitField(packetText, "MERGE_BASE_SHA");
  const targetHeadSha = parseCommitField(packetText, "COMMITTED_TARGET_HEAD_SHA");
  if (!mergeBaseSha || !targetHeadSha) {
    return {
      applied: false,
      reason: "Could not find MERGE_BASE_SHA and/or COMMITTED_TARGET_HEAD_SHA in packet",
    };
  }

  const patchAbsPath = resolveDeclaredPatchArtifactPath({
    packetText,
    packetPath,
    repoRoot,
  });
  if (!patchAbsPath) {
    return { applied: false, reason: "Could not resolve a patch artifact path from packet truth" };
  }

  fs.mkdirSync(path.dirname(patchAbsPath), { recursive: true });
  const mainWorktree = path.resolve(repoRoot, "../handshake_main");
  const diff = String(gitExec("git", ["-C", mainWorktree, "diff", `${mergeBaseSha}..${targetHeadSha}`], {
    encoding: "utf8",
    maxBuffer: 10 * 1024 * 1024,
  }) || "");
  fs.writeFileSync(patchAbsPath, diff, "utf8");
  return {
    applied: true,
    mergeBaseSha,
    targetHeadSha,
    diffLength: diff.length,
    patchPath: normalizePath(path.relative(repoRoot, patchAbsPath)),
  };
}

function buildCloseoutRepairDiagnostics(wpId, {
  repoRoot = REPO_ROOT,
} = {}) {
  const packetContext = resolvePacketContext(wpId, repoRoot);
  const currentMainHeadSha = readCurrentMainHeadSha({ repoRoot });
  const validatorPacketCompleteResult = buildValidatorPacketCompleteResult({ wpId });
  const communicationHealthResult = buildWpCommunicationHealthCheckResult({
    wpId,
    stage: "STATUS",
  });
  const validatorCwd = resolveCloseoutSyncCwd({
    wpId,
    phaseCheckCwd: repoRoot,
  });
  const closeoutResult = buildIntegrationValidatorCloseoutCheckResult({
    wpId,
    repoRootOverride: validatorCwd,
    gitContextOverride: currentGitContextAt(validatorCwd),
  });
  const declaredRuntime = loadDeclaredRuntimeStatus({
    repoRoot,
    packetContent: packetContext.packetText,
  });
  const signedScopeSurfaceValidation = validateSignedScopeSurface(packetContext.packetText, { repoRoot });
  const clauseConsistencyValidation = validateClauseReportConsistency(packetContext.packetText);
  const failureAnalysis = collectCloseoutRepairFailures({
    packetText: packetContext.packetText,
    packetPath: packetContext.packetPath,
    currentMainHeadSha,
    validatorPacketCompleteResult,
    communicationHealthResult,
    closeoutResult,
    signedScopeSurfaceValidation,
    clauseConsistencyValidation,
  });

  return {
    ...packetContext,
    currentMainHeadSha,
    validatorPacketCompleteResult,
    communicationHealthResult,
    closeoutResult,
    closeoutDependencyView: closeoutResult?.closeoutDependencyView || null,
    closeoutSyncGovernance: closeoutResult?.closeoutSyncGovernance || null,
    signedScopeSurfaceValidation,
    clauseConsistencyValidation,
    runtimeStatusLoaded: declaredRuntime.runtimeStatusLoaded,
    ...failureAnalysis,
  };
}

function logDetectedFailures(
  failures = [],
  repoRoot = REPO_ROOT,
  debug = false,
  closeoutSyncGovernance = null,
  closeoutDependencyView = null,
) {
  const latestGovernedAction = closeoutSyncGovernance?.latestGovernedAction || null;
  const latestEvent = closeoutSyncGovernance?.latestEvent || null;
  if (closeoutDependencyView?.summary) {
    log(`  Closeout dependency summary: ${closeoutDependencyView.summary}`);
  }
  if (latestGovernedAction || latestEvent) {
    log(
      `  Last governed closeout sync: ${latestEvent?.mode || "NONE"} via ${latestGovernedAction?.rule_id || "NONE"} `
      + `(${latestGovernedAction?.resume_disposition || "NONE"}) @ ${latestGovernedAction?.updated_at || latestEvent?.timestamp_utc || "<missing>"}`,
    );
  }
  for (const failure of failures) {
    log(`  Detected: ${failure.summary}`);
    if (debug) {
      for (const detail of failure.details.slice(0, 4)) {
        logDebug(debug, `detail[${failure.code}]: ${normalizeDisplayDetail(detail, repoRoot)}`);
      }
    }
  }
  log(`  Total failures identified: ${failures.length}`);
}

function logManualAction(code) {
  if (code === "CLAUSE_COVERAGE_MISMATCH") {
    log("  CLAUSE_COVERAGE_MISMATCH: cannot auto-fix (requires clause-level judgment)");
    log("    Manual action: ensure CLAUSE_CLOSURE_MATRIX rows match VALIDATION_REPORTS CLAUSES_REVIEWED.");
    return;
  }
  if (code === "MISSING_VALIDATION_VERDICT") {
    log("  MISSING_VALIDATION_VERDICT: validator verdict block is incomplete or absent.");
    log("    Manual action: repair the governed validation report / verdict write path before closeout.");
    return;
  }
  if (code === "PACKET_COMPLETENESS_OTHER") {
    log("  PACKET_COMPLETENESS_OTHER: packet completeness still fails outside the known verdict/clause repair classes.");
    log("    Manual action: repair the packet according to validator-packet-complete output, then re-run closeout-repair.");
    return;
  }
  if (code === "COMMUNICATION_HEALTH") {
    log("  COMMUNICATION_HEALTH: direct review route or notification projection is inconsistent.");
    log("    Manual action: verify receipts, notifications, and route residue before closeout.");
    return;
  }
  if (code === "INTEGRATION_VALIDATOR_CLOSEOUT") {
    log("  INTEGRATION_VALIDATOR_CLOSEOUT: topology or session-control truth is not closeout-ready.");
    log("    Manual action: verify final-lane topology, session registry, and broker consistency.");
    return;
  }
  if (code === "SIGNED_SCOPE_COMPATIBILITY_INVALID") {
    log("  SIGNED_SCOPE_COMPATIBILITY_INVALID: signed-scope compatibility truth is invalid outside a simple baseline mismatch.");
    log("    Manual action: repair CURRENT_MAIN_COMPATIBILITY_* / PACKET_WIDENING_* truth before closeout.");
    return;
  }
  if (code === "SIGNED_SCOPE_SURFACE_INVALID") {
    log("  SIGNED_SCOPE_SURFACE_INVALID: signed-scope declarations are invalid outside a simple missing artifact.");
    log("    Manual action: repair the declared target-file / artifact surface before closeout.");
  }
}

export function runCloseoutRepairCli(argv = process.argv.slice(2)) {
  const wpId = argv.find((value) => value.startsWith("WP-"));
  const dryRun = argv.includes("--dry-run");
  const debug = argv.includes("--debug");

  if (!wpId) {
    failWithMemory(
      `Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/closeout-repair.mjs <WP_ID> [--dry-run] [--debug]`,
      { role: "ORCHESTRATOR" },
    );
  }

  log(`Step 1: Running phase-check CLOSEOUT for ${wpId} to identify failures...`);
  const initialCheck = runPhaseCheck(wpId);
  if (initialCheck.passed) {
    log("RESULT: phase-check CLOSEOUT already passes. No repair needed.");
    process.exit(0);
  }

  log("phase-check CLOSEOUT failed. Analyzing failures from direct packet/runtime truth...");
  logDebug(debug, `stdout: ${String(initialCheck.stdout || "").trim()}`);
  logDebug(debug, `stderr: ${String(initialCheck.stderr || "").trim()}`);

  log("Step 2: Running deterministic artifact absorbers before repair classification...");
  const absorberPrepass = runCloseoutAbsorberPrepass({ wpId, dryRun });
  if (absorberPrepass.applied.length > 0) {
    log(
      `  Absorbers applied: ${absorberPrepass.applied.map((entry) => entry.name).join(", ")}`
      + `${dryRun ? " (dry-run, not written)" : ""}`,
    );
    if (!dryRun) {
      const normalizedCheck = runPhaseCheck(wpId);
      if (normalizedCheck.passed) {
        log("RESULT: phase-check CLOSEOUT passes after deterministic absorber pre-pass.");
        process.exit(0);
      }
      log("  Absorber pre-pass applied; closeout still needs repair classification.");
      logDebug(debug, `absorber recheck stdout: ${String(normalizedCheck.stdout || "").trim()}`);
      logDebug(debug, `absorber recheck stderr: ${String(normalizedCheck.stderr || "").trim()}`);
    }
  } else {
    log("  No absorber changes detected.");
  }

  const diagnostics = buildCloseoutRepairDiagnostics(wpId);
  const { failures } = diagnostics;
  if (failures.length === 0) {
    failWithMemory("closeout-repair could not classify failures from direct closeout truth", {
      wpId,
      role: "ORCHESTRATOR",
      details: [
        String(initialCheck.stdout || "").slice(0, 300),
        String(initialCheck.stderr || "").slice(0, 300),
        diagnostics.validatorPacketCompleteResult?.message || "",
        diagnostics.communicationHealthResult?.message || "",
        diagnostics.closeoutResult?.message || "",
      ].filter(Boolean),
    });
  }

  logDetectedFailures(
    failures,
    REPO_ROOT,
    debug,
    diagnostics.closeoutSyncGovernance,
    diagnostics.closeoutDependencyView,
  );

  if (dryRun) {
    log("DRY RUN: Would attempt the following repairs:");
    for (const failure of failures) log(`  - ${failure.code}`);
    process.exit(0);
  }

  log("Step 3: Applying mechanical fixes...");
  const repairs = [];
  let currentPacketText = diagnostics.packetText;

  if (failures.some((failure) => failure.code === "BASELINE_SHA_MISMATCH")) {
    log("  Repairing: CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA...");
    try {
      const result = applyBaselineShaRepair({
        packetText: currentPacketText,
        packetPath: diagnostics.packetPath,
        currentMainHeadSha: diagnostics.currentMainHeadSha,
        repoRoot: REPO_ROOT,
      });
      if (result.applied) {
        currentPacketText = result.packetText;
        repairs.push("BASELINE_SHA_MISMATCH");
        log(`    Fixed: updated to ${result.currentMainHeadSha}`);
      } else {
        log(`    Failed to repair: ${result.reason}`);
      }
    } catch (error) {
      log(`    Failed to repair: ${error.message}`);
    }
  }

  if (failures.some((failure) => failure.code === "MISSING_SIGNED_SCOPE_PATCH")) {
    log("  Repairing: signed-scope patch artifact...");
    try {
      const result = applySignedScopePatchRepair({
        packetText: currentPacketText,
        packetPath: diagnostics.packetPath,
        repoRoot: REPO_ROOT,
      });
      if (result.applied) {
        repairs.push("MISSING_SIGNED_SCOPE_PATCH");
        log(
          `    Fixed: generated ${result.patchPath} from ${result.mergeBaseSha.slice(0, 8)}..${result.targetHeadSha.slice(0, 8)} (${result.diffLength} bytes)`,
        );
      } else {
        log(`    Failed to repair: ${result.reason}`);
      }
    } catch (error) {
      log(`    Failed to repair: ${error.message}`);
    }
  }

  for (const failure of failures) {
    if (repairs.includes(failure.code)) continue;
    logManualAction(failure.code);
    if (debug) {
      for (const detail of failure.details.slice(0, 4)) {
        logDebug(debug, `manual[${failure.code}]: ${normalizeDisplayDetail(detail, REPO_ROOT)}`);
      }
    }
  }

  if (repairs.length > 0) {
    log(`Step 4: Re-verifying after ${repairs.length} repair(s)...`);
    const recheck = runPhaseCheck(wpId);
    if (recheck.passed) {
      log("RESULT: phase-check CLOSEOUT now passes after repair.");
      log(`Repairs applied: ${repairs.join(", ")}`);
      process.exit(0);
    }

    log("RESULT: phase-check CLOSEOUT still fails after repair.");
    log(`Repairs applied: ${repairs.join(", ")}`);
    log(`Remaining failures: ${failures.filter((failure) => !repairs.includes(failure.code)).map((failure) => failure.code).join(", ")}`);
    logDebug(debug, `recheck stdout: ${String(recheck.stdout || "").trim()}`);
    process.exit(1);
  }

  log("RESULT: No mechanical repairs could be applied automatically.");
  log(`Identified failures: ${failures.map((failure) => failure.code).join(", ")}`);
  log("Manual intervention required before Integration Validator launch.");
  process.exit(1);
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url));
if (isMain) {
  runCloseoutRepairCli();
}
