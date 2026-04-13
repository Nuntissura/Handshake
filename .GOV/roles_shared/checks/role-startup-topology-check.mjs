#!/usr/bin/env node

import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import {
  normalizeGovTrackingMode,
  normalizePermanentGovTracking,
} from "../scripts/topology/reseed-permanent-worktree-from-main.mjs";

registerFailCaptureHook("role-startup-topology-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("role-startup-topology-check.mjs", message, { role: "SHARED", details });
}

function formatState(prefix, state) {
  const lines = [
    `${prefix}- repo: ${state.repoDir}`,
    `${prefix}- worktree_id: ${state.worktreeId || "dynamic"}`,
    `${prefix}- worktree_role: ${state.worktreeRole || "dynamic"}`,
    `${prefix}- gov_mode: ${state.mode}`,
    `${prefix}- shared_gov_junction: ${state.sharedGovJunction ? "true" : "false"}`,
    `${prefix}- tracks_gov: ${state.tracksGov ? "true" : "false"}`,
  ];
  return lines.join("\n");
}

function parseArgs(argv) {
  const auditPermanent = argv.includes("--audit-permanent");
  return { auditPermanent };
}

function main() {
  const { auditPermanent } = parseArgs(process.argv.slice(2));
  const current = normalizeGovTrackingMode(process.cwd());
  if (!current.repoDir) {
    fail("Unable to resolve current repo for role-startup topology check.");
  }

  console.log("role-startup-topology-check ok");
  console.log(formatState("", current));

  if (!auditPermanent) return;

  const permanent = normalizePermanentGovTracking();
  console.log("- permanent_audit:");
  for (const state of permanent) {
    const prefix = "  ";
    if (!state.exists) {
      console.log(`${prefix}- worktree_id: ${state.worktreeId}`);
      console.log(`${prefix}- exists: false`);
      continue;
    }
    console.log(`${prefix}- worktree_id: ${state.worktreeId}`);
    console.log(`${prefix}- repo: ${state.repoDir}`);
    console.log(`${prefix}- worktree_role: ${state.worktreeRole || "dynamic"}`);
    console.log(`${prefix}- gov_mode: ${state.mode}`);
    console.log(`${prefix}- shared_gov_junction: ${state.sharedGovJunction ? "true" : "false"}`);
    console.log(`${prefix}- tracks_gov: ${state.tracksGov ? "true" : "false"}`);
  }
}

main();
