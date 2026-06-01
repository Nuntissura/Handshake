import { expect, test } from "./console_error_scan";

import path from "node:path";

import { buildSessionWorkbenchHarness } from "./build_session_workbench_harness";

// WP-KERNEL-004 governance glue #3 — lift operator chat to ALL providers + the
// combined "Session" workbench (chat + captured terminal + transcript link).
// jsdom cannot judge layout / overlap / readability, so the operator requirement
// "test via Handshake's own tools" is met here in a GENUINE Chromium render of
// the REAL <SwarmOperatorSurface> (its real useSwarmRoom hook, the real
// SessionWorkbench, the real provider-rich OperatorChat picker, the real shared
// TerminalPanel + SessionReplayPanel drawers). The only stand-in is a
// deterministic Tauri IPC mock supplied inside the harness — not a replacement
// for the DOM.
//
// We assert, against the real layout engine + the real component output:
//   - the chat picker lists local + cloud + CLI sessions (the local-only filter
//     is gone), each tagged with its provider,
//   - a non-live (CANCELLED) session is offered as a DISABLED option (honest),
//   - selecting a cloud session enables the composer and a real generate renders
//     the model turn,
//   - "Show captured terminal" reveals the shared TerminalPanel focused on this
//     session's captured stdout (bound by instance_id),
//   - "Open full transcript" reveals the shared SessionReplayPanel,
//   - a read-only PNG baseline is written under .GOV/visual_baselines/.

const repoRoot = path.resolve(__dirname, "..", "..");
const baselineDir = path.join(repoRoot, ".GOV", "visual_baselines");

const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; }
    label { display: block; margin: 4px 0; }
    label > span { display: block; font-size: 12px; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="padding:16px; background:#fff; color:#111; width:1100px;">
      <div id="harness-root"></div>
    </main>
  </body>
</html>`;

/** Render the real SwarmOperatorSurface into the page and open the Session disclosure. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function mountSurface(page: any): Promise<void> {
  const { js, css } = await buildSessionWorkbenchHarness();
  await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
  await page.addScriptTag({ content: js });
  await page.locator("[data-testid='swarm-operator-surface']").waitFor({ state: "attached" });
  // Open the "Session" workbench disclosure (id kept stable as operator-chat).
  await page.locator("[data-testid='disclosure-operator-chat-toggle']").click();
  await page.locator("[data-testid='session-workbench']").waitFor({ state: "visible" });
}

test("the Session workbench chat picker lists local + cloud + CLI sessions (local filter gone)", async ({ page }) => {
  await mountSurface(page);

  const picker = page.locator("[data-testid='operator-chat-session']");
  await expect(picker).toBeVisible();

  // All three providers appear as options (data-provider tags them). The
  // local-only filter is gone: cloud + CLI sessions are chattable.
  await expect(picker.locator("option[data-provider='local']")).toHaveCount(1);
  await expect(picker.locator("option[data-provider='byok_cloud']")).toHaveCount(2); // beta (READY) + delta (CANCELLED)
  await expect(picker.locator("option[data-provider='official_cli']")).toHaveCount(1);

  // The non-live CANCELLED session is offered as a DISABLED option (honest).
  // (Playwright's toBeDisabled does not classify a disabled <option> as
  // disabled, so assert the attribute directly — the option is genuinely
  // non-selectable in the real browser.)
  await expect(
    page.locator("[data-testid='operator-chat-option-delta-dead#0']"),
  ).toHaveAttribute("disabled", "");

  // Write the reviewed PNG baseline under .GOV/visual_baselines.
  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "session-workbench.png"),
  });
});

test("selecting a CLOUD session enables chat, sends a real generate, and reveals terminal + transcript", async ({ page }) => {
  await mountSurface(page);

  // Select the cloud (BYOK) session; the composer becomes enabled.
  await page.locator("[data-testid='operator-chat-session']").selectOption("beta-cloud#0");
  const input = page.locator("[data-testid='operator-chat-input']");
  await expect(input).toBeEnabled();

  // Send a message -> a real (mocked-IPC) generate renders the model turn.
  await input.fill("Hello cloud.");
  await page.locator("[data-testid='operator-chat-send']").click();
  await expect(page.locator("[data-role='model']")).toContainText("Hello from the cloud session.");

  // "Show captured terminal" reveals the shared TerminalPanel focused on THIS
  // session's captured stdout (bound by instance_id beta-cloud#0).
  await page.locator("[data-testid='session-workbench-show-terminal']").click();
  await expect(page.locator("[data-testid='disclosure-terminal-toggle']")).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  await expect(page.locator("[data-testid='terminal-panel-active']")).toHaveAttribute(
    "data-active-session",
    "cap-beta",
  );

  // "Open full transcript" reveals the shared SessionReplayPanel.
  await page.locator("[data-testid='session-workbench-open-transcript']").click();
  await expect(page.locator("[data-testid='disclosure-session-replay-toggle']")).toHaveAttribute(
    "aria-expanded",
    "true",
  );
});
