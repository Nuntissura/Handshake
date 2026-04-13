/**
 * spec-bundle-check.mjs [RGF-194]
 *
 * Bundled check that runs all spec discipline checks in one pass
 * and collects ALL failures instead of failing on the first one.
 *
 * Replaces 4 individual gov-check imports:
 *   - spec-debt-registry-check.mjs
 *   - spec-eof-appendices-check.mjs
 *   - spec-growth-discipline-check.mjs
 *   - spec-governance-reference-check.mjs
 */

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("spec-bundle-check");

const checksDir = path.dirname(fileURLToPath(import.meta.url));
const failures = [];

function runCheck(scriptName) {
  try {
    execFileSync(process.execPath, [path.join(checksDir, scriptName)], {
      stdio: ["ignore", "inherit", "inherit"],
      timeout: 60000,
      env: process.env,
      cwd: process.cwd(),
    });
  } catch {
    failures.push(scriptName.replace(".mjs", ""));
  }
}

runCheck("spec-debt-registry-check.mjs");
runCheck("spec-eof-appendices-check.mjs");
runCheck("spec-growth-discipline-check.mjs");
runCheck("spec-governance-reference-check.mjs");

if (failures.length > 0) {
  failWithMemory(`spec-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("spec-bundle-check ok");
