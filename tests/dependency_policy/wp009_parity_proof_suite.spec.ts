// WP-KERNEL-009 / MT-263 — UnifiedWorkSurface full-parity runtime proof suite.
//
// DEC-007 enforcement gate. This is the no-shortcuts whole-WP parity readout:
// an EXECUTABLE suite that exercises EVERY capability row of the parity sources
// in REAL runtime (offline Playwright against the BUILT product harnesses, with
// the network cut), tagged parity:<capability_id>, and emits a machine-readable
// parity-proof report mapping capability_id -> proof_test -> PASS|FAIL|DESCOPED.
//
//   - DESCOPED rows MUST cite a real operator DEC id (from packet.json
//     operator_decisions). A descope with an unknown DEC = the suite FAILS.
//   - Any non-descoped row whose inline runtime proof throws = FAIL = the suite
//     FAILS. There is NO third state.
//   - The row set is RE-MEASURED at runtime; the artifact's stored verdicts are
//     recorded as priorVerdict for drift visibility but are NOT trusted.
//   - Row-count guard: the editor rows MUST exactly match
//     .GOV/reference/wp009_editor_adversarial_parity_v2.json (no silent omission).
//   - Tamper guard (separate test): replacing a proof with a removed binding
//     flips the row to FAIL and the suite to red — proving the suite cannot mark
//     a row PASS without an executed per-row proof.
//
// The honest outcome is the point: a failing parity report is a VALID, useful
// result, not a defect to hide. The operator's separate validation session is
// the authority on accepting the readout.
//
// Run: cd app && pnpm run test:dependency-policy   (or filter to this spec)

