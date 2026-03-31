import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

function normalizeTarget(target) {
  return path.normalize(String(target || "").trim());
}

function resolveRepoRoot(cwd = process.cwd()) {
  try {
    const resolved = execFileSync("git", ["-C", cwd, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    return resolved || cwd;
  } catch {
    return cwd;
  }
}

export function resolveValidatorProductTargetContext(targets = []) {
  const cwd = process.cwd();
  const repoRoot = resolveRepoRoot(cwd);
  const requestedTargets = Array.from(new Set(
    (targets || [])
      .map((target) => normalizeTarget(target))
      .filter(Boolean),
  ));
  const existingTargets = requestedTargets.filter((target) => fs.existsSync(path.resolve(repoRoot, target)));
  const missingTargets = requestedTargets.filter((target) => !fs.existsSync(path.resolve(repoRoot, target)));
  return {
    cwd,
    repoRoot,
    requestedTargets,
    existingTargets,
    missingTargets,
  };
}

export function printValidatorContextMismatchAndExit(toolName, context, extraDetails = []) {
  const label = String(toolName || "validator-check").trim();
  console.error(`${label}: CONTEXT_MISMATCH - product target paths are unavailable from the current checkout.`);
  const details = [
    `cwd=${context.cwd.replace(/\\/g, "/")}`,
    `repo_root=${String(context.repoRoot || context.cwd || "").replace(/\\/g, "/")}`,
    context.requestedTargets.length > 0
      ? `requested_targets=${context.requestedTargets.map((target) => target.replace(/\\/g, "/")).join(", ")}`
      : "requested_targets=<none>",
    context.existingTargets.length > 0
      ? `existing_targets=${context.existingTargets.map((target) => target.replace(/\\/g, "/")).join(", ")}`
      : "existing_targets=<none>",
    context.missingTargets.length > 0
      ? `missing_targets=${context.missingTargets.map((target) => target.replace(/\\/g, "/")).join(", ")}`
      : "missing_targets=<none>",
    "Run this command from a product checkout or pass explicit existing product paths.",
    ...extraDetails,
  ];
  for (const line of details) console.error(`- ${line}`);
  process.exit(2);
}

export function requireValidatorProductTargets(toolName, targets, {
  explicitTargets = false,
  requireAllExplicit = true,
  extraDetails = [],
} = {}) {
  const context = resolveValidatorProductTargetContext(targets);
  const missingExplicitTargets = explicitTargets && requireAllExplicit && context.missingTargets.length > 0;
  if (context.existingTargets.length === 0 || missingExplicitTargets) {
    printValidatorContextMismatchAndExit(toolName, context, extraDetails);
  }
  return context;
}
