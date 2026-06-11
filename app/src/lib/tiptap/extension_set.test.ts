// WP-KERNEL-009 / MT-021 — Tiptap extension set smoke test (jsdom).
//
// Instantiates a REAL @tiptap/core Editor with the full WP-009 extension set
// and proves the schema carries every required node (tables, task lists,
// mentions, tags) plus content round-trips through the table/task-list paths.

import { Editor } from "@tiptap/core";
import { afterEach, describe, expect, it } from "vitest";
import {
  WP009_REQUIRED_NODE_NAMES,
  buildWp009ExtensionSet,
} from "./extension_set";

let editor: Editor | null = null;

afterEach(() => {
  editor?.destroy();
  editor = null;
});

function createEditor(): Editor {
  editor = new Editor({
    element: document.createElement("div"),
    extensions: buildWp009ExtensionSet({
      mentionItems: ({ query }) =>
        ["kernel-builder", "kernel-validator"].filter((m) => m.includes(query)),
      tagItems: ({ query }) => ["wp-009", "loom"].filter((t) => t.includes(query)),
    }),
  });
  return editor;
}

describe("MT-021 tiptap extension set", () => {
  it("instantiates the editor with every WP-009-required schema node", () => {
    const ed = createEditor();
    for (const nodeName of WP009_REQUIRED_NODE_NAMES) {
      expect(ed.schema.nodes[nodeName], `schema node ${nodeName} missing`).toBeDefined();
    }
    // Link mark ships via StarterKit in tiptap v3.
    expect(ed.schema.marks.link).toBeDefined();
  });

  it("round-trips a table through setContent/getJSON", () => {
    const ed = createEditor();
    ed.commands.setContent({
      type: "doc",
      content: [
        {
          type: "table",
          content: [
            {
              type: "tableRow",
              content: [
                {
                  type: "tableHeader",
                  content: [{ type: "paragraph", content: [{ type: "text", text: "ID" }] }],
                },
                {
                  type: "tableCell",
                  content: [{ type: "paragraph", content: [{ type: "text", text: "MT-021" }] }],
                },
              ],
            },
          ],
        },
      ],
    });
    const json = ed.getJSON();
    expect(json.content?.[0]?.type).toBe("table");
    expect(JSON.stringify(json)).toContain("MT-021");
  });

  it("inserts and toggles a task list", () => {
    const ed = createEditor();
    ed.commands.setContent({
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "ship MT-021" }] }],
    });
    ed.commands.selectAll();
    const toggled = ed.commands.toggleTaskList();
    expect(toggled).toBe(true);
    expect(ed.getJSON().content?.[0]?.type).toBe("taskList");
  });

  it("inserts mention and tag nodes with independent names", () => {
    const ed = createEditor();
    ed.commands.insertContent([
      { type: "mention", attrs: { id: "kernel-builder" } },
      { type: "text", text: " " },
      { type: "tagMention", attrs: { id: "wp-009" } },
    ]);
    const serialized = JSON.stringify(ed.getJSON());
    expect(serialized).toContain('"mention"');
    expect(serialized).toContain('"tagMention"');
    expect(serialized).toContain("kernel-builder");
    expect(serialized).toContain("wp-009");
  });

  it("degrades gracefully when an optional extension factory fails (MT-031 path)", async () => {
    const { dependencyFailures } = await import("../dependency_policy/dependency_failure");
    dependencyFailures.clear();
    // Force the tag-mention factory to fail by making tagItems a poisoned
    // getter object that throws during configure.
    const broken = {
      get tagItems(): never {
        throw new Error("simulated extension construction failure");
      },
    };
    const extensions = buildWp009ExtensionSet(broken as never);
    // The poisoned factory was skipped; the core set still built.
    expect(extensions.length).toBeGreaterThanOrEqual(4);
    const failures = dependencyFailures.list();
    expect(failures.length).toBeGreaterThanOrEqual(1);
    expect(failures[0].phase).toBe("extension_init");
    expect(failures[0].message).toContain("Bundled dependency failed to load");
    dependencyFailures.clear();
  });
});
