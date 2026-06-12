// WP-KERNEL-009 / MT-244 — save-to-format export projections (DEC-003).
//
// Pure(ish) projection builders from the RichDocument editor JSON to the
// operator-facing export formats. AUTHORITY NOTE (operator decision DEC-003):
// the Postgres RichDocument/EventLedger record remains the ONLY authority —
// every export produced here is a projection; importing one back never
// bypasses the save path.
//
// Formats:
//   - HTML (PRIMARY projection): Tiptap generateHTML over the real editor
//     schema, post-processed in a detached DOM to carry data-hs-* attributes
//     with the typed link/embed/code semantics for near-lossless round-trip.
//     TWO modes (DEC-003, operator-confirmed):
//       * self_contained  — image asset bytes are fetched from the REAL
//         backend asset endpoint and base64-inlined as data: URLs so the file
//         opens standalone. SIZE GUARDS (adversarial decision, documented):
//         images inline up to HTML_INLINE_IMAGE_MAX_BYTES each and
//         HTML_INLINE_TOTAL_MAX_BYTES per document; VIDEOS ARE NEVER INLINED
//         (base64 blowup ×1.37 on multi-hundred-MB media makes the file
//         unopenable) — a video keeps a reference-linked <video> plus a
//         visible data-hs-inline-skipped="video_size_guard" notice.
//       * reference_linked — media elements reference the backend asset
//         content URLs (file is small; needs the backend running to show
//         media).
//     Unresolvable embeds export FAIL-CLOSED: the typed chip stays visible
//     and carries data-hs-export-error="<errorKind>" — never silently blank.
//   - markdown: a DELIBERATELY LOSSY extractor for interop (DEC-003). Drops:
//     underline/strike nuance, table spans/col widths, code-block round-trip
//     hashes, mention identity (flattened to @id/#id), embed sequencing
//     (flattened to [[kind:value]] tokens), task-item nesting beyond lists.
//   - plain_text: generateText over the schema's renderText serializers
//     (wikilink tokens + fenced code blocks survive as text).
//   - prosemirror_json: the doc JSON in a typed envelope (schema version +
//     projection disclaimer); accepted back by importProseMirrorJson.
//
// The HTML IMPORT path here sanitizes FAIL-CLOSED before any parse reaches
// the editor schema: script/iframe/object/embed/frame/base/form elements are
// removed, every on* handler attribute is stripped, and javascript:/
// vbscript:/data:text-html URLs are removed. Unknown content is dropped, not
// preserved.
//
// All HTML assembly uses DOM APIs (never string concatenation) so attribute
// values — labels, code text, titles — are always serializer-escaped (XSS
// review in the MT-244 adversarial pass).

import { generateHTML, generateText, getSchema, type JSONContent } from "@tiptap/core";
import { DOMParser as PMDOMParser } from "@tiptap/pm/model";
import {
  buildHandshakeEditorExtensions,
  type HandshakeEditorExtensionOptions,
} from "./build_editor_extensions";
import { WP009_RICH_DOCUMENT_SCHEMA_VERSION } from "../tiptap/extension_set";
import {
  assetContentUrl,
  isMediaEmbedKind,
  parseAssetRefList,
  resolveEmbedAsset,
  type EmbedResolverContext,
  type MediaEmbedRefKind,
} from "./embed_assets";
import { API_BASE_URL } from "../api";

export type HtmlExportMode = "self_contained" | "reference_linked";

export type ExportFormatId =
  | "html_self_contained"
  | "html_reference_linked"
  | "markdown"
  | "plain_text"
  | "prosemirror_json";

/** Per-image inline cap (self-contained mode). Above it: reference + notice. */
export const HTML_INLINE_IMAGE_MAX_BYTES = 8 * 1024 * 1024;
/** Whole-document inline budget (self-contained mode). */
export const HTML_INLINE_TOTAL_MAX_BYTES = 64 * 1024 * 1024;

/** The export catalog the toolbar/palette surfaces (stable ids + labels). */
export const EXPORT_FORMATS: ReadonlyArray<{
  id: ExportFormatId;
  label: string;
  extension: string;
  mimeType: string;
  lossy: boolean;
}> = [
  {
    id: "html_self_contained",
    label: "HTML (self-contained, assets inlined)",
    extension: "html",
    mimeType: "text/html",
    lossy: false,
  },
  {
    id: "html_reference_linked",
    label: "HTML (reference-linked assets)",
    extension: "html",
    mimeType: "text/html",
    lossy: false,
  },
  { id: "markdown", label: "Markdown (lossy)", extension: "md", mimeType: "text/markdown", lossy: true },
  { id: "plain_text", label: "Plain text (lossy)", extension: "txt", mimeType: "text/plain", lossy: true },
  {
    id: "prosemirror_json",
    label: "ProseMirror JSON",
    extension: "json",
    mimeType: "application/json",
    lossy: false,
  },
];

