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

import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { runBundledChecks } from "./bundled-check-runner-lib.mjs";

registerFailCaptureHook("governance-structure-bundle-check");

const failures = runBundledChecks(import.meta.url, [
  "protocol-alignment-check.mjs",
  "prevention-ladder-check.mjs",
  "deprecation-sunset-check.mjs",
  "migration-path-truth-check.mjs",
]);

if (failures.length > 0) {
  failWithMemory(`governance-structure-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("governance-structure-bundle-check ok");
