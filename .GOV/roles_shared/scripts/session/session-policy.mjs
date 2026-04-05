import { createHash } from "node:crypto";
import path from "node:path";
import {
  LEGACY_SHARED_GOV_SESSION_CONTROL_BROKER_STATE_FILE,
  LEGACY_SHARED_GOV_SESSION_CONTROL_OUTPUT_DIR,
  LEGACY_SHARED_GOV_SESSION_CONTROL_REQUESTS_FILE,
  LEGACY_SHARED_GOV_SESSION_CONTROL_RESULTS_FILE,
  LEGACY_SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE,
  LEGACY_SHARED_GOV_SESSION_REGISTRY_FILE,
  SHARED_GOV_SESSION_CONTROL_BROKER_STATE_FILE,
  SHARED_GOV_SESSION_CONTROL_OUTPUT_DIR,
  SHARED_GOV_SESSION_CONTROL_REQUESTS_FILE,
  SHARED_GOV_SESSION_CONTROL_RESULTS_FILE,
  SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE,
  SHARED_GOV_SESSION_REGISTRY_FILE,
} from "../lib/runtime-paths.mjs";
import { ACP_BUILD_ID } from "./acp-build-id.mjs";

export const PACKET_FORMAT_VERSION = "2026-04-01";
export const STUB_FORMAT_VERSION = "2026-03-16";
export const SESSION_POLICY_PACKET_MIN_VERSION = "2026-03-12";
export const SESSION_POLICY_STUB_MIN_VERSION = "2026-03-12";
export const EXTERNAL_GOV_RUNTIME_PACKET_MIN_VERSION = "2026-03-16";
export const EXTERNAL_GOV_RUNTIME_STUB_MIN_VERSION = "2026-03-16";
export const STRUCTURED_VALIDATION_REPORT_MIN_VERSION = "2026-03-15";
export const SHARED_REMOTE_WP_BACKUP_PACKET_MIN_VERSION = "2026-03-16";
export const SPEC_CLAUSE_MAP_MIN_VERSION = "2026-03-18";
export const COMPLETION_LAYER_VERDICTS_MIN_VERSION = "2026-03-22";
export const MERGE_CONTAINMENT_PACKET_MIN_VERSION = "2026-03-25";
export const DEDICATED_WP_VALIDATOR_WORKTREE_PACKET_MIN_VERSION = "2026-03-29";