export interface HtmlExportOptions {
  mode: HtmlExportMode;
  /** Document title (head <title> + data-hs-title). */
  title?: string;
  /** Embed workspace/transport context; absent = embeds fail closed. */
  embedContext?: EmbedResolverContext | null;
  /** Extension options (tests/DI); embeds resolve through embedContext. */
  extensionOptions?: HandshakeEditorExtensionOptions;
  /** Injectable clock for deterministic tests. */
  now?: () => Date;
}

export interface HtmlExportResult {
  html: string;
  /** Embeds that exported with a typed error marker (fail-closed visible). */
  embedErrors: Array<{ refKind: string; refValue: string; errorKind: string }>;
  /** Media kept reference-linked by a size guard in self-contained mode. */
  inlineSkips: Array<{ refKind: string; refValue: string; reason: string }>;
  /** Total bytes base64-inlined (self-contained mode). */
  inlinedBytes: number;
}

function buildExtensions(options?: HandshakeEditorExtensionOptions) {
  return buildHandshakeEditorExtensions(options ?? {});
}

/** Base64 of raw bytes without Node Buffer (browser + jsdom safe). */
export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
  }
  return btoa(binary);
}

/** DOM root accepted by the transform helpers (runtime-global types only). */
type DomRoot = Document | Element | DocumentFragment;

interface MediaExportPlan {
  kind: MediaEmbedRefKind;
  refValue: string;
  span: HTMLElement;
}

function collectMediaSpans(root: DomRoot): MediaExportPlan[] {
  const plans: MediaExportPlan[] = [];
  for (const span of Array.from(root.querySelectorAll("span[data-testid='hs-link']"))) {
    const refKind = span.getAttribute("data-ref-kind") ?? "unknown";
    if (!isMediaEmbedKind(refKind)) continue;
    plans.push({
      kind: refKind,
      refValue: span.getAttribute("data-ref-value") ?? "",
      span: span as HTMLElement,
    });
  }
  return plans;
}

/** Copies the typed data-ref-* semantics onto data-hs-* attributes. */
function stampHsSemantics(root: DomRoot): void {
  for (const span of Array.from(root.querySelectorAll("span[data-testid='hs-link']"))) {
    span.setAttribute("data-hs-node", "hsLink");
    span.setAttribute("data-hs-ref-kind", span.getAttribute("data-ref-kind") ?? "unknown");
    span.setAttribute("data-hs-ref-value", span.getAttribute("data-ref-value") ?? "");
    span.setAttribute("data-hs-label", span.getAttribute("data-label") ?? "");
    span.setAttribute("data-hs-resolved", span.getAttribute("data-resolved") ?? "true");
  }
  for (const pre of Array.from(
    root.querySelectorAll("pre[data-testid='monaco-code-block-serialized']"),
  )) {
    pre.setAttribute("data-hs-node", "monacoCodeBlock");
    pre.setAttribute("data-hs-language", pre.getAttribute("data-language") ?? "");
    pre.setAttribute("data-hs-rt-hash", pre.getAttribute("data-rt-hash") ?? "");
  }
}

interface MediaRenderOutcome {
  embedErrors: HtmlExportResult["embedErrors"];
  inlineSkips: HtmlExportResult["inlineSkips"];
  inlinedBytes: number;
}

/**
 * Renders the media embeds inside the exported DOM: resolves each asset
 * against the REAL backend metadata endpoint and appends a real <img>/<video>
 * (sequenced <span data-hs-sequence> for albums/slideshows). Self-contained
 * mode fetches the asset bytes and inlines data: URLs under the size guards
 * documented in the module header. Every failure marks the chip with
 * data-hs-export-error and leaves the typed label text visible (fail-closed).
 */
