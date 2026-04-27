#!/usr/bin/env node
/**
 * intelligent-review-cadence-check.mjs — RGF-254
 *
 * Read the INTELLIGENT_REVIEW_LAST_RUN.json marker, evaluate staleness against
 * the governed cadence (default 7 days), and emit a compact status line.
 *
 * Always exits 0 (governance-support, not product-correctness). When status is
 * DEBT, the closing actor and operator both see the cadence drift in
 * phase-check CLOSEOUT output and the IntVal context brief, so accumulated
 * one-off captures get reviewed instead of dead-lettering behind the
 * startup-injection access-count gate.
 */

import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import { buildIntelligentReviewCadenceStatus } from "../scripts/intelligent-review-status-lib.mjs";

registerFailCaptureHook("intelligent-review-cadence-check.mjs", { role: "MEMORY_MANAGER" });

try {
  const status = buildIntelligentReviewCadenceStatus({ now: Date.now() });
  const summaryParts = [
    `status=${status.status}`,
    `eval=${status.evaluation_status}`,
    `gate=${status.staleness_gate_days}d`,
  ];
  if (status.last_intelligent_review_iso) {
    summaryParts.push(`last=${status.last_intelligent_review_iso}`);
  }
  if (typeof status.days_since_intelligent_review === "number") {
    summaryParts.push(`age=${status.days_since_intelligent_review.toFixed(1)}d`);
  }
  if (status.status === "DEBT") {
    console.log(`intelligent-review-cadence-check DEBT: ${status.reason}`);
    console.log(`  ${summaryParts.join(" | ")}`);
    console.log(`  remediation: just launch-memory-manager-session`);
  } else {
    console.log(`intelligent-review-cadence-check ok: ${status.reason}`);
    console.log(`  ${summaryParts.join(" | ")}`);
  }
  process.exit(0);
} catch (error) {
  failWithMemory("intelligent-review-cadence-check.mjs", "intelligent-review cadence evaluation failed", {
    role: "MEMORY_MANAGER",
    details: [error?.message || String(error || "")],
  });
}
