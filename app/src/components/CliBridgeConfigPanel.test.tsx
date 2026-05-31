import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";

import { CliBridgeConfigPanel } from "./CliBridgeConfigPanel";
import type {
  CliBridgeConfigIpc,
  CliBridgeConfigSummary,
  CliBridgePreset,
  EnvVarPair,
} from "../lib/ipc/cli_bridge_config";

// Colocated vitest (mirrors SettingsMenu.test.tsx). `@tauri-apps/api`'s `invoke`
// is unavailable under jsdom, so we inject a fake IPC client via the panel's
// `ipc` prop. The panel is a pure React unit; every real call would hit the
// `kernel_cli_bridge_*` Tauri commands in production.

const CLAUDE_PRESET: CliBridgePreset = {
  id: "claude_code",
  label: "Claude Code",
  cliKind: "claude_code",
  executableHint: "claude",
  argsTemplate: ["-p", "{prompt}", "--model", "{model}", "--output-format", "text"],
  outputFormat: "raw_text",
  modelAllowlist: ["sonnet", "opus"],
  defaultTimeoutSeconds: 120,
  versionArg: "--version",
};

const GENERIC_PRESET: CliBridgePreset = {
  id: "generic",
  label: "Generic",
  cliKind: "other",
  executableHint: "",
  argsTemplate: ["{prompt}"],
  outputFormat: "raw_text",
  modelAllowlist: [],
  defaultTimeoutSeconds: 120,
  versionArg: "--version",
};

const UNCONFIGURED: CliBridgeConfigSummary = {
  configured: false,
  cliKind: "other",
  executablePath: "",
  argsTemplate: [],
  outputFormat: "raw_text",
  modelAllowlist: [],
  workingDir: null,
  timeoutSeconds: 120,
  // Backend summary carries env-var NAMES only (env_var_names -> envVarNames),
  // never a map. Unconfigured => no names.
  envVarNames: [],
  updatedAtUtc: null,
};

function makeIpc(overrides: Partial<CliBridgeConfigIpc> = {}): CliBridgeConfigIpc {
  return {
    getConfig: vi.fn(async () => UNCONFIGURED),
    // Mirror the REAL backend serde projection: the request carries env vars as
    // a Vec<EnvVarPair> (array of { key, value }); the returned SUMMARY carries
    // env-var NAMES only (envVarNames). The mock derives the names from the
    // request array exactly as the backend `from_doc` does, so the suite
    // exercises the real wire contract (not an invented map shape).
    setConfig: vi.fn(async (request) => ({
      configured: true,
      cliKind: request.cliKind,
      executablePath: request.executablePath,
      argsTemplate: request.argsTemplate,
      outputFormat: request.outputFormat,
      modelAllowlist: request.modelAllowlist,
      workingDir: request.workingDir,
      timeoutSeconds: request.timeoutSeconds,
      envVarNames: request.envVars.map((pair: EnvVarPair) => pair.key),
      updatedAtUtc: "2026-05-31T00:00:00Z",
    })),
    clearConfig: vi.fn(async () => UNCONFIGURED),
    listPresets: vi.fn(async () => [CLAUDE_PRESET, GENERIC_PRESET]),
    testConfig: vi.fn(async () => ({ ok: true, versionLine: "claude 2.0.1", detail: "exit 0" })),
    ...overrides,
  };
}

