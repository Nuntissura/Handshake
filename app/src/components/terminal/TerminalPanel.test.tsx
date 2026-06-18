import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import { TerminalPanel } from "./TerminalPanel";
import type {
  ScrollbackSnapshot,
  TerminalIpc,
  TerminalSession,
} from "../../lib/ipc/terminal";

// TerminalPanel unit tests. Tauri `invoke` is unavailable under jsdom, so we
// inject a recording IPC stub and a fake `renderTerminal` that records the
// readOnly flag it was asked to render with. This lets us assert the panel's
// tab logic + TERM-INVARIANTS read-only/interact gating WITHOUT touching xterm
// (which needs a canvas the Playwright matrix provides instead).

function makeSession(over: Partial<TerminalSession>): TerminalSession {
  return {
    sessionId: "s1",
    sessionType: "HumanDev",
    swarmId: null,
    worktreeId: null,
    instanceId: null,
    title: "session one",
    exited: false,
    exitCode: null,
    interactiveAllowed: true,
    ...over,
  };
}

function makeIpc(sessions: TerminalSession[]): TerminalIpc {
  const snap: ScrollbackSnapshot = { sessionId: "s1", seq: 0, chunkBase64: "", truncated: false };
  return {
    getContext: vi.fn(async () => ({ cwd: "D:/resolved-root", defaultShell: null })),
    listSessions: vi.fn(async () => sessions),
    createSession: vi.fn(async () => sessions[0]),
    authorizeInteractive: vi.fn(async () => {}),
    writeStdin: vi.fn(async () => {}),
    resizeSession: vi.fn(async () => {}),
    closeSession: vi.fn(async () => {}),
    scrollback: vi.fn(async () => snap),
    subscribe: vi.fn(async () => () => {}),
  };
}

/** A fake terminal renderer that records the readOnly flag per render. */
function makeRenderRecorder() {
  const calls: { sessionId: string; readOnly: boolean }[] = [];
  const renderTerminal: NonNullable<ComponentProps<typeof TerminalPanel>["renderTerminal"]> = ({
    session,
    readOnly,
  }) => {
    calls.push({ sessionId: session.sessionId, readOnly });
    return (
      <div data-testid={`fake-term-${session.sessionId}`} data-readonly={readOnly ? "true" : "false"}>
        fake terminal {session.sessionId}
      </div>
    );
  };
  return { calls, renderTerminal };
}

