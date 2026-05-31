import { render, screen, fireEvent } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";
import type { ComponentProps } from "react";

import { SettingsMenu } from "./SettingsMenu";

afterEach(() => {
  localStorage.clear();
});

function renderMenu(overrides: Partial<ComponentProps<typeof SettingsMenu>> = {}) {
  const onClose = vi.fn();
  const onViewModeChange = vi.fn();
  render(
    <SettingsMenu
      isOpen
      onClose={onClose}
      viewMode="SFW"
      onViewModeChange={onViewModeChange}
      {...overrides}
    />,
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

  test("opens as an accessible dialog with the global settings sections", () => {
    renderMenu();
    const dialog = screen.getByTestId("settings-menu");
    expect(dialog).toHaveAttribute("role", "dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(screen.getByText("Appearance")).toBeInTheDocument();
    expect(screen.getByText("Swarm")).toBeInTheDocument();
    expect(screen.getByText("Layout")).toBeInTheDocument();
    expect(screen.getByText("About")).toBeInTheDocument();
  });

  test("the board-default-open toggle persists honestly to localStorage", () => {
    renderMenu();
    const toggle = screen.getByTestId(
      "setting-swarm-board-default-open.control",
    ) as HTMLInputElement;
    expect(toggle.checked).toBe(false); // collapsed-by-default
    fireEvent.click(toggle);
    expect(toggle.checked).toBe(true);
    expect(localStorage.getItem("handshake.swarm.board_default_open")).toBe("true");
  });

  test("not-yet-wired settings render as DISABLED (no fake-working controls)", () => {
    renderMenu();
    const notWired = screen.getAllByLabelText(/not yet wired/i);
    expect(notWired.length).toBeGreaterThanOrEqual(2);
    for (const control of notWired) {
      expect(control).toBeDisabled();
    }
  });

  test("Reset layout is disabled (not a dead no-op) when no handler is provided", () => {
    renderMenu();
    expect(screen.getByTestId("setting-reset-layout.control")).toBeDisabled();
  });

  test("close button invokes onClose", () => {
    const { onClose } = renderMenu();
    fireEvent.click(screen.getByTestId("settings-menu.close"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
