import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";

const subquadStateListMock = vi.hoisted(() => vi.fn());
const subquadStateCommitMock = vi.hoisted(() => vi.fn());
const subquadStateRestoreMock = vi.hoisted(() => vi.fn());
const subquadEvictAllMock = vi.hoisted(() => vi.fn());
const subquadPersistMock = vi.hoisted(() => vi.fn());
const subquadRehydrateMock = vi.hoisted(() => vi.fn());

vi.mock("../../lib/ipc/subquadratic", async () => {
  const actual = await vi.importActual<
    typeof import("../../lib/ipc/subquadratic")
  >("../../lib/ipc/subquadratic");
  return {
    ...actual,
    subquadStateList: subquadStateListMock,
    subquadStateCommit: subquadStateCommitMock,
    subquadStateRestore: subquadStateRestoreMock,
    subquadEvictAll: subquadEvictAllMock,
    subquadPersist: subquadPersistMock,
    subquadRehydrate: subquadRehydrateMock,
  };
});

import { SubquadraticPanel } from "./SubquadraticPanel";

const MODEL_ID = "019a1b2c-0000-7000-8000-aaaaaaaaaaaa";

function caps(overrides: Partial<ModelCapabilities> = {}): ModelCapabilities {
  return {
    supportsLora: false,
    supportsKvPrefixCache: false,
    supportsKvQuantization: "none",
    supportsActivationSteering: false,
    supportsSubquadratic: true,
    supportsSpeculativeDraft: false,
    supportsEagle3: false,
    ...overrides,
  };
}

function emptyOccupancy() {
  return {
    bytesUsed: 0,
    bytesCapacity: 0,
    prefixCacheEntries: 0,
    prefixCacheHitCount: 0,
    prefixCacheMissCount: 0,
    quantLevelCurrent: "none" as const,
  };
}

