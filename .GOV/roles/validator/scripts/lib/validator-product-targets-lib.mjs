import fs from "node:fs";
import path from "node:path";

function normalizeTarget(target) {
  return path.normalize(String(target || "").trim());
}

export function resolveValidatorProductTargetContext(targets = []) {
  const requestedTargets = Array.from(new Set(
    (targets || [])
      .map((target) => normalizeTarget(target))
      .filter(Boolean),
  ));
  const existingTargets = requestedTargets.filter((target) => fs.existsSync(target));
  const missingTargets = requestedTargets.filter((target) => !fs.existsSync(target));
  return {
    cwd: process.cwd(),
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
