/**
 * wp-comm-bundle-check.mjs [RGF-194]
 *
 * Bundled check that runs all WP communication checks in one pass
 * and collects ALL failures instead of failing on the first one.
 *
 * Replaces 2 individual gov-check imports:
 *   - wp-communications-check.mjs
 *   - wp-activation-traceability-check.mjs
 */

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("wp-comm-bundle-check");

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

runCheck("wp-communications-check.mjs");
runCheck("wp-activation-traceability-check.mjs");

if (failures.length > 0) {
  failWithMemory(`wp-comm-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("wp-comm-bundle-check ok");
