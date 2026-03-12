import {
  loadSessionLaunchRequests,
  loadSessionRegistry,
  registrySessionSummary,
} from "./session-registry-lib.mjs";

const repoRoot = process.cwd();
const wpIdFilter = String(process.argv[2] || "").trim();

const { registry } = loadSessionRegistry(repoRoot);
const { requests } = loadSessionLaunchRequests(repoRoot);

const sessions = registry.sessions
  .filter((session) => !wpIdFilter || session.wp_id === wpIdFilter)
  .map((session) => registrySessionSummary(session));

console.log("ROLE_SESSION_REGISTRY");
console.log(`- updated_at: ${registry.updated_at}`);
console.log(`- total_sessions: ${registry.sessions.length}`);
console.log(`- total_processed_requests: ${registry.processed_requests.length}`);
console.log(`- total_launch_requests: ${requests.length}`);

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
  console.log(`  preferred_host: ${session.preferred_host}`);
  console.log(`  active_host: ${session.active_host}`);
  console.log(`  plugin_request_count: ${session.plugin_request_count}`);
  console.log(`  plugin_failure_count: ${session.plugin_failure_count}`);
  console.log(`  plugin_last_result: ${session.plugin_last_result}`);
  console.log(`  cli_escalation_allowed: ${session.cli_escalation_allowed ? "YES" : "NO"}`);
  console.log(`  cli_escalation_used: ${session.cli_escalation_used ? "YES" : "NO"}`);
  console.log(`  active_terminal_title: ${session.active_terminal_title || "<none>"}`);
  console.log(`  updated_at: ${session.updated_at || "<none>"}`);
}
