// WP-KERNEL-009 / MT-161 — TiptapExtensionInventory.
//
// The single machine-readable inventory of every Tiptap extension, custom node,
// custom mark, and embedded Monaco surface the Handshake rich-document editor
// (the operator's VS Code-editing replacement) must ship. This is the
// restartable contract the rest of TiptapMonacoIntegration (MT-162..MT-176)
// builds and asserts against: a no-context model can read this file to learn
// exactly which editor primitives exist, which backend authority surface each
// one persists into, and which MT owns the implementation.
//
// Authority note (Master Spec §2.3.13.11 / §7.1.1.8): the durable authority is
// the versioned RichDocument JSON record in PostgreSQL/EventLedger (loaded/saved
// via /knowledge/documents — app/src/lib/api.ts rich-doc client). This inventory
// is a TYPED PRODUCT CONSTANT, not an authority record: it declares the editor
// schema surface so tooling/tests can verify the editor produces the node/mark
// set the backend block-tree (18 block kinds incl. typed links +
// knowledge_editor_code_nodes) expects. It deliberately carries NO sidecar
// research-paper authority — every entry is a concrete Handshake-native node.
//
// Nothing here mounts an editor or touches the network; it is pure data + types
// so it is unit-testable in jsdom without the Tiptap/Monaco runtime.

import {
  WP009_REQUIRED_NODE_NAMES,
  WP009_COLLABORATION_EXTENSION_NAME,
  WP009_RICH_DOCUMENT_SCHEMA_VERSION,
} from "../tiptap/extension_set";

/** Which engine owns a given editor primitive. */
export type EditorEngine = "tiptap" | "monaco";

/** The kind of ProseMirror/Tiptap schema entity (or a Monaco surface). */
export type EditorPrimitiveKind = "node" | "mark" | "extension" | "monaco_surface";

/**
 * One required editor primitive. `backendBlockKind` ties the editor node to the
 * backend block-tree kind it serializes into (RichDocBlock.kind in
 * app/src/lib/api.ts); `null` means the primitive is editor-only (a mark, a
 * behavior extension, or a Monaco worker surface) with no 1:1 block kind.
 */
export interface EditorPrimitive {
  /** Stable schema/extension name (matches the Tiptap extension `name`). */
  readonly name: string;
  readonly engine: EditorEngine;
  readonly kind: EditorPrimitiveKind;
  /** Human-facing one-line purpose (operator/no-context-model readable). */
  readonly purpose: string;
  /**
   * Backend block-tree kind this primitive persists into, or null when the
   * primitive is editor-only. Used by the persistence bridge (MT-168) and the
   * round-trip tests to assert the editor↔backend mapping.
   */
  readonly backendBlockKind: string | null;
  /** The MT id that implements/activates this primitive. */
  readonly implementedBy: string;
  /** True when a broken init must NOT blank the editor (degrade gracefully). */
  readonly degradesGracefully: boolean;
}

/**
 * Typed link node descriptor for the [[kind:value]] wikilink family
 * (MT-163). Each entry maps a wikilink prefix to the backend ref kind the
 * resolved typed link node carries. Mirrors the link syntax in the
 * hs_rich_text_editor stub field of the WP-KERNEL-009 contract.
 */
export interface WikilinkKindDescriptor {
  /** The [[PREFIX:...]] token, e.g. "note", "file", "wp". */
  readonly prefix: string;
  /** Backend ref kind / link_kind the typed node resolves to. */
  readonly backendRefKind: string;
  /** One-line description of what the link targets. */
  readonly targets: string;
}

/**
 * The wikilink kinds the editor must recognize and turn into typed link nodes
 * (MT-163). Order is the canonical scan order; longest-match is not required
 * because prefixes are disjoint up to the first ":".
 */
export const WP009_WIKILINK_KINDS: readonly WikilinkKindDescriptor[] = [
  { prefix: "note", backendRefKind: "note", targets: "another RichDocument / knowledge note" },
  { prefix: "file", backendRefKind: "file", targets: "a workspace file path" },
  { prefix: "folder", backendRefKind: "folder", targets: "a workspace folder path" },
  { prefix: "project", backendRefKind: "project", targets: "a project entity" },
  { prefix: "spec", backendRefKind: "spec", targets: "a spec section / anchor" },
  { prefix: "wp", backendRefKind: "wp", targets: "a work packet" },
  { prefix: "symbol", backendRefKind: "symbol", targets: "a code symbol" },
  { prefix: "album", backendRefKind: "album", targets: "a media album" },
  { prefix: "video", backendRefKind: "video", targets: "a video media asset" },
  { prefix: "HS_images", backendRefKind: "images", targets: "an image set embed" },
  { prefix: "HS_slideshow", backendRefKind: "slideshow", targets: "a slideshow embed" },
] as const;

/** Fast lookup of a wikilink descriptor by prefix (lower-cased). */
export const WP009_WIKILINK_KIND_BY_PREFIX: ReadonlyMap<string, WikilinkKindDescriptor> =
  new Map(WP009_WIKILINK_KINDS.map((k) => [k.prefix.toLowerCase(), k]));

/**
 * The full required editor primitive inventory. Downstream MTs assert their
 * primitive is present here (so the inventory cannot silently drift from the
 * implementation), and the round-trip tests walk it to verify backend block
 * coverage.
 */