import { expect, test } from "@playwright/test";
import { createServer, type Server } from "node:http";
import { createReadStream, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import type { AddressInfo } from "node:net";
import path from "node:path";
import { PNG } from "pngjs";
import {
  PARITY_ROWS,
  resolveVerdict,
  type HarnessId,
  type ProofContext,
} from "./parity/wp009_parity_registry";

// Repo-root-relative backing-spec paths use forward slashes; resolve portably.
function backingSpecAbsPath(spec: string): string {
  return path.join(repoRoot, ...spec.split("/"));
}

// A backing spec counts as PRESENT only if it exists AND is non-trivial (so an
// emptied or stub file cannot satisfy a relocated proof). 512 bytes is well
// below every real backing spec (smallest is ~3.4 KB) and well above any stub.
const MIN_BACKING_SPEC_BYTES = 512;

function backingSpecOk(spec: string): { ok: boolean; bytes: number } {
  const abs = backingSpecAbsPath(spec);
  if (!existsSync(abs) || !statSync(abs).isFile()) return { ok: false, bytes: 0 };
  const bytes = statSync(abs).size;
  return { ok: bytes >= MIN_BACKING_SPEC_BYTES, bytes };
}

// CJS context (Playwright transpiles spec files to CJS; import.meta unavailable).
const repoRoot = path.resolve(__dirname, "..", "..");
const appDir = path.join(repoRoot, "app");
const distHarness = path.join(appDir, "dist-harness");

// Same sibling artifact root the cargo target-dir + other lanes use
// (single ".." from the worktree root, per the MT artifact-root rule).
const artifactRoot =
  process.env.HANDSHAKE_ARTIFACT_ROOT ?? path.resolve(repoRoot, "..", "Handshake_Artifacts");
const reportDir = path.join(artifactRoot, "wp009-parity-proof");

// Read-only governance authority surfaces (via the worktree .GOV junction).
const editorParityArtifactPath = path.join(
  repoRoot,
  ".GOV",
  "reference",
  "wp009_editor_adversarial_parity_v2.json",
);
const packetPath = path.join(
  repoRoot,
  ".GOV",
  "task_packets",
  "WP-KERNEL-009-Project-Knowledge-Index-Loom-Rich-Editor-v1",
  "packet.json",
);

const HARNESS_HTML: Record<HarnessId, string> = {
  "editor-workbench-chrome": "harness/editor-workbench-chrome.html",
  "rich-editor": "harness/rich-editor.html",
  "rich-editor-embeds": "harness/rich-editor-embeds.html",
};

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

// The embeds harness fetches assets from the REAL Handshake asset routes
// (/workspaces/ws-embed-proof/assets/:id[/content]) against this same loopback
// origin (rich_editor_embeds_harness.tsx EMBED_PROOF_WORKSPACE). The runner
// serves real decodable PNG bytes so the image + slideshow embed NodeViews
// render genuine media offline (no mock, no committed fixture, no CDN). Video
// playback is additionally proven by the named executed editor_embeds_export_find
// spec in this same lane; this runner proves the image + slideshow media path.
const EMBED_WORKSPACE = "ws-embed-proof";

/** Real decodable 8x8 PNG, encoded by pngjs (no fixtures). */
function buildRealPng(): Buffer {
  const png = new PNG({ width: 8, height: 8 });
  for (let y = 0; y < png.height; y++) {
    for (let x = 0; x < png.width; x++) {
      const idx = (png.width * y + x) << 2;
      const red = (x + y) % 2 === 0;
      png.data[idx] = red ? 220 : 30;
      png.data[idx + 1] = 40;
      png.data[idx + 2] = red ? 60 : 200;
      png.data[idx + 3] = 255;
    }
  }
  return PNG.sync.write(png);
}

function pngAssetMetadata(assetId: string, bytes: Buffer): Record<string, unknown> {
  return {
    asset_id: assetId,
    workspace_id: EMBED_WORKSPACE,
    kind: "image",
    mime: "image/png",
    original_filename: `${assetId}.png`,
    content_hash: `hash-${assetId}`,
    size_bytes: bytes.byteLength,
    width: 8,
    height: 8,
    created_at: "2026-06-12T00:00:00Z",
    classification: "proof",
    exportable: true,
    is_proxy_of: null,
    proxy_asset_id: null,
  };
}

function serveDistHarness(): Promise<Server> {
  const png = buildRealPng();
  // Resolvable image + slideshow member assets the embeds harness references.
  const assets = new Map<string, Buffer>([
    ["img-ok", png],
    ["s1", png],
    ["s2", png],
  ]);
  const server = createServer((req, res) => {
    const urlPath = decodeURIComponent((req.url ?? "/").split("?")[0]);

    // Real Handshake asset routes for the embed-proof workspace.
    const assetMatch = new RegExp(
      `^/workspaces/${EMBED_WORKSPACE}/assets/([^/]+)(/content)?$`,
    ).exec(urlPath);
    if (assetMatch) {
      const bytes = assets.get(assetMatch[1]);
      if (!bytes) {
        res.writeHead(404, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: "asset_not_found" }));
        return;
      }
      if (assetMatch[2]) {
        res.writeHead(200, { "content-type": "image/png" });
        res.end(bytes);
      } else {
        res.writeHead(200, { "content-type": "application/json" });
        res.end(JSON.stringify(pngAssetMetadata(assetMatch[1], bytes)));
      }
      return;
    }

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

interface RowResult {
  capability_id: string;
  source: "editor" | "loom";
  label: string;
  prior_verdict: string;
  owner: string;
  proof_kind: "inline" | "harness" | "backed" | "descoped";
  /** Stable proof identity: this spec file + the parity:<id> tag, the
   *  real-backend backing spec, or the DEC-cited descope. Makes the report
   *  machine-readable + auditable. */
  proof_test: string;
  verdict: "PASS" | "FAIL" | "DESCOPED";
  dec_citation: string | null;
  evidence: string;
  error: string | null;
}

const KNOWN_DECS = new Set<string>();

function loadKnownDecs(): void {
  const packet = JSON.parse(readFileSync(packetPath, "utf8")) as {
    lifecycle?: { operator_decisions?: Array<{ id: string }> };
    operator_decisions?: Array<{ id: string }>;
  };
  // operator_decisions lives under packet.lifecycle in the WP-009 packet; fall
  // back to a top-level key for forward-compat.
  const decisions =
    packet.lifecycle?.operator_decisions ?? packet.operator_decisions ?? [];
  for (const d of decisions) KNOWN_DECS.add(d.id);
}

test.describe("WP-KERNEL-009 MT-263 full-parity runtime proof suite (DEC-007 enforcement)", () => {
  let server: Server;
  let baseUrl: string;
  const externalRequests: string[] = [];
  const results: RowResult[] = [];

  test.beforeAll(async () => {
    // Every inline-proof harness must be built (offline product assets).
    for (const html of Object.values(HARNESS_HTML)) {
      expect(
        existsSync(path.join(distHarness, html)),
        `dist-harness missing ${html} — run pnpm run build:harness`,
      ).toBe(true);
    }
    // Every harness-kind row's html must also be a built offline asset.
    for (const row of PARITY_ROWS) {
      if (row.proof.kind === "harness") {
        expect(
          existsSync(path.join(distHarness, row.proof.html)),
          `dist-harness missing ${row.proof.html} (parity:${row.id}) — run pnpm run build:harness`,
        ).toBe(true);
      }
    }
    loadKnownDecs();
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    if (server) await new Promise((resolve) => server.close(resolve));
  });

  // ---- Guard 1: row count matches the artifact (no silent omission). ----
  test("registry editor rows exactly match the adversarial parity artifact", () => {
    const artifact = JSON.parse(readFileSync(editorParityArtifactPath, "utf8")) as {
      parity: Array<{ capability: string }>;
    };
    const artifactIds = artifact.parity.map((r) => r.capability.split(" ")[0]).sort();
    const registryEditorIds = PARITY_ROWS.filter((r) => r.source === "editor")
      .map((r) => r.id)
      .sort();
    expect(
      registryEditorIds,
      "registry editor rows drifted from wp009_editor_adversarial_parity_v2.json",
    ).toEqual(artifactIds);
    // Every registry id is unique across editor + loom.
    const allIds = PARITY_ROWS.map((r) => r.id);
    expect(new Set(allIds).size, "duplicate capability ids in the registry").toBe(allIds.length);
  });

  // ---- The parity-proof suite: one executed proof per capability row. ----
  // A single parameterized test per row so the report is granular and a failing
  // row is attributable, while the network-block + result accumulation are shared.
  for (const row of PARITY_ROWS) {
    test(`parity:${row.id} — ${row.label}`, async ({ page }) => {
      const proofTest =
        row.proof.kind === "inline"
          ? `tests/dependency_policy/wp009_parity_proof_suite.spec.ts::parity:${row.id}`
          : row.proof.kind === "harness"
            ? `tests/dependency_policy/wp009_parity_proof_suite.spec.ts::parity:${row.id} (harness ${row.proof.html})`
            : row.proof.kind === "backed"
              ? `backed:${row.proof.spec}`
              : `descoped:${row.proof.dec}`;

      const base: Omit<RowResult, "verdict" | "error"> = {
        capability_id: row.id,
        source: row.source,
        label: row.label,
        prior_verdict: row.priorVerdict,
        owner: row.owner,
        proof_kind: row.proof.kind,
        proof_test: proofTest,
        dec_citation:
          row.proof.kind === "backed" || row.proof.kind === "descoped" ? row.proof.dec : null,
        evidence: "",
      };

      // ---- descoped (true structural limit / accepted P2): DEC-cite only. ----
      if (row.proof.kind === "descoped") {
        const decKnown = KNOWN_DECS.has(row.proof.dec);
        const verdict = resolveVerdict({ kind: "descoped", decKnown });
        results.push({
          ...base,
          verdict,
          evidence: row.proof.reason,
          error: decKnown ? null : `descope cites unknown DEC ${row.proof.dec}`,
        });
        expect(decKnown, `row ${row.id} descoped with unknown DEC ${row.proof.dec}`).toBe(true);
        return;
      }

      // ---- backed (relocated to a real-backend lane this offline lane cannot
      //      spawn): TAMPER-EVIDENT. The named backing spec MUST exist and be
      //      non-trivial, else the row FAILS — so deleting/emptying the
      //      relocated proof turns this suite red. ----
      if (row.proof.kind === "backed") {
        const decKnown = KNOWN_DECS.has(row.proof.dec);
        const spec = backingSpecOk(row.proof.spec);
        const verdict = resolveVerdict({ kind: "backed", decKnown, specOk: spec.ok });
        const error =
          !decKnown
            ? `backed row cites unknown DEC ${row.proof.dec}`
            : !spec.ok
              ? `backing proof missing/trivial: ${row.proof.spec} (${spec.bytes} bytes)`
              : null;
        results.push({
          ...base,
          verdict,
          evidence: `${row.proof.reason} [backing spec ${row.proof.spec} present: ${spec.bytes} bytes]`,
          error,
        });
        expect(decKnown, `row ${row.id} backed by unknown DEC ${row.proof.dec}`).toBe(true);
        expect(
          spec.ok,
          `row ${row.id} backing proof ${row.proof.spec} missing or trivial (${spec.bytes} bytes) — relocated proof removed`,
        ).toBe(true);
        return;
      }

      // ---- inline / harness: EXECUTE a real per-row runtime proof. ----
      // Install the offline network policy. For a harness with API routes the
      // fixture owns loopback-continue + API-fulfil + external-record; otherwise
      // block every non-loopback request (offline guarantee).
      const recordExternal = (url: string) => externalRequests.push(`${row.id}:${url}`);
      if (row.proof.kind === "harness" && row.proof.routes) {
        await row.proof.routes({ page, baseUrl, recordExternal });
      } else {
        await page.route("**/*", async (route) => {
          const url = route.request().url();
          if (url.startsWith(baseUrl)) {
            await route.continue();
            return;
          }
          if (!url.startsWith("about:")) recordExternal(url);
          await route.abort("connectionfailed");
        });
      }

      const harnessHtml = row.proof.kind === "inline" ? HARNESS_HTML[row.proof.harness] : row.proof.html;
      await page.goto(`${baseUrl}/${harnessHtml}`);
      const ctx: ProofContext = { page, baseUrl };
      let error: string | null = null;
      try {
        await row.proof.run(ctx);
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
      const verdict = resolveVerdict({ kind: row.proof.kind, inlineError: error });
      results.push({
        ...base,
        verdict,
        evidence: `executed offline against ${harnessHtml}`,
        error,
      });
      expect(error, `parity:${row.id} runtime proof failed: ${error}`).toBeNull();
    });
  }

  // ---- Emit the machine-readable report + suite-level invariants. ----
  test.afterAll(() => {
    if (results.length === 0) return; // beforeAll failed; nothing to report.
    const byVerdict = {
      PASS: results.filter((r) => r.verdict === "PASS").length,
      FAIL: results.filter((r) => r.verdict === "FAIL").length,
      DESCOPED: results.filter((r) => r.verdict === "DESCOPED").length,
    };
    // Break out how each row's verdict was REACHED, so the readout is honest
    // about which rows executed a per-row runtime proof IN this offline lane
    // vs. which were proven in a real-backend lane (tamper-evident existence)
    // vs. which are true structural-limit descopes.
    const byProofKind = {
      executed_inline: results.filter((r) => r.proof_kind === "inline").length,
      executed_harness: results.filter((r) => r.proof_kind === "harness").length,
      backed_real_backend: results.filter((r) => r.proof_kind === "backed").length,
      structural_descope: results.filter((r) => r.proof_kind === "descoped").length,
    };
    const report = {
      schema: "wp009.parity_proof_report@1",
      mt_id: "MT-263",
      generated_at_utc: new Date().toISOString(),
      sources: {
        editor: ".GOV/reference/wp009_editor_adversarial_parity_v2.json",
        loom: "derived from LoomObsidianNavigation + UnifiedWorkSurface MT contracts (MT-245..262)",
      },
      offline_guarantee: {
        external_requests_attempted: externalRequests.length,
        external_requests: externalRequests,
      },
      totals: {
        rows: results.length,
        ...byVerdict,
        uncited_fails: results.filter((r) => r.verdict === "FAIL").length,
        executed_runtime_proofs: byProofKind.executed_inline + byProofKind.executed_harness,
        by_proof_kind: byProofKind,
      },
      rows: results.sort((a, b) => a.capability_id.localeCompare(b.capability_id)),
    };
    mkdirSync(reportDir, { recursive: true });
    const reportPath = path.join(reportDir, "wp009_parity_proof_report.json");
    writeFileSync(reportPath, JSON.stringify(report, null, 2), "utf8");
    // Console breadcrumb so the operator/validator can find the readout.
    // eslint-disable-next-line no-console
    console.log(
      `[MT-263] parity-proof report: ${report.totals.PASS} PASS / ${report.totals.FAIL} FAIL / ${report.totals.DESCOPED} DESCOPED -> ${reportPath}`,
    );
  });
});
