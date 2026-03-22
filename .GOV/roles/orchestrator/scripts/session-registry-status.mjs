import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionLaunchRequests,
  loadSessionRegistry,
  registryBatchLaunchSummary,
  registrySessionSummary,
} from "../../../roles_shared/scripts/session/session-registry-lib.mjs";

const repoRoot = process.cwd();
const wpIdFilter = String(process.argv[2] || "").trim();

const { registry } = loadSessionRegistry(repoRoot);
const batchSummary = registryBatchLaunchSummary(registry);
const { requests } = loadSessionLaunchRequests(repoRoot);
const { requests: controlRequests } = loadSessionControlRequests(repoRoot);
const { results: controlResults } = loadSessionControlResults(repoRoot);

const sessions = registry.sessions
  .filter((session) => !wpIdFilter || session.wp_id === wpIdFilter)
  .map((session) => registrySessionSummary(session));

console.log("ROLE_SESSION_REGISTRY");
console.log(`- updated_at: ${registry.updated_at}`);
console.log(`- total_sessions: ${registry.sessions.length}`);
console.log(`- total_processed_requests: ${registry.processed_requests.length}`);
console.log(`- total_launch_requests: ${requests.length}`);
console.log(`- total_control_requests: ${controlRequests.length}`);
console.log(`- total_control_results: ${controlResults.length}`);
console.log(`- launch_batch_mode: ${batchSummary.launch_batch_mode}`);
console.log(`- launch_batch_plugin_failure_count: ${batchSummary.launch_batch_plugin_failure_count}`);
if (batchSummary.launch_batch_switched_at) {
  console.log(`- launch_batch_switched_at: ${batchSummary.launch_batch_switched_at}`);
}
if (batchSummary.launch_batch_switch_reason) {
  console.log(`- launch_batch_switch_reason: ${batchSummary.launch_batch_switch_reason}`);
}

if (sessions.length === 0) {
  console.log("- matching_sessions: 0");
  process.exit(0);
}

for (const session of sessions) {
  console.log("");
  console.log(`- session_key: ${session.session_key}`);
  console.log(`  role: ${session.role}`);
  console.log(`  wp_id: ${session.wp_id}`);
  console.log(`  runtime_state: ${session.runtime_state}`);
  console.log(`  control_mode: ${session.control_mode}`);
  console.log(`  control_protocol: ${session.control_protocol || "<none>"}`);
  console.log(`  control_transport: ${session.control_transport}`);
  console.log(`  session_thread_id: ${session.session_thread_id || "<none>"}`);
  console.log(`  startup_proof_state: ${session.startup_proof_state}`);
  console.log(`  preferred_host: ${session.preferred_host}`);
  console.log(`  active_host: ${session.active_host}`);
  console.log(`  active_terminal_kind: ${session.active_terminal_kind || "<none>"}`);
  console.log(`  plugin_request_count: ${session.plugin_request_count}`);
  console.log(`  plugin_failure_count: ${session.plugin_failure_count}`);
  console.log(`  plugin_last_result: ${session.plugin_last_result}`);
  console.log(`  cli_escalation_allowed: ${session.cli_escalation_allowed ? "YES" : "NO"}`);
  console.log(`  cli_escalation_used: ${session.cli_escalation_used ? "YES" : "NO"}`);
  console.log(`  active_terminal_title: ${session.active_terminal_title || "<none>"}`);
  console.log(`  last_command_kind: ${session.last_command_kind}`);
  console.log(`  last_command_status: ${session.last_command_status}`);
  console.log(`  last_command_output_file: ${session.last_command_output_file || "<none>"}`);
  if (session.last_command_summary) {
    const compactSummary = session.last_command_summary.replace(/\s+/g, " ").trim();
    const clippedSummary = compactSummary.length > 180 ? `${compactSummary.slice(0, 177)}...` : compactSummary;
    console.log(`  last_command_summary: ${clippedSummary}`);
  }
  if (session.runtime_state === "TERMINAL_COMMAND_DISPATCHED") {
    console.log("  note: bridge dispatched the governed command to a VS Code terminal; CLI startup is not yet proven by this state alone");
  } else if (session.runtime_state === "PLUGIN_CONFIRMED") {
    console.log("  note: legacy bridge ack; treat as terminal-only dispatch, not proof of an active CLI session");
  } else if (session.runtime_state === "COMMAND_RUNNING") {
    console.log("  note: governed broker owns the active run; cancellation is available through just session-cancel <ROLE> <WP_ID>");
  } else if (session.runtime_state === "READY") {
    console.log("  note: steerable Codex thread is registered and can be resumed through the governed control lane");
  }
  console.log(`  updated_at: ${session.updated_at || "<none>"}`);
}
