// WP-KERNEL-009 / MT-244 — CKC embed NodeView tests.
//
// Mounts the REAL integrated editor (RichTextEditor → hsLink NodeView) in
// jsdom with an injected fetch and proves:
//   - [[HS_images:…]] renders a real <img> whose src is the typed backend
//     asset content URL,
//   - [[video:…]] renders a real <video controls> element,
//   - [[HS_slideshow:a,b]] / [[album:…]] render the sequenced viewer with
//     prev/next navigation over independently resolved members,
//   - an unresolvable embed renders the TYPED visible error state
//     (role=alert + data-error-kind) — never blank, never mock data,
//   - an absolute-path ref fails closed client-side,
//   - an editor without a workspace context fails closed (no_workspace),
//   - non-media kinds keep the exact MT-163 chip selectors.

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { act } from "react";
import { beforeEach, describe, it, expect, vi } from "vitest";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "./RichTextEditor";
import { getLoomBlock, type LoomBlock } from "../lib/api";
import type { EmbedAssetMetadata } from "../lib/editor/embed_assets";
import { onHsLinkNavigate, type HsLinkNavigateDetail } from "../lib/editor/link_navigation";

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    getLoomBlock: vi.fn(),
  };
});

const WS = "ws-embed-test";
const BASE = "http://127.0.0.1:9";

function asset(id: string, mime: string): EmbedAssetMetadata {
  return {
    asset_id: id,
    workspace_id: WS,
    kind: mime.startsWith("video/") ? "video" : "image",
    mime,
    original_filename: `${id}.bin`,
    content_hash: `hash-${id}`,
    size_bytes: 64,
    width: 4,
    height: 4,
  };
}

function loomBlock(blockId: string, title: string, fullText: string): LoomBlock {
  return {
    block_id: blockId,
    workspace_id: WS,
    content_type: "note",
    document_id: null,
    asset_id: null,
    title,
    original_filename: null,
    content_hash: null,
    pinned: false,
    favorite: false,
    pin_order: null,
    journal_date: null,
    created_at: "2026-06-16T00:00:00Z",
    updated_at: "2026-06-16T00:00:00Z",
    imported_at: null,
    derived: {
      full_text_index: fullText,
      backlink_count: 2,
      mention_count: 3,
      tag_count: 4,
      preview_status: "ready",
    },
  };
}

