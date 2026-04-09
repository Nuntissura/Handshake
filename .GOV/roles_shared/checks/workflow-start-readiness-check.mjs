#!/usr/bin/env node

import { workflowStartReadinessState } from "../scripts/lib/role-resume-utils.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("workflow-start-readiness-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("workflow-start-readiness-check.mjs", message, { role: "SHARED", details });
}

const readiness = workflowStartReadinessState();

if (!readiness.ok) {
  fail("Workflow start readiness is split; repair active governance truth before more execution proceeds.", [
    `checked_wps=${readiness.checkedWps}`,
    `active_task_board_wps=${readiness.activeBoardWpIds.length}`,
    `gate_candidate_wps=${readiness.activeCandidateWpIds.length}`,
    ...readiness.violations,
  ]);
}

console.log("workflow-start-readiness-check ok");
console.log(`- checked_wps: ${readiness.checkedWps}`);
console.log(`- active_task_board_wps: ${readiness.activeBoardWpIds.length}`);
console.log(`- gate_candidate_wps: ${readiness.activeCandidateWpIds.length}`);
