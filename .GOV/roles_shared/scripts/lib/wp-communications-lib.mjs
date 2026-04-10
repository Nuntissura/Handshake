import fs from "node:fs";
import path from "node:path";
import { EXECUTION_OWNER_VALUES } from "../session/session-policy.mjs";
import { MAIN_CONTAINMENT_STATUS_VALUES } from "./merge-progression-truth-lib.mjs";
import { RUNTIME_MILESTONE_VALUES, TASK_BOARD_STATUS_VALUES } from "./wp-authority-projection-lib.mjs";
import {
  GOV_ROOT_REPO_REL,
  LEGACY_TASK_PACKETS_DIRNAME,
  LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT,
  repoPathAbs,
  resolveWorkPacketPath,
  SHARED_GOV_WP_COMMUNICATIONS_ROOT,
  WORK_PACKETS_LOGICAL_DIRNAME,
} from "./runtime-paths.mjs";

export const COMM_ROOT = SHARED_GOV_WP_COMMUNICATIONS_ROOT;
export const LEGACY_COMM_ROOT = LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT;
export const THREAD_FILE_NAME = "THREAD.md";
export const RUNTIME_STATUS_FILE_NAME = "RUNTIME_STATUS.json";
export const RECEIPTS_FILE_NAME = "RECEIPTS.jsonl";
export const NOTIFICATIONS_FILE_NAME = "NOTIFICATIONS.jsonl";
export const NOTIFICATION_CURSOR_FILE_NAME = "NOTIFICATION_CURSOR.json";
export const WP_COMMUNICATION_TRANSACTION_LOCK_SUFFIX = ".tx.lock";
export const RUNTIME_STATUS_SCHEMA_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`;
export const RECEIPT_SCHEMA_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/schemas/WP_RECEIPT.schema.json`;
export const DIRECT_REVIEW_CONTRACT_VERSION = "DIRECT_REVIEW_V1";
export const DIRECT_REVIEW_HEALTH_GATE = "HANDOFF_VERDICT_BLOCKING";
export const DIRECT_REVIEW_PACKET_FORMAT_VERSION = "2026-03-21";
export const FINAL_AUTHORITY_DIRECT_REVIEW_PACKET_FORMAT_VERSION = "2026-03-22";
export const WORKFLOW_INVALIDITY_RECEIPT_KIND = "WORKFLOW_INVALIDITY";
export const OPERATOR_RULE_RESTATEMENT_INVALIDITY_CODE = "OPERATOR_RULE_RESTATEMENT";

