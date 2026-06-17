// WP-KERNEL-009 iteration-3 hardening (H8/M19, proving H1/H3/H4/H6 in a REAL
// browser) — actual keyboard typing against the BUILT integrated editor.
//
// The adversarial review found ZERO real-typing coverage in any lane — every
// test appended at the end or drove commands, which is exactly the seam that
// hid the H1 echo-loop caret teleport. This spec drives the built harness
// (same offline static-server scaffold as rich_editor_roundtrip.spec.ts) with
// page.keyboard:
//   1. MID-DOCUMENT prose typing with a caret assertion after EVERY keystroke
//      (the reviewer's probe showed caret 6 -> 12/13 before the fix),
//   2. undo of typed text (EXT-UNDO smoke),
//   3. typing INSIDE real Monaco + a language switch, with the round-trip hash
//      independently recomputed in this spec (H4 browser proof),
//   4. Monaco column-selection/multi-cursor typing in the embedded code block,
//   5. prose multi-range simultaneous edit application,
//   6. chord containment inside real Monaco (H3 browser proof),
//   7. paste of fenced text through a real ClipboardEvent (H6 browser proof),
//   8. IME composition via CDP Input.imeSetComposition + Input.insertText
//      (composition set + commit; multi-segment candidate-window flows cannot
//      be emulated through CDP — that residue is documented here).
//
// All assertions read the editor's own machine-readable surfaces
// (__HS_EDITOR_DEBUG__ selection/codeBlocks + __RICH_EDITOR_HARNESS__.docJson
// + data-rt-hash), never screen-scraping.

import { expect, test, type Page } from "@playwright/test";
import { createServer, type Server } from "node:http";
import { createReadStream, existsSync, statSync } from "node:fs";
import type { AddressInfo } from "node:net";
import path from "node:path";

const repoRoot = path.resolve(__dirname, "..", "..");
const distHarness = path.join(repoRoot, "app", "dist-harness");

const CONTENT_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ttf": "font/ttf",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".wasm": "application/wasm",
};

