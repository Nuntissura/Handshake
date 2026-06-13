import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, test, vi } from "vitest";

// Governance glue #2: the spawn form lets the operator ASSIGN a session to a
// VM/sandbox worktree and/or a disk location (recorded attribution only, no
// execution routing). We mock only the IPC CALLS of the swarm runtime module —
// the real hook + form components render. This mirrors SwarmOperatorSurface.test.

const spawnSessionMock = vi.fn(async (_request: Record<string, unknown>) => ({
  modelId: "m",
  instance: 0,
  composite: "m#0",
}));
const spawnLocalCloudPairMock = vi.fn(async (_request: Record<string, unknown>) => ({
  local: {
    provider: "local",
    instanceId: { modelId: "local-m", instance: 0, composite: "local-m#0" },
    error: null,
  },
  cloud: {
    provider: "byok_cloud",
    instanceId: { modelId: "cloud-m", instance: 0, composite: "cloud-m#0" },
    error: null,
  },
}));
const spawnWithCloudEscalationMock = vi.fn(async (_request: Record<string, unknown>) => ({
  selected: "cloud",
  escalated: true,
  escalationReason: "SWARM_CONCURRENCY_CAP_REACHED: 4 of 4 permits in use",
  local: {
    provider: "local",
    instanceId: null,
    error: "SWARM_CONCURRENCY_CAP_REACHED: 4 of 4 permits in use",
  },
  cloud: {
    provider: "byok_cloud",
    instanceId: { modelId: "cloud-m", instance: 0, composite: "cloud-m#0" },
    error: null,
  },
}));
const chatGenerateWithCloudEscalationMock = vi.fn(async (_request: Record<string, unknown>) => ({
  selected: "cloud",
  escalated: true,
  escalationReason: "local overloaded",
  localError: null,
  local: null,
  cloudInstance: { modelId: "cloud-m", instance: 0, composite: "cloud-m#0" },
  cloud: { text: "cloud answer", tokenCount: 2, finishReason: "stop" },
  cloudAssistanceReceipt: null,
  cloudError: null,
}));
const listWorktreesMock = vi.fn(async () => [
  { worktreeId: "wt-existing", liveSessionCount: 2 },
]);
const listActiveSessionsMock = vi.fn(async () => [] as unknown[]);

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
      committedMemoryBytesRemaining: 8 * 1024 * 1024 * 1024,
      committedMemoryBytesCap: 16 * 1024 * 1024 * 1024,
      budgetExhausted: false,
    })),
    listActiveSessions: () => listActiveSessionsMock(),
    listWorktrees: () => listWorktreesMock(),
    spawnSession: (request: Record<string, unknown>) => spawnSessionMock(request),
    spawnLocalCloudPair: (request: Record<string, unknown>) => spawnLocalCloudPairMock(request),
    spawnWithCloudEscalation: (request: Record<string, unknown>) =>
      spawnWithCloudEscalationMock(request),
    chatGenerateWithCloudEscalation: (request: Record<string, unknown>) =>
      chatGenerateWithCloudEscalationMock(request),
    cancelSession: vi.fn(),
    chatGenerate: vi.fn(),
  };
});

import {
  SwarmControlRoom,
  committedMemoryMiBToBytes,
  committedMemorySubmitBytes,
  effectiveWorktreeId,
  parseWarmVmRestoreManifestJson,
  swarmBudgetExhaustionLabel,
  swarmResourceBadge,
  NEW_WORKTREE_SENTINEL,
} from "./SwarmControlRoom";

beforeEach(() => {
  spawnSessionMock.mockClear();
  spawnLocalCloudPairMock.mockClear();
  spawnWithCloudEscalationMock.mockClear();
  chatGenerateWithCloudEscalationMock.mockClear();
  listWorktreesMock.mockClear();
  listActiveSessionsMock.mockClear();
  listActiveSessionsMock.mockResolvedValue([]);
});

/** Last argument the spawn IPC was called with (the SwarmSpawnRequest). */
function lastSpawnRequest() {
  const calls = spawnSessionMock.mock.calls;
  expect(calls.length).toBeGreaterThan(0);
  return calls[calls.length - 1][0] as Record<string, unknown>;
}

