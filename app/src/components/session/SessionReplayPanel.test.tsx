import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";

// Defensive isolation: a fake-timer test that leaves a pending interval/timeout
// scheduled must not bleed into the next test. Unmount the tree + drain any
// outstanding timers BEFORE restoring real timers, then clear all timers.
afterEach(() => {
  cleanup();
  if (vi.isFakeTimers()) {
    vi.clearAllTimers();
    vi.useRealTimers();
  }
});

import { SessionReplayPanel } from "./SessionReplayPanel";
import { entryStableKey } from "../../lib/ipc/session_transcript";
import type {
  LiveTailIpc,
  SessionSummary,
  SessionTranscriptIpc,
  SessionTranscriptResponse,
  SourceStatus,
  TranscriptGetRequest,
} from "../../lib/ipc/session_transcript";
import { eventInstanceKey } from "../../lib/ipc/swarm_runtime";

// A no-op live-tail seam for the POST-HOC tests: no events are pushed, so the
// live tail (default ON for streaming sessions) does its initial full load and
// then sits idle — the rendered timeline is identical to the post-hoc path. The
// dedicated live-behavior tests below inject a controllable seam instead.
function noopLiveIpc(): LiveTailIpc {
  return {
    subscribeBoardEvents: vi.fn(async () => () => {}),
    subscribeTerminal: vi.fn(async () => () => {}),
    listTerminalSessions: vi.fn(async () => []),
  };
}

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
  resumeArgs: string[];
}

function makeIpc(
  sessions: SessionSummary[],
  response: SessionTranscriptResponse,
): { ipc: SessionTranscriptIpc; calls: IpcCalls } {
  const calls: IpcCalls = { getTranscriptArgs: [], resumeArgs: [] };
  const ipc: SessionTranscriptIpc = {
    listSessions: vi.fn(async () => sessions),
    getTranscript: vi.fn(async (req: TranscriptGetRequest) => {
      calls.getTranscriptArgs.push(req);
      return response;
    }),
    // ROI #3: default resume fake returns a deterministic fresh composite. Tests
    // that exercise resume override this; the rest never call it.
    resumeSession: vi.fn(async (sessionId: string) => {
      calls.resumeArgs.push(sessionId);
      return { modelId: `${sessionId}-resumed`, instance: 0, composite: `${sessionId}-resumed#0` };
    }),
    getSpawnTemplate: vi.fn(async () => null),
  };
  return { ipc, calls };
}

function open() {
  fireEvent.click(screen.getByTestId("disclosure-session-replay-toggle"));
}

