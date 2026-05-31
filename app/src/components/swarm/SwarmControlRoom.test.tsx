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
      budgetExhausted: false,
    })),
    listActiveSessions: () => listActiveSessionsMock(),
    listWorktrees: () => listWorktreesMock(),
    spawnSession: (request: Record<string, unknown>) => spawnSessionMock(request),
    cancelSession: vi.fn(),
    chatGenerate: vi.fn(),
  };
});

import { SwarmControlRoom, effectiveWorktreeId, NEW_WORKTREE_SENTINEL } from "./SwarmControlRoom";

beforeEach(() => {
  spawnSessionMock.mockClear();
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

    // The mandatory "recorded, not enforced" honesty note is present.
    expect(screen.getByTestId("swarm-isolation-note")).toHaveTextContent(
      /recorded, not yet enforced/i,
    );
    // The disk working-dir field is optional and present.
    expect(screen.getByTestId("swarm-spawn-working-dir")).toBeInTheDocument();
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

    fireEvent.change(screen.getByTestId("swarm-spawn-provider"), {
      target: { value: "byok_cloud" },
    });
    fireEvent.change(screen.getByTestId("swarm-spawn-cloud-model"), {
      target: { value: "gpt-4o" },
    });
    fireEvent.click(screen.getByTestId("swarm-spawn-submit"));

    await waitFor(() => expect(spawnSessionMock).toHaveBeenCalled());
    const req = lastSpawnRequest();
    expect(req.worktreeId).toBe("wt-existing");
    expect(req.workingDir).toBe("D:/work/wt-foo");
    expect(req.isolationTier).toBe("tier3_microvm");
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
  });

  test("the sessions table renders a Worktree column showing the assigned id or — when unassigned", async () => {
    listActiveSessionsMock.mockResolvedValue([
      {
        instanceId: { modelId: "alpha", instance: 0, composite: "alpha#0" },
        state: "READY",
        provider: "local",
        runtimeBinding: "candle",
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
    expect(screen.getByTestId("swarm-session-worktree-beta#0")).toHaveTextContent("—");
  });
});
