import { act, render, screen, waitFor, fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LoomBlockPanel } from "./LoomBlockPanel";
import { getLoomBlock, updateLoomBlock, type LoomBlock } from "../lib/api";

vi.mock("../lib/api", () => ({
  getLoomBlock: vi.fn(),
  updateLoomBlock: vi.fn(),
}));

function loomBlock(overrides: Partial<LoomBlock> = {}): LoomBlock {
  return {
    block_id: "block-alpha",
    workspace_id: "w1",
    content_type: "note",
    document_id: null,
    asset_id: null,
    title: "Alpha Loom note",
    original_filename: null,
    content_hash: "hash-alpha",
    pinned: false,
    favorite: false,
    pin_order: null,
    journal_date: null,
    created_at: "2026-06-15T00:00:00Z",
    updated_at: "2026-06-15T00:00:00Z",
    imported_at: null,
    derived: {
      full_text_index: "Alpha indexed text",
      backlink_count: 1,
      mention_count: 2,
      tag_count: 3,
      preview_status: "ready",
    },
    ...overrides,
  };
}

describe("LoomBlockPanel", () => {
  beforeEach(() => {
    vi.mocked(getLoomBlock).mockReset();
    vi.mocked(updateLoomBlock).mockReset();
  });

  it("edits Loom block properties through the typed backend update API", async () => {
    vi.mocked(getLoomBlock).mockResolvedValueOnce(loomBlock());
    vi.mocked(updateLoomBlock).mockResolvedValueOnce(
      loomBlock({
        title: "Updated Alpha",
        pinned: true,
        favorite: true,
        updated_at: "2026-06-16T00:00:00Z",
      }),
    );

    render(<LoomBlockPanel workspaceId="w1" blockId="block-alpha" />);

    const title = await screen.findByTestId("loom-block-properties.title");
    fireEvent.change(title, { target: { value: "Updated Alpha" } });
    fireEvent.click(screen.getByTestId("loom-block-properties.pinned"));
    fireEvent.click(screen.getByTestId("loom-block-properties.favorite"));
    fireEvent.click(screen.getByTestId("loom-block-properties.save"));

    await waitFor(() => {
      expect(updateLoomBlock).toHaveBeenCalledWith("w1", "block-alpha", {
        title: "Updated Alpha",
        pinned: true,
        favorite: true,
      });
    });
    expect(await screen.findByTestId("loom-block-properties.status")).toHaveTextContent(
      "Properties saved",
    );
    expect(screen.getByRole("heading", { name: "Updated Alpha" })).toBeTruthy();
    expect(screen.getByTestId("loom-block-properties.pinned")).toBeChecked();
    expect(screen.getByTestId("loom-block-properties.favorite")).toBeChecked();
  });

  it("announces bookmark refresh when a property save changes pinned state", async () => {
    vi.mocked(getLoomBlock).mockResolvedValueOnce(loomBlock({ pinned: false }));
    vi.mocked(updateLoomBlock).mockResolvedValueOnce(loomBlock({ pinned: true }));
    const bookmarkChanged = vi.fn();
    window.addEventListener("handshake:loom-bookmarks-changed", bookmarkChanged);

    try {
      render(<LoomBlockPanel workspaceId="w1" blockId="block-alpha" />);

      await screen.findByTestId("loom-block-properties.title");
      fireEvent.click(screen.getByTestId("loom-block-properties.pinned"));
      fireEvent.click(screen.getByTestId("loom-block-properties.save"));

      await waitFor(() => expect(bookmarkChanged).toHaveBeenCalledTimes(1));
      expect((bookmarkChanged.mock.calls[0][0] as CustomEvent).detail).toEqual({
        workspaceId: "w1",
        blockId: "block-alpha",
        pinned: true,
      });
    } finally {
      window.removeEventListener("handshake:loom-bookmarks-changed", bookmarkChanged);
    }
  });

  it("applies external bookmark removal before the next property save", async () => {
    vi.mocked(getLoomBlock).mockResolvedValueOnce(loomBlock({ title: "Pinned Alpha", pinned: true, pin_order: 10 }));
    vi.mocked(updateLoomBlock).mockResolvedValueOnce(
      loomBlock({ title: "Pinned Alpha edited", pinned: false, pin_order: null }),
    );

    render(<LoomBlockPanel workspaceId="w1" blockId="block-alpha" />);

    const title = await screen.findByTestId("loom-block-properties.title");
    expect(screen.getByTestId("loom-block-properties.pinned")).toBeChecked();

    act(() => {
      window.dispatchEvent(
        new CustomEvent("handshake:loom-block-updated", {
          detail: {
            workspaceId: "w1",
            block: loomBlock({ title: "Pinned Alpha", pinned: false, pin_order: null }),
          },
        }),
      );
    });

    expect(screen.getByTestId("loom-block-properties.pinned")).not.toBeChecked();
    fireEvent.change(title, { target: { value: "Pinned Alpha edited" } });
    fireEvent.click(screen.getByTestId("loom-block-properties.save"));

    await waitFor(() => {
      expect(updateLoomBlock).toHaveBeenCalledWith("w1", "block-alpha", {
        title: "Pinned Alpha edited",
        pinned: false,
        favorite: false,
      });
    });
  });

  it("keeps the block visible and shows a typed error when property save fails", async () => {
    vi.mocked(getLoomBlock).mockResolvedValueOnce(loomBlock());
    vi.mocked(updateLoomBlock).mockRejectedValueOnce(new Error("patch failed"));

    render(<LoomBlockPanel workspaceId="w1" blockId="block-alpha" />);

    await screen.findByTestId("loom-block-properties.title");
    fireEvent.click(screen.getByTestId("loom-block-properties.save"));

    expect(await screen.findByTestId("loom-block-properties.error")).toHaveTextContent(
      "patch failed",
    );
    expect(screen.getByTestId("loom-block-panel")).toBeTruthy();
  });
});
