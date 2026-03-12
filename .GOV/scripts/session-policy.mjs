import path from "node:path";

export const PACKET_FORMAT_VERSION = "2026-03-12";
export const STUB_FORMAT_VERSION = "2026-03-12";

export const SESSION_START_AUTHORITY = "ORCHESTRATOR_ONLY";
export const SESSION_HOST_PREFERENCE = "VSCODE_EXTENSION_TERMINAL";
export const SESSION_HOST_FALLBACK = "CLI_ESCALATION_WINDOW";
export const SESSION_LAUNCH_POLICY = "ORCHESTRATOR_PLUGIN_FIRST_WITH_2TRY_ESCALATION";
export const ROLE_SESSION_RUNTIME = "CLI";
export const CLI_SESSION_TOOL = "codex";
export const SESSION_PLUGIN_BRIDGE_ID = "handshake.handshake-session-bridge";
export const SESSION_PLUGIN_BRIDGE_COMMAND = "handshakeSessionBridge.processLaunchQueue";
export const SESSION_PLUGIN_REQUESTS_FILE = ".GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl";
export const SESSION_REGISTRY_FILE = ".GOV/roles_shared/ROLE_SESSION_REGISTRY.json";
export const SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION = 2;
export const SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS = 20;
export const SESSION_WATCH_POLICY = "EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK";
export const SESSION_WAKE_CHANNEL_PRIMARY = "VS_CODE_FILE_WATCH";
export const SESSION_WAKE_CHANNEL_FALLBACK = "WP_HEARTBEAT";
export const CLI_ESCALATION_HOST_DEFAULT = "WINDOWS_TERMINAL";

export const MODEL_FAMILY_POLICY = "OPENAI_GPT_SERIES_ONLY";
export const CODEX_MODEL_ALIASES_ALLOWED = "NO";
export const ROLE_SESSION_PRIMARY_MODEL = "gpt-5.4";
export const ROLE_SESSION_FALLBACK_MODEL = "gpt-5.2";
export const ROLE_SESSION_REASONING_REQUIRED = "EXTRA_HIGH";
export const REASONING_ENFORCEMENT_MODE = "SESSION_BRIEF_AND_CLAIM_CHECK";
export const ROLE_SESSION_REASONING_CONFIG_KEY = "model_reasoning_effort";
export const ROLE_SESSION_REASONING_CONFIG_VALUE = "xhigh";

export const EXECUTION_OWNER_TOKENS = Array.from({ length: 26 }, (_, index) =>
  String.fromCharCode("A".charCodeAt(0) + index),
);
export const EXECUTION_OWNER_VALUES = EXECUTION_OWNER_TOKENS.map((token) => `CODER_${token}`);
export const EXECUTION_OWNER_RANGE_HELP = "Coder-A..Coder-Z";
export const SESSION_ROLES = ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"];
export const SESSION_RUNTIME_STATES = [
  "UNSTARTED",
  "PLUGIN_REQUESTED",
  "PLUGIN_CONFIRMED",
  "CLI_ESCALATION_READY",
  "CLI_ESCALATION_USED",
  "ACTIVE",
  "WAITING",
  "COMPLETED",
  "FAILED",
];
export const SESSION_REQUEST_STATUSES = [
  "QUEUED",
  "PLUGIN_CONFIRMED",
  "PLUGIN_FAILED",
  "PLUGIN_TIMED_OUT",
  "CLI_ESCALATION_USED",
];

export function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

export function defaultCoderBranch(wpId) {
  return `feat/${wpId}`;
}

export function defaultCoderWorktreeDir(wpId) {
  return normalizePath(path.join("..", `wt-${wpId}`));
}

export function defaultWpValidatorBranch(wpId) {
  return `validate/${wpId}`;
}

export function defaultWpValidatorWorktreeDir(wpId) {
  return normalizePath(path.join("..", `wt-WPV-${wpId}`));
}

export function defaultIntegrationValidatorBranch(wpId) {
  return `integrate/${wpId}`;
}

export function defaultIntegrationValidatorWorktreeDir(wpId) {
  return normalizePath(path.join("..", `wt-INTV-${wpId}`));
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
  return version >= PACKET_FORMAT_VERSION;
}

export function stubUsesSessionPolicy(stubFormatVersion) {
  const version = String(stubFormatVersion || "").trim();
  return version >= STUB_FORMAT_VERSION;
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
