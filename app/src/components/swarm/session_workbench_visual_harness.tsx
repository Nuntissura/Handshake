// Real-component mount harness for the Playwright visual matrix (governance
// glue #3: lift operator chat to ALL providers + the combined "Session"
// workbench — chat + captured terminal + transcript link).
//
// Mounts the GENUINE <SwarmOperatorSurface> React component (the same module
// shipped in the app) into a real Chromium page via react-dom — NOT a mockup.
// The only stand-in is a deterministic Tauri IPC mock (mockIPC) so every swarm /
// terminal / session-transcript command the surface (and its SessionWorkbench,
// shared TerminalPanel, and SessionReplayPanel drawers) calls resolves without a
// live backend. Everything the operator sees — the provider-rich chat picker
// (local + cloud + CLI), the real chat generate, the "Show captured terminal"
// focus into the single shared TerminalPanel, and the "Open full transcript"
// reveal — is the REAL component's own render output.
//
// The session list deliberately includes one LOCAL, one BYOK_CLOUD, and one
// OFFICIAL_CLI session (all READY = chattable) plus one non-live CANCELLED
// session, so the visual matrix can assert all three providers appear AND that
// the honest disabled state renders.
import { createElement as h } from "react";
import { createRoot } from "react-dom/client";
import { mockIPC } from "@tauri-apps/api/mocks";

import { SwarmOperatorSurface } from "./SwarmOperatorSurface";

interface SwarmSessionLike {
  instanceId: { modelId: string; instance: number; composite: string };
  state: string;
  provider: string;
  runtimeBinding: string;
  artifactPath: string | null;
  cloudModelName: string | null;
  worktreeId: string | null;
  workingDir: string | null;
}

const SESSIONS: SwarmSessionLike[] = [
  {
    instanceId: { modelId: "alpha-model", instance: 0, composite: "alpha-model#0" },
    state: "READY",
    provider: "local",
    runtimeBinding: "candle",
    artifactPath: "D:/models/alpha/model.safetensors",
    cloudModelName: null,
    worktreeId: "wt-feature-x",
    workingDir: "D:/work/wt-feature-x",
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
    worktreeId: "wt-cli-lane",
    workingDir: null,
  },
  {
    instanceId: { modelId: "delta-dead", instance: 0, composite: "delta-dead#0" },
    state: "CANCELLED",
    provider: "byok_cloud",
    runtimeBinding: "cloud",
    artifactPath: null,
    cloudModelName: "gpt-4o",
    worktreeId: null,
    workingDir: null,
  },
];

// One captured terminal session keyed by the cloud session's composite
// instance_id, so "Show captured terminal" can resolve a real tab via the new
// focusInstanceId binding path.
const TERMINAL_SESSIONS = [
  {
    sessionId: "cap-beta",
    sessionType: "AiJob",
    swarmId: null,
    worktreeId: null,
    instanceId: "beta-cloud#0",
    title: "swarm:beta-cloud#0",
    exited: false,
    exitCode: null,
    interactiveAllowed: false,
  },
];

// Install a deterministic IPC mock for every command the surface can call.
mockIPC((cmd: string) => {
  switch (cmd) {
    case "kernel_swarm_list_active_sessions":
      return SESSIONS;
    case "kernel_swarm_list_worktrees":
      return [
        { worktreeId: "wt-feature-x", liveSessionCount: 1 },
        { worktreeId: "wt-cli-lane", liveSessionCount: 1 },
      ];
    case "kernel_swarm_resource_snapshot":
      return {
        concurrencyCap: 4,
        concurrencyInUse: 3,
        concurrencyAvailable: 1,
        liveSessions: 3,
        lifetimeSpawnsRemaining: 100,
        tokensRemaining: null,
        costMicrosRemaining: null,
        committedMemoryBytesRemaining: 6 * 1024 * 1024 * 1024,
        committedMemoryBytesCap: 12 * 1024 * 1024 * 1024,
        budgetExhausted: false,
      };
    case "kernel_swarm_board_snapshot":
      return { cards: [], liveSessions: 3 };
    case "kernel_swarm_chat_generate":
      // Deterministic real-shaped chat response (tokens come from the backend in
      // production; here the mock stands in only so the surface stays offline).
      return { text: "Hello from the cloud session.", tokenCount: 6, finishReason: "stop" };
    case "kernel_swarm_spawn_session":
      return { modelId: "spawned-model", instance: 0, composite: "spawned-model#0" };
    case "kernel_swarm_cancel_session":
      return null;
    case "kernel_terminal_list_sessions":
      return TERMINAL_SESSIONS;
    case "kernel_terminal_scrollback":
      return { sessionId: "cap-beta", seq: 0, chunkBase64: "", truncated: false };
    case "kernel_session_list":
      return SESSIONS.map((s) => ({
        sessionId: s.instanceId.composite,
        kind: "swarm",
        startedAt: "2026-05-30T10:00:00.000Z",
        lastActivityAt: "2026-05-30T10:06:00.000Z",
        modelId: s.instanceId.composite,
        provider: s.provider,
        title: s.cloudModelName ?? s.instanceId.modelId,
        counts: { chat: 1, fr: 0, terminal: 1, process: 0 },
      }));
    case "kernel_session_transcript_get":
      return {
        sessionId: "beta-cloud#0",
        truncated: false,
        sourceStatus: { chat: "present", fr: "empty", terminal: "present", process: "empty" },
        entries: [
          { kind: "chat_turn", ts: "2026-05-30T10:00:00.000Z", seq: 0, role: "operator", content: "Hello cloud.", messageId: "m1" },
        ],
      };
    default:
      return null;
  }
});

const root = document.getElementById("harness-root");
if (root) {
  createRoot(root).render(h(SwarmOperatorSurface));
}

export {};
