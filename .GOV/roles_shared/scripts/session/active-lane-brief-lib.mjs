import fs from "node:fs";
import { workPacketPath } from "../lib/runtime-paths.mjs";
import { parseJsonFile, parseJsonlFile } from "../lib/wp-communications-lib.mjs";
import { evaluateWpCommunicationHealth } from "../lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../lib/wp-relay-escalation-lib.mjs";
import { checkAllNotifications, checkNotifications } from "../wp/wp-check-notifications.mjs";
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

export function buildActiveLaneBrief({
  repoRoot = process.cwd(),
  role = "",
  wpId = "",
} = {}) {
  const normalizedRole = String(role || "").trim().toUpperCase();
  const roleConfig = resolveRoleConfig(normalizedRole, wpId);
  if (!roleConfig) {
    throw new Error(`Unsupported role for active-lane brief: ${role}`);
  }

  const packetPath = workPacketPath(wpId);
  if (!fs.existsSync(packetPath)) {
    throw new Error(`Task packet not found: ${packetPath}`);
  }
  const packetText = fs.readFileSync(packetPath, "utf8");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const runtimeStatus = runtimeStatusFile && fs.existsSync(runtimeStatusFile)
    ? parseJsonFile(runtimeStatusFile)
    : {};
  const receipts = receiptsFile && fs.existsSync(receiptsFile)
    ? parseJsonlFile(receiptsFile)
    : [];
  const { registry } = loadSessionRegistry(repoRoot);
  const session = (registry.sessions || []).find((entry) => entry.session_key === sessionKey(normalizedRole, wpId)) || null;
  const notifications = checkNotifications({ wpId, role: normalizedRole });
  const pendingNotifications = Object.values(checkAllNotifications({ wpId })).flatMap((entry) => entry.notifications || []);
  const communicationEvaluation = evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath,
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
      runtime_state: normalize(session?.runtime_state),
      thread_id: normalize(session?.session_thread_id),
      last_command_kind: normalize(session?.last_command_kind),
      last_command_status: normalize(session?.last_command_status),
    },
    notifications: {
      pending_count: notifications.pendingCount || 0,
      by_kind: notifications.byKind || {},
    },
    relay: {
      status: relay.status,
      severity: relay.severity,
      summary: relay.summary,
      reason_code: relay.reason_code,
      recommended_command: relay.recommended_command,
    },
    minimal_live_read_set: [
      "startup output",
      "active packet",
      "active WP thread/notifications",
      COMMAND_SURFACE_PATH,
    ],
    next_commands: [
      roleConfig.nextCommand,
      `just check-notifications ${wpId} ${normalizedRole}`,
      `just ack-notifications ${wpId} ${normalizedRole} <your-session>`,
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
    `- SESSION: key=${brief.session.session_key} | runtime_state=${brief.session.runtime_state} | thread=${brief.session.thread_id} | last_command=${brief.session.last_command_kind}/${brief.session.last_command_status}`,
    `- NOTIFICATIONS: pending=${brief.notifications.pending_count} | by_kind=${JSON.stringify(brief.notifications.by_kind)}`,
    `- RELAY: status=${brief.relay.status} | severity=${brief.relay.severity} | reason=${brief.relay.reason_code}`,
    `- RELAY_SUMMARY: ${brief.relay.summary}`,
    ...(brief.relay.recommended_command ? [`- RELAY_COMMAND: ${brief.relay.recommended_command}`] : []),
    `- MINIMAL_LIVE_READ_SET: ${brief.minimal_live_read_set.join(" | ")}`,
    `- NEXT_COMMANDS: ${brief.next_commands.join(" -> ")}`,
    `- FULL_OUTPUT_RULE: use --json for machine-readable detail instead of rereading packet/runtime/session surfaces separately`,
    "",
  ].join("\n");
}
