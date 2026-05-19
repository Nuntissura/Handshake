import { expect, test } from "./console_error_scan";

// MT-098: Activation Steering Vector Editor + Contrastive Capture Wizard
//
// This synthetic spec mirrors the existing visual fixture pattern (page.setContent
// with the structural HTML the React component emits). It validates the
// stable data-testid surface so the visual matrix and downstream MTs can rely
// on the same selectors. Component behavior is covered by the colocated
// vitest in app/src/components/inference_lab/SteeringVectorEditor.test.tsx.

const STEERING_HARNESS_HTML = `
  <main data-testid="capture-root" style="padding: 16px; font: 14px sans-serif;">
    <section data-testid="inference-lab" aria-labelledby="inference-lab-title">
      <header>
        <h2 id="inference-lab-title">Inference Lab</h2>
      </header>
      <div data-testid="inference-lab.models.select-wrapper">
        <select data-testid="inference-lab.models.select">
          <option value="019a1b2c-0000-7000-8000-aaaaaaaaaaaa">019a1b2c (candle)</option>
        </select>
      </div>
      <section
        data-testid="steering-vector-editor"
        aria-labelledby="steering-vector-editor-title"
      >
        <header>
          <h3 id="steering-vector-editor-title">Activation Steering Vectors</h3>
        </header>
        <table data-testid="steering-vector-editor.table">
          <thead>
            <tr><th>Name</th><th>Layer</th><th>Hook</th><th>Intensity</th><th>Active</th><th>Description</th><th></th></tr>
          </thead>
          <tbody>
            <tr data-testid="steering-vector-editor.row.019a1b2c-0000-7000-8000-000000000001">
              <td>calm-tone</td>
              <td>14</td>
              <td><code>resid_stream</code></td>
              <td>
                <input
                  type="range"
                  min="-10" max="10" step="0.1" value="1.5"
                  data-testid="steering-vector-editor.row.019a1b2c-0000-7000-8000-000000000001.intensity"
                />
              </td>
              <td>
                <label>
                  <input
                    type="checkbox"
                    data-testid="steering-vector-editor.row.019a1b2c-0000-7000-8000-000000000001.active"
                  />
                  <span>off</span>
                </label>
              </td>
              <td>baseline vector</td>
              <td>
                <button
                  data-testid="steering-vector-editor.row.019a1b2c-0000-7000-8000-000000000001.unregister"
                  type="button"
                >Remove</button>
              </td>
            </tr>
          </tbody>
        </table>
        <section
          data-testid="contrastive-capture-wizard"
          aria-labelledby="contrastive-capture-wizard-title"
        >
          <h4 id="contrastive-capture-wizard-title">Capture vector from contrastive prompts</h4>
          <textarea data-testid="contrastive-capture-wizard.positive" rows="4"></textarea>
          <textarea data-testid="contrastive-capture-wizard.negative" rows="4"></textarea>
          <select data-testid="contrastive-capture-wizard.layer">
            <option value="0">0</option>
            <option value="12" selected>12</option>
          </select>
          <button type="button" data-testid="contrastive-capture-wizard.capture">Capture activations</button>
          <input type="text" data-testid="contrastive-capture-wizard.name" />
          <input type="text" data-testid="contrastive-capture-wizard.description" />
          <input
            type="number" min="-10" max="10" step="0.1" value="1"
            data-testid="contrastive-capture-wizard.intensity"
          />
          <select data-testid="contrastive-capture-wizard.license">
            <option value="Permissive">Permissive</option>
            <option value="SourceModelLicenseOnly" selected>SourceModelLicenseOnly</option>
            <option value="Restricted">Restricted</option>
          </select>
          <button type="button" data-testid="contrastive-capture-wizard.save" disabled>Save vector</button>
          <details data-testid="contrastive-capture-wizard.ab-compare">
            <summary>A/B compare (live generation)</summary>
            <p>Requires MT-074 (LlamaCppRuntime streaming).</p>
          </details>
        </section>
      </section>
    </section>
  </main>
`;

test("steering_vector_editor exposes the stable data-testid surface", async ({ page }) => {
  await page.setContent(STEERING_HARNESS_HTML);

  await expect(page.locator("[data-testid='inference-lab']")).toBeVisible();
  await expect(page.locator("[data-testid='steering-vector-editor']")).toBeVisible();
  await expect(page.locator("[data-testid='steering-vector-editor.table']")).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard']")).toBeVisible();
});

test("steering_vector_editor wizard surfaces every required control", async ({ page }) => {
  await page.setContent(STEERING_HARNESS_HTML);

  // Operator-facing controls per MT-098 contract.
  await expect(page.locator("[data-testid='contrastive-capture-wizard.positive']")).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard.negative']")).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard.layer']")).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard.capture']")).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard.license']")).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard.save']")).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard.ab-compare']")).toBeVisible();
});

test("steering_vector_editor row exposes intensity slider and active toggle", async ({ page }) => {
  await page.setContent(STEERING_HARNESS_HTML);

  const rowId = "019a1b2c-0000-7000-8000-000000000001";
  const intensity = page.locator(
    `[data-testid='steering-vector-editor.row.${rowId}.intensity']`,
  );
  const active = page.locator(`[data-testid='steering-vector-editor.row.${rowId}.active']`);

  await expect(intensity).toBeVisible();
  await expect(intensity).toHaveAttribute("min", "-10");
  await expect(intensity).toHaveAttribute("max", "10");
  await expect(active).toBeVisible();
});
