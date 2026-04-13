/**
 * governance-structure-bundle-check.mjs [RGF-194]
 *
 * Bundled check that runs all governance structure/alignment checks in one pass
 * and collects ALL failures instead of failing on the first one.
 *
 * Replaces 4 individual gov-check imports:
 *   - protocol-alignment-check.mjs
 *   - prevention-ladder-check.mjs
 *   - deprecation-sunset-check.mjs
 *   - migration-path-truth-check.mjs
 */

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("governance-structure-bundle-check");

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

runCheck("protocol-alignment-check.mjs");
runCheck("prevention-ladder-check.mjs");
runCheck("deprecation-sunset-check.mjs");
runCheck("migration-path-truth-check.mjs");

if (failures.length > 0) {
  failWithMemory(`governance-structure-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("governance-structure-bundle-check ok");
