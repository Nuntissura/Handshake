import fs from "node:fs";
import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionLaunchRequests,
  loadSessionRegistry,
  registryBatchLaunchSummary,
  registrySessionSummary,
} from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { evaluateSessionGovernanceState } from "../../../roles_shared/scripts/session/session-governance-state-lib.mjs";
import { evaluateWpTokenBudget } from "../../../roles_shared/scripts/session/wp-token-budget-lib.mjs";
import { readWpTokenUsageLedger } from "../../../roles_shared/scripts/session/wp-token-usage-lib.mjs";
import { REPO_ROOT, repoPathAbs, resolveWorkPacketPath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { evaluateWpCommunicationHealth } from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../../../roles_shared/scripts/lib/wp-relay-escalation-lib.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { checkAllNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";

const repoRoot = REPO_ROOT;
const wpIdFilter = String(process.argv[2] || "").trim();

const { registry } = loadSessionRegistry(repoRoot);
const batchSummary = registryBatchLaunchSummary(registry);
const { requests } = loadSessionLaunchRequests(repoRoot);
const { requests: controlRequests } = loadSessionControlRequests(repoRoot);
const { results: controlResults } = loadSessionControlResults(repoRoot);
const wpTokenUsage = wpIdFilter ? readWpTokenUsageLedger(repoRoot, wpIdFilter).ledger : null;
const wpTokenBudget = wpTokenUsage ? evaluateWpTokenBudget(wpTokenUsage) : null;
const wpGovernanceState = wpIdFilter
  ? evaluateSessionGovernanceState(repoRoot, { wp_id: wpIdFilter, local_worktree_dir: "" })
  : null;

const sessions = registry.sessions
  .filter((session) => !wpIdFilter || session.wp_id === wpIdFilter)
  .map((session) => registrySessionSummary(session));

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function isTerminalPacketStatus(status = "") {
  const text = String(status || "").trim();
  return /^Validated\s*\(/i.test(text)
    || /^Done$/i.test(text);
}

const terminalHistorySuppressed = Boolean(
  wpGovernanceState?.terminalTaskBoardStatus
  || isTerminalPacketStatus(wpGovernanceState?.packetStatus),
);

function loadRelayStatusForWp(wpId) {
  const packetPath = resolveWorkPacketPath(wpId)?.packetPath || "";
  const packetAbsPath = repoPathAbs(packetPath);
  if (!packetPath || !fs.existsSync(packetAbsPath)) return null;

  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  if (!runtimeStatusFile || !fs.existsSync(repoPathAbs(runtimeStatusFile))) return null;

  const runtimeStatus = parseJsonFile(runtimeStatusFile);
  const receipts = receiptsFile && fs.existsSync(repoPathAbs(receiptsFile)) ? parseJsonlFile(receiptsFile) : [];
  const communicationEvaluation = evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath,
    packetContent: packetText,
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE"),
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    receipts,
    runtimeStatus,
  });
  const pendingNotifications = Object.values(checkAllNotifications({ wpId })).flatMap((entry) => entry.notifications || []);
  return evaluateWpRelayEscalation({
    wpId,
    runtimeStatus,
    communicationEvaluation,
    receipts,
    pendingNotifications,
    registrySessions: registry.sessions || [],
  });
}

const relayStatus = wpIdFilter ? loadRelayStatusForWp(wpIdFilter) : null;

console.log("ROLE_SESSION_REGISTRY");
console.log(`- updated_at: ${registry.updated_at}`);
console.log(`- total_sessions: ${registry.sessions.length}`);
console.log(`- total_processed_requests: ${registry.processed_requests.length}`);
console.log(`- total_launch_requests: ${requests.length}`);
console.log(`- total_control_requests: ${controlRequests.length}`);
console.log(`- total_control_results: ${controlResults.length}`);
console.log(`- launch_batch_mode: ${batchSummary.launch_batch_mode}`);
console.log(`- launch_batch_plugin_failure_count: ${batchSummary.launch_batch_plugin_failure_count}`);
console.log(`- active_terminal_batch_id: ${batchSummary.active_terminal_batch_id || "<none>"}`);
console.log(`- active_terminal_batch_started_at: ${batchSummary.active_terminal_batch_started_at || "<none>"}`);
console.log(`- active_terminal_batch_last_rotated_at: ${batchSummary.active_terminal_batch_last_rotated_at || "<none>"}`);
console.log(`- active_terminal_batch_claimed_at: ${batchSummary.active_terminal_batch_claimed_at || "<none>"}`);
console.log(`- active_terminal_batch_reason: ${batchSummary.active_terminal_batch_reason || "<none>"}`);
if (batchSummary.launch_batch_switched_at) {
  console.log(`- launch_batch_switched_at: ${batchSummary.launch_batch_switched_at}`);
}
if (batchSummary.launch_batch_switch_reason) {
  console.log(`- launch_batch_switch_reason: ${batchSummary.launch_batch_switch_reason}`);
}

if (sessions.length === 0) {
  console.log("- matching_sessions: 0");
  if (!wpTokenUsage || wpTokenUsage.summary.command_count === 0) {
    process.exit(0);
  }
}

for (const session of sessions) {
  const governance = evaluateSessionGovernanceState(repoRoot, session);
  console.log("");
  console.log(`- session_key: ${session.session_key}`);
  console.log(`  role: ${session.role}`);
  console.log(`  wp_id: ${session.wp_id}`);
  console.log(`  runtime_state: ${session.runtime_state}`);
  console.log(`  requested_profile_id: ${session.requested_profile_id || "<none>"}`);
  console.log(`  task_board_status: ${governance.taskBoardStatus || "<missing>"}`);
  console.log(`  packet_status: ${governance.packetStatus || "<missing>"}`);
  console.log(`  local_worktree_exists: ${governance.localWorktreeExists ? "YES" : "NO"}`);
  console.log(`  steering_allowed: ${governance.steeringAllowed ? "YES" : "NO"}`);
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
  console.log(`  terminal_ownership_scope: ${session.terminal_ownership_scope || "<none>"}`);
  console.log(`  owned_terminal_process_id: ${session.owned_terminal_process_id || 0}`);
  console.log(`  owned_terminal_host_kind: ${session.owned_terminal_host_kind || "<none>"}`);
  console.log(`  owned_terminal_window_title: ${session.owned_terminal_window_title || "<none>"}`);
  console.log(`  owned_terminal_batch_id: ${session.owned_terminal_batch_id || "<none>"}`);
  console.log(`  owned_terminal_reclaim_status: ${session.owned_terminal_reclaim_status || "<none>"}`);
  if (session.owned_terminal_recorded_at) {
    console.log(`  owned_terminal_recorded_at: ${session.owned_terminal_recorded_at}`);
  }
  if (session.owned_terminal_reclaimed_at) {
    console.log(`  owned_terminal_reclaimed_at: ${session.owned_terminal_reclaimed_at}`);
  }
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
  } else if (session.runtime_state === "CLI_ESCALATION_USED") {
    console.log("  note: CLI escalation window launched; startup was requested, but no steerable thread or broker proof is registered yet");
  } else if (session.runtime_state === "COMMAND_RUNNING") {
    console.log("  note: governed broker owns the active run; cancellation is available through just session-cancel <ROLE> <WP_ID>");
  } else if (session.runtime_state === "READY" && governance.steeringAllowed) {
    console.log("  note: steerable Codex thread is registered and can be resumed through the governed control lane");
  } else if (session.runtime_state === "READY") {
    const reason = governance.steeringBlockers.join("; ") || "steering is not allowed";
    console.log(`  note: registered steerable thread is stale and should be closed before reuse (${reason})`);
  }
  console.log(`  updated_at: ${session.updated_at || "<none>"}`);
}

