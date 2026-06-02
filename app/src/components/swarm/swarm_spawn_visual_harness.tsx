// Real-component mount harness for the Playwright visual matrix (governance
// glue #2: worktree / disk-location / isolation-tier ASSIGNMENT on swarm spawn).
//
// Mounts the GENUINE <SwarmControlRoom> React component (the same module shipped
// in the app) into a real Chromium page via react-dom — NOT a mockup. The only
// stand-in is a deterministic Tauri IPC mock (mockIPC) so the swarm commands
// resolve without a live backend: listWorktrees returns one discovered
// worktree, listActiveSessions returns one assigned + one unassigned session,
// resourceSnapshot returns a healthy budget, and spawnSession echoes a composite
// id. Everything else (the worktree picker, the new-entry reveal, the disk
// working-dir field, the recorded-only isolation-tier selector + its honesty
// note, the sessions-table Worktree column) is the REAL component's render.
import { createElement as h } from "react";
import { createRoot } from "react-dom/client";
import { mockIPC } from "@tauri-apps/api/mocks";

import { SwarmControlRoom } from "./SwarmControlRoom";

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
    state: "GENERATING",
    provider: "byok_cloud",
    runtimeBinding: "cloud",
    artifactPath: null,
    cloudModelName: "gpt-4o",
    worktreeId: null,
    workingDir: null,
  },
];

// Install a deterministic IPC mock for the swarm commands the control room calls.
mockIPC((cmd: string) => {
  switch (cmd) {
    case "kernel_swarm_list_worktrees":
      return [
        { worktreeId: "wt-feature-x", liveSessionCount: 1 },
        { worktreeId: "wt-audit", liveSessionCount: 0 },
      ];
    case "kernel_swarm_list_active_sessions":
      return SESSIONS;
    case "kernel_swarm_resource_snapshot":
      return {
        concurrencyCap: 4,
        concurrencyInUse: 2,
        concurrencyAvailable: 2,
        liveSessions: 2,
        lifetimeSpawnsRemaining: 100,
        tokensRemaining: null,
        costMicrosRemaining: null,
        committedMemoryBytesRemaining: 8 * 1024 * 1024 * 1024,
        committedMemoryBytesCap: 16 * 1024 * 1024 * 1024,
        budgetExhausted: false,
      };
    case "kernel_swarm_spawn_session":
      return { modelId: "spawned-model", instance: 0, composite: "spawned-model#0" };
    case "kernel_swarm_cancel_session":
      return null;
    default:
      return null;
  }
});

const root = document.getElementById("harness-root");
if (root) {
  createRoot(root).render(h(SwarmControlRoom));
}

export {};
