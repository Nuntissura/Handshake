import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { WorkspaceSidebar } from "./WorkspaceSidebar";

vi.mock("../lib/api", () => {
  const listWorkspaces = vi.fn();
  const listDocuments = vi.fn();
  const listCanvases = vi.fn();
  const queryLoomView = vi.fn();
  const updateLoomBlock = vi.fn();
  const setLoomBlockPinOrder = vi.fn();
  return {
    listWorkspaces,
    listDocuments,
    listCanvases,
    queryLoomView,
    updateLoomBlock,
    setLoomBlockPinOrder,
    createWorkspace: vi.fn(),
    createDocument: vi.fn(),
    createCanvas: vi.fn(),
    deleteWorkspace: vi.fn(),
    __esModule: true,
  };
});

describe("WorkspaceSidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const noop = () => {};

  function deferred<T>() {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>((promiseResolve) => {
      resolve = promiseResolve;
    });
    return { promise, resolve };
  }

  function loomBlock(overrides: Record<string, unknown> = {}) {
    return {
      block_id: "block-alpha",
      workspace_id: "w1",
      content_type: "note",
      document_id: null,
      asset_id: null,
      title: "Pinned Alpha",
      original_filename: null,
      content_hash: "hash-alpha",
      pinned: true,
      favorite: false,
      pin_order: 0,
      journal_date: null,
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-16T00:00:00Z",
      imported_at: null,
      derived: {
        full_text_index: "Pinned alpha text",
        backlink_count: 1,
        mention_count: 2,
        tag_count: 3,
        preview_status: "ready",
      },
      ...overrides,
    };
  }

  it("renders workspaces on successful fetch without error banner", async () => {
    const api = vi.mocked(await import("../lib/api"), true);
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.listWorkspaces.mockResolvedValue([
      { id: "w1", name: "Workspace 1", created_at: "", updated_at: "" },
    ]);

    render(
      <WorkspaceSidebar
        refreshKey={0}
        onSelectDocument={noop}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
        onWorkspaceDeleted={noop}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("Workspace 1")).toBeInTheDocument();
    });
    expect(screen.queryByText(/Could not refresh the workspace list/i)).not.toBeInTheDocument();
  });

  it("shows error banner and Retry button when fetch fails", async () => {
    const api = vi.mocked(await import("../lib/api"), true);
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.listWorkspaces.mockRejectedValue(new Error("network down"));

    render(
      <WorkspaceSidebar
        refreshKey={0}
        onSelectDocument={noop}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
        onWorkspaceDeleted={noop}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText(/Could not refresh the workspace list/i)).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
  });

  it("keeps controlled workspace details scoped and ignores stale previous loads", async () => {
    const api = vi.mocked(await import("../lib/api"), true);
    const staleDocuments = deferred<[{ id: string; workspace_id: string; title: string; created_at: string; updated_at: string }]>();
    const staleCanvases = deferred<[]>();
    const onSelectDocument = vi.fn();
    api.listWorkspaces.mockResolvedValue([
      { id: "w1", name: "Workspace 1", created_at: "", updated_at: "" },
      { id: "w2", name: "Workspace 2", created_at: "", updated_at: "" },
    ]);
    api.listDocuments.mockImplementation(async (workspaceId: string) => {
      if (workspaceId === "w1") {
        return staleDocuments.promise;
      }
      return [{ id: "doc-beta", workspace_id: "w2", title: "Doc Beta", created_at: "", updated_at: "" }];
    });
    api.listCanvases.mockImplementation(async (workspaceId: string) => {
      if (workspaceId === "w1") {
        return staleCanvases.promise;
      }
      return [];
    });

    const { rerender } = render(
      <WorkspaceSidebar
        refreshKey={0}
        activeWorkspaceId="w1"
        onSelectDocument={onSelectDocument}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
        onWorkspaceDeleted={noop}
      />,
    );

    await waitFor(() => expect(api.listDocuments).toHaveBeenCalledWith("w1"));

    rerender(
      <WorkspaceSidebar
        refreshKey={0}
        activeWorkspaceId="w2"
        onSelectDocument={onSelectDocument}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
        onWorkspaceDeleted={noop}
      />,
    );

    await waitFor(() => expect(screen.getByText("Doc Beta")).toBeInTheDocument());
    await act(async () => {
      staleDocuments.resolve([
        { id: "doc-alpha", workspace_id: "w1", title: "Doc Alpha", created_at: "", updated_at: "" },
      ]);
      staleCanvases.resolve([]);
      await staleDocuments.promise;
      await staleCanvases.promise;
    });

    expect(screen.queryByText("Doc Alpha")).not.toBeInTheDocument();
    fireEvent.click(screen.getByText("Doc Beta"));
    expect(onSelectDocument).toHaveBeenCalledWith("doc-beta");

    rerender(
      <WorkspaceSidebar
        refreshKey={0}
        activeWorkspaceId={null}
        onSelectDocument={onSelectDocument}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
        onWorkspaceDeleted={noop}
      />,
    );

    await waitFor(() => expect(screen.queryByText("Doc Beta")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "New Document" })).toBeDisabled();
  });

  it("ignores stale document create responses after the controlled workspace changes", async () => {
    const api = vi.mocked(await import("../lib/api"), true);
    const createdDocument = deferred<{
      id: string;
      workspace_id: string;
      title: string;
      created_at: string;
      updated_at: string;
    }>();
    const onSelectDocument = vi.fn();
    const promptSpy = vi.spyOn(window, "prompt").mockReturnValue("Doc Alpha");
    api.listWorkspaces.mockResolvedValue([
      { id: "w1", name: "Workspace 1", created_at: "", updated_at: "" },
      { id: "w2", name: "Workspace 2", created_at: "", updated_at: "" },
    ]);
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.createDocument.mockReturnValue(createdDocument.promise);

    try {
      const { rerender } = render(
        <WorkspaceSidebar
          refreshKey={0}
          activeWorkspaceId="w1"
          onSelectDocument={onSelectDocument}
          onSelectCanvas={noop}
          selectedDocumentId={null}
          selectedCanvasId={null}
          onWorkspaceDeleted={noop}
        />,
      );

      await waitFor(() => expect(screen.getByRole("button", { name: "New Document" })).not.toBeDisabled());
      fireEvent.click(screen.getByRole("button", { name: "New Document" }));
      expect(api.createDocument).toHaveBeenCalledWith("w1", "Doc Alpha");

      rerender(
        <WorkspaceSidebar
          refreshKey={0}
          activeWorkspaceId="w2"
          onSelectDocument={onSelectDocument}
          onSelectCanvas={noop}
          selectedDocumentId={null}
          selectedCanvasId={null}
          onWorkspaceDeleted={noop}
        />,
      );

      await act(async () => {
        createdDocument.resolve({
          id: "doc-alpha",
          workspace_id: "w1",
          title: "Doc Alpha",
          created_at: "",
          updated_at: "",
        });
        await createdDocument.promise;
      });

      expect(screen.queryByText("Doc Alpha")).not.toBeInTheDocument();
      expect(onSelectDocument).not.toHaveBeenCalled();
    } finally {
      promptSpy.mockRestore();
    }
  });

  it("loads the pins view as a bookmarks tree and opens/removes bookmarked Loom blocks", async () => {
    const api = vi.mocked(await import("../lib/api"), true);
    const onOpenLoomBlock = vi.fn();
    api.listWorkspaces.mockResolvedValue([
      { id: "w1", name: "Workspace 1", created_at: "", updated_at: "" },
    ]);
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.queryLoomView.mockResolvedValue({
      view_type: "pins",
      blocks: [
        loomBlock(),
        loomBlock({
          block_id: "file-alpha",
          content_type: "file",
          title: "Pinned Source File",
          original_filename: "source-file.md",
          pin_order: 1,
        }),
      ],
    });
    api.updateLoomBlock.mockResolvedValue(loomBlock({ pinned: false, pin_order: null }));
    api.setLoomBlockPinOrder.mockResolvedValue(loomBlock({ pinned: false, pin_order: null }));
    const blockUpdated = vi.fn();
    window.addEventListener("handshake:loom-block-updated", blockUpdated);

    try {
      render(
        <WorkspaceSidebar
          refreshKey={0}
          activeWorkspaceId="w1"
          onSelectDocument={noop}
          onSelectCanvas={noop}
          onOpenLoomBlock={onOpenLoomBlock}
          selectedDocumentId={null}
          selectedCanvasId={null}
          onWorkspaceDeleted={noop}
        />,
      );

      expect(await screen.findByTestId("loom-bookmarks-tree")).toHaveTextContent("Pinned Alpha");
      expect(screen.getByTestId("loom-bookmark.block-alpha")).toHaveAttribute("data-bookmark-kind", "block");
      expect(screen.getByTestId("loom-bookmark.file-alpha")).toHaveAttribute("data-bookmark-kind", "file");
      expect(api.queryLoomView).toHaveBeenCalledWith("w1", "pins", { limit: 100, offset: 0 });

      fireEvent.click(screen.getByTestId("loom-bookmark.block-alpha.open"));
      expect(onOpenLoomBlock).toHaveBeenCalledWith("block-alpha");

      fireEvent.click(screen.getByTestId("loom-bookmark.block-alpha.remove"));
      await waitFor(() =>
        expect(api.updateLoomBlock).toHaveBeenCalledWith("w1", "block-alpha", {
          pinned: false,
        }),
      );
      expect(api.setLoomBlockPinOrder).toHaveBeenCalledWith("w1", "block-alpha", null);
      expect(api.setLoomBlockPinOrder.mock.invocationCallOrder[0]).toBeLessThan(
        api.updateLoomBlock.mock.invocationCallOrder[0],
      );
      expect(await screen.findByTestId("loom-bookmarks.status")).toHaveTextContent("Bookmark removed");
      expect(screen.queryByTestId("loom-bookmark.block-alpha")).not.toBeInTheDocument();
      expect(screen.getByTestId("loom-bookmark.file-alpha")).toBeInTheDocument();
      await waitFor(() => expect(blockUpdated).toHaveBeenCalledTimes(1));
      expect((blockUpdated.mock.calls[0][0] as CustomEvent).detail).toEqual({
        workspaceId: "w1",
        block: expect.objectContaining({
          block_id: "block-alpha",
          pinned: false,
          pin_order: null,
        }),
      });
    } finally {
      window.removeEventListener("handshake:loom-block-updated", blockUpdated);
    }
  });

  it("does not unpin a bookmark when clearing pin order fails", async () => {
    const api = vi.mocked(await import("../lib/api"), true);
    api.listWorkspaces.mockResolvedValue([
      { id: "w1", name: "Workspace 1", created_at: "", updated_at: "" },
    ]);
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.queryLoomView.mockResolvedValue({ view_type: "pins", blocks: [loomBlock()] });
    api.setLoomBlockPinOrder.mockRejectedValue(new Error("pin order unavailable"));

    render(
      <WorkspaceSidebar
        refreshKey={0}
        activeWorkspaceId="w1"
        onSelectDocument={noop}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
        onWorkspaceDeleted={noop}
      />,
    );

    expect(await screen.findByTestId("loom-bookmark.block-alpha")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("loom-bookmark.block-alpha.remove"));

    expect(await screen.findByText("pin order unavailable")).toBeInTheDocument();
    expect(api.updateLoomBlock).not.toHaveBeenCalled();
    expect(screen.getByTestId("loom-bookmark.block-alpha")).toBeInTheDocument();
  });

  it("opens document-backed bookmarks through the document selection path", async () => {
    const api = vi.mocked(await import("../lib/api"), true);
    const onSelectDocument = vi.fn();
    const onSelectCanvas = vi.fn();
    const onOpenLoomBlock = vi.fn();
    api.listWorkspaces.mockResolvedValue([
      { id: "w1", name: "Workspace 1", created_at: "", updated_at: "" },
    ]);
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.queryLoomView.mockResolvedValue({
      view_type: "pins",
      blocks: [
        loomBlock({
          block_id: "doc-block",
          document_id: "doc-alpha",
          title: "Document Bookmark",
        }),
      ],
    });

    render(
      <WorkspaceSidebar
        refreshKey={0}
        activeWorkspaceId="w1"
        onSelectDocument={onSelectDocument}
        onSelectCanvas={onSelectCanvas}
        onOpenLoomBlock={onOpenLoomBlock}
        selectedDocumentId={null}
        selectedCanvasId={null}
        onWorkspaceDeleted={noop}
      />,
    );

    expect(await screen.findByTestId("loom-bookmark.doc-block")).toHaveAttribute("data-bookmark-kind", "document");
    fireEvent.click(screen.getByTestId("loom-bookmark.doc-block.open"));

    expect(onSelectDocument).toHaveBeenCalledWith("doc-alpha");
    expect(onSelectCanvas).toHaveBeenCalledWith(null);
    expect(onOpenLoomBlock).not.toHaveBeenCalled();
  });

  it("refreshes bookmarks when a Loom block property save announces a pin change", async () => {
    const api = vi.mocked(await import("../lib/api"), true);
    api.listWorkspaces.mockResolvedValue([
      { id: "w1", name: "Workspace 1", created_at: "", updated_at: "" },
    ]);
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.queryLoomView
      .mockResolvedValueOnce({ view_type: "pins", blocks: [] })
      .mockResolvedValueOnce({ view_type: "pins", blocks: [loomBlock({ block_id: "new-pin", title: "New Pin" })] });

    render(
      <WorkspaceSidebar
        refreshKey={0}
        activeWorkspaceId="w1"
        onSelectDocument={noop}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
        onWorkspaceDeleted={noop}
      />,
    );

    await waitFor(() => expect(api.queryLoomView).toHaveBeenCalledTimes(1));
    act(() => {
      window.dispatchEvent(new CustomEvent("handshake:loom-bookmarks-changed", { detail: { workspaceId: "w1" } }));
    });

    expect(await screen.findByTestId("loom-bookmark.new-pin")).toHaveTextContent("New Pin");
    expect(api.queryLoomView).toHaveBeenCalledTimes(2);
  });
});
