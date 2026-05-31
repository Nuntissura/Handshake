import { expect, test } from "./console_error_scan";

import path from "node:path";

import { buildSwarmSpawnHarness } from "./build_swarm_spawn_harness";

// WP-KERNEL-004 governance glue #2 — operator-facing worktree / disk-location /
// isolation-tier ASSIGNMENT on swarm spawn. jsdom cannot judge layout / overlap
// / readability, so the operator requirement "test via Handshake's own tools" is
// met here in a GENUINE Chromium render of the REAL <SwarmControlRoom> spawn
// form (real useSwarmRoom hook, real SwarmSpawnSection, real SwarmSessionsSection
// Worktree column). The only stand-in is a deterministic Tauri IPC mock supplied
// inside the harness — not a replacement for the form DOM.
//
// We assert, against the real layout engine + the real component output:
//   - the worktree picker, disk working-dir field, and isolation-tier selector
//     are present and operable,
//   - selecting "+ New worktree…" reveals the free-text new-name input,
//   - the mandatory "recorded, not enforced" honesty note is visible,
//   - the sessions table renders the Worktree column (assigned id + "—"),
//   - the controls are distinct, non-overlapping hit targets,
//   - a read-only PNG baseline is written under .GOV/visual_baselines/.

const repoRoot = path.resolve(__dirname, "..", "..");
const baselineDir = path.join(repoRoot, ".GOV", "visual_baselines");

const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; }
    label { display: block; margin: 4px 0; }
    label > span { display: block; font-size: 12px; }
    fieldset { margin: 8px 0; padding: 8px; }
    table { border-collapse: collapse; width: 100%; }
    th, td { border: 1px solid #ddd; padding: 4px 6px; font-size: 12px; text-align: left; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="padding:16px; background:#fff; color:#111; width:1100px;">
      <div id="harness-root"></div>
    </main>
  </body>
</html>`;

/** Render the real SwarmControlRoom spawn form into the page. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function mountSpawnForm(page: any): Promise<void> {
  const { js, css } = await buildSwarmSpawnHarness();
  await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
  await page.addScriptTag({ content: js });
  await page.locator("[data-testid='swarm-spawn-form']").waitFor({ state: "attached" });
}

test("real spawn form exposes the worktree picker, disk field, and recorded-only isolation tier + honesty note", async ({ page }) => {
  await mountSpawnForm(page);

  // The three assignment controls are present and operable.
  await expect(page.locator("[data-testid='swarm-spawn-worktree-select']")).toBeVisible();
  await expect(page.locator("[data-testid='swarm-spawn-working-dir']")).toBeVisible();
  await expect(page.locator("[data-testid='swarm-spawn-isolation-tier']")).toBeVisible();

  // The discovered worktree (from the mocked listWorktrees) is offered as an option.
  await expect(
    page.locator("[data-testid='swarm-spawn-worktree-select'] option", { hasText: "wt-feature-x" }),
  ).toHaveCount(1);

  // The mandatory honesty note is visible (recorded, not enforced).
  await expect(page.locator("[data-testid='swarm-isolation-note']")).toBeVisible();
  await expect(page.locator("[data-testid='swarm-isolation-note']")).toContainText(
    /recorded, not yet enforced/i,
  );

  // The new-name input is hidden until the sentinel is chosen, then revealed.
  await expect(page.locator("[data-testid='swarm-spawn-worktree-new']")).toHaveCount(0);
  await page.locator("[data-testid='swarm-spawn-worktree-select']").selectOption("__new__");
  await expect(page.locator("[data-testid='swarm-spawn-worktree-new']")).toBeVisible();

  // The controls are distinct, non-overlapping hit targets (no clobbering).
  const wtBox = await page.locator("[data-testid='swarm-spawn-worktree-select']").boundingBox();
  const tierBox = await page.locator("[data-testid='swarm-spawn-isolation-tier']").boundingBox();
  expect(wtBox).not.toBeNull();
  expect(tierBox).not.toBeNull();
  if (wtBox && tierBox) {
    const overlapX = Math.max(0, Math.min(wtBox.x + wtBox.width, tierBox.x + tierBox.width) - Math.max(wtBox.x, tierBox.x));
    const overlapY = Math.max(0, Math.min(wtBox.y + wtBox.height, tierBox.y + tierBox.height) - Math.max(wtBox.y, tierBox.y));
    expect(overlapX * overlapY).toBe(0);
    expect(wtBox.width).toBeGreaterThan(20);
    expect(wtBox.height).toBeGreaterThan(10);
  }

  // Write the reviewed PNG baseline under .GOV/visual_baselines.
  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "swarm-spawn-assignment.png"),
  });
});

test("real sessions table renders the Worktree column (assigned id + honest dash)", async ({ page }) => {
  await mountSpawnForm(page);

  // The mocked live sessions land in the table with their assigned worktree.
  await expect(page.locator("[data-testid='swarm-session-worktree-alpha-model#0']")).toContainText(
    "wt-feature-x",
  );
  // The unassigned cloud session shows an honest dash, never a fabricated lane.
  await expect(page.locator("[data-testid='swarm-session-worktree-beta-cloud#0']")).toContainText("—");
});