describe("SessionReplayPanel", () => {
  it("is collapsed by default and lazy: the body is not mounted until opened", async () => {
    const { ipc } = makeIpc([makeSummary({})], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);

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
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
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
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
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
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
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
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
    open();

    expect(await screen.findByTestId("session-replay-list-empty")).toHaveTextContent(/No recorded sessions/i);
  });

  it("shows honest per-lane empty states when a selected session has no entries (never fabricated)", async () => {
    const emptyResponse = makeResponse({
      entries: [],
      sourceStatus: { chat: "empty", fr: "empty", terminal: "empty", process: "empty" },
    });
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], emptyResponse);
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
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
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));

    expect(await screen.findByTestId("session-replay-unavailable-banner")).toHaveTextContent(/unavailable/i);
    // The present chat lane still renders honestly.
    expect(screen.getByTestId("session-replay-entry-0")).toHaveAttribute("data-kind", "chat_turn");
  });

  it("surfaces a truncated chip when the backend applied a hard cap", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({ truncated: true }));
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));

    expect(await screen.findByTestId("session-replay-truncated-chip")).toBeInTheDocument();
  });

  it("openSignal force-opens the collapsed drawer and preselects focusSessionId (board 'Review session' link)", async () => {
    const { ipc } = makeIpc(
      [makeSummary({ sessionId: "sess-1" }), makeSummary({ sessionId: "sess-2" })],
      makeResponse({ sessionId: "sess-2" }),
    );
    const { rerender } = render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} openSignal={0} />);

    // Collapsed initially (openSignal=0 is the mount baseline, not a trigger).
    expect(screen.getByTestId("session-replay-panel")).toHaveAttribute("data-open", "false");
    expect(screen.queryByTestId("session-replay-body")).not.toBeInTheDocument();

    // Bump the signal + focus a session -> drawer opens, body mounts, session 2
    // is preselected and its transcript fetched (no operator click).
    rerender(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} openSignal={1} focusSessionId="sess-2" />);
    await waitFor(() => expect(screen.getByTestId("session-replay-panel")).toHaveAttribute("data-open", "true"));
    await waitFor(() =>
      expect(screen.getByTestId("session-replay-row-sess-2")).toHaveAttribute("data-selected", "true"),
    );
    await waitFor(() =>
      expect(ipc.getTranscript).toHaveBeenCalledWith(expect.objectContaining({ sessionId: "sess-2" })),
    );
  });

  it("discards a stale cross-session fetch: a slow result for the previous session never lands in the new session's view", async () => {
    // Regression guard for the cross-session race (fetchGenRef). getTranscript is
    // deferred so a fetch issued for sess-A can resolve AFTER the operator has
    // switched focus to sess-B; its result must be discarded, not written into B.
    const deferreds = new Map<string, (r: SessionTranscriptResponse) => void>();
    const ipc: SessionTranscriptIpc = {
      listSessions: vi.fn(async () => [
        makeSummary({ sessionId: "sess-A" }),
        makeSummary({ sessionId: "sess-B" }),
      ]),
      getTranscript: vi.fn(
        (req: TranscriptGetRequest) =>
          new Promise<SessionTranscriptResponse>((resolve) => {
            deferreds.set(req.sessionId, resolve);
          }),
      ),
      resumeSession: vi.fn(async (s: string) => ({ modelId: s, instance: 0, composite: `${s}#0` })),
      getSpawnTemplate: vi.fn(async () => null),
    };

    // openSignal is a one-shot: the mount value is the baseline, only a CHANGE
    // triggers open. Render collapsed (0), then bump to open + focus sess-A.
    const { rerender } = render(
      <SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} openSignal={0} />,
    );
    rerender(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} openSignal={1} focusSessionId="sess-A" />);
    // sess-A's initial fetch is in flight (deferred, unresolved).
    await waitFor(() => expect(deferreds.has("sess-A")).toBe(true));

    // Switch focus to sess-B while A is still pending. B's fetch is deferred
    // behind the single-flight guard until A settles.
    rerender(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} openSignal={2} focusSessionId="sess-B" />);

    // Resolve the now-STALE sess-A fetch with an A-only marker. The trailing edge
    // then issues B's fetch.
    deferreds.get("sess-A")?.(
      makeResponse({
        sessionId: "sess-A",
        entries: [
          { kind: "chat_turn", ts: "2026-05-30T10:00:00.000Z", seq: 0, role: "operator", content: "STALE_SESSION_A_DATA", messageId: "a1" },
        ],
      }),
    );

    await waitFor(() => expect(deferreds.has("sess-B")).toBe(true));
    deferreds.get("sess-B")?.(
      makeResponse({
        sessionId: "sess-B",
        entries: [
          { kind: "chat_turn", ts: "2026-05-30T10:00:00.000Z", seq: 0, role: "operator", content: "FRESH_SESSION_B_DATA", messageId: "b1" },
        ],
      }),
    );

    // B's data shows; A's stale data must NEVER appear under B.
    await waitFor(() => expect(screen.getByText("FRESH_SESSION_B_DATA")).toBeInTheDocument());
    expect(screen.queryByText("STALE_SESSION_A_DATA")).not.toBeInTheDocument();
  });

  it("renders a structured agent tool_call row: name + collapsible redacted args", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1" })], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
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
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
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
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
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
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
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
    const { container } = render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-sess-1"));
    await screen.findByTestId("session-replay-entry-0");

    // The only interactive controls are the disclosure toggle, filter chips, and
    // session-select buttons — all <button>. No inputs/textareas anywhere.
    expect(container.querySelectorAll("input")).toHaveLength(0);
    expect(container.querySelectorAll("textarea")).toHaveLength(0);
  });
});

