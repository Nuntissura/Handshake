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

import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { runBundledChecks } from "./bundled-check-runner-lib.mjs";

registerFailCaptureHook("topology-bundle-check.mjs", { role: "SHARED" });

const failures = runBundledChecks(import.meta.url, [
  "worktree-concurrency-check.mjs",
  "drive-agnostic-check.mjs",
  "runtime-placement-check.mjs",
  "role-worktree-surface-check.mjs",
  "topology-registry-check.mjs",
]);

if (failures.length > 0) {
  failWithMemory("topology-bundle-check.mjs", `topology-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("topology-bundle-check ok");