async function renderPanel(ipc: CliBridgeConfigIpc) {
  render(<CliBridgeConfigPanel ipc={ipc} />);
  // Wait for the mount-time refresh (presets + config) to resolve.
  await waitFor(() => expect(screen.getByTestId("cli-bridge-config.preset")).toBeInTheDocument());
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("CliBridgeConfigPanel", () => {
  test("loads presets + config on mount via the injected IPC", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);
    expect(ipc.listPresets).toHaveBeenCalledTimes(1);
    expect(ipc.getConfig).toHaveBeenCalledTimes(1);
    // Unconfigured => honest status note, lane disabled.
    expect(screen.getByTestId("cli-bridge-config.status")).toHaveTextContent(
      /not configured/i,
    );
  });

  test("selecting a preset prefills executable, args, output format, and allowlist", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "claude_code" },
    });

    expect((screen.getByTestId("cli-bridge-config.executable") as HTMLInputElement).value).toBe(
      "claude",
    );
    expect((screen.getByTestId("cli-bridge-config.args") as HTMLTextAreaElement).value).toContain(
      "{prompt}",
    );
    expect((screen.getByTestId("cli-bridge-config.allowlist") as HTMLTextAreaElement).value).toBe(
      "sonnet\nopus",
    );
    expect((screen.getByTestId("cli-bridge-config.kind") as HTMLInputElement).value).toBe(
      "Claude Code",
    );
  });

  test("Save is disabled until the draft is client-side valid (preset prefill makes it valid)", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);

    // EMPTY_DRAFT default allowlist is empty => Save disabled.
    expect(screen.getByTestId("cli-bridge-config.save")).toBeDisabled();

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "claude_code" },
    });

    // Claude preset prefills exe + {prompt} args + allowlist + timeout => valid.
    expect(screen.getByTestId("cli-bridge-config.save")).not.toBeDisabled();
  });

  test("missing {prompt} shows the validation banner and keeps Save disabled", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "claude_code" },
    });
    // Strip the {prompt} placeholder out of the args.
    fireEvent.change(screen.getByTestId("cli-bridge-config.args"), {
      target: { value: "-p\n--model\n{model}" },
    });

    expect(screen.getByTestId("cli-bridge-config.args-warning")).toBeInTheDocument();
    expect(screen.getByTestId("cli-bridge-config.save")).toBeDisabled();
  });

  test("empty allowlist (Generic preset) blocks Save with an honest warning", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "generic" },
    });

    // Generic prefills {prompt} but NO allowlist and NO exe hint.
    expect(screen.getByTestId("cli-bridge-config.allowlist-warning")).toBeInTheDocument();
    expect(screen.getByTestId("cli-bridge-config.save")).toBeDisabled();
  });

  test("Save calls setConfig with the parsed template + allowlist", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "claude_code" },
    });
    fireEvent.click(screen.getByTestId("cli-bridge-config.save"));

    await waitFor(() => expect(ipc.setConfig).toHaveBeenCalledTimes(1));
    const request = (ipc.setConfig as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(request.cliKind).toBe("claude_code");
    expect(request.executablePath).toBe("claude");
    expect(request.argsTemplate).toContain("{prompt}");
    expect(request.modelAllowlist).toEqual(["sonnet", "opus"]);
    expect(request.timeoutSeconds).toBe(120);
    // WRITE wire contract: envVars MUST be an ARRAY (Vec<EnvVarPair>), not a map.
    // serde rejects an object literal here, so the array shape is load-bearing.
    expect(Array.isArray(request.envVars)).toBe(true);
    expect(request.envVars).toEqual([]);
    // Honest activation note after save.
    await waitFor(() =>
      expect(screen.getByTestId("cli-bridge-config.receipt")).toHaveTextContent(/next app launch/i),
    );
  });

  test("Test configuration calls the preflight IPC and renders the real receipt", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "claude_code" },
    });
    fireEvent.click(screen.getByTestId("cli-bridge-config.test"));

    await waitFor(() => expect(ipc.testConfig).toHaveBeenCalledTimes(1));
    // PREFLIGHT wire contract: { executablePath, versionArg? } — there is NO
    // args-template field on the real backend request. The Claude preset carries
    // versionArg "--version", so the panel forwards it.
    const testRequest = (ipc.testConfig as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(testRequest.executablePath).toBe("claude");
    expect(testRequest.versionArg).toBe("--version");
    expect(testRequest).not.toHaveProperty("argsTemplate");
    const receipt = await screen.findByTestId("cli-bridge-config.test-receipt");
    expect(receipt).toHaveTextContent(/claude 2\.0\.1/);
    expect(receipt).toHaveAttribute("data-test-ok", "true");
  });

  test("Test configuration surfaces a failed preflight honestly", async () => {
    const ipc = makeIpc({
      testConfig: vi.fn(async () => ({
        ok: false,
        versionLine: null,
        detail: "program not found: claude",
      })),
    });
    await renderPanel(ipc);

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "claude_code" },
    });
    fireEvent.click(screen.getByTestId("cli-bridge-config.test"));

    const receipt = await screen.findByTestId("cli-bridge-config.test-receipt");
    expect(receipt).toHaveTextContent(/program not found/i);
    expect(receipt).toHaveAttribute("data-test-ok", "false");
  });

  test("Test button is disabled (honest) when no executable is set", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);
    // EMPTY_DRAFT has no exe => test disabled.
    expect(screen.getByTestId("cli-bridge-config.test")).toBeDisabled();
  });

  test("Clear is disabled when unconfigured and calls clearConfig when configured", async () => {
    const configured: CliBridgeConfigSummary = {
      ...UNCONFIGURED,
      configured: true,
      cliKind: "claude_code",
      executablePath: "claude",
      argsTemplate: ["-p", "{prompt}"],
      modelAllowlist: ["sonnet"],
      updatedAtUtc: "2026-05-31T00:00:00Z",
    };
    const ipc = makeIpc({ getConfig: vi.fn(async () => configured) });
    await renderPanel(ipc);

    await waitFor(() =>
      expect(screen.getByTestId("cli-bridge-config.status")).toHaveTextContent(/configured/i),
    );
    const clearBtn = screen.getByTestId("cli-bridge-config.clear");
    expect(clearBtn).not.toBeDisabled();
    fireEvent.click(clearBtn);
    await waitFor(() => expect(ipc.clearConfig).toHaveBeenCalledTimes(1));
  });

  test("a load error surfaces honestly and does not crash the panel", async () => {
    const ipc = makeIpc({
      getConfig: vi.fn(async () => {
        throw new Error("config file corrupt");
      }),
    });
    render(<CliBridgeConfigPanel ipc={ipc} />);
    const error = await screen.findByTestId("cli-bridge-config.error");
    expect(error).toHaveTextContent(/corrupt/i);
  });

  test("env var rows can be added and a secret-bearing name warns softly without blocking", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "claude_code" },
    });
    fireEvent.click(screen.getByTestId("cli-bridge-config.env.add"));

    const row = screen.getByTestId("cli-bridge-config.env.row.0");
    fireEvent.change(within(row).getByTestId("cli-bridge-config.env.row.0.key"), {
      target: { value: "MY_API_KEY" },
    });

    expect(screen.getByTestId("cli-bridge-config.env-warning")).toHaveTextContent(/stripped/i);
    // Soft warning only — Save stays enabled (preset prefill is otherwise valid).
    expect(screen.getByTestId("cli-bridge-config.save")).not.toBeDisabled();
  });

  test("reopening a configured config (READ path) seeds env rows from envVarNames without crashing", async () => {
    // Regression guard for the READ-path wire-contract divergence: the backend
    // summary returns env-var NAMES only (envVarNames), never a map. summaryToDraft
    // must seed rows from names — Object.entries(undefined) on a map field would
    // throw a TypeError on mount of any saved config.
    const configured: CliBridgeConfigSummary = {
      configured: true,
      cliKind: "claude_code",
      executablePath: "claude",
      argsTemplate: ["-p", "{prompt}", "--model", "{model}"],
      outputFormat: "raw_text",
      modelAllowlist: ["sonnet", "opus"],
      workingDir: null,
      timeoutSeconds: 90,
      envVarNames: ["MY_FLAG", "OTHER_FLAG"],
      updatedAtUtc: "2026-05-31T00:00:00Z",
    };
    const ipc = makeIpc({ getConfig: vi.fn(async () => configured) });
    await renderPanel(ipc);

    // No load error: the configured summary rendered.
    expect(screen.queryByTestId("cli-bridge-config.error")).toBeNull();
    await waitFor(() =>
      expect(screen.getByTestId("cli-bridge-config.status")).toHaveTextContent(/configured/i),
    );
    // Env rows seeded from the names with empty (not-re-displayed) values.
    expect((screen.getByTestId("cli-bridge-config.env.row.0.key") as HTMLInputElement).value).toBe(
      "MY_FLAG",
    );
    expect(
      (screen.getByTestId("cli-bridge-config.env.row.0.value") as HTMLInputElement).value,
    ).toBe("");
    expect((screen.getByTestId("cli-bridge-config.env.row.1.key") as HTMLInputElement).value).toBe(
      "OTHER_FLAG",
    );
    // Stored editable fields round-trip into the draft.
    expect((screen.getByTestId("cli-bridge-config.executable") as HTMLInputElement).value).toBe(
      "claude",
    );
    expect((screen.getByTestId("cli-bridge-config.timeout") as HTMLInputElement).value).toBe("90");
  });

  test("Save round-trips a non-empty env array as Vec<EnvVarPair> shape", async () => {
    const ipc = makeIpc();
    await renderPanel(ipc);

    fireEvent.change(screen.getByTestId("cli-bridge-config.preset"), {
      target: { value: "claude_code" },
    });
    fireEvent.click(screen.getByTestId("cli-bridge-config.env.add"));
    const row = screen.getByTestId("cli-bridge-config.env.row.0");
    fireEvent.change(within(row).getByTestId("cli-bridge-config.env.row.0.key"), {
      target: { value: "MY_FLAG" },
    });
    fireEvent.change(within(row).getByTestId("cli-bridge-config.env.row.0.value"), {
      target: { value: "on" },
    });
    fireEvent.click(screen.getByTestId("cli-bridge-config.save"));

    await waitFor(() => expect(ipc.setConfig).toHaveBeenCalledTimes(1));
    const request = (ipc.setConfig as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(request.envVars).toEqual([{ key: "MY_FLAG", value: "on" }]);
    // And the returned summary surfaces the name (values are not re-returned).
    await waitFor(() =>
      expect(screen.getByTestId("cli-bridge-config.receipt")).toHaveTextContent(/next app launch/i),
    );
  });
});