async function renderMediaEmbeds(
  doc: Document,
  mode: HtmlExportMode,
  embedContext: EmbedResolverContext | null,
): Promise<MediaRenderOutcome> {
  const outcome: MediaRenderOutcome = { embedErrors: [], inlineSkips: [], inlinedBytes: 0 };
  const plans = collectMediaSpans(doc.body);
  if (plans.length === 0) return outcome;

  const context = embedContext ?? null;
  const workspaceId = context?.workspaceId ?? "";
  const baseUrl = context?.apiBaseUrl ?? API_BASE_URL;
  const fetchImpl = context?.fetchImpl ?? ((url: string) => globalThis.fetch(url));

  const inlineImage = async (assetId: string, mime: string): Promise<string | { skipped: string }> => {
    const url = assetContentUrl(baseUrl, workspaceId, assetId);
    const response = await fetchImpl(url);
    if (!response.ok) throw new Error(`asset content HTTP ${response.status}`);
    const buffer = new Uint8Array(await response.arrayBuffer());
    if (buffer.byteLength > HTML_INLINE_IMAGE_MAX_BYTES) return { skipped: "image_size_guard" };
    if (outcome.inlinedBytes + buffer.byteLength > HTML_INLINE_TOTAL_MAX_BYTES) {
      return { skipped: "document_inline_budget" };
    }
    outcome.inlinedBytes += buffer.byteLength;
    return `data:${mime};base64,${bytesToBase64(buffer)}`;
  };

  const markError = (plan: MediaExportPlan, errorKind: string) => {
    plan.span.setAttribute("data-hs-export-error", errorKind);
    outcome.embedErrors.push({ refKind: plan.kind, refValue: plan.refValue, errorKind });
  };

  const appendImage = async (
    plan: MediaExportPlan,
    parent: HTMLElement,
    assetId: string,
  ): Promise<boolean> => {
    const resolution = await resolveEmbedAsset(
      plan.kind,
      assetId,
      context ?? { workspaceId: "" },
    );
    if (!resolution.ok) {
      markError(plan, resolution.errorKind);
      return false;
    }
    const img = doc.createElement("img");
    img.setAttribute("data-hs-asset-id", resolution.asset.asset_id);
    img.setAttribute("alt", resolution.asset.original_filename ?? assetId);
    let src = resolution.contentUrl;
    if (mode === "self_contained") {
      try {
        const inlined = await inlineImage(assetId, resolution.asset.mime);
        if (typeof inlined === "string") {
          src = inlined;
        } else {
          plan.span.setAttribute("data-hs-inline-skipped", inlined.skipped);
          outcome.inlineSkips.push({ refKind: plan.kind, refValue: assetId, reason: inlined.skipped });
        }
      } catch (error) {
        markError(plan, `content_fetch_failed:${error instanceof Error ? error.message : String(error)}`);
        return false;
      }
    }
    img.setAttribute("src", src);
    parent.appendChild(img);
    return true;
  };

  for (const plan of plans) {
    if (!context || !context.workspaceId) {
      markError(plan, "no_workspace");
      continue;
    }
    if (plan.kind === "images") {
      await appendImage(plan, plan.span, plan.refValue.trim());
      continue;
    }
    if (plan.kind === "video") {
      const resolution = await resolveEmbedAsset("video", plan.refValue.trim(), context);
      if (!resolution.ok) {
        markError(plan, resolution.errorKind);
        continue;
      }
      const video = doc.createElement("video");
      video.setAttribute("controls", "");
      video.setAttribute("preload", "metadata");
      video.setAttribute("data-hs-asset-id", resolution.asset.asset_id);
      video.setAttribute("src", resolution.contentUrl);
      plan.span.appendChild(video);
      if (mode === "self_contained") {
        // DOCUMENTED GUARD: videos are never base64-inlined (module header).
        plan.span.setAttribute("data-hs-inline-skipped", "video_size_guard");
        outcome.inlineSkips.push({
          refKind: "video",
          refValue: plan.refValue,
          reason: "video_size_guard",
        });
        const note = doc.createElement("span");
        note.setAttribute("data-hs-inline-note", "video_size_guard");
        note.textContent = " [video not inlined: references the workspace asset]";
        plan.span.appendChild(note);
      }
      continue;
    }
    // album / slideshow → ordered sequence of real images.
    const refs = parseAssetRefList(plan.refValue);
    if (refs.length === 0) {
      markError(plan, "empty_ref");
      continue;
    }
    const sequence = doc.createElement("span");
    sequence.setAttribute("data-hs-sequence", plan.kind);
    sequence.setAttribute("data-hs-sequence-length", String(refs.length));
    plan.span.appendChild(sequence);
    for (const ref of refs) {
      await appendImage(plan, sequence, ref);
    }
  }
  return outcome;
}

