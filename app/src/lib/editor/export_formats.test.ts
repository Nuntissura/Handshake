// WP-KERNEL-009 / MT-244 — save-to-format export tests.
//
// Proves, against the REAL editor schema (no mocks):
//   - HTML round-trip: export → importHtml → IDENTICAL document structure
//     (the MT-244 acceptance criterion), including typed hsLink attrs and the
//     embedded Monaco code block's language/code/hash,
//   - data-hs-* semantics are stamped on the primary HTML projection,
//   - self-contained mode inlines REAL asset bytes as data: URLs (and guards:
//     videos are never inlined; oversized images fall back to reference),
//   - reference-linked mode emits backend asset content URLs,
//   - unresolvable embeds export fail-closed with a typed visible marker,
//   - the import sanitizer strips scripts/handlers/executable URLs fail-closed,
//   - markdown is produced and is DOCUMENTED-lossy (code hash dropped) while
//     keeping structure (headings/lists/fences/wikilink tokens),
//   - plain text + ProseMirror JSON projections round-trip their contracts,
//   - export filenames never contain blank spaces.

import { describe, it, expect, vi } from "vitest";
import { Editor, type JSONContent } from "@tiptap/core";
import { buildHandshakeEditorExtensions } from "./build_editor_extensions";
import { makeCodeBlockAttrs } from "./code_block_serialization";
import {
  exportHtml,
  importHtml,
  exportMarkdown,
  exportPlainText,
  exportProseMirrorJson,
  importProseMirrorJson,
  sanitizeImportedDom,
  buildExportFilename,
  bytesToBase64,
  HTML_INLINE_IMAGE_MAX_BYTES,
  EXPORT_FORMATS,
} from "./export_formats";
import type { EmbedAssetMetadata, EmbedResolverContext } from "./embed_assets";

const WS = "ws-export";
const BASE = "http://127.0.0.1:9";

const TRICKY_CODE = `</pre><script>alert("xss")</script>\nconst x = "quotes & <tags>";`;

/** A representative document exercising every export surface. */
const SOURCE_DOC: JSONContent = {
  type: "doc",
  content: [
    { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Export & round-trip <proof>" }] },
    {
      type: "paragraph",
      content: [
        { type: "text", text: "Bold ", marks: [{ type: "bold" }] },
        { type: "text", text: "italic ", marks: [{ type: "italic" }] },
        { type: "text", text: "code", marks: [{ type: "code" }] },
        { type: "text", text: " and a " },
        {
          type: "text",
          text: "link",
          marks: [{ type: "link", attrs: { href: "https://example.com/docs" } }],
        },
        { type: "text", text: "." },
      ],
    },
    {
      type: "paragraph",
      content: [
        { type: "hsLink", attrs: { refKind: "wp", refValue: "WP-KERNEL-009", label: "the WP", resolved: true } },
        { type: "text", text: " plus " },
        { type: "hsLink", attrs: { refKind: "unknown", refValue: "mystery", label: "odd:mystery", resolved: false } },
      ],
    },
    {
      type: "bulletList",
      content: [
        { type: "listItem", content: [{ type: "paragraph", content: [{ type: "text", text: "alpha" }] }] },
        { type: "listItem", content: [{ type: "paragraph", content: [{ type: "text", text: "beta" }] }] },
      ],
    },
    {
      type: "taskList",
      content: [
        {
          type: "taskItem",
          attrs: { checked: true },
          content: [{ type: "paragraph", content: [{ type: "text", text: "done thing" }] }],
        },
        {
          type: "taskItem",
          attrs: { checked: false },
          content: [{ type: "paragraph", content: [{ type: "text", text: "open thing" }] }],
        },
      ],
    },
    { type: "blockquote", content: [{ type: "paragraph", content: [{ type: "text", text: "quoted wisdom" }] }] },
    { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("typescript", TRICKY_CODE) },
    {
      type: "paragraph",
      content: [
        { type: "hsLink", attrs: { refKind: "images", refValue: "img-1", label: "img-1", resolved: true } },
      ],
    },
  ],
};

/** Normalizes a doc through the real schema (defaults filled, text merged). */
function normalize(doc: JSONContent): JSONContent {
  const editor = new Editor({
    extensions: buildHandshakeEditorExtensions(),
    content: doc,
  });
  const json = editor.getJSON();
  editor.destroy();
  return json;
}

function asset(id: string, mime: string, sizeBytes = 16): EmbedAssetMetadata {
  return {
    asset_id: id,
    workspace_id: WS,
    kind: mime.startsWith("video/") ? "video" : "image",
    mime,
    original_filename: `${id}.bin`,
    content_hash: `hash-${id}`,
    size_bytes: sizeBytes,
    width: 2,
    height: 2,
  };
}

/** Real-shaped asset backend: metadata + bytes per id; 404 otherwise. */
function embedContextWith(
  assets: Record<string, { metadata: EmbedAssetMetadata; bytes: Uint8Array }>,
): EmbedResolverContext {
  const fetchImpl = vi.fn(async (url: string) => {
    const contentMatch = /\/assets\/([^/]+)\/content$/.exec(url);
    if (contentMatch) {
      const entry = assets[decodeURIComponent(contentMatch[1])];
      if (!entry) return new Response("nope", { status: 404 });
      return new Response(entry.bytes.slice().buffer as ArrayBuffer, {
        status: 200,
        headers: { "Content-Type": entry.metadata.mime },
      });
    }
    const metaMatch = /\/assets\/([^/]+)$/.exec(url);
    const entry = metaMatch && assets[decodeURIComponent(metaMatch[1])];
    if (entry) {
      return new Response(JSON.stringify(entry.metadata), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }
    return new Response("nope", { status: 404 });
  });
  return { workspaceId: WS, apiBaseUrl: BASE, fetchImpl };
}

const PNG_BYTES = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3, 4]);

