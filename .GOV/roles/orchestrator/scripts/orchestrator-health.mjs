#!/usr/bin/env node

import {
  inspectHandshakeAcpBroker,
} from "../../../roles_shared/scripts/session/handshake-acp-client.mjs";
import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionLaunchRequests,
  loadSessionRegistry,
  registryBatchLaunchSummary,
  registrySessionSummary,
} from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { evaluateSessionGovernanceState } from "../../../roles_shared/scripts/session/session-governance-state-lib.mjs";
import { REPO_ROOT } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  formatDuration,
  latestSessionActivityIso,
  nextSafeCommand,
  parseTimestampMs,
  secondsSince,
  sessionHealthLine,
} from "./orchestrator-health-lib.mjs";

const wpId = String(process.argv[2] || "").trim();
const now = new Date();
const { registry } = loadSessionRegistry(REPO_ROOT);
const batchSummary = registryBatchLaunchSummary(registry);
const { requests: launchRequests } = loadSessionLaunchRequests(REPO_ROOT);
const { requests: controlRequests } = loadSessionControlRequests(REPO_ROOT);
const { results: controlResults } = loadSessionControlResults(REPO_ROOT);
const rawSessions = (registry.sessions || []).filter((session) => !wpId || session.wp_id === wpId);
const allSessions = rawSessions.map((session) => registrySessionSummary(session));
const sessions = selectVisibleSessions(allSessions, { wpId });
const rawByKey = new Map(rawSessions.map((session) => [session.session_key, session]));
const broker = await inspectHandshakeAcpBroker(REPO_ROOT).catch((error) => ({
  brokerIsAlive: false,
  brokerIsReachable: false,
  buildMatches: false,
  state: null,
  error: error?.message || String(error || ""),
}));
const brokerActiveRuns = Array.isArray(broker.state?.active_runs) ? broker.state.active_runs : [];
const filteredBrokerActiveRuns = wpId
  ? brokerActiveRuns.filter((run) => String(run?.wp_id || run?.wpId || "").trim() === wpId)
  : brokerActiveRuns;
const governance = wpId
  ? evaluateSessionGovernanceState(REPO_ROOT, { wp_id: wpId, local_worktree_dir: "" })
  : null;
const latestActivityIso = latestSessionActivityIso({
  updated_at: registry.updated_at,
  health_updated_at: newest(sessions.map((session) => latestSessionActivityIso(session))),
});

function yesNo(value) {
  return value ? "YES" : "NO";
}

function newest(values = []) {
  return values
    .filter(Boolean)
    .sort((left, right) => Date.parse(right) - Date.parse(left))[0] || "";
}

function isTerminalHistoricalSession(session = {}) {
  return ["CLOSED", "COMPLETED"].includes(String(session.runtime_state || "").trim().toUpperCase());
}

function selectVisibleSessions(sessionSummaries = [], { wpId: targetWpId = "" } = {}) {
  if (targetWpId) return sessionSummaries;
  const byActivity = [...sessionSummaries].sort((left, right) =>
    parseTimestampMs(latestSessionActivityIso(right)) - parseTimestampMs(latestSessionActivityIso(left))
  );
  const activeish = byActivity.filter((session) => !isTerminalHistoricalSession(session));
  return (activeish.length > 0 ? activeish : byActivity).slice(0, 12);
}

console.log("ORCHESTRATOR_HEALTH");
console.log(`- wp_id: ${wpId || "<all>"}`);
console.log(`- generated_at: ${now.toISOString()}`);
console.log(`- registry_updated_at: ${registry.updated_at || "<none>"}`);
console.log(`- registry_stale: ${formatDuration(secondsSince(latestActivityIso, now))}`);
console.log(`- total_sessions: ${(registry.sessions || []).length}`);
console.log(`- matching_sessions: ${allSessions.length}`);
console.log(`- displayed_sessions: ${sessions.length}`);
if (!wpId && allSessions.length > sessions.length) {
  console.log(`- hidden_historical_sessions: ${allSessions.length - sessions.length}`);
}
console.log(`- launch_requests: ${launchRequests.length}`);
console.log(`- control_requests: ${controlRequests.length}`);
console.log(`- control_results: ${controlResults.length}`);
console.log(`- broker_alive: ${yesNo(broker.brokerIsAlive)}`);
console.log(`- broker_reachable: ${yesNo(broker.brokerIsReachable)}`);
console.log(`- broker_build_matches: ${yesNo(broker.buildMatches)}`);
console.log(`- broker_pid: ${broker.state?.broker_pid || "<none>"}`);
console.log(`- broker_port: ${broker.state?.port || "<none>"}`);
console.log(`- broker_active_runs: ${filteredBrokerActiveRuns.length}`);
if (broker.error) console.log(`- broker_error: ${broker.error}`);
console.log(`- active_terminal_batch_id: ${batchSummary.active_terminal_batch_id || "<none>"}`);
console.log(`- launch_batch_mode: ${batchSummary.launch_batch_mode || "<none>"}`);

if (governance) {
  console.log("");
  console.log("WP_LIFECYCLE");
  console.log(`- task_board_status: ${governance.taskBoardStatus || "<missing>"}`);
  console.log(`- packet_status: ${governance.packetStatus || "<missing>"}`);
  console.log(`- workflow_lane: ${governance.workflowLane || "<unknown>"}`);
  console.log(`- local_worktree_exists: ${yesNo(governance.localWorktreeExists)}`);
  console.log(`- steering_allowed: ${yesNo(governance.steeringAllowed)}`);
  if (Array.isArray(governance.steeringBlockers) && governance.steeringBlockers.length > 0) {
    console.log(`- steering_blockers: ${governance.steeringBlockers.join("; ")}`);
  }
}

console.log("");
console.log("ROLE_SESSIONS");
if (sessions.length === 0) {
  console.log("- <none>");
} else {
  for (const session of sessions) {
    console.log(`- ${sessionHealthLine(session, rawByKey.get(session.session_key) || {}, now)}`);
  }
}

if (filteredBrokerActiveRuns.length > 0) {
  console.log("");
  console.log("ACP_ACTIVE_RUNS");
  for (const run of filteredBrokerActiveRuns) {
    console.log(`- command=${run.command_id || "<none>"} | role=${run.role || "<unknown>"} | wp=${run.wp_id || run.wpId || "<unknown>"} | started=${run.started_at || "<unknown>"}`);
  }
}

console.log("");
console.log(`NEXT_SAFE_COMMAND: ${nextSafeCommand({ wpId, sessions, brokerActiveRunCount: filteredBrokerActiveRuns.length })}`);
