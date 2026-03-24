#!/usr/bin/env node

import { workflowStartReadinessState } from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";

function fail(message, details = []) {
  console.error(`[ORCHESTRATOR_STARTUP_TRUTH_CHECK] ${message}`);
  for (const detail of details) console.error(`- ${detail}`);
  process.exit(1);
}

function main() {
  const readiness = workflowStartReadinessState();

  if (!readiness.ok) {
    fail("Active orchestrator authority surfaces are split; fix startup truth before more execution proceeds.", [
      `checked_wps=${readiness.checkedWps || 0}`,
      `active_task_board_wps=${readiness.activeBoardWpIds.length}`,
      `gate_candidate_wps=${readiness.activeCandidateWpIds.length}`,
      ...readiness.violations,
      "Run `just orchestrator-next WP-{ID}` on the failing WPs and repair STATUS_SYNC before launching more work.",
    ]);
  }

  console.log("orchestrator-startup-truth-check ok");
  console.log(`- checked_wps: ${readiness.checkedWps}`);
  console.log(`- active_task_board_wps: ${readiness.activeBoardWpIds.length}`);
  console.log(`- gate_candidate_wps: ${readiness.activeCandidateWpIds.length}`);
}

main();