describe("HTML primary projection (DEC-003)", () => {
  it("round-trips export → import to the identical document structure", async () => {
    const original = normalize(SOURCE_DOC);
    const context = embedContextWith({ "img-1": { metadata: asset("img-1", "image/png"), bytes: PNG_BYTES } });
    const { html } = await exportHtml(original, { mode: "reference_linked", title: "RT", embedContext: context });
    const reimported = normalize(importHtml(html));
    expect(reimported).toEqual(original);
  });

  it("round-trips the embedded Monaco code block byte-identically (hash included)", async () => {
    const original = normalize(SOURCE_DOC);
    const { html } = await exportHtml(original, { mode: "reference_linked" });
    const reimported = normalize(importHtml(html));
    const codeBlock = (reimported.content ?? []).find((n) => n.type === "monacoCodeBlock");
    const sourceBlock = (original.content ?? []).find((n) => n.type === "monacoCodeBlock");
    expect(codeBlock?.attrs).toEqual(sourceBlock?.attrs);
    expect(String(codeBlock?.attrs?.code)).toBe(TRICKY_CODE);
  });

  it("stamps data-hs-* typed semantics on links, embeds, code blocks, and the document", async () => {
    const { html } = await exportHtml(normalize(SOURCE_DOC), {
      mode: "reference_linked",
      title: "Semantics",
      now: () => new Date("2026-06-12T00:00:00Z"),
    });
    const dom = new DOMParser().parseFromString(html, "text/html");
    expect(dom.documentElement.getAttribute("data-hs-export")).toBe("rich_document");
    expect(dom.documentElement.getAttribute("data-hs-schema-version")).toBe("rich_document_v1");
    expect(dom.documentElement.getAttribute("data-hs-export-mode")).toBe("reference_linked");
    expect(dom.documentElement.getAttribute("data-hs-exported-at")).toBe("2026-06-12T00:00:00.000Z");
    expect(dom.documentElement.getAttribute("data-hs-authority")).toContain("projection_only");

    const wpLink = dom.querySelector("span[data-hs-ref-kind='wp']");
    expect(wpLink?.getAttribute("data-hs-node")).toBe("hsLink");
    expect(wpLink?.getAttribute("data-hs-ref-value")).toBe("WP-KERNEL-009");
    expect(wpLink?.getAttribute("data-hs-label")).toBe("the WP");

    const pre = dom.querySelector("pre[data-hs-node='monacoCodeBlock']");
    expect(pre?.getAttribute("data-hs-language")).toBe("typescript");
    expect(pre?.getAttribute("data-hs-rt-hash")).toBeTruthy();
  });

  it("self-contained mode inlines REAL asset bytes as data: URLs", async () => {
    const context = embedContextWith({ "img-1": { metadata: asset("img-1", "image/png"), bytes: PNG_BYTES } });
    const result = await exportHtml(normalize(SOURCE_DOC), {
      mode: "self_contained",
      embedContext: context,
    });
    const expected = `data:image/png;base64,${bytesToBase64(PNG_BYTES)}`;
    const dom = new DOMParser().parseFromString(result.html, "text/html");
    const img = dom.querySelector("span[data-hs-ref-kind='images'] img");
    expect(img?.getAttribute("src")).toBe(expected);
    expect(result.inlinedBytes).toBe(PNG_BYTES.byteLength);
    expect(result.embedErrors).toEqual([]);
  });

  it("reference-linked mode emits the backend asset content URL", async () => {
    const context = embedContextWith({ "img-1": { metadata: asset("img-1", "image/png"), bytes: PNG_BYTES } });
    const { html } = await exportHtml(normalize(SOURCE_DOC), {
      mode: "reference_linked",
      embedContext: context,
    });
    const dom = new DOMParser().parseFromString(html, "text/html");
    const img = dom.querySelector("span[data-hs-ref-kind='images'] img");
    expect(img?.getAttribute("src")).toBe(`${BASE}/workspaces/${WS}/assets/img-1/content`);
  });

  it("never inlines videos in self-contained mode (documented size guard)", async () => {
    const doc: JSONContent = {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [{ type: "hsLink", attrs: { refKind: "video", refValue: "vid-1", label: "vid-1", resolved: true } }],
        },
      ],
    };
    const context = embedContextWith({
      "vid-1": { metadata: asset("vid-1", "video/webm"), bytes: new Uint8Array(32) },
    });
    const result = await exportHtml(normalize(doc), { mode: "self_contained", embedContext: context });
    const dom = new DOMParser().parseFromString(result.html, "text/html");
    const video = dom.querySelector("span[data-hs-ref-kind='video'] video");
    expect(video?.getAttribute("src")).toBe(`${BASE}/workspaces/${WS}/assets/vid-1/content`);
    expect(video?.getAttribute("src")).not.toContain("data:");
    expect(result.inlineSkips).toContainEqual({ refKind: "video", refValue: "vid-1", reason: "video_size_guard" });
    expect(dom.querySelector("span[data-hs-inline-skipped='video_size_guard']")).toBeTruthy();
    expect(dom.querySelector("span[data-hs-inline-note]")?.textContent).toContain("not inlined");
  });

  it("falls back to reference-linking for an image over the per-image cap", async () => {
    const big = new Uint8Array(HTML_INLINE_IMAGE_MAX_BYTES + 1);
    const doc: JSONContent = {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [{ type: "hsLink", attrs: { refKind: "images", refValue: "big-1", label: "big-1", resolved: true } }],
        },
      ],
    };
    const context = embedContextWith({ "big-1": { metadata: asset("big-1", "image/png"), bytes: big } });
    const result = await exportHtml(normalize(doc), { mode: "self_contained", embedContext: context });
    const dom = new DOMParser().parseFromString(result.html, "text/html");
    const img = dom.querySelector("span[data-hs-ref-kind='images'] img");
    expect(img?.getAttribute("src")).toBe(`${BASE}/workspaces/${WS}/assets/big-1/content`);
    expect(result.inlineSkips).toContainEqual({ refKind: "images", refValue: "big-1", reason: "image_size_guard" });
    expect(result.inlinedBytes).toBe(0);
  });

  it("exports an unresolvable embed fail-closed with a typed visible marker", async () => {
    const context = embedContextWith({});
    const result = await exportHtml(normalize(SOURCE_DOC), { mode: "self_contained", embedContext: context });
    expect(result.embedErrors).toContainEqual({ refKind: "images", refValue: "img-1", errorKind: "not_found" });
    const dom = new DOMParser().parseFromString(result.html, "text/html");
    const span = dom.querySelector("span[data-hs-ref-kind='images']");
    expect(span?.getAttribute("data-hs-export-error")).toBe("not_found");
    // The typed chip text stays visible — never blank.
    expect(span?.textContent).toContain("img-1");
  });

  it("exports album/slideshow sequences as ordered real image lists", async () => {
    const doc: JSONContent = {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [
            { type: "hsLink", attrs: { refKind: "slideshow", refValue: "s-1,s-2", label: "show", resolved: true } },
          ],
        },
      ],
    };
    const context = embedContextWith({
      "s-1": { metadata: asset("s-1", "image/png"), bytes: PNG_BYTES },
      "s-2": { metadata: asset("s-2", "image/jpeg"), bytes: PNG_BYTES },
    });
    const { html } = await exportHtml(normalize(doc), { mode: "reference_linked", embedContext: context });
    const dom = new DOMParser().parseFromString(html, "text/html");
    const sequence = dom.querySelector("span[data-hs-sequence='slideshow']");
    expect(sequence?.getAttribute("data-hs-sequence-length")).toBe("2");
    const imgs = sequence?.querySelectorAll("img") ?? [];
    expect(imgs.length).toBe(2);
    expect(imgs[0].getAttribute("data-hs-asset-id")).toBe("s-1");
    expect(imgs[1].getAttribute("data-hs-asset-id")).toBe("s-2");
  });
});

