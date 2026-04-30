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

import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { runBundledChecks } from "./bundled-check-runner-lib.mjs";

registerFailCaptureHook("wp-comm-bundle-check.mjs", { role: "SHARED" });

const failures = runBundledChecks(import.meta.url, [
  "wp-communications-check.mjs",
  "wp-activation-traceability-check.mjs",
]);

if (failures.length > 0) {
  failWithMemory("wp-comm-bundle-check.mjs", `wp-comm-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("wp-comm-bundle-check ok");
