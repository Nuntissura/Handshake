// WP-KERNEL-009 / MT-021 + MT-031 — Tiptap extension set for Loom documents.
//
// Single owner of the WP-009 rich-editor extension surface. All packages are
// lockfile-governed bundled libraries (@tiptap/* 3.13.0, MIT — see the
// runtime dependency allowlist, MT-017):
//   - StarterKit        core marks/nodes/history (+ Link + Underline in v3)
//   - TableKit          tables (table/row/header/cell)
//   - TaskList/TaskItem task lists with checkboxes
//   - Mention           @-mentions (people/agents)
//   - Tag mentions      #-tags via a renamed Mention instance
//   - Collaboration     opt-in Yjs/CRDT binding for the RichDocument authority
//                       schema (Master Spec §2.3.13.11 / §7.1.1.8)
//
// This module is also the owner of the WP-009 RichDocument schema-version
// surface. Per Master Spec §2.3.13.11, a RichDocument authority record is "a
// versioned ProseMirror/Tiptap document JSON authority record, with schema
// version, CRDT snapshot refs, EventLedger promotion refs, and projection
// refs", and §7.1.1.8 locks the authority layer to "the versioned RichDocument
// schema". The extension set declares which schema version it targets so a
// consumer (write-box, promotion, migration, projection rebuild) can stamp /
// assert RichDocument.schema_version against the editor that produced the JSON.
//
// Extension construction failures surface through the typed dependency-failure
// registry (MT-031) and degrade gracefully: a broken optional extension is
// skipped, the core editor still boots — never a blank document surface.

import type { AnyExtension } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { TableKit } from "@tiptap/extension-table";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import Mention from "@tiptap/extension-mention";
import Collaboration from "@tiptap/extension-collaboration";
import type { Doc as YDoc } from "yjs";
import {
  dependencyFailures,
  formatDependencyFailureMessage,
} from "../dependency_policy/dependency_failure";

export interface MentionItemsContext {
  query: string;
}

/**
 * Declared schema version of the WP-009 RichDocument JSON authority record this
 * extension set produces (Master Spec §2.3.13.11 — RichDocument carries a
 * schema version; §7.1.1.8 — the authority layer is the *versioned* RichDocument
 * schema). The catalog/write-box/promotion path stamps and asserts
 * `RichDocument.schema_version` with this value; a mismatch on load is a
 * schema-migration signal (§7.1.1.8 "schema migration" round-trip requirement).
 *
 * Bump this constant (and add a migration) whenever the node/mark schema set
 * below changes in a way that is not backward-compatible.
 */
export const WP009_RICH_DOCUMENT_SCHEMA_VERSION = "rich_document_v1" as const;

export type Wp009RichDocumentSchemaVersion =
  typeof WP009_RICH_DOCUMENT_SCHEMA_VERSION;

export interface Wp009ExtensionSetOptions {
  /** Suggestion source for @-mentions (people/agents). Defaults to none. */
  mentionItems?: (context: MentionItemsContext) => string[];
  /** Suggestion source for #-tags (knowledge-index tags). Defaults to none. */
  tagItems?: (context: MentionItemsContext) => string[];
  /**
   * Opt-in Yjs/CRDT document binding. When provided, the Collaboration
   * extension is wired to this doc and StarterKit's local undo/redo history is
   * disabled so the Yjs undo manager owns history (the documented Tiptap
   * collaboration pattern). Authority promotion still flows through
   * WriteBox/EventLedger (§2.3.13.11); this only EXPOSES the collaboration
   * binding contract on the extension set.
   */
  collaborationDocument?: YDoc;
}

/** Names of the WP-009-required schema nodes/marks used by tests and tooling. */
export const WP009_REQUIRED_NODE_NAMES = [
  "table",
  "tableRow",
  "tableHeader",
  "tableCell",
  "taskList",
  "taskItem",
  "mention",
  "tagMention",
] as const;

/**
 * Tiptap extension name of the collaboration binding. Used by tests/tooling to
 * assert the collaboration contract is present in a built set without depending
 * on the extension's internal identity.
 */
export const WP009_COLLABORATION_EXTENSION_NAME = "collaboration" as const;

type ExtensionFactory = { name: string; build: () => AnyExtension };