export const WP009_EDITOR_PRIMITIVES: readonly EditorPrimitive[] = [
  // --- Core prose (StarterKit) ---
  { name: "paragraph", engine: "tiptap", kind: "node", purpose: "Body paragraph", backendBlockKind: "paragraph", implementedBy: "MT-161", degradesGracefully: false },
  { name: "heading", engine: "tiptap", kind: "node", purpose: "Headings (h1-h3)", backendBlockKind: "heading", implementedBy: "MT-161", degradesGracefully: false },
  { name: "bulletList", engine: "tiptap", kind: "node", purpose: "Bulleted list", backendBlockKind: "bullet_list", implementedBy: "MT-161", degradesGracefully: false },
  { name: "orderedList", engine: "tiptap", kind: "node", purpose: "Numbered list", backendBlockKind: "ordered_list", implementedBy: "MT-161", degradesGracefully: false },
  { name: "blockquote", engine: "tiptap", kind: "node", purpose: "Block quote", backendBlockKind: "quote", implementedBy: "MT-161", degradesGracefully: false },
  { name: "bold", engine: "tiptap", kind: "mark", purpose: "Bold inline mark", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: false },
  { name: "italic", engine: "tiptap", kind: "mark", purpose: "Italic inline mark", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: false },
  { name: "link", engine: "tiptap", kind: "mark", purpose: "Plain hyperlink mark", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: false },
  // --- Tables / task lists (TableKit + list kit) ---
  { name: "table", engine: "tiptap", kind: "node", purpose: "Table", backendBlockKind: "table", implementedBy: "MT-161", degradesGracefully: true },
  { name: "tableRow", engine: "tiptap", kind: "node", purpose: "Table row", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: true },
  { name: "tableHeader", engine: "tiptap", kind: "node", purpose: "Table header cell", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: true },
  { name: "tableCell", engine: "tiptap", kind: "node", purpose: "Table body cell", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: true },
  { name: "taskList", engine: "tiptap", kind: "node", purpose: "Task list (checkboxes)", backendBlockKind: "task_list", implementedBy: "MT-161", degradesGracefully: true },
  { name: "taskItem", engine: "tiptap", kind: "node", purpose: "Task list item", backendBlockKind: "task_item", implementedBy: "MT-161", degradesGracefully: true },
  // --- Mentions / tags ---
  { name: "mention", engine: "tiptap", kind: "node", purpose: "@-mention (people/agents)", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: true },
  { name: "tagMention", engine: "tiptap", kind: "node", purpose: "#-tag (knowledge index)", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: true },
  // --- Collaboration binding ---
  { name: WP009_COLLABORATION_EXTENSION_NAME, engine: "tiptap", kind: "extension", purpose: "Yjs/CRDT collaboration binding", backendBlockKind: null, implementedBy: "MT-161", degradesGracefully: true },
  // --- Typed link node (MT-163) ---
  { name: "hsLink", engine: "tiptap", kind: "node", purpose: "Typed [[kind:value]] wikilink resolving to a backend entity", backendBlockKind: "link", implementedBy: "MT-163", degradesGracefully: true },
  // --- Embedded Monaco code block (MT-165) ---
  { name: "monacoCodeBlock", engine: "tiptap", kind: "node", purpose: "Code block whose NodeView mounts a Monaco editor (language id + raw text + round-trip hash)", backendBlockKind: "code", implementedBy: "MT-165", degradesGracefully: true },
  // --- Monaco surfaces (MT-166/MT-167) ---
  { name: "monaco:languages", engine: "monaco", kind: "monaco_surface", purpose: "Registered language ids + detection for code blocks", backendBlockKind: null, implementedBy: "MT-166", degradesGracefully: true },
  { name: "monaco:workers", engine: "monaco", kind: "monaco_surface", purpose: "Locally bundled Monaco web workers (no CDN, offline)", backendBlockKind: null, implementedBy: "MT-167", degradesGracefully: true },
] as const;

/** Names that StarterKit must provide for the editor to boot at all. */
export const WP009_FATAL_PRIMITIVE_NAMES: readonly string[] = [
  "paragraph",
  "heading",
];

/** All primitive names (for fast membership checks in tests/tooling). */
export const WP009_EDITOR_PRIMITIVE_NAMES: readonly string[] =
  WP009_EDITOR_PRIMITIVES.map((p) => p.name);

/** Returns the inventory entry for a primitive name, or undefined. */
export function editorPrimitive(name: string): EditorPrimitive | undefined {
  return WP009_EDITOR_PRIMITIVES.find((p) => p.name === name);
}

/**
 * The set of backend block kinds the editor is responsible for producing. Used
 * by MT-168 / MT-176 to assert the editor↔backend block-kind coverage contract.
 */
export const WP009_EDITOR_BACKEND_BLOCK_KINDS: readonly string[] = Array.from(
  new Set(
    WP009_EDITOR_PRIMITIVES.map((p) => p.backendBlockKind).filter(
      (k): k is string => k !== null,
    ),
  ),
);

/**
 * Validates the inventory against the lower-level extension_set contract
 * (MT-021) so the two cannot drift: every schema node the extension set
 * declares required must appear in this inventory. Returns the list of missing
 * names (empty == consistent). This is a pure check used by the MT-161 test.
 */
export function inventoryMissingRequiredNodes(): string[] {
  const names = new Set(WP009_EDITOR_PRIMITIVE_NAMES);
  return WP009_REQUIRED_NODE_NAMES.filter((n) => !names.has(n));
}

/**
 * The schema version the inventory targets. Re-exported so consumers depend on
 * one place; the value itself is owned by the extension set (MT-021/MT-162).
 */
export const WP009_INVENTORY_SCHEMA_VERSION = WP009_RICH_DOCUMENT_SCHEMA_VERSION;
