import { expect, test } from "./console_error_scan";

import path from "node:path";

import { buildTerminalViewHarness } from "./build_terminal_view_harness";

const repoRoot = path.resolve(__dirname, "..", "..");
const baselineDir = path.join(repoRoot, ".GOV", "visual_baselines");

const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="padding:16px; background:#fff; color:#111; width:900px;">
      <div id="harness-root" style="width:860px; height:300px;"></div>
    </main>
  </body>
</html>`;

async function mountTerminalView(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  page: any,
  mode: "interactive" | "readonly",
): Promise<void> {
  const { js, css } = await buildTerminalViewHarness();
  await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
  await page.evaluate((m: string) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).__TERMINAL_VIEW_HARNESS_MODE__ = m;
  }, mode);
  await page.addScriptTag({ content: js });
  await page.locator("[data-testid='terminal-view-xterm-proof']").waitFor({ state: "attached" });
}

async function pasteIntoTerminal(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  page: any,
  text: string,
): Promise<void> {
  await page.locator(".xterm-helper-textarea").evaluate((textarea: HTMLTextAreaElement, value) => {
    const data = new DataTransfer();
    data.setData("text/plain", value);
    textarea.dispatchEvent(new ClipboardEvent("paste", {
      bubbles: true,
      cancelable: true,
      clipboardData: data,
    }));
  }, text);
}

test("real TerminalView mounts xterm and renders seeded scrollback", async ({ page }) => {
  await mountTerminalView(page, "interactive");

  await expect(page.locator(".xterm")).toBeVisible();
  await expect(page.locator(".xterm-rows")).toContainText("HANDSHAKE_XTERM_SCROLLBACK");
  const box = await page.locator("[data-testid='terminal-view-xterm-proof']").boundingBox();
  expect(box).not.toBeNull();
  if (box) {
    expect(box.width).toBeGreaterThan(300);
    expect(box.height).toBeGreaterThan(200);
  }

  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "terminal-view-xterm.png"),
  });
});

test("real TerminalView forwards stdin only when interactive", async ({ page }) => {
  await mountTerminalView(page, "interactive");
  await page.locator(".xterm-helper-textarea").click();
  await page.keyboard.type("echo HSK_XTERM_INPUT");
  await pasteIntoTerminal(page, " && echo HSK_XTERM_PASTE");
  await expect
    .poll(async () => page.evaluate(() => window.__terminalInputLog?.join("") ?? ""))
    .toContain("echo HSK_XTERM_INPUT && echo HSK_XTERM_PASTE");

  await mountTerminalView(page, "readonly");
  await page.locator(".xterm-helper-textarea").click();
  await page.keyboard.type("echo SHOULD_NOT_FORWARD");
  await pasteIntoTerminal(page, " && echo PASTE_SHOULD_NOT_FORWARD");
  await page.waitForTimeout(100);
  const readonlyInput = await page.evaluate(() => window.__terminalInputLog?.join("") ?? "");
  expect(readonlyInput).not.toContain("SHOULD_NOT_FORWARD");
  expect(readonlyInput).not.toContain("PASTE_SHOULD_NOT_FORWARD");
});
