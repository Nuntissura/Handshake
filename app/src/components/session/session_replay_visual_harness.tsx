// Real-component mount harness for the Playwright visual matrix.
//
// Mounts the GENUINE <SessionReplayPanel> React component (the same module
// shipped in the app) into a real Chromium page via react-dom — NOT a mockup.
// The only stand-in is a deterministic in-memory SessionTranscriptIpc supplying
// one populated session and one empty session so the layout is stable for the
// PNG baseline. Everything else (the Disclosure host, lazy/collapsed gating, the
// session index, the consolidated typed timeline, the kind-filter bar, the
// honest empty/unavailable states) is the REAL component's own render output.
//
// The spec chooses the mount mode via window.__HARNESS_MODE__ set BEFORE the
// bundle runs ("open" = expanded for readability/baseline; "collapsed" = prove
// the genuine collapsed-by-default + lazy gate). When opened, the harness clicks
// the first (populated) session so the consolidated timeline is visible.
import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { SessionReplayPanel } from "./SessionReplayPanel";
import type {
  LiveTailIpc,
  SessionSummary,
  SessionTranscriptEntry,
  SessionTranscriptIpc,
  SessionTranscriptResponse,
  TranscriptGetRequest,
} from "../../lib/ipc/session_transcript";
import type { SwarmBoardDelta } from "../../lib/ipc/swarm_runtime";
import type { TerminalSubscription } from "../../lib/ipc/terminal";

const SESSIONS: SessionSummary[] = [
  {
    sessionId: "claude-sonnet#0",
    kind: "swarm",
    startedAt: "2026-05-30T10:00:00.000Z",
    lastActivityAt: "2026-05-30T10:06:00.000Z",
    modelId: "claude-sonnet#0",
    provider: "cloud",
    title: "build handshake_core",
    counts: { chat: 1, fr: 2, terminal: 1, process: 1 },
  },
  {
    sessionId: "local-qwen#1",
    kind: "agent",
    startedAt: "2026-05-30T11:00:00.000Z",
    lastActivityAt: "2026-05-30T11:00:00.000Z",
    modelId: "local-qwen#1",
    provider: "local",
    title: "empty session",
    counts: { chat: 0, fr: 0, terminal: 0, process: 0 },
  },
];

const POPULATED: SessionTranscriptResponse = {
  sessionId: "claude-sonnet#0",
  truncated: false,
  sourceStatus: { chat: "present", fr: "present", terminal: "present", process: "present" },
  entries: [
    { kind: "chat_turn", ts: "2026-05-30T10:00:00.000Z", seq: 0, role: "operator", content: "Build handshake_core and report the gates.", messageId: "m1" },
    { kind: "agent_activity", ts: "2026-05-30T10:00:20.000Z", seq: 1, activityKind: "thinking", text: "I'll compile the crate first, then read the gate output before reporting.", eventId: "FR-EVT-AGENT-THINKING" },
    { kind: "agent_activity", ts: "2026-05-30T10:00:40.000Z", seq: 2, activityKind: "tool_call", name: "Bash", detail: { command: "cargo build --lib", cwd: "src/backend/handshake_core" }, eventId: "FR-EVT-AGENT-TOOLCALL" },
    { kind: "agent_activity", ts: "2026-05-30T10:00:55.000Z", seq: 3, activityKind: "text", text: "Build succeeded; all gates green.", eventId: "FR-EVT-AGENT-TEXT" },
    { kind: "agent_activity", ts: "2026-05-30T10:00:58.000Z", seq: 4, activityKind: "other", text: "{\"type\":\"future_unknown_event\",\"foo\":1}", eventId: "FR-EVT-AGENT-OTHER" },
    { kind: "terminal_chunk", ts: "2026-05-30T10:01:00.000Z", seq: 5, terminalSessionId: "term-1", command: "cargo build --lib", text: "compiling handshake_core ... ok" },
    { kind: "fr_event", ts: "2026-05-30T10:02:00.000Z", seq: 6, eventType: "llm_inference", frEvent: "FR-EVT-LLM-INFER-END", actor: "agent", modelId: "claude-sonnet#0", payload: { tokens: 184, request_id: "req-7" }, eventId: "e1" },
    { kind: "process", ts: "2026-05-30T10:03:00.000Z", seq: 7, processUuid: "proc-9f2a", phase: "completed", modelId: "claude-sonnet#0", payload: { exit_code: 0 } },
  ],
};

const EMPTY: SessionTranscriptResponse = {
  sessionId: "local-qwen#1",
  truncated: false,
  sourceStatus: { chat: "empty", fr: "empty", terminal: "empty", process: "empty" },
  entries: [],
};