function serveDistHarness(): Promise<Server> {
  const server = createServer((req, res) => {
    const urlPath = decodeURIComponent((req.url ?? "/").split("?")[0]);
    const safePath = path
      .normalize(urlPath)
      .replace(/^([/\\])+/, "")
      .replace(/^(\.\.([/\\]|$))+/, "");
    const filePath = path.join(distHarness, safePath);
    if (!filePath.startsWith(distHarness) || !existsSync(filePath) || !statSync(filePath).isFile()) {
      res.writeHead(404);
      res.end("not found");
      return;
    }
    res.writeHead(200, {
      "content-type": CONTENT_TYPES[path.extname(filePath).toLowerCase()] ?? "application/octet-stream",
    });
    createReadStream(filePath).pipe(res);
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

/** Same FNV-1a 32-bit hash as code_block_serialization.ts — duplicated HERE on
 * purpose so the spec verifies the product hash with an independent
 * implementation instead of importing the code under test. The separator is
 * NUL (U+0000) per the documented invariant — NOT a space (the product source
 * previously carried a literal NUL byte that read like a space). */
function fnvHash(language: string, code: string): string {
  const input = `${language}\u0000${code}`;
  let hash = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

interface DebugShape {
  selection: { from: number; to: number; empty: boolean };
  codeBlocks: Array<{ language: string; contentHash: string; codeLength: number }>;
  nodeCounts: Record<string, number>;
}

interface HarnessWindow {
  __HS_EDITOR_DEBUG__?: DebugShape;
  __RICH_EDITOR_HARNESS__?: { docJson: DocNode | null };
}

interface DocNode {
  type?: string;
  text?: string;
  attrs?: Record<string, unknown>;
  marks?: Array<{ type: string }>;
  content?: DocNode[];
}

async function readDebug(page: Page): Promise<DebugShape> {
  const debug = await page.evaluate(() => (window as unknown as HarnessWindow).__HS_EDITOR_DEBUG__);
  expect(debug, "editor debug payload missing").toBeTruthy();
  return debug!;
}

async function readDoc(page: Page): Promise<DocNode> {
  const doc = await page.evaluate(
    () => (window as unknown as HarnessWindow).__RICH_EDITOR_HARNESS__?.docJson ?? null,
  );
  expect(doc, "harness docJson missing").toBeTruthy();
  return doc!;
}

function collectText(node: DocNode): string {
  if (node.text) return node.text;
  return (node.content ?? []).map(collectText).join("");
}

function firstParagraphText(doc: DocNode): string {
  const para = (doc.content ?? []).find((n) => n.type === "paragraph");
  return para ? collectText(para) : "";
}

function firstCodeBlock(doc: DocNode): { language: string; code: string; contentHash: string } {
  let found: { language: string; code: string; contentHash: string } | null = null;
  const visit = (n: DocNode) => {
    if (found) return;
    if (n.type === "monacoCodeBlock") {
      const attrs = n.attrs ?? {};
      found = {
        language: String(attrs.language ?? ""),
        code: String(attrs.code ?? ""),
        contentHash: String(attrs.contentHash ?? ""),
      };
      return;
    }
    for (const child of n.content ?? []) visit(child);
  };
  visit(doc);
  expect(found, "no monacoCodeBlock in docJson").toBeTruthy();
  return found!;
}

function normalizeLineEndings(value: string): string {
  return value.replace(/\r\n/g, "\n");
}

function hasMark(node: DocNode, mark: string): boolean {
  if ((node.marks ?? []).some((m) => m.type === mark)) return true;
  return (node.content ?? []).some((child) => hasMark(child, mark));
}

test.describe("WP-KERNEL-009 iteration-3 REAL typing in the offline editor (network blocked)", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "rich-editor.html")),
      "dist-harness missing — global setup should have built it (pnpm run build:harness)",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  async function bootEditor(page: Page): Promise<void> {
    const externalRequests: string[] = [];
    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(baseUrl)) {
        await route.continue();
        return;
      }
      externalRequests.push(url);
      await route.abort("connectionfailed");
    });
    await page.goto(`${baseUrl}/harness/rich-editor.html`);
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();
    const codeBlock = page.getByTestId("monaco-code-block").first();
    await expect(codeBlock).toBeVisible();
    await expect.poll(async () => codeBlock.getAttribute("data-monaco-mounted")).toBe("true");
    expect(externalRequests, "external requests attempted during boot").toEqual([]);
  }

  /** Click the intro paragraph and park the caret after "Intro" (5 chars in).
   * The debug payload publishes on a coalesced microtask (M15), so the caret
   * position is polled until STABLE across two consecutive reads. */
  async function caretAfterIntro(page: Page): Promise<number> {
    const paragraph = page.locator("[data-testid='rich-text-editor-surface'] .ProseMirror p").first();
    await paragraph.click();
    await page.keyboard.press("Home");
    const homeFrom = await expectStableCaret(page);
    for (let i = 0; i < 5; i++) await page.keyboard.press("ArrowRight");
    const from = await expectStableCaret(page);
    expect(from).toBe(homeFrom + 5);
    return from;
  }

  /** Polls the debug selection until two consecutive reads agree on a
   * collapsed caret, then returns that position. */
  async function expectStableCaret(page: Page): Promise<number> {
    let last = -1;
    await expect
      .poll(async () => {
        const debug = await readDebug(page);
        if (!debug.selection.empty) return "selection-not-collapsed";
        if (debug.selection.from === last) return "stable";
        last = debug.selection.from;
        return `moving:${last}`;
      })
      .toBe("stable");
    return last;
  }

  async function windowSelectionText(page: Page): Promise<string> {
    return page.evaluate(() => window.getSelection()?.toString() ?? "");
  }

  async function selectParagraphTextRange(page: Page, start: number, length: number): Promise<void> {
    const paragraph = page.locator("[data-testid='rich-text-editor-surface'] .ProseMirror p").first();
    await paragraph.click();
    await page.keyboard.press("Home");
    for (let i = 0; i < start; i++) await page.keyboard.press("ArrowRight");
    await page.keyboard.down("Shift");
    for (let i = 0; i < length; i++) await page.keyboard.press("ArrowRight");
    await page.keyboard.up("Shift");
  }

  async function addCurrentProseMultiRange(page: Page): Promise<void> {
    await page.getByTestId("editor-open-overflow").click();
    await expect(page.getByTestId("rich-text-editor-overflow")).toBeVisible();
    const addRange = page.getByTestId("overflow-cmd-selection.addRange");
    await expect(addRange).toBeVisible({ timeout: 5000 });
    await addRange.click();
  }

  test("H1 (browser): mid-document typing keeps the caret advancing one position per keystroke", async ({ page }) => {
    await bootEditor(page);
    const from0 = await caretAfterIntro(page);

    // Type one character at a time and assert the caret after EVERY keystroke.
    // Pre-fix behavior: the echoed setContent teleported it to the doc end.
    const typed = ["X", "Y", "Z"];
    for (let i = 0; i < typed.length; i++) {
      await page.keyboard.type(typed[i]);
      await expect
        .poll(async () => (await readDebug(page)).selection.from, {
          message: `caret after keystroke ${i + 1}`,
        })
        .toBe(from0 + i + 1);
    }

    // The characters landed mid-paragraph (not at the end).
    await expect
      .poll(async () => firstParagraphText(await readDoc(page)))
      .toBe("IntroXYZ paragraph with a typed link and an embed.");
  });

  test("EXT-UNDO smoke: Mod-z reverts typed text", async ({ page }) => {
    await bootEditor(page);
    await caretAfterIntro(page);
    await page.keyboard.type("UNDOME");
    await expect
      .poll(async () => firstParagraphText(await readDoc(page)))
      .toContain("IntroUNDOME");

    // Typed run groups into history; undo until the marker is gone (bounded).
    for (let i = 0; i < 4; i++) {
      await page.keyboard.press("Control+z");
      const text = firstParagraphText(await readDoc(page));
      if (!text.includes("UNDOME")) break;
    }
    await expect
      .poll(async () => firstParagraphText(await readDoc(page)))
      .toBe("Intro paragraph with a typed link and an embed.");
  });

  test("H4 (browser): typing in real Monaco + language switch keeps the round-trip hash correct", async ({ page }) => {
    await bootEditor(page);
    const monaco = page.locator("[data-testid='monaco-code-block-host'] .monaco-editor").first();
    await monaco.click();
    await page.keyboard.press("Control+End");
    await page.keyboard.type("\n// typed in monaco");

    // The node attrs update through the NodeView bridge.
    await expect
      .poll(async () => firstCodeBlock(await readDoc(page)).code)
      .toContain("// typed in monaco");
    let block = firstCodeBlock(await readDoc(page));
    expect(block.language).toBe("typescript");
    // Independent recomputation of the product hash (FNV duplicated here).
    expect(block.contentHash).toBe(fnvHash(block.language, block.code));

    // Switch the language, then type MORE — the stale-closure defect hashed
    // these keystrokes against the mount-time language.
    await page.getByTestId("monaco-code-block-language").selectOption("javascript");
    await expect
      .poll(async () => firstCodeBlock(await readDoc(page)).language)
      .toBe("javascript");
    await monaco.click();
    await page.keyboard.press("Control+End");
    await page.keyboard.type("\n// after switch");
    await expect
      .poll(async () => firstCodeBlock(await readDoc(page)).code)
      .toContain("// after switch");

    block = firstCodeBlock(await readDoc(page));
    expect(block.language).toBe("javascript");
    expect(block.contentHash).toBe(fnvHash("javascript", block.code));
    // The visible selector mirrors the same hash (MT-172 surface).
    await expect(page.getByTestId("monaco-code-block").first()).toHaveAttribute(
      "data-rt-hash",
      block.contentHash,
    );
  });

  test("MT-251 (browser): Monaco column selection applies same-column multi-cursor edits", async ({ page }) => {
    await bootEditor(page);
    const monaco = page.locator("[data-testid='monaco-code-block-host'] .monaco-editor").first();
    await monaco.click();
    await page.keyboard.press("Control+A");
    await page.keyboard.type("abc\nabc\nabc");
    await expect
      .poll(async () => normalizeLineEndings(firstCodeBlock(await readDoc(page)).code))
      .toBe("abc\nabc\nabc");

    await page.keyboard.press("Control+Home");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.down("Shift");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.up("Shift");
    await page.keyboard.type("Z");

    await expect
      .poll(async () => normalizeLineEndings(firstCodeBlock(await readDoc(page)).code))
      .toBe("aZbc\naZbc\naZbc");
  });

  test("MT-251 (browser): prose snippets expand and Tab traverses remapped tab-stops", async ({ page }) => {
    await bootEditor(page);
    const paragraph = page.locator("[data-testid='rich-text-editor-surface'] .ProseMirror p").first();
    await paragraph.click();
    await page.keyboard.press("End");

    await page.getByTestId("editor-open-overflow").click();
    await expect(page.getByTestId("rich-text-editor-overflow")).toBeVisible();
    await page.getByTestId("overflow-cmd-snippet.prose.meeting").click();

    await expect(page.getByTestId("rich-text-editor")).toHaveAttribute("data-snippet-active", "true");
    await expect.poll(() => windowSelectionText(page)).toBe("Topic");

    await page.keyboard.type("Roadmap");
    await page.keyboard.press("Tab");
    await expect.poll(() => windowSelectionText(page)).toBe("Owner");

    await page.keyboard.type("Ilja");
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("rich-text-editor")).toHaveAttribute("data-snippet-active", "false");
    await expect
      .poll(async () => firstParagraphText(await readDoc(page)))
      .toContain("Meeting: Roadmap / Owner: Ilja / Notes: ");
  });

  test("MT-251 (browser): code snippets use Monaco tab-stop traversal", async ({ page }) => {
    await bootEditor(page);
    const codeBlock = page.getByTestId("monaco-code-block").first();
    const monaco = page.locator("[data-testid='monaco-code-block-host'] .monaco-editor").first();
    await monaco.click();
    await page.keyboard.press("Control+A");
    await page.keyboard.press("Backspace");
    await expect.poll(async () => normalizeLineEndings(firstCodeBlock(await readDoc(page)).code)).toBe("");

    await codeBlock.getByTestId("monaco-code-snippet-code.function").click();
    await page.keyboard.type("build");
    await page.keyboard.press("Tab");
    await page.keyboard.type("input");
    await page.keyboard.press("Tab");
    await page.keyboard.type("return input;");

    await expect
      .poll(async () => normalizeLineEndings(firstCodeBlock(await readDoc(page)).code))
      .toBe("function build(input) {\n    return input;\n}");
  });

  test("MT-251 (browser): prose multi-range selections receive simultaneous typed edits", async ({ page }) => {
    await bootEditor(page);

    await selectParagraphTextRange(page, 0, "Intro".length);
    await addCurrentProseMultiRange(page);
    await expect(page.getByTestId("rich-text-editor")).toHaveAttribute("data-multi-range-count", "1");

    await selectParagraphTextRange(page, "Intro paragraph with a ".length, "typed".length);
    await addCurrentProseMultiRange(page);
    await expect(page.getByTestId("rich-text-editor")).toHaveAttribute("data-multi-range-count", "2");

    const paragraph = page.locator("[data-testid='rich-text-editor-surface'] .ProseMirror p").first();
    await paragraph.click();
    await page.keyboard.type("X");

    await expect
      .poll(async () => firstParagraphText(await readDoc(page)))
      .toBe("X paragraph with a X link and an embed.");
    await expect(page.getByTestId("rich-text-editor")).toHaveAttribute("data-multi-range-count", "2");
  });

  test("H3 (browser): chords typed inside real Monaco edit code only — prose state never mutates", async ({ page }) => {
    await bootEditor(page);
    const before = JSON.stringify(await readDoc(page));
    const monaco = page.locator("[data-testid='monaco-code-block-host'] .monaco-editor").first();
    await monaco.click();

    // Mod-Alt-c (insert code block) and Mod-b (bold) are NOT Monaco bindings;
    // they bubble to the prose layer, where the keystroke guard must contain
    // them: no new block, no bold mark, no node replacement.
    await page.keyboard.press("Control+Alt+c");
    await page.keyboard.press("Control+b");
    await page.waitForTimeout(250);

    const debug = await readDebug(page);
    expect(debug.codeBlocks.length, "a chord typed in Monaco created/destroyed a code block").toBe(1);
    const doc = await readDoc(page);
    expect(hasMark(doc, "bold"), "Mod-b typed in Monaco bolded prose").toBe(false);
    expect(JSON.stringify(doc), "Monaco-origin chords mutated the document").toBe(before);

    // Sanity: Monaco still receives plain typing afterwards.
    await page.keyboard.press("Control+End");
    await page.keyboard.type("x");
    await expect.poll(async () => firstCodeBlock(await readDoc(page)).code).toContain("x");
  });

  test("H6 (browser): pasting fenced text through a real ClipboardEvent creates an embedded code block", async ({ page }) => {
    await bootEditor(page);
    // Caret into prose first (paste lands at the prose selection).
    const paragraph = page.locator("[data-testid='rich-text-editor-surface'] .ProseMirror p").first();
    await paragraph.click();
    await page.keyboard.press("End");

    await page.evaluate(() => {
      const el = document.querySelector("[data-testid='rich-text-editor-surface'] .ProseMirror");
      if (!el) throw new Error("prose surface missing");
      const dt = new DataTransfer();
      dt.setData("text/plain", "```python\nx = 1\n```");
      el.dispatchEvent(
        new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }),
      );
    });

    // A SECOND code block appears, typed as python, carrying the pasted code.
    await expect.poll(async () => (await readDebug(page)).codeBlocks.length).toBe(2);
    const doc = await readDoc(page);
    const blocks: Array<{ language: string; code: string; contentHash: string }> = [];
    const visit = (n: DocNode) => {
      if (n.type === "monacoCodeBlock") {
        const attrs = n.attrs ?? {};
        blocks.push({
          language: String(attrs.language ?? ""),
          code: String(attrs.code ?? ""),
          contentHash: String(attrs.contentHash ?? ""),
        });
      }
      for (const child of n.content ?? []) visit(child);
    };
    visit(doc);
    const pasted = blocks.find((b) => b.language === "python");
    expect(pasted, "pasted python block missing").toBeTruthy();
    expect(pasted!.code).toBe("x = 1");
    expect(pasted!.contentHash).toBe(fnvHash("python", "x = 1"));
  });

  test("M6/M17 (browser): Escape exits the embedded Monaco editor back to prose typing", async ({ page }) => {
    await bootEditor(page);
    const monaco = page.locator("[data-testid='monaco-code-block-host'] .monaco-editor").first();
    await monaco.click();
    const codeBefore = firstCodeBlock(await readDoc(page)).code;

    // Escape leaves the code island (WCAG 2.1.2 keyboard-trap exit)...
    await page.keyboard.press("Escape");
    // ...and subsequent typing lands in the PROSE document, not the code.
    await page.keyboard.type("escaped-to-prose");
    await expect
      .poll(async () => JSON.stringify(await readDoc(page)))
      .toContain("escaped-to-prose");
    const block = firstCodeBlock(await readDoc(page));
    expect(block.code, "Escape-exit typing leaked into the code block").toBe(codeBefore);
    expect(block.code).not.toContain("escaped-to-prose");
  });

  test("IME (CDP): composition set + commit lands exactly once mid-document", async ({ page, context }) => {
    await bootEditor(page);
    const from0 = await caretAfterIntro(page);

    // COVERAGE NOTE: CDP Input.imeSetComposition emulates compositionstart/
    // update; Input.insertText commits (compositionend + insert). This covers
    // the composition lifecycle the echo loop used to break. NOT covered (no
    // CDP surface): multi-segment candidate windows / segment re-selection.
    const client = await context.newCDPSession(page);
    await client.send("Input.imeSetComposition", { text: "k", selectionStart: 1, selectionEnd: 1 });
    await client.send("Input.imeSetComposition", { text: "か", selectionStart: 1, selectionEnd: 1 });
    await client.send("Input.insertText", { text: "か" });

    await expect
      .poll(async () => firstParagraphText(await readDoc(page)))
      .toBe("Introか paragraph with a typed link and an embed.");
    // Exactly once — a broken composition duplicates or drops the glyph.
    const text = firstParagraphText(await readDoc(page));
    expect(text.split("か").length - 1).toBe(1);
    // Caret advanced past the committed glyph.
    const debug = await readDebug(page);
    expect(debug.selection.from).toBe(from0 + 1);
  });
});
