// WP-KERNEL-009 / MT-021 — RichDocument schema-version + collaboration-hook
// contract test (jsdom). FAIL_V1 remediation: the validator found the extension
// set only LOCKED @tiptap/extension-collaboration without exposing the
// schema-version surface or a collaboration-hook binding. These tests prove the
// set EXPOSES both contracts:
//   1. a declared RichDocument schema version (Master Spec §2.3.13.11 — a
//      RichDocument carries a schema version; §7.1.1.8 — the authority layer is
//      the *versioned* RichDocument schema), readable off a built set; and
//   2. a Yjs/CRDT collaboration binding — the Collaboration extension is present
//      in the set, bound to a provided Yjs doc, and drives a real editor —
//      rather than the dependency merely being installed.

import { Editor } from "@tiptap/core";
import { Doc as YDoc, XmlFragment } from "yjs";
import { afterEach, describe, expect, it } from "vitest";
import {
  WP009_COLLABORATION_EXTENSION_NAME,
  WP009_RICH_DOCUMENT_SCHEMA_VERSION,
  buildWp009CollaborativeExtensionSet,
  buildWp009ExtensionSet,
  richDocumentSchemaVersionOf,
} from "./extension_set";

let editor: Editor | null = null;

afterEach(() => {
  editor?.destroy();
  editor = null;
});

function collaborativeEditor(ydoc: YDoc): Editor {
  editor = new Editor({
    element: document.createElement("div"),
    extensions: buildWp009CollaborativeExtensionSet(ydoc, {
      mentionItems: ({ query }) => ["kernel-builder"].filter((m) => m.includes(query)),
      tagItems: ({ query }) => ["wp-009"].filter((t) => t.includes(query)),
    }),
  });
  return editor;
}

describe("MT-021 RichDocument schema-version surface", () => {
  it("declares a stable RichDocument schema version constant (§2.3.13.11)", () => {
    expect(WP009_RICH_DOCUMENT_SCHEMA_VERSION).toBe("rich_document_v1");
  });

  it("exposes the schema version a built extension set targets", () => {
    // Read the schema version off a concrete, instantiated editor surface —
    // the version a consumer would stamp onto RichDocument.schema_version.
    const extensions = buildWp009ExtensionSet();
    expect(richDocumentSchemaVersionOf(extensions)).toBe(
      WP009_RICH_DOCUMENT_SCHEMA_VERSION,
    );

    editor = new Editor({
      element: document.createElement("div"),
      extensions,
    });
    expect(richDocumentSchemaVersionOf(editor.extensionManager.extensions)).toBe(
      "rich_document_v1",
    );
  });
});

describe("MT-021 collaboration-hook contract", () => {
  it("does NOT wire collaboration into the base (non-collaborative) set", () => {
    const names = buildWp009ExtensionSet().map((extension) => extension.name);
    expect(names).not.toContain(WP009_COLLABORATION_EXTENSION_NAME);
  });

  it("wires the Collaboration extension when built collaboratively", () => {
    const ydoc = new YDoc();
    const names = buildWp009CollaborativeExtensionSet(ydoc).map(
      (extension) => extension.name,
    );
    expect(names).toContain(WP009_COLLABORATION_EXTENSION_NAME);
  });

  it("binds the Collaboration extension to the provided Yjs doc and drives a real editor", () => {
    const ydoc = new YDoc();
    const ed = collaborativeEditor(ydoc);

    // The collaboration extension is live in the instantiated editor.
    const liveNames = ed.extensionManager.extensions.map((e) => e.name);
    expect(liveNames).toContain(WP009_COLLABORATION_EXTENSION_NAME);

    // Collaboration storage proves the extension actually initialized (not a
    // dangling reference). A live binding reports an enabled collaboration store.
    expect(ed.storage.collaboration).toBeDefined();
    expect(ed.storage.collaboration.isDisabled).toBe(false);

    // The binding writes editor content INTO the provided Yjs doc — proof the
    // hook is bound to *this* doc, not just present. Tiptap's Yjs sync uses the
    // "default" XML fragment.
    ed.commands.setContent({
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "collab-bound-MT-021" }] },
      ],
    });

    const fragment = ydoc.getXmlFragment("default");
    expect(fragment).toBeInstanceOf(XmlFragment);
    expect(fragment.length).toBeGreaterThan(0);
    expect(fragment.toString()).toContain("collab-bound-MT-021");
  });

  it("still exposes the RichDocument schema version in the collaborative set", () => {
    const ydoc = new YDoc();
    const extensions = buildWp009CollaborativeExtensionSet(ydoc);
    expect(richDocumentSchemaVersionOf(extensions)).toBe(
      WP009_RICH_DOCUMENT_SCHEMA_VERSION,
    );
  });
});
