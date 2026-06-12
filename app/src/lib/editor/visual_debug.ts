// WP-KERNEL-009 / MT-172 — EditorVisualDebugSelectors (debug payload).
//
// A machine-readable snapshot of editor state for a no-context model / the
// Playwright visual matrix to assert against without screen-scraping (GLOBAL-
// BUILD-DIAG / HBR-VIS). Pairs with the stable data-testid selectors the editor
// components expose (rich-text-editor, -toolbar, editor-cmd-<id>,
// monaco-code-block, hs-link, editor-command-palette, …). The payload reports
// what nodes/marks exist, the embedded code blocks (language + round-trip hash),
// the typed links, and the current selection — enough to drive deterministic
// visual-debug assertions.
//
// Pure over a minimal editor shape so it is unit-testable; the components publish
// it on window for the visual lane (see RichTextEditor debug wiring).

export interface CodeBlockDebugInfo {
  language: string;
  contentHash: string;
  codeLength: number;
}

export interface LinkDebugInfo {
  refKind: string;
  refValue: string;
  resolved: boolean;
}

export interface EditorDebugSnapshot {
  /** Counts of each node type present in the document. */
  nodeCounts: Record<string, number>;
  /** Embedded Monaco code blocks. */
  codeBlocks: CodeBlockDebugInfo[];
  /** Typed wikilinks. */
  links: LinkDebugInfo[];
  /** Current selection range (UTF-8-ish char offsets from the model). */
  selection: { from: number; to: number; empty: boolean };
  /** Whether the editor is editable. */
  editable: boolean;
}

/** The minimal editor shape the snapshot needs (decouples from Tiptap types). */
export interface DebuggableEditor {
  isEditable: boolean;
  state: {
    selection: { from: number; to: number; empty: boolean };
    doc: {
      descendants(fn: (node: DebuggableNode) => boolean | void): void;
    };
  };
}

export interface DebuggableNode {
  type: { name: string };
  attrs: Record<string, unknown>;
}

/**
 * Walks the document and builds a machine-readable debug snapshot. Deterministic
 * for a given document + selection.
 */
export function buildEditorDebugSnapshot(editor: DebuggableEditor): EditorDebugSnapshot {
  const nodeCounts: Record<string, number> = {};
  const codeBlocks: CodeBlockDebugInfo[] = [];
  const links: LinkDebugInfo[] = [];

  editor.state.doc.descendants((node) => {
    const name = node.type.name;
    nodeCounts[name] = (nodeCounts[name] ?? 0) + 1;
    if (name === "monacoCodeBlock") {
      codeBlocks.push({
        language: String(node.attrs.language ?? ""),
        contentHash: String(node.attrs.contentHash ?? ""),
        codeLength: String(node.attrs.code ?? "").length,
      });
    } else if (name === "hsLink") {
      links.push({
        refKind: String(node.attrs.refKind ?? ""),
        refValue: String(node.attrs.refValue ?? ""),
        resolved: node.attrs.resolved !== false,
      });
    }
    return true;
  });

  const { from, to, empty } = editor.state.selection;
  return {
    nodeCounts,
    codeBlocks,
    links,
    selection: { from, to, empty },
    editable: editor.isEditable,
  };
}

/** The stable global key the editor publishes its debug snapshot under. */
export const EDITOR_DEBUG_GLOBAL_KEY = "__HS_EDITOR_DEBUG__" as const;

/**
 * The stable global key the editor publishes its LAST EXPORT result under
 * (MT-244 save-to-format): { formatId, filename, bytes, embedErrors,
 * inlineSkips } — lets the visual lane assert export outcomes without
 * screen-scraping.
 */
export const EDITOR_LAST_EXPORT_GLOBAL_KEY = "__HS_EDITOR_LAST_EXPORT__" as const;

/** The canonical set of stable selectors the editor surfaces expose (for docs/tests). */
export const EDITOR_STABLE_SELECTORS = [
  "rich-text-editor",
  "rich-text-editor-toolbar",
  "rich-text-editor-surface",
  "editor-open-palette",
  "editor-command-palette",
  "editor-command-palette-input",
  "monaco-code-block",
  "monaco-code-block-language",
  "monaco-code-block-host",
  "hs-link",
  "rich-text-editor-backend-error",
] as const;