export const SESSION_START_AUTHORITY = "ORCHESTRATOR_ONLY";
export const SESSION_HOST_PREFERENCE = "VSCODE_EXTENSION_TERMINAL";
export const SESSION_HOST_FALLBACK = "CLI_ESCALATION_WINDOW";
export const SESSION_LAUNCH_POLICY = "ORCHESTRATOR_PLUGIN_FIRST_WITH_2TRY_ESCALATION";
export const ROLE_SESSION_RUNTIME = "CLI";
export const CLI_SESSION_TOOL = "codex";
export const SESSION_PLUGIN_BRIDGE_ID = "handshake.handshake-session-bridge";
export const SESSION_PLUGIN_BRIDGE_COMMAND = "handshakeSessionBridge.processLaunchQueue";
export const SESSION_PLUGIN_REQUESTS_FILE = SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE;
export const SESSION_REGISTRY_FILE = SHARED_GOV_SESSION_REGISTRY_FILE;
export const SESSION_CONTROL_MODE = "STEERABLE";
export const SESSION_CONTROL_TRANSPORT_PRIMARY = "CODEX_EXEC_RESUME_JSON";
export const SESSION_CONTROL_PROTOCOL_PRIMARY = "HANDSHAKE_ACP_STDIO_V1";
export const SESSION_CONTROL_HOST_PRIMARY = "HANDSHAKE_ACP_BROKER";
export const SESSION_CONTROL_BROKER_BUILD_ID = ACP_BUILD_ID;
export const SESSION_CONTROL_BROKER_AUTH_MODE = "LOCAL_TOKEN_FILE_V1";
export const SESSION_CONTROL_REQUESTS_FILE = SHARED_GOV_SESSION_CONTROL_REQUESTS_FILE;
export const SESSION_CONTROL_RESULTS_FILE = SHARED_GOV_SESSION_CONTROL_RESULTS_FILE;
export const SESSION_CONTROL_OUTPUT_DIR = SHARED_GOV_SESSION_CONTROL_OUTPUT_DIR;
export const SESSION_CONTROL_BROKER_STATE_FILE = SHARED_GOV_SESSION_CONTROL_BROKER_STATE_FILE;
export const SESSION_CONTROL_RUN_TIMEOUT_SECONDS = 5400;
export const SESSION_CONTROL_RUN_STALE_GRACE_SECONDS = 30;
export const SESSION_CONTROL_BROKER_SHUTDOWN_GRACE_SECONDS = 5;
export const SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION = 2;
export const SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS = 20;
export const SESSION_BATCH_SCOPE = "REPO_GOVERNED_BATCH";
export const SESSION_BATCH_MODE_PLUGIN_FIRST = "PLUGIN_FIRST";
export const SESSION_BATCH_MODE_CLI_ESCALATION = "CLI_ESCALATION_BATCH";
export const SESSION_BATCH_MODE_VALUES = [
  SESSION_BATCH_MODE_PLUGIN_FIRST,
  SESSION_BATCH_MODE_CLI_ESCALATION,
];
export const SESSION_WATCH_POLICY = "EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK";
export const SESSION_WAKE_CHANNEL_PRIMARY = "VS_CODE_FILE_WATCH";
export const SESSION_WAKE_CHANNEL_FALLBACK = "WP_HEARTBEAT";
export const CLI_ESCALATION_HOST_DEFAULT = "SYSTEM_TERMINAL";
export const CLI_ESCALATION_HOST_LEGACY_ALIAS = "WINDOWS_TERMINAL";

export const MODEL_FAMILY_POLICY = "OPENAI_GPT_SERIES_ONLY";
export const CODEX_MODEL_ALIASES_ALLOWED = "NO";
export const ROLE_SESSION_PRIMARY_MODEL = "gpt-5.4";
export const ROLE_SESSION_FALLBACK_MODEL = "gpt-5.2";
export const ROLE_SESSION_REASONING_REQUIRED = "EXTRA_HIGH";
export const REASONING_ENFORCEMENT_MODE = "SESSION_BRIEF_AND_CLAIM_CHECK";
export const ROLE_SESSION_REASONING_CONFIG_KEY = "model_reasoning_effort";
export const ROLE_SESSION_REASONING_CONFIG_VALUE = "xhigh";
export const WP_TOKEN_BUDGET_POLICY_ID = "ORCHESTRATOR_MANAGED_V1";
export const WP_TOKEN_LEDGER_HEALTH_POLICY_ID = "ORCHESTRATOR_MANAGED_LEDGER_V1";
export const WP_TOKEN_LEDGER_HEALTH_THRESHOLDS = Object.freeze({
  warn_command_delta_count: 1,
  fail_command_delta_count: 2,
  warn_turn_delta: 1,
  fail_turn_delta: 2,
  warn_input_token_delta: 1,
  fail_input_token_delta: 50000,
  warn_cached_input_token_delta: 1,
  fail_cached_input_token_delta: 50000,
  warn_output_token_delta: 1,
  fail_output_token_delta: 5000,
  warn_input_token_delta_ratio_pct: 0.1,
  fail_input_token_delta_ratio_pct: 5,
  warn_cached_input_token_delta_ratio_pct: 0.1,
  fail_cached_input_token_delta_ratio_pct: 5,
  warn_output_token_delta_ratio_pct: 0.1,
  fail_output_token_delta_ratio_pct: 5,
});
export const WP_ROLE_TOKEN_BUDGETS = Object.freeze({
  CODER: Object.freeze({
    warn_turn_count: 10,
    fail_turn_count: 14,
    warn_input_tokens: 120000000,
    fail_input_tokens: 180000000,
  }),
  WP_VALIDATOR: Object.freeze({
    warn_turn_count: 8,
    fail_turn_count: 12,
    warn_input_tokens: 60000000,
    fail_input_tokens: 90000000,
  }),
  INTEGRATION_VALIDATOR: Object.freeze({
    warn_turn_count: 6,
    fail_turn_count: 8,
    warn_input_tokens: 40000000,
    fail_input_tokens: 60000000,
  }),
});
export const WP_TOTAL_TOKEN_BUDGET = Object.freeze({
  warn_turn_count: 24,
  fail_turn_count: 32,
  warn_input_tokens: 180000000,
  fail_input_tokens: 260000000,
});

