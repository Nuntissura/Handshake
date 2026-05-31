import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";

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
});
