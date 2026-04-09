#!/usr/bin/env node

import { workflowStartReadinessState } from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import { ensureGovKernelTracksGov } from "../../../roles_shared/scripts/topology/reseed-permanent-worktree-from-main.mjs";
registerFailCaptureHook("orchestrator-startup-truth-check.mjs", { role: "ORCHESTRATOR" });

function fail(message, details = []) {
  failWithMemory("orchestrator-startup-truth-check.mjs", message, { role: "ORCHESTRATOR", details });
}

function main() {
  const govKernelState = ensureGovKernelTracksGov(process.cwd());
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
  if (govKernelState.normalized) {
    console.log("- gov_kernel_tracks_gov: true");
  }
  console.log(`- checked_wps: ${readiness.checkedWps}`);
  console.log(`- active_task_board_wps: ${readiness.activeBoardWpIds.length}`);
  console.log(`- gate_candidate_wps: ${readiness.activeCandidateWpIds.length}`);
}

main();