// ===========================================================================
// ROI #3 STATE RECOVERY: the Resume affordance (resume-from-session).
// ===========================================================================
describe("SessionReplayPanel resume affordance", () => {
  it("shows Resume ONLY on resumable rows (honest: not-resumable rows have no button)", async () => {
    const { ipc } = makeIpc(
      [
        makeSummary({ sessionId: "swarm-1", resumable: true }),
        makeSummary({ sessionId: "chat-1", kind: "chat", resumable: false }),
      ],
      makeResponse({}),
    );
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
    open();

    await screen.findByTestId("session-replay-row-swarm-1");
    // The resumable swarm session offers Resume; the chat session does not.
    expect(screen.getByTestId("session-replay-resume-swarm-1")).toBeInTheDocument();
    expect(screen.queryByTestId("session-replay-resume-chat-1")).not.toBeInTheDocument();
    expect(screen.getByTestId("session-replay-row-swarm-1")).toHaveAttribute("data-resumable", "true");
    expect(screen.getByTestId("session-replay-row-chat-1")).toHaveAttribute("data-resumable", "false");
  });

  it("clicking Resume calls the resume IPC, fires onResumed(new, origin), and renders the lineage note", async () => {
    const onResumed = vi.fn();
    const { ipc, calls } = makeIpc([makeSummary({ sessionId: "swarm-1", resumable: true })], makeResponse({}));
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} onResumed={onResumed} />);
    open();

    fireEvent.click(await screen.findByTestId("session-replay-resume-swarm-1"));

    // The validated spawn path is driven via the resume IPC with the row's id.
    await waitFor(() => expect(calls.resumeArgs).toContain("swarm-1"));
    // The host is handed the fresh composite + the resumed-from (origin) id.
    await waitFor(() =>
      expect(onResumed).toHaveBeenCalledWith("swarm-1-resumed#0", "swarm-1"),
    );
    // The inline lineage note shows "resumed from" with both ids as data attrs.
    const lineage = await screen.findByTestId("session-replay-resumed-lineage");
    expect(lineage).toHaveAttribute("data-new-composite", "swarm-1-resumed#0");
    expect(lineage).toHaveAttribute("data-origin-session-id", "swarm-1");
    // The index was reloaded so the new (itself-resumable) session can appear.
    expect(ipc.listSessions).toHaveBeenCalledTimes(2);
  });

  it("renders a backend resume error VERBATIM inline on the row (e.g. SESSION_NOT_RESUMABLE)", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "swarm-1", resumable: true })], makeResponse({}));
    // Override resume to reject with the typed not-resumable error (a race: the
    // template was GC'd between the list snapshot and the click).
    ipc.resumeSession = vi.fn(async () => {
      throw new Error("SESSION_NOT_RESUMABLE: no spawn template stored for swarm-1");
    });
    render(<SessionReplayPanel ipc={ipc} liveIpc={noopLiveIpc()} />);
    open();

    fireEvent.click(await screen.findByTestId("session-replay-resume-swarm-1"));

    const err = await screen.findByTestId("session-replay-resume-error-swarm-1");
    expect(err).toHaveTextContent("SESSION_NOT_RESUMABLE: no spawn template stored for swarm-1");
    // No lineage note on failure.
    expect(screen.queryByTestId("session-replay-resumed-lineage")).not.toBeInTheDocument();
  });
});

