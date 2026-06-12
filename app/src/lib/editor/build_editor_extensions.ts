// WP-KERNEL-009 / MT-169 (+161) — the full Handshake rich-editor extension set.
//
// Composes the foundation extension set (extension_set.ts: StarterKit, tables,
// task lists, mentions, tags, optional collaboration) with the WP-009 custom
// nodes/rules built in this group:
//   - HsLinkNode (MT-163): typed [[kind:value]] wikilinks,
//   - MonacoCodeBlockNode (MT-165): embedded Monaco code blocks,
//   - AutoCodeBlockRules (MT-164): prose->code conversion.
//
// This is the single extension list the integrated editor component
// (RichTextEditor) mounts, so the toolbar/keyboard/command-palette (MT-169/170)
// all operate over one consistent schema. Construction stays guarded: a broken
// optional extension is reported as a typed dependency failure and skipped (the
// extension_set foundation already does this for its set), never blanking the
// editor.

import type { AnyExtension } from "@tiptap/core";
import type { Doc as YDoc } from "yjs";
import {
  buildWp009ExtensionSet,
  type Wp009ExtensionSetOptions,
} from "../tiptap/extension_set";
import { HsLinkNode } from "../tiptap/hs_link_node";
import { MonacoCodeBlockNode } from "../tiptap/monaco_code_block_node";
import { AutoCodeBlockRules } from "../tiptap/auto_code_block_rules";

export interface HandshakeEditorExtensionOptions extends Wp009ExtensionSetOptions {
  /** Opt-in Yjs/CRDT doc (forwarded to the foundation collaboration binding). */
  collaborationDocument?: YDoc;
}

/**
 * Builds the complete Handshake rich-editor extension list (foundation set +
 * WP-009 custom nodes/rules). Order matters: the foundation set first (so
 * StarterKit's schema is present), then the custom block nodes, then the
 * behavior extension (auto-code-block rules) that references those nodes.
 */
export function buildHandshakeEditorExtensions(
  options: HandshakeEditorExtensionOptions = {},
): AnyExtension[] {
  return [
    ...buildWp009ExtensionSet(options),
    HsLinkNode,
    MonacoCodeBlockNode,
    AutoCodeBlockRules,
  ];
}

/** Names of the WP-009 custom nodes/rules added on top of the foundation set. */
export const HANDSHAKE_EDITOR_CUSTOM_EXTENSION_NAMES = [
  "hsLink",
  "monacoCodeBlock",
  "autoCodeBlockRules",
] as const;
