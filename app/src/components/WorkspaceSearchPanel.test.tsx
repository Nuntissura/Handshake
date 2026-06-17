import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { WorkspaceSearchPanel } from "./WorkspaceSearchPanel";
import {
  getWorkspaceSearchBookmarkState,
  loadRichDocument,
  saveRichDocument,
  saveWorkspaceSearchBookmarkState,
  searchLoomGraph,
  type RichDocLoad,
  type RichDocSaveResult,
} from "../lib/api";

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    searchLoomGraph: vi.fn(),
    loadRichDocument: vi.fn(),
    saveRichDocument: vi.fn(),
    getWorkspaceSearchBookmarkState: vi.fn(),
    saveWorkspaceSearchBookmarkState: vi.fn(),
  };
});

// MT-258: stateful stub of the durable search-bookmark backend so the panel's
// save -> reload projection round-trips through the route mock (NOT localStorage).
function installSearchBookmarkBackendStub() {
  const store = new Map<string, Record<string, unknown> | null>();
  vi.mocked(getWorkspaceSearchBookmarkState).mockImplementation(async (workspaceId: string) => ({
    workspace_id: workspaceId,
    bookmark_state: store.get(workspaceId) ?? null,
    updated_at: null,
    event_ledger_event_id: null,
  }));
  vi.mocked(saveWorkspaceSearchBookmarkState).mockImplementation(
    async (workspaceId: string, bookmarkState: Record<string, unknown>) => {
      store.set(workspaceId, bookmarkState);
      return {
        workspace_id: workspaceId,
        bookmark_state: bookmarkState,
        updated_at: "2026-06-17T00:00:00Z",
        event_ledger_event_id: `evt-${store.size}`,
      };
    },
  );
  return store;
}

const primaryRichDocumentId = "KRD-00000000000000000000000000000001";
const secondaryRichDocumentId = "KRD-00000000000000000000000000000002";

const richDocLoad = (contentText = "alpha beta alpha", documentId = primaryRichDocumentId): RichDocLoad => ({
  document: {
    rich_document_id: documentId,
    workspace_id: "w1",
    document_id: null,
    title: "Alpha design note",
    schema_version: "hsk.rich_document@1",
    doc_version: 4,
    content_json: {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: contentText }] }],
    },
    content_sha256: "sha",
    crdt_document_id: `crdt-${documentId}`,
    crdt_snapshot_id: null,
    promotion_receipt_event_id: null,
    projection_refs: {},
    project_ref: null,
    folder_ref: null,
    authority_label: "rich_document",
    owner_actor_kind: "operator",
    owner_actor_id: "operator",
    created_at: "2026-06-16T00:00:00Z",
    updated_at: "2026-06-16T00:00:00Z",
  },
  tree: {
    schema_version: "hsk.rich_document_tree@1",
    schema_matches: true,
    block_ids: [],
    blocks: [],
  },
  code_nodes: [],
});

const saveResult = (load: RichDocLoad): RichDocSaveResult => ({
  document: {
    ...load.document,
    doc_version: load.document.doc_version + 1,
    content_json: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "omega beta omega" }] }] },
  },
  save_receipt_event_id: "evt-save-1",
  backlinks_persisted: 0,
  backlinks_skipped_reason: null,
});

