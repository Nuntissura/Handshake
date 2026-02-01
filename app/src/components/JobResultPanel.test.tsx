import { vi } from "vitest";

vi.mock("../lib/api", () => ({
  sha256HexUtf8: vi.fn(async () => "h".repeat(64)),
}));

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { AiJob } from "../lib/api";
import { JobResultPanel } from "./JobResultPanel";

function makeJob(overrides: Partial<AiJob> = {}): AiJob {
  const now = "2026-02-01T00:00:00Z";
  return {
    job_id: "job-1",
    trace_id: "trace-1",
    workflow_run_id: null,
    job_kind: "doc_summarize",
    state: "completed",
    protocol_id: "doc-proto-001",
    profile_id: "profile-1",
    capability_profile_id: "cap-1",
    access_mode: "local",
    safety_mode: "default",
    entity_refs: [],
    planned_operations: [],
    metrics: {
      duration_ms: 0,
      total_tokens: 0,
      input_tokens: 0,
      output_tokens: 0,
      tokens_planner: 0,
      tokens_executor: 0,
      entities_read: 0,
      entities_written: 0,
      validators_run_count: 0,
    },
    status_reason: "completed",
    job_inputs: null,
    job_outputs: null,
    created_at: now,
    updated_at: now,
    ...overrides,
  };
}

describe("JobResultPanel (leak-aware outputs)", () => {
  it("does not render outputs until Reveal is clicked (SECRET stays hidden by default)", async () => {
    const job = makeJob({ job_outputs: { summary: "SECRET" } });

    render(<JobResultPanel job={job} onDismiss={vi.fn()} />);

    expect(screen.getByText(job.job_id, { exact: false })).toBeInTheDocument();
    expect(screen.getByText(job.trace_id, { exact: false })).toBeInTheDocument();

    expect(screen.queryByText(/SECRET/)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /reveal output preview/i }));

    expect(await screen.findByText(/SECRET/)).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("h".repeat(64), { exact: false })).toBeInTheDocument();
    });
  });

  it("disables preview in strict safety mode and never reveals SECRET", async () => {
    const job = makeJob({ safety_mode: "strict", job_outputs: { summary: "SECRET" } });

    render(<JobResultPanel job={job} onDismiss={vi.fn()} />);

    const revealButton = screen.getByRole("button", { name: /reveal output preview/i });
    expect(revealButton).toBeDisabled();
    expect(screen.getByText(/Preview disabled in strict safety mode/i)).toBeInTheDocument();

    expect(screen.queryByText(/SECRET/)).not.toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("h".repeat(64), { exact: false })).toBeInTheDocument();
    });
  });
});
