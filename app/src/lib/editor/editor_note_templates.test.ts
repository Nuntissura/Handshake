// WP-KERNEL-009 / MT-258 - note-template expansion and insertion tests.

import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import { buildHandshakeEditorExtensions } from "./build_editor_extensions";
import { EDITOR_COMMAND_BY_ID, filterEditorCommands } from "./editor_commands";
import {
  NOTE_TEMPLATES,
  expandNoteTemplateText,
  formatNoteTemplateDate,
  insertNoteTemplate,
  renderNoteTemplateContent,
} from "./editor_note_templates";

function makeEditor(): Editor {
  return new Editor({
    extensions: buildHandshakeEditorExtensions(),
    content: {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "replace me" }] }],
    },
  });
}

describe("editor note templates (MT-258)", () => {
  it("defines a command-reachable data template with date/title/cursor variables", () => {
    expect(NOTE_TEMPLATES.map((template) => template.id)).toContain("note.daily");
    expect(EDITOR_COMMAND_BY_ID.has("template.note.daily")).toBe(true);
    expect(filterEditorCommands("daily template").map((cmd) => cmd.id)).toContain("template.note.daily");

    const date = new Date(2026, 5, 16);
    expect(formatNoteTemplateDate(date)).toBe("2026-06-16");
    expect(expandNoteTemplateText("{{title}} / {{date}} / {{cursor}}", {
      title: "MT-258",
      now: date,
    })).toEqual({ text: "MT-258 / 2026-06-16 / ", cursorOffset: 22 });
  });

  it("renders structured ProseMirror content without leaking cursor markers", () => {
    const rendered = renderNoteTemplateContent("note.daily", {
      title: "MT-258 Plan",
      now: new Date(2026, 5, 16),
    });

    expect(rendered?.[0]).toEqual({
      type: "heading",
      attrs: { level: 1 },
      content: [{ type: "text", text: "MT-258 Plan" }],
    });
    expect(rendered?.[1]).toEqual({
      type: "paragraph",
      content: [{ type: "text", text: "Date: 2026-06-16" }],
    });
    expect(JSON.stringify(rendered)).not.toContain("HS_TEMPLATE_CURSOR");
  });

  it("inserts the template as real nodes and places the cursor at the cursor variable", () => {
    const editor = makeEditor();
    editor.commands.selectAll();

    expect(insertNoteTemplate(editor, "note.daily", {
      title: "MT-258 Daily",
      now: new Date(2026, 5, 16),
    })).toBe(true);

    const json = editor.getJSON();
    expect(json.content?.[0]?.type).toBe("heading");
    expect((json.content?.[0]?.content?.[0] as { text?: string } | undefined)?.text).toBe(
      "MT-258 Daily",
    );
    expect((json.content?.[1]?.content?.[0] as { text?: string } | undefined)?.text).toBe(
      "Date: 2026-06-16",
    );
    expect(editor.state.doc.textContent).toContain("Notes");
    expect(editor.state.doc.textContent).not.toContain("HS_TEMPLATE_CURSOR");
    expect(editor.state.selection.empty).toBe(true);
    expect(editor.state.selection.$from.parent.type.name).toBe("paragraph");
    expect(editor.state.selection.$from.parent.textContent).toBe("");
    editor.destroy();
  });

  it("declines unknown templates without mutating the editor", () => {
    const editor = makeEditor();
    const before = editor.getJSON();

    expect(insertNoteTemplate(editor, "missing.template")).toBe(false);
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });
});
