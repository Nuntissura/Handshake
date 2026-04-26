import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT, repoPathAbs, workPacketPath } from "../lib/runtime-paths.mjs";
import { parseJsonFile, parseJsonlFile } from "../lib/wp-communications-lib.mjs";
import { evaluateWpCommunicationHealth } from "../lib/wp-communication-health-lib.mjs";
import { readExecutionProjectionView } from "../lib/wp-execution-state-lib.mjs";
import { deriveWpMicrotaskPlan } from "../lib/wp-microtask-lib.mjs";
import {
  normalizeRelayEscalationPolicy,
  relayEscalationPolicyBudgetLabel,
} from "../lib/wp-relay-policy-lib.mjs";
import { evaluateWpRelayEscalation } from "../lib/wp-relay-escalation-lib.mjs";
import { checkAllNotifications, checkNotifications } from "../wp/wp-check-notifications.mjs";
import { evaluateSessionGovernanceState } from "./session-governance-state-lib.mjs";
import { loadSessionRegistry } from "./session-registry-lib.mjs";
import { buildRoleAuthorityString, resolveRoleConfig } from "./session-control-lib.mjs";
import { SESSION_CONTROL_BROKER_STATE_FILE, sessionKey } from "./session-policy.mjs";
import {
  activeRunsForSession,
  buildSessionTelemetry,
  formatPushAlertInline,
  formatSessionRunTelemetryInline,
  formatSessionStepTelemetryInline,
  selectLatestPushAlert,
} from "./session-telemetry-lib.mjs";

const COMMAND_SURFACE_PATH = ".GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md";

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function normalize(value, fallback = "<none>") {
  const text = String(value || "").trim();
  return text || fallback;
}

