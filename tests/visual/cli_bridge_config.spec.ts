import { expect, test } from "./console_error_scan";

// CLI-Bridge operator config surface (WP-KERNEL-004 follow-up).
//
// Synthetic visual spec mirroring steering_vector_editor.spec.ts: page.setContent
// with the structural HTML the React panel emits, asserting the stable
// data-testid surface + the GUI readability checks ([GLOBAL-BUILD-026..031]).
// Component behavior is covered by the colocated vitest in
// app/src/components/CliBridgeConfigPanel.test.tsx. Routing through
// console_error_scan also proves the panel surface raises no console errors.

const CLI_BRIDGE_HARNESS_HTML = `
  <main data-testid="capture-root" style="padding: 16px; font: 14px system-ui, sans-serif; max-width: 720px;">
    <div class="ans001-drawer settings-menu" role="dialog" aria-modal="true" aria-label="Settings" data-testid="settings-menu">
      <section class="settings-section" data-testid="settings-section-cli-bridge">
        <h4 class="settings-section__title">CLI Bridge (Official-CLI swarm lane)</h4>
        <div class="cli-bridge-config" data-testid="cli-bridge-config">
          <p class="muted small" data-testid="cli-bridge-config.intro">
            Configure the official CLI bridge so a CLI-backed swarm session runs in the app
            and streams its stdout into the terminal panel. Auth is via your own CLI login
            (<code>claude auth login</code>) — no API key is stored here.
          </p>

          <div class="settings-row" data-testid="cli-bridge-config.status-row">
            <div class="settings-row__main">
              <span class="settings-row__label">Status</span>
              <span class="settings-note" data-testid="cli-bridge-config.status">
                Not configured — official_cli swarm lane disabled
              </span>
            </div>
          </div>

          <div class="settings-row" data-testid="cli-bridge-config.preset-row">
            <div class="settings-row__main">
              <span class="settings-row__label">Preset</span>
              <span class="muted small">Prefills executable, args template, output format, and model allowlist.</span>
            </div>
            <select class="settings-row__control" aria-label="CLI bridge preset" data-testid="cli-bridge-config.preset">
              <option value="">Custom (no preset)</option>
              <option value="claude_code">Claude Code</option>
              <option value="codex_cli">Codex CLI</option>
              <option value="generic">Generic</option>
            </select>
          </div>

          <div class="settings-row" data-testid="cli-bridge-config.kind-row">
            <div class="settings-row__main">
              <span class="settings-row__label">CLI kind</span>
              <span class="muted small">Derived from the selected preset.</span>
            </div>
            <input type="text" class="settings-row__control" value="Generic / any CLI" readonly
              aria-label="CLI kind (derived from preset)" data-testid="cli-bridge-config.kind" />
          </div>

          <div class="settings-row" data-testid="cli-bridge-config.executable-row">
            <div class="settings-row__main">
              <span class="settings-row__label">Executable path</span>
              <span class="muted small">Full path or a PATH-resolvable name (e.g. <code>claude</code>).</span>
            </div>
            <input type="text" class="settings-row__control" placeholder="claude"
              aria-label="Executable path" data-testid="cli-bridge-config.executable" />
          </div>

          <div class="settings-row" data-testid="cli-bridge-config.args-row">
            <div class="settings-row__main">
              <span class="settings-row__label">Args template</span>
              <span class="muted small">One arg per line. Must contain {prompt}; may contain {model}.</span>
            </div>
            <textarea class="settings-row__control" rows="5" aria-label="Args template (one arg per line)"
              data-testid="cli-bridge-config.args">{prompt}</textarea>
          </div>

          <div class="settings-row" data-testid="cli-bridge-config.output-format-row">
            <div class="settings-row__main"><span class="settings-row__label">Output format</span></div>
            <select class="settings-row__control" aria-label="Output format" data-testid="cli-bridge-config.output-format">
              <option value="raw_text">Raw text (stdout)</option>
              <option value="json">JSON</option>
              <option value="json_stream">JSON stream (JSONL)</option>
            </select>
          </div>

          <div class="settings-row" data-testid="cli-bridge-config.allowlist-row">
            <div class="settings-row__main">
              <span class="settings-row__label">Model allowlist</span>
              <span class="muted small">Comma- or newline-separated CLI model names (at least one required).</span>
            </div>
            <textarea class="settings-row__control" rows="3" aria-label="Model allowlist"
              data-testid="cli-bridge-config.allowlist"></textarea>
          </div>

          <div class="settings-row" data-testid="cli-bridge-config.timeout-row">
            <div class="settings-row__main">
              <span class="settings-row__label">Timeout (seconds)</span>
              <span class="muted small">Per-spawn hard timeout; must be greater than 0.</span>
            </div>
            <input type="number" min="1" class="settings-row__control" value="120"
              aria-label="Timeout in seconds" data-testid="cli-bridge-config.timeout" />
          </div>

          <div class="settings-row" data-testid="cli-bridge-config.actions">
            <button type="button" class="settings-row__control" disabled data-testid="cli-bridge-config.save">
              Save configuration
            </button>
            <button type="button" class="secondary" disabled data-testid="cli-bridge-config.test">
              Test configuration
            </button>
            <button type="button" class="secondary" disabled data-testid="cli-bridge-config.clear">
              Clear
            </button>
          </div>
        </div>
      </section>
    </div>
  </main>
`;

