import { invoke } from "@tauri-apps/api/core";

// Frontend IPC wrappers for the official CLI-bridge operator config surface
// (WP-KERNEL-004 follow-up). Mirrors the cloud-lane IPC client shape
// (app/src/lib/ipc/cloud_lane.ts): typed camelCase interfaces + thin
// `invoke<T>(...)` wrappers. Every call round-trips through the real Tauri
// `kernel_cli_bridge_*` commands; there is NO placeholder data here.
//
// Capability/secret posture: the CLI bridge auths via the operator's own CLI
// login (e.g. `claude auth login`). NO API key is stored by this surface; the
// config is executable path + args template + model allowlist + timeout only.

/** Mirrors backend `StoredCliKind` (snake_case serde) / core `CliKind`. */
export type CliKind = "claude_code" | "codex_cli" | "gemini_cli" | "other";

/** Mirrors backend `StoredOutputFormat` (snake_case serde) / core `CliOutputFormat`. */
export type StoredOutputFormat = "json" | "raw_text" | "json_stream";

/**
 * Mirrors backend `EnvVarPair` (cli_bridge_config.rs, `#[serde(rename_all =
 * "camelCase")]`). The write request carries env vars as an ARRAY of
 * `{ key, value }` pairs — serde deserializes `Vec<EnvVarPair>` from a JSON
 * array, NOT a map. Keep this shape exact or every real Save fails at the IPC
 * deserialize boundary.
 */
export interface EnvVarPair {
  key: string;
  value: string;
}

/**
 * Projection of the stored CLI-bridge config doc (camelCase over the wire).
 * `configured: false` means the official_cli swarm lane stays disabled
 * (honest unconfigured default).
 */
export interface CliBridgeConfigSummary {
  configured: boolean;
  cliKind: CliKind;
  executablePath: string;
  argsTemplate: string[];
  outputFormat: StoredOutputFormat;
  modelAllowlist: string[];
  workingDir: string | null;
  timeoutSeconds: number;
  /**
   * Env-var NAMES only (backend `CliBridgeConfigSummary.env_var_names` ->
   * `envVarNames: string[]`). Values are intentionally NOT returned to the UI
   * by the backend projection, so the summary carries names alone. The panel
   * re-seeds env rows from these names with empty values on reload.
   */
  envVarNames: string[];
  updatedAtUtc: string | null;
}

/** Full set-config request (the doc minus the server-owned schema_version). */
export interface SetCliBridgeConfigRequest {
  cliKind: CliKind;
  executablePath: string;
  argsTemplate: string[];
  outputFormat: StoredOutputFormat;
  modelAllowlist: string[];
  workingDir: string | null;
  timeoutSeconds: number;
  /**
   * Array of `{ key, value }` pairs (backend `Vec<EnvVarPair>`). Send `[]` when
   * empty. Sending a `Record<string,string>` object here makes serde reject the
   * whole request, so this MUST stay an array.
   */
  envVars: EnvVarPair[];
}

/** Static preset prefill returned by `kernel_cli_bridge_list_presets`. */
export interface CliBridgePreset {
  id: string;
  label: string;
  cliKind: CliKind;
  executableHint: string;
  argsTemplate: string[];
  outputFormat: StoredOutputFormat;
  modelAllowlist: string[];
  defaultTimeoutSeconds: number;
  /**
   * The `--version`-style preflight arg the backend preset carries
   * (`version_arg` -> `versionArg`). Used by the Test-configuration preflight so
   * the operator probes with the CLI's real version flag.
   */
  versionArg: string;
}

/** Real preflight receipt from `kernel_cli_bridge_test_config`. */
export interface CliBridgeTestReceipt {
  ok: boolean;
  versionLine: string | null;
  detail: string;
}

/**
 * Preflight request — exe path plus the optional version arg to probe with.
 * Mirrors backend `TestCliBridgeConfigRequest { executable_path, version_arg }`.
 * `versionArg` defaults to `--version` server-side when omitted; there is no
 * args-template field on the real backend request.
 */
export interface TestCliBridgeConfigRequest {
  executablePath: string;
  versionArg?: string;
}

// ---------------------------------------------------------------------------
// Tauri command wrappers.
// ---------------------------------------------------------------------------

export async function getCliBridgeConfig(): Promise<CliBridgeConfigSummary> {
  return await invoke<CliBridgeConfigSummary>("kernel_cli_bridge_get_config");
}

export async function setCliBridgeConfig(
  request: SetCliBridgeConfigRequest,
): Promise<CliBridgeConfigSummary> {
  return await invoke<CliBridgeConfigSummary>("kernel_cli_bridge_set_config", {
    request,
  });
}

export async function clearCliBridgeConfig(
  operatorSignature: string,
): Promise<CliBridgeConfigSummary> {
  return await invoke<CliBridgeConfigSummary>("kernel_cli_bridge_clear_config", {
    operatorSignature,
  });
}

export async function listCliBridgePresets(): Promise<CliBridgePreset[]> {
  return await invoke<CliBridgePreset[]>("kernel_cli_bridge_list_presets");
}

export async function testCliBridgeConfig(
  request: TestCliBridgeConfigRequest,
): Promise<CliBridgeTestReceipt> {
  return await invoke<CliBridgeTestReceipt>("kernel_cli_bridge_test_config", {
    request,
  });
}

// ---------------------------------------------------------------------------
// Injectable IPC surface (mock seam for jsdom unit tests).
//
// `@tauri-apps/api`'s `invoke` is unavailable under jsdom, so the panel takes
// this client as an optional prop. Production renders use the real default
// below; tests pass a fake. This keeps the panel a pure React unit while every
// real call still hits the Tauri command layer.
// ---------------------------------------------------------------------------

export interface CliBridgeConfigIpc {
  getConfig: typeof getCliBridgeConfig;
  setConfig: typeof setCliBridgeConfig;
  clearConfig: typeof clearCliBridgeConfig;
  listPresets: typeof listCliBridgePresets;
  testConfig: typeof testCliBridgeConfig;
}

export const defaultCliBridgeConfigIpc: CliBridgeConfigIpc = {
  getConfig: getCliBridgeConfig,
  setConfig: setCliBridgeConfig,
  clearConfig: clearCliBridgeConfig,
  listPresets: listCliBridgePresets,
  testConfig: testCliBridgeConfig,
};

// ---------------------------------------------------------------------------
// Format helpers for the UI (mirrors cloud_lane.formatStoredOnDate).
// ---------------------------------------------------------------------------

/** Format an RFC3339 timestamp as YYYY-MM-DD for the "stored on" indicator. */
export function formatStoredOnDate(updatedAtUtc: string | null): string {
  if (!updatedAtUtc) return "never";
  const dateMatch = updatedAtUtc.match(/^\d{4}-\d{2}-\d{2}/);
  return dateMatch ? dateMatch[0] : updatedAtUtc;
}

/** Human label for a CliKind, used by the read-only kind row. */
export const CLI_KIND_LABELS: Record<CliKind, string> = {
  claude_code: "Claude Code",
  codex_cli: "Codex CLI",
  gemini_cli: "Gemini CLI",
  other: "Generic / any CLI",
};

/** Client-side mirror of the backend `{prompt}` validation gate. */
export function argsTemplateHasPrompt(argsTemplate: readonly string[]): boolean {
  return argsTemplate.some((arg) => arg.includes("{prompt}"));
}

/** Env-var names that the spawner strips at launch; used for a soft warning. */
export const SECRET_BEARING_ENV_PATTERN = /API_KEY|TOKEN|SECRET|PASSWORD/i;