export const EXECUTION_OWNER_TOKENS = Array.from({ length: 26 }, (_, index) =>
  String.fromCharCode("A".charCodeAt(0) + index),
);
export const EXECUTION_OWNER_VALUES = EXECUTION_OWNER_TOKENS.map((token) => `CODER_${token}`);
export const EXECUTION_OWNER_RANGE_HELP = "Coder-A..Coder-Z";
export const SESSION_ROLES = ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"];
export const SESSION_RUNTIME_STATES = [
  "UNSTARTED",
  "PLUGIN_REQUESTED",
  "TERMINAL_COMMAND_DISPATCHED",
  "PLUGIN_CONFIRMED",
  "CLI_ESCALATION_READY",
  "CLI_ESCALATION_USED",
  "STARTING",
  "READY",
  "COMMAND_RUNNING",
  "ACTIVE",
  "WAITING",
  "COMPLETED",
  "FAILED",
  "STALE",
  "CLOSED",
];
export const SESSION_REQUEST_STATUSES = [
  "QUEUED",
  "PLUGIN_DISPATCHED",
  "PLUGIN_CONFIRMED",
  "PLUGIN_FAILED",
  "PLUGIN_TIMED_OUT",
  "CLI_ESCALATION_USED",
];
export const SESSION_COMMAND_KINDS = [
  "START_SESSION",
  "SEND_PROMPT",
  "CANCEL_SESSION",
  "CLOSE_SESSION",
];
export const SESSION_CONTROL_SUPPORTED_METHODS = [
  "session/new",
  "session/load",
  "session/prompt",
  "session/cancel",
  "session/close",
  "broker/shutdown",
];
export const SESSION_COMMAND_STATUSES = [
  "QUEUED",
  "RUNNING",
  "COMPLETED",
  "FAILED",
];
export const SESSION_ACTIVE_HOST_NONE = "NONE";
export const SESSION_ACTIVE_HOST_VALUES = [
  SESSION_ACTIVE_HOST_NONE,
  SESSION_HOST_PREFERENCE,
  SESSION_CONTROL_HOST_PRIMARY,
  SESSION_HOST_FALLBACK,
];
export const SESSION_ACTIVE_TERMINAL_KIND_NONE = "NONE";
export const SESSION_ACTIVE_TERMINAL_KIND_VALUES = [
  SESSION_ACTIVE_TERMINAL_KIND_NONE,
  SESSION_HOST_PREFERENCE,
  CLI_ESCALATION_HOST_DEFAULT,
  "CURRENT",
  "PRINT",
];
export const SESSION_TERMINAL_OWNERSHIP_SCOPE_NONE = "NONE";
export const SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION = "GOVERNED_SESSION";
export const SESSION_TERMINAL_OWNERSHIP_SCOPE_VALUES = [
  SESSION_TERMINAL_OWNERSHIP_SCOPE_NONE,
  SESSION_TERMINAL_OWNERSHIP_SCOPE_GOVERNED_SESSION,
];
export const SESSION_TERMINAL_RECLAIM_STATUS_NONE = "NONE";
export const SESSION_TERMINAL_RECLAIM_STATUS_OWNED = "OWNED";
export const SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED = "RECLAIMED";
export const SESSION_TERMINAL_RECLAIM_STATUS_ALREADY_EXITED = "ALREADY_EXITED";
export const SESSION_TERMINAL_RECLAIM_STATUS_FAILED = "FAILED";
export const SESSION_TERMINAL_RECLAIM_STATUS_VALUES = [
  SESSION_TERMINAL_RECLAIM_STATUS_NONE,
  SESSION_TERMINAL_RECLAIM_STATUS_OWNED,
  SESSION_TERMINAL_RECLAIM_STATUS_RECLAIMED,
  SESSION_TERMINAL_RECLAIM_STATUS_ALREADY_EXITED,
  SESSION_TERMINAL_RECLAIM_STATUS_FAILED,
];

