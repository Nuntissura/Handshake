// WP-KERNEL-009 RichDocumentCore (MT-145..MT-160): RichDocumentView wires the
// editor to the REAL backend RichDocument authority. This test drives the
// load -> edit -> save round-trip against a mocked authority API and asserts
// the stable selectors (document, blocks, embeds, broken-embeds, backlinks,
// history) render so the later TiptapMonaco/Loom GUI groups can consume them.

import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { act } from "react";
import { vi } from "vitest";
import { RichDocumentView } from "./RichDocumentView";
import type { JSONContent } from "@tiptap/core";
import { makeCodeBlockAttrs } from "../lib/editor/code_block_serialization";

vi.mock("../state/debugEvents", () => ({ logEvent: vi.fn() }));

const richTextEditorPropLog = vi.hoisted(() => [] as Array<{
  collaborationDocument?: unknown;
  collaborationResetToken?: unknown;
  commandPaletteRequest?: { paneId: string; requestId: number; query: string } | null;
  debugId?: string;
}>);

// The save/load/authority plumbing under test does not need the real Monaco
// editor; mock RichTextEditor as a textarea so this suite stays fast and
// deterministic. The integrated editor itself is covered by
// RichTextEditor.test.tsx; the RichDocumentView<->editor wiring (schema assert,
// backend-error pass-through) is covered below via the props the mock receives.
vi.mock("./RichTextEditor", () => ({
  RichTextEditor: ({
    initialContent,
    onChange,
    backendError,
    readOnly,
    collaborationDocument,
    collaborationResetToken,
    commandPaletteRequest,
    debugId,
  }: {
    initialContent: JSONContent | null;
    onChange: (doc: JSONContent | null) => void;
    backendError?: { kind: string; message: string } | null;
    readOnly?: boolean;
    collaborationDocument?: unknown;
    collaborationResetToken?: unknown;
    commandPaletteRequest?: { paneId: string; requestId: number; query: string } | null;
    debugId?: string;
  }) => {
    richTextEditorPropLog.push({ collaborationDocument, collaborationResetToken, commandPaletteRequest, debugId });
    return (
      <div>
      <textarea
        data-testid="tiptap-editor"
        data-readonly={readOnly ? "true" : "false"}
        data-content={JSON.stringify(initialContent)}
        defaultValue={JSON.stringify(initialContent)}
        onChange={() =>
          onChange({
            type: "doc",
            content: [
              { type: "paragraph", content: [{ type: "text", text: "local-edit-marker" }] },
            ],
          })
        }
      />
      {backendError ? (
        <div data-testid="rte-backend-error" data-error-kind={backendError.kind}>
          {backendError.message}
        </div>
      ) : null}
      </div>
    );
  },
}));

const DOC_V1 = {
  rich_document_id: "KRD-00000000000000000000000000000001",
  workspace_id: "w1",
  document_id: null,
  title: "Runbook",
  schema_version: "rich_document_v1",
  doc_version: 1,
  content_json: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "Hi" }] }] },
  content_sha256: "0".repeat(64),
  crdt_document_id: null as string | null,
  crdt_snapshot_id: null as string | null,
  promotion_receipt_event_id: null as string | null,
  projection_refs: [],
  project_ref: "PRJ-x",
  folder_ref: "runbooks",
  authority_label: "promoted",
  owner_actor_kind: "operator",
  owner_actor_id: "operator",
  created_at: "2026-06-12T00:00:00Z",
  updated_at: "2026-06-12T00:00:00Z",
};

vi.mock("../lib/api", () => {
  const loadRichDocument = vi.fn(async () => ({
    document: DOC_V1,
    tree: {
      schema_version: "rich_document_v1",
      schema_matches: true,
      block_ids: ["KBL-a", "KBL-b"],
      blocks: [
        {
          block_id: "KBL-a",
          kind: "heading",
          heading_level: 1,
          sequence: 0,
          content: { raw: {}, derived: { plain_text: "Runbook", word_count: 1, preview: "Runbook" }, display: {} },
        },
        {
          block_id: "KBL-b",
          kind: "image",
          heading_level: null,
          sequence: 1,
          content: { raw: {}, derived: { plain_text: "", word_count: 0, preview: "" }, display: {} },
        },
      ],
    },
    code_nodes: [],
  }));
  const saveRichDocument = vi.fn(async () => ({
    document: { ...DOC_V1, doc_version: 2 },
    save_receipt_event_id: "EVT-1",
    backlinks_persisted: 1,
    backlinks_skipped_reason: null,
  }));
  const loadRichDocumentHistory = vi.fn(async () => ({
    rich_document_id: DOC_V1.rich_document_id,
    current_version: 1,
    authority_label: "promoted",
    owner_actor_kind: "operator",
    owner_actor_id: "operator",
    versions: [
      { rich_document_id: DOC_V1.rich_document_id, doc_version: 1, schema_version: "rich_document_v1", content_sha256: "0".repeat(64), promotion_receipt_event_id: null, created_at: "2026-06-12T00:00:00Z" },
    ],
  }));
  const loadRichDocumentVersion = vi.fn(async (documentId: string, docVersion: number) => ({
    rich_document_id: documentId,
    version: {
      rich_document_id: documentId,
      doc_version: docVersion,
      schema_version: "rich_document_v1",
      content_json: DOC_V1.content_json,
      content_sha256: "0".repeat(64),
      crdt_snapshot_id: null,
      promotion_receipt_event_id: null,
      created_at: "2026-06-12T00:00:00Z",
    },
  }));
  const loadRichDocumentDraft = vi.fn(async () => ({
    rich_document_id: DOC_V1.rich_document_id,
    current_doc_version: DOC_V1.doc_version,
    current_content_sha256: DOC_V1.content_sha256,
    draft: null,
  }));
  const upsertRichDocumentDraft = vi.fn(async () => ({
    rich_document_id: DOC_V1.rich_document_id,
    draft: null,
    cleared: false,
    draft_receipt_event_id: "EVT-DRAFT",
    receipt_error: null,
  }));
  const clearRichDocumentDraft = vi.fn(async () => ({
    rich_document_id: DOC_V1.rich_document_id,
    cleared: true,
    clear_receipt_event_id: "EVT-DRAFT-CLEAR",
    receipt_error: null,
  }));
  const listRichDocumentEmbeds = vi.fn(async () => ({
    rich_document_id: DOC_V1.rich_document_id,
    embeds: [
      { embed_id: "KEMB-1", rich_document_id: DOC_V1.rich_document_id, block_id: "KBL-b", ref_kind: "media", ref_value: "KMED-1", caption: null, repair_state: "broken", repair_reason: "missing" },
    ],
  }));
  const listRichDocumentBrokenEmbeds = vi.fn(async () => ({
    rich_document_id: DOC_V1.rich_document_id,
    broken_embeds: [
      { embed_id: "KEMB-1", rich_document_id: DOC_V1.rich_document_id, block_id: "KBL-b", ref_kind: "media", ref_value: "KMED-1", caption: null, repair_state: "broken", repair_reason: "missing" },
    ],
    available_actions: ["relink", "reresolve", "remove"],
  }));
  const listRichDocumentBacklinks = vi.fn(async () => ({
    source_document_id: DOC_V1.rich_document_id,
    backlinks: [
      { backlink_id: "KDBL-1", workspace_id: "w1", relationship_id: "KDLNK-" + "0".repeat(64), source_document_id: DOC_V1.rich_document_id, link_kind: "wikilink", target: "Guide", block_id: "KBL-a" },
    ],
  }));
  return {
    DEFAULT_RICH_DOC_CONTEXT: { actor_id: "operator", kernel_task_run_id: "KTR-EDITOR-UI", session_run_id: "SR-EDITOR-UI", actor_kind: "operator" },
    loadRichDocument,
    saveRichDocument,
    loadRichDocumentDraft,
    upsertRichDocumentDraft,
    clearRichDocumentDraft,
    loadRichDocumentHistory,
    loadRichDocumentVersion,
    listRichDocumentEmbeds,
    listRichDocumentBrokenEmbeds,
    listRichDocumentBacklinks,
  };
});

