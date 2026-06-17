// WP-KERNEL-009 / MT-234 - editor visual regression fixture.
//
// A production-built harness for the real RichTextEditor, using compact
// adjacent panes so Playwright can prove desktop/narrow overlap behavior for
// editor text, controls, media embeds, graph/backlink context, and UserManual
// links without needing a live backend or a full app shell.

import { StrictMode, useCallback, useEffect } from "react";
import { createRoot } from "react-dom/client";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "../components/RichTextEditor";
import { EDITOR_DEBUG_ENABLE_KEY, EDITOR_DEBUG_GLOBAL_KEY } from "../lib/editor/visual_debug";
import type { EmbedResolverContext } from "../lib/editor/embed_assets";
import "../App.css";

(globalThis as Record<string, unknown>)[EDITOR_DEBUG_ENABLE_KEY] = true;

declare global {
  interface Window {
    __EDITOR_VISUAL_REGRESSION_HARNESS__?: {
      docJson: JSONContent;
      debug: unknown;
    };
  }
}

function link(refKind: string, refValue: string, label: string): JSONContent {
  return {
    type: "hsLink",
    attrs: { refKind, refValue, label, resolved: true },
  };
}

const INITIAL_DOC: JSONContent = {
  type: "doc",
  content: [
    {
      type: "heading",
      attrs: { level: 1 },
      content: [{ type: "text", text: "Visual regression fixture" }],
    },
    {
      type: "paragraph",
      content: [
        { type: "text", text: "Operator-facing links: " },
        link("wp", "WP-KERNEL-009", "WP-KERNEL-009"),
        { type: "text", text: " " },
        link("note", "Visual Debug Loop", "Visual Debug Loop"),
        { type: "text", text: " " },
        link("spec", "7.1.1.8", "UserManual 7.1.1.8"),
      ],
    },
    {
      type: "paragraph",
      content: [
        { type: "text", text: "Embed states: " },
        link("images", "img-ok", "Fixture image asset"),
        { type: "text", text: " " },
        link("slideshow", "slide-a,slide-b", "Two-slide sequence"),
      ],
    },
    {
      type: "paragraph",
      content: [
        { type: "text", text: "Compact visual fixture for backlink and manual adjacency." },
      ],
    },
  ],
};

const embedContext: EmbedResolverContext = {
  workspaceId: "ws-mt234-visual",
  apiBaseUrl: window.location.origin,
};

const fixtureCss = `
  .mt234-root {
    box-sizing: border-box;
    width: 100vw;
    height: 100vh;
    padding: 12px;
    overflow: hidden;
    background: var(--hs-color-bg);
  }

  .mt234-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(260px, 340px);
    gap: 12px;
    height: 100%;
    min-height: 0;
  }

  .mt234-editor-panel,
  .mt234-sidecar-panel {
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--hs-color-border);
    border-radius: 8px;
    background: var(--hs-color-surface);
    box-sizing: border-box;
  }

  .mt234-editor-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    overflow: hidden;
  }

  .mt234-header {
    flex: 0 0 auto;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: baseline;
  }

  .mt234-header h1,
  .mt234-sidecar-panel h2,
  .mt234-sidecar-panel h3 {
    margin: 0;
    font-size: 14px;
    line-height: 1.2;
  }

  .mt234-header p,
  .mt234-sidecar-panel p,
  .mt234-sidecar-panel li {
    margin: 0;
    font-size: 12px;
    line-height: 1.35;
  }

  .mt234-editor-shell {
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
  }

  .mt234-editor-shell .rich-text-editor {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100%;
    min-height: 0;
    box-sizing: border-box;
  }

  .mt234-editor-shell .rich-text-editor__toolbar {
    flex: 0 0 auto;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    padding: 6px;
    border: 1px solid var(--hs-color-border);
    border-radius: 6px;
    background: var(--hs-color-bg);
    box-sizing: border-box;
  }

  .mt234-editor-shell button,
  .mt234-sidecar-panel a {
    border: 1px solid var(--hs-color-border);
    border-radius: 6px;
    background: var(--hs-color-surface);
    color: var(--hs-color-text);
    padding: 4px 7px;
    font: inherit;
    text-decoration: none;
  }

  .mt234-editor-shell button {
    cursor: pointer;
  }

  .mt234-editor-shell button:disabled {
    color: var(--hs-color-text-subtle);
    cursor: not-allowed;
  }

  .mt234-editor-shell .rich-text-editor__surface {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    border: 1px solid var(--hs-color-border);
    border-radius: 6px;
    padding: 10px;
    background: var(--hs-color-surface);
    box-sizing: border-box;
  }

  .mt234-editor-shell .rich-text-editor__surface[data-focus-mode="true"] {
    min-height: min(52vh, 560px);
    border-color: var(--hs-color-focus, #2563eb);
    box-shadow: inset 0 0 0 1px rgba(37, 99, 235, 0.18);
  }

  .mt234-editor-shell .ProseMirror {
    min-height: 100%;
    outline: none;
  }

  .mt234-editor-shell .ProseMirror h1 {
    margin: 0 0 6px;
    font-size: 16px;
    line-height: 1.2;
  }

  .mt234-editor-shell .ProseMirror p {
    margin: 0 0 6px;
    line-height: 1.3;
  }

  .mt234-editor-shell .rich-text-editor__palette,
  .mt234-editor-shell .rich-text-editor__overflow,
  .mt234-editor-shell .rich-text-editor__arg-prompt,
  .mt234-editor-shell .rich-text-editor__export-menu {
    position: absolute;
    z-index: 10;
    top: 48px;
    right: 10px;
    width: min(340px, calc(100% - 20px));
    max-height: min(420px, calc(100% - 60px));
    overflow: auto;
    border: 1px solid var(--hs-color-border-strong);
    border-radius: 8px;
    padding: 10px;
    background: var(--hs-color-surface);
    box-shadow: 0 12px 30px rgba(15, 23, 42, 0.18);
    box-sizing: border-box;
  }

  .mt234-editor-shell .rich-text-editor__palette-input {
    width: 100%;
    box-sizing: border-box;
    margin-bottom: 8px;
  }

  .mt234-editor-shell .rich-text-editor__palette-list,
  .mt234-editor-shell .rich-text-editor__export-list,
  .mt234-sidecar-panel ul {
    display: flex;
    flex-direction: column;
    gap: 6px;
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .mt234-editor-shell .rich-text-editor__palette-item,
  .mt234-editor-shell .rich-text-editor__overflow-item {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    text-align: left;
  }

  .mt234-sidecar {
    display: grid;
    grid-template-rows: repeat(3, minmax(0, 1fr));
    gap: 12px;
    min-height: 0;
    overflow: hidden;
  }

  .mt234-sidecar-panel {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 10px;
    overflow: visible;
  }

  .mt234-sidecar-panel a {
    display: inline-flex;
    max-width: 100%;
    width: fit-content;
  }

  .mt234-pill-row {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .mt234-pill {
    display: inline-flex;
    max-width: 100%;
    border: 1px solid var(--hs-color-border);
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 11px;
    line-height: 1.4;
    color: var(--hs-color-text-subtle);
    background: var(--hs-color-bg);
  }

  .mt234-editor-shell .hs-link {
    display: inline-flex;
    max-width: 100%;
    margin: 0 2px 2px;
    border: 1px solid var(--hs-color-border);
    border-radius: 6px;
    padding: 2px 5px;
    font-size: 12px;
    vertical-align: baseline;
  }

  .mt234-editor-shell .hs-embed__error,
  .mt234-editor-shell .hs-embed__loading {
    max-width: 100%;
    white-space: normal;
  }

  @media (max-width: 700px) {
    .mt234-root {
      padding: 8px;
    }

    .mt234-layout {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(0, 1fr) 320px;
      gap: 8px;
    }

    .mt234-editor-panel {
      padding: 8px;
    }

    .mt234-editor-shell .rich-text-editor__surface[data-focus-mode="true"] {
      min-height: 0;
    }

    .mt234-header {
      align-items: flex-start;
      flex-direction: column;
      gap: 4px;
    }

    .mt234-sidecar {
      gap: 8px;
    }

    .mt234-sidecar-panel {
      padding: 8px;
      gap: 4px;
    }

    .mt234-sidecar-panel p,
    .mt234-sidecar-panel li {
      font-size: 11px;
    }
  }
`;