export const WORKFLOW_LANE_VALUES = ["MANUAL_RELAY", "ORCHESTRATOR_MANAGED"];
export { EXECUTION_OWNER_VALUES };
export const AGENTIC_MODE_VALUES = ["YES", "NO"];
export const PACKET_STATUS_VALUES = [
  "Ready for Dev",
  "In Progress",
  "Blocked",
  "Done",
  "Validated (PASS)",
  "Validated (FAIL)",
  "Validated (OUTDATED_ONLY)",
  "Validated (ABANDONED)",
];
export const RUNTIME_STATUS_VALUES = [
  "submitted",
  "working",
  "input_required",
  "completed",
  "failed",
  "canceled",
];
export const NEXT_ACTOR_VALUES = [
  "OPERATOR",
  "ORCHESTRATOR",
  "CODER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "VALIDATOR",
  "NONE",
];
export const VALIDATOR_TRIGGER_VALUES = [
  "NONE",
  "READY_FOR_VALIDATION",
  "VALIDATOR_QUERY",
  "POST_WORK_PASS",
  "BLOCKED_NEEDS_VALIDATOR",
  "MICROTASK_REVIEW_READY",
  "STALE_HEARTBEAT",
  "HANDOFF_READY",
];
export const ACTIVE_ROLE_VALUES = ["OPERATOR", "ORCHESTRATOR", "CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR"];
export const ACTIVE_SESSION_STATE_VALUES = ["idle", "working", "waiting", "blocked", "completed"];
export const AUTHORITY_KIND_VALUES = [
  "SYSTEM",
  "OPERATOR",
  "WORKFLOW_AUTHORITY",
  "PRIMARY_CODER",
  "MEMORY_MANAGER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "SECONDARY_VALIDATOR",
];
export const VALIDATOR_ROLE_KIND_VALUES = ["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "SECONDARY_VALIDATOR"];
export const RECEIPT_ROLE_VALUES = [
  "SYSTEM",
  "OPERATOR",
  "ORCHESTRATOR",
  "CODER",
  "MEMORY_MANAGER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "VALIDATOR",
];
export const ROUTABLE_ROLE_VALUES = ["OPERATOR", "ORCHESTRATOR", "CODER", "MEMORY_MANAGER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR"];
export const RECEIPT_KIND_VALUES = [
  "ASSIGNMENT",
  "STATUS",
  "HEARTBEAT",
  "HANDOFF",
  "THREAD_MESSAGE",
  "VALIDATOR_KICKOFF",
  "CODER_INTENT",
  "CODER_HANDOFF",
  "VALIDATOR_REVIEW",
  "VALIDATOR_QUERY",
  "VALIDATOR_RESPONSE",
  "REVIEW_REQUEST",
  "REVIEW_RESPONSE",
  "SPEC_GAP",
  "SPEC_CONFIRMATION",
  "VALIDATION_START",
  "VALIDATION_STATUS_SYNC",
  "STEERING",
  "REPAIR",
  WORKFLOW_INVALIDITY_RECEIPT_KIND,
  "MEMORY_PROPOSAL",
  "MEMORY_FLAG",
  "MEMORY_RGF_CANDIDATE",
];
export const REVIEW_OPEN_RECEIPT_KIND_VALUES = [
  "VALIDATOR_KICKOFF",
  "CODER_HANDOFF",
  "VALIDATOR_QUERY",
  "REVIEW_REQUEST",
  "SPEC_GAP",
];
export const REVIEW_RESOLUTION_RECEIPT_KIND_VALUES = [
  "CODER_INTENT",
  "VALIDATOR_REVIEW",
  "VALIDATOR_RESPONSE",
  "REVIEW_RESPONSE",
  "SPEC_CONFIRMATION",
];
export const REVIEW_TRACKED_RECEIPT_KIND_VALUES = [...REVIEW_OPEN_RECEIPT_KIND_VALUES, ...REVIEW_RESOLUTION_RECEIPT_KIND_VALUES];
export const DIRECT_REVIEW_SESSION_ROLE_VALUES = ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"];
export const MICROTASK_REVIEW_MODE_VALUES = ["BLOCKING", "OVERLAP"];
export const MICROTASK_PHASE_GATE_VALUES = ["BOOTSTRAP", "SKELETON", "MICROTASK", "FINAL_REVIEW"];
export const MICROTASK_REVIEW_OUTCOME_VALUES = ["UNKNOWN", "REPAIR_REQUIRED", "APPROVED_FOR_FINAL_REVIEW"];

const RFC3339_UTC_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/;
const SHA_RE = /^[0-9a-f]{7,40}$/i;
const WORKFLOW_INVALIDITY_CODE_RE = /^[A-Z0-9_]+$/;

export function normalize(value) {
  return String(value || "").replace(/\\/g, "/").trim();
}

export function normalizeWorkflowInvalidityCode(value) {
  const raw = String(value || "").trim().toUpperCase();
  return raw || null;
}

export function isPlainObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

export function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

export function isNullableString(value) {
  return value === null || isNonEmptyString(value);
}

export function isNullableBoolean(value) {
  return value === null || typeof value === "boolean";
}

export function isRfc3339Utc(value) {
  return typeof value === "string" && RFC3339_UTC_RE.test(value);
}

export function isNullableRfc3339Utc(value) {
  return value === null || isRfc3339Utc(value);
}

export function isNullableSha(value) {
  return value === null || (typeof value === "string" && SHA_RE.test(value));
}

function validateMicrotaskContract(value, prefix, errors) {
  if (!(value === undefined || value === null || isPlainObject(value))) {
    errors.push(`${prefix} must be null or an object`);
    return;
  }
  if (value === undefined || value === null) return;

  const allowedKeys = new Set([
    "scope_ref",
    "file_targets",
    "proof_commands",
    "risk_focus",
    "expected_receipt_kind",
    "review_mode",
    "phase_gate",
    "review_outcome",
  ]);
  for (const key of Object.keys(value)) {
    if (!allowedKeys.has(key)) errors.push(`${prefix}.${key} is not allowed`);
  }
  if (!(value.scope_ref === undefined || isNullableString(value.scope_ref))) {
    errors.push(`${prefix}.scope_ref must be null or a non-empty string`);
  }
  if (!(value.risk_focus === undefined || isNullableString(value.risk_focus))) {
    errors.push(`${prefix}.risk_focus must be null or a non-empty string`);
  }
  if (!(value.expected_receipt_kind === undefined || value.expected_receipt_kind === null || RECEIPT_KIND_VALUES.includes(value.expected_receipt_kind))) {
    errors.push(`${prefix}.expected_receipt_kind invalid (${value.expected_receipt_kind})`);
  }
  if (!(value.review_mode === undefined || value.review_mode === null || MICROTASK_REVIEW_MODE_VALUES.includes(value.review_mode))) {
    errors.push(`${prefix}.review_mode invalid (${value.review_mode})`);
  }
  if (!(value.phase_gate === undefined || value.phase_gate === null || MICROTASK_PHASE_GATE_VALUES.includes(value.phase_gate))) {
    errors.push(`${prefix}.phase_gate invalid (${value.phase_gate})`);
  }
  if (!(value.review_outcome === undefined || value.review_outcome === null || MICROTASK_REVIEW_OUTCOME_VALUES.includes(value.review_outcome))) {
    errors.push(`${prefix}.review_outcome invalid (${value.review_outcome})`);
  }
  if (!(value.file_targets === undefined || Array.isArray(value.file_targets))) {
    errors.push(`${prefix}.file_targets must be an array when present`);
  } else if (Array.isArray(value.file_targets) && value.file_targets.some((entry) => !isNonEmptyString(entry))) {
    errors.push(`${prefix}.file_targets must contain non-empty strings`);
  }
  if (!(value.proof_commands === undefined || Array.isArray(value.proof_commands))) {
    errors.push(`${prefix}.proof_commands must be an array when present`);
  } else if (Array.isArray(value.proof_commands) && value.proof_commands.some((entry) => !isNonEmptyString(entry))) {
    errors.push(`${prefix}.proof_commands must contain non-empty strings`);
  }

  const hasPayload =
    isNonEmptyString(value.scope_ref)
    || isNonEmptyString(value.risk_focus)
    || isNonEmptyString(value.expected_receipt_kind)
    || isNonEmptyString(value.review_mode)
    || isNonEmptyString(value.phase_gate)
    || isNonEmptyString(value.review_outcome)
    || (Array.isArray(value.file_targets) && value.file_targets.length > 0)
    || (Array.isArray(value.proof_commands) && value.proof_commands.length > 0);
  if (!hasPayload) {
    errors.push(`${prefix} must contain at least one populated field`);
  }
}

export function workflowInvalidityReceipts(receipts = []) {
  return (Array.isArray(receipts) ? receipts : []).filter(
    (entry) => String(entry?.receipt_kind || "").trim().toUpperCase() === WORKFLOW_INVALIDITY_RECEIPT_KIND,
  );
}

export function repairReceipts(receipts = []) {
  return (Array.isArray(receipts) ? receipts : []).filter(
    (entry) => String(entry?.receipt_kind || "").trim().toUpperCase() === "REPAIR",
  );
}

export function latestWorkflowInvalidityReceipt(receipts = []) {
  return [...workflowInvalidityReceipts(receipts)]
    .sort((left, right) => String(left?.timestamp_utc || "").localeCompare(String(right?.timestamp_utc || "")))
    .at(-1) || null;
}

export function activeWorkflowInvalidityReceipt(receipts = []) {
  const ordered = [...(Array.isArray(receipts) ? receipts : [])]
    .sort((left, right) => String(left?.timestamp_utc || "").localeCompare(String(right?.timestamp_utc || "")));
  let active = null;
  for (const entry of ordered) {
    const kind = String(entry?.receipt_kind || "").trim().toUpperCase();
    if (kind === WORKFLOW_INVALIDITY_RECEIPT_KIND) {
      active = entry;
      continue;
    }
    if (kind === "REPAIR") {
      active = null;
    }
  }
  return active;
}

export function ensureSchemaFilesExist() {
  const missing = [RUNTIME_STATUS_SCHEMA_PATH, RECEIPT_SCHEMA_PATH].filter((target) => !fs.existsSync(repoPathAbs(target)));
  if (missing.length > 0) {
    throw new Error(`Missing WP communication schema file(s): ${missing.map(normalize).join(", ")}`);
  }
}

export function addMinutes(iso, minutes) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    throw new Error(`Invalid RFC3339 UTC timestamp: ${iso}`);
  }
  date.setUTCMinutes(date.getUTCMinutes() + Number(minutes || 0));
  return date.toISOString();
}