if (relayStatus) {
  console.log("");
  console.log("WP_RELAY_ESCALATION");
  console.log(`- wp_id: ${wpIdFilter}`);
  if (terminalHistorySuppressed) {
    console.log("- status: TERMINAL_HIDDEN");
    console.log("- severity: NONE");
    console.log("- reason_code: TERMINAL_HISTORY_HIDDEN");
    console.log("- summary: Relay escalation history is hidden by default for terminal WPs.");
  } else {
    console.log(`- status: ${relayStatus.status}`);
    console.log(`- severity: ${relayStatus.severity}`);
    console.log(`- reason_code: ${relayStatus.reason_code}`);
    console.log(`- target: ${relayStatus.target_role || "<none>"}${relayStatus.target_session ? `:${relayStatus.target_session}` : ""}`);
    console.log(`- summary: ${relayStatus.summary}`);
    if (relayStatus.recommended_command) {
      console.log(`- recommended_command: ${relayStatus.recommended_command}`);
    }
    const metrics = relayStatus.metrics || {};
    if (Object.keys(metrics).length > 0) {
      console.log(`- pending_notification_count: ${metrics.pending_notification_count ?? 0}`);
      console.log(`- route_anchor_at: ${metrics.route_anchor_at || "<none>"}`);
      console.log(`- latest_notification_at: ${metrics.latest_notification_at || "<none>"}`);
      console.log(`- latest_target_receipt_at: ${metrics.latest_target_receipt_at || "<none>"}`);
      console.log(`- latest_session_activity_at: ${metrics.latest_session_activity_at || "<none>"}`);
      console.log(`- heartbeat_due_at: ${metrics.heartbeat_due_at || "<none>"}`);
      console.log(`- stale_after: ${metrics.stale_after || "<none>"}`);
    }
  }
}

