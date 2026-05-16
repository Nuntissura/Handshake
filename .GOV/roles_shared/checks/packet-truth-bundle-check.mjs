/**
 * packet-truth-bundle-check.mjs [RGF-194]
 *
 * Bundled check that runs all packet/closure/merge truth checks in one pass
 * and collects ALL failures instead of failing on the first one.
 *
 * Sub-checks:
 *   - task-board-check.mjs
 *   - packet-closure-monitor-check.mjs
 *   - packet-truth-check.mjs
 *   - merge-progression-truth-check.mjs
 *   - task-packet-claim-check.mjs
 *   - packet-contract-projection-check.mjs
 *   - flat-packet-legacy-inventory-check.mjs
 *   - task-packet-stub-contract-check.mjs
 *   - wp-traceability-registry-projection-check.mjs (MD-ELIM-PHASE-2)
 */

import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { runBundledChecks } from "./bundled-check-runner-lib.mjs";

registerFailCaptureHook("packet-truth-bundle-check.mjs", { role: "SHARED" });

const failures = runBundledChecks(import.meta.url, [
  "task-board-check.mjs",
  "packet-closure-monitor-check.mjs",
  "packet-truth-check.mjs",
  "merge-progression-truth-check.mjs",
  "task-packet-claim-check.mjs",
  "packet-contract-projection-check.mjs",
  "flat-packet-legacy-inventory-check.mjs",
  "task-packet-stub-contract-check.mjs",
  "wp-traceability-registry-projection-check.mjs",
]);

if (failures.length > 0) {
  failWithMemory("packet-truth-bundle-check.mjs", `packet-truth-bundle: ${failures.length} sub-check(s) failed`, {
    role: "SHARED",
    details: failures,
  });
}

console.log("packet-truth-bundle-check ok");