export function parseJsonFile(filePath) {
  const text = fs.readFileSync(repoPathAbs(filePath), "utf8");
  return JSON.parse(text);
}

export function parseJsonlFile(filePath) {
  const text = fs.readFileSync(repoPathAbs(filePath), "utf8");
  const lines = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  return lines.map((line, index) => {
    try {
      return JSON.parse(line);
    } catch (error) {
      throw new Error(`${normalize(filePath)} line ${index + 1}: invalid JSON (${error.message})`);
    }
  });
}

export function validateRuntimeStatus(data) {
  const errors = [];
  if (!isPlainObject(data)) {
    return ["runtime status must be a JSON object"];
  }

  const requiredKeys = [
    "schema_version",
    "wp_id",
    "base_wp_id",
    "task_packet",
    "communication_dir",
    "thread_file",
    "runtime_status_file",
    "receipts_file",
    "workflow_lane",
    "execution_owner",
    "workflow_authority",
    "technical_advisor",
    "technical_authority",
    "merge_authority",
    "wp_validator_of_record",
    "integration_validator_of_record",
    "secondary_validator_sessions",
    "agentic_mode",
    "current_packet_status",
    "runtime_status",
    "current_phase",
    "next_expected_actor",
    "waiting_on",
    "validator_trigger",
    "validator_trigger_reason",
    "attention_required",
    "ready_for_validation",
    "ready_for_validation_reason",
    "current_branch",
    "current_worktree_dir",
    "current_files_touched",
    "active_role_sessions",
    "last_event",
    "last_event_at",
    "last_heartbeat_at",
    "heartbeat_interval_minutes",
    "heartbeat_due_at",
    "stale_after",
    "max_coder_revision_cycles",
    "max_validator_review_cycles",
    "max_relay_escalation_cycles",
    "current_coder_revision_cycle",
    "current_validator_review_cycle",
    "current_relay_escalation_cycle",
    "last_backup_push_at",
    "last_backup_push_sha",
  ];

  const optionalKeys = [
    "next_expected_session",
    "waiting_on_session",
    "open_review_items",
    "current_task_board_status",
    "current_milestone",
    "last_milestone_sync_at",
    "main_containment_status",
    "merged_main_commit",
    "main_containment_verified_at_utc",
    "current_main_compatibility_status",
    "current_main_compatibility_baseline_sha",
    "current_main_compatibility_verified_at_utc",
    "packet_widening_decision",
    "packet_widening_evidence",
  ];
  const allowedKeys = new Set([...requiredKeys, ...optionalKeys]);
  for (const key of requiredKeys) {
    if (!(key in data)) errors.push(`missing key: ${key}`);
  }
  for (const key of Object.keys(data)) {
    if (!allowedKeys.has(key)) errors.push(`unexpected key: ${key}`);
  }

  if (data.schema_version !== "wp_runtime_status@1") errors.push("schema_version must be wp_runtime_status@1");
  if (!isNonEmptyString(data.wp_id) || !/^WP-/.test(data.wp_id)) errors.push("wp_id must start with WP-");
  if (!isNonEmptyString(data.base_wp_id) || !/^WP-/.test(data.base_wp_id)) errors.push("base_wp_id must start with WP-");
  const taskPacketPrefix = GOV_ROOT_REPO_REL.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const taskPacketFallback = "\\.GOV";
  const normalizedTaskPacket = normalize(data.task_packet);
  const authoritativeTaskPacketAbs = normalize(resolveWorkPacketPath(data.wp_id)?.packetAbsPath || "");
  const resolvedTaskPacketAbs = isNonEmptyString(data.task_packet) ? normalize(repoPathAbs(data.task_packet)) : "";
  const matchesPacketPath = (prefix) =>
    [LEGACY_TASK_PACKETS_DIRNAME, WORK_PACKETS_LOGICAL_DIRNAME].some((dirName) =>
      new RegExp(`^${prefix}/${dirName}/WP-.*\\.md$`).test(normalizedTaskPacket)
      || new RegExp(`^${prefix}/${dirName}/WP-[^/]+/packet\\.md$`).test(normalizedTaskPacket)
    );
  const taskPacketMatchesAuthoritativePath = authoritativeTaskPacketAbs
    ? resolvedTaskPacketAbs === authoritativeTaskPacketAbs
    : false;
  if (!isNonEmptyString(data.task_packet) || !(
    taskPacketMatchesAuthoritativePath
    || matchesPacketPath(taskPacketPrefix)
    || matchesPacketPath(taskPacketFallback)
  )) {
    errors.push(`task_packet must point to ${GOV_ROOT_REPO_REL}/task_packets/WP-*.md, ${GOV_ROOT_REPO_REL}/task_packets/WP-*/packet.md, or the logical ${GOV_ROOT_REPO_REL}/work_packets equivalents`);
  }
  const currentPaths = communicationPathsForWp(data.wp_id);
  const declaredCommDir = normalize(data.communication_dir);
  const declaredThreadFile = normalize(data.thread_file);
  const declaredRuntimeStatusFile = normalize(data.runtime_status_file);
  const declaredReceiptsFile = normalize(data.receipts_file);

  if (!isNonEmptyString(data.communication_dir) || declaredCommDir !== currentPaths.dir) {
    errors.push(`communication_dir must point to ${currentPaths.dir}`);
  }
  if (!isNonEmptyString(data.thread_file) || declaredThreadFile !== currentPaths.threadFile) {
    errors.push("thread_file must point to THREAD.md in the declared WP communication directory");
  }
  if (!isNonEmptyString(data.runtime_status_file) || declaredRuntimeStatusFile !== currentPaths.runtimeStatusFile) {
    errors.push("runtime_status_file must point to RUNTIME_STATUS.json in the declared WP communication directory");
  }
  if (!isNonEmptyString(data.receipts_file) || declaredReceiptsFile !== currentPaths.receiptsFile) {
    errors.push("receipts_file must point to RECEIPTS.jsonl in the declared WP communication directory");
  }
  if (!WORKFLOW_LANE_VALUES.includes(data.workflow_lane)) errors.push(`workflow_lane invalid (${data.workflow_lane})`);
  if (!EXECUTION_OWNER_VALUES.includes(data.execution_owner)) errors.push(`execution_owner invalid (${data.execution_owner})`);
  if (data.workflow_authority !== "ORCHESTRATOR") errors.push(`workflow_authority invalid (${data.workflow_authority})`);
  if (!["WP_VALIDATOR", "NONE"].includes(data.technical_advisor)) errors.push(`technical_advisor invalid (${data.technical_advisor})`);
  if (!["INTEGRATION_VALIDATOR", "NONE"].includes(data.technical_authority)) errors.push(`technical_authority invalid (${data.technical_authority})`);
  if (!["INTEGRATION_VALIDATOR", "OPERATOR", "NONE"].includes(data.merge_authority)) errors.push(`merge_authority invalid (${data.merge_authority})`);
  if (!isNullableString(data.wp_validator_of_record)) errors.push("wp_validator_of_record must be null or a non-empty string");
  if (!isNullableString(data.integration_validator_of_record)) errors.push("integration_validator_of_record must be null or a non-empty string");
  if (!Array.isArray(data.secondary_validator_sessions) || data.secondary_validator_sessions.some((value) => !isNonEmptyString(value))) {
    errors.push("secondary_validator_sessions must be an array of non-empty strings");
  }
  if (!AGENTIC_MODE_VALUES.includes(data.agentic_mode)) errors.push(`agentic_mode invalid (${data.agentic_mode})`);
  if (!PACKET_STATUS_VALUES.includes(data.current_packet_status)) {
    errors.push(`current_packet_status invalid (${data.current_packet_status})`);
  }
  if ("main_containment_status" in data) {
    if (data.main_containment_status !== null && !MAIN_CONTAINMENT_STATUS_VALUES.includes(data.main_containment_status)) {
      errors.push(`main_containment_status invalid (${data.main_containment_status})`);
    }
  }
  if ("merged_main_commit" in data && !isNullableSha(data.merged_main_commit)) {
    errors.push(`merged_main_commit invalid (${data.merged_main_commit})`);
  }
  if ("main_containment_verified_at_utc" in data && !isNullableRfc3339Utc(data.main_containment_verified_at_utc)) {
    errors.push(`main_containment_verified_at_utc invalid (${data.main_containment_verified_at_utc})`);
  }
  if ("current_main_compatibility_status" in data && !isNullableString(data.current_main_compatibility_status)) {
    errors.push(`current_main_compatibility_status invalid (${data.current_main_compatibility_status})`);
  }
  if ("current_main_compatibility_baseline_sha" in data && !isNullableSha(data.current_main_compatibility_baseline_sha)) {
    errors.push(`current_main_compatibility_baseline_sha invalid (${data.current_main_compatibility_baseline_sha})`);
  }
  if ("current_main_compatibility_verified_at_utc" in data && !isNullableRfc3339Utc(data.current_main_compatibility_verified_at_utc)) {
    errors.push(`current_main_compatibility_verified_at_utc invalid (${data.current_main_compatibility_verified_at_utc})`);
  }
  if ("packet_widening_decision" in data && !isNullableString(data.packet_widening_decision)) {
    errors.push(`packet_widening_decision invalid (${data.packet_widening_decision})`);
  }
  if ("packet_widening_evidence" in data && !isNullableString(data.packet_widening_evidence)) {
    errors.push(`packet_widening_evidence invalid (${data.packet_widening_evidence})`);
  }
  if (!RUNTIME_STATUS_VALUES.includes(data.runtime_status)) errors.push(`runtime_status invalid (${data.runtime_status})`);
  if (!isNonEmptyString(data.current_phase) || !/^[A-Z][A-Z0-9_]*$/.test(data.current_phase)) {
    errors.push(`current_phase invalid (${data.current_phase})`);
  }
  if ("current_task_board_status" in data) {
    if (!(data.current_task_board_status === null || TASK_BOARD_STATUS_VALUES.includes(data.current_task_board_status))) {
      errors.push(`current_task_board_status invalid (${data.current_task_board_status})`);
    }
  }
  if ("current_milestone" in data) {
    if (!(data.current_milestone === null || RUNTIME_MILESTONE_VALUES.includes(data.current_milestone))) {
      errors.push(`current_milestone invalid (${data.current_milestone})`);
    }
  }
  if ("last_milestone_sync_at" in data && !isNullableRfc3339Utc(data.last_milestone_sync_at)) {
    errors.push(`last_milestone_sync_at invalid (${data.last_milestone_sync_at})`);
  }
  if (!NEXT_ACTOR_VALUES.includes(data.next_expected_actor)) {
    errors.push(`next_expected_actor invalid (${data.next_expected_actor})`);
  }
  if (!(data.next_expected_session === undefined || isNullableString(data.next_expected_session))) {
    errors.push("next_expected_session must be null or a non-empty string");
  }
  if (!isNonEmptyString(data.waiting_on)) errors.push("waiting_on must be a non-empty string");
  if (!(data.waiting_on_session === undefined || isNullableString(data.waiting_on_session))) {
    errors.push("waiting_on_session must be null or a non-empty string");
  }
  if (!VALIDATOR_TRIGGER_VALUES.includes(data.validator_trigger)) {
    errors.push(`validator_trigger invalid (${data.validator_trigger})`);
  }
  if (!isNullableString(data.validator_trigger_reason)) errors.push("validator_trigger_reason must be null or a non-empty string");
  if (typeof data.attention_required !== "boolean") errors.push("attention_required must be boolean");
  if (typeof data.ready_for_validation !== "boolean") errors.push("ready_for_validation must be boolean");
  if (!isNullableString(data.ready_for_validation_reason)) errors.push("ready_for_validation_reason must be null or a non-empty string");
  if (!isNonEmptyString(data.current_branch)) errors.push("current_branch must be a non-empty string");
  if (!isNonEmptyString(data.current_worktree_dir)) errors.push("current_worktree_dir must be a non-empty string");
  if (!Array.isArray(data.current_files_touched) || data.current_files_touched.some((value) => !isNonEmptyString(value))) {
    errors.push("current_files_touched must be an array of non-empty strings");
  }
  if (!Array.isArray(data.active_role_sessions)) {
    errors.push("active_role_sessions must be an array");
  } else {
    data.active_role_sessions.forEach((entry, index) => {
      if (!isPlainObject(entry)) {
        errors.push(`active_role_sessions[${index}] must be an object`);
        return;
      }
      const required = ["role", "session_id", "authority_kind", "validator_role_kind", "worktree_dir", "state", "last_heartbeat_at"];
      for (const key of required) {
        if (!(key in entry)) errors.push(`active_role_sessions[${index}] missing key: ${key}`);
      }
      const allowed = new Set(required);
      for (const key of Object.keys(entry)) {
        if (!allowed.has(key)) errors.push(`active_role_sessions[${index}] unexpected key: ${key}`);
      }
      if (!ACTIVE_ROLE_VALUES.includes(entry.role)) errors.push(`active_role_sessions[${index}].role invalid (${entry.role})`);
      if (!isNonEmptyString(entry.session_id)) errors.push(`active_role_sessions[${index}].session_id must be a non-empty string`);
      if (!AUTHORITY_KIND_VALUES.includes(entry.authority_kind)) errors.push(`active_role_sessions[${index}].authority_kind invalid (${entry.authority_kind})`);
      if (!(entry.validator_role_kind === null || VALIDATOR_ROLE_KIND_VALUES.includes(entry.validator_role_kind))) {
        errors.push(`active_role_sessions[${index}].validator_role_kind invalid (${entry.validator_role_kind})`);
      } else if (entry.role === "WP_VALIDATOR" && entry.validator_role_kind !== "WP_VALIDATOR") {
        errors.push(`active_role_sessions[${index}].validator_role_kind must be WP_VALIDATOR when role is WP_VALIDATOR`);
      } else if (entry.role === "INTEGRATION_VALIDATOR" && entry.validator_role_kind !== "INTEGRATION_VALIDATOR") {
        errors.push(`active_role_sessions[${index}].validator_role_kind must be INTEGRATION_VALIDATOR when role is INTEGRATION_VALIDATOR`);
      }
      if (!isNonEmptyString(entry.worktree_dir)) errors.push(`active_role_sessions[${index}].worktree_dir must be a non-empty string`);
      if (!ACTIVE_SESSION_STATE_VALUES.includes(entry.state)) errors.push(`active_role_sessions[${index}].state invalid (${entry.state})`);
      if (!isRfc3339Utc(entry.last_heartbeat_at)) errors.push(`active_role_sessions[${index}].last_heartbeat_at must be RFC3339 UTC`);
    });
  }
  if (!(data.open_review_items === undefined || Array.isArray(data.open_review_items))) {
    errors.push("open_review_items must be an array when present");
  } else if (Array.isArray(data.open_review_items)) {
    data.open_review_items.forEach((entry, index) => {
      if (!isPlainObject(entry)) {
        errors.push(`open_review_items[${index}] must be an object`);
        return;
      }
      const required = [
        "correlation_id",
        "receipt_kind",
        "summary",
        "opened_by_role",
        "opened_by_session",
        "target_role",
        "target_session",
        "spec_anchor",
        "packet_row_ref",
        "requires_ack",
        "opened_at",
        "updated_at",
      ];
      const allowed = new Set([...required, "microtask_contract"]);
      for (const key of required) {
        if (!(key in entry)) errors.push(`open_review_items[${index}] missing key: ${key}`);
      }
      for (const key of Object.keys(entry)) {
        if (!allowed.has(key)) errors.push(`open_review_items[${index}] unexpected key: ${key}`);
      }
      if (!isNonEmptyString(entry.correlation_id)) errors.push(`open_review_items[${index}].correlation_id must be a non-empty string`);
      if (!REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) {
        errors.push(`open_review_items[${index}].receipt_kind invalid (${entry.receipt_kind})`);
      }
      if (!isNonEmptyString(entry.summary)) errors.push(`open_review_items[${index}].summary must be a non-empty string`);
      if (!RECEIPT_ROLE_VALUES.includes(entry.opened_by_role)) {
        errors.push(`open_review_items[${index}].opened_by_role invalid (${entry.opened_by_role})`);
      }
      if (!isNonEmptyString(entry.opened_by_session)) errors.push(`open_review_items[${index}].opened_by_session must be a non-empty string`);
      if (!ROUTABLE_ROLE_VALUES.includes(entry.target_role)) {
        errors.push(`open_review_items[${index}].target_role invalid (${entry.target_role})`);
      }
      if (!isNullableString(entry.target_session)) errors.push(`open_review_items[${index}].target_session must be null or a non-empty string`);
      if (!isNullableString(entry.spec_anchor)) errors.push(`open_review_items[${index}].spec_anchor must be null or a non-empty string`);
      if (!isNullableString(entry.packet_row_ref)) errors.push(`open_review_items[${index}].packet_row_ref must be null or a non-empty string`);
      validateMicrotaskContract(entry.microtask_contract, `open_review_items[${index}].microtask_contract`, errors);
      if (typeof entry.requires_ack !== "boolean") errors.push(`open_review_items[${index}].requires_ack must be boolean`);
      if (!isRfc3339Utc(entry.opened_at)) errors.push(`open_review_items[${index}].opened_at must be RFC3339 UTC`);
      if (!isRfc3339Utc(entry.updated_at)) errors.push(`open_review_items[${index}].updated_at must be RFC3339 UTC`);
    });
  }
  if (!isNonEmptyString(data.last_event)) errors.push("last_event must be a non-empty string");
  if (!isRfc3339Utc(data.last_event_at)) errors.push("last_event_at must be RFC3339 UTC");
  if (!isRfc3339Utc(data.last_heartbeat_at)) errors.push("last_heartbeat_at must be RFC3339 UTC");
  if (!Number.isInteger(data.heartbeat_interval_minutes) || data.heartbeat_interval_minutes < 1) {
    errors.push("heartbeat_interval_minutes must be an integer >= 1");
  }
  if (!isRfc3339Utc(data.heartbeat_due_at)) errors.push("heartbeat_due_at must be RFC3339 UTC");
  if (!isRfc3339Utc(data.stale_after)) errors.push("stale_after must be RFC3339 UTC");
  if (!Number.isInteger(data.max_coder_revision_cycles) || data.max_coder_revision_cycles < 1) {
    errors.push("max_coder_revision_cycles must be an integer >= 1");
  }
  if (!Number.isInteger(data.max_validator_review_cycles) || data.max_validator_review_cycles < 1) {
    errors.push("max_validator_review_cycles must be an integer >= 1");
  }
  if (!Number.isInteger(data.max_relay_escalation_cycles) || data.max_relay_escalation_cycles < 1) {
    errors.push("max_relay_escalation_cycles must be an integer >= 1");
  }
  if (!Number.isInteger(data.current_coder_revision_cycle) || data.current_coder_revision_cycle < 0) {
    errors.push("current_coder_revision_cycle must be an integer >= 0");
  } else if (Number.isInteger(data.max_coder_revision_cycles) && data.current_coder_revision_cycle > data.max_coder_revision_cycles) {
    errors.push("current_coder_revision_cycle exceeds max_coder_revision_cycles");
  }
  if (!Number.isInteger(data.current_validator_review_cycle) || data.current_validator_review_cycle < 0) {
    errors.push("current_validator_review_cycle must be an integer >= 0");
  } else if (
    Number.isInteger(data.max_validator_review_cycles) &&
    data.current_validator_review_cycle > data.max_validator_review_cycles
  ) {
    errors.push("current_validator_review_cycle exceeds max_validator_review_cycles");
  }
  if (!Number.isInteger(data.current_relay_escalation_cycle) || data.current_relay_escalation_cycle < 0) {
    errors.push("current_relay_escalation_cycle must be an integer >= 0");
  } else if (
    Number.isInteger(data.max_relay_escalation_cycles) &&
    data.current_relay_escalation_cycle > data.max_relay_escalation_cycles
  ) {
    errors.push("current_relay_escalation_cycle exceeds max_relay_escalation_cycles");
  }
  if (!isNullableRfc3339Utc(data.last_backup_push_at)) errors.push("last_backup_push_at must be null or RFC3339 UTC");
  if (!isNullableSha(data.last_backup_push_sha)) errors.push("last_backup_push_sha must be null or a commit SHA");

  return errors;
}