const fakeIpc: SessionTranscriptIpc = {
  listSessions: async () => SESSIONS,
  getTranscript: async (req: TranscriptGetRequest) =>
    req.sessionId === "local-qwen#1" ? EMPTY : POPULATED,
};

// ---------------------------------------------------------------------------
// Live mode harness. A growing-tail IPC + a scripted live seam push two ticks
// (a toolcall fr_event, then a terminal_chunk) shortly after mount so the
// Playwright baseline captures the REAL panel in a live-tailing state (the live
// status chip + appended rows). The streaming session "claude-sonnet#0" is a
// swarm kind, so Live defaults ON.
// ---------------------------------------------------------------------------

const LIVE_BASE: SessionTranscriptEntry[] = POPULATED.entries.slice();
// Extra rows the tail re-fetch reveals after the scripted ticks fire.
const LIVE_TICK_ROWS: SessionTranscriptEntry[] = [
  { kind: "fr_event", ts: "2026-05-30T10:04:00.000Z", seq: 0, eventType: "llm_inference", frEvent: "FR-EVT-LLM-INFER-START", actor: "agent", modelId: "claude-sonnet#0", payload: { request_id: "req-8" }, eventId: "live-1" },
  { kind: "terminal_chunk", ts: "2026-05-30T10:04:05.000Z", seq: 0, terminalSessionId: "term-1", command: "cargo test --lib", text: "running 42 tests ... ok" },
];
let liveRevealed = 0;

const liveGrowingIpc: SessionTranscriptIpc = {
  listSessions: async () => SESSIONS,
  getTranscript: async (req: TranscriptGetRequest) => {
    if (req.sessionId === "local-qwen#1") return EMPTY;
    // The tail re-fetch returns the base rows plus whatever ticks have fired.
    const entries = LIVE_BASE.concat(LIVE_TICK_ROWS.slice(0, liveRevealed));
    return { ...POPULATED, entries };
  },
};

// A scripted live seam: on subscribe, schedule two ticks that reveal a row and
// then poke the corresponding stream so the panel tail-fetches it.
const liveSeam: LiveTailIpc = {
  subscribeBoardEvents: async (
    onDelta: (d: SwarmBoardDelta) => void,
    _onResync: (n: number) => void,
  ) => {
    void _onResync;
    const id = { model_id: "claude-sonnet", instance: 0 };
    window.setTimeout(() => {
      liveRevealed = 1;
      onDelta({ seq: 1, event: { SessionStateChanged: { instance_id: id, from: "READY", to: "GENERATING" } } });
    }, 400);
    return () => {};
  },
  subscribeTerminal: async (sub: TerminalSubscription) => {
    window.setTimeout(() => {
      liveRevealed = 2;
      sub.onOutput("term-1", new Uint8Array());
    }, 800);
    return () => {};
  },
  listTerminalSessions: async () => [
    {
      sessionId: "term-1",
      sessionType: "AiJob" as const,
      swarmId: null,
      worktreeId: null,
      instanceId: "claude-sonnet#0",
      title: "capture",
      exited: false,
      exitCode: null,
      interactiveAllowed: false,
    },
  ],
};

// Mount mode is chosen by the spec via window.__HARNESS_MODE__ set BEFORE this
// bundle runs:
//   "open"      -> render the real panel expanded (readability/baseline)
//   "collapsed" -> render the real panel collapsed-by-default (lazy gate)
//   "live"      -> render expanded with a scripted live seam (live tailing)
// We read it with a LOCAL cast rather than augmenting the global Window type, so
// this harness does not collide with the terminal harness's own __HARNESS_MODE__
// global augmentation (which has a narrower union) at typecheck time.
type HarnessMode = "open" | "collapsed" | "live";

// A safe no-op live seam for the non-live modes: this harness runs in a plain
// Chromium page with NO Tauri runtime, so the real defaultLiveTailIpc (which
// calls Tauri `listen`) is unavailable. The populated session is a swarm kind
// (Live defaults ON), so even the "open" baseline needs an injected seam — it
// simply never pushes, leaving the panel in a quiescent live-tailing state.
const noopLiveSeam: LiveTailIpc = {
  subscribeBoardEvents: async () => () => {},
  subscribeTerminal: async () => () => {},
  listTerminalSessions: async () => [],
};

const root = document.getElementById("harness-root");
if (root) {
  const mode = ((window as unknown as { __HARNESS_MODE__?: HarnessMode }).__HARNESS_MODE__ ?? "open") as HarnessMode;
  const open = mode !== "collapsed";
  createRoot(root).render(
    h(SessionReplayPanel, {
      ipc: mode === "live" ? liveGrowingIpc : fakeIpc,
      liveIpc: mode === "live" ? liveSeam : noopLiveSeam,
      defaultOpen: open,
    }),
  );
}

export {};
