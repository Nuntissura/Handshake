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

vi.mock("./TiptapEditor", () => ({
  TiptapEditor: ({
    initialContent,
    onChange,
  }: {
    initialContent: JSONContent | null;
    onChange: (doc: JSONContent | null) => void;
  }) => (
    <textarea
      data-testid="tiptap-editor"
      defaultValue={JSON.stringify(initialContent)}
      onChange={() => onChange({ type: "doc", content: [{ type: "paragraph" }] })}
    />
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
});