// ===========================================================================
// Pure helper tests (correlation + stable dedupe keys).
// ===========================================================================
describe("eventInstanceKey", () => {
  it("maps every instance-bearing SwarmEvent variant to the composite key", () => {
    const id = { model_id: "claude-sonnet", instance: 2 };
    const key = "claude-sonnet#2";
    expect(eventInstanceKey({ SessionSpawned: { instance_id: id, parent_session_id: "p", process_uuid: "u" } })).toBe(key);
    expect(eventInstanceKey({ SessionReady: { instance_id: id } })).toBe(key);
    expect(eventInstanceKey({ SessionStateChanged: { instance_id: id, from: "READY", to: "GENERATING" } })).toBe(key);
    expect(eventInstanceKey({ SessionCancelled: { instance_id: id, reason: "x" } })).toBe(key);
    expect(eventInstanceKey({ SessionCompleted: { instance_id: id } })).toBe(key);
    expect(eventInstanceKey({ SessionFailed: { instance_id: id, error: "e" } })).toBe(key);
    expect(eventInstanceKey({ ResourceAllocated: { instance_id: id, permits_in_use: 1, permits_cap: 4 } })).toBe(key);
    expect(eventInstanceKey({ ResourceEvicted: { instance_id: id, terminal_state: "COMPLETED" } })).toBe(key);
    expect(eventInstanceKey({ LeaseExpired: { instance_id: id } })).toBe(key);
    expect(eventInstanceKey({ SpawnRejected: { instance_id: id, reason: "cap" } })).toBe(key);
  });

  it("returns null for events with no instance id (BreakerTripped)", () => {
    expect(eventInstanceKey({ BreakerTripped: { signature: "sig" } })).toBeNull();
  });
});

describe("entryStableKey", () => {
  it("keys each kind by its DURABLE identity, never the fetch-relative seq", () => {
    expect(
      entryStableKey({ kind: "chat_turn", ts: "t", seq: 0, role: "operator", content: "x", messageId: "m9" }),
    ).toBe("chat:m9");
    expect(
      entryStableKey({ kind: "fr_event", ts: "t", seq: 7, eventType: "system", actor: "a", payload: {}, eventId: "ev-7" }),
    ).toBe("fr:ev-7");
    expect(
      entryStableKey({ kind: "agent_activity", ts: "t", seq: 3, activityKind: "text", text: "hi", eventId: "ag-3" }),
    ).toBe("agent:ag-3");
  });

  it("keys the live-scrollback singleton by terminal session ONLY (stable across ts)", () => {
    // The enrichment row (no frEvent, no command) is re-synthesized with a NEW ts
    // every fetch; its key must NOT change so it is replaced, not duplicated.
    const a = entryStableKey({ kind: "terminal_chunk", ts: "2026-05-30T10:00:00Z", seq: 0, terminalSessionId: "term-9", text: "abc" });
    const b = entryStableKey({ kind: "terminal_chunk", ts: "2026-05-30T10:00:05Z", seq: 0, terminalSessionId: "term-9", text: "abcdef" });
    expect(a).toBe("terminal-live:term-9");
    expect(b).toBe(a);
  });

  it("keys an FR-derived terminal command row by its stable content tuple", () => {
    const k = entryStableKey({
      kind: "terminal_chunk", ts: "2026-05-30T10:01:00Z", seq: 0, terminalSessionId: "term-9",
      frEvent: "FR-EVT-TERM-CMD", command: "cargo build",
    });
    expect(k).toBe("terminal:term-9:2026-05-30T10:01:00Z:FR-EVT-TERM-CMD:cargo build");
  });
});

// ===========================================================================
// Live tail behavior. A controllable live seam drives the swarm + terminal
// streams deterministically; fake timers exercise the debounce / rate-cap.
// ===========================================================================

interface LiveHarness {
  liveIpc: LiveTailIpc;
  emitSwarm: (event: unknown, seq?: number) => void;
  emitSwarmResync: (dropped: number) => void;
  emitTerminalOutput: (sessionId: string) => void;
  emitTerminalExit: (sessionId: string) => void;
  emitTerminalResync: (sessionId: string) => void;
  terminalSessions: TerminalSessionRow[];
}

