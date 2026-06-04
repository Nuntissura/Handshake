import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";

// SessionWorkbench unit tests (governance glue #3): the combined "Session"
// surface composes the REAL OperatorChat with two affordances that act on the
// host surface's SINGLE shared TerminalPanel / SessionReplayPanel via injected
// callbacks (no second xterm, no new IPC). We mock only chatGenerate; the
// Workbench + OperatorChat render for real. The `room` is a minimal fake of the
// useSwarmRoom shape the Workbench actually reads.

vi.mock("../../lib/ipc/swarm_runtime", async () => {
  const actual = await vi.importActual<typeof import("../../lib/ipc/swarm_runtime")>(
    "../../lib/ipc/swarm_runtime",
  );
  return { ...actual, chatGenerate: vi.fn() };
});

import { SessionWorkbench } from "./SessionWorkbench";
import type { SwarmRoom } from "./SwarmControlRoom";
import type { SwarmSession } from "../../lib/ipc/swarm_runtime";

function makeSession(over: Partial<SwarmSession> & { modelId: string }): SwarmSession {
  return {
    instanceId: { modelId: over.modelId, instance: 0, composite: `${over.modelId}#0` },
    state: "READY",
    provider: "byok_cloud",
    runtimeBinding: "cloud",
    artifactPath: null,
    cloudModelName: "claude-sonnet-4",
    worktreeId: null,
    workingDir: null,
    ...over,
  } as SwarmSession;
}

/** Minimal SwarmRoom fake exposing only what the Workbench reads. */
function makeRoom(over: Partial<SwarmRoom>): SwarmRoom {
  return {
    chatInstanceId: null,
    allSessions: [],
    setChatInstanceId: vi.fn(),
    // The rest of the room is unused by the Workbench; cast through unknown.
    ...over,
  } as unknown as SwarmRoom;
}

describe("SessionWorkbench with a selected cloud session", () => {
  const cloud = makeSession({ modelId: "beta-cloud" });

  test("renders chat and both session affordances", () => {
    render(
      <SessionWorkbench
        room={makeRoom({ chatInstanceId: "beta-cloud#0", allSessions: [cloud] })}
        onShowTerminal={vi.fn()}
        onReviewSession={vi.fn()}
      />,
    );
    expect(screen.getByTestId("operator-chat")).toBeInTheDocument();
    expect(screen.getByTestId("session-workbench-show-terminal")).toBeEnabled();
    expect(screen.getByTestId("session-workbench-open-transcript")).toBeEnabled();
  });

  test("'Resume this session' is enabled and calls onResumeSession(composite)", () => {
    const onResumeSession = vi.fn();
    render(
      <SessionWorkbench
        room={makeRoom({ chatInstanceId: "beta-cloud#0", allSessions: [cloud] })}
        onShowTerminal={vi.fn()}
        onReviewSession={vi.fn()}
        onResumeSession={onResumeSession}
      />,
    );
    const btn = screen.getByTestId("session-workbench-resume");
    expect(btn).toBeEnabled();
    fireEvent.click(btn);
    expect(onResumeSession).toHaveBeenCalledWith("beta-cloud#0");
  });

  test("'Show captured terminal' calls onShowTerminal(composite)", () => {
    const onShowTerminal = vi.fn();
    render(
      <SessionWorkbench
        room={makeRoom({ chatInstanceId: "beta-cloud#0", allSessions: [cloud] })}
        onShowTerminal={onShowTerminal}
        onReviewSession={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByTestId("session-workbench-show-terminal"));
    expect(onShowTerminal).toHaveBeenCalledWith("beta-cloud#0");
  });

  test("'Open full transcript' calls onReviewSession(composite)", () => {
    const onReviewSession = vi.fn();
    render(
      <SessionWorkbench
        room={makeRoom({ chatInstanceId: "beta-cloud#0", allSessions: [cloud] })}
        onShowTerminal={vi.fn()}
        onReviewSession={onReviewSession}
      />,
    );
    fireEvent.click(screen.getByTestId("session-workbench-open-transcript"));
    expect(onReviewSession).toHaveBeenCalledWith("beta-cloud#0");
  });
});

describe("SessionWorkbench honest disabled states", () => {
  test("no session selected: both affordances disabled + a guidance note", () => {
    render(
      <SessionWorkbench
        room={makeRoom({ chatInstanceId: null, allSessions: [] })}
        onShowTerminal={vi.fn()}
        onReviewSession={vi.fn()}
        onResumeSession={vi.fn()}
      />,
    );
    expect(screen.getByTestId("session-workbench-show-terminal")).toBeDisabled();
    expect(screen.getByTestId("session-workbench-open-transcript")).toBeDisabled();
    // Resume is honestly disabled with no selection.
    expect(screen.getByTestId("session-workbench-resume")).toBeDisabled();
    expect(screen.getByTestId("session-workbench-no-selection")).toBeInTheDocument();
  });

  test("resume not wired: the Resume button is honestly disabled", () => {
    render(
      <SessionWorkbench
        room={makeRoom({ chatInstanceId: "beta-cloud#0", allSessions: [makeSession({ modelId: "beta-cloud" })] })}
        onShowTerminal={vi.fn()}
        onReviewSession={vi.fn()}
        onResumeSession={undefined}
      />,
    );
    const btn = screen.getByTestId("session-workbench-resume");
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("title", "Resume not wired");
  });

  test("transcript review not wired: the transcript button is honestly disabled", () => {
    render(
      <SessionWorkbench
        room={makeRoom({ chatInstanceId: "beta-cloud#0", allSessions: [makeSession({ modelId: "beta-cloud" })] })}
        onShowTerminal={vi.fn()}
        onReviewSession={undefined}
      />,
    );
    const btn = screen.getByTestId("session-workbench-open-transcript");
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("title", "Session Replay not wired");
  });
});
