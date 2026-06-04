import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

const generateAbMock = vi.hoisted(() => vi.fn());

vi.mock("../../lib/ipc/steering", () => ({
  generateAb: generateAbMock,
}));

import { ABCompare } from "./ABCompare";

const MODEL_ID = "019a1b2c-0000-7000-8000-aaaaaaaaaaaa";
const VECTOR_ID = "019a1b2c-0000-7000-8000-000000000001";

describe("ABCompare", () => {
  beforeEach(() => {
    generateAbMock.mockReset();
  });

  it("renders both BEFORE and AFTER panes side by side after generating", async () => {
    generateAbMock.mockResolvedValueOnce({
      comparisons: [
        {
          prompt: "describe the scene",
          inactiveCompletion: "BASELINE-OUTPUT-TEXT",
          activeCompletion: "STEERED-OUTPUT-TEXT",
        },
      ],
      activeVectorIds: [VECTOR_ID],
      inactiveVectorIds: [],
      eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
    });

    render(
      <ABCompare modelId={MODEL_ID} activeVectorId={VECTOR_ID} vectorName="calm" />,
    );

    fireEvent.change(screen.getByTestId("ab-compare.prompts"), {
      target: { value: "describe the scene" },
    });
    fireEvent.click(screen.getByTestId("ab-compare.generate"));

    expect(screen.getByTestId("ab-compare.loading")).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByTestId("ab-compare.results")).toBeInTheDocument();
    });

    // The IPC was called with the proposed vector active and inactive empty.
    expect(generateAbMock).toHaveBeenCalledWith({
      modelId: MODEL_ID,
      prompts: ["describe the scene"],
      activeVectorIds: [VECTOR_ID],
      inactiveVectorIds: [],
      maxTokens: 64,
    });

    // Both panes render side by side with their respective completions.
    const inactivePane = screen.getByTestId("ab-compare.pair.0.inactive");
    const activePane = screen.getByTestId("ab-compare.pair.0.active");
    expect(inactivePane).toBeInTheDocument();
    expect(activePane).toBeInTheDocument();
    expect(
      screen.getByTestId("ab-compare.pair.0.inactive-text").textContent,
    ).toContain("BASELINE-OUTPUT-TEXT");
    expect(
      screen.getByTestId("ab-compare.pair.0.active-text").textContent,
    ).toContain("STEERED-OUTPUT-TEXT");
  });

  it("surfaces the kernel error verbatim (capture-not-available path)", async () => {
    generateAbMock.mockRejectedValueOnce(
      "capture_not_available: generate_ab requires a live ModelRuntime adapter attached for model " +
        MODEL_ID,
    );

    render(
      <ABCompare modelId={MODEL_ID} activeVectorId={VECTOR_ID} />,
    );

    fireEvent.change(screen.getByTestId("ab-compare.prompts"), {
      target: { value: "x" },
    });
    fireEvent.click(screen.getByTestId("ab-compare.generate"));

    await waitFor(() => {
      const err = screen.getByTestId("ab-compare.error");
      expect(err.textContent).toContain("capture_not_available");
    });
  });

  it("sends explicit before and after vector sets from UI state", async () => {
    const beforeVectorId = "019a1b2c-0000-7000-8000-000000000002";
    const applyActive = vi.fn(async () => undefined);
    const revertInactive = vi.fn(async () => undefined);
    generateAbMock.mockResolvedValueOnce({
      comparisons: [
        {
          prompt: "compare tone",
          inactiveCompletion: "BEFORE-VECTOR-TEXT",
          activeCompletion: "AFTER-VECTOR-TEXT",
        },
      ],
      activeVectorIds: [VECTOR_ID],
      inactiveVectorIds: [beforeVectorId],
      eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
    });

    render(
      <ABCompare
        modelId={MODEL_ID}
        activeVectorIds={[VECTOR_ID]}
        inactiveVectorIds={[beforeVectorId]}
        activeLabel="After (calm)"
        inactiveLabel="Before (direct)"
        onApplyActive={applyActive}
        onRevertInactive={revertInactive}
      />,
    );

    fireEvent.change(screen.getByTestId("ab-compare.prompts"), {
      target: { value: "compare tone" },
    });
    fireEvent.click(screen.getByTestId("ab-compare.generate"));

    await waitFor(() => {
      expect(screen.getByTestId("ab-compare.results")).toBeInTheDocument();
    });

    expect(generateAbMock).toHaveBeenCalledWith({
      modelId: MODEL_ID,
      prompts: ["compare tone"],
      activeVectorIds: [VECTOR_ID],
      inactiveVectorIds: [beforeVectorId],
      maxTokens: 64,
    });
    expect(screen.getByText("After (calm)")).toBeInTheDocument();
    expect(screen.getByText("Before (direct)")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("ab-compare.apply-active"));
    await waitFor(() => {
      expect(applyActive).toHaveBeenCalledWith([VECTOR_ID]);
    });
    expect(screen.getByTestId("ab-compare.apply-status").textContent).toContain(
      "Applied after set",
    );

    fireEvent.click(screen.getByTestId("ab-compare.revert-inactive"));
    await waitFor(() => {
      expect(revertInactive).toHaveBeenCalledWith([beforeVectorId]);
    });
    expect(screen.getByTestId("ab-compare.apply-status").textContent).toContain(
      "Reverted to before set",
    );
  });

  it("surfaces apply/revert errors without losing comparison results", async () => {
    const applyActive = vi.fn(async () => {
      throw new Error("review gate denied");
    });
    generateAbMock.mockResolvedValueOnce({
      comparisons: [
        {
          prompt: "compare tone",
          inactiveCompletion: "BEFORE",
          activeCompletion: "AFTER",
        },
      ],
      activeVectorIds: [VECTOR_ID],
      inactiveVectorIds: [],
      eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
    });

    render(
      <ABCompare
        modelId={MODEL_ID}
        activeVectorId={VECTOR_ID}
        onApplyActive={applyActive}
      />,
    );

    fireEvent.change(screen.getByTestId("ab-compare.prompts"), {
      target: { value: "compare tone" },
    });
    fireEvent.click(screen.getByTestId("ab-compare.generate"));

    await waitFor(() => {
      expect(screen.getByTestId("ab-compare.results")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("ab-compare.apply-active"));

    await waitFor(() => {
      expect(screen.getByTestId("ab-compare.apply-error").textContent).toContain(
        "review gate denied",
      );
    });
    expect(screen.getByTestId("ab-compare.pair.0.active-text").textContent).toContain("AFTER");
  });

  it("clears generated results when the selected vector set changes", async () => {
    const nextVectorId = "019a1b2c-0000-7000-8000-000000000003";
    const applyActive = vi.fn(async () => undefined);
    generateAbMock.mockResolvedValueOnce({
      comparisons: [
        {
          prompt: "compare tone",
          inactiveCompletion: "BEFORE",
          activeCompletion: "AFTER",
        },
      ],
      activeVectorIds: [VECTOR_ID],
      inactiveVectorIds: [],
      eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
    });

    const { rerender } = render(
      <ABCompare
        modelId={MODEL_ID}
        activeVectorIds={[VECTOR_ID]}
        activeLabel="After (calm)"
        onApplyActive={applyActive}
      />,
    );

    fireEvent.change(screen.getByTestId("ab-compare.prompts"), {
      target: { value: "compare tone" },
    });
    fireEvent.click(screen.getByTestId("ab-compare.generate"));

    await waitFor(() => {
      expect(screen.getByTestId("ab-compare.results")).toBeInTheDocument();
    });

    rerender(
      <ABCompare
        modelId={MODEL_ID}
        activeVectorIds={[nextVectorId]}
        activeLabel="After (direct)"
        onApplyActive={applyActive}
      />,
    );

    await waitFor(() => {
      expect(screen.queryByTestId("ab-compare.results")).not.toBeInTheDocument();
    });
    expect(screen.queryByTestId("ab-compare.apply-active")).not.toBeInTheDocument();
    expect(applyActive).not.toHaveBeenCalled();
  });

  it("can revert to a baseline empty steering set", async () => {
    const revertInactive = vi.fn(async () => undefined);
    generateAbMock.mockResolvedValueOnce({
      comparisons: [
        {
          prompt: "compare baseline",
          inactiveCompletion: "BASELINE",
          activeCompletion: "AFTER",
        },
      ],
      activeVectorIds: [VECTOR_ID],
      inactiveVectorIds: [],
      eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
    });

    render(
      <ABCompare
        modelId={MODEL_ID}
        activeVectorId={VECTOR_ID}
        onRevertInactive={revertInactive}
      />,
    );

    fireEvent.change(screen.getByTestId("ab-compare.prompts"), {
      target: { value: "compare baseline" },
    });
    fireEvent.click(screen.getByTestId("ab-compare.generate"));

    await waitFor(() => {
      expect(screen.getByTestId("ab-compare.results")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("ab-compare.revert-inactive"));

    await waitFor(() => {
      expect(revertInactive).toHaveBeenCalledWith([]);
    });
    expect(screen.getByTestId("ab-compare.apply-status").textContent).toContain(
      "Reverted to before set",
    );
  });

  it("clamps max tokens before dispatching the live A/B request", async () => {
    generateAbMock.mockResolvedValueOnce({
      comparisons: [],
      activeVectorIds: [VECTOR_ID],
      inactiveVectorIds: [],
      eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
    });

    render(
      <ABCompare modelId={MODEL_ID} activeVectorId={VECTOR_ID} />,
    );

    fireEvent.change(screen.getByTestId("ab-compare.prompts"), {
      target: { value: "bounded request" },
    });
    fireEvent.change(screen.getByTestId("ab-compare.max-tokens"), {
      target: { value: "999" },
    });
    fireEvent.click(screen.getByTestId("ab-compare.generate"));

    await waitFor(() => {
      expect(generateAbMock).toHaveBeenCalledWith(
        expect.objectContaining({ maxTokens: 256 }),
      );
    });
  });

  it("disables generation until a prompt is entered", () => {
    render(
      <ABCompare modelId={MODEL_ID} activeVectorId={VECTOR_ID} />,
    );
    expect(screen.getByTestId("ab-compare.generate")).toBeDisabled();
  });
});
