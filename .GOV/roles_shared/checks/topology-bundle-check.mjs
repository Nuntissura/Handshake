/**
 * topology-bundle-check.mjs [RGF-194]
 *
 * Bundled check that runs all worktree/topology/placement checks in one pass
 * and collects ALL failures instead of failing on the first one.
 *
 * Replaces 5 individual gov-check imports:
 *   - worktree-concurrency-check.mjs
 *   - drive-agnostic-check.mjs
 *   - runtime-placement-check.mjs
 *   - role-worktree-surface-check.mjs
 *   - topology-registry-check.mjs
 */

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("topology-bundle-check");

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

runCheck("worktree-concurrency-check.mjs");
runCheck("drive-agnostic-check.mjs");
runCheck("runtime-placement-check.mjs");
runCheck("role-worktree-surface-check.mjs");
runCheck("topology-registry-check.mjs");

if (failures.length > 0) {
  failWithMemory(`topology-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("topology-bundle-check ok");