const EXPORT_STYLES = `
body { font-family: system-ui, sans-serif; max-width: 56rem; margin: 2rem auto; padding: 0 1rem; line-height: 1.55; }
pre[data-hs-node='monacoCodeBlock'] { background: #111; color: #eee; padding: 0.75rem; border-radius: 6px; overflow: auto; }
span[data-hs-node='hsLink'] { background: #eef; border-radius: 4px; padding: 0 0.25rem; }
span[data-hs-node='hsLink'] img, span[data-hs-node='hsLink'] video { display: block; max-width: 100%; margin: 0.5rem 0; }
span[data-hs-export-error] { background: #fee; border: 1px solid #c66; }
table { border-collapse: collapse; } td, th { border: 1px solid #999; padding: 0.25rem 0.5rem; }
`;

/**
 * Exports the editor document JSON to the PRIMARY HTML projection (DEC-003).
 * Near-lossless: importHtml() on the result reproduces the identical document
 * structure (proven in export_formats.test.ts).
 */
export async function exportHtml(
  docJson: JSONContent,
  options: HtmlExportOptions,
): Promise<HtmlExportResult> {
  const extensions = buildExtensions(options.extensionOptions);
  const bodyHtml = generateHTML(docJson, extensions);

  const dom = new DOMParser().parseFromString(
    "<!doctype html><html><head></head><body><main data-hs-content=''></main></body></html>",
    "text/html",
  );
  const main = dom.querySelector("main[data-hs-content]") as HTMLElement;
  // The generated fragment is parsed into the SAME detached document; all
  // subsequent transforms are DOM-API only (serializer-escaped output).
  const fragment = dom.createElement("template");
  fragment.innerHTML = bodyHtml;
  main.appendChild(fragment.content);

  stampHsSemantics(main);
  const outcome = await renderMediaEmbeds(dom, options.mode, options.embedContext ?? null);

  const html = dom.documentElement;
  html.setAttribute("lang", "en");
  html.setAttribute("data-hs-export", "rich_document");
  html.setAttribute("data-hs-schema-version", WP009_RICH_DOCUMENT_SCHEMA_VERSION);
  html.setAttribute("data-hs-export-mode", options.mode);
  html.setAttribute("data-hs-exported-at", (options.now?.() ?? new Date()).toISOString());
  html.setAttribute("data-hs-authority", "projection_only:postgres_rich_document_is_authority");

  const head = dom.head;
  const charset = dom.createElement("meta");
  charset.setAttribute("charset", "utf-8");
  head.appendChild(charset);
  const titleEl = dom.createElement("title");
  titleEl.textContent = options.title ?? "Handshake document";
  head.appendChild(titleEl);
  if (options.title) html.setAttribute("data-hs-title", options.title);
  const style = dom.createElement("style");
  style.textContent = EXPORT_STYLES;
  head.appendChild(style);

  return {
    html: `<!doctype html>\n${html.outerHTML}`,
    embedErrors: outcome.embedErrors,
    inlineSkips: outcome.inlineSkips,
    inlinedBytes: outcome.inlinedBytes,
  };
}

const FORBIDDEN_ELEMENTS = [
  "script",
  "iframe",
  "object",
  "embed",
  "frame",
  "frameset",
  "base",
  "form",
  "noscript",
];

function isForbiddenUrl(value: string): boolean {
  // Strip whitespace, control chars, and U+FFFD to defeat scheme obfuscation:
  // browsers strip tab/LF/CR inside URL schemes ("java	script:" executes in
  // a real browser), and HTML parsing replaces NUL with U+FFFD before this
  // code ever sees it (adversarial finding, MT-244 review).
  // eslint-disable-next-line no-control-regex -- the control range IS the filter
  const normalized = value.trim().toLowerCase().replace(/[\s\u0000-\u001f\ufffd-]+/g, "");
  return (
    normalized.startsWith("javascript:") ||
    normalized.startsWith("vbscript:") ||
    normalized.startsWith("data:text/html") ||
    normalized.startsWith("data:image/svg") // svg can carry scripts
  );
}

/**
 * Sanitizes a parsed HTML document FAIL-CLOSED in place: forbidden elements
 * are removed entirely, on* handlers are stripped everywhere, and executable
 * URL schemes are removed from href/src/srcset/xlink:href.
 */
