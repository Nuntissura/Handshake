import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";
import type { ComponentProps } from "react";

import { SettingsMenu } from "./SettingsMenu";
import type {
  CliBridgeConfigIpc,
  CliBridgeConfigSummary,
} from "../lib/ipc/cli_bridge_config";

afterEach(() => {
  localStorage.clear();
});

const UNCONFIGURED_SUMMARY: CliBridgeConfigSummary = {
  configured: false,
  cliKind: "other",
  executablePath: "",
  argsTemplate: [],
  outputFormat: "raw_text",
  modelAllowlist: [],
  workingDir: null,
  timeoutSeconds: 120,
  envVarNames: [],
  updatedAtUtc: null,
};

const WORKSPACE_SETTINGS = {
  schema_id: "hsk.workspace_settings_state@1",
  theme: "light",
  custom_theme_tokens: {},
  keybindings: {
    "app.quick_switcher.open": "Mod-p",
    "app.command_palette.open": "Mod-Shift-p",
  },
  settings: {
    view_mode: "SFW",
    swarm_board_default_open: false,
  },
};

// Controlled CLI-bridge IPC mock so the embedded panel mounts without
// `@tauri-apps/api`'s `invoke` rejecting under jsdom (which would flood every
// SettingsMenu test with un-acted async-state warnings). Resolves immediately
// with an honest unconfigured summary + empty preset list.
function makeCliBridgeIpc(): CliBridgeConfigIpc {
  return {
    getConfig: vi.fn(async () => UNCONFIGURED_SUMMARY),
    setConfig: vi.fn(async () => UNCONFIGURED_SUMMARY),
    clearConfig: vi.fn(async () => UNCONFIGURED_SUMMARY),
    listPresets: vi.fn(async () => []),
    testConfig: vi.fn(async () => ({ ok: true, versionLine: null, detail: "exit 0" })),
  };
}

async function renderMenu(overrides: Partial<ComponentProps<typeof SettingsMenu>> = {}) {
  const onClose = vi.fn();
  const onViewModeChange = vi.fn();
  render(
    <SettingsMenu
      isOpen
      onClose={onClose}
      viewMode="SFW"
      onViewModeChange={onViewModeChange}
      cliBridgeIpc={makeCliBridgeIpc()}
      {...overrides}
    />,
  );
  // Flush the embedded CLI-bridge panel's mount-time refresh inside act() so the
  // resolving mock IPC does not land an un-acted state update after the test
  // body. The panel renders its honest unconfigured status once settled.
  await waitFor(() =>
    expect(screen.getByTestId("cli-bridge-config.status")).toHaveTextContent(/not configured/i),
  );
  return { onClose, onViewModeChange };
}

