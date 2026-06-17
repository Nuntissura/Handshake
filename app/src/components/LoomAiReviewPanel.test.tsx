// WP-KERNEL-009 / MT-260 — LoomAiReviewPanel tests.
//
// Mounts the panel in jsdom with the api module mocked and proves:
//   - run-job calls the backend job runner with the resolved block ids,
//   - pending suggestions render grouped by kind with per-item accept/reject,
//   - accept / reject / accept-all call the right confirm-to-promote endpoints
//     and refresh, and
//   - a no-model 409 (HSK-409-LOOM-AI-NO-MODEL) renders the typed decline,
//     never a fabricated suggestion.

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, it, expect, vi } from "vitest";
import { LoomAiReviewPanel } from "./LoomAiReviewPanel";
import {
  acceptAllLoomAiSuggestions,
  acceptLoomAiSuggestion,
  listLoomAiSuggestions,
  queryLoomView,
  rejectLoomAiSuggestion,
  runLoomAiJob,
  ApiRequestError,
  type LoomAiSuggestion,
} from "../lib/api";

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    queryLoomView: vi.fn(),
    listLoomAiSuggestions: vi.fn(),
    runLoomAiJob: vi.fn(),
    acceptLoomAiSuggestion: vi.fn(),
    rejectLoomAiSuggestion: vi.fn(),
    acceptAllLoomAiSuggestions: vi.fn(),
  };
});

const WS = "ws-loom-ai-test";

function suggestion(over: Partial<LoomAiSuggestion> = {}): LoomAiSuggestion {
  return {
    suggestion_id: "LAIS-1",
    job_id: "LAIJ-1",
    workspace_id: WS,
    kind: "auto_tag",
    block_id: "blk-1",
    target_block_id: null,
    suggested_value: { tag: "roadmap" },
    model_attribution: { model: "qwen-test", trace_id: "t1" },
    prompt_sha256: "a".repeat(64),
    output_sha256: "b".repeat(64),
    review_state: "pending",
    decided_by: null,
    decision_reason: null,
    recorded_event_id: "EV-1",
    decided_event_id: null,
    promotion_requested_event_id: null,
    promotion_accepted_event_id: null,
    promoted_artifact_ref: null,
    value_hash: "c".repeat(64),
    created_at_utc: "2026-06-17T00:00:00Z",
    ...over,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(queryLoomView).mockResolvedValue({
    view_type: "all",
    blocks: [{ block_id: "blk-1" } as never, { block_id: "blk-2" } as never],
  } as never);
  vi.mocked(listLoomAiSuggestions).mockResolvedValue([]);
  vi.mocked(runLoomAiJob).mockResolvedValue({ job_id: "LAIJ-1", kind: "auto_tag", suggestions: [] });
  vi.mocked(acceptLoomAiSuggestion).mockResolvedValue(suggestion({ review_state: "promoted" }));
  vi.mocked(rejectLoomAiSuggestion).mockResolvedValue(suggestion({ review_state: "rejected" }));
  vi.mocked(acceptAllLoomAiSuggestions).mockResolvedValue({ promoted: ["LAIS-1"], denied: [], skipped: [] });
});

describe("LoomAiReviewPanel", () => {
  it("runs a job over the auto-loaded workspace blocks", async () => {
    render(<LoomAiReviewPanel workspaceId={WS} />);
    await screen.findByTestId("loom-ai-review-panel");
    await waitFor(() => expect(vi.mocked(queryLoomView)).toHaveBeenCalled());

    fireEvent.click(screen.getByTestId("loom-ai-run-auto_tag"));
    await waitFor(() =>
      expect(vi.mocked(runLoomAiJob)).toHaveBeenCalledWith(WS, {
        kind: "auto_tag",
        block_ids: ["blk-1", "blk-2"],
        tag_candidates: [],
      }),
    );
  });

  it("renders pending suggestions grouped by kind and accepts one", async () => {
    vi.mocked(listLoomAiSuggestions).mockResolvedValue([suggestion()]);
    render(<LoomAiReviewPanel workspaceId={WS} />);

    await screen.findByTestId("loom-ai-group-auto_tag");
    expect(screen.getByTestId("loom-ai-suggestion-LAIS-1")).toBeTruthy();
    expect(screen.getByText("#roadmap")).toBeTruthy();

    fireEvent.click(screen.getByTestId("loom-ai-accept-LAIS-1"));
    await waitFor(() =>
      expect(vi.mocked(acceptLoomAiSuggestion)).toHaveBeenCalledWith(WS, "LAIS-1", undefined),
    );
  });

  it("rejects a suggestion", async () => {
    vi.mocked(listLoomAiSuggestions).mockResolvedValue([suggestion()]);
    render(<LoomAiReviewPanel workspaceId={WS} />);
    await screen.findByTestId("loom-ai-suggestion-LAIS-1");

    fireEvent.click(screen.getByTestId("loom-ai-reject-LAIS-1"));
    await waitFor(() =>
      expect(vi.mocked(rejectLoomAiSuggestion)).toHaveBeenCalledWith(WS, "LAIS-1", undefined),
    );
  });

  it("accepts all of a kind", async () => {
    vi.mocked(listLoomAiSuggestions).mockResolvedValue([suggestion()]);
    render(<LoomAiReviewPanel workspaceId={WS} />);
    await screen.findByTestId("loom-ai-group-auto_tag");

    fireEvent.click(screen.getByTestId("loom-ai-accept-all-auto_tag"));
    await waitFor(() =>
      expect(vi.mocked(acceptAllLoomAiSuggestions)).toHaveBeenCalledWith(
        WS,
        "LAIJ-1",
        "auto_tag",
        undefined,
      ),
    );
  });

  it("shows the typed no-model decline on a 409 NO-MODEL response", async () => {
    vi.mocked(runLoomAiJob).mockRejectedValue(
      new ApiRequestError(409, "Conflict", '{"error":"HSK-409-LOOM-AI-NO-MODEL"}'),
    );
    render(<LoomAiReviewPanel workspaceId={WS} />);
    await screen.findByTestId("loom-ai-review-panel");
    await waitFor(() => expect(vi.mocked(queryLoomView)).toHaveBeenCalled());

    fireEvent.click(screen.getByTestId("loom-ai-run-auto_tag"));
    await screen.findByTestId("loom-ai-no-model");
    expect(screen.queryByTestId("loom-ai-suggestion-LAIS-1")).toBeNull();
  });
});