function HarnessShell() {
  const onChange = useCallback((docJson: JSONContent) => {
    window.__EDITOR_VISUAL_REGRESSION_HARNESS__ = {
      docJson,
      debug: (globalThis as Record<string, unknown>)[EDITOR_DEBUG_GLOBAL_KEY],
    };
  }, []);

  useEffect(() => {
    window.__EDITOR_VISUAL_REGRESSION_HARNESS__ = {
      docJson: INITIAL_DOC,
      debug: (globalThis as Record<string, unknown>)[EDITOR_DEBUG_GLOBAL_KEY],
    };
  }, []);

  return (
    <main className="mt234-root" data-testid="editor-visual-regression-root">
      <style>{fixtureCss}</style>
      <section className="mt234-layout" data-testid="editor-visual-regression-layout">
        <article className="mt234-editor-panel" data-testid="editor-visual-regression-editor-panel">
          <header className="mt234-header">
            <div>
              <p className="app-eyebrow">WP-KERNEL-009 / MT-234</p>
              <h1>Editor visual regression fixture</h1>
            </div>
            <p className="muted">Desktop and narrow overlap proof</p>
          </header>
          <div className="mt234-editor-shell">
            <RichTextEditor
              initialContent={INITIAL_DOC}
              onChange={onChange}
              embedContext={embedContext}
              documentTitle="MT-234 visual regression fixture"
              debugId="mt234-editor-visual-regression"
            />
          </div>
        </article>

        <aside
          className="mt234-sidecar"
          data-testid="editor-visual-regression-sidecar"
          aria-label="Editor visual context"
        >
          <section
            className="mt234-sidecar-panel"
            data-testid="editor-visual-regression-embeds"
            aria-label="Embed fixture states"
          >
            <h2>Embeds</h2>
            <p>Media refs render through local loopback asset routes.</p>
            <div className="mt234-pill-row" aria-label="Embed references">
              <span className="mt234-pill">images:img-ok</span>
              <span className="mt234-pill">slideshow:slide-a,slide-b</span>
            </div>
          </section>

          <section
            className="mt234-sidecar-panel"
            data-testid="editor-visual-regression-backlinks"
            aria-label="Graph and backlink fixture"
          >
            <h2>Graph / Backlinks</h2>
            <ul>
              <li>note -&gt; Visual Debug Loop</li>
              <li>wp -&gt; WP-KERNEL-009</li>
              <li>palette command -&gt; graph.backlink</li>
            </ul>
          </section>

          <section
            className="mt234-sidecar-panel"
            data-testid="editor-visual-regression-usermanual"
            aria-label="UserManual fixture links"
          >
            <h2>UserManual</h2>
            <p>Spec chips and command entries keep manual navigation visible.</p>
            <a
              href="#usermanual-7.1.1.8"
              data-testid="editor-visual-regression-usermanual-link"
              aria-label="Open UserManual section 7.1.1.8"
            >
              UserManual 7.1.1.8
            </a>
          </section>
        </aside>
      </section>
    </main>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
