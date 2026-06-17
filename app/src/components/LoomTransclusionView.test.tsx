// WP-KERNEL-009 / MT-258 — LoomTransclusionView NodeView tests.
//
// Mounts the REAL integrated editor (RichTextEditor → loomTransclusion
// NodeView) in jsdom and proves:
//   - read-through renders the SOURCE document content (resolved state),
//   - "Edit source" + Save routes the write to the SOURCE document id via
//     saveRichDocument (not the host doc) and shows the receipt,
//   - an unresolved block renders the typed unresolved state (never blank),
//   - a resolve failure renders the typed error state (role=alert),
//   - the HOST document onChange JSON keeps ONLY the atom node (NO-COPY).

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { act } from "react";
import { beforeEach, describe, it, expect, vi } from "vitest";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "./RichTextEditor";
import {
  getLoomBlockTransclusion,
  saveRichDocument,
  type LoomBlockTransclusion,
} from "../lib/api";

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    getLoomBlockTransclusion: vi.fn(),
    saveRichDocument: vi.fn(),
  };
});

const WS = "ws-transclusion-test";
const BLOCK = "block-source";
const SRC_DOC = "KRD-source-1";

function hostDoc(): JSONContent {
  return {
    type: "doc",
    content: [
      { type: "paragraph", content: [{ type: "text", text: "Host preamble." }] },
      { type: "loomTransclusion", attrs: { refValue: BLOCK } },
    ],
  };
}

function sourceJson(text: string): JSONContent {
  return { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text }] }] };
}

function resolved(text: string, version: number): LoomBlockTransclusion {
  return {
    block_id: BLOCK,
    workspace_id: WS,
    source_document_id: SRC_DOC,
    source_doc_version: version,
    content_json: sourceJson(text),
    resolved: true,
  };
}

describe("LoomTransclusionView (MT-258)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the SOURCE content read-through", async () => {
    vi.mocked(getLoomBlockTransclusion).mockResolvedValue(resolved("ORIGINAL source body", 1));
    let latest: JSONContent | null = null;
    await act(async () => {
      render(
        <RichTextEditor
          initialContent={hostDoc()}
          onChange={(next) => {
            latest = next;
          }}
          embedContext={{ workspaceId: WS }}
          documentTitle="host"
        />,
      );
    });
    await screen.findByTestId("loom-transclusion-content");
    expect(screen.getByTestId("loom-transclusion-content").textContent).toContain(
      "ORIGINAL source body",
    );
    expect(getLoomBlockTransclusion).toHaveBeenCalledWith(WS, BLOCK);
    // NO-COPY: the host onChange JSON has only the atom node (no source body).
    const hostText = JSON.stringify(latest ?? hostDoc());
    expect(hostText).toContain("loomTransclusion");
    expect(hostText).not.toContain("ORIGINAL source body");
  });

  it("routes Edit source + Save to the SOURCE document id", async () => {
    vi.mocked(getLoomBlockTransclusion).mockResolvedValue(resolved("ORIGINAL source body", 3));
    vi.mocked(saveRichDocument).mockResolvedValue({
      rich_document_id: SRC_DOC,
      doc_version: 4,
      save_receipt_event_id: "EVT-SAVE-1",
    } as unknown as Awaited<ReturnType<typeof saveRichDocument>>);

    await act(async () => {
      render(
        <RichTextEditor
          initialContent={hostDoc()}
          onChange={() => undefined}
          embedContext={{ workspaceId: WS }}
          documentTitle="host"
        />,
      );
    });
    await screen.findByTestId("loom-transclusion-content");

    await act(async () => {
      fireEvent.click(screen.getByTestId("loom-transclusion.edit-source"));
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("loom-transclusion.save-source"));
    });

    await waitFor(() => expect(saveRichDocument).toHaveBeenCalled());
    const [docId, version] = vi.mocked(saveRichDocument).mock.calls[0];
    expect(docId).toBe(SRC_DOC);
    expect(version).toBe(3);
    await screen.findByTestId("loom-transclusion.save-status");
    expect(screen.getByTestId("loom-transclusion.save-status").textContent).toContain("EVT-SAVE-1");
  });

  it("renders a typed unresolved state when the block has no source", async () => {
    vi.mocked(getLoomBlockTransclusion).mockResolvedValue({
      block_id: BLOCK,
      workspace_id: WS,
      source_document_id: null,
      source_doc_version: null,
      content_json: null,
      resolved: false,
      unresolved_reason: "loom_block_has_no_source_document",
    });
    await act(async () => {
      render(
        <RichTextEditor
          initialContent={hostDoc()}
          onChange={() => undefined}
          embedContext={{ workspaceId: WS }}
          documentTitle="host"
        />,
      );
    });
    const unresolved = await screen.findByTestId("loom-transclusion-unresolved");
    expect(unresolved.getAttribute("data-reason")).toBe("loom_block_has_no_source_document");
  });

  it("renders a typed error state when resolve fails", async () => {
    vi.mocked(getLoomBlockTransclusion).mockRejectedValue(new Error("backend down"));
    await act(async () => {
      render(
        <RichTextEditor
          initialContent={hostDoc()}
          onChange={() => undefined}
          embedContext={{ workspaceId: WS }}
          documentTitle="host"
        />,
      );
    });
    const err = await screen.findByTestId("loom-transclusion-error");
    expect(err.getAttribute("role")).toBe("alert");
    expect(err.textContent).toContain("backend down");
  });
});