export function sanitizeImportedDom(root: DomRoot): void {
  for (const tag of FORBIDDEN_ELEMENTS) {
    for (const el of Array.from(root.querySelectorAll(tag))) el.remove();
  }
  const all = root.querySelectorAll("*");
  for (const el of Array.from(all)) {
    for (const attr of Array.from(el.attributes)) {
      const name = attr.name.toLowerCase();
      if (name.startsWith("on")) {
        el.removeAttribute(attr.name);
        continue;
      }
      if (["href", "src", "srcset", "xlink:href", "formaction", "action"].includes(name)) {
        if (isForbiddenUrl(attr.value)) el.removeAttribute(attr.name);
      }
    }
  }
}

/**
 * Imports an HTML export back to editor document JSON: sanitize fail-closed,
 * then parse with the REAL editor schema (preserveWhitespace so prose text
 * survives byte-identical). The hsLink/monacoCodeBlock parseHTML rules read
 * the typed attributes back, so export→import round-trips the structure.
 *
 * Import produces an EDITOR DOCUMENT only — persisting it still flows through
 * the normal save path (the export file is never authority, DEC-003).
 */
export function importHtml(
  html: string,
  extensionOptions?: HandshakeEditorExtensionOptions,
): JSONContent {
  const dom = new DOMParser().parseFromString(html, "text/html");
  sanitizeImportedDom(dom);
  const content = dom.querySelector("main[data-hs-content]") ?? dom.body;
  const schema = getSchema(buildExtensions(extensionOptions));
  const parsed = PMDOMParser.fromSchema(schema).parse(content, { preserveWhitespace: true });
  return parsed.toJSON() as JSONContent;
}

// ---------------------------------------------------------------------------
// Markdown (deliberately lossy — see module header for the documented drops).
// ---------------------------------------------------------------------------

type MarkLike = { type: string; attrs?: Record<string, unknown> };

function markdownInline(node: JSONContent): string {
  if (node.type === "text") {
    let text = node.text ?? "";
    const marks = (node.marks ?? []) as MarkLike[];
    const has = (type: string) => marks.some((mark) => mark.type === type);
    if (has("code")) text = `\`${text}\``;
    if (has("bold")) text = `**${text}**`;
    if (has("italic")) text = `*${text}*`;
    const link = marks.find((mark) => mark.type === "link");
    if (link) text = `[${text}](${String(link.attrs?.href ?? "")})`;
    return text;
  }
  if (node.type === "hsLink") {
    const refKind = String(node.attrs?.refKind ?? "unknown");
    const refValue = String(node.attrs?.refValue ?? "");
    const label = String(node.attrs?.label ?? "");
    return label && label !== refValue
      ? `[[${refKind}:${refValue}|${label}]]`
      : `[[${refKind}:${refValue}]]`;
  }
  if (node.type === "mention") return `@${String(node.attrs?.id ?? "")}`;
  if (node.type === "tagMention") return `#${String(node.attrs?.id ?? "")}`;
  if (node.type === "hardBreak") return "  \n";
  return (node.content ?? []).map(markdownInline).join("");
}

function markdownChildren(node: JSONContent): string {
  return (node.content ?? []).map(markdownInline).join("");
}

function markdownBlock(node: JSONContent, indent: string): string[] {
  switch (node.type) {
    case "heading": {
      const level = Number(node.attrs?.level ?? 1);
      return [`${"#".repeat(Math.min(6, Math.max(1, level)))} ${markdownChildren(node)}`];
    }
    case "paragraph":
      return [indent + markdownChildren(node)];
    case "blockquote":
      return (node.content ?? [])
        .flatMap((child) => markdownBlock(child, ""))
        .map((line) => `> ${line}`);
    case "bulletList":
      return (node.content ?? []).flatMap((item) => listItem(item, indent, "- "));
    case "orderedList": {
      let ordinal = Number(node.attrs?.start ?? 1);
      return (node.content ?? []).flatMap((item) => listItem(item, indent, `${ordinal++}. `));
    }
    case "taskList":
      return (node.content ?? []).flatMap((item) => {
        const checked = item.attrs?.checked === true;
        return listItem(item, indent, checked ? "- [x] " : "- [ ] ");
      });
    case "monacoCodeBlock": {
      const language = String(node.attrs?.language ?? "");
      const code = String(node.attrs?.code ?? "");
      return ["```" + language, ...code.split("\n"), "```"];
    }
    case "codeBlock": {
      const language = String(node.attrs?.language ?? "");
      const code = (node.content ?? []).map((child) => child.text ?? "").join("");
      return ["```" + language, ...code.split("\n"), "```"];
    }
    case "table": {
      const rows = (node.content ?? []).map((row) =>
        (row.content ?? []).map((cell) =>
          (cell.content ?? []).map(markdownChildren).join(" ").replace(/\|/g, "\\|"),
        ),
      );
      if (rows.length === 0) return [];
      const width = Math.max(...rows.map((row) => row.length));
      const line = (cells: string[]) =>
        `| ${Array.from({ length: width }, (_, i) => cells[i] ?? "").join(" | ")} |`;
      return [line(rows[0]), `|${" --- |".repeat(width)}`, ...rows.slice(1).map(line)];
    }
    case "horizontalRule":
      return ["---"];
    default:
      // Unknown blocks flatten to their inline text (lossy by design).
      return [indent + markdownChildren(node)];
  }
}

