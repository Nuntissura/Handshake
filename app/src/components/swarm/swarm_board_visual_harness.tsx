// Real-component mount harness for the live SwarmBoard Playwright smoke.
//
// Mounts the GENUINE <SwarmBoard> component into Chromium. The only stand-ins
// are deterministic Tauri IPC/event mocks plus an injected TerminalIpc so the
// board can render stable swarm/worktree lanes without a live backend.
import { createRoot } from "react-dom/client";
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";

import { SwarmBoard } from "./SwarmBoard";
import type {
  TerminalIpc,
  TerminalSession,
  TerminalSubscription,
} from "../../lib/ipc/terminal";

declare global {
  interface Window {
    __HS_SWARM_BOARD_EMIT_READY__?: () => Promise<void>;
    __HS_SWARM_BOARD_INSPECTED__?: string | null;
    __HS_SWARM_BOARD_REVIEWED__?: string | null;
  }
}

const BOARD_CARDS = [
  {
    instanceId: { modelId: "alpha-local", instance: 0, composite: "alpha-local#0" },
    state: "LOADING",
    provider: "local",
    runtimeBinding: "microvm",
    swarmId: "swarm-alpha",
    worktreeId: "wt-alpha",
  },
  {
    instanceId: { modelId: "alpha-cloud", instance: 0, composite: "alpha-cloud#0" },
    state: "GENERATING",
    provider: "byok_cloud",
    runtimeBinding: "cloud",
    swarmId: "swarm-alpha",
    worktreeId: "wt-alpha",
  },
  {
    instanceId: { modelId: "queued-local", instance: 0, composite: "queued-local#0" },
    state: "QUEUED",
    provider: "local",
    runtimeBinding: "candle",
    swarmId: "swarm-alpha",
    worktreeId: "wt-alpha",
  },
  {
    instanceId: { modelId: "beta-review", instance: 0, composite: "beta-review#0" },
    state: "FAILED",
    provider: "local",
    runtimeBinding: "llama_cpp",
    swarmId: null,
    worktreeId: "wt-beta",
  },
];

const TERMINAL_SESSIONS: TerminalSession[] = [
  {
    sessionId: "term-alpha-1",
    sessionType: "AiJob",
    swarmId: "swarm-alpha",
    worktreeId: "wt-alpha",
    instanceId: "alpha-local#0",
    title: "swarm-alpha capture",
    exited: false,
    exitCode: null,
    interactiveAllowed: false,
  },
];

const terminalIpc: TerminalIpc = {
  async getContext() {
    return { cwd: "D:/repo", defaultShell: null };
  },
  async createSession() {
    return TERMINAL_SESSIONS[0];
  },
  async authorizeInteractive() {},
  async writeStdin() {},
  async resizeSession() {},
  async closeSession() {},
  async listSessions() {
    return TERMINAL_SESSIONS;
  },
  async scrollback(sessionId: string) {
    return { sessionId, seq: 0, chunkBase64: "", truncated: false };
  },
  async subscribe(_sub: TerminalSubscription) {
    return () => {};
  },
};

mockIPC((cmd: string) => {
  switch (cmd) {
    case "kernel_swarm_board_snapshot":
      return { cards: BOARD_CARDS, liveSessions: BOARD_CARDS.length };
    case "kernel_swarm_cancel_session":
      return null;
    default:
      return null;
  }
}, { shouldMockEvents: true });

window.__HS_SWARM_BOARD_INSPECTED__ = null;
window.__HS_SWARM_BOARD_REVIEWED__ = null;
let deltaSeq = 0;
window.__HS_SWARM_BOARD_EMIT_READY__ = async () => {
  await emit("swarm://event", {
    seq: ++deltaSeq,
    event: {
      SessionReady: {
        instance_id: { model_id: "alpha-local", instance: 0 },
      },
    },
  });
};

const root = document.getElementById("harness-root");
if (root) {
  createRoot(root).render(
    <SwarmBoard
      terminalIpc={terminalIpc}
      onInspectTerminal={(swarmId: string) => {
        window.__HS_SWARM_BOARD_INSPECTED__ = swarmId;
      }}
      onReviewSession={(instanceId: string) => {
        window.__HS_SWARM_BOARD_REVIEWED__ = instanceId;
      }}
    />,
  );
}

export {};