export function normalizeActiveHostValue(value) {
  const token = String(value || "").trim();
  if (!token || token === SESSION_ACTIVE_HOST_NONE) return SESSION_ACTIVE_HOST_NONE;
  if (token === SESSION_CONTROL_PROTOCOL_PRIMARY || token === SESSION_CONTROL_TRANSPORT_PRIMARY) {
    return SESSION_CONTROL_HOST_PRIMARY;
  }
  if (
    token === CLI_ESCALATION_HOST_DEFAULT ||
    token === CLI_ESCALATION_HOST_LEGACY_ALIAS ||
    token === "CURRENT" ||
    token === "PRINT"
  ) {
    return SESSION_HOST_FALLBACK;
  }
  return token;
}

export function normalizeActiveTerminalKindValue(value) {
  const token = String(value || "").trim();
  if (!token || token === SESSION_ACTIVE_TERMINAL_KIND_NONE) return SESSION_ACTIVE_TERMINAL_KIND_NONE;
  if (token === SESSION_CONTROL_PROTOCOL_PRIMARY || token === SESSION_CONTROL_TRANSPORT_PRIMARY) {
    return SESSION_ACTIVE_TERMINAL_KIND_NONE;
  }
  if (token === SESSION_CONTROL_HOST_PRIMARY || token === "HANDSHAKE_ACP_BRIDGE") return SESSION_ACTIVE_TERMINAL_KIND_NONE;
  if (token === CLI_ESCALATION_HOST_LEGACY_ALIAS) return CLI_ESCALATION_HOST_DEFAULT;
  if (token === SESSION_HOST_FALLBACK) return CLI_ESCALATION_HOST_DEFAULT;
  return token;
}

export function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

export function defaultCoderBranch(wpId) {
  return `feat/${wpId}`;
}

function shortWorktreeName(prefix, wpId) {
  // Convert "WP-1-Structured-Collaboration-Schema-Registry-v3" to "schema-registry-v3"
  // Drop "WP-{N}-" prefix, take last meaningful segments, lowercase.
  const id = String(wpId || "").trim();
  const withoutWpPrefix = id.replace(/^WP-\d+-/i, "");
  const parts = withoutWpPrefix.split("-").filter(Boolean);
  // Keep version suffix if present (e.g. "v3")
  const versionIdx = parts.findLastIndex((p) => /^v\d+$/i.test(p));
  let shortParts;
  if (versionIdx >= 0 && parts.length > 3) {
    // Take last 2 meaningful parts before version + version
    const meaningful = parts.slice(0, versionIdx);
    shortParts = [...meaningful.slice(-2), parts[versionIdx]];
  } else if (parts.length > 3) {
    shortParts = parts.slice(-3);
  } else {
    shortParts = parts;
  }
  const name = shortParts.join("-").toLowerCase();
  return normalizePath(path.join("..", `${prefix}-${name}`));
}

export function defaultCoderWorktreeDir(wpId) {
  return shortWorktreeName("wtc", wpId);
}