describe("HTML import sanitization (fail-closed)", () => {
  it("strips script elements, on* handlers, and executable URLs", () => {
    const dom = new DOMParser().parseFromString(
      `<html><body><main data-hs-content="">
        <p>safe</p>
        <script>window.x = 1;</script>
        <iframe src="https://evil.example"></iframe>
        <img src="x.png" onerror="alert(1)">
        <a href="javascript:alert(1)">bad</a>
        <a href="java	script:alert(1)">obfuscated</a>
        <object data="x"></object>
        <form action="javascript:alert(1)"><input></form>
      </main></body></html>`,
      "text/html",
    );
    sanitizeImportedDom(dom);
    expect(dom.querySelector("script")).toBeNull();
    expect(dom.querySelector("iframe")).toBeNull();
    expect(dom.querySelector("object")).toBeNull();
    expect(dom.querySelector("form")).toBeNull();
    expect(dom.querySelector("img")?.hasAttribute("onerror")).toBe(false);
    for (const anchor of Array.from(dom.querySelectorAll("a"))) {
      expect(anchor.hasAttribute("href")).toBe(false);
    }
  });

  it("imports a malicious export without executing or preserving script content", () => {
    const hostile = `<!doctype html><html><body><main data-hs-content="">
      <p>before</p><script>document.title="pwned"</script><p onclick="alert(1)">after</p>
    </main></body></html>`;
    const doc = importHtml(hostile);
    const text = JSON.stringify(doc);
    expect(text).not.toContain("pwned");
    expect(text).not.toContain("alert");
    expect(text).toContain("before");
    expect(text).toContain("after");
  });

  it("survives a code block whose code embeds </pre><script> (attribute-escaped)", async () => {
    const original = normalize(SOURCE_DOC);
    const { html } = await exportHtml(original, { mode: "reference_linked" });
    // The raw script text must not exist as an executable element.
    const dom = new DOMParser().parseFromString(html, "text/html");
    expect(dom.querySelector("script")).toBeNull();
    const reimported = normalize(importHtml(html));
    expect(reimported).toEqual(original);
  });
});