export function validateReceipt(entry) {
  const errors = [];
  if (!isPlainObject(entry)) {
    return ["receipt entry must be a JSON object"];
  }

  const requiredKeys = [
    "schema_version",
    "timestamp_utc",
    "wp_id",
    "actor_role",
    "actor_session",
    "actor_authority_kind",
    "validator_role_kind",
    "receipt_kind",
    "summary",
    "branch",
    "worktree_dir",
    "state_before",
    "state_after",
    "refs",
  ];
  const optionalKeys = ["target_role", "target_session", "correlation_id", "requires_ack", "ack_for", "spec_anchor", "packet_row_ref", "microtask_contract", "workflow_invalidity_code"];
  const allowedKeys = new Set([...requiredKeys, ...optionalKeys]);
  for (const key of requiredKeys) {
    if (!(key in entry)) errors.push(`missing key: ${key}`);
  }
  for (const key of Object.keys(entry)) {
    if (!allowedKeys.has(key)) errors.push(`unexpected key: ${key}`);
  }

  if (entry.schema_version !== "wp_receipt@1") errors.push("schema_version must be wp_receipt@1");
  if (!isRfc3339Utc(entry.timestamp_utc)) errors.push("timestamp_utc must be RFC3339 UTC");
  if (!isNonEmptyString(entry.wp_id) || !/^WP-/.test(entry.wp_id)) errors.push("wp_id must start with WP-");
  if (!RECEIPT_ROLE_VALUES.includes(entry.actor_role)) errors.push(`actor_role invalid (${entry.actor_role})`);
  if (!isNonEmptyString(entry.actor_session)) errors.push("actor_session must be a non-empty string");
  if (!AUTHORITY_KIND_VALUES.includes(entry.actor_authority_kind)) errors.push(`actor_authority_kind invalid (${entry.actor_authority_kind})`);
  if (!(entry.validator_role_kind === null || VALIDATOR_ROLE_KIND_VALUES.includes(entry.validator_role_kind))) {
    errors.push(`validator_role_kind invalid (${entry.validator_role_kind})`);
  } else if (entry.actor_role === "WP_VALIDATOR" && entry.validator_role_kind !== "WP_VALIDATOR") {
    errors.push("validator_role_kind must be WP_VALIDATOR when actor_role is WP_VALIDATOR");
  } else if (entry.actor_role === "INTEGRATION_VALIDATOR" && entry.validator_role_kind !== "INTEGRATION_VALIDATOR") {
    errors.push("validator_role_kind must be INTEGRATION_VALIDATOR when actor_role is INTEGRATION_VALIDATOR");
  }
  if (!RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) errors.push(`receipt_kind invalid (${entry.receipt_kind})`);
  if (!isNonEmptyString(entry.summary)) errors.push("summary must be a non-empty string");
  if (!isNullableString(entry.branch)) errors.push("branch must be null or a non-empty string");
  if (!isNullableString(entry.worktree_dir)) errors.push("worktree_dir must be null or a non-empty string");
  if (!isNullableString(entry.state_before)) errors.push("state_before must be null or a non-empty string");
  if (!isNullableString(entry.state_after)) errors.push("state_after must be null or a non-empty string");
  if (!(entry.target_role === undefined || entry.target_role === null || ROUTABLE_ROLE_VALUES.includes(entry.target_role))) {
    errors.push(`target_role invalid (${entry.target_role})`);
  }
  if (!(entry.target_session === undefined || isNullableString(entry.target_session))) {
    errors.push("target_session must be null or a non-empty string");
  }
  if (!(entry.correlation_id === undefined || isNullableString(entry.correlation_id))) {
    errors.push("correlation_id must be null or a non-empty string");
  }
  if (!(entry.requires_ack === undefined || typeof entry.requires_ack === "boolean")) {
    errors.push("requires_ack must be boolean");
  }
  if (!(entry.ack_for === undefined || isNullableString(entry.ack_for))) {
    errors.push("ack_for must be null or a non-empty string");
  }
  if (!(entry.spec_anchor === undefined || isNullableString(entry.spec_anchor))) {
    errors.push("spec_anchor must be null or a non-empty string");
  }
  if (!(entry.packet_row_ref === undefined || isNullableString(entry.packet_row_ref))) {
    errors.push("packet_row_ref must be null or a non-empty string");
  }
  validateMicrotaskContract(entry.microtask_contract, "microtask_contract", errors);
  if (!(entry.workflow_invalidity_code === undefined || isNullableString(entry.workflow_invalidity_code))) {
    errors.push("workflow_invalidity_code must be null or a non-empty string");
  }
  const workflowInvalidityCode = normalizeWorkflowInvalidityCode(entry.workflow_invalidity_code);
  if (entry.receipt_kind === WORKFLOW_INVALIDITY_RECEIPT_KIND) {
    if (!workflowInvalidityCode) {
      errors.push(`workflow_invalidity_code is required for ${WORKFLOW_INVALIDITY_RECEIPT_KIND}`);
    } else if (!WORKFLOW_INVALIDITY_CODE_RE.test(workflowInvalidityCode)) {
      errors.push(`workflow_invalidity_code invalid (${entry.workflow_invalidity_code})`);
    }
  } else if (workflowInvalidityCode) {
    errors.push("workflow_invalidity_code is only allowed for WORKFLOW_INVALIDITY receipts");
  }
  if (REVIEW_TRACKED_RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) {
    if (!isNonEmptyString(entry.correlation_id)) {
      errors.push(`correlation_id is required for ${entry.receipt_kind}`);
    }
    if (!(typeof entry.target_role === "string" && ROUTABLE_ROLE_VALUES.includes(entry.target_role))) {
      errors.push(`target_role is required for ${entry.receipt_kind}`);
    }
    if (
      DIRECT_REVIEW_SESSION_ROLE_VALUES.includes(String(entry.actor_role || "").trim().toUpperCase())
      && DIRECT_REVIEW_SESSION_ROLE_VALUES.includes(String(entry.target_role || "").trim().toUpperCase())
      && !isNonEmptyString(entry.target_session)
    ) {
      errors.push(`target_session is required for ${entry.receipt_kind}`);
    }
  }
  if (REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) {
    if (!isNonEmptyString(entry.ack_for)) {
      errors.push(`ack_for is required for ${entry.receipt_kind}`);
    } else if (isNonEmptyString(entry.correlation_id) && entry.ack_for !== entry.correlation_id) {
      errors.push(`ack_for must match correlation_id for ${entry.receipt_kind}`);
    }
  }
  if (!Array.isArray(entry.refs) || entry.refs.some((value) => !isNonEmptyString(value))) {
    errors.push("refs must be an array of non-empty strings");
  }

  return errors;
}