function warmRestoreManifest(worktreeId = "wt-existing") {
  return {
    protocol_id: "hsk.warm_agent",
    protocol_version: 1,
    worktree_id: worktreeId,
    model_artifact_sha256: "ab".repeat(32),
    model_guest_path: "/models/tiny.gguf",
    ready_nonce: "ready-nonce",
    snapshot: {
      id: "018f3b8e-1111-7111-8111-111111111111",
      adapter_id: "cloud_hypervisor",
      snapshot_dir: "/snapshots/wt-existing",
      created_at_utc: "2026-06-02T00:00:00Z",
      observe_path: null,
    },
  };
}

describe("effectiveWorktreeId", () => {
  test("uses the free-text value when the new-worktree sentinel is selected", () => {
    expect(effectiveWorktreeId(NEW_WORKTREE_SENTINEL, "  wt-new  ")).toBe("wt-new");
  });
  test("uses the picked existing id otherwise, trimmed", () => {
    expect(effectiveWorktreeId("wt-existing", "ignored")).toBe("wt-existing");
  });
  test("blank => empty (unassigned, honest)", () => {
    expect(effectiveWorktreeId("", "")).toBe("");
    expect(effectiveWorktreeId(NEW_WORKTREE_SENTINEL, "   ")).toBe("");
  });
});

describe("committedMemoryMiBToBytes", () => {
  test("converts positive MiB strings to byte estimates", () => {
    expect(committedMemoryMiBToBytes("6144")).toBe(6 * 1024 * 1024 * 1024);
  });

  test("blank and invalid values omit the estimate", () => {
    expect(committedMemoryMiBToBytes("")).toBeUndefined();
    expect(committedMemoryMiBToBytes("0")).toBeUndefined();
    expect(committedMemoryMiBToBytes("-1")).toBeUndefined();
    expect(committedMemoryMiBToBytes("not-a-number")).toBeUndefined();
  });

  test("preserves exact template bytes when the prefilled MiB value is unchanged", () => {
    expect(committedMemorySubmitBytes("118", 123456789)).toBe(123456789);
  });
});

describe("parseWarmVmRestoreManifestJson", () => {
  test("parses the core snake_case manifest shape and rejects non-objects", () => {
    const manifest = warmRestoreManifest();
    expect(parseWarmVmRestoreManifestJson(JSON.stringify(manifest))).toEqual(manifest);
    expect(parseWarmVmRestoreManifestJson("   ")).toBeUndefined();
    expect(() => parseWarmVmRestoreManifestJson("[]")).toThrow(/JSON object/);
  });
});

describe("swarm budget labels", () => {
  const baseSnapshot = {
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
  };

  test("committed-memory-only exhaustion is local-lane specific", () => {
    expect(swarmBudgetExhaustionLabel(baseSnapshot)).toBe(
      "Local committed memory exhausted - local spawns are blocked; cloud lanes remain available",
    );
    expect(swarmResourceBadge(baseSnapshot)).toBe("local memory exhausted");
  });

  test("global budget exhaustion still reports all spawns blocked", () => {
    const tokenExhausted = { ...baseSnapshot, tokensRemaining: 0 };
    expect(swarmBudgetExhaustionLabel(tokenExhausted)).toBe(
      "Budget exhausted - spawns are blocked",
    );
    expect(swarmResourceBadge(tokenExhausted)).toBe("budget exhausted");
  });
});