export function defaultWpValidatorBranch(wpId) {
  return `validate/${wpId}`;
}

export function defaultWpValidatorWorktreeDir(wpId) {
  return shortWorktreeName("wtv", wpId);
}

export function defaultIntegrationValidatorBranch(wpId) {
  // Integration validator operates from handshake_main on branch main [CX-212D].
  return "main";
}

export function defaultIntegrationValidatorWorktreeDir(wpId) {
  // Integration validator operates from handshake_main — no WP-specific worktree [CX-212D].
  return "../handshake_main";
}

export function sessionKey(role, wpId) {
  return `${String(role || "").trim().toUpperCase()}:${String(wpId || "").trim()}`;
}

export function terminalTitle(role, wpId) {
  if (role === "CODER") return `CODER ${wpId}`;
  if (role === "WP_VALIDATOR") return `WPVAL ${wpId}`;
  if (role === "INTEGRATION_VALIDATOR") return `INTVAL ${wpId}`;
  return `${String(role || "").trim().toUpperCase()} ${wpId}`.trim();
}

export function normalizeExecutionOwner(raw) {
  const value = String(raw || "").trim();
  if (!value) return "";

  const compact = value.toUpperCase().replace(/[\s_]+/g, "-");
  const directMatch = compact.match(/^CODER-([A-Z])$/);
  if (directMatch) return `Coder-${directMatch[1]}`;
  const singleTokenMatch = compact.match(/^([A-Z])$/);
  if (singleTokenMatch) return `Coder-${singleTokenMatch[1]}`;
  return null;
}

export function executionOwnerToPacketValue(raw) {
  const normalized = normalizeExecutionOwner(raw);
  if (!normalized) return null;
  return normalized.replace("-", "_").toUpperCase();
}

export function executionOwnerDisplay(value) {
  const normalized = normalizeExecutionOwner(value);
  return normalized || "";
}

export function isDisallowedCodexModelAlias(value) {
  return /codex/i.test(String(value || ""));
}

export function isAllowedPrimaryOrFallbackModel(value) {
  const token = String(value || "").trim().toLowerCase();
  return token === ROLE_SESSION_PRIMARY_MODEL || token === ROLE_SESSION_FALLBACK_MODEL;
}

export function buildRemoteBackupUrl(originTreeBase, branch) {
  if (!originTreeBase || originTreeBase === "<pending>") return "<pending>";
  return `${originTreeBase.replace(/\/+$/, "")}/tree/${branch}`;
}

export function packetUsesSessionPolicy(packetFormatVersion) {
  const version = String(packetFormatVersion || "").trim();
  return version >= SESSION_POLICY_PACKET_MIN_VERSION;
}

export function packetUsesExternalGovernanceRuntime(packetFormatVersion) {
  const version = String(packetFormatVersion || "").trim();
  return version >= EXTERNAL_GOV_RUNTIME_PACKET_MIN_VERSION;
}

export function packetUsesStructuredValidationReport(packetFormatVersion) {
  const version = String(packetFormatVersion || "").trim();
  return version >= STRUCTURED_VALIDATION_REPORT_MIN_VERSION;
}

export function packetUsesSharedRemoteWpBackup(packetFormatVersion) {
  const version = String(packetFormatVersion || "").trim();
  return version >= SHARED_REMOTE_WP_BACKUP_PACKET_MIN_VERSION;
}

export function packetRequiresSpecClauseMap(packetFormatVersion) {
  const version = String(packetFormatVersion || "").trim();
  return version >= SPEC_CLAUSE_MAP_MIN_VERSION;
}

export function packetRequiresCompletionLayerVerdicts(packetFormatVersion) {
  const version = String(packetFormatVersion || "").trim();
  return version >= COMPLETION_LAYER_VERDICTS_MIN_VERSION;
}

export function packetRequiresMergeContainmentTruth(packetFormatVersion) {
  const version = String(packetFormatVersion || "").trim();
  return version >= MERGE_CONTAINMENT_PACKET_MIN_VERSION;
}