describe("SettingsMenu", () => {
  test("renders nothing when closed", () => {
    render(
      <SettingsMenu isOpen={false} onClose={() => {}} viewMode="SFW" onViewModeChange={() => {}} />,
    );
    expect(screen.queryByTestId("settings-menu")).toBeNull();
  });

  test("opens as an accessible dialog with the global settings sections", async () => {
    await renderMenu();
    const dialog = screen.getByTestId("settings-menu");
    expect(dialog).toHaveAttribute("role", "dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(screen.getByText("Appearance")).toBeInTheDocument();
    expect(screen.getByText("Swarm")).toBeInTheDocument();
    expect(screen.getByText("Layout")).toBeInTheDocument();
    expect(screen.getByText("About")).toBeInTheDocument();
  });

  test("mounts the CLI Bridge config section", async () => {
    await renderMenu();
    expect(screen.getByTestId("settings-section-cli-bridge")).toBeInTheDocument();
    // The panel mounts its own surface, loading config/presets via the injected
    // mock IPC (renderMenu already flushed the mount-time refresh inside act).
    expect(screen.getByTestId("cli-bridge-config")).toBeInTheDocument();
    expect(screen.getByTestId("cli-bridge-config.status")).toHaveTextContent(/not configured/i);
  });

  test("the board-default-open toggle persists honestly to localStorage", async () => {
    await renderMenu();
    const toggle = screen.getByTestId(
      "setting-swarm-board-default-open.control",
    ) as HTMLInputElement;
    expect(toggle.checked).toBe(false); // collapsed-by-default
    fireEvent.click(toggle);
    expect(toggle.checked).toBe(true);
    expect(localStorage.getItem("handshake.swarm.board_default_open")).toBe("true");
  });

  test("not-yet-wired settings render as DISABLED (no fake-working controls)", async () => {
    await renderMenu();
    const notWired = screen.getAllByLabelText(/not yet wired/i);
    expect(notWired.length).toBeGreaterThanOrEqual(2);
    for (const control of notWired) {
      expect(control).toBeDisabled();
    }
  });

  test("search filters real workspace settings and keybindings", async () => {
    await renderMenu({
      workspaceSettings: WORKSPACE_SETTINGS,
      onWorkspaceSettingsChange: vi.fn(),
    } as Partial<ComponentProps<typeof SettingsMenu>>);

    const search = screen.getByTestId("settings-menu.search");
    expect(screen.getByTestId("setting-theme")).toBeInTheDocument();
    expect(screen.getByTestId("setting-keybinding-app.quick_switcher.open")).toBeInTheDocument();

    fireEvent.change(search, { target: { value: "quick" } });

    expect(screen.queryByTestId("setting-theme")).toBeNull();
    expect(screen.getByTestId("setting-keybinding-app.quick_switcher.open")).toBeInTheDocument();
  });

  test("theme selection emits a workspace settings update", async () => {
    const onWorkspaceSettingsChange = vi.fn();
    await renderMenu({
      workspaceSettings: WORKSPACE_SETTINGS,
      onWorkspaceSettingsChange,
    } as Partial<ComponentProps<typeof SettingsMenu>>);

    fireEvent.change(screen.getByTestId("setting-theme.control"), { target: { value: "dark" } });

    expect(onWorkspaceSettingsChange).toHaveBeenCalledWith(
      expect.objectContaining({
        theme: "dark",
        schema_id: "hsk.workspace_settings_state@1",
      }),
    );
  });

  test("duplicate app keybindings are blocked before they update workspace settings", async () => {
    const onWorkspaceSettingsChange = vi.fn();
    await renderMenu({
      workspaceSettings: WORKSPACE_SETTINGS,
      onWorkspaceSettingsChange,
    } as Partial<ComponentProps<typeof SettingsMenu>>);

    fireEvent.change(screen.getByTestId("setting-keybinding-app.quick_switcher.open.control"), {
      target: { value: "Mod-Shift-p" },
    });

    expect(screen.getByTestId("settings-keybinding-conflict")).toHaveTextContent(
      /Quick Switcher and Command Palette/i,
    );
    expect(onWorkspaceSettingsChange).not.toHaveBeenCalled();
  });

  test("keybinding reset restores the default chord through workspace settings", async () => {
    const onWorkspaceSettingsChange = vi.fn();
    await renderMenu({
      workspaceSettings: {
        ...WORKSPACE_SETTINGS,
        keybindings: {
          ...WORKSPACE_SETTINGS.keybindings,
          "app.quick_switcher.open": "Alt-q",
        },
      },
      onWorkspaceSettingsChange,
    } as Partial<ComponentProps<typeof SettingsMenu>>);

    fireEvent.click(screen.getByTestId("setting-keybinding-app.quick_switcher.open.reset"));

    expect(onWorkspaceSettingsChange).toHaveBeenCalledWith(
      expect.objectContaining({
        keybindings: expect.objectContaining({
          "app.quick_switcher.open": "Mod-p",
        }),
      }),
    );
  });

  test("Reset layout is disabled (not a dead no-op) when no handler is provided", async () => {
    await renderMenu();
    expect(screen.getByTestId("setting-reset-layout.control")).toBeDisabled();
  });

  test("close button invokes onClose", async () => {
    const { onClose } = await renderMenu();
    fireEvent.click(screen.getByTestId("settings-menu.close"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