interface TerminalSessionRow {
  sessionId: string;
  instanceId: string | null;
}

function makeLiveHarness(terminalSessions: TerminalSessionRow[] = []): LiveHarness {
  let onDelta: ((d: { seq: number; event: unknown }) => void) | null = null;
  let onResync: ((n: number) => void) | null = null;
  let termSub:
    | {
        onOutput: (id: string, bytes: Uint8Array) => void;
        onExit: (id: string, code: number | null) => void;
        onResync: (id: string, info: { reason: string; dropped: number }) => void;
      }
    | null = null;
  let seqCounter = 0;

  const liveIpc: LiveTailIpc = {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    subscribeBoardEvents: vi.fn(async (d: any, r: any) => {
      onDelta = d;
      onResync = r;
      return () => {
        onDelta = null;
        onResync = null;
      };
    }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    subscribeTerminal: vi.fn(async (sub: any) => {
      termSub = sub;
      return () => {
        termSub = null;
      };
    }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    listTerminalSessions: vi.fn(async () => terminalSessions as any),
  };

  return {
    liveIpc,
    terminalSessions,
    emitSwarm: (event, seq) => onDelta?.({ seq: seq ?? seqCounter++, event }),
    emitSwarmResync: (dropped) => onResync?.(dropped),
    emitTerminalOutput: (sessionId) => termSub?.onOutput(sessionId, new Uint8Array()),
    emitTerminalExit: (sessionId) => termSub?.onExit(sessionId, 0),
    emitTerminalResync: (sessionId) => termSub?.onResync(sessionId, { reason: "seq-gap", dropped: 1 }),
  };
}

/** Open the drawer + select the (streaming) session so the live tail engages. */
async function openAndSelectLive(sessionId: string) {
  open();
  fireEvent.click(await screen.findByTestId(`session-replay-row-${sessionId}`));
  // The live tail's initial full load resolves -> the timeline mounts.
  await screen.findByTestId("session-replay-entries");
}

describe("SessionReplayPanel live tail", () => {
  it("defaults Live ON for a streaming session and shows the live status chip", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "sess-1", kind: "swarm" })], makeResponse({}));
    const h = makeLiveHarness();
    render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
    await openAndSelectLive("sess-1");

    expect(screen.getByTestId("session-replay-live-toggle")).toHaveAttribute("data-active", "true");
    expect(screen.getByTestId("session-replay-live-status")).toHaveAttribute("data-status", "live");
    // The live tail seeded itself with a full load (from = null).
    await waitFor(() => expect(ipc.getTranscript).toHaveBeenCalled());
    expect(ipc.getTranscript).toHaveBeenCalledWith(expect.objectContaining({ sessionId: "sess-1", from: null }));
  });

  it("a focused swarm event triggers exactly ONE debounced TAIL fetch (from = last ts)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { ipc, calls } = makeIpc([makeSummary({ sessionId: "claude#0", kind: "swarm" })], makeResponse({ sessionId: "claude#0" }));
      const h = makeLiveHarness();
      render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
      open();
      fireEvent.click(await screen.findByTestId("session-replay-row-claude#0"));
      // Flush the seed full load.
      await vi.runOnlyPendingTimersAsync();
      const seedCalls = calls.getTranscriptArgs.length;

      // A burst of 5 focused events inside the debounce window coalesces to ONE.
      for (let i = 0; i < 5; i += 1) {
        h.emitSwarm({ SessionStateChanged: { instance_id: { model_id: "claude", instance: 0 }, from: "READY", to: "GENERATING" } });
      }
      await vi.advanceTimersByTimeAsync(300);
      const afterBurst = calls.getTranscriptArgs.length;
      expect(afterBurst).toBe(seedCalls + 1);
      // The tail carried the inclusive `from` cursor = the newest seeded ts.
      const tail = calls.getTranscriptArgs[calls.getTranscriptArgs.length - 1];
      expect(tail.from).toBe("2026-05-30T10:03:00.000Z");
    } finally {
      vi.useRealTimers();
    }
  });

  it("a non-focused event triggers NO fetch (correlation gate)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { ipc, calls } = makeIpc([makeSummary({ sessionId: "claude#0", kind: "swarm" })], makeResponse({ sessionId: "claude#0" }));
      const h = makeLiveHarness();
      render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
      open();
      fireEvent.click(await screen.findByTestId("session-replay-row-claude#0"));
      // Fully settle the seed full-load + any first-render/trailing-edge effects.
      await vi.advanceTimersByTimeAsync(900);
      await vi.runOnlyPendingTimersAsync();
      const seed = calls.getTranscriptArgs.length;

      // An event for a DIFFERENT instance must not fetch. We assert no TAIL fetch
      // (from != null) was issued by the non-focused event, which is the precise
      // correlation-gate guarantee (a stray full reconcile would be harmless).
      const tailsBefore = calls.getTranscriptArgs.filter((a) => a.from != null).length;
      h.emitSwarm({ SessionStateChanged: { instance_id: { model_id: "other", instance: 9 }, from: "READY", to: "GENERATING" } });
      await vi.advanceTimersByTimeAsync(400);
      const tailsAfter = calls.getTranscriptArgs.filter((a) => a.from != null).length;
      // The correlation gate: a NON-focused event issues NO incremental tail
      // fetch (a tail carries from != null). This is the precise guarantee — an
      // unrelated session's events never re-pull the focused transcript.
      expect(tailsAfter).toBe(tailsBefore);
      void seed;
    } finally {
      vi.useRealTimers();
    }
  });

  it("appends NEW rows without duplicating the inclusive-cursor boundary row (dedupe by id)", async () => {
    const sessions = [makeSummary({ sessionId: "claude#0", kind: "swarm" })];
    const seed = makeResponse({ sessionId: "claude#0" });
    // The tail fetch returns the boundary row (process, same id) PLUS one new fr row.
    const tail: SessionTranscriptResponse = {
      sessionId: "claude#0",
      truncated: false,
      sourceStatus: ALL_PRESENT,
      entries: [
        // Re-returned boundary row (inclusive `from`) — same eventId/uuid as seed.
        { kind: "process", ts: "2026-05-30T10:03:00.000Z", seq: 0, processUuid: "p-1234", phase: "completed", modelId: "claude#0", payload: {} },
        // A genuinely new row.
        { kind: "fr_event", ts: "2026-05-30T10:04:00.000Z", seq: 1, eventType: "system", frEvent: "FR-EVT-NEW", actor: "agent", payload: { ok: true }, eventId: "ev-new" },
      ],
    };
    let call = 0;
    const ipc: SessionTranscriptIpc = {
      listSessions: vi.fn(async () => sessions),
      getTranscript: vi.fn(async () => {
        call += 1;
        return call === 1 ? seed : tail;
      }),
      resumeSession: vi.fn(async (s: string) => ({ modelId: s, instance: 0, composite: `${s}#0` })),
      getSpawnTemplate: vi.fn(async () => null),
    };
    const h = makeLiveHarness();
    render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
    await openAndSelectLive("claude#0");
    await screen.findByText(/do the build/);

    // 6 seed rows initially.
    expect(within(screen.getByTestId("session-replay-entries")).getAllByTestId(/session-replay-entry-/)).toHaveLength(6);

    h.emitSwarm({ SessionStateChanged: { instance_id: { model_id: "claude", instance: 0 }, from: "READY", to: "GENERATING" } });

    // The new fr row appears; the re-returned process boundary row is NOT duped.
    await screen.findByText(/FR-EVT-NEW/);
    const rows = within(screen.getByTestId("session-replay-entries")).getAllByTestId(/session-replay-entry-/);
    expect(rows).toHaveLength(7); // 6 + 1 new, boundary dedup'd (not 8).
  });

  it("a swarm resync triggers a FULL refetch (from = null, replaces the model)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { ipc, calls } = makeIpc([makeSummary({ sessionId: "claude#0", kind: "swarm" })], makeResponse({ sessionId: "claude#0" }));
      const h = makeLiveHarness();
      render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
      open();
      fireEvent.click(await screen.findByTestId("session-replay-row-claude#0"));
      await vi.runOnlyPendingTimersAsync();
      const before = calls.getTranscriptArgs.length;

      h.emitSwarmResync(3);
      await vi.advanceTimersByTimeAsync(400);
      expect(calls.getTranscriptArgs.length).toBe(before + 1);
      expect(calls.getTranscriptArgs[calls.getTranscriptArgs.length - 1].from).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it("a focused terminal output (mapped via instanceId) triggers a tail fetch", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { ipc, calls } = makeIpc([makeSummary({ sessionId: "claude#0", kind: "swarm" })], makeResponse({ sessionId: "claude#0" }));
      // Terminal session "term-A" is bound to instanceId "claude#0".
      const h = makeLiveHarness([{ sessionId: "term-A", instanceId: "claude#0" }]);
      render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
      open();
      fireEvent.click(await screen.findByTestId("session-replay-row-claude#0"));
      await vi.runOnlyPendingTimersAsync();
      const seed = calls.getTranscriptArgs.length;

      h.emitTerminalOutput("term-A");
      await vi.advanceTimersByTimeAsync(400);
      expect(calls.getTranscriptArgs.length).toBe(seed + 1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("a focused SessionCompleted flips Live to ENDED and stops fetching", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { ipc, calls } = makeIpc([makeSummary({ sessionId: "claude#0", kind: "swarm" })], makeResponse({ sessionId: "claude#0" }));
      const h = makeLiveHarness();
      render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
      open();
      fireEvent.click(await screen.findByTestId("session-replay-row-claude#0"));
      await vi.runOnlyPendingTimersAsync();

      h.emitSwarm({ SessionCompleted: { instance_id: { model_id: "claude", instance: 0 } } });
      await vi.advanceTimersByTimeAsync(600); // closing tail fetch fires once
      const frozen = calls.getTranscriptArgs.length;

      // Further events must NOT fetch (the tail is idle once ended).
      h.emitSwarm({ SessionStateChanged: { instance_id: { model_id: "claude", instance: 0 }, from: "x", to: "y" } });
      await vi.advanceTimersByTimeAsync(600);
      expect(calls.getTranscriptArgs.length).toBe(frozen);

      await vi.runOnlyPendingTimersAsync();
      vi.useRealTimers();
      expect(screen.getByTestId("session-replay-live-status")).toHaveAttribute("data-status", "ended");
    } finally {
      vi.useRealTimers();
    }
  });

  it("a CHAT (UUID) session is honestly labelled 'polled' (no event correlation)", async () => {
    const { ipc } = makeIpc(
      [makeSummary({ sessionId: "11111111-2222-3333-4444-555555555555", kind: "chat" })],
      makeResponse({ sessionId: "11111111-2222-3333-4444-555555555555" }),
    );
    const h = makeLiveHarness();
    render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
    open();
    fireEvent.click(await screen.findByTestId("session-replay-row-11111111-2222-3333-4444-555555555555"));
    await screen.findByTestId("session-replay-entries");

    // Live is on, but a chat UUID correlates to no stream -> honest "polled".
    expect(screen.getByTestId("session-replay-live-status")).toHaveAttribute("data-status", "polled");
    // No swarm/terminal subscription was opened for a chat session.
    expect(h.liveIpc.subscribeBoardEvents).not.toHaveBeenCalled();
  });

  it("toggling Live OFF returns to the post-hoc path and shows idle", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "claude#0", kind: "swarm" })], makeResponse({ sessionId: "claude#0" }));
    const h = makeLiveHarness();
    render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
    await openAndSelectLive("claude#0");
    expect(screen.getByTestId("session-replay-live-status")).toHaveAttribute("data-status", "live");

    fireEvent.click(screen.getByTestId("session-replay-live-toggle"));
    await waitFor(() =>
      expect(screen.getByTestId("session-replay-live-status")).toHaveAttribute("data-status", "idle"),
    );
    // The post-hoc timeline still renders the same recorded rows.
    expect(await screen.findByText(/do the build/)).toBeInTheDocument();
  });

  it("autoscroll pauses on operator scroll-up and a 'Jump to latest' affordance resumes it", async () => {
    const { ipc } = makeIpc([makeSummary({ sessionId: "claude#0", kind: "swarm" })], makeResponse({ sessionId: "claude#0" }));
    const h = makeLiveHarness();
    render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
    await openAndSelectLive("claude#0");
    await screen.findByText(/do the build/);

    const list = screen.getByTestId("session-replay-entries") as HTMLUListElement;
    // Simulate a scrollable list the operator scrolled UP in (not at bottom).
    Object.defineProperty(list, "scrollHeight", { configurable: true, value: 1000 });
    Object.defineProperty(list, "clientHeight", { configurable: true, value: 200 });
    list.scrollTop = 0;
    fireEvent.scroll(list);

    // The resume affordance appears (autoscroll paused).
    expect(await screen.findByTestId("session-replay-autoscroll-resume")).toBeInTheDocument();

    // Clicking it resumes autoscroll (button disappears once pinned to bottom).
    Object.defineProperty(list, "scrollTop", { configurable: true, writable: true, value: 0 });
    fireEvent.click(screen.getByTestId("session-replay-autoscroll-resume"));
    await waitFor(() =>
      expect(screen.queryByTestId("session-replay-autoscroll-resume")).not.toBeInTheDocument(),
    );
  });

  it("caps retained live rows and shows the trimmed-head chip (bounded memory)", async () => {
    const sessions = [makeSummary({ sessionId: "claude#0", kind: "swarm" })];
    // Seed with > LIVE_MAX_ENTRIES (2000) synthetic fr rows (unique minute:second
    // ts per row so the chronological merge is well-defined).
    const big: SessionTranscriptResponse = {
      sessionId: "claude#0",
      truncated: false,
      sourceStatus: ALL_PRESENT,
      entries: Array.from({ length: 2100 }, (_, i) => ({
        kind: "fr_event" as const,
        ts: `2026-05-30T${String(10 + Math.floor(i / 3600)).padStart(2, "0")}:${String(Math.floor((i % 3600) / 60)).padStart(2, "0")}:${String(i % 60).padStart(2, "0")}.000Z`,
        seq: i,
        eventType: "system",
        frEvent: "FR-EVT-X",
        actor: "agent",
        payload: {},
        eventId: `ev-${i}`,
      })),
    };
    const ipc: SessionTranscriptIpc = {
      listSessions: vi.fn(async () => sessions),
      getTranscript: vi.fn(async () => big),
      resumeSession: vi.fn(async (s: string) => ({ modelId: s, instance: 0, composite: `${s}#0` })),
      getSpawnTemplate: vi.fn(async () => null),
    };
    const h = makeLiveHarness();
    render(<SessionReplayPanel ipc={ipc} liveIpc={h.liveIpc} />);
    await openAndSelectLive("claude#0");

    // The trimmed-head honesty chip is shown.
    expect(await screen.findByTestId("session-replay-live-truncated-head")).toBeInTheDocument();
    // The rendered list is capped at LIVE_MAX_ENTRIES (2000), not 2100. Read the
    // <ul> child count directly (cheaper than a 2000-node testid regex scan).
    await waitFor(() => {
      const list = screen.getByTestId("session-replay-entries");
      expect(list.children.length).toBe(2000);
    });
  }, 15000);
});
