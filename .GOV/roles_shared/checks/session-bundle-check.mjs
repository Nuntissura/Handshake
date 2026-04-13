/**
 * session-bundle-check.mjs [RGF-194]
 *
 * Bundled check that runs all session/lifecycle checks in one pass
 * and collects ALL failures instead of failing on the first one.
 *
 * Replaces 4 individual gov-check imports:
 *   - session-policy-check.mjs
 *   - session-launch-runtime-check.mjs
 *   - session-control-runtime-check.mjs
 *   - lifecycle-ux-check.mjs
 */

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("session-bundle-check");

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

runCheck("session-policy-check.mjs");
runCheck("session-launch-runtime-check.mjs");
runCheck("session-control-runtime-check.mjs");
runCheck("lifecycle-ux-check.mjs");

if (failures.length > 0) {
  failWithMemory(`session-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("session-bundle-check ok");
