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
      committedMemoryBytesRemaining: null,
      committedMemoryBytesCap: null,
      budgetExhausted: false,
    })),
    listActiveSessions: vi.fn(async () => []),
    listWorktrees: vi.fn(async () => []),
    boardSnapshot: vi.fn(async () => ({ cards: [], liveSessions: 0 })),
    subscribeBoardEvents: vi.fn(async () => () => {}),
    spawnSession: vi.fn(),
    cancelSession: vi.fn(),
    chatGenerate: vi.fn(),
    // ROI #3: the edit-then-resume path reads the stored template; default to a
    // populated template so the prefill test can assert the form is filled.
    getSpawnTemplate: vi.fn(async () => ({
      provider: "byok_cloud",
      cloudModelName: "claude-sonnet-4",
      worktreeId: "wt-recovery-1",
      workingDir: "D:/work/wt-recovery-1",
      isolationTier: "tier3_microvm",
      originSessionId: "beta-cloud#0",
      capturedAt: "2026-05-30T10:00:00.000Z",
    })),
  };
});

import { SwarmOperatorSurface } from "./SwarmOperatorSurface";
import {
  boardSnapshot,
  getSpawnTemplate,
  listActiveSessions,
  resourceSnapshot,
} from "../../lib/ipc/swarm_runtime";

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

  test("resource badge treats committed-memory exhaustion as local-only", async () => {
    vi.mocked(resourceSnapshot).mockResolvedValueOnce({
      concurrencyCap: 4,
      concurrencyInUse: 0,
      concurrencyAvailable: 4,
      liveSessions: 0,
      lifetimeSpawnsRemaining: 100,
      tokensRemaining: null,
      costMicrosRemaining: null,
      committedMemoryBytesRemaining: 0,
      committedMemoryBytesCap: 16 * 1024 * 1024 * 1024,
      budgetExhausted: true,
    });

    render(<SwarmOperatorSurface />);

    expect(await screen.findByTestId("disclosure-resource-budget-toggle")).toHaveTextContent(
      /local memory exhausted/i,
    );
    expect(await screen.findByTestId("swarm-stat-exhausted")).toHaveTextContent(
      /cloud lanes remain available/i,
    );
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

  test("the Session workbench chat picker lists local + cloud + CLI sessions (local-only filter is gone)", async () => {
    // Governance glue #3: the chat picker must offer ALL providers, not just
    // local. Seed one local, one byok_cloud, one official_cli session (all
    // READY), open the Session disclosure, and assert each provider's option is
    // present and tagged via data-provider.
    vi.mocked(listActiveSessions).mockResolvedValue([
      {
        instanceId: { modelId: "alpha-model", instance: 0, composite: "alpha-model#0" },
        state: "READY",
        provider: "local",
        runtimeBinding: "candle",
        artifactPath: "D:/models/alpha/model.safetensors",
        cloudModelName: null,
        worktreeId: null,
        workingDir: null,
      },
      {
        instanceId: { modelId: "beta-cloud", instance: 0, composite: "beta-cloud#0" },
        state: "READY",
        provider: "byok_cloud",
        runtimeBinding: "cloud",
        artifactPath: null,
        cloudModelName: "claude-sonnet-4",
        worktreeId: null,
        workingDir: null,
      },
      {
        instanceId: { modelId: "gamma-cli", instance: 0, composite: "gamma-cli#0" },
        state: "READY",
        provider: "official_cli",
        runtimeBinding: "cloud",
        artifactPath: null,
        cloudModelName: "claude-code",
        worktreeId: null,
        workingDir: null,
      },
    ]);

    render(<SwarmOperatorSurface />);
    // Open the "Session" workbench disclosure (id kept stable as operator-chat).
    fireEvent.click(screen.getByTestId("disclosure-operator-chat-toggle"));

    const optLocal = await screen.findByTestId("operator-chat-option-alpha-model#0");
    expect(optLocal).toHaveAttribute("data-provider", "local");
    expect(screen.getByTestId("operator-chat-option-beta-cloud#0")).toHaveAttribute(
      "data-provider",
      "byok_cloud",
    );
    expect(screen.getByTestId("operator-chat-option-gamma-cli#0")).toHaveAttribute(
      "data-provider",
      "official_cli",
    );
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

  test("workbench Resume reads the template, PREFILLS the Spawn form, and force-opens the Spawn disclosure (edit-then-resume)", async () => {
    // ROI #3 STATE RECOVERY (edit-then-resume): seed one live cloud session, open
    // the Session workbench, select it via Chat (sets chatInstanceId), then click
    // "Resume this session". The host reads its template (mocked) and prefills the
    // existing Spawn form so the operator can tweak before re-spawning — reusing
    // the validated spawn path, no new spawn logic.
    vi.mocked(getSpawnTemplate).mockClear();
    vi.mocked(listActiveSessions).mockResolvedValue([
      {
        instanceId: { modelId: "beta-cloud", instance: 0, composite: "beta-cloud#0" },
        state: "READY",
        provider: "byok_cloud",
        runtimeBinding: "cloud",
        artifactPath: null,
        cloudModelName: "claude-sonnet-4",
        worktreeId: null,
        workingDir: null,
      },
    ]);

    render(<SwarmOperatorSurface />);
    // Open the Session workbench and select the session as the chat session
    // (the picker is a <select>; selecting fires its onChange -> setChatInstanceId).
    fireEvent.click(screen.getByTestId("disclosure-operator-chat-toggle"));
    await screen.findByTestId("operator-chat-option-beta-cloud#0");
    fireEvent.change(screen.getByTestId("operator-chat-session"), {
      target: { value: "beta-cloud#0" },
    });

    // Click Resume -> the host reads the stored template for this session.
    const resumeBtn = await screen.findByTestId("session-workbench-resume");
    await vi.waitFor(() => expect(resumeBtn).not.toBeDisabled());
    fireEvent.click(resumeBtn);
    await vi.waitFor(() => expect(getSpawnTemplate).toHaveBeenCalledWith("beta-cloud#0"));

    // The Spawn disclosure force-opens and the form is prefilled from the template
    // (provider = byok_cloud, cloud model = claude-sonnet-4).
    await vi.waitFor(() =>
      expect(screen.getByTestId("disclosure-spawn-session-toggle")).toHaveAttribute("aria-expanded", "true"),
    );
    await vi.waitFor(() =>
      expect((screen.getByTestId("swarm-spawn-cloud-model") as HTMLInputElement).value).toBe("claude-sonnet-4"),
    );
    expect((screen.getByTestId("swarm-spawn-provider") as HTMLSelectElement).value).toBe("byok_cloud");
    // The recorded worktree is threaded through the free-text new-worktree entry.
    expect((screen.getByTestId("swarm-spawn-worktree-new") as HTMLInputElement).value).toBe("wt-recovery-1");
    expect((screen.getByTestId("swarm-spawn-working-dir") as HTMLInputElement).value).toBe("D:/work/wt-recovery-1");
  });
});