function docFixtureFor(documentId: string, overrides: Partial<typeof DOC_V1> = {}): typeof DOC_V1 {
  return {
    ...DOC_V1,
    rich_document_id: documentId,
    title: `Runbook ${documentId.slice(-4)}`,
    content_json: {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: documentId }] }],
    },
    ...overrides,
  };
}

function restoreRichDocumentApiDefaults(api: typeof import("../lib/api")): void {
  vi.mocked(api.loadRichDocument).mockImplementation(async (requestedId: string) => ({
    document: docFixtureFor(requestedId),
    tree: {
      schema_version: "rich_document_v1",
      schema_matches: true,
      block_ids: [],
      blocks: [],
    },
    code_nodes: [],
  }));
  vi.mocked(api.saveRichDocument).mockImplementation(
    async (docId, _expectedVersion, contentJson, _ctx, metadata) => ({
      document: docFixtureFor(docId, {
        doc_version: 2,
        content_json: contentJson as typeof DOC_V1.content_json,
        crdt_document_id: metadata?.crdt_document_id ?? null,
      }),
      save_receipt_event_id: "EVT-1",
      backlinks_persisted: 1,
      backlinks_skipped_reason: null,
    }),
  );
  vi.mocked(api.loadRichDocumentDraft).mockImplementation(async (requestedId: string) => {
    const doc = docFixtureFor(requestedId);
    return {
      rich_document_id: requestedId,
      current_doc_version: doc.doc_version,
      current_content_sha256: doc.content_sha256,
      draft: null,
    };
  });
  vi.mocked(api.upsertRichDocumentDraft).mockResolvedValue({
    rich_document_id: DOC_V1.rich_document_id,
    draft: null,
    cleared: false,
    draft_receipt_event_id: "EVT-DRAFT",
    receipt_error: null,
  });
  vi.mocked(api.clearRichDocumentDraft).mockResolvedValue({
    rich_document_id: DOC_V1.rich_document_id,
    cleared: true,
    clear_receipt_event_id: "EVT-DRAFT-CLEAR",
    receipt_error: null,
  });
}