describe("WorkspaceSearchPanel (MT-250)", () => {
  beforeEach(() => {
    window.localStorage.clear();
    vi.mocked(searchLoomGraph).mockReset();
    vi.mocked(loadRichDocument).mockReset();
    vi.mocked(saveRichDocument).mockReset();
    vi.mocked(getWorkspaceSearchBookmarkState).mockReset();
    vi.mocked(saveWorkspaceSearchBookmarkState).mockReset();
    installSearchBookmarkBackendStub();
    vi.mocked(searchLoomGraph).mockResolvedValue([
      {
        result_kind: "knowledge_entity",
        source_kind: "document",
        ref_id: primaryRichDocumentId,
        title: "Alpha design note",
        excerpt: "alpha beta alpha",
        score: 7,
        metadata: { rich_document_id: primaryRichDocumentId, path: "src/app.md" },
      },
    ]);
  });

  it("queries the backend canonical search surface with filters and search operators", async () => {
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "Alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.tag-filter"), { target: { value: "tag-1, tag-2" } });
    fireEvent.change(screen.getByTestId("workspace-search.path-filter"), { target: { value: "src/app" } });
    fireEvent.change(screen.getByTestId("workspace-search.kind-filter"), { target: { value: "document" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));

    await waitFor(() =>
    expect(searchLoomGraph).toHaveBeenCalledWith("w1", {
      q: "Alpha",
      limit: 500,
        offset: 0,
        sourceKinds: ["document"],
        tagIds: ["tag-1", "tag-2"],
        caseSensitive: false,
        wholeWord: false,
        isRegex: false,
        path: "src/app",
      }),
    );
    expect(await screen.findByTestId("workspace-search.result.document.KRD-00000000000000000000000000000001")).toHaveTextContent(
      "Alpha design note",
    );
  });

  it("saves workspace search bookmarks and restores their filters for a backend search", async () => {
    const store = installSearchBookmarkBackendStub();
    const { unmount } = render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "Alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.tag-filter"), { target: { value: "tag-1, tag-2" } });
    fireEvent.change(screen.getByTestId("workspace-search.path-filter"), { target: { value: "src/app" } });
    fireEvent.change(screen.getByTestId("workspace-search.kind-filter"), { target: { value: "document" } });
    fireEvent.click(screen.getByTestId("workspace-search.toggle-case"));
    fireEvent.click(screen.getByTestId("workspace-search.toggle-word"));
    fireEvent.click(screen.getByTestId("workspace-search.save-bookmark"));

    expect(await screen.findByTestId("workspace-search.bookmark.alpha-document-tag-1-tag-2-src-app-case-word")).toHaveTextContent(
      "Alpha",
    );

    // Durability: the save went to the backend route (PostgreSQL + EventLedger),
    // carries the canonical schema_id, and NOT to localStorage.
    await waitFor(() => expect(saveWorkspaceSearchBookmarkState).toHaveBeenCalled());
    const savedBlob = store.get("w1") as { schema_id?: string; bookmarks?: unknown[] };
    expect(savedBlob.schema_id).toBe("hsk.workspace_search_bookmark_state@1");
    expect(savedBlob.bookmarks).toHaveLength(1);
    expect(window.localStorage.length).toBe(0);

    // Re-mounting re-hydrates the saved search from the durable backend, proving
    // the UI is a projection of canonical state (not browser-local memory).
    unmount();
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );
    expect(
      await screen.findByTestId("workspace-search.bookmark.alpha-document-tag-1-tag-2-src-app-case-word"),
    ).toHaveTextContent("Alpha");

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "Beta" } });
    fireEvent.change(screen.getByTestId("workspace-search.tag-filter"), { target: { value: "" } });
    fireEvent.change(screen.getByTestId("workspace-search.path-filter"), { target: { value: "" } });
    fireEvent.change(screen.getByTestId("workspace-search.kind-filter"), { target: { value: "all" } });
    fireEvent.click(screen.getByTestId("workspace-search.toggle-case"));
    fireEvent.click(screen.getByTestId("workspace-search.toggle-word"));

    fireEvent.click(screen.getByTestId("workspace-search.bookmark.alpha-document-tag-1-tag-2-src-app-case-word.restore"));
    expect(screen.getByTestId("workspace-search.query")).toHaveValue("Alpha");
    expect(screen.getByTestId("workspace-search.tag-filter")).toHaveValue("tag-1, tag-2");
    expect(screen.getByTestId("workspace-search.path-filter")).toHaveValue("src/app");
    expect(screen.getByTestId("workspace-search.kind-filter")).toHaveValue("document");
    expect(screen.getByTestId("workspace-search.toggle-case")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("workspace-search.toggle-word")).toHaveAttribute("aria-pressed", "true");

    vi.mocked(searchLoomGraph).mockClear();
    fireEvent.click(screen.getByTestId("workspace-search.search"));
    await waitFor(() =>
      expect(searchLoomGraph).toHaveBeenCalledWith("w1", {
        q: "Alpha",
        limit: 500,
        offset: 0,
        sourceKinds: ["document"],
        tagIds: ["tag-1", "tag-2"],
        caseSensitive: true,
        wholeWord: true,
        isRegex: false,
        path: "src/app",
      }),
    );
  });

  it("translates inline search operators into canonical backend filters", async () => {
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), {
      target: { value: 'Alpha tag:#tag-1 path:"src/app" kind:document mention:MT-258' },
    });
    fireEvent.click(screen.getByTestId("workspace-search.search"));

    await waitFor(() =>
      expect(searchLoomGraph).toHaveBeenCalledWith("w1", {
        q: "Alpha",
        limit: 500,
        offset: 0,
        sourceKinds: ["document"],
        tagIds: ["tag-1"],
        mentionIds: ["MT-258"],
        caseSensitive: false,
        wholeWord: false,
        isRegex: false,
        path: "src/app",
      }),
    );
  });

  it("pages through the backend canonical search set instead of stopping at the first result page", async () => {
    const pageOne = Array.from({ length: 500 }, (_, index) => ({
      result_kind: "knowledge_entity" as const,
      source_kind: "document" as const,
      ref_id: `KRD-page-one-${index}`,
      title: `Alpha page one ${index}`,
      excerpt: "alpha first page",
      score: 7,
      metadata: { rich_document_id: `KRD-page-one-${index}` },
    }));
    vi.mocked(searchLoomGraph)
      .mockResolvedValueOnce(pageOne)
      .mockResolvedValueOnce([
        {
          result_kind: "knowledge_entity",
          source_kind: "document",
          ref_id: secondaryRichDocumentId,
          title: "Alpha result beyond first page",
          excerpt: "alpha second page",
          score: 6,
          metadata: { rich_document_id: secondaryRichDocumentId },
        },
      ]);

    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));

    expect(
      await screen.findByTestId(`workspace-search.result.document.${secondaryRichDocumentId}`),
    ).toHaveTextContent("Alpha result beyond first page");
    expect(searchLoomGraph).toHaveBeenNthCalledWith(
      2,
      "w1",
      expect.objectContaining({ limit: 500, offset: 500 }),
    );
  });

  it("opens document search hits without requiring the hit to be visible elsewhere", async () => {
    const onOpenDocument = vi.fn();
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={onOpenDocument}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "Alpha" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));

    const result = await screen.findByTestId("workspace-search.result.document.KRD-00000000000000000000000000000001");
    fireEvent.click(result);
    expect(onOpenDocument).toHaveBeenCalledWith("KRD-00000000000000000000000000000001", {
      caseSensitive: false,
      isRegex: false,
      query: "Alpha",
      wholeWord: false,
    });
  });

  it("opens advertised non-document result kinds through the app navigation callbacks", async () => {
    const onOpenCodeSymbol = vi.fn();
    const onOpenWorkPacket = vi.fn();
    const onOpenMicroTask = vi.fn();
    const onOpenUserManualPage = vi.fn();
    const onOpenWikiPage = vi.fn();
    vi.mocked(searchLoomGraph).mockResolvedValueOnce([
      {
        result_kind: "knowledge_entity",
        source_kind: "symbol",
        ref_id: "KEN-symbol-alpha",
        title: "AlphaSymbol",
        excerpt: "alpha symbol",
        score: 7,
        metadata: {},
      },
      {
        result_kind: "knowledge_entity",
        source_kind: "work_packet",
        ref_id: "WP-KERNEL-009",
        title: "WP Alpha",
        excerpt: "alpha work packet",
        score: 6,
        metadata: { work_packet_id: "WP-KERNEL-009" },
      },
      {
        result_kind: "knowledge_entity",
        source_kind: "micro_task",
        ref_id: "MT-250",
        title: "MT Alpha",
        excerpt: "alpha microtask",
        score: 5,
        metadata: { micro_task_id: "MT-250", work_packet_id: "WP-KERNEL-009" },
      },
      {
        result_kind: "user_manual_page",
        source_kind: "user_manual_page",
        ref_id: "manual-alpha",
        title: "Manual Alpha",
        excerpt: "alpha manual",
        score: 4,
        metadata: { page_slug: "manual-alpha" },
      },
      {
        result_kind: "wiki_page",
        source_kind: "wiki_page",
        ref_id: "KWP-alpha",
        title: "Wiki Alpha",
        excerpt: "alpha wiki",
        score: 3,
        metadata: { projection_id: "KWP-alpha" },
      },
    ]);

    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
        onOpenCodeSymbol={onOpenCodeSymbol}
        onOpenWorkPacket={onOpenWorkPacket}
        onOpenMicroTask={onOpenMicroTask}
        onOpenUserManualPage={onOpenUserManualPage}
        onOpenWikiPage={onOpenWikiPage}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));

    fireEvent.click(await screen.findByTestId("workspace-search.result.symbol.KEN-symbol-alpha"));
    fireEvent.click(await screen.findByTestId("workspace-search.result.work_packet.WP-KERNEL-009"));
    fireEvent.click(await screen.findByTestId("workspace-search.result.micro_task.MT-250"));
    fireEvent.click(await screen.findByTestId("workspace-search.result.user_manual_page.manual-alpha"));
    fireEvent.click(await screen.findByTestId("workspace-search.result.wiki_page.KWP-alpha"));

    expect(onOpenCodeSymbol).toHaveBeenCalledWith("KEN-symbol-alpha");
    expect(onOpenWorkPacket).toHaveBeenCalledWith("WP-KERNEL-009");
    expect(onOpenMicroTask).toHaveBeenCalledWith({ mtId: "MT-250", wpId: "WP-KERNEL-009" });
    expect(onOpenUserManualPage).toHaveBeenCalledWith("manual-alpha");
    expect(onOpenWikiPage).toHaveBeenCalledWith("KWP-alpha");
  });

  it("blocks invalid regex search before calling the backend", async () => {
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("workspace-search.toggle-regex"));
    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "(unclosed" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));

    expect(searchLoomGraph).not.toHaveBeenCalled();
    expect(await screen.findByTestId("workspace-search.status")).toHaveTextContent("Invalid regular expression");
  });

  it("previews replace-in-files before saving and cancel leaves the document untouched", async () => {
    const loaded = richDocLoad();
    vi.mocked(loadRichDocument).mockResolvedValue(loaded);
    vi.mocked(saveRichDocument).mockResolvedValue(saveResult(loaded));
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.replace"), { target: { value: "omega" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));
    await screen.findByTestId("workspace-search.result.document.KRD-00000000000000000000000000000001");

    fireEvent.click(screen.getByTestId("workspace-search.preview-replace"));

    expect(await screen.findByTestId("workspace-search.preview.KRD-00000000000000000000000000000001")).toHaveTextContent(
      "2 matches",
    );
    expect(screen.getByTestId("workspace-search.preview.KRD-00000000000000000000000000000001")).toHaveTextContent(
      "omega beta omega",
    );
    expect(saveRichDocument).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("workspace-search.cancel-replace"));
    expect(screen.queryByTestId("workspace-search.preview.KRD-00000000000000000000000000000001")).toBeNull();
    expect(saveRichDocument).not.toHaveBeenCalled();
  });

  it("shows one confirmed preview row per replacement match before applying", async () => {
    const loaded = richDocLoad("alpha beta alphabet alpha");
    vi.mocked(loadRichDocument).mockResolvedValue(loaded);
    vi.mocked(saveRichDocument).mockResolvedValue(saveResult(loaded));
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("workspace-search.toggle-word"));
    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.replace"), { target: { value: "omega" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));
    await screen.findByTestId("workspace-search.result.document.KRD-00000000000000000000000000000001");

    fireEvent.click(screen.getByTestId("workspace-search.preview-replace"));

    expect(await screen.findByTestId("workspace-search.preview.KRD-00000000000000000000000000000001")).toHaveTextContent(
      "2 matches",
    );
    expect(
      await screen.findByTestId("workspace-search.preview.KRD-00000000000000000000000000000001.match.1"),
    ).toHaveTextContent("Match 1");
    expect(screen.getByTestId("workspace-search.preview.KRD-00000000000000000000000000000001.match.1.before")).toHaveTextContent(
      "alpha beta alphabet",
    );
    expect(screen.getByTestId("workspace-search.preview.KRD-00000000000000000000000000000001.match.1.after")).toHaveTextContent(
      "omega beta alphabet",
    );
    expect(
      await screen.findByTestId("workspace-search.preview.KRD-00000000000000000000000000000001.match.2"),
    ).toHaveTextContent("Match 2");
    expect(screen.getByTestId("workspace-search.preview.KRD-00000000000000000000000000000001.match.2.before")).toHaveTextContent(
      "alphabet alpha",
    );
    expect(screen.getByTestId("workspace-search.preview.KRD-00000000000000000000000000000001.match.2.after")).toHaveTextContent(
      "alphabet omega",
    );
    expect(saveRichDocument).not.toHaveBeenCalled();
  });

  it("previews replacements against the backend result set even when a hit is hidden by client filters", async () => {
    vi.mocked(searchLoomGraph).mockResolvedValueOnce([
      {
        result_kind: "knowledge_entity",
        source_kind: "document",
        ref_id: primaryRichDocumentId,
        title: "alpha visible note",
        excerpt: "alpha visible hit",
        score: 7,
        metadata: { rich_document_id: primaryRichDocumentId },
      },
      {
        result_kind: "knowledge_entity",
        source_kind: "document",
        ref_id: secondaryRichDocumentId,
        title: "ALPHA hidden note",
        excerpt: "ALPHA hidden by case-sensitive client filter",
        score: 6,
        metadata: { rich_document_id: secondaryRichDocumentId },
      },
    ]);
    vi.mocked(loadRichDocument).mockImplementation(async (documentId) =>
      documentId === secondaryRichDocumentId
        ? richDocLoad("alpha hidden canonical doc", secondaryRichDocumentId)
        : richDocLoad("alpha visible doc", primaryRichDocumentId),
    );

    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("workspace-search.toggle-case"));
    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.replace"), { target: { value: "omega" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));

    await screen.findByTestId(`workspace-search.result.document.${primaryRichDocumentId}`);
    expect(screen.queryByTestId(`workspace-search.result.document.${secondaryRichDocumentId}`)).toBeNull();

    fireEvent.click(screen.getByTestId("workspace-search.preview-replace"));

    expect(await screen.findByTestId(`workspace-search.preview.${secondaryRichDocumentId}`)).toHaveTextContent(
      "omega hidden canonical doc",
    );
    expect(loadRichDocument).toHaveBeenCalledWith(secondaryRichDocumentId);
    expect(saveRichDocument).not.toHaveBeenCalled();
  });

  it("applies confirmed replacements through the rich-document save path with a receipt", async () => {
    const loaded = richDocLoad();
    vi.mocked(loadRichDocument).mockResolvedValue(loaded);
    vi.mocked(saveRichDocument).mockResolvedValue(saveResult(loaded));
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.replace"), { target: { value: "omega" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));
    await screen.findByTestId("workspace-search.result.document.KRD-00000000000000000000000000000001");
    fireEvent.click(screen.getByTestId("workspace-search.preview-replace"));
    await screen.findByTestId("workspace-search.preview.KRD-00000000000000000000000000000001");

    fireEvent.click(screen.getByTestId("workspace-search.apply-replace"));

    await waitFor(() => expect(saveRichDocument).toHaveBeenCalledTimes(1));
    const [documentId, expectedVersion, contentJson, , metadata] = vi.mocked(saveRichDocument).mock.calls[0];
    expect(documentId).toBe("KRD-00000000000000000000000000000001");
    expect(expectedVersion).toBe(4);
    expect(JSON.stringify(contentJson)).toContain("omega beta omega");
    expect(metadata).toEqual({ crdt_document_id: "crdt-KRD-00000000000000000000000000000001" });
    expect(await screen.findByTestId("workspace-search.replace-status")).toHaveTextContent("evt-save-1");
  });

  it("blocks apply when the replacement controls changed after preview", async () => {
    const loaded = richDocLoad();
    vi.mocked(loadRichDocument).mockResolvedValue(loaded);
    vi.mocked(saveRichDocument).mockResolvedValue(saveResult(loaded));
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.replace"), { target: { value: "omega" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));
    await screen.findByTestId("workspace-search.result.document.KRD-00000000000000000000000000000001");
    fireEvent.click(screen.getByTestId("workspace-search.preview-replace"));
    await screen.findByTestId("workspace-search.preview.KRD-00000000000000000000000000000001");

    fireEvent.change(screen.getByTestId("workspace-search.replace"), { target: { value: "zeta" } });
    fireEvent.click(screen.getByTestId("workspace-search.apply-replace"));

    expect(saveRichDocument).not.toHaveBeenCalled();
    expect(await screen.findByTestId("workspace-search.replace-status")).toHaveTextContent("Preview is stale");
  });

  it("blocks replace preview when search controls changed after the last backend search", async () => {
    const loaded = richDocLoad();
    vi.mocked(loadRichDocument).mockResolvedValue(loaded);
    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.replace"), { target: { value: "omega" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));
    await screen.findByTestId("workspace-search.result.document.KRD-00000000000000000000000000000001");

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "beta" } });
    fireEvent.click(screen.getByTestId("workspace-search.preview-replace"));

    expect(loadRichDocument).not.toHaveBeenCalled();
    expect(await screen.findByTestId("workspace-search.replace-status")).toHaveTextContent("Search results are stale");
  });

  it("surfaces successful receipts when a later replace apply save fails", async () => {
    vi.mocked(searchLoomGraph).mockResolvedValueOnce([
      {
        result_kind: "knowledge_entity",
        source_kind: "document",
        ref_id: primaryRichDocumentId,
        title: "alpha primary",
        excerpt: "alpha primary",
        score: 7,
        metadata: { rich_document_id: primaryRichDocumentId },
      },
      {
        result_kind: "knowledge_entity",
        source_kind: "document",
        ref_id: secondaryRichDocumentId,
        title: "alpha secondary",
        excerpt: "alpha secondary",
        score: 6,
        metadata: { rich_document_id: secondaryRichDocumentId },
      },
    ]);
    vi.mocked(loadRichDocument).mockImplementation(async (documentId) => richDocLoad("alpha", documentId));
    vi.mocked(saveRichDocument)
      .mockResolvedValueOnce({
        ...saveResult(richDocLoad("alpha", primaryRichDocumentId)),
        save_receipt_event_id: "evt-primary",
      })
      .mockRejectedValueOnce(new Error("secondary save failed"));

    render(
      <WorkspaceSearchPanel
        open={true}
        workspaceId="w1"
        onClose={vi.fn()}
        onOpenDocument={vi.fn()}
        onOpenLoomBlock={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "alpha" } });
    fireEvent.change(screen.getByTestId("workspace-search.replace"), { target: { value: "omega" } });
    fireEvent.click(screen.getByTestId("workspace-search.search"));
    await screen.findByTestId(`workspace-search.result.document.${secondaryRichDocumentId}`);
    fireEvent.click(screen.getByTestId("workspace-search.preview-replace"));
    await screen.findByTestId(`workspace-search.preview.${secondaryRichDocumentId}`);

    fireEvent.click(screen.getByTestId("workspace-search.apply-replace"));

    await waitFor(() => expect(saveRichDocument).toHaveBeenCalledTimes(2));
    expect(await screen.findByTestId("workspace-search.replace-status")).toHaveTextContent("evt-primary");
    expect(await screen.findByTestId("workspace-search.status")).toHaveTextContent("secondary save failed");
    expect(screen.getByTestId("workspace-search.apply-replace")).toBeDisabled();
  });
});