describe("SubquadraticPanel", () => {
  beforeEach(() => {
    subquadStateListMock.mockReset();
    subquadStateCommitMock.mockReset();
    subquadStateRestoreMock.mockReset();
    subquadEvictAllMock.mockReset();
    subquadPersistMock.mockReset();
    subquadRehydrateMock.mockReset();
    // Reset Work Profile opt-out flag for each test.
    if (typeof window !== "undefined") {
      window.localStorage.removeItem("settings.ui.show_subquadratic_panel");
    }
  });

  it("renders nothing when capabilities.supportsSubquadratic is false", () => {
    const { container } = render(
      <SubquadraticPanel
        modelId={MODEL_ID}
        capabilities={caps({ supportsSubquadratic: false })}
      />,
    );
    expect(container.firstChild).toBeNull();
    expect(subquadStateListMock).not.toHaveBeenCalled();
  });

  it("renders nothing when capabilities is null (not yet probed)", () => {
    const { container } = render(
      <SubquadraticPanel modelId={MODEL_ID} capabilities={null} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders the panel + variant badge fallback when variant not supplied", async () => {
    subquadStateListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      occupancy: emptyOccupancy(),
    });
    render(<SubquadraticPanel modelId={MODEL_ID} capabilities={caps()} />);
    await waitFor(() =>
      screen.getByTestId("subquadratic-panel.variant-badge"),
    );
    expect(
      screen.getByTestId("subquadratic-panel.variant-badge").textContent,
    ).toContain("variant detection pending");
  });

  it("renders the variant badge with the supplied SSM variant label", async () => {
    subquadStateListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      occupancy: emptyOccupancy(),
    });
    render(
      <SubquadraticPanel
        modelId={MODEL_ID}
        capabilities={caps()}
        variant="mamba2"
      />,
    );
    const badge = await screen.findByTestId(
      "subquadratic-panel.variant-badge",
    );
    expect(badge.textContent).toBe("Mamba2");
  });

  it("opt-out toggle hides the state-vector controls section", async () => {
    subquadStateListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      occupancy: emptyOccupancy(),
    });
    render(<SubquadraticPanel modelId={MODEL_ID} capabilities={caps()} />);
    await waitFor(() => screen.getByTestId("state-vector-controls"));
    const toggle = screen.getByTestId(
      "subquadratic-panel.opt-out-toggle",
    ) as HTMLInputElement;
    fireEvent.click(toggle);
    expect(
      screen.queryByTestId("state-vector-controls"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByTestId("subquadratic-panel.opted-out-note").textContent,
    ).toContain("hidden by Work Profile preference");
  });

  it("commit -> restore round-trips via the IPC mocks", async () => {
    subquadStateListMock.mockResolvedValue({
      modelId: MODEL_ID,
      occupancy: emptyOccupancy(),
    });
    subquadStateCommitMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-SUBQUAD-STATE-COMMIT",
      prefixHandle: {
        prefixId: "019a1b2c-0000-7000-8000-bbbbbbbbbbbb",
        contentHashHex: "aa".repeat(32),
        tokenCount: 3,
      },
      occupancy: { ...emptyOccupancy(), prefixCacheEntries: 1 },
    });
    subquadStateRestoreMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-SUBQUAD-STATE-RESTORE",
      prefixHandle: {
        prefixId: "019a1b2c-0000-7000-8000-bbbbbbbbbbbb",
        contentHashHex: "aa".repeat(32),
        tokenCount: 3,
      },
      hit: true,
      occupancy: {
        ...emptyOccupancy(),
        prefixCacheEntries: 1,
        prefixCacheHitCount: 1,
      },
    });

    render(<SubquadraticPanel modelId={MODEL_ID} capabilities={caps()} />);
    await waitFor(() => screen.getByTestId("state-vector-controls"));

    const input = screen.getByTestId(
      "state-vector-controls.commit-input",
    ) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: "1, 2, 3" } });
    fireEvent.click(screen.getByTestId("state-vector-controls.commit-button"));

    await waitFor(() =>
      expect(subquadStateCommitMock).toHaveBeenCalledTimes(1),
    );
    expect(subquadStateCommitMock.mock.calls[0][0]).toEqual({
      modelId: MODEL_ID,
      prefixTokens: [1, 2, 3],
    });

    await screen.findByTestId("state-vector-controls.commit-receipt");
    fireEvent.click(screen.getByTestId("state-vector-controls.restore-button"));
    await waitFor(() =>
      expect(subquadStateRestoreMock).toHaveBeenCalledTimes(1),
    );
  });

  it("commit input rejects empty token list with inline error", async () => {
    subquadStateListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      occupancy: emptyOccupancy(),
    });
    render(<SubquadraticPanel modelId={MODEL_ID} capabilities={caps()} />);
    await waitFor(() => screen.getByTestId("state-vector-controls"));
    fireEvent.click(screen.getByTestId("state-vector-controls.commit-button"));
    expect(
      screen.getByTestId("state-vector-controls.commit-error").textContent,
    ).toContain("prefix tokens required");
    expect(subquadStateCommitMock).not.toHaveBeenCalled();
  });

  it("persist surfaces MT-117 deferral banner instead of crashing", async () => {
    subquadStateListMock.mockResolvedValue({
      modelId: MODEL_ID,
      occupancy: emptyOccupancy(),
    });
    subquadStateCommitMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-SUBQUAD-STATE-COMMIT",
      prefixHandle: {
        prefixId: "019a1b2c-0000-7000-8000-cccccccccccc",
        contentHashHex: "bb".repeat(32),
        tokenCount: 2,
      },
      occupancy: { ...emptyOccupancy(), prefixCacheEntries: 1 },
    });
    subquadPersistMock.mockRejectedValueOnce(
      new Error(
        "subquad persist deferred: capability subquadratic_persist_disk_deferred_mt117 is not supported by adapter candle",
      ),
    );

    render(<SubquadraticPanel modelId={MODEL_ID} capabilities={caps()} />);
    await waitFor(() => screen.getByTestId("state-vector-controls"));

    const input = screen.getByTestId(
      "state-vector-controls.commit-input",
    ) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: "10 20" } });
    fireEvent.click(screen.getByTestId("state-vector-controls.commit-button"));
    await screen.findByTestId("state-vector-controls.commit-receipt");

    fireEvent.click(screen.getByTestId("state-vector-controls.persist-button"));
    await waitFor(() =>
      expect(
        screen.getByTestId("state-vector-controls.deferral-banner").textContent,
      ).toContain("MT-117"),
    );
  });

  it("rehydrate surfaces MT-117 deferral banner", async () => {
    subquadStateListMock.mockResolvedValue({
      modelId: MODEL_ID,
      occupancy: emptyOccupancy(),
    });
    subquadRehydrateMock.mockRejectedValueOnce(
      new Error(
        "subquad rehydrate deferred: capability subquadratic_rehydrate_disk_deferred_mt117 is not supported by adapter candle",
      ),
    );
    render(<SubquadraticPanel modelId={MODEL_ID} capabilities={caps()} />);
    await waitFor(() => screen.getByTestId("state-vector-controls"));

    fireEvent.click(
      screen.getByTestId("state-vector-controls.rehydrate-button"),
    );
    await waitFor(() =>
      expect(
        screen.getByTestId("state-vector-controls.deferral-banner").textContent,
      ).toContain("MT-117"),
    );
  });

  it("evict_all confirm dialog: cancelling does NOT dispatch", async () => {
    subquadStateListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      occupancy: emptyOccupancy(),
    });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<SubquadraticPanel modelId={MODEL_ID} capabilities={caps()} />);
    await waitFor(() => screen.getByTestId("state-vector-controls"));
    fireEvent.click(
      screen.getByTestId("state-vector-controls.evict-all-button"),
    );
    expect(confirmSpy).toHaveBeenCalledTimes(1);
    expect(subquadEvictAllMock).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });
});
