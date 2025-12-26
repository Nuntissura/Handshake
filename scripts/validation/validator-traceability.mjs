#!/usr/bin/env node
/**
 * Traceability audit:
 * - Ensures job_id appears in governed mutation paths.
 * - Emits a warning (non-fatal) if trace_id is absent.
 *
 * Exits non-zero only if required fields are absent.
 */
import { execSync } from "node:child_process";

const targets = process.argv.slice(2);
const defaultTargets = [
  "src/backend/handshake_core/src/workflows.rs",
  "src/backend/handshake_core/src/api",
  "src/backend/handshake_core/src/storage",
];

const scopes = targets.length > 0 ? targets : defaultTargets;

function runRg(pattern) {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${scopes.join(
    " "
  )}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    console.error(`validator-traceability: scan failed: ${err.message}`);
    process.exit(1);
  }
}

const jobHits = runRg("job_id");
const traceHits = runRg("trace_id");

const failures = [];
const warnings = [];
if (!jobHits) failures.push("job_id not found in governed paths");
if (!traceHits) warnings.push("trace_id not found in governed paths (warning only)");

if (failures.length > 0) {
  console.error("validator-traceability: FAIL — missing traceability fields");
  failures.forEach((f) => console.error(`- ${f}`));
  warnings.forEach((w) => console.error(`- ${w}`));
  process.exit(1);
}

warnings.forEach((w) => console.warn(`validator-traceability: WARN — ${w}`));
console.log("validator-traceability: PASS — required traceability fields present.");
