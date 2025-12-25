#!/usr/bin/env node
/**
 * Traceability audit:
 * - Ensures trace_id/job_id appear in governed mutation paths.
 * - Optional glob filters can be passed as arguments.
 *
 * Exits non-zero if required trace fields are absent.
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

const traceHits = runRg("trace_id");
const jobHits = runRg("job_id");

const failures = [];
if (!traceHits) failures.push("trace_id not found in governed paths");
if (!jobHits) failures.push("job_id not found in governed paths");

if (failures.length > 0) {
  console.error("validator-traceability: FAIL — missing traceability fields");
  failures.forEach((f) => console.error(`- ${f}`));
  process.exit(1);
}

console.log("validator-traceability: PASS — trace_id/job_id present in governed paths.");