function listItem(item: JSONContent, indent: string, prefix: string): string[] {
  const lines: string[] = [];
  let first = true;
  for (const child of item.content ?? []) {
    const childLines = markdownBlock(child, "");
    for (const line of childLines) {
      if (first) {
        lines.push(`${indent}${prefix}${line}`);
        first = false;
      } else {
        lines.push(`${indent}${" ".repeat(prefix.length)}${line}`);
      }
    }
  }
  if (first) lines.push(`${indent}${prefix}`);
  return lines;
}

/**
 * Markdown projection — DELIBERATELY LOSSY (DEC-003: "markdown remains a
 * deliberately lossy extractor for interop"). See module header for drops.
 */
export function exportMarkdown(docJson: JSONContent): string {
  const blocks = (docJson.content ?? []).flatMap((node) => {
    const lines = markdownBlock(node, "");
    return lines.length > 0 ? [lines.join("\n")] : [];
  });
  return blocks.join("\n\n") + "\n";
}

/** Plain-text projection via the schema renderText serializers. */
export function exportPlainText(
  docJson: JSONContent,
  extensionOptions?: HandshakeEditorExtensionOptions,
): string {
  return generateText(docJson, buildExtensions(extensionOptions), {
    blockSeparator: "\n\n",
  });
}

export interface ProseMirrorJsonEnvelope {
  format: "handshake_rich_document";
  schema_version: string;
  exported_at_utc: string;
  authority: "projection_only:postgres_rich_document_is_authority";
  doc: JSONContent;
}

/** ProseMirror JSON projection in a typed envelope. */
export function exportProseMirrorJson(docJson: JSONContent, now?: () => Date): string {
  const envelope: ProseMirrorJsonEnvelope = {
    format: "handshake_rich_document",
    schema_version: WP009_RICH_DOCUMENT_SCHEMA_VERSION,
    exported_at_utc: (now?.() ?? new Date()).toISOString(),
    authority: "projection_only:postgres_rich_document_is_authority",
    doc: docJson,
  };
  return JSON.stringify(envelope, null, 2);
}

/**
 * Accepts a ProseMirror JSON export (envelope or bare doc) back as editor
 * document JSON. Throws a typed Error on anything that is not a doc.
 */
export function importProseMirrorJson(json: string): JSONContent {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch (error) {
    throw new Error(`not valid JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
  const candidate =
    typeof parsed === "object" && parsed !== null && "doc" in (parsed as Record<string, unknown>)
      ? (parsed as Record<string, unknown>).doc
      : parsed;
  if (
    typeof candidate !== "object" ||
    candidate === null ||
    (candidate as Record<string, unknown>).type !== "doc"
  ) {
    throw new Error("not a ProseMirror doc export (missing type: 'doc')");
  }
  return candidate as JSONContent;
}

/** Export filename: no blank spaces (naming policy), stable + readable. */
export function buildExportFilename(
  title: string | undefined,
  format: ExportFormatId,
  now?: () => Date,
): string {
  const descriptor = EXPORT_FORMATS.find((entry) => entry.id === format);
  const slug = (title ?? "handshake-document")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64) || "handshake-document";
  const stamp = (now?.() ?? new Date()).toISOString().replace(/[:.]/g, "-").slice(0, 19);
  const modeSuffix =
    format === "html_self_contained" ? "-self-contained" : format === "html_reference_linked" ? "-referenced" : "";
  return `${slug}${modeSuffix}-${stamp}.${descriptor?.extension ?? "txt"}`;
}
