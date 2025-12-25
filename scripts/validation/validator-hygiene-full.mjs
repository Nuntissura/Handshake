#!/usr/bin/env node
/**
 * Composite hygiene runner for validators.
 * Runs scan, error-codes, traceability, and git hygiene checks.
 */
import { execSync } from "node:child_process";

const cmds = [
  "node scripts/validation/validator-scan.mjs",
  "node scripts/validation/validator-error-codes.mjs",
  "node scripts/validation/validator-traceability.mjs",
  "node scripts/validation/validator-git-hygiene.mjs",
];

function run(cmd) {
  try {
    execSync(cmd, { stdio: "inherit" });
  } catch (err) {
    console.error(`validator-hygiene-full: FAIL — command failed: ${cmd}`);
    process.exit(1);
  }
}

for (const cmd of cmds) {
  run(cmd);
}

console.log("validator-hygiene-full: PASS — composite hygiene clean.");
