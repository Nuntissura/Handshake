import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CodeSymbolPanel } from "./CodeSymbolPanel";
import { getCodeFileLens, getCodeSymbol } from "../lib/api";

vi.mock("../lib/api", () => ({
  getCodeSymbol: vi.fn(),
  getCodeFileLens: vi.fn(),
}));

describe("CodeSymbolPanel", () => {
  beforeEach(() => {
    vi.mocked(getCodeSymbol).mockReset();
    vi.mocked(getCodeFileLens).mockReset();
  });

  it("fetches and renders the target file lens from the symbol staleness payload", async () => {
    vi.mocked(getCodeSymbol).mockResolvedValue({
      symbol: {
        symbol_entity_id: "KEN-add",
        symbol_key: "rust:src/lib.rs#add",
        display_name: "add",
        symbol_kind: "function",
        owning_wp: "WP-KERNEL-009",
        primary_source_id: "source-lib",
        lifecycle_state: "active",
        definition: {
          span_id: "span-add",
          source_id: "source-lib",
          line_start: 2,
          line_end: 2,
          range_start: 10,
          range_end: 20,
        },
        staleness: {
          state: "fresh",
          fresh: true,
          indexed_content_hash: "hash-lib",
          indexed_parser_version: "tree-sitter-rust@1",
        },
      },
      nav_receipt_event_id: "EVT-symbol",
      quiet_background_work_receipt_id: "quiet-symbol",
    });
    vi.mocked(getCodeFileLens).mockResolvedValue({
      workspace_id: "workspace-alpha",
      relative_path: "src/lib.rs",
      staleness: { state: "fresh", fresh: true },
      truncated: false,
      entries: [
        {
          symbol_entity_id: "KEN-add",
          symbol_key: "rust:src/lib.rs#add",
          display_name: "add",
          symbol_kind: "function",
          definition: { start_line: 2, end_line: 2 },
          references: [{ start_line: 8, end_line: 8 }],
          doc: "Adds two numbers.",
          caller_count: 1,
        },
      ],
      nav_receipt_event_id: "EVT-lens",
      quiet_background_work_receipt_id: "quiet-lens",
    });

    render(<CodeSymbolPanel symbolEntityId="KEN-add" workspaceId="workspace-alpha" />);

    await waitFor(() =>
      expect(getCodeFileLens).toHaveBeenCalledWith(
        "workspace-alpha",
        "src/lib.rs",
        "hash-lib",
        "tree-sitter-rust@1",
      ),
    );
    expect(screen.getByTestId("code-symbol-panel.file-lens")).toHaveTextContent("src/lib.rs");
    expect(screen.getByTestId("code-symbol-panel.file-lens")).toHaveTextContent("Adds two numbers.");
    expect(screen.getByTestId("code-symbol-panel.file-lens")).toHaveTextContent("line 2");
  });
});
