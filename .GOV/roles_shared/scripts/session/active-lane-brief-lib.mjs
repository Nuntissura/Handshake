import fs from "node:fs";
import path from "node:path";
import { REPO_ROOT, repoPathAbs, workPacketPath } from "../lib/runtime-paths.mjs";
import { parseJsonFile, parseJsonlFile } from "../lib/wp-communications-lib.mjs";
import { evaluateWpCommunicationHealth } from "../lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../lib/wp-relay-escalation-lib.mjs";
import { checkAllNotifications, checkNotifications } from "../wp/wp-check-notifications.mjs";
import { evaluateSessionGovernanceState } from "./session-governance-state-lib.mjs";
import { loadSessionRegistry } from "./session-registry-lib.mjs";
import { buildRoleAuthorityString, resolveRoleConfig } from "./session-control-lib.mjs";
import { sessionKey } from "./session-policy.mjs";

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
  };
}

function normalizeSession(value) {
  const text = String(value || "").trim();
  return text || null;
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
  const runtimeStatus = runtimeStatusFile && fs.existsSync(repoPathAbs(runtimeStatusFile))
    ? parseJsonFile(runtimeStatusFile)
    : {};
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
    || isTerminalPacketStatus(runtimeStatus.current_packet_status),
  );
  const visibleNotifications = terminalNoiseSuppressed
    ? { pendingCount: 0, byKind: {} }
    : notifications;
  const hiddenHistory = terminalNoiseSuppressed
    ? {
        pending_notification_count: notifications.pendingCount || 0,
        pending_notification_by_kind: notifications.byKind || {},
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
  const preferredSession = preferredRoleSession(runtimeStatus, normalizedRole);
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
      status: normalize(runtimeStatus.runtime_status),
      phase: normalize(runtimeStatus.current_phase),
      next_expected_actor: normalize(runtimeStatus.next_expected_actor),
      next_expected_session: normalize(runtimeStatus.next_expected_session),
      waiting_on: normalize(runtimeStatus.waiting_on),
      waiting_on_session: normalize(runtimeStatus.waiting_on_session),
      last_event: normalize(runtimeStatus.last_event),
      open_review_items: Array.isArray(runtimeStatus.open_review_items) ? runtimeStatus.open_review_items.length : 0,
    },
    session: {
      session_key: normalize(session?.session_key),
      actor_session: normalize(preferredSession),
      runtime_state: normalize(session?.runtime_state),
      thread_id: normalize(session?.session_thread_id),
      last_command_kind: normalize(session?.last_command_kind),
      last_command_status: normalize(session?.last_command_status),
    },
    notifications: {
      pending_count: visibleNotifications.pendingCount || 0,
      by_kind: visibleNotifications.byKind || {},
      history_hidden: terminalNoiseSuppressed,
      hidden_history: hiddenHistory,
    },
    review_queue: reviewQueue,
    relay: {
      status: relayView.status,
      severity: relayView.severity,
      summary: relayView.summary,
      reason_code: relayView.reason_code,
      recommended_command: relayView.recommended_command,
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
  return [
    "ACTIVE_LANE_BRIEF [CX-LANE-001]",
    `- ROLE: ${brief.role} | WP_ID: ${brief.wp_id}`,
    `- AUTHORITY: ${brief.authority}`,
    `- PACKET: ${brief.packet_path}`,
    `- ROLE_CONTEXT: branch=${brief.role_config.branch} | worktree=${brief.role_config.worktree_dir}`,
    `- RUNTIME: status=${brief.runtime.status} | phase=${brief.runtime.phase} | next=${brief.runtime.next_expected_actor}${brief.runtime.next_expected_session !== "<none>" ? `:${brief.runtime.next_expected_session}` : ""} | waiting_on=${brief.runtime.waiting_on}${brief.runtime.waiting_on_session !== "<none>" ? ` (${brief.runtime.waiting_on_session})` : ""}`,
    `- SESSION: key=${brief.session.session_key} | actor_session=${brief.session.actor_session} | runtime_state=${brief.session.runtime_state} | thread=${brief.session.thread_id} | last_command=${brief.session.last_command_kind}/${brief.session.last_command_status}`,
    `- NOTIFICATIONS: pending=${brief.notifications.pending_count} | by_kind=${JSON.stringify(brief.notifications.by_kind)}`,
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
            }
            return lines;
          }),
        ]
      : []),
    `- RELAY: status=${brief.relay.status} | severity=${brief.relay.severity} | reason=${brief.relay.reason_code}`,
    `- RELAY_SUMMARY: ${brief.relay.summary}`,
    ...(brief.relay.recommended_command ? [`- RELAY_COMMAND: ${brief.relay.recommended_command}`] : []),
    `- MINIMAL_LIVE_READ_SET: ${brief.minimal_live_read_set.join(" | ")}`,
    `- NEXT_COMMANDS: ${brief.next_commands.join(" -> ")}`,
    `- FULL_OUTPUT_RULE: use --json for machine-readable detail instead of rereading packet/runtime/session surfaces separately`,
    "",
  ].join("\n");
}