function buildFactories(options: Wp009ExtensionSetOptions): ExtensionFactory[] {
  const collaborationDoc = options.collaborationDocument;
  const factories: ExtensionFactory[] = [
    {
      name: "@tiptap/starter-kit",
      build: () =>
        StarterKit.configure({
          heading: { levels: [1, 2, 3] },
          link: { openOnClick: false },
          // Yjs owns history when collaborating; StarterKit's local undo/redo
          // would conflict with the CRDT undo manager (Tiptap collaboration
          // pattern).
          ...(collaborationDoc ? { undoRedo: false as const } : {}),
        }),
    },
    {
      name: "@tiptap/extension-table",
      build: () => TableKit.configure({ table: { resizable: false } }),
    },
    { name: "@tiptap/extension-list:TaskList", build: () => TaskList },
    {
      name: "@tiptap/extension-list:TaskItem",
      build: () => TaskItem.configure({ nested: true }),
    },
    {
      name: "@tiptap/extension-mention",
      build: () => {
        // Captured at build time: a faulty options object fails INSIDE the
        // guarded factory (typed dependency failure) instead of later at
        // first keystroke.
        const items = options.mentionItems;
        return Mention.configure({
          suggestion: {
            char: "@",
            items: ({ query }) => items?.({ query }) ?? [],
          },
        });
      },
    },
    {
      name: "@tiptap/extension-mention:tagMention",
      build: () => {
        const items = options.tagItems;
        return Mention.extend({ name: "tagMention" }).configure({
          suggestion: {
            char: "#",
            items: ({ query }) => items?.({ query }) ?? [],
          },
        });
      },
    },
  ];

  if (collaborationDoc) {
    factories.push({
      name: "@tiptap/extension-collaboration",
      build: () => Collaboration.configure({ document: collaborationDoc }),
    });
  }

  return factories;
}

/**
 * Builds the full WP-009 extension list. A factory that throws is reported as
 * a typed dependency failure and skipped, so one broken extension cannot blank
 * the whole editor. StarterKit itself failing is fatal (rethrown) — without
 * the document core there is nothing to degrade to.
 */
export function buildWp009ExtensionSet(
  options: Wp009ExtensionSetOptions = {},
): AnyExtension[] {
  const extensions: AnyExtension[] = [];
  for (const factory of buildFactories(options)) {
    try {
      extensions.push(factory.build());
    } catch (error) {
      const failure = {
        dependency: factory.name,
        component: `extension:${factory.name}`,
        phase: "extension_init" as const,
        cause: error instanceof Error ? error.message : String(error),
      };
      dependencyFailures.report({
        ...failure,
        message: formatDependencyFailureMessage(failure),
      });
      if (factory.name === "@tiptap/starter-kit") throw error;
    }
  }
  return extensions;
}

/**
 * Builds the WP-009 extension set with the Yjs/CRDT collaboration binding wired
 * to `ydoc`. This EXPOSES the collaboration contract of the rich editor — the
 * Collaboration extension is bound to the provided Yjs document so a consumer
 * (provider wiring, write-box, EventLedger promotion) can drive real-time
 * editing — rather than only locking the @tiptap/extension-collaboration
 * dependency. Authority promotion still flows through WriteBox/EventLedger
 * (Master Spec §2.3.13.11); this builder owns the editor-side binding only.
 *
 * Convenience over `buildWp009ExtensionSet({ collaborationDocument: ydoc })`.
 */
export function buildWp009CollaborativeExtensionSet(
  ydoc: YDoc,
  options: Omit<Wp009ExtensionSetOptions, "collaborationDocument"> = {},
): AnyExtension[] {
  return buildWp009ExtensionSet({ ...options, collaborationDocument: ydoc });
}

/**
 * Reports the RichDocument schema version a built extension set targets so a
 * consumer/test can read the version the editor was constructed for and stamp /
 * assert `RichDocument.schema_version` (Master Spec §2.3.13.11). The version is
 * a property of the schema contract this module owns, not of any individual
 * extension instance; the built set is accepted to bind the assertion to a
 * concrete, instantiated editor surface rather than a bare constant.
 *
 * Throws if handed an empty set — a versioned RichDocument schema requires the
 * extensions that define that schema, so a versionless / extensionless set is a
 * contract error rather than a silently-stamped document.
 */
export function richDocumentSchemaVersionOf(
  extensions: readonly AnyExtension[],
): Wp009RichDocumentSchemaVersion {
  if (extensions.length === 0) {
    throw new Error(
      "richDocumentSchemaVersionOf: empty extension set has no RichDocument schema to version",
    );
  }
  return WP009_RICH_DOCUMENT_SCHEMA_VERSION;
}
