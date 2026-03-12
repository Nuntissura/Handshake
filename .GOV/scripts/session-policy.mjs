import path from "node:path";

export const PACKET_FORMAT_VERSION = "2026-03-12";
export const STUB_FORMAT_VERSION = "2026-03-12";

export const SESSION_HOST_PREFERENCE = "VSCODE_INTEGRATED_TERMINAL";
export const SESSION_HOST_FALLBACK = "WINDOWS_TERMINAL";
export const SESSION_LAUNCH_POLICY = "ORCHESTRATOR_LAUNCHES_CLI_SESSIONS";
export const ROLE_SESSION_RUNTIME = "CLI";
export const CLI_SESSION_TOOL = "codex";

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
