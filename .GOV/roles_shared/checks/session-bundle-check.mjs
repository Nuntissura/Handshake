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

import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { runBundledChecks } from "./bundled-check-runner-lib.mjs";

registerFailCaptureHook("session-bundle-check.mjs", { role: "SHARED" });

const failures = runBundledChecks(import.meta.url, [
  "session-policy-check.mjs",
  "session-launch-runtime-check.mjs",
  "session-control-runtime-check.mjs",
  "lifecycle-ux-check.mjs",
]);

if (failures.length > 0) {
  failWithMemory("session-bundle-check.mjs", `session-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("session-bundle-check ok");