function communicationPathsForRoot(root, wpId) {
  const dir = normalize(path.join(root, wpId));
  return {
    dir,
    threadFile: normalize(path.join(dir, THREAD_FILE_NAME)),
    runtimeStatusFile: normalize(path.join(dir, RUNTIME_STATUS_FILE_NAME)),
    receiptsFile: normalize(path.join(dir, RECEIPTS_FILE_NAME)),
  };
}

export function communicationPathsForWp(wpId) {
  return communicationPathsForRoot(COMM_ROOT, wpId);
}

export function ensurePacketlessWpCommunicationScaffold(wpId, {
  threadHeading = "",
  noteLines = [],
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId || !/^WP-/.test(normalizedWpId)) {
    throw new Error("WP_ID is required");
  }

  const paths = communicationPathsForWp(normalizedWpId);
  const dirAbsPath = repoPathAbs(paths.dir);
  fs.mkdirSync(dirAbsPath, { recursive: true });

  const threadAbsPath = repoPathAbs(paths.threadFile);
  if (!fs.existsSync(threadAbsPath)) {
    const heading = String(threadHeading || `# WP Thread: ${normalizedWpId}`).trim();
    const bodyLines = [
      heading,
      "",
      `Synthetic packetless communication lane for ${normalizedWpId}.`,
      ...[].concat(noteLines || []).map((line) => String(line || "").trim()).filter(Boolean),
      "",
    ];
    fs.writeFileSync(threadAbsPath, bodyLines.join("\n"), "utf8");
  }

  const receiptsAbsPath = repoPathAbs(paths.receiptsFile);
  if (!fs.existsSync(receiptsAbsPath)) {
    fs.writeFileSync(receiptsAbsPath, "", "utf8");
  }

  const notificationsFile = normalize(path.join(paths.dir, NOTIFICATIONS_FILE_NAME));
  const notificationsAbsPath = repoPathAbs(notificationsFile);
  if (!fs.existsSync(notificationsAbsPath)) {
    fs.writeFileSync(notificationsAbsPath, "", "utf8");
  }

  const cursorFile = normalize(path.join(paths.dir, NOTIFICATION_CURSOR_FILE_NAME));
  const cursorAbsPath = repoPathAbs(cursorFile);
  if (!fs.existsSync(cursorAbsPath)) {
    fs.writeFileSync(cursorAbsPath, `${JSON.stringify({ schema_version: "wp_notification_cursor@1", cursors: {} }, null, 2)}\n`, "utf8");
  }

  return {
    dir: paths.dir,
    threadFile: paths.threadFile,
    receiptsFile: paths.receiptsFile,
    notificationsFile,
    cursorFile,
  };
}