describe("RichDocumentView (MT-145..MT-160)", () => {
  it("does not report an initial clean rich document state as a dirty transition", async () => {
    const dirtyReports: boolean[] = [];

    render(
      <RichDocumentView
        documentId="KRD-00000000000000000000000000000001"
        onDirtyChange={(dirty) => dirtyReports.push(dirty)}
      />,
    );

    await screen.findByTestId("rich-document-view");
    await act(async () => {});

    expect(dirtyReports).toEqual([]);
  });

  it("loads from authority and renders stable block/embed/backlink/history selectors", async () => {
    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });

    // Document load selector with stable id + authority metadata (MT-145).
    const view = await screen.findByTestId("rich-document-view");
    expect(view.getAttribute("data-stable-id")).toBe("KRD-00000000000000000000000000000001");
    expect(view.getAttribute("data-authority-label")).toBe("promoted");
    expect(view.getAttribute("data-schema-matches")).toBe("true");

    // Block render with stable per-block selectors (MT-146/148).
    await waitFor(() => {
      const blocks = screen.getAllByTestId("rich-document-block");
      expect(blocks).toHaveLength(2);
      expect(blocks[0].getAttribute("data-stable-id")).toBe("KBL-a");
      expect(blocks[0].getAttribute("data-block-kind")).toBe("heading");
      expect(blocks[1].getAttribute("data-block-kind")).toBe("image");
    });

    // Embed + broken-embed selectors (MT-152/153).
    await waitFor(() => {
      const embed = screen.getByTestId("rich-document-embed");
      expect(embed.getAttribute("data-embed-state")).toBe("broken");
      expect(screen.getByTestId("rich-document-broken-embeds")).toBeTruthy();
    });

    // Backlink (MT-155) + history (MT-156) selectors.
    await waitFor(() => {
      expect(screen.getByTestId("rich-document-backlink").getAttribute("data-link-kind")).toBe("wikilink");
      expect(screen.getByTestId("rich-document-version").getAttribute("data-doc-version")).toBe("1");
    });
  });

  it("loads a selected history pair and renders prose plus Monaco-ready code diffs (MT-247)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000000001";
    const before = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "Intro v1" }] },
        { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("ts", "const count = 1;") },
      ],
    };
    const after = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "Intro v2" }] },
        { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("typescript", "const count = 2;") },
      ],
    };
    const versionLoader = (
      api as unknown as {
        loadRichDocumentVersion: ReturnType<typeof vi.fn>;
      }
    ).loadRichDocumentVersion;
    vi.mocked(api.loadRichDocumentHistory).mockResolvedValueOnce({
      rich_document_id: documentId,
      current_version: 2,
      authority_label: "promoted",
      owner_actor_kind: "operator",
      owner_actor_id: "operator",
      versions: [
        { rich_document_id: documentId, doc_version: 1, schema_version: "rich_document_v1", content_sha256: "1".repeat(64), promotion_receipt_event_id: null, created_at: "2026-06-12T00:00:00Z" },
        { rich_document_id: documentId, doc_version: 2, schema_version: "rich_document_v1", content_sha256: "2".repeat(64), promotion_receipt_event_id: null, created_at: "2026-06-12T01:00:00Z" },
      ],
    });
    versionLoader.mockImplementation(async (_requestedId: string, docVersion: number) => ({
      rich_document_id: documentId,
      version: {
        rich_document_id: documentId,
        doc_version: docVersion,
        schema_version: "rich_document_v1",
        content_json: docVersion === 1 ? before : after,
        content_sha256: String(docVersion).repeat(64),
        crdt_snapshot_id: null,
        promotion_receipt_event_id: null,
        created_at: `2026-06-12T0${docVersion - 1}:00:00Z`,
      },
    }));

    await act(async () => {
      render(<RichDocumentView documentId={documentId} />);
    });

    expect(((await screen.findByTestId("rich-document-diff-left-version")) as HTMLSelectElement).value).toBe("1");
    expect((screen.getByTestId("rich-document-diff-right-version") as HTMLSelectElement).value).toBe("2");
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-diff-load"));
    });

    await waitFor(() => {
      expect(versionLoader).toHaveBeenCalledWith(documentId, 1);
      expect(versionLoader).toHaveBeenCalledWith(documentId, 2);
    });
    const diffPanel = await screen.findByTestId("rich-document-diff-panel");
    expect(diffPanel.getAttribute("data-left-version")).toBe("1");
    expect(diffPanel.getAttribute("data-right-version")).toBe("2");
    const rows = screen.getAllByTestId("rich-document-diff-block");
    expect(rows.map((row) => row.getAttribute("data-diff-status"))).toEqual(["modified", "modified"]);
    expect(rows.map((row) => row.getAttribute("data-diff-kind"))).toEqual(["prose", "code"]);
    expect(screen.getByTestId("rich-document-diff-block-0-left").textContent).toContain("Intro v1");
    expect(screen.getByTestId("rich-document-diff-block-0-right").textContent).toContain("Intro v2");

    const codeDiff = screen.getByTestId("rich-document-code-diff-1");
    expect(codeDiff.getAttribute("data-monaco-diff")).toBe("true");
    expect(codeDiff.getAttribute("data-language")).toBe("typescript");
    expect(screen.getByTestId("rich-document-code-diff-1-left").textContent).toContain("const count = 1;");
    expect(screen.getByTestId("rich-document-code-diff-1-right").textContent).toContain("const count = 2;");
  });

  it("passes app command-palette requests through to the rich text editor", async () => {
    richTextEditorPropLog.length = 0;

    await act(async () => {
      render(
        <RichDocumentView
          documentId="KRD-00000000000000000000000000000001"
          commandPaletteRequest={{ paneId: "pane-a", requestId: 7, query: "Bold" }}
        />,
      );
    });

    await screen.findByTestId("rich-document-view");
    await waitFor(() =>
      expect(richTextEditorPropLog.some((entry) => entry.commandPaletteRequest?.query === "Bold")).toBe(true),
    );
  });

  it("shares one CRDT collaboration document across same-document rich editor shells (MT-246)", async () => {
    richTextEditorPropLog.length = 0;

    render(
      <>
        <RichDocumentView documentId="KRD-00000000000000000000000000000001" />
        <RichDocumentView documentId="KRD-00000000000000000000000000000001" />
      </>,
    );

    await waitFor(() => {
      const sameDocumentEditors = richTextEditorPropLog.filter(
        (entry) => entry.debugId === "KRD-00000000000000000000000000000001",
      );
      const collaborationDocuments = sameDocumentEditors
        .map((entry) => entry.collaborationDocument)
        .filter(Boolean);
      expect(collaborationDocuments.length).toBeGreaterThanOrEqual(2);
      expect(new Set(collaborationDocuments).size).toBe(1);
    });
  });

  it("keeps an active same-document collaboration document cached while many other docs open (MT-246)", async () => {
    const api = await import("../lib/api");
    vi.mocked(api.loadRichDocument).mockImplementation(async (documentId: string) => ({
      document: docFixtureFor(documentId),
      tree: {
        schema_version: "rich_document_v1",
        schema_matches: true,
        block_ids: [],
        blocks: [],
      },
      code_nodes: [],
    }));
    richTextEditorPropLog.length = 0;

    const firstDocId = "KRD-00000000000000000000000000009999";
    const mountedViews: Array<ReturnType<typeof render>> = [];
    await act(async () => {
      mountedViews.push(render(<RichDocumentView documentId={firstDocId} />));
    });
    await waitFor(() => {
      expect(richTextEditorPropLog.some((entry) => entry.debugId === firstDocId)).toBe(true);
    });
    const firstCollaborationDocument = richTextEditorPropLog.find(
      (entry) => entry.debugId === firstDocId,
    )?.collaborationDocument;
    expect(firstCollaborationDocument).toBeTruthy();

    let others: Array<ReturnType<typeof render>> = [];
    await act(async () => {
      others = Array.from({ length: 70 }, (_, index) =>
        render(
          <RichDocumentView
            documentId={`KRD-${String(index).padStart(32, "0")}`}
          />,
        ),
      );
      mountedViews.push(...others);
    });
    await waitFor(() => {
      expect(
        richTextEditorPropLog.filter((entry) => entry.debugId && entry.debugId !== firstDocId).length,
      ).toBeGreaterThanOrEqual(70);
    });

    await act(async () => {
      mountedViews.push(render(<RichDocumentView documentId={firstDocId} />));
    });
    await waitFor(() => {
      const sameDocumentEditors = richTextEditorPropLog.filter((entry) => entry.debugId === firstDocId);
      expect(sameDocumentEditors.length).toBeGreaterThanOrEqual(2);
      expect(sameDocumentEditors[sameDocumentEditors.length - 1]?.collaborationDocument).toBe(
        firstCollaborationDocument,
      );
    });

    for (const rendered of mountedViews) rendered.unmount();
  });

  it("carries a provisional collaboration document across backend CRDT id promotion (MT-246)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000008888";
    const crdtDocumentId = "KCRDT-000000000000000000000000008888";
    vi.mocked(api.loadRichDocument)
      .mockResolvedValueOnce({
        document: docFixtureFor(documentId, { crdt_document_id: null }),
        tree: {
          schema_version: "rich_document_v1",
          schema_matches: true,
          block_ids: [],
          blocks: [],
        },
        code_nodes: [],
      })
      .mockResolvedValueOnce({
        document: docFixtureFor(documentId, { crdt_document_id: crdtDocumentId }),
        tree: {
          schema_version: "rich_document_v1",
          schema_matches: true,
          block_ids: [],
          blocks: [],
        },
        code_nodes: [],
      });
    richTextEditorPropLog.length = 0;
    const documentEntries = () => richTextEditorPropLog.filter((entry) => entry.debugId === documentId);

    render(<RichDocumentView documentId={documentId} />);

    await waitFor(() => {
      expect(documentEntries().length).toBeGreaterThan(0);
    });
    const firstCollaborationDocument = documentEntries()[0]?.collaborationDocument;
    expect(firstCollaborationDocument).toBeTruthy();

    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-reload"));
    });

    await waitFor(() => {
      const entries = documentEntries();
      expect(entries.length).toBeGreaterThanOrEqual(2);
      expect(entries[entries.length - 1]?.collaborationDocument).toBe(firstCollaborationDocument);
      expect(entries[entries.length - 1]?.collaborationResetToken).toBe(1);
    });
  });

  it("drops stale CRDT-key aliases when their inactive collaboration document is evicted (MT-246)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000009999";
    const defaultCrdtDocumentId = "KCRDT-00000000000000000000000000009999";
    const crdtDocumentId = "KCRDT-000000000000000000000000009999";
    let targetBackendCrdtId: string | null = null;
    vi.mocked(api.loadRichDocument).mockImplementation(async (requestedId: string) => {
      if (requestedId === documentId) {
        return {
          document: docFixtureFor(documentId, {
            crdt_document_id: targetBackendCrdtId,
          }),
          tree: {
            schema_version: "rich_document_v1",
            schema_matches: true,
            block_ids: [],
            blocks: [],
          },
          code_nodes: [],
        };
      }
      return {
        document: docFixtureFor(requestedId, {
          crdt_document_id: requestedId.replace("KRD-", "KCRDT-"),
        }),
        tree: {
          schema_version: "rich_document_v1",
          schema_matches: true,
          block_ids: [],
          blocks: [],
        },
        code_nodes: [],
      };
    });
    const loadCallsBefore = vi.mocked(api.loadRichDocument).mock.calls.length;
    richTextEditorPropLog.length = 0;
    const lastGuidFor = (id: string) =>
      (
        richTextEditorPropLog
          .filter((entry) => entry.debugId === id)
          .slice(-1)[0]?.collaborationDocument as { guid?: string } | undefined
      )?.guid;

    const firstRender = render(<RichDocumentView documentId={documentId} />);
    await waitFor(() => expect(lastGuidFor(documentId)).toBe(defaultCrdtDocumentId));
    const provisionalDocument = richTextEditorPropLog.filter((entry) => entry.debugId === documentId).slice(-1)[0]
      ?.collaborationDocument;
    targetBackendCrdtId = crdtDocumentId;
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-reload"));
    });
    await waitFor(() => expect(api.loadRichDocument).toHaveBeenCalledTimes(loadCallsBefore + 2));
    await waitFor(() => {
      const latestEntry = richTextEditorPropLog.filter((entry) => entry.debugId === documentId).slice(-1)[0];
      expect(latestEntry?.collaborationDocument).toBe(provisionalDocument);
      expect(latestEntry?.collaborationResetToken).toBe(1);
    });
    firstRender.unmount();

    for (let index = 0; index < 80; index += 1) {
      const fillId = `KRD-fill-${index.toString().padStart(4, "0")}`;
      const rendered = render(<RichDocumentView documentId={fillId} />);
      await waitFor(() => expect(lastGuidFor(fillId)).toBe(fillId.replace("KRD-", "KCRDT-")));
      rendered.unmount();
    }

    targetBackendCrdtId = null;
    render(<RichDocumentView documentId={documentId} />);
    await waitFor(() => {
      const latestEntry = richTextEditorPropLog.filter((entry) => entry.debugId === documentId).slice(-1)[0];
      expect(latestEntry?.collaborationDocument).not.toBe(provisionalDocument);
      expect(lastGuidFor(documentId)).toBe(defaultCrdtDocumentId);
    });
  });

  it("saves through the authority API with the optimistic-concurrency version (MT-149)", async () => {
    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });

    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "edited" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });

    const api = await import("../lib/api");
    const saveRichDocument = vi.mocked(api.saveRichDocument);
    await waitFor(() => {
      expect(saveRichDocument).toHaveBeenCalledTimes(1);
      const [docId, expectedVersion] = saveRichDocument.mock.calls[0];
      expect(docId).toBe("KRD-00000000000000000000000000000001");
      expect(expectedVersion).toBe(1);
    });
  });

  it("shares save-version authority across duplicate same-document panes (MT-246)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000007700";
    const loadRichDocument = vi.mocked(api.loadRichDocument);
    const saveRichDocument = vi.mocked(api.saveRichDocument);
    const saveCallsBefore = saveRichDocument.mock.calls.length;
    let serverVersion = 1;
    try {
      loadRichDocument.mockResolvedValue({
        document: docFixtureFor(documentId, { doc_version: serverVersion }),
        tree: {
          schema_version: "rich_document_v1",
          schema_matches: true,
          block_ids: [],
          blocks: [],
        },
        code_nodes: [],
      });
      saveRichDocument.mockImplementation(
        async (docId, expectedVersion, contentJson, _ctx, metadata) => {
          if (expectedVersion !== serverVersion) {
            throw new Error(`HSK-409 version conflict: expected_version ${expectedVersion} got ${serverVersion}`);
          }
          serverVersion += 1;
          return {
            document: docFixtureFor(docId, {
              doc_version: serverVersion,
              content_json: contentJson as typeof DOC_V1.content_json,
              crdt_document_id: metadata?.crdt_document_id ?? null,
            }),
            save_receipt_event_id: `EVT-${serverVersion}`,
            backlinks_persisted: 0,
            backlinks_skipped_reason: null,
          };
        },
      );

      render(
        <>
          <RichDocumentView documentId={documentId} />
          <RichDocumentView documentId={documentId} />
        </>,
      );

      await waitFor(() => expect(screen.getAllByTestId("rich-document-view")).toHaveLength(2));
      const editors = screen.getAllByTestId("tiptap-editor");
      await act(async () => {
        fireEvent.change(editors[0], { target: { value: "pane A edit" } });
        fireEvent.change(editors[1], { target: { value: "pane B observed same shared edit" } });
      });

      const saveButtons = screen.getAllByTestId("rich-document-save");
      await act(async () => {
        fireEvent.click(saveButtons[0]);
      });
      await waitFor(() => {
        expect(screen.getAllByTestId("rich-document-view").every((view) => view.getAttribute("data-dirty") === "false"))
          .toBe(true);
      });

      await act(async () => {
        fireEvent.change(editors[1], { target: { value: "pane B follow-up edit" } });
      });
      await act(async () => {
        fireEvent.click(screen.getAllByTestId("rich-document-save")[1]);
      });

      await waitFor(() => {
        const savesFromThisTest = saveRichDocument.mock.calls.slice(saveCallsBefore);
        expect(savesFromThisTest).toHaveLength(2);
        expect(savesFromThisTest[0][1]).toBe(1);
        expect(savesFromThisTest[1][1]).toBe(2);
      });
      expect(screen.queryByTestId("rich-document-save-error")).toBeNull();
    } finally {
      restoreRichDocumentApiDefaults(api);
    }
  });

  it("ignores stale duplicate-pane reloads older than shared save authority (MT-246)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000007701";
    const loadRichDocument = vi.mocked(api.loadRichDocument);
    const saveRichDocument = vi.mocked(api.saveRichDocument);
    const loadCallsBefore = loadRichDocument.mock.calls.length;
    try {
      loadRichDocument.mockImplementation(async () => ({
        document: docFixtureFor(documentId, { doc_version: 1 }),
        tree: {
          schema_version: "rich_document_v1",
          schema_matches: true,
          block_ids: [],
          blocks: [],
        },
        code_nodes: [],
      }));
      saveRichDocument.mockImplementation(
        async (docId, expectedVersion, contentJson, _ctx, metadata) => {
          if (expectedVersion !== 1) {
            throw new Error(`HSK-409 version conflict: expected_version ${expectedVersion} got 1`);
          }
          return {
            document: docFixtureFor(docId, {
              doc_version: 2,
              content_json: contentJson as typeof DOC_V1.content_json,
              crdt_document_id: metadata?.crdt_document_id ?? null,
            }),
            save_receipt_event_id: "EVT-2",
            backlinks_persisted: 0,
            backlinks_skipped_reason: null,
          };
        },
      );

      render(
        <>
          <RichDocumentView documentId={documentId} />
          <RichDocumentView documentId={documentId} />
        </>,
      );

      await waitFor(() => expect(screen.getAllByTestId("rich-document-view")).toHaveLength(2));
      const editors = screen.getAllByTestId("tiptap-editor");
      await act(async () => {
        fireEvent.change(editors[0], { target: { value: "pane A edit" } });
        fireEvent.change(editors[1], { target: { value: "pane B shared edit" } });
      });
      await act(async () => {
        fireEvent.click(screen.getAllByTestId("rich-document-save")[0]);
      });
      await waitFor(() => {
        expect(screen.getAllByTestId("rich-document-view").every((view) => view.getAttribute("data-doc-version") === "2"))
          .toBe(true);
      });
      await act(async () => {
        fireEvent.change(editors[1], { target: { value: "pane B unsaved follow-up before stale reload" } });
      });
      await waitFor(() => {
        expect(screen.getAllByTestId("rich-document-view")[1].getAttribute("data-dirty")).toBe("true");
      });

      await act(async () => {
        fireEvent.click(screen.getAllByTestId("rich-document-reload")[1]);
      });
      await waitFor(() => expect(loadRichDocument.mock.calls.length).toBeGreaterThan(loadCallsBefore + 2));
      expect(screen.getAllByTestId("rich-document-view").every((view) => view.getAttribute("data-doc-version") === "2"))
        .toBe(true);
      expect(screen.getAllByTestId("rich-document-view")[1].getAttribute("data-dirty")).toBe("true");
      expect((screen.getAllByTestId("rich-document-save")[1] as HTMLButtonElement).disabled).toBe(false);
      expect(screen.queryByTestId("rich-document-save-error")).toBeNull();
    } finally {
      restoreRichDocumentApiDefaults(api);
    }
  });

  it("keeps a duplicate pane dirty when a stale save response would downgrade shared authority (MT-246)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000007702";
    const loadRichDocument = vi.mocked(api.loadRichDocument);
    const saveRichDocument = vi.mocked(api.saveRichDocument);
    const saveCallsBefore = saveRichDocument.mock.calls.length;
    let saveCall = 0;
    try {
      loadRichDocument.mockResolvedValue({
        document: docFixtureFor(documentId, { doc_version: 1 }),
        tree: {
          schema_version: "rich_document_v1",
          schema_matches: true,
          block_ids: [],
          blocks: [],
        },
        code_nodes: [],
      });
      saveRichDocument.mockImplementation(
        async (docId, expectedVersion, contentJson, _ctx, metadata) => {
          saveCall += 1;
          if (saveCall === 1) {
            if (expectedVersion !== 1) {
              throw new Error(`HSK-409 version conflict: expected_version ${expectedVersion} got 1`);
            }
            return {
              document: docFixtureFor(docId, {
                doc_version: 2,
                content_json: contentJson as typeof DOC_V1.content_json,
                crdt_document_id: metadata?.crdt_document_id ?? null,
              }),
              save_receipt_event_id: "EVT-2",
              backlinks_persisted: 0,
              backlinks_skipped_reason: null,
            };
          }
          if (expectedVersion !== 2) {
            throw new Error(`HSK-409 version conflict: expected_version ${expectedVersion} got 2`);
          }
          return {
            document: docFixtureFor(docId, {
              doc_version: 1,
              content_json: contentJson as typeof DOC_V1.content_json,
              crdt_document_id: null,
            }),
            save_receipt_event_id: "EVT-stale",
            backlinks_persisted: 0,
            backlinks_skipped_reason: null,
          };
        },
      );

      render(
        <>
          <RichDocumentView documentId={documentId} />
          <RichDocumentView documentId={documentId} />
        </>,
      );

      await waitFor(() => expect(screen.getAllByTestId("rich-document-view")).toHaveLength(2));
      const editors = screen.getAllByTestId("tiptap-editor");
      await act(async () => {
        fireEvent.change(editors[0], { target: { value: "pane A edit" } });
        fireEvent.change(editors[1], { target: { value: "pane B observed same shared edit" } });
      });
      await act(async () => {
        fireEvent.click(screen.getAllByTestId("rich-document-save")[0]);
      });
      await waitFor(() => {
        expect(screen.getAllByTestId("rich-document-view").every((view) => view.getAttribute("data-doc-version") === "2"))
          .toBe(true);
        expect(screen.getAllByTestId("rich-document-view").every((view) => view.getAttribute("data-dirty") === "false"))
          .toBe(true);
      });

      await act(async () => {
        fireEvent.change(editors[1], { target: { value: "pane B follow-up edit" } });
      });
      await waitFor(() => {
        expect(screen.getAllByTestId("rich-document-view")[1].getAttribute("data-dirty")).toBe("true");
      });
      await act(async () => {
        fireEvent.click(screen.getAllByTestId("rich-document-save")[1]);
      });

      await waitFor(() => {
        const savesFromThisTest = saveRichDocument.mock.calls.slice(saveCallsBefore);
        expect(savesFromThisTest).toHaveLength(2);
        expect(savesFromThisTest[0][1]).toBe(1);
        expect(savesFromThisTest[1][1]).toBe(2);
      });
      expect(screen.getAllByTestId("rich-document-view").every((view) => view.getAttribute("data-doc-version") === "2"))
        .toBe(true);
      expect(screen.getAllByTestId("rich-document-view")[1].getAttribute("data-dirty")).toBe("true");
      expect((screen.getAllByTestId("rich-document-save")[1] as HTMLButtonElement).disabled).toBe(false);
      expect(screen.getByTestId("rich-document-save-error").textContent).toContain("would downgrade");
      expect(screen.getByTestId("rte-backend-error").getAttribute("data-error-kind")).toBe("conflict");
    } finally {
      restoreRichDocumentApiDefaults(api);
    }
  });

  it("passes active CRDT promotion metadata when saving a rich document (MT-246)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000007777";
    const crdtDocumentId = "KCRDT-000000000000000000000000007777";
    vi.mocked(api.loadRichDocument).mockResolvedValueOnce({
      document: docFixtureFor(documentId, {
        doc_version: 7,
        crdt_document_id: crdtDocumentId,
        crdt_snapshot_id: "stale-snapshot-from-prior-version",
        promotion_receipt_event_id: "EVT-stale-prior-version",
      }),
      tree: {
        schema_version: "rich_document_v1",
        schema_matches: true,
        block_ids: [],
        blocks: [],
      },
      code_nodes: [],
    });
    const saveRichDocument = vi.mocked(api.saveRichDocument);
    const callsBefore = saveRichDocument.mock.calls.length;

    await act(async () => {
      render(<RichDocumentView documentId={documentId} />);
    });
    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "edited" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });

    await waitFor(() => {
      expect(saveRichDocument.mock.calls.length).toBe(callsBefore + 1);
      const [docId, expectedVersion, , , metadata] = saveRichDocument.mock.calls[callsBefore];
      expect(docId).toBe(documentId);
      expect(expectedVersion).toBe(7);
      expect(metadata).toEqual({
        crdt_document_id: crdtDocumentId,
      });
    });
  });

  it("derives and saves a CRDT document id for a backend-null rich document (MT-246)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000006666";
    const expectedCrdtDocumentId = "KCRDT-00000000000000000000000000006666";
    vi.mocked(api.loadRichDocument).mockResolvedValueOnce({
      document: docFixtureFor(documentId, {
        doc_version: 3,
        crdt_document_id: null,
      }),
      tree: {
        schema_version: "rich_document_v1",
        schema_matches: true,
        block_ids: [],
        blocks: [],
      },
      code_nodes: [],
    });
    const saveRichDocument = vi.mocked(api.saveRichDocument);
    const callsBefore = saveRichDocument.mock.calls.length;

    await act(async () => {
      render(<RichDocumentView documentId={documentId} />);
    });
    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "edited" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });

    await waitFor(() => {
      expect(saveRichDocument.mock.calls.length).toBe(callsBefore + 1);
      const [, , , , metadata] = saveRichDocument.mock.calls[callsBefore];
      expect(metadata).toEqual({
        crdt_document_id: expectedCrdtDocumentId,
      });
    });
  });

  it("surfaces a save conflict as a typed inline backend error (MT-174)", async () => {
    const api = await import("../lib/api");
    vi.mocked(api.saveRichDocument).mockRejectedValueOnce(
      new Error("HSK-409 version conflict: expected_version 1 got 2"),
    );

    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });
    await act(async () => {
      fireEvent.change(await screen.findByTestId("tiptap-editor"), { target: { value: "edited" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });

    // The classified backend error (conflict) is passed to the editor and
    // rendered inline (not a blank screen).
    await waitFor(() => {
      const err = screen.getByTestId("rte-backend-error");
      expect(err.getAttribute("data-error-kind")).toBe("conflict");
    });
  });

  it("fails CLOSED on a newer-schema document: read-only editor, save blocked, typed banner (iteration-3 H2)", async () => {
    const api = await import("../lib/api");
    vi.mocked(api.loadRichDocument).mockResolvedValueOnce({
      document: { ...DOC_V1, schema_version: "rich_document_v999" },
      tree: {
        schema_version: "rich_document_v999",
        schema_matches: false,
        block_ids: [],
        blocks: [],
      },
      code_nodes: [],
    } as never);

    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });

    // The view is marked blocked and the editor mounts READ-ONLY.
    const view = await screen.findByTestId("rich-document-view");
    expect(view.getAttribute("data-schema-blocked")).toBe("true");
    const editor = screen.getByTestId("tiptap-editor");
    expect(editor.getAttribute("data-readonly")).toBe("true");

    // The typed schema banner reaches the editor surface.
    const err = screen.getByTestId("rte-backend-error");
    expect(err.getAttribute("data-error-kind")).toBe("schema");
    expect(err.textContent).toContain("rich_document_v999");

    // DESTRUCTIVE-SAVE NEGATIVE TEST: even if edits somehow land (ProseMirror
    // would have silently dropped unknown nodes), the save path never reaches
    // the backend — the stripped doc can never overwrite the authority record.
    const saveMock = vi.mocked(api.saveRichDocument);
    const callsBefore = saveMock.mock.calls.length;
    await act(async () => {
      fireEvent.change(editor, { target: { value: "forced edit" } });
    });
    const save = screen.getByTestId("rich-document-save") as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    await act(async () => {
      fireEvent.click(save);
    });
    expect(saveMock.mock.calls.length).toBe(callsBefore);
  });

  it("surfaces code-block hash violations on load as a typed integrity banner (iteration-3 M9)", async () => {
    const api = await import("../lib/api");
    vi.mocked(api.loadRichDocument).mockResolvedValueOnce({
      document: {
        ...DOC_V1,
        content_json: {
          type: "doc",
          content: [
            {
              type: "monacoCodeBlock",
              // Stored hash does not match {language, code} — out-of-band edit.
              attrs: { language: "json", code: '{"a":2}', contentHash: "deadbeef" },
            },
          ],
        },
      },
      tree: {
        schema_version: "rich_document_v1",
        schema_matches: true,
        block_ids: [],
        blocks: [],
      },
      code_nodes: [],
    } as never);

    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });

    // Typed integrity banner; the document still loads and stays editable
    // (backend content_sha256 remains the durable authority; a re-save
    // re-mints the editor-layer hashes).
    const err = await screen.findByTestId("rte-backend-error");
    expect(err.getAttribute("data-error-kind")).toBe("integrity");
    expect(err.textContent).toContain("1 of 1");
    expect(screen.getByTestId("tiptap-editor").getAttribute("data-readonly")).toBe("false");
    expect(screen.getByTestId("rich-document-view").getAttribute("data-schema-blocked")).toBe("false");
  });

  it("keeps the document dirty when keystrokes land during an in-flight save (iteration-3 H5)", async () => {
    const api = await import("../lib/api");
    let resolveSave: (value: unknown) => void = () => {};
    vi.mocked(api.saveRichDocument).mockImplementationOnce(
      () => new Promise((resolve) => (resolveSave = resolve)) as never,
    );

    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });
    const editor = await screen.findByTestId("tiptap-editor");

    // Edit -> dirty -> start the save (request stays pending).
    await act(async () => {
      fireEvent.change(editor, { target: { value: "first edit" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });
    expect(screen.getByTestId("rich-document-save").textContent).toBe("Saving...");

    // A keystroke lands WHILE the save is in flight.
    await act(async () => {
      fireEvent.change(editor, { target: { value: "second edit during save" } });
    });

    // The save resolves — the dirty flag must SURVIVE (the in-flight edit is
    // not yet persisted; clearing it would lose the save affordance).
    await act(async () => {
      resolveSave({
        document: { ...DOC_V1, doc_version: 2 },
        save_receipt_event_id: "EVT-2",
        backlinks_persisted: 0,
        backlinks_skipped_reason: null,
      });
    });
    await waitFor(() => {
      expect(screen.getByTestId("rich-document-view").getAttribute("data-dirty")).toBe("true");
      expect((screen.getByTestId("rich-document-save") as HTMLButtonElement).disabled).toBe(false);
    });
  });

  it("clears the dirty flag after a save with no in-flight edits", async () => {
    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });
    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "edit" } });
    });
    expect(screen.getByTestId("rich-document-view").getAttribute("data-dirty")).toBe("true");
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("rich-document-view").getAttribute("data-dirty")).toBe("false");
      expect((screen.getByTestId("rich-document-save") as HTMLButtonElement).disabled).toBe(true);
    });
  });

  it("persists dirty editor changes to the backend draft slot after debounce (MT-255)", async () => {
    const api = await import("../lib/api");
    restoreRichDocumentApiDefaults(api);
    await act(async () => {
      render(<RichDocumentView documentId={DOC_V1.rich_document_id} />);
    });
    const editor = await screen.findByTestId("tiptap-editor");
    const draftCallsBefore = vi.mocked(api.upsertRichDocumentDraft).mock.calls.length;

    vi.useFakeTimers();
    try {
      await act(async () => {
        fireEvent.change(editor, { target: { value: "draft me" } });
      });
      expect(vi.mocked(api.upsertRichDocumentDraft).mock.calls.length).toBe(draftCallsBefore);

      await act(async () => {
        vi.advanceTimersByTime(650);
      });
      await act(async () => {});
    } finally {
      vi.useRealTimers();
    }

    const draftCalls = vi.mocked(api.upsertRichDocumentDraft).mock.calls.slice(draftCallsBefore);
    expect(draftCalls).toHaveLength(1);
    expect(draftCalls[0][0]).toBe(DOC_V1.rich_document_id);
    expect(draftCalls[0][1]).toBe(1);
    expect(draftCalls[0][2]).toBe(DOC_V1.content_sha256);
    expect(JSON.stringify(draftCalls[0][3])).toContain("local-edit-marker");
  });

  it("offers backend draft recovery, restores it byte-exact, and removes the offer after save (MT-255)", async () => {
    const api = await import("../lib/api");
    restoreRichDocumentApiDefaults(api);
    const draftContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "backend crash sentinel" }] }],
    };
    vi.mocked(api.loadRichDocumentDraft).mockResolvedValueOnce({
      rich_document_id: DOC_V1.rich_document_id,
      current_doc_version: 1,
      current_content_sha256: DOC_V1.content_sha256,
      draft: {
        rich_document_id: DOC_V1.rich_document_id,
        workspace_id: DOC_V1.workspace_id,
        base_doc_version: 1,
        base_content_sha256: DOC_V1.content_sha256,
        draft_content_json: draftContent,
        draft_content_sha256: "1".repeat(64),
        actor_kind: "operator",
        actor_id: "operator",
        kernel_task_run_id: "KTR-EDITOR-UI",
        session_run_id: "SR-EDITOR-UI",
        created_at: "2026-06-12T00:00:00Z",
        updated_at: "2026-06-12T00:01:00Z",
      },
    });

    await act(async () => {
      render(<RichDocumentView documentId={DOC_V1.rich_document_id} />);
    });

    const panel = await screen.findByTestId("rich-document-local-snapshot");
    expect(panel.getAttribute("data-snapshot-reason")).toBe("draft_recovery");
    expect(panel.textContent).toContain("Unsaved draft recovered");
    expect(screen.getByTestId("rich-document-merge-panel")).toBeTruthy();

    await act(async () => {
      fireEvent.click(screen.getByTestId("snapshot-restore"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("tiptap-editor").getAttribute("data-content")).toContain(
        "backend crash sentinel",
      );
      expect(screen.getByTestId("rich-document-view").getAttribute("data-dirty")).toBe("true");
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });
    await waitFor(() => {
      expect(screen.queryByTestId("rich-document-local-snapshot")).toBeNull();
    });
    expect(api.saveRichDocument).toHaveBeenCalled();
  });

  it("discards a backend recovery draft without saving over the current head (MT-255)", async () => {
    const api = await import("../lib/api");
    restoreRichDocumentApiDefaults(api);
    const saveCallsBefore = vi.mocked(api.saveRichDocument).mock.calls.length;
    const draftContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "discard-only sentinel" }] }],
    };
    vi.mocked(api.loadRichDocumentDraft).mockResolvedValueOnce({
      rich_document_id: DOC_V1.rich_document_id,
      current_doc_version: 1,
      current_content_sha256: DOC_V1.content_sha256,
      draft: {
        rich_document_id: DOC_V1.rich_document_id,
        workspace_id: DOC_V1.workspace_id,
        base_doc_version: 1,
        base_content_sha256: DOC_V1.content_sha256,
        draft_content_json: draftContent,
        draft_content_sha256: "2".repeat(64),
        actor_kind: "operator",
        actor_id: "operator",
        kernel_task_run_id: "KTR-EDITOR-UI",
        session_run_id: "SR-EDITOR-UI",
        created_at: "2026-06-12T00:00:00Z",
        updated_at: "2026-06-12T00:02:00Z",
      },
    });

    await act(async () => {
      render(<RichDocumentView documentId={DOC_V1.rich_document_id} />);
    });

    await screen.findByTestId("rich-document-local-snapshot");
    await act(async () => {
      fireEvent.click(screen.getByTestId("snapshot-discard"));
    });
    await waitFor(() => {
      expect(api.clearRichDocumentDraft).toHaveBeenCalledWith(DOC_V1.rich_document_id);
      expect(screen.queryByTestId("rich-document-local-snapshot")).toBeNull();
    });
    expect(vi.mocked(api.saveRichDocument).mock.calls.length).toBe(saveCallsBefore);
    const editorContent = screen.getByTestId("tiptap-editor").getAttribute("data-content") ?? "";
    expect(editorContent).toContain(DOC_V1.rich_document_id);
    expect(editorContent).not.toContain("discard-only sentinel");
  });

  it("preserves local edits as a snapshot on save conflict; restore re-applies them; reload never discards (iteration-3 H5)", async () => {
    const api = await import("../lib/api");
    richTextEditorPropLog.length = 0;
    vi.mocked(api.saveRichDocument).mockRejectedValueOnce(
      new Error("HSK-409 version conflict: expected_version 1 got 2"),
    );
    const lastResetToken = () =>
      richTextEditorPropLog.filter((entry) => entry.debugId === DOC_V1.rich_document_id).slice(-1)[0]
        ?.collaborationResetToken;

    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });
    const editor = await screen.findByTestId("tiptap-editor");
    expect(lastResetToken()).toBeUndefined();
    await act(async () => {
      fireEvent.change(editor, { target: { value: "local work" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });

    // The snapshot panel appears with download/restore/discard.
    const panel = await screen.findByTestId("rich-document-local-snapshot");
    expect(panel.getAttribute("data-snapshot-reason")).toBe("conflict");
    expect(screen.getByTestId("snapshot-download")).toBeTruthy();
    expect(screen.getByTestId("snapshot-restore")).toBeTruthy();
    expect(screen.getByTestId("snapshot-discard")).toBeTruthy();

    // Reload (the conflict remediation) must NOT discard the snapshot.
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-reload"));
    });
    expect(screen.getByTestId("rich-document-local-snapshot")).toBeTruthy();
    await waitFor(() => {
      expect(lastResetToken()).toBe(1);
    });

    // Restore puts the LOCAL version (the onChange marker doc) back into the
    // editor and marks the document dirty again.
    await act(async () => {
      fireEvent.click(screen.getByTestId("snapshot-restore"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("tiptap-editor").getAttribute("data-content")).toContain(
        "local-edit-marker",
      );
      expect(screen.getByTestId("rich-document-view").getAttribute("data-dirty")).toBe("true");
      expect(lastResetToken()).toBe(2);
    });

    // Discard is the only way the snapshot disappears — and it is explicit.
    await act(async () => {
      fireEvent.click(screen.getByTestId("snapshot-discard"));
    });
    expect(screen.queryByTestId("rich-document-local-snapshot")).toBeNull();
  });

  it("offers an explicit merge choice after a conflict reload (MT-247)", async () => {
    const api = await import("../lib/api");
    const documentId = "KRD-00000000000000000000000000000001";
    const baseDoc = docFixtureFor(documentId, {
      doc_version: 1,
      content_json: {
        type: "doc",
        content: [{ type: "paragraph", content: [{ type: "text", text: "Base heading" }] }],
      },
    });
    const remoteDoc = docFixtureFor(documentId, {
      doc_version: 2,
      content_json: {
        type: "doc",
        content: [{ type: "paragraph", content: [{ type: "text", text: "Remote heading" }] }],
      },
    });
    const tree = {
      schema_version: "rich_document_v1",
      schema_matches: true,
      block_ids: [],
      blocks: [],
    };
    vi.mocked(api.loadRichDocument)
      .mockResolvedValueOnce({ document: baseDoc, tree, code_nodes: [] })
      .mockResolvedValueOnce({ document: remoteDoc, tree, code_nodes: [] });
    vi.mocked(api.saveRichDocument).mockRejectedValueOnce(
      new Error("HSK-409 version conflict: expected_version 1 got 2"),
    );

    await act(async () => {
      render(<RichDocumentView documentId={documentId} />);
    });
    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "local work" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });
    await screen.findByTestId("rich-document-local-snapshot");

    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-reload"));
    });

    const mergePanel = await screen.findByTestId("rich-document-merge-panel");
    expect(mergePanel.getAttribute("data-conflicts")).toBe("1");
    expect((screen.getByTestId("snapshot-merge-apply") as HTMLButtonElement).disabled).toBe(true);

    await act(async () => {
      fireEvent.click(screen.getByTestId("snapshot-merge-block-0-remote"));
    });
    await waitFor(() => {
      expect((screen.getByTestId("snapshot-merge-apply") as HTMLButtonElement).disabled).toBe(false);
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("snapshot-merge-apply"));
    });

    await waitFor(() => {
      expect(screen.getByTestId("tiptap-editor").getAttribute("data-content")).toContain("Remote heading");
      expect(screen.queryByTestId("rich-document-local-snapshot")).toBeNull();
    });
  });

  it("blocks unload while dirty via beforeunload and releases it after save (iteration-3 H5)", async () => {
    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });
    const editor = await screen.findByTestId("tiptap-editor");

    // Clean: unload is not blocked.
    let event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);

    // Dirty: unload is blocked.
    await act(async () => {
      fireEvent.change(editor, { target: { value: "edit" } });
    });
    event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);

    // Saved clean: released again.
    await act(async () => {
      fireEvent.click(screen.getByTestId("rich-document-save"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("rich-document-view").getAttribute("data-dirty")).toBe("false");
    });
    event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
  });

  it("snapshots un-saved edits when switching documents; restore only offered on the owning doc (iteration-3 H5)", async () => {
    const { rerender } = render(
      <RichDocumentView documentId="KRD-00000000000000000000000000000001" />,
    );
    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "un-saved work" } });
    });

    // Switch to another document while dirty.
    await act(async () => {
      rerender(<RichDocumentView documentId="KRD-00000000000000000000000000000002" />);
    });

    const panel = await screen.findByTestId("rich-document-local-snapshot");
    expect(panel.getAttribute("data-snapshot-reason")).toBe("doc_switch");
    expect(panel.getAttribute("data-snapshot-for")).toBe(
      "KRD-00000000000000000000000000000001",
    );
    // The edits belong to doc ...001 — no restore onto doc ...002; download +
    // discard remain available.
    expect(screen.queryByTestId("snapshot-restore")).toBeNull();
    expect(screen.getByTestId("snapshot-download")).toBeTruthy();
    expect(screen.getByTestId("snapshot-discard")).toBeTruthy();
  });

  it("does not report the previous rich document dirty state after switching documents", async () => {
    const dirtyReports: Array<{ documentId: string; dirty: boolean }> = [];

    const { rerender } = render(
      <RichDocumentView
        documentId="KRD-00000000000000000000000000000001"
        onDirtyChange={(dirty) =>
          dirtyReports.push({ documentId: "KRD-00000000000000000000000000000001", dirty })
        }
      />,
    );
    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "un-saved work" } });
    });
    await waitFor(() => {
      expect(dirtyReports).toContainEqual({
        documentId: "KRD-00000000000000000000000000000001",
        dirty: true,
      });
    });

    dirtyReports.length = 0;
    await act(async () => {
      rerender(
        <RichDocumentView
          documentId="KRD-00000000000000000000000000000002"
          onDirtyChange={(dirty) =>
            dirtyReports.push({ documentId: "KRD-00000000000000000000000000000002", dirty })
          }
        />,
      );
    });

    await waitFor(() => {
      expect(dirtyReports).toContainEqual({
        documentId: "KRD-00000000000000000000000000000002",
        dirty: false,
      });
    });
    expect(dirtyReports).not.toContainEqual({
      documentId: "KRD-00000000000000000000000000000002",
      dirty: true,
    });
  });
});