if (wpTokenUsage) {
  console.log("");
  console.log("WP_TOKEN_USAGE");
  console.log(`- wp_id: ${wpTokenUsage.wp_id}`);
  if (terminalHistorySuppressed) {
    console.log("- history_hidden: YES");
    console.log("- summary: Historical token-ledger drift and budget residue are hidden by default for terminal WPs.");
    console.log(`- settlement_status: ${wpTokenUsage.settlement?.status || "UNSETTLED"}`);
    if (wpTokenUsage.settlement?.settled_at) console.log(`- settled_at: ${wpTokenUsage.settlement.settled_at}`);
    console.log(`- command_count: ${wpTokenUsage.summary.command_count}`);
    console.log(`- turn_count: ${wpTokenUsage.summary.turn_count}`);
    console.log(`- input_tokens: ${wpTokenUsage.summary.usage_totals.input_tokens}`);
    console.log(`- output_tokens: ${wpTokenUsage.summary.usage_totals.output_tokens}`);
    console.log(`- summary_source: ${wpTokenUsage.summary_source}`);
  } else {
    console.log(`- summary_source: ${wpTokenUsage.summary_source}`);
    console.log(`- settlement_status: ${wpTokenUsage.settlement?.status || "UNSETTLED"}`);
    if (wpTokenUsage.settlement?.settled_at) console.log(`- settled_at: ${wpTokenUsage.settlement.settled_at}`);
    if (wpTokenUsage.settlement?.settled_reason) console.log(`- settled_reason: ${wpTokenUsage.settlement.settled_reason}`);
    if (wpTokenUsage.settlement?.settled_by) console.log(`- settled_by: ${wpTokenUsage.settlement.settled_by}`);
    console.log(`- ledger_health: ${wpTokenUsage.ledger_health.status}`);
    console.log(`- ledger_health_severity: ${wpTokenUsage.ledger_health.severity}`);
    console.log(`- ledger_health_drift_class: ${wpTokenUsage.ledger_health.drift_class}`);
    console.log(`- ledger_health_policy_id: ${wpTokenUsage.ledger_health.policy_id}`);
    console.log(`- ledger_health_blocker_class: ${wpTokenUsage.ledger_health.blocker_class}`);
    if (wpTokenUsage.ledger_health.invalidity_code) {
      console.log(`- ledger_health_invalidity_code: ${wpTokenUsage.ledger_health.invalidity_code}`);
    }
    console.log(`- ledger_health_summary: ${wpTokenUsage.ledger_health.summary}`);
    console.log(`- command_count: ${wpTokenUsage.summary.command_count}`);
    console.log(`- turn_count: ${wpTokenUsage.summary.turn_count}`);
    console.log(`- input_tokens: ${wpTokenUsage.summary.usage_totals.input_tokens}`);
    console.log(`- cached_input_tokens: ${wpTokenUsage.summary.usage_totals.cached_input_tokens}`);
    console.log(`- output_tokens: ${wpTokenUsage.summary.usage_totals.output_tokens}`);
    if (wpTokenUsage.ledger_health.status !== "NO_OUTPUTS") {
      console.log(`- tracked_command_count: ${wpTokenUsage.tracked_summary.command_count}`);
      console.log(`- tracked_turn_count: ${wpTokenUsage.tracked_summary.turn_count}`);
      console.log(`- raw_output_command_count: ${wpTokenUsage.raw_scan.summary.command_count}`);
      console.log(`- raw_output_turn_count: ${wpTokenUsage.raw_scan.summary.turn_count}`);
    }
    if (wpTokenUsage.ledger_health.status === "DRIFT") {
      console.log(`- drift_reason: ${wpTokenUsage.ledger_health.reason}`);
      console.log(`- command_delta_count: ${wpTokenUsage.ledger_health.metrics.command_delta_count}`);
      console.log(`- turn_delta: ${wpTokenUsage.ledger_health.metrics.turn_delta}`);
      console.log(`- input_token_delta: ${wpTokenUsage.ledger_health.metrics.input_token_delta}`);
      console.log(`- input_token_delta_ratio_pct: ${wpTokenUsage.ledger_health.metrics.input_token_delta_ratio_pct}`);
      if (wpTokenUsage.ledger_health.missing_tracked_command_count > 0) {
        console.log(`- missing_tracked_command_count: ${wpTokenUsage.ledger_health.missing_tracked_command_count}`);
        console.log(`- missing_tracked_command_ids_sample: ${wpTokenUsage.ledger_health.missing_tracked_command_ids_sample.join(", ")}`);
      }
      if (wpTokenUsage.ledger_health.stale_tracked_command_count > 0) {
        console.log(`- stale_tracked_command_count: ${wpTokenUsage.ledger_health.stale_tracked_command_count}`);
        console.log(`- stale_tracked_command_ids_sample: ${wpTokenUsage.ledger_health.stale_tracked_command_ids_sample.join(", ")}`);
      }
      if (wpTokenUsage.ledger_health.warnings.length > 0) {
        console.log(`- ledger_health_warnings: ${wpTokenUsage.ledger_health.warnings.join(" | ")}`);
      }
      if (wpTokenUsage.ledger_health.failures.length > 0) {
        console.log(`- ledger_health_failures: ${wpTokenUsage.ledger_health.failures.join(" | ")}`);
      }
    }
    const roleNames = Object.keys(wpTokenUsage.role_totals || {}).sort((left, right) => left.localeCompare(right));
    if (roleNames.length === 0) {
      console.log("- role_totals: <none>");
    } else {
      for (const roleName of roleNames) {
        const roleTotals = wpTokenUsage.role_totals[roleName];
        console.log(`- role: ${roleName}`);
        console.log(`  command_count: ${roleTotals.command_count}`);
        console.log(`  turn_count: ${roleTotals.turn_count}`);
        console.log(`  input_tokens: ${roleTotals.usage_totals.input_tokens}`);
        console.log(`  cached_input_tokens: ${roleTotals.usage_totals.cached_input_tokens}`);
        console.log(`  output_tokens: ${roleTotals.usage_totals.output_tokens}`);
      }
    }

    if (wpTokenBudget) {
      console.log("");
      console.log("WP_TOKEN_BUDGET");
      console.log(`- policy_id: ${wpTokenBudget.policy_id}`);
      console.log(`- status: ${wpTokenBudget.status}`);
      console.log(`- blocker_class: ${wpTokenBudget.blocker_class}`);
      if (wpTokenBudget.invalidity_code) {
        console.log(`- invalidity_code: ${wpTokenBudget.invalidity_code}`);
      }
      console.log(`- summary: ${wpTokenBudget.summary}`);
      if (wpTokenBudget.warnings.length > 0) {
        console.log(`- warnings: ${wpTokenBudget.warnings.join(" | ")}`);
      }
      if (wpTokenBudget.failures.length > 0) {
        console.log(`- failures: ${wpTokenBudget.failures.join(" | ")}`);
      }
    }
  }
}
