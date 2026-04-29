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

import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { runBundledChecks } from "./bundled-check-runner-lib.mjs";

registerFailCaptureHook("spec-bundle-check.mjs", { role: "SHARED" });

const failures = runBundledChecks(import.meta.url, [
  "spec-debt-registry-check.mjs",
  "spec-eof-appendices-check.mjs",
  "spec-growth-discipline-check.mjs",
  "spec-governance-reference-check.mjs",
]);

if (failures.length > 0) {
  failWithMemory("spec-bundle-check.mjs", `spec-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("spec-bundle-check ok");