function isTerminalPacketStatus(status = "") {
  const text = String(status || "").trim();
  return /^Validated\s*\(/i.test(text)
    || /^Done$/i.test(text);
}

function summarizeMicrotaskContract(value = null) {
  if (!value || typeof value !== "object") return null;
  return {
    scope_ref: normalize(value.scope_ref),
    file_targets: Array.isArray(value.file_targets) ? value.file_targets.filter(Boolean).slice(0, 6) : [],
    proof_commands: Array.isArray(value.proof_commands) ? value.proof_commands.filter(Boolean).slice(0, 4) : [],
    risk_focus: normalize(value.risk_focus),
    expected_receipt_kind: normalize(value.expected_receipt_kind),
    review_mode: normalize(value.review_mode),
    phase_gate: normalize(value.phase_gate),
    review_outcome: normalize(value.review_outcome),
  };
}

function normalizeSession(value) {
  const text = String(value || "").trim();
  return text || null;
}

function summarizeMicrotaskEntry(entry = null) {
  if (!entry || typeof entry !== "object") return null;
  return {
    mt_id: normalize(entry.mt_id),
    clause: normalize(entry.clause),
    state: normalize(entry.state),
    state_reason: normalize(entry.state_reason),
    correlation_id: normalize(entry.correlation_id),
    last_receipt_kind: normalize(entry.last_receipt_kind),
    last_actor_role: normalize(entry.last_actor_role),
    last_activity_at: normalize(entry.last_activity_at),
    code_surfaces: Array.isArray(entry.code_surfaces) ? entry.code_surfaces.slice(0, 6) : [],
  };
}

function preferredRoleSession(runtimeStatus = {}, role = "") {
  const normalizedRole = String(role || "").trim().toUpperCase();
  if (!normalizedRole) return null;

  const nextRole = String(runtimeStatus?.next_expected_actor || "").trim().toUpperCase();
  const nextSession = normalizeSession(runtimeStatus?.next_expected_session);
  if (nextRole === normalizedRole && nextSession) return nextSession;

  const activeSessions = (Array.isArray(runtimeStatus?.active_role_sessions) ? runtimeStatus.active_role_sessions : [])
    .filter((entry) => String(entry?.role || "").trim().toUpperCase() === normalizedRole)
    .map((entry) => normalizeSession(entry?.session_id))
    .filter(Boolean);
  const uniqueSessions = [...new Set(activeSessions)];
  if (uniqueSessions.length === 1) return uniqueSessions[0];

  return null;
}

export function buildActiveLaneBrief({
  repoRoot = REPO_ROOT,
  role = "",
  wpId = "",
} = {}) {
  const normalizedRole = String(role || "").trim().toUpperCase();
  const roleConfig = resolveRoleConfig(normalizedRole, wpId);
  if (!roleConfig) {
    throw new Error(`Unsupported role for active-lane brief: ${role}`);
  }

  const packetPath = workPacketPath(wpId);
  const packetAbsPath = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbsPath)) {
    throw new Error(`Task packet not found: ${packetPath}`);
  }
  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  const governanceRepoRoot = path.resolve(path.dirname(packetAbsPath), "..", "..", "..");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const runtimeProjection = runtimeStatusFile && fs.existsSync(repoPathAbs(runtimeStatusFile))
    ? readExecutionProjectionView(parseJsonFile(runtimeStatusFile))
    : readExecutionProjectionView({});
  const runtimeStatus = runtimeProjection.runtime;
  const receipts = receiptsFile && fs.existsSync(repoPathAbs(receiptsFile))
    ? parseJsonlFile(receiptsFile)
    : [];
  const { registry } = loadSessionRegistry(repoRoot);
  const session = (registry.sessions || []).find((entry) => entry.session_key === sessionKey(normalizedRole, wpId)) || null;
  const governanceState = evaluateSessionGovernanceState(governanceRepoRoot, {
    wp_id: wpId,
    local_worktree_dir: roleConfig.worktreeDir,
  });
  const notifications = checkNotifications({ wpId, role: normalizedRole });
  const pendingNotifications = Object.values(checkAllNotifications({ wpId })).flatMap((entry) => entry.notifications || []);
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
  const relay = evaluateWpRelayEscalation({
    wpId,
    runtimeStatus,
    communicationEvaluation,
    receipts,
    pendingNotifications,
    registrySessions: registry.sessions || [],
  });
  const terminalNoiseSuppressed = Boolean(
    governanceState.terminalTaskBoardStatus
    || isTerminalPacketStatus(governanceState.packetStatus)
    || runtimeProjection.terminal,
  );
  const hiddenNotificationCount = notifications.historyHidden
    ? Number(notifications.hiddenPendingCount || 0)
    : Number(notifications.pendingCount || 0);
  const hiddenNotificationKinds = notifications.historyHidden
    ? (notifications.hiddenByKind || {})
    : (notifications.byKind || {});
  const visibleNotifications = terminalNoiseSuppressed
    ? { pendingCount: 0, byKind: {} }
    : notifications;
  const hiddenHistory = terminalNoiseSuppressed || notifications.historyHidden
    ? {
        pending_notification_count: hiddenNotificationCount,
        pending_notification_by_kind: hiddenNotificationKinds,
      }
    : null;
  const relayView = terminalNoiseSuppressed
    ? {
        status: "TERMINAL_HIDDEN",
        severity: "NONE",
        summary: "Relay escalation is hidden for terminal WPs by default; use runtime/history surfaces only when explicitly needed.",
        reason_code: "TERMINAL_HISTORY_HIDDEN",
        recommended_command: null,
      }
    : relay;
  const relayPolicyView = terminalNoiseSuppressed
    ? null
    : normalizeRelayEscalationPolicy(runtimeStatus?.relay_escalation_policy);
  const preferredSession = preferredRoleSession(runtimeStatus, normalizedRole);
  const brokerActiveRuns = fs.existsSync(repoPathAbs(SESSION_CONTROL_BROKER_STATE_FILE))
    ? (parseJsonFile(SESSION_CONTROL_BROKER_STATE_FILE)?.active_runs || [])
    : [];
  const sessionTelemetry = buildSessionTelemetry({
    session,
    activeRuns: activeRunsForSession(session, brokerActiveRuns),
    repoRoot,
  });
  const latestPushAlert = selectLatestPushAlert(pendingNotifications, {
    targetRole: normalizedRole,
    targetSession: preferredSession || "",
  });
  const reviewQueue = terminalNoiseSuppressed
    ? []
    : (Array.isArray(runtimeStatus.open_review_items) ? runtimeStatus.open_review_items : [])
        .filter((item) => {
          if (String(item?.target_role || "").trim().toUpperCase() !== normalizedRole) return false;
          const targetSession = normalizeSession(item?.target_session);
          if (!targetSession) return true;
          if (!preferredSession) return false;
          return targetSession === preferredSession;
        })
        .slice(0, 4)
        .map((item) => ({
          correlation_id: normalize(item?.correlation_id),
          receipt_kind: normalize(item?.receipt_kind),
          summary: normalize(item?.summary),
          opened_by_role: normalize(item?.opened_by_role),
          opened_by_session: normalize(item?.opened_by_session),
          target_session: normalize(item?.target_session),
          spec_anchor: normalize(item?.spec_anchor),
          packet_row_ref: normalize(item?.packet_row_ref),
          microtask_contract: summarizeMicrotaskContract(item?.microtask_contract),
        }));
  const microtaskPlan = terminalNoiseSuppressed
    ? { declared_count: 0, active_microtask: null, previous_microtask: null, suggested_next_microtask: null, items: [] }
    : deriveWpMicrotaskPlan({
        wpId,
        receipts,
        runtimeStatus,
      });

  return {
    schema_id: "hsk.active_lane_brief@1",
    schema_version: "active_lane_brief_v1",
    role: normalizedRole,
    wp_id: wpId,
    authority: buildRoleAuthorityString(normalizedRole, wpId),
    packet_path: packetPath,
    command_surface_path: COMMAND_SURFACE_PATH,
    role_config: {
      branch: roleConfig.branch,
      worktree_dir: roleConfig.worktreeDir,
      startup_command: roleConfig.startupCommand,
      next_command: roleConfig.nextCommand,
    },
    runtime: {
      status: normalize(runtimeProjection.runtime_status),
      phase: normalize(runtimeProjection.phase),
      milestone: normalize(runtimeProjection.milestone),
      task_board_status: normalize(runtimeProjection.task_board_status),
      next_expected_actor: normalize(runtimeProjection.next_expected_actor),
      next_expected_session: normalize(runtimeProjection.next_expected_session),
      waiting_on: normalize(runtimeProjection.waiting_on),
      waiting_on_session: normalize(runtimeProjection.waiting_on_session),
      last_event: normalize(runtimeProjection.last_event),
      open_review_items: Number(runtimeProjection.open_review_items_count || 0),
    },
    session: {
      session_key: normalize(session?.session_key),
      actor_session: normalize(preferredSession),
      requested_profile_id: normalize(session?.requested_profile_id),
      runtime_state: normalize(session?.runtime_state),
      thread_id: normalize(session?.session_thread_id),
      last_command_kind: normalize(session?.last_command_kind),
      last_command_status: normalize(session?.last_command_status),
      effective_governed_action: {
        command_kind: normalize(session?.effective_governed_action?.command_kind),
        command_status: normalize(session?.effective_governed_action?.status),
        action_kind: normalize(session?.effective_governed_action?.action_kind),
        action_state: normalize(session?.effective_governed_action?.action_state),
        rule_id: normalize(session?.effective_governed_action?.rule_id),
        resume_disposition: normalize(session?.effective_governed_action?.resume_disposition),
        source: normalize(session?.effective_governed_action?.source),
      },
      last_governed_action: {
        action_id: normalize(session?.last_governed_action?.action_id),
        rule_id: normalize(session?.last_governed_action?.rule_id),
        action_kind: normalize(session?.last_governed_action?.action_kind),
        action_state: normalize(session?.last_governed_action?.action_state),
        resume_disposition: normalize(session?.last_governed_action?.resume_disposition),
      },
      pending_control_queue_count: Number(session?.pending_control_queue_count || 0),
      next_queued_control_request: {
        command_id: normalize(session?.next_queued_control_request?.command_id),
        command_kind: normalize(session?.next_queued_control_request?.command_kind),
        queue_reason_code: normalize(session?.next_queued_control_request?.queue_reason_code),
      },
      telemetry: {
        run: sessionTelemetry.run,
        step: sessionTelemetry.step,
        latest_push_alert: latestPushAlert,
      },
    },
    notifications: {
      pending_count: visibleNotifications.pendingCount || 0,
      by_kind: visibleNotifications.byKind || {},
      history_hidden: terminalNoiseSuppressed || notifications.historyHidden,
      hidden_history: hiddenHistory,
    },
    microtasks: {
      declared_count: Number(microtaskPlan.declared_count || 0),
      active_microtask: summarizeMicrotaskEntry(microtaskPlan.active_microtask),
      previous_microtask: summarizeMicrotaskEntry(microtaskPlan.previous_microtask),
      suggested_next_microtask: summarizeMicrotaskEntry(microtaskPlan.suggested_next_microtask),
      items: Array.isArray(microtaskPlan.items) ? microtaskPlan.items.map((entry) => summarizeMicrotaskEntry(entry)) : [],
    },
    review_queue: reviewQueue,
    relay: {
      status: relayView.status,
      severity: relayView.severity,
      summary: relayView.summary,
      reason_code: relayView.reason_code,
      recommended_command: relayView.recommended_command,
      policy: relayPolicyView,
    },
    minimal_live_read_set: [
      "startup output",
      "active packet",
      "active WP thread/notifications",
      COMMAND_SURFACE_PATH,
    ],
    next_commands: [
      roleConfig.nextCommand,
      `just check-notifications ${wpId} ${normalizedRole} ${preferredSession || "<your-session>"}`,
      `just ack-notifications ${wpId} ${normalizedRole} ${preferredSession || "<your-session>"}`,
    ],
  };
}

