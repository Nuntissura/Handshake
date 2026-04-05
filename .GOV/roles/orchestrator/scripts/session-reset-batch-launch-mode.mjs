#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { assertOrchestratorLaunchAuthority, mutateSessionRegistrySync, resetBatchLaunchMode } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";

function currentBranch() {
  return execFileSync("git", ["branch", "--show-current"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  }).trim();
}

const reason = String(process.argv.slice(2).join(" ") || "").trim() || "operator-approved new governed batch";
const repoRoot = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  encoding: "utf8",
  stdio: ["ignore", "pipe", "ignore"],
}).trim();

assertOrchestratorLaunchAuthority(currentBranch());

const summary = mutateSessionRegistrySync(repoRoot, (registry) => {
  resetBatchLaunchMode(registry, reason);
  return {
    launch_batch_mode: registry.launch_batch_mode,
    launch_batch_plugin_failure_count: registry.launch_batch_plugin_failure_count,
    launch_batch_last_reset_at: registry.launch_batch_last_reset_at,
    launch_batch_switch_reason: registry.launch_batch_switch_reason,
    active_terminal_batch_id: registry.active_terminal_batch_id,
    active_terminal_batch_started_at: registry.active_terminal_batch_started_at,
  };
});

console.log("[SESSION_RESET_BATCH_LAUNCH_MODE] ok");
console.log(`mode=${summary.launch_batch_mode}`);
console.log(`plugin_failure_count=${summary.launch_batch_plugin_failure_count}`);
console.log(`last_reset_at=${summary.launch_batch_last_reset_at}`);
console.log(`reason=${summary.launch_batch_switch_reason}`);
console.log(`active_terminal_batch_id=${summary.active_terminal_batch_id}`);
console.log(`active_terminal_batch_started_at=${summary.active_terminal_batch_started_at}`);