test("cli_bridge_config exposes the stable config surface inside the settings menu", async ({ page }) => {
  await page.setContent(CLI_BRIDGE_HARNESS_HTML);

  await expect(page.locator("[data-testid='settings-menu']")).toBeVisible();
  await expect(page.locator("[data-testid='settings-section-cli-bridge']")).toBeVisible();
  await expect(page.locator("[data-testid='cli-bridge-config']")).toBeVisible();
  await expect(page.locator("[data-testid='cli-bridge-config.status']")).toContainText(
    /not configured/i,
  );
});

test("cli_bridge_config surfaces every required operator control", async ({ page }) => {
  await page.setContent(CLI_BRIDGE_HARNESS_HTML);

  for (const testid of [
    "cli-bridge-config.preset",
    "cli-bridge-config.kind",
    "cli-bridge-config.executable",
    "cli-bridge-config.args",
    "cli-bridge-config.output-format",
    "cli-bridge-config.allowlist",
    "cli-bridge-config.timeout",
    "cli-bridge-config.save",
    "cli-bridge-config.test",
    "cli-bridge-config.clear",
  ]) {
    await expect(page.locator(`[data-testid='${testid}']`)).toBeVisible();
  }
});

test("cli_bridge_config preset selector offers Claude Code / Codex CLI / Generic", async ({
  page,
}) => {
  await page.setContent(CLI_BRIDGE_HARNESS_HTML);
  const options = await page.locator("[data-testid='cli-bridge-config.preset'] option").allTextContents();
  expect(options.join("|")).toMatch(/Claude Code/);
  expect(options.join("|")).toMatch(/Codex CLI/);
  expect(options.join("|")).toMatch(/Generic/);
});

test("cli_bridge_config honest-disabled states render (no fake-working controls)", async ({
  page,
}) => {
  await page.setContent(CLI_BRIDGE_HARNESS_HTML);
  // Unconfigured + empty draft => Save/Test/Clear all honestly disabled.
  await expect(page.locator("[data-testid='cli-bridge-config.save']")).toBeDisabled();
  await expect(page.locator("[data-testid='cli-bridge-config.test']")).toBeDisabled();
  await expect(page.locator("[data-testid='cli-bridge-config.clear']")).toBeDisabled();
  // CLI kind is read-only (derived).
  await expect(page.locator("[data-testid='cli-bridge-config.kind']")).toHaveAttribute(
    "readonly",
    "",
  );
});

test("cli_bridge_config is readable: labels present and no zero-size control overlap", async ({
  page,
}) => {
  await page.setContent(CLI_BRIDGE_HARNESS_HTML);

  // GUI readability ([GLOBAL-BUILD-026..031]): each labelled row has a visible label.
  for (const label of ["Status", "Preset", "CLI kind", "Executable path", "Args template", "Model allowlist", "Timeout (seconds)"]) {
    await expect(page.getByText(label, { exact: false }).first()).toBeVisible();
  }

  // Controls have non-zero bounding boxes (no collapsed/overlapping widgets).
  for (const testid of ["cli-bridge-config.preset", "cli-bridge-config.executable", "cli-bridge-config.args", "cli-bridge-config.save"]) {
    const box = await page.locator(`[data-testid='${testid}']`).boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(0);
    expect(box!.height).toBeGreaterThan(0);
  }
});