describe("markdown projection (deliberately lossy)", () => {
  it("keeps structure: headings, marks, lists, tasks, quote, fence, wikilinks", () => {
    const markdown = exportMarkdown(normalize(SOURCE_DOC));
    expect(markdown).toContain("# Export & round-trip <proof>");
    expect(markdown).toContain("**Bold **");
    expect(markdown).toContain("[link](https://example.com/docs)");
    expect(markdown).toContain("[[wp:WP-KERNEL-009|the WP]]");
    expect(markdown).toContain("- alpha");
    expect(markdown).toContain("- [x] done thing");
    expect(markdown).toContain("- [ ] open thing");
    expect(markdown).toContain("> quoted wisdom");
    expect(markdown).toContain("```typescript");
    expect(markdown).toContain('const x = "quotes & <tags>";');
    expect(markdown).toContain("[[images:img-1]]");
  });

  it("is documented-lossy: the code-block round-trip hash does not survive", () => {
    const markdown = exportMarkdown(normalize(SOURCE_DOC));
    const sourceBlock = (normalize(SOURCE_DOC).content ?? []).find((n) => n.type === "monacoCodeBlock");
    expect(markdown).not.toContain(String(sourceBlock?.attrs?.contentHash));
    expect(EXPORT_FORMATS.find((f) => f.id === "markdown")?.lossy).toBe(true);
  });

  it("renders pipe tables", () => {
    const doc: JSONContent = {
      type: "doc",
      content: [
        {
          type: "table",
          content: [
            {
              type: "tableRow",
              content: [
                { type: "tableHeader", content: [{ type: "paragraph", content: [{ type: "text", text: "H1" }] }] },
                { type: "tableHeader", content: [{ type: "paragraph", content: [{ type: "text", text: "H2" }] }] },
              ],
            },
            {
              type: "tableRow",
              content: [
                { type: "tableCell", content: [{ type: "paragraph", content: [{ type: "text", text: "a|b" }] }] },
                { type: "tableCell", content: [{ type: "paragraph", content: [{ type: "text", text: "c" }] }] },
              ],
            },
          ],
        },
      ],
    };
    const markdown = exportMarkdown(normalize(doc));
    expect(markdown).toContain("| H1 | H2 |");
    expect(markdown).toContain("| --- | --- |");
    expect(markdown).toContain("| a\\|b | c |");
  });
});