describe("TerminalPanel", () => {
  it("is collapsed by default and lazy: the body is not mounted until opened", async () => {
    const ipc = makeIpc([makeSession({})]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);

    // Disclosure renders collapsed (data-open="false") and, being lazy, does not
    // mount the body -> listSessions is never called while collapsed.
    const disclosure = screen.getByTestId("terminal-panel");
    expect(disclosure).toHaveAttribute("data-open", "false");
    expect(screen.queryByTestId("terminal-panel-body")).not.toBeInTheDocument();
    expect(ipc.listSessions).not.toHaveBeenCalled();
  });

  it("mounts the body and lists sessions once opened", async () => {
    const ipc = makeIpc([makeSession({ title: "human shell" })]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);

    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    expect(await screen.findByTestId("terminal-panel-body")).toBeInTheDocument();
    await waitFor(() => expect(ipc.listSessions).toHaveBeenCalled());
    expect(screen.getByTestId("terminal-tab-s1")).toBeInTheDocument();
  });

  it("creates a new HumanDev terminal with the workspace root cwd and selects it", async () => {
    const created = makeSession({ sessionId: "s-new", title: "Terminal" });
    const ipc = makeIpc([makeSession({ sessionId: "s1", title: "human shell" })]);
    vi.mocked(ipc.createSession).mockResolvedValueOnce(created);
    const { renderTerminal } = makeRenderRecorder();
    render(
      <TerminalPanel
        ipc={ipc}
        renderTerminal={renderTerminal}
        defaultOpen
        workspaceRoot="D:/repo"
        defaultShell="pwsh"
      />,
    );

    await screen.findByTestId("terminal-panel-active");
    fireEvent.click(screen.getByTestId("terminal-new-session"));

    await waitFor(() => expect(ipc.createSession).toHaveBeenCalledWith({
      sessionType: "HumanDev",
      shell: "pwsh",
      cwd: "D:/repo",
      title: "Terminal",
    }));
    expect(await screen.findByTestId("terminal-tab-s-new")).toBeInTheDocument();
    expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "s-new");
  });

  it("creates a new HumanDev terminal with the IPC terminal context cwd when props are omitted", async () => {
    const created = makeSession({ sessionId: "s-new", title: "Terminal" });
    const ipc = makeIpc([makeSession({ sessionId: "s1", title: "human shell" })]);
    vi.mocked(ipc.createSession).mockResolvedValueOnce(created);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} defaultOpen />);

    await screen.findByTestId("terminal-panel-active");
    fireEvent.click(screen.getByTestId("terminal-new-session"));

    await waitFor(() => expect(ipc.getContext).toHaveBeenCalled());
    await waitFor(() => expect(ipc.createSession).toHaveBeenCalledWith({
      sessionType: "HumanDev",
      cwd: "D:/resolved-root",
      title: "Terminal",
    }));
    expect(await screen.findByTestId("terminal-tab-s-new")).toBeInTheDocument();
  });

  it("closes the active terminal tab through IPC and selects a remaining tab", async () => {
    const ipc = makeIpc([
      makeSession({ sessionId: "s1", title: "first" }),
      makeSession({ sessionId: "s2", title: "second" }),
    ]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} defaultOpen />);

    await screen.findByTestId("terminal-tab-s1");
    fireEvent.click(screen.getByTestId("terminal-tab-s2"));
    expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "s2");

    fireEvent.click(screen.getByTestId("terminal-close-session"));

    await waitFor(() => expect(ipc.closeSession).toHaveBeenCalledWith("s2"));
    expect(screen.queryByTestId("terminal-tab-s2")).not.toBeInTheDocument();
    expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "s1");
  });

  it("does not allow Close on captured non-HumanDev sessions", async () => {
    const ipc = makeIpc([
      makeSession({
        sessionId: "capture-1",
        sessionType: "AiJob",
        title: "captured ai job",
        interactiveAllowed: true,
      }),
    ]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} defaultOpen />);

    await screen.findByTestId("terminal-panel-active");
    const close = screen.getByTestId("terminal-close-session");

    expect(close).toBeDisabled();
    fireEvent.click(close);

    expect(ipc.closeSession).not.toHaveBeenCalled();
    expect(screen.getByTestId("terminal-tab-capture-1")).toBeInTheDocument();
    expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "capture-1");
  });

  it("restarts the active HumanDev terminal by closing then recreating it", async () => {
    const restarted = makeSession({ sessionId: "s-restart", title: "dev shell" });
    const ipc = makeIpc([makeSession({ sessionId: "s1", title: "dev shell" })]);
    vi.mocked(ipc.createSession).mockResolvedValueOnce(restarted);
    const { renderTerminal } = makeRenderRecorder();
    render(
      <TerminalPanel
        ipc={ipc}
        renderTerminal={renderTerminal}
        defaultOpen
        workspaceRoot="D:/repo"
        defaultShell="pwsh"
      />,
    );

    await screen.findByTestId("terminal-panel-active");
    fireEvent.click(screen.getByTestId("terminal-restart-session"));

    await waitFor(() => expect(ipc.closeSession).toHaveBeenCalledWith("s1"));
    expect(ipc.createSession).toHaveBeenCalledWith({
      sessionType: "HumanDev",
      shell: "pwsh",
      cwd: "D:/repo",
      title: "dev shell",
    });
    expect(vi.mocked(ipc.closeSession).mock.invocationCallOrder[0]).toBeLessThan(
      vi.mocked(ipc.createSession).mock.invocationCallOrder[0],
    );
    expect(await screen.findByTestId("terminal-tab-s-restart")).toBeInTheDocument();
    expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "s-restart");
  });

  it("renders an AiJob tab READ-ONLY by default and only interactive after Take control", async () => {
    const ipc = makeIpc([makeSession({ sessionType: "AiJob", title: "captured cloud cli", interactiveAllowed: true })]);
    const { calls, renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);
    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    await screen.findByTestId("terminal-panel-active");

    // Default: read-only badge shown, terminal rendered readOnly=true.
    expect(screen.getByTestId("terminal-readonly-badge")).toBeInTheDocument();
    expect(screen.getByTestId("fake-term-s1")).toHaveAttribute("data-readonly", "true");
    expect(calls[calls.length - 1]).toEqual({ sessionId: "s1", readOnly: true });

    // Flip Take control -> interactive.
    fireEvent.click(screen.getByTestId("terminal-take-control-checkbox"));

    await waitFor(() => expect(ipc.authorizeInteractive).toHaveBeenCalledWith("s1"));
    await waitFor(() => expect(screen.getByTestId("terminal-interactive-badge")).toBeInTheDocument());
    expect(screen.getByTestId("fake-term-s1")).toHaveAttribute("data-readonly", "false");
    expect(calls[calls.length - 1]).toEqual({ sessionId: "s1", readOnly: false });
  });

  it("keeps a captured session read-only when Take control authorization fails", async () => {
    const ipc = makeIpc([makeSession({ sessionType: "AiJob", title: "captured cloud cli", interactiveAllowed: true })]);
    vi.mocked(ipc.authorizeInteractive).mockRejectedValueOnce(new Error("capability denied"));
    const { calls, renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);
    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    await screen.findByTestId("terminal-panel-active");
    fireEvent.click(screen.getByTestId("terminal-take-control-checkbox"));

    await waitFor(() => expect(ipc.authorizeInteractive).toHaveBeenCalledWith("s1"));
    expect(await screen.findByTestId("terminal-panel-error")).toHaveTextContent("capability denied");
    expect(screen.getByTestId("terminal-readonly-badge")).toBeInTheDocument();
    expect(screen.getByTestId("fake-term-s1")).toHaveAttribute("data-readonly", "true");
    expect(calls[calls.length - 1]).toEqual({ sessionId: "s1", readOnly: true });
  });

  it("disables Take control for an AiJob session the backend has NOT granted (honest-disabled, never faked)", async () => {
    const ipc = makeIpc([makeSession({ sessionType: "AiJob", interactiveAllowed: false })]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);
    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    await screen.findByTestId("terminal-panel-active");

    const checkbox = screen.getByTestId("terminal-take-control-checkbox") as HTMLInputElement;
    expect(checkbox).toBeDisabled();
    // Even if a click is forced, it cannot flip to interactive.
    fireEvent.click(checkbox);
    expect(screen.getByTestId("terminal-readonly-badge")).toBeInTheDocument();
    expect(screen.getByTestId("fake-term-s1")).toHaveAttribute("data-readonly", "true");
  });

  it("renders a HumanDev session as interactive (no Take-control gate offered)", async () => {
    const ipc = makeIpc([makeSession({ sessionType: "HumanDev", interactiveAllowed: true })]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);
    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    await screen.findByTestId("terminal-panel-active");
    expect(screen.getByTestId("terminal-interactive-badge")).toBeInTheDocument();
    expect(screen.queryByTestId("terminal-take-control")).not.toBeInTheDocument();
    expect(screen.getByTestId("fake-term-s1")).toHaveAttribute("data-readonly", "false");
  });

  it("renders an exited HumanDev session as read-only inspection state", async () => {
    const ipc = makeIpc([
      makeSession({
        sessionType: "HumanDev",
        interactiveAllowed: true,
        exited: true,
        exitCode: 0,
      }),
    ]);
    const { calls, renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);
    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    await screen.findByTestId("terminal-panel-active");
    expect(screen.getByTestId("terminal-tab-exited-s1")).toHaveTextContent("exited (0)");
    expect(screen.getByTestId("terminal-readonly-badge")).toBeInTheDocument();
    expect(screen.queryByTestId("terminal-interactive-badge")).not.toBeInTheDocument();
    expect(screen.queryByTestId("terminal-take-control")).not.toBeInTheDocument();
    expect(screen.getByTestId("fake-term-s1")).toHaveAttribute("data-readonly", "true");
    expect(calls[calls.length - 1]).toEqual({ sessionId: "s1", readOnly: true });
  });

  it("switches the active session on tab click", async () => {
    const ipc = makeIpc([
      makeSession({ sessionId: "s1", title: "first" }),
      makeSession({ sessionId: "s2", title: "second" }),
    ]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);
    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    await screen.findByTestId("terminal-tab-s1");
    // First session active by default.
    expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "s1");

    fireEvent.click(screen.getByTestId("terminal-tab-s2"));
    expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "s2");
  });

  it("groups tabs into swarm swimlanes", async () => {
    const ipc = makeIpc([
      makeSession({ sessionId: "s1", swarmId: "swarm-A", title: "a" }),
      makeSession({ sessionId: "s2", swarmId: "swarm-B", title: "b" }),
    ]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);
    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    const laneA = await screen.findByTestId("terminal-lane-swarm:swarm-A");
    const laneB = screen.getByTestId("terminal-lane-swarm:swarm-B");
    expect(within(laneA).getByTestId("terminal-tab-s1")).toBeInTheDocument();
    expect(within(laneB).getByTestId("terminal-tab-s2")).toBeInTheDocument();
  });

  it("shows an empty-state message when there are no captured sessions", async () => {
    const ipc = makeIpc([]);
    const { renderTerminal } = makeRenderRecorder();
    render(<TerminalPanel ipc={ipc} renderTerminal={renderTerminal} />);
    fireEvent.click(screen.getByTestId("disclosure-terminal-toggle"));

    expect(await screen.findByTestId("terminal-panel-empty")).toBeInTheDocument();
  });

  it("openSignal force-opens the collapsed drawer (board 'Inspect terminal' link)", async () => {
    // The panel is collapsed-by-default; the board affordance reveals it by
    // bumping openSignal. Re-render with a new signal must open the disclosure
    // without any operator click.
    const ipc = makeIpc([makeSession({ sessionId: "s1", swarmId: "swarm-A", title: "a" })]);
    const { renderTerminal } = makeRenderRecorder();
    const { rerender } = render(
      <TerminalPanel ipc={ipc} renderTerminal={renderTerminal} openSignal={0} />,
    );

    // Collapsed initially (openSignal=0 is the mount baseline, not a trigger).
    expect(screen.getByTestId("terminal-panel")).toHaveAttribute("data-open", "false");
    expect(screen.queryByTestId("terminal-panel-body")).not.toBeInTheDocument();

    // Bump the signal -> drawer force-opens + body mounts (no click).
    rerender(
      <TerminalPanel ipc={ipc} renderTerminal={renderTerminal} openSignal={1} focusSwarmId="swarm-A" />,
    );
    await waitFor(() => expect(screen.getByTestId("terminal-panel")).toHaveAttribute("data-open", "true"));
    expect(await screen.findByTestId("terminal-panel-body")).toBeInTheDocument();
  });

  it("focusInstanceId selects the captured session whose source instance_id matches (SessionWorkbench link)", async () => {
    // The SessionWorkbench knows the selected chat session's composite
    // instance_id (the capture binding key), not the capture session's own id or
    // a swarm_id. The panel must resolve it via TerminalSession.instanceId.
    const ipc = makeIpc([
      makeSession({ sessionId: "cap-a", instanceId: "alpha-model#0", title: "a" }),
      makeSession({ sessionId: "cap-b", instanceId: "beta-cloud#0", title: "b" }),
    ]);
    const { renderTerminal } = makeRenderRecorder();
    render(
      <TerminalPanel
        ipc={ipc}
        renderTerminal={renderTerminal}
        defaultOpen
        openSignal={1}
        focusInstanceId="beta-cloud#0"
      />,
    );

    // Despite cap-a sorting first, focus targets the session bound to the
    // requested composite instance_id (beta-cloud#0 -> cap-b).
    await waitFor(() =>
      expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "cap-b"),
    );
  });

  it("focusSwarmId selects the first captured session of that swarm on open", async () => {
    // The board knows a swarm_id, not a session_id. The panel must resolve the
    // swarm_id to its first session and make that tab active.
    const ipc = makeIpc([
      makeSession({ sessionId: "s1", swarmId: "swarm-A", title: "a" }),
      makeSession({ sessionId: "s2", swarmId: "swarm-B", title: "b" }),
    ]);
    const { renderTerminal } = makeRenderRecorder();
    render(
      <TerminalPanel
        ipc={ipc}
        renderTerminal={renderTerminal}
        defaultOpen
        openSignal={1}
        focusSwarmId="swarm-B"
      />,
    );

    // Despite swarm-A's s1 sorting first, focus targets swarm-B's session.
    await waitFor(() =>
      expect(screen.getByTestId("terminal-panel-active")).toHaveAttribute("data-active-session", "s2"),
    );
  });
});
