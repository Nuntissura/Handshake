import { useCallback, useEffect, useMemo, useState } from "react";

import {
  type CliBridgeConfigIpc,
  type CliBridgeConfigSummary,
  type CliBridgePreset,
  type CliBridgeTestReceipt,
  type CliKind,
  type StoredOutputFormat,
  CLI_KIND_LABELS,
  SECRET_BEARING_ENV_PATTERN,
  argsTemplateHasPrompt,
  defaultCliBridgeConfigIpc,
  formatStoredOnDate,
} from "../lib/ipc/cli_bridge_config";

// CLI-Bridge operator config surface (WP-KERNEL-004 follow-up).
//
// Operator-facing settings panel that configures the official CLI bridge so a
// CLI-backed swarm session goes live in the app. Mounted inside SettingsMenu's
// "CLI Bridge" section. Self-contained: loads its own config + presets on mount
// via the injected IPC client (like CloudLanePanel.refresh), keeps an editable
// local draft, validates client-side (mirrors the backend register_bridge
// gate), and writes through the real `kernel_cli_bridge_*` Tauri commands.
//
// HONESTY:
//  - No mockups. Every control is backed by a real IPC command.
//  - NO API key field — the bridge auths via the operator's own CLI login.
//  - Save is disabled until the draft is client-side valid.
//  - "Test configuration" runs a REAL backend preflight (<exe> --version).
//  - The lane goes live at the NEXT app start (factory reads config at
//    production() construction) — surfaced honestly in the status note.

const OUTPUT_FORMAT_LABELS: Record<StoredOutputFormat, string> = {
  raw_text: "Raw text (stdout)",
  json: "JSON",
  json_stream: "JSON stream (JSONL)",
};

const OUTPUT_FORMATS: StoredOutputFormat[] = ["raw_text", "json", "json_stream"];

interface EnvVarRow {
  key: string;
  value: string;
}

/** Local editable draft mirroring the stored doc's editable fields. */
interface ConfigDraft {
  presetId: string;
  cliKind: CliKind;
  executablePath: string;
  argsText: string; // newline-delimited; split into argsTemplate on save
  outputFormat: StoredOutputFormat;
  allowlistText: string; // comma/newline-delimited
  workingDir: string;
  timeoutSeconds: string; // kept as string for honest empty/zero handling
  envVars: EnvVarRow[];
}

const EMPTY_DRAFT: ConfigDraft = {
  presetId: "",
  cliKind: "other",
  executablePath: "",
  argsText: "{prompt}",
  outputFormat: "raw_text",
  allowlistText: "",
  workingDir: "",
  timeoutSeconds: "120",
  envVars: [],
};

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

/** Split a newline-delimited args block into a template array. Blank lines are
 * dropped; trailing/leading whitespace per line preserved-as-trimmed so the
 * operator can format the textarea freely. */
function parseArgsText(argsText: string): string[] {
  return argsText
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/** Parse a comma/newline-delimited allowlist into a deduped, trimmed list. */
function parseAllowlist(allowlistText: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of allowlistText.split(/[\n,]/)) {
    const item = raw.trim();
    if (item.length > 0 && !seen.has(item)) {
      seen.add(item);
      out.push(item);
    }
  }
  return out;
}

function summaryToDraft(
  summary: CliBridgeConfigSummary,
  presets: CliBridgePreset[],
): ConfigDraft {
  if (!summary.configured) {
    return EMPTY_DRAFT;
  }
  // Best-effort: if the stored kind matches a preset, surface that preset in the
  // selector; otherwise leave the selector on the "custom" sentinel.
  const matchingPreset = presets.find((p) => p.cliKind === summary.cliKind);
  return {
    presetId: matchingPreset?.id ?? "",
    cliKind: summary.cliKind,
    executablePath: summary.executablePath,
    argsText: summary.argsTemplate.join("\n"),
    outputFormat: summary.outputFormat,
    allowlistText: summary.modelAllowlist.join("\n"),
    workingDir: summary.workingDir ?? "",
    timeoutSeconds: String(summary.timeoutSeconds),
    // The backend summary returns env-var NAMES only (values are intentionally
    // not re-surfaced — see CliBridgeConfigSummary.envVarNames). Seed each row
    // from a name with an empty value; the operator re-enters the value if they
    // want to change it (a blank value is saved as-is, which is the honest
    // behavior since the stored value was never returned to the UI).
    envVars: summary.envVarNames.map((key) => ({ key, value: "" })),
  };
}

