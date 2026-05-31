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

  it("disables generation until a prompt is entered", () => {
    render(
      <ABCompare modelId={MODEL_ID} activeVectorId={VECTOR_ID} />,
    );
    expect(screen.getByTestId("ab-compare.generate")).toBeDisabled();
  });
});