export function stubUsesSessionPolicy(stubFormatVersion) {
  const version = String(stubFormatVersion || "").trim();
  return version >= SESSION_POLICY_STUB_MIN_VERSION;
}

export function stubUsesExternalGovernanceRuntime(stubFormatVersion) {
  const version = String(stubFormatVersion || "").trim();
  return version >= EXTERNAL_GOV_RUNTIME_STUB_MIN_VERSION;
}

export function sessionPluginRequestsFileForPacketVersion(packetFormatVersion) {
  return packetUsesExternalGovernanceRuntime(packetFormatVersion)
    ? SESSION_PLUGIN_REQUESTS_FILE
    : LEGACY_SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE;
}

export function sessionRegistryFileForPacketVersion(packetFormatVersion) {
  return packetUsesExternalGovernanceRuntime(packetFormatVersion)
    ? SESSION_REGISTRY_FILE
    : LEGACY_SHARED_GOV_SESSION_REGISTRY_FILE;
}

export function sessionControlRequestsFileForPacketVersion(packetFormatVersion) {
  return packetUsesExternalGovernanceRuntime(packetFormatVersion)
    ? SESSION_CONTROL_REQUESTS_FILE
    : LEGACY_SHARED_GOV_SESSION_CONTROL_REQUESTS_FILE;
}

export function sessionControlResultsFileForPacketVersion(packetFormatVersion) {
  return packetUsesExternalGovernanceRuntime(packetFormatVersion)
    ? SESSION_CONTROL_RESULTS_FILE
    : LEGACY_SHARED_GOV_SESSION_CONTROL_RESULTS_FILE;
}

export function sessionControlOutputDirForPacketVersion(packetFormatVersion) {
  return packetUsesExternalGovernanceRuntime(packetFormatVersion)
    ? SESSION_CONTROL_OUTPUT_DIR
    : LEGACY_SHARED_GOV_SESSION_CONTROL_OUTPUT_DIR;
}

export function sessionControlBrokerStateFileForPacketVersion(packetFormatVersion) {
  return packetUsesExternalGovernanceRuntime(packetFormatVersion)
    ? SESSION_CONTROL_BROKER_STATE_FILE
    : LEGACY_SHARED_GOV_SESSION_CONTROL_BROKER_STATE_FILE;
}

export function sessionPluginRequestsFileForStubVersion(stubFormatVersion) {
  return stubUsesExternalGovernanceRuntime(stubFormatVersion)
    ? SESSION_PLUGIN_REQUESTS_FILE
    : LEGACY_SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE;
}

export function sessionRegistryFileForStubVersion(stubFormatVersion) {
  return stubUsesExternalGovernanceRuntime(stubFormatVersion)
    ? SESSION_REGISTRY_FILE
    : LEGACY_SHARED_GOV_SESSION_REGISTRY_FILE;
}

export function roleStartupCommand(role) {
  if (role === "CODER") return "just coder-startup";
  if (role === "WP_VALIDATOR" || role === "INTEGRATION_VALIDATOR") return "just validator-startup";
  return "just orchestrator-startup";
}

export function roleNextCommand(role, wpId) {
  if (role === "CODER") return `just coder-next ${wpId}`;
  if (role === "WP_VALIDATOR" || role === "INTEGRATION_VALIDATOR") return `just validator-next ${wpId}`;
  return `just orchestrator-next ${wpId}`;
}

export function roleStageLabel(role) {
  if (role === "CODER") return "CODER";
  if (role === "WP_VALIDATOR") return "WP_VALIDATOR";
  if (role === "INTEGRATION_VALIDATOR") return "INTEGRATION_VALIDATOR";
  return "ORCHESTRATOR";
}

export function roleLaunchAuthority(role) {
  if (SESSION_ROLES.includes(role)) return SESSION_START_AUTHORITY;
  return "DIRECT_ONLY";
}
