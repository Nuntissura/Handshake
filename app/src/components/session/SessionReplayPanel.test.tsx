import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import { SessionReplayPanel } from "./SessionReplayPanel";
import type {
  SessionSummary,
  SessionTranscriptIpc,
  SessionTranscriptResponse,
  SourceStatus,
  TranscriptGetRequest,
} from "../../lib/ipc/session_transcript";

// SessionReplayPanel unit tests. Tauri `invoke` is unavailable under jsdom, so we
// inject a recording IPC stub. These assert the panel's index rendering, timeline
// ordering/typing, kind filtering, the lazy collapsed-by-default gate, and the
// HONEST empty / unavailable states (never fabricated rows; read-only review).

const ALL_PRESENT: SourceStatus = { chat: "present", fr: "present", terminal: "present", process: "present" };

function makeSummary(over: Partial<SessionSummary>): SessionSummary {
  return {
    sessionId: "sess-1",
    kind: "swarm",
    startedAt: "2026-05-30T10:00:00.000Z",
    lastActivityAt: "2026-05-30T10:05:00.000Z",
    modelId: "claude#0",
    provider: "cloud",
    title: null,
    counts: { chat: 1, fr: 2, terminal: 1, process: 1 },
    ...over,
  };
}

function makeResponse(over: Partial<SessionTranscriptResponse>): SessionTranscriptResponse {
  return {
    sessionId: "sess-1",
    truncated: false,
    sourceStatus: ALL_PRESENT,
    entries: [
      { kind: "chat_turn", ts: "2026-05-30T10:00:00.000Z", seq: 0, role: "operator", content: "do the build", messageId: "m1" },
      { kind: "agent_activity", ts: "2026-05-30T10:00:30.000Z", seq: 1, activityKind: "thinking", text: "I should run the build first.", eventId: "FR-EVT-AGENT-THINKING" },
      { kind: "agent_activity", ts: "2026-05-30T10:00:45.000Z", seq: 2, activityKind: "tool_call", name: "Bash", detail: { command: "cargo build" }, eventId: "FR-EVT-AGENT-TOOLCALL" },
      { kind: "terminal_chunk", ts: "2026-05-30T10:01:00.000Z", seq: 3, terminalSessionId: "t1", command: "cargo build" },
      { kind: "fr_event", ts: "2026-05-30T10:02:00.000Z", seq: 4, eventType: "llm_inference", frEvent: "FR-EVT-LLM-INFER-END", actor: "agent", modelId: "claude#0", payload: { tokens: 42 }, eventId: "e1" },
      { kind: "process", ts: "2026-05-30T10:03:00.000Z", seq: 5, processUuid: "p-1234", phase: "completed", modelId: "claude#0", payload: {} },
    ],
    ...over,
  };
}

interface IpcCalls {
  getTranscriptArgs: TranscriptGetRequest[];
}

function makeIpc(
  sessions: SessionSummary[],
  response: SessionTranscriptResponse,
): { ipc: SessionTranscriptIpc; calls: IpcCalls } {
  const calls: IpcCalls = { getTranscriptArgs: [] };
  const ipc: SessionTranscriptIpc = {
    listSessions: vi.fn(async () => sessions),
    getTranscript: vi.fn(async (req: TranscriptGetRequest) => {
      calls.getTranscriptArgs.push(req);
      return response;
    }),
  };
  return { ipc, calls };
}

function open() {
  fireEvent.click(screen.getByTestId("disclosure-session-replay-toggle"));
}