export interface CliBridgeConfigPanelProps {
  /** Injected IPC client. Defaults to the real Tauri-backed client; tests pass
   * a fake because `@tauri-apps/api`'s `invoke` is unavailable under jsdom. */
  ipc?: CliBridgeConfigIpc;
}

export function CliBridgeConfigPanel({
  ipc = defaultCliBridgeConfigIpc,
}: CliBridgeConfigPanelProps = {}) {
  const [presets, setPresets] = useState<CliBridgePreset[]>([]);
  const [summary, setSummary] = useState<CliBridgeConfigSummary | null>(null);
  const [draft, setDraft] = useState<ConfigDraft>(EMPTY_DRAFT);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [lastReceipt, setLastReceipt] = useState<string | null>(null);
  const [testReceipt, setTestReceipt] = useState<CliBridgeTestReceipt | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const [nextPresets, nextSummary] = await Promise.all([
        ipc.listPresets(),
        ipc.getConfig(),
      ]);
      setPresets(nextPresets);
      setSummary(nextSummary);
      setDraft(summaryToDraft(nextSummary, nextPresets));
      setLoadError(null);
    } catch (error) {
      setLoadError(errorMessage(error));
    }
  }, [ipc]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // ---- derived draft projections -----------------------------------------
  const argsTemplate = useMemo(() => parseArgsText(draft.argsText), [draft.argsText]);
  const allowlist = useMemo(() => parseAllowlist(draft.allowlistText), [draft.allowlistText]);
  const timeoutValue = Number.parseInt(draft.timeoutSeconds, 10);

  // ---- client-side validation (mirrors register_bridge backend gate) ------
  const hasPrompt = argsTemplateHasPrompt(argsTemplate);
  const exeNonEmpty = draft.executablePath.trim().length > 0;
  const timeoutValid = Number.isInteger(timeoutValue) && timeoutValue > 0;
  const allowlistNonEmpty = allowlist.length > 0;
  const canSave = hasPrompt && exeNonEmpty && timeoutValid && allowlistNonEmpty && !saving;

  // Soft, non-blocking warning: env names the spawner strips at launch.
  const secretEnvWarning = useMemo(
    () =>
      draft.envVars
        .filter((row) => SECRET_BEARING_ENV_PATTERN.test(row.key))
        .map((row) => row.key),
    [draft.envVars],
  );

  // ---- handlers -----------------------------------------------------------
  const handlePresetChange = (presetId: string) => {
    const preset = presets.find((p) => p.id === presetId);
    if (!preset) {
      setDraft((prev) => ({ ...prev, presetId: "" }));
      return;
    }
    setTestReceipt(null);
    setDraft((prev) => ({
      ...prev,
      presetId,
      cliKind: preset.cliKind,
      // Prefill exe only when the preset gives a hint; Generic = operator-supplied.
      executablePath: preset.executableHint.length > 0 ? preset.executableHint : prev.executablePath,
      argsText: preset.argsTemplate.join("\n"),
      outputFormat: preset.outputFormat,
      allowlistText: preset.modelAllowlist.join("\n"),
      timeoutSeconds: String(preset.defaultTimeoutSeconds),
    }));
  };

  const setField = <K extends keyof ConfigDraft>(key: K, value: ConfigDraft[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
  };

  const handleAddEnvVar = () => {
    setDraft((prev) => ({ ...prev, envVars: [...prev.envVars, { key: "", value: "" }] }));
  };

  const handleEnvVarChange = (index: number, field: keyof EnvVarRow, value: string) => {
    setDraft((prev) => {
      const next = prev.envVars.slice();
      next[index] = { ...next[index], [field]: value };
      return { ...prev, envVars: next };
    });
  };

  const handleRemoveEnvVar = (index: number) => {
    setDraft((prev) => ({
      ...prev,
      envVars: prev.envVars.filter((_, i) => i !== index),
    }));
  };

  const handleSave = async () => {
    if (!canSave) return;
    setSaving(true);
    try {
      // Backend expects Vec<EnvVarPair> -> a JSON ARRAY of { key, value }, NOT a
      // map. Emit the array (empty -> []) so the request deserializes server-side.
      const envVars = draft.envVars
        .map((row) => ({ key: row.key.trim(), value: row.value }))
        .filter((row) => row.key.length > 0);
      const workingDir = draft.workingDir.trim();
      const next = await ipc.setConfig({
        cliKind: draft.cliKind,
        executablePath: draft.executablePath.trim(),
        argsTemplate,
        outputFormat: draft.outputFormat,
        modelAllowlist: allowlist,
        workingDir: workingDir.length > 0 ? workingDir : null,
        timeoutSeconds: timeoutValue,
        envVars,
      });
      setSummary(next);
      setLastReceipt("Configuration saved — takes effect at next app launch.");
      setLoadError(null);
    } catch (error) {
      setLoadError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async () => {
    if (!exeNonEmpty) return;
    setTesting(true);
    setTestReceipt(null);
    try {
      // Probe with the selected preset's real version flag when known; otherwise
      // omit so the backend falls back to its `--version` default. The backend
      // request shape is { executablePath, versionArg? } — there is no
      // args-template field on the preflight contract.
      const selectedPreset = presets.find((p) => p.id === draft.presetId);
      const receipt = await ipc.testConfig({
        executablePath: draft.executablePath.trim(),
        ...(selectedPreset?.versionArg
          ? { versionArg: selectedPreset.versionArg }
          : {}),
      });
      setTestReceipt(receipt);
    } catch (error) {
      // A thrown IPC error is itself an honest failed preflight.
      setTestReceipt({ ok: false, versionLine: null, detail: errorMessage(error) });
    } finally {
      setTesting(false);
    }
  };

  const handleClear = async () => {
    try {
      const next = await ipc.clearConfig(`operator-${Date.now()}`);
      setSummary(next);
      setDraft(EMPTY_DRAFT);
      setTestReceipt(null);
      setLastReceipt("Configuration cleared — official_cli swarm lane disabled at next launch.");
      setLoadError(null);
    } catch (error) {
      setLoadError(errorMessage(error));
    }
  };

  // ---- render -------------------------------------------------------------
  const configured = summary?.configured ?? false;

  return (
    <div className="cli-bridge-config" data-testid="cli-bridge-config">
      <p className="muted small" data-testid="cli-bridge-config.intro">
        Configure the official CLI bridge so a CLI-backed swarm session runs in the
        app and streams its stdout into the terminal panel. Auth is via your own CLI
        login (e.g. <code>claude auth login</code>) — no API key is stored here.
      </p>

      {/* Status row -------------------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.status-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Status</span>
          {configured ? (
            <span className="settings-note" data-testid="cli-bridge-config.status">
              Configured · stored {formatStoredOnDate(summary?.updatedAtUtc ?? null)} ·
              takes effect at next launch
            </span>
          ) : (
            <span className="settings-note" data-testid="cli-bridge-config.status">
              Not configured — official_cli swarm lane disabled
            </span>
          )}
        </div>
      </div>

      {loadError ? (
        <p className="settings-note" role="alert" data-testid="cli-bridge-config.error">
          {loadError}
        </p>
      ) : null}
      {lastReceipt ? (
        <p className="settings-note" data-testid="cli-bridge-config.receipt">
          {lastReceipt}
        </p>
      ) : null}

      {/* Preset ------------------------------------------------------------ */}
      <div className="settings-row" data-testid="cli-bridge-config.preset-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Preset</span>
          <span className="muted small">
            Prefills executable, args template, output format, and model allowlist.
            Every field stays editable after selecting.
          </span>
        </div>
        <select
          className="settings-row__control"
          value={draft.presetId}
          onChange={(event) => handlePresetChange(event.target.value)}
          aria-label="CLI bridge preset"
          data-testid="cli-bridge-config.preset"
        >
          <option value="">Custom (no preset)</option>
          {presets.map((preset) => (
            <option key={preset.id} value={preset.id}>
              {preset.label}
            </option>
          ))}
        </select>
      </div>

      {/* CLI kind (read-only, derived from preset) ------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.kind-row">
        <div className="settings-row__main">
          <span className="settings-row__label">CLI kind</span>
          <span className="muted small">Derived from the selected preset.</span>
        </div>
        <input
          type="text"
          className="settings-row__control"
          value={CLI_KIND_LABELS[draft.cliKind]}
          readOnly
          aria-label="CLI kind (derived from preset)"
          data-testid="cli-bridge-config.kind"
        />
      </div>

      {/* Executable path --------------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.executable-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Executable path</span>
          <span className="muted small">
            Full path or a PATH-resolvable name (e.g. <code>claude</code>).
          </span>
        </div>
        <input
          type="text"
          className="settings-row__control"
          value={draft.executablePath}
          onChange={(event) => setField("executablePath", event.target.value)}
          placeholder="claude"
          aria-label="Executable path"
          data-testid="cli-bridge-config.executable"
        />
      </div>
      {!exeNonEmpty ? (
        <p className="settings-note" data-testid="cli-bridge-config.executable-warning">
          Executable is required.
        </p>
      ) : null}

      {/* Args template ----------------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.args-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Args template</span>
          <span className="muted small">
            One arg per line. Must contain <code>{"{prompt}"}</code>; may contain{" "}
            <code>{"{model}"}</code> (substituted with the allowlisted model at spawn).
          </span>
        </div>
        <textarea
          className="settings-row__control"
          rows={5}
          value={draft.argsText}
          onChange={(event) => setField("argsText", event.target.value)}
          aria-label="Args template (one arg per line)"
          data-testid="cli-bridge-config.args"
        />
      </div>
      {!hasPrompt ? (
        <p className="settings-note" role="alert" data-testid="cli-bridge-config.args-warning">
          Args template must contain a {"{prompt}"} placeholder.
        </p>
      ) : null}

      {/* Output format ----------------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.output-format-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Output format</span>
        </div>
        <select
          className="settings-row__control"
          value={draft.outputFormat}
          onChange={(event) => setField("outputFormat", event.target.value as StoredOutputFormat)}
          aria-label="Output format"
          data-testid="cli-bridge-config.output-format"
        >
          {OUTPUT_FORMATS.map((format) => (
            <option key={format} value={format}>
              {OUTPUT_FORMAT_LABELS[format]}
            </option>
          ))}
        </select>
      </div>

      {/* Model allowlist --------------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.allowlist-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Model allowlist</span>
          <span className="muted small">
            Comma- or newline-separated CLI model names (at least one required).
          </span>
        </div>
        <textarea
          className="settings-row__control"
          rows={3}
          value={draft.allowlistText}
          onChange={(event) => setField("allowlistText", event.target.value)}
          aria-label="Model allowlist"
          data-testid="cli-bridge-config.allowlist"
        />
      </div>
      {!allowlistNonEmpty ? (
        <p className="settings-note" data-testid="cli-bridge-config.allowlist-warning">
          At least one allowlisted model is required.
        </p>
      ) : null}

      {/* Timeout ----------------------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.timeout-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Timeout (seconds)</span>
          <span className="muted small">Per-spawn hard timeout; must be greater than 0.</span>
        </div>
        <input
          type="number"
          min={1}
          className="settings-row__control"
          value={draft.timeoutSeconds}
          onChange={(event) => setField("timeoutSeconds", event.target.value)}
          aria-label="Timeout in seconds"
          data-testid="cli-bridge-config.timeout"
        />
      </div>
      {!timeoutValid ? (
        <p className="settings-note" data-testid="cli-bridge-config.timeout-warning">
          Timeout must be a whole number greater than 0.
        </p>
      ) : null}

      {/* Working dir (optional) ------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.working-dir-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Working directory (optional)</span>
        </div>
        <input
          type="text"
          className="settings-row__control"
          value={draft.workingDir}
          onChange={(event) => setField("workingDir", event.target.value)}
          aria-label="Working directory (optional)"
          data-testid="cli-bridge-config.working-dir"
        />
      </div>

      {/* Env vars (optional) ---------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.env-row">
        <div className="settings-row__main">
          <span className="settings-row__label">Environment variables (optional)</span>
          <span className="muted small">
            Non-secret only — secret-bearing names are stripped at spawn. Stored
            values are not re-displayed on reload; re-enter a value to change it.
          </span>
        </div>
        <button
          type="button"
          className="secondary settings-row__control"
          onClick={handleAddEnvVar}
          data-testid="cli-bridge-config.env.add"
        >
          Add variable
        </button>
      </div>
      {draft.envVars.map((row, index) => (
        <div
          className="settings-row"
          key={index}
          data-testid={`cli-bridge-config.env.row.${index}`}
        >
          <input
            type="text"
            className="settings-row__control"
            value={row.key}
            onChange={(event) => handleEnvVarChange(index, "key", event.target.value)}
            placeholder="NAME"
            aria-label={`Env var name ${index + 1}`}
            data-testid={`cli-bridge-config.env.row.${index}.key`}
          />
          <input
            type="text"
            className="settings-row__control"
            value={row.value}
            onChange={(event) => handleEnvVarChange(index, "value", event.target.value)}
            placeholder="value"
            aria-label={`Env var value ${index + 1}`}
            data-testid={`cli-bridge-config.env.row.${index}.value`}
          />
          <button
            type="button"
            className="secondary"
            onClick={() => handleRemoveEnvVar(index)}
            aria-label={`Remove env var ${index + 1}`}
            data-testid={`cli-bridge-config.env.row.${index}.remove`}
          >
            Remove
          </button>
        </div>
      ))}
      {secretEnvWarning.length > 0 ? (
        <p className="settings-note" data-testid="cli-bridge-config.env-warning">
          {secretEnvWarning.join(", ")} look like secret-bearing names and will be
          stripped at spawn.
        </p>
      ) : null}

      {/* Actions ----------------------------------------------------------- */}
      <div className="settings-row" data-testid="cli-bridge-config.actions">
        <button
          type="button"
          className="settings-row__control"
          onClick={() => void handleSave()}
          disabled={!canSave}
          data-testid="cli-bridge-config.save"
        >
          {saving ? "Saving…" : "Save configuration"}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void handleTest()}
          disabled={!exeNonEmpty || testing}
          data-testid="cli-bridge-config.test"
        >
          {testing ? "Testing…" : "Test configuration"}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void handleClear()}
          disabled={!configured}
          data-testid="cli-bridge-config.clear"
        >
          Clear
        </button>
      </div>

      {/* Test receipt ------------------------------------------------------ */}
      {testReceipt ? (
        <p
          className="settings-note"
          role="status"
          data-testid="cli-bridge-config.test-receipt"
          data-test-ok={testReceipt.ok ? "true" : "false"}
          style={{ color: testReceipt.ok ? "var(--ok, #1a7f37)" : "var(--err, #b42318)" }}
        >
          {testReceipt.ok
            ? `OK — ${testReceipt.versionLine ?? testReceipt.detail}`
            : `Failed — ${testReceipt.detail}`}
        </p>
      ) : null}
    </div>
  );
}