export function communicationTransactionLockPathForWp(wpId) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId || !/^WP-/.test(normalizedWpId)) {
    throw new Error("WP_ID is required");
  }
  return normalize(path.join(COMM_ROOT, `${normalizedWpId}${WP_COMMUNICATION_TRANSACTION_LOCK_SUFFIX}`));
}

export function legacyCommunicationPathsForWp(wpId) {
  return communicationPathsForRoot(LEGACY_COMM_ROOT, wpId);
}

export function allCommunicationRoots() {
  return Array.from(new Set([COMM_ROOT, LEGACY_COMM_ROOT].map(normalize)));
}

export function deriveAuthorityKinds({ actorRole, actorSession, runtimeStatus }) {
  const role = String(actorRole || "").trim().toUpperCase();
  const session = String(actorSession || "").trim();
  if (role === "SYSTEM") return { authorityKind: "SYSTEM", validatorRoleKind: null };
  if (role === "OPERATOR") return { authorityKind: "OPERATOR", validatorRoleKind: null };
  if (role === "ORCHESTRATOR") return { authorityKind: "WORKFLOW_AUTHORITY", validatorRoleKind: null };
  if (role === "CODER") return { authorityKind: "PRIMARY_CODER", validatorRoleKind: null };
  if (role === "MEMORY_MANAGER") return { authorityKind: "MEMORY_MANAGER", validatorRoleKind: null };
  if (role === "WP_VALIDATOR") return { authorityKind: "WP_VALIDATOR", validatorRoleKind: "WP_VALIDATOR" };
  if (role === "INTEGRATION_VALIDATOR") return { authorityKind: "INTEGRATION_VALIDATOR", validatorRoleKind: "INTEGRATION_VALIDATOR" };
  if (role === "VALIDATOR") {
    if (session && runtimeStatus?.integration_validator_of_record === session) {
      return { authorityKind: "INTEGRATION_VALIDATOR", validatorRoleKind: "INTEGRATION_VALIDATOR" };
    }
    if (session && runtimeStatus?.wp_validator_of_record === session) {
      return { authorityKind: "WP_VALIDATOR", validatorRoleKind: "WP_VALIDATOR" };
    }
    if (Array.isArray(runtimeStatus?.secondary_validator_sessions) && runtimeStatus.secondary_validator_sessions.includes(session)) {
      return { authorityKind: "SECONDARY_VALIDATOR", validatorRoleKind: "SECONDARY_VALIDATOR" };
    }
    return { authorityKind: "SECONDARY_VALIDATOR", validatorRoleKind: "SECONDARY_VALIDATOR" };
  }
  return { authorityKind: "SYSTEM", validatorRoleKind: null };
}
