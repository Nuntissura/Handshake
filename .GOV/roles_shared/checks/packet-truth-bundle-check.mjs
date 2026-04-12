/**
 * packet-truth-bundle-check.mjs [RGF-194]
 *
 * Bundled check that runs all packet/closure/merge truth checks in one pass
 * and collects ALL failures instead of failing on the first one.
 *
 * Replaces 5 individual gov-check imports:
 *   - task-board-check.mjs
 *   - packet-closure-monitor-check.mjs
 *   - packet-truth-check.mjs
 *   - merge-progression-truth-check.mjs
 *   - task-packet-claim-check.mjs
 */

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("packet-truth-bundle-check");

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

runCheck("task-board-check.mjs");
runCheck("packet-closure-monitor-check.mjs");
runCheck("packet-truth-check.mjs");
runCheck("merge-progression-truth-check.mjs");
runCheck("task-packet-claim-check.mjs");

if (failures.length > 0) {
  failWithMemory(`packet-truth-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("packet-truth-bundle-check ok");