describe("plain text + ProseMirror JSON projections", () => {
  it("plain text keeps wikilink tokens and fenced code (renderText serializers)", () => {
    const text = exportPlainText(normalize(SOURCE_DOC));
    expect(text).toContain("Export & round-trip <proof>");
    expect(text).toContain("[[wp:WP-KERNEL-009|the WP]]");
    expect(text).toContain("```typescript");
    expect(text).toContain('const x = "quotes & <tags>";');
  });

  it("JSON envelope round-trips and refuses non-doc payloads", () => {
    const original = normalize(SOURCE_DOC);
    const json = exportProseMirrorJson(original, () => new Date("2026-06-12T00:00:00Z"));
    const envelope = JSON.parse(json);
    expect(envelope.format).toBe("handshake_rich_document");
    expect(envelope.schema_version).toBe("rich_document_v1");
    expect(envelope.authority).toContain("projection_only");
    expect(importProseMirrorJson(json)).toEqual(original);
    // Bare doc accepted too.
    expect(importProseMirrorJson(JSON.stringify(original))).toEqual(original);
    expect(() => importProseMirrorJson("{\"nope\":true}")).toThrow(/doc/);
    expect(() => importProseMirrorJson("not json")).toThrow(/JSON/);
  });
});

describe("export filenames (naming policy: no blank spaces)", () => {
  it("slugs titles and never emits spaces", () => {
    const name = buildExportFilename("My Grand Document (v2)", "html_self_contained", () => new Date("2026-06-12T03:04:05Z"));
    expect(name).toBe("my-grand-document-v2-self-contained-2026-06-12T03-04-05.html");
    expect(name).not.toMatch(/\s/);
    expect(buildExportFilename(undefined, "markdown", () => new Date("2026-06-12T03:04:05Z"))).toMatch(/^handshake-document-.*\.md$/);
  });
});