export function formatActiveLaneBrief(brief) {
  const relayPolicyBudget = brief.relay.policy
    ? relayEscalationPolicyBudgetLabel(brief.relay.policy)
    : null;
  return [
    "ACTIVE_LANE_BRIEF [CX-LANE-001]",
    `- ROLE: ${brief.role} | WP_ID: ${brief.wp_id}`,
    `- AUTHORITY: ${brief.authority}`,
    `- PACKET: ${brief.packet_path}`,
    `- ROLE_CONTEXT: branch=${brief.role_config.branch} | worktree=${brief.role_config.worktree_dir}`,
    `- RUNTIME: status=${brief.runtime.status} | phase=${brief.runtime.phase} | milestone=${brief.runtime.milestone} | board=${brief.runtime.task_board_status} | next=${brief.runtime.next_expected_actor}${brief.runtime.next_expected_session !== "<none>" ? `:${brief.runtime.next_expected_session}` : ""} | waiting_on=${brief.runtime.waiting_on}${brief.runtime.waiting_on_session !== "<none>" ? ` (${brief.runtime.waiting_on_session})` : ""}`,
    `- SESSION: key=${brief.session.session_key} | actor_session=${brief.session.actor_session} | runtime_state=${brief.session.runtime_state} | thread=${brief.session.thread_id} | effective_command=${brief.session.effective_governed_action.command_kind}/${brief.session.effective_governed_action.command_status} | effective_action=${brief.session.effective_governed_action.action_kind}/${brief.session.effective_governed_action.action_state} | disposition=${brief.session.effective_governed_action.resume_disposition} | source=${brief.session.effective_governed_action.source} | queued=${brief.session.pending_control_queue_count}${brief.session.next_queued_control_request.command_id !== "<none>" ? ` | next_queue=${brief.session.next_queued_control_request.command_kind}:${brief.session.next_queued_control_request.command_id}` : ""}`,
    `- SESSION_TELEMETRY: ${formatSessionRunTelemetryInline(brief.session.telemetry?.run)} | ${formatSessionStepTelemetryInline(brief.session.telemetry?.step)}${brief.session.telemetry?.latest_push_alert ? ` | ${formatPushAlertInline(brief.session.telemetry.latest_push_alert)}` : ""}`,
    `- NOTIFICATIONS: pending=${brief.notifications.pending_count} | by_kind=${JSON.stringify(brief.notifications.by_kind)}`,
    `- MICROTASKS: declared=${brief.microtasks.declared_count} | active=${brief.microtasks.active_microtask?.mt_id || "<none>"} | next=${brief.microtasks.suggested_next_microtask?.mt_id || "<none>"}`,
    ...(brief.microtasks.active_microtask
      ? [
          `- ACTIVE_MICROTASK: ${brief.microtasks.active_microtask.mt_id} | state=${brief.microtasks.active_microtask.state} | reason=${brief.microtasks.active_microtask.state_reason}`,
          `- ACTIVE_MICROTASK_CLAUSE: ${brief.microtasks.active_microtask.clause}`,
        ]
      : []),
    ...(brief.microtasks.previous_microtask
      ? [
          `- PREVIOUS_MICROTASK: ${brief.microtasks.previous_microtask.mt_id} | state=${brief.microtasks.previous_microtask.state}`,
          `- PREVIOUS_MICROTASK_CLAUSE: ${brief.microtasks.previous_microtask.clause}`,
        ]
      : []),
    ...(brief.microtasks.suggested_next_microtask
      && brief.microtasks.suggested_next_microtask.mt_id !== brief.microtasks.active_microtask?.mt_id
      ? [
          `- NEXT_MICROTASK: ${brief.microtasks.suggested_next_microtask.mt_id} | state=${brief.microtasks.suggested_next_microtask.state}`,
          `- NEXT_MICROTASK_CLAUSE: ${brief.microtasks.suggested_next_microtask.clause}`,
        ]
      : []),
    ...(brief.notifications.history_hidden
      ? [`- NOTIFICATIONS_HISTORY_HIDDEN: pending=${brief.notifications.hidden_history?.pending_notification_count || 0} | by_kind=${JSON.stringify(brief.notifications.hidden_history?.pending_notification_by_kind || {})}`]
      : []),
    ...(Array.isArray(brief.review_queue) && brief.review_queue.length > 0
      ? [
          `- REVIEW_QUEUE: ${brief.review_queue.length} item(s) targeted to this role`,
          ...brief.review_queue.flatMap((item, index) => {
            const lines = [
              `  ${index + 1}. ${item.receipt_kind} | from=${item.opened_by_role}:${item.opened_by_session} | correlation=${item.correlation_id}`,
              `     summary=${item.summary}`,
            ];
            if (item.spec_anchor !== "<none>" || item.packet_row_ref !== "<none>") {
              lines.push(`     spec=${item.spec_anchor} | packet=${item.packet_row_ref}`);
            }
            if (item.microtask_contract) {
              lines.push(`     microtask.scope_ref=${item.microtask_contract.scope_ref}`);
              if (item.microtask_contract.file_targets.length > 0) {
                lines.push(`     microtask.files=${item.microtask_contract.file_targets.join(", ")}`);
              }
              if (item.microtask_contract.proof_commands.length > 0) {
                lines.push(`     microtask.proof=${item.microtask_contract.proof_commands.join(" ; ")}`);
              }
              if (item.microtask_contract.risk_focus !== "<none>") {
                lines.push(`     microtask.risk=${item.microtask_contract.risk_focus}`);
              }
              if (item.microtask_contract.expected_receipt_kind !== "<none>") {
                lines.push(`     microtask.expected_receipt=${item.microtask_contract.expected_receipt_kind}`);
              }
              if (item.microtask_contract.review_mode !== "<none>") {
                lines.push(`     microtask.review_mode=${item.microtask_contract.review_mode}`);
              }
              if (item.microtask_contract.phase_gate !== "<none>") {
                lines.push(`     microtask.phase_gate=${item.microtask_contract.phase_gate}`);
              }
              if (item.microtask_contract.review_outcome !== "<none>") {
                lines.push(`     microtask.review_outcome=${item.microtask_contract.review_outcome}`);
              }
            }
            return lines;
          }),
        ]
      : []),
    `- RELAY: status=${brief.relay.status} | severity=${brief.relay.severity} | reason=${brief.relay.reason_code}`,
    `- RELAY_SUMMARY: ${brief.relay.summary}`,
    ...(brief.relay.recommended_command ? [`- RELAY_COMMAND: ${brief.relay.recommended_command}`] : []),
    ...(brief.relay.policy
      ? [
          `- RELAY_POLICY: failure_class=${brief.relay.policy.failure_class} | state=${brief.relay.policy.policy_state} | next_strategy=${brief.relay.policy.next_strategy} | budget=${relayPolicyBudget}`,
          `- RELAY_POLICY_META: source=${brief.relay.policy.source_surface} | reason=${brief.relay.policy.reason_code} | updated_at=${brief.relay.policy.updated_at}`,
          `- RELAY_POLICY_SUMMARY: ${brief.relay.policy.summary}`,
        ]
      : []),
    `- MINIMAL_LIVE_READ_SET: ${brief.minimal_live_read_set.join(" | ")}`,
    `- NEXT_COMMANDS: ${brief.next_commands.join(" -> ")}`,
    `- FULL_OUTPUT_RULE: use --json for machine-readable detail instead of rereading packet/runtime/session surfaces separately`,
    "",
  ].join("\n");
}

export function runActiveLaneBriefCli(argv = process.argv.slice(2)) {
  const role = String(argv[0] || "").trim().toUpperCase();
  const wpId = String(argv[1] || "").trim();
  const json = argv.slice(2).includes("--json");

  if (!role || !wpId || !/^WP-/.test(wpId)) {
    console.error("Usage: node .GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> WP-{ID} [--json]");
    process.exit(1);
  }

  const brief = buildActiveLaneBrief({
    repoRoot: REPO_ROOT,
    role,
    wpId,
  });

  if (json) {
    process.stdout.write(`${JSON.stringify(brief, null, 2)}\n`);
  } else {
    process.stdout.write(formatActiveLaneBrief(brief));
  }
}

function sameCliPath(left, right) {
  const leftPath = path.resolve(left);
  const rightPath = path.resolve(right);
  if (leftPath === rightPath) return true;
  try {
    return fs.realpathSync.native(leftPath) === fs.realpathSync.native(rightPath);
  } catch {
    return false;
  }
}

const isMain = process.argv[1] && sameCliPath(process.argv[1], fileURLToPath(import.meta.url));
if (isMain) {
  runActiveLaneBriefCli();
}