describe("SwarmControlRoom spawn-form assignment controls", () => {
  test("the worktree picker lists discovered worktrees + a new-entry option, and the isolation honesty note is shown", async () => {
    render(<SwarmControlRoom />);
    const select = (await screen.findByTestId(
      "swarm-spawn-worktree-select",
    )) as HTMLSelectElement;
    // Discovered worktree from listWorktrees() is offered as an option.
    await waitFor(() =>
      expect(
        within(select).queryByRole("option", { name: /wt-existing/ }),
      ).toBeInTheDocument(),
    );
    // Plus the unassigned + new-entry options.
    expect(within(select).getByRole("option", { name: /Unassigned/ })).toBeInTheDocument();
    expect(within(select).getByRole("option", { name: /New worktree/ })).toBeInTheDocument();

    // The mandatory Tier3 scope honesty note is present.
    expect(screen.getByTestId("swarm-isolation-note")).toHaveTextContent(
      /Tier3 local llama\.cpp uses cold microVM unless Warm VM is selected/i,
    );
    expect(screen.getByTestId("swarm-isolation-note")).toHaveTextContent(
      /resident guest agent support/i,
    );
    // The disk working-dir field is optional and present.
    expect(screen.getByTestId("swarm-spawn-working-dir")).toBeInTheDocument();
    expect(screen.getByTestId("swarm-spawn-committed-memory-mib")).toBeInTheDocument();
    expect(screen.getByTestId("swarm-stat-committed-memory")).toHaveTextContent(
      /Committed memory: 8\.0 GiB \/ 16\.0 GiB remaining/i,
    );
  });

  test("selecting '+ New worktree…' reveals a free-text input whose value is threaded onto the spawn request", async () => {
    render(<SwarmControlRoom />);
    const select = await screen.findByTestId("swarm-spawn-worktree-select");

    // The new-name input is hidden until the sentinel is chosen.
    expect(screen.queryByTestId("swarm-spawn-worktree-new")).toBeNull();
    fireEvent.change(select, { target: { value: NEW_WORKTREE_SENTINEL } });
    const newInput = await screen.findByTestId("swarm-spawn-worktree-new");
    fireEvent.change(newInput, { target: { value: "  wt-brand-new  " } });

    // Supply a cloud model so the spawn is well-formed, then submit.
    fireEvent.change(screen.getByTestId("swarm-spawn-provider"), {
      target: { value: "byok_cloud" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-cloud-model"), {
      target: { value: "gpt-4o" },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await waitFor(() => expect(spawnSessionMock).toHaveBeenCalled());
    expect(lastSpawnRequest().worktreeId).toBe("wt-brand-new");
  });

  test("an existing worktree pick + working dir + isolation tier are threaded (trimmed) onto the spawn request", async () => {
    render(<SwarmControlRoom />);
    const select = await screen.findByTestId("swarm-spawn-worktree-select");
    await waitFor(() =>
      expect(
        within(select as HTMLElement).queryByRole("option", { name: /wt-existing/ }),
      ).toBeInTheDocument(),
    );
    fireEvent.change(select, { target: { value: "wt-existing" } });
    fireEvent.change(screen.getByTestId("swarm-spawn-working-dir"), {
      target: { value: "  D:/work/wt-foo  " },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-isolation-tier"), {
      target: { value: "tier3_microvm" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-committed-memory-mib"), {
      target: { value: "6144" },
    });

    fireEvent.change(screen.getByTestId("swarm-spawn-artifact-path"), {
      target: { value: "D:/models/tiny.safetensors" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-sha256"), {
      target: { value: "ab".repeat(32) },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await waitFor(() => expect(spawnSessionMock).toHaveBeenCalled());
    const req = lastSpawnRequest();
    expect(req.provider).toBe("local");
    expect(req.worktreeId).toBe("wt-existing");
    expect(req.workingDir).toBe("D:/work/wt-foo");
    expect(req.isolationTier).toBe("tier3_microvm");
    expect(req.committedMemoryBytes).toBe(6 * 1024 * 1024 * 1024);
  });

  test("cloud-only spawn hides and omits the local committed-memory estimate", async () => {
    render(<SwarmControlRoom />);
    await screen.findByTestId("swarm-spawn-worktree-select");
    expect(screen.getByTestId("swarm-spawn-committed-memory-mib")).toBeInTheDocument();
    fireEvent.change(screen.getByTestId("swarm-spawn-committed-memory-mib"), {
      target: { value: "6144" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-provider"), {
      target: { value: "byok_cloud" },
    });
    expect(screen.queryByTestId("swarm-spawn-committed-memory-mib")).toBeNull();
    fireEvent.change(screen.getByTestId("swarm-spawn-cloud-model"), {
      target: { value: "gpt-4o" },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await waitFor(() => expect(spawnSessionMock).toHaveBeenCalled());
    const req = lastSpawnRequest();
    expect(req.provider).toBe("byok_cloud");
    expect("committedMemoryBytes" in req).toBe(false);
  });

  test("warm VM spawn sends parsed restore manifest when provided", async () => {
    render(<SwarmControlRoom />);
    await screen.findByTestId("swarm-spawn-worktree-select");
    const manifest = warmRestoreManifest();

    fireEvent.change(screen.getByTestId("swarm-spawn-binding"), {
      target: { value: "llama_cpp" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-local-execution-mode"), {
      target: { value: "warm_vm" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-artifact-path"), {
      target: { value: "D:/models/tiny.gguf" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-sha256"), {
      target: { value: "ab".repeat(32) },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-worktree-select"), {
      target: { value: "wt-existing" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-isolation-tier"), {
      target: { value: "tier3_microvm" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-warm-restore-manifest"), {
      target: { value: JSON.stringify(manifest) },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await waitFor(() => expect(spawnSessionMock).toHaveBeenCalled());
    const req = lastSpawnRequest();
    expect(req.localExecutionMode).toBe("warm_vm");
    expect(req.warmVmRestoreManifest).toEqual(manifest);
  });

  test("invalid warm VM restore manifest JSON fails before IPC", async () => {
    render(<SwarmControlRoom />);
    await screen.findByTestId("swarm-spawn-worktree-select");

    fireEvent.change(screen.getByTestId("swarm-spawn-local-execution-mode"), {
      target: { value: "warm_vm" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-warm-restore-manifest"), {
      target: { value: "[]" },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await screen.findByTestId("swarm-spawn-error");
    expect(spawnSessionMock).not.toHaveBeenCalled();
    expect(screen.getByTestId("swarm-spawn-error")).toHaveTextContent(/JSON object/);
  });

  test("blank assignment omits worktree/workingDir/isolationTier (honest unassigned)", async () => {
    render(<SwarmControlRoom />);
    await screen.findByTestId("swarm-spawn-worktree-select");
    fireEvent.change(screen.getByTestId("swarm-spawn-provider"), {
      target: { value: "byok_cloud" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-cloud-model"), {
      target: { value: "gpt-4o" },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await waitFor(() => expect(spawnSessionMock).toHaveBeenCalled());
    const req = lastSpawnRequest();
    expect("worktreeId" in req).toBe(false);
    expect("workingDir" in req).toBe(false);
    expect("isolationTier" in req).toBe(false);
    expect("committedMemoryBytes" in req).toBe(false);
    expect("swarmId" in req).toBe(false);
  });

  test("local+cloud pair workflow sends both lane requests with shared assignment", async () => {
    render(<SwarmControlRoom />);
    await screen.findByTestId("swarm-spawn-worktree-select");

    fireEvent.change(screen.getByTestId("swarm-spawn-workflow"), {
      target: { value: "local_cloud_pair" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-binding"), {
      target: { value: "llama_cpp" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-local-execution-mode"), {
      target: { value: "warm_vm" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-artifact-path"), {
      target: { value: "D:/models/tiny.gguf" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-sha256"), {
      target: { value: "ab".repeat(32) },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-cloud-provider"), {
      target: { value: "official_cli" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-cloud-model"), {
      target: { value: "claude-sonnet-4" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-worktree-select"), {
      target: { value: NEW_WORKTREE_SENTINEL },
    });
    fireEvent.change(await screen.findByTestId("swarm-spawn-worktree-new"), {
      target: { value: "wt-pair" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-working-dir"), {
      target: { value: "./worktrees/wt-pair" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-isolation-tier"), {
      target: { value: "tier3_microvm" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-committed-memory-mib"), {
      target: { value: "4096" },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await waitFor(() => expect(spawnLocalCloudPairMock).toHaveBeenCalled());
    const pairCalls = spawnLocalCloudPairMock.mock.calls;
    const request = pairCalls[pairCalls.length - 1][0] as {
      local: Record<string, unknown>;
      cloud: Record<string, unknown>;
    };
    expect(request.local.provider).toBe("local");
    expect(request.local.runtimeBinding).toBe("llama_cpp");
    expect(request.local.localExecutionMode).toBe("warm_vm");
    expect(request.local.swarmId).toBe("wt-pair");
    expect(request.local.worktreeId).toBe("wt-pair");
    expect(request.local.committedMemoryBytes).toBe(4 * 1024 * 1024 * 1024);
    expect(request.cloud.provider).toBe("official_cli");
    expect(request.cloud.cloudModelName).toBe("claude-sonnet-4");
    expect(request.cloud.swarmId).toBe("wt-pair");
    expect(request.cloud.worktreeId).toBe("wt-pair");
    expect(request.cloud.workingDir).toBe("./worktrees/wt-pair");
    expect("localExecutionMode" in request.cloud).toBe(false);
    expect("committedMemoryBytes" in request.cloud).toBe(false);
    expect(screen.getByTestId("swarm-spawn-notice")).toHaveTextContent(/Pair attempted/i);
  });

  test("capacity escalation workflow calls the explicit local-then-cloud IPC", async () => {
    render(<SwarmControlRoom />);
    await screen.findByTestId("swarm-spawn-worktree-select");

    fireEvent.change(screen.getByTestId("swarm-spawn-workflow"), {
      target: { value: "local_cloud_escalation" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-artifact-path"), {
      target: { value: "D:/models/tiny.safetensors" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-sha256"), {
      target: { value: "cd".repeat(32) },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-cloud-model"), {
      target: { value: "gpt-4o" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-worktree-select"), {
      target: { value: "wt-existing" },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await waitFor(() => expect(spawnWithCloudEscalationMock).toHaveBeenCalled());
    const escalationCalls = spawnWithCloudEscalationMock.mock.calls;
    const request = escalationCalls[escalationCalls.length - 1][0] as {
      local: Record<string, unknown>;
      cloud: Record<string, unknown>;
    };
    expect(request.local.provider).toBe("local");
    expect(request.cloud.provider).toBe("byok_cloud");
    expect(request.cloud.byokCloudProvider).toBe("openai");
    expect(request.local.swarmId).toBe("wt-existing");
    expect(request.local.worktreeId).toBe("wt-existing");
    expect(request.cloud.swarmId).toBe("wt-existing");
    expect(request.cloud.worktreeId).toBe("wt-existing");
    expect(screen.getByTestId("swarm-spawn-notice")).toHaveTextContent(/Escalated to cloud/i);
  });

  test("operator chat disables cloud fallback until receipt context is available", async () => {
    listActiveSessionsMock.mockResolvedValue([
      {
        instanceId: { modelId: "local-m", instance: 0, composite: "local-m#0" },
        state: "READY",
        provider: "local",
        runtimeBinding: "candle",
        artifactPath: "/m/local.safetensors",
        cloudModelName: null,
        worktreeId: "wt-selected-session",
        workingDir: "D:/work/selected-session",
      },
    ]);
    render(<SwarmControlRoom />);
    await screen.findByTestId("swarm-spawn-worktree-select");

    fireEvent.change(screen.getByTestId("swarm-spawn-workflow"), {
      target: { value: "local_cloud_escalation" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-cloud-model"), {
      target: { value: "gpt-4o" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-worktree-select"), {
      target: { value: "wt-existing" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-swarm-id"), {
      target: { value: "stale-form-swarm" },
    });
    fireEvent.change(screen.getByTestId("operator-chat-session"), {
      target: { value: "local-m#0" },
    });

    expect(screen.getByTestId("operator-chat-escalation")).toBeInTheDocument();
    expect(screen.getByTestId("operator-chat-escalation-enabled")).toBeDisabled();
    expect(screen.getByTestId("operator-chat-escalation")).toHaveTextContent(
      /openai · gpt-4o/i,
    );
    expect(screen.getByTestId("operator-chat-escalation-note")).toHaveTextContent(
      /receipt context/i,
    );
    expect(chatGenerateWithCloudEscalationMock).not.toHaveBeenCalled();
  });

  test("the sessions table renders a Worktree column showing the assigned id or — when unassigned", async () => {
    listActiveSessionsMock.mockResolvedValue([
      {
        instanceId: { modelId: "alpha", instance: 0, composite: "alpha#0" },
        state: "READY",
        provider: "local",
        runtimeBinding: "candle",
        localExecutionMode: "warm_vm",
        artifactPath: "/m/a.safetensors",
        cloudModelName: null,
        worktreeId: "wt-assigned",
        workingDir: "D:/work/a",
      },
      {
        instanceId: { modelId: "beta", instance: 0, composite: "beta#0" },
        state: "READY",
        provider: "byok_cloud",
        runtimeBinding: "cloud",
        localExecutionMode: null,
        artifactPath: null,
        cloudModelName: "gpt-4o",
        worktreeId: null,
        workingDir: null,
      },
    ]);

    render(<SwarmControlRoom />);
    expect(await screen.findByTestId("swarm-session-worktree-alpha#0")).toHaveTextContent(
      "wt-assigned",
    );
    expect(screen.getByTestId("swarm-session-execution-mode-alpha#0")).toHaveTextContent(
      "warm_vm",
    );
    expect(screen.getByTestId("swarm-session-worktree-beta#0")).toHaveTextContent("—");
    expect(screen.getByTestId("swarm-session-execution-mode-beta#0")).toHaveTextContent("—");
  });
});