/** Real-shaped backend: metadata per asset id, 404 for anything else. */
function backendFetch(assets: Record<string, EmbedAssetMetadata>) {
  return vi.fn(async (url: string) => {
    const match = /\/workspaces\/([^/]+)\/assets\/([^/]+)$/.exec(url);
    const found = match && assets[decodeURIComponent(match[2])];
    if (found) {
      return new Response(JSON.stringify(found), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }
    return new Response("not found", { status: 404 });
  });
}

function docWithLink(refKind: string, refValue: string): JSONContent {
  return {
    type: "doc",
    content: [
      {
        type: "paragraph",
        content: [{ type: "text", text: "before " }],
      },
      {
        type: "paragraph",
        content: [
          {
            type: "hsLink",
            attrs: { refKind, refValue, label: refValue, resolved: true },
          },
        ],
      },
    ],
  };
}

async function mountWithEmbeds(
  doc: JSONContent,
  assets: Record<string, EmbedAssetMetadata>,
  withContext = true,
) {
  const fetchImpl = backendFetch(assets);
  await act(async () => {
    render(
      <RichTextEditor
        initialContent={doc}
        onChange={() => {}}
        embedContext={withContext ? { workspaceId: WS, apiBaseUrl: BASE, fetchImpl } : undefined}
      />,
    );
  });
  return { fetchImpl };
}

describe("HsLinkView media embeds (MT-244)", () => {
  beforeEach(() => {
    vi.mocked(getLoomBlock).mockReset();
  });

  it("renders a real <img> from the typed backend asset content URL", async () => {
    await mountWithEmbeds(docWithLink("images", "img-1"), { "img-1": asset("img-1", "image/png") });
    const img = await screen.findByTestId("hs-embed-image");
    expect(img.getAttribute("src")).toBe(`${BASE}/workspaces/${WS}/assets/img-1/content`);
    expect(img.getAttribute("data-asset-mime")).toBe("image/png");
    expect(img.getAttribute("alt")).toBeTruthy();
  });

  it("renders a real <video controls> for a video embed", async () => {
    await mountWithEmbeds(docWithLink("video", "vid-1"), { "vid-1": asset("vid-1", "video/webm") });
    const video = await screen.findByTestId("hs-embed-video");
    expect(video.tagName.toLowerCase()).toBe("video");
    expect(video.getAttribute("src")).toBe(`${BASE}/workspaces/${WS}/assets/vid-1/content`);
    expect(video.hasAttribute("controls")).toBe(true);
  });

  it("renders the sequenced slideshow viewer with prev/next over resolved members", async () => {
    await mountWithEmbeds(docWithLink("slideshow", "s-1,s-2"), {
      "s-1": asset("s-1", "image/png"),
      "s-2": asset("s-2", "image/jpeg"),
    });
    const sequence = await screen.findByTestId("hs-embed-sequence");
    expect(sequence.getAttribute("data-sequence-length")).toBe("2");
    expect(sequence.getAttribute("data-embed-kind")).toBe("slideshow");
    expect(screen.getByTestId("hs-embed-sequence-position").textContent).toBe("1/2");
    expect(screen.getByTestId("hs-embed-sequence-image").getAttribute("data-asset-id")).toBe("s-1");

    await act(async () => {
      fireEvent.click(screen.getByTestId("hs-embed-sequence-next"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("hs-embed-sequence-image").getAttribute("data-asset-id")).toBe("s-2");
    });
    expect(screen.getByTestId("hs-embed-sequence-position").textContent).toBe("2/2");
    // Next disabled at the end; prev re-enabled.
    expect((screen.getByTestId("hs-embed-sequence-next") as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByTestId("hs-embed-sequence-prev") as HTMLButtonElement).disabled).toBe(false);
  });

  it("renders a broken album member as a typed per-item error while the rest resolves", async () => {
    await mountWithEmbeds(docWithLink("album", "ok-1,gone-2"), { "ok-1": asset("ok-1", "image/png") });
    const sequence = await screen.findByTestId("hs-embed-sequence");
    expect(sequence.getAttribute("data-sequence-resolved")).toBe("1");
    // Navigate to the broken member → typed error visible inside the sequence.
    await act(async () => {
      fireEvent.click(screen.getByTestId("hs-embed-sequence-next"));
    });
    const error = await screen.findByTestId("hs-embed-error");
    expect(error.getAttribute("role")).toBe("alert");
    expect(error.getAttribute("data-error-kind")).toBe("not_found");
  });

  it("renders the typed visible error state for an unresolvable embed (never blank)", async () => {
    await mountWithEmbeds(docWithLink("images", "missing-1"), {});
    const error = await screen.findByTestId("hs-embed-error");
    expect(error.getAttribute("role")).toBe("alert");
    expect(error.getAttribute("data-error-kind")).toBe("not_found");
    expect(error.textContent).toContain("missing-1");
  });

  it("fails closed on a kind mismatch (video asset inside an images embed)", async () => {
    await mountWithEmbeds(docWithLink("images", "vid-9"), { "vid-9": asset("vid-9", "video/webm") });
    const error = await screen.findByTestId("hs-embed-error");
    expect(error.getAttribute("data-error-kind")).toBe("kind_mismatch");
  });

  it("rejects an absolute-path ref client-side (backend MT-152 mirror)", async () => {
    const { fetchImpl } = await mountWithEmbeds(docWithLink("images", "C:\\evil\\x.png"), {});
    const error = await screen.findByTestId("hs-embed-error");
    expect(error.getAttribute("data-error-kind")).toBe("absolute_path_rejected");
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it("fails closed with no_workspace when the editor has no embed context", async () => {
    await mountWithEmbeds(docWithLink("images", "img-1"), { "img-1": asset("img-1", "image/png") }, false);
    const error = await screen.findByTestId("hs-embed-error");
    expect(error.getAttribute("data-error-kind")).toBe("no_workspace");
  });

  it("keeps the exact MT-163 chip for non-media kinds (selectors unchanged)", async () => {
    await mountWithEmbeds(docWithLink("wp", "WP-KERNEL-009"), {});
    const chip = await screen.findByTestId("hs-link");
    expect(chip.getAttribute("data-ref-kind")).toBe("wp");
    expect(chip.getAttribute("data-ref-value")).toBe("WP-KERNEL-009");
    expect(chip.className).toContain("hs-link--resolved");
    expect(chip.getAttribute("title")).toContain("wp:WP-KERNEL-009");
    expect(chip.textContent).toContain("WP-KERNEL-009");
    // No embed surfaces for a chip kind.
    expect(screen.queryByTestId("hs-embed-error")).toBeNull();
    expect(screen.queryByTestId("hs-embed-loading")).toBeNull();
  });

  it("loads a Loom block hover preview for non-media Loom links", async () => {
    vi.mocked(getLoomBlock).mockResolvedValueOnce(
      loomBlock("block-alpha", "Alpha Loom note", "Alpha hover preview text from the indexed block body."),
    );
    await mountWithEmbeds(docWithLink("note", "block-alpha"), {});

    const chip = await screen.findByTestId("hs-link");
    await act(async () => {
      fireEvent.mouseEnter(chip);
    });

    expect(await screen.findByTestId("hs-link-preview")).toHaveTextContent("Alpha Loom note");
    expect(screen.getByTestId("hs-link-preview")).toHaveTextContent("Alpha hover preview text");
    expect(screen.getByTestId("hs-link-preview")).toHaveTextContent("4 tags");
    expect(getLoomBlock).toHaveBeenCalledWith(WS, "block-alpha");
  });

  it("renders a visible hover preview error when a Loom block cannot resolve", async () => {
    vi.mocked(getLoomBlock).mockRejectedValueOnce(new Error("missing block"));
    await mountWithEmbeds(docWithLink("note", "missing-block"), {});

    const chip = await screen.findByTestId("hs-link");
    await act(async () => {
      fireEvent.mouseEnter(chip);
    });

    const error = await screen.findByTestId("hs-link-preview-error");
    expect(error.getAttribute("role")).toBe("alert");
    expect(error).toHaveTextContent("missing block");
  });

  it("dispatches a typed navigation intent when a chip is clicked (iteration-3 EXT-NAV-LINK-001)", async () => {
    await mountWithEmbeds(docWithLink("wp", "WP-KERNEL-009"), {});
    const chip = await screen.findByTestId("hs-link");
    expect(chip.getAttribute("data-navigable")).toBe("true");

    const received: HsLinkNavigateDetail[] = [];
    const unsubscribe = onHsLinkNavigate((detail) => received.push(detail));
    await act(async () => {
      fireEvent.click(chip);
    });
    unsubscribe();
    expect(received).toHaveLength(1);
    expect(received[0]).toEqual({
      refKind: "wp",
      refValue: "WP-KERNEL-009",
      label: "WP-KERNEL-009",
    });
  });
});
