#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { inspectHandshakeAcpBroker, shutdownHandshakeAcpBroker } from "../../../roles_shared/scripts/session/handshake-acp-client.mjs";
import { assertOrchestratorLaunchAuthority } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
registerFailCaptureHook("session-control-broker.mjs", { role: "ORCHESTRATOR" });

const action = String(process.argv[2] || "").trim().toLowerCase() || "status";

function fail(message) {
  failWithMemory("session-control-broker.mjs", message, { role: "ORCHESTRATOR" });
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], windowsHide: true }).trim();
}

const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
assertOrchestratorLaunchAuthority(runGit(["branch", "--show-current"]));

if (!["status", "stop"].includes(action)) {
  fail("Usage: node .GOV/roles/orchestrator/scripts/session-control-broker.mjs <status|stop>");
}

const inspected = await inspectHandshakeAcpBroker(repoRoot);
console.log(`[BROKER_CONTROL] alive=${inspected.brokerIsAlive ? "yes" : "no"}`);
console.log(`[BROKER_CONTROL] reachable=${inspected.brokerIsReachable ? "yes" : "no"}`);
console.log(`[BROKER_CONTROL] build_matches=${inspected.buildMatches ? "yes" : "no"}`);
if (inspected.state?.broker_pid) console.log(`[BROKER_CONTROL] pid=${inspected.state.broker_pid}`);
if (inspected.state?.port) console.log(`[BROKER_CONTROL] port=${inspected.state.port}`);
if (inspected.state?.broker_build_id) console.log(`[BROKER_CONTROL] build_id=${inspected.state.broker_build_id}`);
console.log(`[BROKER_CONTROL] active_runs=${Array.isArray(inspected.state?.active_runs) ? inspected.state.active_runs.length : 0}`);

if (action === "status") {
  process.exit(0);
}

const stopped = await shutdownHandshakeAcpBroker(repoRoot);
console.log(`[BROKER_CONTROL] status=${stopped.status}`);
if (stopped.result?.broker_build_id) {
  console.log(`[BROKER_CONTROL] result_build_id=${stopped.result.broker_build_id}`);
}

