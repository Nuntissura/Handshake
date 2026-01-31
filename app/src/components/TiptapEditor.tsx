import { useEffect, useState } from "react";
import { EditorContent, JSONContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";

type TiptapEditorProps = {
  initialContent: JSONContent | null;
  onChange: (doc: JSONContent) => void;
  readOnly?: boolean;
  onSelectionChange?: (text: string) => void;
};

export function TiptapEditor({
  initialContent,
  onChange,
  readOnly = false,
  onSelectionChange,
}: TiptapEditorProps) {
  const [, forceSelectionRefresh] = useState(0);
  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        heading: { levels: [1, 2, 3] },
      }),
    ],
    content: initialContent ?? { type: "doc", content: [{ type: "paragraph" }] },
    editable: !readOnly,
    onUpdate: ({ editor }) => {
      onChange(editor.getJSON());
    },
  });

  // Refresh editor content when upstream initialContent changes (e.g., switching documents).
  useEffect(() => {
    if (!editor) return;
    if (initialContent) {
      editor.commands.setContent(initialContent, { emitUpdate: false });
    } else {
      editor.commands.setContent({ type: "doc", content: [{ type: "paragraph" }] }, { emitUpdate: false });
    }
  }, [editor, initialContent]);

  useEffect(() => {
    if (!editor) return;
    const refreshHandler = () => forceSelectionRefresh((tick) => tick + 1);
    const selectionHandler = () => {
      refreshHandler();
      if (!onSelectionChange) return;
      const { from, to } = editor.state.selection;
      onSelectionChange(editor.state.doc.textBetween(from, to, "\n"));
    };

    editor.on("selectionUpdate", selectionHandler);
    editor.on("transaction", refreshHandler);
    return () => {
      editor.off("selectionUpdate", selectionHandler);
      editor.off("transaction", refreshHandler);
    };
  }, [editor, onSelectionChange]);

  if (!editor) return null;

  const toggleList = (listType: "bulletList" | "orderedList") => {
    const { state } = editor;
    const { $from } = state.selection;
    const parent = $from.parent;
    const parentIsEmptyParagraph = parent.type.name === "paragraph" && parent.content.size === 0;
    const docIsEmpty =
      state.doc.childCount === 1 &&
      state.doc.firstChild?.type.name === "paragraph" &&
      state.doc.firstChild.content.size === 0;

    const chain = editor.chain().focus();

    // If we're on an empty paragraph (or empty doc), toggling should still create a list item ready to type.
    if (parentIsEmptyParagraph || docIsEmpty) {
      chain.setParagraph(); // ensure we start from a paragraph before toggling
    }

    if (listType === "bulletList") {
      chain.toggleBulletList().run();
    } else {
      chain.toggleOrderedList().run();
    }
  };

  const mkButton = (label: string, handler: () => void, isActive: boolean, disabled = false) => (
    <button
      type="button"
      className={isActive ? "tt-button tt-button--active" : "tt-button"}
      onClick={handler}
      disabled={disabled}
    >
      {label}
    </button>
  );

  return (
    <div className="tiptap-editor">
      <div className="tiptap-toolbar tiptap-toolbar--sticky">
        {mkButton(
          "Bold",
          () => editor.chain().focus().toggleBold().run(),
          editor.isActive("bold"),
          readOnly
        )}
        {mkButton(
          "Italic",
          () => editor.chain().focus().toggleItalic().run(),
          editor.isActive("italic"),
          readOnly
        )}
        {mkButton(
          "H1",
          () => editor.chain().focus().toggleHeading({ level: 1 }).run(),
          editor.isActive("heading", { level: 1 }),
          readOnly
        )}
        {mkButton(
          "H2",
          () => editor.chain().focus().toggleHeading({ level: 2 }).run(),
          editor.isActive("heading", { level: 2 }),
          readOnly
        )}
        {mkButton(
          "H3",
          () => editor.chain().focus().toggleHeading({ level: 3 }).run(),
          editor.isActive("heading", { level: 3 }),
          readOnly
        )}
        {mkButton(
          "Paragraph",
          () => editor.chain().focus().setParagraph().run(),
          editor.isActive("paragraph"),
          readOnly
        )}
        {mkButton(
          "Bullet List",
          () => toggleList("bulletList"),
          editor.isActive("bulletList"),
          readOnly
        )}
        {mkButton(
          "Numbered List",
          () => toggleList("orderedList"),
          editor.isActive("orderedList"),
          readOnly
        )}
        {mkButton(
          "Code Block",
          () => editor.chain().focus().toggleCodeBlock().run(),
          editor.isActive("codeBlock"),
          readOnly
        )}
        {mkButton(
          "Block Quote",
          () => editor.chain().focus().toggleBlockquote().run(),
          editor.isActive("blockquote"),
          readOnly
        )}
      </div>
      <div className="tiptap-surface tiptap-scroll">
        <EditorContent editor={editor} />
      </div>
    </div>
  );
}
