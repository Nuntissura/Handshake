// WP-KERNEL-009 iteration-3 hardening (H3) — code-block keystroke containment.
//
// A top-priority ProseMirror plugin that makes the embedded Monaco code block a
// real keyboard island:
//
//   1. CONTAINMENT — modifier chords that bubble out of a code block (Monaco
//      resolves and stops most of its own keys; UNRESOLVED chords bubble) are
//      claimed here so they can never reach the prose keymaps. Without this,
//      StarterKit's native bindings ran against the PROSE state while the
//      operator typed in code — Mod-Alt-1 retitled the neighboring paragraph,
//      and the component keymap could REPLACE the node-selected code block
//      (the adversarial review's H3). Plain typing, Enter, Backspace, undo and
//      clipboard keys are deliberately NOT touched (see editor_keymap.ts).
//
//   2. SINGLE CHORD OWNER — Mod-Alt-c is advertised as "insert an embedded
//      Monaco code block", but StarterKit's plain codeBlock ALSO binds
//      Mod-Alt-c natively. Both fired before: StarterKit toggled a plain code
//      block AND the component handler opened its prompt. This plugin runs
//      first (extension priority 110 > StarterKit's 100) and inserts the
//      Monaco-backed block the catalog promises.
//
// The component-level DOM listener (RichTextEditor) keeps the palette escape
// hatch: chords this plugin claims are preventDefault-ed by ProseMirror, which
// the component listener detects via event.defaultPrevented.

import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import {
  chordFromEvent,
  shouldContainChordFromCodeBlock,
} from "../editor/editor_keymap";

/** Selector of the code-block NodeView host (Monaco or its textarea fallback). */
const CODE_BLOCK_HOST_SELECTOR = "[data-testid='monaco-code-block']";

export const CodeBlockKeystrokeGuard = Extension.create({
  name: "codeBlockKeystrokeGuard",
  // Before StarterKit (default 100) so this handleKeyDown sees chords first.
  priority: 110,

  addProseMirrorPlugins() {
    const editor = this.editor;
    return [
      new Plugin({
        key: new PluginKey("codeBlockKeystrokeGuard"),
        props: {
          handleKeyDown: (_view, event) => {
            const target = event.target instanceof Element ? event.target : null;
            if (target?.closest(CODE_BLOCK_HOST_SELECTOR)) {
              return shouldContainChordFromCodeBlock(event);
            }
            if (chordFromEvent(event) === "Mod-Alt-c" && !event.isComposing) {
              // Insert the embedded Monaco block (language is picked on the
              // block header). Returning true preventDefaults, so StarterKit's
              // plain-codeBlock toggle and the component listener stay out.
              return editor.commands.insertCodeBlockFromSlash();
            }
            return false;
          },
        },
      }),
    ];
  },
});
