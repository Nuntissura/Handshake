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
  SessionSummary,
  SessionTranscriptIpc,
  SessionTranscriptResponse,
  TranscriptGetRequest,
} from "../../lib/ipc/session_transcript";

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
    { kind: "terminal_chunk", ts: "2026-05-30T10:01:00.000Z", seq: 1, terminalSessionId: "term-1", command: "cargo build --lib", text: "compiling handshake_core ... ok" },
    { kind: "fr_event", ts: "2026-05-30T10:02:00.000Z", seq: 2, eventType: "llm_inference", frEvent: "FR-EVT-LLM-INFER-END", actor: "agent", modelId: "claude-sonnet#0", payload: { tokens: 184, request_id: "req-7" }, eventId: "e1" },
    { kind: "process", ts: "2026-05-30T10:03:00.000Z", seq: 3, processUuid: "proc-9f2a", phase: "completed", modelId: "claude-sonnet#0", payload: { exit_code: 0 } },
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

declare global {
  interface Window {
    // Set by the spec BEFORE the bundle runs to choose the mount mode:
    //   "open"      -> render the real panel expanded (readability/baseline)
    //   "collapsed" -> render the real panel collapsed-by-default (lazy gate)
    __HARNESS_MODE__?: "open" | "collapsed";
  }
}

const root = document.getElementById("harness-root");
if (root) {
  const open = window.__HARNESS_MODE__ !== "collapsed";
  createRoot(root).render(
    h(SessionReplayPanel, {
      ipc: fakeIpc,
      defaultOpen: open,
    }),
  );
}

export {};