describe("SessionReplayPanel", () => {
  it("is collapsed by default and lazy: the body is not mounted until opened", async () => {
    const { ipc } = makeIpc([makeSummary({})], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} />);

    const disclosure = screen.getByTestId("session-replay-panel");
    expect(disclosure).toHaveAttribute("data-open", "false");
    expect(screen.queryByTestId("session-replay-body")).not.toBeInTheDocument();
    expect(ipc.listSessions).not.toHaveBeenCalled();
  });

  it("mounts the body and lists recorded sessions once opened", async () => {
    const { ipc } = makeIpc(
      [makeSummary({ sessionId: "sess-1", title: "build run" }), makeSummary({ sessionId: "sess-2", kind: "chat" })],
      makeResponse({}),
    );
    render(<SessionReplayPanel ipc={ipc} />);
    open();

    expect(await screen.findByTestId("session-replay-body")).toBeInTheDocument();
    await waitFor(() => expect(ipc.listSessions).toHaveBeenCalled());
    expect(screen.getByTestId("session-replay-row-sess-1")).toBeInTheDocument();
    expect(screen.getByTestId("session-replay-row-sess-2")).toBeInTheDocument();
    // Nothing selected yet -> the timeline shows the pick-a-session prompt.
    expect(screen.getByTestId("session-replay-timeline")).toHaveTextContent(/Select a session/i);
    expect(ipc.getTranscript).not.toHaveBeenCalled();
  });

  it("selecting a session renders its consolidated timeline in ts/seq order", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} />);
    open();

    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));

    await screen.findByTestId("session-replay-entry-0");
    const entries = screen.getByTestId("session-replay-entries");
    const rows = within(entries).getAllByTestId(/session-replay-entry-/);
    // Merged entries, in the seq order the aggregator assigned (agent-activity
    // rows merge into the timeline by ts alongside the other lanes).
    expect(rows.map((r) => r.getAttribute("data-kind"))).toEqual([
      "chat_turn",
      "agent_activity",
      "agent_activity",
      "terminal_chunk",
      "fr_event",
      "process",
    ]);
    // The fetch carried the selected session id.
    await waitFor(() =>
      expect(ipc.getTranscript).toHaveBeenCalledWith(expect.objectContaining({ sessionId: "sess-1" })),
    );
  });

  it("kind filter toggles re-query the backend and hide lanes client-side", async () => {
    const { ipc, calls } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));
    await screen.findByTestId("session-replay-entry-0");

    // All lanes on -> the chat row is visible.
    expect(screen.getByTestId("session-replay-entry-0")).toHaveAttribute("data-kind", "chat_turn");

    // Toggle Chat off -> the chat row disappears (client-side refilter) and the
    // backend is re-queried with an explicit kinds list (server-side filter).
    fireEvent.click(screen.getByTestId("session-replay-filter-chat_turn"));
    await waitFor(() => {
      const last = calls.getTranscriptArgs[calls.getTranscriptArgs.length - 1];
      expect(last.kinds).toBeTruthy();
      expect(last.kinds).not.toContain("chat_turn");
    });
    expect(screen.queryByTestId("session-replay-entry-0")).not.toBeInTheDocument();
    // The other lanes remain (agent_activity at seq 1, terminal at seq 3).
    expect(screen.getByTestId("session-replay-entry-1")).toHaveAttribute("data-kind", "agent_activity");
    expect(screen.getByTestId("session-replay-entry-3")).toHaveAttribute("data-kind", "terminal_chunk");
  });

  it("shows an honest empty-list state when no sessions are recorded", async () => {
    const { ipc } = makeIpc([], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} />);
    open();

    expect(await screen.findByTestId("session-replay-list-empty")).toHaveTextContent(/No recorded sessions/i);
  });

  it("shows honest per-lane empty states when a selected session has no entries (never fabricated)", async () => {
    const emptyResponse = makeResponse({
      entries: [],
      sourceStatus: { chat: "empty", fr: "empty", terminal: "empty", process: "empty" },
    });
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], emptyResponse);
    render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));

    const empty = await screen.findByTestId("session-replay-empty");
    expect(empty).toBeInTheDocument();
    expect(screen.getByTestId("session-replay-empty-chat_turn")).toHaveTextContent(/No chat turns/i);
    expect(screen.getByTestId("session-replay-empty-fr_event")).toHaveTextContent(/No Flight Recorder/i);
    // No transcript rows were fabricated.
    expect(screen.queryByTestId(/session-replay-entry-/)).not.toBeInTheDocument();
  });

  it("renders the unavailable banner when a source is unavailable (honest-disabled)", async () => {
    const res = makeResponse({
      sourceStatus: { chat: "present", fr: "unavailable", terminal: "unavailable", process: "unavailable" },
      entries: [
        { kind: "chat_turn", ts: "2026-05-30T10:00:00.000Z", seq: 0, role: "operator", content: "hi", messageId: "m1" },
      ],
    });
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1", kind: "chat" })], res);
    render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));

    expect(await screen.findByTestId("session-replay-unavailable-banner")).toHaveTextContent(/unavailable/i);
    // The present chat lane still renders honestly.
    expect(screen.getByTestId("session-replay-entry-0")).toHaveAttribute("data-kind", "chat_turn");
  });

  it("surfaces a truncated chip when the backend applied a hard cap", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({ truncated: true }));
    render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));

    expect(await screen.findByTestId("session-replay-truncated-chip")).toBeInTheDocument();
  });

  it("openSignal force-opens the collapsed drawer and preselects focusSessionId (board 'Review session' link)", async () => {
    const { ipc } = makeIpc(
      [makeSummary({ sessionId: "sess-1" }), makeSummary({ sessionId: "sess-2" })],
      makeResponse({ sessionId: "sess-2" }),
    );
    const { rerender } = render(<SessionReplayPanel ipc={ipc} openSignal={0} />);

    // Collapsed initially (openSignal=0 is the mount baseline, not a trigger).
    expect(screen.getByTestId("session-replay-panel")).toHaveAttribute("data-open", "false");
    expect(screen.queryByTestId("session-replay-body")).not.toBeInTheDocument();

    // Bump the signal + focus a session -> drawer opens, body mounts, session 2
    // is preselected and its transcript fetched (no operator click).
    rerender(<SessionReplayPanel ipc={ipc} openSignal={1} focusSessionId="sess-2" />);
    await waitFor(() => expect(screen.getByTestId("session-replay-panel")).toHaveAttribute("data-open", "true"));
    await waitFor(() =>
      expect(screen.getByTestId("session-replay-row-sess-2")).toHaveAttribute("data-selected", "true"),
    );
    await waitFor(() =>
      expect(ipc.getTranscript).toHaveBeenCalledWith(expect.objectContaining({ sessionId: "sess-2" })),
    );
  });

  it("renders a structured agent tool_call row: name + collapsible redacted args", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));
    await screen.findByTestId("session-replay-entry-0");

    // seq 2 is the tool_call (Bash). The row carries the agent kind + tool name,
    // and the args are in a collapsible <details> (a real <pre> with the input).
    const toolRow = screen.getByTestId("session-replay-entry-2");
    expect(toolRow).toHaveAttribute("data-kind", "agent_activity");
    const tool = within(toolRow).getByText(/Bash/);
    expect(tool).toBeInTheDocument();
    // The collapsible args contain the (redacted-on-the-backend) command input.
    expect(toolRow).toHaveTextContent(/cargo build/);
    expect(toolRow.querySelector("details[data-agent-kind='tool_call']")).not.toBeNull();
  });

  it("renders a structured agent thinking row distinctly (muted/italic visible reasoning)", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));
    await screen.findByTestId("session-replay-entry-0");

    // seq 1 is the thinking row.
    const thinkRow = screen.getByTestId("session-replay-entry-1");
    expect(thinkRow).toHaveAttribute("data-kind", "agent_activity");
    expect(thinkRow).toHaveTextContent(/run the build first/);
    const body = thinkRow.querySelector("[data-agent-kind='thinking']") as HTMLElement | null;
    expect(body).not.toBeNull();
    expect(body!.style.fontStyle).toBe("italic");
  });

  it("renders an agent 'other' row with a raw chip (HONEST defensive fallback, never dropped)", async () => {
    const res = makeResponse({
      entries: [
        { kind: "agent_activity", ts: "2026-05-30T10:00:00.000Z", seq: 0, activityKind: "other", text: "{not valid json", eventId: "FR-EVT-AGENT-OTHER" },
      ],
    });
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], res);
    render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));

    const row = await screen.findByTestId("session-replay-entry-0");
    expect(row).toHaveAttribute("data-kind", "agent_activity");
    // The raw line is preserved verbatim and flagged as raw (honest fallback).
    expect(row).toHaveTextContent("raw");
    expect(row).toHaveTextContent("{not valid json");
    expect(row.querySelector("[data-agent-kind='other']")).not.toBeNull();
  });

  it("kind filter includes the Agent lane and toggling it off hides agent rows", async () => {
    const { ipc, calls } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));
    await screen.findByTestId("session-replay-entry-0");

    // The Agent filter chip exists and the agent rows are visible by default.
    expect(screen.getByTestId("session-replay-filter-agent_activity")).toBeInTheDocument();
    expect(screen.getByTestId("session-replay-entry-1")).toHaveAttribute("data-kind", "agent_activity");
    expect(screen.getByTestId("session-replay-entry-2")).toHaveAttribute("data-kind", "agent_activity");

    // Toggle Agent off -> agent rows disappear; backend re-queried without it.
    fireEvent.click(screen.getByTestId("session-replay-filter-agent_activity"));
    await waitFor(() => {
      const last = calls.getTranscriptArgs[calls.getTranscriptArgs.length - 1];
      expect(last.kinds).toBeTruthy();
      expect(last.kinds).not.toContain("agent_activity");
    });
    expect(screen.queryByTestId("session-replay-entry-1")).not.toBeInTheDocument();
    expect(screen.queryByTestId("session-replay-entry-2")).not.toBeInTheDocument();
    // Non-agent lanes remain (chat row at seq 0).
    expect(screen.getByTestId("session-replay-entry-0")).toHaveAttribute("data-kind", "chat_turn");
  });

  it("is read-only: renders no text inputs, textareas, or stdin affordances", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({}));
    const { container } = render(<SessionReplayPanel ipc={ipc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));
    await screen.findByTestId("session-replay-entry-0");

    // The only interactive controls are the disclosure toggle, filter chips, and
    // session-select buttons — all <button>. No inputs/textareas anywhere.
    expect(container.querySelectorAll("input")).toHaveLength(0);
    expect(container.querySelectorAll("textarea")).toHaveLength(0);
  });
});
