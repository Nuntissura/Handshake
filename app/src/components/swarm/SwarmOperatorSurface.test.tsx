import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";

// The integrated Terminal drawer is now mounted in this surface. TerminalView is
// the ONLY module that touches xterm (needs a real canvas/matchMedia, absent
// under jsdom), so stub it with a marker — the same boundary the vitest suite
// uses elsewhere. The panel's tab/grouping/gating logic is the real component.
vi.mock("../terminal/TerminalView", () => ({
  TerminalView: ({ sessionId, readOnly }: { sessionId: string; readOnly: boolean }) => (
    <div data-testid={`fake-term-${sessionId}`} data-readonly={readOnly ? "true" : "false"} />
  ),
}));

// Mock only the IPC CALLS of the swarm runtime module; keep the real helpers
// (BOARD_COLUMNS / cardKey / eventTargetState / types) that the board renders.
vi.mock("../../lib/ipc/swarm_runtime", async () => {
  const actual = await vi.importActual<typeof import("../../lib/ipc/swarm_runtime")>(
    "../../lib/ipc/swarm_runtime",
  );
  return {
    ...actual,
    resourceSnapshot: vi.fn(async () => ({
      concurrencyCap: 4,
      concurrencyInUse: 0,
      concurrencyAvailable: 4,
      liveSessions: 0,
      lifetimeSpawnsRemaining: 100,
      tokensRemaining: null,
      costMicrosRemaining: null,
      budgetExhausted: false,
    })),
    listActiveSessions: vi.fn(async () => []),
    boardSnapshot: vi.fn(async () => ({ cards: [], liveSessions: 0 })),
    subscribeBoardEvents: vi.fn(async () => () => {}),
    spawnSession: vi.fn(),
    cancelSession: vi.fn(),
    chatGenerate: vi.fn(),
  };
});

import { SwarmOperatorSurface } from "./SwarmOperatorSurface";
import { boardSnapshot } from "../../lib/ipc/swarm_runtime";

describe("SwarmOperatorSurface", () => {
  test("the Swarm Board is behind a disclosure COLLAPSED by default and not everything-at-once", () => {
    render(<SwarmOperatorSurface />);
    // Five labelled collapsible sections (not a dense wall).
    expect(screen.getByTestId("disclosure-swarm-board-toggle")).toBeInTheDocument();
    expect(screen.getByTestId("disclosure-resource-budget-toggle")).toBeInTheDocument();
    expect(screen.getByTestId("disclosure-spawn-session-toggle")).toBeInTheDocument();
    expect(screen.getByTestId("disclosure-live-sessions-toggle")).toBeInTheDocument();
    expect(screen.getByTestId("disclosure-operator-chat-toggle")).toBeInTheDocument();

    // Board collapsed by default.
    const boardToggle = screen.getByTestId("disclosure-swarm-board-toggle");
    expect(boardToggle).toHaveAttribute("aria-expanded", "false");
  });

  test("the heavy board (and its live subscription) does NOT mount while collapsed", () => {
    const boardSnapshotMock = vi.mocked(boardSnapshot);
    boardSnapshotMock.mockClear();
    render(<SwarmOperatorSurface />);
    // Lazy: the board component is absent and boardSnapshot was never called.
    expect(screen.queryByTestId("swarm-board")).toBeNull();
    expect(boardSnapshotMock).not.toHaveBeenCalled();
  });

  test("opening the board disclosure lazy-mounts the live board", async () => {
    render(<SwarmOperatorSurface />);
    fireEvent.click(screen.getByTestId("disclosure-swarm-board-toggle"));
    expect(screen.getByTestId("disclosure-swarm-board-toggle")).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    expect(await screen.findByTestId("swarm-board")).toBeInTheDocument();
  });

  test("the off-main-window Terminal drawer is mounted (reachable) and collapsed by default", () => {
    // HIGH-defect regression guard: the core 'inspect all background work'
    // surface must actually be present in the operator surface (not unmounted /
    // import-only), hosted by its own collapsed-by-default Disclosure.
    render(<SwarmOperatorSurface />);
    const terminalToggle = screen.getByTestId("disclosure-terminal-toggle");
    expect(terminalToggle).toBeInTheDocument();
    expect(terminalToggle).toHaveAttribute("aria-expanded", "false");
    // Lazy + collapsed: the body is not mounted yet.
    expect(screen.queryByTestId("terminal-panel-body")).toBeNull();
  });

  test("clicking a board lane's 'Inspect terminal' force-opens the Terminal drawer", async () => {
    // HIGH-defect regression guard: the board affordance must actually reveal the
    // off-main-window panel (onInspectTerminal is wired, not undefined). We seed a
    // swarm card AND a captured terminal session for that swarm so the affordance
    // is enabled, then assert the click opens the previously-collapsed drawer.
    const swarmRuntime = await import("../../lib/ipc/swarm_runtime");
    vi.mocked(swarmRuntime.boardSnapshot).mockResolvedValueOnce({
      liveSessions: 1,
      cards: [
        {
          instanceId: { composite: "m#0", modelId: "modelxyz", instance: 0 },
          swarmId: "swarm-A",
          worktreeId: null,
          provider: "cloud",
          runtimeBinding: "cloud",
          state: "GENERATING",
        },
      ],
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);

    const terminalIpc = await import("../../lib/ipc/terminal");
    const listSpy = vi
      .spyOn(terminalIpc.defaultTerminalIpc, "listSessions")
      .mockResolvedValue([
        {
          sessionId: "cap-1",
          sessionType: "AiJob",
          swarmId: "swarm-A",
          worktreeId: null,
          instanceId: "inst-1",
          title: "captured cloud cli",
          exited: false,
          exitCode: null,
          interactiveAllowed: false,
        },
      ]);

    render(<SwarmOperatorSurface />);

    // Terminal drawer starts collapsed.
    expect(screen.getByTestId("disclosure-terminal-toggle")).toHaveAttribute("aria-expanded", "false");

    // Open the board, wait for the enabled inspect affordance, click it.
    fireEvent.click(screen.getByTestId("disclosure-swarm-board-toggle"));
    const inspectBtn = await screen.findByTestId("swarm-inspect-terminal-swarm-A");
    await vi.waitFor(() => expect(inspectBtn).not.toBeDisabled());
    fireEvent.click(inspectBtn);

    // The previously-collapsed off-main-window drawer is now open + body mounted,
    // and it focuses the captured session of swarm-A.
    await vi.waitFor(() =>
      expect(screen.getByTestId("disclosure-terminal-toggle")).toHaveAttribute("aria-expanded", "true"),
    );
    expect(await screen.findByTestId("terminal-panel-body")).toBeInTheDocument();
    await vi.waitFor(() =>
      expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "cap-1"),
    );

    listSpy.mockRestore();
  });
});
