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
//
// Extension construction failures surface through the typed dependency-failure
// registry (MT-031) and degrade gracefully: a broken optional extension is
// skipped, the core editor still boots — never a blank document surface.

import type { AnyExtension } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { TableKit } from "@tiptap/extension-table";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import Mention from "@tiptap/extension-mention";
import {
  dependencyFailures,
  formatDependencyFailureMessage,
} from "../dependency_policy/dependency_failure";

export interface MentionItemsContext {
  query: string;
}

export interface Wp009ExtensionSetOptions {
  /** Suggestion source for @-mentions (people/agents). Defaults to none. */
  mentionItems?: (context: MentionItemsContext) => string[];
  /** Suggestion source for #-tags (knowledge-index tags). Defaults to none. */
  tagItems?: (context: MentionItemsContext) => string[];
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

type ExtensionFactory = { name: string; build: () => AnyExtension };

function buildFactories(options: Wp009ExtensionSetOptions): ExtensionFactory[] {
  return [
    {
      name: "@tiptap/starter-kit",
      build: () =>
        StarterKit.configure({
          heading: { levels: [1, 2, 3] },
          link: { openOnClick: false },
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
