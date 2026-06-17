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

vi.mock("../state/debugEvents", () => ({ logEvent: vi.fn() }));

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
    onSaveRequested,
    documentStatus,
  }: {
    initialContent: JSONContent | null;
    onChange: (doc: JSONContent | null) => void;
    backendError?: { kind: string; message: string } | null;
    readOnly?: boolean;
    onSaveRequested?: () => void;
    documentStatus?: {
      dirty: boolean;
      saving: boolean;
      blocked: boolean;
      backendErrorKind?: string | null;
      lastSavedAt?: string | null;
    };
  }) => (
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
      <div
        data-testid="mock-editor-status"
        data-dirty={documentStatus?.dirty ? "true" : "false"}
        data-saving={documentStatus?.saving ? "true" : "false"}
        data-blocked={documentStatus?.blocked ? "true" : "false"}
        data-error-kind={documentStatus?.backendErrorKind ?? ""}
        data-last-saved-at={documentStatus?.lastSavedAt ?? ""}
      />
      <button type="button" data-testid="mock-editor-palette-save" onClick={onSaveRequested}>
        Palette save
      </button>
      {backendError ? (
        <div data-testid="rte-backend-error" data-error-kind={backendError.kind}>
          {backendError.message}
        </div>
      ) : null}
    </div>
  ),
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
  crdt_document_id: null,
  crdt_snapshot_id: null,
  promotion_receipt_event_id: null,
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
    loadRichDocumentHistory,
    listRichDocumentEmbeds,
    listRichDocumentBrokenEmbeds,
    listRichDocumentBacklinks,
  };
});

describe("RichDocumentView (MT-145..MT-160)", () => {
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

  it("routes editor palette save through the same guarded authority save path (MT-245/EXT-SAVE-001)", async () => {
    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });

    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "edited" } });
    });
    expect(screen.getByTestId("mock-editor-status")).toHaveAttribute("data-dirty", "true");
    const api = await import("../lib/api");
    const saveRichDocument = vi.mocked(api.saveRichDocument);
    const callsBefore = saveRichDocument.mock.calls.length;
    await act(async () => {
      fireEvent.click(screen.getByTestId("mock-editor-palette-save"));
    });

    await waitFor(() => {
      expect(saveRichDocument.mock.calls.length).toBe(callsBefore + 1);
      expect(saveRichDocument.mock.calls[callsBefore][0]).toBe("KRD-00000000000000000000000000000001");
      expect(saveRichDocument.mock.calls[callsBefore][1]).toBe(1);
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
      expect(screen.getByTestId("mock-editor-status")).toHaveAttribute("data-error-kind", "conflict");
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

  it("preserves local edits as a snapshot on save conflict; restore re-applies them; reload never discards (iteration-3 H5)", async () => {
    const api = await import("../lib/api");
    vi.mocked(api.saveRichDocument).mockRejectedValueOnce(
      new Error("HSK-409 version conflict: expected_version 1 got 2"),
    );

    await act(async () => {
      render(<RichDocumentView documentId="KRD-00000000000000000000000000000001" />);
    });
    const editor = await screen.findByTestId("tiptap-editor");
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
    });

    // Discard is the only way the snapshot disappears — and it is explicit.
    await act(async () => {
      fireEvent.click(screen.getByTestId("snapshot-discard"));
    });
    expect(screen.queryByTestId("rich-document-local-snapshot")).toBeNull();
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
});
